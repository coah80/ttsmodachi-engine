use crate::frames::PhonemeFrame;

pub const AUDIO_CAPACITY: usize = 0x10266;

#[derive(Clone, Copy, Debug, Default)]
pub struct SpeechEvent {
    pub kind: u16,
    pub value: i32,
    pub position: u32,
    pub character: u16,
    pub command: u32,
}

#[derive(Clone, Debug)]
pub struct AudioQueue {
    pub offsets: [usize; 128],
    pub lengths: [i32; 128],
    pub count: i32,
    pub written: i32,
    pub pending: i32,
}

impl Default for AudioQueue {
    fn default() -> Self {
        Self {
            offsets: [0; 128],
            lengths: [0; 128],
            count: 0,
            written: 0,
            pending: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpeechFrame {
    pub started: bool,
    pub start_value: i32,
    pub boundary_value: i32,
    pub commands: [u32; 256],
    pub command_count: usize,
    pub audio: AudioQueue,
    pub character_count: usize,
    pub characters: [u16; 128],
    pub character_offsets: [i32; 128],
    pub group_count: usize,
    pub group_starts: [i32; 128],
    pub group_lengths: [i32; 128],
    pub output_count: usize,
    pub output_starts: [i32; 128],
    pub output_lengths: [i32; 128],
    pub current_time: i32,
    pub before_time: i32,
    pub after_time: i32,
    pub pauses: [i32; 16],
    pub pause_count: usize,
    pub pause_total: i32,
    pub position: u32,
    pub last_command: i32,
    pub last_pause: i32,
    pub suppress_boundary: i8,
}

impl Default for SpeechFrame {
    fn default() -> Self {
        Self {
            started: false,
            start_value: 0,
            boundary_value: 0,
            commands: [0; 256],
            command_count: 0,
            audio: AudioQueue::default(),
            character_count: 0,
            characters: [0; 128],
            character_offsets: [0; 128],
            group_count: 0,
            group_starts: [0; 128],
            group_lengths: [0; 128],
            output_count: 0,
            output_starts: [0; 128],
            output_lengths: [0; 128],
            current_time: 0,
            before_time: 0,
            after_time: 0,
            pauses: [0; 16],
            pause_count: 0,
            pause_total: 0,
            position: u32::MAX,
            last_command: 0,
            last_pause: 0,
            suppress_boundary: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VoiceBoundary {
    pub enabled: bool,
    pub distance: i32,
}

impl VoiceBoundary {
    /// fn_1b9090 / EU fn_1b9140 / KR fn_1b92dc.
    pub fn distance(&self) -> i32 {
        if self.enabled {
            self.distance
        } else {
            0
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssemblyResult {
    pub ready: bool,
    pub samples: i32,
    pub events: usize,
}

impl SpeechFrame {
    pub fn phonemes(&self) -> PhonemeFrame<'_> {
        PhonemeFrame {
            count: self.character_count as i32,
            symbols: &self.characters,
            starts: &self.character_offsets,
        }
    }

    /// fn_1b7710 / EU fn_1b77c0 / KR fn_1b7960.
    pub fn group(&mut self, unvoiced: &[u8; 256]) {
        let base = self.phonemes().leading_sample();
        let mut index = 0;
        if base > 0 {
            while self.characters[index] == 0x23 && self.character_offsets[0] == 0 {
                index += 1;
            }
        }
        let mut start: Option<i32> = None;
        self.group_count = 0;
        while index < self.character_count {
            if let Some(start) = start {
                self.group_starts[self.group_count] = start.wrapping_sub(base);
                self.group_lengths[self.group_count] =
                    self.character_offsets[index].wrapping_sub(start);
                self.group_count += 1;
            }
            start = if unvoiced[self.characters[index] as u8 as usize] == 0 {
                Some(self.character_offsets[index])
            } else {
                None
            };
            index += 1;
        }
        if let Some(start) = start {
            self.group_starts[self.group_count] = start.wrapping_sub(base);
            self.group_lengths[self.group_count] = self.audio.written.wrapping_sub(start);
            self.group_count += 1;
        }
    }

    /// fn_1b0364 / EU fn_1b03b8 / KR fn_1b044c.
    /// Storage replaces D_a7b794. Event, command and sample capacities are the ARM bounds.
    pub fn assemble(
        &mut self,
        storage: &mut [i16; AUDIO_CAPACITY],
        position: u32,
        audio: &[i16],
        events: &[SpeechEvent],
        include_time: bool,
        flush: bool,
        voice: VoiceBoundary,
        unvoiced: &[u8; 256],
    ) -> AssemblyResult {
        self.assemble_region(
            storage,
            position,
            audio,
            events,
            include_time,
            flush,
            voice,
            unvoiced,
            false,
        )
    }

    /// KR fn_1b044c / US fn_1b0364 / EU fn_1b03b8.
    pub fn assemble_korean(
        &mut self,
        storage: &mut [i16; AUDIO_CAPACITY],
        position: u32,
        audio: &[i16],
        events: &[SpeechEvent],
        include_time: bool,
        flush: bool,
        voice: VoiceBoundary,
        unvoiced: &[u8; 256],
    ) -> AssemblyResult {
        self.assemble_region(
            storage,
            position,
            audio,
            events,
            include_time,
            flush,
            voice,
            unvoiced,
            true,
        )
    }

    fn assemble_region(
        &mut self,
        storage: &mut [i16; AUDIO_CAPACITY],
        position: u32,
        audio: &[i16],
        events: &[SpeechEvent],
        include_time: bool,
        flush: bool,
        voice: VoiceBoundary,
        unvoiced: &[u8; 256],
        korean: bool,
    ) -> AssemblyResult {
        let mut previous = u32::MAX;
        let mut ready = false;
        let mut next_position = 0x7fffffff;
        let mut end = i32::MAX as usize;
        let mut spacing = 0;
        let mut first = 0;
        let mut boundary = i32::MAX as usize;
        for (i, event) in events.iter().enumerate() {
            if event.kind == 2 {
                if !korean && self.suppress_boundary != 0 && event.position != previous {
                    continue;
                }
                if self.position == u32::MAX {
                    if event.position == position {
                        self.position = event.position;
                        first = i;
                        self.boundary_value = event.value;
                        continue;
                    }
                    self.position = position;
                }
                if ready {
                    if event.position != next_position {
                        break;
                    }
                    spacing = i - boundary;
                }
                boundary = i;
                next_position = event.position;
                ready = true;
            } else if !korean && event.kind == 8 {
                if self.suppress_boundary == 0 && event.command == 30 {
                    self.suppress_boundary = 1;
                    previous = event.position;
                } else if self.suppress_boundary == 1 && event.command == 31 {
                    self.suppress_boundary = 0;
                }
            }
        }
        if self.position == u32::MAX {
            self.position = position;
        }
        if ready {
            if spacing != 0 {
                first = boundary - spacing;
                end = first + 1;
            } else if self.position == next_position {
                end = first + 1;
            } else {
                end = events
                    .iter()
                    .take_while(|e| e.position < next_position)
                    .count();
            }
        } else if !audio.is_empty() {
            let target = self
                .position
                .wrapping_add(self.audio.written as u32)
                .wrapping_add(audio.len() as u32);
            let mut first = None;
            let mut previous = None;
            end = events.iter().take_while(|e| e.position <= target).count();
            for (i, event) in events[..end].iter().enumerate() {
                if event.kind == 4 && unvoiced[event.character as u8 as usize] == 0 {
                    first = previous;
                    previous = Some(i);
                }
            }
            if self.pause_count == 0 && !flush {
                let pauses = events[..end]
                    .iter()
                    .filter(|e| e.command.wrapping_sub(150000000) < 10000000)
                    .count();
                if pauses == 0 && previous.is_some() {
                    if let Some(first) = first {
                        let event = &events[first + 1];
                        let distance = target.wrapping_sub(event.position) as i32;
                        if voice.distance() <= distance {
                            end = first + 1;
                            next_position = event.position;
                        }
                    }
                    ready = true;
                }
            }
        }
        if flush {
            ready = true;
        }
        let mut consumed = 0;
        for (i, event) in events.iter().enumerate() {
            if event.kind == 1 && i < end && event.position == self.position {
                self.started = true;
                self.start_value = event.value;
            }
            if i < end && event.kind == 4 {
                let offset = event.position.wrapping_sub(self.position) as i32;
                if offset >= AUDIO_CAPACITY as i32 {
                    if i < end || flush {
                        consumed += 1;
                    }
                    continue;
                }
                if self.character_count < 128 {
                    self.characters[self.character_count] = event.character;
                    self.character_offsets[self.character_count] = offset;
                    self.character_count += 1;
                }
            }
            if event.kind == 8 {
                let kind = event.command / 10000000;
                let value = (event.command % 10000000) as i32;
                if kind == 21 {
                    let time = if event.position == self.position {
                        if include_time {
                            Some(&mut self.current_time)
                        } else {
                            None
                        }
                    } else if i < boundary {
                        Some(&mut self.before_time)
                    } else {
                        Some(&mut self.after_time)
                    };
                    if let Some(time) = time {
                        *time = time.wrapping_add(value);
                    }
                } else if i < end {
                    if kind == 15 {
                        if event.position != self.position && self.last_command > self.last_pause {
                            self.before_time = self.before_time.wrapping_add(value);
                        } else {
                            if self.pause_count < 16 {
                                self.pauses[self.pause_count] = value;
                                self.pause_count += 1;
                            } else {
                                assert_eq!(
                                    self.pause_count, 16,
                                    "pause overflow already corrupted frame"
                                );
                                self.pause_count =
                                    (self.pause_count as i32).wrapping_add(value) as usize;
                            }
                            self.pause_total = self.pause_total.wrapping_add(value);
                        }
                        self.last_pause = event.position as i32;
                    } else {
                        if kind == 2 {
                            self.last_command = event.position as i32;
                        }
                        self.commands[self.command_count] = event.command;
                        self.command_count += 1;
                    }
                }
            }
            if i < end || flush {
                consumed += 1;
            }
        }
        let mut length = audio.len() as i32;
        if next_position != 0x7fffffff {
            length = next_position
                .wrapping_sub(self.position)
                .wrapping_sub(self.audio.written.wrapping_add(self.audio.pending) as u32)
                as i32;
        }
        if length > 0 || self.audio.count == 0 {
            let copied = (AUDIO_CAPACITY as i32 - self.audio.written).min(length);
            assert!(copied >= 0, "negative audio copy");
            if copied < length {
                self.audio.pending = self.audio.pending.wrapping_add(length - copied);
            }
            let offset = self.audio.written as usize;
            if self.audio.count == 128 {
                self.audio.count = copied + 128;
            } else {
                let index = self.audio.count as usize;
                self.audio.offsets[index] = offset;
                self.audio.lengths[index] = copied;
                self.audio.count += 1;
            }
            self.audio.written += copied;
            storage[offset..offset + copied as usize].copy_from_slice(&audio[..copied as usize]);
        }
        if !korean && self.pause_count == 0 && length > 0 {
            ready = true;
        }
        if ready {
            self.group(unvoiced);
            let mut offset = 0i32;
            let base = self.phonemes().leading_sample();
            self.output_count = 0;
            for i in 0..self.group_count {
                let end = if i + 1 == self.group_count {
                    self.audio.written.wrapping_sub(base)
                } else {
                    self.group_lengths[i].wrapping_add(self.group_starts[i])
                };
                let start = offset.wrapping_add(base);
                let length = end.wrapping_sub(offset);
                offset = offset.wrapping_add(length);
                self.output_starts[self.output_count] = start;
                self.output_lengths[self.output_count] = length;
                self.output_count += 1;
            }
        }
        AssemblyResult {
            ready,
            samples: length,
            events: consumed,
        }
    }
}
