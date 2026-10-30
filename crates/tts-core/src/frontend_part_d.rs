//! Caller-owned APDB expansion state.
fn word(m: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(m[p as usize..p as usize + 4].try_into().unwrap())
}
fn half(m: &[u8], p: u32) -> u16 {
    u16::from_le_bytes(m[p as usize..p as usize + 2].try_into().unwrap())
}
fn store(m: &mut [u8], p: u32, v: u32) {
    m[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn store16(m: &mut [u8], p: u32, v: u16) {
    m[p as usize..p as usize + 2].copy_from_slice(&v.to_le_bytes());
}
fn wide_len(m: &[u8], p: u32) -> u32 {
    let mut n = 0;
    while half(m, p + n * 2) != 0 {
        n += 1;
    }
    n
}
fn byte_len(m: &[u8], p: u32) -> u32 {
    m[p as usize..].iter().position(|&c| c == 0).unwrap() as u32
}
fn space(c: u16) -> bool {
    matches!(c, 0 | 9 | 10 | 13 | 32)
}
fn contains(m: &[u8], p: u32, c: u16) -> bool {
    let c = c as u8;
    m[p as usize..p as usize + byte_len(m, p) as usize + 1].contains(&c)
}
fn move_bytes(m: &mut [u8], dst: u32, src: u32, n: u32) {
    m.copy_within(src as usize..(src + n) as usize, dst as usize);
}
fn widen(m: &mut [u8], dst: u32, src: u32, append: bool) {
    let mut at = if append { wide_len(m, dst) } else { 0 };
    let mut i = 0;
    loop {
        let c = m[(src + i) as usize];
        store16(m, dst + at * 2, c as u16);
        if c == 0 {
            break;
        }
        at += 1;
        i += 1;
    }
}
/// Outputs of fn_279988's caller-supplied marker inspection.
#[derive(Default)]
pub struct OriginalPromptSpan {
    pub expected: u16,
    pub missing: u16,
    pub actual: u16,
    pub start: u32,
    pub end: u32,
}
/// A lossless fn_17486c vararg. Null is distinct from an empty string.
pub enum ExpansionLogArg<'a> {
    Null,
    String(&'a [u8]),
    Number(u32),
}
use ExpansionLogArg::{Null, Number as Num, String as Str};
/// Heap, marker decoding and diagnostics supplied by the engine host.
pub trait OriginalExpansionServices {
    /// fn_17497c. Queried at every ROM call site, not cached.
    fn logging_enabled(&mut self, logger: u32) -> bool {
        let _ = logger;
        false
    }
    /// fn_173fd8.
    fn diagnostic_level(&mut self, logger: u32) -> u32 {
        let _ = logger;
        0
    }
    /// fn_17486c. Formats and arguments are passed without host formatting.
    fn log(&mut self, logger: u32, site: u32, format: &[u8], args: &[ExpansionLogArg<'_>]) {
        let _ = (logger, site, format, args);
    }
    /// fn_1748a0. Capture the payload now, before subsequent mutations.
    fn payload(
        &mut self,
        mem: &[u8],
        logger: u32,
        site: u32,
        label: &[u8],
        media_type: &[u8],
        pointer: u32,
        bytes: u32,
    ) {
        let _ = (mem, logger, site, label, media_type, pointer, bytes);
    }
    /// fn_1738e8 accepted-match diagnostics. Names remain arena pointers.
    fn accepted(
        &mut self,
        mem: &[u8],
        logger: u32,
        source: &[u8],
        level: u32,
        flags: u32,
        format: &[u8],
        names: [u32; 2],
        lo: u32,
        hi: u32,
        left: &[u8],
        right: &[u8],
    ) {
        let _ = (
            mem, logger, source, level, flags, format, names, lo, hi, left, right,
        );
    }
    fn span(
        &mut self,
        mem: &mut [u8],
        text: u32,
        entries: u32,
        count: u32,
        index: u32,
        out: &mut OriginalPromptSpan,
    ) -> bool;
    fn reallocate(&mut self, heap: u32, entries: u32, bytes: u32) -> u32;
    /// Reallocate caller memory, including copying live bytes if the block moves.
    fn reallocate_state(&mut self, mem: &mut [u8], heap: u32, entries: u32, bytes: u32) -> u32 {
        let _ = mem;
        self.reallocate(heap, entries, bytes)
    }
    fn allocate(&mut self, heap: u32, bytes: u32) -> u32;
    fn register(&mut self, mem: &mut [u8], registry: u32, pointer: u32) -> bool;
    fn free(&mut self, heap: u32, pointer: u32);
    fn node(&mut self, heap: u32, kind: u32, start: u32, length: u32) -> u32;
    fn error(&mut self, logger: u32, code: u32);
    /// fn_1738e8. Pointers in args address caller memory; strings exclude NUL.
    fn diagnostic(
        &mut self,
        mem: &[u8],
        logger: u32,
        source: &[u8],
        level: u32,
        flags: u32,
        format: &[u8],
        args: &[u32],
    ) {
        let _ = (mem, logger, source, level, flags, format, args);
    }
    /// fn_17486c's string-only rejection messages, including unconditional calls.
    fn log_strings(&mut self, logger: u32, site: u32, format: &[u8], args: &[&[u8]]) {
        let _ = (logger, site, format, args);
    }
}
fn rejection(
    mem: &[u8],
    logger: u32,
    rec: u32,
    lo: u32,
    hi: u32,
    normalized: bool,
    pitch: bool,
    api: &mut impl OriginalExpansionServices,
) {
    let format = if pitch {
        &b"    Match with [%s::%s] at %u-%u deactivated because of ESC pitch, volume or rate"[..]
    } else {
        &b"    Match with [%s::%s] at %u-%u deactivated because of ESC tn"[..]
    };
    api.diagnostic(
        mem,
        logger,
        b"DOMAIN_MNGR",
        5,
        0,
        format,
        &[word(mem, word(mem, rec) + 88), word(mem, rec + 8), lo, hi],
    );
    let detail: &[u8] = if pitch {
        b"AP DEACTIVATED BECAUSE OF ESC PITCH, VOLUME OR RATE"
    } else {
        b"AP DEACTIVATED BECAUSE OF ESC TN"
    };
    api.log_strings(
        logger,
        if normalized { 0x11b } else { 0xdf },
        b"%s%s %s%s",
        &[b"RES", b"NOMATCH", b"INFO", detail],
    );
}
fn arena_string(mem: &[u8], p: u32) -> &[u8] {
    &mem[p as usize..(p + byte_len(mem, p)) as usize]
}
const MARKER_TYPE: &[u8] = b"application/x-realspeak-markers-pp;version=4.0";
const WIDE_TYPE: &[u8] = b"text/plain;charset=utf-16";
const NORMAL_TYPE: &[u8] = b"text/plain;charset=depes";
const ACCEPT_ORIGINAL: &[u8] =
    b"    Usermatch with [%s::%s], begPos %ld, endPos %ld, begCtxt %s, endCtxt %s";
const ACCEPT_NORMAL: &[u8] =
    b"    Match with [%s::%s], begPos %ld, endPos %ld, begCtxt %s, endCtxt %s";
fn class_count(a: &[u8]) -> usize {
    a.iter().position(|&c| c == 0).unwrap() + 1
}
/// The fn_279c28 context summary, in a fixed local buffer, without core::fmt.
fn summary(flag: bool, kind: u8, ch: u8, classes: &[u8], mark: u8) -> [u8; 128] {
    const NAMES: [&[u8]; 8] = [
        b"[*]", b"[S]", b"[P]", b"[S|P]", b"[...]", b"[p]", b"[P|...]", b"[PG]",
    ];
    fn append(out: &mut [u8], s: &[u8]) {
        let n = out.iter().position(|&c| c == 0).unwrap();
        out[n..n + s.len()].copy_from_slice(s);
        out[n + s.len()] = 0;
    }
    fn class(out: &mut [u8], kind: u8, ch: u8) {
        if kind == 5 {
            append(out, &[b'[', ch, b']']);
        } else {
            append(out, NAMES[kind as usize]);
        }
    }
    let mut out = [0; 128];
    class(&mut out, kind, ch);
    append(
        &mut out,
        if flag {
            b" matches ("
        } else {
            b" does not match ("
        },
    );
    for (i, &c) in classes.iter().enumerate() {
        if i != 0 {
            append(&mut out, b" ");
        }
        class(&mut out, c, mark);
    }
    append(&mut out, b")");
    out
}
fn summary_bytes(a: &[u8]) -> &[u8] {
    &a[..a.iter().position(|&c| c == 0).unwrap()]
}
fn context_log(
    mem: &[u8],
    rec: u32,
    logger: u32,
    site: u32,
    matched: bool,
    a: &[u8],
    mark: u8,
    api: &mut impl OriginalExpansionServices,
) {
    if api.logging_enabled(logger) {
        let right = matches!(site, 0xde | 0x11a) as u32;
        let out = summary(
            matched,
            mem[(rec + 14 + right) as usize],
            mem[(rec + 12 + right) as usize],
            a,
            mark,
        );
        api.log(
            logger,
            site,
            b"%s%s %s%s",
            &[
                Str(b"RES"),
                Str(if matched { b"MATCH" } else { b"NOMATCH" }),
                Str(b"INFO"),
                Str(summary_bytes(&out)),
            ],
        );
    }
}
fn original_bytes(mem: &[u8], lk: u32, sl: u32, base: u32) -> u32 {
    let mut last = lk;
    while word(mem, last + 24) != 0 {
        last = word(mem, last + 24);
    }
    if sl != 0 && half(mem, sl) != 0 {
        word(mem, last + 4)
            .wrapping_add(word(mem, last + 8))
            .wrapping_sub(base)
    } else {
        0
    }
}
/// fn_279d54 / EU fn_279f10 / KR fn_279fbc, including host logging.
pub fn expand_original_record(
    mem: &mut [u8],
    rec: u32,
    pp: u32,
    fp: u32,
    sl: u32,
    lk: u32,
    pad: u32,
    api: &mut impl OriginalExpansionServices,
) -> u32 {
    if word(mem, rec + 32) == 0 {
        return 0;
    }
    let logger = word(mem, word(mem, word(mem, word(mem, rec)) + 8) + 16);
    let base = word(mem, word(mem, fp) + 12);
    let text = word(mem, pp);
    let idx = (word(mem, lk + 4).wrapping_sub(base) as u16 as u32) >> 1;
    if api.logging_enabled(logger) {
        let bytes = original_bytes(mem, lk, sl, base);
        api.log(
            logger,
            0xd9,
            b"%s%s %s%s %s%d %s%d",
            &[
                Str(b"BEG"),
                Null,
                Str(b"NAME"),
                Str(arena_string(mem, word(mem, rec + 8))),
                Str(b"IDXBEG"),
                Num(idx),
                Str(b"IDXEND"),
                Num(word(mem, lk + 4)
                    .wrapping_add(word(mem, lk + 8))
                    .wrapping_sub(base)
                    >> 1),
            ],
        );
        api.payload(mem, logger, 0xda, b"", WIDE_TYPE, text, bytes);
        api.payload(
            mem,
            logger,
            0xda,
            b"",
            MARKER_TYPE,
            word(mem, fp),
            half(mem, sl) as u32 * 24,
        );
    }
    let mut matched = false;
    let rc = original_body(mem, rec, pp, fp, sl, lk, pad, api, &mut matched);
    if api.logging_enabled(logger) {
        api.payload(
            mem,
            logger,
            0xdb,
            b"",
            WIDE_TYPE,
            text,
            original_bytes(mem, lk, sl, word(mem, word(mem, fp) + 12)),
        );
        api.payload(
            mem,
            logger,
            0xdb,
            b"",
            MARKER_TYPE,
            word(mem, fp),
            half(mem, sl) as u32 * 24,
        );
        api.log(
            logger,
            0xd9,
            b"%s%s %s%s %s%u",
            &[
                Str(b"END"),
                Null,
                Str(b"RES"),
                Str(if matched { b"MATCH" } else { b"NOMATCH" }),
                Str(b"RETC"),
                Num(rc),
            ],
        );
    }
    rc
}
fn classes(begin: bool, mark: u16, sentence: bool) -> [u8; 6] {
    let mut a = [0u8; 6];
    let mut n = 0;
    let mut push = |v| {
        a[n] = v;
        n += 1;
    };
    if begin {
        push(1);
        push(3);
    } else if mark == 0 || !sentence {
        push(4);
        push(6);
    } else {
        push(2);
        push(3);
        push(6);
    }
    if mark != 0 {
        push(5);
    }
    a
}
fn class_matches(class: u8, ch: u8, mark: u16, classes: [u8; 6]) -> bool {
    class == 0 || (classes.contains(&class) && (class != 5 || ch == mark as u8))
}
/// fn_279d54 / EU fn_279f10 / KR fn_279fbc.
/// Match original UTF-16 orthography, insert an AP marker, remove the
/// matched text and split the caller's node chain. Diagnostics are host-owned.
fn original_body(
    mem: &mut [u8],
    rec: u32,
    pp: u32,
    fp: u32,
    sl: u32,
    lk: u32,
    pad: u32,
    api: &mut impl OriginalExpansionServices,
    last_match: &mut bool,
) -> u32 {
    let ctx = word(mem, rec);
    let obj = word(mem, ctx);
    let eng = word(mem, obj + 8);
    let heap = word(mem, eng + 4);
    let logger = word(mem, eng + 16);
    let pattern = word(mem, rec + 32);
    if pattern == 0 {
        return 0;
    }
    let text = word(mem, pp);
    let mut recs = word(mem, fp);
    let len = wide_len(mem, text) as u16 as u32;
    let base = word(mem, recs + 12);
    let span = word(mem, lk + 4).wrapping_sub(base) as u16 as u32;
    let mlen = wide_len(mem, pattern) as u16 as u32;
    let mut idx = span >> 1;
    let set = word(mem, ctx + 104);
    let sentence = word(mem, ctx + 100);
    let mut prompt = OriginalPromptSpan::default();
    let mut matched_length = 0;
    let (mut begin, mut finish) = (0, 0);
    let mut consumed;
    let (mut ca, mut cb) = ([0u8; 6], [0u8; 6]);
    let (mut na, mut nb) = (0, 0);
    let (mut lm, mut rm) = (0, 0);
    'search: loop {
        if ((mlen + idx) * 2) as i32 > word(mem, lk + 8).wrapping_add(span) as i16 as i32
            || half(mem, text + idx * 2) == 0
        {
            return 0;
        }
        let mut pos = idx;
        let mut ri = 0;
        let mut missing = 0;
        let mut current = 0;
        let mut matched = true;
        loop {
            if len <= pos {
                matched_length = pos.wrapping_sub(idx) & 65535;
                break;
            }
            if missing != 0
                && pos + 1 < len
                && half(mem, text + pos * 2) == 32
                && half(mem, text + pos * 2 + 2) == 21
            {
                missing = (missing - 1) & 65535;
                pos = (pos + 2) & 65535;
                continue;
            }
            let n = half(mem, sl) as u32;
            let mut found = n;
            let key = base.wrapping_add(pos * 2);
            while ri < n {
                let e = recs + ri * 24;
                let q = word(mem, e + 12);
                if key < q {
                    break;
                }
                if key == q && half(mem, e) == 34 {
                    found = ri;
                    break;
                }
                ri = (ri + 1) & 65535;
            }
            if found < n {
                if !api.span(mem, text, recs, n, found, &mut prompt) {
                    matched = false;
                    break;
                }
                if prompt.missing != 0 {
                    missing = prompt.actual as u32;
                }
            }
            if mlen <= current {
                matched_length = pos.wrapping_sub(idx) & 65535;
                break;
            }
            if half(mem, pattern + current * 2) != half(mem, text + pos * 2) {
                matched = false;
            }
            current = (current + 1) & 65535;
            pos = (pos + 1) & 65535;
            if !matched {
                break;
            }
        }
        if matched && prompt.end < pos && missing != 0 {
            while pos < len && half(mem, text + pos * 2) != 21 {
                pos = (pos + 1) & 65535;
            }
            if pos == len {
                return 0;
            }
            pos = pos.wrapping_sub(1).wrapping_add(missing * 2) & 65535;
            if len < pos {
                return 0;
            }
        }
        let mut tail = (matched_length + idx) & 65535;
        consumed = pos.wrapping_sub(idx) & 65535;
        if matched {
            while tail < len && !space(half(mem, text + tail * 2)) {
                if !contains(mem, set, half(mem, text + tail * 2)) {
                    matched = false;
                    break;
                }
                tail = (tail + 1) & 65535;
            }
            tail = (idx + consumed) & 65535;
            if matched {
                for i in 0..half(mem, sl) as u32 {
                    if half(mem, recs + i * 24) == 34 {
                        if !api.span(mem, text, recs, half(mem, sl) as u32, i, &mut prompt) {
                            matched = false;
                            break;
                        }
                        if prompt.missing != 0
                            && ((prompt.start < idx && idx <= prompt.end)
                                || (prompt.start < tail && tail <= prompt.end))
                        {
                            matched = false;
                            break;
                        }
                    }
                }
            }
        }
        if api.logging_enabled(logger) {
            api.log(
                logger,
                0xdc,
                b"%s%d %s%d %s%s",
                &[
                    Str(b"IDXBEG"),
                    Num(idx),
                    Str(b"IDXEND"),
                    Num(mlen + idx),
                    Str(b"RES"),
                    Str(if matched { b"MATCH" } else { b"NOMATCH" }),
                ],
            );
            if matched && mem[(rec + 14) as usize] == 0 && mem[(rec + 15) as usize] == 0 {
                for site in [0xdd, 0xde] {
                    api.log(
                        logger,
                        site,
                        b"%s%s %s%s",
                        &[
                            Str(b"RES"),
                            Str(b"MATCH"),
                            Str(b"INFO"),
                            Str(b"[*] matches"),
                        ],
                    );
                }
            }
        }
        if matched {
            begin = idx;
            finish = (consumed + idx) & 65535;
            if mem[(rec + 14) as usize] != 0 || mem[(rec + 15) as usize] != 0 {
                let n = half(mem, sl) as u32;
                let mut after = finish;
                let mut marker = false;
                for i in 0..n {
                    if half(mem, recs + i * 24) == 40
                        && base.wrapping_add(finish * 2) <= word(mem, recs + i * 24 + 12)
                    {
                        marker = true;
                        break;
                    }
                }
                let mut at_end = false;
                if !marker {
                    while after < wide_len(mem, text) {
                        let c = half(mem, text + after * 2);
                        if !space(c) {
                            // The ROM compares the full UTF-16 unit with each set byte here.
                            let bytes =
                                &mem[set as usize..set as usize + byte_len(mem, set) as usize];
                            if !bytes.iter().any(|&b| b as u16 == c) {
                                break;
                            }
                        }
                        after = (after + 1) & 65535;
                    }
                    at_end = after == wide_len(mem, text);
                }
                let mut left = idx as i32 - 1;
                while left >= 0 && space(half(mem, text + (left as u32) * 2)) {
                    left -= 1;
                }
                let mut left_mark = 0u16;
                if left as i16 >= 0 {
                    let c = half(mem, text + (left as u32) * 2);
                    if mem[set as usize..set as usize + byte_len(mem, set) as usize]
                        .iter()
                        .any(|&b| b as u16 == c)
                    {
                        left_mark = c;
                        for i in 0..n {
                            let e = recs + i * 24;
                            let q = word(mem, e + 12);
                            if half(mem, e) == 40
                                && base.wrapping_add((left as u32) * 2) <= q
                                && q <= base.wrapping_add(idx * 2)
                            {
                                left_mark = 0;
                                break;
                            }
                        }
                    }
                }
                let mut right = (matched_length + idx) as u32;
                let mut right_mark = 0u16;
                if (right as i16) < (wide_len(mem, text) as i16) {
                    while space(half(mem, text + right * 2)) {
                        right += 1;
                    }
                    let c = half(mem, text + right * 2);
                    if mem[set as usize..set as usize + byte_len(mem, set) as usize]
                        .iter()
                        .any(|&b| b as u16 == c)
                    {
                        right_mark = c;
                        for i in 0..n {
                            let e = recs + i * 24;
                            let q = word(mem, e + 12);
                            if half(mem, e) == 40
                                && base.wrapping_add((matched_length + idx) * 2) <= q
                                && q <= base.wrapping_add(right * 2)
                            {
                                right_mark = 0;
                                break;
                            }
                        }
                    }
                }
                ca = classes(idx == 0, left_mark, contains(mem, sentence, left_mark));
                cb = classes(at_end, right_mark, contains(mem, sentence, right_mark));
                na = class_count(&ca);
                nb = class_count(&cb);
                lm = left_mark as u8;
                rm = right_mark as u8;
                matched = class_matches(
                    mem[(rec + 14) as usize],
                    mem[(rec + 12) as usize],
                    left_mark,
                    ca,
                );
                context_log(mem, rec, logger, 0xdd, matched, &ca[..na], lm, api);
                if matched {
                    matched = class_matches(
                        mem[(rec + 15) as usize],
                        mem[(rec + 13) as usize],
                        right_mark,
                        cb,
                    );
                    context_log(mem, rec, logger, 0xde, matched, &cb[..nb], rm, api);
                }
            }
            if matched && mem[(rec + 4) as usize] == 2 {
                let lo = base.wrapping_add(begin * 2);
                let hi = base.wrapping_add(finish * 2);
                for i in 0..half(mem, sl) as u32 {
                    let e = recs + i * 24;
                    let q = word(mem, e + 12);
                    if matches!(half(mem, e), 16 | 18 | 19 | 24) && lo < q && q < hi {
                        matched = false;
                        rejection(mem, logger, rec, lo, hi, false, true, api);
                        break;
                    }
                }
            }
        }
        *last_match = matched;
        if matched {
            break 'search;
        }
        while half(mem, text + idx * 2) != 0 && !space(half(mem, text + idx * 2)) {
            idx = (idx + 1) & 65535;
        }
        while half(mem, text + idx * 2) != 0 && space(half(mem, text + idx * 2)) {
            idx = (idx + 1) & 65535;
        }
    }
    if api.diagnostic_level(logger) > 4 {
        let left = summary(
            true,
            mem[(rec + 14) as usize],
            mem[(rec + 12) as usize],
            &ca[..na],
            lm,
        );
        let right = summary(
            true,
            mem[(rec + 15) as usize],
            mem[(rec + 13) as usize],
            &cb[..nb],
            rm,
        );
        api.accepted(
            mem,
            logger,
            b"DOMAIN_MNGR",
            5,
            0,
            ACCEPT_ORIGINAL,
            [word(mem, ctx + 88), word(mem, rec + 8)],
            base.wrapping_add(begin * 2),
            base.wrapping_add(finish * 2),
            summary_bytes(&left),
            summary_bytes(&right),
        );
    }
    let mut i = 0u32;
    while i < half(mem, sl) as u32 {
        if half(mem, recs + i * 24) == 34
            && api.span(mem, text, recs, half(mem, sl) as u32, i, &mut prompt)
            && begin <= prompt.start
            && prompt.end <= finish
        {
            move_bytes(
                mem,
                recs + i * 24,
                recs + (i + 1) * 24,
                (half(mem, sl) as u32 - i - 1) * 24,
            );
            store16(mem, sl, half(mem, sl).wrapping_sub(1));
        } else {
            i = (i + 1) & 65535;
        }
    }
    i = 0;
    let position = word(mem, lk + 4).wrapping_add(idx * 2).wrapping_sub(span);
    while i < half(mem, sl) as u32 && word(mem, recs + i * 24 + 12) <= position {
        i = (i + 1) & 65535;
    }
    let fail = |api: &mut dyn OriginalExpansionServices| {
        api.error(logger, 0xb798);
        0x88e0200a
    };
    let p = api.reallocate_state(mem, heap, recs, (half(mem, sl) as u32 + 1) * 24);
    if p == 0 {
        return fail(api);
    }
    recs = p;
    store(mem, fp, p);
    if i < half(mem, sl) as u32 {
        move_bytes(
            mem,
            p + (i + 1) * 24,
            p + i * 24,
            (half(mem, sl) as u32 - i) * 24,
        );
    }
    store16(mem, sl, half(mem, sl).wrapping_add(1));
    let e = recs + i * 24;
    mem[e as usize..e as usize + 24].fill(0);
    store16(mem, e, 40);
    if i != 0 {
        store(mem, e + 4, word(mem, e - 20));
    }
    store(mem, e + 12, position);
    store(mem, e + 8, mlen * 2);
    store(mem, e + 16, mlen * 2);
    let name = word(mem, ctx + 88);
    let record_name = word(mem, rec + 8);
    let p = api.allocate(
        heap,
        (byte_len(mem, name) + byte_len(mem, record_name) + 3) * 2,
    );
    store(mem, e + 20, p);
    if p == 0 {
        return fail(api);
    }
    if !api.register(mem, word(mem, obj + 44), e + 20) {
        api.free(heap, word(mem, e + 20));
        store(mem, e + 20, 0);
        return fail(api);
    }
    let p = word(mem, e + 20);
    widen(mem, p, name, false);
    let at = p + wide_len(mem, p) * 2;
    store16(mem, at, 58);
    store16(mem, at + 2, 58);
    store16(mem, at + 4, 0);
    widen(mem, p, record_name, true);
    let n = wide_len(mem, text);
    move_bytes(
        mem,
        text + idx * 2,
        text + (consumed + idx) * 2,
        (n - idx - consumed + 1) * 2,
    );
    store16(
        mem,
        pad,
        half(mem, pad).wrapping_sub((consumed as u16).wrapping_mul(2)),
    );
    let mark = word(mem, e + 12);
    for j in i + 1..half(mem, sl) as u32 {
        let p = recs + j * 24 + 12;
        let q = word(mem, p);
        let value = if (q as i16).wrapping_sub(mark as i16) < ((consumed * 2) as i16) {
            mark
        } else {
            q.wrapping_sub(consumed * 2)
        };
        store(mem, p, value);
    }
    let mut node = word(mem, lk + 24);
    while node != 0 {
        store(
            mem,
            node + 4,
            word(mem, node + 4).wrapping_sub(consumed * 2),
        );
        node = word(mem, node + 24);
    }
    let next = api.node(
        heap,
        1,
        position,
        word(mem, lk + 8)
            .wrapping_sub(idx * 2)
            .wrapping_add(span)
            .wrapping_sub(consumed * 2),
    );
    if next == 0 {
        return fail(api);
    }
    store(mem, next + 24, word(mem, lk + 24));
    let n = (idx * 2).wrapping_sub(span);
    store(mem, lk + 8, n);
    if n == 0 {
        store(mem, lk, 0);
    }
    store(mem, lk + 24, next);
    0
}

/// Host prompt properties and normalized-text replacement.
pub trait NormalizedExpansionServices: OriginalExpansionServices {
    fn first(&mut self, handle: [u32; 2], kind: u32, parent: u16, out: &mut u16) -> u32;
    fn next(&mut self, handle: [u32; 2], token: u16, out: &mut u16) -> u32;
    fn property(&mut self, handle: [u32; 2], token: u16, property: u32, out: &mut u16) -> u32;
    fn string(&mut self, handle: [u32; 2], token: u16, out: &mut u32) -> u32;
    fn allocate_copy(&mut self, heap: u32, bytes: u32) -> u32;
    fn named(&mut self, mem: &[u8], name: u32, wide: Option<u32>) -> bool;
    fn replace(
        &mut self,
        mem: &mut [u8],
        rec: u32,
        current: u16,
        consumed: u32,
        base: u32,
        text: u32,
        entries: u32,
        count: u32,
        index: u32,
        node: u32,
        handle: [u32; 2],
        pad: u32,
    ) -> u32;
}
fn byte_space(c: u8) -> bool {
    matches!(c, 0 | 32 | 95)
}
fn byte_classes(begin: bool, mark: u16, sentence: bool) -> [u8; 7] {
    let mut a = [0; 7];
    let mut n = 0;
    let mut push = |c| {
        a[n] = c;
        n += 1;
    };
    if begin {
        push(1);
        push(3);
    } else if mark == 0 || !sentence {
        push(4);
        push(7);
        push(6);
    } else {
        push(2);
        push(3);
        push(6);
    }
    if mark != 0 {
        push(5);
    }
    a
}
fn normalized_span(
    mem: &[u8],
    text: u32,
    recs: u32,
    n: u32,
    mut index: u32,
    p: &mut OriginalPromptSpan,
) -> bool {
    if n == 0 || index >= n || half(mem, recs) != 0x4000 || half(mem, recs + index * 24) != 34 {
        return false;
    }
    p.expected = 1;
    p.missing = 0;
    p.actual = 0;
    let selected = recs + index * 24;
    let origin = word(mem, recs + 12);
    let name = word(mem, selected + 20);
    let length = wide_len(mem, name);
    let mut i = 1u16;
    if length.wrapping_sub(1) > 1 {
        loop {
            if matches!(half(mem, name + i as u32 * 2), 95 | 35) {
                p.expected = p.expected.wrapping_add(1);
            }
            i = i.wrapping_add(1);
            if i as u32 >= length.wrapping_sub(1) {
                break;
            }
        }
    }
    p.start = word(mem, selected + 12).wrapping_sub(origin);
    let mut mark = 0;
    loop {
        index = (index + 1) & 65535;
        if n <= index {
            break;
        }
        let e = recs + index * 24;
        if matches!(half(mem, e), 21 | 34) && word(mem, e + 12) != word(mem, selected + 12) {
            mark = word(mem, e + 12).wrapping_sub(origin);
            break;
        }
    }
    if index == n {
        mark = byte_len(mem, text);
    }
    let mut end = mark.wrapping_sub(1);
    while p.start < end
        && (byte_space(mem[(text + end) as usize]) || mem[(text + end) as usize] == 21)
    {
        end = end.wrapping_sub(1);
    }
    while p.start < end {
        if byte_space(mem[(text + end) as usize]) {
            end = end.wrapping_add(1);
            break;
        }
        end = end.wrapping_sub(1);
    }
    p.end = end;
    for i in p.start..mark {
        if mem[(text + i) as usize] == 21 {
            p.actual = p.actual.wrapping_add(1);
        }
    }
    if p.actual <= p.expected {
        p.missing = p.expected.wrapping_sub(p.actual);
        true
    } else {
        false
    }
}
fn prompt_tokens(
    mem: &mut [u8],
    heap: u32,
    list: u32,
    value: u32,
    right: bool,
    api: &mut impl NormalizedExpansionServices,
) -> bool {
    let p = api.allocate_copy(heap, byte_len(mem, list) + 1);
    let n = byte_len(mem, list) + 1;
    move_bytes(mem, p, list, n);
    let mut start = p;
    let mut found = false;
    loop {
        let n = byte_len(mem, start);
        let split = mem[start as usize..(start + n) as usize]
            .iter()
            .position(|&c| c == 32)
            .map(|i| start + i as u32);
        if let Some(s) = split {
            mem[s as usize] = 0;
        }
        let n = byte_len(mem, start);
        let v = byte_len(mem, value);
        if right {
            if n <= v {
                for i in 0..=v - n {
                    if mem[(value + i) as usize..(value + i + n) as usize]
                        == mem[start as usize..(start + n) as usize]
                    {
                        found = mem[value as usize..(value + i) as usize]
                            .iter()
                            .all(|&c| matches!(c, 39 | 34 | 50));
                        break;
                    }
                }
            }
        } else if n <= v {
            found = mem[(value + v - n) as usize..(value + v) as usize]
                == mem[start as usize..(start + n) as usize];
        }
        if found {
            break;
        }
        match split {
            None => break,
            Some(s) => start = s + 1,
        }
    }
    api.free(heap, p);
    found
}
fn normalized_property_match(
    mem: &mut [u8],
    ctx: u32,
    heap: u32,
    handle: [u32; 2],
    class: u8,
    key: u8,
    current: u32,
    right: bool,
    api: &mut impl NormalizedExpansionServices,
) -> Result<bool, u32> {
    if class != 7 {
        return Ok(true);
    }
    let list = word(mem, word(mem, ctx + 120) + key as u32 * 4);
    if list == 0 {
        return Ok(false);
    }
    let mut parent = 0;
    let mut token = 0;
    let rc = api.first(handle, 1, 0, &mut parent);
    if rc >= 0x80000000 {
        return Err(rc);
    }
    let rc = api.first(handle, 2, parent, &mut token);
    if rc >= 0x80000000 {
        return Err(rc);
    }
    while token != 0 {
        let mut begin = 0;
        let mut end = 0;
        let rc = api.property(handle, token, 1, &mut begin);
        if rc >= 0x80000000 {
            return Err(rc);
        }
        if !right {
            let rc = api.property(handle, token, 2, &mut end);
            if rc >= 0x80000000 {
                return Err(rc);
            }
        }
        if (right && current == (begin as u32).wrapping_sub(1))
            || (!right && current == end as u32 + 1)
        {
            if right {
                let rc = api.property(handle, token, 2, &mut end);
                if rc >= 0x80000000 {
                    return Err(rc);
                }
            }
            let mut kind = 0;
            let rc = api.property(handle, token, 0, &mut kind);
            if rc >= 0x80000000 {
                return Err(rc);
            }
            if matches!(kind as u8, 4 | 9 | 10 | 11 | 12 | 13 | 14) {
                let mut value = 0;
                let rc = api.string(handle, token, &mut value);
                if rc >= 0x80000000 {
                    return Err(rc);
                }
                return Ok(prompt_tokens(mem, heap, list, value, right, api));
            }
        }
        let rc = api.next(handle, token, &mut token);
        if rc >= 0x80000000 {
            return Err(rc);
        }
    }
    Ok(false)
}
/// fn_27b904 / EU fn_27bac0 / KR fn_27bb6c, including host logging.
pub fn expand_normalized_record(
    mem: &mut [u8],
    rec: u32,
    name: u32,
    pp: u32,
    fp: u32,
    sl: u32,
    lk: u32,
    handle: [u32; 2],
    pad: u32,
    api: &mut impl NormalizedExpansionServices,
) -> u32 {
    let logger = word(mem, word(mem, word(mem, word(mem, rec)) + 8) + 16);
    let text = word(mem, pp);
    let base = word(mem, word(mem, fp) + 12);
    let idx = word(mem, lk + 4).wrapping_sub(base) as u16 as u32;
    if api.logging_enabled(logger) {
        api.log(
            logger,
            0x115,
            b"%s%s %s%s %s%d %s%d",
            &[
                Str(b"BEG"),
                Null,
                Str(b"NAME"),
                Str(arena_string(mem, word(mem, rec + 8))),
                Str(b"IDXBEG"),
                Num(idx),
                Str(b"IDXEND"),
                Num(word(mem, lk + 4)
                    .wrapping_add(word(mem, lk + 8))
                    .wrapping_sub(base)),
            ],
        );
        api.payload(
            mem,
            logger,
            0x116,
            b"",
            NORMAL_TYPE,
            text,
            byte_len(mem, text),
        );
        api.payload(
            mem,
            logger,
            0x116,
            b"",
            MARKER_TYPE,
            word(mem, fp),
            half(mem, sl) as u32 * 24,
        );
    }
    let rc = normalized_body(mem, rec, name, pp, fp, sl, lk, handle, pad, api);
    if api.logging_enabled(logger) {
        api.payload(
            mem,
            logger,
            0x117,
            b"",
            NORMAL_TYPE,
            word(mem, pp),
            byte_len(mem, word(mem, pp)),
        );
        api.payload(
            mem,
            logger,
            0x117,
            b"",
            MARKER_TYPE,
            word(mem, fp),
            half(mem, sl) as u32 * 24,
        );
        api.log(logger, 0x115, b"%s%s", &[Str(b"END"), Null]);
    }
    rc
}
/// fn_27b904 / EU fn_27bac0 / KR fn_27bb6c.
/// Match normalized APDB patterns and prompt/class restrictions, then replace
/// text through the caller's rewrite service and insert the accepted marker.
fn normalized_body(
    mem: &mut [u8],
    rec: u32,
    name: u32,
    pp: u32,
    fp: u32,
    sl: u32,
    lk: u32,
    handle: [u32; 2],
    pad: u32,
    api: &mut impl NormalizedExpansionServices,
) -> u32 {
    let ctx = word(mem, rec);
    let obj = word(mem, ctx);
    let eng = word(mem, obj + 8);
    let heap = word(mem, eng + 4);
    let logger = word(mem, eng + 16);
    let text = word(mem, pp);
    let mut recs = word(mem, fp);
    let base = word(mem, recs + 12);
    let span = word(mem, lk + 4).wrapping_sub(base) as u16 as u32;
    let pattern = word(mem, rec + 16);
    let set = word(mem, ctx + 104);
    let sentence = word(mem, ctx + 100);
    let pattern_len = byte_len(mem, pattern);
    let prompt = mem[pattern as usize..(pattern + pattern_len) as usize]
        .iter()
        .position(|&c| c == 17)
        .map(|i| pattern + i as u32);
    let setlen = prompt.map_or(pattern_len, |p| p - pattern) as u16 as u32;
    let textlen = byte_len(mem, text) as u16 as u32;
    let mut idx = span;
    let mut original = OriginalPromptSpan::default();
    let mut matchlen = 0;
    let mut current = 0;
    let mut save_c = 0;
    let (mut ca, mut cb) = ([0u8; 7], [0u8; 7]);
    let (mut na, mut nb) = (0, 0);
    let (mut left_mark, mut right_mark) = (0, 0);
    let mut consumed;
    'search: loop {
        if ((setlen + idx) & 65535) > span.wrapping_add(word(mem, lk + 8))
            || mem[(text + idx) as usize] == 0
        {
            return 0;
        }
        let mut end = idx;
        let mut remaining = 0;
        let mut k = 0;
        let mut matched = true;
        loop {
            if end >= textlen {
                matchlen = end.wrapping_sub(idx) & 65535;
                break;
            }
            if remaining != 0
                && end + 1 < textlen
                && mem[(text + end) as usize] == 32
                && mem[(text + end + 1) as usize] == 21
            {
                remaining = (remaining - 1) & 65535;
                end = (end + 2) & 65535;
                continue;
            }
            let n = half(mem, sl) as u32;
            let mut v = n;
            if end == 0 || byte_space(mem[(text + end - 1) as usize]) {
                v = 0;
                while v < n {
                    let e = recs + v * 24;
                    if half(mem, e) == 34 && word(mem, e + 12) == end.wrapping_add(base) {
                        break;
                    }
                    v = (v + 1) & 65535;
                }
            }
            if v < n {
                if !normalized_span(mem, text, recs, n, v, &mut original) {
                    matched = false;
                    break;
                }
                if original.missing != 0 {
                    remaining = original.actual as u32;
                }
            }
            if setlen <= k {
                matchlen = end.wrapping_sub(idx) & 65535;
                break;
            }
            if mem[(pattern + k) as usize] != mem[(text + end) as usize] {
                matched = false;
            }
            k = (k + 1) & 65535;
            end = (end + 1) & 65535;
            if !matched {
                break;
            }
        }
        if matched && original.end < end && remaining != 0 {
            while end < textlen && mem[(text + end) as usize] != 21 {
                end = (end + 1) & 65535;
            }
            if end == textlen {
                return 0;
            }
            end = end.wrapping_add(remaining * 2).wrapping_sub(1) & 65535;
            if textlen < end {
                return 0;
            }
        }
        consumed = end.wrapping_sub(idx) & 65535;
        if matched {
            let e = idx + consumed;
            for i in 0..half(mem, sl) as u32 {
                if half(mem, recs + i * 24) == 34 {
                    if !normalized_span(mem, text, recs, half(mem, sl) as u32, i, &mut original) {
                        matched = false;
                        break;
                    }
                    if original.missing != 0
                        && ((original.start < idx && idx <= original.end)
                            || (original.start < e && e <= original.end))
                    {
                        matched = false;
                        break;
                    }
                }
            }
            if matched && prompt.is_some() {
                let mut parent = 0;
                let mut token = 0;
                let rc = api.first(handle, 1, 0, &mut parent);
                if rc >= 0x80000000 {
                    return rc;
                }
                let rc = api.first(handle, 2, parent, &mut token);
                if rc >= 0x80000000 {
                    return rc;
                }
                let mut y = mem[pattern as usize..(pattern + pattern_len) as usize]
                    .iter()
                    .position(|&c| c == 21)
                    .map(|i| pattern + i as u32);
                let mut nx = prompt.unwrap() + 1;
                while matched && y.is_some() {
                    let mut located = false;
                    while token != 0 {
                        let mut pos = 0;
                        let mut kind = 0;
                        let rc = api.property(handle, token, 1, &mut pos);
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        let rc = api.property(handle, token, 0, &mut kind);
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        if kind as u8 == 5 && ((y.unwrap() - pattern + idx) & 65535) == pos as u32 {
                            let mut value = 0;
                            let rc = api.string(handle, token, &mut value);
                            if rc >= 0x80000000 {
                                return rc;
                            }
                            let n = byte_len(mem, value);
                            matched = mem[nx as usize..(nx + n) as usize]
                                == mem[value as usize..(value + n) as usize]
                                && matches!(mem[(nx + n) as usize], 0 | 17);
                            located = true;
                            break;
                        }
                        let rc = api.next(handle, token, &mut token);
                        if rc >= 0x80000000 {
                            return rc;
                        }
                    }
                    if !located {
                        matched = false;
                    }
                    let mut v = 1;
                    while !matches!(mem[nx as usize], 0 | 17) {
                        nx += 1;
                        if matches!(mem[nx as usize], 95 | 35)
                            && !matches!(mem[(nx - 1) as usize], 32 | 95 | 35)
                        {
                            v = (v + 1) & 65535;
                        }
                    }
                    let p = y.unwrap() + v * 2;
                    y = mem[p as usize..(pattern + pattern_len + 1) as usize]
                        .iter()
                        .position(|&c| c == 21)
                        .map(|i| p + i as u32);
                    if y.is_none() {
                        break;
                    }
                    if mem[nx as usize] == 0 {
                        matched = false;
                        break;
                    }
                    nx += 1;
                }
            }
            if matched {
                current = idx;
                save_c = (current + consumed) & 65535;
                let mut x = matchlen + current;
                while (x & 65535) < byte_len(mem, text)
                    && !byte_space(mem[(text + (x & 65535)) as usize])
                {
                    x &= 65535;
                    if !contains(mem, set, mem[(text + x) as usize] as u16) {
                        matched = false;
                        break;
                    }
                    x += 1;
                }
            }
        }
        if api.logging_enabled(logger) {
            api.log(
                logger,
                0x118,
                b"%s%d %s%d %s%s",
                &[
                    Str(b"IDXBEG"),
                    Num(idx),
                    Str(b"IDXEND"),
                    Num(setlen + idx),
                    Str(b"RES"),
                    Str(if matched { b"MATCH" } else { b"NOMATCH" }),
                ],
            );
            if matched && mem[(rec + 14) as usize] == 0 && mem[(rec + 15) as usize] == 0 {
                for site in [0x119, 0x11a] {
                    api.log(
                        logger,
                        site,
                        b"%s%s %s%s",
                        &[
                            Str(b"RES"),
                            Str(b"MATCH"),
                            Str(b"INFO"),
                            Str(b"[*] matches"),
                        ],
                    );
                }
            }
        }
        if matched && (mem[(rec + 14) as usize] != 0 || mem[(rec + 15) as usize] != 0) {
            let mut x = save_c;
            while x < byte_len(mem, text) && !byte_space(mem[(text + x) as usize]) {
                x = (x + 1) & 65535;
            }
            let end = x == byte_len(mem, text);
            let mut left = current as i32 - 1;
            while left >= 0 && byte_space(mem[(text + left as u32) as usize]) {
                left -= 1;
            }
            let lm = if left as i16 >= 0
                && contains(mem, set, mem[(text + left as u32) as usize] as u16)
            {
                mem[(text + left as u32) as usize] as u16
            } else {
                0
            };
            let mut rm = 0u16;
            let mut m = current + matchlen;
            if (m as i16) < (byte_len(mem, text) as i16) {
                while byte_space(mem[(text + m) as usize]) {
                    m += 1;
                }
                if contains(mem, set, mem[(text + m) as usize] as u16) {
                    rm = mem[(text + m) as usize] as u16;
                }
            }
            let mut a = byte_classes(current == 0, lm, contains(mem, sentence, lm));
            let mut b = byte_classes(end, rm, contains(mem, sentence, rm));
            let edge = (consumed + current) & 65535;
            for i in 0..half(mem, sl) as u32 {
                let e = recs + i * 24;
                let p = word(mem, e + 20);
                if half(mem, e) == 50 && p != 0 && half(mem, p) != 0 {
                    let mut text = [0u8; 64];
                    let mut n = 0usize;
                    while n < 63 && half(mem, p + n as u32 * 2) != 0 {
                        text[n] = half(mem, p + n as u32 * 2) as u8;
                        n += 1;
                    }
                    let cls = match &text[..n] {
                        b"medial" => [4, 7, 0, 0, 0, 0, 0],
                        b"phrase-break" => [2, 3, 0, 0, 0, 0, 0],
                        b"sentence-break" => [1, 3, 0, 0, 0, 0, 0],
                        _ => [0; 7],
                    };
                    let q = word(mem, e + 12).wrapping_sub(base);
                    if q == current {
                        a = cls;
                    }
                    if q == edge || q.wrapping_sub(1) == edge {
                        b = cls;
                    }
                }
            }
            ca = a;
            cb = b;
            na = class_count(&a);
            nb = class_count(&b);
            left_mark = lm as u8;
            right_mark = rm as u8;
            let lc = mem[(rec + 14) as usize];
            let lk = mem[(rec + 12) as usize];
            let rc = mem[(rec + 15) as usize];
            let rk = mem[(rec + 13) as usize];
            matched = lc == 0 || (a.contains(&lc) && (lc != 5 || lk == lm as u8));
            if matched {
                matched = match normalized_property_match(
                    mem, ctx, heap, handle, lc, lk, current, false, api,
                ) {
                    Ok(v) => v,
                    Err(rc) => return rc,
                };
            }
            context_log(mem, rec, logger, 0x119, matched, &a[..na], lm as u8, api);
            let left_matched = matched;
            if matched {
                matched = rc == 0 || (b.contains(&rc) && (rc != 5 || rk == rm as u8));
            }
            if matched {
                matched = match normalized_property_match(
                    mem, ctx, heap, handle, rc, rk, save_c, true, api,
                ) {
                    Ok(v) => v,
                    Err(rc) => return rc,
                };
            }
            if left_matched {
                context_log(mem, rec, logger, 0x11a, matched, &b[..nb], rm as u8, api);
            }
        }
        if matched {
            let restricted = !api.named(mem, name, None);
            if mem[(rec + 4) as usize] == 2 || restricted {
                let lo = base.wrapping_add(current);
                let hi = base.wrapping_add(save_c);
                let mut named = false;
                for i in 0..half(mem, sl) as u32 {
                    let e = recs + i * 24;
                    let p = word(mem, e + 12);
                    if matches!(half(mem, e), 16 | 18 | 19 | 24)
                        && mem[(rec + 4) as usize] == 2
                        && lo < p
                        && p < hi
                    {
                        matched = false;
                        rejection(mem, logger, rec, lo, hi, true, true, api);
                        break;
                    }
                    if half(mem, e) == 21 && restricted {
                        let mut length = u32::MAX;
                        for j in i + 1..half(mem, sl) as u32 {
                            let q = recs + j * 24;
                            if half(mem, q) == 21 && p <= word(mem, q + 12) {
                                let (mut a, mut b) = (word(mem, q + 20), word(mem, e + 20));
                                while half(mem, a) != 0 && half(mem, a) == half(mem, b) {
                                    a += 2;
                                    b += 2;
                                }
                                if half(mem, a) != half(mem, b) {
                                    length = word(mem, q + 12).wrapping_sub(p);
                                    break;
                                }
                            }
                        }
                        if p <= lo
                            && (length == u32::MAX || hi <= length.wrapping_add(p))
                            && api.named(mem, name, Some(word(mem, e + 20)))
                        {
                            named = true;
                        }
                    }
                }
                if matched && restricted && !named {
                    rejection(mem, logger, rec, lo, hi, true, false, api);
                    matched = false;
                }
            }
        }
        if matched {
            break 'search;
        }
        while mem[(text + idx) as usize] != 0 && !byte_space(mem[(text + idx) as usize]) {
            idx = (idx + 1) & 65535;
        }
        while mem[(text + idx) as usize] != 0 && byte_space(mem[(text + idx) as usize]) {
            idx = (idx + 1) & 65535;
        }
    }
    let mut v = 0;
    while v < half(mem, sl) as u32 && current > word(mem, recs + v * 24 + 12).wrapping_sub(base) {
        v = (v + 1) & 65535;
    }
    if api.diagnostic_level(logger) != 0 {
        let left = summary(
            true,
            mem[(rec + 14) as usize],
            mem[(rec + 12) as usize],
            &ca[..na],
            left_mark,
        );
        let right = summary(
            true,
            mem[(rec + 15) as usize],
            mem[(rec + 13) as usize],
            &cb[..nb],
            right_mark,
        );
        let n = half(mem, sl) as u32;
        let mut lo = u32::MAX;
        let mut hi = u32::MAX;
        if current >= word(mem, recs + v * 24 + 12).wrapping_sub(base) {
            let mut r = v;
            while r < n
                && current >= word(mem, recs + r * 24 + 12).wrapping_sub(base)
                && half(mem, recs + r * 24) == 0x24
            {
                r = (r + 1) & 65535;
            }
            if r == n {
                let e = recs + (n - 1) * 24;
                lo = word(mem, e + 4);
                hi = lo;
                if half(mem, e) != 0x24 {
                    hi = lo.wrapping_add(word(mem, e + 8));
                }
            } else {
                lo = word(mem, recs + r * 24 + 4);
                hi = lo;
                while r < n && save_c > word(mem, recs + r * 24 + 12).wrapping_sub(base) {
                    if half(mem, recs + r * 24) != 0x24 {
                        hi =
                            word(mem, recs + r * 24 + 4).wrapping_add(word(mem, recs + r * 24 + 8));
                    }
                    r = (r + 1) & 65535;
                }
            }
        }
        api.accepted(
            mem,
            logger,
            b"DOMAIN_MNGR",
            1,
            0,
            ACCEPT_NORMAL,
            [word(mem, ctx + 88), word(mem, rec + 8)],
            lo,
            hi,
            summary_bytes(&left),
            summary_bytes(&right),
        );
    }
    let rc = api.replace(
        mem,
        rec,
        current as u16,
        consumed,
        base,
        pp,
        fp,
        sl,
        v,
        lk,
        handle,
        pad,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut q = 0;
    let lim = base.wrapping_add(current);
    recs = word(mem, fp);
    while q < half(mem, sl) as u32 && lim >= word(mem, recs + q * 24 + 12) {
        q = (q + 1) & 65535;
    }
    let v = api.reallocate_state(mem, heap, recs, (half(mem, sl) as u32 + 1) * 24);
    if v != 0 {
        recs = v;
        store(mem, fp, v);
        if q < half(mem, sl) as u32 {
            move_bytes(
                mem,
                v + (q + 1) * 24,
                v + q * 24,
                (half(mem, sl) as u32 - q) * 24,
            );
        }
        store16(mem, sl, half(mem, sl).wrapping_add(1));
        let e = recs + q * 24;
        mem[e as usize..e as usize + 24].fill(0);
        store16(mem, e, 40);
        if q != 0 {
            store(mem, e + 4, word(mem, e - 20));
        }
        store(mem, e + 12, lim);
        let a = word(mem, ctx + 88);
        let b = word(mem, rec + 8);
        let v = api.allocate(heap, (byte_len(mem, a) + byte_len(mem, b) + 3) * 2);
        store(mem, e + 20, v);
        if v != 0 {
            if api.register(mem, word(mem, obj + 44), e + 20) {
                let v = word(mem, e + 20);
                widen(mem, v, a, false);
                let at = v + wide_len(mem, v) * 2;
                store16(mem, at, 58);
                store16(mem, at + 2, 58);
                store16(mem, at + 4, 0);
                widen(mem, v, b, true);
                return rc;
            }
            api.free(heap, word(mem, e + 20));
            store(mem, e + 20, 0);
        }
    }
    api.error(logger, 0xb798);
    0x88e0200a
}
