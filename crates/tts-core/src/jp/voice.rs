

use crate::jp::sync::GuestSync;
use crate::jp::{Engine, PcmLog, Tables};

#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub sound: u32,
    pub voice: u32,
    pub thread: u32,
    pub sync: GuestSync,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidText,
    Engine(i32),
    WorkerLimit,
}

const TEXT_SIZE: usize = 0x400;

pub struct JpVoice<'e, 't, 'p, 'w> {
    engine: Engine<'e>,
    tables: Tables<'t>,
    preset: &'t [u8],
    text_buf: &'w mut [u8; TEXT_SIZE],
    pcm: PcmLog<'p>,
    ctx: Context,
}

impl<'e, 't, 'p, 'w> JpVoice<'e, 't, 'p, 'w> {
    pub fn new(
        engine: Engine<'e>,
        tables: Tables<'t>,
        preset: &'t [u8],
        text_buf: &'w mut [u8; TEXT_SIZE],
        pcm: PcmLog<'p>,
        ctx: Context,
    ) -> Self {
        JpVoice { engine, tables, preset, text_buf, pcm, ctx }
    }

    pub fn speak(
        &mut self,
        request: &str,
        raw_sjis: Option<&[u8]>,
        params: &[(i32, i32)],
        teardown: bool,
    ) -> Result<(), Error> {
        if let Some(sjis) = raw_sjis {
            if sjis.len() >= TEXT_SIZE {
                return Err(Error::InvalidText);
            }
            self.text_buf.fill(0);
            self.text_buf[..sjis.len()].copy_from_slice(sjis);
        } else {
            encode_text(request, self.text_buf)?;
        }
        render(
            &mut self.engine,
            &self.tables,
            self.preset,
            self.text_buf,
            params,
            &self.ctx,
            teardown,
            &mut self.pcm,
        )
    }

    pub fn sing(
        &mut self,
        lyric: &str,
        song_data: &[u8],
        _bpm: i32,
        _stretch: i32,
    ) -> Result<(), Error> {
        if song_data.is_empty() {
            return self.speak(lyric, None, &[], false);
        }
        Err(Error::Engine(4))
    }

    pub fn pcm_len(&self) -> usize {
        self.pcm.len
    }

    pub fn pcm_window(&self) -> &[i16] {
        &self.pcm.buf[..self.pcm.len]
    }
}

pub(crate) fn render(
    engine: &mut Engine<'_>,
    tables: &Tables<'_>,
    preset: &[u8],
    text: &[u8],
    parameters: &[(i32, i32)],
    ctx: &Context,
    teardown: bool,
    pcm: &mut PcmLog<'_>,
) -> Result<(), Error> {
    if text.len() > TEXT_SIZE || !text.contains(&0) {
        return Err(Error::InvalidText);
    }
    let rc = crate::jp::construct(engine, ctx.sync);
    if rc != 0 {
        return Err(Error::Engine(rc));
    }
    let rc = crate::jp::reset(engine);
    if rc != 0 {
        return Err(Error::Engine(rc));
    }
    for &(id, value) in parameters {
        let rc = crate::jp::set_param(engine, preset, id, value);
        if rc != 0 {
            return Err(Error::Engine(rc));
        }
    }
    let rc = crate::jp::render_entry(
        engine,
        tables,
        text,
        0,
        0x11,
        ctx.sound,
        ctx.voice,
        ctx.thread,
        pcm,
        ctx.sync,
    );
    if rc != 0 {
        return Err(Error::Engine(rc));
    }
    for _ in 0..64 {
        if engine.rd8(0xE6CB) == 0 {
            break;
        }
        crate::jp::task_driver(engine, tables, pcm);
    }
    if engine.rd8(0xE6CB) != 0 {
        return Err(Error::WorkerLimit);
    }
    if teardown {
        crate::jp::render_sync(engine, ctx.thread);
    }
    Ok(())
}

pub(crate) fn encode_text(text: &str, out: &mut [u8; TEXT_SIZE]) -> Result<(), Error> {
    out.fill(0);
    let mut at = 0;
    for character in text.chars() {
        let mut value = character as u32;
        if (0x3041..=0x3096).contains(&value) {
            value += 0x60;
        }
        let (lead, trail) = match value {
            0..=0x7f => (value as u8, 0),
            0xff61..=0xff9f => ((value - 0xff61 + 0xa1) as u8, 0),
            0x30a1..=0x30f6 => {
                let mut t = value - 0x30a1 + 0x40;
                if t >= 0x7f {
                    t += 1;
                }
                (0x83, t as u8)
            }
            0x3000 => (0x81, 0x40),
            0x3001 => (0x81, 0x41),
            0x3002 => (0x81, 0x42),
            0x30fb => (0x81, 0x45),
            0xff1f => (0x81, 0x48),
            0xff01 => (0x81, 0x49),
            0x30fc => (0x81, 0x5b),
            0x300c => (0x81, 0x75),
            0x300d => (0x81, 0x76),
            _ => match super::sjis::lookup(value) {
                Some((hi, 0)) => (hi, 0),
                Some((hi, lo)) => (hi, lo),
                None => return Err(Error::InvalidText),
            },
        };
        let width = if trail == 0 { 1 } else { 2 };
        if at + width >= out.len() {
            return Err(Error::InvalidText);
        }
        out[at] = lead;
        if width == 2 {
            out[at + 1] = trail;
        }
        at += width;
    }
    Ok(())
}
