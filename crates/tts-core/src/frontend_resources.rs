use crate::frontend_types::Error;

pub struct MemoryStream<'a> {
    pub data: Option<&'a [u8]>,
    pub length: u32,
    pub position: u32,
}

impl MemoryStream<'_> {
    /// fn_281b30 / EU fn_281cec / KR fn_281d98.
    pub fn read(&mut self, size: u32, count: u32, output: &mut [u8]) -> Result<u32, Error> {
        if size == 0 {
            return Err(Error::InvalidResource);
        }
        let bytes = size
            .wrapping_mul(count)
            .min(self.length.wrapping_sub(self.position));
        if bytes as usize > output.len() {
            return Err(Error::Capacity {
                required: bytes as usize,
                available: output.len(),
            });
        }
        if bytes != 0 {
            let data = self.data.ok_or(Error::InvalidResource)?;
            let start = self.position as usize;
            let source = data
                .get(start..start + bytes as usize)
                .ok_or(Error::InvalidResource)?;
            output[..bytes as usize].copy_from_slice(source);
            self.position = self.position.wrapping_add(bytes);
        }
        Ok(bytes / size)
    }

    /// fn_281b8c / EU fn_281d48 / KR fn_281df4.
    pub fn seek(&mut self, offset: i32, origin: u32) -> u32 {
        let distance = offset.unsigned_abs();
        let position = match origin {
            0 if offset >= 0 => distance,
            1 if offset >= 0 => self.position.wrapping_add(distance),
            1 if self.position >= distance => self.position - distance,
            2 if offset < 0 && self.length >= distance => self.length - distance,
            _ => return 1,
        };
        if position >= self.length {
            return 1;
        }
        self.position = position;
        0
    }
}

/// fn_281564 / EU fn_281720 / KR fn_2817cc.
pub fn read_word(
    data: &[u8],
    end: u32,
    cursor: &mut u32,
    output: Option<&mut [u8; 4]>,
) -> Result<u32, Error> {
    let position = cursor.wrapping_add(3) & !3;
    *cursor = position;
    if position.wrapping_add(4) > end {
        return Ok(0x88e02009);
    }
    if let Some(output) = output {
        let source = data
            .get(position as usize..position as usize + 4)
            .ok_or(Error::InvalidResource)?;
        output.copy_from_slice(source);
    }
    *cursor = cursor.wrapping_add(4);
    Ok(0)
}

fn terminated(text: &[u8]) -> &[u8] {
    &text[..text
        .iter()
        .position(|&byte| byte == 0)
        .expect("unterminated resource string")]
}

/// fn_28165c / EU fn_281818 / KR fn_2818c4.
pub fn contains_name(list: Option<&[u8]>, name: Option<&[u8]>) -> u32 {
    let (Some(list), Some(name)) = (list, name) else {
        return 0;
    };
    let (list, name) = (terminated(list), terminated(name));
    if list.is_empty() || name.is_empty() {
        return 0;
    }
    let mut cursor = 0;
    while cursor + name.len() <= list.len() {
        let Some(relative) = list[cursor..]
            .windows(name.len())
            .position(|bytes| bytes == name)
        else {
            break;
        };
        let found = cursor + relative;
        let end = found + name.len();
        if (found == 0 || list[found - 1] == b'|') && (end == list.len() || list[end] == b'|') {
            return 1;
        }
        cursor = end;
    }
    0
}

/// fn_2816e4 / EU fn_2818a0 / KR fn_28194c.
pub fn contains_wide_name(list: Option<&[u8]>, name: Option<&[u16]>) -> u32 {
    let Some(name) = name else { return 0 };
    let length = name
        .iter()
        .position(|&unit| unit == 0)
        .expect("unterminated resource string");
    if length == 0 || length >= 64 {
        return 0;
    }
    let mut text = [0; 64];
    for (out, &unit) in text.iter_mut().zip(&name[..length]) {
        *out = if unit & 0xff00 != 0 { b'?' } else { unit as u8 };
    }
    contains_name(list, Some(&text[..=length]))
}

/// fn_2815c0 / EU fn_28177c / KR fn_281828.
pub fn copy_name(
    output: Option<&mut [u8]>,
    source: Option<&[u8]>,
    capacity: u32,
) -> Result<u32, Error> {
    let (Some(output), Some(source)) = (output, source) else {
        return Ok(0x88e02007);
    };
    if capacity == 0 {
        return Err(Error::Capacity {
            required: 1,
            available: 0,
        });
    }
    let source = terminated(source);
    let count = source.len().min(capacity as usize - 1);
    if count + 1 > output.len() {
        return Err(Error::Capacity {
            required: count + 1,
            available: output.len(),
        });
    }
    output[..count].copy_from_slice(&source[..count]);
    output[count] = 0;
    Ok(0)
}

/// fn_230308 / EU fn_2304c4 / KR fn_230570.
pub fn retain_audio(references: Option<&mut u16>) -> u32 {
    match references {
        Some(references) => {
            *references = references.wrapping_add(1);
            0
        }
        None => 0x87402007,
    }
}

/// fn_281498 / EU fn_281654 / KR fn_281700.
pub fn domain_flag(value: Option<u8>) -> u32 {
    value.unwrap_or(0) as u32
}

/// fn_2814a4 / EU fn_281660 / KR fn_28170c.
pub fn set_domain_flag(output: Option<&mut u8>, value: u32) -> u32 {
    match output {
        Some(output) => {
            *output = value as u8;
            0
        }
        None => 0x88e02007,
    }
}

fn interface(selector: u32, table: &[u8], error: u32) -> Result<&[u8], u32> {
    if selector <= 1 {
        Ok(table)
    } else {
        Err(error)
    }
}

/// fn_15174c / EU fn_151810 / KR fn_151730.
pub fn stream_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    interface(selector, table, 0x88f02001)
}

/// fn_15db5c / EU fn_15dc20 / KR fn_15db40.
pub fn linguistic_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    interface(selector, table, 0x89202001)
}

/// fn_2113b0 / EU fn_21156c / KR fn_211618.
pub fn initializer_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    interface(selector, table, 0x89a02001)
}

/// fn_1f446c / EU fn_1f4628 / KR fn_1f46d4.
pub fn prompt_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    interface(selector, table, 0x8ba02001)
}

/// fn_1f4d58 / EU fn_1f4f14 / KR fn_1f4fc0.
pub fn prompt_processor_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    interface(selector, table, 0x8c802001)
}

/// fn_22e1f8 / EU fn_22e3b4 / KR fn_22e460.
pub fn audio_inserter_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    interface(selector, table, 0x8b902001)
}

/// fn_2304f0 / EU fn_2306ac / KR fn_230758.
pub fn audio_resource_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    interface(selector, table, 0x87402001)
}

/// fn_157a54 / EU fn_157b18 / KR fn_157a38.
pub fn callback_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    if matches!(selector, 0 | 3) {
        Ok(table)
    } else {
        Err(0x81202001)
    }
}

/// fn_200374 / EU fn_200530 / KR fn_2005dc. The ARM error is 0x88e02001.
pub fn domain_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    interface(selector, table, 0x88e02001)
}

/// fn_27dc3c / EU fn_27ddf8 / KR fn_27dea4.
pub fn prompt_word_boundary(text: &[u8]) -> u32 {
    let end = text
        .iter()
        .position(|&byte| byte == 0)
        .expect("unterminated prompt");
    let end = text[..end]
        .iter()
        .position(|&byte| byte == 0x11)
        .unwrap_or(end) as u16 as usize;
    text[..end]
        .iter()
        .position(|&byte| byte == b' ' || byte == b'_')
        .map_or(0xffff, |index| index as u32)
}

/// fn_27de8c's packed prompt record, as laid out in the APDB entry table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PackedPrompt {
    pub data_offset: u32,
    pub flags: [u8; 4],
    pub text_offset: u16,
    pub auxiliary_offset: u16,
    pub entry_index: u32,
    pub count: u16,
    pub suffix_offset: u16,
}

/// Offsets are relative to the domain data block; `0xffff` selects no auxiliary
/// or suffix text. This is the decoded form the expansion walkers consume.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DecodedPrompt {
    pub kind: u8,
    pub flags: [u8; 4],
    pub data_offset: u32,
    pub text_offset: u32,
    pub auxiliary_offset: Option<u32>,
    pub entry_index: u32,
    pub count: u16,
    pub suffix_offset: Option<u32>,
}

impl PackedPrompt {
    /// fn_27de8c / EU fn_27e048 / KR fn_27e0f4.
    pub fn decode(&self, kind: u8) -> DecodedPrompt {
        let data = self.data_offset;
        let relative = |offset: u16| (offset != 0xffff).then(|| data.wrapping_add(offset as u32));
        DecodedPrompt {
            kind,
            flags: self.flags,
            data_offset: data,
            text_offset: data.wrapping_add(self.text_offset as u32),
            auxiliary_offset: relative(self.auxiliary_offset),
            entry_index: self.entry_index,
            count: self.count,
            suffix_offset: relative(self.suffix_offset),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceListNode {
    pub key: u32,
    pub value: u32,
    pub next: Option<usize>,
}

pub struct ResourceList<'a> {
    pub nodes: &'a mut [ResourceListNode],
    pub head: Option<usize>,
    pub tail: Option<usize>,
}

impl ResourceList<'_> {
    /// fn_2817ac / EU fn_281968 / KR fn_281a14.
    pub fn initialize(&mut self) -> u32 {
        self.head = None;
        self.tail = None;
        0
    }

    /// fn_2817c0 / EU fn_28197c / KR fn_281a28.
    pub fn remove(&mut self, value: u32, mut release: impl FnMut(usize)) -> u32 {
        let mut previous: Option<usize> = None;
        let mut current = self.head;
        while let Some(index) = current {
            let node = self.nodes[index];
            if node.value == value {
                if let Some(previous) = previous {
                    self.nodes[previous].next = node.next;
                } else {
                    self.head = node.next;
                }
                if node.next.is_none() {
                    self.tail = previous;
                }
                release(index);
                return 0;
            }
            previous = current;
            current = node.next;
        }
        0x85002014
    }

    /// fn_281864 / EU fn_281a20 / KR fn_281acc.
    pub fn clear(&mut self, mut release: impl FnMut(usize)) -> u32 {
        while let Some(index) = self.head {
            self.head = self.nodes[index].next;
            if self.head.is_none() {
                self.tail = None;
            }
            release(index);
        }
        0
    }

    /// fn_2818b8 / EU fn_281a74 / KR fn_281b20.
    pub fn insert(&mut self, free: Option<usize>, value: u32) -> u32 {
        let Some(index) = free else { return 0x8500200a };
        self.nodes[index] = ResourceListNode {
            key: u32::MAX,
            value,
            next: None,
        };
        if let Some(first) = self.head {
            if self.nodes[first].key == u32::MAX {
                self.nodes[index].next = Some(first);
                self.head = Some(index);
                return 0;
            }
            let mut last = first;
            while let Some(next) = self.nodes[last].next {
                last = next;
            }
            self.nodes[last].next = Some(index);
        } else {
            self.head = Some(index);
        }
        self.tail = Some(index);
        0
    }

    /// fn_281948 / EU fn_281b04 / KR fn_281bb0.
    pub fn set_key(&mut self, value: u32, key: u32) -> u32 {
        let Some(mut node) = self.head else {
            return 0x85002014;
        };
        let mut current = Some(node);
        let mut previous = None;
        let mut predecessor = None;
        let mut insertion = None;
        while let Some(index) = current {
            let item = self.nodes[index];
            if item.value == value {
                if item.key == key {
                    return 0;
                }
                predecessor = previous;
            }
            if item.key == key && key != u32::MAX {
                return 0x85002419;
            }
            if item.key <= key && item.value != value {
                insertion = current;
            }
            previous = current;
            if predecessor.is_some() && insertion.is_some() {
                break;
            }
            current = item.next;
        }
        if key == u32::MAX {
            if let Some(index) = insertion {
                if self.nodes[index].next != self.tail {
                    insertion = self.tail;
                }
            }
        }
        if predecessor == insertion {
            node = predecessor.map_or(node, |index| self.nodes[index].next.unwrap());
            self.nodes[node].key = key;
            return 0;
        }
        if let Some(index) = predecessor {
            node = self.nodes[index].next.unwrap();
            self.nodes[index].next = self.nodes[node].next;
            if self.nodes[node].next.is_none() {
                self.tail = predecessor;
            }
        } else {
            if self.nodes[node].value != value {
                return 0x85002014;
            }
            self.head = self.nodes[node].next;
            if self.nodes[node].next.is_none() {
                self.tail = None;
            }
        }
        if let Some(index) = insertion {
            self.nodes[node].next = self.nodes[index].next;
            self.nodes[index].next = Some(node);
            if self.nodes[node].next.is_none() {
                self.tail = Some(node);
            }
        } else {
            self.nodes[node].next = self.head;
            self.head = Some(node);
        }
        self.nodes[node].key = key;
        0
    }

    /// fn_281aa4 / EU fn_281c60 / KR fn_281d0c.
    pub fn next(&self, cursor: &mut Option<usize>) -> u32 {
        *cursor = cursor.map_or(self.head, |index| self.nodes[index].next);
        cursor.map_or(0, |index| self.nodes[index].value)
    }
}

/// fn_1f3da4 / EU fn_1f3f60 / KR fn_1f400c.
pub fn validate_prompt(validation: u32) -> u32 {
    if validation >= 0x80000000 {
        0x8ba02008
    } else {
        validation
    }
}

/// fn_21037c / EU fn_210538 / KR fn_2105e4.
pub fn validate_initializer(validation: u32) -> u32 {
    if validation >= 0x80000000 {
        0x89a02008
    } else {
        validation
    }
}

/// fn_200e10 / EU fn_200fcc / KR fn_201078.
pub fn validate_prompt_orthography(validation: u32) -> u32 {
    if validation >= 0x80000000 {
        0x8bb02008
    } else {
        validation
    }
}

/// fn_22e260 / EU fn_22e41c / KR fn_22e4c8.
pub fn finish_audio_fetcher() -> u32 {
    0
}

/// fn_2002e0 / EU fn_20049c / KR fn_200548.
pub fn retain_domain_key() -> u32 {
    1
}

/// fn_2002e8 / EU fn_2004a4 / KR fn_200550.
pub fn release_domain_key<T>(key: Option<T>, mut release: impl FnMut(T)) {
    if let Some(key) = key {
        release(key);
    }
}

/// fn_1ff714 / EU fn_1ff8d0 / KR fn_1ff97c.
pub fn unsupported_domain_operation(validation: u32, mut context: impl FnMut() -> u32) -> u32 {
    if validation >= 0x80000000 {
        return validation;
    }
    let status = context();
    if status >= 0x80000000 {
        status
    } else {
        0x88e02001
    }
}

/// fn_1ff754 / EU fn_1ff910 / KR fn_1ff9bc.
pub fn domain_resource_types<'a>(
    validation: u32,
    mut context: impl FnMut() -> u32,
    types: &'a [u8],
    output: &mut Option<&'a [u8]>,
) {
    if validation < 0x80000000 && context() < 0x80000000 {
        *output = Some(types);
    }
}

impl MemoryStream<'_> {
    /// fn_281afc / EU fn_281cb8 / KR fn_281d64.
    pub fn close(&mut self, owned: bool, mut release: impl FnMut(&[u8])) -> u32 {
        if let Some(data) = self.data.take() {
            if owned {
                release(data);
            }
        }
        0
    }
}

#[derive(Clone, Copy)]
pub struct PromptGroup<'a> {
    pub kind: u8,
    pub name: Option<&'a [u8]>,
    pub count: u32,
}

pub struct PromptDomain<'a> {
    pub validation: u8,
    pub enabled: u8,
    pub original_orthography: u8,
    pub groups: &'a [PromptGroup<'a>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptTextMode {
    Original,
    Normalized { first: u32, second: u32 },
}

pub trait PromptExpansion {
    type Decoded: Copy;
    fn validate(&mut self) -> u8;
    fn first_link(&self) -> Option<usize>;
    fn next_link(&self, link: usize) -> Option<usize>;
    fn link_kind(&self, link: usize) -> u8;
    fn decode(&mut self, group: usize, record: u32) -> Self::Decoded;
    fn priority(&self, record: Self::Decoded) -> (u32, u8, u8);
    fn matches_marker(&self, names: Option<&[u8]>) -> bool;
    fn expand(
        &mut self,
        record: Self::Decoded,
        group: usize,
        link: usize,
        position: &mut u16,
        mode: PromptTextMode,
    ) -> u32;
}

impl PromptDomain<'_> {
    fn group_enabled(
        &self,
        index: usize,
        mode: PromptTextMode,
        runtime: &impl PromptExpansion,
    ) -> bool {
        contains_name(self.groups[index].name, Some(b"*\0")) != 0
            || matches!(mode, PromptTextMode::Normalized { .. })
                && runtime.matches_marker(self.groups[index].name)
    }

    fn process(&mut self, runtime: &mut impl PromptExpansion, mode: PromptTextMode) -> u32 {
        if self.validation == 0 {
            self.validation = runtime.validate();
        }
        if self.validation != 2
            || self.enabled == 0
            || matches!(mode, PromptTextMode::Original) && self.original_orthography == 0
        {
            return 0;
        }
        let mut link = runtime.first_link();
        let mut status = 0;
        while let Some(current) = link {
            status = 0;
            if runtime.link_kind(current) != 0 {
                let mut position = 0;
                let paired =
                    self.groups.len() >= 2 && self.groups[0].kind == 1 && self.groups[1].kind == 2;
                let start = if paired {
                    if self.group_enabled(0, mode, runtime) && self.group_enabled(1, mode, runtime)
                    {
                        let mut indices = [0u32; 2];
                        let mut decoded = [runtime.decode(0, 0), runtime.decode(1, 0)];
                        while runtime.link_kind(current) == 1
                            && (indices[0] < self.groups[0].count
                                || indices[1] < self.groups[1].count)
                        {
                            let selected = if indices[1] >= self.groups[1].count {
                                0
                            } else if indices[0] == self.groups[0].count
                                || runtime.priority(decoded[1]) > runtime.priority(decoded[0])
                            {
                                1
                            } else {
                                0
                            };
                            status = runtime.expand(
                                decoded[selected],
                                selected,
                                current,
                                &mut position,
                                mode,
                            );
                            if status >= 0x80000000 {
                                return status;
                            }
                            indices[selected] = indices[selected].wrapping_add(1);
                            if indices[selected] < self.groups[selected].count {
                                decoded[selected] = runtime.decode(selected, indices[selected]);
                            }
                        }
                    }
                    2
                } else {
                    0
                };
                for group in start..self.groups.len() {
                    if !self.group_enabled(group, mode, runtime) {
                        continue;
                    }
                    let mut index = 0;
                    while runtime.link_kind(current) == 1 && index < self.groups[group].count {
                        let decoded = runtime.decode(group, index);
                        status = runtime.expand(decoded, group, current, &mut position, mode);
                        if status >= 0x80000000 {
                            return status;
                        }
                        index = index.wrapping_add(1);
                    }
                }
            }
            link = runtime.next_link(current);
        }
        status
    }

    /// fn_27f7cc / EU fn_27f988 / KR fn_27fa34. Game-unreachable without an APDB resource.
    pub fn expand_original(&mut self, runtime: &mut impl PromptExpansion) -> u32 {
        self.process(runtime, PromptTextMode::Original)
    }

    /// fn_27feb4 / EU fn_280070 / KR fn_28011c. Game-unreachable without an APDB resource.
    pub fn expand_normalized(
        &mut self,
        first: u32,
        second: u32,
        runtime: &mut impl PromptExpansion,
    ) -> u32 {
        self.process(runtime, PromptTextMode::Normalized { first, second })
    }
}
