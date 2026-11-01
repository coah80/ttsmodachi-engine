use std::io::{Read, Write};
use std::net::TcpListener;

mod pool;

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
    fn as_union_int(&self) -> Option<i128> {
        match self {
            J::Num(n) => Some(*n),
            J::Float(f) if f.fract() == 0.0 && f.is_finite() => Some(*f as i128),
            _ => None,
        }
    }
}

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
                    o.push_str(&format!(
                        "\\u{:04x}\\u{:04x}",
                        0xd800 + (n >> 10),
                        0xdc00 + (n & 0x3ff)
                    ));
                } else {
                    o.push_str(&format!("\\u{:04x}", n));
                }
            }
        }
    }
    o.push('"');
    o
}

fn py_float_repr(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
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

                let s = std::str::from_utf8(&p.b[p.i - 1..]).map_err(|_| "bad utf8".to_string())?;
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

    for (word, val) in [
        ("Infinity", f64::INFINITY),
        ("inf", f64::INFINITY),
        ("NaN", f64::NAN),
        ("nan", f64::NAN),
    ] {
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
    let s = std::str::from_utf8(&p.b[st..p.i]).map_err(|_| "bad number".to_string())?;
    if s.is_empty() || s == "-" || s == "+" {
        return Err("bad number".into());
    }
    if !is_float {
        if let Ok(n) = s.parse::<i128>() {
            return Ok(J::Num(n));
        }
    }
    s.parse::<f64>()
        .map(J::Float)
        .map_err(|_| "bad number".into())
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
        let prev = if start > 0 {
            Some(text[start - 1])
        } else {
            None
        };
        let next = text.get(end + 1).copied();
        if let (Some(a), Some(b)) = (prev, next) {
            if a.is_numeric() && b.is_numeric() {
                return false;
            }
        }
        let w = word_before(text, start).to_lowercase();
        let w = w.trim_end_matches('.');
        const ABBR: [&str; 10] = [
            "dr", "jr", "mr", "mrs", "ms", "mx", "prof", "sr", "st", "vs",
        ];
        if ABBR.contains(&w) || w.chars().count() == 1 {
            return false;
        }
    }
    true
}

fn add_grammar_pauses(text: &str) -> String {
    let mut norm = String::with_capacity(text.len());
    let mut was_ws = true;
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

fn markup(s: &str) -> String {
    s.replace("<bleep>", "\x1b\\mrk=6\\")
        .replace("</bleep>", "\x1b\\mrk=7\\")
        .replace("<echo>", "\x1b\\mrk=4\\")
        .replace("</echo>", "\x1b\\mrk=5\\")
}

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
        let mut b = (s * gain) as i64;
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

mod sparse {
    use std::ffi::c_void;
    use std::io;
    use std::os::unix::io::RawFd;
    use std::ptr;
    use std::slice;

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
            Ok(Self {
                ptr: ptr as *mut u8,
                len,
            })
        }

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

        pub fn shared_ro(fd: RawFd, len: usize) -> io::Result<Self> {
            let len = len.div_ceil(0x1000) * 0x1000;
            let ptr = unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    len,
                    libc::PROT_READ,
                    libc::MAP_SHARED,
                    fd,
                    0,
                )
            };
            Self::check(ptr, len, "shared mmap")
        }

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

        pub fn slice(&self, len: usize) -> &[u8] {
            assert!(len <= self.len);
            &self.as_slice()[..len]
        }
        pub fn slice_mut(&mut self, len: usize) -> &mut [u8] {
            assert!(len <= self.len);
            &mut self.as_mut()[..len]
        }

        pub fn reset(&self) {
            let rc =
                unsafe { libc::madvise(self.ptr as *mut c_void, self.len, libc::MADV_DONTNEED) };
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

    pub struct BootState {
        fd: RawFd,
        pub size: usize,
    }

    impl BootState {
        pub fn fd_private(&self) -> RawFd {
            self.fd
        }
        pub fn create(size: usize) -> io::Result<Self> {
            let fd = unsafe { libc::memfd_create(c"tts-worker-boot".as_ptr(), 0) };
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

    impl Drop for BootState {
        fn drop(&mut self) {
            unsafe {
                libc::close(self.fd);
            }
        }
    }
}

fn sha256_hex(data: &[u8]) -> String {
    tts_rom::sha256_hex(data)
}

fn cache_key(render_text: &str, v: &VoiceDone, mode: &str, version: &str) -> String {
    let fragment = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}",
        v.engine, v.lang, v.pitch, v.speed, v.quality, v.tone, v.accent, v.intonation, v.volume,
    );
    let payload = [version, mode, render_text, &fragment].join("\0");
    sha256_hex(payload.as_bytes())
}

fn tts_env(name: &str) -> Option<String> {
    for key in [
        format!("{name}"),
        format!(
            "TALKMODACHI_{}",
            name.strip_prefix("TTSMODACHI_").unwrap_or(name)
        ),
    ] {
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
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
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

enum UsJob {
    Speak {
        text16: Vec<u16>,
        vp: tts_core::engine_us::VoiceParams,
    },
    NoAudio,
    EmptySing,
}

struct UsLane {
    arena: sparse::Mmap,
    snap: std::sync::Arc<sparse::Mmap>,
    workspace: sparse::Mmap,
    scratch: sparse::Mmap,
    size: usize,
    boot: tts_core::engine_us::BootArgs,
    heap: HeapOs,
    nap: Nap,
    sink: Vec<i16>,
    out: Vec<i16>,

    fresh: bool,
}

impl pool::LaneEngine<UsJob> for UsLane {
    fn render_once(&mut self, job: &UsJob) -> Result<Vec<i16>, pool::RenderFailure> {
        let (text16, vp) = match job {
            UsJob::Speak { text16, vp } => (text16, vp),
            UsJob::NoAudio => return Err(pool::RenderFailure::NoAudio),
            UsJob::EmptySing => return Ok(Vec::new()),
        };
        let Self {
            arena,
            snap,
            workspace,
            scratch,
            size,
            boot,
            heap,
            nap,
            sink,
            out,
            fresh,
        } = self;
        let (size, boot, first) = (*size, *boot, *fresh);
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
            let mut output = tts_core::engine_live::OutputRuntime {
                inner: &mut runtime,
                guest_capture: false,
            };
            let mut engine = tts_core::engine_us::UsEngine::new(
                boot,
                arena.slice_mut(size),
                snap.slice(size),
                workspace.slice_mut(size),
                scratch.slice_mut(size),
                heap,
                &mut output,
            );
            let n = tts_core::engine_us::speak(&mut engine, text16, vp, out)?;
            if first {
                tts_core::engine_us::unprepare_voice(&mut engine);
            }
            Ok::<_, tts_core::engine::Error>(n)
        }));
        *fresh = false;
        match r {
            Ok(Ok(n)) if n > 0 => Ok(out[..n].to_vec()),
            Ok(Ok(_)) => Err(pool::RenderFailure::NoAudio),
            Ok(Err(e)) => Err(pool::RenderFailure::Engine(format!("engine error: {e:?}"))),
            Err(_) => Err(pool::RenderFailure::Engine("engine panic".to_string())),
        }
    }

    fn restart(&mut self) {
        self.arena.reset();
        self.workspace.reset();
        self.scratch.reset();
        self.fresh = true;
    }
}

struct UsFactory {
    boot_fd: std::os::unix::io::RawFd,
    size: usize,
    boot: tts_core::engine_us::BootArgs,
    snap: std::sync::Arc<sparse::Mmap>,
}

impl pool::EngineFactory<UsJob> for UsFactory {
    fn start(&self) -> Result<Box<dyn pool::LaneEngine<UsJob>>, String> {
        let map =
            |what: &str, m: std::io::Result<sparse::Mmap>| m.map_err(|e| format!("{what}: {e}"));
        Ok(Box::new(UsLane {
            arena: map(
                "arena map",
                sparse::Mmap::private_rw(self.boot_fd, self.size),
            )?,
            snap: self.snap.clone(),

            workspace: map("workspace map", sparse::Mmap::anon(self.size))?,
            scratch: map("scratch map", sparse::Mmap::anon(self.size))?,
            size: self.size,
            boot: self.boot,
            heap: HeapOs,
            nap: Nap,
            sink: vec![0i16; 4_000_000],
            out: vec![0i16; 4_000_000],
            fresh: true,
        }))
    }
}

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

fn http_date() -> String {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86400) as i64;
    let tod = (secs % 86400) as i64;

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
    h.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let _ = s.write_all(h.as_bytes());
    if !head_only {
        let _ = s.write_all(body);
    }
}

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

fn read_request(s: &mut std::net::TcpStream) -> Result<(String, String, Vec<u8>, String), String> {
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
        if let Some(v) = l
            .strip_prefix("Content-Length:")
            .or(l.strip_prefix("content-length:"))
        {
            len = v
                .trim()
                .parse::<usize>()
                .map_err(|_| "bad length".to_string())?;
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

const LANG_TO_ID: &[&str] = &["useng", "eueng", "fr", "de", "it", "es", "jp", "kr"];

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
            let expected = allowed
                .iter()
                .map(|a| format!("'{a}'"))
                .collect::<Vec<_>>()
                .join(" or ");
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

fn json_invalid(body: &[u8]) -> RenderErr {
    let preview: String = body.iter().take(32).map(|b| *b as char).collect();
    let msg = format!("Expecting value: line 1 column 1 (char 0), got {preview:?}");
    RenderErr::errors(&[format!(
        "{{\"type\":\"json_invalid\",\"loc\":[\"body\",1],\"msg\":\"JSON decode error\",\"input\":{{}},\"ctx\":{{\"error\":{}}}}}",
        py_json_str(&msg),
    )])
}

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

fn union_field(obj: &[(String, J)], key: &str, dflt: NumField, errs: &mut Vec<String>) -> NumField {
    let v = match get(obj, key) {
        None => return dflt,
        Some(v) => v,
    };
    if let Some(n) = v.as_union_int() {
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

fn py_repr_str(s: &str) -> String {
    if s.contains('\'') && !s.contains('"') {
        format!("\"{s}\"")
    } else {
        format!("'{s}'")
    }
}

fn py_int(s: &str) -> Result<i128, String> {
    let t = s.trim_matches(|c: char| c.is_whitespace() && (c as u32) < 0x80);
    let t = t.strip_prefix('+').unwrap_or(t);
    let neg = t.starts_with('-');
    let t = if neg { &t[1..] } else { t };
    if t.is_empty() {
        return Err(format!(
            "invalid literal for int() with base 10: {}",
            py_repr_str(s)
        ));
    }
    let mut digits = String::with_capacity(t.len());
    let mut last_underscore = true;
    for c in t.chars() {
        if c == '_' {
            if last_underscore {
                return Err(format!(
                    "invalid literal for int() with base 10: {}",
                    py_repr_str(s)
                ));
            }
            last_underscore = true;
            continue;
        }
        if !c.is_ascii_digit() {
            return Err(format!(
                "invalid literal for int() with base 10: {}",
                py_repr_str(s)
            ));
        }
        last_underscore = false;
        digits.push(c);
    }
    if last_underscore {
        return Err(format!(
            "invalid literal for int() with base 10: {}",
            py_repr_str(s)
        ));
    }
    let n: i128 = digits
        .parse()
        .map_err(|_| format!("invalid literal for int() with base 10: {}", py_repr_str(s)))?;
    Ok(if neg { -n } else { n })
}

fn parse_render(body: &[u8]) -> Result<RenderReq, RenderErr> {
    if body.is_empty() {
        return Err(RenderErr::errors(&[
            "{\"type\":\"missing\",\"loc\":[\"body\"],\"msg\":\"Field required\",\"input\":null}"
                .to_string(),
        ]));
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

    let mode = literal_field(top, "mode", "text", &["text", "sing"])?;
    let backend = literal_field(top, "backend", "citra", &["citra", "standalone"])?;

    let mut vce = Voice::defaults();
    let mut errs: Vec<String> = Vec::new();
    if let Some(vv) = get(top, "voice") {
        match vv {
            J::Obj(o) => {
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
    let bad: Vec<&str> = [
        ("pitch", pitch),
        ("speed", speed),
        ("quality", quality),
        ("tone", tone),
        ("accent", accent),
    ]
    .into_iter()
    .filter(|(_, v)| !(0..=100).contains(v))
    .map(|(k, _)| k)
    .collect();
    if !bad.is_empty() {
        return Err(RenderErr::detail(
            400,
            format!("Voice parameter out of range: {}", bad.join(", ")),
        ));
    }
    if !(25..=300).contains(&volume) {
        return Err(RenderErr::detail(
            400,
            "Volume must be between 25 and 300".to_string(),
        ));
    }
    if ![1, 2, 3, 4].contains(&intonation) {
        return Err(RenderErr::detail(
            400,
            "Intonation must be 1, 2, 3, or 4".to_string(),
        ));
    }

    let lang = match &vce.lang {
        NumField::Int(n) => {
            return Err(RenderErr::detail(400, format!("Unsupported language: {n}")))
        }
        NumField::Str(s) => {
            if !LANG_TO_ID.contains(&s.as_str()) {
                return Err(RenderErr::detail(400, format!("Unsupported language: {s}")));
            }

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
    Ok(RenderReq {
        text,
        voice: VoiceDone {
            pitch,
            speed,
            quality,
            tone,
            accent,
            intonation,
            lang,
            volume,
            engine,
        },
        mode,
        backend,
    })
}

#[derive(Clone)]
struct Served {
    wav: Vec<u8>,
    elapsed: String,
    cache: &'static str,
}

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
            respond_err(
                &mut s,
                405,
                b"{\"detail\":\"Method Not Allowed\"}",
                head_only,
            );
            return;
        }
        render_flow(&mut s, &sh, &body, head_only);
        return;
    }
    if path == "/pool/render" {
        if method != "POST" {
            respond_err(
                &mut s,
                405,
                b"{\"detail\":\"Method Not Allowed\"}",
                head_only,
            );
            return;
        }
        pool_render_flow(&mut s, &sh, &body, head_only);
        return;
    }
    if path == "/health" {
        if method != "GET" {
            respond_err(
                &mut s,
                405,
                b"{\"detail\":\"Method Not Allowed\"}",
                head_only,
            );
            return;
        }
        let b = health_body(&sh);
        respond(&mut s, 200, "application/json", &[], false, &b, head_only);
        return;
    }
    respond_err(&mut s, 404, b"{\"detail\":\"Not Found\"}", head_only);
}

fn is_py_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

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
                while j < n && !matches!(b[j], b'\t' | b'\n' | b'\x0c' | b'\r' | b' ' | b'/' | b'>')
                {
                    j += 1;
                }

                if find_byte(b, i + 1, b'>').is_none() {
                    i += 2;
                    continue;
                }
                if text[s..j].eq_ignore_ascii_case("lyric") {
                    return true;
                }

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

fn render_flow(s: &mut std::net::TcpStream, sh: &Shared, body: &[u8], head_only: bool) {
    let req = match parse_render(body) {
        Ok(r) => r,
        Err(e) => {
            respond_err(s, e.code, &e.body, head_only);
            return;
        }
    };
    let v = req.voice;

    let pre = req.text.replace('\n', " ");
    let pre = py_strip(&pre);
    let limit = if v.lang == "jp" { 1024 } else { 2000 };
    if pre.chars().count() > limit {
        err_detail(
            s,
            400,
            &format!("Text is longer than {limit} characters"),
            head_only,
        );
        return;
    }

    let sing_empty = req.mode == "sing" && !has_lyrics(pre);
    if req.mode == "sing" && !sing_empty {
        err_detail(
            s,
            500,
            "engine error: sing mode is not supported by this worker",
            head_only,
        );
        return;
    }
    let render_text = if sh.grammar && req.mode == "text" {
        add_grammar_pauses(pre)
    } else {
        pre.to_string()
    };

    let standalone_available = false;
    let use_standalone = req.backend == "standalone"
        && standalone_available
        && req.mode == "text"
        && v.lang == "useng";
    let backend_tag = if use_standalone { ":standalone" } else { "" };
    let version = format!(
        "{}:gain{}:grammarpauses1{backend_tag}",
        sh.engine_version, sh.gain_pct
    );
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
            let cache = if served.cache == "MISS" && !created {
                "DEDUPED"
            } else {
                served.cache
            };
            respond(
                s,
                200,
                "audio/wav",
                &[
                    ("X-Cache", cache),
                    ("X-Render-Time-Ms", &served.elapsed),
                    ("X-Backend", "citra"),
                ],
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

fn render_to_cache(
    sh: &Shared,
    cache_path: &std::path::Path,
    job: UsJob,
    volume: i128,
) -> SynthResult {
    if let Ok(wav) = std::fs::read(cache_path) {
        return Ok(Served {
            wav,
            elapsed: String::new(),
            cache: "HIT",
        });
    }
    let rendered = {
        let _slot = sh.executor.acquire();
        sh.pool.render(std::sync::Arc::new(job))?
    };
    let wav = amplify(
        wav_bytes(&rendered.samples),
        effective_volume(volume, sh.gain_pct),
    );
    write_cache(sh, cache_path, &wav);
    Ok(Served {
        wav,
        elapsed: py_float2(rendered.elapsed_ms),
        cache: "MISS",
    })
}

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
        err_detail(
            s,
            500,
            "engine error: sing mode is not supported by this worker",
            head_only,
        );
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

fn finish_inflight(sh: &Shared, key: &str, entry: &Inflight, result: SynthResult) {
    *entry.result.lock().unwrap() = Some(result);
    *entry.done.lock().unwrap() = true;
    entry.cvar.notify_all();
    sh.inflight.lock().unwrap().remove(key);
}

fn write_cache(sh: &Shared, path: &std::path::Path, wav: &[u8]) {
    let n = sh
        .tmp_counter
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = sh
        .cache_dir
        .join(format!(".tmp-{}-{n}", std::process::id()));
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

fn options() -> Result<(std::path::PathBuf, u16), String> {
    let mut rom = std::env::var_os("TTS_ROM").map(std::path::PathBuf::from);
    let mut port = std::env::var("RUST_WORKER_PORT")
        .ok()
        .map(|v| {
            v.parse::<u16>()
                .map_err(|_| "invalid RUST_WORKER_PORT".to_owned())
        })
        .transpose()?
        .unwrap_or(18082);
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--rom") => rom = Some(args.next().ok_or("--rom needs a dump path")?.into()),
            Some("--port") => {
                port = args
                    .next()
                    .ok_or("--port needs a number")?
                    .to_str()
                    .ok_or("invalid port")?
                    .parse()
                    .map_err(|_| "invalid port")?
            }
            Some("--help") | Some("-h") => {
                println!("usage: tts-worker [--rom DUMP] [--port PORT]\nset TTS_ROM when --rom is omitted");
                std::process::exit(0);
            }
            _ => return Err("unknown option, use --rom DUMP or --port PORT".into()),
        }
    }
    Ok((
        rom.ok_or("set TTS_ROM or pass --rom with your decrypted US Rev 1 dump")?,
        port,
    ))
}

fn main() {
    let (rom_path, port) = match options() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("tts-worker: {error}");
            std::process::exit(2);
        }
    };
    let gain_pct: i64 = tts_env("TTSMODACHI_OUTPUT_GAIN_PERCENT")
        .and_then(|v| v.parse().ok())
        .map(|v: i64| v.clamp(25, 300))
        .unwrap_or(125);
    let grammar = tts_env("TTSMODACHI_GRAMMAR_PAUSES")
        .map(|v| !matches!(v.to_lowercase().as_str(), "0" | "false" | "no" | "off"))
        .unwrap_or(true);

    let image = match tts_rom::BootImage::load(&rom_path) {
        Ok(image) => image,
        Err(error) => {
            eprintln!("tts-worker: {error}");
            std::process::exit(2);
        }
    };
    let size = image.arena.len();
    let boot = image.boot;
    let store = sparse::BootState::create(size).expect("boot memory");
    for (index, page) in image.arena.chunks(4096).enumerate() {
        if page.iter().any(|b| *b != 0) {
            store
                .write_at((index * 4096) as u64, page)
                .expect("boot memory write");
        }
    }
    drop(image);
    let boot_fd = store.fd_private();
    let snap = sparse::Mmap::shared_ro(boot_fd, size).expect("boot memory mapping");
    std::mem::forget(store);

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
        eprintln!(
            "tts-worker: cache dir {} not writable; caching disabled",
            cache_dir.display()
        );
    }

    let pool = pool::RendererPool::new(
        pool_config(),
        std::sync::Arc::new(UsFactory {
            boot_fd,
            size,
            boot,
            snap: std::sync::Arc::new(snap),
        }),
    );
    pool.start();

    let shared = std::sync::Arc::new(Shared {
        pool,
        executor: pool::ExecutorSlots::new(default_executor_threads()),
        max_inflight: env_int("TTSMODACHI_MAX_INFLIGHT_RENDERS", MAX_INFLIGHT_DEFAULT).max(0)
            as usize,
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
    let listener = match TcpListener::bind((host.as_str(), port)) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("tts-worker: cannot listen on {host}:{port}: {error}");
            std::process::exit(2);
        }
    };
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

const LANE_STACK_BYTES: usize = 512 * 1024 * 1024;
const WEDGED_EXIT_AFTER: std::time::Duration = std::time::Duration::from_secs(120);

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

fn pool_config() -> pool::PoolConfig {
    let roms = tts_env("TTSMODACHI_WORKER_ROMS").unwrap_or_else(|| "US".to_string());
    if roms
        .split(',')
        .map(|r| r.trim().to_uppercase())
        .any(|r| !r.is_empty() && r != "US")
    {
        eprintln!("tts-worker: TTSMODACHI_WORKER_ROMS={roms}: only the US engine is available");
    }
    let workers = env_int("TTSMODACHI_US_WORKERS", 1).max(0);
    let default_min = env_int("TTSMODACHI_MIN_WORKERS", 2).max(1);
    let min_active = env_int("TTSMODACHI_US_MIN_WORKERS", default_min)
        .min(workers)
        .max(1);
    let autoscale = !matches!(
        tts_env("TTSMODACHI_AUTOSCALE_WORKERS")
            .unwrap_or_else(|| "true".to_string())
            .to_lowercase()
            .as_str(),
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

fn default_executor_threads() -> usize {
    let cpus = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) }.max(1) as usize;
    (cpus + 4).min(32)
}
