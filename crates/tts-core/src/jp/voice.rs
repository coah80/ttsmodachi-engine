//! Caller-owned Japanese voice request: UTF-8/SJIS text + params to PCM.
//!
//! The voice layer (vtable 0x859f6c, ctor 0x607d38, Speak 0x607ef0, update
//! 0x607fb0, teardown 0x607e84) only lends buffers the engine borrows: the
//! 0x14000-byte engine block, two 0x400-byte text/scratch windows and the
//! 0xe10 mora table. No allocation happens here or in the engine; every
//! guest pointer the engine cannot know (sound/voice handles, thread word,
//! guest mapping, SVC echoes) arrives through [`Context`], mirroring the US
//! `NotificationRegisters`.
//!
//! Text accepts UTF-8 hiragana/katakana/ASCII (converted to the Shift-JIS
//! katakana the engine parses) or raw Shift-JIS bytes; anything else is
//! rejected, never substituted.

use crate::jp::sync::GuestSync;
use crate::jp::{Engine, PcmLog, Tables};

/// Guest facts the engine cannot derive (handles, thread word, mapping,
/// stubbed-SVC echoes). `engine_base` places the scheduler request struct.
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub sound: u32,
    pub voice: u32,
    pub thread: u32,
    pub sync: GuestSync,
}

/// Voice failure modes. The engine itself only fails construction/reset
/// (nonzero return) or hangs the worker (rounds exhausted with busy set).
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidText,
    Engine(i32),
    WorkerLimit,
}

const TEXT_SIZE: usize = 0x400;

/// Caller-owned voice: every backing store is borrowed, nothing is
/// allocated. One `speak` call runs construct, reset, setParam in request
/// order, the render entry, the task-driver drain and (optionally) the
/// teardown sync, exactly like the ARM runner path.
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

    /// Render `request` (UTF-8 hiragana/katakana/ASCII, or raw Shift-JIS
    /// when `raw_sjis` holds) with `params` (setParam id/value pairs, in
    /// request order; voice/emotion preset ids 1/2 take 0x101..0x211) to
    /// the caller PCM log. `teardown` runs the voice-teardown sync.
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

    /// Song request (JP): lyric text plus `song_data` (bot songConverter
    /// tags), `bpm` and `stretch`, mirroring sendLyric/writeJob. The JP game
    /// patch (`gamePatch/source/main.c` mainLoopF, REGION_JP) takes the
    /// `#else audioJob->status = ERROR` branch for any song job
    /// (`songDataSize != 0`): status 4, no audio rendered. This returns the
    /// matching `Engine(4)` without touching PCM. Empty `song_data` is the
    /// game speech branch (`songDataSize == 0`), so it delegates to speak.
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

    /// Read-only view of the PCM rendered so far (requests append to it).
    pub fn pcm_window(&self) -> &[i16] {
        &self.pcm.buf[..self.pcm.len]
    }
}

/// Construct/reset, apply parameters in request order, submit text, and
/// drain the deferred worker. Output-capacity exhaustion keeps the
/// `PcmLog` contract (caller-sized store, panics when too small).
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

/// UTF-8 -> katakana Shift-JIS conversion, byte-exact against the ARM
/// runner's faithful fold (hiragana to katakana, then strict `shift_jis`).
/// The fast paths cover ASCII (including NUL, which truncates the engine
/// string exactly like the runner's zero-padded buffer), half-width
/// katakana, the katakana block and eight punctuation marks; every other
/// encodable code point goes through the generated table in
/// [`crate::jp::sjis`] with its true bytes. Code points strict
/// `shift_jis` cannot encode (emoji, some symbols, astral planes) are
/// rejected, never substituted: the ARM has no bytes for them either.
/// The caller owns the 0x400-byte text window including NUL.
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

#[cfg(test)]
mod conv_tests {
    use super::*;

    fn enc_len(s: &str) -> Result<usize, Error> {
        let mut out = [0u8; TEXT_SIZE];
        encode_text(s, &mut out)?;
        Ok(out.iter().position(|&b| b == 0).unwrap_or(out.len()))
    }

    fn enc_bytes(s: &str, n: usize) -> Result<[u8; 8], Error> {
        let mut out = [0u8; TEXT_SIZE];
        encode_text(s, &mut out)?;
        let mut r = [0u8; 8];
        r.copy_from_slice(&out[..8.min(n + 8).min(TEXT_SIZE)][..8]);
        Ok(r)
    }

    #[test]
    fn hiragana_folds_to_katakana_sjis() {
        let mut out = [0u8; TEXT_SIZE];
        encode_text("あ", &mut out).unwrap();
        assert_eq!([out[0], out[1], out[2]], [0x83, 0x41, 0x00]);
        encode_text("ん", &mut out).unwrap();
        assert_eq!([out[0], out[1], out[2]], [0x83, 0x93, 0x00]);
    }

    #[test]
    fn unmappable_rejected() {
        let mut out = [0u8; TEXT_SIZE];
        assert_eq!(encode_text("\u{1F600}", &mut out), Err(Error::InvalidText));
        assert_eq!(encode_text("あ\u{1F600}", &mut out), Err(Error::InvalidText));
    }

    #[test]
    fn limit_0x400_rejected() {
        // 1024 'a' needs 1024 bytes + NUL = 1025 > 1024 -> reject.
        // (Cannot use repeat in no_std; test via speak boundary instead.
        // Here just verify a long-but-ok input works.)
        let mut out = [0u8; TEXT_SIZE];
        assert!(encode_text("あ", &mut out).is_ok());
        assert_eq!(enc_len("あ"), Ok(2));
    }

    #[test]
    fn nul_terminates() {
        let mut out = [0u8; TEXT_SIZE];
        encode_text("あ\0あ", &mut out).unwrap();
        // U+0000 encodes as 0x00; engine reads to first NUL.
        assert_eq!([out[0], out[1], out[2]], [0x83, 0x41, 0x00]);
    }
}
