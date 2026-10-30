//! Drop-in tts-worker on `engine_us::speak()` (lane us r20).
//!
//! Same HTTP contract as the bot's `ttsmodachi_bot/renderer_service.py`
//! `/render` (plus `/health`), read-only reference:
//!   - `POST /render` JSON `{"text": str, "voice": {...}, "mode": "text",
//!     "backend": "citra"|"standalone"}` -> 200 `audio/wav` (WAV file with
//!     a real 44-byte RIFF header, 16 kHz mono s16, gain applied).
//!   - Voice mapping: bot-level ints, `engine_intonation = intonation - 1`;
//!     unknown/non-US `lang` and non-`text` modes are rejected (production
//!     coerces/retries through citra; this worker is US-only).
//!   - App-layer text handling before callTTS, 1:1 from `api/tts.py` +
//!     `renderer_service.py`: newline -> space, strip, 2000-char limit,
//!     grammar pauses (port of `prosody.py`), bleep/echo markup rewrite.
//!     `speak()` itself appends the NUL (as `sendText` does).
//!   - `pool.rs` is renderer_pool.py: lanes (512 MB stack, >= 128 MB required)
//!     each own a `UsEngine` and serve in order with carried state, exactly
//!     like one citra worker. Any failure (incl. a finished job with no
//!     audio) rebuilds the engine pristine and retries once (restart + 1
//!     retry); a failed retry restarts the lane and moves to the next one.
//!
//! Debug/test scaffolding for the binary only; engine untouched.

use std::io::{Read, Write};
use std::net::TcpListener;

mod pool;

// ---------------------------------------------------------------------------
// reqZ template loader (copy of tts-core/tests/reqz_snapshot.rs, which is
// test/example-only scaffolding; duplicated so the binary is self-contained).
// ---------------------------------------------------------------------------

mod reqz {
    use std::{collections::HashSet, fs, path::PathBuf};

    const CODE_START: u32 = 0x0010_0000;
    const CODE_END: u32 = 0x0100_0000;
    const LINEAR_START: u32 = 0x1400_0000;
    const HEAP_START: u32 = 0x0800_0000;
    const PAGE: usize = 0x1000;

    pub fn snapshot_dir() -> PathBuf {
        PathBuf::from(std::env::var_os("TTS_DATA").unwrap_or_else(|| "data".into()))
    }

    pub fn seg2_dir() -> PathBuf {
        snapshot_dir()
    }

    /// Parsed wire image: page runs as (base, file offset, len) into the
    /// retained `data` buffer. Only the 2.4 MB wire file stays resident.
    pub struct WireIndex {
        pub pages: Vec<(u32, usize, usize)>,
        pub data: Vec<u8>,
        pub args: [u32; 7],
        pub max_end: usize,
        pub have: HashSet<u32>,
    }

    pub fn parse_wire(seg2: &PathBuf) -> WireIndex {
        let raw = fs::read(seg2.join("reqZ-191e4b.wire")).expect("wire image");
        assert_eq!(&raw[..4], b"WIR2");
        let word =
            |at: usize| u32::from_le_bytes(raw[at..at + 4].try_into().expect("word"));
        let entry = word(4);
        assert_eq!(entry, 0x191e4b, "reqZ template entry");
        let mut args = [0u32; 7];
        for (i, a) in args.iter_mut().enumerate() {
            *a = word(8 + i * 4);
        }
        let n = word(36) as usize;
        let mut at = 40;
        let mut pages = Vec::with_capacity(n);
        for _ in 0..n {
            let base = word(at);
            let len = word(at + 4) as usize;
            at += 8;
            pages.push((base, at, len));
            at += len;
        }
        assert_eq!(at, raw.len(), "wire image length");
        let have: HashSet<u32> = pages.iter().map(|(b, _, _)| *b).collect();
        let max_end = pages
            .iter()
            .map(|(b, _, len)| *b as usize + *len)
            .max()
            .expect("pages");
        WireIndex { pages, data: raw, args, max_end, have }
    }

    /// Write every template page run into `store` in the exact order the old
    /// collect-then-sort-then-copy path produced (stable by base; wire
    /// segments, then code fills, then linear fills), so overlap resolution
    /// is identical. Snapshot files are paged in per page, never read whole.
    pub fn write_template(
        store: &crate::sparse::Template,
        wire: &WireIndex,
    ) -> ([u32; 7], usize) {
        use std::os::unix::fs::FileExt;
        assert_eq!(store.size, wire.max_end, "template sized from wire");
        let snap = snapshot_dir();
        let code = fs::File::open(snap.join("code.bin")).expect("code.bin");
        let linear = fs::File::open(snap.join("linear.bin")).expect("linear.bin");
        let code_end = CODE_END.min(wire.max_end as u32).min(CODE_START + code_len(&code));
        let linear_end =
            (wire.max_end as u32).min(LINEAR_START + linear_len(&linear));
        let mut runs: Vec<(u32, u8, usize)> = wire
            .pages
            .iter()
            .enumerate()
            .map(|(i, (b, _, _))| (*b, 0u8, i))
            .collect();
        let mut base = CODE_START;
        while base + PAGE as u32 <= code_end {
            if !wire.have.contains(&base) {
                runs.push((base, 1, 0));
            }
            base += PAGE as u32;
        }
        base = LINEAR_START;
        while base + PAGE as u32 <= wire.max_end as u32 && base + PAGE as u32 <= linear_end {
            if !wire.have.contains(&base) {
                runs.push((base, 2, 0));
            }
            base += PAGE as u32;
        }
        runs.sort_by_key(|(b, _, _)| *b);
        let mut page = [0u8; PAGE];
        for (b, tag, i) in runs {
            match tag {
                0 => {
                    let (_, off, len) = wire.pages[i];
                    store
                        .write_at(b as u64, &wire.data[off..off + len])
                        .expect("wire page");
                }
                1 => {
                    let off = (b - CODE_START) as u64;
                    code.read_exact_at(&mut page, off).expect("code page");
                    store.write_at(b as u64, &page).expect("code fill");
                }
                _ => {
                    let off = (b - LINEAR_START) as u64;
                    linear.read_exact_at(&mut page, off).expect("linear page");
                    store.write_at(b as u64, &page).expect("linear fill");
                }
            }
        }
        // request Z never touches the warm-up block, so its header page is
        // outside the wire; a fresh engine frees that block (cold boot has
        // none), and tfree needs the real header, not a zero hole
        let header = warmup_audio_block().wrapping_sub(16);
        let header_page = header & !(PAGE as u32 - 1);
        if header >= HEAP_START && !wire.have.contains(&header_page) {
            let heap = fs::File::open(snap.join("heap.bin")).expect("heap.bin");
            heap.read_exact_at(&mut page, (header_page - HEAP_START) as u64).expect("heap page");
            // heap.bin predates request Z, whose intake appended its blocks
            // after this one: restore the forward link from the block whose
            // prev points back here (walking the wire's used list from the tail)
            let heap0 = wire_word(wire, 0xb6f4ec).expect("heap table");
            let mut b = wire_word(wire, heap0 + 0x48).unwrap_or(0);
            while b != 0 {
                let Some(prev) = wire_word(wire, b + 8) else { break };
                if prev == header {
                    let at = (header + 12 - header_page) as usize;
                    page[at..at + 4].copy_from_slice(&b.to_le_bytes());
                    break;
                }
                b = prev;
            }
            store.write_at(header_page as u64, &page).expect("heap fill");
        }
        (wire.args, wire.max_end)
    }

    fn wire_word(wire: &WireIndex, addr: u32) -> Option<u32> {
        wire.pages.iter().find_map(|&(b, off, len)| {
            let at = addr.checked_sub(b)? as usize;
            (at + 4 <= len).then(|| {
                u32::from_le_bytes(wire.data[off + at..off + at + 4].try_into().expect("word"))
            })
        })
    }

    fn code_len(f: &fs::File) -> u32 {
        f.metadata().map(|m| m.len().min(u32::MAX as u64) as u32).unwrap_or(0)
    }
    fn linear_len(f: &fs::File) -> u32 {
        f.metadata().map(|m| m.len().min(u32::MAX as u64) as u32).unwrap_or(0)
    }

    /// The warm-up render's abandoned audioData, as the snapshot's job
    /// struct recorded it (the reqZ image has the job fields zeroed).
    pub fn warmup_audio_block() -> u32 {
        use std::os::unix::fs::FileExt;
        let code = fs::File::open(snapshot_dir().join("code.bin")).expect("code.bin");
        let mut word = [0u8; 4];
        code.read_exact_at(&mut word, (tts_core::engine_live_seg2::JOB_ADDR + 16 - CODE_START) as u64)
            .expect("job audioData");
        u32::from_le_bytes(word)
    }
}

// ---------------------------------------------------------------------------
// Minimal JSON parser (schema-known bodies only).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum J {
    Str(String),
    Num(i128),
    Float(f64),
    Obj(Vec<(String, J)>),
    Arr(Vec<J>),
    Null,
}

impl J {
    /// Numeric value for an int|str union field: ints (and integral
    /// floats, which pydantic coerces) become Int; anything else stays.
    fn as_union_int(&self) -> Option<i128> {
        match self {
            J::Num(n) => Some(*n),
            J::Float(f) if f.fract() == 0.0 && f.is_finite() => Some(*f as i128),
            _ => None,
        }
    }
}

/// Python-json-compatible serializer (separators (",", ":"),
/// ensure_ascii) for echoing invalid inputs back exactly.
fn py_json(v: &J) -> String {
    match v {
        J::Str(s) => py_json_str(s),
        J::Num(n) => n.to_string(),
        J::Float(f) => py_float_repr(*f),
        J::Obj(o) => {
            let mut s = String::from("{");
            for (i, (k, vv)) in o.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                s.push_str(&py_json_str(k));
                s.push(':');
                s.push_str(&py_json(vv));
            }
            s.push('}');
            s
        }
        J::Arr(items) => {
            let mut s = String::from("[");
            for (i, vv) in items.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                s.push_str(&py_json(vv));
            }
            s.push(']');
            s
        }
        J::Null => "null".to_string(),
    }
}

fn py_json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\u{08}' => o.push_str("\\b"),
            '\u{0c}' => o.push_str("\\f"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c if (c as u32) < 0x7f => o.push(c),
            c => {
                let n = c as u32;
                if n > 0xffff {
                    let n = n - 0x10000;
                    o.push_str(&format!("\\u{:04x}\\u{:04x}", 0xd800 + (n >> 10), 0xdc00 + (n & 0x3ff)));
                } else {
                    o.push_str(&format!("\\u{:04x}", n));
                }
            }
        }
    }
    o.push('"');
    o
}

/// Python repr() for floats as emitted inside JSON bodies.
fn py_float_repr(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    if f.fract() == 0.0 && f.abs() < 1e17 {
        return format!("{}.0", f as i64);
    }
    let s = format!("{f:?}");
    match s.find('e') {
        Some(i) if !s[i + 1..].starts_with(['+', '-']) => format!("{}e+{}", &s[..i], &s[i + 1..]),
        _ => s,
    }
}

/// Python round(x, 2) rendered with str(): trailing zeros dropped.
fn py_float2(ms: f64) -> String {
    let r = (ms * 100.0).round() / 100.0;
    if r.fract() == 0.0 && r.abs() < 1e15 {
        return format!("{}.0", r as i64);
    }
    let s = format!("{r:.2}");
    s.trim_end_matches('0').to_string()
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

fn ws(p: &mut P) {
    while p.i < p.b.len() && matches!(p.b[p.i], b' ' | b'\t' | b'\n' | b'\r') {
        p.i += 1;
    }
}

fn parse_str(p: &mut P) -> Result<String, String> {
    if p.b.get(p.i) != Some(&b'"') {
        return Err("expected string".into());
    }
    p.i += 1;
    let mut o = String::new();
    let mut hi: Option<u32> = None;
    loop {
        let c = *p.b.get(p.i).ok_or("unterminated string")?;
        p.i += 1;
        match c {
            b'"' => {
                if hi.is_some() {
                    return Err("lone surrogate".into());
                }
                return Ok(o);
            }
            b'\\' => {
                let e = *p.b.get(p.i).ok_or("bad escape")?;
                p.i += 1;
                match e {
                    b'"' => o.push('"'),
                    b'\\' => o.push('\\'),
                    b'/' => o.push('/'),
                    b'n' => o.push('\n'),
                    b'r' => o.push('\r'),
                    b't' => o.push('\t'),
                    b'b' => o.push('\x08'),
                    b'f' => o.push('\x0c'),
                    b'u' => {
                        if p.i + 4 > p.b.len() {
                            return Err("bad \\u".into());
                        }
                        let h = std::str::from_utf8(&p.b[p.i..p.i + 4])
                            .map_err(|_| "bad \\u".to_string())?;
                        let u = u32::from_str_radix(h, 16).map_err(|_| "bad \\u".to_string())?;
                        p.i += 4;
                        if (0xD800..0xDC00).contains(&u) {
                            if hi.is_some() {
                                return Err("double hi surrogate".into());
                            }
                            hi = Some(u);
                            continue;
                        }
                        if (0xDC00..0xE000).contains(&u) {
                            match hi.take() {
                                Some(h) => {
                                    let c = 0x10000 + ((h - 0xD800) << 10) + (u - 0xDC00);
                                    o.push(char::from_u32(c).ok_or("bad scalar")?);
                                }
                                None => return Err("lone lo surrogate".into()),
                            }
                            continue;
                        }
                        if hi.is_some() {
                            return Err("lone hi surrogate".into());
                        }
                        o.push(char::from_u32(u).ok_or("bad scalar")?);
                    }
                    _ => return Err("bad escape".into()),
                }
            }
            _ => {
                if hi.is_some() {
                    return Err("lone surrogate".into());
                }
                // raw UTF-8 tail: decode one char
                let s = std::str::from_utf8(&p.b[p.i - 1..])
                    .map_err(|_| "bad utf8".to_string())?;
                let ch = s.chars().next().ok_or("bad utf8".to_string())?;
                o.push(ch);
                p.i += ch.len_utf8() - 1;
            }
        }
    }
}

fn parse_num(p: &mut P) -> Result<J, String> {
    let st = p.i;
    let neg = p.b.get(p.i) == Some(&b'-');
    if neg || p.b.get(p.i) == Some(&b'+') {
        p.i += 1;
    }
    // Infinity / NaN (Python's json accepts them on decode).
    for (word, val) in
        [("Infinity", f64::INFINITY), ("inf", f64::INFINITY), ("NaN", f64::NAN), ("nan", f64::NAN)]
    {
        if p.b[p.i..].starts_with(word.as_bytes()) {
            p.i += word.len();
            return Ok(J::Float(if neg { -val } else { val }));
        }
    }
    while p.i < p.b.len() && p.b[p.i].is_ascii_digit() {
        p.i += 1;
    }
    let mut is_float = false;
    if p.b.get(p.i) == Some(&b'.') {
        is_float = true;
        p.i += 1;
        while p.i < p.b.len() && p.b[p.i].is_ascii_digit() {
            p.i += 1;
        }
    }
    if matches!(p.b.get(p.i), Some(b'e') | Some(b'E')) {
        is_float = true;
        p.i += 1;
        if matches!(p.b.get(p.i), Some(b'-') | Some(b'+')) {
            p.i += 1;
        }
        while p.i < p.b.len() && p.b[p.i].is_ascii_digit() {
            p.i += 1;
        }
    }
    let s =
        std::str::from_utf8(&p.b[st..p.i]).map_err(|_| "bad number".to_string())?;
    if s.is_empty() || s == "-" || s == "+" {
        return Err("bad number".into());
    }
    if !is_float {
        if let Ok(n) = s.parse::<i128>() {
            return Ok(J::Num(n));
        }
    }
    s.parse::<f64>().map(J::Float).map_err(|_| "bad number".into())
}

fn parse_val(p: &mut P) -> Result<J, String> {
    ws(p);
    match p.b.get(p.i) {
        Some(b'"') => Ok(J::Str(parse_str(p)?)),
        Some(b'[') => parse_array(p),
        Some(b'{') => {
            p.i += 1;
            let mut o = Vec::new();
            ws(p);
            if p.b.get(p.i) == Some(&b'}') {
                p.i += 1;
                return Ok(J::Obj(o));
            }
            loop {
                ws(p);
                let k = parse_str(p)?;
                ws(p);
                if p.b.get(p.i) != Some(&b':') {
                    return Err("expected :".into());
                }
                p.i += 1;
                o.push((k, parse_val(p)?));
                ws(p);
                match p.b.get(p.i) {
                    Some(b',') => {
                        p.i += 1;
                    }
                    Some(b'}') => {
                        p.i += 1;
                        return Ok(J::Obj(o));
                    }
                    _ => return Err("expected , or }".into()),
                }
            }
        }
        Some(c) if *c == b'-' || c.is_ascii_digit() => parse_num(p),
        // Bare Infinity/NaN: invalid JSON, but Python's decoder accepts them.
        Some(b'I') if p.b[p.i..].starts_with(b"Infinity") => {
            p.i += 8;
            Ok(J::Float(f64::INFINITY))
        }
        Some(b'N') if p.b[p.i..].starts_with(b"NaN") => {
            p.i += 3;
            Ok(J::Float(f64::NAN))
        }
        Some(b'i') if p.b[p.i..].starts_with(b"inf") => {
            p.i += 3;
            Ok(J::Float(f64::INFINITY))
        }
        Some(b'n') if p.b[p.i..].starts_with(b"null") => {
            p.i += 4;
            Ok(J::Null)
        }
        Some(b'n') if p.b[p.i..].starts_with(b"nan") => {
            p.i += 3;
            Ok(J::Float(f64::NAN))
        }
        Some(b't') if p.b[p.i..].starts_with(b"true") => {
            p.i += 4;
            Ok(J::Num(1))
        }
        Some(b'f') if p.b[p.i..].starts_with(b"false") => {
            p.i += 5;
            Ok(J::Num(0))
        }
        _ => Err("unexpected value".into()),
    }
}

fn parse_array(p: &mut P) -> Result<J, String> {
    // JSON arrays (only needed for input echo shapes, e.g. text: []).
    p.i += 1;
    let mut items = Vec::new();
    ws(p);
    if p.b.get(p.i) == Some(&b']') {
        p.i += 1;
        return Ok(J::Arr(items));
    }
    loop {
        ws(p);
        items.push(parse_val(p)?);
        ws(p);
        match p.b.get(p.i) {
            Some(b',') => {
                p.i += 1;
            }
            Some(b']') => {
                p.i += 1;
                return Ok(J::Arr(items));
            }
            _ => return Err("expected , or ]".into()),
        }
    }
}

fn get<'a>(o: &'a [(String, J)], k: &str) -> Option<&'a J> {
    o.iter().find(|(kk, _)| kk == k).map(|(_, v)| v)
}

// ---------------------------------------------------------------------------
// Text pipeline (1:1 from renderer_service.py + prosody.py + api/tts.py).
// ---------------------------------------------------------------------------

fn pause_cmd(ms: u32) -> String {
    format!("\x1b\\pause=0\\\x1b\\mrk=21{:07}\\", ms)
}

fn is_opening(c: char) -> bool {
    matches!(c, '"' | '\'' | '(' | '[' | '{')
}
fn is_closing(c: char) -> bool {
    matches!(c, '"' | '\'' | ')' | ']' | '}')
}

fn word_before(text: &[char], index: usize) -> String {
    let before: String = text[..index].iter().collect();
    let t = before.trim_end_matches(|c: char| c.is_whitespace());
    if t.is_empty() {
        return String::new();
    }
    let last = t.split(|c: char| c.is_whitespace()).last().unwrap_or("");
    last.trim_matches(|c: char| matches!(c, '"' | '\'' | '(' | '[' | '{'))
        .to_string()
}

fn has_next_word(text: &[char], mut i: usize) -> bool {
    while i < text.len() && text[i].is_whitespace() {
        i += 1;
    }
    while i < text.len() && is_opening(text[i]) {
        i += 1;
    }
    i < text.len() && text[i].is_alphanumeric()
}

fn should_pause(text: &[char], start: usize, end: usize, mark: &str) -> bool {
    if !has_next_word(text, end + 1) {
        return false;
    }
    if mark == "." {
        let prev = if start > 0 { Some(text[start - 1]) } else { None };
        let next = text.get(end + 1).copied();
        if let (Some(a), Some(b)) = (prev, next) {
            if a.is_numeric() && b.is_numeric() {
                return false;
            }
        }
        let w = word_before(text, start).to_lowercase();
        let w = w.trim_end_matches('.');
        const ABBR: [&str; 10] =
            ["dr", "jr", "mr", "mrs", "ms", "mx", "prof", "sr", "st", "vs"];
        if ABBR.contains(&w) || w.chars().count() == 1 {
            return false;
        }
    }
    true
}

/// Port of `prosody.add_grammar_pauses`.
fn add_grammar_pauses(text: &str) -> String {
    // REPEATED_WHITESPACE_RE `\s+` -> " ", then strip.
    let mut norm = String::with_capacity(text.len());
    let mut was_ws = true; // leading strip
    for c in text.chars() {
        if c.is_whitespace() {
            if !was_ws {
                norm.push(' ');
                was_ws = true;
            }
        } else {
            norm.push(c);
            was_ws = false;
        }
    }
    if was_ws && !norm.is_empty() {
        norm.pop();
    }
    if norm.is_empty() || norm.contains('\x1b') {
        return norm;
    }
    let t: Vec<char> = norm.chars().collect();
    let mut out = String::new();
    let mut pauses = 0u32;
    let mut i = 0usize;
    while i < t.len() {
        let c = t[i];
        if matches!(c, '.' | '!' | '?') {
            let start = i;
            while i + 1 < t.len() && matches!(t[i + 1], '.' | '!' | '?') {
                i += 1;
            }
            let mark: String = t[start..=i].iter().collect();
            out.push_str(&mark);
            while i + 1 < t.len() && is_closing(t[i + 1]) {
                i += 1;
                out.push(t[i]);
            }
            if pauses < 24 && should_pause(&t, start, i, &mark) {
                let mut ms = if mark.contains("...") { 980 } else { 680 };
                if mark.contains('!') || mark.contains('?') {
                    ms = ms.max(780);
                }
                out.push_str(&pause_cmd(ms));
                pauses += 1;
            }
            i += 1;
            continue;
        }
        if matches!(c, ',' | ';' | ':') {
            out.push(c);
            let next = t.get(i + 1).copied().unwrap_or('\0');
            let prev = if i > 0 { t[i - 1] } else { '\0' };
            if pauses < 24
                && has_next_word(&t, i + 1)
                && !((c == ',' || c == ':') && prev.is_numeric() && next.is_numeric())
            {
                out.push_str(&pause_cmd(if c == ',' { 280 } else { 410 }));
                pauses += 1;
            }
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// `api/tts.py sendText` markup rewrite.
fn markup(s: &str) -> String {
    s.replace("<bleep>", "\x1b\\mrk=6\\")
        .replace("</bleep>", "\x1b\\mrk=7\\")
        .replace("<echo>", "\x1b\\mrk=4\\")
        .replace("</echo>", "\x1b\\mrk=5\\")
}

// ---------------------------------------------------------------------------
// WAV writer + gain (1:1 from api/tts.py convertDataToMp3 + audio.amplify_wav).
// ---------------------------------------------------------------------------

fn wav_bytes(samples: &[i16]) -> Vec<u8> {
    let n = (samples.len() * 2) as u32;
    let mut o = Vec::with_capacity(44 + samples.len() * 2);
    o.extend_from_slice(b"RIFF");
    o.extend_from_slice(&(36 + n).to_le_bytes());
    o.extend_from_slice(b"WAVE");
    o.extend_from_slice(b"fmt ");
    o.extend_from_slice(&16u32.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&16000u32.to_le_bytes());
    o.extend_from_slice(&32000u32.to_le_bytes());
    o.extend_from_slice(&2u16.to_le_bytes());
    o.extend_from_slice(&16u16.to_le_bytes());
    o.extend_from_slice(b"data");
    o.extend_from_slice(&n.to_le_bytes());
    for s in samples {
        o.extend_from_slice(&s.to_le_bytes());
    }
    o
}

fn amplify(wav: Vec<u8>, volume: i64) -> Vec<u8> {
    let gain = volume as f64 / 100.0;
    if (gain - 1.0).abs() < 0.01 {
        return wav;
    }
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return wav;
    }
    let mut o = wav[..44].to_vec();
    let frames = &wav[44..];
    let mut i = 0;
    while i + 2 <= frames.len() {
        let s = i16::from_le_bytes([frames[i], frames[i + 1]]) as f64;
        let mut b = (s * gain) as i64; // truncates toward zero, like int()
        if b > 32767 {
            b = 32767;
        } else if b < -32768 {
            b = -32768;
        }
        o.extend_from_slice(&(b as i16).to_le_bytes());
        i += 2;
    }
    o
}

// ---------------------------------------------------------------------------
// Sparse image store (lane us r21): the 338 MiB guest range is ~90% holes
// (measured: 8575 of 86589 pages carry template bytes; the engine writes
// ~100-260 pages per request). Four flat images cost ~1.45 GiB RSS.
// Instead: one memfd template file holds the covered pages; the snapshot
// is a MAP_SHARED read-only view and the arena a MAP_PRIVATE writable view
// of the same file, so read-only pages exist once in the page cache and
// are shared between the two mappings. Workspace/scratch are anonymous
// virtual-only mappings the engine never faults (tts-core stages single
// base-0 regions in place). Reset is one MADV_DONTNEED over the arena,
// which drops private copies and reverts to the pristine file pages.
// Untouched pages (holes, unread template, unused workspace) stay
// non-resident and cost no RSS. tts-core is untouched by this: it only
// ever sees plain &[u8]/&mut [u8] slices.
// ---------------------------------------------------------------------------

mod sparse {
    use std::ffi::c_void;
    use std::io;
    use std::os::unix::io::RawFd;
    use std::ptr;
    use std::slice;

    /// One anonymous or file-backed mapping. Moved to its lane thread; only
    /// the read-only snapshot view is shared between them.
    pub struct Mmap {
        ptr: *mut u8,
        len: usize,
    }
    unsafe impl Send for Mmap {}
    unsafe impl Sync for Mmap {}

    impl Mmap {
        fn check(ptr: *mut c_void, len: usize, what: &'static str) -> io::Result<Self> {
            if ptr == libc::MAP_FAILED {
                return Err(io::Error::new(io::ErrorKind::Other, what));
            }
            Ok(Self { ptr: ptr as *mut u8, len })
        }
        /// Anonymous virtual-only reservation. Stays non-resident until touched.
        pub fn anon(len: usize) -> io::Result<Self> {
            let len = len.div_ceil(0x1000) * 0x1000;
            let ptr = unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    len,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                    -1,
                    0,
                )
            };
            Self::check(ptr, len, "anon mmap")
        }
        /// Read-only shared view of `fd` (the snapshot: never written).
        pub fn shared_ro(fd: RawFd, len: usize) -> io::Result<Self> {
            let len = len.div_ceil(0x1000) * 0x1000;
            let ptr = unsafe {
                libc::mmap(ptr::null_mut(), len, libc::PROT_READ, libc::MAP_SHARED, fd, 0)
            };
            Self::check(ptr, len, "shared mmap")
        }
        /// Writable private view of `fd` (the arena: reads share file pages,
        /// writes copy on write).
        pub fn private_rw(fd: RawFd, len: usize) -> io::Result<Self> {
            let len = len.div_ceil(0x1000) * 0x1000;
            let ptr = unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    len,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE,
                    fd,
                    0,
                )
            };
            Self::check(ptr, len, "private mmap")
        }
        pub fn as_slice(&self) -> &[u8] {
            unsafe { slice::from_raw_parts(self.ptr, self.len) }
        }
        pub fn as_mut(&mut self) -> &mut [u8] {
            unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
        }
        /// Exact-size view for the engine (mappings are page-rounded up).
        pub fn slice(&self, len: usize) -> &[u8] {
            assert!(len <= self.len);
            &self.as_slice()[..len]
        }
        pub fn slice_mut(&mut self, len: usize) -> &mut [u8] {
            assert!(len <= self.len);
            &mut self.as_mut()[..len]
        }
        /// Drop all private copies: the mapping reverts to pristine file
        /// pages (or zero pages over file holes) on next access.
        pub fn reset(&self) {
            let rc = unsafe { libc::madvise(self.ptr as *mut c_void, self.len, libc::MADV_DONTNEED) };
            assert_eq!(rc, 0, "madvise DONTNEED");
        }
    }

    impl Drop for Mmap {
        fn drop(&mut self) {
            unsafe {
                libc::munmap(self.ptr as *mut c_void, self.len);
            }
        }
    }

    /// Sealed-at-rest template file: anonymous memfd, sized to the guest
    /// range, holding exactly the covered pages. Shared by the snapshot
    /// and arena mappings through the page cache.
    pub struct Template {
        fd: RawFd,
        pub size: usize,
    }

    impl Template {
        /// Raw fd for mapping. The mappings persist independently; the
        /// caller keeps the store alive (or forgets it) for clarity.
        pub fn fd_private(&self) -> RawFd {
            self.fd
        }
        pub fn create(size: usize) -> io::Result<Self> {
            let fd = unsafe {
                libc::memfd_create(c"tts-worker-tmpl".as_ptr(), 0)
            };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            let rc = unsafe { libc::ftruncate(fd, size as libc::off_t) };
            if rc != 0 {
                unsafe { libc::close(fd) };
                return Err(io::Error::last_os_error());
            }
            Ok(Self { fd, size })
        }
        /// Write one byte run at its guest offset. Partial writes are
        /// retried; holes are never written, so they stay holes (read as
        /// zero pages, cost nothing).
        pub fn write_at(&self, base: u64, bytes: &[u8]) -> io::Result<()> {
            let mut off = 0usize;
            while off < bytes.len() {
                let n = unsafe {
                    libc::pwrite(
                        self.fd,
                        bytes[off..].as_ptr() as *const c_void,
                        (bytes.len() - off) as libc::size_t,
                        base as libc::off_t + off as libc::off_t,
                    )
                };
                if n <= 0 {
                    return Err(io::Error::last_os_error());
                }
                off += n as usize;
            }
            Ok(())
        }
    }

    impl Drop for Template {
        fn drop(&mut self) {
            unsafe {
                libc::close(self.fd);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Minimal SHA-256 (FIPS 180-4) for cache keys (1:1 with voices.cache_key).
// ---------------------------------------------------------------------------

fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bitlen = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(22) ^ a.rotate_right(13);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = String::with_capacity(64);
    for v in h {
        out.push_str(&format!("{v:08x}"));
    }
    out
}

// ---------------------------------------------------------------------------
// Disk cache (1:1 with renderer_service._render_to_cache): sha256 key over
// version\0mode\0text\0fragment, atomic temp+rename writes, LRU prune.
// ---------------------------------------------------------------------------

/// cache_key(render_text, voice, mode, version), voices.cache_key shape.
fn cache_key(render_text: &str, v: &VoiceDone, mode: &str, version: &str) -> String {
    let fragment = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}",
        v.engine, v.lang, v.pitch, v.speed, v.quality, v.tone, v.accent, v.intonation, v.volume,
    );
    let payload = [version, mode, render_text, &fragment].join("\0");
    sha256_hex(payload.as_bytes())
}

/// 1:1 with ttsmodachi_bot/env.py: TTSMODACHI_ first (non-empty wins),
/// then the TALKMODACHI_ legacy alias, else None. Citra honors the legacy
/// prefix for every TTSMODACHI_ var, so the worker must too or cache keys
/// (engine version) and limits diverge in production.
fn tts_env(name: &str) -> Option<String> {
    for key in [format!("{name}"), format!("TALKMODACHI_{}", name.strip_prefix("TTSMODACHI_").unwrap_or(name))] {
        if let Ok(v) = std::env::var(&key) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

fn prune_cache(dir: &std::path::Path, max_bytes: u64) {
    if max_bytes == 0 {
        return;
    }
    let mut entries: Vec<(u128, u64, std::path::PathBuf)> = Vec::new();
    let mut total = 0u64;
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("wav") {
            continue;
        }
        let Ok(m) = e.metadata() else { continue };
        let mtime = m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        total += m.len();
        entries.push((mtime, m.len(), p));
    }
    if total <= max_bytes {
        return;
    }
    entries.sort();
    for (_, size, path) in entries {
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(size);
        }
        if total <= max_bytes {
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// Engine shelf: pristine template + serial synthesis on one big-stack thread.
// ---------------------------------------------------------------------------

struct HeapOs;
impl tts_core::engine::HeapPlatform for HeapOs {
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
    fn arbiter_address(&self) -> u32 {
        0xa7bb7c
    }
    fn arbitrate(&mut self, _: &mut tts_core::boot::Memory<'_>, _: &[u32]) -> u32 {
        panic!("contended heap locks are not bound")
    }
}

struct Nap;
impl tts_core::engine_live::SleepPlatform for Nap {
    fn sleep_ns(&mut self, _: u64) -> Result<(), tts_core::engine::Error> {
        Ok(())
    }
}

/// One pool render. Text the ARM engine finishes with no audio on (empty after
/// strip, fresh or carried, on the unicorn oracle) and lyric-less sing
/// (singText returns the empty WAV before the engine) skip the engine, but go
/// through a lane like citra's: they rotate the lanes and restart them the same.
enum UsJob {
    Speak { text16: Vec<u16>, vp: tts_core::engine_us::VoiceParams },
    NoAudio,
    EmptySing,
}

/// readRenderedAudio's test on the finished job (api/tts.py structDef
/// "BBBBBBBBBiIiBB", native alignment): audio only when audioSize (i32,
/// +12) > 0 and audioData (+16) != 0, else render_once raises
/// "Renderer returned no audio".
fn job_has_audio(arena: &[u8]) -> bool {
    let job = tts_core::engine_live_seg2::JOB_ADDR as usize;
    let word = |o: usize| {
        arena
            .get(job + o..job + o + 4)
            .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    (word(12) as i32) > 0 && word(16) != 0
}

/// One lane's engine. Fresh mappings are pristine: the arena already equals
/// the template (private view of the template file) and the workspace and
/// scratch mappings are untouched virtual reservations the engine never faults.
struct UsLane {
    arena: sparse::Mmap,
    snap: std::sync::Arc<sparse::Mmap>,
    workspace: sparse::Mmap,
    scratch: sparse::Mmap,
    size: usize,
    boot: tts_core::engine_us::BootArgs,
    warmup_block: u32,
    heap: HeapOs,
    nap: Nap,
    sink: Vec<i16>,
    out: Vec<i16>,
    /// citra's worker boots cold at startup and on every restart; the template
    /// is cold boot + the warm-up render, whose audioData the ARM abandoned.
    /// Each fresh engine frees that block first, and clears the voice-prepared
    /// flag after its first request (engine_us::unprepare_voice).
    fresh: bool,
}

impl pool::LaneEngine<UsJob> for UsLane {
    fn render_once(&mut self, job: &UsJob) -> Result<Vec<i16>, pool::RenderFailure> {
        let (text16, vp) = match job {
            UsJob::Speak { text16, vp } => (text16, vp),
            UsJob::NoAudio => return Err(pool::RenderFailure::NoAudio),
            UsJob::EmptySing => return Ok(Vec::new()),
        };
        let Self { arena, snap, workspace, scratch, size, boot, warmup_block, heap, nap, sink, out, fresh } =
            self;
        let (size, boot, warmup_block, first) = (*size, *boot, *warmup_block, *fresh);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut runtime = tts_core::engine_live::RuntimeM {
                backend: None,
                pcm: sink,
                written: 0,
                tick: boot.tick,
                registers: tts_core::engine::NotificationRegisters {
                    s16: boot.s16,
                    s17: boot.s17,
                },
                sleeper: nap,
            };
            let mut engine = tts_core::engine_us::UsEngine::new(
                boot,
                arena.slice_mut(size),
                snap.slice(size),
                workspace.slice_mut(size),
                scratch.slice_mut(size),
                heap,
                &mut runtime,
            );
            if first && warmup_block != 0 {
                tts_core::engine_us::guest_free(&mut engine, warmup_block)?;
            }
            let n = tts_core::engine_us::speak(&mut engine, text16, vp, out)?;
            if first {
                tts_core::engine_us::unprepare_voice(&mut engine);
            }
            Ok::<_, tts_core::engine::Error>(n)
        }));
        *fresh = false;
        match r {
            Ok(Ok(n)) if n > 0 && job_has_audio(arena.slice(size)) => Ok(out[..n].to_vec()),
            Ok(Ok(_)) => Err(pool::RenderFailure::NoAudio),
            Ok(Err(e)) => Err(pool::RenderFailure::Engine(format!("engine error: {e:?}"))),
            Err(_) => Err(pool::RenderFailure::Engine("engine panic".to_string())),
        }
    }

    /// Drop every private page so the arena reverts to the template file.
    /// Workspace/scratch are reset too, although the engine never faults
    /// them (in-place staging); MADV_DONTNEED on non-resident pages is free.
    fn restart(&mut self) {
        self.arena.reset();
        self.workspace.reset();
        self.scratch.reset();
        self.fresh = true;
    }
}

struct UsFactory {
    template_fd: std::os::unix::io::RawFd,
    size: usize,
    boot: tts_core::engine_us::BootArgs,
    snap: std::sync::Arc<sparse::Mmap>,
    warmup_block: u32,
}

impl pool::EngineFactory<UsJob> for UsFactory {
    fn start(&self) -> Result<Box<dyn pool::LaneEngine<UsJob>>, String> {
        let map = |what: &str, m: std::io::Result<sparse::Mmap>| m.map_err(|e| format!("{what}: {e}"));
        Ok(Box::new(UsLane {
            arena: map("arena map", sparse::Mmap::private_rw(self.template_fd, self.size))?,
            snap: self.snap.clone(),
            // virtual-only reservations: the engine stages single base-0
            // regions in place, these only satisfy the capacity contract
            workspace: map("workspace map", sparse::Mmap::anon(self.size))?,
            scratch: map("scratch map", sparse::Mmap::anon(self.size))?,
            size: self.size,
            boot: self.boot,
            warmup_block: self.warmup_block,
            heap: HeapOs,
            nap: Nap,
            sink: vec![0i16; 4_000_000],
            out: vec![0i16; 4_000_000],
            fresh: true,
        }))
    }
}

// ---------------------------------------------------------------------------
// HTTP plumbing (std only, Connection: close).
// ---------------------------------------------------------------------------

fn reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        307 => "Temporary Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

/// Byte-exact response writer. Header order mirrors the citra worker
/// (Starlette): custom headers first in the given order, then
/// Content-Type/Disposition, then Content-Length. HEAD responses carry
/// the same headers with no body.
/// IMF-fixdate Date header (what uvicorn sends on every response).
fn http_date() -> String {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] =
        ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86400) as i64;
    let tod = (secs % 86400) as i64;
    // Howard Hinnant's civil-from-days.
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as i64;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as usize;
    let y = (if m <= 2 { y + 1 } else { y }) as i64;
    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        DAYS[days.rem_euclid(7) as usize],
        d,
        MONTHS[m - 1],
        y,
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60,
    )
}

fn respond(
    s: &mut std::net::TcpStream,
    code: u16,
    ctype: &str,
    extra: &[(&str, &str)],
    disposition: bool,
    body: &[u8],
    head_only: bool,
) {
    let mut h = format!("HTTP/1.1 {} {}\r\n", code, reason(code));
    h.push_str(&format!("date: {}\r\nserver: uvicorn\r\n", http_date()));
    for (k, v) in extra {
        h.push_str(&format!("{k}: {v}\r\n"));
    }
    h.push_str(&format!("Content-Type: {ctype}\r\n"));
    if disposition {
        h.push_str("Content-Disposition: attachment; filename=\"speech.wav\"\r\n");
    }
    h.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n", body.len()));
    let _ = s.write_all(h.as_bytes());
    if !head_only {
        let _ = s.write_all(body);
    }
}

/// Error responses: Content-Length then Content-Type (Starlette order).
fn respond_err(s: &mut std::net::TcpStream, code: u16, body: &[u8], head_only: bool) {
    let h = format!(
        "HTTP/1.1 {} {}\r\ndate: {}\r\nserver: uvicorn\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
        code,
        reason(code),
        http_date(),
        body.len()
    );
    let _ = s.write_all(h.as_bytes());
    if !head_only {
        let _ = s.write_all(body);
    }
}

fn err_detail(s: &mut std::net::TcpStream, code: u16, msg: &str, head_only: bool) {
    let b = format!("{{\"detail\":{}}}", py_json_str(msg));
    respond_err(s, code, b.as_bytes(), head_only);
}

fn redirect(s: &mut std::net::TcpStream, location: &str, head_only: bool) {
    let h = format!(
        "HTTP/1.1 307 {}\r\ndate: {}\r\nserver: uvicorn\r\nContent-Length: 0\r\nLocation: {location}\r\nConnection: close\r\n\r\n",
        reason(307),
        http_date(),
    );
    let _ = s.write_all(h.as_bytes());
    let _ = head_only;
}

fn read_request(
    s: &mut std::net::TcpStream,
) -> Result<(String, String, Vec<u8>, String), String> {
    let mut buf = vec![0u8; 65536];
    let mut got = 0usize;
    let hdr_end = loop {
        let n = s.read(&mut buf[got..]).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("eof".into());
        }
        got += n;
        if let Some(p) = find_sub(&buf[..got], b"\r\n\r\n") {
            break p + 4;
        }
        if got == buf.len() {
            return Err("headers too big".into());
        }
    };
    let head = String::from_utf8_lossy(&buf[..hdr_end]).into_owned();
    let mut lines = head.lines();
    let rl = lines.next().ok_or("no request line")?;
    let mut sp = rl.split_whitespace();
    let method = sp.next().ok_or("no method")?.to_string();
    let path = sp.next().ok_or("no path")?.to_string();
    let mut len = 0usize;
    let mut host = String::new();
    for l in lines {
        if l.is_empty() {
            break;
        }
        if let Some(v) = l.strip_prefix("Content-Length:").or(l.strip_prefix("content-length:"))
        {
            len = v.trim().parse::<usize>().map_err(|_| "bad length".to_string())?;
        }
        if let Some(v) = l.strip_prefix("Host:").or(l.strip_prefix("host:")) {
            host = v.trim().to_string();
        }
    }
    if len > 4 * 1024 * 1024 {
        return Err("body too big".into());
    }
    let mut body = vec![0u8; len];
    let buffered = got - hdr_end;
    let take = buffered.min(len);
    body[..take].copy_from_slice(&buf[hdr_end..hdr_end + take]);
    let mut have = take;
    while have < len {
        let n = s.read(&mut body[have..]).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("eof in body".into());
        }
        have += n;
    }
    Ok((method, path, body, host))
}

fn find_sub(hay: &[u8], ndl: &[u8]) -> Option<usize> {
    hay.windows(ndl.len()).position(|w| w == ndl)
}

/// Parsedint|str union field: validated ints stay numeric, validated
/// strings are converted with Python int() semantics in from_mapping.
#[derive(Debug, Clone)]
enum NumField {
    Int(i128),
    Str(String),
}

struct Voice {
    pitch: NumField,
    speed: NumField,
    quality: NumField,
    tone: NumField,
    accent: NumField,
    intonation: NumField,
    lang: NumField,
    volume: NumField,
    engine: NumField,
}

impl Voice {
    fn defaults() -> Self {
        Self {
            pitch: NumField::Int(50),
            speed: NumField::Int(50),
            quality: NumField::Int(50),
            tone: NumField::Int(50),
            accent: NumField::Int(50),
            intonation: NumField::Int(1),
            lang: NumField::Str("useng".to_string()),
            volume: NumField::Int(200),
            engine: NumField::Str("tl3ds".to_string()),
        }
    }
}

struct RenderReq {
    text: String,
    voice: VoiceDone,
    mode: String,
    backend: String,
}

/// Validated, coerced voice values (from_mapping + coerce_supported_voice).
struct VoiceDone {
    pitch: i128,
    speed: i128,
    quality: i128,
    tone: i128,
    accent: i128,
    intonation: i128,
    lang: String,
    volume: i128,
    engine: String,
}

const LANG_TO_ID: &[&str] =
    &["useng", "eueng", "fr", "de", "it", "es", "jp", "kr"];

/// Literal field with a default when absent; exact 422 shape otherwise.
fn literal_field(
    top: &[(String, J)],
    key: &str,
    dflt: &str,
    allowed: &[&str],
) -> Result<String, RenderErr> {
    match get(top, key) {
        None => Ok(dflt.to_string()),
        Some(J::Str(s)) if allowed.contains(&s.as_str()) => Ok(s.clone()),
        Some(other) => {
            let input = py_json(other);
            let expected = allowed.iter().map(|a| format!("'{a}'")).collect::<Vec<_>>().join(" or ");
            let mut e = String::from("{\"type\":\"literal_error\",\"loc\":[\"body\",\"");
            e.push_str(key);
            e.push_str("\"],\"msg\":\"Input should be ");
            e.push_str(&expected);
            e.push_str("\",\"input\":");
            e.push_str(&input);
            e.push_str(",\"ctx\":{\"expected\":\"");
            e.push_str(&expected);
            e.push_str("\"}}");
            Err(RenderErr::errors(&[e]))
        }
    }
}

/// Malformed JSON body: 422 json_invalid. The ctx error text is
/// best-effort (CPython's messages are not reproduced verbatim).
fn json_invalid(body: &[u8]) -> RenderErr {
    let preview: String = body.iter().take(32).map(|b| *b as char).collect();
    let msg = format!("Expecting value: line 1 column 1 (char 0), got {preview:?}");
    RenderErr::errors(&[format!(
        "{{\"type\":\"json_invalid\",\"loc\":[\"body\",1],\"msg\":\"JSON decode error\",\"input\":{{}},\"ctx\":{{\"error\":{}}}}}",
        py_json_str(&msg),
    )])
}

/// Error with an exact response body (422 FastAPI bodies are byte-exact).
struct RenderErr {
    code: u16,
    body: Vec<u8>,
}

impl RenderErr {
    fn detail(code: u16, msg: String) -> Self {
        let body = format!("{{\"detail\":{}}}", py_json_str(&msg)).into_bytes();
        Self { code, body }
    }
    fn errors(errs: &[String]) -> Self {
        let body = format!("{{\"detail\":[{}]}}", errs.join(",")).into_bytes();
        Self { code: 422, body }
    }
}

/// One int|str union field validation, 1:1 with pydantic's error shapes.
/// `path` is the field name (loc tail); `input` is the raw JSON value.
fn union_field(obj: &[(String, J)], key: &str, dflt: NumField, errs: &mut Vec<String>) -> NumField {
    let v = match get(obj, key) {
        None => return dflt,
        Some(v) => v,
    };
    if let Some(n) = v.as_union_int() {
        // bool true/false arrive as Num(1)/Num(0): pydantic's int branch
        // accepts them, so they validate as ints.
        return NumField::Int(n);
    }
    if let J::Str(s) = v {
        return NumField::Str(s.clone());
    }
    let input = py_json(v);
    let loc = format!("\"body\",\"voice\",\"{key}\"");
    let int_err = match v {
        J::Float(f) if f.is_finite() && f.fract() != 0.0 => format!(
            "{{\"type\":\"int_from_float\",\"loc\":[{loc},\"int\"],\"msg\":\"Input should be a valid integer, got a number with a fractional part\",\"input\":{input}}}",
        ),
        _ => format!(
            "{{\"type\":\"int_type\",\"loc\":[{loc},\"int\"],\"msg\":\"Input should be a valid integer\",\"input\":{input}}}",
        ),
    };
    let str_err = format!(
        "{{\"type\":\"string_type\",\"loc\":[{loc},\"str\"],\"msg\":\"Input should be a valid string\",\"input\":{input}}}",
    );
    errs.push(int_err);
    errs.push(str_err);
    dflt
}

/// Python repr() for str (single quotes unless the value holds one).
fn py_repr_str(s: &str) -> String {
    if s.contains('\'') && !s.contains('"') {
        format!("\"{s}\"")
    } else {
        format!("'{s}'")
    }
}

/// Python int(str, 10) semantics: optional whitespace, sign, digits with
/// single underscores between them. Returns the value or the exact
/// ValueError message text citra surfaces.
fn py_int(s: &str) -> Result<i128, String> {
    let t = s.trim_matches(|c: char| c.is_whitespace() && (c as u32) < 0x80);
    let t = t.strip_prefix('+').unwrap_or(t);
    let neg = t.starts_with('-');
    let t = if neg { &t[1..] } else { t };
    if t.is_empty() {
        return Err(format!("invalid literal for int() with base 10: {}", py_repr_str(s)));
    }
    let mut digits = String::with_capacity(t.len());
    let mut last_underscore = true;
    for c in t.chars() {
        if c == '_' {
            if last_underscore {
                return Err(format!("invalid literal for int() with base 10: {}", py_repr_str(s)));
            }
            last_underscore = true;
            continue;
        }
        if !c.is_ascii_digit() {
            return Err(format!("invalid literal for int() with base 10: {}", py_repr_str(s)));
        }
        last_underscore = false;
        digits.push(c);
    }
    if last_underscore {
        return Err(format!("invalid literal for int() with base 10: {}", py_repr_str(s)));
    }
    let n: i128 = digits.parse().map_err(|_| format!("invalid literal for int() with base 10: {}", py_repr_str(s)))?;
    Ok(if neg { -n } else { n })
}

fn parse_render(body: &[u8]) -> Result<RenderReq, RenderErr> {
    // Empty body: FastAPI reports the model itself missing.
    if body.is_empty() {
        return Err(RenderErr::errors(&["{\"type\":\"missing\",\"loc\":[\"body\"],\"msg\":\"Field required\",\"input\":null}".to_string()]));
    }
    let mut p = P { b: body, i: 0 };
    let v = match parse_val(&mut p) {
        Ok(v) => {
            let mut q = p;
            ws(&mut q);
            if q.i != q.b.len() {
                return Err(json_invalid(body));
            }
            v
        }
        Err(_) => return Err(json_invalid(body)),
    };
    let J::Obj(top) = &v else {
        let input = py_json(&v);
        return Err(RenderErr::errors(&[format!(
            "{{\"type\":\"model_attributes_type\",\"loc\":[\"body\"],\"msg\":\"Input should be a valid dictionary or object to extract fields from\",\"input\":{input}}}",
        )]));
    };
    // text: required str, min_length 1.
    let text = match get(top, "text") {
        None => {
            return Err(RenderErr::errors(&[format!(
                "{{\"type\":\"missing\",\"loc\":[\"body\",\"text\"],\"msg\":\"Field required\",\"input\":{}}}",
                py_json(&v),
            )]));
        }
        Some(J::Str(t)) => {
            if t.is_empty() {
                return Err(RenderErr::errors(&["{\"type\":\"string_too_short\",\"loc\":[\"body\",\"text\"],\"msg\":\"String should have at least 1 character\",\"input\":\"\",\"ctx\":{\"min_length\":1}}".to_string()]));
            }
            t.clone()
        }
        Some(other) => {
            let input = py_json(other);
            return Err(RenderErr::errors(&[format!(
                "{{\"type\":\"string_type\",\"loc\":[\"body\",\"text\"],\"msg\":\"Input should be a valid string\",\"input\":{input}}}",
            )]));
        }
    };
    // mode / backend: Literals with defaults, exact error shapes.
    let mode = literal_field(top, "mode", "text", &["text", "sing"])?;
    let backend =
        literal_field(top, "backend", "citra", &["citra", "standalone"])?;
    // voice: int|str union fields validated first (422 shapes), then
    // from_mapping value checks (400 messages), in mapping order.
    let mut vce = Voice::defaults();
    let mut errs: Vec<String> = Vec::new();
    if let Some(vv) = get(top, "voice") {
        match vv {
            J::Obj(o) => {
                // Dataclass field order, matching pydantic's error order.
                vce.engine = union_field(o, "engine", Voice::defaults().engine, &mut errs);
                vce.pitch = union_field(o, "pitch", Voice::defaults().pitch, &mut errs);
                vce.speed = union_field(o, "speed", Voice::defaults().speed, &mut errs);
                vce.quality = union_field(o, "quality", Voice::defaults().quality, &mut errs);
                vce.tone = union_field(o, "tone", Voice::defaults().tone, &mut errs);
                vce.accent = union_field(o, "accent", Voice::defaults().accent, &mut errs);
                vce.intonation =
                    union_field(o, "intonation", Voice::defaults().intonation, &mut errs);
                vce.lang = union_field(o, "lang", Voice::defaults().lang, &mut errs);
                vce.volume = union_field(o, "volume", Voice::defaults().volume, &mut errs);
            }
            other => {
                let input = py_json(other);
                errs.push(format!(
                    "{{\"type\":\"dict_type\",\"loc\":[\"body\",\"voice\"],\"msg\":\"Input should be a valid dictionary\",\"input\":{input}}}",
                ));
            }
        }
    }
    if !errs.is_empty() {
        return Err(RenderErr::errors(&errs));
    }
    // from_mapping: str values go through Python int(); bad ones 400.
    let num = |f: &NumField| -> Result<i128, RenderErr> {
        match f {
            NumField::Int(n) => Ok(*n),
            NumField::Str(s) => py_int(s).map_err(|m| RenderErr::detail(400, m)),
        }
    };
    let pitch = num(&vce.pitch)?;
    let speed = num(&vce.speed)?;
    let quality = num(&vce.quality)?;
    let tone = num(&vce.tone)?;
    let accent = num(&vce.accent)?;
    let intonation = num(&vce.intonation)?;
    let volume = num(&vce.volume)?;
    let bad: Vec<&str> = [("pitch", pitch), ("speed", speed), ("quality", quality), ("tone", tone), ("accent", accent)]
        .into_iter()
        .filter(|(_, v)| !(0..=100).contains(v))
        .map(|(k, _)| k)
        .collect();
    if !bad.is_empty() {
        return Err(RenderErr::detail(400, format!("Voice parameter out of range: {}", bad.join(", "))));
    }
    if !(25..=300).contains(&volume) {
        return Err(RenderErr::detail(400, "Volume must be between 25 and 300".to_string()));
    }
    if ![1, 2, 3, 4].contains(&intonation) {
        return Err(RenderErr::detail(400, "Intonation must be 1, 2, 3, or 4".to_string()));
    }
    // Coerce to the pool's ROMs (renderer_service.coerce_supported_voice;
    // this worker is US-only so everything non-useng renders as useng).
    // Ints never match a language/engine id: same 400s as citra.
    let lang = match &vce.lang {
        NumField::Int(n) => return Err(RenderErr::detail(400, format!("Unsupported language: {n}"))),
        NumField::Str(s) => {
            if !LANG_TO_ID.contains(&s.as_str()) {
                return Err(RenderErr::detail(400, format!("Unsupported language: {s}")));
            }
            // US-only pool: coerce everything to useng, like citra.
            "useng".to_string()
        }
    };
    let engine = match &vce.engine {
        NumField::Int(n) => return Err(RenderErr::detail(400, format!("Unsupported engine: {n}"))),
        NumField::Str(s) => {
            if s != "tl3ds" {
                return Err(RenderErr::detail(400, format!("Unsupported engine: {s}")));
            }
            s.clone()
        }
    };
    Ok(RenderReq { text, voice: VoiceDone { pitch, speed, quality, tone, accent, intonation, lang, volume, engine }, mode, backend })
}

// ---------------------------------------------------------------------------
// Drop-in state: lane pool, disk cache, inflight dedup, health.
// Mirrors renderer_service on a US-only worker.
// ---------------------------------------------------------------------------

/// One render as renderer_service hands it around: the amplified WAV, the
/// lane's elapsed ms and what the X-Cache header starts from.
#[derive(Clone)]
struct Served {
    wav: Vec<u8>,
    elapsed: String,
    cache: &'static str,
}

/// What every joiner of one render gets, the failure as the pool worded it.
type SynthResult = Result<Served, String>;

struct Inflight {
    done: std::sync::Mutex<bool>,
    cvar: std::sync::Condvar,
    result: std::sync::Mutex<Option<SynthResult>>,
}

struct Shared {
    pool: std::sync::Arc<pool::RendererPool<UsJob>>,
    executor: pool::ExecutorSlots,
    max_inflight: usize,
    gain_pct: i64,
    grammar: bool,
    port: u16,
    engine_version: String,
    cache_dir: std::path::PathBuf,
    cache_max_bytes: u64,
    prune_interval_secs: u64,
    prune_next: std::sync::Mutex<std::time::Instant>,
    tmp_counter: std::sync::atomic::AtomicU64,
    inflight: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<Inflight>>>,
}

fn effective_volume(volume: i128, gain_pct: i64) -> i64 {
    ((volume as f64 * gain_pct as f64 / 100.0).round() as i64).clamp(25, 300)
}

fn health_body(sh: &Shared) -> Vec<u8> {
    let pid = std::process::id();
    let h = sh.pool.health();
    let float = |v: Option<f64>| v.map_or_else(|| "null".to_string(), py_float2);
    let text = |v: &Option<String>| v.as_deref().map_or_else(|| "null".to_string(), py_json_str);
    let workers: Vec<String> = h
        .workers
        .iter()
        .map(|w| {
            let pid = if w.running { pid.to_string() } else { "null".to_string() };
            format!(
                "{{\"name\":{},\"rom\":{},\"port\":{},\"pid\":{pid},\"worker_pid\":{pid},\"citra_pid\":null,\"alive\":{},\"ready\":{},\"paused\":{},\"active_job_count\":{},\"idle_seconds\":{},\"last_render_ms\":{},\"resume_count\":{},\"restart_count\":{},\"render_retry_count\":{},\"last_recovery_reason\":{},\"last_error\":{}}}",
                py_json_str(&w.name),
                py_json_str(&w.rom),
                sh.port,
                w.alive,
                w.ready,
                w.paused,
                w.active_job_count,
                float(w.idle_seconds),
                float(w.last_render_ms),
                w.resume_count,
                w.restart_count,
                w.render_retry_count,
                text(&w.last_recovery_reason),
                text(&w.last_error),
            )
        })
        .collect();
    let rom = py_json_str(&h.rom);
    format!(
        "{{\"ok\":true,\"cache_dir\":{},\"pool\":{{\"autoscale\":{{\"enabled\":{},\"scale_down_idle_seconds\":{},\"min_active_by_rom\":{{{rom}:{}}},\"running_by_rom\":{{{rom}:{}}},\"total_running\":{},\"total_configured\":{}}},\"workers\":[{}]}}}}",
        py_json_str(&sh.cache_dir.to_string_lossy()),
        h.autoscale,
        py_float_repr(h.scale_down_idle),
        h.min_active,
        h.running,
        h.running,
        h.configured,
        workers.join(","),
    )
    .into_bytes()
}

fn handle(mut s: std::net::TcpStream, sh: std::sync::Arc<Shared>) {
    let (method, raw_path, body, host) = match read_request(&mut s) {
        Ok(r) => r,
        Err(_) => return,
    };
    let (path, query) = match raw_path.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (raw_path.as_str(), None),
    };
    let head_only = method == "HEAD";
    // Starlette redirect_slashes: trailing slash redirects with an
    // absolute Location (Host echo), preserving the query string.
    if path == "/render/" {
        let mut location = if host.is_empty() {
            "/render".to_string()
        } else {
            format!("http://{host}/render")
        };
        if let Some(q) = query {
            location.push('?');
            location.push_str(q);
        }
        redirect(&mut s, &location, head_only);
        return;
    }
    if path == "/render" {
        if method != "POST" {
            respond_err(&mut s, 405, b"{\"detail\":\"Method Not Allowed\"}", head_only);
            return;
        }
        render_flow(&mut s, &sh, &body, head_only);
        return;
    }
    if path == "/pool/render" {
        if method != "POST" {
            respond_err(&mut s, 405, b"{\"detail\":\"Method Not Allowed\"}", head_only);
            return;
        }
        pool_render_flow(&mut s, &sh, &body, head_only);
        return;
    }
    if path == "/health" {
        if method != "GET" {
            respond_err(&mut s, 405, b"{\"detail\":\"Method Not Allowed\"}", head_only);
            return;
        }
        let b = health_body(&sh);
        respond(&mut s, 200, "application/json", &[], false, &b, head_only);
        return;
    }
    respond_err(&mut s, 404, b"{\"detail\":\"Not Found\"}", head_only);
}

/// Python `str.strip()` (no-args) code-point set: Rust's
/// `char::is_whitespace` (Unicode White_Space) plus U+001C..=U+001F,
/// which Python's `str.isspace` covers and White_Space does not.
/// (U+0085 is White_Space, so `is_whitespace` already covers it.)
fn is_py_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Serving pre-strip with Python `str.strip()` semantics
/// (`renderer_service`: `text.replace("\n", " ").strip()`). A
/// whitespace-only request must reach the empty mapping as `""` on
/// both workers, or the engine sees different inputs.
fn py_strip(s: &str) -> &str {
    s.trim_matches(is_py_space)
}

fn find_byte(hay: &[u8], from: usize, needle: u8) -> Option<usize> {
    let mut i = from;
    while i < hay.len() {
        if hay[i] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_bytes(hay: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len().saturating_sub(from) {
        return None;
    }
    let mut i = from;
    while i + needle.len() <= hay.len() {
        if &hay[i..i + needle.len()] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// 1:1 with `songConverter.parseSong` emptiness. `SongParser` (plain
/// `html.parser.HTMLParser`) appends one lyric per `</lyric>` end tag;
/// nothing else creates entries. End-tag rule, probed against the
/// worker image: `</`, optional Python-whitespace, an ASCII-letter tag
/// of `[A-Za-z][^\t\n\r\f />]*`, optional junk, committed only when a
/// `>` exists later; the tag is lowercased. `<!--...-->` comments
/// (unclosed: through the next `>`), `<!...>`/`<?...>` decls and
/// `<tag ...>` start tags (through the next `>`) swallow their
/// contents, exactly like the parser.
fn has_lyrics(text: &str) -> bool {
    let b = text.as_bytes();
    let n = b.len();
    let mut i = 0;
    while i < n {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        if i + 4 <= n && &b[i..i + 4] == b"<!--" {
            if let Some(k) = find_bytes(b, i + 4, b"-->") {
                i = k + 3;
            } else if let Some(k) = find_byte(b, i + 4, b'>') {
                i = k + 1;
            } else {
                break;
            }
            continue;
        }
        if i + 2 <= n && &b[i..i + 2] == b"</" {
            let mut skipped = 0;
            for c in text[i + 2..].chars() {
                if is_py_space(c) {
                    skipped += c.len_utf8();
                } else {
                    break;
                }
            }
            let s = i + 2 + skipped;
            if s < n && b[s].is_ascii_alphabetic() {
                let mut j = s + 1;
                while j < n && !matches!(b[j], b'\t' | b'\n' | b'\x0c' | b'\r' | b' ' | b'/' | b'>') {
                    j += 1;
                }
                // HTMLParser only commits the end tag when a `>`
                // exists later; otherwise the `</` is plain data.
                if find_byte(b, i + 1, b'>').is_none() {
                    i += 2;
                    continue;
                }
                if text[s..j].eq_ignore_ascii_case("lyric") {
                    return true;
                }
                // Junk after a non-lyric tag holds no `<`, so
                // resuming at the tag end misses nothing.
                i = j;
                continue;
            }
            i += 2;
            continue;
        }
        if i + 2 <= n && (b[i + 1] == b'!' || b[i + 1] == b'?') {
            if let Some(k) = find_byte(b, i + 2, b'>') {
                i = k + 1;
            } else {
                break;
            }
            continue;
        }
        if i + 1 < n && b[i + 1].is_ascii_alphabetic() {
            if let Some(k) = find_byte(b, i + 1, b'>') {
                i = k + 1;
            } else {
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    false
}

/// Citra's exact steady-state body when the engine finishes with no
/// audio (empty-after-strip text): the worker-level no-audio 500.
/// Deterministic across containers and days (no PIDs, addresses or
/// timestamps); pinned to worker image 698e47b27009. Served 1:1 so the
/// API battery matches byte-exact; the same class of serving-contract
/// string as the already-matched 400/422 messages. The pool builds it now
/// (pool::NO_AUDIO_TRACEBACK per lane); this pins the one-lane shape.
#[cfg(test)]
const NO_AUDIO_BODY: &str = r#"{"detail":"All renderer workers failed for ROM US: US-1: Traceback (most recent call last):\n  File \"/app/ttsmodachi_bot/renderer_pool.py\", line 190, in render_with_recovery\n    return render_once(payload, voice)\n  File \"/app/ttsmodachi_bot/renderer_pool.py\", line 184, in render_once\n    raise RuntimeError(\"Renderer returned no audio\")\nRuntimeError: Renderer returned no audio\n\nDuring handling of the above exception, another exception occurred:\n\nTraceback (most recent call last):\n  File \"/app/ttsmodachi_bot/renderer_pool.py\", line 257, in _worker_loop\n    audio = render_with_recovery(payload, voice)\n  File \"/app/ttsmodachi_bot/renderer_pool.py\", line 197, in render_with_recovery\n    return render_once(payload, voice)\n  File \"/app/ttsmodachi_bot/renderer_pool.py\", line 184, in render_once\n    raise RuntimeError(\"Renderer returned no audio\")\nRuntimeError: Renderer returned no audio\n"}"#;

fn render_flow(s: &mut std::net::TcpStream, sh: &Shared, body: &[u8], head_only: bool) {
    let req = match parse_render(body) {
        Ok(r) => r,
        Err(e) => {
            respond_err(s, e.code, &e.body, head_only);
            return;
        }
    };
    let v = req.voice;
    // renderer_service: newline -> space, strip, per-language limit.
    // The strip is Python `str.strip()` semantics (see py_strip).
    let pre = req.text.replace('\n', " ");
    let pre = py_strip(&pre);
    let limit = if v.lang == "jp" { 1024 } else { 2000 };
    if pre.chars().count() > limit {
        err_detail(s, 400, &format!("Text is longer than {limit} characters"), head_only);
        return;
    }
    // Sing with no lyrics never reaches the engine on citra either:
    // singText parses the text and, with zero `</lyric>` entries,
    // returns the empty WAV straight to the cache path. Singing itself
    // stays unported: lyric-bearing sing fails loud like any open stage.
    let sing_empty = req.mode == "sing" && !has_lyrics(pre);
    if req.mode == "sing" && !sing_empty {
        err_detail(s, 500, "engine error: sing mode is not supported by this worker", head_only);
        return;
    }
    let render_text =
        if sh.grammar && req.mode == "text" { add_grammar_pauses(pre) } else { pre.to_string() };
    // Standalone is never available (no snapshot manifest, like production),
    // so the citra backend always renders: X-Backend stays "citra".
    // Structure mirrors renderer_service (backend_tag is always empty here).
    let standalone_available = false;
    let use_standalone = req.backend == "standalone"
        && standalone_available
        && req.mode == "text"
        && v.lang == "useng";
    let backend_tag = if use_standalone { ":standalone" } else { "" };
    let version = format!("{}:gain{}:grammarpauses1{backend_tag}", sh.engine_version, sh.gain_pct);
    let key = cache_key(&render_text, &v, &req.mode, &version);
    let cache_path = sh.cache_dir.join(format!("{key}.wav"));
    if let Ok(wav) = std::fs::read(&cache_path) {
        respond(
            s,
            200,
            "audio/wav",
            &[("X-Cache", "HIT"), ("X-Backend", "citra")],
            true,
            &wav,
            head_only,
        );
        return;
    }
    // Inflight dedup (same key joins) and queue cap (distinct keys).
    let (entry, created) = {
        let mut map = sh.inflight.lock().unwrap();
        if let Some(e) = map.get(&key) {
            (e.clone(), false)
        } else {
            if map.len() >= sh.max_inflight {
                err_detail(s, 429, "Renderer queue is full", head_only);
                return;
            }
            let e = std::sync::Arc::new(Inflight {
                done: std::sync::Mutex::new(false),
                cvar: std::sync::Condvar::new(),
                result: std::sync::Mutex::new(None),
            });
            map.insert(key.clone(), e.clone());
            (e, true)
        }
    };
    let outcome = if created {
        let job = us_job(&req.mode, pre, &render_text, sing_empty, &v);
        let outcome = render_to_cache(sh, &cache_path, job, v.volume);
        finish_inflight(sh, &key, &entry, outcome.clone());
        outcome
    } else {
        let guard = entry.done.lock().unwrap();
        let _guard = entry.cvar.wait_while(guard, |d| !*d).unwrap();
        let result = entry.result.lock().unwrap().clone();
        result.expect("inflight result")
    };
    match outcome {
        Ok(served) => {
            let cache = if served.cache == "MISS" && !created { "DEDUPED" } else { served.cache };
            respond(
                s,
                200,
                "audio/wav",
                &[("X-Cache", cache), ("X-Render-Time-Ms", &served.elapsed), ("X-Backend", "citra")],
                true,
                &served.wav,
                head_only,
            );
        }
        Err(detail) => err_detail(s, 500, &detail, head_only),
    }
}

fn us_job(mode: &str, pre: &str, render_text: &str, sing_empty: bool, v: &VoiceDone) -> UsJob {
    if sing_empty {
        return UsJob::EmptySing;
    }
    if mode == "text" && pre.is_empty() {
        return UsJob::NoAudio;
    }
    UsJob::Speak {
        text16: markup(render_text).encode_utf16().collect(),
        vp: tts_core::engine_us::VoiceParams {
            pitch: v.pitch as u8,
            speed: v.speed as u8,
            quality: v.quality as u8,
            tone: v.tone as u8,
            accent: v.accent as u8,
            intonation: (v.intonation - 1) as u8,
            language: 1,
        },
    }
}

/// `_render_to_cache`.
fn render_to_cache(sh: &Shared, cache_path: &std::path::Path, job: UsJob, volume: i128) -> SynthResult {
    if let Ok(wav) = std::fs::read(cache_path) {
        return Ok(Served { wav, elapsed: String::new(), cache: "HIT" });
    }
    let rendered = {
        let _slot = sh.executor.acquire();
        sh.pool.render(std::sync::Arc::new(job))?
    };
    let wav = amplify(wav_bytes(&rendered.samples), effective_volume(volume, sh.gain_pct));
    write_cache(sh, cache_path, &wav);
    Ok(Served { wav, elapsed: py_float2(rendered.elapsed_ms), cache: "MISS" })
}

/// What `pool.render` returns to a renderer_service that keeps its own cache,
/// gain, dedup and grammar pauses: the raw WAV and the lane's elapsed ms, the
/// failure as the pool worded it. Its text is final, so it's only stripped to
/// spot the empty case (the service can't post "": it sends a space).
fn pool_render_flow(s: &mut std::net::TcpStream, sh: &Shared, body: &[u8], head_only: bool) {
    let req = match parse_render(body) {
        Ok(r) => r,
        Err(e) => {
            respond_err(s, e.code, &e.body, head_only);
            return;
        }
    };
    let pre = py_strip(&req.text);
    let sing_empty = req.mode == "sing" && !has_lyrics(pre);
    if req.mode == "sing" && !sing_empty {
        err_detail(s, 500, "engine error: sing mode is not supported by this worker", head_only);
        return;
    }
    let job = us_job(&req.mode, pre, &req.text, sing_empty, &req.voice);
    match sh.pool.render(std::sync::Arc::new(job)) {
        Ok(rendered) => respond(
            s,
            200,
            "audio/wav",
            &[("X-Render-Time-Ms", &py_float2(rendered.elapsed_ms))],
            false,
            &wav_bytes(&rendered.samples),
            head_only,
        ),
        Err(detail) => err_detail(s, 500, &detail, head_only),
    }
}

fn finish_inflight(
    sh: &Shared,
    key: &str,
    entry: &Inflight,
    result: SynthResult,
) {
    *entry.result.lock().unwrap() = Some(result);
    *entry.done.lock().unwrap() = true;
    entry.cvar.notify_all();
    sh.inflight.lock().unwrap().remove(key);
}

fn write_cache(sh: &Shared, path: &std::path::Path, wav: &[u8]) {
    let n = sh.tmp_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = sh.cache_dir.join(format!(".tmp-{}-{n}", std::process::id()));
    if std::fs::write(&tmp, wav).is_err() {
        return;
    }
    if std::fs::rename(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return;
    }
    let now = std::time::Instant::now();
    let mut next = sh.prune_next.lock().unwrap();
    if now >= *next {
        *next = now + std::time::Duration::from_secs(sh.prune_interval_secs);
        drop(next);
        prune_cache(&sh.cache_dir, sh.cache_max_bytes);
    }
}

fn main() {
    let port: u16 = std::env::var("RUST_WORKER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(18082);
    let gain_pct: i64 = tts_env("TTSMODACHI_OUTPUT_GAIN_PERCENT")
        .and_then(|v| v.parse().ok())
        .map(|v: i64| v.clamp(25, 300))
        .unwrap_or(125);
    let grammar = tts_env("TTSMODACHI_GRAMMAR_PAUSES")
        .map(|v| !matches!(v.to_lowercase().as_str(), "0" | "false" | "no" | "off"))
        .unwrap_or(true);

    let dir = reqz::seg2_dir();
    // Streaming load: only the 2.4 MB wire image is ever held whole. The
    // template file is sized from the wire index, then every page run is
    // written straight into it (snapshot files are paged per page).
    let wire = reqz::parse_wire(&dir);
    let size = wire.max_end;
    let wargs = wire.args;
    let store = sparse::Template::create(size).expect("template memfd");
    let _ = reqz::write_template(&store, &wire);
    drop(wire);
    let template_fd = store.fd_private();
    let snap = sparse::Mmap::shared_ro(template_fd, size).expect("snap map");
    // The template fd stays open for the process lifetime: every lane start
    // maps it again, so the store is intentionally leaked.
    std::mem::forget(store);
    let boot = tts_core::engine_us::BootArgs {
        engine: wargs[0],
        magic: wargs[1],
        request: wargs[2],
        request_sp: wargs[3],
        voice: tts_core::engine_us::VOICE_WRAPPER,
        s16: wargs[4],
        s17: wargs[5],
        tick: wargs[6] as u64,
    };

    let engine_version =
        tts_env("TTSMODACHI_ENGINE_VERSION").unwrap_or_else(|| "ttsmodachi-v1".to_string());
    let cache_dir = std::path::PathBuf::from(
        tts_env("TTSMODACHI_CACHE_DIR").unwrap_or_else(|| "cache".to_string()),
    );
    let cache_max_bytes: u64 = tts_env("TTSMODACHI_CACHE_MAX_BYTES")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1024 * 1024 * 1024);
    let prune_interval_secs: u64 = tts_env("TTSMODACHI_CACHE_PRUNE_INTERVAL_SECONDS")
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    if std::fs::create_dir_all(&cache_dir).is_err() {
        eprintln!("tts-worker: cache dir {} not writable; caching disabled", cache_dir.display());
    }

    let pool = pool::RendererPool::new(
        pool_config(),
        std::sync::Arc::new(UsFactory {
            template_fd,
            size,
            boot,
            snap: std::sync::Arc::new(snap),
            warmup_block: reqz::warmup_audio_block(),
        }),
    );
    pool.start();

    let shared = std::sync::Arc::new(Shared {
        pool,
        executor: pool::ExecutorSlots::new(default_executor_threads()),
        max_inflight: env_int("TTSMODACHI_MAX_INFLIGHT_RENDERS", MAX_INFLIGHT_DEFAULT).max(0) as usize,
        gain_pct,
        grammar,
        port,
        engine_version,
        cache_dir,
        cache_max_bytes,
        prune_interval_secs,
        prune_next: std::sync::Mutex::new(std::time::Instant::now()),
        tmp_counter: std::sync::atomic::AtomicU64::new(0),
        inflight: std::sync::Mutex::new(std::collections::HashMap::new()),
    });

    let host = std::env::var("RUST_WORKER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let listener = TcpListener::bind((host.as_str(), port))
        .unwrap_or_else(|e| panic!("bind {host}:{port}: {e}"));
    eprintln!("tts-worker listening on {host}:{port}");
    for conn in listener.incoming() {
        match conn {
            Ok(s) => {
                let sh = shared.clone();
                std::thread::spawn(move || handle(s, sh));
            }
            Err(e) => eprintln!("accept: {e}"),
        }
    }
}

const MAX_INFLIGHT_DEFAULT: i128 = 32;
/// Contract needs >= 128 MB; parity_probe runs speak() under the same 512 MB.
const LANE_STACK_BYTES: usize = 512 * 1024 * 1024;
const WEDGED_EXIT_AFTER: std::time::Duration = std::time::Duration::from_secs(120);

/// ttsmodachi_bot/env.py env_int: a bad value stops startup, like its ValueError.
fn env_int(name: &str, default: i128) -> i128 {
    match tts_env(name) {
        Some(v) => py_int(&v).unwrap_or_else(|e| panic!("{name}: {e}")),
        None => default,
    }
}

fn env_float(name: &str, default: f64) -> f64 {
    match tts_env(name) {
        Some(v) => v.trim().parse().unwrap_or_else(|e| panic!("{name}: {e}")),
        None => default,
    }
}

/// RendererPool.from_env, for the one ROM this worker has an engine for.
fn pool_config() -> pool::PoolConfig {
    let roms = tts_env("TTSMODACHI_WORKER_ROMS").unwrap_or_else(|| "US".to_string());
    if roms.split(',').map(|r| r.trim().to_uppercase()).any(|r| !r.is_empty() && r != "US") {
        eprintln!("tts-worker: TTSMODACHI_WORKER_ROMS={roms}: only the US engine is available");
    }
    let workers = env_int("TTSMODACHI_US_WORKERS", 1).max(0);
    let default_min = env_int("TTSMODACHI_MIN_WORKERS", 2).max(1);
    let min_active = env_int("TTSMODACHI_US_MIN_WORKERS", default_min).min(workers).max(1);
    let autoscale = !matches!(
        tts_env("TTSMODACHI_AUTOSCALE_WORKERS").unwrap_or_else(|| "true".to_string()).to_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    );
    pool::PoolConfig {
        rom: "US".to_string(),
        workers: workers as usize,
        min_active: min_active as usize,
        render_timeout: env_float("TTSMODACHI_RENDER_TIMEOUT", 20.0),
        autoscale,
        scale_down_idle: env_float("TTSMODACHI_SCALE_DOWN_IDLE_SECONDS", 60.0),
        idle_suspend: env_float("TTSMODACHI_IDLE_SUSPEND_SECONDS", 10.0),
        stack_size: LANE_STACK_BYTES,
        wedged_exit_after: Some(WEDGED_EXIT_AFTER),
    }
}

/// What asyncio.to_thread runs on: `min(32, os.cpu_count() + 4)`, and
/// cpu_count ignores cgroup quotas.
fn default_executor_threads() -> usize {
    let cpus = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) }.max(1) as usize;
    (cpus + 4).min(32)
}

#[cfg(test)]
mod gp_tests {
    use super::{add_grammar_pauses, cache_key, has_lyrics, py_float2, py_int, py_json, py_strip, VoiceDone, J, NO_AUDIO_BODY};

    #[test]
    fn python_int_shapes() {
        assert_eq!(py_int("50"), Ok(50));
        assert_eq!(py_int("  +50 "), Ok(50));
        assert_eq!(py_int("-3"), Ok(-3));
        assert_eq!(py_int("1_0"), Ok(10));
        assert_eq!(
            py_int("high"),
            Err("invalid literal for int() with base 10: 'high'".to_string())
        );
        assert!(py_int("").is_err());
        assert!(py_int("5x").is_err());
        assert!(py_int("0x12").is_err());
        assert!(py_int("50.0").is_err());
    }

    #[test]
    fn python_float_shapes() {
        assert_eq!(py_float2(5764.38), "5764.38");
        assert_eq!(py_float2(0.5), "0.5");
        assert_eq!(py_float2(100.0), "100.0");
        assert_eq!(py_float2(53.40), "53.4");
        assert_eq!(py_float2(0.0), "0.0");
    }

    #[test]
    fn cache_key_matches_python() {
        let v = VoiceDone {
            pitch: 50,
            speed: 50,
            quality: 50,
            tone: 50,
            accent: 50,
            intonation: 1,
            lang: "useng".to_string(),
            volume: 200,
            engine: "tl3ds".to_string(),
        };
        assert_eq!(
            cache_key("hi", &v, "text", "ttsmodachi-v1:gain125:grammarpauses1"),
            "91d3cf27fbdac69776b224597fef7815220155d0a65590c5c92a0daa75e60894"
        );
    }

    #[test]
    fn json_echo_matches_python() {
        let mut p = super::P { b: "{\"b\":50.5,\"a\":null}".as_bytes(), i: 0 };
        let v = super::parse_val(&mut p).unwrap();
        assert_eq!(py_json(&v), "{\"b\":50.5,\"a\":null}");
        assert_eq!(py_json(&J::Str("\u{1f354}".to_string())), "\"\\ud83c\\udf54\"");
    }
    #[test]
    fn vectors_match_python() {
        let cases: &[(&str, &str)] = &[
            ("Hello world. How are you? Really??? Yes, I am here.", "Hello world.\x1b\\pause=0\\\x1b\\mrk=210000680\\ How are you?\x1b\\pause=0\\\x1b\\mrk=210000780\\ Really???\x1b\\pause=0\\\x1b\\mrk=210000780\\ Yes,\x1b\\pause=0\\\x1b\\mrk=210000280\\ I am here."),
            ("I left my charger at home, does anyone have a spare?", "I left my charger at home,\x1b\\pause=0\\\x1b\\mrk=210000280\\ does anyone have a spare?"),
            ("a.b 1.5 x vs. y Mr. Smith went... home! Wait: what; ok", "a.b 1.5 x vs. y Mr. Smith went...\x1b\\pause=0\\\x1b\\mrk=210000980\\ home!\x1b\\pause=0\\\x1b\\mrk=210000780\\ Wait:\x1b\\pause=0\\\x1b\\mrk=210000410\\ what;\x1b\\pause=0\\\x1b\\mrk=210000410\\ ok"),
            ("no punctuation here", "no punctuation here"),
            ("trailing....", "trailing...."),
            ("\"quoted?\" she asked.", "\"quoted?\"\x1b\\pause=0\\\x1b\\mrk=210000780\\ she asked."),
            ("1,000 fish: 2, and 3; done", "1,000 fish:\x1b\\pause=0\\\x1b\\mrk=210000410\\ 2,\x1b\\pause=0\\\x1b\\mrk=210000280\\ and 3;\x1b\\pause=0\\\x1b\\mrk=210000410\\ done"),
            ("a <bleep>test</bleep> here. ok", "a <bleep>test</bleep> here.\x1b\\pause=0\\\x1b\\mrk=210000680\\ ok"),
            ("...", "..."),
            ("Dr. Smith, plz respond... ASAP!!! What???", "Dr. Smith,\x1b\\pause=0\\\x1b\\mrk=210000280\\ plz respond...\x1b\\pause=0\\\x1b\\mrk=210000980\\ ASAP!!!\x1b\\pause=0\\\x1b\\mrk=210000780\\ What???"),
        ];
        for (inp, want) in cases {
            assert_eq!(&add_grammar_pauses(inp), want);
        }
    }
    #[test]
    fn strip_matches_python_ws_controls() {
        assert_eq!(py_strip("   \n  "), "");
        assert_eq!(py_strip(""), "");
        assert_eq!(py_strip("hi"), "hi");
        assert_eq!(py_strip("  hi  "), "hi");
        // U+001C..=U+001F: Python str.strip eats them, Rust trim does not.
        assert_eq!(py_strip("\u{1c}\u{1d}\u{1e}\u{1f}"), "");
        assert_eq!(py_strip("\u{1c}hi\u{1f}"), "hi");
        assert_eq!(py_strip("\u{85}"), "");
        assert_eq!(py_strip("\u{3000}"), "");
    }
    #[test]
    fn lyric_detection_matches_song_parser() {
        // Every vector probed against the worker image's SongParser
        // (feed whole text, count getSong()).
        for t in [
            "hi",
            "<lyric>",
            "<lyric>x",
            "<!-- </lyric> -->",
            "<!-- </lyric> x> y",
            "</lyric",
            "x</lyricx>",
            "&lt;/lyric&gt;",
            "<![CDATA[</lyric>]]>",
            "<",
            "</",
            "</>",
            "<!>",
            "<!---->",
            "<a title=\"</lyric>\">",
            "<? </lyric> ?>",
            "<lyric </lyric>",
            "a</lyric b",
            "</lyr ic>",
            "<//lyric>",
        ] {
            assert!(!has_lyrics(t), "{t:?}");
        }
        for t in [
            "</lyric>",
            "</LYRIC>",
            "</lyric >",
            "</ lyric>",
            "</\tlyric>",
            "a</lyric>b</lyric>c",
            "a</LyRiC  >b",
            "</lyric/>",
            "</lyric / >",
            "</lyric x>",
            "</lyric/x>",
            "x</lyric>",
            "</lyric>trail",
            "a</lyric b>c",
            "<a title=x></lyric>",
            "x</ lyric  >y",
        ] {
            assert!(has_lyrics(t), "{t:?}");
        }
    }
    #[test]
    fn no_audio_body_shape() {
        // Citra's steady-state no-audio 500, pinned to the worker image:
        // exact JSON body, no volatile fields. The battery proves the
        // bytes against live citra; this guards against edit mangling.
        assert_eq!(NO_AUDIO_BODY.len(), 924);
        assert!(NO_AUDIO_BODY.starts_with("{\"detail\":\"All renderer workers failed for ROM US: US-1: "));
        assert!(NO_AUDIO_BODY.ends_with("Renderer returned no audio\\n\"}"));
    }
    #[test]
    fn pool_builds_the_pinned_no_audio_body() {
        let detail = format!(
            "All renderer workers failed for ROM US: US-1: {}",
            crate::pool::NO_AUDIO_TRACEBACK
        );
        assert_eq!(format!("{{\"detail\":{}}}", super::py_json_str(&detail)), NO_AUDIO_BODY);
    }
    #[test]
    fn job_audio_needs_size_and_pointer() {
        // readRenderedAudio: audioSize > 0 (signed) and audioData != 0
        let job = tts_core::engine_live_seg2::JOB_ADDR as usize;
        let mut arena = vec![0u8; job + 32];
        let mut set = |size: u32, data: u32| {
            arena[job + 12..job + 16].copy_from_slice(&size.to_le_bytes());
            arena[job + 16..job + 20].copy_from_slice(&data.to_le_bytes());
            super::job_has_audio(&arena)
        };
        assert!(set(294, 0x0880_0000));
        assert!(!set(0, 0x0880_0000));
        assert!(!set(294, 0));
        assert!(!set(0x8000_0000, 0x0880_0000));
        assert!(!super::job_has_audio(&[]));
    }
}
