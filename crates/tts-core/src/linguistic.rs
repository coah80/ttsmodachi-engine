#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub id: u16,
    pub kind: u8,
    pub parent: Option<usize>,
    pub next: Option<usize>,
    pub previous: Option<usize>,
    pub children: [Option<usize>; 2],
}

pub struct Database<'a> {
    pub records: &'a [Option<Record>],
    pub parents: &'a [u8],
    pub allocated: bool,
    pub valid: bool,
}

impl Database<'_> {
    fn record(&self, index: u32) -> Result<&Record, u32> {
        if !self.valid {
            return Err(0x89202008);
        }
        self.records
            .get(index as usize)
            .and_then(Option::as_ref)
            .ok_or(0x89203e11)
    }

    fn link(&self, index: u32, which: usize, output: &mut u16) -> u32 {
        let record = match self.record(index) {
            Ok(record) => record,
            Err(code) => return code,
        };
        let link = [record.parent, record.next, record.previous][which];
        *output = link
            .map(|index| self.records[index].as_ref().unwrap().id)
            .unwrap_or(0);
        0
    }

    pub fn parent(&self, index: u32, output: &mut u16) -> u32 {
        self.link(index, 0, output)
    }

    pub fn next(&self, index: u32, output: &mut u16) -> u32 {
        self.link(index, 1, output)
    }

    pub fn previous(&self, index: u32, output: &mut u16) -> u32 {
        self.link(index, 2, output)
    }

    pub fn next_in_sentence(&self, index: u32, output: &mut u16) -> u32 {
        self.in_sentence(index, 1, output)
    }

    pub fn previous_in_sentence(&self, index: u32, output: &mut u16) -> u32 {
        self.in_sentence(index, 2, output)
    }

    fn in_sentence(&self, index: u32, which: usize, output: &mut u16) -> u32 {
        let mut value = 0;
        let status = self.link(index, which, &mut value);
        if status >= 0x80000000 {
            return status;
        }
        if value == 0 && !matches!(self.records[index as usize].as_ref().unwrap().kind, 2 | 3) {
            return 0x89203e09;
        }
        *output = value;
        0
    }

    fn child(&self, kind: u32, index: u32) -> Result<u16, u32> {
        let record = self.record(index)?;
        if self.parents.get(kind as usize).copied() != Some(record.kind) {
            return Err(0x89203e07);
        }
        for child in record.children {
            let Some(child) = child else { break };
            let child = self.records[child].as_ref().unwrap();
            if child.kind as u32 == kind && child.id != 0 {
                return Ok(child.id);
            }
        }
        Ok(0)
    }

    pub fn find_child(&self, kind: u32, index: u32, output: &mut u16) -> u32 {
        if !self.valid {
            return 0x89202008;
        }
        if kind == 1 {
            *output = u16::from(self.allocated);
            return 0;
        }
        match self.child(kind, index) {
            Ok(0) => 0x89203e08,
            Ok(id) => {
                *output = id;
                0
            }
            Err(code) => code,
        }
    }

    pub fn enquire_child(&self, kind: u32, index: u32, output: &mut u16) -> u32 {
        if !self.valid {
            return 0x89202008;
        }
        if kind == 1 {
            *output = 1;
            return 0;
        }
        match self.child(kind, index) {
            Ok(id) => {
                *output = id;
                0
            }
            Err(code) => code,
        }
    }

    pub fn find_record(&self, kind: u32, ordinal: u32, output: &mut u16) -> u32 {
        let mut id = 0;
        let mut status = self.find_child(1, 0, &mut id);
        if status >= 0x80000000 {
            return status;
        }
        if kind == 1 {
            *output = id;
            return status;
        }
        if kind != 2 {
            return 0x89203e09;
        }
        status = self.find_child(2, id as u32, &mut id);
        if status >= 0x80000000 {
            return status;
        }
        if id == 0 {
            return 0x89203e0a;
        }
        let mut index = 0;
        while index < ordinal.wrapping_sub(1) && id != 0 {
            status = self.next(id as u32, &mut id);
            if status >= 0x80000000 {
                return status;
            }
            index += 1;
        }
        *output = id;
        status
    }
}

pub fn dirty(valid: bool, value: u8, output: &mut u8) -> u32 {
    if valid {
        *output = value;
    }
    0
}

pub fn clear_dirty(valid: bool, value: &mut u8) -> u32 {
    if valid {
        *value = 0;
    }
    0
}

pub struct DumpTable<'a> {
    pub count: u16,
    pub bytes: &'a [u8],
}

pub struct DumpTables<'a> {
    pub sent: DumpTable<'a>,
    pub tok: DumpTable<'a>,
    pub word: DumpTable<'a>,
}

fn walk_dump_table(
    fp: u32,
    buf: u32,
    level: u8,
    id: u16,
    table: &DumpTable<'_>,
    emit: &mut impl FnMut(u32, u32, u8, u8, u32, u16, u16),
) {
    if table.count == 0 {
        return;
    }
    let mut index = 0u8;
    loop {
        let base = index as usize * 20;
        let tag = u16::from_le_bytes(table.bytes[base..base + 2].try_into().unwrap());
        let arg = u32::from_le_bytes(table.bytes[base + 4..base + 8].try_into().unwrap());
        emit(fp, buf, level, index, arg, tag, id);
        index = index.wrapping_add(1);
        if index as u16 >= table.count {
            break;
        }
    }
}

pub fn dump_sentence_walk(
    fp: u32,
    buf: u32,
    tables: &DumpTables<'_>,
    first: &mut impl FnMut(u32, u32, u32, u32) -> (u32, u16),
    group: &mut impl FnMut(u32, u32, u16, u16) -> (u32, u16),
    next: &mut impl FnMut(u32, u32, u16) -> u16,
    emit: &mut impl FnMut(u32, u32, u8, u8, u32, u16, u16),
) -> u32 {
    let (rc, id) = first(fp, buf, 1, 0);
    if rc != 0 {
        return 0;
    }
    if id == 0 {
        return 0;
    }
    walk_dump_table(fp, buf, 0, id, &tables.sent, emit);
    let (rc, mut index) = group(fp, buf, 3, id);
    if rc == 0 && index != 0 {
        loop {
            walk_dump_table(fp, buf, 1, index, &tables.tok, emit);
            index = next(fp, buf, index);
            if index == 0 {
                break;
            }
        }
    }
    let (rc, mut index) = group(fp, buf, 2, id);
    if rc == 0 && index != 0 {
        loop {
            walk_dump_table(fp, buf, 1, index, &tables.word, emit);
            index = next(fp, buf, index);
            if index == 0 {
                break;
            }
        }
    }
    0
}

#[allow(clippy::too_many_arguments)]
pub fn dump_sentence(
    owner: Option<u32>,
    arg: u32,
    requested: u32,
    _name: u32,
    flag: u32,
    level: u32,
    dirty_byte: &mut u8,
    template: &[u8; 66],
    tables: &DumpTables<'_>,
    first: &mut impl FnMut(u32, u32, u32, u32) -> (u32, u16),
    group: &mut impl FnMut(u32, u32, u16, u16) -> (u32, u16),
    next: &mut impl FnMut(u32, u32, u16) -> u16,
    emit: &mut impl FnMut(u32, u32, u8, u8, u32, u16, u16),
) -> u32 {
    let mut _buffer = *template;
    if requested != 0 {
        _buffer[64..66].copy_from_slice(&(requested as u16).to_le_bytes());
    }
    if owner.is_none() || arg != 0xb2c2 {
        return 0x89202008;
    }
    if level < 100 {
        return 0;
    }
    if flag == 1 {
        let mut enabled = 0u8;
        if dirty(true, *dirty_byte, &mut enabled) != 0 || enabled != 1 {
            return 0;
        }
        clear_dirty(true, dirty_byte);
    }
    dump_sentence_walk(owner.unwrap(), arg, tables, first, group, next, emit)
}

pub fn finish() -> u32 {
    0
}

pub fn unsupported() -> u32 {
    0x89202001
}

pub fn record_property(table: &[[u32; 2]], index: usize, output: &mut u32) -> u32 {
    *output = table[index][1];
    0
}

pub fn field_property(table: &[[u32; 2]], index: usize, output: &mut u32) -> u32 {
    *output = table[index][1];
    0
}

#[derive(Clone, Copy)]
pub struct Field<'a> {
    pub data: Option<&'a [u8]>,
    pub state: u8,
    pub fixed_count: u16,
    pub count: Option<u16>,
    pub element_size: u16,
}

pub struct FieldDatabase<'a> {
    pub records: &'a [Option<Record>],
    pub fields: &'a [&'a [Field<'a>]],
    pub valid: bool,
}

impl<'a> FieldDatabase<'a> {
    fn field(&self, record: u32, field: u32) -> Result<&Field<'a>, u32> {
        if !self.valid {
            return Err(0x89202008);
        }
        let record_index = record as usize;
        let record = self
            .records
            .get(record_index)
            .and_then(Option::as_ref)
            .ok_or(0x89203e11u32)?;
        if record.kind == 0 {
            return Err(0x89203e0d);
        }
        self.fields[record_index]
            .get(field as usize)
            .ok_or(0x89203e0d)
    }

    pub fn field_state(&self, record: u32, field: u32, output: &mut u16) -> u32 {
        match self.field(record, field) {
            Ok(field) => {
                *output = field.state as u16;
                0
            }
            Err(code) => code,
        }
    }

    pub fn field_data(
        &self,
        record: u32,
        field: u32,
        output: &mut Option<&'a [u8]>,
        count: &mut u16,
    ) -> u32 {
        let field = match self.field(record, field) {
            Ok(field) => field,
            Err(code) => return code,
        };
        if field.state != 1 {
            return 0x89203e0e;
        }
        let Some(data) = field.data else {
            return 0x89203e11;
        };
        *output = Some(data);
        *count = field.count.unwrap_or(field.fixed_count);
        0
    }

    pub fn read_field(
        &self,
        record: u32,
        field: u32,
        capacity: u32,
        output: &mut [u8],
        written: &mut u16,
    ) -> Result<u32, crate::frontend_types::Error> {
        *written = 0;
        let field = match self.field(record, field) {
            Ok(field) => field,
            Err(code) => return Ok(code),
        };
        if field.state != 1 {
            return Ok(0x89203e0e);
        }
        let Some(data) = field.data else {
            return Ok(0x89203e11);
        };
        let count = (field.count.unwrap_or(field.fixed_count) as u32).min(capacity);
        let bytes = (field.element_size as u32).wrapping_mul(count) as usize;
        if bytes > output.len() {
            return Err(crate::frontend_types::Error::Capacity {
                required: bytes,
                available: output.len(),
            });
        }
        let source = data
            .get(..bytes)
            .ok_or(crate::frontend_types::Error::InvalidResource)?;
        output[..bytes].copy_from_slice(source);
        *written = count as u16;
        Ok(0)
    }
}

pub struct DeleteSlot {
    pub present: bool,
    pub record: Record,
}

pub fn delete_record(slots: &mut [DeleteSlot], index: u32, root: u32) -> u32 {
    if slots.get(index as usize).filter(|slot| slot.present).is_none() {
        return 0x89203e11;
    }

    let mut i = 0;
    while i < 2 {
        let Some(child) = slots[index as usize].record.children[i] else {
            break;
        };
        let status = delete_record(slots, slots[child].record.id as u32, root);
        if status != 0 {
            return status;
        }
        i += 1;
    }
    if index != root {
        if let Some(next) = slots[index as usize].record.next {
            let status = delete_record(slots, slots[next].record.id as u32, root);
            if status != 0 {
                return status;
            }
        }
        slots[index as usize].present = false;
        return 0;
    }
    let rec = slots[index as usize].record;
    if let Some(previous) = rec.previous {
        match rec.next {
            None => slots[previous].record.next = None,
            Some(next) => {
                slots[next].record.previous = Some(previous);
                slots[previous].record.next = rec.next;
            }
        }
    } else {
        let parent = rec.parent.expect("root without previous must have a parent");
        let children = slots[parent].record.children;
        let mut slot = 0;
        let mut found = false;
        let mut i = 0;
        while i < 2 && !found && children[i].is_some() {
            if children[i] == Some(index as usize) {
                slot = i;
                found = true;
            }
            i += 1;
        }
        if !found {
            return 0x89203e0f;
        }
        match rec.next {
            Some(next) => {
                slots[parent].record.children[slot] = rec.next;
                slots[next].record.previous = None;
            }
            None => {
                slots[parent].record.children[slot] = None;
                let mut i = slot + 1;
                while i < 2 && slots[parent].record.children[i].is_some() {
                    slots[parent].record.children[i - 1] = slots[parent].record.children[i];
                    slots[parent].record.children[i] = None;
                    i += 1;
                }
            }
        }
    }
    slots[index as usize].present = false;
    0
}

pub fn delete_tree(
    valid: bool,
    dirty: &mut u8,
    slots: &mut [DeleteSlot],
    index: u32,
) -> u32 {
    if !valid {
        return 0x89202008;
    }
    *dirty = 1;
    delete_record(slots, index, index)
}
