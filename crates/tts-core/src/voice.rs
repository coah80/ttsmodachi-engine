use crate::{math, tables::NumericTables};

pub const QUEUE_CAPACITY: usize = 256;

#[derive(Clone, Copy, Debug, Default)]
pub struct VoiceEvent {
    pub kind: i32,
    pub value: f32,
    pub target: f32,
    pub duration: f32,
    pub delay: i32,
}

#[derive(Clone, Debug)]
pub struct VoiceQueue {
    pub events: [VoiceEvent; QUEUE_CAPACITY],
    pub read: u8,
    pub write: u8,
}

impl Default for VoiceQueue {
    fn default() -> Self {
        Self {
            events: [VoiceEvent::default(); QUEUE_CAPACITY],
            read: 0,
            write: 0,
        }
    }
}

impl VoiceQueue {
    fn relative_delay(&self, base: i32, delay: i32) -> i32 {
        let mut delay = delay.wrapping_add(base);
        let mut index = self.read;
        if index != self.write {
            loop {
                delay = delay.wrapping_sub(self.events[index as usize].delay);
                index = index.wrapping_add(1);
                if index == self.write {
                    break;
                }
            }
            delay = delay.max(0);
        }
        delay
    }

    pub fn enqueue(&mut self, base: i32, kind: i32, delay: i32, value: f32) -> u32 {
        let delay = self.relative_delay(base, delay);
        let event = &mut self.events[self.write as usize];
        event.kind = kind;
        event.delay = delay;
        event.value = value;
        self.write = self.write.wrapping_add(1);
        0
    }

    pub fn enqueue_flag(&mut self, base: i32, value: u32, delay: i32) -> u32 {
        self.enqueue(base, 20, delay, value as f32)
    }

    pub fn enqueue_pair(
        &mut self,
        base: i32,
        kind: i32,
        delay: i32,
        value: f32,
        target: f32,
    ) -> i32 {
        let delay = self.relative_delay(base, delay);
        let event = &mut self.events[self.write as usize];
        event.kind = kind;
        event.delay = delay;
        event.value = value;
        event.target = target;
        self.write = self.write.wrapping_add(1);
        0
    }

    pub fn enqueue_transition(
        &mut self,
        base: i32,
        kind: i32,
        delay: i32,
        value: f32,
        target: f32,
        duration: f32,
    ) -> i32 {
        let delay = self.relative_delay(base, delay);
        self.events[self.write as usize] = VoiceEvent {
            kind,
            value,
            target,
            duration,
            delay,
        };
        self.write = self.write.wrapping_add(1);
        0
    }

    pub fn schedule_frequency(&mut self, base: i32, duration: i32, frequency: f32) {
        self.enqueue_transition(base, 17, 0, 3.0, frequency, duration as f32);
    }

    pub fn enqueue_duration(&mut self, base: i32, delay: i32, value: f32, target: f32) -> i32 {
        self.enqueue_pair(base, 11, delay, value, target)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VoiceTransition {
    pub kind: i32,
    pub reciprocal: f32,
    pub remaining: i32,
}

impl Default for VoiceTransition {
    fn default() -> Self {
        Self {
            kind: 0,
            reciprocal: 0.0,
            remaining: i32::MIN,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct VoiceState {
    pub sample_rate: f32,
    pub enabled: bool,
    pub position: f32,
    pub time: i32,
    pub value6c: f32,
    pub value70: f32,
    pub gain: f32,
    pub rate: f32,
    pub period: f32,
    pub value84: f32,
    pub value88: f32,
    pub base8c: f32,
    pub coefficient90: f32,
    pub target94: f32,
    pub base98: f32,
    pub coefficient9c: f32,
    pub targeta0: f32,
    pub valuea4: f32,
    pub durationa8: i32,
    pub valueb0: f32,
    pub durationb4: i32,
    pub active: [bool; 3],
    pub targets: [f32; 3],
    pub phases: [f32; 3],
    pub offsetd8: i32,
    pub offsetdc: i32,
    pub flag: bool,
    pub transitions: [VoiceTransition; 2],
}

impl VoiceState {

    pub fn advance_timers(&mut self, dt: i32) {
        for transition in &mut self.transitions {
            if transition.remaining == i32::MIN {
                continue;
            }
            transition.remaining = transition.remaining.wrapping_sub(dt);
            if transition.remaining <= 0 && transition.kind == 16 {
                let scale = self.rate;
                let k = transition.reciprocal;
                let adjustment =
                    (self.gain * scale * ((1.0 - k) * transition.remaining as f32)) as i32;
                self.rate = scale * k;
                transition.remaining = i32::MIN;
                self.position += adjustment as f32;
            }
        }
    }

    pub fn consume(
        &mut self,
        queue: &mut VoiceQueue,
        mut commands: Option<&mut VoiceQueue>,
        elapsed: i32,
        tables: &NumericTables<'_>,
    ) -> i32 {
        if queue.write != queue.read {
            let event = &mut queue.events[queue.read as usize];
            event.delay = event.delay.wrapping_sub(elapsed);
        }
        let mut count = 0;
        while queue.events[queue.read as usize].delay <= 0 && queue.write != queue.read {
            let event = queue.events[queue.read as usize];
            if event.kind == 17 {
                let destination = match commands.as_deref_mut() {
                    Some(q) => q,
                    None => &mut *queue,
                };
                destination.enqueue(
                    self.time,
                    event.value as i32,
                    (event.duration as i32)
                        .wrapping_add(event.delay)
                        .wrapping_sub(self.time),
                    event.target,
                );
            } else {
                self.apply_event(event, tables);
            }
            queue.read = queue.read.wrapping_add(1);
            let next = &mut queue.events[queue.read as usize];
            next.delay = next.delay.wrapping_add(event.delay);
            count += 1;
        }
        count
    }

    fn apply_event(&mut self, event: VoiceEvent, tables: &NumericTables<'_>) {
        match event.kind {
            0 => self.value6c = event.value,
            1 => {
                self.value70 = if event.value.to_bits() as i32 > 0x3e800000 {
                    event.value
                } else {
                    0.25
                }
            }
            2 => self.gain = event.value,
            3 => self.period = self.sample_rate / event.value,
            4..=6 => {
                let index = (event.kind - 4) as usize;
                self.active[index] = event.value != 0.0;
                self.targets[index] = event.target;
                self.phases[index] = 0.0;
            }
            7 => {
                self.value88 = if event.value != 0.0 {
                    math::expf(-1.0 / (event.value * 200.0), tables)
                } else {
                    0.0
                }
            }
            8 => self.value84 = 1.0 - event.value,
            9 | 10 => {
                let (base, coefficient, target) = if event.kind == 9 {
                    (
                        &mut self.base8c,
                        &mut self.coefficient90,
                        &mut self.target94,
                    )
                } else {
                    (
                        &mut self.base98,
                        &mut self.coefficient9c,
                        &mut self.targeta0,
                    )
                };
                if event.value != 0.0 {
                    *base = event.value;
                }
                let duration = event.duration * 200.0;
                *coefficient =
                    math::expf(math::logf(event.target / *base, tables) / duration, tables);
                *target = event.target;
            }
            11 => {
                self.valuea4 = event.value;
                self.durationa8 = (event.target * 5.12) as i32;
            }
            12 => {
                self.valueb0 = event.value;
                self.durationb4 = (event.target * 5.12) as i32;
            }
            13 => self.enabled = false,
            14 => self.enabled = true,
            15 => {
                if let Some(transition) = self
                    .transitions
                    .iter_mut()
                    .find(|t| t.remaining == i32::MIN)
                {
                    let factor = event.value;
                    let rate = self.rate;
                    self.rate = rate * factor;
                    transition.kind = 16;
                    transition.reciprocal = 1.0 / factor;
                    let adjustment =
                        ((self.gain * rate) * ((1.0 - factor) * event.delay as f32)) as i32;
                    transition.remaining = (event.delay as f32 + event.target) as i32;
                    self.position += adjustment as f32;
                }
            }
            18 => self.offsetd8 = self.offsetd8.wrapping_add(event.value as i32),
            19 => self.offsetdc = self.offsetdc.wrapping_add(event.value as i32),
            20 => self.flag = event.value != 0.0,
            _ => {}
        }
    }
}
