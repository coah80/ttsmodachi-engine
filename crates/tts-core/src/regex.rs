use crate::frontend_types::Error;

#[derive(Clone, Copy)]
pub struct PcreTables<'a> {

    pub op_lengths: &'a [u8; 256],

    pub ctypes: &'a [u8; 256],

    pub default_tables: &'a [u8; 1088],

    pub escapes: &'a [i16; 75],

    pub utf8_limits: &'a [i32; 6],

    pub utf8_first: &'a [i32; 6],

    pub utf8_cont: &'a [u8; 64],

    pub utf8_mask: &'a [u32; 6],

    pub rep_min: &'a [u8; 6],
    pub rep_max: &'a [u8; 6],

    pub posix_lengths: &'a [u8],

    pub posix_names: &'a [&'a [u8]],

    pub posix_maps: &'a [i32],

    pub ctypes_overspill: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompileError {
    BackslashAtEnd,
    ControlAtEnd,
    UnrecognizedEscape,
    BadRangeOrder,
    RangeTooBig,
    MissingClassEnd,
    BadClassEscape,
    RangeOutOfOrder,
    NothingToRepeat,
    EmptyRepeat,
    UnexpectedRepeat,
    BadGroupOpen,
    PosixOutsideClass,
    MissingParen,
    BadSubpatternRef,
    NullErrorOffset,
    UnknownOption,
    MissingCommentEnd,
    NestingTooDeep,
    PatternTooLarge,
    NoMemory,
    UnmatchedParens,
    CodeOverflow,
    BadLookbehindOpen,
    VariableLookbehind,
    BadConditionNumber,
    TooManyCondBranches,
    CondNeedsAssertion,
    BadRecurseSyntax,
    UnknownPosixClass,
    NoCollatingElements,
    NoUtf8Support,
    Spare,
    BadHexValue,
    BadConditionZero,
    ByteInLookbehind,
    NoCaseEscape,
    CalloutTooBig,
    MissingCalloutEnd,
    RecurseLoop,
    BadNameSyntax,
    BadNameUse,
    DuplicateName,
    BadUtf8,
    NoPropertySupport,
    BadPropertySequence,
    UnknownProperty,
}

impl CompileError {

    pub fn id(self) -> u32 {
        self as u32
    }
}

pub trait PcreHost {

    fn validate(&mut self, object: u32, key: u32, tag: u32, size: u32) -> u32;

    fn resolve_owner(&mut self, handle: u32, key: i32) -> (u32, u32);

    fn heap_of(&mut self, owner: u32) -> u32;

    fn alloc(&mut self, heap: u32, count: u32, size: u32) -> Option<u32>;

    fn free(&mut self, heap: u32, id: u32);

    fn guest_callout(&mut self, _block: &CalloutBlock) -> i32 {
        0
    }
}

pub fn encode_utf8(value: i32, buffer: &mut [u8], tables: &PcreTables<'_>) -> Result<usize, Error> {
    let mut i = 0;
    while i < 6 && value > tables.utf8_limits[i] {
        i += 1;
    }
    let first = *tables.utf8_first.get(i).ok_or(Error::InvalidResource)?;
    if buffer.len() < i + 1 {
        return Err(Error::Capacity {
            required: i + 1,
            available: buffer.len(),
        });
    }
    let mut v = value;
    for slot in buffer[1..i + 1].iter_mut().rev() {
        *slot = 0x80 | (v & 0x3f) as u8;
        v >>= 6;
    }
    buffer[0] = (first | v) as u8;
    Ok(i + 1)
}

fn byte_at(pattern: &[u8], offset: usize) -> Result<u8, Error> {
    pattern.get(offset).copied().ok_or(Error::InvalidResource)
}

pub fn is_range_quantifier(
    pattern: &[u8],
    offset: usize,
    tables: &PcreTables<'_>,
) -> Result<bool, Error> {
    let mut p = offset;
    if tables.ctypes[byte_at(pattern, p)? as usize] & 4 == 0 {
        return Ok(false);
    }
    p += 1;
    while tables.ctypes[byte_at(pattern, p)? as usize] & 4 != 0 {
        p += 1;
    }
    if byte_at(pattern, p)? == b'}' {
        return Ok(true);
    }
    if byte_at(pattern, p)? != b',' {
        return Ok(false);
    }
    p += 1;
    if byte_at(pattern, p)? == b'}' {
        return Ok(true);
    }
    if tables.ctypes[byte_at(pattern, p)? as usize] & 4 == 0 {
        return Ok(false);
    }
    p += 1;
    while tables.ctypes[byte_at(pattern, p)? as usize] & 4 != 0 {
        p += 1;
    }
    Ok(byte_at(pattern, p)? == b'}')
}

pub struct RangeRead {
    pub offset: usize,
    pub min: i32,
    pub max: i32,
}

pub fn read_range(
    pattern: &[u8],
    offset: usize,
    tables: &PcreTables<'_>,
) -> Result<Result<RangeRead, (usize, CompileError)>, Error> {
    let mut p = offset;
    let mut min: i32 = 0;
    let mut max: i32 = -1;
    while tables.ctypes[byte_at(pattern, p)? as usize] & 4 != 0 {
        min = min
            .wrapping_mul(10)
            .wrapping_add(byte_at(pattern, p)? as i32)
            .wrapping_sub('0' as i32);
        p += 1;
    }
    if byte_at(pattern, p)? == b'}' {
        max = min;
    } else {
        p += 1;
        if byte_at(pattern, p)? != b'}' {
            max = 0;
            while tables.ctypes[byte_at(pattern, p)? as usize] & 4 != 0 {
                max = max
                    .wrapping_mul(10)
                    .wrapping_add(byte_at(pattern, p)? as i32)
                    .wrapping_sub('0' as i32);
                p += 1;
            }
            if max < min {
                return Ok(Err((p, CompileError::BadRangeOrder)));
            }
        }
    }
    if min > 65535 || max > 65535 {
        return Ok(Err((p, CompileError::RangeTooBig)));
    }
    Ok(Ok(RangeRead {
        offset: p,
        min,
        max,
    }))
}

pub struct EscapeRead {
    pub offset: usize,
    pub value: i32,
    pub error: Option<CompileError>,
}

pub fn read_escape(
    pattern: &[u8],
    offset: usize,
    brackets: i32,
    options: u32,
    in_class: bool,
    tables: &PcreTables<'_>,
) -> Result<EscapeRead, Error> {
    let mut p = offset.wrapping_add(1);
    let mut c = byte_at(pattern, p)? as i32;
    if c == 0 {
        return Ok(EscapeRead {
            offset: p,
            value: 0,
            error: Some(CompileError::BackslashAtEnd),
        });
    }
    if (c.wrapping_sub('0' as i32) as u32) >= 75 {
        return Ok(EscapeRead {
            offset: p,
            value: c,
            error: None,
        });
    }
    let n = tables.escapes[(c - '0' as i32) as usize] as i32;
    if n != 0 {
        return Ok(EscapeRead {
            offset: p,
            value: n,
            error: None,
        });
    }
    let error = match c as u8 {
        b'L' | b'N' | b'U' | b'l' | b'u' => Some(CompileError::NoCaseEscape),
        b'1'..=b'9' => {
            if !in_class {
                let start = p;
                c -= '0' as i32;
                while tables.ctypes[byte_at(pattern, p.wrapping_add(1))? as usize] & 4 != 0 {
                    p += 1;
                    c = c
                        .wrapping_mul(10)
                        .wrapping_add(byte_at(pattern, p)? as i32)
                        .wrapping_sub('0' as i32);
                }
                if c < 10 || c <= brackets {
                    c = (c.wrapping_add(20)).wrapping_neg();
                    return Ok(EscapeRead {
                        offset: p,
                        value: c,
                        error: None,
                    });
                }
                p = start;
            }
            c = byte_at(pattern, p)? as i32;
            if c >= '8' as i32 {
                return Ok(EscapeRead {
                    offset: p.wrapping_sub(1),
                    value: 0,
                    error: None,
                });
            }
            c -= '0' as i32;
            let mut count = 0;
            while count < 2 {
                let next = byte_at(pattern, p.wrapping_add(1))? as i32;
                if next < '0' as i32 || next > '7' as i32 {
                    break;
                }
                p += 1;
                c = c
                    .wrapping_mul(8)
                    .wrapping_add(byte_at(pattern, p)? as i32)
                    .wrapping_sub('0' as i32);
                count += 1;
            }
            c &= 255;
            None
        }
        b'0' => {
            c -= '0' as i32;
            let mut count = 0;
            while count < 2 {
                let next = byte_at(pattern, p.wrapping_add(1))? as i32;
                if next < '0' as i32 || next > '7' as i32 {
                    break;
                }
                p += 1;
                c = c
                    .wrapping_mul(8)
                    .wrapping_add(byte_at(pattern, p)? as i32)
                    .wrapping_sub('0' as i32);
                count += 1;
            }
            c &= 255;
            None
        }
        b'x' => {
            if byte_at(pattern, p.wrapping_add(1))? == b'{' && (options & 0x800) != 0 {
                let mut q = p.wrapping_add(2);
                c = 0;
                let mut count = 0;
                while tables.ctypes[byte_at(pattern, q)? as usize] & 8 != 0 {
                    let mut digit = byte_at(pattern, q)? as i32;
                    q += 1;
                    if digit >= 'a' as i32 {
                        digit -= 32;
                    }
                    c = c.wrapping_mul(16).wrapping_add(digit.wrapping_sub(
                        if digit < 'A' as i32 {
                            '0' as i32
                        } else {
                            'A' as i32 - 10
                        },
                    ));
                    count += 1;
                }
                if byte_at(pattern, q)? == b'}' {
                    p = q;
                    if c < 0 || count > 8 {
                        return Ok(EscapeRead {
                            offset: p,
                            value: c,
                            error: Some(CompileError::BadHexValue),
                        });
                    }
                    return Ok(EscapeRead {
                        offset: p,
                        value: c,
                        error: None,
                    });
                }
            }
            c = 0;
            let mut count = 0;
            while count < 2 && tables.ctypes[byte_at(pattern, p.wrapping_add(1))? as usize] & 8 != 0
            {
                p += 1;
                let mut digit = byte_at(pattern, p)? as i32;
                if digit >= 'a' as i32 {
                    digit -= 32;
                }
                c = c
                    .wrapping_mul(16)
                    .wrapping_add(digit.wrapping_sub(if digit < 'A' as i32 {
                        '0' as i32
                    } else {
                        'A' as i32 - 10
                    }));
                count += 1;
            }
            None
        }
        b'c' => {
            p += 1;
            c = byte_at(pattern, p)? as i32;
            if c == 0 {

                return Ok(EscapeRead {
                    offset,
                    value: 0,
                    error: Some(CompileError::ControlAtEnd),
                });
            }
            if c >= 'a' as i32 && c <= 'z' as i32 {
                c -= 32;
            }
            c ^= 64;
            None
        }
        _ => {
            if (options & 0x40) != 0 {
                Some(CompileError::UnrecognizedEscape)
            } else {
                None
            }
        }
    };

    Ok(EscapeRead {
        offset: p,
        value: c,
        error,
    })
}

fn decode_xclass_char(
    program: &[u8],
    cursor: &mut usize,
    tables: &PcreTables<'_>,
) -> Result<i32, Error> {
    let b = byte_at(program, *cursor)? as i32;
    *cursor += 1;
    let mut v = b;
    if (b & 0xc0) == 0xc0 {
        let n = tables.utf8_cont[(b & 63) as usize] as i32;
        let mut shift = n.wrapping_mul(6);
        let mask = *tables
            .utf8_mask
            .get(n as usize)
            .ok_or(Error::InvalidResource)? as i32;
        v = (b & mask).wrapping_shl(shift as u32);

        let mut q = *cursor - 1;
        shift -= 6;
        if n & 1 != 0 {
            q += 1;
            v |= ((byte_at(program, q)? as i32) & 63).wrapping_shl(shift as u32);
            shift -= 6;
        }
        let mut pairs = n >> 1;
        while pairs != 0 {
            v |= ((byte_at(program, q.wrapping_add(1))? as i32) & 63).wrapping_shl(shift as u32);
            shift -= 6;
            v |= ((byte_at(program, q.wrapping_add(2))? as i32) & 63).wrapping_shl(shift as u32);
            shift -= 6;
            q += 2;
            pairs -= 1;
        }
        *cursor = (*cursor as u32).wrapping_add(n as u32) as usize;
    }
    Ok(v)
}

pub fn xclass_match(c: i32, program: &[u8], tables: &PcreTables<'_>) -> Result<bool, Error> {
    let flags = byte_at(program, 0)?;
    let neg = flags & 1 != 0;
    if c < 256
        && (flags & 2) != 0
        && (byte_at(program, 1 + (c / 8) as usize)? & (1 << (c as u32 & 7))) != 0
    {
        return Ok(!neg);
    }
    let mut cursor = 1usize;
    if (flags & 2) != 0 {
        cursor += 32;
    }
    loop {
        let b = byte_at(program, cursor)?;
        cursor += 1;
        if b == 0 {
            return Ok(neg);
        }
        if b == 1 {
            let v = decode_xclass_char(program, &mut cursor, tables)?;
            if v == c {
                return Ok(!neg);
            }
        } else if b == 2 {
            let lo = decode_xclass_char(program, &mut cursor, tables)?;
            let hi = decode_xclass_char(program, &mut cursor, tables)?;
            if lo <= c && c <= hi {
                return Ok(!neg);
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct MatchFrame {

    pub prev: Option<FrameRef>,

    pub eptr: i64,

    pub ecode: usize,

    pub offset_top: i32,

    pub ims: u32,

    pub eptrb: Option<FrameRef>,

    pub flags: u32,

    pub callpat: usize,
    pub charptr: usize,
    pub data: usize,
    pub next: usize,
    pub pp: i64,
    pub prev_code: usize,
    pub saved_eptr: i64,

    pub rec_prev: Option<FrameRef>,
    pub rec_group: i32,
    pub rec_after: usize,
    pub rec_start: i64,
    pub rec_saved_max: i32,
    pub rec_scratch: bool,
    pub rec_scratch_base: usize,

    pub rec_spill: [i32; 1024],

    pub cur_is_word: i32,
    pub condition: i32,
    pub minimize: i32,
    pub prev_is_word: i32,

    pub original_ims: u32,

    pub ctype: i32,
    pub fc: i32,
    pub fi: i32,
    pub length: i32,
    pub max: i32,
    pub min: i32,
    pub number: i32,
    pub offset: i32,
    pub op: i32,

    pub save_capture_last: i32,
    pub save_offset1: i32,
    pub save_offset2: i32,
    pub save_offset3: i32,

    pub stacksave: [i32; 30],

    pub block_eptr: i64,
    pub block_prev: Option<FrameRef>,

    pub result: i32,
}

impl Default for MatchFrame {
    fn default() -> Self {
        Self {
            prev: None,
            eptr: 0,
            ecode: 0,
            offset_top: 0,
            ims: 0,
            eptrb: None,
            flags: 0,
            callpat: 0,
            charptr: 0,
            data: 0,
            next: 0,
            pp: 0,
            prev_code: 0,
            saved_eptr: 0,
            rec_prev: None,
            rec_group: 0,
            rec_after: 0,
            rec_start: 0,
            rec_saved_max: 0,
            rec_scratch: false,
            rec_scratch_base: 0,
            rec_spill: [0; 1024],
            cur_is_word: 0,
            condition: 0,
            minimize: 0,
            prev_is_word: 0,
            original_ims: 0,
            ctype: 0,
            fc: 0,
            fi: 0,
            length: 0,
            max: 0,
            min: 0,
            number: 0,
            offset: 0,
            op: 0,
            save_capture_last: 0,
            save_offset1: 0,
            save_offset2: 0,
            save_offset3: 0,
            stacksave: [0; 30],
            block_eptr: 0,
            block_prev: None,
            result: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameBlockState {
    pub present: bool,
    pub live: bool,
    pub used: u32,
    pub count: u8,

    pub code_id: u32,
    pub extra_id: u32,
    pub entry_id: u32,
}

pub struct FrameStore<'a> {

    pub frames: &'a mut [MatchFrame],

    pub tags: &'a mut [u8],

    pub blocks: &'a mut [FrameBlockState],
    pub frames_per_block: u16,
    pub frame_size: u16,
    pub max_blocks: u16,
    pub block_count: u16,
    pub current_block: u16,
    pub highest_block: u16,
    pub max_frames: u16,

    pub fresh: MatchFrame,

    pub heap: u32,

    pub blocks_id: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameRef {
    pub block: u16,
    pub offset: u32,
}

impl FrameStore<'_> {
    fn block_base(&self, block: u16) -> Result<usize, Error> {
        let per = self.frames_per_block as usize;
        let base = (block as usize)
            .checked_mul(per)
            .ok_or(Error::InvalidResource)?;
        if base.wrapping_add(per) > self.frames.len() || base.wrapping_add(per) > self.tags.len() {
            return Err(Error::Capacity {
                required: base.wrapping_add(per),
                available: self.frames.len().min(self.tags.len()),
            });
        }
        Ok(base)
    }

    pub fn alloc(&mut self, entry: u8, host: &mut impl PcreHost) -> Result<FrameRef, u32> {
        if self.block_count == 0 {
            return Err(0x8a502000);
        }
        let frame_size = self.frame_size as u32;
        let used = match self.blocks.get(self.current_block as usize) {
            Some(header) if header.present => header.used,
            _ => return Err(0x8a50200a),
        };
        let capacity = frame_size
            .wrapping_mul(self.frames_per_block as u32)
            .wrapping_add(1);
        if capacity <= frame_size.wrapping_add(used) {
            if (self.block_count as u32).wrapping_add(1) >= self.max_blocks as u32 {
                return Err(0x8a502000);
            }
            if (self.current_block as u32).wrapping_add(1) >= self.block_count as u32 {
                let status = self.allocate_block(host);
                if status != 0 {
                    return Err(status);
                }
            }
            self.current_block = self.current_block.wrapping_add(1);
            if self.current_block > self.highest_block {
                self.highest_block = self.current_block;
            }
        }
        let current = self.current_block;
        let base = match self.block_base(current) {
            Ok(base) => base,
            Err(_) => return Err(0x8a50200a),
        };
        let header = match self.blocks.get_mut(current as usize) {
            Some(h) => h,
            None => return Err(0x8a50200a),
        };
        if !header.present {
            return Err(0x8a50200a);
        }
        let offset = header.used;
        header.used = header.used.wrapping_add(frame_size);
        let slot = match base.checked_add(header.count as usize) {
            Some(slot) => slot,
            None => return Err(0x8a50200a),
        };
        match self.tags.get_mut(slot) {
            Some(tag) => *tag = entry,
            None => return Err(0x8a50200a),
        }
        header.count = header.count.wrapping_add(1);
        if (header.count as u16) > self.max_frames {
            self.max_frames = header.count as u16;
        }
        Ok(FrameRef {
            block: current,
            offset,
        })
    }

    fn allocate_block(&mut self, host: &mut impl PcreHost) -> u32 {
        let at = self.block_count as usize;
        if self.blocks.get(at).is_none() {
            return 0x8a50200a;
        }
        let created = host.alloc(self.heap, 1, 20);
        if created.is_none() {
            self.blocks[at] = FrameBlockState {
                present: false,
                live: false,
                used: 0,
                count: 0,
                code_id: 0,
                extra_id: 0,
                entry_id: 0,
            };
            return 0x8a50200a;
        }
        let created = created.unwrap_or(0);
        let capacity = (self.frame_size as u32)
            .wrapping_mul(self.frames_per_block as u32)
            .wrapping_add(1);
        let data = host.alloc(self.heap, 1, capacity);
        if data.is_none() {
            host.free(self.heap, created);
            self.blocks[at] = FrameBlockState {
                present: true,
                live: false,
                used: 0,
                count: 0,
                code_id: 0,
                extra_id: 0,
                entry_id: created,
            };
            return 0x8a50200a;
        }
        let data = data.unwrap_or(0);
        let types = host.alloc(self.heap, 1, (self.frames_per_block as u32).wrapping_add(1));
        let types_id = match types {
            Some(id) => id,
            None => {
                host.free(self.heap, data);
                host.free(self.heap, created);
                self.blocks[at] = FrameBlockState {
                    present: true,
                    live: false,
                    used: 0,
                    count: 0,
                    code_id: 0,
                    extra_id: 0,
                    entry_id: created,
                };
                return 0x8a50200a;
            }
        };
        self.blocks[at] = FrameBlockState {
            present: true,
            live: true,
            used: 0,
            count: 0,
            code_id: data,
            extra_id: types_id,
            entry_id: created,
        };

        if let Ok(base) = self.block_base(self.block_count) {
            let per = self.frames_per_block as usize;
            for slot in self.frames.iter_mut().skip(base).take(per) {
                *slot = self.fresh;
            }
            for slot in self.tags.iter_mut().skip(base).take(per) {
                *slot = 0;
            }
        }
        self.block_count = self.block_count.wrapping_add(1);
        0
    }

    pub fn pop(&mut self) -> Option<u8> {
        if self.block_count == 0 {
            return None;
        }
        let mut current = self.current_block;
        let header = self.blocks.get(current as usize)?;
        if header.used == 0 {
            if current == 0 {
                return None;
            }
            current -= 1;
            self.current_block = current;
        }
        let base = self.block_base(current).ok()?;
        let frame_size = self.frame_size as u32;
        let header = self.blocks.get_mut(current as usize)?;
        header.used = header.used.wrapping_sub(frame_size);
        header.count = header.count.wrapping_sub(1);
        let tag = *self.tags.get(base + header.count as usize)?;
        Some(tag)
    }

    pub fn slot(&self, frame: FrameRef) -> Result<usize, Error> {
        let per = self.frames_per_block as usize;
        let base = self.block_base(frame.block)?;
        let index = frame
            .offset
            .checked_div(self.frame_size.max(1) as u32)
            .ok_or(Error::InvalidResource)? as usize;
        if index >= per {
            return Err(Error::InvalidResource);
        }
        base.checked_add(index).ok_or(Error::InvalidResource)
    }
}

pub fn skip_options(
    code: &[u8],
    offset: usize,
    mask: Option<&mut u32>,
    bits: i32,
    flag: bool,
    tables: &PcreTables<'_>,
) -> Result<usize, Error> {
    let mut mask = mask;
    let mut p = offset;
    loop {
        let c = byte_at(code, p)?;
        match c {
            18 => {
                if bits > 0 {
                    let value = byte_at(code, p.wrapping_add(1))? as i32;
                    let current = mask.as_deref().copied().unwrap_or(0);
                    if (value & bits) != ((current as i32) & bits) {
                        if let Some(slot) = mask.as_deref_mut() {
                            *slot = value as u32;
                        }
                    }
                }
                p += 2;
            }
            70 | 71 | 72 => {
                if !flag {
                    return Ok(p);
                }
                loop {
                    let link = (u16::from(byte_at(code, p.wrapping_add(1))?) << 8)
                        | u16::from(byte_at(code, p.wrapping_add(2))?);
                    p = p.wrapping_add(link as usize);
                    if byte_at(code, p)? != 65 {
                        break;
                    }
                }
                let next = byte_at(code, p)?;
                p = p.wrapping_add(tables.op_lengths[next as usize] as usize);
            }
            3 | 4 => {
                if !flag {
                    return Ok(p);
                }
                p = p.wrapping_add(tables.op_lengths[c as usize] as usize);
            }
            64 | 76 | 79 => {
                p = p.wrapping_add(tables.op_lengths[c as usize] as usize);
            }
            _ => return Ok(p),
        }
    }
}

pub fn error_status(code: i32) -> u32 {
    match code {
        -2 => 0x8a50210d,
        -5 | -4 => 0x8a503e66,
        -6 => 0x8a50200a,
        -7 | -1 => 0x8a502014,
        -8 => 0x8a503e60,
        -9 => 0x8a503e61,
        -10 => 0x8a502105,
        -11 => 0x8a503e65,
        -12 => 0x8a503e62,
        -13 => 0x8a503e63,
        -14 => 0x8a503e64,
        -15 | -3 => 0x8a502007,
        -16 => 0x8a502008,
        _ => 0x8a503e67,
    }
}

pub fn byteswap_pattern(
    code: &[u8],
    output: &mut [u8],
    study: Option<&[u8]>,
    study_output: Option<&mut [u8]>,
) -> Result<bool, Error> {
    if code.len() < 40 || output.len() < 40 {
        return Err(Error::Capacity {
            required: 40,
            available: code.len().min(output.len()),
        });
    }
    let magic = u32::from_le_bytes(code[0..4].try_into().map_err(|_| Error::InvalidResource)?);
    if magic.swap_bytes() != 0x50435245 {
        return Ok(false);
    }
    output[..40].copy_from_slice(&code[..40]);
    for at in [4, 8] {
        let word = u32::from_le_bytes(
            code[at..at + 4]
                .try_into()
                .map_err(|_| Error::InvalidResource)?,
        );
        output[at..at + 4].copy_from_slice(&word.swap_bytes().to_le_bytes());
    }
    for at in [16, 18, 20, 22, 24, 26, 28] {
        let half = u16::from_le_bytes(
            code[at..at + 2]
                .try_into()
                .map_err(|_| Error::InvalidResource)?,
        );
        output[at..at + 2].copy_from_slice(&half.swap_bytes().to_le_bytes());
    }
    if let (Some(study), Some(study_output)) = (study, study_output) {
        if study.len() < 40 || study_output.len() < 40 {
            return Err(Error::Capacity {
                required: 40,
                available: study.len().min(study_output.len()),
            });
        }
        study_output[..40].copy_from_slice(&study[..40]);
        for at in [0, 4] {
            let word = u32::from_le_bytes(
                study[at..at + 4]
                    .try_into()
                    .map_err(|_| Error::InvalidResource)?,
            );
            study_output[at..at + 4].copy_from_slice(&word.swap_bytes().to_le_bytes());
        }
    }
    Ok(true)
}

fn link_at(code: &[u8], offset: usize) -> Result<u16, Error> {
    let hi = byte_at(code, offset.wrapping_add(1))? as u16;
    let lo = byte_at(code, offset.wrapping_add(2))? as u16;
    Ok((hi << 8) | lo)
}

pub fn is_anchored(
    code: &[u8],
    offset: usize,
    state: &mut i32,
    x: i32,
    y: i32,
    tables: &PcreTables<'_>,
) -> Result<bool, Error> {
    let both = x & y;
    let mut p = offset;
    loop {
        let mask = 2;
        let mut cur = p.wrapping_add(3);
        loop {
            let c = byte_at(code, cur)?;
            match c {
                18 => {
                    let value = byte_at(code, cur.wrapping_add(1))? as i32;
                    if (value & mask) != (*state & mask) {
                        *state = value;
                    }
                    cur = cur.wrapping_add(2);
                }
                64 | 76 | 79 => {
                    cur = cur.wrapping_add(tables.op_lengths[c as usize] as usize);
                }
                _ => {
                    let v = c as i32;
                    if v > 80 {
                        let mut d = v - 80;
                        if d > 100 {
                            d = link_at(code, cur.wrapping_add(3))? as i32;
                        }
                        if d < 32 {
                            d = 1 << d;
                        } else {
                            d = 1;
                        }
                        if !is_anchored(code, cur, state, d | x, y, tables)? {
                            return Ok(false);
                        }
                        break;
                    }
                    if v == 80 || v == 69 || v == 74 || v == 75 {
                        if !is_anchored(code, cur, state, x, y, tables)? {
                            return Ok(false);
                        }
                        break;
                    }
                    if v == 42 || v == 43 {
                        if (*state & 4) != 0 {
                            if byte_at(code, cur.wrapping_add(1))? != 11 || both != 0 {
                                return Ok(false);
                            }
                            break;
                        }
                    }
                    if v == 1 || v == 2 {
                        break;
                    }
                    if (*state & 2) == 0 && v == 19 {
                        break;
                    }
                    return Ok(false);
                }
            }
        }
        let off = link_at(code, p)? as usize;
        p = p.wrapping_add(off);
        if byte_at(code, p)? != 65 {
            return Ok(true);
        }
    }
}

pub fn first_char_is_fixed(
    code: &[u8],
    offset: usize,
    mask: u32,
    bits: i32,
    tables: &PcreTables<'_>,
) -> Result<bool, Error> {
    let t = mask & bits as u32;
    let mut p = offset;
    loop {
        let mut q = p.wrapping_add(3);
        loop {
            let c = byte_at(code, q)?;
            if c == 70 {
                break;
            }
            if c < 70 {
                if c == 3 || c == 4 {
                    break;
                }
                if c == 18 {
                    q = q.wrapping_add(2);
                    continue;
                }
                if c == 64 {
                    q = q.wrapping_add(tables.op_lengths[c as usize] as usize);
                    continue;
                }
                break;
            }
            if c == 71 || c == 72 {
                break;
            }
            if c == 76 || c == 79 {
                q = q.wrapping_add(tables.op_lengths[c as usize] as usize);
                continue;
            }
            break;
        }
        let c = byte_at(code, q)? as i32;
        if c > 80 {
            let mut d = c - 80;
            if d > 100 {
                d = link_at(code, q.wrapping_add(3))? as i32;
            }
            if d < 32 {
                d = 1 << d;
            } else {
                d = 1;
            }
            if !first_char_is_fixed(code, q, mask | d as u32, bits, tables)? {
                return Ok(false);
            }
        } else if c == 80 || c == 69 || c == 74 || c == 75 {
            if !first_char_is_fixed(code, q, mask, bits, tables)? {
                return Ok(false);
            }
        } else if c == 42 || c == 43 {
            if byte_at(code, q.wrapping_add(1))? != 11 || t != 0 {
                return Ok(false);
            }
        } else if c != 19 {
            return Ok(false);
        }
        let off = link_at(code, p)? as usize;
        p = p.wrapping_add(off);
        if byte_at(code, p)? != 65 {
            return Ok(true);
        }
    }
}

pub fn find_required_char(
    code: &[u8],
    offset: usize,
    mask: &mut u32,
    mode: i32,
    tables: &PcreTables<'_>,
) -> Result<i32, Error> {
    let mut acc = -1;
    let mut p = offset;
    'outer: loop {

        let bits = 1u32;
        let mut q = p.wrapping_add(3);
        loop {
            let c = byte_at(code, q)?;
            if c == 70 {

                loop {
                    let link = link_at(code, q)? as usize;
                    q = q.wrapping_add(link);
                    if byte_at(code, q)? != 65 {
                        break;
                    }
                }
                let next = byte_at(code, q)?;
                q = q.wrapping_add(tables.op_lengths[next as usize] as usize);
                continue;
            }
            if c < 70 {
                if c == 3 || c == 4 || c == 64 {
                    q = q.wrapping_add(tables.op_lengths[c as usize] as usize);
                    continue;
                }
                if c == 18 {
                    let v = byte_at(code, q.wrapping_add(1))? as u32;
                    q = q.wrapping_add(2);
                    if (v & bits) != (*mask & bits) {
                        *mask = v;
                    }
                    continue;
                }
            } else {
                if c == 71 || c == 72 {

                    loop {
                        let link = link_at(code, q)? as usize;
                        q = q.wrapping_add(link);
                        if byte_at(code, q)? != 65 {
                            break;
                        }
                    }
                    let next = byte_at(code, q)?;
                    q = q.wrapping_add(tables.op_lengths[next as usize] as usize);
                    continue;
                }
                if c == 76 || c == 79 {
                    q = q.wrapping_add(tables.op_lengths[c as usize] as usize);
                    continue;
                }
            }
            let mut reduced = c as i32;
            if reduced > 80 {
                reduced = 80;
            }
            if reduced == 32 {
                q = q.wrapping_add(2);
            } else if reduced < 32 {
                if reduced != 21 && reduced != 22 && reduced != 26 && reduced != 27 {
                    return Ok(-1);
                }
            } else if reduced != 69 && reduced != 74 && reduced != 75 && reduced != 80 {
                return Ok(-1);
            } else {
                let r = find_required_char(code, q, mask, (reduced == 69) as i32, tables)?;
                if r < 0 {
                    return Ok(-1);
                }
                if acc < 0 {
                    acc = r;
                } else if acc != r {
                    return Ok(-1);
                }
                let link = link_at(code, p)? as usize;
                p = p.wrapping_add(link);
                if byte_at(code, p)? != 65 {
                    return Ok(acc);
                }
                continue 'outer;
            }

            if mode == 0 {
                return Ok(-1);
            }
            if acc < 0 {
                acc = byte_at(code, q.wrapping_add(1))? as i32;
                if (*mask & 1) != 0 {
                    acc |= 0x100;
                }
            } else if byte_at(code, q.wrapping_add(1))? as i32 != acc {
                return Ok(-1);
            }
            let link = link_at(code, p)? as usize;
            p = p.wrapping_add(link);
            if byte_at(code, p)? != 65 {
                return Ok(acc);
            }
            continue 'outer;
        }
    }
}

pub fn fixed_branch_length(
    code: &[u8],
    offset: usize,
    options: u32,
    tables: &PcreTables<'_>,
) -> Result<i32, Error> {
    let mut length = -1;
    let mut branchlength = 0i32;
    let mut cc = offset.wrapping_add(3);
    loop {
        let raw = byte_at(code, cc)?;
        let op = if raw >= 80 { 80 } else { raw };
        match op {
            80 | 74 | 75 => {
                let d = fixed_branch_length(code, cc, options, tables)?;
                if d < 0 {
                    return Ok(d);
                }
                branchlength = branchlength.wrapping_add(d);
                loop {
                    let link = link_at(code, cc)? as usize;
                    cc = cc.wrapping_add(link);
                    if byte_at(code, cc)? != 65 {
                        break;
                    }
                }
                cc = cc.wrapping_add(3);
            }
            65 | 66 | 67 | 68 | 0 => {
                if length < 0 {
                    length = branchlength;
                } else if length != branchlength {
                    return Ok(-1);
                }
                if byte_at(code, cc)? != 65 {
                    return Ok(length);
                }
                cc = cc.wrapping_add(3);
                branchlength = 0;
            }
            69 | 70 | 71 | 72 => {
                loop {
                    let link = link_at(code, cc)? as usize;
                    cc = cc.wrapping_add(link);
                    if byte_at(code, cc)? != 65 {
                        break;
                    }
                }
                let next = byte_at(code, cc)?;
                cc = cc.wrapping_add(tables.op_lengths[next as usize] as usize);
            }
            73 | 79 | 76 | 18 | 64 | 1 | 2 | 17 | 16 | 19 | 20 | 3 | 4 => {
                cc = cc.wrapping_add(tables.op_lengths[raw as usize] as usize);
            }
            21 | 22 => {
                branchlength = branchlength.wrapping_add(1);
                cc = cc.wrapping_add(2);
                if (options & 0x800) != 0 {
                    while byte_at(code, cc)? & 0xc0 == 0x80 {
                        cc = cc.wrapping_add(1);
                    }
                }
            }
            32 => {
                branchlength = branchlength.wrapping_add(link_at(code, cc)? as i32);
                cc = cc.wrapping_add(4);
                if (options & 0x800) != 0 {
                    while byte_at(code, cc)? & 0x80 == 0x80 {
                        cc = cc.wrapping_add(1);
                    }
                }
            }
            50 => {
                branchlength = branchlength.wrapping_add(link_at(code, cc)? as i32);
                cc = cc.wrapping_add(4);
            }
            14 | 13 => {
                cc = cc.wrapping_add(1);
                branchlength = branchlength.wrapping_add(1);
                cc = cc.wrapping_add(1);
            }
            5 | 6 | 7 | 8 | 9 | 10 | 11 => {
                branchlength = branchlength.wrapping_add(1);
                cc = cc.wrapping_add(1);
            }
            12 => return Ok(-2),
            61 => {

                let link = link_at(code, cc)? as usize;
                cc = cc.wrapping_add(link).wrapping_sub(33);
                cc = cc.wrapping_add(33);
                match class_tail(code, cc)? {
                    ClassTail::Length(add, next) => {
                        branchlength = branchlength.wrapping_add(add);
                        cc = next;
                    }
                    ClassTail::Fail => return Ok(-1),
                }
            }
            59 | 60 => {
                cc = cc.wrapping_add(33);
                match class_tail(code, cc)? {
                    ClassTail::Length(add, next) => {
                        branchlength = branchlength.wrapping_add(add);
                        cc = next;
                    }
                    ClassTail::Fail => return Ok(-1),
                }
            }
            _ => return Ok(-1),
        }
    }
}

enum ClassTail {
    Length(i32, usize),
    Fail,
}

fn class_tail(code: &[u8], cc: usize) -> Result<ClassTail, Error> {
    match byte_at(code, cc)? {
        51 | 52 | 55 | 56 => Ok(ClassTail::Fail),
        57 | 58 => {
            if link_at(code, cc)? as i32 != link_at(code, cc.wrapping_add(2))? as i32 {
                return Ok(ClassTail::Fail);
            }
            let add = link_at(code, cc)? as i32;
            Ok(ClassTail::Length(add, cc.wrapping_add(5)))
        }
        _ => Ok(ClassTail::Length(1, cc)),
    }
}

pub fn branch_can_be_empty(
    code: &[u8],
    offset: usize,
    end: usize,
    utf8: bool,
    tables: &PcreTables<'_>,
) -> Result<bool, Error> {
    let mut pos = skip_options(code, offset.wrapping_add(3), None, 0, true, tables)?;
    while pos < end {
        let mut c = byte_at(code, pos)? as i32;
        if c >= 80 {
            if link_at(code, pos)? == 0 {
                return Ok(true);
            }
            let mut empty_branch = false;
            loop {
                if !empty_branch && branch_can_be_empty(code, pos, end, utf8, tables)? {
                    empty_branch = true;
                }
                let link = link_at(code, pos)? as usize;
                pos = pos.wrapping_add(link);
                if byte_at(code, pos)? != 65 {
                    break;
                }
            }
            if !empty_branch {
                return Ok(false);
            }
            pos = pos.wrapping_add(3);
            c = byte_at(code, pos)? as i32;
        } else {
            match c {
                61 => {
                    let link = link_at(code, pos)? as usize;
                    let ccode = pos.wrapping_add(link);
                    if !class_can_be_empty(code, ccode)? {
                        return Ok(false);
                    }
                }
                59 | 60 => {
                    let ccode = pos.wrapping_add(33);
                    if !class_can_be_empty(code, ccode)? {
                        return Ok(false);
                    }
                }
                14 | 13 | 15 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 21 | 22 | 23 | 26 | 27 | 32
                | 35 | 36 | 41 | 44 | 45 | 50 => return Ok(false),
                66 | 67 | 68 | 65 => return Ok(true),
                24 | 25 | 28 | 29 | 30 | 31 => {
                    if utf8 {
                        while byte_at(code, pos.wrapping_add(2))? & 0xc0 == 0x80 {
                            pos = pos.wrapping_add(1);
                        }
                    }
                }
                _ => {}
            }
        }
        pos = pos.wrapping_add(tables.op_lengths[c as usize] as usize);
        loop {
            match byte_at(code, pos)? {
                18 => {
                    pos = pos.wrapping_add(2);
                }
                70 | 71 | 72 => {
                    loop {
                        let link = link_at(code, pos)? as usize;
                        pos = pos.wrapping_add(link);
                        if byte_at(code, pos)? != 65 {
                            break;
                        }
                    }
                    let next = byte_at(code, pos)?;
                    pos = pos.wrapping_add(tables.op_lengths[next as usize] as usize);
                }
                4 | 3 | 64 | 76 | 79 => {
                    let next = byte_at(code, pos)?;
                    pos = pos.wrapping_add(tables.op_lengths[next as usize] as usize);
                }
                _ => break,
            }
        }
    }
    Ok(true)
}

fn class_can_be_empty(code: &[u8], ccode: usize) -> Result<bool, Error> {
    match byte_at(code, ccode)? {
        51 | 52 | 55 | 56 => Ok(true),
        57 | 58 => Ok(link_at(code, ccode)? == 0),
        _ => Ok(false),
    }
}

pub fn validate_utf8(text: &[u8], len: i32, tables: &PcreTables<'_>) -> i32 {
    let mut len = len;
    if len < 0 {
        len = text
            .iter()
            .position(|&b| b == 0)
            .map_or(text.len() as i32, |p| p as i32);
    }

    if len as usize > text.len() {
        len = text.len() as i32;
    }
    let mut cur = 0usize;
    while len > 0 {
        len -= 1;
        let b = text[cur];
        if b >= 0x80 {
            if (!b & 0xc0) != 0 {
                return cur as i32;
            }
            let n = tables.utf8_cont[(b & 0x3f) as usize] as i32;
            if n > len {
                return cur as i32;
            }
            cur += 1;
            len -= n;
            let b2 = text[cur];
            if (b2 & 0xc0) != 0x80 {
                return cur as i32;
            }
            let bad = match n {
                1 => (b & 0x3e) == 0,
                2 => b == 0xe0 && (b2 & 0x20) == 0,
                3 => b == 0xf0 && (b2 & 0x30) == 0,
                4 => b == 0xf8 && (b2 & 0x38) == 0,
                5 => b == 0xfe || b == 0xff || (b == 0xfc && (b2 & 0x3c) == 0),
                _ => false,
            };
            if bad {
                return cur as i32;
            }
            for _ in 1..n {
                cur += 1;
                if (text[cur] & 0xc0) != 0x80 {
                    return cur as i32;
                }
            }
        }
        cur += 1;
    }
    -1
}

pub mod op {
    pub const END: u8 = 0;
    pub const SOD: u8 = 1;
    pub const SOM: u8 = 2;
    pub const NOT_WORD_BOUNDARY: u8 = 3;
    pub const WORD_BOUNDARY: u8 = 4;
    pub const NOT_DIGIT: u8 = 5;
    pub const DIGIT: u8 = 6;
    pub const NOT_WHITESPACE: u8 = 7;
    pub const WHITESPACE: u8 = 8;
    pub const NOT_WORDCHAR: u8 = 9;
    pub const WORDCHAR: u8 = 10;
    pub const ANY: u8 = 11;
    pub const ANYBYTE: u8 = 12;
    pub const NOTPROP: u8 = 13;
    pub const PROP: u8 = 14;
    pub const EXTUNI: u8 = 15;
    pub const EODN: u8 = 16;
    pub const EOD: u8 = 17;
    pub const OPT: u8 = 18;
    pub const CIRC: u8 = 19;
    pub const DOLL: u8 = 20;
    pub const CHAR: u8 = 21;
    pub const CHARNC: u8 = 22;
    pub const NOT: u8 = 23;
    pub const STAR: u8 = 24;
    pub const MINSTAR: u8 = 25;
    pub const PLUS: u8 = 26;
    pub const MINPLUS: u8 = 27;
    pub const QUERY: u8 = 28;
    pub const MINQUERY: u8 = 29;
    pub const UPTO: u8 = 30;
    pub const MINUPTO: u8 = 31;
    pub const EXACT: u8 = 32;
    pub const NOTSTAR: u8 = 33;
    pub const NOTMINSTAR: u8 = 34;
    pub const NOTPLUS: u8 = 35;
    pub const NOTMINPLUS: u8 = 36;
    pub const NOTQUERY: u8 = 37;
    pub const NOTMINQUERY: u8 = 38;
    pub const NOTUPTO: u8 = 39;
    pub const NOTMINUPTO: u8 = 40;
    pub const NOTEXACT: u8 = 41;
    pub const TYPESTAR: u8 = 42;
    pub const TYPEMINSTAR: u8 = 43;
    pub const TYPEPLUS: u8 = 44;
    pub const TYPEMINPLUS: u8 = 45;
    pub const TYPEQUERY: u8 = 46;
    pub const TYPEMINQUERY: u8 = 47;
    pub const TYPEUPTO: u8 = 48;
    pub const TYPEMINUPTO: u8 = 49;
    pub const TYPEEXACT: u8 = 50;
    pub const CRSTAR: u8 = 51;
    pub const CRMINSTAR: u8 = 52;
    pub const CRPLUS: u8 = 53;
    pub const CRMINPLUS: u8 = 54;
    pub const CRQUERY: u8 = 55;
    pub const CRMINQUERY: u8 = 56;
    pub const CRRANGE: u8 = 57;
    pub const CRMINRANGE: u8 = 58;
    pub const CLASS: u8 = 59;
    pub const NCLASS: u8 = 60;
    pub const XCLASS: u8 = 61;
    pub const REF: u8 = 62;
    pub const RECURSE: u8 = 63;
    pub const CALLOUT: u8 = 64;
    pub const ALT: u8 = 65;
    pub const KET: u8 = 66;
    pub const KETRMAX: u8 = 67;
    pub const KETRMIN: u8 = 68;
    pub const ASSERT: u8 = 69;
    pub const ASSERT_NOT: u8 = 70;
    pub const ASSERTBACK: u8 = 71;
    pub const ASSERTBACK_NOT: u8 = 72;
    pub const REVERSE: u8 = 73;
    pub const ONCE: u8 = 74;
    pub const COND: u8 = 75;
    pub const CREF: u8 = 76;
    pub const BRAZERO: u8 = 77;
    pub const BRAMINZERO: u8 = 78;
    pub const BRANUMBER: u8 = 79;
    pub const BRA: u8 = 80;
}

pub mod pcre {
    pub const CASELESS: u32 = 0x0001;
    pub const MULTILINE: u32 = 0x0002;
    pub const DOTALL: u32 = 0x0004;
    pub const EXTENDED: u32 = 0x0008;
    pub const ANCHORED: u32 = 0x0010;
    pub const DOLLAR_ENDONLY: u32 = 0x0020;
    pub const EXTRA: u32 = 0x0040;
    pub const NOTBOL: u32 = 0x0080;
    pub const NOTEOL: u32 = 0x0100;
    pub const UNGREEDY: u32 = 0x0200;
    pub const NOTEMPTY: u32 = 0x0400;
    pub const UTF8: u32 = 0x0800;
    pub const NO_AUTO_CAPTURE: u32 = 0x1000;
    pub const NO_UTF8_CHECK: u32 = 0x2000;
    pub const AUTO_CALLOUT: u32 = 0x4000;
    pub const IMS: u32 = CASELESS | MULTILINE | DOTALL;
    pub const FIRSTSET: u32 = 0x4000_0000;
    pub const REQCHSET: u32 = 0x2000_0000;
    pub const STARTLINE: u32 = 0x1000_0000;
    pub const ICHANGED: u32 = 0x0800_0000;
    pub const NOPARTIAL: u32 = 0x0400_0000;
    pub const REQ_CASELESS: i32 = 0x0100;
    pub const REQ_VARY: i32 = 0x0200;
    pub const REQ_UNSET: i32 = -2;
    pub const REQ_NONE: i32 = -1;
}

pub mod esc {
    pub const A: i32 = 1;
    pub const G: i32 = 2;
    pub const B: i32 = 3;
    pub const B_LOWER: i32 = 4;
    pub const D_UPPER: i32 = 5;
    pub const D: i32 = 6;
    pub const S_UPPER: i32 = 7;
    pub const S: i32 = 8;
    pub const W_UPPER: i32 = 9;
    pub const W: i32 = 10;
    pub const C: i32 = 12;
    pub const P_UPPER: i32 = 13;
    pub const P: i32 = 14;
    pub const X: i32 = 15;
    pub const Z_UPPER: i32 = 16;
    pub const Z: i32 = 17;
    pub const E: i32 = 18;
    pub const Q: i32 = 19;
    pub const REF: i32 = 20;
}

const LINK_SIZE: usize = 2;
const EXTRACT_BASIC_MAX: i32 = 100;
const CREF_RECURSE: i32 = 0xffff;
const XCL_NOT: u8 = 1;
const XCL_MAP: u8 = 2;
const XCL_END: u8 = 0;
const XCL_SINGLE: u8 = 1;
const XCL_RANGE: u8 = 2;

fn put(code: &mut [u8], at: usize, value: usize) -> Result<(), Error> {
    let bytes = code.get_mut(at..at + 2).ok_or(Error::InvalidResource)?;
    bytes[0] = (value >> 8) as u8;
    bytes[1] = value as u8;
    Ok(())
}

fn emit(code: &mut [u8], pos: &mut usize, byte: u8) -> Result<(), Error> {
    let available = code.len();
    *code.get_mut(*pos).ok_or(Error::Capacity {
        required: *pos + 1,
        available,
    })? = byte;
    *pos += 1;
    Ok(())
}

fn shift(code: &mut [u8], src: usize, len: usize, dst: usize) -> Result<(), Error> {
    let (Some(end), Some(dend)) = (src.checked_add(len), dst.checked_add(len)) else {
        return Err(Error::InvalidResource);
    };
    if end > code.len() || dend > code.len() {
        return Err(Error::Capacity {
            required: dend.max(end),
            available: code.len(),
        });
    }
    code.copy_within(src..end, dst);
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompileFailure {
    pub error: CompileError,
    pub offset: usize,
    pub brackets: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompileFail {
    Invalid(CompileFailure),
    Resource(Error),
}

impl From<Error> for CompileFail {
    fn from(e: Error) -> Self {
        CompileFail::Resource(e)
    }
}

fn fail(error: CompileError, offset: usize) -> CompileFail {
    CompileFail::Invalid(CompileFailure {
        error,
        offset,
        brackets: 0,
    })
}

fn fail_at(error: CompileError, offset: usize, brackets: i32) -> CompileFail {
    CompileFail::Invalid(CompileFailure {
        error,
        offset,
        brackets,
    })
}

fn fail_st(error: CompileError, st: &BranchState) -> CompileFail {
    fail_at(error, st.ptr, st.brackets)
}

pub struct BranchChain<'a> {
    pub outer: Option<&'a BranchChain<'a>>,
    pub current: usize,
}

pub struct CompileData<'a, 't> {
    pub tables: &'t PcreTables<'t>,
    pub lcc: &'a [u8],
    pub fcc: &'a [u8],
    pub cbits: &'a [u8],
    pub ctypes: &'a [u8],
    pub name_table: &'a mut [u8],
    pub names_found: i32,
    pub name_entry_size: usize,
    pub top_backref: i32,
    pub backref_map: u32,
    pub req_varyopt: i32,
    pub nopartial: bool,
    pub firstbyte: i32,
    pub bracount: i32,
}

impl<'a, 't> CompileData<'a, 't> {

    pub fn new(
        tables: &'t PcreTables<'t>,
        block: &'a [u8; 1088],
        name_table: &'a mut [u8],
        name_entry_size: usize,
    ) -> Self {
        CompileData {
            tables,
            lcc: &block[0..256],
            fcc: &block[256..512],
            cbits: &block[512..832],
            ctypes: &block[832..1088],
            name_table,
            names_found: 0,
            name_entry_size,
            top_backref: 0,
            backref_map: 0,
            req_varyopt: 0,
            nopartial: false,
            firstbyte: pcre::REQ_UNSET,
            bracount: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompileOut {
    pub code: usize,
    pub ptr: usize,
    pub firstbyte: i32,
    pub reqbyte: i32,
    pub options: u32,
    pub brackets: i32,
}

fn find_bracket(
    code: &[u8],
    mut pos: usize,
    number: i32,
    utf8: bool,
    tables: &PcreTables<'_>,
) -> Result<Option<usize>, Error> {
    loop {
        let c = byte_at(code, pos)?;
        if c == op::END {
            return Ok(None);
        } else if c > op::BRA {
            let mut n = c as i32 - op::BRA as i32;
            if n > EXTRACT_BASIC_MAX {
                n = link_at(code, pos.wrapping_add(2).wrapping_add(LINK_SIZE))? as i32;
            }
            if n == number {
                return Ok(Some(pos));
            }
            pos = pos.wrapping_add(tables.op_lengths[op::BRA as usize] as usize);
        } else {
            pos = pos.wrapping_add(tables.op_lengths[c as usize] as usize);
            if utf8 {
                match c {
                    op::CHAR
                    | op::CHARNC
                    | op::EXACT
                    | op::UPTO
                    | op::MINUPTO
                    | op::STAR
                    | op::MINSTAR
                    | op::PLUS
                    | op::MINPLUS
                    | op::QUERY
                    | op::MINQUERY => {
                        while byte_at(code, pos)? & 0xc0 == 0x80 {
                            pos = pos.wrapping_add(1);
                        }
                    }
                    op::XCLASS => {

                        pos = pos
                            .wrapping_add(link_at(code, pos)? as usize)
                            .wrapping_add(1);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn find_recurse(
    code: &[u8],
    mut pos: usize,
    utf8: bool,
    tables: &PcreTables<'_>,
) -> Result<Option<usize>, Error> {
    loop {
        let c = byte_at(code, pos)?;
        if c == op::END {
            return Ok(None);
        } else if c == op::RECURSE {
            return Ok(Some(pos));
        } else if c > op::BRA {
            pos = pos.wrapping_add(tables.op_lengths[op::BRA as usize] as usize);
        } else {
            pos = pos.wrapping_add(tables.op_lengths[c as usize] as usize);
            if utf8 {
                match c {
                    op::CHAR
                    | op::CHARNC
                    | op::EXACT
                    | op::UPTO
                    | op::MINUPTO
                    | op::STAR
                    | op::MINSTAR
                    | op::PLUS
                    | op::MINPLUS
                    | op::QUERY
                    | op::MINQUERY => {
                        while byte_at(code, pos)? & 0xc0 == 0x80 {
                            pos = pos.wrapping_add(1);
                        }
                    }
                    op::XCLASS => {
                        pos = pos
                            .wrapping_add(link_at(code, pos)? as usize)
                            .wrapping_add(1);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn could_be_empty(
    code: &[u8],
    start: usize,
    end: usize,
    bc: Option<&BranchChain<'_>>,
    utf8: bool,
    tables: &PcreTables<'_>,
) -> Result<bool, Error> {
    let mut bc = bc;
    while let Some(node) = bc {
        if node.current < start {
            break;
        }
        if !branch_can_be_empty(code, node.current, end, utf8, tables)? {
            return Ok(false);
        }
        bc = node.outer;
    }
    Ok(true)
}

fn check_posix_syntax(pattern: &[u8], pos: usize, ctypes: &[u8]) -> Result<Option<usize>, Error> {
    let mut p = pos.wrapping_add(1);
    let terminator = byte_at(pattern, p)?;
    p = p.wrapping_add(1);
    if byte_at(pattern, p)? == b'^' {
        p = p.wrapping_add(1);
    }
    while ctypes[byte_at(pattern, p)? as usize] & 0x02 != 0 {
        p = p.wrapping_add(1);
    }
    if byte_at(pattern, p)? == terminator && byte_at(pattern, p.wrapping_add(1))? == b']' {
        Ok(Some(p))
    } else {
        Ok(None)
    }
}

fn check_posix_name(name: &[u8], tables: &PcreTables<'_>) -> i32 {
    let mut yield_ = 0;
    while tables.posix_lengths.get(yield_).copied().unwrap_or(0) != 0 {
        if name.len() == tables.posix_lengths[yield_] as usize
            && tables.posix_names.get(yield_).copied().unwrap_or(&[]) == name
        {
            return yield_ as i32;
        }
        yield_ += 1;
    }
    -1
}

fn adjust_recurse(
    code: &mut [u8],
    group: usize,
    adjust: usize,
    utf8: bool,
    tables: &PcreTables<'_>,
) -> Result<(), Error> {
    let mut ptr = group;
    while let Some(found) = find_recurse(code, ptr, utf8, tables)? {
        let offset = link_at(code, found)? as usize;
        if offset >= group {
            put(code, found, offset.wrapping_add(adjust))?;
        }
        ptr = found.wrapping_add(1).wrapping_add(LINK_SIZE);
    }
    Ok(())
}

fn auto_callout(code: &mut [u8], pos: usize, ptr: usize) -> Result<usize, Error> {
    let mut pos = pos;
    emit(code, &mut pos, op::CALLOUT)?;
    emit(code, &mut pos, 255)?;
    put(code, pos, ptr)?;
    pos += LINK_SIZE;
    put(code, pos, 0)?;
    pos += LINK_SIZE;
    Ok(pos)
}

fn complete_callout(code: &mut [u8], callout: usize, ptr: usize) -> Result<(), Error> {

    let start = link_at(code, callout.wrapping_add(1))? as usize;
    put(
        code,
        callout.wrapping_add(2).wrapping_add(LINK_SIZE),
        ptr.wrapping_sub(start),
    )
}

fn getchar(pattern: &[u8], pos: &mut usize, tables: &PcreTables<'_>) -> Result<i32, Error> {
    let mut c = byte_at(pattern, *pos)? as i32;
    if (c & 0xc0) == 0xc0 {
        let count = tables.utf8_cont[(c & 0x3f) as usize] as i32;
        let mut shift = 6 * count;
        c = (c & tables.utf8_mask[count as usize] as i32) << shift;
        for i in 1..=count {
            shift -= 6;
            c |= ((byte_at(pattern, pos.wrapping_add(i as usize))? & 0x3f) as i32) << shift;
        }
        *pos = pos.wrapping_add(count as usize);
    }
    Ok(c)
}

struct BranchState {
    code: usize,
    ptr: usize,
    options: u32,
    brackets: i32,
    firstbyte: i32,
    reqbyte: i32,
    zerofirstbyte: i32,
    zeroreqbyte: i32,
    previous: Option<usize>,
    previous_callout: Option<usize>,
    after_manual_callout: i32,
    inescq: bool,
    groupsetfirstbyte: bool,
    req_caseopt: i32,
}

pub fn compile_branch(
    code: &mut [u8],
    code_pos: usize,
    pattern: &[u8],
    ptr_pos: usize,
    options: u32,
    brackets: i32,
    bc: Option<&BranchChain<'_>>,
    cd: &mut CompileData<'_, '_>,
) -> Result<CompileOut, CompileFail> {
    let tables = cd.tables;
    let utf8 = options & pcre::UTF8 != 0;
    let mut greedy_default = (options & pcre::UNGREEDY != 0) as i32;
    let mut greedy_non_default = greedy_default ^ 1;
    let mut st = BranchState {
        code: code_pos,
        ptr: ptr_pos,
        options,
        brackets,
        firstbyte: pcre::REQ_UNSET,
        reqbyte: pcre::REQ_UNSET,
        zerofirstbyte: pcre::REQ_UNSET,
        zeroreqbyte: pcre::REQ_UNSET,
        previous: None,
        previous_callout: None,
        after_manual_callout: 0,
        inescq: false,
        groupsetfirstbyte: false,
        req_caseopt: if options & pcre::CASELESS != 0 {
            pcre::REQ_CASELESS
        } else {
            0
        },
    };
    let mut classbits = [0u8; 32];
    let mut utf8_char = [0u8; 6];
    let mut mcbuffer = [0u8; 8];
    loop {
        let mut c = byte_at(pattern, st.ptr)? as i32;
        if st.inescq && c != 0 {
            if c == b'\\' as i32 && byte_at(pattern, st.ptr.wrapping_add(1))? == b'E' {
                st.inescq = false;
                st.ptr = st.ptr.wrapping_add(2);
                continue;
            } else {
                if let Some(callout) = st.previous_callout {
                    complete_callout(code, callout, st.ptr)?;
                    st.previous_callout = None;
                }
                if st.options & pcre::AUTO_CALLOUT != 0 {
                    st.previous_callout = Some(st.code);
                    st.code = auto_callout(code, st.code, st.ptr)?;
                }
                one_char(
                    code,
                    &mut st,
                    pattern,
                    c,
                    &mut mcbuffer,
                    utf8,
                    cd.req_varyopt,
                )?;
                st.ptr = st.ptr.wrapping_add(1);
                continue;
            }
        }
        let is_quantifier = c == b'*' as i32
            || c == b'+' as i32
            || c == b'?' as i32
            || (c == b'{' as i32 && is_range_quantifier(pattern, st.ptr.wrapping_add(1), tables)?);
        if !is_quantifier {
            if let Some(callout) = st.previous_callout {
                st.after_manual_callout -= 1;
                if st.after_manual_callout < 0 {
                    complete_callout(code, callout, st.ptr)?;
                    st.previous_callout = None;
                }
            }
        }
        if st.options & pcre::EXTENDED != 0 {
            if cd.ctypes[c as usize] & 0x01 != 0 {
                st.ptr = st.ptr.wrapping_add(1);
                continue;
            }
            if c == b'#' as i32 {
                loop {
                    st.ptr = st.ptr.wrapping_add(1);
                    c = byte_at(pattern, st.ptr)? as i32;
                    if c == 0 || c == b'\n' as i32 {
                        break;
                    }
                }
                if c != 0 {
                    st.ptr = st.ptr.wrapping_add(1);
                    continue;
                }
            }
        }
        if st.options & pcre::AUTO_CALLOUT != 0 && !is_quantifier {
            st.previous_callout = Some(st.code);
            st.code = auto_callout(code, st.code, st.ptr)?;
        }
        match c as u8 {
            0 | b'|' | b')' => {
                return Ok(CompileOut {
                    code: st.code,
                    ptr: st.ptr,
                    firstbyte: st.firstbyte,
                    reqbyte: st.reqbyte,
                    options: st.options,
                    brackets: st.brackets,
                });
            }
            b'^' => {
                if st.options & pcre::MULTILINE != 0 && st.firstbyte == pcre::REQ_UNSET {
                    st.firstbyte = pcre::REQ_NONE;
                }
                st.previous = None;
                emit(code, &mut st.code, op::CIRC)?;
            }
            b'$' => {
                st.previous = None;
                emit(code, &mut st.code, op::DOLL)?;
            }
            b'.' => {
                if st.firstbyte == pcre::REQ_UNSET {
                    st.firstbyte = pcre::REQ_NONE;
                }
                st.zerofirstbyte = st.firstbyte;
                st.zeroreqbyte = st.reqbyte;
                st.previous = Some(st.code);
                emit(code, &mut st.code, op::ANY)?;
            }
            b'[' => {
                compile_class(
                    code,
                    &mut st,
                    pattern,
                    &mut classbits,
                    &mut mcbuffer,
                    utf8,
                    cd,
                )?;
            }
            b'{' | b'*' | b'+' | b'?' => {
                let (repeat_min, repeat_max) = if c == b'{' as i32 {
                    if !is_quantifier {
                        one_char(
                            code,
                            &mut st,
                            pattern,
                            c,
                            &mut mcbuffer,
                            utf8,
                            cd.req_varyopt,
                        )?;
                        st.ptr = st.ptr.wrapping_add(1);
                        continue;
                    }
                    match read_range(pattern, st.ptr.wrapping_add(1), tables)? {
                        Ok(read) => {
                            st.ptr = read.offset;
                            (read.min, read.max)
                        }
                        Err((off, error)) => return Err(fail_at(error, off, st.brackets)),
                    }
                } else if c == b'*' as i32 {
                    (0, -1)
                } else if c == b'+' as i32 {
                    (1, -1)
                } else {
                    (0, 1)
                };
                emit_repeat(
                    code,
                    &mut st,
                    pattern,
                    repeat_min,
                    repeat_max,
                    greedy_default,
                    greedy_non_default,
                    &mut utf8_char,
                    utf8,
                    cd,
                )?;
            }
            b'(' => {
                let action = compile_paren(
                    code,
                    &mut st,
                    pattern,
                    &mut greedy_default,
                    &mut greedy_non_default,
                    utf8,
                    bc,
                    cd,
                )?;
                match action {
                    ParenAction::Next => {
                        st.ptr = st.ptr.wrapping_add(1);
                        continue;
                    }
                    ParenAction::Break => {}
                }
            }
            b'\\' => {
                let read = read_escape(pattern, st.ptr, st.brackets, st.options, false, tables)?;
                st.ptr = read.offset;
                c = read.value;
                if c < 0 {
                    if -c == esc::Q {
                        if byte_at(pattern, st.ptr.wrapping_add(1))? == b'\\'
                            && byte_at(pattern, st.ptr.wrapping_add(2))? == b'E'
                        {
                            st.ptr = st.ptr.wrapping_add(2);
                        } else {
                            st.inescq = true;
                        }
                        st.ptr = st.ptr.wrapping_add(1);
                        continue;
                    }
                    if st.firstbyte == pcre::REQ_UNSET && -c > esc::B_LOWER && -c < esc::Z_UPPER {
                        st.firstbyte = pcre::REQ_NONE;
                    }
                    st.zerofirstbyte = st.firstbyte;
                    st.zeroreqbyte = st.reqbyte;
                    if -c >= esc::REF {
                        let number = -c - esc::REF;
                        st.previous = Some(st.code);
                        emit(code, &mut st.code, op::REF)?;
                        put(code, st.code, number as usize)?;
                        st.code += LINK_SIZE;
                    } else {
                        st.previous = if -c > esc::B_LOWER && -c < esc::Z_UPPER {
                            Some(st.code)
                        } else {
                            None
                        };
                        emit(code, &mut st.code, (-c) as u8)?;
                    }
                    st.ptr = st.ptr.wrapping_add(1);
                    continue;
                }
                let mclength = if utf8 && c > 127 {
                    encode_utf8(c, &mut mcbuffer, tables)?
                } else {
                    mcbuffer[0] = c as u8;
                    1
                };
                let options = st.options;
                one_char_fixed(
                    code,
                    &mut st,
                    &mcbuffer[..mclength],
                    cd.req_varyopt,
                    options,
                )?;
            }
            _ => {
                one_char(
                    code,
                    &mut st,
                    pattern,
                    c,
                    &mut mcbuffer,
                    utf8,
                    cd.req_varyopt,
                )?;
            }
        }
        st.ptr = st.ptr.wrapping_add(1);
    }
}

fn one_char(
    code: &mut [u8],
    st: &mut BranchState,
    pattern: &[u8],
    c: i32,
    mcbuffer: &mut [u8; 8],
    utf8: bool,
    req_varyopt: i32,
) -> Result<(), Error> {
    let mut mclength = 1;
    mcbuffer[0] = c as u8;
    if utf8 && (c & 0xc0) == 0xc0 {
        while byte_at(pattern, st.ptr.wrapping_add(1))? & 0xc0 == 0x80 {
            *mcbuffer.get_mut(mclength).ok_or(Error::InvalidResource)? =
                byte_at(pattern, st.ptr.wrapping_add(1))?;
            mclength += 1;
            st.ptr = st.ptr.wrapping_add(1);
        }
    }
    let options = st.options;
    one_char_fixed(code, st, &mcbuffer[..mclength], req_varyopt, options)?;
    Ok(())
}

fn one_char_fixed(
    code: &mut [u8],
    st: &mut BranchState,
    ch: &[u8],
    req_varyopt: i32,
    options: u32,
) -> Result<(), Error> {
    st.previous = Some(st.code);
    emit(
        code,
        &mut st.code,
        if options & pcre::CASELESS != 0 {
            op::CHARNC
        } else {
            op::CHAR
        },
    )?;
    for &b in ch {
        emit(code, &mut st.code, b)?;
    }
    if st.firstbyte == pcre::REQ_UNSET {
        st.zerofirstbyte = pcre::REQ_NONE;
        st.zeroreqbyte = st.reqbyte;
        if ch.len() == 1 || st.req_caseopt == 0 {
            st.firstbyte = ch[0] as i32 | st.req_caseopt;
            if ch.len() != 1 {
                st.reqbyte = code[st.code.wrapping_sub(1)] as i32 | req_varyopt;
            }
        } else {
            st.firstbyte = pcre::REQ_NONE;
            st.reqbyte = pcre::REQ_NONE;
        }
    } else {
        st.zerofirstbyte = st.firstbyte;
        st.zeroreqbyte = st.reqbyte;
        if ch.len() == 1 || st.req_caseopt == 0 {
            st.reqbyte = code[st.code.wrapping_sub(1)] as i32 | st.req_caseopt | req_varyopt;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn compile_class(
    code: &mut [u8],
    st: &mut BranchState,
    pattern: &[u8],
    classbits: &mut [u8; 32],
    mcbuffer: &mut [u8; 8],
    utf8: bool,
    cd: &mut CompileData<'_, '_>,
) -> Result<(), CompileFail> {
    let tables = cd.tables;
    st.previous = Some(st.code);
    if (byte_at(pattern, st.ptr.wrapping_add(1))? == b':'
        || byte_at(pattern, st.ptr.wrapping_add(1))? == b'.'
        || byte_at(pattern, st.ptr.wrapping_add(1))? == b'=')
        && check_posix_syntax(pattern, st.ptr, cd.ctypes)?.is_some()
    {
        return Err(fail_st(
            if byte_at(pattern, st.ptr.wrapping_add(1))? == b':' {
                CompileError::PosixOutsideClass
            } else {
                CompileError::NoCollatingElements
            },
            st,
        ));
    }
    st.ptr = st.ptr.wrapping_add(1);
    let mut c = byte_at(pattern, st.ptr)? as i32;
    let negate_class;
    if c == b'^' as i32 {
        negate_class = true;
        st.ptr = st.ptr.wrapping_add(1);
        c = byte_at(pattern, st.ptr)? as i32;
    } else {
        negate_class = false;
    }
    let mut class_charcount = 0;
    let mut class_lastchar = -1;
    let mut class_utf8 = false;
    let mut utf8data = st.code.wrapping_add(LINK_SIZE).wrapping_add(34);
    classbits.fill(0);
    loop {
        if utf8 && c > 127 {
            c = getchar(pattern, &mut st.ptr, tables)?;
        }
        if st.inescq {
            if c == b'\\' as i32 && byte_at(pattern, st.ptr.wrapping_add(1))? == b'E' {
                st.inescq = false;
                st.ptr = st.ptr.wrapping_add(1);
            } else {
                lone_class_char(
                    code,
                    st,
                    classbits,
                    &mut class_charcount,
                    &mut class_lastchar,
                    &mut class_utf8,
                    &mut utf8data,
                    c,
                    utf8,
                    cd,
                )?;
            }
        } else if c == b'[' as i32
            && (byte_at(pattern, st.ptr.wrapping_add(1))? == b':'
                || byte_at(pattern, st.ptr.wrapping_add(1))? == b'.'
                || byte_at(pattern, st.ptr.wrapping_add(1))? == b'=')
        {
            let Some(end) = check_posix_syntax(pattern, st.ptr, cd.ctypes)? else {
                class_item_tail(
                    code,
                    st,
                    pattern,
                    classbits,
                    &mut class_charcount,
                    &mut class_lastchar,
                    &mut class_utf8,
                    &mut utf8data,
                    c,
                    utf8,
                    cd,
                )?;
                st.ptr = st.ptr.wrapping_add(1);
                c = byte_at(pattern, st.ptr)? as i32;
                if c == b']' as i32 && !st.inescq {
                    break;
                }
                continue;
            };
            if byte_at(pattern, st.ptr.wrapping_add(1))? != b':' {
                return Err(fail_st(CompileError::NoCollatingElements, st));
            }
            st.ptr = st.ptr.wrapping_add(2);
            let mut local_negate = false;
            if byte_at(pattern, st.ptr)? == b'^' {
                local_negate = true;
                st.ptr = st.ptr.wrapping_add(1);
            }
            let name = pattern.get(st.ptr..end).ok_or(Error::InvalidResource)?;
            let mut posix_class = check_posix_name(name, tables);
            if posix_class < 0 {
                return Err(fail_st(CompileError::UnknownPosixClass, st));
            }
            if st.options & pcre::CASELESS != 0 && posix_class <= 2 {
                posix_class = 0;
            }
            posix_class *= 3;
            for i in 0..3 {

                let blankclass = pattern
                    .get(st.ptr..st.ptr.wrapping_add(5))
                    .is_some_and(|w| w == b"blank");
                let taboffset = tables.posix_maps[(posix_class + i) as usize];
                if taboffset < 0 {
                    break;
                }
                if local_negate {
                    if i == 0 {
                        for (slot, &bit) in classbits
                            .iter_mut()
                            .zip(cd.cbits[taboffset as usize..].iter())
                        {
                            *slot |= !bit;
                        }
                    } else {
                        for (slot, &bit) in classbits
                            .iter_mut()
                            .zip(cd.cbits[taboffset as usize..].iter())
                        {
                            *slot &= !bit;
                        }
                    }
                    if blankclass {
                        classbits[1] |= 0x3c;
                    }
                } else {
                    for (slot, &bit) in classbits
                        .iter_mut()
                        .zip(cd.cbits[taboffset as usize..].iter())
                    {
                        *slot |= bit;
                    }
                    if blankclass {
                        classbits[1] &= !0x3c;
                    }
                }
            }
            st.ptr = end.wrapping_add(1);
            class_charcount = 10;
        } else {
            if c == b'\\' as i32 {
                let read = read_escape(pattern, st.ptr, st.brackets, st.options, true, tables)?;
                st.ptr = read.offset;
                c = read.value;
                if -c == esc::B_LOWER {
                    c = 8;
                } else if -c == esc::X {
                    c = b'X' as i32;
                } else if -c == esc::Q {
                    if byte_at(pattern, st.ptr.wrapping_add(1))? == b'\\'
                        && byte_at(pattern, st.ptr.wrapping_add(2))? == b'E'
                    {
                        st.ptr = st.ptr.wrapping_add(2);
                    } else {
                        st.inescq = true;
                    }

                    st.ptr = st.ptr.wrapping_add(1);
                    c = byte_at(pattern, st.ptr)? as i32;
                    if c == b']' as i32 && !st.inescq {
                        break;
                    }
                    continue;
                }
                if c < 0 {
                    class_charcount += 2;
                    let kind = -c;
                    if kind == esc::D
                        || kind == esc::D_UPPER
                        || kind == esc::W
                        || kind == esc::W_UPPER
                        || kind == esc::S
                        || kind == esc::S_UPPER
                    {
                        let (offset, invert) = match kind {
                            x if x == esc::D => (64, false),
                            x if x == esc::D_UPPER => (64, true),
                            x if x == esc::W => (160, false),
                            x if x == esc::W_UPPER => (160, true),
                            x if x == esc::S => (0, false),
                            _ => (0, true),
                        };
                        for (slot, &bit) in classbits.iter_mut().zip(cd.cbits[offset..].iter()) {
                            *slot |= if invert { !bit } else { bit };
                        }
                        if kind == esc::S {
                            classbits[1] &= !0x08;
                        } else if kind == esc::S_UPPER {
                            classbits[1] |= 0x08;
                        }
                    } else if st.options & pcre::EXTRA != 0 {
                        return Err(fail_st(CompileError::BadClassEscape, st));
                    } else {
                        c = byte_at(pattern, st.ptr)? as i32;
                        class_charcount -= 2;
                    }
                }
                if c >= 0 {
                    class_item_tail(
                        code,
                        st,
                        pattern,
                        classbits,
                        &mut class_charcount,
                        &mut class_lastchar,
                        &mut class_utf8,
                        &mut utf8data,
                        c,
                        utf8,
                        cd,
                    )?;
                }
            } else {
                class_item_tail(
                    code,
                    st,
                    pattern,
                    classbits,
                    &mut class_charcount,
                    &mut class_lastchar,
                    &mut class_utf8,
                    &mut utf8data,
                    c,
                    utf8,
                    cd,
                )?;
            }
        }
        st.ptr = st.ptr.wrapping_add(1);
        c = byte_at(pattern, st.ptr)? as i32;
        if c == b']' as i32 && !st.inescq {
            break;
        }
    }
    if class_charcount == 1 && (!utf8 || (!class_utf8 && (!negate_class || class_lastchar < 128))) {
        st.zeroreqbyte = st.reqbyte;
        if negate_class {
            if st.firstbyte == pcre::REQ_UNSET {
                st.firstbyte = pcre::REQ_NONE;
            }
            st.zerofirstbyte = st.firstbyte;
            emit(code, &mut st.code, op::NOT)?;
            emit(code, &mut st.code, class_lastchar as u8)?;
            return Ok(());
        }
        let mclength = if utf8 && class_lastchar > 127 {
            encode_utf8(class_lastchar, mcbuffer, tables)?
        } else {
            mcbuffer[0] = class_lastchar as u8;
            1
        };
        let req_varyopt = cd.req_varyopt;
        let options = st.options;
        one_char_fixed(code, st, &mcbuffer[..mclength], req_varyopt, options)?;
        return Ok(());
    }
    if st.firstbyte == pcre::REQ_UNSET {
        st.firstbyte = pcre::REQ_NONE;
    }
    st.zerofirstbyte = st.firstbyte;
    st.zeroreqbyte = st.reqbyte;
    if class_utf8 {
        emit(code, &mut utf8data, XCL_END)?;
        let item = st.previous.unwrap_or(st.code);
        emit(code, &mut st.code, op::XCLASS)?;
        st.code += LINK_SIZE;
        let flags = if negate_class { XCL_NOT } else { 0 };
        *code.get_mut(st.code).ok_or(Error::InvalidResource)? = flags;
        if class_charcount > 0 {
            *code.get_mut(st.code).ok_or(Error::InvalidResource)? |= XCL_MAP;
            st.code += 1;
            let end = st.code.wrapping_add(32);
            code.get_mut(st.code..end)
                .ok_or(Error::InvalidResource)?
                .copy_from_slice(&classbits[..]);
            st.code = utf8data;
        } else {
            let len = utf8data.wrapping_sub(st.code.wrapping_add(33));
            shift(code, st.code + 33, len, st.code + 1)?;
            st.code += len + 1;
        }
        put(code, item.wrapping_add(1), st.code.wrapping_sub(item))?;
        return Ok(());
    }
    if negate_class {
        emit(code, &mut st.code, op::NCLASS)?;
        for i in 0..32 {
            let v = !classbits[i];
            emit(code, &mut st.code, v)?;
        }
    } else {
        emit(code, &mut st.code, op::CLASS)?;
        let end = st.code.wrapping_add(32);
        code.get_mut(st.code..end)
            .ok_or(Error::InvalidResource)?
            .copy_from_slice(&classbits[..]);
        st.code = end;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn class_item_tail(
    code: &mut [u8],
    st: &mut BranchState,
    pattern: &[u8],
    classbits: &mut [u8; 32],
    class_charcount: &mut i32,
    class_lastchar: &mut i32,
    class_utf8: &mut bool,
    utf8data: &mut usize,
    c: i32,
    utf8: bool,
    cd: &mut CompileData<'_, '_>,
) -> Result<(), CompileFail> {
    if byte_at(pattern, st.ptr.wrapping_add(1))? == b'-'
        && byte_at(pattern, st.ptr.wrapping_add(2))? != b']'
    {
        class_range(
            code,
            st,
            pattern,
            classbits,
            class_charcount,
            class_lastchar,
            class_utf8,
            utf8data,
            c,
            utf8,
            cd,
        )
    } else {
        lone_class_char(
            code,
            st,
            classbits,
            class_charcount,
            class_lastchar,
            class_utf8,
            utf8data,
            c,
            utf8,
            cd,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn lone_class_char(
    code: &mut [u8],
    st: &mut BranchState,
    classbits: &mut [u8; 32],
    class_charcount: &mut i32,
    class_lastchar: &mut i32,
    class_utf8: &mut bool,
    utf8data: &mut usize,
    mut c: i32,
    utf8: bool,
    cd: &mut CompileData<'_, '_>,
) -> Result<(), CompileFail> {
    if utf8 && (c > 255 || (st.options & pcre::CASELESS != 0 && c > 127)) {
        *class_utf8 = true;
        emit(code, utf8data, XCL_SINGLE)?;
        *utf8data = utf8data.wrapping_add(encode_utf8(
            c,
            code.get_mut(*utf8data..).ok_or(Error::InvalidResource)?,
            cd.tables,
        )?);
    } else {
        let slot = classbits
            .get_mut((c / 8) as usize)
            .ok_or(Error::InvalidResource)?;
        *slot |= 1 << (c & 7);
        if st.options & pcre::CASELESS != 0 {
            c = *cd.fcc.get(c as usize).ok_or(Error::InvalidResource)? as i32;
            let slot = classbits
                .get_mut((c / 8) as usize)
                .ok_or(Error::InvalidResource)?;
            *slot |= 1 << (c & 7);
        }
        *class_charcount += 1;
        *class_lastchar = c;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn class_range(
    code: &mut [u8],
    st: &mut BranchState,
    pattern: &[u8],
    classbits: &mut [u8; 32],
    class_charcount: &mut i32,
    class_lastchar: &mut i32,
    class_utf8: &mut bool,
    utf8data: &mut usize,
    c: i32,
    utf8: bool,
    cd: &mut CompileData<'_, '_>,
) -> Result<(), CompileFail> {
    let tables = cd.tables;
    st.ptr = st.ptr.wrapping_add(2);
    let mut d = if utf8 {
        getchar(pattern, &mut st.ptr, tables)?
    } else {
        byte_at(pattern, st.ptr)? as i32
    };
    if d == b'\\' as i32 {
        let oldptr = st.ptr;
        let read = read_escape(pattern, st.ptr, st.brackets, st.options, true, tables)?;
        st.ptr = read.offset;
        d = read.value;
        if d < 0 {
            if d == -esc::B_LOWER {
                d = 8;
            } else if d == -esc::X {
                d = b'X' as i32;
            } else {
                st.ptr = oldptr.wrapping_sub(2);
                return lone_class_char(
                    code,
                    st,
                    classbits,
                    class_charcount,
                    class_lastchar,
                    class_utf8,
                    utf8data,
                    c,
                    utf8,
                    cd,
                );
            }
        }
    }
    if d == c {
        return lone_class_char(
            code,
            st,
            classbits,
            class_charcount,
            class_lastchar,
            class_utf8,
            utf8data,
            c,
            utf8,
            cd,
        );
    }

    if utf8 && (d > 255 || (st.options & pcre::CASELESS != 0 && d > 127)) {
        *class_utf8 = true;
        emit(code, utf8data, XCL_RANGE)?;
        *utf8data = utf8data.wrapping_add(encode_utf8(
            c,
            code.get_mut(*utf8data..).ok_or(Error::InvalidResource)?,
            tables,
        )?);
        *utf8data = utf8data.wrapping_add(encode_utf8(
            d,
            code.get_mut(*utf8data..).ok_or(Error::InvalidResource)?,
            tables,
        )?);
        if st.options & pcre::CASELESS == 0 || c > 127 {
            return Ok(());
        }
        d = 127;
    }
    let mut c = c;
    while c <= d {
        let slot = classbits
            .get_mut((c / 8) as usize)
            .ok_or(Error::InvalidResource)?;
        *slot |= 1 << (c & 7);
        if st.options & pcre::CASELESS != 0 {
            let uc = cd.fcc[c as usize] as i32;
            let slot = classbits
                .get_mut((uc / 8) as usize)
                .ok_or(Error::InvalidResource)?;
            *slot |= 1 << (uc & 7);
        }
        *class_charcount += 1;
        *class_lastchar = c;
        c += 1;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_repeat(
    code: &mut [u8],
    st: &mut BranchState,
    pattern: &[u8],
    repeat_min: i32,
    repeat_max: i32,
    greedy_default: i32,
    greedy_non_default: i32,
    utf8_char: &mut [u8; 6],
    utf8: bool,
    cd: &mut CompileData<'_, '_>,
) -> Result<(), CompileFail> {
    let previous = st
        .previous
        .ok_or_else(|| fail_st(CompileError::NothingToRepeat, st))?;
    if repeat_min == 0 {
        st.firstbyte = st.zerofirstbyte;
        st.reqbyte = st.zeroreqbyte;
    }
    let reqvary = if repeat_min == repeat_max {
        0
    } else {
        pcre::REQ_VARY
    };
    let mut op_type = 0i32;
    let mut possessive_quantifier = false;
    let tempcode = previous;
    let repeat_type;
    if byte_at(pattern, st.ptr.wrapping_add(1))? == b'+' {
        repeat_type = 0;
        possessive_quantifier = true;
        st.ptr = st.ptr.wrapping_add(1);
    } else if byte_at(pattern, st.ptr.wrapping_add(1))? == b'?' {
        repeat_type = greedy_non_default;
        st.ptr = st.ptr.wrapping_add(1);
    } else {
        repeat_type = greedy_default;
    }
    if byte_at(code, previous)? == op::RECURSE {
        shift(code, previous, 3, previous + 3)?;
        st.code += 1 + LINK_SIZE;
        *code.get_mut(previous).ok_or(Error::InvalidResource)? = op::BRA;
        put(
            code,
            previous.wrapping_add(1),
            st.code.wrapping_sub(previous),
        )?;
        *code.get_mut(st.code).ok_or(Error::InvalidResource)? = op::KET;
        put(
            code,
            st.code.wrapping_add(1),
            st.code.wrapping_sub(previous),
        )?;
        st.code += 1 + LINK_SIZE;
    }
    let prev_op = byte_at(code, previous)?;
    let mut skip_possessive = false;
    if prev_op == op::CHAR || prev_op == op::CHARNC {
        let c;
        if utf8 && byte_at(code, st.code.wrapping_sub(1))? & 0x80 != 0 {
            let mut lastchar = st.code.wrapping_sub(1);
            while byte_at(code, lastchar)? & 0xc0 == 0x80 {
                lastchar = lastchar.wrapping_sub(1);
            }
            let len = st.code.wrapping_sub(lastchar);
            utf8_char
                .get_mut(..len)
                .ok_or(Error::InvalidResource)?
                .copy_from_slice(code.get(lastchar..st.code).ok_or(Error::InvalidResource)?);
            c = (len as i32) | 0x80;
        } else {
            c = byte_at(code, st.code.wrapping_sub(1))? as i32;
            if repeat_min > 1 {
                st.reqbyte = c | st.req_caseopt | cd.req_varyopt;
            }
        }
        skip_possessive = emit_single_repeat(
            code,
            st,
            c,
            op_type,
            repeat_min,
            repeat_max,
            repeat_type,
            utf8_char,
            utf8,
            cd,
        )?;
    } else if prev_op == op::NOT {
        op_type = op::NOTSTAR as i32 - op::STAR as i32;
        let c = byte_at(code, previous.wrapping_add(1))? as i32;
        skip_possessive = emit_single_repeat(
            code,
            st,
            c,
            op_type,
            repeat_min,
            repeat_max,
            repeat_type,
            utf8_char,
            utf8,
            cd,
        )?;
    } else if prev_op < op::EODN {
        op_type = op::TYPESTAR as i32 - op::STAR as i32;
        let c = prev_op as i32;
        skip_possessive = emit_single_repeat(
            code,
            st,
            c,
            op_type,
            repeat_min,
            repeat_max,
            repeat_type,
            utf8_char,
            utf8,
            cd,
        )?;
    } else if prev_op == op::CLASS
        || prev_op == op::NCLASS
        || prev_op == op::XCLASS
        || prev_op == op::REF
    {
        let mut repeat_max = repeat_max;
        if repeat_max == 0 {
            st.code = previous;
            skip_possessive = true;
        } else {
            if repeat_max != 1 {
                cd.nopartial = true;
            }
            if repeat_min == 0 && repeat_max == -1 {
                emit(
                    code,
                    &mut st.code,
                    op::CRSTAR.wrapping_add(repeat_type as u8),
                )?;
            } else if repeat_min == 1 && repeat_max == -1 {
                emit(
                    code,
                    &mut st.code,
                    op::CRPLUS.wrapping_add(repeat_type as u8),
                )?;
            } else if repeat_min == 0 && repeat_max == 1 {
                emit(
                    code,
                    &mut st.code,
                    op::CRQUERY.wrapping_add(repeat_type as u8),
                )?;
            } else {
                emit(
                    code,
                    &mut st.code,
                    op::CRRANGE.wrapping_add(repeat_type as u8),
                )?;
                put(code, st.code, repeat_min as usize)?;
                st.code += 2;
                if repeat_max == -1 {
                    repeat_max = 0;
                }
                put(code, st.code, repeat_max as usize)?;
                st.code += 2;
            }
        }
    } else if prev_op >= op::BRA || prev_op == op::ONCE || prev_op == op::COND {
        skip_possessive = repeat_group(
            code,
            st,
            previous,
            repeat_min,
            repeat_max,
            repeat_type,
            utf8,
            cd,
        )?;
    } else {
        return Err(fail_st(CompileError::UnexpectedRepeat, st));
    }
    if possessive_quantifier && !skip_possessive {
        let len = st.code.wrapping_sub(tempcode);
        shift(code, tempcode, len, tempcode + 1 + LINK_SIZE)?;
        st.code += 1 + LINK_SIZE;
        let len = len + 1 + LINK_SIZE;
        *code.get_mut(tempcode).ok_or(Error::InvalidResource)? = op::ONCE;
        emit(code, &mut st.code, op::KET)?;
        put(code, st.code, len)?;
        st.code += LINK_SIZE;
        put(code, tempcode.wrapping_add(1), len)?;
    }
    st.previous = None;
    cd.req_varyopt |= reqvary;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_single_repeat(
    code: &mut [u8],
    st: &mut BranchState,
    c: i32,
    op_type: i32,
    repeat_min: i32,
    repeat_max: i32,
    repeat_type: i32,
    utf8_char: &[u8; 6],
    utf8: bool,
    cd: &mut CompileData<'_, '_>,
) -> Result<bool, Error> {

    let previous = st.previous.unwrap_or(st.code);
    let prop_type =
        if byte_at(code, previous)? == op::PROP || byte_at(code, previous)? == op::NOTPROP {
            byte_at(code, previous.wrapping_add(1))? as i32
        } else {
            -1
        };
    let oldcode = st.code;
    st.code = previous;
    if repeat_max == 0 {
        return Ok(true);
    }
    if repeat_max != 1 {
        cd.nopartial = true;
    }
    let repeat_type = repeat_type + op_type;
    let mut repeat_max = repeat_max;
    if repeat_min == 0 {
        if repeat_max == -1 {
            emit(code, &mut st.code, op::STAR.wrapping_add(repeat_type as u8))?;
        } else if repeat_max == 1 {
            emit(
                code,
                &mut st.code,
                op::QUERY.wrapping_add(repeat_type as u8),
            )?;
        } else {
            emit(code, &mut st.code, op::UPTO.wrapping_add(repeat_type as u8))?;
            put(code, st.code, repeat_max as usize)?;
            st.code += 2;
        }
    } else if repeat_min == 1 {
        if repeat_max == -1 {
            emit(code, &mut st.code, op::PLUS.wrapping_add(repeat_type as u8))?;
        } else {
            st.code = oldcode;
            if repeat_max == 1 {
                return Ok(true);
            }
            emit(code, &mut st.code, op::UPTO.wrapping_add(repeat_type as u8))?;
            put(code, st.code, repeat_max.wrapping_sub(1) as usize)?;
            st.code += 2;
        }
    } else {
        emit(code, &mut st.code, op::EXACT.wrapping_add(op_type as u8))?;
        put(code, st.code, repeat_min as usize)?;
        st.code += 2;
        if repeat_max < 0 {
            if utf8 && c >= 128 {
                let len = (c & 7) as usize;
                code.get_mut(st.code..st.code.wrapping_add(len))
                    .ok_or(Error::InvalidResource)?
                    .copy_from_slice(&utf8_char[..len]);
                st.code += len;
            } else {
                emit(code, &mut st.code, c as u8)?;
                if prop_type >= 0 {
                    emit(code, &mut st.code, prop_type as u8)?;
                }
            }
            emit(code, &mut st.code, op::STAR.wrapping_add(repeat_type as u8))?;
        } else if repeat_max != repeat_min {
            if utf8 && c >= 128 {
                let len = (c & 7) as usize;
                code.get_mut(st.code..st.code.wrapping_add(len))
                    .ok_or(Error::InvalidResource)?
                    .copy_from_slice(&utf8_char[..len]);
                st.code += len;
            } else {
                emit(code, &mut st.code, c as u8)?;
            }
            if prop_type >= 0 {
                emit(code, &mut st.code, prop_type as u8)?;
            }
            repeat_max -= repeat_min;
            emit(code, &mut st.code, op::UPTO.wrapping_add(repeat_type as u8))?;
            put(code, st.code, repeat_max as usize)?;
            st.code += 2;
        }
    }
    if utf8 && c >= 128 {
        let len = (c & 7) as usize;
        code.get_mut(st.code..st.code.wrapping_add(len))
            .ok_or(Error::InvalidResource)?
            .copy_from_slice(&utf8_char[..len]);
        st.code += len;
    } else {
        emit(code, &mut st.code, c as u8)?;
    }
    Ok(false)
}

#[allow(clippy::too_many_arguments)]
fn repeat_group(
    code: &mut [u8],
    st: &mut BranchState,
    previous: usize,
    repeat_min: i32,
    repeat_max: i32,
    repeat_type: i32,
    utf8: bool,
    cd: &mut CompileData<'_, '_>,
) -> Result<bool, CompileFail> {

    let tables = cd.tables;
    let mut ketoffset = 0;
    let len = st.code.wrapping_sub(previous);
    let mut bralink: Option<usize> = None;

    let mut src = previous;
    let mut repeat_max = repeat_max;
    if repeat_max == -1 {
        let mut ket = previous;
        loop {
            ket = ket.wrapping_add(link_at(code, ket)? as usize);
            if byte_at(code, ket)? == op::KET {
                break;
            }
        }
        ketoffset = (st.code as i32).wrapping_sub(ket as i32);
    }
    if repeat_min == 0 {
        if repeat_max == 0 {
            st.code = previous;
            return Ok(true);
        } else if repeat_max <= 1 {
            *code.get_mut(st.code).ok_or(Error::InvalidResource)? = op::END;
            adjust_recurse(code, previous, 1, utf8, tables)?;
            shift(code, previous, len, previous + 1)?;
            st.code += 1;
            *code.get_mut(previous).ok_or(Error::InvalidResource)? =
                op::BRAZERO.wrapping_add(repeat_type as u8);
            src = previous + 1;
        } else {
            *code.get_mut(st.code).ok_or(Error::InvalidResource)? = op::END;
            adjust_recurse(code, previous, 2 + LINK_SIZE, utf8, tables)?;
            shift(code, previous, len, previous + 2 + LINK_SIZE)?;
            st.code += 2 + LINK_SIZE;
            let mut at = previous;
            emit(code, &mut at, op::BRAZERO.wrapping_add(repeat_type as u8))?;
            emit(code, &mut at, op::BRA)?;
            let offset = bralink.map_or(0, |link| at.wrapping_sub(link));
            bralink = Some(at);
            put(code, at, offset)?;
            src = previous + 2 + LINK_SIZE;
        }
        repeat_max -= 1;
    } else {
        if repeat_min > 1 {
            if st.groupsetfirstbyte && st.reqbyte < 0 {
                st.reqbyte = st.firstbyte;
            }
            for _ in 1..repeat_min {
                let dst = st.code;
                shift(code, src, len, dst)?;
                st.code = dst + len;
            }
        }
        if repeat_max > 0 {
            repeat_max -= repeat_min;
        }
    }
    if repeat_max >= 0 {
        let mut i = repeat_max - 1;
        while i >= 0 {
            emit(
                code,
                &mut st.code,
                op::BRAZERO.wrapping_add(repeat_type as u8),
            )?;
            if i != 0 {
                emit(code, &mut st.code, op::BRA)?;
                let offset = bralink.map_or(0, |link| st.code.wrapping_sub(link));
                bralink = Some(st.code);
                put(code, st.code, offset)?;
                st.code += LINK_SIZE;
            }
            let dst = st.code;
            shift(code, src, len, dst)?;
            st.code = dst + len;
            i -= 1;
        }
        while let Some(link) = bralink {
            let offset = st.code.wrapping_sub(link).wrapping_add(1);
            let bra = st.code.wrapping_sub(offset);
            let oldlinkoffset = link_at(code, bra)? as usize;
            bralink = if oldlinkoffset == 0 {
                None
            } else {
                Some(link.wrapping_sub(oldlinkoffset))
            };
            emit(code, &mut st.code, op::KET)?;
            put(code, st.code, offset)?;
            st.code += LINK_SIZE;
            put(code, bra.wrapping_add(1), offset)?;
        }
    } else {
        let at = (st.code as i32).wrapping_sub(ketoffset);
        *code.get_mut(at as usize).ok_or(Error::InvalidResource)? =
            op::KETRMAX.wrapping_add(repeat_type as u8);
    }
    Ok(false)
}

enum ParenAction {
    Next,
    Break,
}

#[allow(clippy::too_many_arguments)]
fn compile_paren(
    code: &mut [u8],
    st: &mut BranchState,
    pattern: &[u8],
    greedy_default: &mut i32,
    greedy_non_default: &mut i32,
    utf8: bool,
    bc: Option<&BranchChain<'_>>,
    cd: &mut CompileData<'_, '_>,
) -> Result<ParenAction, CompileFail> {
    let tables = cd.tables;
    let mut newoptions = st.options;
    let mut skipbytes = 0;
    let bravalue;
    st.ptr = st.ptr.wrapping_add(1);
    if byte_at(pattern, st.ptr)? == b'?' {
        st.ptr = st.ptr.wrapping_add(1);
        match byte_at(pattern, st.ptr)? {
            b'#' => {
                st.ptr = st.ptr.wrapping_add(1);
                while byte_at(pattern, st.ptr)? != b')' {
                    st.ptr = st.ptr.wrapping_add(1);
                }
                return Ok(ParenAction::Next);
            }
            b':' => {
                bravalue = op::BRA;
                st.ptr = st.ptr.wrapping_add(1);
            }
            b'(' => {
                bravalue = op::COND;
                if byte_at(pattern, st.ptr.wrapping_add(1))? == b'R' {
                    *code
                        .get_mut(st.code.wrapping_add(1).wrapping_add(LINK_SIZE))
                        .ok_or(Error::InvalidResource)? = op::CREF;
                    put(
                        code,
                        st.code.wrapping_add(2).wrapping_add(LINK_SIZE),
                        CREF_RECURSE as usize,
                    )?;
                    skipbytes = 3;
                    st.ptr = st.ptr.wrapping_add(3);
                } else if tables.ctypes[byte_at(pattern, st.ptr.wrapping_add(1))? as usize] != 0 {

                    st.ptr = st.ptr.wrapping_add(1);
                    let mut condref = byte_at(pattern, st.ptr)? as i32 - '0' as i32;
                    loop {
                        st.ptr = st.ptr.wrapping_add(1);
                        if byte_at(pattern, st.ptr)? == b')' {
                            break;
                        }
                        condref = condref * 10 + byte_at(pattern, st.ptr)? as i32 - '0' as i32;
                    }
                    if condref == 0 {
                        return Err(fail_st(CompileError::BadConditionZero, st));
                    }
                    st.ptr = st.ptr.wrapping_add(1);
                    *code
                        .get_mut(st.code.wrapping_add(1).wrapping_add(LINK_SIZE))
                        .ok_or(Error::InvalidResource)? = op::CREF;
                    put(
                        code,
                        st.code.wrapping_add(2).wrapping_add(LINK_SIZE),
                        condref as usize,
                    )?;
                    skipbytes = 3;
                } else {

                }
            }
            b'=' => {
                bravalue = op::ASSERT;
                st.ptr = st.ptr.wrapping_add(1);
            }
            b'!' => {
                bravalue = op::ASSERT_NOT;
                st.ptr = st.ptr.wrapping_add(1);
            }
            b'<' => {
                st.ptr = st.ptr.wrapping_add(1);
                bravalue = match byte_at(pattern, st.ptr)? {
                    b'=' => op::ASSERTBACK,
                    b'!' => op::ASSERTBACK_NOT,

                    _ => return Err(fail_st(CompileError::BadLookbehindOpen, st)),
                };
                st.ptr = st.ptr.wrapping_add(1);
            }
            b'>' => {
                bravalue = op::ONCE;
                st.ptr = st.ptr.wrapping_add(1);
            }
            b'C' => {
                st.previous_callout = Some(st.code);
                st.after_manual_callout = 1;
                emit(code, &mut st.code, op::CALLOUT)?;
                let mut n = 0;
                loop {
                    st.ptr = st.ptr.wrapping_add(1);
                    if tables.ctypes[byte_at(pattern, st.ptr)? as usize] & 0x04 == 0 {
                        break;
                    }
                    n = n * 10 + byte_at(pattern, st.ptr)? as i32 - '0' as i32;
                }
                if n > 255 {
                    return Err(fail_st(CompileError::CalloutTooBig, st));
                }
                emit(code, &mut st.code, n as u8)?;
                put(code, st.code, st.ptr.wrapping_add(1))?;
                st.code += LINK_SIZE;
                put(code, st.code, 0)?;
                st.code += LINK_SIZE;
                st.previous = None;
                return Ok(ParenAction::Next);
            }
            b'P' => {
                st.ptr = st.ptr.wrapping_add(1);
                if byte_at(pattern, st.ptr)? == b'<' {
                    let name_start = st.ptr.wrapping_add(1);
                    let mut name_end = name_start;
                    while byte_at(pattern, name_end)? != b'>' {
                        name_end = name_end.wrapping_add(1);
                    }
                    let namelen = name_end.wrapping_sub(name_start);
                    st.ptr = name_end.wrapping_add(1);
                    let name = pattern
                        .get(name_start..name_end)
                        .ok_or(Error::InvalidResource)?;
                    let entry_size = cd.name_entry_size;
                    let mut slot: usize = 0;
                    let mut i = 0;
                    while i < cd.names_found {
                        let base = slot;
                        let have = cd
                            .name_table
                            .get(base.wrapping_add(2)..base.wrapping_add(2).wrapping_add(namelen))
                            .ok_or(Error::InvalidResource)?;
                        let mut crc = memcmp(name, have);
                        if crc == 0 {
                            if cd
                                .name_table
                                .get(base.wrapping_add(2).wrapping_add(namelen))
                                .copied()
                                .unwrap_or(1)
                                == 0
                            {
                                return Err(fail_st(CompileError::DuplicateName, st));
                            }
                            crc = -1;
                        }
                        if crc < 0 {
                            shift(
                                cd.name_table,
                                slot,
                                (cd.names_found - i) as usize * entry_size,
                                slot + entry_size,
                            )?;
                            break;
                        }
                        slot += entry_size;
                        i += 1;
                    }
                    put(cd.name_table, slot, (st.brackets + 1) as usize)?;
                    cd.name_table
                        .get_mut(slot.wrapping_add(2)..slot.wrapping_add(2).wrapping_add(namelen))
                        .ok_or(Error::InvalidResource)?
                        .copy_from_slice(name);
                    *cd.name_table
                        .get_mut(slot.wrapping_add(2).wrapping_add(namelen))
                        .ok_or(Error::InvalidResource)? = 0;
                    cd.names_found += 1;
                    bravalue = numbered_group(code, st, &mut skipbytes)?;
                } else if byte_at(pattern, st.ptr)? == b'=' || byte_at(pattern, st.ptr)? == b'>' {
                    let typ = byte_at(pattern, st.ptr)?;
                    st.ptr = st.ptr.wrapping_add(1);
                    let name_start = st.ptr;
                    while byte_at(pattern, st.ptr)? != b')' {
                        st.ptr = st.ptr.wrapping_add(1);
                    }
                    let namelen = st.ptr.wrapping_sub(name_start);
                    let name = pattern
                        .get(name_start..st.ptr)
                        .ok_or(Error::InvalidResource)?;
                    let entry_size = cd.name_entry_size;
                    let mut slot: usize = 0;
                    let mut i = 0;
                    while i < cd.names_found {
                        let have = cd
                            .name_table
                            .get(slot.wrapping_add(2)..slot.wrapping_add(2).wrapping_add(namelen))
                            .ok_or(Error::InvalidResource)?;
                        if memcmp(name, have) == 0 {
                            break;
                        }
                        slot += entry_size;
                        i += 1;
                    }
                    if i >= cd.names_found {
                        return Err(fail_st(CompileError::BadSubpatternRef, st));
                    }
                    let recno = link_at(cd.name_table, slot)? as i32;
                    if typ == b'>' {
                        return handle_recursion(code, st, pattern, recno, utf8, bc, cd);
                    }
                    st.previous = Some(st.code);
                    emit(code, &mut st.code, op::REF)?;
                    put(code, st.code, recno as usize)?;
                    st.code += LINK_SIZE;
                    cd.backref_map |= if recno < 32 { 1 << recno } else { 1 };
                    if recno > cd.top_backref {
                        cd.top_backref = recno;
                    }
                    return Ok(ParenAction::Next);
                } else {

                    return Err(fail_st(CompileError::BadNameSyntax, st));
                }
            }
            b'R' => {
                st.ptr = st.ptr.wrapping_add(1);
                return handle_recursion(code, st, pattern, 0, utf8, bc, cd);
            }
            c if c.is_ascii_digit() => {
                let mut recno = 0;
                while tables.ctypes[byte_at(pattern, st.ptr)? as usize] & 0x04 != 0 {
                    recno = recno * 10 + byte_at(pattern, st.ptr)? as i32 - '0' as i32;
                    st.ptr = st.ptr.wrapping_add(1);
                }
                return handle_recursion(code, st, pattern, recno, utf8, bc, cd);
            }
            _ => {
                let mut set = 0u32;
                let mut unset = 0u32;
                let mut to_set = true;
                while byte_at(pattern, st.ptr)? != b')' && byte_at(pattern, st.ptr)? != b':' {
                    let opt = byte_at(pattern, st.ptr)?;
                    st.ptr = st.ptr.wrapping_add(1);
                    match opt {
                        b'-' => to_set = false,
                        b'i' => {
                            if to_set {
                                set |= pcre::CASELESS;
                            } else {
                                unset |= pcre::CASELESS;
                            }
                        }
                        b'm' => {
                            if to_set {
                                set |= pcre::MULTILINE;
                            } else {
                                unset |= pcre::MULTILINE;
                            }
                        }
                        b's' => {
                            if to_set {
                                set |= pcre::DOTALL;
                            } else {
                                unset |= pcre::DOTALL;
                            }
                        }
                        b'x' => {
                            if to_set {
                                set |= pcre::EXTENDED;
                            } else {
                                unset |= pcre::EXTENDED;
                            }
                        }
                        b'U' => {
                            if to_set {
                                set |= pcre::UNGREEDY;
                            } else {
                                unset |= pcre::UNGREEDY;
                            }
                        }
                        b'X' => {
                            if to_set {
                                set |= pcre::EXTRA;
                            } else {
                                unset |= pcre::EXTRA;
                            }
                        }
                        _ => {}
                    }
                }
                newoptions = (st.options | set) & !unset;
                if byte_at(pattern, st.ptr)? == b')' {
                    if st.options & pcre::IMS != newoptions & pcre::IMS {
                        emit(code, &mut st.code, op::OPT)?;
                        emit(code, &mut st.code, (newoptions & pcre::IMS) as u8)?;
                    }
                    st.options = newoptions;
                    *greedy_default = (newoptions & pcre::UNGREEDY != 0) as i32;
                    *greedy_non_default = *greedy_default ^ 1;
                    st.req_caseopt = if st.options & pcre::CASELESS != 0 {
                        pcre::REQ_CASELESS
                    } else {
                        0
                    };
                    st.previous = None;
                    return Ok(ParenAction::Next);
                }
                bravalue = op::BRA;
                st.ptr = st.ptr.wrapping_add(1);
            }
        }
    } else if st.options & pcre::NO_AUTO_CAPTURE != 0 {
        bravalue = op::BRA;
    } else {
        bravalue = numbered_group(code, st, &mut skipbytes)?;
    }
    st.previous = if bravalue >= op::ONCE {
        Some(st.code)
    } else {
        None
    };
    *code.get_mut(st.code).ok_or(Error::InvalidResource)? = bravalue;
    let group_start = st.code;
    let tempreqvary = cd.req_varyopt;
    let out = compile_group(
        code,
        st.code,
        pattern,
        st.ptr,
        newoptions,
        st.options & pcre::IMS,
        st.brackets,
        bravalue == op::ASSERTBACK || bravalue == op::ASSERTBACK_NOT,
        skipbytes,
        bc,
        cd,
    )?;
    st.code = out.code;
    st.ptr = out.ptr;
    st.brackets = out.brackets;
    let (mut subfirstbyte, mut subreqbyte) = (out.firstbyte, out.reqbyte);
    if bravalue == op::COND {
        let mut tc = group_start;
        let mut condcount = 0;
        loop {
            condcount += 1;
            tc = tc.wrapping_add(link_at(code, tc)? as usize);
            if byte_at(code, tc)? == op::KET {
                break;
            }
        }
        if condcount > 2 {
            return Err(fail_st(CompileError::TooManyCondBranches, st));
        }
        if condcount == 1 {
            subfirstbyte = pcre::REQ_NONE;
            subreqbyte = pcre::REQ_NONE;
        }
    }
    st.zeroreqbyte = st.reqbyte;
    st.zerofirstbyte = st.firstbyte;
    st.groupsetfirstbyte = false;
    if bravalue >= op::BRA || bravalue == op::ONCE || bravalue == op::COND {
        if st.firstbyte == pcre::REQ_UNSET {
            if subfirstbyte >= 0 {
                st.firstbyte = subfirstbyte;
                st.groupsetfirstbyte = true;
            } else {
                st.firstbyte = pcre::REQ_NONE;
            }
            st.zerofirstbyte = pcre::REQ_NONE;
        } else if subfirstbyte >= 0 && subreqbyte < 0 {
            subreqbyte = subfirstbyte | tempreqvary;
        }
        if subreqbyte >= 0 {
            st.reqbyte = subreqbyte;
        }
    } else if bravalue == op::ASSERT && subreqbyte >= 0 {
        st.reqbyte = subreqbyte;
    }
    if byte_at(pattern, st.ptr)? != b')' {
        return Err(fail_st(CompileError::MissingParen, st));
    }
    Ok(ParenAction::Break)
}

fn numbered_group(
    code: &mut [u8],
    st: &mut BranchState,
    skipbytes: &mut usize,
) -> Result<u8, Error> {
    st.brackets += 1;
    if st.brackets > EXTRACT_BASIC_MAX {
        *code
            .get_mut(st.code.wrapping_add(1).wrapping_add(LINK_SIZE))
            .ok_or(Error::InvalidResource)? = op::BRANUMBER;
        put(
            code,
            st.code.wrapping_add(2).wrapping_add(LINK_SIZE),
            st.brackets as usize,
        )?;
        *skipbytes = 3;
        Ok(op::BRA
            .wrapping_add(EXTRACT_BASIC_MAX as u8)
            .wrapping_add(1))
    } else {
        Ok(op::BRA.wrapping_add(st.brackets as u8))
    }
}

fn handle_recursion(
    code: &mut [u8],
    st: &mut BranchState,
    pattern: &[u8],
    recno: i32,
    utf8: bool,
    bc: Option<&BranchChain<'_>>,
    cd: &mut CompileData<'_, '_>,
) -> Result<ParenAction, CompileFail> {
    let tables = cd.tables;
    st.previous = Some(st.code);
    *code.get_mut(st.code).ok_or(Error::InvalidResource)? = op::END;
    let called = if recno == 0 {
        Some(0)
    } else {
        find_bracket(code, 0, recno, utf8, tables)?
    };
    let Some(called) = called else {
        return Err(fail_st(CompileError::BadSubpatternRef, st));
    };
    if link_at(code, called)? == 0 && could_be_empty(code, called, st.code, bc, utf8, tables)? {
        return Err(fail_st(CompileError::RecurseLoop, st));
    }
    *code.get_mut(st.code).ok_or(Error::InvalidResource)? = op::RECURSE;
    put(code, st.code.wrapping_add(1), called)?;
    st.code += 1 + LINK_SIZE;
    let _ = pattern;
    Ok(ParenAction::Next)
}

fn memcmp(a: &[u8], b: &[u8]) -> i32 {
    for (&x, &y) in a.iter().zip(b.iter()) {
        if x != y {
            return x as i32 - y as i32;
        }
    }
    0
}

#[allow(clippy::too_many_arguments)]
pub fn compile_group(
    code: &mut [u8],
    code_pos: usize,
    pattern: &[u8],
    ptr_pos: usize,
    options: u32,
    oldims: u32,
    brackets: i32,
    lookbehind: bool,
    skipbytes: usize,
    bc_outer: Option<&BranchChain<'_>>,
    cd: &mut CompileData<'_, '_>,
) -> Result<CompileOut, CompileFail> {
    let tables = cd.tables;
    let mut options = options;
    let mut brackets = brackets;
    let mut code_pos = code_pos;
    let mut ptr_pos = ptr_pos;
    let mut last_branch = code_pos;
    let start_bracket = code_pos;
    let mut reverse_count = 0;
    let (mut firstbyte, mut reqbyte) = (pcre::REQ_UNSET, pcre::REQ_UNSET);
    put(code, code_pos.wrapping_add(1), 0)?;
    code_pos = code_pos
        .wrapping_add(1)
        .wrapping_add(LINK_SIZE)
        .wrapping_add(skipbytes);

    let mut bc_current = start_bracket;
    loop {
        if options & pcre::IMS != oldims {
            emit(code, &mut code_pos, op::OPT)?;
            emit(code, &mut code_pos, (options & pcre::IMS) as u8)?;
        }
        if lookbehind {
            emit(code, &mut code_pos, op::REVERSE)?;
            reverse_count = code_pos;
            put(code, code_pos, 0)?;
            code_pos += LINK_SIZE;
        }
        let node = BranchChain {
            outer: bc_outer,
            current: bc_current,
        };
        let out = compile_branch(
            code,
            code_pos,
            pattern,
            ptr_pos,
            options,
            brackets,
            Some(&node),
            cd,
        )?;
        code_pos = out.code;
        ptr_pos = out.ptr;
        options = out.options;
        brackets = out.brackets;
        let (branchfirstbyte, branchreqbyte) = (out.firstbyte, out.reqbyte);
        if byte_at(code, last_branch)? != op::ALT {
            firstbyte = branchfirstbyte;
            reqbyte = branchreqbyte;
        } else {
            if firstbyte >= 0 && firstbyte != branchfirstbyte {
                if reqbyte < 0 {
                    reqbyte = firstbyte;
                }
                firstbyte = pcre::REQ_NONE;
            }
            let branchreqbyte = if firstbyte < 0 && branchfirstbyte >= 0 && branchreqbyte < 0 {
                branchfirstbyte
            } else {
                branchreqbyte
            };
            if (reqbyte & !pcre::REQ_VARY) != (branchreqbyte & !pcre::REQ_VARY) {
                reqbyte = pcre::REQ_NONE;
            } else {
                reqbyte |= branchreqbyte;
            }
        }
        if lookbehind {
            *code.get_mut(code_pos).ok_or(Error::InvalidResource)? = op::END;
            let length = fixed_branch_length(code, last_branch, options, tables)?;
            if length < 0 {
                return Err(fail_at(
                    if length == -2 {
                        CompileError::ByteInLookbehind
                    } else {
                        CompileError::VariableLookbehind
                    },
                    ptr_pos,
                    brackets,
                ));
            }
            put(code, reverse_count, length as usize)?;
        }
        if byte_at(pattern, ptr_pos)? != b'|' {
            let mut length = code_pos.wrapping_sub(last_branch);
            loop {
                let prev_length = link_at(code, last_branch)? as usize;
                put(code, last_branch.wrapping_add(1), length)?;
                length = prev_length;
                last_branch = last_branch.wrapping_sub(length);
                if length == 0 {
                    break;
                }
            }
            *code.get_mut(code_pos).ok_or(Error::InvalidResource)? = op::KET;
            put(
                code,
                code_pos.wrapping_add(1),
                code_pos.wrapping_sub(start_bracket),
            )?;
            code_pos = code_pos.wrapping_add(1).wrapping_add(LINK_SIZE);
            if options & pcre::IMS != oldims && byte_at(pattern, ptr_pos)? == b')' {
                emit(code, &mut code_pos, op::OPT)?;
                emit(code, &mut code_pos, oldims as u8)?;
            }
            return Ok(CompileOut {
                code: code_pos,
                ptr: ptr_pos,
                firstbyte,
                reqbyte,
                options,
                brackets,
            });
        }
        *code.get_mut(code_pos).ok_or(Error::InvalidResource)? = op::ALT;
        put(
            code,
            code_pos.wrapping_add(1),
            code_pos.wrapping_sub(last_branch),
        )?;
        bc_current = code_pos;
        last_branch = code_pos;
        code_pos = code_pos.wrapping_add(1).wrapping_add(LINK_SIZE);
        ptr_pos = ptr_pos.wrapping_add(1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Compiled {
    pub size: usize,
}

struct Prescan {
    length: i32,
    name_count: i32,
    max_name_size: i32,
    backref_map: u32,
    top_backref: i32,
    options: i32,
}

#[allow(clippy::too_many_arguments)]
pub fn compile(
    host: &mut impl PcreHost,
    object: u32,
    key: u32,
    heap: u32,
    pattern: &[u8],
    options: i32,
    tables: &PcreTables<'_>,
    out: &mut [u8],
    error_offset: Option<&mut i32>,
) -> Result<Option<Compiled>, CompileFail> {
    let mut error_offset = error_offset;
    if host.validate(object, key, 0xdd57, 24) >= 0x8000_0000 {
        return Ok(None);
    }
    let off = match error_offset.as_mut() {
        Some(off) => off,
        None => return Err(fail(CompileError::NullErrorOffset, 0)),
    };
    **off = 0;
    let utf8 = options & pcre::UTF8 as i32 != 0;
    if utf8 && options & pcre::NO_UTF8_CHECK as i32 == 0 {
        let bad = validate_utf8(pattern, -1, tables);
        **off = bad;
        if bad >= 0 {
            return Err(fail(CompileError::BadUtf8, bad as usize));
        }
    }
    if options & !(0x7a7f as i32) != 0 {
        **off = 0;
        return Err(fail(CompileError::UnknownOption, 0));
    }
    let pre = match prescan(pattern, options, tables) {
        Ok(pre) => pre,
        Err(CompileFail::Invalid(f)) => {
            **off = f.offset as i32;
            return Err(CompileFail::Invalid(f));
        }
        Err(e) => return Err(e),
    };
    if pre.length > 65536 {
        return Err(fail(CompileError::PatternTooLarge, **off as usize));
    }
    let entry_size = pre.max_name_size.wrapping_add(3);
    let size = pre
        .length
        .wrapping_add(40)
        .wrapping_add(pre.name_count.wrapping_mul(entry_size));
    let alloc_id = match host.alloc(heap, 1, size as u32) {
        Some(id) => id,
        None => {
            return Err(fail(CompileError::NoMemory, **off as usize));
        }
    };

    if out.len() < 40 {
        return Err(CompileFail::Resource(Error::Capacity {
            required: 40,
            available: out.len(),
        }));
    }
    let (head, tail) = out.split_at_mut(40);
    head[0..4].copy_from_slice(&0x5043_5245u32.to_le_bytes());
    head[4..8].copy_from_slice(&(size as u32).to_le_bytes());
    head[8..12].copy_from_slice(&(pre.options as u32).to_le_bytes());
    head[12..16].copy_from_slice(&0u32.to_le_bytes());
    head[24..26].copy_from_slice(&40u16.to_le_bytes());
    head[26..28].copy_from_slice(&(entry_size as u16).to_le_bytes());
    head[28..30].copy_from_slice(&(pre.name_count as u16).to_le_bytes());
    head[30..32].copy_from_slice(&0u16.to_le_bytes());
    head[32..36].copy_from_slice(&0u32.to_le_bytes());
    head[36..40].copy_from_slice(&0u32.to_le_bytes());
    let names_bytes = (pre.name_count as usize).wrapping_mul(entry_size as usize);
    if names_bytes > tail.len() {
        return Err(CompileFail::Resource(Error::Capacity {
            required: 40 + names_bytes,
            available: 40 + tail.len(),
        }));
    }
    let (names, code) = tail.split_at_mut(names_bytes);
    let mut cd = CompileData::new(tables, tables.default_tables, names, entry_size as usize);
    cd.backref_map = pre.backref_map;
    cd.top_backref = pre.top_backref;
    *code.get_mut(0).ok_or(Error::InvalidResource)? = op::BRA;
    cd.bracount = 0;

    let (outc, mut error) = match compile_group(
        code,
        0,
        pattern,
        0,
        pre.options as u32,
        pre.options as u32 & pcre::IMS,
        0,
        false,
        0,
        None,
        &mut cd,
    ) {
        Ok(outc) => {
            let error = if byte_at(pattern, outc.ptr)? != 0 {
                Some((CompileError::UnmatchedParens, outc.ptr))
            } else {
                None
            };
            (outc, error)
        }
        Err(CompileFail::Invalid(f)) => {

            let outc = CompileOut {
                code: 0,
                ptr: f.offset,
                firstbyte: 0,
                reqbyte: 0,
                options: 0,
                brackets: f.brackets,
            };
            (outc, Some((f.error, f.offset)))
        }
        Err(e) => return Err(e),
    };
    cd.firstbyte = outc.firstbyte;
    cd.bracount = outc.brackets;
    let reqbyte = outc.reqbyte;
    if error.is_none() {
        *code.get_mut(outc.code).ok_or(Error::InvalidResource)? = op::END;
    }

    if outc.code.wrapping_add(1) as i32 > pre.length {
        error = Some((CompileError::CodeOverflow, outc.ptr));
    }
    let top_bracket = cd.bracount as u16;
    let top_backref = cd.top_backref as u16;
    if top_backref > top_bracket {
        error = Some((CompileError::BadSubpatternRef, outc.ptr));
    }
    head[16..18].copy_from_slice(&top_bracket.to_le_bytes());
    head[18..20].copy_from_slice(&top_backref.to_le_bytes());
    if let Some((error, at)) = error {
        host.free(heap, alloc_id);
        **off = at as i32;
        return Err(fail(error, at));
    }
    let mut flags = pre.options as u32;
    if cd.nopartial {
        flags |= pcre::NOPARTIAL;
    }
    if flags & pcre::ANCHORED == 0 {
        let mut tempint = pre.options;
        if is_anchored(code, 0, &mut tempint, 0, cd.backref_map as i32, tables)? {
            flags |= pcre::ANCHORED;
        } else {
            if cd.firstbyte < 0 {
                let mut mask = tempint as u32;
                cd.firstbyte = find_required_char(code, 0, &mut mask, 0, tables)?;
            }
            if cd.firstbyte >= 0 {
                let fb = cd.firstbyte;

                let first = if fb & pcre::REQ_CASELESS != 0
                    && tables.default_tables[256 + (fb & 0xff) as usize] == (fb & 0xff) as u8
                {
                    (fb & !pcre::REQ_CASELESS) as u16
                } else {
                    fb as u16
                };
                head[20..22].copy_from_slice(&first.to_le_bytes());
                flags |= pcre::FIRSTSET;
            } else if first_char_is_fixed(code, 0, 0, cd.backref_map as i32, tables)? {
                flags |= pcre::STARTLINE;
            }
        }
    }
    if reqbyte >= 0 && (flags & pcre::ANCHORED == 0 || reqbyte & pcre::REQ_VARY != 0) {

        let req = if reqbyte & pcre::REQ_CASELESS != 0
            && tables.default_tables[256 + (reqbyte & 0xff) as usize] == (reqbyte & 0xff) as u8
        {
            (reqbyte & !pcre::REQ_CASELESS) as u16
        } else {
            reqbyte as u16
        };
        head[22..24].copy_from_slice(&req.to_le_bytes());
        flags |= pcre::REQCHSET;
    }
    head[8..12].copy_from_slice(&flags.to_le_bytes());
    Ok(Some(Compiled {
        size: size as usize,
    }))
}

fn prescan(pattern: &[u8], options: i32, tables: &PcreTables<'_>) -> Result<Prescan, CompileFail> {
    let ctypes = &tables.default_tables[832..1088];
    let mut options = options;
    let utf8 = options & pcre::UTF8 as i32 != 0;
    let mut length: i32 = 1 + LINK_SIZE as i32;
    let mut c: i32;
    let mut bracount = 0;
    let mut branch_extra = 0;
    let mut branch_newextra = 0;
    let mut item_count = -1;
    let mut name_count = 0;
    let mut max_name_size = 0;
    let mut lastitemlength = 0;
    let mut inescq = false;
    let mut brastackptr = 0;
    let mut brastack = [0i32; 200];
    let mut bralenstack = [0u8; 200];
    let mut duplength = 0;
    let mut backref_map = 0u32;
    let mut top_backref = 0;
    let mut ptr = 0;
    loop {
        c = byte_at(pattern, ptr)? as i32;
        if c == 0 {
            break;
        }
        if inescq {
            if options & pcre::AUTO_CALLOUT as i32 != 0 {
                length = length.wrapping_add(2 + 2 * LINK_SIZE as i32);
            }
            ptr = prescan_normal_char(
                pattern,
                ptr,
                &mut length,
                &mut lastitemlength,
                &mut inescq,
                utf8,
            )?;
            continue;
        }
        if options & pcre::EXTENDED as i32 != 0 {
            if ctypes[c as usize] & 0x01 != 0 {
                ptr = ptr.wrapping_add(1);
                continue;
            }
            if c == b'#' as i32 {
                loop {
                    ptr = ptr.wrapping_add(1);
                    c = byte_at(pattern, ptr)? as i32;
                    if c == 0 || c == b'\n' as i32 {
                        break;
                    }
                }
                if c == 0 {
                    break;
                }
                ptr = ptr.wrapping_add(1);
                continue;
            }
        }
        item_count += 1;
        if options & pcre::AUTO_CALLOUT as i32 != 0
            && c != b'*' as i32
            && c != b'+' as i32
            && c != b'?' as i32
            && (c != b'{' as i32 || !is_range_quantifier(pattern, ptr.wrapping_add(1), tables)?)
        {
            length = length.wrapping_add(2 + 2 * LINK_SIZE as i32);
        }
        match c as u8 {
            b'\\' => {
                let read = read_escape(pattern, ptr, bracount, options as u32, false, tables)?;
                ptr = read.offset;
                if let Some(error) = read.error {
                    return Err(fail(error, ptr));
                }
                c = read.value;
                lastitemlength = 1;
                if c >= 0 {
                    length = length.wrapping_add(2);
                    if utf8 && c > 127 {
                        let mut i = 0;
                        while i < 6 && c > tables.utf8_limits[i] {
                            i += 1;
                        }
                        length = length.wrapping_add(i as i32);
                        lastitemlength += i as i32;
                    }
                    ptr = ptr.wrapping_add(1);
                    continue;
                }
                if -c == esc::Q {
                    inescq = true;
                    ptr = ptr.wrapping_add(1);
                    continue;
                }
                if -c == esc::X || -c == esc::P_UPPER || -c == esc::P {
                    return Err(fail(CompileError::NoPropertySupport, ptr));
                }
                length = length.wrapping_add(1);
                if -c >= esc::REF {
                    let refnum = -c - esc::REF;
                    backref_map |= if refnum < 32 { 1 << refnum } else { 1 };
                    if refnum > top_backref {
                        top_backref = refnum;
                    }
                    length = length.wrapping_add(2);
                    if byte_at(pattern, ptr.wrapping_add(1))? == b'{'
                        && is_range_quantifier(pattern, ptr.wrapping_add(2), tables)?
                    {
                        match read_range(pattern, ptr.wrapping_add(2), tables)? {
                            Ok(read) => {
                                ptr = read.offset;

                                if (read.min == 0 && (read.max == 1 || read.max == -1))
                                    || (read.min == 1 && read.max == -1)
                                {
                                    length = length.wrapping_add(1);
                                } else {
                                    length = length.wrapping_add(5);
                                }
                            }
                            Err((off, error)) => return Err(fail(error, off)),
                        }
                        if byte_at(pattern, ptr.wrapping_add(1))? == b'?' {
                            ptr = ptr.wrapping_add(1);
                        }
                    }
                }
                ptr = ptr.wrapping_add(1);
                continue;
            }
            b'^' | b'.' | b'$' => {
                length = length.wrapping_add(1);
                lastitemlength = 1;
                ptr = ptr.wrapping_add(1);
                continue;
            }
            b'*' | b'+' | b'?' => {
                length = length.wrapping_add(1);
                ptr = prescan_possessive(pattern, ptr, &mut length)?;
                continue;
            }
            b'{' => {
                if !is_range_quantifier(pattern, ptr.wrapping_add(1), tables)? {
                    ptr = prescan_normal_char(
                        pattern,
                        ptr,
                        &mut length,
                        &mut lastitemlength,
                        &mut inescq,
                        utf8,
                    )?;
                    continue;
                }
                match read_range(pattern, ptr.wrapping_add(1), tables)? {
                    Ok(read) => {
                        ptr = read.offset;
                        if (read.min == 0 && (read.max == 1 || read.max == -1))
                            || (read.min == 1 && read.max == -1)
                        {
                            length = length.wrapping_add(1);
                        } else {
                            if read.min != 1 {
                                length = length.wrapping_sub(lastitemlength);
                                if read.min > 0 {
                                    length = length.wrapping_add(3).wrapping_add(lastitemlength);
                                }
                            }
                            length = length.wrapping_add(lastitemlength);
                            length = length.wrapping_add(if read.max > 0 { 3 } else { 1 });
                        }
                    }
                    Err((off, error)) => return Err(fail(error, off)),
                }
                if byte_at(pattern, ptr.wrapping_add(1))? == b'?' {
                    ptr = ptr.wrapping_add(1);
                }
                ptr = prescan_possessive(pattern, ptr, &mut length)?;
                continue;
            }
            b'|' => {
                length = length
                    .wrapping_add(1 + LINK_SIZE as i32)
                    .wrapping_add(branch_extra);
                ptr = ptr.wrapping_add(1);
                continue;
            }
            b'[' => {
                ptr = prescan_class(
                    pattern,
                    ptr,
                    &mut length,
                    &mut inescq,
                    &mut bracount,
                    options,
                    utf8,
                    tables,
                )?;
                continue;
            }
            b'(' => {
                let flow = prescan_paren(
                    pattern,
                    ptr,
                    &mut length,
                    &mut options,
                    &mut bracount,
                    &mut branch_extra,
                    &mut branch_newextra,
                    &mut item_count,
                    &mut name_count,
                    &mut max_name_size,
                    &mut brastack,
                    &mut bralenstack,
                    &mut brastackptr,
                    &mut duplength,
                    utf8,
                    tables,
                    ctypes,
                )?;
                ptr = match flow {
                    PrescanParen::Advance(next) => next,
                    PrescanParen::Quantified(next) => {
                        let (min, max, qnext) = prescan_bracket_quantifier(pattern, next, tables)?;
                        prescan_quantified_brackets(
                            pattern,
                            qnext,
                            min,
                            max,
                            &mut length,
                            duplength,
                        )?
                    }
                };
                continue;
            }
            b')' => {
                length = length.wrapping_add(1 + LINK_SIZE as i32);
                if brastackptr > 0 {
                    brastackptr -= 1;
                    duplength = length.wrapping_sub(brastack[brastackptr]);
                    branch_extra = bralenstack[brastackptr] as i32;
                } else {
                    duplength = 0;
                }
                let (min, max, next) = prescan_bracket_quantifier(pattern, ptr, tables)?;
                ptr = prescan_quantified_brackets(pattern, next, min, max, &mut length, duplength)?;
                continue;
            }
            _ => {
                ptr = prescan_normal_char(
                    pattern,
                    ptr,
                    &mut length,
                    &mut lastitemlength,
                    &mut inescq,
                    utf8,
                )?;
                continue;
            }
        }
    }
    length = length.wrapping_add(2 + LINK_SIZE as i32);
    if options & pcre::AUTO_CALLOUT as i32 != 0 {
        length = length.wrapping_add(2 + 2 * LINK_SIZE as i32);
    }
    Ok(Prescan {
        length,
        name_count,
        max_name_size,
        backref_map,
        top_backref,
        options,
    })
}

fn prescan_normal_char(
    pattern: &[u8],
    mut ptr: usize,
    length: &mut i32,
    lastitemlength: &mut i32,
    inescq: &mut bool,
    utf8: bool,
) -> Result<usize, Error> {
    let c = byte_at(pattern, ptr)? as i32;
    if *inescq && c == b'\\' as i32 && byte_at(pattern, ptr.wrapping_add(1))? == b'E' {
        *inescq = false;
        return Ok(ptr.wrapping_add(2));
    }
    *length = length.wrapping_add(2);
    *lastitemlength = 1;
    if utf8 && (c & 0xc0) == 0xc0 {
        while byte_at(pattern, ptr.wrapping_add(1))? & 0xc0 == 0x80 {
            *lastitemlength += 1;
            *length = length.wrapping_add(1);
            ptr = ptr.wrapping_add(1);
        }
    }
    Ok(ptr.wrapping_add(1))
}

fn prescan_possessive(pattern: &[u8], ptr: usize, length: &mut i32) -> Result<usize, Error> {
    if byte_at(pattern, ptr.wrapping_add(1))? == b'+' {
        *length = length.wrapping_add(2 + 2 * LINK_SIZE as i32);
        Ok(ptr.wrapping_add(2))
    } else {
        Ok(ptr.wrapping_add(1))
    }
}

fn prescan_bracket_quantifier(
    pattern: &[u8],
    ptr: usize,
    tables: &PcreTables<'_>,
) -> Result<(i32, i32, usize), CompileFail> {
    let c = byte_at(pattern, ptr.wrapping_add(1))? as i32;
    if c == b'{' as i32 && is_range_quantifier(pattern, ptr.wrapping_add(2), tables)? {
        match read_range(pattern, ptr.wrapping_add(2), tables)? {
            Ok(read) => Ok((read.min, read.max, read.offset)),
            Err((off, error)) => Err(fail(error, off)),
        }
    } else if c == b'*' as i32 {
        Ok((0, -1, ptr.wrapping_add(1)))
    } else if c == b'+' as i32 {
        Ok((1, -1, ptr.wrapping_add(1)))
    } else if c == b'?' as i32 {
        Ok((0, 1, ptr.wrapping_add(1)))
    } else {
        Ok((1, 1, ptr))
    }
}

enum PrescanParen {
    Advance(usize),
    Quantified(usize),
}

#[allow(clippy::too_many_arguments)]
fn prescan_paren(
    pattern: &[u8],
    mut ptr: usize,
    length: &mut i32,
    options: &mut i32,
    bracount: &mut i32,
    branch_extra: &mut i32,
    branch_newextra: &mut i32,
    item_count: &mut i32,
    name_count: &mut i32,
    max_name_size: &mut i32,
    brastack: &mut [i32; 200],
    bralenstack: &mut [u8; 200],
    brastackptr: &mut usize,
    duplength: &mut i32,
    utf8: bool,
    tables: &PcreTables<'_>,
    ctypes: &[u8],
) -> Result<PrescanParen, CompileFail> {
    let _ = utf8;
    *branch_newextra = 0;
    let mut bracket_length = 1 + LINK_SIZE as i32;
    if byte_at(pattern, ptr.wrapping_add(1))? == b'?' {
        match byte_at(pattern, ptr.wrapping_add(2))? {
            b'#' => {
                ptr = ptr.wrapping_add(3);
                while byte_at(pattern, ptr)? != 0 && byte_at(pattern, ptr)? != b')' {
                    ptr = ptr.wrapping_add(1);
                }
                if byte_at(pattern, ptr)? == 0 {
                    return Err(fail(CompileError::MissingCommentEnd, ptr));
                }
                return Ok(PrescanParen::Advance(ptr.wrapping_add(1)));
            }
            b':' | b'=' | b'!' | b'>' => {
                ptr = ptr.wrapping_add(2);
            }
            b'P' => {
                ptr = ptr.wrapping_add(3);
                if byte_at(pattern, ptr)? == b'<' {
                    ptr = ptr.wrapping_add(1);
                    let p = ptr;
                    while ctypes[byte_at(pattern, ptr)? as usize] & 0x10 != 0 {
                        ptr = ptr.wrapping_add(1);
                    }
                    if byte_at(pattern, ptr)? != b'>' {
                        return Err(fail(CompileError::BadNameUse, ptr));
                    }
                    *name_count += 1;
                    let namelen = ptr.wrapping_sub(p) as i32;
                    if namelen > *max_name_size {
                        *max_name_size = namelen;
                    }
                } else if byte_at(pattern, ptr)? == b'=' || byte_at(pattern, ptr)? == b'>' {
                    loop {
                        ptr = ptr.wrapping_add(1);
                        if ctypes[byte_at(pattern, ptr)? as usize] & 0x10 == 0 {
                            break;
                        }
                    }
                    if byte_at(pattern, ptr)? != b')' {
                        return Err(fail(CompileError::BadNameUse, ptr));
                    }
                } else {
                    return Err(fail(CompileError::BadNameSyntax, ptr));
                }
            }
            b'R' => {
                ptr = ptr.wrapping_add(1);
                return prescan_recurse(pattern, ptr, length, duplength, tables, b'R');
            }
            c if c.is_ascii_digit() => {
                return prescan_recurse(pattern, ptr, length, duplength, tables, c);
            }
            b'C' => {
                ptr = ptr.wrapping_add(2);
                loop {
                    ptr = ptr.wrapping_add(1);
                    if tables.ctypes[byte_at(pattern, ptr)? as usize] & 0x04 == 0 {
                        break;
                    }
                }
                if byte_at(pattern, ptr)? != b')' {
                    return Err(fail(CompileError::MissingCalloutEnd, ptr));
                }
                *length = length.wrapping_add(2 + 2 * LINK_SIZE as i32);
                return Ok(PrescanParen::Advance(ptr.wrapping_add(1)));
            }
            b'<' => {
                ptr = ptr.wrapping_add(3);
                if byte_at(pattern, ptr)? == b'=' || byte_at(pattern, ptr)? == b'!' {
                    *branch_newextra = 1 + LINK_SIZE as i32;
                    *length = length.wrapping_add(1 + LINK_SIZE as i32);
                } else {
                    return Err(fail(CompileError::BadLookbehindOpen, ptr));
                }
            }
            b'(' => {
                if byte_at(pattern, ptr.wrapping_add(3))? == b'R'
                    && byte_at(pattern, ptr.wrapping_add(4))? == b')'
                {
                    ptr = ptr.wrapping_add(4);
                    *length = length.wrapping_add(3);
                } else if tables.ctypes[byte_at(pattern, ptr.wrapping_add(3))? as usize] & 0x04 != 0
                {
                    ptr = ptr.wrapping_add(4);
                    *length = length.wrapping_add(3);
                    while tables.ctypes[byte_at(pattern, ptr)? as usize] & 0x04 != 0 {
                        ptr = ptr.wrapping_add(1);
                    }
                    if byte_at(pattern, ptr)? != b')' {
                        return Err(fail(CompileError::BadConditionNumber, ptr));
                    }
                } else {
                    ptr = ptr.wrapping_add(1);
                    let q = byte_at(pattern, ptr.wrapping_add(2))?;
                    let a = byte_at(pattern, ptr.wrapping_add(3))?;
                    if q != b'?' || (a != b'=' && a != b'!' && a != b'<') {
                        ptr = ptr.wrapping_add(2);
                        return Err(fail(CompileError::CondNeedsAssertion, ptr));
                    }
                }
            }
            _ => {
                let mut set = 0u32;
                let mut unset = 0u32;
                let mut to_set = true;
                ptr = ptr.wrapping_add(2);
                let mut c;
                loop {
                    c = byte_at(pattern, ptr)? as i32;
                    match c as u8 {
                        b'i' => {
                            if to_set {
                                set |= pcre::CASELESS;
                            } else {
                                unset |= pcre::CASELESS;
                            }
                        }
                        b'm' => {
                            if to_set {
                                set |= pcre::MULTILINE;
                            } else {
                                unset |= pcre::MULTILINE;
                            }
                        }
                        b's' => {
                            if to_set {
                                set |= pcre::DOTALL;
                            } else {
                                unset |= pcre::DOTALL;
                            }
                        }
                        b'x' => {
                            if to_set {
                                set |= pcre::EXTENDED;
                            } else {
                                unset |= pcre::EXTENDED;
                            }
                        }
                        b'X' => {
                            if to_set {
                                set |= pcre::EXTRA;
                            } else {
                                unset |= pcre::EXTRA;
                            }
                        }
                        b'U' => {
                            if to_set {
                                set |= pcre::UNGREEDY;
                            } else {
                                unset |= pcre::UNGREEDY;
                            }
                        }
                        b'-' => to_set = false,
                        b')' => {
                            if *item_count == 0 {
                                *options = ((*options as u32 | set) & !unset) as i32;
                                set = 0;
                                unset = 0;
                                *item_count -= 1;
                            }

                            if (set | unset) & pcre::IMS != 0 {
                                *length = length.wrapping_add(4);
                                *branch_newextra = 2;
                                if (set | unset) & pcre::CASELESS != 0 {
                                    *options |= pcre::ICHANGED as i32;
                                }
                            }
                            if *branch_newextra == 2
                                && (*branch_extra == 0 || *branch_extra == 1 + LINK_SIZE as i32)
                            {
                                *branch_extra += *branch_newextra;
                            }
                            return Ok(PrescanParen::Advance(ptr.wrapping_add(1)));
                        }
                        b':' => {
                            if (set | unset) & pcre::IMS != 0 {
                                *length = length.wrapping_add(4);
                                *branch_newextra = 2;
                                if (set | unset) & pcre::CASELESS != 0 {
                                    *options |= pcre::ICHANGED as i32;
                                }
                            }
                            break;
                        }
                        _ => return Err(fail(CompileError::BadGroupOpen, ptr)),
                    }
                    ptr = ptr.wrapping_add(1);
                }
            }
        }
    } else if *options & pcre::NO_AUTO_CAPTURE as i32 == 0 {
        *bracount += 1;
        if *bracount > EXTRACT_BASIC_MAX {
            bracket_length += 3;
        }
    }
    if *brastackptr >= 200 {
        return Err(fail(CompileError::NestingTooDeep, ptr));
    }
    bralenstack[*brastackptr] = *branch_extra as u8;
    *branch_extra = *branch_newextra;
    brastack[*brastackptr] = *length;
    *brastackptr += 1;
    *length = length.wrapping_add(bracket_length);
    Ok(PrescanParen::Advance(ptr.wrapping_add(1)))
}

fn prescan_recurse(
    pattern: &[u8],
    mut ptr: usize,
    length: &mut i32,
    duplength: &mut i32,
    tables: &PcreTables<'_>,
    c: u8,
) -> Result<PrescanParen, CompileFail> {
    ptr = ptr.wrapping_add(2);
    if c != b'R' {
        while tables.ctypes[byte_at(pattern, ptr)? as usize] & 0x04 != 0 {
            ptr = ptr.wrapping_add(1);
        }
    }
    if byte_at(pattern, ptr)? != b')' {
        return Err(fail(CompileError::BadRecurseSyntax, ptr));
    }
    *length = length.wrapping_add(1 + LINK_SIZE as i32);
    let next = byte_at(pattern, ptr.wrapping_add(1))?;
    if next == b'+' || next == b'*' || next == b'?' || next == b'{' {
        *length = length.wrapping_add(2 + 2 * LINK_SIZE as i32);
        *duplength = 5 + 3 * LINK_SIZE as i32;
        Ok(PrescanParen::Quantified(ptr))
    } else {
        Ok(PrescanParen::Advance(ptr.wrapping_add(1)))
    }
}

#[allow(clippy::too_many_arguments)]
fn prescan_class(
    pattern: &[u8],
    mut ptr: usize,
    length: &mut i32,
    inescq: &mut bool,
    bracount: &mut i32,
    options: i32,
    utf8: bool,
    tables: &PcreTables<'_>,
) -> Result<usize, CompileFail> {
    let ctypes = &tables.default_tables[832..1088];
    ptr = ptr.wrapping_add(1);
    let mut class_optcount;
    if byte_at(pattern, ptr)? == b'^' {
        class_optcount = 10;
        ptr = ptr.wrapping_add(1);
    } else {
        class_optcount = 0;
    }
    let mut class_utf8 = false;
    if byte_at(pattern, ptr)? != 0 {
        loop {
            if *inescq {
                if byte_at(pattern, ptr)? != b'\\' || byte_at(pattern, ptr.wrapping_add(1))? != b'E'
                {
                    let (c, next) = prescan_class_char(pattern, ptr, utf8, tables)?;
                    ptr = next;
                    ptr = prescan_class_nonspecial(
                        pattern,
                        ptr,
                        c,
                        length,
                        &mut class_optcount,
                        &mut class_utf8,
                        bracount,
                        options,
                        utf8,
                        tables,
                    )?;
                } else {
                    *inescq = false;
                    ptr = ptr.wrapping_add(1);
                }
            } else if byte_at(pattern, ptr)? == b'\\' {
                let read = read_escape(pattern, ptr, *bracount, options as u32, true, tables)?;
                ptr = read.offset;
                if let Some(error) = read.error {
                    return Err(fail(error, ptr));
                }
                let c0 = read.value;
                if -c0 == esc::Q {
                    *inescq = true;
                } else {
                    let c = if -c0 == esc::B_LOWER {
                        8
                    } else if -c0 == esc::X {
                        b'X' as i32
                    } else {
                        c0
                    };
                    if c >= 0 {
                        ptr = prescan_class_nonspecial(
                            pattern,
                            ptr,
                            c,
                            length,
                            &mut class_optcount,
                            &mut class_utf8,
                            bracount,
                            options,
                            utf8,
                            tables,
                        )?;
                    } else {
                        class_optcount = 10;
                        if -c == esc::P_UPPER || -c == esc::P {
                            if !class_utf8 {
                                class_utf8 = true;
                                *length = length.wrapping_add(LINK_SIZE as i32 + 2);
                            }
                            *length = length.wrapping_add(2);
                        }
                    }
                }
            } else if byte_at(pattern, ptr)? == b'[' {
                let np = ptr.wrapping_add(1);
                let mut p = np.wrapping_add(1);
                if byte_at(pattern, p)? == b'^' {
                    p = p.wrapping_add(1);
                }
                let mut c = byte_at(pattern, p)? as i32;
                while ctypes[c as usize] & 0x02 != 0 {
                    p = p.wrapping_add(1);
                    c = byte_at(pattern, p)? as i32;
                }
                if c == byte_at(pattern, np)? as i32 && byte_at(pattern, p.wrapping_add(1))? == b']'
                {
                    ptr = p.wrapping_add(1);
                    class_optcount = 10;
                } else {

                    let (c, next) = prescan_class_char(pattern, ptr, utf8, tables)?;
                    ptr = next;
                    ptr = prescan_class_nonspecial(
                        pattern,
                        ptr,
                        c,
                        length,
                        &mut class_optcount,
                        &mut class_utf8,
                        bracount,
                        options,
                        utf8,
                        tables,
                    )?;
                }
            } else {
                let (c, next) = prescan_class_char(pattern, ptr, utf8, tables)?;
                ptr = next;
                ptr = prescan_class_nonspecial(
                    pattern,
                    ptr,
                    c,
                    length,
                    &mut class_optcount,
                    &mut class_utf8,
                    bracount,
                    options,
                    utf8,
                    tables,
                )?;
            }
            ptr = ptr.wrapping_add(1);
            if byte_at(pattern, ptr)? == 0 || (!*inescq && byte_at(pattern, ptr)? == b']') {
                break;
            }
        }
    }
    if byte_at(pattern, ptr)? == 0 {
        return Err(fail(CompileError::MissingClassEnd, ptr));
    }
    if class_optcount == 1 {
        *length = length.wrapping_add(3);
    } else {
        *length = length.wrapping_add(33);
        if byte_at(pattern, ptr.wrapping_add(1))? == b'{'
            && is_range_quantifier(pattern, ptr.wrapping_add(2), tables)?
        {
            match read_range(pattern, ptr.wrapping_add(2), tables)? {
                Ok(read) => {
                    ptr = read.offset;

                    if (read.min == 0 && (read.max == 1 || read.max == -1))
                        || (read.min == 1 && read.max == -1)
                    {
                        *length = length.wrapping_add(1);
                    } else {
                        *length = length.wrapping_add(5);
                    }
                }
                Err((off, error)) => return Err(fail(error, off)),
            }
            if byte_at(pattern, ptr.wrapping_add(1))? == b'+' {
                ptr = ptr.wrapping_add(1);
                *length = length.wrapping_add(2 + 2 * LINK_SIZE as i32);
            } else if byte_at(pattern, ptr.wrapping_add(1))? == b'?' {
                ptr = ptr.wrapping_add(1);
            }
        }
    }
    Ok(ptr.wrapping_add(1))
}

fn prescan_class_char(
    pattern: &[u8],
    mut ptr: usize,
    utf8: bool,
    tables: &PcreTables<'_>,
) -> Result<(i32, usize), Error> {
    if utf8 {
        let mut c = byte_at(pattern, ptr)? as i32;
        if (c & 0xc0) == 0xc0 {
            let mut extra = 0;
            let a = tables.utf8_cont[(c & 0x3f) as usize] as i32;
            let mut s = 6 * a;
            c = (tables.utf8_mask[a as usize] as i32) & c;
            c <<= s;
            let mut q = ptr;
            let mut aa = a;
            while aa > 0 {
                aa -= 1;
                q = q.wrapping_add(1);
                s -= 6;
                c |= ((byte_at(pattern, q)? & 0x3f) as i32) << s;
            }
            extra += tables.utf8_cont[byte_at(pattern, ptr)? as usize & 0x3f] as usize;
            ptr = ptr.wrapping_add(extra);
        }
        Ok((c, ptr))
    } else {
        Ok((byte_at(pattern, ptr)? as i32, ptr))
    }
}

#[allow(clippy::too_many_arguments)]
fn prescan_class_nonspecial(
    pattern: &[u8],
    mut ptr: usize,
    c: i32,
    length: &mut i32,
    class_optcount: &mut i32,
    class_utf8: &mut bool,
    bracount: &mut i32,
    options: i32,
    utf8: bool,
    tables: &PcreTables<'_>,
) -> Result<usize, CompileFail> {
    *class_optcount += 1;
    let mut d = -1;
    if byte_at(pattern, ptr.wrapping_add(1))? == b'-' {
        let hyptr = ptr;
        ptr = ptr.wrapping_add(1);
        if byte_at(pattern, ptr.wrapping_add(1))? == b'\\' {
            ptr = ptr.wrapping_add(1);
            let read = read_escape(pattern, ptr, *bracount, options as u32, true, tables)?;
            ptr = read.offset;
            if let Some(error) = read.error {
                return Err(fail(error, ptr));
            }
            d = read.value;
            if -d == esc::B_LOWER {
                d = 8;
            } else if -d == esc::X {
                d = b'X' as i32;
            }
        } else if byte_at(pattern, ptr.wrapping_add(1))? != 0
            && byte_at(pattern, ptr.wrapping_add(1))? != b']'
        {
            ptr = ptr.wrapping_add(1);
            let (dd, next) = prescan_class_char(pattern, ptr, utf8, tables)?;
            ptr = next;
            d = dd;
        }
        if d < 0 {
            ptr = hyptr;
        }
    }
    if d >= 0 {
        *class_optcount = 10;
        if d < c {
            return Err(fail(CompileError::RangeOutOfOrder, ptr));
        }
        if utf8 && (d > 255 || (options & pcre::CASELESS as i32 != 0 && d > 127)) {
            let mut buffer = [0u8; 6];
            if !*class_utf8 {
                *class_utf8 = true;
                *length = length.wrapping_add(LINK_SIZE as i32 + 2);
            }
            *length = length
                .wrapping_add(1)
                .wrapping_add(encode_utf8(c, &mut buffer, tables)? as i32)
                .wrapping_add(encode_utf8(d, &mut buffer, tables)? as i32);
        }
    } else if utf8 && (c > 255 || (options & pcre::CASELESS as i32 != 0 && c > 127)) {
        let mut buffer = [0u8; 6];
        *class_optcount = 10;
        if !*class_utf8 {
            *class_utf8 = true;
            *length = length.wrapping_add(LINK_SIZE as i32 + 2);
        }
        *length = length
            .wrapping_add(1)
            .wrapping_add(encode_utf8(c, &mut buffer, tables)? as i32);
    }
    Ok(ptr)
}

fn prescan_quantified_brackets(
    pattern: &[u8],
    ptr: usize,
    min: i32,
    max: i32,
    length: &mut i32,
    duplength: i32,
) -> Result<usize, Error> {
    if min == 0 {
        *length = length.wrapping_add(1);
        if max > 0 {
            *length =
                length
                    .wrapping_add((max - 1).wrapping_mul(
                        duplength.wrapping_add(3).wrapping_add(2 * LINK_SIZE as i32),
                    ));
        }
    } else {
        *length = length.wrapping_add((min - 1).wrapping_mul(duplength));
        if max > min {
            *length = length
                .wrapping_add(
                    (max - min)
                        .wrapping_mul(duplength.wrapping_add(3).wrapping_add(2 * LINK_SIZE as i32)),
                )
                .wrapping_sub(2 + 2 * LINK_SIZE as i32);
        }
    }
    prescan_possessive(pattern, ptr, length)
}

impl FrameStore<'_> {

    pub fn frame_mut(&mut self, frame: FrameRef) -> &mut MatchFrame {
        let slot = self.slot(frame).expect("match frame slot");
        &mut self.frames[slot]
    }

    pub fn frame_ref(&self, frame: FrameRef) -> &MatchFrame {
        let slot = self.slot(frame).expect("match frame slot");
        &self.frames[slot]
    }
}

const MATCH_MATCH: i32 = 1;
const MATCH_NOMATCH: i32 = 0;
const MATCH_UNKNOWN_NODE: i32 = -5;
const MATCH_NOMEMORY: i32 = -6;
const MATCH_MATCHLIMIT: i32 = -8;
const MATCH_INTERNAL: i32 = -14;

pub struct ExecState<'a> {
    pub call_count: u32,
    pub match_limit: u32,
    pub offset_end: i32,
    pub offset_max: i32,
    pub offset_overflow: bool,
    pub notbol: bool,
    pub noteol: bool,
    pub utf8: bool,
    pub endonly: bool,
    pub notempty: bool,
    pub partial: bool,
    pub hitend: bool,
    pub lcc: &'a [u8],
    pub ctypes: &'a [u8],
    pub subj_start: i64,
    pub subj_end: i64,
    pub start_match: i64,
    pub end_match: i64,
    pub end_offset_top: i32,
    pub capture_last: i32,
    pub start_offset: i32,
    pub recursive: Option<FrameRef>,
    pub thisframe: Option<FrameRef>,
    pub user: &'a mut [i32],
    pub temp: [i32; 1024],
    pub temp_len: usize,
    pub using_temp: bool,
}

impl ExecState<'_> {
    fn ovec(&self) -> &[i32] {
        if self.using_temp {
            &self.temp[..self.temp_len]
        } else {
            self.user
        }
    }

    fn ovec_mut(&mut self) -> &mut [i32] {
        if self.using_temp {
            let len = self.temp_len;
            &mut self.temp[..len]
        } else {
            &mut *self.user
        }
    }
}

pub struct CalloutBlock<'a> {
    pub version: i32,
    pub number: i32,
    pub offsets: &'a [i32],
    pub subject: &'a [u8],
    pub start_match: i32,
    pub current_position: i32,
    pub capture_top: i32,
    pub capture_last: i32,
    pub callout_data: u32,
    pub pattern_position: i32,
    pub next_item_length: i32,
}

pub type CalloutFn = fn(&CalloutBlock) -> i32;

fn getchar_at(subject: &[u8], eptr: i64, tables: &PcreTables<'_>) -> i32 {
    let c = subject[eptr as usize] as i32;
    if c & 0xc0 != 0xc0 {
        return c;
    }
    let count = tables.utf8_cont[(c & 0x3f) as usize] as i32;
    let mut shift = 6 * count;
    let mut out = (c & tables.utf8_mask[count as usize] as i32) << shift as u32;
    for i in 1..=count {
        shift -= 6;
        out |= ((subject[(eptr + i as i64) as usize] & 0x3f) as i32) << shift as u32;
    }
    out
}

fn getchar_inc(subject: &[u8], eptr: &mut i64, tables: &PcreTables<'_>) -> i32 {
    let c = subject[*eptr as usize] as i32;
    *eptr += 1;
    if c & 0xc0 != 0xc0 {
        return c;
    }
    let mut count = tables.utf8_cont[(c & 0x3f) as usize] as i32;
    let mut shift = 6 * count;
    let mut out = (c & tables.utf8_mask[count as usize] as i32) << shift as u32;
    while count > 0 {
        count -= 1;
        shift -= 6;
        out |= ((subject[*eptr as usize] & 0x3f) as i32) << shift as u32;
        *eptr += 1;
    }
    out
}

fn getchar_inc_test(subject: &[u8], eptr: &mut i64, utf8: bool, tables: &PcreTables<'_>) -> i32 {
    if !utf8 {
        let c = subject[*eptr as usize] as i32;
        *eptr += 1;
        return c;
    }
    getchar_inc(subject, eptr, tables)
}

fn getchar_len(subject: &[u8], eptr: i64, len: &mut i32, tables: &PcreTables<'_>) -> i32 {
    let c = getchar_at(subject, eptr, tables);
    if (subject[eptr as usize] as i32) & 0xc0 == 0xc0 {
        *len += tables.utf8_cont[(subject[eptr as usize] & 0x3f) as usize] as i32;
    }
    c
}

fn back_char(subject: &[u8], eptr: &mut i64) {
    while subject[*eptr as usize] & 0xc0 == 0x80 {
        *eptr -= 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn match_ref(
    offset: i32,
    eptr: i64,
    length: i32,
    vector: &[i32],
    subj_start: i64,
    subj_end: i64,
    subject: &[u8],
    lcc: &[u8],
    ims: u32,
) -> bool {

    let base = vector.get(offset as usize).copied().unwrap_or(0) as i64;
    let p = subj_start + base;
    if length as i64 > subj_end - eptr {
        return false;
    }
    if ims & pcre::CASELESS != 0 {
        let mut left = length;
        let mut pp = p;
        let mut ep = eptr;
        while left > 0 {
            left -= 1;
            if lcc[subject[pp as usize] as usize] != lcc[subject[ep as usize] as usize] {
                return false;
            }
            pp += 1;
            ep += 1;
        }
    } else {
        let mut left = length;
        let mut pp = p;
        let mut ep = eptr;
        while left > 0 {
            left -= 1;
            if subject[pp as usize] != subject[ep as usize] {
                return false;
            }
            pp += 1;
            ep += 1;
        }
    }
    true
}

const FLAG_CONDATTR: u32 = 0x01;
const FLAG_ISGROUP: u32 = 0x02;

fn get2(code: &[u8], at: usize) -> i32 {
    ((code[at] as i32) << 8) | code[at + 1] as i32
}

macro_rules! rmatch {
    ($store:expr, $host:expr, $status:expr, $cur:expr, $st:expr, $rrc:expr, $entry:expr, $eptr:expr, $ecode:expr, $top:expr, $ims:expr, $eptrb:expr, $flags:expr, $rw:expr, $outer:lifetime, $run:lifetime) => {{
        let want: u8 = $rw;
        match $store.alloc(want, $host) {
            Ok(next) => {
                $status = 0;
                let up = $cur.expect("match push without frame");
                let (pe, pc, pt, pi, pb, pf) =
                    ($eptr, $ecode, $top, $ims, $eptrb, $flags);
                let frame = $store.frame_mut(next);
                frame.eptr = pe;
                frame.ecode = pc;
                frame.offset_top = pt;
                frame.ims = pi;
                frame.eptrb = pb;
                frame.prev = Some(up);
                frame.flags = pf;
                $cur = Some(next);
                continue $run;
            }
            Err(failed) => {
                $status = failed;
                rreturn!($store, $cur, $st, $rrc, $entry, MATCH_NOMEMORY, $outer);
            }
        }
    }};
}

macro_rules! rreturn {
    ($store:expr, $cur:expr, $st:expr, $rrc:expr, $entry:expr, $ra:expr, $outer:lifetime) => {{
        let back = $ra;
        let live = $cur.expect("match return without frame");
        let parent = $store.frame_ref(live).prev;
        let tag = $store.pop().expect("match frame underflow");
        match parent {
            Some(up) => {
                $store.frame_mut(up).result = back;
                $st.thisframe = Some(up);
                $rrc = back;
                $entry = tag;
                $cur = Some(up);
                continue $outer;
            }
            None => return back,
        }
    }};
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_assignments)]
pub fn pcre_match(
    host: &mut impl PcreHost,
    store: &mut FrameStore,
    entry: u8,
    subject_full: &[u8],
    code: &[u8],
    eptr: i64,
    top: i32,
    st: &mut ExecState,
    options: u32,
    blocks: Option<FrameRef>,
    frame_flags: i32,
    tables: &PcreTables<'_>,
    callout: Option<CalloutFn>,
    callout_data: u32,
) -> i32 {
    let mut entry = entry;
    let mut cur: Option<FrameRef> = None;
    let mut status: u32 = 0;
    let mut rrc: i32 = 0;
    let mut c: i32 = 0;
    let mut i: i32 = 0;
    let (init_eptr, init_ecode, init_top, init_ims, init_blocks, init_flags) =
        (eptr, 0usize, top, options, blocks, frame_flags);
    'outer: loop {
        let mut dispatched = false;

        let mut skip_head = false;
        'run: loop {
            if !dispatched {
                dispatched = true;
                match entry {
                    99 => match store.alloc(99, host) {
                        Ok(next) => {
                            status = 0;
                            let frame = store.frame_mut(next);
                            frame.eptr = init_eptr;
                            frame.prev = None;
                            frame.ecode = init_ecode;
                            frame.offset_top = init_top;
                            frame.ims = init_ims;
                            frame.eptrb = init_blocks;
                            frame.flags = init_flags as u32;
                            cur = Some(next);
                        }
                        Err(_) => {
                            return MATCH_NOMEMORY;
                        }
                    },
                    0 => {
                        let live = cur.expect("match reentry without frame");
                        if status >= 0x8000_0000 {
                            return MATCH_NOMEMORY;
                        }
                        let frame = store.frame_mut(live);
                        frame.eptr = init_eptr;
                        frame.prev = None;
                        frame.ecode = init_ecode;
                        frame.offset_top = init_top;
                        frame.ims = init_ims;
                        frame.eptrb = init_blocks;
                        frame.flags = init_flags as u32;
                    }
                    1 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let save_cap = store.frames[fslot].save_capture_last;
                        st.capture_last = save_cap;
                        let ecode = store.frames[fslot].ecode;
                        let ecode2 = ecode + get2(code, ecode + 1) as usize;
                        store.frames[fslot].ecode = ecode2;
                        if code[ecode2] == op::ALT {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe, fc + 1 + 2, ft, fi2, fb,
                                FLAG_ISGROUP, 1, 'outer, 'run
                            );
                        }
                        let offset = store.frames[fslot].offset;
                        let number = store.frames[fslot].number;
                        let save1 = store.frames[fslot].save_offset1;
                        let save2 = store.frames[fslot].save_offset2;
                        let save3 = store.frames[fslot].save_offset3;
                        st.ovec_mut()[offset as usize] = save1;
                        st.ovec_mut()[offset as usize + 1] = save2;
                        let at = (st.offset_end - number) as usize;
                        st.ovec_mut()[at] = save3;
                        rreturn!(store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer);
                    }
                    2 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let ecode = store.frames[fslot].ecode;
                        let ecode2 = ecode + get2(code, ecode + 1) as usize;
                        store.frames[fslot].ecode = ecode2;
                        if code[ecode2] == op::ALT {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe, fc + 1 + 2, ft, fi2, fb,
                                FLAG_ISGROUP, 2, 'outer, 'run
                            );
                        }
                        rreturn!(store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer);
                    }
                    3 => {
                        rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                    }
                    4 => {
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        if rrc == MATCH_MATCH {
                            let mut ecode = store.frames[fslot].ecode;
                            ecode += 1 + 2 + get2(code, ecode + 2 + 2) as usize;
                            while code[ecode] == op::ALT {
                                ecode += get2(code, ecode + 1) as usize;
                            }
                            store.frames[fslot].ecode = ecode;
                        } else if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        } else {
                            let ecode = store.frames[fslot].ecode;
                            let ecode2 = ecode + get2(code, ecode + 1) as usize;
                            store.frames[fslot].ecode = ecode2;
                        }
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc + 1 + 2, ft, fi2, fb,
                            FLAG_ISGROUP, 5, 'outer, 'run
                        );
                    }
                    5 => {
                        rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                    }
                    6 => {
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        if rrc == MATCH_MATCH {

                        } else if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        } else {
                            let ecode = store.frames[fslot].ecode;
                            let ecode2 = ecode + get2(code, ecode + 1) as usize;
                            store.frames[fslot].ecode = ecode2;
                            if code[ecode2] == op::ALT {
                                let fr = store.frame_ref(live);
                                let (fe, fc, ft, fi2) = (fr.eptr, fr.ecode, fr.offset_top, fr.ims);
                                rmatch!(
                                    store, host, status, cur, st, rrc, entry, fe, fc + 1 + 2, ft, fi2,
                                    None, FLAG_ISGROUP, 6, 'outer, 'run
                                );
                            }
                        }
                        let ecode = store.frames[fslot].ecode;
                        if code[ecode] == op::KET {
                            rreturn!(store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer);
                        }
                        if store.frames[fslot].flags & FLAG_CONDATTR != 0 {
                            rreturn!(store, cur, st, rrc, entry, MATCH_MATCH, 'outer);
                        }
                        let mut ecode = ecode;
                        loop {
                            ecode += get2(code, ecode + 1) as usize;
                            if code[ecode] != op::ALT {
                                break;
                            }
                        }
                        ecode += 1 + 2;
                        store.frames[fslot].ecode = ecode;
                        store.frames[fslot].offset_top = st.end_offset_top;
                        skip_head = true;
                    }
                    7 => {
                        if rrc == MATCH_MATCH {
                            rreturn!(store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer);
                        }
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let ecode = store.frames[fslot].ecode;
                        let ecode2 = ecode + get2(code, ecode + 1) as usize;
                        store.frames[fslot].ecode = ecode2;
                        if code[ecode2] == op::ALT {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2) = (fr.eptr, fr.ecode, fr.offset_top, fr.ims);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe, fc + 1 + 2, ft, fi2,
                                None, FLAG_ISGROUP, 7, 'outer, 'run
                            );
                        }
                        if store.frames[fslot].flags & FLAG_CONDATTR != 0 {
                            rreturn!(store, cur, st, rrc, entry, MATCH_MATCH, 'outer);
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1 + 2;
                        skip_head = true;
                    }
                    8 => {
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        if rrc == MATCH_MATCH {
                            let prev = store.frames[fslot].rec_prev;
                            st.recursive = prev;
                            if !store.frames[fslot].rec_scratch {
                                let id = store.frames[fslot].rec_scratch_base as u32;
                                host.free(store.heap, id);
                            }
                            rreturn!(store, cur, st, rrc, entry, MATCH_MATCH, 'outer);
                        }
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        st.recursive = Some(live);
                        {
                            let max = store.frames[fslot].rec_saved_max as usize;
                            let frame = store.frame_ref(live);
                            let save: &[i32] = if frame.rec_scratch {
                                &frame.stacksave[..max]
                            } else {
                                &frame.rec_spill[..max]
                            };
                            st.ovec_mut()[..max].copy_from_slice(save);
                        }
                        let callpat = store.frames[fslot].callpat;
                        let callpat2 = callpat + get2(code, callpat + 1) as usize;
                        store.frames[fslot].callpat = callpat2;
                        if code[callpat2] == op::ALT {
                            let fr = store.frame_ref(live);
                            let (fe, ft, fi2, fb) = (fr.eptr, fr.offset_top, fr.ims, fr.eptrb);
                            let callpat3 = store.frames[fslot].callpat;
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe, callpat3 + 1 + 2, ft,
                                fi2, fb, FLAG_ISGROUP, 8, 'outer, 'run
                            );
                        }
                        let prev = store.frames[fslot].rec_prev;
                        st.recursive = prev;
                        if !store.frames[fslot].rec_scratch {
                            let id = store.frames[fslot].rec_scratch_base as u32;
                            host.free(store.heap, id);
                        }
                        rreturn!(store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer);
                    }
                    9 => {
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        if rrc == MATCH_MATCH {

                        } else if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        } else {
                            let ecode = store.frames[fslot].ecode;
                            let ecode2 = ecode + get2(code, ecode + 1) as usize;
                            store.frames[fslot].ecode = ecode2;
                            if code[ecode2] == op::ALT {
                                let fr = store.frame_ref(live);
                                let (fe, fc, ft, fi2, fb) =
                                    (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                rmatch!(
                                    store, host, status, cur, st, rrc, entry,
                                    fe, fc + 1 + 2, ft, fi2, fb, FLAG_ISGROUP,
                                    9, 'outer, 'run
                                );
                            }
                        }
                        let mut ecode = store.frames[fslot].ecode;
                        if code[ecode] != op::ONCE && code[ecode] != op::ALT {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        loop {
                            ecode += get2(code, ecode + 1) as usize;
                            if code[ecode] != op::ALT {
                                break;
                            }
                        }
                        let top = st.end_offset_top;
                        let end = st.end_match;
                        store.frames[fslot].offset_top = top;
                        store.frames[fslot].eptr = end;
                        store.frames[fslot].ecode = ecode;
                        if code[ecode] == op::KET || end == store.frames[fslot].saved_eptr {
                            store.frames[fslot].ecode = ecode + 1 + 2;
                            skip_head = true;
                        } else {
                            if code[ecode + 1 + 2] == op::OPT {
                                let ims = store.frames[fslot].ims;
                                store.frames[fslot].ims = (ims & !7) | code[ecode + 4] as u32;
                            }
                            if code[ecode] == op::KETRMIN {
                                let fr = store.frame_ref(live);
                                let (fe, fc, ft, fi2, fb) =
                                    (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                rmatch!(
                                    store, host, status, cur, st, rrc, entry,
                                    fe, fc + 1 + 2, ft, fi2, fb, 0, 10,
                                    'outer, 'run
                                );
                            } else {
                                let fr = store.frame_ref(live);
                                let (fe, ft, fi2, fb) = (fr.eptr, fr.offset_top, fr.ims, fr.eptrb);
                                let prev = store.frames[fslot].prev_code;
                                rmatch!(
                                    store, host, status, cur, st, rrc, entry,
                                    fe, prev, ft, fi2, fb, FLAG_ISGROUP, 12,
                                    'outer, 'run
                                );
                            }
                        }
                    }
                    10 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fr = store.frame_ref(live);
                        let (fe, ft, fi2, fb) = (fr.eptr, fr.offset_top, fr.ims, fr.eptrb);
                        let fslot = store.slot(live).expect("match frame slot");
                        let prev = store.frames[fslot].prev_code;
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, prev,
                            ft, fi2, fb, FLAG_ISGROUP, 11, 'outer, 'run
                        );
                    }
                    11 => {
                        rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                    }
                    12 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe,
                            fc + 1 + 2, ft, fi2, fb, 0, 13, 'outer, 'run
                        );
                    }
                    13 => {
                        rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                    }
                    14 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let mut next = store.frames[fslot].next;
                        loop {
                            next += get2(code, next + 1) as usize;
                            if code[next] != op::ALT {
                                break;
                            }
                        }
                        store.frames[fslot].next = next;
                        store.frames[fslot].ecode = next + 1 + 2;
                        skip_head = true;
                    }
                    15 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                        skip_head = true;
                    }
                    16 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fr = store.frame_ref(live);
                        let (fe, ft, fi2, fb) = (fr.eptr, fr.offset_top, fr.ims, fr.eptrb);
                        let fslot = store.slot(live).expect("match frame slot");
                        let prev = store.frames[fslot].prev_code;
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, prev,
                            ft, fi2, fb, FLAG_ISGROUP, 17, 'outer, 'run
                        );
                    }
                    17 => {
                        rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                    }
                    18 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe,
                            fc + 1 + 2, ft, fi2, fb, 0, 19, 'outer, 'run
                        );
                    }
                    19 => {
                        rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                    }
                    20 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let fr = store.frame_ref(live);
                        let (offset, eptr, length, fi, max) =
                            (fr.offset, fr.eptr, fr.length, fr.fi, fr.max);
                        let ims = fr.ims;
                        let ss = st.subj_start;
                        let se = st.subj_end;
                        let lcc = st.lcc;
                        if fi >= max
                            || !match_ref(
                                offset,
                                eptr,
                                length,
                                st.ovec(),
                                ss,
                                se,
                                subject_full,
                                lcc,
                                ims,
                            )
                        {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].eptr = eptr + length as i64;
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 20, 'outer, 'run
                        );
                    }
                    21 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr - store.frames[fslot].length as i64;
                        store.frames[fslot].eptr = eptr;
                        if eptr >= store.frames[fslot].pp {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                fc, ft, fi2, fb, 0, 21, 'outer, 'run
                            );
                        }
                        rreturn!(
                            store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer
                        );
                    }
                    22 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        let mut eptr = store.frames[fslot].eptr;
                        if fi >= max || eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        c = getchar_inc(subject_full, &mut eptr, tables);
                        store.frames[fslot].eptr = eptr;
                        let op_now = store.frames[fslot].op;
                        let data = store.frames[fslot].data;
                        if c > 255 {
                            if op_now == op::CLASS as i32 {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                        } else if ((code[data + c as usize / 8] as i32) & (1i32 << (c & 7))) == 0 {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 22, 'outer, 'run
                        );
                    }
                    23 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        let eptr = store.frames[fslot].eptr;
                        if fi >= max || eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        c = subject_full[eptr as usize] as i32;
                        store.frames[fslot].eptr = eptr + 1;
                        let data = store.frames[fslot].data;
                        if ((code[data + c as usize / 8] as i32) & (1i32 << (c & 7))) == 0 {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 23, 'outer, 'run
                        );
                    }
                    24 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr;
                        if eptr == store.frames[fslot].pp {
                            store.frames[fslot].eptr = eptr - 1;
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let mut back = eptr - 1;
                        back_char(subject_full, &mut back);
                        store.frames[fslot].eptr = back;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 24, 'outer, 'run
                        );
                    }
                    25 => {
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr - 1;
                        store.frames[fslot].eptr = eptr;
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        if eptr >= store.frames[fslot].pp {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                fc, ft, fi2, fb, 0, 25, 'outer, 'run
                            );
                        }
                        rreturn!(
                            store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer
                        );
                    }
                    26 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        let mut eptr = store.frames[fslot].eptr;
                        if fi >= max || eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        c = getchar_inc(subject_full, &mut eptr, tables);
                        store.frames[fslot].eptr = eptr;
                        let data = store.frames[fslot].data;
                        if !xclass_match(c, &code[data..], tables).expect("xclass data in program")
                        {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 26, 'outer, 'run
                        );
                    }
                    27 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr;
                        if eptr == store.frames[fslot].pp {
                            store.frames[fslot].eptr = eptr - 1;
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let mut back = eptr - 1;
                        back_char(subject_full, &mut back);
                        store.frames[fslot].eptr = back;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 27, 'outer, 'run
                        );
                    }
                    28 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        let eptr = store.frames[fslot].eptr;
                        if fi >= max || eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let length = store.frames[fslot].length as usize;
                        let charptr = store.frames[fslot].charptr;
                        let a = eptr as usize;

                        if subject_full[a..a + length] == code[charptr..charptr + length] {
                            store.frames[fslot].eptr = eptr + length as i64;
                        } else {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 28, 'outer, 'run
                        );
                    }
                    29 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr - store.frames[fslot].length as i64;
                        store.frames[fslot].eptr = eptr;
                        if eptr >= store.frames[fslot].pp {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                fc, ft, fi2, fb, 0, 29, 'outer, 'run
                            );
                        }
                        rreturn!(
                            store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer
                        );
                    }
                    30 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        let eptr = store.frames[fslot].eptr;
                        if fi >= max || eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let b = subject_full[eptr as usize];
                        store.frames[fslot].eptr = eptr + 1;
                        if store.frames[fslot].fc != st.lcc[b as usize] as i32 {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 30, 'outer, 'run
                        );
                    }
                    31 => {
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr - 1;
                        store.frames[fslot].eptr = eptr;
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        if eptr >= store.frames[fslot].pp {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                fc, ft, fi2, fb, 0, 31, 'outer, 'run
                            );
                        }
                        rreturn!(
                            store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer
                        );
                    }
                    32 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        let eptr = store.frames[fslot].eptr;
                        if fi >= max || eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let b = subject_full[eptr as usize] as i32;
                        store.frames[fslot].eptr = eptr + 1;
                        if store.frames[fslot].fc != b {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 32, 'outer, 'run
                        );
                    }
                    33 => {
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr - 1;
                        store.frames[fslot].eptr = eptr;
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        if eptr >= store.frames[fslot].pp {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                fc, ft, fi2, fb, 0, 33, 'outer, 'run
                            );
                        }
                        rreturn!(
                            store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer
                        );
                    }
                    34 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let mut eptr = store.frames[fslot].eptr;
                        let mut d = getchar_inc(subject_full, &mut eptr, tables);
                        store.frames[fslot].eptr = eptr;
                        if d < 256 {
                            d = st.lcc[d as usize] as i32;
                        }
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        if fi >= max || eptr >= st.subj_end || store.frames[fslot].fc == d {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 34, 'outer, 'run
                        );
                    }
                    35 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        let eptr = store.frames[fslot].eptr;
                        if fi >= max || eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let b = subject_full[eptr as usize];
                        store.frames[fslot].eptr = eptr + 1;
                        if store.frames[fslot].fc == st.lcc[b as usize] as i32 {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 35, 'outer, 'run
                        );
                    }
                    36 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr;
                        if eptr == store.frames[fslot].pp {
                            store.frames[fslot].eptr = eptr - 1;
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let mut back = eptr - 1;
                        back_char(subject_full, &mut back);
                        store.frames[fslot].eptr = back;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 36, 'outer, 'run
                        );
                    }
                    37 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr - 1;
                        store.frames[fslot].eptr = eptr;
                        if eptr >= store.frames[fslot].pp {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                fc, ft, fi2, fb, 0, 37, 'outer, 'run
                            );
                        }
                        rreturn!(
                            store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer
                        );
                    }
                    38 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let mut eptr = store.frames[fslot].eptr;
                        let d = getchar_inc(subject_full, &mut eptr, tables);
                        store.frames[fslot].eptr = eptr;
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        if fi >= max || eptr >= st.subj_end || store.frames[fslot].fc == d {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 38, 'outer, 'run
                        );
                    }
                    39 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        let eptr = store.frames[fslot].eptr;
                        if fi >= max || eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let b = subject_full[eptr as usize] as i32;
                        store.frames[fslot].eptr = eptr + 1;
                        if store.frames[fslot].fc == b {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 39, 'outer, 'run
                        );
                    }
                    40 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr;
                        if eptr == store.frames[fslot].pp {
                            store.frames[fslot].eptr = eptr - 1;
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let mut back = eptr - 1;
                        back_char(subject_full, &mut back);
                        store.frames[fslot].eptr = back;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 40, 'outer, 'run
                        );
                    }
                    41 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr - 1;
                        store.frames[fslot].eptr = eptr;
                        if eptr >= store.frames[fslot].pp {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                fc, ft, fi2, fb, 0, 41, 'outer, 'run
                            );
                        }
                        rreturn!(
                            store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer
                        );
                    }
                    44 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        let mut eptr = store.frames[fslot].eptr;
                        if fi >= max || eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        c = getchar_inc(subject_full, &mut eptr, tables);
                        store.frames[fslot].eptr = eptr;
                        let ctype = store.frames[fslot].ctype;
                        let ims = store.frames[fslot].ims;
                        let fail = match ctype as u8 {
                            op::ANY => ims & pcre::DOTALL == 0 && c == 10,
                            op::ANYBYTE => false,
                            op::NOT_DIGIT => c < 256 && st.ctypes[c as usize] & 0x04 != 0,
                            op::DIGIT => c >= 256 || st.ctypes[c as usize] & 0x04 == 0,
                            op::NOT_WHITESPACE => c < 256 && st.ctypes[c as usize] & 0x01 != 0,
                            op::WHITESPACE => c >= 256 || st.ctypes[c as usize] & 0x01 == 0,
                            op::NOT_WORDCHAR => c < 256 && st.ctypes[c as usize] & 0x10 != 0,
                            op::WORDCHAR => {
                                if c < 256 {

                                    false
                                } else {
                                    let idx = (c - 256) as usize;
                                    (tables.ctypes_overspill.get(idx).copied().unwrap_or(0) & 0x10)
                                        == 0
                                }
                            }
                            _ => {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_INTERNAL,
                                    'outer
                                );
                            }
                        };
                        if fail {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 44, 'outer, 'run
                        );
                    }
                    45 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let fi = store.frames[fslot].fi;
                        let max = store.frames[fslot].max;
                        let eptr = store.frames[fslot].eptr;
                        if fi >= max || eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        c = subject_full[eptr as usize] as i32;
                        store.frames[fslot].eptr = eptr + 1;
                        let ctype = store.frames[fslot].ctype;
                        let ims = store.frames[fslot].ims;
                        let fail = match ctype as u8 {
                            op::ANY => ims & pcre::DOTALL == 0 && c == 10,
                            op::ANYBYTE => false,
                            op::NOT_DIGIT => st.ctypes[c as usize] & 0x04 != 0,
                            op::DIGIT => st.ctypes[c as usize] & 0x04 == 0,
                            op::NOT_WHITESPACE => st.ctypes[c as usize] & 0x01 != 0,
                            op::WHITESPACE => st.ctypes[c as usize] & 0x01 == 0,
                            op::NOT_WORDCHAR => st.ctypes[c as usize] & 0x10 != 0,
                            op::WORDCHAR => st.ctypes[c as usize] & 0x10 == 0,
                            _ => {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_INTERNAL,
                                    'outer
                                );
                            }
                        };
                        if fail {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        store.frames[fslot].fi = fi + 1;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 45, 'outer, 'run
                        );
                    }
                    48 => {
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr;
                        if eptr == store.frames[fslot].pp {
                            store.frames[fslot].eptr = eptr - 1;
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let mut back = eptr - 1;
                        back_char(subject_full, &mut back);
                        store.frames[fslot].eptr = back;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc,
                            ft, fi2, fb, 0, 48, 'outer, 'run
                        );
                    }
                    49 => {
                        let live = cur.expect("match dispatch without frame");
                        let fslot = store.slot(live).expect("match frame slot");
                        let eptr = store.frames[fslot].eptr - 1;
                        store.frames[fslot].eptr = eptr;
                        if rrc != MATCH_NOMATCH {
                            rreturn!(store, cur, st, rrc, entry, rrc, 'outer);
                        }
                        if eptr >= store.frames[fslot].pp {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                fc, ft, fi2, fb, 0, 49, 'outer, 'run
                            );
                        }
                        rreturn!(
                            store, cur, st, rrc, entry, MATCH_NOMATCH, 'outer
                        );
                    }
                    _ => match store.alloc(entry, host) {
                        Ok(next) => {
                            status = 0;
                            let frame = store.frame_mut(next);
                            frame.eptr = init_eptr;
                            frame.prev = None;
                            frame.ecode = init_ecode;
                            frame.offset_top = init_top;
                            frame.ims = init_ims;
                            frame.eptrb = init_blocks;
                            frame.flags = init_flags as u32;
                            cur = Some(next);
                        }
                        Err(_) => {
                            return MATCH_NOMEMORY;
                        }
                    },
                }
            }
            if !skip_head {
                {
                    let count = st.call_count;
                    st.call_count = count.wrapping_add(1);
                    if count >= st.match_limit {
                        rreturn!(store, cur, st, rrc, entry, MATCH_MATCHLIMIT, 'outer);
                    }
                }
                {
                    let live = cur.expect("match frame");
                    let fslot = store.slot(live).expect("match frame slot");
                    let ims_now = store.frames[fslot].ims;
                    store.frames[fslot].original_ims = ims_now;
                    if store.frames[fslot].flags & FLAG_ISGROUP != 0 {
                        let (eb, ep) = (store.frames[fslot].eptrb, store.frames[fslot].eptr);
                        store.frames[fslot].block_prev = eb;
                        store.frames[fslot].block_eptr = ep;
                        store.frames[fslot].eptrb = Some(live);
                    }
                }
            }
            skip_head = false;
            'ops: loop {
                let live = cur.expect("match op without frame");
                let fslot = store.slot(live).expect("match frame slot");
                {
                    let eptr_now = store.frames[fslot].eptr;
                    if st.partial && eptr_now >= st.subj_end && eptr_now > st.start_match {
                        st.hitend = true;
                    }
                }
                let ecode_now = store.frames[fslot].ecode;
                let raw = code[ecode_now];
                store.frames[fslot].op = raw as i32;
                store.frames[fslot].minimize = 0;
                let mut op = raw;
                if op > op::BRA {
                    let mut number = op.wrapping_sub(op::BRA) as i32;
                    if number > 100 {
                        number = get2(code, ecode_now + 2 + 2);
                    }
                    let offset = number << 1;
                    store.frames[fslot].number = number;
                    store.frames[fslot].offset = offset;
                    if offset < st.offset_max {
                        let save1 = st.ovec()[offset as usize];
                        let save2 = st.ovec()[offset as usize + 1];
                        let save3 = st.ovec()[(st.offset_end - number) as usize];
                        let save_cap = st.capture_last;
                        store.frames[fslot].save_offset1 = save1;
                        store.frames[fslot].save_offset2 = save2;
                        store.frames[fslot].save_offset3 = save3;
                        store.frames[fslot].save_capture_last = save_cap;
                        let at = (st.offset_end - number) as usize;
                        let eptr_here = store.frames[fslot].eptr;
                        st.ovec_mut()[at] = (eptr_here - st.subj_start) as i32;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, fc + 1 + 2, ft, fi2,
                            fb, FLAG_ISGROUP, 1, 'outer, 'run
                        );
                    } else {
                        op = op::BRA;
                        store.frames[fslot].op = op as i32;
                    }
                }
                match op {
                    op::BRA => {
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe,
                            fc + 1 + 2, ft, fi2, fb, FLAG_ISGROUP, 2, 'outer,
                            'run
                        );
                    }
                    op::COND => {
                        let ecode = store.frames[fslot].ecode;
                        if code[ecode + 2 + 1] == op::CREF {
                            let offset = get2(code, ecode + 2 + 2) << 1;
                            store.frames[fslot].offset = offset;
                            let cond = if offset == 0xffff * 2 {
                                st.recursive.is_some()
                            } else {
                                offset < store.frames[fslot].offset_top
                                    && st.ovec()[offset as usize] >= 0
                            };
                            store.frames[fslot].condition = cond as i32;
                            let dest = if cond {
                                ecode + (2 + 4)
                            } else {
                                ecode + (2 + 1 + get2(code, ecode + 1) as usize)
                            };
                            let fr = store.frame_ref(live);
                            let (fe, ft, fi2, fb) = (fr.eptr, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                dest, ft, fi2, fb, FLAG_ISGROUP, 3, 'outer,
                                'run
                            );
                        } else {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2) = (fr.eptr, fr.ecode, fr.offset_top, fr.ims);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                fc + 1 + 2, ft, fi2, None,
                                FLAG_CONDATTR | FLAG_ISGROUP, 4, 'outer, 'run
                            );
                        }
                    }
                    op::CREF | op::BRANUMBER => {
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 3;
                    }
                    op::END => {
                        let mut restored = false;
                        if let Some(rec) = st.recursive {
                            if store.frame_ref(rec).rec_group == 0 {
                                let rf = store.frame_ref(rec);
                                let (prev, max, start, after) = (
                                    rf.rec_prev,
                                    rf.rec_saved_max as usize,
                                    rf.rec_start,
                                    rf.rec_after,
                                );
                                let scratch = rf.rec_scratch;
                                st.recursive = prev;
                                if scratch {
                                    let src = store.frame_ref(rec).stacksave;
                                    st.ovec_mut()[..max].copy_from_slice(&src[..max]);
                                } else {
                                    let spill = store.frame_ref(rec).rec_spill;
                                    st.ovec_mut()[..max].copy_from_slice(&spill[..max]);
                                }
                                st.start_match = start;
                                let orig = store.frames[fslot].original_ims;
                                store.frames[fslot].ims = orig;
                                store.frames[fslot].ecode = after;
                                restored = true;
                            }
                        }
                        if !restored {
                            let eptr = store.frames[fslot].eptr;
                            if st.notempty && eptr == st.start_match {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            st.end_match = eptr;
                            st.end_offset_top = store.frames[fslot].offset_top;
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_MATCH, 'outer
                            );
                        }
                    }
                    op::OPT => {
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ims = code[ecode + 1] as u32;
                        store.frames[fslot].ecode = ecode + 2;
                    }
                    op::ASSERT | op::ASSERTBACK => {
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2) = (fr.eptr, fr.ecode, fr.offset_top, fr.ims);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe,
                            fc + 1 + 2, ft, fi2, None, FLAG_ISGROUP, 6,
                            'outer, 'run
                        );
                    }
                    op::ASSERT_NOT | op::ASSERTBACK_NOT => {
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2) = (fr.eptr, fr.ecode, fr.offset_top, fr.ims);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe,
                            fc + 1 + 2, ft, fi2, None, FLAG_ISGROUP, 7,
                            'outer, 'run
                        );
                    }
                    op::REVERSE => {
                        if st.utf8 {
                            let ecode = store.frames[fslot].ecode;
                            c = get2(code, ecode + 1);
                            i = 0;
                            while i < c {
                                let eptr = store.frames[fslot].eptr - 1;
                                store.frames[fslot].eptr = eptr;
                                if eptr < st.subj_start {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                let mut back = eptr;
                                back_char(subject_full, &mut back);
                                store.frames[fslot].eptr = back;
                                i += 1;
                            }
                        } else {
                            let ecode = store.frames[fslot].ecode;
                            let eptr = store.frames[fslot].eptr - get2(code, ecode + 1) as i64;
                            store.frames[fslot].eptr = eptr;
                            if eptr < st.subj_start {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1 + 2;
                    }
                    op::CALLOUT => {
                        let ecode = store.frames[fslot].ecode;
                        let eptr = store.frames[fslot].eptr;
                        let top = store.frames[fslot].offset_top;
                        let subj = &subject_full[st.subj_start as usize..st.subj_end as usize];
                        let block = CalloutBlock {
                            version: 1,
                            number: code[ecode + 1] as i32,
                            offsets: st.ovec(),
                            subject: subj,
                            start_match: (st.start_match - st.subj_start) as i32,
                            current_position: (eptr - st.subj_start) as i32,
                            capture_top: top / 2,
                            capture_last: st.capture_last,
                            callout_data,
                            pattern_position: get2(code, ecode + 2),
                            next_item_length: get2(code, ecode + 2 + 2),
                        };
                        if let Some(cb) = callout {
                            rrc = cb(&block);
                        } else {
                            rrc = host.guest_callout(&block);
                        }
                        if rrc > 0 {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        if rrc < 0 {
                            rreturn!(
                                store, cur, st, rrc, entry, rrc, 'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 2 + 2 * 2;
                    }
                    op::RECURSE => {
                        let ecode = store.frames[fslot].ecode;
                        let callpat = get2(code, ecode + 1) as usize;
                        store.frames[fslot].callpat = callpat;
                        let mut group = code[callpat] as i32 - op::BRA as i32;
                        if group > 100 {
                            group = get2(code, callpat + 2 + 2);
                        }
                        store.frames[fslot].rec_group = group;
                        store.frames[fslot].rec_prev = st.recursive;
                        st.recursive = Some(live);
                        let ecode2 = ecode + 1 + 2;
                        store.frames[fslot].ecode = ecode2;
                        store.frames[fslot].rec_after = ecode2;
                        let max = st.offset_end;
                        store.frames[fslot].rec_saved_max = max;
                        if max <= 30 {
                            store.frames[fslot].rec_scratch = true;
                            let maxu = max as usize;
                            let from = st.ovec();
                            let to = &mut store.frames[fslot].stacksave[..maxu];
                            to.copy_from_slice(&from[..maxu]);
                        } else {
                            if max as usize > 1024 {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMEMORY,
                                    'outer
                                );
                            }
                            let bytes = max as u32 * 4;
                            match host.alloc(store.heap, 1, bytes) {
                                Some(id) => {
                                    store.frames[fslot].rec_scratch = false;
                                    store.frames[fslot].rec_scratch_base = id as usize;
                                    let maxu = max as usize;
                                    let from = st.ovec();
                                    let to = &mut store.frames[fslot].rec_spill[..maxu];
                                    to.copy_from_slice(&from[..maxu]);
                                }
                                None => {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMEMORY, 'outer
                                    );
                                }
                            }
                        }
                        store.frames[fslot].rec_start = st.start_match;
                        st.start_match = store.frames[fslot].eptr;
                        let fr = store.frame_ref(live);
                        let (fe, ft, fi2, fb) = (fr.eptr, fr.offset_top, fr.ims, fr.eptrb);
                        let cp = store.frames[fslot].callpat;
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe,
                            cp + 1 + 2, ft, fi2, fb, FLAG_ISGROUP, 8, 'outer,
                            'run
                        );
                    }
                    op::ONCE => {
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].prev_code = ecode;
                        store.frames[fslot].saved_eptr = store.frames[fslot].eptr;
                        let fr = store.frame_ref(live);
                        let (fe, fc, ft, fi2, fb) =
                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe,
                            fc + 1 + 2, ft, fi2, fb, FLAG_ISGROUP, 9, 'outer,
                            'run
                        );
                    }
                    op::ALT => {
                        let mut ecode = store.frames[fslot].ecode;
                        loop {
                            ecode += get2(code, ecode + 1) as usize;
                            if code[ecode] != op::ALT {
                                break;
                            }
                        }
                        store.frames[fslot].ecode = ecode;
                    }
                    op::BRAZERO => {
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].next = ecode + 1;
                        let fr = store.frame_ref(live);
                        let (fe, ft, fi2, fb) = (fr.eptr, fr.offset_top, fr.ims, fr.eptrb);
                        let next = store.frames[fslot].next;
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe, next,
                            ft, fi2, fb, FLAG_ISGROUP, 14, 'outer, 'run
                        );
                    }
                    op::BRAMINZERO => {
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].next = ecode + 1;
                        let mut next = ecode + 1;
                        loop {
                            next += get2(code, next + 1) as usize;
                            if code[next] != op::ALT {
                                break;
                            }
                        }
                        store.frames[fslot].next = next;
                        let fr = store.frame_ref(live);
                        let (fe, ft, fi2, fb) = (fr.eptr, fr.offset_top, fr.ims, fr.eptrb);
                        rmatch!(
                            store, host, status, cur, st, rrc, entry, fe,
                            next + 1 + 2, ft, fi2, fb, FLAG_ISGROUP, 15,
                            'outer, 'run
                        );
                    }
                    op::KET | op::KETRMIN | op::KETRMAX => {
                        let ecode = store.frames[fslot].ecode;
                        let prev = ecode - get2(code, ecode + 1) as usize;
                        store.frames[fslot].prev_code = prev;
                        let owner = store.frames[fslot].eptrb.expect("ket without eptr block");
                        let oslot = store.slot(owner).expect("match frame slot");
                        let saved = store.frames[oslot].block_eptr;
                        let eprev = store.frames[oslot].block_prev;
                        store.frames[fslot].saved_eptr = saved;
                        store.frames[fslot].eptrb = eprev;
                        if matches!(
                            code[prev],
                            op::ASSERT
                                | op::ASSERT_NOT
                                | op::ASSERTBACK
                                | op::ASSERTBACK_NOT
                                | op::ONCE
                        ) {
                            st.end_match = store.frames[fslot].eptr;
                            st.end_offset_top = store.frames[fslot].offset_top;
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_MATCH, 'outer
                            );
                        }
                        if code[prev] != op::COND {
                            let mut number = code[prev] as i32 - op::BRA as i32;
                            if number > 100 {
                                number = get2(code, prev + 2 + 2);
                            }
                            let offset = number << 1;
                            store.frames[fslot].number = number;
                            store.frames[fslot].offset = offset;
                            if number > 0 {
                                st.capture_last = number;
                                if offset >= st.offset_max {
                                    st.offset_overflow = true;
                                } else {
                                    let at = (st.offset_end - number) as usize;
                                    let open = st.ovec()[at];
                                    st.ovec_mut()[offset as usize] = open;
                                    let eptr = store.frames[fslot].eptr;
                                    st.ovec_mut()[offset as usize + 1] =
                                        (eptr - st.subj_start) as i32;
                                    let top = store.frames[fslot].offset_top;
                                    if top <= offset {
                                        store.frames[fslot].offset_top = offset + 2;
                                    }
                                }
                                if let Some(rec) = st.recursive {
                                    if store.frame_ref(rec).rec_group == number {
                                        let rf = store.frame_ref(rec);
                                        let (rprev, max, start, after) = (
                                            rf.rec_prev,
                                            rf.rec_saved_max as usize,
                                            rf.rec_start,
                                            rf.rec_after,
                                        );
                                        let scratch = rf.rec_scratch;
                                        st.recursive = rprev;
                                        if scratch {
                                            let src = store.frame_ref(rec).stacksave;
                                            st.ovec_mut()[..max].copy_from_slice(&src[..max]);
                                        } else {
                                            let spill = store.frame_ref(rec).rec_spill;
                                            st.ovec_mut()[..max].copy_from_slice(&spill[..max]);
                                        }
                                        st.start_match = start;
                                        store.frames[fslot].ecode = after;
                                        let orig = store.frames[fslot].original_ims;
                                        store.frames[fslot].ims = orig;

                                        continue;
                                    }
                                }
                            }
                        }
                        let orig = store.frames[fslot].original_ims;
                        store.frames[fslot].ims = orig;
                        let ecode = store.frames[fslot].ecode;
                        let eptr = store.frames[fslot].eptr;
                        if code[ecode] == op::KET || eptr == store.frames[fslot].saved_eptr {
                            store.frames[fslot].ecode = ecode + 1 + 2;
                        } else if code[ecode] == op::KETRMIN {
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                fc + 1 + 2, ft, fi2, fb, 0, 16, 'outer, 'run
                            );
                        } else {
                            let fr = store.frame_ref(live);
                            let (fe, ft, fi2, fb) = (fr.eptr, fr.offset_top, fr.ims, fr.eptrb);
                            let prev = store.frames[fslot].prev_code;
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                prev, ft, fi2, fb, FLAG_ISGROUP, 18, 'outer,
                                'run
                            );
                        }
                    }
                    op::CIRC => {
                        let eptr = store.frames[fslot].eptr;
                        let ims = store.frames[fslot].ims;
                        if st.notbol && eptr == st.subj_start {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        if ims & pcre::MULTILINE != 0 {
                            if eptr != st.subj_start && subject_full[(eptr - 1) as usize] != 10 {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            let ecode = store.frames[fslot].ecode;
                            store.frames[fslot].ecode = ecode + 1;
                        } else {

                            if eptr != st.subj_start {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            let ecode = store.frames[fslot].ecode;
                            store.frames[fslot].ecode = ecode + 1;
                        }
                    }
                    op::SOD => {
                        if store.frames[fslot].eptr != st.subj_start {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::SOM => {
                        let at = st.subj_start + st.start_offset as i64;
                        if store.frames[fslot].eptr != at {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::DOLL => {
                        let eptr = store.frames[fslot].eptr;
                        let ims = store.frames[fslot].ims;
                        if ims & pcre::MULTILINE != 0 {
                            if eptr < st.subj_end {
                                if subject_full[eptr as usize] != 10 {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                            } else if st.noteol {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            let ecode = store.frames[fslot].ecode;
                            store.frames[fslot].ecode = ecode + 1;
                        } else {
                            if st.noteol {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            if !st.endonly {
                                if eptr < st.subj_end - 1
                                    || (eptr == st.subj_end - 1
                                        && subject_full[eptr as usize] != 10)
                                {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                let ecode = store.frames[fslot].ecode;
                                store.frames[fslot].ecode = ecode + 1;
                            } else {

                                if eptr < st.subj_end {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                let ecode = store.frames[fslot].ecode;
                                store.frames[fslot].ecode = ecode + 1;
                            }
                        }
                    }
                    op::EOD => {
                        if store.frames[fslot].eptr < st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::EODN => {
                        let eptr = store.frames[fslot].eptr;
                        if eptr < st.subj_end - 1
                            || (eptr == st.subj_end - 1 && subject_full[eptr as usize] != 10)
                        {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::NOT_WORD_BOUNDARY | op::WORD_BOUNDARY => {
                        let eptr = store.frames[fslot].eptr;
                        if st.utf8 {
                            if eptr == st.subj_start {
                                store.frames[fslot].prev_is_word = 0;
                            } else {
                                let mut last = eptr - 1;
                                while subject_full[last as usize] & 0xc0 == 0x80 {
                                    last -= 1;
                                }
                                c = getchar_at(subject_full, last, tables);
                                let word = c < 256 && st.ctypes[c as usize] & 0x10 != 0;
                                store.frames[fslot].prev_is_word = word as i32;
                            }
                            if eptr >= st.subj_end {
                                store.frames[fslot].cur_is_word = 0;
                            } else {
                                c = getchar_at(subject_full, eptr, tables);
                                let word = c < 256 && st.ctypes[c as usize] & 0x10 != 0;
                                store.frames[fslot].cur_is_word = word as i32;
                            }
                        } else {
                            let prev = eptr != st.subj_start
                                && st.ctypes[subject_full[(eptr - 1) as usize] as usize] & 0x10
                                    != 0;
                            let curw = eptr < st.subj_end
                                && st.ctypes[subject_full[eptr as usize] as usize] & 0x10 != 0;
                            store.frames[fslot].prev_is_word = prev as i32;
                            store.frames[fslot].cur_is_word = curw as i32;
                        }
                        let ecode = store.frames[fslot].ecode;
                        let is_boundary = code[ecode] == op::WORD_BOUNDARY;
                        store.frames[fslot].ecode = ecode + 1;
                        let curw = store.frames[fslot].cur_is_word;
                        let prev = store.frames[fslot].prev_is_word;
                        if is_boundary {
                            if curw == prev {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                        } else if curw != prev {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                    }
                    op::ANY => {
                        let mut eptr = store.frames[fslot].eptr;
                        let ims = store.frames[fslot].ims;
                        if ims & pcre::DOTALL == 0
                            && eptr < st.subj_end
                            && subject_full[eptr as usize] == 10
                        {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let at = eptr;
                        eptr += 1;
                        store.frames[fslot].eptr = eptr;
                        if at >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        if st.utf8 {
                            while eptr < st.subj_end && subject_full[eptr as usize] & 0xc0 == 0x80 {
                                eptr += 1;
                            }
                            store.frames[fslot].eptr = eptr;
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::ANYBYTE => {
                        let eptr = store.frames[fslot].eptr;
                        store.frames[fslot].eptr = eptr + 1;
                        if eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::NOT_DIGIT => {
                        let mut eptr = store.frames[fslot].eptr;
                        if eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        c = getchar_inc_test(subject_full, &mut eptr, st.utf8, tables);
                        store.frames[fslot].eptr = eptr;
                        if c < 256 && st.ctypes[c as usize] & 0x04 != 0 {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::DIGIT => {
                        let mut eptr = store.frames[fslot].eptr;
                        if eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        c = getchar_inc_test(subject_full, &mut eptr, st.utf8, tables);
                        store.frames[fslot].eptr = eptr;
                        if c >= 256 || st.ctypes[c as usize] & 0x04 == 0 {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::NOT_WHITESPACE => {
                        let mut eptr = store.frames[fslot].eptr;
                        if eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        c = getchar_inc_test(subject_full, &mut eptr, st.utf8, tables);
                        store.frames[fslot].eptr = eptr;
                        if c < 256 && st.ctypes[c as usize] & 0x01 != 0 {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::WHITESPACE => {
                        let mut eptr = store.frames[fslot].eptr;
                        if eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        c = getchar_inc_test(subject_full, &mut eptr, st.utf8, tables);
                        store.frames[fslot].eptr = eptr;
                        if c >= 256 || st.ctypes[c as usize] & 0x01 == 0 {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::NOT_WORDCHAR => {
                        let mut eptr = store.frames[fslot].eptr;
                        if eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        c = getchar_inc_test(subject_full, &mut eptr, st.utf8, tables);
                        store.frames[fslot].eptr = eptr;
                        if c < 256 && st.ctypes[c as usize] & 0x10 != 0 {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::WORDCHAR => {
                        let mut eptr = store.frames[fslot].eptr;
                        if eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        c = getchar_inc_test(subject_full, &mut eptr, st.utf8, tables);
                        store.frames[fslot].eptr = eptr;
                        if c >= 256 || st.ctypes[c as usize] & 0x10 == 0 {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ecode = ecode + 1;
                    }
                    op::REF => {
                        let ecode = store.frames[fslot].ecode;
                        let offset = get2(code, ecode + 1) << 1;
                        store.frames[fslot].offset = offset;
                        store.frames[fslot].ecode = ecode + 3;
                        let top = store.frames[fslot].offset_top;
                        let eptr0 = store.frames[fslot].eptr;
                        let length = if offset >= top || st.ovec()[offset as usize] < 0 {
                            (st.subj_end - eptr0 + 1) as i32
                        } else {
                            st.ovec()[offset as usize + 1] - st.ovec()[offset as usize]
                        };
                        store.frames[fslot].length = length;
                        let ecode = store.frames[fslot].ecode;
                        match code[ecode] {
                            op::CRSTAR
                            | op::CRMINSTAR
                            | op::CRPLUS
                            | op::CRMINPLUS
                            | op::CRQUERY
                            | op::CRMINQUERY => {
                                c = code[ecode] as i32 - op::CRSTAR as i32;
                                store.frames[fslot].ecode = ecode + 1;
                                store.frames[fslot].minimize = (c & 1) as i32;
                                store.frames[fslot].min = tables.rep_min[c as usize] as i32;
                                let mut max = tables.rep_max[c as usize] as i32;
                                if max == 0 {
                                    max = 0x7fff_ffff;
                                }
                                store.frames[fslot].max = max;
                            }
                            op::CRRANGE | op::CRMINRANGE => {
                                let min_upto = code[ecode] == op::CRMINRANGE;
                                store.frames[fslot].minimize = min_upto as i32;
                                store.frames[fslot].min = get2(code, ecode + 1);
                                let mut max = get2(code, ecode + 3);
                                if max == 0 {
                                    max = 0x7fff_ffff;
                                }
                                store.frames[fslot].max = max;
                                store.frames[fslot].ecode = ecode + 5;
                            }
                            _ => {
                                let fr = store.frame_ref(live);
                                let ss = st.subj_start;
                                let se = st.subj_end;
                                let lcc = st.lcc;
                                let ok = match_ref(
                                    fr.offset,
                                    fr.eptr,
                                    fr.length,
                                    st.ovec(),
                                    ss,
                                    se,
                                    subject_full,
                                    lcc,
                                    fr.ims,
                                );
                                if !ok {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                let eptr =
                                    store.frames[fslot].eptr + store.frames[fslot].length as i64;
                                store.frames[fslot].eptr = eptr;
                                continue 'ops;
                            }
                        }
                        if store.frames[fslot].length == 0 {

                        } else {
                            let min = store.frames[fslot].min;
                            i = 1;
                            while i <= min {
                                let fr = store.frame_ref(live);
                                let ss = st.subj_start;
                                let se = st.subj_end;
                                let lcc = st.lcc;
                                let ok = match_ref(
                                    fr.offset,
                                    fr.eptr,
                                    fr.length,
                                    st.ovec(),
                                    ss,
                                    se,
                                    subject_full,
                                    lcc,
                                    fr.ims,
                                );
                                if !ok {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                let eptr =
                                    store.frames[fslot].eptr + store.frames[fslot].length as i64;
                                store.frames[fslot].eptr = eptr;
                                i += 1;
                            }
                            let min = store.frames[fslot].min;
                            let max = store.frames[fslot].max;
                            if min != max {
                                if store.frames[fslot].minimize != 0 {
                                    store.frames[fslot].fi = min;
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, 20,
                                        'outer, 'run
                                    );
                                } else {
                                    store.frames[fslot].pp = store.frames[fslot].eptr;
                                    i = min;
                                    while i < max {
                                        let fr = store.frame_ref(live);
                                        let ss = st.subj_start;
                                        let se = st.subj_end;
                                        let lcc = st.lcc;
                                        let ok = match_ref(
                                            fr.offset,
                                            fr.eptr,
                                            fr.length,
                                            st.ovec(),
                                            ss,
                                            se,
                                            subject_full,
                                            lcc,
                                            fr.ims,
                                        );
                                        if !ok {
                                            break;
                                        }
                                        let eptr = store.frames[fslot].eptr
                                            + store.frames[fslot].length as i64;
                                        store.frames[fslot].eptr = eptr;
                                        i += 1;
                                    }
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, 21,
                                        'outer, 'run
                                    );
                                }
                            }
                        }
                    }
                    op::NCLASS | op::CLASS => {
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].data = ecode + 1;
                        let ecode = ecode + 33;
                        store.frames[fslot].ecode = ecode;
                        match code[ecode] {
                            op::CRSTAR
                            | op::CRMINSTAR
                            | op::CRPLUS
                            | op::CRMINPLUS
                            | op::CRQUERY
                            | op::CRMINQUERY => {
                                c = code[ecode] as i32 - op::CRSTAR as i32;
                                store.frames[fslot].ecode = ecode + 1;
                                store.frames[fslot].minimize = c & 1;
                                store.frames[fslot].min = tables.rep_min[c as usize] as i32;
                                let mut max = tables.rep_max[c as usize] as i32;
                                if max == 0 {
                                    max = 0x7fff_ffff;
                                }
                                store.frames[fslot].max = max;
                            }
                            op::CRRANGE | op::CRMINRANGE => {
                                let min_upto = code[ecode] == op::CRMINRANGE;
                                store.frames[fslot].minimize = min_upto as i32;
                                store.frames[fslot].min = get2(code, ecode + 1);
                                let mut max = get2(code, ecode + 3);
                                if max == 0 {
                                    max = 0x7fff_ffff;
                                }
                                store.frames[fslot].max = max;
                                store.frames[fslot].ecode = ecode + 5;
                            }
                            _ => {
                                store.frames[fslot].min = 1;
                                store.frames[fslot].max = 1;
                            }
                        }
                        let min = store.frames[fslot].min;
                        let op_now = store.frames[fslot].op;
                        let data = store.frames[fslot].data;
                        if st.utf8 {
                            i = 1;
                            while i <= min {
                                let mut eptr = store.frames[fslot].eptr;
                                if eptr >= st.subj_end {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                c = getchar_inc(subject_full, &mut eptr, tables);
                                store.frames[fslot].eptr = eptr;
                                if c > 255 {
                                    if op_now == op::CLASS as i32 {
                                        rreturn!(
                                            store, cur, st, rrc, entry,
                                            MATCH_NOMATCH, 'outer
                                        );
                                    }
                                } else if ((code[data + c as usize / 8] as i32) & (1i32 << (c & 7)))
                                    == 0
                                {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                i += 1;
                            }
                        } else {
                            i = 1;
                            while i <= min {
                                let eptr = store.frames[fslot].eptr;
                                if eptr >= st.subj_end {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                c = subject_full[eptr as usize] as i32;
                                store.frames[fslot].eptr = eptr + 1;
                                if ((code[data + c as usize / 8] as i32) & (1i32 << (c & 7))) == 0 {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                i += 1;
                            }
                        }
                        let max = store.frames[fslot].max;
                        if min != max {
                            if store.frames[fslot].minimize != 0 {
                                store.frames[fslot].fi = min;
                                let site = if st.utf8 { 22 } else { 23 };
                                let fr = store.frame_ref(live);
                                let (fe, fc, ft, fi2, fb) =
                                    (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                rmatch!(
                                    store, host, status, cur, st, rrc, entry,
                                    fe, fc, ft, fi2, fb, 0, site, 'outer, 'run
                                );
                            } else {
                                store.frames[fslot].pp = store.frames[fslot].eptr;
                                if st.utf8 {
                                    i = min;
                                    while i < max {
                                        let mut eptr = store.frames[fslot].eptr;
                                        if eptr >= st.subj_end {
                                            break;
                                        }
                                        c = getchar_inc(subject_full, &mut eptr, tables);
                                        let hit = if c > 255 {
                                            op_now != op::CLASS as i32
                                        } else {
                                            ((code[data + c as usize / 8] as i32)
                                                & (1i32 << (c & 7)))
                                                != 0
                                        };
                                        if !hit {
                                            break;
                                        }
                                        store.frames[fslot].eptr = eptr;
                                        i += 1;
                                    }
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, 24,
                                        'outer, 'run
                                    );
                                } else {
                                    i = min;
                                    while i < max {
                                        let eptr = store.frames[fslot].eptr;
                                        if eptr >= st.subj_end {
                                            break;
                                        }
                                        c = subject_full[eptr as usize] as i32;
                                        if ((code[data + c as usize / 8] as i32)
                                            & (1i32 << (c & 7)))
                                            == 0
                                        {
                                            break;
                                        }
                                        store.frames[fslot].eptr = eptr + 1;
                                        i += 1;
                                    }
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, 25,
                                        'outer, 'run
                                    );
                                }
                            }
                        }
                    }
                    op::XCLASS => {
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].data = ecode + 1 + 2;
                        let ecode = ecode + get2(code, ecode + 1) as usize;
                        store.frames[fslot].ecode = ecode;
                        match code[ecode] {
                            op::CRSTAR
                            | op::CRMINSTAR
                            | op::CRPLUS
                            | op::CRMINPLUS
                            | op::CRQUERY
                            | op::CRMINQUERY => {
                                c = code[ecode] as i32 - op::CRSTAR as i32;
                                store.frames[fslot].ecode = ecode + 1;
                                store.frames[fslot].minimize = c & 1;
                                store.frames[fslot].min = tables.rep_min[c as usize] as i32;
                                let mut max = tables.rep_max[c as usize] as i32;
                                if max == 0 {
                                    max = 0x7fff_ffff;
                                }
                                store.frames[fslot].max = max;
                            }
                            op::CRRANGE | op::CRMINRANGE => {
                                let min_upto = code[ecode] == op::CRMINRANGE;
                                store.frames[fslot].minimize = min_upto as i32;
                                store.frames[fslot].min = get2(code, ecode + 1);
                                let mut max = get2(code, ecode + 3);
                                if max == 0 {
                                    max = 0x7fff_ffff;
                                }
                                store.frames[fslot].max = max;
                                store.frames[fslot].ecode = ecode + 5;
                            }
                            _ => {
                                store.frames[fslot].min = 1;
                                store.frames[fslot].max = 1;
                            }
                        }
                        let min = store.frames[fslot].min;
                        let data = store.frames[fslot].data;
                        i = 1;
                        while i <= min {
                            let mut eptr = store.frames[fslot].eptr;
                            if eptr >= st.subj_end {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            c = getchar_inc(subject_full, &mut eptr, tables);
                            store.frames[fslot].eptr = eptr;
                            if !xclass_match(c, &code[data..], tables)
                                .expect("xclass data in program")
                            {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            i += 1;
                        }
                        let max = store.frames[fslot].max;
                        if min != max {
                            if store.frames[fslot].minimize != 0 {
                                store.frames[fslot].fi = min;
                                let fr = store.frame_ref(live);
                                let (fe, fc, ft, fi2, fb) =
                                    (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                rmatch!(
                                    store, host, status, cur, st, rrc, entry,
                                    fe, fc, ft, fi2, fb, 0, 26, 'outer, 'run
                                );
                            } else {
                                store.frames[fslot].pp = store.frames[fslot].eptr;
                                i = min;
                                while i < max {
                                    let eptr = store.frames[fslot].eptr;
                                    if eptr >= st.subj_end {
                                        break;
                                    }
                                    let mut len = 1;
                                    c = getchar_len(subject_full, eptr, &mut len, tables);
                                    if !xclass_match(c, &code[data..], tables)
                                        .expect("xclass data in program")
                                    {
                                        break;
                                    }
                                    store.frames[fslot].eptr = eptr + len as i64;
                                    i += 1;
                                }
                                let fr = store.frame_ref(live);
                                let (fe, fc, ft, fi2, fb) =
                                    (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                rmatch!(
                                    store, host, status, cur, st, rrc, entry,
                                    fe, fc, ft, fi2, fb, 0, 27, 'outer, 'run
                                );
                            }
                        }
                    }
                    op::CHAR => {
                        if st.utf8 {
                            let mut length = 1;
                            let mut ecode = store.frames[fslot].ecode + 1;
                            let fc = getchar_len(code, ecode as i64, &mut length, tables);
                            store.frames[fslot].fc = fc;
                            store.frames[fslot].length = length;
                            let mut eptr = store.frames[fslot].eptr;
                            if length as i64 > st.subj_end - eptr {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }

                            loop {
                                let old = store.frames[fslot].length;
                                store.frames[fslot].length = old - 1;
                                if old <= 0 {
                                    break;
                                }
                                store.frames[fslot].ecode = ecode + 1;
                                store.frames[fslot].eptr = eptr + 1;
                                if code[ecode] != subject_full[eptr as usize] {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                ecode += 1;
                                eptr += 1;
                            }
                            store.frames[fslot].ecode = ecode;
                            store.frames[fslot].eptr = eptr;
                        } else {
                            let eptr = store.frames[fslot].eptr;
                            if st.subj_end - eptr < 1 {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            let ecode = store.frames[fslot].ecode;
                            store.frames[fslot].eptr = eptr + 1;
                            if code[ecode + 1] != subject_full[eptr as usize] {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            store.frames[fslot].eptr = eptr + 1;
                            store.frames[fslot].ecode = ecode + 2;
                        }
                    }
                    op::CHARNC => {
                        if st.utf8 {
                            let mut length = 1;
                            let ecode0 = store.frames[fslot].ecode + 1;
                            let fc = getchar_len(code, ecode0 as i64, &mut length, tables);
                            store.frames[fslot].fc = fc;
                            store.frames[fslot].length = length;
                            let eptr = store.frames[fslot].eptr;
                            if length as i64 > st.subj_end - eptr {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            if fc < 128 {
                                store.frames[fslot].ecode = ecode0 + 1;
                                store.frames[fslot].eptr = eptr + 1;
                                if st.lcc[code[ecode0] as usize]
                                    != st.lcc[subject_full[eptr as usize] as usize]
                                {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                store.frames[fslot].ecode = ecode0 + 1;
                                store.frames[fslot].eptr = eptr + 1;
                            } else {
                                let mut eptr2 = eptr;
                                let dc = getchar_inc(subject_full, &mut eptr2, tables);
                                store.frames[fslot].ecode = ecode0 + length as usize;
                                store.frames[fslot].eptr = eptr2;
                                if fc != dc {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                            }
                        } else {
                            let eptr = store.frames[fslot].eptr;
                            if st.subj_end - eptr < 1 {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            let ecode = store.frames[fslot].ecode;
                            store.frames[fslot].eptr = eptr + 1;
                            if st.lcc[code[ecode + 1] as usize]
                                != st.lcc[subject_full[eptr as usize] as usize]
                            {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            store.frames[fslot].eptr = eptr + 1;
                            store.frames[fslot].ecode = ecode + 2;
                        }
                    }
                    op::EXACT
                    | op::UPTO
                    | op::MINUPTO
                    | op::STAR
                    | op::MINSTAR
                    | op::PLUS
                    | op::MINPLUS
                    | op::QUERY
                    | op::MINQUERY => {
                        match op {
                            op::EXACT => {
                                let ecode = store.frames[fslot].ecode;
                                let v = get2(code, ecode + 1);
                                store.frames[fslot].min = v;
                                store.frames[fslot].max = v;
                                store.frames[fslot].ecode = ecode + 3;
                            }
                            op::UPTO | op::MINUPTO => {
                                let ecode = store.frames[fslot].ecode;
                                store.frames[fslot].min = 0;
                                store.frames[fslot].max = get2(code, ecode + 1);
                                store.frames[fslot].minimize = (op == op::MINUPTO) as i32;
                                store.frames[fslot].ecode = ecode + 3;
                            }
                            _ => {
                                let ecode = store.frames[fslot].ecode;
                                c = op as i32 - op::STAR as i32;
                                store.frames[fslot].minimize = c & 1;
                                store.frames[fslot].min = tables.rep_min[c as usize] as i32;
                                let mut max = tables.rep_max[c as usize] as i32;
                                if max == 0 {
                                    max = 0x7fff_ffff;
                                }
                                store.frames[fslot].max = max;
                                store.frames[fslot].ecode = ecode + 1;
                            }
                        }

                        if st.utf8 {
                            let mut length = 1;
                            let charptr = store.frames[fslot].ecode;
                            let fc = getchar_len(code, charptr as i64, &mut length, tables);
                            store.frames[fslot].fc = fc;
                            store.frames[fslot].length = length;
                            store.frames[fslot].charptr = charptr;
                            let min = store.frames[fslot].min;
                            if min as i64 * length as i64 > st.subj_end - store.frames[fslot].eptr {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            store.frames[fslot].ecode = charptr + length as usize;
                            if length > 1 {
                                i = 1;
                                while i <= min {
                                    let eptr = store.frames[fslot].eptr;
                                    let a = eptr as usize;
                                    let lu = length as usize;
                                    if subject_full[a..a + lu] == code[charptr..charptr + lu] {
                                        store.frames[fslot].eptr = eptr + lu as i64;
                                    } else {

                                        rreturn!(
                                            store, cur, st, rrc, entry,
                                            MATCH_NOMATCH, 'outer
                                        );
                                    }
                                    i += 1;
                                }
                                let max = store.frames[fslot].max;
                                if min == max {
                                    continue 'ops;
                                }
                                if store.frames[fslot].minimize != 0 {
                                    store.frames[fslot].fi = min;
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, 28,
                                        'outer, 'run
                                    );
                                } else {
                                    store.frames[fslot].pp = store.frames[fslot].eptr;
                                    i = min;
                                    while i < max {
                                        let eptr = store.frames[fslot].eptr;
                                        if eptr > st.subj_end - length as i64 {
                                            break;
                                        }
                                        let a = eptr as usize;
                                        let lu = length as usize;
                                        if subject_full[a..a + lu] == code[charptr..charptr + lu] {
                                            store.frames[fslot].eptr = eptr + lu as i64;
                                        } else {
                                            break;
                                        }
                                        i += 1;
                                    }
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, 29,
                                        'outer, 'run
                                    );
                                }
                            }
                        } else {
                            let min = store.frames[fslot].min;
                            if min as i64 > st.subj_end - store.frames[fslot].eptr {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                            let ecode = store.frames[fslot].ecode;
                            store.frames[fslot].fc = code[ecode] as i32;
                            store.frames[fslot].ecode = ecode + 1;
                        }
                        let ims = store.frames[fslot].ims;
                        if ims & pcre::CASELESS != 0 {
                            let fc = store.frames[fslot].fc;
                            store.frames[fslot].fc = st.lcc[fc as usize] as i32;
                            let min = store.frames[fslot].min;
                            i = 1;
                            while i <= min {
                                let eptr = store.frames[fslot].eptr;
                                let fc = store.frames[fslot].fc;
                                store.frames[fslot].eptr = eptr + 1;
                                if fc != st.lcc[subject_full[eptr as usize] as usize] as i32 {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                store.frames[fslot].eptr = eptr + 1;
                                i += 1;
                            }
                            let max = store.frames[fslot].max;
                            if min != max {
                                if store.frames[fslot].minimize != 0 {
                                    store.frames[fslot].fi = min;
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, 30,
                                        'outer, 'run
                                    );
                                } else {
                                    store.frames[fslot].pp = store.frames[fslot].eptr;
                                    i = min;
                                    while i < max {
                                        let eptr = store.frames[fslot].eptr;
                                        let fc = store.frames[fslot].fc;
                                        if eptr >= st.subj_end
                                            || fc
                                                != st.lcc[subject_full[eptr as usize] as usize]
                                                    as i32
                                        {
                                            break;
                                        }
                                        store.frames[fslot].eptr = eptr + 1;
                                        i += 1;
                                    }
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, 31,
                                        'outer, 'run
                                    );
                                }
                            }
                        } else {
                            let min = store.frames[fslot].min;
                            i = 1;
                            while i <= min {
                                let eptr = store.frames[fslot].eptr;
                                let fc = store.frames[fslot].fc;
                                store.frames[fslot].eptr = eptr + 1;
                                if fc != subject_full[eptr as usize] as i32 {
                                    rreturn!(
                                        store, cur, st, rrc, entry,
                                        MATCH_NOMATCH, 'outer
                                    );
                                }
                                store.frames[fslot].eptr = eptr + 1;
                                i += 1;
                            }
                            let max = store.frames[fslot].max;
                            if min != max {
                                if store.frames[fslot].minimize != 0 {
                                    store.frames[fslot].fi = min;
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, 32,
                                        'outer, 'run
                                    );
                                } else {
                                    store.frames[fslot].pp = store.frames[fslot].eptr;
                                    i = min;
                                    while i < max {
                                        let eptr = store.frames[fslot].eptr;
                                        let fc = store.frames[fslot].fc;
                                        if eptr >= st.subj_end
                                            || fc != subject_full[eptr as usize] as i32
                                        {
                                            break;
                                        }
                                        store.frames[fslot].eptr = eptr + 1;
                                        i += 1;
                                    }
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, 33,
                                        'outer, 'run
                                    );
                                }
                            }
                        }
                    }
                    op::NOT => {
                        let eptr = store.frames[fslot].eptr;
                        if eptr >= st.subj_end {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let mut ecode = store.frames[fslot].ecode + 1;
                        let mut eptr2 = eptr;
                        c = getchar_inc_test(subject_full, &mut eptr2, st.utf8, tables);
                        store.frames[fslot].eptr = eptr2;
                        let ims = store.frames[fslot].ims;
                        if ims & pcre::CASELESS != 0 {
                            if c < 256 {
                                c = st.lcc[c as usize] as i32;
                            }
                            if st.lcc[code[ecode] as usize] as i32 == c {
                                rreturn!(
                                    store, cur, st, rrc, entry, MATCH_NOMATCH,
                                    'outer
                                );
                            }
                        } else if code[ecode] as i32 == c {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        ecode += 1;
                        store.frames[fslot].ecode = ecode;
                    }
                    op::NOTEXACT
                    | op::NOTUPTO
                    | op::NOTMINUPTO
                    | op::NOTSTAR
                    | op::NOTMINSTAR
                    | op::NOTPLUS
                    | op::NOTMINPLUS
                    | op::NOTQUERY
                    | op::NOTMINQUERY => {
                        match op {
                            op::NOTEXACT => {
                                let ecode = store.frames[fslot].ecode;
                                let v = get2(code, ecode + 1);
                                store.frames[fslot].min = v;
                                store.frames[fslot].max = v;
                                store.frames[fslot].ecode = ecode + 3;
                            }
                            op::NOTUPTO | op::NOTMINUPTO => {
                                let ecode = store.frames[fslot].ecode;
                                store.frames[fslot].min = 0;
                                store.frames[fslot].max = get2(code, ecode + 1);
                                store.frames[fslot].minimize = (op == op::NOTMINUPTO) as i32;
                                store.frames[fslot].ecode = ecode + 3;
                            }
                            _ => {
                                let ecode = store.frames[fslot].ecode;
                                c = op as i32 - op::NOTSTAR as i32;
                                store.frames[fslot].minimize = c & 1;
                                store.frames[fslot].min = tables.rep_min[c as usize] as i32;
                                let mut max = tables.rep_max[c as usize] as i32;
                                if max == 0 {
                                    max = 0x7fff_ffff;
                                }
                                store.frames[fslot].max = max;
                                store.frames[fslot].ecode = ecode + 1;
                            }
                        }

                        let min = store.frames[fslot].min;
                        if min as i64 > st.subj_end - store.frames[fslot].eptr {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].fc = code[ecode] as i32;
                        store.frames[fslot].ecode = ecode + 1;
                        let ims = store.frames[fslot].ims;
                        if ims & pcre::CASELESS != 0 {
                            let fc = store.frames[fslot].fc;
                            store.frames[fslot].fc = st.lcc[fc as usize] as i32;
                            if st.utf8 {
                                i = 1;
                                while i <= min {
                                    let mut eptr = store.frames[fslot].eptr;
                                    let mut d = getchar_inc(subject_full, &mut eptr, tables);
                                    store.frames[fslot].eptr = eptr;
                                    if d < 256 {
                                        d = st.lcc[d as usize] as i32;
                                    }
                                    if store.frames[fslot].fc == d {
                                        rreturn!(
                                            store, cur, st, rrc, entry,
                                            MATCH_NOMATCH, 'outer
                                        );
                                    }
                                    i += 1;
                                }
                            } else {
                                i = 1;
                                while i <= min {
                                    let eptr = store.frames[fslot].eptr;
                                    let fc = store.frames[fslot].fc;
                                    let b = subject_full[eptr as usize];
                                    store.frames[fslot].eptr = eptr + 1;
                                    if fc == st.lcc[b as usize] as i32 {
                                        rreturn!(
                                            store, cur, st, rrc, entry,
                                            MATCH_NOMATCH, 'outer
                                        );
                                    }
                                    i += 1;
                                }
                            }
                            let max = store.frames[fslot].max;
                            if min != max {
                                if store.frames[fslot].minimize != 0 {
                                    store.frames[fslot].fi = min;
                                    let site = if st.utf8 { 34 } else { 35 };
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, site,
                                        'outer, 'run
                                    );
                                } else {
                                    store.frames[fslot].pp = store.frames[fslot].eptr;
                                    if st.utf8 {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end {
                                                break;
                                            }
                                            let mut len = 1;
                                            let mut d =
                                                getchar_len(subject_full, eptr, &mut len, tables);
                                            if d < 256 {
                                                d = st.lcc[d as usize] as i32;
                                            }
                                            if store.frames[fslot].fc == d {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + len as i64;
                                            i += 1;
                                        }
                                        let fr = store.frame_ref(live);
                                        let (fe, fc, ft, fi2, fb) =
                                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                        rmatch!(
                                            store, host, status, cur, st, rrc,
                                            entry, fe, fc, ft, fi2, fb, 0, 36,
                                            'outer, 'run
                                        );
                                    } else {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            let fc = store.frames[fslot].fc;
                                            if eptr >= st.subj_end
                                                || fc
                                                    == st.lcc[subject_full[eptr as usize] as usize]
                                                        as i32
                                            {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + 1;
                                            i += 1;
                                        }
                                        let fr = store.frame_ref(live);
                                        let (fe, fc, ft, fi2, fb) =
                                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                        rmatch!(
                                            store, host, status, cur, st, rrc,
                                            entry, fe, fc, ft, fi2, fb, 0, 37,
                                            'outer, 'run
                                        );
                                    }
                                }
                            }
                        } else {
                            if st.utf8 {
                                i = 1;
                                while i <= min {
                                    let mut eptr = store.frames[fslot].eptr;
                                    let d = getchar_inc(subject_full, &mut eptr, tables);
                                    store.frames[fslot].eptr = eptr;
                                    if store.frames[fslot].fc == d {
                                        rreturn!(
                                            store, cur, st, rrc, entry,
                                            MATCH_NOMATCH, 'outer
                                        );
                                    }
                                    i += 1;
                                }
                            } else {
                                i = 1;
                                while i <= min {
                                    let eptr = store.frames[fslot].eptr;
                                    let fc = store.frames[fslot].fc;
                                    let b = subject_full[eptr as usize] as i32;
                                    store.frames[fslot].eptr = eptr + 1;
                                    if fc == b {
                                        rreturn!(
                                            store, cur, st, rrc, entry,
                                            MATCH_NOMATCH, 'outer
                                        );
                                    }
                                    i += 1;
                                }
                            }
                            let max = store.frames[fslot].max;
                            if min != max {
                                if store.frames[fslot].minimize != 0 {
                                    store.frames[fslot].fi = min;
                                    let site = if st.utf8 { 38 } else { 39 };
                                    let fr = store.frame_ref(live);
                                    let (fe, fc, ft, fi2, fb) =
                                        (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                    rmatch!(
                                        store, host, status, cur, st, rrc,
                                        entry, fe, fc, ft, fi2, fb, 0, site,
                                        'outer, 'run
                                    );
                                } else {
                                    store.frames[fslot].pp = store.frames[fslot].eptr;
                                    if st.utf8 {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end {
                                                break;
                                            }
                                            let mut len = 1;
                                            let d =
                                                getchar_len(subject_full, eptr, &mut len, tables);
                                            if store.frames[fslot].fc == d {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + len as i64;
                                            i += 1;
                                        }
                                        let fr = store.frame_ref(live);
                                        let (fe, fc, ft, fi2, fb) =
                                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                        rmatch!(
                                            store, host, status, cur, st, rrc,
                                            entry, fe, fc, ft, fi2, fb, 0, 40,
                                            'outer, 'run
                                        );
                                    } else {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            let fc = store.frames[fslot].fc;
                                            if eptr >= st.subj_end
                                                || fc == subject_full[eptr as usize] as i32
                                            {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + 1;
                                            i += 1;
                                        }
                                        let fr = store.frame_ref(live);
                                        let (fe, fc, ft, fi2, fb) =
                                            (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                        rmatch!(
                                            store, host, status, cur, st, rrc,
                                            entry, fe, fc, ft, fi2, fb, 0, 41,
                                            'outer, 'run
                                        );
                                    }
                                }
                            }
                        }
                    }
                    op::TYPEEXACT
                    | op::TYPEUPTO
                    | op::TYPEMINUPTO
                    | op::TYPESTAR
                    | op::TYPEMINSTAR
                    | op::TYPEPLUS
                    | op::TYPEMINPLUS
                    | op::TYPEQUERY
                    | op::TYPEMINQUERY => {
                        match op {
                            op::TYPEEXACT => {
                                let ecode = store.frames[fslot].ecode;
                                let v = get2(code, ecode + 1);
                                store.frames[fslot].min = v;
                                store.frames[fslot].max = v;
                                store.frames[fslot].ecode = ecode + 3;
                            }
                            op::TYPEUPTO | op::TYPEMINUPTO => {
                                let ecode = store.frames[fslot].ecode;
                                store.frames[fslot].min = 0;
                                store.frames[fslot].max = get2(code, ecode + 1);
                                store.frames[fslot].minimize = (op == op::TYPEMINUPTO) as i32;
                                store.frames[fslot].ecode = ecode + 3;
                            }
                            _ => {
                                let ecode = store.frames[fslot].ecode;
                                c = op as i32 - op::TYPESTAR as i32;
                                store.frames[fslot].minimize = c & 1;
                                store.frames[fslot].min = tables.rep_min[c as usize] as i32;
                                let mut max = tables.rep_max[c as usize] as i32;
                                if max == 0 {
                                    max = 0x7fff_ffff;
                                }
                                store.frames[fslot].max = max;
                                store.frames[fslot].ecode = ecode + 1;
                            }
                        }

                        let ecode = store.frames[fslot].ecode;
                        store.frames[fslot].ctype = code[ecode] as i32;
                        store.frames[fslot].ecode = ecode + 1;
                        let min = store.frames[fslot].min;
                        if min as i64 > st.subj_end - store.frames[fslot].eptr {
                            rreturn!(
                                store, cur, st, rrc, entry, MATCH_NOMATCH,
                                'outer
                            );
                        }
                        if min > 0 {
                            let ctype = store.frames[fslot].ctype as u8;
                            if st.utf8 {
                                match ctype {
                                    op::ANY => {
                                        let ims = store.frames[fslot].ims;
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end
                                                || (subject_full[eptr as usize] == 10
                                                    && ims & pcre::DOTALL == 0)
                                            {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            let mut eptr = eptr + 1;
                                            while eptr < st.subj_end
                                                && subject_full[eptr as usize] & 0xc0 == 0x80
                                            {
                                                eptr += 1;
                                            }
                                            store.frames[fslot].eptr = eptr;
                                            i += 1;
                                        }
                                    }
                                    op::ANYBYTE => {
                                        let eptr = store.frames[fslot].eptr + min as i64;
                                        store.frames[fslot].eptr = eptr;
                                    }
                                    op::NOT_DIGIT => {
                                        i = 1;
                                        while i <= min {
                                            let mut eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            c = getchar_inc(subject_full, &mut eptr, tables);
                                            store.frames[fslot].eptr = eptr;
                                            if c < 128 && st.ctypes[c as usize] & 0x04 != 0 {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            i += 1;
                                        }
                                    }
                                    op::DIGIT => {
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end
                                                || subject_full[eptr as usize] >= 128
                                                || st.ctypes[subject_full[eptr as usize] as usize]
                                                    & 0x04
                                                    == 0
                                            {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            store.frames[fslot].eptr = eptr + 1;
                                            i += 1;
                                        }
                                    }
                                    op::NOT_WHITESPACE => {
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }

                                            let b = subject_full[eptr as usize];
                                            if b < 128 {
                                                store.frames[fslot].eptr = eptr + 1;
                                                if st.ctypes[b as usize] & 0x01 != 0 {
                                                    rreturn!(
                                                        store, cur, st, rrc,
                                                        entry, MATCH_NOMATCH,
                                                        'outer
                                                    );
                                                }
                                            }
                                            let mut eptr = store.frames[fslot].eptr;
                                            while eptr < st.subj_end
                                                && subject_full[eptr as usize] & 0xc0 == 0x80
                                            {
                                                eptr += 1;
                                            }
                                            store.frames[fslot].eptr = eptr;
                                            i += 1;
                                        }
                                    }
                                    op::WHITESPACE => {
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end
                                                || subject_full[eptr as usize] >= 128
                                                || st.ctypes[subject_full[eptr as usize] as usize]
                                                    & 0x01
                                                    == 0
                                            {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            store.frames[fslot].eptr = eptr + 1;
                                            i += 1;
                                        }
                                    }
                                    op::NOT_WORDCHAR => {
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }

                                            let b = subject_full[eptr as usize];
                                            if b < 128 {
                                                store.frames[fslot].eptr = eptr + 1;
                                                if st.ctypes[b as usize] & 0x10 != 0 {
                                                    rreturn!(
                                                        store, cur, st, rrc,
                                                        entry, MATCH_NOMATCH,
                                                        'outer
                                                    );
                                                }
                                            }
                                            let mut eptr = store.frames[fslot].eptr;
                                            while eptr < st.subj_end
                                                && subject_full[eptr as usize] & 0xc0 == 0x80
                                            {
                                                eptr += 1;
                                            }
                                            store.frames[fslot].eptr = eptr;
                                            i += 1;
                                        }
                                    }
                                    op::WORDCHAR => {
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end
                                                || subject_full[eptr as usize] >= 128
                                                || st.ctypes[subject_full[eptr as usize] as usize]
                                                    & 0x10
                                                    == 0
                                            {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            store.frames[fslot].eptr = eptr + 1;
                                            i += 1;
                                        }
                                    }
                                    _ => {
                                        rreturn!(
                                            store, cur, st, rrc, entry,
                                            MATCH_INTERNAL, 'outer
                                        );
                                    }
                                }
                            } else {
                                match ctype {
                                    op::ANY => {
                                        let ims = store.frames[fslot].ims;
                                        if ims & pcre::DOTALL == 0 {
                                            i = 1;
                                            while i <= min {
                                                let eptr = store.frames[fslot].eptr;
                                                let b = subject_full[eptr as usize];
                                                store.frames[fslot].eptr = eptr + 1;
                                                if b == 10 {
                                                    rreturn!(
                                                        store, cur, st, rrc,
                                                        entry, MATCH_NOMATCH,
                                                        'outer
                                                    );
                                                }
                                                i += 1;
                                            }
                                        } else {
                                            let eptr = store.frames[fslot].eptr + min as i64;
                                            store.frames[fslot].eptr = eptr;
                                        }
                                    }
                                    op::ANYBYTE => {
                                        let eptr = store.frames[fslot].eptr + min as i64;
                                        store.frames[fslot].eptr = eptr;
                                    }
                                    op::NOT_DIGIT => {
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            store.frames[fslot].eptr = eptr + 1;
                                            if st.ctypes[b as usize] & 0x04 != 0 {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            i += 1;
                                        }
                                    }
                                    op::DIGIT => {
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            store.frames[fslot].eptr = eptr + 1;
                                            if st.ctypes[b as usize] & 0x04 == 0 {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            i += 1;
                                        }
                                    }
                                    op::NOT_WHITESPACE => {
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            store.frames[fslot].eptr = eptr + 1;
                                            if st.ctypes[b as usize] & 0x01 != 0 {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            i += 1;
                                        }
                                    }
                                    op::WHITESPACE => {
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            store.frames[fslot].eptr = eptr + 1;
                                            if st.ctypes[b as usize] & 0x01 == 0 {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            i += 1;
                                        }
                                    }
                                    op::NOT_WORDCHAR => {
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            store.frames[fslot].eptr = eptr + 1;
                                            if st.ctypes[b as usize] & 0x10 != 0 {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            i += 1;
                                        }
                                    }
                                    op::WORDCHAR => {
                                        i = 1;
                                        while i <= min {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            store.frames[fslot].eptr = eptr + 1;
                                            if st.ctypes[b as usize] & 0x10 == 0 {
                                                rreturn!(
                                                    store, cur, st, rrc,
                                                    entry, MATCH_NOMATCH,
                                                    'outer
                                                );
                                            }
                                            i += 1;
                                        }
                                    }
                                    _ => {
                                        rreturn!(
                                            store, cur, st, rrc, entry,
                                            MATCH_INTERNAL, 'outer
                                        );
                                    }
                                }
                            }
                        }
                        let max = store.frames[fslot].max;
                        if min == max {

                        } else if store.frames[fslot].minimize != 0 {
                            store.frames[fslot].fi = min;
                            let site = if st.utf8 { 44 } else { 45 };
                            let fr = store.frame_ref(live);
                            let (fe, fc, ft, fi2, fb) =
                                (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                            rmatch!(
                                store, host, status, cur, st, rrc, entry, fe,
                                fc, ft, fi2, fb, 0, site, 'outer, 'run
                            );
                        } else {
                            store.frames[fslot].pp = store.frames[fslot].eptr;
                            let ctype = store.frames[fslot].ctype as u8;
                            if st.utf8 {
                                match ctype {
                                    op::ANY => {
                                        let ims = store.frames[fslot].ims;
                                        if max < 0x7fff_ffff {
                                            if ims & pcre::DOTALL == 0 {
                                                i = min;
                                                while i < max {
                                                    let eptr = store.frames[fslot].eptr;
                                                    if eptr >= st.subj_end
                                                        || subject_full[eptr as usize] == 10
                                                    {
                                                        break;
                                                    }
                                                    let mut eptr = eptr + 1;
                                                    while eptr < st.subj_end
                                                        && subject_full[eptr as usize] & 0xc0
                                                            == 0x80
                                                    {
                                                        eptr += 1;
                                                    }
                                                    store.frames[fslot].eptr = eptr;
                                                    i += 1;
                                                }
                                            } else {
                                                i = min;
                                                while i < max {
                                                    let mut eptr = store.frames[fslot].eptr + 1;
                                                    while eptr < st.subj_end
                                                        && subject_full[eptr as usize] & 0xc0
                                                            == 0x80
                                                    {
                                                        eptr += 1;
                                                    }
                                                    store.frames[fslot].eptr = eptr;
                                                    i += 1;
                                                }
                                            }
                                        } else if ims & pcre::DOTALL == 0 {
                                            i = min;
                                            while i < max {
                                                let eptr = store.frames[fslot].eptr;
                                                if eptr >= st.subj_end
                                                    || subject_full[eptr as usize] == 10
                                                {
                                                    break;
                                                }
                                                store.frames[fslot].eptr = eptr + 1;
                                                i += 1;
                                            }
                                        } else {
                                            c = max - min;
                                            if c as i64 > st.subj_end - store.frames[fslot].eptr {
                                                c = (st.subj_end - store.frames[fslot].eptr) as i32;
                                            }
                                            let eptr = store.frames[fslot].eptr + c as i64;
                                            store.frames[fslot].eptr = eptr;
                                        }
                                    }
                                    op::ANYBYTE => {
                                        c = max - min;
                                        if c as i64 > st.subj_end - store.frames[fslot].eptr {
                                            c = (st.subj_end - store.frames[fslot].eptr) as i32;
                                        }
                                        let eptr = store.frames[fslot].eptr + c as i64;
                                        store.frames[fslot].eptr = eptr;
                                    }
                                    op::NOT_DIGIT => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end {
                                                break;
                                            }
                                            let mut len = 1;
                                            c = getchar_len(subject_full, eptr, &mut len, tables);
                                            if c < 256 && st.ctypes[c as usize] & 0x04 != 0 {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + len as i64;
                                            i += 1;
                                        }
                                    }
                                    op::DIGIT => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end {
                                                break;
                                            }
                                            let mut len = 1;
                                            c = getchar_len(subject_full, eptr, &mut len, tables);
                                            if c >= 256 || st.ctypes[c as usize] & 0x04 == 0 {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + len as i64;
                                            i += 1;
                                        }
                                    }
                                    op::NOT_WHITESPACE => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end {
                                                break;
                                            }
                                            let mut len = 1;
                                            c = getchar_len(subject_full, eptr, &mut len, tables);
                                            if c < 256 && st.ctypes[c as usize] & 0x01 != 0 {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + len as i64;
                                            i += 1;
                                        }
                                    }
                                    op::WHITESPACE => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end {
                                                break;
                                            }
                                            let mut len = 1;
                                            c = getchar_len(subject_full, eptr, &mut len, tables);
                                            if c >= 256 || st.ctypes[c as usize] & 0x01 == 0 {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + len as i64;
                                            i += 1;
                                        }
                                    }
                                    op::NOT_WORDCHAR => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end {
                                                break;
                                            }
                                            let mut len = 1;
                                            c = getchar_len(subject_full, eptr, &mut len, tables);
                                            if c < 256 && st.ctypes[c as usize] & 0x10 != 0 {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + len as i64;
                                            i += 1;
                                        }
                                    }
                                    op::WORDCHAR => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            if eptr >= st.subj_end {
                                                break;
                                            }
                                            let mut len = 1;
                                            c = getchar_len(subject_full, eptr, &mut len, tables);
                                            if c >= 256 || st.ctypes[c as usize] & 0x10 == 0 {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + len as i64;
                                            i += 1;
                                        }
                                    }
                                    _ => {
                                        rreturn!(
                                            store, cur, st, rrc, entry,
                                            MATCH_INTERNAL, 'outer
                                        );
                                    }
                                }
                                let fr = store.frame_ref(live);
                                let (fe, fc, ft, fi2, fb) =
                                    (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                rmatch!(
                                    store, host, status, cur, st, rrc, entry,
                                    fe, fc, ft, fi2, fb, 0, 48, 'outer, 'run
                                );
                            } else {
                                match ctype {
                                    op::ANY => {
                                        let ims = store.frames[fslot].ims;
                                        if ims & pcre::DOTALL == 0 {
                                            i = min;
                                            while i < max {
                                                let eptr = store.frames[fslot].eptr;
                                                if eptr >= st.subj_end
                                                    || subject_full[eptr as usize] == 10
                                                {
                                                    break;
                                                }
                                                store.frames[fslot].eptr = eptr + 1;
                                                i += 1;
                                            }
                                        } else {

                                            c = max - min;
                                            if c as i64 > st.subj_end - store.frames[fslot].eptr {
                                                c = (st.subj_end - store.frames[fslot].eptr) as i32;
                                            }
                                            let eptr = store.frames[fslot].eptr + c as i64;
                                            store.frames[fslot].eptr = eptr;
                                        }
                                    }
                                    op::ANYBYTE => {
                                        c = max - min;
                                        if c as i64 > st.subj_end - store.frames[fslot].eptr {
                                            c = (st.subj_end - store.frames[fslot].eptr) as i32;
                                        }
                                        let eptr = store.frames[fslot].eptr + c as i64;
                                        store.frames[fslot].eptr = eptr;
                                    }
                                    op::NOT_DIGIT => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            if eptr >= st.subj_end
                                                || st.ctypes[b as usize] & 0x04 != 0
                                            {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + 1;
                                            i += 1;
                                        }
                                    }
                                    op::DIGIT => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            if eptr >= st.subj_end
                                                || st.ctypes[b as usize] & 0x04 == 0
                                            {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + 1;
                                            i += 1;
                                        }
                                    }
                                    op::NOT_WHITESPACE => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            if eptr >= st.subj_end
                                                || st.ctypes[b as usize] & 0x01 != 0
                                            {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + 1;
                                            i += 1;
                                        }
                                    }
                                    op::WHITESPACE => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            if eptr >= st.subj_end
                                                || st.ctypes[b as usize] & 0x01 == 0
                                            {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + 1;
                                            i += 1;
                                        }
                                    }
                                    op::NOT_WORDCHAR => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            if eptr >= st.subj_end
                                                || st.ctypes[b as usize] & 0x10 != 0
                                            {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + 1;
                                            i += 1;
                                        }
                                    }
                                    op::WORDCHAR => {
                                        i = min;
                                        while i < max {
                                            let eptr = store.frames[fslot].eptr;
                                            let b = subject_full[eptr as usize];
                                            if eptr >= st.subj_end
                                                || st.ctypes[b as usize] & 0x10 == 0
                                            {
                                                break;
                                            }
                                            store.frames[fslot].eptr = eptr + 1;
                                            i += 1;
                                        }
                                    }
                                    _ => {
                                        rreturn!(
                                            store, cur, st, rrc, entry,
                                            MATCH_INTERNAL, 'outer
                                        );
                                    }
                                }
                                let fr = store.frame_ref(live);
                                let (fe, fc, ft, fi2, fb) =
                                    (fr.eptr, fr.ecode, fr.offset_top, fr.ims, fr.eptrb);
                                rmatch!(
                                    store, host, status, cur, st, rrc, entry,
                                    fe, fc, ft, fi2, fb, 0, 49, 'outer, 'run
                                );
                            }
                        }
                    }
                    _ => {
                        rreturn!(store, cur, st, rrc, entry, MATCH_UNKNOWN_NODE, 'outer);
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct ExecStudy<'a> {
    pub options: u32,
    pub start_bits: &'a [u8; 32],

    pub size: u32,
}

#[derive(Clone, Copy)]
pub struct ExecExtra<'a> {
    pub flags: u32,
    pub study: Option<ExecStudy<'a>>,
    pub match_limit: u32,
    pub callout_data: u32,
    pub tables: Option<&'a [u8]>,
}

fn clear_store_entries(host: &mut impl PcreHost, store: &mut FrameStore) {
    let heap = store.heap;
    let count = store.block_count as usize;
    assert!(
        count <= store.blocks.len(),
        "exec block count beyond caller blocks"
    );
    for slot in store.blocks.iter_mut().take(count).skip(1) {
        if slot.present {
            host.free(heap, slot.code_id);
            slot.code_id = 0;
            host.free(heap, slot.extra_id);
            slot.extra_id = 0;
        }
        host.free(heap, slot.entry_id);
        slot.entry_id = 0;
        slot.present = false;
        slot.live = false;
    }
    store.block_count = 1;
    store.current_block = 0;
    store.highest_block = 0;
    store.max_frames = 0;
}

#[allow(clippy::too_many_arguments)]
pub fn pcre_exec(
    host: &mut impl PcreHost,
    object: u32,
    key: u32,
    store: &mut FrameStore,
    program: &[u8],
    extra: Option<&ExecExtra>,
    subject_full: &[u8],
    subj_start: usize,
    subj_len: usize,
    start_offset: i32,
    options: u32,
    offsets: Option<&mut [i32]>,
    offsetcount: i32,
    tables: &PcreTables<'_>,
    callout: Option<CalloutFn>,
) -> i32 {
    const MAGIC: u32 = 0x5043_5245;

    const DEFAULT_MATCH_LIMIT: u32 = 0x0098_9680;
    let status = host.validate(object, key, 0xdd57, 0x18);
    if status >= 0x8000_0000 {
        return -16;
    }
    clear_store_entries(host, store);
    if options & !0xa590 != 0 {
        return -3;
    }
    if offsets.is_none() && offsetcount > 0 {
        return -2;
    }
    if program.len() < 40 {

        return -2;
    }
    if offsetcount < 0 {
        return -15;
    }
    let mut study: Option<ExecStudy<'_>> = None;
    let mut match_limit = DEFAULT_MATCH_LIMIT;
    let mut callout_data = 0u32;
    let mut custom_tables: Option<&[u8]> = None;
    if let Some(extra) = extra {
        if extra.flags & 1 != 0 {
            study = extra.study;
        }
        if extra.flags & 2 != 0 {
            match_limit = extra.match_limit;
        }
        if extra.flags & 4 != 0 {
            callout_data = extra.callout_data;
        }
        if extra.flags & 8 != 0 {
            custom_tables = extra.tables;
        }
    }
    if read_u32(program, 32) != 0 {
        panic!("exec program carries a tables pointer");
    }
    let table_block: &[u8] = custom_tables.unwrap_or(tables.default_tables.as_slice());

    let mut internal_re = [0u8; 40];
    let mut internal_study_bits = [0u8; 32];
    let re_magic = read_u32(program, 0);
    if re_magic != MAGIC {
        match byteswap_pattern(program, &mut internal_re, None, None) {
            Ok(true) => {}
            _ => return -4,
        }
        if let Some(s) = study {

            internal_study_bits.copy_from_slice(s.start_bits);
            study = Some(ExecStudy {
                options: s.options.swap_bytes(),
                start_bits: &internal_study_bits,
                size: s.size.swap_bytes(),
            });
        }
    }
    let hdr: &[u8] = if re_magic != MAGIC {
        &internal_re
    } else {
        program
    };
    let re_options = read_u32(hdr, 8);
    let re_top_bracket = read_u16(hdr, 16) as i32;
    let re_top_backref = read_u16(hdr, 18) as i32;
    let re_first_byte = read_u16(hdr, 20);
    let re_req_byte = read_u16(hdr, 22);
    let name_table_offset = read_u16(hdr, 24) as u32;
    let name_entry_size = read_u16(hdr, 26) as u32;
    let name_count = read_u16(hdr, 28) as u32;
    let anchored = (re_options | options) & 0x10 != 0;
    let startline = re_options & 0x1000_0000 != 0;
    let code_off =
        name_table_offset.wrapping_add(name_count.wrapping_mul(name_entry_size)) as usize;
    let code: &[u8] = program.get(code_off..).unwrap_or(&[]);
    let subj_start64 = subj_start as i64;
    let subj_end64 = subj_start64 + subj_len as i64;
    let endonly = re_options & 0x20 != 0;
    let utf8 = re_options & 0x800 != 0;
    if (options & 0x8000) != 0 && re_options & 0x0400_0000 != 0 {
        return -13;
    }
    if utf8 && options & 0x2000 == 0 {
        let data = &subject_full[subj_start..subj_start + subj_len];
        if validate_utf8(data, subj_len as i32, tables) >= 0 {
            return -10;
        }
        if start_offset > 0 && (start_offset as usize) < subj_len {
            let mut tb = subject_full[subj_start + start_offset as usize] as i32;
            if tb > 127 {
                tb &= 0xc0;
                if tb != 0 && tb != 0xc0 {
                    return -11;
                }
            }
        }
    }
    let ims = re_options & 7;
    let mut ocount = offsetcount - offsetcount % 3;
    let mut temp_id = 0u32;
    let mut using_temp = false;
    if re_top_backref > 0 && re_top_backref >= ocount / 3 {
        ocount = re_top_backref * 3 + 3;
        if ocount as usize > 1024 {
            return -6;
        }
        let bytes = ocount as u32 * 4;
        match host.alloc(store.heap, 1, bytes) {
            Some(id) => temp_id = id,
            None => return -6,
        }
        using_temp = true;
    }
    let offset_max = 2i32.wrapping_mul(ocount) / 3;
    let mut resetcount = 2 + re_top_bracket * 2;
    if resetcount > offsetcount {
        resetcount = ocount;
    }
    let temp = [0i32; 1024];
    let temp_len = ocount as usize;
    let user_vec: &mut [i32] = offsets.unwrap_or(&mut []);
    let mut st = ExecState {
        call_count: 0,
        match_limit,
        offset_end: ocount,
        offset_max,
        offset_overflow: false,
        notbol: options & 0x80 != 0,
        noteol: options & 0x100 != 0,
        utf8,
        endonly,
        notempty: options & 0x400 != 0,
        partial: options & 0x8000 != 0,
        hitend: false,
        lcc: &table_block[..256],
        ctypes: &table_block[832..832 + 256],
        subj_start: subj_start64,
        subj_end: subj_end64,
        start_match: subj_start64 + start_offset as i64,
        end_match: 0,
        end_offset_top: 0,
        capture_last: -1,
        start_offset,
        recursive: None,
        thisframe: None,
        user: user_vec,
        temp,
        temp_len,
        using_temp,
    };

    {
        let ovec = st.ovec_mut();
        let mut idx = ocount as i64;
        let end = ocount as i64 - resetcount as i64 / 2 + 1;
        while {
            idx -= 1;
            idx >= end
        } {
            ovec[idx as usize] = -1;
        }
    }
    let mut first_byte = -1i32;
    let mut first_byte_caseless = false;
    let mut start_bits: Option<&[u8]> = None;
    if !anchored {
        if re_options & 0x4000_0000 != 0 {
            first_byte = (re_first_byte & 255) as i32;
            first_byte_caseless = re_first_byte & 0x100 != 0;
            if first_byte_caseless {
                first_byte = st.lcc[first_byte as usize] as i32;
            }
        } else if !startline {
            if let Some(s) = &study {
                if s.options & 1 != 0 {
                    start_bits = Some(s.start_bits);
                }
            }
        }
    }
    let mut req_byte = -1i32;
    let mut req_byte_caseless = false;
    let mut req_byte2 = 0i32;
    if re_options & 0x2000_0000 != 0 {
        req_byte = (re_req_byte & 255) as i32;
        req_byte_caseless = re_req_byte & 0x100 != 0;
        req_byte2 = table_block[256 + req_byte as usize] as i32;
    }
    let mut start_match = subj_start64 + start_offset as i64;
    let mut req_byte_ptr = start_match - 1;
    loop {

        {
            let ovec = st.ovec_mut();
            let end = resetcount as usize;
            for slot in ovec.iter_mut().take(end) {
                *slot = -1;
            }
        }
        if first_byte >= 0 {
            if first_byte_caseless {
                while start_match < subj_end64
                    && st.lcc[subject_full[start_match as usize] as usize] as i32 != first_byte
                {
                    start_match += 1;
                }
            } else {
                while start_match < subj_end64
                    && subject_full[start_match as usize] as i32 != first_byte
                {
                    start_match += 1;
                }
            }
        } else if startline {
            if start_match > subj_start64 + start_offset as i64 {
                while start_match < subj_end64 && subject_full[(start_match - 1) as usize] != 10 {
                    start_match += 1;
                }
            }
        } else if let Some(bits) = start_bits {
            while start_match < subj_end64 {
                let byte = subject_full[start_match as usize];
                if (bits[byte as usize / 8] & (1 << (byte & 7))) == 0 {
                    start_match += 1;
                } else {
                    break;
                }
            }
        }
        if req_byte >= 0 && subj_end64 - start_match < 1000 && !st.partial {
            let mut scan = start_match + if first_byte >= 0 { 1 } else { 0 };
            if scan > req_byte_ptr {
                if req_byte_caseless {
                    while scan < subj_end64 {
                        let pp = subject_full[scan as usize] as i32;
                        scan += 1;
                        if pp == req_byte || pp == req_byte2 {
                            scan -= 1;
                            break;
                        }
                    }
                } else {
                    while scan < subj_end64 {
                        let pp = subject_full[scan as usize] as i32;
                        scan += 1;
                        if pp == req_byte {
                            scan -= 1;
                            break;
                        }
                    }
                }
                if scan >= subj_end64 {
                    break;
                }
                req_byte_ptr = scan;
            }
        }
        st.start_match = start_match;
        st.call_count = 0;
        let rc = pcre_match(
            host,
            store,
            99,
            subject_full,
            code,
            start_match,
            2,
            &mut st,
            ims,
            None,
            2,
            tables,
            callout,
            callout_data,
        );
        if rc == 0 {
            start_match += 1;
            if st.utf8 {
                while start_match < subj_end64 && subject_full[start_match as usize] & 0xc0 == 0x80
                {
                    start_match += 1;
                }
            }
            if anchored || start_match > subj_end64 {
                break;
            }
            continue;
        }
        if rc != 1 {
            return rc;
        }
        if using_temp {
            if offsetcount >= 4 {
                let end = offsetcount as usize;
                st.user[2..end].copy_from_slice(&st.temp[2..end]);
            }
            if st.end_offset_top > offsetcount {
                st.offset_overflow = true;
            }
            host.free(store.heap, temp_id);
        }
        let mut rc = if st.offset_overflow {
            0
        } else {
            st.end_offset_top / 2
        };
        if offsetcount < 2 {
            rc = 0;
        } else {
            st.user[0] = (start_match - subj_start64) as i32;
            st.user[1] = (st.end_match - subj_start64) as i32;
        }
        return rc;
    }
    if using_temp {
        host.free(store.heap, temp_id);
    }
    if st.partial && st.hitend {
        -12
    } else {
        -1
    }
}

#[allow(clippy::too_many_arguments)]
pub fn pcre_exec_simple(
    host: &mut impl PcreHost,
    object: u32,
    key: u32,
    store: &mut FrameStore,
    program: &[u8],
    extra: Option<&ExecExtra>,
    subject_full: &[u8],
    subj_start: usize,
    subj_len: usize,
    start_offset: i32,
    options: u32,
    offsets: Option<&mut [i32]>,
    offsetcount: i32,
    tables: &PcreTables<'_>,
) -> i32 {
    pcre_exec(
        host,
        object,
        key,
        store,
        program,
        extra,
        subject_full,
        subj_start,
        subj_len,
        start_offset,
        options,
        offsets,
        offsetcount,
        tables,
        None,
    )
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .unwrap_or(0)
}

fn read_u16(bytes: &[u8], at: usize) -> u16 {
    bytes
        .get(at..at + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .unwrap_or(0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InfoValue<'a> {
    Word(u32),
    Int(i32),
    StartBits(Option<&'a [u8; 32]>),
    NameTable(usize),
    DefaultTables(&'a [u8; 1088]),
}

pub fn pcre_info<'a>(
    program: &'a [u8],
    extra: Option<&ExecExtra<'a>>,
    selector: i32,
    tables: &PcreTables<'a>,
) -> Result<InfoValue<'a>, i32> {
    const MAGIC: u32 = 0x5043_5245;
    if program.len() < 40 {
        return Err(-2);
    }
    let flag_study = extra.map_or(false, |e| e.flags & 1 != 0);
    let mut internal_re = [0u8; 40];
    let mut internal_study = [0u8; 40];
    let re_magic = read_u32(program, 0);

    let mut study_size = 0u32;
    let mut study_options = 0u32;
    let mut have_study = false;
    let hdr: &[u8] = if re_magic != MAGIC {
        let study_in: Option<[u8; 40]> = if flag_study {
            extra.and_then(|e| e.study).map(|st| {
                let mut buf = [0u8; 40];
                buf[0..4].copy_from_slice(&st.size.to_le_bytes());
                buf[4..8].copy_from_slice(&st.options.to_le_bytes());
                buf[8..40].copy_from_slice(st.start_bits);
                buf
            })
        } else {
            None
        };
        let study_out = if study_in.is_some() {
            Some(internal_study.as_mut_slice())
        } else {
            None
        };
        match byteswap_pattern(
            program,
            internal_re.as_mut_slice(),
            study_in.as_ref().map(|b| b.as_slice()),
            study_out,
        ) {
            Ok(true) => {}
            _ => return Err(-4),
        }
        if extra.and_then(|e| e.study).filter(|_| flag_study).is_some() {
            have_study = true;
            study_size = read_u32(&internal_study, 0);
            study_options = read_u32(&internal_study, 4);
        }
        &internal_re
    } else {
        if let Some(study) = extra.and_then(|e| e.study).filter(|_| flag_study) {
            have_study = true;
            study_size = study.size;
            study_options = study.options;
        }
        program
    };
    let options = read_u32(hdr, 8);
    let size = read_u32(hdr, 4);
    match selector {
        0 => Ok(InfoValue::Word(options & 0x7a7f)),
        1 => Ok(InfoValue::Word(size)),
        10 => Ok(InfoValue::Word(if have_study { study_size } else { 0 })),
        2 => Ok(InfoValue::Int(read_u16(hdr, 16) as i32)),
        3 => Ok(InfoValue::Int(read_u16(hdr, 18) as i32)),
        4 => Ok(InfoValue::Int(if options & 0x4000_0000 != 0 {
            read_u16(hdr, 20) as i32
        } else if options & 0x1000_0000 != 0 {
            -1
        } else {
            -2
        })),
        5 => {
            let bits = if have_study && study_options & 1 != 0 {
                extra.and_then(|e| e.study).map(|st| st.start_bits)
            } else {
                None
            };
            Ok(InfoValue::StartBits(bits))
        }
        6 => Ok(InfoValue::Int(if options & 0x2000_0000 != 0 {
            read_u16(hdr, 22) as i32
        } else {
            -1
        })),
        7 => Ok(InfoValue::Int(read_u16(hdr, 26) as i32)),
        8 => Ok(InfoValue::Int(read_u16(hdr, 28) as i32)),
        9 => Ok(InfoValue::NameTable(read_u16(hdr, 24) as usize)),
        11 => Ok(InfoValue::DefaultTables(tables.default_tables)),
        _ => Err(-3),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PcreHandleOut {
    pub object: Option<u32>,
    pub key: u32,
}

pub fn pcre_open(
    host: &mut impl PcreHost,
    handle_ptr: u32,
    handle_key: i32,
    out: Option<&mut PcreHandleOut>,
) -> u32 {
    let slot = match out {
        Some(slot) => slot,
        None => return 0x8a502007,
    };
    slot.object = None;
    slot.key = 0;
    let (rc, owner) = host.resolve_owner(handle_ptr, handle_key);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let heap = host.heap_of(owner);
    match host.alloc(heap, 1, 24) {
        Some(id) => {
            slot.object = Some(id);
            slot.key = 0xdd57;
        }
        None => return 0x8a50200a,
    }
    rc
}

pub fn pcre_counters(
    host: &mut impl PcreHost,
    object: u32,
    key: u32,
    store: &FrameStore,
    a: &mut u16,
    b: &mut u16,
) -> u32 {
    let rc = host.validate(object, key, 0xdd57, 24);
    if rc >= 0x8000_0000 {
        return rc;
    }
    *a = store.highest_block.wrapping_add(1);
    *b = store.max_frames;
    rc
}

pub fn pcre_deinit(host: &mut impl PcreHost, object: u32, key: u32, store: &mut FrameStore) -> u32 {
    let status = host.validate(object, key, 0xdd57, 0x18);
    if status >= 0x8000_0000 {
        return status;
    }
    let heap = store.heap;
    let count = store.block_count as usize;
    assert!(
        count <= store.blocks.len(),
        "deinit block count beyond caller blocks"
    );
    for slot in store.blocks.iter_mut().take(count).skip(1) {
        if slot.present {
            host.free(heap, slot.code_id);
            slot.code_id = 0;
            host.free(heap, slot.extra_id);
            slot.extra_id = 0;
        }
        host.free(heap, slot.entry_id);
        slot.entry_id = 0;
        slot.present = false;
        slot.live = false;
    }
    store.block_count = 1;
    store.current_block = 0;
    store.highest_block = 0;
    store.max_frames = 0;
    if let Some(array_id) = store.blocks_id {
        if let Some(head) = store.blocks.first_mut() {

            if head.present {
                host.free(heap, head.code_id);
                head.code_id = 0;
                host.free(heap, head.extra_id);
                head.extra_id = 0;
                host.free(heap, head.entry_id);
                head.entry_id = 0;
                head.present = false;
                head.live = false;
            }
        }
        host.free(heap, array_id);
        store.blocks_id = None;
    }
    0
}

pub fn pcre_close(host: &mut impl PcreHost, heap: u32, object: u32, key: u32) -> u32 {
    let status = host.validate(object, key, 0xdd57, 0x18);
    if status >= 0x8000_0000 {
        return status;
    }
    host.free(heap, object);
    0
}

pub fn pcre_init(
    host: &mut impl PcreHost,
    object: u32,
    key: u32,
    store: &mut FrameStore,
    capacity: i32,
    width: i32,
) -> u32 {
    let result = host.validate(object, key, 0xdd57, 24);
    if result >= 0x8000_0000 {
        return result;
    }
    let heap = store.heap;
    store.frame_size = 284;
    store.block_count = 0;
    store.max_blocks = capacity as u16;
    store.current_block = 0;
    store.highest_block = 0;
    store.max_frames = 0;
    store.frames_per_block = width as u16;
    let array_bytes = capacity.wrapping_add(1).wrapping_mul(4) as u32;
    match host.alloc(heap, 1, array_bytes) {
        Some(id) => store.blocks_id = Some(id),
        None => {
            store.blocks_id = None;
            pcre_deinit(host, object, key, store);
            return 0x8a50200a;
        }
    }
    let header_id = host.alloc(heap, 1, 20);
    {
        let head = store
            .blocks
            .first_mut()
            .expect("init without caller blocks");
        match header_id {
            Some(id) => {
                head.entry_id = id;
                head.present = true;
                head.live = true;
            }
            None => {
                head.entry_id = 0;
                head.present = false;
                head.live = false;
            }
        }
        head.used = 0;
        head.count = 0;
        head.code_id = 0;
        head.extra_id = 0;
    }
    if header_id.is_none() {
        pcre_deinit(host, object, key, store);
        return 0x8a50200a;
    }
    let data_bytes = 284u32.wrapping_mul(width as u32).wrapping_add(1);
    match host.alloc(heap, 1, data_bytes) {
        Some(id) => store.blocks[0].code_id = id,
        None => {
            pcre_deinit(host, object, key, store);
            return 0x8a50200a;
        }
    }
    let scratch_bytes = (width as u32).wrapping_add(1);
    match host.alloc(heap, 1, scratch_bytes) {
        Some(id) => store.blocks[0].extra_id = id,
        None => {
            pcre_deinit(host, object, key, store);
            return 0x8a50200a;
        }
    }
    store.block_count = 1;
    store.highest_block = 1;
    result
}
