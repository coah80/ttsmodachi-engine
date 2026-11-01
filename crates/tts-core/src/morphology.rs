use crate::frontend_types::Error;
use crate::heap::Heap;
use crate::pronunciation::{
    append_terminated, caseless_differs, class_index, copy_fixed, copy_terminated, find_substring,
    parse_int, read_offset, render_number, string_differs, terminated_len, terminated_len16,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BitCursor {
    pub position: u32,
    pub bit: u8,
}

impl BitCursor {

    pub fn decode(&mut self, data: &[u8], tree: &[[u16; 2]], root: u32) -> Result<u32, Error> {
        if self.bit > 7 {
            return Err(Error::InvalidResource);
        }
        let mut node = root as usize;
        let mut current = *data
            .get(self.position as usize)
            .ok_or(Error::InvalidResource)?;
        loop {
            let children = tree.get(node).ok_or(Error::InvalidResource)?;
            if children[0] == u16::MAX {
                return Ok(node as u32);
            }
            if self.bit == 0 {
                current = *data
                    .get(self.position as usize)
                    .ok_or(Error::InvalidResource)?;
            }
            node = usize::from(children[usize::from(current & (1 << self.bit) == 0)]);
            if self.bit < 7 {
                self.bit += 1;
            } else {
                self.position = self.position.checked_add(1).ok_or(Error::InvalidResource)?;
                self.bit = 0;
            }
        }
    }

    pub fn read_shifted(&mut self, data: &[u8], bits: u8, shift: u8) -> Result<u32, Error> {
        if self.bit > 7 || bits > 32 {
            return Err(Error::InvalidResource);
        }
        let start = self.position as usize;
        let end = u32::from(bits) + u32::from(self.bit);
        let mut value = u32::from(*data.get(start).ok_or(Error::InvalidResource)?) >> self.bit;
        if end > 8 {
            let mut i = 1;
            if end > 16 {
                loop {
                    let byte = *data.get(start + i).ok_or(Error::InvalidResource)?;
                    value = value.wrapping_add(u32::from(byte) << (i * 8 - usize::from(self.bit)));
                    i += 1;
                    if (i + 1) * 8 >= end as usize {
                        break;
                    }
                }
            }
            let remaining = (i * 8 + 8) as u32 - u32::from(self.bit) - u32::from(bits);
            let byte = *data.get(start + i).ok_or(Error::InvalidResource)?;
            let mask = 0xffu32 >> remaining;
            let extra = (u32::from(byte) & mask)
                .checked_shl((i * 8) as u32 - u32::from(self.bit))
                .unwrap_or(0);
            value = value.wrapping_add(extra);
        } else {
            value &= 0xffu32 >> (8 - bits);
        }
        self.position = self.position.wrapping_add((end & 0x38) >> 3);
        self.bit = (end & 7) as u8;
        Ok(value.checked_shl(u32::from(shift)).unwrap_or(0))
    }

    pub fn decode_count(
        &mut self,
        data: &[u8],
        tree: &[[u16; 2]],
        root: u16,
        values: &[u16],
        width: u8,
    ) -> Result<u32, Error> {
        let index = self.decode(data, tree, u32::from(root))?;
        if width <= 16 {
            return values
                .get(index as usize)
                .copied()
                .map(u32::from)
                .ok_or(Error::InvalidResource);
        }
        packed_value(values, index, width, false)
    }

    pub fn decode_offset(&mut self, data: &[u8], table: OffsetTable<'_>) -> Result<u32, Error> {
        let index = self.decode(data, table.tree, u32::from(table.root))?;
        let value = packed_value(table.values, index, table.width, true)?;
        Ok(if value > u32::from(table.inline_limit) {
            value.checked_shl(u32::from(table.shift)).unwrap_or(0)
        } else {
            value
        })
    }
}

#[derive(Clone, Copy)]
pub struct OffsetTable<'a> {
    pub tree: &'a [[u16; 2]],
    pub root: u16,
    pub values: &'a [u16],
    pub width: u8,
    pub inline_limit: u8,
    pub shift: u8,
}

fn packed_value(values: &[u16], index: u32, width: u8, mask_first: bool) -> Result<u32, Error> {
    if width > 32 {
        return Err(Error::InvalidResource);
    }
    let position = index.wrapping_mul(u32::from(width));
    let shift = position & 15;
    let word = ((position & 0xfffff) >> 4) as usize;
    let first = u32::from(*values.get(word).ok_or(Error::InvalidResource)?) >> shift;
    let width = u32::from(width);
    let remaining = 16 - shift;
    if width <= remaining {
        return Ok(if mask_first {
            first & (0xffff >> (16 - width))
        } else {
            first
        });
    }
    let second = u32::from(*values.get(word + 1).ok_or(Error::InvalidResource)?);
    if width <= 32 - shift {
        Ok(first.wrapping_add((second & (0xffff >> (32 - width - shift))) << remaining))
    } else {
        let third = u32::from(*values.get(word + 2).ok_or(Error::InvalidResource)?);
        Ok(first
            .wrapping_add(second << remaining)
            .wrapping_add((third & (0xffff >> (48 - width - shift))) << (32 - shift)))
    }
}

#[derive(Clone, Copy)]
pub struct TrieTables<'a> {
    pub alphabet: &'a [u8],
    pub first_character: u8,
    pub initial_characters: &'a [u8],
    pub initial_offsets: &'a [u32],
    pub initial_indices: &'a [u32],
    pub initial_position: u32,
    pub initial_bit: u8,
    pub direct_count: u16,
    pub count_tree: &'a [[u16; 2]],
    pub count_root: u16,
    pub counts: &'a [u16],
    pub character_tree: &'a [[u16; 2]],
    pub character_root: u16,
    pub characters: &'a [u8],
    pub delta_tree: &'a [[u16; 2]],
    pub delta_root: u16,
    pub deltas: &'a [u8],
    pub skipped_tree: &'a [[u16; 2]],
    pub skipped_root: u16,
    pub skipped_values: &'a [u16],
    pub skipped_width: u8,
    pub offsets: OffsetTable<'a>,
    pub terminator: u8,
    pub inline_limit: u8,
    pub alignment_mask: u8,
    pub data_offset: u32,
    pub page_size: u16,
    pub fallback_bits: u8,
    pub fallback_shift: u8,
}

pub trait DictionaryPages {
    fn read(&mut self, offset: u32, length: u32) -> Result<&[u8], Error>;
    fn release(&mut self);
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TrieCursor {
    pub bits: BitCursor,
    pub page_offset: Option<u32>,
}

impl TrieCursor {

    pub fn lookup(
        &mut self,
        text: &[u8],
        start: u32,
        end: u32,
        fold: bool,
        tables: TrieTables<'_>,
        pages: &mut impl DictionaryPages,
    ) -> Result<i32, Error> {
        let mapped = |index: usize| -> Result<u8, Error> {
            let c = *text.get(index).ok_or(Error::InvalidResource)?;
            let c = if fold { lowercase_byte(c) } else { c };
            let index = c
                .checked_sub(tables.first_character)
                .ok_or(Error::InvalidResource)?;
            tables
                .alphabet
                .get(usize::from(index))
                .copied()
                .ok_or(Error::InvalidResource)
        };
        let wanted = mapped(start as usize)?;
        let mut result = 0i32;
        let mut index = 0;
        while index < tables.initial_characters.len() && tables.initial_characters[index] < wanted {
            result = result.wrapping_add(
                *tables
                    .initial_indices
                    .get(index)
                    .ok_or(Error::InvalidResource)? as i32,
            );
            index += 1;
        }
        if tables.initial_characters.get(index) != Some(&wanted) {
            return Ok(-1);
        }
        let mut position = *tables
            .initial_offsets
            .get(index)
            .ok_or(Error::InvalidResource)?;
        if position <= u32::from(tables.inline_limit) {
            self.bits.position = tables.initial_position;
            self.bits.bit = tables.initial_bit;
            position = self
                .bits
                .position
                .wrapping_mul(8)
                .wrapping_add(u32::from(self.bits.bit));
        }
        let length = end.wrapping_sub(start).wrapping_add(1) as u16;
        let mut step = 1u16;
        let mut run = 0u16;
        loop {
            let wanted = if step == length {
                tables.terminator
            } else {
                mapped(start as usize + usize::from(step))?
            };
            let Some(skipped) = self.step(wanted, &mut position, &mut run, tables, pages)? else {
                return Ok(-1);
            };
            result = result.wrapping_add(skipped as i32);
            step = step.wrapping_add(1);
            if wanted == tables.terminator {
                return Ok(if position == 1 { result } else { -1 });
            }
        }
    }

    fn step(
        &mut self,
        wanted: u8,
        position: &mut u32,
        run: &mut u16,
        tables: TrieTables<'_>,
        pages: &mut impl DictionaryPages,
    ) -> Result<Option<u32>, Error> {
        let byte_position = *position >> 3;
        let offset = tables.data_offset.wrapping_add(byte_position);
        let data = match pages.read(offset, u32::from(tables.page_size)) {
            Ok(data) => data,
            Err(_) => return Ok(None),
        };
        self.page_offset = Some(offset);
        self.bits.position = 0;
        self.bits.bit = (*position & 7) as u8;
        let result = self.step_page(wanted, position, run, byte_position, tables, data);
        pages.release();
        result
    }

    fn step_page(
        &mut self,
        wanted: u8,
        position: &mut u32,
        run: &mut u16,
        byte_position: u32,
        tables: TrieTables<'_>,
        data: &[u8],
    ) -> Result<Option<u32>, Error> {
        let leaf = self
            .bits
            .decode(data, tables.count_tree, u32::from(tables.count_root))?;
        let count = *tables
            .counts
            .get(leaf as usize)
            .ok_or(Error::InvalidResource)?;
        let mut skipped = 0u32;
        if count <= tables.direct_count {
            let mut index = 1u16;
            let leaf = self.bits.decode(
                data,
                tables.character_tree,
                u32::from(tables.character_root),
            )?;
            let mut current = *tables
                .characters
                .get(leaf as usize)
                .ok_or(Error::InvalidResource)?;
            let mut next = 0;
            loop {
                if current != tables.terminator {
                    next = self.bits.decode_offset(data, tables.offsets)?;
                    if next == 0 {
                        next = self.bits.read_shifted(
                            data,
                            tables.fallback_bits,
                            tables.fallback_shift,
                        )?;
                    }
                    if current != wanted {
                        skipped = skipped.wrapping_add(self.bits.decode_count(
                            data,
                            tables.skipped_tree,
                            tables.skipped_root,
                            tables.skipped_values,
                            tables.skipped_width,
                        )?);
                    }
                } else if current != wanted {
                    skipped = skipped.wrapping_add(1);
                }
                if index >= count || current >= wanted {
                    break;
                }
                let leaf =
                    self.bits
                        .decode(data, tables.delta_tree, u32::from(tables.delta_root))?;
                current = current.wrapping_add(
                    *tables
                        .deltas
                        .get(leaf as usize)
                        .ok_or(Error::InvalidResource)?,
                );
                index += 1;
            }
            if current != wanted {
                return Ok(None);
            }
            if wanted == tables.terminator {
                *position = 1;
                return Ok(Some(skipped));
            }
            if next <= u32::from(tables.inline_limit) {
                if index < count {
                    self.bits.decode_count(
                        data,
                        tables.skipped_tree,
                        tables.skipped_root,
                        tables.skipped_values,
                        tables.skipped_width,
                    )?;
                }
                while index < count {
                    let leaf =
                        self.bits
                            .decode(data, tables.delta_tree, u32::from(tables.delta_root))?;
                    current = current.wrapping_add(
                        *tables
                            .deltas
                            .get(leaf as usize)
                            .ok_or(Error::InvalidResource)?,
                    );
                    if current != tables.terminator {
                        if self.bits.decode_offset(data, tables.offsets)? == 0 {
                            self.bits.read_shifted(
                                data,
                                tables.fallback_bits,
                                tables.fallback_shift,
                            )?;
                        }
                        if index < count - 1 {
                            self.bits.decode_count(
                                data,
                                tables.skipped_tree,
                                tables.skipped_root,
                                tables.skipped_values,
                                tables.skipped_width,
                            )?;
                        }
                    }
                    index += 1;
                }
                self.align(tables.alignment_mask);
                *position = u32::from(self.bits.bit).wrapping_add(
                    self.bits
                        .position
                        .wrapping_add(byte_position)
                        .wrapping_mul(8),
                );
            } else {
                *position = next;
            }
        } else {
            let count = count - tables.direct_count;
            let mut index = 0u16;
            let current = loop {
                let leaf = self.bits.decode(
                    data,
                    tables.character_tree,
                    u32::from(tables.character_root),
                )?;
                let current = *tables
                    .characters
                    .get(leaf as usize)
                    .ok_or(Error::InvalidResource)?;
                index = index.checked_add(1).ok_or(Error::InvalidResource)?;
                if index > *run {
                    break current;
                }
            };
            if current != wanted {
                return Ok(None);
            }
            *run = run.wrapping_add(1);
            if count <= *run {
                *run = 0;
                if wanted == tables.terminator {
                    *position = 1;
                    return Ok(Some(0));
                }
                *position = self.bits.decode_offset(data, tables.offsets)?;
                if *position == 0 {
                    *position = self.bits.read_shifted(
                        data,
                        tables.fallback_bits,
                        tables.fallback_shift,
                    )?;
                } else if *position <= u32::from(tables.inline_limit) {
                    self.align(tables.alignment_mask);
                    *position = u32::from(self.bits.bit).wrapping_add(
                        self.bits
                            .position
                            .wrapping_add(byte_position)
                            .wrapping_mul(8),
                    );
                }
            }
        }
        Ok((*position != 0).then_some(skipped))
    }

    fn align(&mut self, mask: u8) {
        while self.bits.bit & mask != 0 {
            self.bits.bit = self.bits.bit.wrapping_add(1);
        }
        if self.bits.bit == 8 {
            self.bits.bit = 0;
            self.bits.position = self.bits.position.wrapping_add(1);
        }
    }
}

pub(crate) fn lowercase_byte(c: u8) -> u8 {
    match c {
        b'A'..=b'Z' => c + 32,
        0x80 => 0x87,
        0x8e => 0x84,
        0x8f => 0x86,
        0x90 => 0x82,
        0x92 => 0x91,
        0x99 => 0x94,
        0x9a => 0x81,
        0xa5 => 0xa4,
        _ => c,
    }
}

#[derive(Clone, Copy)]
pub struct EntryTables<'a> {
    pub trie: TrieTables<'a>,
    pub pairs: &'a [[u8; 2]],
    pub pair_base: u8,
    pub input_min: u8,
    pub input_max: u8,
    pub separator: u8,
    pub prefix_min: u8,
    pub prefix_max: u8,
    pub max_length: i16,
    pub offset_bits: u8,
    pub offset_size: u16,
    pub offset_bytes: u32,
    pub offsets_offset: u32,
    pub record_size: u16,
    pub records_offset: u32,
}

impl TrieCursor {

    pub fn lookup_entry(
        &mut self,
        text: &[u8],
        start: i16,
        end: i16,
        fold: bool,
        tables: EntryTables<'_>,
        pages: &mut impl DictionaryPages,
        output: &mut [u8],
        output_length: &mut i16,
    ) -> Result<bool, Error> {
        let length = end.wrapping_sub(start).wrapping_add(1);
        if length <= 0 || length > tables.max_length {
            return Ok(false);
        }
        if start > end {
            return Ok(false);
        }
        let first = usize::try_from(start).map_err(|_| Error::InvalidResource)?;
        let last = usize::try_from(end).map_err(|_| Error::InvalidResource)?;
        for &c in text.get(first..=last).ok_or(Error::InvalidResource)? {
            let c = if fold { lowercase_byte(c) } else { c };
            if c < tables.input_min || c > tables.input_max {
                return Ok(false);
            }
        }
        let index = self.lookup(text, start as u32, end as u32, fold, tables.trie, pages)?;
        if index < 0 {
            return Ok(false);
        }
        let Some(offset) = self.entry_offset(index as u32, tables, pages)? else {
            return Ok(false);
        };
        if (offset as i32) < 0 {
            return Ok(false);
        }
        let address = tables.records_offset.wrapping_add(offset);
        let data = match pages.read(address, u32::from(tables.record_size)) {
            Ok(data) => data,
            Err(_) => return Ok(false),
        };
        self.page_offset = Some(address);
        let result = expand_entry(data, tables, output);
        pages.release();
        *output_length = result? as i16;
        Ok(true)
    }

    fn entry_offset(
        &mut self,
        index: u32,
        tables: EntryTables<'_>,
        pages: &mut impl DictionaryPages,
    ) -> Result<Option<u32>, Error> {
        let width = u32::from(tables.offset_bits);
        if width > 32 {
            return Err(Error::InvalidResource);
        }
        let position = index.wrapping_mul(width);
        let bit = position & 7;
        let offset = ((position as i32) >> 3) as u32;
        let mut size = u32::from(tables.offset_size);
        if offset.wrapping_add(size) > tables.offset_bytes {
            size = tables.offset_bytes.wrapping_sub(offset);
        }
        let address = tables.offsets_offset.wrapping_add(offset);
        let data = match pages.read(address, size) {
            Ok(data) => data,
            Err(_) => return Ok(None),
        };
        self.page_offset = Some(address);
        let result = read_entry_offset(data, bit, width);
        pages.release();
        result.map(Some)
    }
}

fn read_entry_offset(data: &[u8], bit: u32, width: u32) -> Result<u32, Error> {
    if width == 0 {
        return Ok(0);
    }
    let first = u32::from(*data.first().ok_or(Error::InvalidResource)?);
    let total = bit + width;
    if total <= 8 {
        return Ok(first >> (8 - total) & (u32::MAX >> (32 - width)));
    }
    let mut value = (first & (0xff >> bit)).checked_shl(total - 8).unwrap_or(0);
    let mut byte = 1usize;
    if total > 16 {
        loop {
            let next = u32::from(*data.get(byte).ok_or(Error::InvalidResource)?);
            value = value.wrapping_add(next << (total - byte as u32 * 8 - 8));
            byte += 1;
            if total <= (byte as u32 + 1) * 8 {
                break;
            }
        }
    }
    let last = u32::from(*data.get(byte).ok_or(Error::InvalidResource)?);
    Ok(value.wrapping_add(last >> ((byte as u32 + 1) * 8 - width - bit)))
}

fn expand_pair(
    output: &mut [u8],
    count: &mut usize,
    expanded: usize,
    tables: EntryTables<'_>,
) -> Result<(), Error> {
    loop {
        let c = *output.get(expanded).ok_or(Error::InvalidResource)?;
        let index = c
            .checked_sub(tables.pair_base)
            .ok_or(Error::InvalidResource)?;
        let pair = *tables
            .pairs
            .get(usize::from(index))
            .ok_or(Error::InvalidResource)?;
        if pair[0] == 0 {
            return Ok(());
        }
        if *count >= output.len() {
            return Err(Error::Capacity {
                required: *count + 1,
                available: output.len(),
            });
        }
        output.copy_within(expanded + 1..*count, expanded + 2);
        output[expanded] = pair[0];
        output[expanded + 1] = pair[1];
        *count += 1;
    }
}

fn expand_entry(data: &[u8], tables: EntryTables<'_>, output: &mut [u8]) -> Result<usize, Error> {
    let output_capacity = output.len();
    let prefix = |c| c >= tables.prefix_min && c <= tables.prefix_max;
    let mut position = 0;
    let mut count = 0;
    let mut expanded = 0;
    loop {
        *output.get_mut(count).ok_or(Error::Capacity {
            required: count + 1,
            available: output_capacity,
        })? = *data.get(position).ok_or(Error::InvalidResource)?;
        count += 1;
        position += 1;
        loop {
            expand_pair(output, &mut count, expanded, tables)?;
            expanded += 1;
            if expanded >= count {
                break;
            }
        }
        let c = *data.get(position).ok_or(Error::InvalidResource)?;
        if c == tables.separator || prefix(c) {
            break;
        }
    }
    let mut current_prefix = output[0];
    while prefix(current_prefix) {
        let c = loop {
            let c = *data.get(position).ok_or(Error::InvalidResource)?;
            if c == tables.separator || prefix(c) {
                break c;
            }
            position += 1;
        };
        if c == tables.separator {
            position += 1
        }
        let next_prefix = if prefix(c) {
            c
        } else {
            tables.prefix_min.wrapping_sub(1)
        };
        if next_prefix < current_prefix {
            let mut temporary = [0u8; 44];
            let mut length = 0;
            let mut expanded = 0;
            let mut remaining = current_prefix;
            loop {
                *temporary.get_mut(length).ok_or(Error::Capacity {
                    required: length + 1,
                    available: 44,
                })? = *data.get(position).ok_or(Error::InvalidResource)?;
                length += 1;
                position += 1;
                loop {
                    expand_pair(&mut temporary, &mut length, expanded, tables)?;
                    expanded += 1;
                    remaining = remaining.wrapping_sub(1);
                    if expanded >= length || remaining < next_prefix {
                        break;
                    }
                }
                if remaining < next_prefix {
                    break;
                }
            }
            let additional = usize::from(current_prefix - next_prefix);
            let length = count + additional;
            if length > output.len() {
                return Err(Error::Capacity {
                    required: length,
                    available: output.len(),
                });
            }
            output.copy_within(1..count, additional + 1);
            output[..additional + 1].copy_from_slice(
                temporary
                    .get(..additional + 1)
                    .ok_or(Error::InvalidResource)?,
            );
            count = length;
            current_prefix = next_prefix;
        } else {
            position += 1;
        }
    }
    Ok(count)
}

pub fn number_hyphens(source: &[u8], output: &mut [u8], lo: i16, hi: i16) -> Result<u32, Error> {
    let first = lo.wrapping_sub(1);
    let last = hi.wrapping_sub(1);
    if i32::from(last) - i32::from(first) + 1 <= 0 {
        return Ok(1);
    }
    let start = usize::try_from(first).map_err(|_| Error::InvalidResource)?;
    let end = usize::try_from(last).map_err(|_| Error::InvalidResource)? + 1;
    let source = source.get(start..end).ok_or(Error::InvalidResource)?;
    let available = output.len();
    let output = output.get_mut(start..end).ok_or(Error::Capacity {
        required: end,
        available,
    })?;
    let mut number = 1i16;
    for (&c, out) in source.iter().zip(output.iter_mut()).rev() {
        if c == b'-' {
            if number < 10 {
                *out = number.wrapping_add(i16::from(b'0')) as u8;
            }
            number = number.wrapping_add(1);
        }
    }
    Ok(1)
}

pub fn copy_range(source: &[u8], output: &mut [u8], lo: i16, hi: i16) -> Result<u32, Error> {
    if hi < lo {
        return Ok(1);
    }
    let start = usize::try_from(i32::from(lo) - 1).map_err(|_| Error::InvalidResource)?;
    let end = usize::try_from(hi).map_err(|_| Error::InvalidResource)?;
    let available = output.len();
    output
        .get_mut(start..end)
        .ok_or(Error::Capacity {
            required: end,
            available,
        })?
        .copy_from_slice(source.get(start..end).ok_or(Error::InvalidResource)?);
    Ok(1)
}

#[derive(Clone, Copy)]
pub struct DictionaryHeader<'a> {
    pub bytes: &'a [u8],
}

impl DictionaryHeader<'_> {

    pub fn result_capacity(&self) -> Result<u16, Error> {
        let bytes = self.bytes.get(48..50).ok_or(Error::InvalidResource)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn separator(&self) -> Result<u8, Error> {
        self.bytes.get(9).copied().ok_or(Error::InvalidResource)
    }
}

pub fn set_dictionary_resource(resource: &mut u32, value: u32) {
    *resource = value;
}

pub fn activate_dictionary(active: &mut u32) {
    *active = 1;
}

pub fn duration_frequency(rate: u32, duration: u32) -> u16 {
    if duration == 0 {
        0
    } else {
        ((rate << 12) / duration) as u16
    }
}

pub fn duration_samples(rate: u32, duration: u32) -> u32 {
    rate.wrapping_mul(duration).wrapping_mul(0x1064) >> 16
}

pub fn byte_precedes(left: u8, right: u8) -> bool {
    left < right
}

pub trait SystemDictionaryIo<'a> {
    fn open(&mut self, owner: u32, name: &[u8], flags: u32, file: &mut u32) -> u32;
    fn read(&mut self, owner: u32, file: u32, offset: u32, size: u32) -> Result<&'a [u8], u32>;
    fn release(&mut self, owner: u32, file: u32, data: &'a [u8]);
    fn close(&mut self, owner: u32, file: u32);
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SystemDictionaryFile {
    pub owner: u32,
    pub file: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DictionarySections<'a> {
    pub data: &'a [u8],
    pub offsets: [u32; 19],
}

fn dictionary_word(data: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = data.get(offset..offset + 4).ok_or(Error::InvalidResource)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn dictionary_halfword(data: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = data.get(offset..offset + 2).ok_or(Error::InvalidResource)?;
    Ok(u32::from(u16::from_le_bytes([bytes[0], bytes[1]])))
}

fn dictionary_index(data: &[u8], key: &[u8]) -> Result<Option<usize>, Error> {
    let mut position = 0usize;
    while position < data.len() {
        let length = data[position..]
            .iter()
            .position(|&byte| byte == 0)
            .ok_or(Error::InvalidResource)?;
        let name = &data[position..position + length];
        let value = (position + length + 4) & !3;
        dictionary_word(data, value)?;
        if name.len() == key.len()
            && name.iter().zip(key).all(|(&a, &b)| {
                let fold = |c: u8| if c.is_ascii_uppercase() { c + 32 } else { c };
                fold(a) == fold(b)
            })
        {
            return Ok(Some(value));
        }
        position = value + 4;
    }
    Ok(None)
}

impl SystemDictionaryFile {

    pub fn open<'a>(
        &mut self,
        name: &[u8],
        size: &mut u32,
        io: &mut impl SystemDictionaryIo<'a>,
    ) -> Result<u32, Error> {
        let status = io.open(self.owner, name, 1, &mut self.file);
        if status != 0 {
            self.file = 0;
            return Ok(status);
        }
        let header = match io.read(self.owner, self.file, 0, 40) {
            Ok(data) => data,
            Err(status) => {
                io.close(self.owner, self.file);
                self.file = 0;
                return Ok(status);
            }
        };
        if dictionary_word(header, 0)? != 0xfeff
            || header.get(4..12) != Some(b"SCANSOFT")
            || header.get(12..19) != Some(b"SYS_DCT")
        {
            return Ok(0x89802019);
        }
        let length = dictionary_word(header, 20)?;
        io.release(self.owner, self.file, header);
        *size = length.wrapping_sub(40);
        Ok(0)
    }

    pub fn close<'a>(&self, io: &mut impl SystemDictionaryIo<'a>) -> u32 {
        if self.file != 0 {
            io.close(self.owner, self.file);
        }
        0
    }

    pub fn contains<'a>(
        &mut self,
        key: &[u8],
        size: u32,
        found: &mut u8,
        io: &mut impl SystemDictionaryIo<'a>,
    ) -> Result<u32, Error> {
        *found = 0;
        let data = match io.read(self.owner, self.file, 40, size) {
            Ok(data) => data,
            Err(status) => {
                io.close(self.owner, self.file);
                self.file = 0;
                return Ok(status);
            }
        };
        let index = dictionary_index(
            data.get(..size as usize).ok_or(Error::InvalidResource)?,
            key,
        )?;
        *found = u8::from(index.is_some());
        io.release(self.owner, self.file, data);
        Ok(0)
    }

    pub fn load<'a>(
        &mut self,
        key: &[u8],
        size: u32,
        heap: u32,
        sections: &mut Option<DictionarySections<'a>>,
        io: &mut impl SystemDictionaryIo<'a>,
    ) -> Result<u32, Error> {
        let data = match io.read(self.owner, self.file, 40, size) {
            Ok(data) => data,
            Err(status) => {
                io.close(self.owner, self.file);
                self.file = 0;
                return Ok(status);
            }
        };
        let index = dictionary_index(
            data.get(..size as usize).ok_or(Error::InvalidResource)?,
            key,
        )?;
        let Some(index) = index else {
            io.release(self.owner, self.file, data);
            return Ok(0x89802014);
        };
        let offset = dictionary_word(data, index)?;
        io.release(self.owner, self.file, data);
        let header = match io.read(self.owner, self.file, offset, 4) {
            Ok(data) => data,
            Err(status) => {
                io.close(self.owner, self.file);
                self.file = 0;
                return Ok(status);
            }
        };
        let length = dictionary_word(header, 0)?;
        io.release(self.owner, self.file, header);
        let data = match io.read(self.owner, heap, offset, length) {
            Ok(data) => data,
            Err(status) => return Ok(status),
        };
        let mut offsets = [0u32; 19];
        let mut cursor = 72u32;
        offsets[1] = cursor;
        let first = *data.get(4).ok_or(Error::InvalidResource)?;
        let last = *data.get(5).ok_or(Error::InvalidResource)?;
        cursor = cursor.wrapping_add(
            (u32::from(last)
                .wrapping_sub(u32::from(first))
                .wrapping_add(4))
                & !3,
        );
        offsets[2] = cursor;
        let first = *data.get(6).ok_or(Error::InvalidResource)?;
        let last = *data.get(7).ok_or(Error::InvalidResource)?;
        cursor = cursor.wrapping_add(
            u32::from(last)
                .wrapping_sub(u32::from(first))
                .wrapping_mul(2)
                .wrapping_add(5)
                & !3,
        );
        offsets[3] = cursor;
        let count = dictionary_halfword(data, 46)?;
        cursor = cursor.wrapping_add((count + 3) & !3);
        offsets[4] = cursor;
        cursor = cursor.wrapping_add(count * 4);
        offsets[5] = cursor;
        cursor = cursor.wrapping_add(count * 4);
        offsets[6] = cursor;
        for (section, field) in (7..=11).zip((26..=34).step_by(2)) {
            cursor = cursor.wrapping_add(dictionary_halfword(data, field)? * 4 + 4);
            offsets[section] = cursor;
        }
        for (section, field, width) in [(12, 36, 2), (13, 38, 1), (14, 40, 1), (15, 42, 2)] {
            cursor = cursor.wrapping_add((dictionary_halfword(data, field)? * width + 3) & !3);
            offsets[section] = cursor;
        }
        offsets[16] = offset.wrapping_add(length);
        offsets[17] = offsets[16].wrapping_add(dictionary_word(data, 60)?.wrapping_add(3) & !3);
        offsets[18] = offsets[17].wrapping_add(dictionary_word(data, 64)?.wrapping_add(3) & !3);
        *sections = Some(DictionarySections { data, offsets });
        Ok(0)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OnewordEnv {
    pub heap: u32,
    pub log: u32,
    pub params: u32,
    pub res: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OnewordHandle {
    pub a: u32,
    pub b: u32,
}

#[derive(Clone, Debug)]
pub struct Oneword {
    pub env_ptr: u32,
    pub env: OnewordEnv,
    pub data: OnewordHandle,
    pub out_vt: u32,
    pub h: OnewordHandle,
    pub cfg_vt: u32,
    pub h2: OnewordHandle,
    pub trans_vt: u32,
    pub h3: OnewordHandle,
    pub vt3: u32,
    pub h4: OnewordHandle,
    pub clm_map: u32,
    pub buf40: [u32; 4],
    pub f50: i16,
    pub pad52: [u8; 2],
    pub buf54: [u32; 5],
    pub f68: u16,
    pub pad6a: [u8; 8],
    pub mode: [u8; 256],
    pub g2p_part: u8,
    pub f173: u8,
    pub lex: u32,
    pub dt_lhp: u8,
    pub no_phon_lhp: u8,
    pub no_rich_trans: u8,
    pub clm_variants: u8,
    pub clc_extra_main_trans: u8,
    pub extra_main_trans: u8,
    pub pad17e: [u8; 2],
    pub alloc: [u32; 4],
    pub f190: [u8; 20],
    pub lidenx: u8,
    pub f1a5: u8,
    pub lidenx_chars: [u8; 256],
    pub dps_multi_trans: u8,
    pub n_out_trans: u8,
    pub no_ext_clc: u8,
    pub pad2a9: [u8; 3],
    pub lang_code: u32,
    pub esc_tn: u32,
    pub g2p_on: u8,
    pub g2p_map: [[u8; 3]; 176],
    pub pad4c5: [u8; 3],
}

impl Default for Oneword {
    fn default() -> Self {
        Self {
            env_ptr: 0,
            env: OnewordEnv::default(),
            data: OnewordHandle::default(),
            out_vt: 0,
            h: OnewordHandle::default(),
            cfg_vt: 0,
            h2: OnewordHandle::default(),
            trans_vt: 0,
            h3: OnewordHandle::default(),
            vt3: 0,
            h4: OnewordHandle::default(),
            clm_map: 0,
            buf40: [0; 4],
            f50: 0,
            pad52: [0; 2],
            buf54: [0; 5],
            f68: 0,
            pad6a: [0; 8],
            mode: [0; 256],
            g2p_part: 0,
            f173: 0,
            lex: 0,
            dt_lhp: 0,
            no_phon_lhp: 0,
            no_rich_trans: 0,
            clm_variants: 0,
            clc_extra_main_trans: 0,
            extra_main_trans: 0,
            pad17e: [0; 2],
            alloc: [0; 4],
            f190: [0; 20],
            lidenx: 0,
            f1a5: 0,
            lidenx_chars: [0; 256],
            dps_multi_trans: 0,
            n_out_trans: 0,
            no_ext_clc: 0,
            pad2a9: [0; 3],
            lang_code: 0,
            esc_tn: 0,
            g2p_on: 0,
            g2p_map: [[0; 3]; 176],
            pad4c5: [0; 3],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OnewordDictEntry {
    pub name: u32,
    pub size: u32,
    pub id: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OnewordMarker {
    pub type_: u16,
    pub pos: i32,
    pub len: i32,
    pub val: u32,
}

pub type StrKey = Option<(u16, u32, u32)>;

pub enum OnewordText<'a> {
    Slice(&'a [u8]),
    Heap(u32),
}

pub trait OnewordHost {
    fn check(&mut self, handle: u32, tag: u32, size: u32) -> u32;
}

pub trait OnewordParams {
    fn get_dec(&mut self, params: u32, key: &[u8], d: u32, out: &mut [u8]) -> (u32, bool);
    fn get(&mut self, params: u32, key: &[u8], out: &mut [u8]) -> (u32, bool);
}

pub trait OnewordCfg {

    fn get_string(
        &mut self,
        handle: OnewordHandle,
        section: &[u8],
        key: &[u8],
        value: &mut [u8],
        len: &mut u16,
        sep: &mut u8,
    ) -> u32;

    fn add_word(&mut self, handle: OnewordHandle, mode: &[u8], word: &[u8]) -> u32;

    fn set_markers(
        &mut self,
        handle: OnewordHandle,
        a: u32,
        b: u32,
        markers: &[OnewordMarker],
    ) -> u32;

    fn normalize(&mut self, handle: OnewordHandle, mode: &[u8], word: &mut [u8], size: u32) -> u32;
}

pub trait OnewordOut {
    fn add_child(&mut self, a: u32, b: u32, type_: u32, parent: u32, id: &mut u16) -> u32;
    fn new_record(&mut self, a: u32, b: u32, c: u32, id: &mut u16) -> u32;
    fn insert_before(&mut self, a: u32, b: u32, id: u32, new_id: &mut u16) -> u32;
    fn get_first(&mut self, a: u32, b: u32, type_: u32, parent: u32, id: &mut u16) -> u32;
    fn get_next(&mut self, a: u32, b: u32, id: u32, next: &mut u16) -> u32;

    fn set_field(
        &mut self,
        a: u32,
        b: u32,
        id: u16,
        field: u32,
        size: u16,
        value: &[u8],
        x: &mut u16,
    ) -> u32;

    fn get_field(
        &mut self,
        a: u32,
        b: u32,
        id: u32,
        field: u32,
        count: u32,
        out: &mut [u8],
        n: &mut u16,
    ) -> u32;

    fn field_text(
        &mut self,
        a: u32,
        b: u32,
        id: u32,
        field: u32,
        out: &mut [u8],
        n: &mut u16,
    ) -> u32;

    fn field_text_refresh(&mut self, a: u32, b: u32, id: u32, field: u32, out: &mut [u8]);

    fn field_text_set(&mut self, a: u32, b: u32, id: u32, field: u32, index: usize, value: u8);

    fn field_markers(
        &mut self,
        a: u32,
        b: u32,
        id: u32,
        out: &mut [OnewordMarker],
        n: &mut u16,
    ) -> u32;
    fn has_field(&mut self, a: u32, b: u32, id: u32, field: u32, has: &mut u16) -> u32;
    fn remove(&mut self, a: u32, b: u32, id: u32) -> u32;

    fn marker_text(&self, val: u32) -> &[u16];
}

pub trait OnewordTrans {

    fn set_string(&mut self, handle: OnewordHandle, index: u32, text: &[u8]) -> u32;

    fn select(&mut self, handle: OnewordHandle, name: &[u8]) -> u32;

    fn get_string(
        &mut self,
        handle: OnewordHandle,
        index: u32,
        out: &mut [u8],
        len: &mut u16,
    ) -> u32;
    fn set_pos(&mut self, handle: OnewordHandle, pos: i32, z: u32) -> u32;
    fn get_pos(&mut self, handle: OnewordHandle, pos: &mut i32, z: u32) -> u32;
}

pub trait OnewordDict {

    fn word_single(
        &mut self,
        ctx: &mut Oneword,
        heap: &mut impl Heap,
        mode: &[u8],
        word: &mut [u8],
        separator: &mut u8,
        tmp: &mut [u32; 3],
    ) -> u32;

    fn word_multi(
        &mut self,
        ctx: &mut Oneword,
        heap: &mut impl Heap,
        mode: &[u8],
        word: &mut [u8],
    ) -> u32;

    fn lex_entry(
        &mut self,
        ctx: &mut Oneword,
        heap: &mut impl Heap,
        zero: u32,
        a: u32,
        b: u32,
        id: u32,
        mode: &[u8],
    ) -> u32;

    fn trans_fallback(
        &mut self,
        ctx: &mut Oneword,
        heap: &mut impl Heap,
        res: &[u8],
        empty1: &[u8],
        empty2: &[u8],
        zero: u32,
        one: u32,
    ) -> u32;

    fn dict_multi(
        &mut self,
        ctx: &mut Oneword,
        heap: &mut impl Heap,
        text_copy: &mut u32,
        start: u16,
        end: u16,
        mode: &[u8],
        a: u32,
        c: u32,
        tok: &mut u16,
        markers: &mut [OnewordMarker],
        mi: &mut u16,
        n_markers: &mut u16,
        base: i32,
        rich: u32,
    ) -> u32;

    fn dict_phon(
        &mut self,
        ctx: &mut Oneword,
        heap: &mut impl Heap,
        text: &[u8],
        word: &[u8],
        start: u16,
        end: u16,
        a: u32,
        c: u32,
        tok: &mut u16,
        markers: &mut [OnewordMarker],
        mi: &mut u16,
        n_markers: &mut u16,
        base: i32,
    ) -> u32;
}

pub trait OnewordLex {

    fn lookup(&mut self, lex: u32, word: &[u8]) -> (u32, OnewordDictEntry);

    fn has(&mut self, lex: u32) -> u32;

    fn begin(&mut self, lex: u32, first: u32, last: u32, iter: &mut u32) -> u32;

    fn next(&mut self, arena: &mut [u8], iter: u32, key: &mut u32, entry: &mut u32) -> u32;

    fn close(&mut self, iter: u32);
}

pub trait OnewordEngine {
    fn dt_set(&mut self, id: u32, type_: u8, val: i32) -> u32;
}

pub trait OnewordClm {

    fn map(&mut self, handle: OnewordHandle, code: &[u8], src: &[u8], dst: &mut [u8]) -> u32;
}

fn fetch_str(
    out: &mut impl OnewordOut,
    a: u32,
    c: u32,
    tok: u16,
    field: u32,
    buf: &mut [u8],
    key: &mut StrKey,
    version: u32,
    n: &mut u16,
) -> u32 {
    let rc = out.field_text(a, c, u32::from(tok), field, buf, n);
    if rc < 0x80000000 {
        *key = Some((tok, field, version));
    }
    rc
}

fn cached_str<'b>(
    out: &mut impl OnewordOut,
    a: u32,
    c: u32,
    buf: &'b mut [u8],
    key: &mut StrKey,
    version: u32,
) -> &'b [u8] {
    match *key {
        None => &[],
        Some((tok, field, fetched)) => {
            if fetched != version {
                out.field_text_refresh(a, c, u32::from(tok), field, buf);
                *key = Some((tok, field, version));
            }
            buf
        }
    }
}

fn fetched_str<'b>(
    out: &mut impl OnewordOut,
    a: u32,
    c: u32,
    tok: u16,
    field: u32,
    buf: &'b mut [u8],
    key: &mut StrKey,
    version: u32,
) -> &'b [u8] {
    match *key {
        Some((cached_tok, cached_field, fetched)) if cached_tok == tok && cached_field == field => {
            if fetched != version {
                out.field_text_refresh(a, c, u32::from(tok), field, buf);
                *key = Some((tok, field, version));
            }
            buf
        }
        _ => panic!("oneword string used without fetching its source"),
    }
}

fn get_num8(
    out: &mut impl OnewordOut,
    a: u32,
    c: u32,
    id: u16,
    field: u32,
    val: &mut u8,
    n: &mut u16,
) -> u32 {
    let mut buf = [0u8; 1];
    let rc = out.get_field(a, c, u32::from(id), field, 1, &mut buf, n);
    *val = buf[0];
    rc
}

fn get_num16(
    out: &mut impl OnewordOut,
    a: u32,
    c: u32,
    id: u16,
    field: u32,
    val: &mut u16,
    n: &mut u16,
) -> u32 {
    let mut buf = [0u8; 2];
    let rc = out.get_field(a, c, u32::from(id), field, 1, &mut buf, n);
    *val = u16::from_le_bytes(buf);
    rc
}

fn arena_copy_fixed(heap: &mut impl Heap, dst: u32, src: u32, n: u32) {
    let mut i = 0u32;
    while i < n {
        let byte = heap.arena()[src.wrapping_add(i) as usize];
        heap.arena()[dst.wrapping_add(i) as usize] = byte;
        i += 1;
        if byte == 0 {
            break;
        }
    }
    while i < n {
        heap.arena()[dst.wrapping_add(i) as usize] = 0;
        i += 1;
    }
}

fn read_marker(arena: &[u8], at: u32) -> OnewordMarker {
    let at = at as usize;
    OnewordMarker {
        type_: u16::from_le_bytes(arena[at..at + 2].try_into().unwrap()),
        pos: i32::from_le_bytes(arena[at + 12..at + 16].try_into().unwrap()),
        len: i32::from_le_bytes(arena[at + 16..at + 20].try_into().unwrap()),
        val: u32::from_le_bytes(arena[at + 20..at + 24].try_into().unwrap()),
    }
}

fn write_marker(arena: &mut [u8], at: u32, type_: u16, pos: i32, val: u32) {
    let at = at as usize;
    arena[at..at + 2].copy_from_slice(&type_.to_le_bytes());
    arena[at + 12..at + 16].copy_from_slice(&pos.to_le_bytes());
    arena[at + 20..at + 24].copy_from_slice(&val.to_le_bytes());
}

fn str_index(s: &[u8], c: u8) -> Option<usize> {
    let end = terminated_len(s) as usize;
    if c == 0 {
        return Some(end);
    }
    class_index(&s[..end], c)
}

fn arena_index(heap: &mut impl Heap, base: u32, c: u8) -> Option<u32> {
    str_index(&heap.arena()[base as usize..], c).map(|rel| base.wrapping_add(rel as u32))
}

fn arena_append(heap: &mut impl Heap, dst: u32, src: u32) {
    let at = terminated_len(&heap.arena()[dst as usize..]) as usize;
    let mut i = 0u32;
    loop {
        let byte = heap.arena()[src.wrapping_add(i) as usize];
        heap.arena()[dst.wrapping_add(at as u32).wrapping_add(i) as usize] = byte;
        i += 1;
        if byte == 0 {
            break;
        }
    }
}

fn free_out(heap: &mut impl Heap, text_copy: u32, word_buf: u32, clm_buf: u32, alt_buf: u32) {
    if text_copy != 0 {
        heap.release(text_copy);
    }
    if word_buf != 0 {
        heap.release(word_buf);
    }
    if clm_buf != 0 {
        heap.release(clm_buf);
    }
    if alt_buf != 0 {
        heap.release(alt_buf);
    }
}

macro_rules! oneword_checked {
    ($heap:expr, $mk:expr, $text_copy:expr, $word_buf:expr, $clm_buf:expr, $alt_buf:expr, $call:expr) => {{
        let checked = $call;
        if checked >= 0x80000000 {
            if $mk != 0 {
                $heap.release($mk);
            }
            free_out($heap, $text_copy, $word_buf, $clm_buf, $alt_buf);
            return checked;
        }
        checked
    }};
}

pub fn oneword_lookup_word(
    ctx: &mut Oneword,
    heap: &mut impl Heap,
    cfg: &mut impl OnewordCfg,
    out: &mut impl OnewordOut,
    trans: &mut impl OnewordTrans,
    dict: &mut impl OnewordDict,
    lex: &mut impl OnewordLex,
    out_version: &mut u32,
    a: u32,
    b: u32,
    c: u32,
    start: u16,
    end: u16,
    text: OnewordText<'_>,
) -> u32 {
    let len = i32::from(end) - i32::from(start);
    if len >= 128 {
        return 0;
    }
    let len = len as usize;
    let mut word = [0u8; 128];
    match text {
        OnewordText::Slice(slice) => {
            copy_fixed(&mut word, &slice[start as usize..], len as u32);
        }
        OnewordText::Heap(off) => {
            let from = off.wrapping_add(u32::from(start)) as usize;
            let arena = heap.arena();
            copy_fixed(&mut word, &arena[from..], len as u32);
        }
    }
    word[len] = 0;
    let rc = cfg.normalize(ctx.h, b"normal\0", &mut word, 128);
    if rc >= 0x80000000 {
        return rc;
    }
    for i in 1..5 {
        let off = ctx.buf54[i];
        heap.arena()[off as usize] = 0;
    }
    let word_len = terminated_len(&word);
    if word_len == 0 {
        return rc;
    }
    let mut rc;
    let mut done_jump = false;
    if word_len == 1 {
        let mut separator = b' ';
        let mut tmp = [0u32; 3];
        rc = dict.word_single(ctx, heap, b"normal\0", &mut word, &mut separator, &mut tmp);
        *out_version = out_version.wrapping_add(1);
    } else {
        rc = dict.word_multi(ctx, heap, b"normal\0", &mut word);
        *out_version = out_version.wrapping_add(1);
        if rc >= 0x80000000 {
            return rc;
        }
        let mut name = [0u8; 128];
        copy_terminated(&mut name, b"one_word\0").expect("table name fits");
        for (index, slot) in [(0u32, 0usize), (2, 1), (3, 2)] {
            let off = ctx.buf40[slot] as usize;
            let arena = heap.arena();
            let staged = terminated_len(&arena[off..]) as usize;
            let rc_set = trans.set_string(ctx.h2, index, &arena[off..off + staged]);
            if rc_set >= 0x80000000 {
                return rc_set;
            }
        }
        let (lex_rc, entry) = lex.lookup(ctx.lex, b"normal\0");
        if lex_rc < 0x80000000 {
            rc = dict.lex_entry(ctx, heap, 0, a, b, entry.id, b"normal\0");
            *out_version = out_version.wrapping_add(1);
        } else {
            let rc_select = trans.select(ctx.h2, &name);
            if rc_select >= 0x80000000 {
                rc = rc_select;
                done_jump = true;
            } else {
                let mut res_len = 0u16;
                let mut res = [0u8; 512];
                let rc_get = trans.get_string(ctx.h2, 0, &mut res, &mut res_len);
                if rc_get >= 0x80000000 {
                    rc = rc_get;
                    done_jump = true;
                } else {
                    res[res_len as usize] = 0;
                    rc = dict.trans_fallback(ctx, heap, &res, b"\0", b"\0", 0, 1);
                    *out_version = out_version.wrapping_add(1);
                }
            }
        }
    }
    if !done_jump
        && rc < 0x80000000
        && string_differs(&heap.arena()[ctx.buf54[1] as usize..], b"\0")
    {
        let mut id = 0u16;
        let mut x = 0u16;
        rc = out.new_record(a, b, c, &mut id);
        *out_version = out_version.wrapping_add(1);
        if rc >= 0x80000000 {
            return rc;
        }
        rc = out.set_field(a, b, id, 0, 1, &[15u8], &mut x);
        *out_version = out_version.wrapping_add(1);
        if rc >= 0x80000000 {
            return rc;
        }
        rc = out.set_field(a, b, id, 1, 1, &start.to_le_bytes()[..1], &mut x);
        *out_version = out_version.wrapping_add(1);
        if rc >= 0x80000000 {
            return rc;
        }
        rc = out.set_field(a, b, id, 2, 1, &end.to_le_bytes()[..1], &mut x);
        *out_version = out_version.wrapping_add(1);
        if rc >= 0x80000000 {
            return rc;
        }
        let s1 = ctx.buf54[1] as usize;
        let cut = terminated_len(&heap.arena()[s1..]) as usize;
        heap.arena()[s1 + cut - 1] = 0;
        let arena = heap.arena();
        let sub = terminated_len(&arena[s1 + 1..]).wrapping_add(1);
        let size = sub as u16;
        let end_sub = s1 + 1 + sub as usize;
        rc = out.set_field(a, b, id, 3, size, &arena[s1 + 1..end_sub], &mut x);
        *out_version = out_version.wrapping_add(1);
        if rc >= 0x80000000 {
            return rc;
        }
        rc = out.set_field(a, b, id, 5, 2, b"*\0", &mut x);
        *out_version = out_version.wrapping_add(1);
        if rc >= 0x80000000 {
            return rc;
        }
        rc = out.set_field(a, b, id, 6, 2, b"*\0", &mut x);
        *out_version = out_version.wrapping_add(1);
        if rc >= 0x80000000 {
            return rc;
        }
        let one = 1u16;
        rc = out.set_field(a, b, id, 7, 1, &one.to_le_bytes()[..1], &mut x);
        *out_version = out_version.wrapping_add(1);
        if rc >= 0x80000000 {
            return rc;
        }
        let lc = ctx.lang_code as usize;
        let arena = heap.arena();
        let lang_len = terminated_len(&arena[lc..]).wrapping_add(1);
        let size = lang_len as u16;
        let end_lang = lc + lang_len as usize;
        rc = out.set_field(a, b, id, 9, size, &arena[lc..end_lang], &mut x);
        *out_version = out_version.wrapping_add(1);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if rc == 0x89702004 || rc == 4 {
        0
    } else {
        rc
    }
}

pub fn oneword_process(
    ctx: &mut Oneword,
    heap: &mut impl Heap,
    host: &mut impl OnewordHost,
    params: &mut impl OnewordParams,
    cfg: &mut impl OnewordCfg,
    out: &mut impl OnewordOut,
    trans: &mut impl OnewordTrans,
    dict: &mut impl OnewordDict,
    lex: &mut impl OnewordLex,
    engine: &mut impl OnewordEngine,
    clm: &mut impl OnewordClm,
    b: u32,
    a: u32,
    c: u32,
    done: &mut u8,
    latin_addr: u32,
    normal_addr: u32,
    pos_key: &[u8; 8],
) -> u32 {
    let mut out_version = 0u32;
    let mut word_buf = 0u32;
    let mut clm_buf = 0u32;
    let mut alt_buf = 0u32;
    let mut text_copy: u32;
    let mut mk = 0u32;
    let mut first = true;
    let mut start = 0u16;
    let mut end = 0u16;
    let mut pos: u16;
    let mut tok = 0u16;
    let mut n = 0u16;
    let mut root = 0u16;
    let mut wpos = 0u16;
    let mut w2: u16;
    let mut w = 0u16;
    let mut x = 0u16;
    let mut has = 0u16;
    let mut len: u16;
    let mut mi = 0u16;
    let mut n_markers = 0u16;
    let mut n_mk = 0u16;
    let mut cap_mk = 0u16;
    let mut n_del = 0u8;
    let mut wtype = 0u8;
    let mut type_ = 0u8;
    let mut sep = 0u8;
    let mut same_start = false;
    let mut keep_alt = false;
    let mut pos_char = 0u8;
    let mut mode = b'N';
    let base: i32;
    let mut text_pos = 0i32;
    let mut del_ids = [0u16; 128];
    let mut marks = [OnewordMarker::default(); 64];
    let mut mk_parse = [OnewordMarker::default(); 64];
    let mut key = *pos_key;
    let mut norm_str = [0u8; 128];
    let mut tmp = [0u8; 84];
    let mut num_buf = [0u8; 4];
    let mut code = [0u8; 4];
    let mut text_buf = [0u8; 1024];
    let mut text_key: StrKey = None;
    let mut s3 = [0u8; 256];
    let mut k3: StrKey = None;
    let mut s4 = [0u8; 256];
    let mut k4: StrKey = None;
    let mut s5 = [0u8; 256];
    let mut k5: StrKey = None;
    let mut s6 = [0u8; 256];
    let mut k6: StrKey = None;
    let mut alt = [0u8; 256];
    let mut k_alt: StrKey = None;
    let mut lang = [0u8; 256];
    let mut k_lang: StrKey = None;
    let mut out0 = [0u8; 512];
    let mut out2 = [0u8; 512];
    let mut cfg_val = [0u8; 128];
    let mut opt = [0u8; 64];
    let mut clm_val = [0u8; 64];
    let mut rc;

    rc = host.check(b, 0xf384, 0x4c8);
    if rc >= 0x80000000 {
        return 0x89b02008;
    }
    *done = 1;
    let (rc_clmvars, some) = params.get_dec(ctx.env.params, b"clmvariants\0", 0, &mut opt);
    if rc_clmvars >= 0x80000000 || !some || !caseless_differs(&opt, b"no\0") {
        ctx.clm_variants = 0;
    } else {
        ctx.clm_variants = 1;
    }
    let (rc_extra, some) = params.get(ctx.env.params, b"clcmlextramaintrans\0", &mut opt);
    if rc_extra >= 0x80000000
        || !some
        || !caseless_differs(&opt, b"no\0")
        || ctx.extra_main_trans == 0
    {
        ctx.clc_extra_main_trans = 0;
    } else {
        ctx.clc_extra_main_trans = 1;
    }
    let (rc_nout, some) =
        params.get_dec(ctx.env.params, b"numberoutputtranscriptions\0", 0, &mut opt);
    ctx.n_out_trans = 1;
    if rc_nout < 0x80000000 && some {
        let nt = parse_int(&opt) as u8;
        ctx.n_out_trans = nt;
        if i32::from(nt) - 1 <= i32::from(ctx.dps_multi_trans) {
            ctx.dps_multi_trans = nt.wrapping_sub(1);
        }
    }
    let (rc_rich, some) = params.get_dec(ctx.env.params, b"norichtranscription\0", 0, &mut opt);
    if rc_rich >= 0x80000000 || !some || !caseless_differs(&opt, b"false\0") {
        ctx.no_rich_trans = 0;
    } else {
        ctx.no_rich_trans = 1;
    }
    let (rc_fecfg, some) = params.get_dec(ctx.env.params, b"fecfg\0", 0, &mut opt);
    if ctx.no_rich_trans != 0
        && rc_fecfg < 0x80000000
        && some
        && caseless_differs(&opt, b"mpthree\0")
        && caseless_differs(&opt, b"mpthreeml\0")
        && caseless_differs(&opt, b"vadvde\0")
    {
        ctx.no_rich_trans = 0;
    }
    if lex.has(ctx.lex) != 0 {
        let mut score = [0u8; 64];
        let mut hist = [0u8; 64];
        let mut variant = [0u8; 64];
        let mut ntrans = [0u8; 64];
        let (rc_score, has_score) =
            params.get(ctx.env.params, b"decisiontreescorepruning\0", &mut score);
        let use_score = rc_score < 0x80000000 && has_score;
        let (rc_hist, has_hist) =
            params.get(ctx.env.params, b"decisiontreehistogrampruning\0", &mut hist);
        let use_hist = rc_hist < 0x80000000 && has_hist;
        let (rc_variant, has_variant) = params.get(
            ctx.env.params,
            b"decisiontreevariantpruning\0",
            &mut variant,
        );
        let use_variant = rc_variant < 0x80000000 && has_variant;
        let (rc_ntrans, has_ntrans) =
            params.get(ctx.env.params, b"numberoutputtranscriptions\0", &mut ntrans);
        let use_ntrans = rc_ntrans < 0x80000000 && has_ntrans;
        let mut iter = 0u32;
        if lex.begin(ctx.lex, 0, 1, &mut iter) < 0x80000000 {
            let mut dkey = 0u32;
            let mut entry = 0u32;
            while lex.next(heap.arena(), iter, &mut dkey, &mut entry) < 0x80000000 {
                if use_score {
                    rc = engine.dt_set(
                        read_offset(heap.arena(), entry.wrapping_add(8)),
                        1,
                        parse_int(&score),
                    );
                    if rc != 0 {
                        return rc.wrapping_add(0x1ea0) | 0x89b02000;
                    }
                }
                if use_hist {
                    rc = engine.dt_set(
                        read_offset(heap.arena(), entry.wrapping_add(8)),
                        2,
                        parse_int(&hist),
                    );
                    if rc != 0 {
                        return rc.wrapping_add(0x1ea0) | 0x89b02000;
                    }
                }
                if use_variant {
                    rc = engine.dt_set(
                        read_offset(heap.arena(), entry.wrapping_add(8)),
                        5,
                        parse_int(&variant),
                    );
                    if rc != 0 {
                        return rc.wrapping_add(0x1ea0) | 0x89b02000;
                    }
                }
                if use_ntrans {
                    rc = engine.dt_set(
                        read_offset(heap.arena(), entry.wrapping_add(8)),
                        0,
                        parse_int(&ntrans),
                    );
                    if rc != 0 {
                        return rc.wrapping_add(0x1ea0) | 0x89b02000;
                    }
                }
            }
            lex.close(iter);
        }
    }
    rc = out.get_first(a, c, 1, 0, &mut root);
    if rc >= 0x80000000 {
        return rc;
    }
    if out.has_field(a, c, u32::from(root), 0, &mut has) >= 0x80000000 || has != 1 {
        return rc;
    }
    rc = fetch_str(
        out,
        a,
        c,
        root,
        0,
        &mut text_buf,
        &mut text_key,
        out_version,
        &mut n,
    );
    if rc >= 0x80000000 || n <= 1 {
        return rc;
    }
    text_copy = match heap.allocate(terminated_len(&text_buf).wrapping_add(1)) {
        Some(off) => off,
        None => return 0x89b0200a,
    };
    copy_terminated(&mut heap.arena()[text_copy as usize..], &text_buf)
        .expect("textCopy fits its block");
    del_ids.fill(0);

    oneword_checked!(
        heap,
        mk,
        text_copy,
        word_buf,
        clm_buf,
        alt_buf,
        out.field_markers(a, c, u32::from(root), &mut marks, &mut n_markers)
    );
    base = marks[0].pos;
    ctx.f50 = 0;
    while mi < n_markers && marks[mi as usize].pos == base {
        mi += 1;
    }
    oneword_checked!(
        heap,
        mk,
        text_copy,
        word_buf,
        clm_buf,
        alt_buf,
        trans.get_pos(ctx.h2, &mut text_pos, 0)
    );
    ctx.f1a5 = 0;
    oneword_checked!(
        heap,
        mk,
        text_copy,
        word_buf,
        clm_buf,
        alt_buf,
        out.get_first(a, c, 3, u32::from(root), &mut tok)
    );
    len = 0xffff;
    oneword_checked!(
        heap,
        mk,
        text_copy,
        word_buf,
        clm_buf,
        alt_buf,
        cfg.get_string(
            ctx.h,
            b"fecfg\0",
            b"normal\0",
            &mut cfg_val,
            &mut len,
            &mut sep
        )
    );
    copy_terminated(&mut norm_str, b"normal\0").expect("mode fits");
    if len != 0 {
        if let Some(at) = str_index(&cfg_val, sep) {
            cfg_val[at] = 0;
        }
        copy_terminated(&mut norm_str, &cfg_val).expect("config value fits its 128");
    }
    if ctx.f173 == 1 {
        mk = match heap.allocate_zeroed(1, 11 * 24) {
            Some(off) => off,
            None => {
                free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
                return 0x89b0200a;
            }
        };
        cap_mk = 10;
        n_mk = 2;
        let arena = heap.arena();
        write_marker(arena, mk, 0x4000, base, 0);
        write_marker(arena, mk.wrapping_add(24), 36, base, normal_addr);
    }
    while tok != 0 {
        let prev: u16;
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            get_num8(out, a, c, tok, 0, &mut type_, &mut n)
        );
        if type_ == 5 || type_ == 6 || type_ == 7 {
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                get_num16(out, a, c, tok, 1, &mut start, &mut n)
            );
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                get_num16(out, a, c, tok, 2, &mut end, &mut n)
            );
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                fetch_str(out, a, c, tok, 4, &mut s4, &mut k4, out_version, &mut n)
            );
            if !string_differs(
                fetched_str(out, a, c, tok, 4, &mut s4, &mut k4, out_version),
                b"phon\0",
            ) {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    fetch_str(out, a, c, tok, 3, &mut s3, &mut k3, out_version, &mut n)
                );
            }
            if type_ == 5 {
                has = 0;
                k6 = None;
                if out.has_field(a, c, u32::from(tok), 6, &mut has) < 0x80000000 && has == 1 {
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        fetch_str(out, a, c, tok, 6, &mut s6, &mut k6, out_version, &mut n)
                    );
                }
            }
            if first {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.add_child(a, c, 2, u32::from(root), &mut w)
                );
                out_version = out_version.wrapping_add(1);
                first = false;
            } else {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.new_record(a, c, u32::from(w), &mut w)
                );
                out_version = out_version.wrapping_add(1);
            }
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, w, 0, 1, &[type_], &mut x)
            );
            out_version = out_version.wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, w, 1, 1, &start.to_le_bytes()[..1], &mut x)
            );
            out_version = out_version.wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, w, 2, 1, &end.to_le_bytes()[..1], &mut x)
            );
            out_version = out_version.wrapping_add(1);
            let s4v = fetched_str(out, a, c, tok, 4, &mut s4, &mut k4, out_version);
            let s4len = terminated_len(s4v).wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, w, 4, s4len as u16, &s4v[..s4len as usize], &mut x)
            );
            out_version = out_version.wrapping_add(1);
            if !string_differs(
                fetched_str(out, a, c, tok, 4, &mut s4, &mut k4, out_version),
                b"phon\0",
            ) {
                let s3v = fetched_str(out, a, c, tok, 3, &mut s3, &mut k3, out_version);
                let s3len = terminated_len(s3v).wrapping_add(1);
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.set_field(a, c, w, 3, s3len as u16, &s3v[..s3len as usize], &mut x)
                );
                out_version = out_version.wrapping_add(1);
            }
            if type_ == 5 && k6.is_some() {
                let s6v = fetched_str(out, a, c, tok, 6, &mut s6, &mut k6, out_version);
                let s6len = terminated_len(s6v).wrapping_add(1);
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.set_field(a, c, w, 9, s6len as u16, &s6v[..s6len as usize], &mut x)
                );
                out_version = out_version.wrapping_add(1);
            }
        }
        if ctx.f173 == 1 && type_ == 5 && k4.is_some() {
            let mut new_mode: u8;
            let s4v = cached_str(out, a, c, &mut s4, &mut k4, out_version);
            if !string_differs(s4v, b"phon\0") || !string_differs(s4v, b"prompt\0") {
                new_mode = b'N';
            } else {
                if ctx.esc_tn == 0 {
                    if mk != 0 {
                        heap.release(mk);
                    }
                    free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
                    return 0x89b02006;
                }
                let esc_off = ctx.esc_tn as usize;
                let hit = find_substring(&heap.arena()[esc_off..], s4v);
                if !string_differs(s4v, b"latin\0") {
                    new_mode = b'L';
                } else if !string_differs(s4v, &norm_str) {
                    new_mode = b'N';
                } else {
                    let s4len = terminated_len(s4v) as usize;
                    let piped = match hit {
                        None => true,
                        Some(at) => heap.arena()[esc_off + at + s4len] != b'|',
                    };
                    if piped {
                        new_mode = b'L';
                    } else {
                        let mut k = s4len as u16;
                        let mut si = 0usize;
                        new_mode = b'N';
                        while k != 0 {
                            if s4v[si] == b'_' {
                                new_mode = b'L';
                            }
                            si += 1;
                            k -= 1;
                        }
                    }
                }
            }
            if mode != new_mode {
                if start == 0 {
                    let val = if new_mode == b'L' {
                        latin_addr
                    } else {
                        normal_addr
                    };
                    let at = mk.wrapping_add(24) as usize;
                    heap.arena()[at + 20..at + 24].copy_from_slice(&val.to_le_bytes());
                } else {
                    if n_mk.wrapping_add(1) > cap_mk {
                        let new_size = u32::from(cap_mk).wrapping_add(11).wrapping_mul(24);
                        mk = match heap.reallocate(mk, new_size) {
                            Some(off) => off,
                            None => {
                                if mk != 0 {
                                    heap.release(mk);
                                }
                                free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
                                return 0x89b0200a;
                            }
                        };
                        cap_mk = cap_mk.wrapping_add(10);
                    }
                    let at = mk.wrapping_add(u32::from(n_mk).wrapping_mul(24));
                    let mpos = marks[0].pos.wrapping_add(i32::from(start));
                    let val = if new_mode == b'L' {
                        latin_addr
                    } else {
                        normal_addr
                    };
                    write_marker(heap.arena(), at, 36, mpos, val);
                    n_mk = n_mk.wrapping_add(1);
                }
            }
            mode = new_mode;
        }
        prev = tok;
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.get_next(a, c, u32::from(prev), &mut tok)
        );
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.remove(a, c, u32::from(prev))
        );
        out_version = out_version.wrapping_add(1);
    }
    if ctx.f173 == 1 {
        let arena = heap.arena();
        for i in 0..n_mk {
            mk_parse[i as usize] =
                read_marker(arena, mk.wrapping_add(u32::from(i).wrapping_mul(24)));
        }
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            cfg.set_markers(ctx.h, a, c, &mk_parse[..n_mk as usize])
        );
    } else {
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            cfg.set_markers(ctx.h, a, c, &marks[..n_markers as usize])
        );
    }

    oneword_checked!(
        heap,
        mk,
        text_copy,
        word_buf,
        clm_buf,
        alt_buf,
        out.get_first(a, c, 3, u32::from(root), &mut tok)
    );
    oneword_checked!(
        heap,
        mk,
        text_copy,
        word_buf,
        clm_buf,
        alt_buf,
        out.get_first(a, c, 2, u32::from(root), &mut w)
    );
    oneword_checked!(
        heap,
        mk,
        text_copy,
        word_buf,
        clm_buf,
        alt_buf,
        get_num16(out, a, c, w, 1, &mut wpos, &mut n)
    );
    if wpos != 0 {
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.insert_before(a, c, u32::from(w), &mut w)
        );
        out_version = out_version.wrapping_add(1);
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            get_num8(out, a, c, tok, 0, &mut type_, &mut n)
        );
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            get_num16(out, a, c, tok, 1, &mut start, &mut n)
        );
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            get_num16(out, a, c, tok, 2, &mut end, &mut n)
        );
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.set_field(a, c, w, 0, 1, &[type_], &mut x)
        );
        out_version = out_version.wrapping_add(1);
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.set_field(a, c, w, 1, 1, &start.to_le_bytes()[..1], &mut x)
        );
        out_version = out_version.wrapping_add(1);
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.set_field(a, c, w, 2, 1, &end.to_le_bytes()[..1], &mut x)
        );
        out_version = out_version.wrapping_add(1);
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.get_next(a, c, u32::from(tok), &mut tok)
        );
    }
    start = 0;
    {
        let mut last = w;
        while wpos < start && w != 0 {
            last = w;
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.get_next(a, c, u32::from(w), &mut w)
            );
            if w != 0 {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    get_num16(out, a, c, w, 1, &mut wpos, &mut n)
                );
            } else {
                wpos = terminated_len(fetched_str(
                    out,
                    a,
                    c,
                    root,
                    0,
                    &mut text_buf,
                    &mut text_key,
                    out_version,
                )) as u16;
            }
        }
        w = last;
    }
    while tok != 0 {
        let mut last: u16;
        let prev: u16;
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            get_num8(out, a, c, tok, 0, &mut type_, &mut n)
        );
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            get_num16(out, a, c, tok, 1, &mut start, &mut n)
        );
        last = w;
        while wpos < start && w != 0 {
            last = w;
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.get_next(a, c, u32::from(w), &mut w)
            );
            if w != 0 {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    get_num16(out, a, c, w, 1, &mut wpos, &mut n)
                );
            } else {
                wpos = terminated_len(fetched_str(
                    out,
                    a,
                    c,
                    root,
                    0,
                    &mut text_buf,
                    &mut text_key,
                    out_version,
                )) as u16;
            }
        }
        if start != wpos {
            w = last;
        }
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            get_num16(out, a, c, tok, 2, &mut end, &mut n)
        );
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            get_num8(out, a, c, w, 0, &mut wtype, &mut n)
        );
        if wtype == 5 && start == wpos {
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.insert_before(a, c, u32::from(w), &mut w)
            );
            out_version = out_version.wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, w, 0, 1, &[type_], &mut x)
            );
            out_version = out_version.wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, w, 1, 1, &start.to_le_bytes()[..1], &mut x)
            );
            out_version = out_version.wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, w, 2, 1, &end.to_le_bytes()[..1], &mut x)
            );
            out_version = out_version.wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.get_next(a, c, u32::from(w), &mut w)
            );
        } else {
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.new_record(a, c, u32::from(w), &mut w)
            );
            out_version = out_version.wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, w, 0, 1, &[type_], &mut x)
            );
            out_version = out_version.wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, w, 1, 1, &start.to_le_bytes()[..1], &mut x)
            );
            out_version = out_version.wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, w, 2, 1, &end.to_le_bytes()[..1], &mut x)
            );
            out_version = out_version.wrapping_add(1);
        }
        prev = tok;
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.get_next(a, c, u32::from(prev), &mut tok)
        );
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.remove(a, c, u32::from(prev))
        );
        out_version = out_version.wrapping_add(1);
    }

    oneword_checked!(
        heap,
        mk,
        text_copy,
        word_buf,
        clm_buf,
        alt_buf,
        out.get_first(a, c, 2, u32::from(root), &mut tok)
    );
    ctx.f68 = tok;
    w2 = tok;
    while tok != 0 && w2 != 0 {
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            get_num8(out, a, c, w2, 0, &mut wtype, &mut n)
        );
        if wtype == 5 {
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                get_num16(out, a, c, w2, 1, &mut start, &mut n)
            );
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                get_num16(out, a, c, w2, 2, &mut end, &mut n)
            );
            if start != end {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    fetch_str(out, a, c, w2, 4, &mut s4, &mut k4, out_version, &mut n)
                );
                if string_differs(
                    fetched_str(out, a, c, w2, 4, &mut s4, &mut k4, out_version),
                    b"phon\0",
                ) {
                    let mut rich = 0u32;
                    has = 0;
                    if out.has_field(a, c, u32::from(w2), 9, &mut has) < 0x80000000 && has == 1 {
                        rich = 1;
                    }
                    let mode_v = fetched_str(out, a, c, w2, 4, &mut s4, &mut k4, out_version);
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        dict.dict_multi(
                            ctx,
                            heap,
                            &mut text_copy,
                            start,
                            end,
                            mode_v,
                            a,
                            c,
                            &mut tok,
                            &mut marks,
                            &mut mi,
                            &mut n_markers,
                            base,
                            rich,
                        )
                    );
                    out_version = out_version.wrapping_add(1);
                } else {
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        fetch_str(out, a, c, w2, 3, &mut s3, &mut k3, out_version, &mut n)
                    );
                    let text_v = fetched_str(
                        out,
                        a,
                        c,
                        root,
                        0,
                        &mut text_buf,
                        &mut text_key,
                        out_version,
                    );
                    let s3v = fetched_str(out, a, c, w2, 3, &mut s3, &mut k3, out_version);
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        dict.dict_phon(
                            ctx,
                            heap,
                            text_v,
                            s3v,
                            start,
                            end,
                            a,
                            c,
                            &mut tok,
                            &mut marks,
                            &mut mi,
                            &mut n_markers,
                            base,
                        )
                    );
                    out_version = out_version.wrapping_add(1);
                }
                w2 = tok;
            } else {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.get_next(a, c, u32::from(w2), &mut w2)
                );
            }
        } else {
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.get_next(a, c, u32::from(w2), &mut w2)
            );
        }
    }

    if ctx.f50 > 0 {
        while mi < n_markers {
            marks[mi as usize].pos = marks[mi as usize].pos.wrapping_add(i32::from(ctx.f50));
            mi = mi.wrapping_add(1);
        }
        let mut wstart = 0u16;
        let mut wend = 0u16;
        if ctx.f68 != 0 {
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                get_num16(out, a, c, ctx.f68, 1, &mut wstart, &mut n)
            );
        }
        while {
            let text_v = fetched_str(
                out,
                a,
                c,
                root,
                0,
                &mut text_buf,
                &mut text_key,
                out_version,
            );
            i32::from(terminated_len(text_v) as i16) >= i32::from(wstart) && ctx.f68 != 0
        } {
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                get_num8(out, a, c, ctx.f68, 0, &mut wtype, &mut n)
            );
            if wtype == 5 || wtype == 6 || wtype == 4 || wtype == 9 || wtype == 10 || wtype == 7 {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    get_num16(out, a, c, ctx.f68, 2, &mut wend, &mut n)
                );
                wstart = wstart.wrapping_add(ctx.f50 as u16);
                wend = wend.wrapping_add(ctx.f50 as u16);
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.set_field(a, c, ctx.f68, 1, 1, &wstart.to_le_bytes()[..1], &mut x)
                );
                out_version = out_version.wrapping_add(1);
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.set_field(a, c, ctx.f68, 2, 1, &wend.to_le_bytes()[..1], &mut x)
                );
                out_version = out_version.wrapping_add(1);
            }
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.get_next(a, c, u32::from(ctx.f68), &mut ctx.f68)
            );
            if ctx.f68 != 0 {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    get_num16(out, a, c, ctx.f68, 1, &mut wstart, &mut n)
                );
            }
        }
    }
    text_pos = text_pos.wrapping_add(i32::from(ctx.f50));
    oneword_checked!(
        heap,
        mk,
        text_copy,
        word_buf,
        clm_buf,
        alt_buf,
        trans.set_pos(ctx.h2, text_pos, 0)
    );
    marks[0].len = terminated_len(&heap.arena()[text_copy as usize..]) as i32;
    let tc = text_copy as usize;
    let arena = heap.arena();
    let tc_len = terminated_len(&arena[tc..]).wrapping_add(1);
    let end_tc = tc + tc_len as usize;
    oneword_checked!(
        heap,
        mk,
        text_copy,
        word_buf,
        clm_buf,
        alt_buf,
        out.set_field(a, c, root, 0, tc_len as u16, &arena[tc..end_tc], &mut x)
    );
    out_version = out_version.wrapping_add(1);

    oneword_checked!(
        heap,
        mk,
        text_copy,
        word_buf,
        clm_buf,
        alt_buf,
        out.get_first(a, c, 2, u32::from(root), &mut tok)
    );
    {
        let mut prev = 0u16;
        while tok != 0 {
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                get_num8(out, a, c, tok, 0, &mut wtype, &mut n)
            );
            if wtype <= 3 {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.remove(a, c, u32::from(tok))
                );
                out_version = out_version.wrapping_add(1);
                tok = prev;
            }
            if tok == 0 {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.get_first(a, c, 2, u32::from(root), &mut tok)
                );
            } else {
                prev = tok;
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.get_next(a, c, u32::from(prev), &mut tok)
                );
            }
        }
    }

    mi = 0;
    {
        let mut cur_start = terminated_len(&heap.arena()[text_copy as usize..]) as u16;
        rc = oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.get_first(a, c, 2, u32::from(root), &mut tok)
        );
        while tok != 0 {
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                get_num8(out, a, c, tok, 0, &mut wtype, &mut n)
            );
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                get_num16(out, a, c, tok, 1, &mut start, &mut n)
            );
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                get_num16(out, a, c, tok, 2, &mut end, &mut n)
            );
            if wtype == 9 || wtype == 4 {
                has = 0;
                if out.has_field(a, c, u32::from(tok), 11, &mut has) < 0x80000000 && has == 1 {
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        fetch_str(
                            out,
                            a,
                            c,
                            tok,
                            11,
                            &mut alt,
                            &mut k_alt,
                            out_version,
                            &mut n
                        )
                    );
                    if let Some(at) = find_substring(&alt, b"\x12;_") {
                        alt[at] = 0;
                        out.field_text_set(a, c, u32::from(tok), 11, at, 0);
                    }
                    if let Some(at) = find_substring(&alt, b";_") {
                        alt[at] = 0;
                        out.field_text_set(a, c, u32::from(tok), 11, at, 0);
                    }
                }
            } else if wtype != 10 {
                rc = oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.get_next(a, c, u32::from(tok), &mut tok)
                );
                continue;
            }
            if start == cur_start {
                same_start = true;
            } else {
                if same_start && keep_alt {
                    let mut k = 0u8;
                    while k < n_del {
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            out.remove(a, c, u32::from(del_ids[k as usize]))
                        );
                        out_version = out_version.wrapping_add(1);
                        k = k.wrapping_add(1);
                    }
                }
                n_del = 0;
                same_start = false;
                keep_alt = false;
                cur_start = start;
                pos_char = 0;
            }
            while mi < n_markers
                && ((marks[mi as usize].pos.wrapping_sub(base) as i16)
                    < (start.wrapping_add(1) as i16))
            {
                let m = marks[mi as usize];
                if m.type_ == 37 {
                    let new_size = terminated_len16(out.marker_text(m.val)).wrapping_add(1);
                    word_buf = match heap.reallocate(word_buf, new_size) {
                        Some(off) => off,
                        None => {
                            if mk != 0 {
                                heap.release(mk);
                            }
                            free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
                            return 0x89b0200a;
                        }
                    };
                    let text16 = out.marker_text(m.val);
                    let mut k = 0u16;
                    pos = 0;
                    while text16[pos as usize] != 0 {
                        let ch = text16[pos as usize] as u8;
                        if ch != b' ' {
                            heap.arena()[word_buf as usize + k as usize] = ch;
                            k = k.wrapping_add(1);
                        }
                        pos = pos.wrapping_add(1);
                    }
                    heap.arena()[word_buf as usize + k as usize] = 0;
                    let wb = word_buf as usize;
                    let arena = heap.arena();
                    let wb_len = terminated_len(&arena[wb..]).wrapping_add(1);
                    let end_wb = wb + wb_len as usize;
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        out.set_field(a, c, tok, 10, wb_len as u16, &arena[wb..end_wb], &mut x)
                    );
                    out_version = out_version.wrapping_add(1);
                } else if m.type_ == 9 {
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        fetch_str(out, a, c, tok, 3, &mut s3, &mut k3, out_version, &mut n)
                    );
                    if let Some(at) = str_index(&s3, b'\'') {
                        s3[at] = 0x13;
                        out.field_text_set(a, c, u32::from(tok), 3, at, 0x13);
                    }
                } else if m.type_ == 14 && m.val != 64 {
                    len = 0xffff;
                    key[4] = m.val as u8;
                    key[5] = 0;
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        cfg.get_string(ctx.h, b"fecfg\0", &key, &mut cfg_val, &mut len, &mut sep)
                    );
                    if len != 0 {
                        pos_char = cfg_val[0];
                    } else {
                        pos_char = key[4];
                    }
                }
                mi = mi.wrapping_add(1);
            }
            if pos_char != 0 {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    fetch_str(out, a, c, tok, 5, &mut s5, &mut k5, out_version, &mut n)
                );
                if str_index(
                    fetched_str(out, a, c, tok, 5, &mut s5, &mut k5, out_version),
                    pos_char,
                )
                .is_some()
                {
                    keep_alt = true;
                } else {
                    del_ids[n_del as usize] = tok;
                    n_del = n_del.wrapping_add(1);
                }
                s5[0] = pos_char;
                out.field_text_set(a, c, u32::from(tok), 5, 0, pos_char);
                s5[1] = 0;
                out.field_text_set(a, c, u32::from(tok), 5, 1, 0);
            }
            if wtype == 9 {
                let mut wstart = 0u16;
                let wend = 0u16;
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    fetch_str(out, a, c, tok, 3, &mut s3, &mut k3, out_version, &mut n)
                );
                if str_index(
                    fetched_str(out, a, c, tok, 3, &mut s3, &mut k3, out_version),
                    b' ',
                )
                .is_some()
                {
                    let mut f7 = 0u16;
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        get_num16(out, a, c, tok, 2, &mut end, &mut n)
                    );
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        get_num16(out, a, c, tok, 7, &mut f7, &mut n)
                    );
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        fetch_str(
                            out,
                            a,
                            c,
                            tok,
                            9,
                            &mut lang,
                            &mut k_lang,
                            out_version,
                            &mut n
                        )
                    );
                    has = 0;
                    if out.has_field(a, c, u32::from(tok), 8, &mut has) < 0x80000000 && has == 1 {
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            get_num16(out, a, c, tok, 8, &mut wstart, &mut n)
                        );
                    }
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        fetch_str(out, a, c, tok, 5, &mut s5, &mut k5, out_version, &mut n)
                    );
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        fetch_str(out, a, c, tok, 6, &mut s6, &mut k6, out_version, &mut n)
                    );
                    for (slot, field, buf, key) in [
                        (1usize, 3u32, &mut s3 as &mut [u8], &mut k3 as &mut StrKey),
                        (2, 5, &mut s5 as &mut [u8], &mut k5 as &mut StrKey),
                        (3, 6, &mut s6 as &mut [u8], &mut k6 as &mut StrKey),
                    ] {
                        let v = fetched_str(out, a, c, tok, field, buf, key, out_version);
                        let need = terminated_len(v).wrapping_add(1);
                        if need > 128 {
                            ctx.buf54[slot] = match heap.reallocate(ctx.buf54[slot], need) {
                                Some(off) => off,
                                None => {
                                    if mk != 0 {
                                        heap.release(mk);
                                    }
                                    free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
                                    return 0x89b0200a;
                                }
                            };
                        }
                        copy_terminated(&mut heap.arena()[ctx.buf54[slot] as usize..], v)
                            .expect("split string fits its block");
                    }
                    let mut ph = ctx.buf54[1];
                    let mut pos5 = ctx.buf54[2];
                    let mut pos6 = ctx.buf54[3];
                    let mut sp3 = arena_index(heap, ph, b' ');
                    let mut sp5 = arena_index(heap, pos5, b' ');
                    let mut sp6 = arena_index(heap, pos6, b' ');
                    pos = start;
                    while let Some(s3off) = sp3 {
                        loop {
                            let ch = heap.arena()[text_copy.wrapping_add(u32::from(pos)) as usize];
                            if ch == 0
                                || ch == b' '
                                || ch == b'\t'
                                || ch == b'\n'
                                || ch == b'\r'
                                || ch == b'_'
                            {
                                break;
                            }
                            pos = pos.wrapping_add(1);
                        }
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            out.set_field(a, c, tok, 2, 1, &pos.to_le_bytes()[..1], &mut x)
                        );
                        out_version = out_version.wrapping_add(1);
                        pos = pos.wrapping_add(1);
                        heap.arena()[s3off as usize] = 0;
                        let arena = heap.arena();
                        let ph_len = terminated_len(&arena[ph as usize..]).wrapping_add(1);
                        let end_ph = ph as usize + ph_len as usize;
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            out.set_field(
                                a,
                                c,
                                tok,
                                3,
                                ph_len as u16,
                                &arena[ph as usize..end_ph],
                                &mut x,
                            )
                        );
                        out_version = out_version.wrapping_add(1);
                        ph = s3off.wrapping_add(1);
                        if let Some(s5off) = sp5 {
                            heap.arena()[s5off as usize] = 0;
                        }
                        let arena = heap.arena();
                        let p5_len = terminated_len(&arena[pos5 as usize..]).wrapping_add(1);
                        let end_p5 = pos5 as usize + p5_len as usize;
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            out.set_field(
                                a,
                                c,
                                tok,
                                5,
                                p5_len as u16,
                                &arena[pos5 as usize..end_p5],
                                &mut x,
                            )
                        );
                        out_version = out_version.wrapping_add(1);
                        if let Some(s5off) = sp5 {
                            pos5 = s5off.wrapping_add(1);
                        }
                        let s6off = sp6.expect("str6b holds a space wherever str3 does");
                        heap.arena()[s6off as usize] = 0;
                        let arena = heap.arena();
                        let p6_len = terminated_len(&arena[pos6 as usize..]).wrapping_add(1);
                        let end_p6 = pos6 as usize + p6_len as usize;
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            out.set_field(
                                a,
                                c,
                                tok,
                                6,
                                p6_len as u16,
                                &arena[pos6 as usize..end_p6],
                                &mut x,
                            )
                        );
                        out_version = out_version.wrapping_add(1);
                        pos6 = s6off.wrapping_add(1);
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            out.set_field(a, c, tok, 8, 1, &wend.to_le_bytes()[..1], &mut x)
                        );
                        out_version = out_version.wrapping_add(1);
                        let mut prev = tok;
                        while tok != 0 && start < pos {
                            prev = tok;
                            oneword_checked!(
                                heap,
                                mk,
                                text_copy,
                                word_buf,
                                clm_buf,
                                alt_buf,
                                out.get_next(a, c, u32::from(prev), &mut tok)
                            );
                            if tok != 0 {
                                oneword_checked!(
                                    heap,
                                    mk,
                                    text_copy,
                                    word_buf,
                                    clm_buf,
                                    alt_buf,
                                    get_num16(out, a, c, tok, 1, &mut start, &mut n)
                                );
                            }
                        }
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            out.new_record(a, c, u32::from(prev), &mut tok)
                        );
                        out_version = out_version.wrapping_add(1);
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            out.set_field(a, c, tok, 1, 1, &pos.to_le_bytes()[..1], &mut x)
                        );
                        out_version = out_version.wrapping_add(1);
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            out.set_field(a, c, tok, 0, 1, &[wtype], &mut x)
                        );
                        out_version = out_version.wrapping_add(1);
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            out.set_field(a, c, tok, 7, 1, &f7.to_le_bytes()[..1], &mut x)
                        );
                        out_version = out_version.wrapping_add(1);
                        let lang_v = cached_str(out, a, c, &mut lang, &mut k_lang, out_version);
                        let lang_len = terminated_len(lang_v).wrapping_add(1);
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            out.set_field(
                                a,
                                c,
                                tok,
                                9,
                                lang_len as u16,
                                &lang_v[..lang_len as usize],
                                &mut x,
                            )
                        );
                        out_version = out_version.wrapping_add(1);
                        sp3 = arena_index(heap, ph, b' ');
                        sp5 = arena_index(heap, pos5, b' ');
                        sp6 = arena_index(heap, pos6, b' ');
                    }
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        out.set_field(a, c, tok, 2, 1, &end.to_le_bytes()[..1], &mut x)
                    );
                    out_version = out_version.wrapping_add(1);
                    let arena = heap.arena();
                    let ph_len = terminated_len(&arena[ph as usize..]).wrapping_add(1);
                    let end_ph = ph as usize + ph_len as usize;
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        out.set_field(
                            a,
                            c,
                            tok,
                            3,
                            ph_len as u16,
                            &arena[ph as usize..end_ph],
                            &mut x
                        )
                    );
                    out_version = out_version.wrapping_add(1);
                    let arena = heap.arena();
                    let p5_len = terminated_len(&arena[pos5 as usize..]).wrapping_add(1);
                    let end_p5 = pos5 as usize + p5_len as usize;
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        out.set_field(
                            a,
                            c,
                            tok,
                            5,
                            p5_len as u16,
                            &arena[pos5 as usize..end_p5],
                            &mut x,
                        )
                    );
                    out_version = out_version.wrapping_add(1);
                    let arena = heap.arena();
                    let p6_len = terminated_len(&arena[pos6 as usize..]).wrapping_add(1);
                    let end_p6 = pos6 as usize + p6_len as usize;
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        out.set_field(
                            a,
                            c,
                            tok,
                            6,
                            p6_len as u16,
                            &arena[pos6 as usize..end_p6],
                            &mut x,
                        )
                    );
                    out_version = out_version.wrapping_add(1);
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        out.set_field(a, c, tok, 8, 1, &wstart.to_le_bytes()[..1], &mut x)
                    );
                    out_version = out_version.wrapping_add(1);
                }
            }
            rc = oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.get_next(a, c, u32::from(tok), &mut tok)
            );
        }
        if same_start && keep_alt {
            let mut k = 0u8;
            while k < n_del {
                rc = oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.remove(a, c, u32::from(del_ids[k as usize]))
                );
                out_version = out_version.wrapping_add(1);
                k = k.wrapping_add(1);
            }
        }
    }

    if ctx.dps_multi_trans != 0 {
        rc = oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.get_first(a, c, 2, u32::from(root), &mut tok)
        );
        while tok != 0 {
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                get_num8(out, a, c, tok, 0, &mut wtype, &mut n)
            );
            if wtype == 9 {
                if out.has_field(a, c, u32::from(tok), 11, &mut has) >= 0x80000000 {
                    has = 0;
                } else if has != 0 {
                    rc = oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        out.get_next(a, c, u32::from(tok), &mut tok)
                    );
                    continue;
                }
                let mut total = 3u16;
                let mut tlen = 0u16;
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    get_num16(out, a, c, tok, 1, &mut start, &mut n)
                );
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    get_num16(out, a, c, tok, 2, &mut end, &mut n)
                );
                copy_terminated(&mut tmp, b"multi_trans_\0").expect("prefix fits tmp");
                let span = i32::from(end) - i32::from(start) + 1;
                if span > 128 {
                    ctx.buf40[0] = match heap.reallocate(ctx.buf40[0], span as u32) {
                        Some(off) => off,
                        None => {
                            if mk != 0 {
                                heap.release(mk);
                            }
                            free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
                            return 0x89b0200a;
                        }
                    };
                }
                let copy_len = (span - 1) as u32;
                arena_copy_fixed(
                    heap,
                    ctx.buf40[0],
                    text_copy.wrapping_add(u32::from(start)),
                    copy_len,
                );
                heap.arena()[ctx.buf40[0].wrapping_add(copy_len) as usize] = 0;
                let _ = cfg.add_word(
                    ctx.h,
                    cached_str(out, a, c, &mut s4, &mut k4, out_version),
                    &heap.arena()[ctx.buf40[0] as usize..],
                );
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    fetch_str(out, a, c, tok, 3, &mut s3, &mut k3, out_version, &mut n)
                );
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    fetch_str(
                        out,
                        a,
                        c,
                        tok,
                        9,
                        &mut lang,
                        &mut k_lang,
                        out_version,
                        &mut n
                    )
                );
                heap.arena()[ctx.buf40[1] as usize] = 0;
                let mut k = 0u8;
                while k < ctx.dps_multi_trans {
                    render_number(u32::from(k) + 1, &mut num_buf, 10);
                    append_terminated(&mut tmp, &num_buf);
                    let s3v = fetched_str(out, a, c, tok, 3, &mut s3, &mut k3, out_version);
                    let s3len = terminated_len(s3v) as usize;
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        trans.set_string(ctx.h2, 0, &s3v[..s3len])
                    );
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        trans.get_string(ctx.h2, 0, &mut out0, &mut tlen)
                    );
                    out0[tlen as usize] = 0;
                    let mut i = 0u16;
                    while i < tlen {
                        if out0[i as usize] == b'~' {
                            out0[i as usize] = 0xad;
                        }
                        i = i.wrapping_add(1);
                    }
                    let lang_v =
                        fetched_str(out, a, c, tok, 9, &mut lang, &mut k_lang, out_version);
                    if lang_v[0] != 0 {
                        oneword_checked!(
                            heap,
                            mk,
                            text_copy,
                            word_buf,
                            clm_buf,
                            alt_buf,
                            trans.get_string(ctx.h2, 2, &mut out2, &mut tlen)
                        );
                        out2[tlen as usize] = 0;
                        if tlen != 0 {
                            let f = lang_v[0];
                            let mut i = 0u16;
                            while i < tlen {
                                out2[i as usize] = f;
                                i = i.wrapping_add(1);
                            }
                        }
                    }
                    let b0 = ctx.buf40[0] as usize;
                    let arena = heap.arena();
                    let b0len = terminated_len(&arena[b0..]) as usize;
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        trans.set_string(ctx.h2, 1, &arena[b0..b0 + b0len])
                    );
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        trans.select(ctx.h2, &tmp)
                    );
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        trans.get_string(ctx.h2, 0, &mut out0, &mut tlen)
                    );
                    out0[tlen as usize] = 0;
                    pos = tlen;
                    let mut i = 0u16;
                    while i < pos {
                        if out0[i as usize] == b'~' {
                            let mut j = i;
                            while j < pos {
                                out0[j as usize] = out0[j.wrapping_add(1) as usize];
                                j = j.wrapping_add(1);
                            }
                            pos = pos.wrapping_sub(1);
                        } else if out0[i as usize] == 0xad {
                            out0[i as usize] = b'~';
                            i = i.wrapping_add(1);
                        } else {
                            i = i.wrapping_add(1);
                        }
                    }
                    if string_differs(
                        fetched_str(out, a, c, tok, 3, &mut s3, &mut k3, out_version),
                        &out0,
                    ) {
                        total = total
                            .wrapping_add(terminated_len(&out0) as u16)
                            .wrapping_add(1);
                        if total > 128 {
                            ctx.buf40[1] = match heap.reallocate(ctx.buf40[1], u32::from(total)) {
                                Some(off) => off,
                                None => {
                                    if mk != 0 {
                                        heap.release(mk);
                                    }
                                    free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
                                    return 0x89b0200a;
                                }
                            };
                        }
                        append_terminated(&mut heap.arena()[ctx.buf40[1] as usize..], &out0);
                        append_terminated(&mut heap.arena()[ctx.buf40[1] as usize..], b"\x12\0");
                    }
                    k = k.wrapping_add(1);
                }
                if heap.arena()[ctx.buf40[1] as usize] != 0 {
                    let b1 = ctx.buf40[1] as usize;
                    let blen = terminated_len(&heap.arena()[b1..]) as usize;
                    heap.arena()[b1 + blen - 1] = 0;
                    let arena = heap.arena();
                    let flen = terminated_len(&arena[b1..]).wrapping_add(1);
                    let end_f = b1 + flen as usize;
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        out.set_field(a, c, tok, 11, flen as u16, &arena[b1..end_f], &mut x)
                    );
                    out_version = out_version.wrapping_add(1);
                }
            }
            rc = oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.get_next(a, c, u32::from(tok), &mut tok)
            );
        }
    }

    if ctx.no_ext_clc == 0 {
        if mk != 0 {
            heap.release(mk);
        }
        free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
        return rc;
    }
    if ctx.clc_extra_main_trans != 0 && ctx.n_out_trans > 1 {
        let mut prev_start = 0u16;
        let mut prev_end = 0u16;
        let mut foreign = false;
        let mut prev_tok = 0u16;
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.get_first(a, c, 2, u32::from(root), &mut tok)
        );
        if tok != 0 {
            loop {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    get_num8(out, a, c, tok, 0, &mut wtype, &mut n)
                );
                if wtype == 9 || wtype == 4 {
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        get_num16(out, a, c, tok, 1, &mut start, &mut n)
                    );
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        get_num16(out, a, c, tok, 2, &mut end, &mut n)
                    );
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        fetch_str(
                            out,
                            a,
                            c,
                            tok,
                            9,
                            &mut lang,
                            &mut k_lang,
                            out_version,
                            &mut n
                        )
                    );
                    if start != prev_start {
                        if !foreign {
                            oneword_checked!(
                                heap,
                                mk,
                                text_copy,
                                word_buf,
                                clm_buf,
                                alt_buf,
                                oneword_lookup_word(
                                    ctx,
                                    heap,
                                    cfg,
                                    out,
                                    trans,
                                    dict,
                                    lex,
                                    &mut out_version,
                                    a,
                                    c,
                                    u32::from(prev_tok),
                                    prev_start,
                                    prev_end,
                                    OnewordText::Heap(text_copy),
                                )
                            );
                        }
                        foreign = false;
                    }
                    let lang_v =
                        fetched_str(out, a, c, tok, 9, &mut lang, &mut k_lang, out_version);
                    let lc = ctx.lang_code as usize;
                    let arena = heap.arena();
                    if !caseless_differs(lang_v, &arena[lc..])
                        || !string_differs(lang_v, b"\0")
                        || !string_differs(lang_v, b"normal\0")
                    {
                        foreign = true;
                    }
                    prev_end = end;
                    prev_start = start;
                }
                prev_tok = tok;
                rc = oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.get_next(a, c, u32::from(prev_tok), &mut tok)
                );
                if tok == 0 {
                    break;
                }
            }
            if !foreign {
                rc = oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    oneword_lookup_word(
                        ctx,
                        heap,
                        cfg,
                        out,
                        trans,
                        dict,
                        lex,
                        &mut out_version,
                        a,
                        c,
                        u32::from(prev_tok),
                        prev_start,
                        prev_end,
                        OnewordText::Heap(text_copy),
                    )
                );
            }
        } else {
            rc = oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                oneword_lookup_word(
                    ctx,
                    heap,
                    cfg,
                    out,
                    trans,
                    dict,
                    lex,
                    &mut out_version,
                    a,
                    c,
                    u32::from(prev_tok),
                    prev_start,
                    prev_end,
                    OnewordText::Heap(text_copy),
                )
            );
        }
        if ctx.no_ext_clc == 0 {
            if mk != 0 {
                heap.release(mk);
            }
            free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
            return rc;
        }
    }

    rc = out.get_first(a, c, 2, u32::from(root), &mut tok);
    if rc >= 0x80000000 || tok == 0 {
        if mk != 0 {
            heap.release(mk);
        }
        free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
        return rc;
    }
    loop {
        oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            get_num8(out, a, c, tok, 0, &mut wtype, &mut n)
        );
        if wtype == 9 || wtype == 4 || wtype == 10 {
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                fetch_str(out, a, c, tok, 3, &mut s3, &mut k3, out_version, &mut n)
            );
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                fetch_str(
                    out,
                    a,
                    c,
                    tok,
                    9,
                    &mut lang,
                    &mut k_lang,
                    out_version,
                    &mut n
                )
            );
            let lang_v = fetched_str(out, a, c, tok, 9, &mut lang, &mut k_lang, out_version);
            let lang_len = terminated_len(lang_v).wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(
                    a,
                    c,
                    tok,
                    12,
                    lang_len as u16,
                    &lang_v[..lang_len as usize],
                    &mut x,
                )
            );
            out_version = out_version.wrapping_add(1);
            copy_terminated(&mut tmp, b"clm\0").expect("prefix fits tmp");
            append_terminated(&mut tmp, lang_v);
            let (rc_get1, has1) = params.get(ctx.env.params, &tmp, &mut clm_val);
            let mut clm_some = rc_get1 < 0x80000000 && has1;
            let mut skip_to_next = false;
            if clm_some {
                if string_differs(&clm_val, b"yes\0")
                    && (string_differs(&clm_val, b"no\0") || ctx.clm_variants == 0)
                {
                    skip_to_next = true;
                }
            } else {
                copy_terminated(&mut tmp, b"noclmset\0").expect("key fits tmp");
                let (rc_get2, has2) = params.get(ctx.env.params, &tmp, &mut clm_val);
                let found = if rc_get2 >= 0x80000000 || !has2 {
                    false
                } else {
                    let lang_v2 =
                        fetched_str(out, a, c, tok, 9, &mut lang, &mut k_lang, out_version);
                    find_substring(&clm_val, lang_v2).is_some()
                };
                if !found {
                    clm_some = false;
                } else {
                    copy_terminated(&mut clm_val, b"no\0").expect("value fits");
                    clm_some = true;
                    if string_differs(&clm_val, b"yes\0")
                        && (string_differs(&clm_val, b"no\0") || ctx.clm_variants == 0)
                    {
                        skip_to_next = true;
                    }
                }
            }
            if skip_to_next {
                rc = oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.get_next(a, c, u32::from(tok), &mut tok)
                );
                if tok == 0 {
                    break;
                }
                continue;
            }
            let lang_v = fetched_str(out, a, c, tok, 9, &mut lang, &mut k_lang, out_version);
            let lc = ctx.lang_code as usize;
            let arena = heap.arena();
            if !caseless_differs(lang_v, &arena[lc..])
                || !string_differs(lang_v, b"\0")
                || !string_differs(lang_v, b"normal\0")
            {
                rc = oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.get_next(a, c, u32::from(tok), &mut tok)
                );
                if tok == 0 {
                    break;
                }
                continue;
            }
            let s3v = fetched_str(out, a, c, tok, 3, &mut s3, &mut k3, out_version);
            let mut clm_size = terminated_len(s3v).wrapping_add(1).wrapping_mul(4);
            let mut new_tok = tok;
            for i in 0..3 {
                let signed = i32::from(lang_v[i] as i8);
                code[i] = if signed > 0x60 {
                    (signed - 0x20) as u8
                } else {
                    lang_v[i]
                };
            }
            code[3] = 0;
            clm_buf = match heap.reallocate(clm_buf, clm_size) {
                Some(off) => off,
                None => {
                    if mk != 0 {
                        heap.release(mk);
                    }
                    free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
                    return 0x89b0200a;
                }
            };
            heap.arena()[clm_buf as usize] = 0;
            if clm_some && !string_differs(&clm_val, b"no\0") && ctx.clm_variants != 0 {
                let one = 1u16;
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.new_record(a, c, u32::from(tok), &mut new_tok)
                );
                out_version = out_version.wrapping_add(1);
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    get_num16(out, a, c, tok, 1, &mut start, &mut n)
                );
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    get_num16(out, a, c, tok, 2, &mut end, &mut n)
                );
                wtype = 15;
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.set_field(a, c, new_tok, 0, 1, &[wtype], &mut x)
                );
                out_version = out_version.wrapping_add(1);
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.set_field(a, c, new_tok, 1, 1, &start.to_le_bytes()[..1], &mut x)
                );
                out_version = out_version.wrapping_add(1);
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.set_field(a, c, new_tok, 2, 1, &end.to_le_bytes()[..1], &mut x)
                );
                out_version = out_version.wrapping_add(1);
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.set_field(a, c, new_tok, 5, 1, b"*", &mut x)
                );
                out_version = out_version.wrapping_add(1);
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.set_field(a, c, new_tok, 6, 1, b"*", &mut x)
                );
                out_version = out_version.wrapping_add(1);
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.set_field(a, c, new_tok, 7, 1, &one.to_le_bytes()[..1], &mut x)
                );
                out_version = out_version.wrapping_add(1);
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    out.set_field(a, c, new_tok, 12, 4, b"map\0", &mut x)
                );
                out_version = out_version.wrapping_add(1);
            }
            let s3v = fetched_str(out, a, c, tok, 3, &mut s3, &mut k3, out_version);
            let s3len = terminated_len(s3v).wrapping_add(1);
            let cb = clm_buf as usize;
            let arena = heap.arena();
            clm.map(
                ctx.h4,
                &code,
                &s3v[..s3len as usize],
                &mut arena[cb..cb + clm_size as usize],
            );
            let arena = heap.arena();
            let cb_len = terminated_len(&arena[cb..]).wrapping_add(1);
            let end_cb = cb + cb_len as usize;
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, new_tok, 3, cb_len as u16, &arena[cb..end_cb], &mut x)
            );
            out_version = out_version.wrapping_add(1);
            oneword_checked!(
                heap,
                mk,
                text_copy,
                word_buf,
                clm_buf,
                alt_buf,
                out.set_field(a, c, new_tok, 9, 1, b"\0", &mut x)
            );
            out_version = out_version.wrapping_add(1);
            has = 0;
            if out.has_field(a, c, u32::from(tok), 11, &mut has) < 0x80000000 && has == 1 {
                oneword_checked!(
                    heap,
                    mk,
                    text_copy,
                    word_buf,
                    clm_buf,
                    alt_buf,
                    fetch_str(out, a, c, tok, 11, &mut s3, &mut k3, out_version, &mut n)
                );
                if string_differs(
                    fetched_str(out, a, c, tok, 11, &mut s3, &mut k3, out_version),
                    b"\0",
                ) {
                    let need = terminated_len(fetched_str(
                        out,
                        a,
                        c,
                        tok,
                        11,
                        &mut s3,
                        &mut k3,
                        out_version,
                    ))
                    .wrapping_add(1)
                    .wrapping_mul(4);
                    alt_buf = match heap.reallocate(alt_buf, need) {
                        Some(off) => off,
                        None => {
                            if mk != 0 {
                                heap.release(mk);
                            }
                            free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
                            return 0x89b0200a;
                        }
                    };
                    heap.arena()[alt_buf as usize] = 0;
                    let mut s3off: Option<usize> = Some(0);
                    while let Some(off) = s3off {
                        let found = str_index(&s3[off..], 0x12).map(|rel| off + rel);
                        if let Some(at) = found {
                            s3[at] = 0;
                        }
                        if heap.arena()[alt_buf as usize] != 0 {
                            append_terminated(&mut heap.arena()[alt_buf as usize..], b"\x12\0");
                        }
                        copy_terminated(&mut heap.arena()[clm_buf as usize..], b"_\0")
                            .expect("underscore fits clmBuf");
                        let seg_len = terminated_len(&s3[off..]).wrapping_add(1).wrapping_mul(4);
                        if clm_size < seg_len {
                            clm_size = seg_len;
                            clm_buf = match heap.reallocate(clm_buf, clm_size) {
                                Some(off) => off,
                                None => {
                                    if mk != 0 {
                                        heap.release(mk);
                                    }
                                    free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
                                    return 0x89b0200a;
                                }
                            };
                        }
                        let seg_end = off + (seg_len / 4) as usize;
                        let cb = clm_buf as usize;
                        let arena = heap.arena();
                        clm.map(
                            ctx.h4,
                            &code,
                            &s3[off..seg_end],
                            &mut arena[cb..cb + clm_size as usize],
                        );
                        arena_append(heap, alt_buf, clm_buf);
                        if let Some(at) = found {
                            s3[at] = 0x12;
                            s3off = Some(at + 1);
                        } else {
                            s3off = None;
                        }
                    }
                    let ab = alt_buf as usize;
                    let arena = heap.arena();
                    let ab_len = terminated_len(&arena[ab..]).wrapping_add(1);
                    let end_ab = ab + ab_len as usize;
                    oneword_checked!(
                        heap,
                        mk,
                        text_copy,
                        word_buf,
                        clm_buf,
                        alt_buf,
                        out.set_field(a, c, new_tok, 11, ab_len as u16, &arena[ab..end_ab], &mut x)
                    );
                    out_version = out_version.wrapping_add(1);
                }
            }
        }
        rc = oneword_checked!(
            heap,
            mk,
            text_copy,
            word_buf,
            clm_buf,
            alt_buf,
            out.get_next(a, c, u32::from(tok), &mut tok)
        );
        if tok == 0 {
            break;
        }
    }
    if mk != 0 {
        heap.release(mk);
    }
    free_out(heap, text_copy, word_buf, clm_buf, alt_buf);
    rc
}
