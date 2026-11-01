

use crate::boot::Memory;

pub fn engine_kind(object: &[u8]) -> u32 {
    match object[4] {
        x @ 0..=2 => x as u32,
        5 => 3,
        _ => 5,
    }
}

pub fn language(object: &[u8], map: &[u8; 256]) -> u8 {
    map[object[5] as usize]
}

pub fn append(dst: &mut [u16], max: usize, src: &[u16], n: usize) {
    let len = dst
        .iter()
        .position(|&x| x == 0)
        .expect("unterminated destination");
    let count = n.min(max.saturating_sub(len));
    dst[len..len + count].copy_from_slice(&src[..count]);
    dst[len + count] = 0;
}

pub struct CharacterTables<'a> {
    pub initialized: u8,
    pub bitmap: &'a mut [i8; 208],
    pub pairs: &'a [u16; 49],
}
pub fn special_character(c: u16, engine: u32, lang: u8, tables: &mut CharacterTables<'_>) -> bool {
    let c = c as u32;
    let ascii = c.wrapping_sub(97) <= 25 || c.wrapping_sub(65) <= 25 || c.wrapping_sub(48) <= 9;
    if engine == 1 {
        return ascii;
    }
    match lang {
        1 => ascii,
        2 => {
            if ascii {
                return true;
            }
            if tables.initialized == 0 {
                tables.bitmap.fill(0);
                for i in 0..23 {
                    for j in [1, 2] {
                        let ch = tables.pairs[2 * i + j] as usize;
                        tables.bitmap[ch - 0xb0] = 1;
                    }
                }

                let ch = tables.pairs[47] as usize;
                if ch >= 0xb0 && ch - 0xb0 < 208 {
                    tables.bitmap[ch - 0xb0] = 1;
                }
                for i in [0x52, 0x53, 0x78] {
                    tables.bitmap[80 + i] = 1;
                }

            }
            if c.wrapping_sub(0xb0) < 0x50 {
                tables.bitmap[c as usize - 0xb0] != 0
            } else if c.wrapping_sub(0x100) < 0x80 {
                tables.bitmap[80 + c as usize - 0x100] != 0
            } else {
                false
            }
        }
        3 => {
            ascii
                || [
                    0xc1, 0xe1, 0xc9, 0xe9, 0xcd, 0xed, 0xd3, 0xf3, 0xda, 0xfa, 0xdd, 0xfd, 0xc4,
                    0xe4, 0xd6, 0xf6, 0xdc, 0xfc, 0xdf,
                ]
                .contains(&c)
        }
        4 => {
            ascii
                || [
                    0xc0, 0xe0, 0xc8, 0xe8, 0xcc, 0xec, 0xd2, 0xf2, 0xd9, 0xf9, 0xc9, 0xe9,
                ]
                .contains(&c)
        }
        5 => {
            ascii
                || [
                    0xc1, 0xe1, 0xc9, 0xe9, 0xcd, 0xed, 0xd3, 0xf3, 0xda, 0xfa, 0xdd, 0xfd, 0xd1,
                    0xf1,
                ]
                .contains(&c)
        }
        6 => {
            ascii
                || c.wrapping_sub(0x1100) <= 0xff
                || c.wrapping_sub(0x3130) <= 0x5f
                || c.wrapping_sub(0xa960) <= 0x1f
                || c.wrapping_sub(0xac00) <= 0x2baf
                || c.wrapping_sub(0xd7b0) <= 0x4f
                || c == 0x30fc
        }
        _ => true,
    }
}

#[derive(Clone, Copy, Default)]
pub struct Event {
    pub kind: i32,
    pub duration: i32,
}

pub fn snap_durations(out: &mut [i32], events: &[Event], last: i32) -> i32 {
    let n = out.len();
    assert!(n <= 64);
    let mut tmp = [0i32; 64];
    let mut cumulative = 0i32;
    for pass in 0..2 {
        for i in 0..n {
            let previous = if pass == 0 { cumulative } else { tmp[i] };
            let mut inner = 0i32;
            let mut best = i32::MAX;
            let mut best_cumulative = -1;
            for event in events {
                if !matches!(event.kind, 1 | 6 | 7 | 8) {
                    let d = inner.wrapping_sub(previous);
                    if d.wrapping_abs() <= best.wrapping_abs() {
                        best = d;
                        best_cumulative = inner;
                    }
                    inner = inner.wrapping_add(event.duration.wrapping_mul(10));
                }
            }
            tmp[i] = best_cumulative;
            if pass == 0 {
                cumulative = previous.wrapping_add(out[i]);
            }
        }
        let mut pos = 0;
        while pos < n {
            let cur = tmp[pos];
            let mut run = 1;
            while pos + run < n && tmp[pos + run] == cur {
                run += 1;
            }
            let next = if pos + run < n { tmp[pos + run] } else { last };
            let d = next.wrapping_sub(cur);
            let mut off = 0i32;
            for j in 0..run {
                tmp[pos + j] = cur.wrapping_add(off.wrapping_div(run as i32));
                off = off.wrapping_add(d);
            }
            pos += run;
        }
    }
    for i in 0..n {
        let next = if i + 1 < n { tmp[i + 1] } else { last };
        out[i] = next.wrapping_sub(tmp[i]);
    }
    0
}

pub struct GeneratorTables<'a> {
    pub prefix: &'a [u16],
    pub pause_format: &'a [u16],
    pub volume_format: &'a [u16],
    pub default_volumes: &'a [i32; 8],
    pub rate_override: i32,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    MissingCallee(u32),
}

fn decimal_format(out: &mut [u16], format: &[u16], values: &[i32]) -> usize {
    let mut pos = 0;
    let mut arg = 0;
    let mut i = 0;
    while i < format.len() && format[i] != 0 {
        if format[i] == b'%' as u16 {
            assert_eq!(format[i + 1], b'd' as u16);
            let v = values[arg];
            arg += 1;
            if v < 0 {
                out[pos] = b'-' as u16;
                pos += 1;
            }
            let mut x = v.unsigned_abs();
            let mut digits = [0u16; 10];
            let mut n = 0;
            loop {
                digits[n] = b'0' as u16 + (x % 10) as u16;
                n += 1;
                x /= 10;
                if x == 0 {
                    break;
                }
            }
            for j in (0..n).rev() {
                out[pos] = digits[j];
                pos += 1;
            }
            i += 2;
        } else {
            out[pos] = format[i];
            pos += 1;
            i += 1;
        }
    }
    out[pos] = 0;
    pos
}
fn literal(dst: &mut [u16], max: usize, value: &[u8]) {
    let mut buffer = [0u16; 32];
    for (i, &b) in value.iter().enumerate() {
        buffer[i] = b as u16;
    }
    append(dst, max, &buffer, value.len());
}
fn command(dst: &mut [u16], max: usize, name: &[u8], value: i32) {
    let mut fmt = [0u16; 32];
    fmt[0] = 27;
    fmt[1] = 92;
    for (i, &b) in name.iter().enumerate() {
        fmt[i + 2] = b as u16;
    }
    let p = name.len() + 2;
    fmt[p..p + 4].copy_from_slice(&[61, 37, 100, 92]);
    let mut buffer = [0u16; 64];
    let n = decimal_format(&mut buffer, &fmt, &[value]);
    append(dst, max, &buffer, n);
}

#[allow(clippy::too_many_arguments)]
pub fn generate_western(
    text: &[u16],
    parameters: &[u8],
    engine: u32,
    lang: u8,
    characters: &mut CharacterTables<'_>,
    tables: &GeneratorTables<'_>,
    out: &mut [u16],
    max: usize,
) -> Result<(), Error> {
    generate_western_inner(
        text, parameters, engine, lang, characters, tables, out, max, None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn generate_western_with_random(
    text: &[u16],
    parameters: &[u8],
    engine: u32,
    lang: u8,
    characters: &mut CharacterTables<'_>,
    tables: &GeneratorTables<'_>,
    out: &mut [u16],
    max: usize,
    host: &mut dyn RandomHost,
) -> Result<(), Error> {
    generate_western_inner(
        text,
        parameters,
        engine,
        lang,
        characters,
        tables,
        out,
        max,
        Some(host),
    )
}

#[allow(clippy::too_many_arguments)]
fn generate_western_inner(
    text: &[u16],
    parameters: &[u8],
    engine: u32,
    lang: u8,
    characters: &mut CharacterTables<'_>,
    tables: &GeneratorTables<'_>,
    out: &mut [u16],
    max: usize,
    mut host: Option<&mut dyn RandomHost>,
) -> Result<(), Error> {
    if parameters.len() < 0x234 || out.len() <= max || max < 256 {
        return Err(Error::InvalidInput);
    }
    let half = |p| i16::from_le_bytes(parameters[p..p + 2].try_into().unwrap()) as i32;
    let word = |p| i32::from_le_bytes(parameters[p..p + 4].try_into().unwrap());
    let count = half(0x200);
    if !(1..=64).contains(&count) || !text.contains(&0) {
        return Err(Error::InvalidInput);
    }
    let mut storage = [Event::default(); 64];
    for i in 0..count as usize {
        storage[i] = Event {
            kind: word(i * 8),
            duration: word(i * 8 + 4),
        };
    }
    let events = &storage[..count as usize];
    let frequency = half(0x20c) as f32;
    let pitch = f32::from_bits(word(0x210) as u32);
    let mut gcd = events[0].duration;
    for event in events {
        if gcd == 1 {
            break;
        }
        let mut divisor = gcd;
        gcd = event.duration;
        if divisor != 0 {
            loop {
                let next = gcd.wrapping_rem(divisor);
                gcd = divisor;
                divisor = next;
                if divisor == 0 {
                    break;
                }
            }
        }
    }
    let quantum = gcd * 10;
    let mut event_total = 0;
    let mut speech_total = 0;
    let mut pauses = [(0i32, 0i32, 0i32); 32];
    let mut controls = [(0i32, 0i32, 0i32); 32];
    let mut intonations = [(0i32, 0i32); 32];
    let (mut np, mut nc, mut ni) = (0, 0, 0);
    let (mut elapsed, mut chars) = (0, 0);
    for event in events {
        let (kind, d) = (event.kind, event.duration);
        if !matches!(kind, 6 | 7 | 8) {
            event_total += d * 10;
        }
        match kind {
            1 => {
                if np == 32 {
                    return Err(Error::InvalidInput);
                }
                pauses[np] = (elapsed, d * 10, chars);
                np += 1;
                elapsed += d * 10;
            }
            2..=5 => {
                if ni == 32 {
                    return Err(Error::InvalidInput);
                }
                intonations[ni] = (elapsed, kind);
                ni += 1;
            }
            6..=8 => {
                if nc == 32 {
                    return Err(Error::InvalidInput);
                }
                let mut v = if kind == 6 { d } else { d << 6 };
                if kind == 7 {
                    if pitch.to_bits() as i32 > 0x3f800000 {
                        v *= 2;
                    }
                    if (pitch.to_bits() as i32) < 0x3f800000 {
                        v /= 2;
                    }
                }
                controls[nc] = (elapsed, kind + 53, v);
                nc += 1;
            }
            _ => {
                chars += 1;
                speech_total += d * 10;
                elapsed += d * 10;
            }
        }
    }
    let mut starts = [0usize; 64];
    let mut lengths = [0usize; 64];
    let mut counts = [0i32; 64];
    let mut durations = [0i32; 64];
    let mut special = [false; 64];
    let mut pause_ends = [-1i32; 32];
    let (mut n, mut active, mut char_count, mut pause_index) = (0usize, false, 0, 0usize);
    for i in 0..text.len().min(512) {
        let ch = text[i];
        if ch == 0 {
            break;
        }
        let allowed = if engine == 1 {
            ch < 0x2b0
        } else {
            match lang {
                1..=5 => ch < 0x180,
                6 => {
                    ch < 0x180
                        || (ch as u32).wrapping_sub(0x1100) <= 0xff
                        || (ch as u32).wrapping_sub(0x3130) <= 0x5f
                        || (ch as u32).wrapping_sub(0xa960) <= 0x1f
                        || (ch as u32).wrapping_sub(0xac00) <= 0x2baf
                        || (ch as u32).wrapping_sub(0xd7b0) <= 0x4f
                        || ch == 0x30fc
                }
                _ => true,
            }
        };
        if !allowed || matches!(ch, 32 | 10 | 92 | 34) {
            active = false;
            continue;
        }
        if !active {
            if matches!(ch, 33 | 63 | 45 | 44 | 46 | 0x30fc) {
                continue;
            }
            if n == 64 {
                return Err(Error::InvalidInput);
            }
            active = true;
            starts[n] = i;
            n += 1;
        }
        let split = if lang == 6 && pause_index < np && pauses[pause_index].2 == char_count {
            pause_index += 1;
            counts[n - 1] > 0
        } else {
            false
        };
        if split || lengths[n - 1] > 19 {
            if n == 64 {
                return Err(Error::InvalidInput);
            }
            starts[n] = i;
            n += 1;
        }
        lengths[n - 1] += 1;
        if !matches!(ch, 39 | 46 | 33 | 63 | 45 | 44) {
            char_count += 1;
            counts[n - 1] += 1;
        }
    }
    if n == 0 {
        n = 1;
        lengths[0] = text.iter().position(|&x| x == 0).unwrap().min(20);
        counts[0] = lengths[0] as i32;
    }
    let mut total_count = 0;
    for i in 0..n {
        for &ch in &text[starts[i]..starts[i] + lengths[i]] {
            if special_character(ch, engine, lang, characters) {
                special[i] = true;
                break;
            }
        }
        if counts[i] == 0 {
            counts[i] = 1;
        }
        total_count += counts[i];
    }
    let mut allocated = 0;
    let direct = lang == 6;
    if direct {
        let mut j = 0;
        for i in 0..n {
            for _ in 0..counts[i] {
                while j < events.len() && matches!(events[j].kind, 1..=8) {
                    j += 1;
                }
                if j < events.len() {
                    durations[i] += events[j].duration * 10;
                }
                j += 1;
            }
            allocated += durations[i];
        }
    } else {
        let mut amount = 0;
        for (p, index) in [
            (0x228, 0),
            (0x22a, 1),
            (0x22c, n.saturating_sub(2)),
            (0x22e, n - 1),
        ] {
            let d = half(p);
            if d > 0 && (p == 0x228 || p == 0x22e || n > 1) {
                amount += d * 10;
                durations[index] += d * 10;
                total_count -= counts[index];
                counts[index] = 0;
            }
        }
        for i in 0..n {
            if durations[i] == 0 {
                durations[i] = ((counts[i] * (speech_total - amount)) as f32 / total_count as f32
                    + 0.5
                    + 0.0) as i32;
            }
            allocated += durations[i];
        }
    }
    if allocated < speech_total {
        durations[n - 1] += speech_total - allocated;
    }
    let mut remainder = 0;
    if np > 0 {
        let mut j = 0;
        for i in 0..np {
            let mut target = pauses[i].0;
            if i > 0 {
                target -= pauses[i - 1].1 + pauses[i - 1].0;
            }
            let start = j;
            let mut sum = 0;
            while sum < target && j < n {
                let d = durations[j];
                if j > start && d / 2 + sum > target {
                    break;
                }
                sum += d;
                j += 1;
            }
            pause_ends[i] = j as i32 - 1;
            if j > start {
                let scale = 1.0 / sum as f32;
                for v in &mut durations[start..j] {
                    *v = (0.5 + (*v * target) as f32 * scale) as i32;
                }
            }
        }
        if j < n {
            let target = event_total - (pauses[np - 1].0 + pauses[np - 1].1);
            let sum: i32 = durations[j..n].iter().sum();
            let scale = 1.0 / sum as f32;
            for v in &mut durations[j..n] {
                *v = (0.5 + (*v * target) as f32 * scale) as i32;
            }
        }
        let sum =
            durations[..n].iter().sum::<i32>() + pauses[..np].iter().map(|p| p.1).sum::<i32>();
        if sum < event_total {
            remainder = event_total - sum;
        }
        if quantum >= 1 {
            for i in 0..n - 1 {
                let old = durations[i];
                let rounded =
                    ((old + ((quantum - (quantum >> 31)) >> 1)) / quantum * quantum).max(quantum);
                durations[i] = rounded;
                durations[i + 1] -= rounded - old;
            }
        }
    } else {
        snap_durations(&mut durations[..n], events, speech_total);
    }
    if (np == 0 || quantum >= 1) && quantum > 0 {
        let mut amount = quantum;
        while amount > 0 {
            for i in 0..n {
                let j = (n - 1 - i + n - 1) % n;
                let old = durations[n - 1 - i];
                if old < amount {
                    durations[n - 1 - i] = amount;
                    durations[j] -= amount - old;
                }
            }
            if durations[n - 1] > 0 {
                break;
            }
            amount /= 2;
        }
    }
    let mut amount = match frequency as i32 {
        146 => 50,
        272 => 58,
        308 => 73,
        384 => 80,
        _ => 74,
    };
    if tables.rate_override > 50 {
        amount = tables.rate_override;
    }
    let mut buffer = [0u16; 256];
    let count = decimal_format(
        &mut buffer,
        tables.prefix,
        &[
            amount,
            5,
            (frequency * 100.0) as i32 + 100000000,
            (pitch * 10000.0) as i32 + 10000000,
            60000000,
            80010000,
            70000020,
            110000000,
            130000000,
        ],
    );
    out[..count + 1].copy_from_slice(&buffer[..count + 1]);
    literal(out, max, b"\x1b\\wait=0\\");
    command(out, max, b"mrk", parameters[0x224] as i32 + 180000000);
    let pause = u16::from_le_bytes(parameters[0x226..0x228].try_into().unwrap()) as i32;
    command(out, max, b"mrk", pause + 190000000);
    let initial_volume = if engine == 1 {
        tables.default_volumes[1] & 255
    } else if (1..=6).contains(&lang) {
        tables.default_volumes[lang as usize + 1] & 255
    } else {
        80
    };
    let mut current_volume = initial_volume;
    for (i, base) in [200000000, 220000000, 230000000].iter().enumerate() {
        let v = word(0x218 + i * 4);
        command(out, max, b"mrk", base + v);
        if v != 10000 {
            current_volume = current_volume * 91 / 100;
        }
    }
    if current_volume != initial_volume {
        command(out, max, b"vol", current_volume);
    }
    command(out, max, b"mrk", 0x30fd + 50000000);
    let sum: i32 = durations[..n].iter().sum();
    elapsed = 0;
    for event in events {
        if !matches!(event.kind, 1..=8) {
            command(out, max, b"mrk", event.kind + 20000000 + elapsed * 10000);
            elapsed += event.duration;
            if sum <= elapsed * 10 {
                break;
            }
        }
    }
    let (mut pi, mut j, mut k) = (0, 0, 0);
    elapsed = 0;
    char_count = 0;
    for i in 0..n {
        let following = i < n - 1 && durations[i + 1] > 0;
        if durations[i] > 0 {
            elapsed += durations[i];
            if !direct || !matches!(text.get(starts[i] + 1), Some(0x30fc | 0x7e)) {
                if j < ni && intonations[j].0 < elapsed {
                    command(
                        out,
                        max,
                        b"mrk",
                        match intonations[j].1 {
                            2 => 9,
                            3 => 10,
                            4 => 11,
                            _ => 8,
                        },
                    );
                    j += 1;
                } else if ni == 0 && i == 0 {
                    command(out, max, b"mrk", 9);
                }
            } else {
                command(out, max, b"mrk", 10);
            }
            char_count += durations[i];
            if k < nc && controls[k].0 < char_count {
                command(
                    out,
                    max,
                    b"mrk",
                    controls[k].2
                        + match controls[k].1 {
                            59 => 70000000,
                            60 => 110000000,
                            _ => 120000000,
                        },
                );
                k += 1;
            }
            command(out, max, b"mrk", durations[i] + 150000000);
        }
        if lengths[i] > 20 {
            command(out, max, b"mrk", 6);
            append(out, max, &text[starts[i]..], 20);
            literal(out, max, b" ");
            command(out, max, b"mrk", 7);
        } else if special[i] {
            if lengths[i] >= 5 {
                command(out, max, b"mrk", 30);
            }
            append(out, max, &text[starts[i]..], lengths[i]);
            if lengths[i] >= 5 {
                command(out, max, b"mrk", 31);
            }
            if following {
                literal(out, max, b" ");
            }
        } else {
            match host.as_deref_mut() {
                Some(host) => random_fallback(out, max, lengths[i] as i32, host),
                None => return Err(Error::MissingCallee(0x741198)),
            }
        }
        if pi < 32 && pause_ends[pi] == i as i32 {
            if following {
                if pauses[pi].1 > 0 {
                    command(out, max, b"pause", 5);
                    let count = decimal_format(
                        &mut buffer,
                        tables.pause_format,
                        &[pauses[pi].1 + 210000000],
                    );
                    append(out, max, &buffer, count);
                }
                pi += 1;
            } else {
                for p in &pauses[pi..np] {
                    remainder += p.1;
                }
            }
        }
    }
    let tail = word(0x208) + remainder;
    if tail > 0 {
        let count = decimal_format(&mut buffer, tables.pause_format, &[tail + 210000000]);
        append(out, max, &buffer, count);
    }
    if current_volume != initial_volume {
        let count = decimal_format(&mut buffer, tables.volume_format, &[initial_volume]);
        append(out, max, &buffer, count);
    }
    literal(out, max, b"\x1b\\eos=1\\");
    Ok(())
}

pub fn special_character_korean(
    c: u16,
    engine: u32,
    lang: u8,
    tables: &mut CharacterTables<'_>,
) -> bool {
    if engine != 1 && lang == 6 && c == 0x30fc {
        false
    } else {
        special_character(c, engine, lang, tables)
    }
}

pub trait RandomHost {
    fn random_below(&mut self, limit: u32) -> u32;
}

pub struct SeededRandom {
    pub state: u64,
}
impl RandomHost for SeededRandom {
    fn random_below(&mut self, limit: u32) -> u32 {
        self.state = self
            .state
            .wrapping_mul(0x690379b2b2e3d431)
            .wrapping_add(0x508ebd);
        (((self.state >> 32) * limit as u64) >> 32) as u32
    }
}

pub fn random_fallback<H: RandomHost + ?Sized>(out: &mut [u16], max: usize, n: i32, host: &mut H) {
    random_fallback_inner(out, max, n, host, true);
}

pub fn random_fallback_korean<H: RandomHost + ?Sized>(
    out: &mut [u16],
    max: usize,
    n: i32,
    host: &mut H,
) {
    random_fallback_inner(out, max, n, host, false);
}

fn random_fallback_inner<H: RandomHost + ?Sized>(
    out: &mut [u16],
    max: usize,
    n: i32,
    host: &mut H,
    trailing_space: bool,
) {
    let c = n.min(12);
    loop {
        let (syllable, repetitions) = match host.random_below(8) {
            0 if c / 3 > 0 => (b"la".as_slice(), c / 3),
            1 if c >= 4 => {
                literal(out, max, b"sha");
                (b"bada".as_slice(), (c - 4) / 4)
            }
            2 if c / 5 > 0 => (b"daba".as_slice(), c / 5),
            3 if c / 4 > 0 => (b"dubi".as_slice(), c / 4),
            4 if c <= 3 => (b"oh".as_slice(), 1),
            5 if c <= 3 => (b"wii".as_slice(), 1),
            6 if c <= 3 => (b"ah".as_slice(), 1),
            7 if c / 5 > 0 => (b"doowup".as_slice(), c / 5),
            _ => continue,
        };
        for _ in 0..repetitions {
            literal(out, max, syllable);
        }
        if trailing_space {
            literal(out, max, b" ");
        }
        break;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn generate_korean<H: RandomHost + ?Sized>(
    text: &[u16],
    parameters: &[u8],
    engine: u32,
    lang: u8,
    characters: &mut CharacterTables<'_>,
    tables: &GeneratorTables<'_>,
    out: &mut [u16],
    max: usize,
    host: &mut H,
    use_voice: bool,
    add_pause: bool,
    marker_volume: i32,
) -> Result<(), Error> {
    if parameters.len() < 0x240 || out.len() <= max || max < 256 {
        return Err(Error::InvalidInput);
    }
    let half = |p| i16::from_le_bytes(parameters[p..p + 2].try_into().unwrap()) as i32;
    let word = |p| i32::from_le_bytes(parameters[p..p + 4].try_into().unwrap());
    let count = half(0x200);
    if !(1..=64).contains(&count) || !text.contains(&0) {
        return Err(Error::InvalidInput);
    }
    let mut storage = [Event::default(); 64];
    for i in 0..count as usize {
        storage[i] = Event {
            kind: word(i * 8),
            duration: word(i * 8 + 4),
        };
    }
    let events = &storage[..count as usize];
    let frequency = half(0x20c) as f32;
    let pitch = f32::from_bits(word(0x210) as u32);
    let syllable_count = parameters[0x214] as usize;
    if syllable_count > 12 {
        return Err(Error::InvalidInput);
    }
    let syllables = &parameters[0x216..0x222];
    let mut syllable_index = 0usize;
    let mut syllable_notes = 0usize;
    let mut exact_pauses = true;
    let mut eos = parameters[0x23d] as i8 != 0;
    let mut gcd = events[0].duration;
    for event in events {
        if gcd == 1 {
            break;
        }
        let mut divisor = gcd;
        gcd = event.duration;
        if divisor != 0 {
            loop {
                let next = gcd.wrapping_rem(divisor);
                gcd = divisor;
                divisor = next;
                if divisor == 0 {
                    break;
                }
            }
        }
    }
    let quantum = gcd * 10;
    let mut event_total = 0;
    let mut speech_total = 0;
    let mut pauses = [(0i32, 0i32, 0i32); 32];
    let mut controls = [(0i32, 0i32, 0i32); 32];
    let mut intonations = [(0i32, 0i32); 32];
    let (mut np, mut nc, mut ni) = (0, 0, 0);
    let (mut elapsed, mut chars) = (0, 0);
    for event in events {
        let (kind, d) = (event.kind, event.duration);
        if !matches!(kind, 6 | 7 | 8) {
            event_total += d * 10;
        }
        match kind {
            1 => {
                if np == 32 {
                    return Err(Error::InvalidInput);
                }
                pauses[np] = (
                    elapsed,
                    d * 10,
                    if add_pause || syllable_notes != 0 {
                        chars
                    } else {
                        -1
                    },
                );
                np += 1;
                elapsed += d * 10;
            }
            2..=5 => {
                if ni == 32 {
                    return Err(Error::InvalidInput);
                }
                intonations[ni] = (elapsed, kind);
                ni += 1;
            }
            6..=8 => {
                if nc == 32 {
                    return Err(Error::InvalidInput);
                }
                let mut v = if kind == 6 { d } else { d << 6 };
                if kind == 7 {
                    if pitch.to_bits() as i32 > 0x3f800000 {
                        v *= 2;
                    }
                    if (pitch.to_bits() as i32) < 0x3f800000 {
                        v /= 2;
                    }
                }
                controls[nc] = (elapsed, kind + 53, v);
                nc += 1;
            }
            _ => {
                syllable_notes += 1;
                chars += 1;
                if syllable_index >= 12 {
                    return Err(Error::InvalidInput);
                }
                if syllables[syllable_index] as usize <= syllable_notes {
                    syllable_notes = 0;
                    syllable_index += 1;
                }
                speech_total += d * 10;
                elapsed += d * 10;
            }
        }
    }
    let mut starts = [0usize; 64];
    let mut lengths = [0usize; 64];
    let mut counts = [0i32; 64];
    let mut durations = [0i32; 64];
    let mut special = [false; 64];
    let mut pause_ends = [-1i32; 32];
    let (mut n, mut active, mut char_count, mut pause_index) = (0usize, false, 0, 0usize);
    for i in 0..text.len().min(512) {
        let ch = text[i];
        if ch == 0 {
            break;
        }
        let allowed = if engine == 1 {
            ch < 0x2b0
        } else {
            match lang {
                1..=5 => ch < 0x180,
                6 => {
                    ch < 0x180
                        || (ch as u32).wrapping_sub(0x1100) <= 0xff
                        || (ch as u32).wrapping_sub(0x3130) <= 0x5f
                        || (ch as u32).wrapping_sub(0xa960) <= 0x1f
                        || (ch as u32).wrapping_sub(0xac00) <= 0x2baf
                        || (ch as u32).wrapping_sub(0xd7b0) <= 0x4f
                        || ch == 0x30fc
                }
                _ => true,
            }
        };
        if !allowed || matches!(ch, 32 | 10 | 92 | 34) {
            active = false;
            continue;
        }
        if !active {
            if matches!(ch, 33 | 63 | 45 | 44 | 46 | 0x30fc) {
                continue;
            }
            if n == 64 {
                return Err(Error::InvalidInput);
            }
            if n > 0
                && !add_pause
                && (syllable_count <= n - 1 || lengths[n - 1] != syllables[n - 1] as usize)
            {
                exact_pauses = false;
            }
            active = true;
            starts[n] = i;
            n += 1;
        }
        let split = if exact_pauses && pause_index < np && pauses[pause_index].2 == char_count {
            pause_index += 1;
            counts[n - 1] > 0
        } else {
            false
        };
        if split || lengths[n - 1] > 19 {
            if n == 64 {
                return Err(Error::InvalidInput);
            }
            if n > 0
                && !add_pause
                && (syllable_count <= n - 1 || lengths[n - 1] != syllables[n - 1] as usize)
            {
                exact_pauses = false;
            }
            starts[n] = i;
            n += 1;
        }
        lengths[n - 1] += 1;
        if !matches!(ch, 39 | 46 | 33 | 63 | 45 | 44) {
            char_count += 1;
            counts[n - 1] += 1;
        }
    }
    if n == 0 {
        n = 1;
        lengths[0] = text.iter().position(|&x| x == 0).unwrap().min(20);
        counts[0] = lengths[0] as i32;
    }
    let mut total_count = 0;
    for i in 0..n {
        for &ch in &text[starts[i]..starts[i] + lengths[i]] {
            if special_character_korean(ch, engine, lang, characters) {
                special[i] = true;
                break;
            }
        }
        if counts[i] == 0 {
            counts[i] = 1;
        }
        total_count += counts[i];
    }
    if lengths[n - 1] > 0 && text[starts[n - 1] + lengths[n - 1] - 1].wrapping_sub(48) < 10 {
        eos = true;
    }
    let note_count = events.iter().filter(|e| !matches!(e.kind, 1..=8)).count();
    let mut allocated = 0;
    let direct = true;
    if lang == 6 && (total_count as usize == note_count || add_pause) {
        let mut j = 0;
        for i in 0..n {
            for _ in 0..counts[i] {
                while j < events.len() && matches!(events[j].kind, 1..=8) {
                    j += 1;
                }
                if j < events.len() {
                    durations[i] += events[j].duration * 10;
                }
                j += 1;
            }
            allocated += durations[i];
        }
    } else {
        let mut notes = [0i32; 64];
        let mut ni = 0;
        for e in events {
            if !matches!(e.kind, 1..=8) {
                notes[ni] = e.duration;
                ni += 1;
            }
        }
        let mut available = note_count;
        let mut timing = [half(0x234), half(0x236), half(0x238), half(0x23a)];
        if timing[3] == 0 && syllable_count > 0 {
            let size = syllables[syllable_count - 1] as usize;
            if available > size {
                timing[3] = notes[note_count - size..note_count].iter().sum();
                available -= size;
            }
        }
        if timing[0] == 0 && syllable_count > 1 {
            let size = syllables[0] as usize;
            if size < available {
                timing[0] = notes[..size].iter().sum();
                available -= size;
            }
        }
        if timing[1] == 0 && syllable_count > 2 {
            let size = syllables[1] as usize;
            if size < available && n > 1 {
                let start = syllables[0] as usize;
                timing[1] = notes[start..start + size].iter().sum();
                available -= size;
            }
        }
        if timing[2] == 0 && syllable_count > 3 {
            let size = syllables[syllable_count - 2] as usize;
            if size < available && n > 1 && size > 0 {
                let start = note_count - size - syllables[syllable_count - 1] as usize;
                timing[2] = notes[start..start + size].iter().sum();
            }
        }
        let mut amount = 0;
        for (t, index) in [(0, 0), (1, 1), (2, n.saturating_sub(2)), (3, n - 1)] {
            let d = timing[t];
            if d > 0 && (t == 0 || t == 3 || n > 1) {
                amount += d * 10;
                durations[index] += d * 10;
                total_count -= counts[index];
                counts[index] = 0;
            }
        }
        for i in 0..n {
            if durations[i] == 0 {
                durations[i] = ((counts[i] * (speech_total - amount)) as f32 / total_count as f32
                    + 0.5
                    + 0.0) as i32;
            }
            allocated += durations[i];
        }
    }
    if allocated < speech_total {
        durations[n - 1] += speech_total - allocated;
    }
    let mut remainder = 0;
    if np > 0 {
        let mut j = 0;
        for i in 0..np {
            let mut target = pauses[i].0;
            if i > 0 {
                target -= pauses[i - 1].1 + pauses[i - 1].0;
            }
            let start = j;
            let mut sum = 0;
            while sum < target && j < n {
                let d = durations[j];
                if j > start && d / 2 + sum > target {
                    break;
                }
                sum += d;
                j += 1;
            }
            pause_ends[i] = j as i32 - 1;
            if j > start {
                let scale = 1.0 / sum as f32;
                for v in &mut durations[start..j] {
                    *v = (0.5 + (*v * target) as f32 * scale) as i32;
                }
            }
        }
        if j < n {
            let target = event_total - (pauses[np - 1].0 + pauses[np - 1].1);
            let sum: i32 = durations[j..n].iter().sum();
            let scale = 1.0 / sum as f32;
            for v in &mut durations[j..n] {
                *v = (0.5 + (*v * target) as f32 * scale) as i32;
            }
        }
        let sum =
            durations[..n].iter().sum::<i32>() + pauses[..np].iter().map(|p| p.1).sum::<i32>();
        if sum < event_total {
            remainder = event_total - sum;
        }
        if quantum >= 1 {
            for i in 0..n - 1 {
                let old = durations[i];
                let rounded =
                    ((old + ((quantum - (quantum >> 31)) >> 1)) / quantum * quantum).max(quantum);
                durations[i] = rounded;
                durations[i + 1] -= rounded - old;
            }
        }
    } else {
        snap_durations(&mut durations[..n], events, speech_total);
    }
    for i in 0..n {
        let j = (n - 1 - i + n - 1) % n;
        let old = durations[n - 1 - i];
        if old < 0 {
            durations[n - 1 - i] = 0;
            durations[j] += old;
        }
    }
    let mut breath = 0;
    let mut breath_scale = 10000;
    let mut amount = match frequency as i32 {
        110 => 78,
        130 => 72,
        144 => 68,
        146 => 74,
        150 | 170 => 70,
        244 => {
            breath = 700;
            breath_scale = 7000;
            74
        }
        272 => {
            breath_scale = 4000;
            breath = ((1200000.0 / frequency) * 0.25) as i32;
            59
        }
        308 => {
            breath_scale = 9000;
            breath = ((1200000.0 / frequency) * 0.25) as i32;
            71
        }
        328 => {
            breath_scale = 8000;
            breath = ((1200000.0 / frequency) * 0.25) as i32;
            74
        }
        384 => {
            breath_scale = 8000;
            breath = ((1200000.0 / frequency) * 0.25) as i32;
            86
        }
        400 => {
            breath = 550;
            breath_scale = 5000;
            63
        }
        440 => {
            breath_scale = 9000;
            breath = ((1200000.0 / frequency) * 0.25) as i32;
            78
        }
        _ => 74,
    };
    if tables.rate_override > 50 {
        amount = tables.rate_override;
    }
    let mut buffer = [0u16; 256];
    let count = decimal_format(
        &mut buffer,
        tables.prefix,
        &[
            amount,
            if use_voice { 4 } else { 5 },
            (frequency * 100.0) as i32 + 100000000,
            (pitch * 10000.0) as i32 + 10000000,
            60000000,
            80010000,
            70000020,
            110000000,
            130000000,
        ],
    );
    out[..count + 1].copy_from_slice(&buffer[..count + 1]);
    if breath != 0 {
        command(out, max, b"mrk", breath + 240000000);
    }
    if breath_scale != 10000 {
        command(out, max, b"mrk", breath_scale + 250000000);
    }
    literal(out, max, b"\x1b\\wait=0\\");
    command(out, max, b"mrk", parameters[0x230] as i32 + 180000000);
    let pause = u16::from_le_bytes(parameters[0x232..0x234].try_into().unwrap()) as i32;
    command(out, max, b"mrk", pause + 190000000);
    let initial_volume = if engine == 1 {
        tables.default_volumes[1] & 255
    } else if (1..=6).contains(&lang) {
        tables.default_volumes[lang as usize + 1] & 255
    } else {
        80
    };
    let mut current_volume = initial_volume;
    for (i, base) in [200000000, 220000000, 230000000].iter().enumerate() {
        let v = word(0x224 + i * 4);
        command(out, max, b"mrk", base + v);
        if v != 10000 {
            current_volume = current_volume * 91 / 100;
        }
    }
    if current_volume != initial_volume {
        command(out, max, b"vol", current_volume);
    }
    if marker_volume > 0 {
        command(out, max, b"mrk", marker_volume + 50000000);
    }
    let sum: i32 = durations[..n].iter().sum();
    elapsed = 0;
    for event in events {
        if !matches!(event.kind, 1..=8) {
            command(out, max, b"mrk", event.kind + 20000000 + elapsed * 10000);
            elapsed += event.duration;
            if sum <= elapsed * 10 {
                break;
            }
        }
    }
    let (mut pi, mut j, mut k) = (0, 0, 0);
    elapsed = 0;
    char_count = 0;
    let mut note_index = 0;
    for i in 0..n {
        if durations[i] > 0 {
            elapsed += durations[i];
            if !direct || !matches!(text.get(starts[i] + 1), Some(0x30fc | 0x7e)) {
                if j < ni && intonations[j].0 < elapsed {
                    command(
                        out,
                        max,
                        b"mrk",
                        match intonations[j].1 {
                            2 => 9,
                            3 => 10,
                            4 => 11,
                            _ => 8,
                        },
                    );
                    j += 1;
                } else if ni == 0 && i == 0 {
                    command(out, max, b"mrk", 9);
                }
            } else {
                command(out, max, b"mrk", 10);
            }
            char_count += durations[i];
            if k < nc && controls[k].0 < char_count {
                command(
                    out,
                    max,
                    b"mrk",
                    controls[k].2
                        + match controls[k].1 {
                            59 => 70000000,
                            60 => 110000000,
                            _ => 120000000,
                        },
                );
                k += 1;
            }
            while durations[i] > 0 {
                let mut duration = durations[i];
                if direct && total_count as usize == note_count && note_index < events.len() {
                    let e = events[note_index];
                    note_index += 1;
                    if matches!(e.kind, 1..=8) {
                        continue;
                    }
                    duration = e.duration * 10;
                }
                command(out, max, b"mrk", duration + 150000000);
                durations[i] -= duration;
            }
        }
        if durations[i] < 0 {
            if i + 1 >= 64 {
                return Err(Error::InvalidInput);
            }
            durations[i + 1] += durations[i];
            durations[i] = 0;
        }
        let following = i < n - 1 && durations[i + 1] > 0;
        if lengths[i] > 20 {
            command(out, max, b"mrk", 6);
            append(out, max, &text[starts[i]..], 20);
            literal(out, max, b" ");
            command(out, max, b"mrk", 7);
        } else {
            if special[i] {
                append(out, max, &text[starts[i]..], lengths[i]);
            } else {
                random_fallback_korean(out, max, lengths[i] as i32, host);
            }
            if following {
                literal(out, max, b" ");
                if lengths[i] > 0 && text[starts[i] + lengths[i] - 1].wrapping_sub(48) < 10 {
                    literal(out, max, b"\x1b\\eos=1\\");
                }
            }
        }
        if pi < 32 && pause_ends[pi] == i as i32 {
            if following {
                if pauses[pi].1 > 0 {
                    command(out, max, b"pause", 5);
                    let count = decimal_format(
                        &mut buffer,
                        tables.pause_format,
                        &[pauses[pi].1 + 210000000],
                    );
                    append(out, max, &buffer, count);
                }
                pi += 1;
            } else {
                for p in &pauses[pi..np] {
                    remainder += p.1;
                }
            }
        }
    }
    if add_pause {
        command(out, max, b"pause", (pause >> 4) + 50);
    }
    let tail = word(0x208) + remainder;
    if tail > 0 {
        let count = decimal_format(&mut buffer, tables.pause_format, &[tail + 210000000]);
        append(out, max, &buffer, count);
    }
    if current_volume != initial_volume {
        let count = decimal_format(&mut buffer, tables.volume_format, &[initial_volume]);
        append(out, max, &buffer, count);
    }
    if eos {
        literal(out, max, b"\x1b\\eos=1\\");
    }
    Ok(())
}

const NOTE_OBJ_PTR: u32 = 0x00ac_b580;
const NOTE_LANG_MAP: u32 = 0x008c_80c3;
const NOTE_RATE_FLAG: u32 = 0x00a7_b428;
const NOTE_VOLUMES: u32 = 0x008b_f9a0;
const NOTE_PREFIX_FMT: u32 = 0x008b_f1f8;
const NOTE_PAUSE_FMT: u32 = 0x0074_3214;
const NOTE_VOLUME_FMT: u32 = 0x0074_3228;
const NOTE_RNG_STATE: u32 = 0x00b8_0948;
const NOTE_CHAR_BITMAP: u32 = 0x00b6_5f0c;
const NOTE_CHAR_PAIRS: u32 = 0x008b_f156;
const NOTE_CHAR_EXTRA: u32 = 0x008b_f158;
const NOTE_CHAR_SINGLE: u32 = 0x00b6_5e5c;

const NOTE_FMT_WAIT: u32 = 0x0074_31bc;
const NOTE_FMT_MRK2: u32 = 0x0074_31d8;
const NOTE_FMT_MRK1: u32 = 0x0074_31ea;
const NOTE_FMT_MRK6: u32 = 0x0074_391c;
const NOTE_SPACE: u32 = 0x0074_3930;
const NOTE_FMT_MRK7: u32 = 0x0074_3934;
const NOTE_FMT_MRK30: u32 = 0x0074_3948;
const NOTE_FMT_MRK31: u32 = 0x0074_395c;
const NOTE_FMT_PAUSE: u32 = 0x0074_3970;
const NOTE_FMT_EOS: u32 = 0x0074_3994;

const NOTE_ROM_ADJUST: i32 = 0x0098_9680;

pub struct SongNoteArgs {
    pub text: u32,
    pub events: u32,
    pub event_count: i32,
    pub flags: i32,
    pub frequency: f32,
    pub pitch: f32,
    pub tail_pause: i32,
    pub rate: i32,
    pub volume: i32,
    pub use_voice: i32,
    pub voice: u32,
    pub marker: i32,
    pub pause: u32,
    pub add_pause: i32,
    pub first_duration: i32,
    pub second_duration: i32,
    pub penultimate_duration: i32,
    pub last_duration: i32,
    pub out: u32,
    pub capacity: i32,
}

fn note_divmod(dividend: i32, divisor: i32) -> (i32, i32) {
    if divisor == 0 {
        return (0, dividend);
    }
    (
        dividend.wrapping_div(divisor),
        dividend.wrapping_rem(divisor),
    )
}

fn note_strlen(m: &Memory<'_>, mut p: u32) -> u32 {
    let mut n = 0u32;
    while m.half(p) != 0 {
        n += 1;
        p = p.wrapping_add(2);
    }
    n
}

fn note_out_len(m: &Memory<'_>, dst: u32) -> usize {
    let mut len = 0usize;
    while m.half(dst.wrapping_add((len as u32).wrapping_mul(2))) != 0 {
        len += 1;
    }
    len
}

fn append_guest(m: &mut Memory<'_>, dst: u32, max: usize, src: u32, n: usize) {
    let len = note_out_len(m, dst);
    let count = n.min(max.saturating_sub(len));
    for i in 0..count {
        m.put_half(
            dst.wrapping_add(((len + i) as u32).wrapping_mul(2)),
            m.half(src.wrapping_add((i as u32).wrapping_mul(2))),
        );
    }
    m.put_half(
        dst.wrapping_add(((len + count) as u32).wrapping_mul(2)),
        0,
    );
}

fn append_ascii(m: &mut Memory<'_>, dst: u32, max: usize, bytes: &[u8]) {
    let len = note_out_len(m, dst);
    let count = bytes.len().min(max.saturating_sub(len));
    for (i, &b) in bytes[..count].iter().enumerate() {
        m.put_half(
            dst.wrapping_add(((len + i) as u32).wrapping_mul(2)),
            b as u16,
        );
    }
    m.put_half(
        dst.wrapping_add(((len + count) as u32).wrapping_mul(2)),
        0,
    );
}

fn emit_unit(
    m: &mut Memory<'_>,
    out: u32,
    bound: usize,
    written: &mut usize,
    total: &mut usize,
    unit: u16,
) {
    *total += 1;
    if *written < bound {
        m.put_half(
            out.wrapping_add((*written as u32).wrapping_mul(2)),
            unit,
        );
        *written += 1;
    }
}

pub fn note_printf(
    m: &mut Memory<'_>,
    out: u32,
    size: i32,
    fmt: u32,
    args: &[i32],
) -> i32 {
    let bound = size.max(0) as usize;
    let mut written = 0usize;
    let mut total = 0usize;
    let mut arg = 0usize;
    let mut fp = fmt;
    loop {
        let unit = m.half(fp);
        fp = fp.wrapping_add(2);
        if unit == 0 {
            break;
        }
        if unit == b'%' as u16 {
            let next = m.half(fp);
            fp = fp.wrapping_add(2);
            if next == b'%' as u16 {
                emit_unit(m, out, bound, &mut written, &mut total, b'%' as u16);
            } else {

                assert!(next == b'd' as u16);
                let v = args[arg];
                arg += 1;
                if v < 0 {
                    emit_unit(m, out, bound, &mut written, &mut total, b'-' as u16);
                }
                let mut mag = if v < 0 {
                    (v as u32).wrapping_neg()
                } else {
                    v as u32
                };
                let mut digits = [0u16; 10];
                let mut nd = 0usize;
                loop {
                    digits[nd] = b'0' as u16 + (mag % 10) as u16;
                    nd += 1;
                    mag /= 10;
                    if mag == 0 {
                        break;
                    }
                }
                for j in (0..nd).rev() {
                    emit_unit(m, out, bound, &mut written, &mut total, digits[j]);
                }
            }
        } else {
            emit_unit(m, out, bound, &mut written, &mut total, unit);
        }
    }
    if size != 0 {
        m.put_half(out.wrapping_add((written as u32).wrapping_mul(2)), 0);
    }
    if total < bound {
        total as i32
    } else {
        -1
    }
}

pub fn note_format(
    m: &mut Memory<'_>,
    out: u32,
    size: i32,
    count: i32,
    fmt: u32,
    args: &[i32],
) -> i32 {
    let limit = if size > count.wrapping_add(1) {
        count.wrapping_add(1)
    } else {
        size
    };
    note_printf(m, out, limit, fmt, args)
}

fn emit_format(
    m: &mut Memory<'_>,
    out: u32,
    capacity: i32,
    fmt: u32,
    args: &[i32],
    size: i32,
    count: i32,
) {
    let len = note_out_len(m, out);
    let cap = capacity.max(0) as usize;

    let bound = (cap.saturating_sub(len) as i32).min(size);
    note_format(
        m,
        out.wrapping_add((len as u32).wrapping_mul(2)),
        bound,
        count,
        fmt,
        args,
    );
}

fn special_note(m: &mut Memory<'_>, c: u32, engine: u32, lang: u8) -> bool {
    let ascii = c.wrapping_sub(97) <= 25 || c.wrapping_sub(65) <= 25 || c.wrapping_sub(48) <= 9;
    if engine == 1 {
        return ascii;
    }
    match lang {
        1 => ascii,
        2 => {
            if ascii {
                return true;
            }
            if m.byte(NOTE_RATE_FLAG) == 0 {
                for i in 0..0x50u32 {
                    m.put_byte(NOTE_CHAR_BITMAP.wrapping_add(i), 0);
                }
                for i in 0..0x80u32 {
                    m.put_byte(NOTE_CHAR_BITMAP.wrapping_add(0x50 + i), 0);
                }
                let mut p = NOTE_CHAR_PAIRS;
                for _ in 0..23 {
                    let cur = m.half(p.wrapping_add(2));
                    let nxt = m.half(p.wrapping_add(4));
                    p = p.wrapping_add(4);
                    m.put_byte(
                        NOTE_CHAR_BITMAP.wrapping_add(cur.wrapping_sub(0xb0) as u32),
                        1,
                    );
                    m.put_byte(
                        NOTE_CHAR_BITMAP.wrapping_add(nxt.wrapping_sub(0xb0) as u32),
                        1,
                    );
                }
                let extra = m.half(NOTE_CHAR_EXTRA.wrapping_add(46 * 2));
                m.put_byte(NOTE_CHAR_SINGLE.wrapping_add(extra as u32), 1);
                m.put_byte(NOTE_CHAR_BITMAP.wrapping_add(0x50 + 0x52), 1);
                m.put_byte(NOTE_CHAR_BITMAP.wrapping_add(0x50 + 0x53), 1);
                m.put_byte(NOTE_CHAR_BITMAP.wrapping_add(0x50 + 0x78), 1);
            }
            if c.wrapping_sub(0xb0) < 0x50 {
                m.byte(NOTE_CHAR_BITMAP.wrapping_add(c.wrapping_sub(0xb0))) as i8 != 0
            } else if c.wrapping_sub(0x100) >= 0x80 {
                false
            } else {
                m.byte(NOTE_CHAR_BITMAP.wrapping_add(0x50 + c.wrapping_sub(0x100))) as i8 != 0
            }
        }
        3 => {
            ascii
                || [
                    0xc1, 0xe1, 0xc9, 0xe9, 0xcd, 0xed, 0xd3, 0xf3, 0xda, 0xfa,
                    0xdd, 0xfd, 0xc4, 0xe4, 0xd6, 0xf6, 0xdc, 0xfc, 0xdf,
                ]
                .contains(&c)
        }
        4 => {
            ascii
                || [
                    0xc0, 0xe0, 0xc8, 0xe8, 0xcc, 0xec, 0xd2, 0xf2, 0xd9, 0xf9,
                    0xc9, 0xe9,
                ]
                .contains(&c)
        }
        5 => {
            ascii
                || [
                    0xc1, 0xe1, 0xc9, 0xe9, 0xcd, 0xed, 0xd3, 0xf3, 0xda, 0xfa,
                    0xdd, 0xfd, 0xd1, 0xf1,
                ]
                .contains(&c)
        }
        6 => {
            ascii
                || c.wrapping_sub(0x1100) <= 0xff
                || c.wrapping_sub(0x3130) <= 0x5f
                || c.wrapping_sub(0xa960) <= 0x1f
                || c.wrapping_sub(0xac00) <= 0x2baf
                || c.wrapping_sub(0xd7b0) <= 0x4f
                || c == 0x30fc
        }
        _ => true,
    }
}

fn random_fallback_guest(m: &mut Memory<'_>, dst: u32, max: usize, n: i32) {
    let lo = m.word(NOTE_RNG_STATE);
    let hi = m.word(NOTE_RNG_STATE.wrapping_add(4));
    let mut rng = SeededRandom {
        state: ((hi as u64) << 32) | lo as u64,
    };
    let cap = max;
    let c = n.min(12);
    loop {
        let pick = rng.random_below(8);
        let (head, syllable, repetitions): (&[u8], &[u8], i32) = match pick {
            0 if c / 3 > 0 => (b"", b"la", c / 3),
            1 if c >= 4 => (b"sha", b"bada", (c - 4) / 4),
            2 if c / 5 > 0 => (b"", b"daba", c / 5),
            3 if c / 4 > 0 => (b"", b"dubi", c / 4),
            4 if c <= 3 => (b"", b"oh", 1),
            5 if c <= 3 => (b"", b"wii", 1),
            6 if c <= 3 => (b"", b"ah", 1),
            7 if c / 5 > 0 => (b"", b"doowup", c / 5),
            _ => continue,
        };
        if !head.is_empty() {
            append_ascii(m, dst, cap, head);
        }
        for _ in 0..repetitions {
            append_ascii(m, dst, cap, syllable);
        }
        append_ascii(m, dst, cap, b" ");
        break;
    }
    m.put_word(NOTE_RNG_STATE, rng.state as u32);
    m.put_word(NOTE_RNG_STATE.wrapping_add(4), (rng.state >> 32) as u32);
}

#[allow(clippy::too_many_lines)]
pub fn song_notes(m: &mut Memory<'_>, a: &SongNoteArgs) -> u32 {
    let mut durations = [0i32; 64];
    let mut counts = [0i32; 64];
    let mut lengths = [0usize; 64];
    let mut starts = [0usize; 64];
    let mut pause_starts = [0i32; 32];
    let mut control_values = [0i32; 32];
    let mut pause_durations = [0i32; 32];
    let mut control_types = [0i32; 32];
    let mut intonation_types = [0i32; 32];
    let mut pause_ends = [-1i32; 32];
    let mut intonation_starts = [0i32; 32];
    let mut pause_chars = [0i32; 32];
    let mut control_starts = [0i32; 32];
    let mut special = [false; 64];

    let obj = m.word(NOTE_OBJ_PTR);
    let engine = match m.byte(obj.wrapping_add(4)) {
        x @ 0..=2 => x as u32,
        5 => 3,
        _ => 5,
    };
    let language = m.byte(NOTE_LANG_MAP.wrapping_add(m.byte(obj.wrapping_add(5)) as u32));

    let n_ev = a.event_count.max(0) as usize;
    let first_duration = m.word(a.events.wrapping_add(4)) as i32;
    let mut ev = [Event::default(); 64];
    for i in 0..n_ev {
        ev[i] = Event {
            kind: m.word(a.events.wrapping_add((i as u32).wrapping_mul(8))) as i32,
            duration: m.word(a.events.wrapping_add((i as u32).wrapping_mul(8) + 4)) as i32,
        };
    }

    let mut gcd = first_duration;
    let mut i = 0usize;
    while i < n_ev && gcd != 1 {
        let divisor = gcd;
        gcd = ev[i].duration;
        if divisor != 0 {
            let mut d = divisor;
            while d != 0 {
                let next = note_divmod(gcd, d).1;
                gcd = d;
                d = next;
            }
        }
        i += 1;
    }
    let quantum = gcd.wrapping_mul(10);
    let mut event_total = 0i32;
    let mut speech_total = 0i32;
    let mut pause_count = 0usize;
    let mut control_count = 0usize;
    let mut intonation_count = 0usize;
    let mut elapsed = 0i32;
    let mut char_count = 0i32;
    for e in ev.iter().take(n_ev) {
        if e.kind != 6 && e.kind != 7 && e.kind != 8 {
            event_total = event_total.wrapping_add(e.duration.wrapping_mul(10));
        }
        match e.kind {
            1 => {
                pause_starts[pause_count] = elapsed;
                pause_durations[pause_count] = e.duration.wrapping_mul(10);
                pause_chars[pause_count] = char_count;
                pause_count += 1;
                elapsed = elapsed.wrapping_add(e.duration.wrapping_mul(10));
            }
            2 | 3 | 4 | 5 => {
                intonation_starts[intonation_count] = elapsed;
                intonation_types[intonation_count] = e.kind;
                intonation_count += 1;
            }
            6 | 7 | 8 => {
                control_starts[control_count] = elapsed;
                if e.kind == 6 {
                    control_types[control_count] = 59;
                    control_values[control_count] = e.duration;
                } else if e.kind == 7 {
                    control_types[control_count] = 60;
                    control_values[control_count] = e.duration.wrapping_shl(6);
                    if a.pitch.to_bits() as i32 > 0x3f80_0000 {
                        control_values[control_count] =
                            control_values[control_count].wrapping_mul(2);
                    }
                    if (a.pitch.to_bits() as i32) < 0x3f80_0000 {
                        control_values[control_count] /= 2;
                    }
                } else {
                    control_types[control_count] = 61;
                    control_values[control_count] = e.duration.wrapping_shl(6);
                }
                control_count += 1;
            }
            _ => {
                char_count += 1;
                speech_total = speech_total.wrapping_add(e.duration.wrapping_mul(10));
                elapsed = elapsed.wrapping_add(e.duration.wrapping_mul(10));
            }
        }
    }

    let mut n = 0usize;
    let mut active = false;
    char_count = 0;
    let mut pause_index = 0usize;
    let mut t = 0usize;
    while t < 512 {
        let ch = m.half(a.text.wrapping_add((t as u32).wrapping_mul(2))) as u32;
        if ch == 0 {
            break;
        }
        let printable = ch != 32 && ch != 10 && ch != 92 && ch != 34;
        let mut sep = false;
        if engine == 1 {
            if ch >= 0x2b0 {
                sep = true;
            }
        } else {
            match language {
                1 | 2 | 3 | 4 | 5 => {
                    if ch >= 0x180 {
                        sep = true;
                    }
                }
                6 => {
                    if ch >= 0x180
                        && ch.wrapping_sub(0x1100) > 0xff
                        && ch.wrapping_sub(0x3130) > 0x5f
                        && ch.wrapping_sub(0xa960) > 0x1f
                        && ch.wrapping_sub(0xac00) > 0x2baf
                        && ch.wrapping_sub(0xd7b0) > 0x4f
                        && ch != 0x30fc
                    {
                        sep = true;
                    }
                }
                _ => {}
            }
        }
        if !sep && printable {
            let mut skip = false;
            if !active {
                if matches!(ch, 33 | 63 | 45 | 44 | 46 | 0x30fc) {
                    skip = true;
                } else {
                    active = true;
                    starts[n] = t;
                    lengths[n] = 0;
                    counts[n] = 0;
                    n += 1;
                }
            }
            if !skip {
                let mut split = false;
                if language == 6
                    && pause_index < pause_count
                    && pause_chars[pause_index] == char_count
                {
                    pause_index += 1;
                    if counts[n - 1] > 0 {
                        split = true;
                    }
                }
                if split || lengths[n - 1] > 19 {
                    active = true;
                    starts[n] = t;
                    lengths[n] = 0;
                    counts[n] = 0;
                    n += 1;
                }
                lengths[n - 1] += 1;
                if !matches!(ch, 39 | 46 | 33 | 63 | 45 | 44) {
                    char_count += 1;
                    counts[n - 1] += 1;
                }
            }
        } else {
            active = false;
        }
        t += 1;
    }
    if n == 0 {
        lengths[0] = (note_strlen(m, a.text) as usize).min(20);
        n = 1;
        starts[0] = 0;
        counts[0] = lengths[0] as i32;
    }
    for i in 0..n {
        special[i] = false;
        for j in 0..lengths[i] {
            let ch = m.half(
                a.text.wrapping_add(((starts[i] + j) as u32).wrapping_mul(2)),
            ) as u32;
            if special_note(m, ch, engine, language) {
                special[i] = true;
                break;
            }
        }
    }
    for i in 0..n {
        if counts[i] == 0 {
            counts[i] = 1;
        }
    }
    let mut total_count = 0i32;
    for i in 0..n {
        total_count = total_count.wrapping_add(counts[i]);
    }
    let mut allocated = 0i32;
    let mut direct = false;
    if language == 6 {
        let mut j = 0usize;
        for i in 0..n {
            for _ in 0..counts[i] {
                while ev[j].kind == 1
                    || ev[j].kind == 2
                    || ev[j].kind == 3
                    || ev[j].kind == 4
                    || ev[j].kind == 5
                    || ev[j].kind == 6
                    || ev[j].kind == 7
                    || ev[j].kind == 8
                {
                    j += 1;
                }
                if j < n_ev {
                    durations[i] = durations[i].wrapping_add(ev[j].duration.wrapping_mul(10));
                }
                j += 1;
            }
            allocated = allocated.wrapping_add(durations[i]);
        }
        direct = true;
    } else {
        let mut amount = 0i32;
        if a.first_duration > 0 && n > 0 {
            amount = amount.wrapping_add(a.first_duration.wrapping_mul(10));
            durations[0] = durations[0].wrapping_add(a.first_duration.wrapping_mul(10));
            total_count = total_count.wrapping_sub(counts[0]);
            counts[0] = 0;
        }
        if a.second_duration > 0 && n > 1 {
            amount = amount.wrapping_add(a.second_duration.wrapping_mul(10));
            durations[1] = durations[1].wrapping_add(a.second_duration.wrapping_mul(10));
            total_count = total_count.wrapping_sub(counts[1]);
            counts[1] = 0;
        }
        if a.penultimate_duration > 0 && n > 1 {
            amount = amount.wrapping_add(a.penultimate_duration.wrapping_mul(10));
            durations[n - 2] =
                durations[n - 2].wrapping_add(a.penultimate_duration.wrapping_mul(10));
            total_count = total_count.wrapping_sub(counts[n - 2]);
            counts[n - 2] = 0;
        }
        if a.last_duration > 0 && n > 0 {
            amount = amount.wrapping_add(a.last_duration.wrapping_mul(10));
            durations[n - 1] =
                durations[n - 1].wrapping_add(a.last_duration.wrapping_mul(10));
            total_count = total_count.wrapping_sub(counts[n - 1]);
            counts[n - 1] = 0;
        }
        for i in 0..n {
            if durations[i] == 0 {
                durations[i] = (counts[i].wrapping_mul(speech_total.wrapping_sub(amount)) as f32
                    / total_count as f32
                    + 0.5) as i32;
            }
            allocated = allocated.wrapping_add(durations[i]);
        }
    }
    if allocated < speech_total {
        durations[n - 1] = durations[n - 1].wrapping_add(speech_total.wrapping_sub(allocated));
    }
    let mut remainder = 0i32;
    if pause_count > 0 {
        let mut selected = [0i32; 64];
        let mut j = 0i32;
        for i in 0..pause_count {
            let mut target = pause_starts[i];
            if i > 0 {
                target = target
                    .wrapping_sub(pause_durations[i - 1].wrapping_add(pause_starts[i - 1]));
            }
            let mut sum = 0i32;
            let mut k = 0i32;
            while sum < target && j < n as i32 {
                let duration = durations[j as usize];
                if k > 0 && duration / 2 + sum > target {
                    break;
                }
                selected[k as usize] = j;
                sum = sum.wrapping_add(duration);
                k += 1;
                j += 1;
            }
            pause_ends[i] = j - 1;
            if k > 0 {
                let scale = 1.0f32 / sum as f32;
                for s in 0..k as usize {
                    durations[selected[s] as usize] = (0.5f32
                        + durations[selected[s] as usize].wrapping_mul(target) as f32 * scale)
                        as i32;
                }
            }
        }
        if j < n as i32 {
            let target = event_total
                .wrapping_sub(pause_starts[pause_count - 1].wrapping_add(pause_durations[pause_count - 1]));
            let mut sum = 0i32;
            for i in j as usize..n {
                sum = sum.wrapping_add(durations[i]);
            }
            let scale = 1.0f32 / sum as f32;
            for i in j as usize..n {
                durations[i] =
                    (0.5f32 + durations[i].wrapping_mul(target) as f32 * scale) as i32;
            }
        }
        let mut sum = 0i32;
        for i in 0..n {
            sum = sum.wrapping_add(durations[i]);
        }
        for i in 0..pause_count {
            sum = sum.wrapping_add(pause_durations[i]);
        }
        if sum < event_total {
            remainder = event_total.wrapping_sub(sum);
        }
        if quantum >= 1 {
            for i in 0..n - 1 {
                let old = durations[i];
                let rounded = note_divmod(
                    old.wrapping_add((quantum.wrapping_sub(quantum >> 31)) >> 1),
                    quantum,
                )
                .0
                .wrapping_mul(quantum)
                .max(quantum);
                durations[i] = rounded;
                durations[i + 1] =
                    durations[i + 1].wrapping_sub(rounded.wrapping_sub(old));
            }
        }
    } else {
        snap_durations(&mut durations[..n], &ev[..n_ev], speech_total);
    }
    if (pause_count == 0 || quantum >= 1) && quantum > 0 {
        let mut amount = quantum;
        while amount > 0 {
            for i in 0..n {
                let j = note_divmod(
                    n as i32 - 1 - i as i32 + n as i32 - 1,
                    n as i32,
                )
                .1 as usize;
                let old = durations[n - 1 - i];
                if old < amount {
                    durations[n - 1 - i] = amount;
                    durations[j] = durations[j].wrapping_sub(amount.wrapping_sub(old));
                }
            }
            if durations[n - 1] > 0 {
                break;
            }
            amount /= 2;
        }
    }

    let cap = a.capacity.max(0) as usize;
    let mut amount = match a.frequency as i32 {
        146 => 50,
        272 => 58,
        308 => 73,
        384 => 80,
        _ => 74,
    };
    if m.word(NOTE_RATE_FLAG.wrapping_add(1)) as i32 > 50 {
        amount = m.word(NOTE_RATE_FLAG.wrapping_add(1)) as i32;
    }
    let command_pitch =
        (a.pitch * 10000.0) as i32 + NOTE_ROM_ADJUST;
    let command_frequency =
        (a.frequency * 100.0) as i32 + (110_000_000 - NOTE_ROM_ADJUST);
    note_format(
        m,
        a.out,
        a.capacity,
        a.capacity.wrapping_sub(1),
        NOTE_PREFIX_FMT,
        &[
            amount,
            if a.use_voice != 0 { 4 } else { 5 },
            command_frequency,
            command_pitch,
            110_000_000 - ((110_000_000 - NOTE_ROM_ADJUST) >> 1),
            80_010_000,
            a.rate.wrapping_add(70_000_000),
            110_000_000,
            130_000_000,
        ],
    );
    emit_format(m, a.out, a.capacity, NOTE_FMT_WAIT, &[], 32, 31);
    emit_format(
        m,
        a.out,
        a.capacity,
        NOTE_FMT_MRK2,
        &[
            a.marker.wrapping_add(180_000_000),
            a.pause.wrapping_add(190_000_000) as i32,
        ],
        48,
        47,
    );
    let mut initial_volume = 80i32;
    if engine == 1 {
        initial_volume = (m.word(NOTE_VOLUMES.wrapping_add(4)) & 255) as i32;
    } else {
        match language {
            1 => initial_volume = (m.word(NOTE_VOLUMES.wrapping_add(8)) & 255) as i32,
            2 => initial_volume = (m.word(NOTE_VOLUMES.wrapping_add(12)) & 255) as i32,
            3 => initial_volume = (m.word(NOTE_VOLUMES.wrapping_add(16)) & 255) as i32,
            4 => initial_volume = (m.word(NOTE_VOLUMES.wrapping_add(20)) & 255) as i32,
            5 => initial_volume = (m.word(NOTE_VOLUMES.wrapping_add(24)) & 255) as i32,
            6 => initial_volume = (m.word(NOTE_VOLUMES.wrapping_add(28)) & 255) as i32,
            _ => {}
        }
    }
    let mut current_volume = initial_volume;
    for i in 0..3 {
        let base: i32 = if i == 0 {
            200_000_000
        } else if i == 1 {
            220_000_000
        } else {
            230_000_000
        };
        let event_value = m.word(a.voice.wrapping_add((i as u32).wrapping_mul(4))) as i32;
        emit_format(m, a.out, a.capacity, NOTE_FMT_MRK1, &[base.wrapping_add(event_value)], 32, 31);
        if event_value != 10000 {
            current_volume = current_volume * 91 / 100;
        }
    }
    if current_volume != initial_volume {
        emit_format(m, a.out, a.capacity, NOTE_VOLUME_FMT, &[current_volume], 32, 31);
    }
    if a.volume > 0 {
        emit_format(
            m,
            a.out,
            a.capacity,
            NOTE_FMT_MRK1,
            &[a.volume.wrapping_add(50_000_000)],
            32,
            31,
        );
    }
    let mut sum = 0i32;
    elapsed = 0;
    for i in 0..n {
        sum = sum.wrapping_add(durations[i]);
    }
    for e in ev.iter().take(n_ev) {
        if e.kind != 1
            && e.kind != 2
            && e.kind != 3
            && e.kind != 4
            && e.kind != 5
            && e.kind != 6
            && e.kind != 7
            && e.kind != 8
        {
            emit_format(
                m,
                a.out,
                a.capacity,
                NOTE_FMT_MRK1,
                &[e.kind
                    .wrapping_add(20_000_000)
                    .wrapping_add(elapsed.wrapping_mul(625).wrapping_mul(16))],
                32,
                31,
            );
            elapsed = elapsed.wrapping_add(e.duration);
            if sum <= elapsed.wrapping_mul(10) {
                break;
            }
        }
    }
    pause_index = 0;
    let mut j = 0usize;
    let mut k = 0usize;
    elapsed = 0;
    char_count = 0;
    let mut command = 0i32;
    for i in 0..n {
        let following = i + 1 < n && durations[i + 1] > 0;
        if durations[i] > 0 {
            elapsed = elapsed.wrapping_add(durations[i]);
            let second = m.half(
                a.text.wrapping_add(((starts[i] + 1) as u32).wrapping_mul(2)),
            ) as u32;
            if !direct || (second != 0x30fc && second != 0x7e) {
                if j < intonation_count && intonation_starts[j] < elapsed {
                    let intonation = intonation_types[j];
                    if intonation == 2 {
                        command = 9;
                    } else if intonation == 3 {
                        command = 10;
                    } else if intonation == 4 {
                        command = 11;
                    } else if intonation == 5 {
                        command = 8;
                    }
                    emit_format(m, a.out, a.capacity, NOTE_FMT_MRK1, &[command], 32, 31);
                    j += 1;
                } else if intonation_count == 0 && i == 0 {
                    command = 9;
                    emit_format(m, a.out, a.capacity, NOTE_FMT_MRK1, &[9], 32, 31);
                }
            } else {
                command = 10;
                emit_format(m, a.out, a.capacity, NOTE_FMT_MRK1, &[10], 32, 31);
            }
            char_count = char_count.wrapping_add(durations[i]);
            if k < control_count && control_starts[k] < char_count {
                let control = control_types[k];
                if control == 59 {
                    command = control_values[k].wrapping_add(70_000_000);
                } else if control == 60 {
                    command = control_values[k].wrapping_add(110_000_000);
                } else if control == 61 {
                    command = control_values[k].wrapping_add(120_000_000);
                }
                emit_format(m, a.out, a.capacity, NOTE_FMT_MRK1, &[command], 32, 31);
                k += 1;
            }
            if durations[i] > 0 {
                command = durations[i].wrapping_add(150_000_000);
                emit_format(m, a.out, a.capacity, NOTE_FMT_MRK1, &[command], 32, 31);
            }
        }
        if lengths[i] > 20 {
            emit_format(m, a.out, a.capacity, NOTE_FMT_MRK6, &[], 32, 31);
            append_guest(
                m,
                a.out,
                cap,
                a.text.wrapping_add((starts[i] as u32).wrapping_mul(2)),
                20,
            );
            append_guest(m, a.out, cap, NOTE_SPACE, 1);
            emit_format(m, a.out, a.capacity, NOTE_FMT_MRK7, &[], 32, 31);
        } else if special[i] {
            if lengths[i] >= 5 {
                emit_format(m, a.out, a.capacity, NOTE_FMT_MRK30, &[], 32, 31);
            }
            append_guest(
                m,
                a.out,
                cap,
                a.text.wrapping_add((starts[i] as u32).wrapping_mul(2)),
                lengths[i],
            );
            if lengths[i] >= 5 {
                emit_format(m, a.out, a.capacity, NOTE_FMT_MRK31, &[], 32, 31);
            }
            if following {
                append_guest(m, a.out, cap, NOTE_SPACE, 1);
            }
        } else {
            random_fallback_guest(m, a.out, cap, lengths[i] as i32);
        }
        if pause_ends[pause_index] == i as i32 {
            if following {
                if pause_durations[pause_index] > 0 {
                    emit_format(m, a.out, a.capacity, NOTE_FMT_PAUSE, &[5], 32, 31);
                    command = pause_durations[pause_index].wrapping_add(210_000_000);
                    emit_format(m, a.out, a.capacity, NOTE_PAUSE_FMT, &[command], 32, 31);
                }
                pause_index += 1;
            } else {
                for limit in pause_index..pause_count {
                    remainder = remainder.wrapping_add(pause_durations[limit]);
                }
            }
        }
    }
    if a.add_pause != 0 {
        emit_format(
            m,
            a.out,
            a.capacity,
            NOTE_FMT_PAUSE,
            &[(a.pause >> 4).wrapping_add(50) as i32],
            32,
            31,
        );
    }
    if a.tail_pause.wrapping_add(remainder) > 0 {
        emit_format(
            m,
            a.out,
            a.capacity,
            NOTE_PAUSE_FMT,
            &[a.tail_pause.wrapping_add(remainder).wrapping_add(210_000_000)],
            32,
            31,
        );
    }
    if current_volume != initial_volume {
        emit_format(m, a.out, a.capacity, NOTE_VOLUME_FMT, &[initial_volume], 32, 31);
    }
    emit_format(m, a.out, a.capacity, NOTE_FMT_EOS, &[], 32, 31);
    a.out.wrapping_add((note_out_len(m, a.out) as u32).wrapping_mul(2))
}
