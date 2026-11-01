use crate::frontend_types::Error;

pub fn read_bytes<'a>(data: &'a [u8], offset: &mut u32, size: u32) -> Result<&'a [u8], Error> {
    let end = offset.wrapping_add(size);
    if end as usize > data.len() {
        return Err(Error::Engine(0x89702009));
    }
    let result = data
        .get(*offset as usize..end as usize)
        .ok_or(Error::InvalidResource)?;
    *offset = end;
    Ok(result)
}

pub fn read_aligned(data: &[u8], offset: &mut u32, output: &mut [u8]) -> Result<(), Error> {
    *offset = offset.wrapping_add(3) & !3;
    let bytes = read_bytes(
        data,
        offset,
        output
            .len()
            .try_into()
            .map_err(|_| Error::InvalidResource)?,
    )?;
    output.copy_from_slice(bytes);
    Ok(())
}

pub struct CharacterTables<'a> {
    pub classes: &'a [u8],
    pub groups: &'a [u8],
    pub values: &'a [u8],
    pub capture_types: &'a [u8],
}

impl CharacterTables<'_> {
    fn lookup(table: &[u8], character: u8, group: u8, bias: usize) -> Result<u8, Error> {
        let index = (character as usize + group as usize * 162)
            .checked_sub(bias)
            .ok_or(Error::InvalidResource)?;
        table.get(index).copied().ok_or(Error::InvalidResource)
    }

    pub fn capture_matches(&self, character: u8, capture: u32) -> Result<bool, Error> {
        let index = (capture as usize)
            .checked_mul(2)
            .and_then(|v| v.checked_sub(2))
            .ok_or(Error::InvalidResource)?;
        let kind = *self
            .capture_types
            .get(index)
            .ok_or(Error::InvalidResource)?;
        match kind {
            4 | 5 => {
                let group = *self
                    .capture_types
                    .get(index + 1)
                    .ok_or(Error::InvalidResource)?;
                let table = if kind == 4 { self.classes } else { self.groups };
                Ok(Self::lookup(table, character, group, 14)? != 0)
            }
            _ => Ok(true),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PredicateState {
    pub current: usize,
    pub saved: Option<usize>,
    pub cursor: usize,
    pub character: u8,
    pub captured: u8,
    pub value: u8,
}

impl PredicateState {

    pub fn evaluate(
        &mut self,
        code: &[u8],
        end: u32,
        position: usize,
        layers: &[&[u8]],
        captures: &mut [u8],
        tables: &CharacterTables<'_>,
    ) -> Result<bool, Error> {
        self.saved = None;
        loop {
            let opcode = *code.get(self.cursor).ok_or(Error::InvalidResource)?;
            if opcode as u32 == end {
                return Ok(true);
            }
            self.cursor += 1;
            let operand = *code.get(self.cursor).ok_or(Error::InvalidResource)?;
            let matched = match opcode {
                0xef | 0xf1 => {
                    self.saved = Some(self.current);
                    self.current = (operand as usize)
                        .checked_sub(1)
                        .ok_or(Error::InvalidResource)?;
                    self.character = *layers
                        .get(self.current)
                        .and_then(|v| position.checked_sub(1).and_then(|i| v.get(i)))
                        .ok_or(Error::InvalidResource)?;
                    self.character != 14
                }
                0xee | 0xf0 => {
                    let Some(saved) = self.saved else {
                        return Ok(false);
                    };
                    self.current = saved;
                    self.saved = None;
                    self.character = *layers
                        .get(self.current)
                        .and_then(|v| position.checked_sub(1).and_then(|i| v.get(i)))
                        .ok_or(Error::InvalidResource)?;
                    self.character != 14
                }
                0xf2 => self.character != 14 && self.character != operand,
                0xf3 => self.character != 14 && self.character == operand,
                0xf8 | 0xf9 | 0xfa | 0xfb | 0xfc => {
                    if self.character == 14 {
                        false
                    } else {
                        let table = if opcode >= 0xfb {
                            tables.classes
                        } else {
                            tables.groups
                        };
                        let value = CharacterTables::lookup(table, self.character, operand, 176)?;
                        let matched = if opcode == 0xf9 || opcode == 0xfb {
                            value == 0
                        } else {
                            value != 0
                        };
                        if matched && opcode == 0xf8 {
                            self.value = CharacterTables::lookup(
                                tables.values,
                                self.character,
                                operand,
                                176,
                            )?;
                        }
                        matched
                    }
                }
                0xfd => {
                    let captured = *captures
                        .get(operand as usize)
                        .ok_or(Error::InvalidResource)?;
                    if captured != 0 {
                        captured == self.character
                    } else if tables.capture_matches(self.character, operand as u32)? {
                        self.captured = 1;
                        captures[operand as usize] = self.character;
                        true
                    } else {
                        false
                    }
                }
                0xea => {
                    let captured = *captures
                        .get(operand as usize)
                        .ok_or(Error::InvalidResource)?;
                    captured == 0 || captured != self.character
                }
                _ => true,
            };
            if !matched {
                if !matches!(opcode, 0xef | 0xf1 | 0xee | 0xf0) {
                    if let Some(saved) = self.saved {
                        self.current = saved;
                    }
                }
                return Ok(false);
            }
            self.cursor += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MatchState {
    pub predicate: PredicateState,
    pub saved_layer: Option<usize>,
    pub capture_index: u8,
    pub repeat_limit: i16,
    pub step: i16,
    pub next_position: i16,
    pub restarted: i16,
}

impl MatchState {
    fn character(&self, layers: &[&[u8]], position: i32) -> Result<u8, Error> {
        let index = usize::try_from(position).map_err(|_| Error::InvalidResource)?;
        layers
            .get(self.predicate.current)
            .and_then(|v| v.get(index))
            .copied()
            .ok_or(Error::InvalidResource)
    }

    fn skip(&self, layers: &[&[u8]], position: &mut i32, step: i16) -> Result<(), Error> {
        while self.character(layers, *position)? == b'~' {
            *position = (*position + step as i32) as i16 as i32;
        }
        Ok(())
    }

    fn operand(&self, code: &[u8], ahead: usize) -> Result<u8, Error> {
        code.get(self.predicate.cursor + ahead)
            .copied()
            .ok_or(Error::InvalidResource)
    }

    pub fn match_prefix(
        &mut self,
        code: &[u8],
        start: i16,
        restart: i32,
        layers: &[&[u8]],
        captures: &mut [u8],
        tables: &CharacterTables<'_>,
    ) -> Result<i16, Error> {
        let mut position = start.wrapping_sub(1) as i32;
        self.restarted = 0;
        loop {
            let opcode = self.operand(code, 0)?;
            match opcode {
                0xff => return Ok((position + 1) as i16),
                0xef | 0xf1 => {
                    self.saved_layer = Some(self.predicate.current);
                    self.predicate.current = (self.operand(code, 1)? as usize)
                        .checked_sub(1)
                        .ok_or(Error::InvalidResource)?;
                    self.step = 0;
                    self.predicate.cursor += 2;
                    continue;
                }
                0xee | 0xf0 => {
                    self.predicate.current = self.saved_layer.ok_or(Error::InvalidResource)?;
                    self.step = 0;
                    self.predicate.cursor += 2;
                    continue;
                }
                0xeb => {
                    position = restart;
                    self.restarted = 1;
                    self.step = 1;
                    self.predicate.cursor += 1;
                    continue;
                }
                0xec => {
                    self.skip(layers, &mut position, 1)?;
                    self.predicate.character = self.character(layers, position)?;
                    self.step = 1;
                    self.predicate.cursor += 1;
                    continue;
                }
                0xf4 | 0xed => self.next_position = position as i16,
                _ => (),
            }
            if self.step != 0 {
                self.skip(layers, &mut position, self.step)?;
            }
            let character = self.character(layers, position)?;
            self.predicate.character = character;
            match opcode {
                0xfd => {
                    self.predicate.cursor += 1;
                    let operand = self.operand(code, 0)?;
                    let captured = *captures
                        .get(operand as usize)
                        .ok_or(Error::InvalidResource)?;
                    if captured == 0 {
                        if !tables.capture_matches(character, operand as u32)? {
                            return Ok(0);
                        }
                        self.predicate.captured = 1;
                        captures[operand as usize] = character;
                    } else if character != captured {
                        return Ok(0);
                    }
                }
                0xea => {
                    if character == 14 {
                        return Ok(0);
                    }
                    self.predicate.cursor += 1;
                    let captured = *captures
                        .get(self.operand(code, 0)? as usize)
                        .ok_or(Error::InvalidResource)?;
                    if captured != 0 && character == captured {
                        return Ok(0);
                    }
                }
                0xf8..=0xfc => {
                    if opcode != 0xf8 && character == 14 {
                        return Ok(0);
                    }
                    self.predicate.cursor += 1;
                    let operand = self.operand(code, 0)?;
                    let table = if opcode >= 0xfb {
                        tables.classes
                    } else {
                        tables.groups
                    };
                    let value = CharacterTables::lookup(table, character, operand, 176)?;
                    let matched = match opcode {
                        0xf8 => value == 1,
                        0xf9 | 0xfb => value == 0,
                        _ => value != 0,
                    };
                    if !matched {
                        return Ok(0);
                    }
                    if opcode == 0xf8 {
                        self.predicate.value =
                            CharacterTables::lookup(tables.values, character, operand, 176)?;
                    }
                }
                0xf7 => {
                    self.predicate.cursor += 1;
                    if !self.predicate.evaluate(
                        code,
                        0xf7,
                        (position + 1) as i16 as u16 as usize,
                        layers,
                        captures,
                        tables,
                    )? {
                        return Ok(0);
                    }
                }
                0xf4 => {
                    let rule = self.predicate.cursor;
                    self.repeat_limit = self.operand(code, 2)? as i16;
                    if self.repeat_limit == 0xdc {
                        self.repeat_limit = 5000;
                    }
                    let mut count = 0_i16;
                    while count < self.repeat_limit {
                        let next = position + 1;
                        self.predicate.cursor = rule + 3;
                        if !self.predicate.evaluate(
                            code,
                            0xf4,
                            next as i16 as u16 as usize,
                            layers,
                            captures,
                            tables,
                        )? {
                            break;
                        }
                        self.next_position = next as i16;
                        position = next as i16 as i32;
                        self.skip(layers, &mut position, 1)?;
                        self.predicate.character = self.character(layers, position)?;
                        count += 1;
                    }
                    position = self.next_position.wrapping_sub(1) as i32;
                    if (count as u16) < *code.get(rule + 1).ok_or(Error::InvalidResource)? as u16 {
                        return Ok(0);
                    }
                    while self.operand(code, 0)? != 0xf4 {
                        self.predicate.cursor += 1;
                    }
                }
                0xed => {
                    self.repeat_limit = self.operand(code, 2)? as i16;
                    if self.repeat_limit == 0xdc {
                        self.repeat_limit = 5000;
                    }
                    let kind = self.operand(code, 3)?;
                    self.capture_index = self.operand(code, 4)?;
                    let operand = self.capture_index;
                    let mut count = 0_i16;
                    if matches!(kind, 0xea | 0xf2 | 0xf3 | 0xf9..=0xfd) {
                        while count < self.repeat_limit {
                            let current = self.character(layers, position)?;
                            let matched = match kind {
                                0xf9..=0xfc => {
                                    self.predicate.character = current;
                                    if current == 14 {
                                        false
                                    } else {
                                        let table = if kind >= 0xfb {
                                            tables.classes
                                        } else {
                                            tables.groups
                                        };
                                        let value =
                                            CharacterTables::lookup(table, current, operand, 176)?;
                                        if kind == 0xf9 || kind == 0xfb {
                                            value == 0
                                        } else {
                                            value != 0
                                        }
                                    }
                                }
                                0xf2 | 0xf3 => {
                                    self.predicate.character = current;
                                    current != 14
                                        && if kind == 0xf2 {
                                            current != operand
                                        } else {
                                            current == operand
                                        }
                                }
                                0xfd => {
                                    let captured = *captures
                                        .get(operand as usize)
                                        .ok_or(Error::InvalidResource)?;
                                    if captured == 0 {
                                        if tables.capture_matches(
                                            self.predicate.character,
                                            operand as u32,
                                        )? {
                                            self.predicate.captured = 1;
                                            captures[operand as usize] = current;
                                            true
                                        } else {
                                            false
                                        }
                                    } else {
                                        current == captured
                                    }
                                }
                                _ => {
                                    let captured = *captures
                                        .get(operand as usize)
                                        .ok_or(Error::InvalidResource)?;
                                    captured == 0 || current != captured
                                }
                            };
                            if !matched {
                                break;
                            }
                            self.next_position = (position + 1) as i16;
                            position = (position + 1) as i16 as i32;
                            self.skip(layers, &mut position, 1)?;
                            count += 1;
                        }
                    }
                    position = self.next_position.wrapping_sub(1) as i32;
                    if (count as u16) < self.operand(code, 1)? as u16 {
                        return Ok(0);
                    }
                    self.predicate.cursor += 5;
                }
                0xf2 | 0xf3 => {
                    if character == 14 {
                        return Ok(0);
                    }
                    self.predicate.cursor += 1;
                    let operand = self.operand(code, 0)?;
                    if if opcode == 0xf3 {
                        operand != character
                    } else {
                        operand == character
                    } {
                        return Ok(0);
                    }
                }
                _ => {
                    if character != opcode {
                        return Ok(0);
                    }
                }
            }
            self.step = 1;
            position = (position + 1) as i16 as i32;
            self.predicate.cursor += 1;
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct ScanFrame {
    count: u32,
    observed: u32,
    after_child: bool,
}

pub fn scan_repeat(
    code: &[u8],
    cursor: &mut usize,
    kind: i32,
    count: i32,
    frames: &mut [ScanFrame],
) -> Result<bool, Error> {
    if frames.is_empty() {
        return Err(Error::Capacity {
            required: 1,
            available: 0,
        });
    }
    frames[0] = ScanFrame {
        count: count as u32,
        observed: 0,
        after_child: false,
    };
    let mut depth = 0;
    loop {
        if !frames[depth].after_child {
            let token = *code.get(*cursor).ok_or(Error::InvalidResource)? as i32;
            if token == kind {
                frames[depth].observed =
                    *code.get(*cursor + 1).ok_or(Error::InvalidResource)? as u32;
            }
            if token == kind && frames[depth].observed == frames[depth].count {
                if depth == 0 {
                    return Ok(true);
                }
                depth -= 1;
                continue;
            }
            if token == kind
                && (frames[depth].observed == 1 || kind == 0xf5 && frames[depth].observed != 0)
            {
                frames[depth].after_child = true;
                *cursor += 1;
                if depth + 1 == frames.len() {
                    return Err(Error::Capacity {
                        required: depth + 2,
                        available: frames.len(),
                    });
                }
                depth += 1;
                frames[depth] = ScanFrame::default();
                continue;
            }
        } else {
            *cursor += 1;
            frames[depth].after_child = false;
        }
        *cursor += 1;
        if *code.get(*cursor).ok_or(Error::InvalidResource)? as i32 == kind
            && *code.get(*cursor + 1).ok_or(Error::InvalidResource)? == 0
        {
            if depth == 0 {
                return Ok(false);
            }
            depth -= 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RepeatFrame {
    pub repeats: u8,
    pub position: Option<usize>,
    pub offset: i16,
    pub length: i16,
    pub command: i16,
    pub current: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SequenceState {
    pub predicate: PredicateState,
    pub active: u8,
    pub saved_layer: Option<usize>,
    pub argument: u8,
    pub maximum: i16,
    pub depth: i16,
    pub offset: i16,
    pub step: i16,
    pub skip: i16,
    pub previous: i16,
}

impl SequenceState {
    fn character(&self, layers: &[&[u8]]) -> Result<u8, Error> {
        let index = usize::try_from(self.offset as i32 - 1).map_err(|_| Error::InvalidResource)?;
        layers
            .get(self.predicate.current)
            .and_then(|v| v.get(index))
            .copied()
            .ok_or(Error::InvalidResource)
    }

    fn skip_tildes(&mut self, layers: &[&[u8]], step: i16) -> Result<(), Error> {
        while self.character(layers)? == b'~' {
            if step == 0 {
                return Err(Error::InvalidResource);
            }
            self.offset = self.offset.wrapping_add(step);
        }
        Ok(())
    }

    fn load_character(&mut self, layers: &[&[u8]]) -> Result<u8, Error> {
        if self.skip != 0 {
            self.skip_tildes(layers, self.skip)?;
        }
        let value = self.character(layers)?;
        self.predicate.character = value;
        Ok(value)
    }

    fn frame_index(&self, frames: &[RepeatFrame]) -> Result<usize, Error> {
        let index = usize::try_from(self.depth).map_err(|_| Error::InvalidResource)?;
        if index >= frames.len() {
            return Err(Error::Capacity {
                required: index + 1,
                available: frames.len(),
            });
        }
        Ok(index)
    }

    fn operand(&self, code: &[u8], ahead: usize) -> Result<u8, Error> {
        code.get(self.predicate.cursor + ahead)
            .copied()
            .ok_or(Error::InvalidResource)
    }

    fn evaluate(
        &mut self,
        code: &[u8],
        end: u32,
        layers: &[&[u8]],
        captures: &mut [u8],
        tables: &CharacterTables<'_>,
    ) -> Result<i32, Error> {
        let position = usize::try_from(self.offset).map_err(|_| Error::InvalidResource)?;
        Ok(self
            .predicate
            .evaluate(code, end, position, layers, captures, tables)? as i32)
    }

    pub fn execute_operation(
        &mut self,
        code: &[u8],
        layers: &[&[u8]],
        captures: &mut [u8],
        tables: &CharacterTables<'_>,
        frames: &mut [RepeatFrame],
        scanner: &mut [ScanFrame],
    ) -> Result<i32, Error> {
        let rule = self.predicate.cursor;
        let opcode = self.operand(code, 0)?;
        match opcode {
            0xfd | 0xea => {
                let character = self.load_character(layers)?;
                self.predicate.cursor = rule + 1;
                let argument = self.operand(code, 0)?;
                self.skip = self.step;
                let captured = *captures
                    .get(argument as usize)
                    .ok_or(Error::InvalidResource)?;
                if opcode == 0xea {
                    return Ok((captured == 0 || character != captured) as i32);
                }
                if captured != 0 {
                    return Ok((character == captured) as i32);
                }
                if !tables.capture_matches(character, argument as u32)? {
                    return Ok(0);
                }
                self.predicate.captured = 1;
                captures[argument as usize] = character;
                Ok(1)
            }
            0xf9..=0xfc | 0xf2 | 0xf3 => {
                if self.skip != 0 {
                    self.skip_tildes(layers, self.skip)?;
                }
                self.predicate.cursor = rule + 1;
                let argument = self.operand(code, 0)?;
                self.skip = self.step;
                let character = self.character(layers)?;
                self.predicate.character = character;
                if character == 14 {
                    return Ok(0);
                }
                if opcode == 0xf2 {
                    return Ok((argument != character) as i32);
                }
                if opcode == 0xf3 {
                    return Ok((argument == character) as i32);
                }
                let table = if opcode >= 0xfb {
                    tables.classes
                } else {
                    tables.groups
                };
                let value = CharacterTables::lookup(table, character, argument, 176)?;
                Ok(if opcode == 0xfa || opcode == 0xfc {
                    value as i32
                } else {
                    (value == 0) as i32
                })
            }
            0xf7 => {
                self.predicate.cursor = rule + 3;
                let mut result = 1;
                if self.active != 0 {
                    self.depth = self.depth.wrapping_add(1);
                    let index = self.frame_index(frames)?;
                    frames[index].command = 0xf7;
                    frames[index].current = Some(self.predicate.current);
                    frames[index].position = Some(rule);
                    frames[index].repeats = 0;
                    let minimum = *code.get(rule + 1).ok_or(Error::InvalidResource)?;
                    while frames[index].repeats < minimum && result != 0 {
                        self.load_character(layers)?;
                        frames[index].repeats = frames[index].repeats.wrapping_add(1);
                        self.predicate.cursor = rule + 3;
                        result = self.evaluate(code, 0xf7, layers, captures, tables)?;
                        self.skip = self.step;
                        self.offset = self.offset.wrapping_add(self.step);
                    }
                    frames[index].length = self.skip;
                    if frames[index].repeats == 0 {
                        while self.operand(code, 0)? != 0xf7 {
                            self.predicate.cursor += 2;
                        }
                    }
                    frames[index].offset = self.offset;
                    if result == 0 {
                        self.depth = self.depth.wrapping_sub(1);
                    }
                    self.offset = self.offset.wrapping_sub(self.step);
                } else {
                    self.load_character(layers)?;
                    self.active = 1;
                    result = self.evaluate(code, 0xf7, layers, captures, tables)?;
                    let index = self.frame_index(frames)?;
                    frames[index].offset = self.offset.wrapping_add(self.step);
                    frames[index].length = self.step;
                    if result == 0 {
                        self.depth = self.depth.wrapping_sub(1);
                    }
                    self.skip = self.step;
                }
                Ok(result)
            }
            0xf6 | 0xf5 => {
                self.predicate.cursor = rule + 1;
                let count = self.operand(code, 0)?;
                if opcode == 0xf5 && count != 0 || opcode == 0xf6 && count == 1 {
                    self.depth = self.depth.wrapping_add(1);
                    let index = self.frame_index(frames)?;
                    frames[index].length = self.skip;
                    frames[index].current = Some(self.predicate.current);
                    frames[index].command = opcode as i16;
                    frames[index].offset = self.offset;
                    frames[index].position = Some(rule + 2);
                    frames[index].repeats = if opcode == 0xf6 { count } else { 0 };
                    if opcode == 0xf5 {
                        scan_repeat(code, &mut self.predicate.cursor, opcode as i32, 0, scanner)?;
                        self.predicate.cursor += 1;
                    }
                } else if opcode == 0xf6 && count > 1 {
                    scan_repeat(code, &mut self.predicate.cursor, opcode as i32, 0, scanner)?;
                    self.predicate.cursor += 1;
                }
                self.offset = self.offset.wrapping_sub(self.step);
                Ok(1)
            }
            0xf4 => {
                self.previous = self.offset.wrapping_sub(self.step);
                self.load_character(layers)?;
                self.maximum = *code.get(rule + 2).ok_or(Error::InvalidResource)? as i16;
                if self.maximum == 0xdc {
                    self.maximum = 5000;
                }
                let mut count = 0_i16;
                while count < self.maximum {
                    self.predicate.cursor = rule + 3;
                    if self.evaluate(code, 0xf4, layers, captures, tables)? == 0 {
                        break;
                    }
                    self.previous = self.offset;
                    self.offset = self.offset.wrapping_add(self.step);
                    self.skip_tildes(layers, self.step)?;
                    self.predicate.character = self.character(layers)?;
                    count += 1;
                }
                self.offset = self.previous;
                while self.operand(code, 0)? != 0xf4 {
                    self.predicate.cursor += 1;
                }
                self.skip = self.step;
                Ok(
                    ((count as u16) >= *code.get(rule + 1).ok_or(Error::InvalidResource)? as u16)
                        as i32,
                )
            }
            0xed => {
                self.previous = self.offset.wrapping_sub(self.step);
                self.load_character(layers)?;
                self.maximum = *code.get(rule + 2).ok_or(Error::InvalidResource)? as i16;
                if self.maximum == 0xdc {
                    self.maximum = 5000;
                }
                self.argument = *code.get(rule + 4).ok_or(Error::InvalidResource)?;
                let kind = *code.get(rule + 3).ok_or(Error::InvalidResource)?;
                let argument = self.argument;
                let mut count = 0_i16;
                if matches!(kind, 0xea | 0xf2 | 0xf3 | 0xf9..=0xfd) {
                    while count < self.maximum {
                        let character = self.character(layers)?;
                        let matched = match kind {
                            0xf9..=0xfc => {
                                self.predicate.character = character;
                                if character == 14 {
                                    false
                                } else {
                                    let table = if kind >= 0xfb {
                                        tables.classes
                                    } else {
                                        tables.groups
                                    };
                                    let value =
                                        CharacterTables::lookup(table, character, argument, 176)?;
                                    if kind == 0xf9 || kind == 0xfb {
                                        value == 0
                                    } else {
                                        value != 0
                                    }
                                }
                            }
                            0xf2 | 0xf3 => {
                                self.predicate.character = character;
                                character != 14
                                    && if kind == 0xf2 {
                                        argument != character
                                    } else {
                                        argument == character
                                    }
                            }
                            0xfd => {
                                let captured = *captures
                                    .get(argument as usize)
                                    .ok_or(Error::InvalidResource)?;
                                if captured != 0 {
                                    character == captured
                                } else if tables
                                    .capture_matches(self.predicate.character, argument as u32)?
                                {
                                    self.predicate.captured = 1;
                                    captures[argument as usize] = character;
                                    true
                                } else {
                                    false
                                }
                            }
                            _ => {
                                let captured = *captures
                                    .get(argument as usize)
                                    .ok_or(Error::InvalidResource)?;
                                captured == 0 || character != captured
                            }
                        };
                        if !matched {
                            break;
                        }
                        self.previous = self.offset;
                        self.offset = self.offset.wrapping_add(self.step);
                        self.skip_tildes(layers, self.step)?;
                        count += 1;
                    }
                }
                self.offset = self.previous;
                self.predicate.cursor += 5;
                self.skip = self.step;
                Ok(((count as u16)
                    >= *code
                        .get(self.predicate.cursor - 4)
                        .ok_or(Error::InvalidResource)? as u16) as i32)
            }
            0xef | 0xf1 => {
                self.predicate.cursor = rule + 1;
                self.saved_layer = Some(self.predicate.current);
                self.predicate.current = (self.operand(code, 0)? as usize)
                    .checked_sub(1)
                    .ok_or(Error::InvalidResource)?;
                self.offset = self.offset.wrapping_sub(self.step);
                self.skip = 0;
                Ok(1)
            }
            0xee | 0xf0 => {
                self.predicate.cursor = rule + 1;
                self.predicate.current = self.saved_layer.ok_or(Error::InvalidResource)?;
                self.offset = self.offset.wrapping_sub(self.step);
                self.skip = 0;
                Ok(1)
            }
            0xec => {
                self.skip_tildes(layers, self.step)?;
                self.predicate.character = self.character(layers)?;
                self.offset = self.offset.wrapping_sub(self.step);
                self.skip = self.step;
                Ok(1)
            }
            _ => {
                let character = self.load_character(layers)?;
                self.skip = self.step;
                Ok((character == opcode) as i32)
            }
        }
    }
}

impl SequenceState {
    fn run_operations(
        &mut self,
        code: &[u8],
        layers: &[&[u8]],
        captures: &mut [u8],
        tables: &CharacterTables<'_>,
        frames: &mut [RepeatFrame],
        scanner: &mut [ScanFrame],
    ) -> Result<i32, Error> {
        let mut result = 1;
        while self.operand(code, 0)? != 0xff && result != 0 {
            result = self.execute_operation(code, layers, captures, tables, frames, scanner)?;
            self.predicate.cursor += 1;
            self.offset = self.offset.wrapping_add(self.step);
        }
        Ok(result)
    }

    pub fn match_sequence(
        &mut self,
        code: &[u8],
        layers: &[&[u8]],
        captures: &mut [u8],
        tables: &CharacterTables<'_>,
        frames: &mut [RepeatFrame],
        scanner: &mut [ScanFrame],
    ) -> Result<i32, Error> {
        self.predicate.cursor = self
            .predicate
            .cursor
            .checked_add(1)
            .ok_or(Error::InvalidResource)?;
        self.skip = self.step;
        self.depth = 0;
        self.active = 1;
        let mut result = self.run_operations(code, layers, captures, tables, frames, scanner)?;
        while result == 0 && self.depth > 0 {
            let index = self.frame_index(frames)?;
            self.predicate.cursor = frames[index].position.ok_or(Error::InvalidResource)?;
            self.offset = frames[index].offset;
            frames[index].repeats = frames[index].repeats.wrapping_add(1);
            self.skip = frames[index].length;
            self.predicate.current = frames[index].current.ok_or(Error::InvalidResource)?;
            match frames[index].command {
                0xf7 => {
                    if frames[index].repeats > self.operand(code, 2)? {
                        self.depth = self.depth.wrapping_sub(1);
                        continue;
                    }
                    self.active = 0;
                    result =
                        self.run_operations(code, layers, captures, tables, frames, scanner)?;
                }
                0xf6 => {
                    if scan_repeat(
                        code,
                        &mut self.predicate.cursor,
                        0xf6,
                        frames[index].repeats as i32,
                        scanner,
                    )? {
                        self.predicate.cursor += 2;
                        result =
                            self.run_operations(code, layers, captures, tables, frames, scanner)?;
                    } else {
                        self.predicate.cursor += 2;
                        self.depth = self.depth.wrapping_sub(1);
                    }
                }
                0xf5 => {
                    self.depth = self.depth.wrapping_sub(1);
                    result =
                        self.run_operations(code, layers, captures, tables, frames, scanner)?;
                }
                _ => return Err(Error::InvalidResource),
            }
        }
        Ok(result)
    }
}

pub fn insert_question_command(
    rows: &mut [&mut [u8]],
    start: i16,
    end: &mut i16,
) -> Result<u32, Error> {
    let at = |row: &[u8], index: i32| -> Result<u8, Error> {
        let index = usize::try_from(index).map_err(|_| Error::InvalidResource)?;
        row.get(index).copied().ok_or(Error::InvalidResource)
    };
    let lower = i32::from(start) - 1;
    let mut scan = i32::from(end.wrapping_sub(2));
    let row = rows.get(1).ok_or(Error::InvalidResource)?;
    while lower < scan && matches!(at(row, scan)?, b'~' | b' ') {
        scan = i32::from((scan as i16).wrapping_sub(1));
    }
    if at(row, scan)? != b'?' {
        return Ok(1);
    }
    let row = rows.first().ok_or(Error::InvalidResource)?;
    let mut last = i32::from(end.wrapping_sub(1));
    while lower < last && matches!(at(row, last)?, 0x14 | b'~' | b' ') {
        last = i32::from((last as i16).wrapping_sub(1));
    }
    let mut count = 0;
    while lower < last && count < 3 {
        let value = at(row, last)?;
        if matches!(value, b' ' | b'-') {
            count += 1;
        }
        if value == b'#' {
            break;
        }
        last = i32::from((last as i16).wrapping_sub(1));
    }
    loop {
        last = i32::from((last as i16).wrapping_add(1));
        if !matches!(at(row, last)?, 0x14 | b'~' | b' ' | b'#') {
            break;
        }
    }
    let insertion = usize::try_from(last).map_err(|_| Error::InvalidResource)?;
    let old_end = usize::try_from(*end).map_err(|_| Error::InvalidResource)?;
    let needed = if insertion <= old_end {
        old_end.checked_add(5)
    } else {
        insertion.checked_add(4)
    }
    .ok_or(Error::InvalidResource)?;
    if rows.len() < 4 {
        return Err(Error::InvalidResource);
    }
    if rows[..4].iter().any(|row| row.len() < needed) {
        return Err(Error::Capacity {
            required: needed,
            available: rows[..4].iter().map(|row| row.len()).min().unwrap_or(0),
        });
    }
    for (index, row) in rows[..4].iter_mut().enumerate() {
        if insertion <= old_end {
            row.copy_within(insertion..=old_end, insertion + 4);
        }
        row[insertion..insertion + 4].copy_from_slice(if index == 0 { b"\\TQ\\" } else { b"~~~~" });
    }
    *end = end.wrapping_add(4);
    Ok(1)
}

pub struct ReplacementRules<'a> {
    pub kinds: &'a [u8],
    pub matrix: &'a [u8],
    pub lengths: &'a [u8],
    pub stride: usize,
    pub default_start: i16,
    pub reset: i16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReplacementState {
    pub active: u8,
    pub tail: i16,
    pub error: u8,
    pub depth: i16,
    pub changed: u8,
    pub output: usize,
    pub cursor: usize,
    pub span: i16,
    pub distance: i16,
    pub variant: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleWork {
    pub rule: usize,
    pub offset: u16,
    pub minimum: u16,
    pub group_end: i16,
    pub input: usize,
    pub output: usize,
    pub reset: i16,
    pub start: i16,
    pub end: i16,
    pub counts: [u16; 176],
    pub length: i16,
    pub maximum: u8,
    pub active: u8,
}

pub struct Expansion<'a> {
    pub status: i16,
    pub separator: u8,
    pub text: &'a mut [u8],
    pub ends: &'a mut [i16],
}

pub trait ReplacementCallback {
    fn expand(&mut self, code: u8, text: &mut [u8]) -> Result<Expansion<'_>, Error>;

    fn execute(
        &mut self,
        state: &mut ReplacementState,
        code: u8,
        kind: u8,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
        start: &mut i16,
        end: &mut i16,
    ) -> Result<u8, Error>;
}

fn text_index(index: i32) -> Result<usize, Error> {
    usize::try_from(index).map_err(|_| Error::InvalidResource)
}

fn text_byte(data: &[u8], index: i32) -> Result<u8, Error> {
    data.get(text_index(index)?)
        .copied()
        .ok_or(Error::InvalidResource)
}

fn text_write(data: &mut [u8], index: i32, value: u8) -> Result<(), Error> {
    let index = text_index(index)?;
    let available = data.len();
    *data.get_mut(index).ok_or(Error::Capacity {
        required: index + 1,
        available,
    })? = value;
    Ok(())
}

fn move_column(rows: &mut [&mut [u8]], destination: i32, source: i32) -> Result<(), Error> {
    for row in rows {
        let value = text_byte(row, source)?;
        text_write(row, destination, value)?;
    }
    Ok(())
}

fn adjust_histogram(
    work: &mut RuleWork,
    rows: &[&mut [u8]],
    begin: i32,
    end: i32,
    add: bool,
) -> Result<(), Error> {
    let row = rows.first().ok_or(Error::InvalidResource)?;
    for index in begin..end {
        let value = text_byte(row, index)? as usize;
        let count = work.counts.get_mut(value).ok_or(Error::InvalidResource)?;
        *count = if add {
            count.wrapping_add(1)
        } else {
            count.wrapping_sub(1)
        };
    }
    Ok(())
}

impl ReplacementState {
    fn finish(work: &mut RuleWork, length: i16) {
        work.length = if work.maximum != 0 {
            work.length.max(length)
        } else {
            length
        };
    }

    fn fail(&mut self, work: &mut RuleWork, rules: &ReplacementRules<'_>) {
        work.length = work.end.wrapping_sub(work.start);
        work.start = rules.default_start;
        work.reset = rules.reset;
        self.depth = self.depth.wrapping_sub(1);
    }

    pub fn replace(
        &mut self,
        code: &[u8],
        index: usize,
        work: &mut RuleWork,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
        captures: &[u8],
        rules: &ReplacementRules<'_>,
        callback: &mut impl ReplacementCallback,
    ) -> Result<(), Error> {
        let byte = |cursor: usize| code.get(cursor).copied().ok_or(Error::InvalidResource);
        let boundary = work.start.wrapping_add(self.span);
        self.output = work.output;
        self.active = 1;
        self.cursor += 1;
        if byte(self.cursor)? == 0xfe {
            self.cursor += 1;
            let command = byte(self.cursor)?;
            if command == 1 || command == 2 {
                let length = self.span.wrapping_sub(1);
                if command == 2 {
                    work.start = rules.default_start;
                    work.reset = rules.reset;
                }
                Self::finish(work, length);
                return Ok(());
            }
            adjust_histogram(
                work,
                rows,
                i32::from(work.start) - 1,
                i32::from(boundary) - 1,
                false,
            )?;
            let mut position = work.end;
            while i32::from(position) > i32::from(boundary) - 2 {
                self.tail = self.tail.wrapping_sub(1);
                move_column(rows, i32::from(self.tail) - 1, i32::from(position))?;
                let marker = text_byte(markers, i32::from(position))?;
                text_write(markers, i32::from(self.tail) - 1, marker)?;
                text_write(markers, i32::from(position), 0)?;
                position = position.wrapping_sub(1);
            }
            work.end = boundary.wrapping_sub(1);
            self.changed = 0;
            self.depth = self.depth.wrapping_add(1);
            let saved = self.distance;
            let kind = *rules
                .kinds
                .get(
                    usize::from(command)
                        .checked_sub(1)
                        .ok_or(Error::InvalidResource)?,
                )
                .ok_or(Error::InvalidResource)?;
            let active = if kind == 3 {
                let row = rows.first_mut().ok_or(Error::InvalidResource)?;
                text_write(row, i32::from(work.end), 0)?;
                let begin = text_index(i32::from(work.start) - 1)?;
                let source = row.get_mut(begin..).ok_or(Error::InvalidResource)?;
                let expansion = callback.expand(command, source)?;
                if expansion.status != 1 {
                    0
                } else if expansion.ends.len() < rows.len() {
                    self.error = 0x9b;
                    0
                } else {
                    expansion.ends[..rows.len()].fill(work.end);
                    let mut offset = 0;
                    let mut maximum = 0i16;
                    for layer in 0..rows.len() {
                        let remaining =
                            expansion.text.get(offset..).ok_or(Error::InvalidResource)?;
                        let zero = remaining
                            .iter()
                            .position(|&value| value == 0)
                            .ok_or(Error::InvalidResource)?;
                        let next = remaining[..=zero]
                            .iter()
                            .position(|&value| value == expansion.separator);
                        let length = next.unwrap_or(zero);
                        if let Some(next) = next {
                            expansion.text[offset + next] = 0;
                        }
                        if length != 0 {
                            for index in 0..length {
                                text_write(
                                    rows[layer],
                                    begin as i32 + index as i32,
                                    expansion.text[offset + index],
                                )?;
                            }
                            expansion.ends[layer] =
                                work.start.wrapping_add(length as i16).wrapping_sub(1);
                        }
                        maximum = maximum.max(expansion.ends[layer]);
                        if next.is_none() {
                            break;
                        }
                        offset += length + 1;
                    }
                    for (layer, row) in rows.iter_mut().enumerate() {
                        for index in i32::from(expansion.ends[layer])..i32::from(maximum) {
                            text_write(row, index, b'~')?;
                        }
                    }
                    work.end = maximum;
                    1
                }
            } else {
                callback.execute(
                    self,
                    command,
                    kind,
                    rows,
                    markers,
                    &mut work.start,
                    &mut work.end,
                )?
            };
            self.active = match kind {
                0 => 1,
                1 => 0,
                _ => active,
            };
            self.distance = saved;
            if i32::from(work.end) >= i32::from(self.tail) - 1 || self.error != 0 {
                if self.error == 0 {
                    self.error = 0x9c;
                }
                work.active = 0;
                self.active = 1;
                if i32::from(work.end) >= i32::from(self.tail) - 1 {
                    work.end = self.tail.wrapping_sub(2);
                }
                self.fail(work, rules);
                return Ok(());
            }
            self.depth = self.depth.wrapping_sub(1);
            if self.changed == 0 {
                let second = rows.get(1).ok_or(Error::InvalidResource)?;
                let is_boundary = text_byte(second, i32::from(work.end) - 1)? == 14;
                let destination = i32::from(work.end) - i32::from(!is_boundary);
                let mut position = i32::from(boundary) - 2;
                while position > destination {
                    let value = text_byte(markers, position)?;
                    if value != 0 {
                        let sum = text_byte(markers, destination)?.wrapping_add(value);
                        text_write(markers, destination, sum)?;
                    }
                    position -= 1;
                }
                if !is_boundary {
                    text_write(markers, i32::from(work.end), 0)?;
                }
                for position in i32::from(boundary)..=i32::from(work.end) {
                    text_write(markers, position - 1, 0)?;
                }
            }
            adjust_histogram(
                work,
                rows,
                i32::from(work.start) - 1,
                i32::from(work.end),
                true,
            )?;
            let length = if self.active != 0 {
                self.distance
                    .wrapping_add(work.end.wrapping_sub(work.start))
            } else {
                0
            };
            let mut position = work.end;
            let value = text_byte(markers, i32::from(position))?;
            if value != 0 {
                let sum = text_byte(markers, i32::from(self.tail) - 1)?.wrapping_add(value);
                text_write(markers, i32::from(self.tail) - 1, sum)?;
            }
            loop {
                position = position.wrapping_add(1);
                move_column(rows, i32::from(position) - 1, i32::from(self.tail) - 1)?;
                let value = text_byte(markers, i32::from(self.tail) - 1)?;
                text_write(markers, i32::from(position) - 1, value)?;
                self.tail = self.tail.wrapping_add(1);
                if text_byte(
                    rows.first().ok_or(Error::InvalidResource)?,
                    i32::from(position) - 1,
                )? == 14
                {
                    break;
                }
            }
            work.end = position.wrapping_sub(1);
            Self::finish(work, length);
            return Ok(());
        }
        let mut length = i16::from(
            *rules
                .lengths
                .get(index.checked_sub(1).ok_or(Error::InvalidResource)?)
                .ok_or(Error::InvalidResource)?,
        );
        let mut padding = 0;
        if byte(self.cursor)? == 0xe9 {
            if length < self.span {
                padding = self.span.wrapping_sub(length);
                length = self.span;
            }
            self.cursor += 1;
        }
        let delta = length.wrapping_sub(self.span);
        length = length.wrapping_sub(1);
        adjust_histogram(
            work,
            rows,
            i32::from(work.start) - 1,
            i32::from(boundary) - 1,
            false,
        )?;
        let old_end = work.end;
        let mut position = old_end;
        let mut full = false;
        if delta > 0 {
            let mut destination = old_end.wrapping_add(delta).wrapping_add(1);
            if i32::from(destination) < i32::from(self.tail) - 1 {
                while i32::from(position) > i32::from(boundary) - 2 {
                    move_column(rows, i32::from(destination) - 1, i32::from(position))?;
                    let value = text_byte(markers, i32::from(position))?;
                    text_write(markers, i32::from(destination) - 1, value)?;
                    destination = destination.wrapping_sub(1);
                    position = position.wrapping_sub(1);
                }
                destination = boundary.wrapping_add(delta).wrapping_sub(1);
                for index in i32::from(boundary)..=i32::from(destination) {
                    text_write(markers, index - 1, 0)?;
                }
            } else {
                full = true;
            }
            position = old_end.wrapping_add(delta);
        } else if delta < 0 {
            let mut sum = 0u16;
            for index in 0..-i32::from(delta) {
                sum = sum.wrapping_add(u16::from(text_byte(
                    markers,
                    i32::from(boundary) + i32::from(delta) + index - 1,
                )?));
            }
            let value = text_byte(markers, i32::from(boundary) - 1)?.wrapping_add(sum as u8);
            text_write(markers, i32::from(boundary) - 1, value)?;
            let mut destination = boundary.wrapping_add(delta);
            for position in i32::from(boundary) - 1..=i32::from(old_end) {
                move_column(rows, i32::from(destination) - 1, position)?;
                let value = text_byte(markers, position)?;
                text_write(markers, i32::from(destination) - 1, value)?;
                destination = destination.wrapping_add(1);
            }
            destination = old_end.wrapping_add(delta).wrapping_add(1);
            for position in i32::from(destination)..=i32::from(old_end) {
                text_write(markers, position, 0)?;
            }
            position = old_end.wrapping_add(delta);
        }
        if full || position == 0 {
            self.error = 0x9c;
            work.active = 0;
            self.active = 1;
            self.fail(work, rules);
            return Ok(());
        }
        work.end = work.end.wrapping_add(delta);
        for row in rows.iter_mut() {
            for index in 0..i32::from(delta) {
                text_write(row, i32::from(boundary) + index - 1, b'~')?;
            }
        }
        position = work.start;
        while byte(self.cursor)? != 0xff {
            match byte(self.cursor)? {
                0xec => {
                    let row = rows.get_mut(self.output).ok_or(Error::InvalidResource)?;
                    for index in 0..i32::from(padding) {
                        text_write(row, i32::from(position) + index - 1, b'~')?;
                    }
                    position = position.wrapping_add(padding);
                }
                opcode @ (0xfd | 0xf8) => {
                    self.cursor += 1;
                    let operand = usize::from(byte(self.cursor)?);
                    let value = if opcode == 0xfd {
                        *captures.get(operand).ok_or(Error::InvalidResource)?
                    } else {
                        let offset = operand
                            .checked_sub(1)
                            .and_then(|v| v.checked_mul(rules.stride))
                            .and_then(|v| v.checked_add(usize::from(self.variant)))
                            .and_then(|v| v.checked_sub(1))
                            .ok_or(Error::InvalidResource)?;
                        *rules.matrix.get(offset).ok_or(Error::InvalidResource)?
                    };
                    text_write(
                        rows.get_mut(self.output).ok_or(Error::InvalidResource)?,
                        i32::from(position) - 1,
                        value,
                    )?;
                    position = position.wrapping_add(1);
                }
                0xef | 0xf1 => {
                    self.cursor += 1;
                    self.output = usize::from(byte(self.cursor)?)
                        .checked_sub(1)
                        .ok_or(Error::InvalidResource)?;
                }
                0xee | 0xf0 => {
                    self.cursor += 1;
                    byte(self.cursor)?;
                    self.output = work.output;
                }
                value => {
                    text_write(
                        rows.get_mut(self.output).ok_or(Error::InvalidResource)?,
                        i32::from(position) - 1,
                        value,
                    )?;
                    position = position.wrapping_add(1);
                }
            }
            self.cursor += 1;
        }
        adjust_histogram(
            work,
            rows,
            i32::from(work.start) - 1,
            i32::from(position) - 1,
            true,
        )?;
        Self::finish(work, length);
        Ok(())
    }
}

pub struct VmRules<'a> {
    pub code: &'a [u8],
    pub module_starts: &'a [u16],
    pub module_ends: &'a [i16],
    pub rules: &'a [usize],
    pub offsets: &'a [u8],
    pub minimums: &'a [u8],
    pub jumps: &'a [u8],
    pub groups: &'a [u8],
    pub required_first: &'a [u8],
    pub required_second: &'a [u8],
    pub header: usize,
    pub capture_count: usize,
    pub replacement: ReplacementRules<'a>,
    pub characters: CharacterTables<'a>,
}

pub struct VmScratch<'a> {
    pub frames: &'a mut [RepeatFrame],
    pub scanner: &'a mut [ScanFrame],
}

pub trait ExpansionCallback {
    fn expand(&mut self, code: u8, text: &mut [u8]) -> Result<Expansion<'_>, Error>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VmState {
    pub sequence: SequenceState,
    pub prefix_saved: Option<usize>,
    pub replacement: ReplacementState,
    pub dispatch_type: i16,
}

struct VmCall<'a, 'r, 's, H> {
    sequence: &'a mut SequenceState,
    prefix_saved: &'a mut Option<usize>,
    dispatch_type: &'a mut i16,
    captures: &'a mut [u8],
    rules: &'a VmRules<'r>,
    scratch: &'a mut VmScratch<'s>,
    host: &'a mut H,
}

impl<H: ExpansionCallback> ReplacementCallback for VmCall<'_, '_, '_, H> {
    fn expand(&mut self, code: u8, text: &mut [u8]) -> Result<Expansion<'_>, Error> {
        self.host.expand(code, text)
    }

    fn execute(
        &mut self,
        state: &mut ReplacementState,
        code: u8,
        kind: u8,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
        start: &mut i16,
        end: &mut i16,
    ) -> Result<u8, Error> {
        match kind {
            0..=2 => self.dispatch(
                state,
                usize::from(code),
                i32::from(*start),
                end,
                rows,
                markers,
            ),
            4 | 5 => {
                let (first, rest) = rows.split_first_mut().ok_or(Error::InvalidResource)?;
                let output = rest
                    .get_mut(if kind == 4 { 2 } else { 0 })
                    .ok_or(Error::InvalidResource)?;
                let result = if kind == 4 {
                    crate::morphology::number_hyphens(first, output, *start, *end)?
                } else {
                    crate::morphology::copy_range(first, output, *start, *end)?
                };
                Ok(result as u8)
            }
            6 => Ok(insert_question_command(rows, *start, end)? as u8),
            _ => Err(Error::InvalidResource),
        }
    }
}

impl<H: ExpansionCallback> VmCall<'_, '_, '_, H> {
    fn match_rule(
        &mut self,
        state: &mut ReplacementState,
        index: usize,
        position: i16,
        work: &mut RuleWork,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
    ) -> Result<(), Error> {
        if rows.len() > 5 {
            return Err(Error::Capacity {
                required: rows.len(),
                available: 5,
            });
        }
        let mut layers: [&[u8]; 5] = [&[]; 5];
        for (destination, source) in layers.iter_mut().zip(rows.iter()) {
            *destination = source;
        }
        let layers = &layers[..rows.len()];
        let sequence = &mut *self.sequence;
        let mut prefix = MatchState {
            predicate: sequence.predicate,
            saved_layer: *self.prefix_saved,
            capture_index: sequence.argument,
            repeat_limit: sequence.maximum,
            step: sequence.skip,
            next_position: sequence.previous,
            restarted: state.distance,
        };
        let next = prefix.match_prefix(
            self.rules.code,
            position,
            i32::from(work.end),
            layers,
            self.captures,
            &self.rules.characters,
        )?;
        sequence.predicate = prefix.predicate;
        *self.prefix_saved = prefix.saved_layer;
        sequence.argument = prefix.capture_index;
        sequence.maximum = prefix.repeat_limit;
        sequence.skip = prefix.step;
        sequence.previous = prefix.next_position;
        state.distance = prefix.restarted;
        let mut matched = false;
        if next != 0 {
            state.span = next.wrapping_sub(work.start);
            sequence.step = -1;
            sequence.offset = work.start.wrapping_sub(1);
            if sequence.match_sequence(
                self.rules.code,
                layers,
                self.captures,
                &self.rules.characters,
                self.scratch.frames,
                self.scratch.scanner,
            )? != 0
            {
                sequence.offset = next;
                sequence.step = 1;
                matched = sequence.match_sequence(
                    self.rules.code,
                    layers,
                    self.captures,
                    &self.rules.characters,
                    self.scratch.frames,
                    self.scratch.scanner,
                )? != 0;
            }
        }
        state.cursor = sequence.predicate.cursor;
        state.variant = sequence.predicate.value;
        if matched {
            self.replace(state, index, work, rows, markers)?;
            if state.active != 0 {
                work.active = 1;
            }
        }
        self.sequence.predicate.cursor = state.cursor;
        if self.sequence.predicate.captured != 0 {
            let length = self
                .rules
                .capture_count
                .checked_add(1)
                .ok_or(Error::InvalidResource)?;
            let available = self.captures.len();
            self.captures
                .get_mut(..length)
                .ok_or(Error::Capacity {
                    required: length,
                    available,
                })?
                .fill(0);
            self.sequence.predicate.captured = 0;
        }
        self.sequence.predicate.current = work.input;
        Ok(())
    }

    fn replace(
        &mut self,
        state: &mut ReplacementState,
        index: usize,
        work: &mut RuleWork,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
    ) -> Result<(), Error> {
        let rules = self.rules;
        let mut captures = [0u8; 83];
        if self.captures.len() > captures.len() {
            return Err(Error::Capacity {
                required: self.captures.len(),
                available: captures.len(),
            });
        }
        captures[..self.captures.len()].copy_from_slice(self.captures);
        state.replace(
            rules.code,
            index,
            work,
            rows,
            markers,
            &captures[..self.captures.len()],
            &rules.replacement,
            self,
        )?;
        self.sequence.predicate.cursor = state.cursor;
        self.sequence.predicate.value = state.variant;
        Ok(())
    }

    fn metadata(values: &[u8], index: i16) -> Result<u8, Error> {
        let index = text_index(i32::from(index) - 1)?;
        values.get(index).copied().ok_or(Error::InvalidResource)
    }

    fn set_rule(&mut self, index: i16, advance: usize) -> Result<(), Error> {
        let offset = *self
            .rules
            .rules
            .get(text_index(i32::from(index) - 1)?)
            .ok_or(Error::InvalidResource)?;
        self.sequence.predicate.cursor = offset
            .checked_add(self.rules.header)
            .and_then(|v| v.checked_add(advance))
            .ok_or(Error::InvalidResource)?;
        Ok(())
    }

    fn required(&self, work: &RuleWork, index: i16, both: bool) -> Result<bool, Error> {
        let first = Self::metadata(self.rules.required_first, index)? as usize;
        let second = Self::metadata(self.rules.required_second, index)? as usize;
        let first = *work.counts.get(first).ok_or(Error::InvalidResource)? != 0;
        Ok(first && (!both || *work.counts.get(second).ok_or(Error::InvalidResource)? != 0))
    }

    fn dispatch(
        &mut self,
        state: &mut ReplacementState,
        module: usize,
        start: i32,
        end: &mut i16,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
    ) -> Result<u8, Error> {
        if state.error != 0 {
            return Ok(0);
        }
        let next_start = start.wrapping_add(1) as i16;
        let length = i32::from(*end);
        for row in rows.iter_mut() {
            for position in (start..=length).rev() {
                let value = text_byte(row, position - 1)?;
                text_write(row, position, value)?;
            }
            text_write(row, length + 1, 14)?;
            text_write(row, start - 1, 14)?;
        }
        for position in (start..=length).rev() {
            let value = text_byte(markers, position - 1)?;
            text_write(markers, position, value)?;
        }
        text_write(markers, length + 1, 0)?;
        text_write(markers, start - 1, 0)?;
        *end = end.wrapping_add(1);
        self.sequence.predicate.current = 0;
        state.output = 0;
        let mut work = RuleWork {
            rule: 0,
            offset: 0,
            minimum: 0,
            group_end: 0,
            input: 0,
            output: 0,
            reset: 0,
            start: 0,
            end: *end,
            counts: [0; 176],
            length: 0,
            maximum: 0,
            active: 0,
        };
        for position in start..i32::from(work.end) {
            let value = text_byte(rows.first().ok_or(Error::InvalidResource)?, position)? as usize;
            if value > 175 {
                state.error = 0x9c;
                return Ok(0);
            }
            work.counts[value] = work.counts[value].wrapping_add(1);
        }
        work.counts[14] = 2;
        let module = module.checked_sub(1).ok_or(Error::InvalidResource)?;
        work.reset = (*self
            .rules
            .module_starts
            .get(module)
            .ok_or(Error::InvalidResource)?)
        .wrapping_sub(1) as i16;
        if work.reset < 0 {
            return Ok(0);
        }
        let last = *self
            .rules
            .module_ends
            .get(module)
            .ok_or(Error::InvalidResource)?;
        while work.reset < last {
            let index = text_index(i32::from(work.reset))?;
            work.rule = *self.rules.rules.get(index).ok_or(Error::InvalidResource)?;
            work.offset = u16::from(
                *self
                    .rules
                    .offsets
                    .get(index)
                    .ok_or(Error::InvalidResource)?,
            );
            work.reset = work.reset.wrapping_add(1);
            work.minimum = u16::from(Self::metadata(self.rules.minimums, work.reset)?);
            let tag = *self
                .rules
                .code
                .get(
                    work.rule
                        .checked_add(self.rules.header)
                        .and_then(|v| v.checked_sub(1))
                        .ok_or(Error::InvalidResource)?,
                )
                .ok_or(Error::InvalidResource)?;
            *self.dispatch_type = i16::from(if tag > 100 { tag - 100 } else { tag });
            let kind = *self.dispatch_type;
            if kind == 99 {
                let cursor = work
                    .rule
                    .checked_add(self.rules.header)
                    .ok_or(Error::InvalidResource)?;
                work.input =
                    usize::from(*self.rules.code.get(cursor).ok_or(Error::InvalidResource)?)
                        .checked_sub(1)
                        .ok_or(Error::InvalidResource)?;
                work.output = usize::from(
                    *self
                        .rules
                        .code
                        .get(cursor + 1)
                        .ok_or(Error::InvalidResource)?,
                )
                .checked_sub(1)
                .ok_or(Error::InvalidResource)?;
                if work.input >= rows.len() || work.output >= rows.len() {
                    return Err(Error::InvalidResource);
                }
                self.sequence.predicate.current = work.input;
                state.output = work.output;
                continue;
            }
            if !(1..=16).contains(&kind) {
                continue;
            }
            if kind <= 8 && !self.required(&work, work.reset, true)? {
                continue;
            }
            let forward = kind % 2 != 0;
            let lower = i32::from(work.offset) + i32::from(next_start);
            work.start = if forward {
                lower as i16
            } else {
                (i32::from(work.end) - i32::from(work.minimum) + 1) as i16
            };
            if matches!(kind, 3 | 4 | 7 | 8 | 11 | 12 | 15 | 16) {
                state.active = 0;
            }
            if matches!(kind, 9 | 13) {
                work.maximum = 1;
            }
            if matches!(kind, 11 | 12) {
                work.group_end = work
                    .reset
                    .wrapping_add(i16::from(Self::metadata(self.rules.groups, work.reset)?));
            }
            loop {
                let upper = i32::from(work.end) - i32::from(work.minimum)
                    + i32::from(!matches!(kind, 1 | 3));
                if (forward && i32::from(work.start) > upper)
                    || (!forward && i32::from(work.start) < lower)
                    || state.error != 0
                {
                    break;
                }
                if matches!(kind, 7 | 8 | 11 | 12 | 15 | 16) && state.active != 0 {
                    break;
                }
                if kind <= 4 {
                    let character = *self
                        .rules
                        .code
                        .get(work.rule + self.rules.header)
                        .ok_or(Error::InvalidResource)?;
                    let current = rows
                        .get(self.sequence.predicate.current)
                        .ok_or(Error::InvalidResource)?;
                    if text_byte(current, i32::from(work.start) - 1)? == character {
                        if kind == 1 {
                            work.length = 0;
                        }
                        self.set_rule(work.reset, 1)?;
                        self.sequence.skip = 1;
                        self.match_rule(
                            state,
                            text_index(i32::from(work.reset))?,
                            work.start.wrapping_add(1),
                            &mut work,
                            rows,
                            markers,
                        )?;
                        if kind == 1 {
                            work.start = work.start.wrapping_add(work.length);
                        }
                    }
                    work.start = work.start.wrapping_add(if forward { 1 } else { -1 });
                    if matches!(kind, 3 | 4) && state.active == 1 {
                        break;
                    }
                } else if kind <= 8 {
                    if kind == 5 {
                        work.length = 0;
                    }
                    self.set_rule(work.reset, 0)?;
                    self.sequence.skip = 0;
                    self.match_rule(
                        state,
                        text_index(i32::from(work.reset))?,
                        work.start,
                        &mut work,
                        rows,
                        markers,
                    )?;
                    let step = if kind == 5 {
                        work.length.wrapping_add(1)
                    } else if forward {
                        1
                    } else {
                        -1
                    };
                    work.start = work.start.wrapping_add(step);
                } else {
                    if matches!(kind, 9 | 13) {
                        work.length = -2;
                    }
                    if matches!(kind, 9 | 10) {
                        work.group_end = work.reset.wrapping_add(i16::from(Self::metadata(
                            self.rules.groups,
                            work.reset,
                        )?));
                    }
                    if matches!(kind, 13 | 14) {
                        state.active = 0;
                    }
                    let mut offset = 0i16;
                    let mut member = work.reset;
                    loop {
                        if kind <= 12 {
                            if member > work.group_end {
                                break;
                            }
                        } else {
                            let group_length =
                                i16::from(Self::metadata(self.rules.groups, work.reset)?);
                            if offset > group_length || state.active != 0 || state.error != 0 {
                                break;
                            }
                            member = work.reset.wrapping_add(offset);
                        }
                        if kind != 9 && state.error != 0 {
                            break;
                        }
                        if self.required(&work, member, false)? {
                            self.set_rule(member, 0)?;
                            self.sequence.skip = 0;
                            self.match_rule(
                                state,
                                text_index(i32::from(member))?,
                                work.start,
                                &mut work,
                                rows,
                                markers,
                            )?;
                        }
                        member = member.wrapping_add(1);
                        offset = offset.wrapping_add(1);
                        if kind == 13 && work.reset > self.rules.replacement.reset {
                            break;
                        }
                    }
                    let step = match kind {
                        9 => {
                            if work.length == -2 {
                                1
                            } else {
                                work.length.wrapping_add(1)
                            }
                        }
                        13 => {
                            if state.active != 0 {
                                work.length.wrapping_add(1)
                            } else {
                                1
                            }
                        }
                        _ => {
                            if forward {
                                1
                            } else {
                                -1
                            }
                        }
                    };
                    work.start = work.start.wrapping_add(step);
                    if kind == 13 && work.reset > self.rules.replacement.reset {
                        break;
                    }
                }
            }
            let jump = match kind {
                3 | 4 | 7 | 8 if state.active == 1 => Some(self.rules.jumps),
                9 | 10 | 14 => Some(self.rules.groups),
                11 | 12 | 15 | 16 => Some(if state.active != 0 {
                    self.rules.jumps
                } else {
                    self.rules.groups
                }),
                13 if work.reset < self.rules.replacement.reset => Some(self.rules.groups),
                _ => None,
            };
            if let Some(values) = jump {
                work.reset = work
                    .reset
                    .wrapping_add(i16::from(Self::metadata(values, work.reset)?));
            }
            if matches!(kind, 9 | 13) {
                work.maximum = 0;
            }
        }
        *end = work.end;
        let mut position = i32::from(next_start);
        while position < i32::from(work.end) + 1 {
            move_column(rows, position - 2, position - 1)?;
            let value = text_byte(markers, position - 1)?;
            text_write(markers, position - 2, value)?;
            text_write(markers, position - 1, 0)?;
            position += 1;
        }
        let value =
            text_byte(markers, position - 1)?.wrapping_add(text_byte(markers, position - 2)?);
        text_write(markers, position - 2, value)?;
        text_write(markers, position - 1, 0)?;
        *end = end.wrapping_sub(1);
        if state.depth == 0 && *end != 0 {
            let value = text_byte(markers, i32::from(*end) - 1)?
                .wrapping_add(text_byte(markers, i32::from(*end))?);
            text_write(markers, i32::from(*end) - 1, value)?;
        }
        state.changed = 1;
        if state.error != 0 {
            *end = next_start.wrapping_sub(2);
        }
        Ok(work.active)
    }
}

impl VmState {

    pub fn replace(
        &mut self,
        index: usize,
        work: &mut RuleWork,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
        captures: &mut [u8],
        rules: &VmRules<'_>,
        scratch: &mut VmScratch<'_>,
        host: &mut impl ExpansionCallback,
    ) -> Result<(), Error> {
        VmCall {
            sequence: &mut self.sequence,
            prefix_saved: &mut self.prefix_saved,
            dispatch_type: &mut self.dispatch_type,
            captures,
            rules,
            scratch,
            host,
        }
        .replace(&mut self.replacement, index, work, rows, markers)
    }

    pub fn dispatch(
        &mut self,
        module: usize,
        start: i32,
        end: &mut i16,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
        captures: &mut [u8],
        rules: &VmRules<'_>,
        scratch: &mut VmScratch<'_>,
        host: &mut impl ExpansionCallback,
    ) -> Result<u8, Error> {
        VmCall {
            sequence: &mut self.sequence,
            prefix_saved: &mut self.prefix_saved,
            dispatch_type: &mut self.dispatch_type,
            captures,
            rules,
            scratch,
            host,
        }
        .dispatch(&mut self.replacement, module, start, end, rows, markers)
    }

    pub fn match_rule(
        &mut self,
        index: usize,
        position: i16,
        work: &mut RuleWork,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
        captures: &mut [u8],
        rules: &VmRules<'_>,
        scratch: &mut VmScratch<'_>,
        host: &mut impl ExpansionCallback,
    ) -> Result<(), Error> {
        VmCall {
            sequence: &mut self.sequence,
            prefix_saved: &mut self.prefix_saved,
            dispatch_type: &mut self.dispatch_type,
            captures,
            rules,
            scratch,
            host,
        }
        .match_rule(&mut self.replacement, index, position, work, rows, markers)
    }
}

impl VmState {

    pub fn execute_module(
        &mut self,
        name: &[u8],
        sequences: &[u8],
        end: &mut i16,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
        captures: &mut [u8],
        rules: &VmRules<'_>,
        scratch: &mut VmScratch<'_>,
        host: &mut impl ExpansionCallback,
    ) -> Result<u32, Error> {
        let name = &name[..name
            .iter()
            .position(|&value| value == 0)
            .unwrap_or(name.len())];
        let mut offset = 0usize;
        let modules = loop {
            if offset >= sequences.len() {
                return Ok(0x89702004);
            }
            let rest = &sequences[offset..];
            let length = rest
                .iter()
                .position(|&value| value == 0)
                .ok_or(Error::InvalidResource)?;
            let position = offset
                .checked_add(length + 1)
                .and_then(|value| value.checked_add(3))
                .ok_or(Error::InvalidResource)?
                & !3;
            let count = sequences
                .get(position..position + 4)
                .ok_or(Error::InvalidResource)?;
            let count =
                u32::from_le_bytes(count.try_into().map_err(|_| Error::InvalidResource)?) as usize;
            let position = position.checked_add(4).ok_or(Error::InvalidResource)?;
            let next = position.checked_add(count).ok_or(Error::InvalidResource)?;
            if rest[..length].eq_ignore_ascii_case(name) {
                break sequences
                    .get(position..next)
                    .ok_or(Error::InvalidResource)?;
            }
            offset = next;
        };
        if modules.len() > 104 {
            return Err(Error::Capacity {
                required: modules.len(),
                available: 104,
            });
        }
        let mut copied = [0u8; 104];
        copied[..modules.len()].copy_from_slice(modules);
        self.replacement.depth = 0;
        self.replacement.error = 0;
        self.replacement.tail = rules.replacement.default_start;
        let available = captures.len();
        captures
            .get_mut(..rules.capture_count)
            .ok_or(Error::Capacity {
                required: rules.capture_count,
                available,
            })?
            .fill(0);
        self.sequence.predicate.captured = 1;
        for &module in &copied[..modules.len()] {
            self.dispatch(
                usize::from(module),
                1,
                end,
                rows,
                markers,
                captures,
                rules,
                scratch,
                host,
            )?;
        }
        Ok(match self.replacement.error {
            0x9b => 0x8970200a,
            0x9c => 0x89702000,
            _ => 0,
        })
    }
}

pub trait HandleValidator {
    fn validate(&mut self, handle: u32, tag: u32, size: u32) -> u32;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModuleOptions {
    pub first: i32,
    pub second: i32,
}

impl ModuleOptions {

    pub fn set(
        &mut self,
        handle: u32,
        value: i32,
        which: i32,
        validator: &mut impl HandleValidator,
    ) -> u32 {
        let status = validator.validate(handle, 0xf37e, 0x40);
        if status >= 0x80000000 {
            return 0x89702008;
        }
        if which == 1 {
            self.first = value;
        } else {
            self.second = value;
        }
        status
    }

    pub fn get(
        &self,
        handle: u32,
        output: &mut i32,
        which: i32,
        validator: &mut impl HandleValidator,
    ) -> u32 {
        let status = validator.validate(handle, 0xf37e, 0x40);
        if status >= 0x80000000 {
            return 0x89702008;
        }
        *output = if which == 1 { self.first } else { self.second };
        status
    }
}

pub fn select_interface<'a, T>(kind: u32, interface: &'a T, output: &mut Option<&'a T>) -> u32 {
    if kind <= 1 {
        *output = Some(interface);
        0
    } else {
        0x89702001
    }
}

pub fn get_layer<'a>(
    handle: u32,
    index: i32,
    layers: &'a [&'a [u8]],
    count: i16,
    length: u16,
    output: Option<&mut Option<&'a [u8]>>,
    output_length: &mut u16,
    validator: &mut impl HandleValidator,
) -> Result<u32, Error> {
    let status = validator.validate(handle, 0xf37e, 0x40);
    if status >= 0x80000000 {
        return Ok(0x89702008);
    }
    if i32::from(count) <= index || output.is_none() {
        return Ok(0x89702007);
    }
    let index = usize::try_from(index).map_err(|_| Error::InvalidResource)?;
    let layer = *layers.get(index).ok_or(Error::InvalidResource)?;
    *output_length = length;
    *output.ok_or(Error::InvalidResource)? = Some(layer);
    Ok(status)
}

pub fn get_markers<'a>(
    handle: u32,
    markers: &'a [u8],
    length: u16,
    output: Option<&mut Option<&'a [u8]>>,
    output_length: &mut u16,
    validator: &mut impl HandleValidator,
) -> u32 {
    let status = validator.validate(handle, 0xf37e, 0x40);
    if status >= 0x80000000 {
        return 0x89702008;
    }
    if let Some(output) = output {
        *output_length = length;
        *output = Some(markers);
        status
    } else {
        0x89702007
    }
}

pub fn set_markers(
    handle: u32,
    source: Option<&[u8]>,
    length: u32,
    markers: &mut [u8],
    total: u16,
    limit: i16,
    validator: &mut impl HandleValidator,
) -> Result<u32, Error> {
    let status = validator.validate(handle, 0xf37e, 0x40);
    if status >= 0x80000000 {
        return Ok(0x89702008);
    }
    let length = length.min(10000) as usize;
    if source.is_none() || length as i32 > i32::from(limit) {
        return Ok(0x89702007);
    }
    let source = source.ok_or(Error::InvalidResource)?;
    if length > source.len() {
        return Err(Error::Capacity {
            required: length,
            available: source.len(),
        });
    }
    let total = usize::from(total);
    if total < length {
        return Err(Error::InvalidResource);
    }
    let available = markers.len();
    let markers = markers.get_mut(..total).ok_or(Error::Capacity {
        required: total,
        available,
    })?;
    markers[..length].copy_from_slice(&source[..length]);
    markers[length..].fill(0);
    Ok(status)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayerState {
    pub count: u16,
    pub capacity: u16,
    pub length: u16,
}

impl LayerState {

    pub fn set_layer_ex(
        &mut self,
        handle: u32,
        index: i32,
        source: Option<&[u8]>,
        length: u32,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
        validator: &mut impl HandleValidator,
    ) -> Result<u32, Error> {
        let status = validator.validate(handle, 0xf37e, 0x40);
        if status >= 0x80000000 {
            return Ok(0x89702008);
        }
        if index >= i32::from(self.count as i16) || source.is_none() {
            return Ok(status);
        }
        let count = usize::from(self.count);
        let index = usize::try_from(index).map_err(|_| Error::InvalidResource)?;
        let length = length.min(10000) as usize;
        let needed = length * 3;
        if count > rows.len() {
            return Err(Error::Capacity {
                required: count,
                available: rows.len(),
            });
        }
        if needed as i32 > i32::from(self.capacity as i16) {
            let previous =
                usize::try_from(self.capacity as i16).map_err(|_| Error::InvalidResource)?;
            for row in &mut rows[..count] {
                let available = row.len();
                let row = row.get_mut(previous..needed).ok_or(Error::Capacity {
                    required: needed,
                    available,
                })?;
                row.fill(b' ');
            }
            let available = markers.len();
            markers
                .get_mut(previous..needed)
                .ok_or(Error::Capacity {
                    required: needed,
                    available,
                })?
                .fill(0);
            self.capacity = needed as u16;
        }
        let source = source.ok_or(Error::InvalidResource)?;
        if length > source.len() {
            return Err(Error::Capacity {
                required: length,
                available: source.len(),
            });
        }
        let row = rows.get_mut(index).ok_or(Error::InvalidResource)?;
        let available = row.len();
        let row = row
            .get_mut(..usize::from(self.capacity))
            .ok_or(Error::Capacity {
                required: usize::from(self.capacity),
                available,
            })?;
        if length > row.len() {
            return Err(Error::InvalidResource);
        }
        row[..length].copy_from_slice(&source[..length]);
        row[length..].fill(b' ');
        if index == 0 {
            for row in &mut rows[1..count] {
                let available = row.len();
                row.get_mut(..usize::from(self.capacity))
                    .ok_or(Error::Capacity {
                        required: usize::from(self.capacity),
                        available,
                    })?
                    .fill(b' ');
            }
            self.length = length as u16;
        } else if (self.length as i16) < length as i16 {
            self.length = length as u16;
        }
        Ok(status)
    }

    pub fn set_layer(
        &mut self,
        handle: u32,
        index: i32,
        source: Option<&[u8]>,
        length_bits: u32,
        rows: &mut [&mut [u8]],
        markers: &mut [u8],
        validator: &mut impl HandleValidator,
    ) -> Result<u32, Error> {
        if validator.validate(handle, 0xf37e, 0x40) >= 0x80000000 {
            return Ok(0x89702008);
        }
        if i32::from(self.count as i16) <= index || source.is_none() {
            return Ok(0x89702007);
        }
        self.set_layer_ex(handle, index, source, length_bits, rows, markers, validator)
    }
}

pub trait ModuleExecutor {
    fn execute(&mut self, grammar: usize, name: &[u8], end: &mut i16) -> Result<u32, Error>;
}

pub struct GrammarInstance<'a, 'r> {
    pub state: &'a mut VmState,
    pub sequences: &'r [u8],
    pub captures: &'a mut [u8],
    pub rules: &'a VmRules<'r>,
}

pub struct VmModules<'a, 'r, 's, 'b, H> {
    pub grammars: &'a mut [GrammarInstance<'a, 'r>],
    pub rows: &'a mut [&'b mut [u8]],
    pub markers: &'a mut [u8],
    pub scratch: &'a mut VmScratch<'s>,
    pub host: &'a mut H,
}

impl<H: ExpansionCallback> ModuleExecutor for VmModules<'_, '_, '_, '_, H> {
    fn execute(&mut self, grammar: usize, name: &[u8], end: &mut i16) -> Result<u32, Error> {
        let grammar = self
            .grammars
            .get_mut(grammar)
            .ok_or(Error::InvalidResource)?;
        grammar.state.execute_module(
            name,
            grammar.sequences,
            end,
            self.rows,
            self.markers,
            grammar.captures,
            grammar.rules,
            self.scratch,
            self.host,
        )
    }
}

pub fn execute_named_module(
    handle: u32,
    name: &[u8],
    first_active: bool,
    second_active: bool,
    end: &mut i16,
    validator: &mut impl HandleValidator,
    executor: &mut impl ModuleExecutor,
) -> Result<u32, Error> {
    if validator.validate(handle, 0xf37e, 0x40) >= 0x80000000 {
        return Ok(0x89702008);
    }
    let mut result = 0x89702004;
    if second_active {
        result = executor.execute(1, name, end)?;
    }
    if first_active && result == 0x89702004 {
        result = executor.execute(0, name, end)?;
    }
    Ok(result)
}

pub trait ResourceParameters<'a> {
    fn get(&mut self, name: &[u8]) -> (u32, Option<&'a [u8]>);
}

fn parameter_text(text: &[u8]) -> &[u8] {
    &text[..text
        .iter()
        .position(|&value| value == 0)
        .unwrap_or(text.len())]
}

fn append_path(output: &mut [u8], position: &mut usize, text: &[u8]) -> Result<(), Error> {
    let text = parameter_text(text);
    let end = position
        .checked_add(text.len())
        .ok_or(Error::InvalidResource)?;
    let required = end.checked_add(1).ok_or(Error::InvalidResource)?;
    let available = output.len();
    let destination = output.get_mut(*position..required).ok_or(Error::Capacity {
        required,
        available,
    })?;
    destination[..text.len()].copy_from_slice(text);
    destination[text.len()] = 0;
    *position = end;
    Ok(())
}

fn resource_path<'a>(
    voice_specific: bool,
    mode: i32,
    output: &mut [u8],
    parameters: &mut impl ResourceParameters<'a>,
) -> Result<u32, Error> {
    let (status, language) = parameters.get(b"langcode");
    if status >= 0x80000000 {
        return Ok(status);
    }
    let voice = if voice_specific {
        let (status, voice) = parameters.get(b"voice");
        if status >= 0x80000000 {
            return Ok(status);
        }
        voice
    } else {
        None
    };
    let (status, prefix) = parameters.get(b"fecfg");
    if status >= 0x80000000 {
        return Ok(status);
    }
    let mut position = 0;
    append_path(output, &mut position, b"")?;
    let (config_status, config) = parameters.get(b"fedataprefix");
    if config_status < 0x80000000 {
        if let Some(config) = config {
            if !parameter_text(config).is_empty() {
                append_path(output, &mut position, config)?;
            }
        }
    }
    let (packaging_status, packaging) = parameters.get(b"fedatapackaging");
    let language = language.ok_or(Error::InvalidResource)?;
    if packaging_status < 0x80000000
        && packaging.is_some_and(|value| parameter_text(value) == b"clc")
    {
        append_path(output, &mut position, b"clc/")?;
        append_path(output, &mut position, language)?;
        if voice_specific {
            let voice = voice.ok_or(Error::InvalidResource)?;
            if !parameter_text(voice).windows(3).any(|part| part == b"uni") {
                append_path(output, &mut position, b"/")?;
                append_path(output, &mut position, voice)?;
            }
        }
        append_path(output, &mut position, b"/")?;
        append_path(output, &mut position, prefix.ok_or(Error::InvalidResource)?)?;
        append_path(output, &mut position, b":")?;
    }
    append_path(output, &mut position, b"depes/")?;
    append_path(output, &mut position, language)?;
    if voice_specific {
        let voice = voice.ok_or(Error::InvalidResource)?;
        append_path(output, &mut position, b"/")?;
        append_path(output, &mut position, voice)?;
    }
    if mode == 0 {
        append_path(output, &mut position, b"/")?;
        append_path(output, &mut position, prefix.ok_or(Error::InvalidResource)?)?;
    }
    for byte in &mut output[..usize::from(position as u16)] {
        *byte = crate::selection::lowercase_voice_byte(u32::from(*byte)) as u8;
    }
    Ok(status)
}

pub fn voice_resource_path<'a>(
    mode: i32,
    output: &mut [u8],
    parameters: &mut impl ResourceParameters<'a>,
) -> Result<u32, Error> {
    resource_path(true, mode, output, parameters)
}

pub fn language_resource_path<'a>(
    mode: i32,
    output: &mut [u8],
    parameters: &mut impl ResourceParameters<'a>,
) -> Result<u32, Error> {
    resource_path(false, mode, output, parameters)
}

#[derive(Clone, Copy)]
pub struct ResourceBytes<'a> {
    pub data: &'a [u8],
    pub alignment: u8,
}

pub trait ResourceFiles<'a> {
    fn open(&mut self, handle: u32, mode: u32, output: &mut u32) -> u32;
    fn read(&mut self, handle: u32, offset: u32, length: u32) -> (u32, Option<ResourceBytes<'a>>);
    fn release(&mut self, handle: u32, data: ResourceBytes<'a>);
    fn close(&mut self, handle: u32);
}

fn resource_word(data: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = data.get(offset..offset + 4).ok_or(Error::InvalidResource)?;
    Ok(u32::from_le_bytes(
        bytes.try_into().map_err(|_| Error::InvalidResource)?,
    ))
}

pub fn open_resource<'a>(
    handle: u32,
    output_handle: &mut u32,
    output_size: &mut u32,
    files: &mut impl ResourceFiles<'a>,
) -> Result<u32, Error> {
    if files.open(handle, 1, output_handle) >= 0x80000000 {
        *output_handle = 0;
        return Ok(0);
    }
    let (status, resource) = files.read(*output_handle, 0, 0x48);
    if status >= 0x80000000 {
        files.close(*output_handle);
        *output_handle = 0;
        return Ok(status);
    }
    let resource = resource.ok_or(Error::InvalidResource)?;
    if resource.alignment & 3 != 0
        || resource_word(resource.data, 0)? != 0xfeff
        || !resource
            .data
            .get(4..12)
            .ok_or(Error::InvalidResource)?
            .eq_ignore_ascii_case(b"SCANSOFT")
        || !resource
            .data
            .get(12..17)
            .ok_or(Error::InvalidResource)?
            .eq_ignore_ascii_case(b"depes")
    {
        return Ok(0x89702019);
    }
    let total_size = resource_word(resource.data, 20)?;
    files.release(*output_handle, resource);
    *output_size = total_size.wrapping_sub(0x48);
    Ok(status)
}

pub trait DepesHost {

    fn allocate(&mut self, count: u32, size: u32) -> Option<u32>;

    fn allocate_block(&mut self, size: u32) -> Option<u32>;

    fn free(&mut self, address: u32);

    fn validate(&mut self, handle: u32, tag: u32, size: u32) -> u32;

    fn resolve(&mut self, params: u32, name: &[u8]) -> (u32, u32);

    fn acquire(&mut self, name: &[u8], metadata: &mut [u32; 3]) -> u32;

    fn release(&mut self, name: &[u8]);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GrammarHeader {

    pub size: u32,

    pub table: u32,

    pub layer_count: i16,

    pub string_count: u32,

    pub record_count: u32,

    pub record_count_2: u32,

    pub record_count_3: u32,

    pub word_count: i16,

    pub short_count: i16,

    pub value_24: u32,

    pub value_28: u32,

    pub sections: [u32; 8],

    pub values: [u32; 8],
}

pub struct GrammarState<'a> {
    pub header: GrammarHeader,
    pub strings: &'a mut [u32],
    pub layers: &'a mut [u32],
    pub string_data: &'a mut [u8],
    pub capacity: Option<u32>,
    pub allocator: u32,
    pub metadata: [u32; 3],
}

fn byte_at(data: &[u8], offset: u32) -> u8 {
    data.get(offset as usize).copied().unwrap_or(0)
}

fn word_at(data: &[u8], offset: u32) -> Option<u32> {
    let bytes = data.get(offset as usize..offset as usize + 4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

fn name_length(data: &[u8], offset: u32) -> u32 {
    let mut length = 0u32;
    while byte_at(data, offset.wrapping_add(length)) != 0 {
        length = length.wrapping_add(1);
    }
    length
}

fn names_match(data: &[u8], offset: u32, name: &[u8]) -> bool {
    let mut index = 0u32;
    loop {
        let left = byte_at(data, offset.wrapping_add(index));
        let right = name.get(index as usize).copied().unwrap_or(0);
        if left.to_ascii_lowercase() != right.to_ascii_lowercase() {
            return false;
        }
        if left == 0 {
            return true;
        }
        index = index.wrapping_add(1);
    }
}

pub fn load_grammar<'a>(
    name: &[u8],
    handle: u32,
    mut file_size: u32,
    grammar: &mut GrammarState<'_>,
    files: &mut impl ResourceFiles<'a>,
) -> Result<(u32, &'a [u8]), Error> {
    let (status, region) = files.read(handle, 0x48, file_size);
    if status >= 0x8000_0000 {
        return Err(Error::Engine(status));
    }
    let region = region.ok_or(Error::InvalidResource)?;

    let base = region.data;

    let mut offset = 0u32;
    loop {
        if names_match(base, offset, name) || file_size <= offset {
            break;
        }
        offset = offset.wrapping_add(name_length(base, offset).wrapping_add(1));
        offset = offset.wrapping_add(3) & !3;
        offset = offset.wrapping_add(8);
    }
    if offset >= file_size {
        files.release(handle, region);
        return Err(Error::Engine(0x89702004));
    }

    offset = offset.wrapping_add(name_length(base, offset).wrapping_add(1));
    offset = offset.wrapping_add(3) & !3;
    let section = word_at(base, offset).ok_or(Error::InvalidResource)?;
    file_size = word_at(base, offset.wrapping_add(4)).ok_or(Error::InvalidResource)?;
    offset = section;
    files.release(handle, region);

    let (status, region) = files.read(handle, offset, file_size);
    if status >= 0x8000_0000 {
        return Err(Error::Engine(status));
    }
    let region = region.ok_or(Error::InvalidResource)?;
    let window = region.data;
    let data = window
        .get(..file_size as usize)
        .ok_or(Error::InvalidResource)?;

    let mut offset = 0u32;
    let mut word = [0u8; 4];
    read_aligned(data, &mut offset, &mut word)?;
    grammar.header.size = u32::from_le_bytes(word);
    let start = offset;
    read_bytes(data, &mut offset, grammar.header.size)?;
    grammar.header.table = start;

    read_aligned(data, &mut offset, &mut word)?;
    grammar.header.layer_count = u32::from_le_bytes(word) as u16 as i16;
    read_aligned(data, &mut offset, &mut word)?;
    grammar.header.string_count = u32::from_le_bytes(word);
    read_aligned(data, &mut offset, &mut word)?;
    grammar.header.record_count = u32::from_le_bytes(word);
    read_aligned(data, &mut offset, &mut word)?;
    grammar.header.record_count_2 = u32::from_le_bytes(word);
    read_aligned(data, &mut offset, &mut word)?;
    grammar.header.record_count_3 = u32::from_le_bytes(word);
    read_aligned(data, &mut offset, &mut word)?;
    grammar.header.short_count = u32::from_le_bytes(word) as u16 as i16;
    read_aligned(data, &mut offset, &mut word)?;
    grammar.header.word_count = u32::from_le_bytes(word) as u16 as i16;
    read_aligned(data, &mut offset, &mut word)?;
    grammar.header.value_24 = u32::from_le_bytes(word);
    let start = offset;
    read_bytes(data, &mut offset, grammar.header.string_count)?;
    grammar.header.value_28 = start;

    let required = grammar.header.string_count as usize;
    let available = grammar.strings.len();
    if available < required {
        return Err(Error::Capacity {
            required,
            available,
        });
    }
    let count = grammar.header.string_count as u16 as i16;
    let mut index = 0i16;
    while index < count {
        let start = offset;
        if read_bytes(data, &mut offset, 1).is_ok() {
            grammar.strings[index as usize] = start;
            if byte_at(window, start) != 0 {
                while byte_at(window, offset) != 0 {
                    offset = offset.wrapping_add(1);
                }
                offset = offset.wrapping_add(1);
            }
        }
        index += 1;
    }

    let size = grammar.header.string_count.wrapping_mul(2);
    offset = offset.wrapping_add(1) & !1;
    if offset.wrapping_add(size) > file_size {
        return Err(Error::Engine(0x89702009));
    }
    grammar.header.sections[0] = offset;
    offset = offset.wrapping_add(size);

    offset = offset.wrapping_add(1) & !1;
    if offset.wrapping_add(size) > file_size {
        return Err(Error::Engine(0x89702009));
    }
    grammar.header.sections[1] = offset;
    offset = offset.wrapping_add(size);

    let start = offset;
    read_bytes(
        data,
        &mut offset,
        grammar.header.record_count.wrapping_mul(0xa2),
    )?;
    grammar.header.sections[2] = start;

    let start = offset;
    read_bytes(
        data,
        &mut offset,
        grammar.header.record_count_2.wrapping_mul(0xa2),
    )?;
    grammar.header.sections[3] = start;

    let start = offset;
    read_bytes(
        data,
        &mut offset,
        grammar.header.record_count_2.wrapping_mul(0xa2),
    )?;
    grammar.header.sections[4] = start;

    let start = offset;
    read_bytes(
        data,
        &mut offset,
        grammar
            .header
            .record_count_2
            .wrapping_mul(grammar.header.record_count_3),
    )?;
    grammar.header.sections[5] = start;

    let start = offset;
    read_bytes(
        data,
        &mut offset,
        (grammar.header.short_count as u16).wrapping_mul(2) as u32,
    )?;
    grammar.header.sections[6] = start;

    let size = (grammar.header.word_count as u16).wrapping_mul(4) as u32;
    let aligned = offset.wrapping_add(3) & !3;
    if aligned.wrapping_add(size) > file_size {
        return Err(Error::Engine(0x89702009));
    }
    grammar.header.sections[7] = aligned;
    offset = aligned.wrapping_add(size);

    let words = grammar.header.word_count as u16 as u32;
    for slot in 0..7 {
        let start = offset;
        read_bytes(data, &mut offset, words)?;
        grammar.header.values[slot] = start;
    }
    let start = offset;
    read_bytes(data, &mut offset, grammar.header.value_24)?;
    grammar.header.values[7] = start;

    let required = grammar.header.layer_count.max(0) as usize;
    let available = grammar.layers.len();
    if available < required {
        return Err(Error::Capacity {
            required,
            available,
        });
    }
    let required = (grammar.header.short_count as i32 + 1) as u16 as usize;
    let available = grammar.string_data.len();
    if available < required {
        return Err(Error::Capacity {
            required,
            available,
        });
    }
    Ok((0, window))
}

fn lw(data: &[u8], offset: u32) -> u32 {
    let low = data.get(offset as usize).copied().unwrap_or(0) as u32;
    let mid0 = data
        .get(offset.wrapping_add(1) as usize)
        .copied()
        .unwrap_or(0) as u32;
    let mid1 = data
        .get(offset.wrapping_add(2) as usize)
        .copied()
        .unwrap_or(0) as u32;
    let high = data
        .get(offset.wrapping_add(3) as usize)
        .copied()
        .unwrap_or(0) as u32;
    low | mid0 << 8 | mid1 << 16 | high << 24
}

fn lh(data: &[u8], offset: u32) -> u16 {
    let low = data.get(offset as usize).copied().unwrap_or(0) as u16;
    let high = data
        .get(offset.wrapping_add(1) as usize)
        .copied()
        .unwrap_or(0) as u16;
    low | high << 8
}

fn lputw(data: &mut [u8], offset: u32, value: u32) {
    for (index, byte) in value.to_le_bytes().iter().enumerate() {
        if let Some(slot) = data.get_mut(offset as usize + index) {
            *slot = *byte;
        }
    }
}

fn lputh(data: &mut [u8], offset: u32, value: u16) {
    if let Some(slot) = data.get_mut(offset as usize) {
        *slot = value as u8;
    }
    if let Some(slot) = data.get_mut(offset.wrapping_add(1) as usize) {
        *slot = (value >> 8) as u8;
    }
}

pub trait DepesLifecycleHost: HandleValidator {

    fn calloc(&mut self, arena: &mut [u8], heap: u32, count: u32, size: u32) -> Option<u32>;

    fn alloc(&mut self, arena: &mut [u8], heap: u32, size: u32) -> Option<u32>;

    fn free(&mut self, heap: u32, ptr: u32);

    fn resolve(&mut self, params: u32, name: u32) -> (u32, u32);

    fn acquire(&mut self, arena: &mut [u8], resources: u32, name: u32) -> (u32, u32);

    fn release_named(&mut self, resources: u32, name: u32);

    fn close_file(&mut self, files: u32, handle: u32);

    fn unmap_file(&mut self, files: u32, handle: u32, base: u32);

    fn registry_int(&mut self, arena: &mut [u8], reg: u32, key: u32) -> (u32, u32);

    fn grammar_status(
        &mut self,
        arena: &mut [u8],
        engine: u32,
        name: u32,
        handle: u32,
        file_size: u32,
        grammar: u32,
    ) -> u32;

    fn open_resource_file(
        &mut self,
        arena: &mut [u8],
        engine: u32,
        handle: u32,
        out_handle: &mut u32,
        out_size: &mut u32,
    ) -> u32;

    fn config_path(
        &mut self,
        arena: &mut [u8],
        engine: u32,
        out: u32,
        mode: u32,
        which: u32,
    ) -> u32;

    fn detach_table(&mut self, table: u32) -> u32;
}

pub fn exit_module(
    arena: &mut [u8],
    obj: u32,
    handle: u32,
    host: &mut impl DepesLifecycleHost,
) -> u32 {
    let status = host.validate(handle, 0xf37e, 0x40);
    if status >= 0x8000_0000 {
        return 0x8970_2008;
    }
    let context = lw(arena, obj);
    let allocator = lw(arena, context.wrapping_add(4));
    let files = lw(arena, context.wrapping_add(12));
    if lw(arena, obj.wrapping_add(40)) != 0 {
        let count = lh(arena, obj.wrapping_add(48)) as i16;
        if count > 0 {
            let mut i = 0i32;
            loop {
                let layers = lw(arena, obj.wrapping_add(40));
                let slot = layers.wrapping_add((i as u32).wrapping_mul(4));
                if lw(arena, slot) != 0 {
                    host.free(allocator, lw(arena, slot));
                    lputw(arena, slot, 0);
                }
                if lw(arena, obj.wrapping_add(16)) != 0 {
                    let first = lw(arena, obj.wrapping_add(32));
                    let table = lw(arena, first.wrapping_add(0x39c));
                    lputw(arena, table.wrapping_add((i as u32).wrapping_mul(4)), 0);
                }
                if lw(arena, obj.wrapping_add(20)) != 0 {
                    let second = lw(arena, obj.wrapping_add(36));
                    let table = lw(arena, second.wrapping_add(0x39c));
                    lputw(arena, table.wrapping_add((i as u32).wrapping_mul(4)), 0);
                }
                i = (i.wrapping_add(1) as i16) as i32;
                if !(count as i32 > i) {
                    break;
                }
            }
        }
        host.free(allocator, lw(arena, obj.wrapping_add(40)));
        lputw(arena, obj.wrapping_add(40), 0);
    }
    if lw(arena, obj.wrapping_add(44)) != 0 {
        host.free(allocator, lw(arena, obj.wrapping_add(44)));
        lputw(arena, obj.wrapping_add(44), 0);
    }
    for handle_off in [16u32, 20u32] {
        if lw(arena, obj.wrapping_add(handle_off)) != 0 {
            let grammar = lw(arena, obj.wrapping_add(handle_off + 16));
            if grammar != 0 {
                if lw(arena, grammar) != 0 {
                    host.unmap_file(
                        files,
                        lw(arena, obj.wrapping_add(handle_off)),
                        lw(arena, grammar),
                    );
                }
                for field in [0x39cu32, 0x3a0, 0x3a4] {
                    if lw(arena, grammar.wrapping_add(field)) != 0 {
                        host.free(allocator, lw(arena, grammar.wrapping_add(field)));
                        lputw(arena, grammar.wrapping_add(field), 0);
                    }
                }
            }
            if lw(arena, obj.wrapping_add(handle_off + 16)) != 0 {
                lputw(arena, lw(arena, obj.wrapping_add(handle_off + 16)), 0);
            }
        }
    }
    0
}

pub fn close_module(
    arena: &mut [u8],
    obj: u32,
    handle: u32,
    lookup_name: u32,
    host: &mut impl DepesLifecycleHost,
) -> u32 {
    let status = host.validate(handle, 0xf37e, 0x40);
    if status >= 0x8000_0000 {
        return 0x8970_2008;
    }
    if obj != 0 {
        let context = lw(arena, obj);
        let allocator = lw(arena, context.wrapping_add(4));
        let files = lw(arena, context.wrapping_add(12));
        let resources = lw(arena, context.wrapping_add(24));
        host.release_named(resources, lookup_name);
        exit_module(arena, obj, handle, host);
        if lw(arena, obj.wrapping_add(32)) != 0 {
            host.free(allocator, lw(arena, obj.wrapping_add(32)));
            lputw(arena, obj.wrapping_add(32), 0);
        }
        if lw(arena, obj.wrapping_add(36)) != 0 {
            host.free(allocator, lw(arena, obj.wrapping_add(36)));
            lputw(arena, obj.wrapping_add(36), 0);
        }
        if lw(arena, obj.wrapping_add(16)) != 0 {
            host.close_file(files, lw(arena, obj.wrapping_add(16)));
            lputw(arena, obj.wrapping_add(16), 0);
        }
        if lw(arena, obj.wrapping_add(20)) != 0 {
            host.close_file(files, lw(arena, obj.wrapping_add(20)));
            lputw(arena, obj.wrapping_add(20), 0);
        }
        host.free(allocator, obj);
        return 0;
    }
    status
}

pub fn init_module(
    arena: &mut [u8],
    obj: u32,
    handle: u32,
    name: u32,
    out_count: &mut u16,
    host: &mut impl DepesLifecycleHost,
) -> u32 {
    let mut status = host.validate(handle, 0xf37e, 0x40);
    if status >= 0x8000_0000 {
        return 0x8970_2008;
    }
    *out_count = 0;
    let engine = lw(arena, obj);
    let allocator = lw(arena, engine.wrapping_add(4));
    if lw(arena, obj.wrapping_add(16)) != 0 {
        if lw(arena, obj.wrapping_add(32)) == 0 {
            status = 0x8970_2007;
        } else {
            let first = lw(arena, obj.wrapping_add(32));
            status = host.grammar_status(
                arena,
                engine,
                name,
                lw(arena, obj.wrapping_add(16)),
                lw(arena, obj.wrapping_add(24)),
                first,
            );
            if status < 0x8000_0000 {
                let layers = lh(arena, first.wrapping_add(12)) as i16;
                if *out_count != 0 && *out_count as i32 != layers as i32 {
                    status = 0x8970_2019;
                } else {
                    *out_count = layers as u16;
                    lputw(arena, first.wrapping_add(0x6c), obj.wrapping_add(50));
                }
            }
        }
        if status >= 0x8000_0000 {
            exit_module(arena, obj, handle, host);
            return status;
        }
    }
    if lw(arena, obj.wrapping_add(20)) != 0 {
        if lw(arena, obj.wrapping_add(36)) == 0 {
            status = 0x8970_2007;
        } else {
            let second = lw(arena, obj.wrapping_add(36));
            status = host.grammar_status(
                arena,
                engine,
                name,
                lw(arena, obj.wrapping_add(20)),
                lw(arena, obj.wrapping_add(28)),
                second,
            );
            if status < 0x8000_0000 {
                let layers = lh(arena, second.wrapping_add(12)) as i16;
                if *out_count != 0 && *out_count as i32 != layers as i32 {
                    status = 0x8970_2019;
                } else {
                    *out_count = layers as u16;
                    lputw(arena, second.wrapping_add(0x6c), obj.wrapping_add(50));
                }
            }
        }
        if status >= 0x8000_0000 {
            exit_module(arena, obj, handle, host);
            return status;
        }
    }
    match host.calloc(arena, allocator, *out_count as u32, 4) {
        Some(layers) => lputw(arena, obj.wrapping_add(40), layers),
        None => {
            status = 0x8970_200a;
            exit_module(arena, obj, handle, host);
            return status;
        }
    }
    lputh(arena, obj.wrapping_add(48), *out_count);
    let mut i = 0i16;
    while i < lh(arena, obj.wrapping_add(48)) as i16 {
        let layers = lw(arena, obj.wrapping_add(40));
        let slot = layers.wrapping_add((i as u32).wrapping_mul(4));
        lputw(arena, slot, 0);
        match host.alloc(arena, allocator, 0x400) {
            Some(row) => lputw(arena, slot, row),
            None => return 0x8970_200a,
        }
        let row = lw(arena, slot);
        if lw(arena, obj.wrapping_add(16)) != 0 {
            let first = lw(arena, obj.wrapping_add(32));
            lputw(
                arena,
                lw(arena, first.wrapping_add(0x39c)).wrapping_add((i as u32).wrapping_mul(4)),
                row,
            );
        }
        if lw(arena, obj.wrapping_add(20)) != 0 {
            let second = lw(arena, obj.wrapping_add(36));
            lputw(
                arena,
                lw(arena, second.wrapping_add(0x39c)).wrapping_add((i as u32).wrapping_mul(4)),
                row,
            );
        }
        i = i.wrapping_add(1);
    }
    match host.calloc(arena, allocator, 0x400, 1) {
        Some(flags) => lputw(arena, obj.wrapping_add(44), flags),
        None => return 0x8970_200a,
    }
    let flags = lw(arena, obj.wrapping_add(44));
    if lw(arena, obj.wrapping_add(16)) != 0 {
        lputw(
            arena,
            lw(arena, obj.wrapping_add(32)).wrapping_add(0x3b8),
            flags,
        );
    }
    if lw(arena, obj.wrapping_add(20)) != 0 {
        lputw(
            arena,
            lw(arena, obj.wrapping_add(36)).wrapping_add(0x3b8),
            flags,
        );
    }
    status
}

pub const CONFIG_SCRATCH: u32 = 0;

pub const LOOKUP_NAME_SCRATCH: u32 = 256;

pub const GRAMMAR_NAME_SCRATCH: u32 = 272;

pub fn open_object(
    arena: &mut [u8],
    params: u32,
    name: u32,
    out: u32,
    scratch: u32,
    host: &mut impl DepesLifecycleHost,
) -> u32 {
    let lookup_name = scratch.wrapping_add(LOOKUP_NAME_SCRATCH);
    let grammar_name = scratch.wrapping_add(GRAMMAR_NAME_SCRATCH);
    lputw(arena, scratch, 0);
    if out == 0 {
        return 0x8970_2007;
    }
    lputw(arena, out, 0);
    lputw(arena, out.wrapping_add(4), 0);
    let (status, context) = host.resolve(params, name);
    if status >= 0x8000_0000 {
        return status;
    }
    let allocator = lw(arena, context.wrapping_add(4));
    let resources = lw(arena, context.wrapping_add(24));
    let (status, info) = host.acquire(arena, resources, lookup_name);
    if status >= 0x8000_0000 {
        return status;
    }
    let object = match host.calloc(arena, allocator, 1, 0x40) {
        Some(object) => object,
        None => {
            host.release_named(resources, lookup_name);
            return 0x8970_200a;
        }
    };
    lputw(arena, out.wrapping_add(4), 0xf37e);
    lputw(arena, out, object);
    lputw(arena, object, context);
    lputw(
        arena,
        object.wrapping_add(4),
        lw(arena, info.wrapping_add(8)),
    );
    lputw(
        arena,
        object.wrapping_add(8),
        lw(arena, info.wrapping_add(12)),
    );
    lputw(
        arena,
        object.wrapping_add(12),
        lw(arena, info.wrapping_add(4)),
    );
    lputw(arena, object.wrapping_add(16), 0);
    lputw(arena, object.wrapping_add(20), 0);
    lputw(arena, object.wrapping_add(24), 0);
    lputw(arena, object.wrapping_add(28), 0);
    lputw(arena, object.wrapping_add(40), 0);
    lputw(arena, object.wrapping_add(32), 0);
    lputw(arena, object.wrapping_add(36), 0);
    lputh(arena, object.wrapping_add(48), 0);
    lputw(arena, object.wrapping_add(44), 0);
    lputh(arena, object.wrapping_add(50), 0x400);
    lputh(arena, object.wrapping_add(52), 0);
    lputw(arena, object.wrapping_add(56), 0);
    lputw(arena, object.wrapping_add(60), 0);
    let mut first_handle = 0u32;
    let mut first_size = 0u32;
    let mut status = host.config_path(arena, context, scratch, 0, 0);
    let mut first_done = false;
    if status < 0x8000_0000 {
        status =
            host.open_resource_file(arena, context, scratch, &mut first_handle, &mut first_size);
        lputw(arena, object.wrapping_add(16), first_handle);
        lputw(arena, object.wrapping_add(24), first_size);
        if status < 0x8000_0000 && first_handle != 0 {
            first_done = true;
        }
    }
    if !first_done {
        status = host.config_path(arena, context, scratch, 1, 0);
        if status >= 0x8000_0000 {
            return cleanup_open(arena, out, scratch, status, host);
        }
        status =
            host.open_resource_file(arena, context, scratch, &mut first_handle, &mut first_size);
        lputw(arena, object.wrapping_add(16), first_handle);
        lputw(arena, object.wrapping_add(24), first_size);
        if status >= 0x8000_0000 {
            return cleanup_open(arena, out, scratch, status, host);
        }
    }
    let mut second_handle = 0u32;
    let mut second_size = 0u32;
    status = host.config_path(arena, context, scratch, 0, 1);
    let mut second_done = false;
    if status < 0x8000_0000 {
        status = host.open_resource_file(
            arena,
            context,
            scratch,
            &mut second_handle,
            &mut second_size,
        );
        lputw(arena, object.wrapping_add(20), second_handle);
        lputw(arena, object.wrapping_add(28), second_size);
        if status < 0x8000_0000 && second_handle != 0 {
            second_done = true;
        }
    }
    if !second_done {
        status = host.config_path(arena, context, scratch, 1, 1);
        if status >= 0x8000_0000 {
            return cleanup_open(arena, out, scratch, status, host);
        }
        status = host.open_resource_file(
            arena,
            context,
            scratch,
            &mut second_handle,
            &mut second_size,
        );
        lputw(arena, object.wrapping_add(20), second_handle);
        lputw(arena, object.wrapping_add(28), second_size);
        if status >= 0x8000_0000 {
            return cleanup_open(arena, out, scratch, status, host);
        }
    }
    if first_handle == 0 {
        if second_handle == 0 {
            return cleanup_open(arena, out, scratch, 0x8970_2003, host);
        }
    } else {
        let first = match host.calloc(arena, allocator, 1, 0x3bc) {
            Some(first) => first,
            None => return cleanup_open(arena, out, scratch, 0x8970_200a, host),
        };
        init_grammar(arena, first, allocator, info);
        lputw(arena, object.wrapping_add(32), first);
        if second_handle == 0 {
            return open_grammars(arena, out, scratch, grammar_name, host);
        }
    }
    let second = match host.calloc(arena, allocator, 1, 0x3bc) {
        Some(second) => second,
        None => return cleanup_open(arena, out, scratch, 0x8970_200a, host),
    };
    init_grammar(arena, second, allocator, info);
    lputw(arena, object.wrapping_add(36), second);
    open_grammars(arena, out, scratch, grammar_name, host)
}

fn init_grammar(arena: &mut [u8], grammar: u32, allocator: u32, info: u32) {
    lputw(arena, grammar.wrapping_add(0x39c), 0);
    lputw(arena, grammar.wrapping_add(0x3b8), 0);
    lputw(arena, grammar.wrapping_add(0x3a0), 0);
    lputw(arena, grammar.wrapping_add(0x3a4), 0);
    lputw(arena, grammar.wrapping_add(0x3a8), allocator);
    lputw(
        arena,
        grammar.wrapping_add(0x3ac),
        lw(arena, info.wrapping_add(8)),
    );
    lputw(
        arena,
        grammar.wrapping_add(0x3b0),
        lw(arena, info.wrapping_add(12)),
    );
    lputw(
        arena,
        grammar.wrapping_add(0x3b4),
        lw(arena, info.wrapping_add(4)),
    );
    lputw(arena, grammar, 0);
}

fn cleanup_open(
    arena: &mut [u8],
    out: u32,
    scratch: u32,
    status: u32,
    host: &mut impl DepesLifecycleHost,
) -> u32 {
    close_module(
        arena,
        lw(arena, out),
        lw(arena, out.wrapping_add(4)),
        scratch.wrapping_add(LOOKUP_NAME_SCRATCH),
        host,
    );
    lputw(arena, out, 0);
    lputw(arena, out.wrapping_add(4), 0);
    status
}

fn open_grammars(
    arena: &mut [u8],
    out: u32,
    scratch: u32,
    grammar_name: u32,
    host: &mut impl DepesLifecycleHost,
) -> u32 {
    let mut module_count = 0u16;
    let status = init_module(
        arena,
        lw(arena, out),
        lw(arena, out.wrapping_add(4)),
        grammar_name,
        &mut module_count,
        host,
    );
    if status < 0x8000_0000 {
        return status;
    }
    close_module(
        arena,
        lw(arena, out),
        lw(arena, out.wrapping_add(4)),
        scratch.wrapping_add(LOOKUP_NAME_SCRATCH),
        host,
    );
    lputw(arena, out, 0);
    lputw(arena, out.wrapping_add(4), 0);
    status
}

pub fn reopen_object(
    arena: &mut [u8],
    object: u32,
    handle: u32,
    scratch: u32,
    host: &mut impl DepesLifecycleHost,
) -> u32 {
    let grammar_name = scratch.wrapping_add(GRAMMAR_NAME_SCRATCH);
    lputw(arena, scratch, 0);
    let mut status = host.validate(handle, 0xf37e, 0x40);
    if status >= 0x8000_0000 {
        return 0x8970_2008;
    }
    if object == 0 {
        return status;
    }
    let engine = lw(arena, object);
    let allocator = lw(arena, engine.wrapping_add(4));
    let files = lw(arena, engine.wrapping_add(12));
    exit_module(arena, object, handle, host);
    if lw(arena, object.wrapping_add(32)) != 0 {
        host.free(allocator, lw(arena, object.wrapping_add(32)));
        lputw(arena, object.wrapping_add(32), 0);
    }
    if lw(arena, object.wrapping_add(36)) != 0 {
        host.free(allocator, lw(arena, object.wrapping_add(36)));
        lputw(arena, object.wrapping_add(36), 0);
    }
    if lw(arena, object.wrapping_add(16)) != 0 {
        host.close_file(files, lw(arena, object.wrapping_add(16)));
        lputw(arena, object.wrapping_add(16), 0);
    }
    if lw(arena, object.wrapping_add(20)) != 0 {
        host.close_file(files, lw(arena, object.wrapping_add(20)));
        lputw(arena, object.wrapping_add(20), 0);
    }
    lputw(arena, object.wrapping_add(32), 0);
    lputw(arena, object.wrapping_add(36), 0);
    lputh(arena, object.wrapping_add(48), 0);
    lputw(arena, object.wrapping_add(44), 0);
    lputh(arena, object.wrapping_add(50), 0x400);
    lputh(arena, object.wrapping_add(52), 0);
    lputw(arena, object.wrapping_add(16), 0);
    lputw(arena, object.wrapping_add(56), 0);
    lputw(arena, object.wrapping_add(60), 0);
    let mut first_handle = 0u32;
    let mut first_size = 0u32;
    status = host.config_path(arena, engine, scratch, 0, 0);
    let mut first_fallback = status >= 0x8000_0000;
    if !first_fallback {
        status =
            host.open_resource_file(arena, engine, scratch, &mut first_handle, &mut first_size);
        lputw(arena, object.wrapping_add(16), first_handle);
        lputw(arena, object.wrapping_add(24), first_size);
        first_fallback = status >= 0x8000_0000 || first_handle == 0;
    }
    if first_fallback {
        status = host.config_path(arena, engine, scratch, 1, 0);
        if status >= 0x8000_0000 {
            return status;
        }
        status =
            host.open_resource_file(arena, engine, scratch, &mut first_handle, &mut first_size);
        lputw(arena, object.wrapping_add(16), first_handle);
        lputw(arena, object.wrapping_add(24), first_size);
        if status >= 0x8000_0000 {
            return status;
        }
    }
    let mut second_handle = 0u32;
    let mut second_size = 0u32;
    status = host.config_path(arena, engine, scratch, 0, 1);
    let mut second_fallback = status >= 0x8000_0000;
    if !second_fallback {
        status =
            host.open_resource_file(arena, engine, scratch, &mut second_handle, &mut second_size);
        lputw(arena, object.wrapping_add(20), second_handle);
        lputw(arena, object.wrapping_add(28), second_size);
        second_fallback = status >= 0x8000_0000 || second_handle == 0;
    }
    if second_fallback {
        status = host.config_path(arena, engine, scratch, 1, 1);
        if status >= 0x8000_0000 {
            return status;
        }
        status =
            host.open_resource_file(arena, engine, scratch, &mut second_handle, &mut second_size);
        lputw(arena, object.wrapping_add(20), second_handle);
        lputw(arena, object.wrapping_add(28), second_size);
        if status >= 0x8000_0000 {
            return status;
        }
    }
    if first_handle == 0 {
        if second_handle == 0 {
            return 0x8970_2003;
        }
    } else {
        match host.calloc(arena, allocator, 1, 0x3bc) {
            Some(first) => {
                init_reopened(arena, object, first, allocator);
                lputw(arena, object.wrapping_add(32), first);
            }
            None => return 0x8970_200a,
        }
    }
    if second_handle != 0 {
        match host.calloc(arena, allocator, 1, 0x3bc) {
            Some(second) => {
                init_reopened(arena, object, second, allocator);
                lputw(arena, object.wrapping_add(36), second);
            }
            None => return 0x8970_200a,
        }
    }
    let mut module_count = 0u16;
    status = init_module(arena, object, handle, grammar_name, &mut module_count, host);
    status
}

fn init_reopened(arena: &mut [u8], object: u32, grammar: u32, allocator: u32) {
    lputw(arena, grammar.wrapping_add(0x39c), 0);
    lputw(arena, grammar.wrapping_add(0x3b8), 0);
    lputw(arena, grammar.wrapping_add(0x3a0), 0);
    lputw(arena, grammar.wrapping_add(0x3a4), 0);
    lputw(arena, grammar.wrapping_add(0x3a8), allocator);
    lputw(
        arena,
        grammar.wrapping_add(0x3ac),
        lw(arena, object.wrapping_add(4)),
    );
    lputw(
        arena,
        grammar.wrapping_add(0x3b0),
        lw(arena, object.wrapping_add(8)),
    );
    lputw(
        arena,
        grammar.wrapping_add(0x3b4),
        lw(arena, object.wrapping_add(12)),
    );
}

pub const NAME_SYNTHSTREAM: u32 = 0;

pub const NAME_FE_DEPES: u32 = 12;

pub const NAME_DCTEG: u32 = 21;

pub const NAME_LOOKUP_LEN: u32 = 27;

pub const NAME_DICT_LEN: u32 = 44;

pub const NAME_NUMBER_OUTPUT: u32 = 62;

pub fn component_open(
    arena: &mut [u8],
    hh_a: u32,
    hh_b: u32,
    out: u32,
    names: u32,
    host: &mut impl DepesLifecycleHost,
) -> u32 {
    if out == 0 {
        return 0x8a80_2007;
    }
    lputw(arena, out, 0);
    lputw(arena, out.wrapping_add(4), 0);
    let (status, ctx) = host.resolve(hh_a, hh_b);
    if status >= 0x8000_0000 {
        return status;
    }
    let heap = lw(arena, ctx.wrapping_add(4));
    let reg = lw(arena, ctx.wrapping_add(20));
    let res = lw(arena, ctx.wrapping_add(24));
    let synth = names.wrapping_add(NAME_SYNTHSTREAM);
    let depes_name = names.wrapping_add(NAME_FE_DEPES);
    let dcteg_name = names.wrapping_add(NAME_DCTEG);
    let (status, stream) = host.acquire(arena, res, synth);
    if status >= 0x8000_0000 {
        return status;
    }
    let (status, depes) = host.acquire(arena, res, depes_name);
    if status >= 0x8000_0000 {
        host.release_named(res, synth);
        return status;
    }

    let (_dcteg_status, dcteg) = host.acquire(arena, res, dcteg_name);
    let p = match host.calloc(arena, heap, 1, 184) {
        Some(p) => p,
        None => {
            host.release_named(res, synth);
            host.release_named(res, depes_name);
            if dcteg != 0 {
                host.release_named(res, dcteg_name);
            }
            return 0x8a80_200a;
        }
    };
    lputw(arena, p.wrapping_add(108), 0);
    match host.alloc(arena, heap, 8) {
        Some(block) => lputw(arena, p.wrapping_add(108), block),
        None => {
            host.release_named(res, synth);
            host.release_named(res, depes_name);
            if dcteg != 0 {
                host.release_named(res, dcteg_name);
            }
            return 0x8a80_200a;
        }
    }
    lputw(arena, p, hh_a);
    lputw(arena, p.wrapping_add(4), hh_b);
    lputw(arena, p.wrapping_add(8), ctx);
    lputw(arena, p.wrapping_add(24), lw(arena, stream.wrapping_add(4)));
    for field in [60u32, 64, 68, 80, 84, 72, 76] {
        lputw(arena, p.wrapping_add(field), 0);
    }
    lputw(arena, p.wrapping_add(20), lw(arena, depes.wrapping_add(4)));
    lputw(arena, p.wrapping_add(12), lw(arena, depes.wrapping_add(8)));
    lputw(arena, p.wrapping_add(16), lw(arena, depes.wrapping_add(12)));
    if dcteg != 0 && lw(arena, dcteg.wrapping_add(4)) != 0 {
        lputw(arena, p.wrapping_add(88), lw(arena, dcteg.wrapping_add(4)));
        lputw(arena, p.wrapping_add(92), lw(arena, dcteg.wrapping_add(8)));
        lputw(arena, p.wrapping_add(96), lw(arena, dcteg.wrapping_add(12)));
    }
    lputh(arena, p.wrapping_add(102), 512);
    let (status, value) = host.registry_int(arena, reg, names.wrapping_add(NAME_LOOKUP_LEN));
    if status < 0x8000_0000 {
        lputh(arena, p.wrapping_add(102), value as u16);
    }
    lputh(arena, p.wrapping_add(100), 512);
    let (status, value) = host.registry_int(arena, reg, names.wrapping_add(NAME_DICT_LEN));
    if status < 0x8000_0000 {
        lputh(arena, p.wrapping_add(100), value as u16);
    }
    lputh(arena, p.wrapping_add(104), 1);
    let (status, value) = host.registry_int(arena, reg, names.wrapping_add(NAME_NUMBER_OUTPUT));
    if status < 0x8000_0000 {
        lputh(arena, p.wrapping_add(104), value as u16);
    }

    for field in [112u32, 116, 120, 124, 148, 152, 156, 160] {
        lputw(arena, p.wrapping_add(field), 0);
    }
    if let Some(slot) = arena.get_mut(p.wrapping_add(164) as usize) {
        *slot = 0;
    }

    for i in 0..16u32 {
        let byte = arena
            .get(p.wrapping_add(112).wrapping_add(i) as usize)
            .copied()
            .unwrap_or(0);
        if let Some(slot) = arena.get_mut(p.wrapping_add(128).wrapping_add(i) as usize) {
            *slot = byte;
        }
    }
    lputw(arena, p.wrapping_add(144), heap);

    let rc = map_table_status(0);
    lputw(arena, out.wrapping_add(4), 0xf38a);
    lputw(arena, out, p);
    rc
}

fn map_table_status(rc: u32) -> u32 {
    if rc != 0 {
        rc.wrapping_add(0x1ea0) | 0x8a80_2000
    } else {
        rc
    }
}

pub fn component_refresh(
    arena: &mut [u8],
    obj: u32,
    handle: u32,
    names: u32,
    host: &mut impl DepesLifecycleHost,
) -> u32 {
    let validated = host.validate(handle, 0xf38a, 0xb8);
    if validated >= 0x8000_0000 {
        return 0x8a80_2008;
    }
    if obj == 0 {
        return validated;
    }
    let reg = lw(arena, lw(arena, obj.wrapping_add(8)).wrapping_add(20));
    lputh(arena, obj.wrapping_add(102), 512);
    let (status, value) = host.registry_int(arena, reg, names.wrapping_add(NAME_LOOKUP_LEN));
    if status < 0x8000_0000 {
        lputh(arena, obj.wrapping_add(102), value as u16);
    }
    lputh(arena, obj.wrapping_add(100), 512);
    let (status, value) = host.registry_int(arena, reg, names.wrapping_add(NAME_DICT_LEN));
    if status < 0x8000_0000 {
        lputh(arena, obj.wrapping_add(100), value as u16);
    }
    lputh(arena, obj.wrapping_add(104), 1);
    let (status, value) = host.registry_int(arena, reg, names.wrapping_add(NAME_NUMBER_OUTPUT));
    if status < 0x8000_0000 {
        lputh(arena, obj.wrapping_add(104), value as u16);
    }
    validated
}

pub fn component_close(
    arena: &mut [u8],
    obj: u32,
    handle: u32,
    names: u32,
    host: &mut impl DepesLifecycleHost,
) -> u32 {
    let status = host.validate(handle, 0xf38a, 0xb8);
    if status >= 0x8000_0000 {
        return 0x8a80_2008;
    }
    if obj == 0 {
        return status;
    }
    let ctx = lw(arena, obj.wrapping_add(8));
    let heap = lw(arena, ctx.wrapping_add(4));
    let files = lw(arena, ctx.wrapping_add(12));
    let res = lw(arena, ctx.wrapping_add(24));
    if lw(arena, obj.wrapping_add(80)) != 0 {
        host.detach_table(lw(arena, obj.wrapping_add(80)));
        host.free(heap, lw(arena, obj.wrapping_add(80)));
        lputw(arena, obj.wrapping_add(80), 0);
        lputw(arena, obj.wrapping_add(84), 0);
    }
    if lw(arena, obj.wrapping_add(72)) != 0 {
        host.detach_table(lw(arena, obj.wrapping_add(72)));
        host.free(heap, lw(arena, obj.wrapping_add(72)));
        lputw(arena, obj.wrapping_add(72), 0);
        lputw(arena, obj.wrapping_add(76), 0);
    }
    if lw(arena, obj.wrapping_add(60)) != 0 {
        host.unmap_file(
            files,
            lw(arena, obj.wrapping_add(60)),
            lw(arena, obj.wrapping_add(64)),
        );
        lputw(arena, obj.wrapping_add(64), 0);
        host.close_file(files, lw(arena, obj.wrapping_add(60)));
        lputw(arena, obj.wrapping_add(60), 0);
    }
    host.release_named(res, names.wrapping_add(NAME_SYNTHSTREAM));
    host.release_named(res, names.wrapping_add(NAME_FE_DEPES));
    if lw(arena, obj.wrapping_add(88)) != 0 {
        host.release_named(res, names.wrapping_add(NAME_DCTEG));
    }
    if lw(arena, obj.wrapping_add(108)) != 0 {
        host.free(heap, lw(arena, obj.wrapping_add(108)));
    }
    host.free(heap, obj);
    0
}
