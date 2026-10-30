//! Prompt, audio request, and database operations. Addresses are arena-relative.
//! Callers own both memory and service state; no allocator or global state is used.

fn word(m: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(m[p as usize..p as usize + 4].try_into().unwrap())
}
fn store(m: &mut [u8], p: u32, v: u32) {
    m[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn half(m: &[u8], p: u32) -> u16 {
    u16::from_le_bytes(m[p as usize..p as usize + 2].try_into().unwrap())
}
fn store_half(m: &mut [u8], p: u32, v: u16) {
    m[p as usize..p as usize + 2].copy_from_slice(&v.to_le_bytes());
}
fn text(m: &[u8], p: u32) -> &[u8] {
    let s = &m[p as usize..];
    &s[..s.iter().position(|&x| x == 0).unwrap()]
}
fn write_text(m: &mut [u8], p: u32, s: &[u8]) {
    m[p as usize..p as usize + s.len()].copy_from_slice(s);
    m[p as usize + s.len()] = 0;
}
fn decimal(mut v: u32, out: &mut [u8; 11]) -> &[u8] {
    let mut i = 11;
    loop {
        i -= 1;
        out[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    &out[i..]
}
fn parse_mime_decimal(s: &[u8]) -> u32 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], 32 | 9) {
        i += 1;
    }
    if s.get(i) == Some(&b'+') {
        i += 1;
    }
    let mut n = 0u32;
    while i < s.len() && s[i].is_ascii_digit() {
        n = n.wrapping_mul(10).wrapping_add((s[i] - b'0') as u32);
        i += 1;
    }
    n
}
fn parse_decimal(s: &[u8]) -> u32 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], 9..=13 | 32) {
        i += 1;
    }
    let negative = s.get(i) == Some(&b'-');
    if negative || s.get(i) == Some(&b'+') {
        i += 1;
    }
    let mut n = 0u32;
    while i < s.len() && s[i].is_ascii_digit() {
        n = n.wrapping_mul(10).wrapping_add((s[i] - b'0') as u32);
        i += 1;
    }
    if negative {
        n.wrapping_neg()
    } else {
        n
    }
}

/// External services used by the audio and linguistic database lifecycle.
pub trait PromptServices {
    fn validate(&mut self, object: u32, tag: u32, expected: u32, size: u32) -> u32;
    fn resolve(&mut self, handle: [u32; 2], context: &mut u32) -> u32;
    fn release_resource(&mut self, registry: u32, name: &[u8]);
    fn free(&mut self, heap: u32, object: u32);
    fn allocate(&mut self, heap: u32, bytes: u32) -> u32;
    fn insert(&mut self, map: u32, key: u32, value: u32) -> u32;
    fn error(&mut self, log: u32, code: u32);
    fn request_log(&mut self, mem: &[u8], log: u32, request: u32, name: &[u8], from: &[u8]);
    fn audio_log(&mut self, log: u32, url: u32, fields: [u32; 5]);
}

/// fn_21037c / EU fn_210538 / KR fn_2105e4.
pub fn check_linguistic_database(object: u32, tag: u32, api: &mut impl PromptServices) -> u32 {
    let rc = api.validate(object, tag, 0xf381, 52);
    if rc >= 0x80000000 {
        0x89a02008
    } else {
        rc
    }
}

/// fn_21030c / EU fn_2104c8 / KR fn_210574.
pub fn close_linguistic_database(
    mem: &[u8],
    object: u32,
    tag: u32,
    api: &mut impl PromptServices,
) -> u32 {
    let rc = check_linguistic_database(object, tag, api);
    if rc >= 0x80000000 || object == 0 {
        return rc;
    }
    let ctx = word(mem, object);
    for name in [b"LINGDB".as_slice(), b"SYNTHSTREAM", b"FE_DEPES"] {
        api.release_resource(word(mem, ctx + 24), name);
    }
    api.free(word(mem, ctx + 4), object);
    0
}

/// fn_23077c / EU fn_230938 / KR fn_2309e4.
/// Parse the case-sensitive MIME grammar, preserving the ROM's default channels.
pub fn parse_audio_type(
    input: &[u8],
    kind: &mut u8,
    rate: &mut u16,
    channels: &mut u16,
    mut order: Option<&mut u8>,
) -> u32 {
    let s = &input[..input.iter().position(|&x| x == 0).unwrap_or(input.len())];
    *kind = 0;
    *rate = 0;
    *channels = 0;
    if let Some(v) = order.as_deref_mut() {
        *v = 0;
    }
    if s == b"audio/wav" || s == b"audio/x-wav" || s == b"audio/x-nist" {
        if let Some(v) = order {
            *v = 1;
            *kind = if s == b"audio/x-nist" { 7 } else { 5 };
            return 1;
        }
    } else if s == b"audio/x-alaw-basic" {
        *kind = 1;
        *rate = 8000;
        *channels = 1;
    } else if s == b"audio/basic" {
        if let Some(v) = order {
            *v = 1;
            *kind = 6;
            return 1;
        }
        *kind = 2;
        *rate = 8000;
        *channels = 1;
    } else if s.starts_with(b"audio/L16;") {
        *channels = 2;
        let mut big = true;
        let mut count = 1u16;
        for (i, &c) in s.iter().enumerate() {
            if c != b';' {
                continue;
            }
            let p = &s[i + 1..];
            if p.starts_with(b"endian=little;") {
                big = false;
            } else if p.starts_with(b"endian=big;") {
                big = true;
            } else if let Some(v) = p.strip_prefix(b"rate=") {
                *rate = parse_mime_decimal(v) as u16;
            } else if let Some(v) = p.strip_prefix(b"channels=") {
                count = parse_mime_decimal(v) as u16;
            }
        }
        if count == 1 {
            *kind = if big { 4 } else { 3 };
        }
    }
    u32::from(*kind != 0)
}

/// fn_230508 / EU fn_2306c4 / KR fn_230770.
pub fn name_audio_request(
    mem: &mut [u8],
    session: u32,
    request: u32,
    output: u32,
    api: &mut impl PromptServices,
) -> u32 {
    store(mem, output, 0);
    let original = word(mem, request + 12);
    let len = if original == 0 {
        0
    } else {
        text(mem, original).len() as u32
    };
    let from = original.wrapping_add(len.saturating_sub(80));
    let len = len.min(80);
    let ctx = word(mem, session + 8);
    let buf = api.allocate(word(mem, ctx + 4), len * 2 + 32);
    store(mem, request + 4, buf);
    if buf == 0 {
        api.error(word(mem, ctx + 16), 100);
        return 0x8740200a;
    }
    let mut number = [0; 11];
    let digits = decimal(word(mem, session + 16), &mut number);
    let mut name = [0; 32];
    name[..7].copy_from_slice(b"__NUAN_");
    name[7..7 + digits.len()].copy_from_slice(digits);
    name[7 + digits.len()] = b'_';
    let name = &name[..8 + digits.len()];
    for (i, &b) in name.iter().enumerate() {
        store_half(mem, buf + (i as u32) * 2, b as u16);
    }
    store_half(mem, buf + (name.len() as u32) * 2, 0);
    if original != 0 && mem[from as usize] != 0 {
        for i in 0..=len {
            let b = mem[(from + i) as usize];
            store_half(mem, buf + ((name.len() as u32) + i) * 2, b as u16);
        }
    }
    let rc = api.insert(word(mem, session + 20), buf, request);
    if rc >= 0x80000000 {
        api.error(word(mem, ctx + 16), 100);
    } else {
        let id = word(mem, session + 16);
        store(mem, session + 16, if id == u32::MAX { 1 } else { id + 1 });
        store(mem, output, buf);
        api.request_log(
            mem,
            word(mem, ctx + 16),
            request,
            name,
            if original == 0 { b"" } else { text(mem, from) },
        );
    }
    rc
}

/// fn_2aeb24 / EU fn_2aece0 / KR fn_2aed8c.
pub fn parse_nist_integer(
    mem: &mut [u8],
    resource: u32,
    _name: u32,
    start: u32,
    end: u32,
    out: u32,
    api: &mut impl PromptServices,
) -> u32 {
    let mut p = start;
    while p < end && !matches!(mem[p as usize], b' ' | b'\t') {
        p += 1;
    }
    while p < end && matches!(mem[p as usize], b' ' | b'\t') {
        p += 1;
    }
    let code = if p >= end {
        121
    } else if end - p < 2
        || &mem[p as usize..p as usize + 2] != b"-i"
        || !matches!(mem[(p + 2) as usize], 9 | 10 | 13 | 32)
    {
        122
    } else {
        p += 2;
        while p < end && matches!(mem[p as usize], b' ' | b'\t') {
            p += 1;
        }
        if p >= end {
            123
        } else {
            let n = parse_decimal(&mem[p as usize..]);
            store(mem, out, n);
            return 0;
        }
    };
    api.error(word(mem, resource + 16), code);
    0x87402004
}

/// fn_2b0674 / EU fn_2b0830 / KR fn_2b08dc.
/// Decode AU headers and copy the payload, or treat a non-AU buffer as mu-law.
pub fn load_au_audio(
    mem: &mut [u8],
    handle: [u32; 2],
    url: u32,
    data: u32,
    size: u32,
    mime: u32,
    out: u32,
    size_out: u32,
    api: &mut impl PromptServices,
) -> u32 {
    if url == 0
        || mem[url as usize] == 0
        || data == 0
        || size == 0
        || mime == 0
        || out == 0
        || size_out == 0
    {
        return 0x87402007;
    }
    mem[mime as usize] = 0;
    store(mem, out, 0);
    store(mem, size_out, 0);
    let mut ctx = 0;
    let rc = api.resolve(handle, &mut ctx);
    if rc >= 0x80000000 {
        return rc;
    }
    let be = |p: u32| u32::from_be_bytes(mem[p as usize..p as usize + 4].try_into().unwrap());
    let (enc, ch, hz, off, sz) = if &mem[data as usize..data as usize + 4] != b".snd" {
        (1, 1, 8000, 0, size)
    } else {
        if size < 0xc0 {
            api.error(word(mem, ctx + 16), 101);
            return 0x87402004;
        }
        (
            be(data + 12),
            be(data + 20),
            be(data + 16),
            be(data + 4),
            be(data + 8),
        )
    };
    let log = word(mem, ctx + 16);
    api.audio_log(log, url, [enc, ch, hz, off, sz]);
    let error = if ch != 1 {
        102
    } else if enc == 1 && hz != 8000 {
        104
    } else if enc == 27 && hz != 8000 {
        103
    } else if !matches!(enc, 1 | 3 | 27) {
        105
    } else {
        0
    };
    if error != 0 {
        api.error(log, error);
        return 0x87402004;
    }
    if enc == 3 {
        let prefix = b"audio/L16;endian=big;rate=";
        write_text(mem, mime, prefix);
        let mut number = [0; 36];
        crate::frontend::render_signed(hz as i32, &mut number, 10).unwrap();
        let n = number.iter().position(|&b| b == 0).unwrap();
        write_text(mem, mime + prefix.len() as u32, &number[..n]);
    } else {
        write_text(
            mem,
            mime,
            if enc == 1 {
                b"audio/basic"
            } else {
                b"audio/x-alaw-basic"
            },
        );
    }
    let error = if sz == 0 {
        106
    } else if off.wrapping_add(sz) > size {
        107
    } else {
        0
    };
    if error != 0 {
        api.error(log, error);
        return 0x87402004;
    }
    let dest = api.allocate(word(mem, ctx + 4), sz);
    store(mem, out, dest);
    if dest == 0 {
        api.error(log, 108);
        return 0x8740200a;
    }
    mem.copy_within(
        (data + off) as usize..(data + off + sz) as usize,
        dest as usize,
    );
    store(mem, size_out, sz);
    rc
}

/// Caller-owned component methods. Output slots are full-width words unless
/// the interface explicitly specifies a halfword.
pub trait PromptMethods: PromptServices {
    fn release_interface(&mut self, registry: u32, interface: u32) -> u32;
    fn close_payload(&mut self, mem: &mut [u8], interface: u32, slot: u32);
    fn lookup_synth(&mut self, registry: u32, interface: &mut u32) -> u32;
    fn open_synth(&mut self, handle: [u32; 2], id: u32, name: u32, output: &mut u32) -> u32;
    fn synth_rate(&mut self, payload: u32, rate: &mut u16) -> u32;
    fn release_domain(&mut self, handle: [u32; 2], resource: u32);
    fn destroy_prompt(&mut self, prompt: u32);
    fn destroy_vector(&mut self, vector: u32);
    fn close_subobject(&mut self, interface: u32, object: u32) -> u32;
    fn close_decoder(&mut self, interface: u32, handle: [u32; 2]) -> u32;
    fn remove_key(&mut self, map: u32, key: u32) -> u32;
}

/// fn_230498 / EU fn_230654 / KR fn_230700.
pub fn release_speechbase(mem: &mut [u8], object: u32, heap: u32, api: &mut impl PromptMethods) {
    if object == 0 {
        return;
    }
    let interface = word(mem, object + 24);
    if interface != 0 {
        if word(mem, object + 28) != 0 {
            api.close_payload(mem, interface, object + 28);
        }
        api.release_interface(word(mem, word(mem, object + 16) + 12), interface);
    }
    api.free(heap, object);
}

/// fn_23032c / EU fn_2304e8 / KR fn_230594.
pub fn copy_speechbase(
    mem: &mut [u8],
    source: u32,
    heap: u32,
    out: u32,
    api: &mut impl PromptMethods,
) -> u32 {
    if source == 0 {
        return 0x87402007;
    }
    let object = api.allocate(heap, 32);
    if object == 0 {
        return 0x8740200a;
    }
    mem.copy_within(source as usize..source as usize + 32, object as usize);
    let context = word(mem, object + 16);
    let mut interface = word(mem, object + 24);
    let mut rc = api.lookup_synth(word(mem, context + 12), &mut interface);
    store(mem, object + 24, interface);
    if rc < 0x80000000 {
        let mut payload = word(mem, object + 28);
        rc = api.open_synth(
            [word(mem, object + 8), word(mem, object + 12)],
            word(mem, object + 20),
            word(mem, object + 4),
            &mut payload,
        );
        store(mem, object + 28, payload);
        if rc < 0x80000000 {
            let mut rate = 0;
            rc = api.synth_rate(payload, &mut rate);
            if rc < 0x80000000 && half(mem, object) == rate {
                store(mem, out, object);
                return rc;
            }
            api.error(word(mem, context + 16), 134);
            rc = 0x87402019;
            api.close_payload(mem, interface, object + 28);
            store(mem, object + 28, 0);
        }
        api.release_interface(word(mem, context + 12), interface);
        store(mem, object + 24, 0);
    }
    api.free(heap, object);
    store(mem, out, 0);
    rc
}

/// fn_27dcf8 / EU fn_27deb4 / KR fn_27df60.
pub fn release_prompt_domain(mem: &mut [u8], object: u32, api: &mut impl PromptMethods) -> u32 {
    if object == 0 {
        return 0x88e02007;
    }
    let inner = word(mem, object);
    let mut context = 0;
    let rc = api.resolve([word(mem, inner), word(mem, inner + 4)], &mut context);
    if rc >= 0x80000000 {
        return rc;
    }
    if word(mem, object + 8) != 0 {
        api.release_domain(
            [word(mem, inner + 24), word(mem, inner + 28)],
            word(mem, object + 8),
        );
    }
    if word(mem, object + 4) != 0 {
        api.destroy_prompt(word(mem, object + 4));
    }
    let heap = word(mem, context + 4);
    for off in [16, 20, 44, 88, 92, 96, 100, 104] {
        let p = word(mem, object + off);
        if p != 0 {
            api.free(heap, p);
        }
    }
    if word(mem, object + 116) != 0 {
        api.destroy_vector(word(mem, object + 116));
    }
    if mem[(object + 124) as usize] != 0 {
        let array = word(mem, object + 120);
        for i in 0..100 {
            let p = word(mem, array + i * 4);
            if p != 0 {
                api.free(heap, p);
            }
        }
        if array != 0 {
            api.free(heap, array);
        }
    }
    api.free(heap, object);
    rc
}

/// fn_22fb50 / EU fn_22fd0c / KR fn_22fdb8.
pub fn close_audio_decoder(
    mem: &mut [u8],
    object: u32,
    tag: u32,
    detach: u32,
    api: &mut impl PromptMethods,
) -> u32 {
    let mut err = api.validate(object, tag, 0x2bf, 28);
    if err >= 0x80000000 {
        return err;
    }
    let env = word(mem, object);
    let sub = word(mem, object + 8);
    if word(mem, sub + 20) != 0 {
        err = api.close_subobject(word(mem, sub + 16), word(mem, sub + 20));
        if err >= 0x80000000 {
            api.error(word(mem, env + 16), 159);
        }
    }
    let interface = word(mem, object + 12);
    if interface != 0 && word(mem, object + 16) != 0 {
        let rc = api.close_decoder(interface, [word(mem, object + 16), word(mem, object + 20)]);
        if rc >= 0x80000000 {
            api.error(word(mem, env + 16), 164);
        }
        if rc >= 0x80000000 && err < 0x80000000 {
            err = rc;
        }
    }
    if interface != 0 {
        let rc = api.release_interface(word(mem, env + 12), interface);
        if rc >= 0x80000000 && err < 0x80000000 {
            err = rc;
        }
    }
    if detach != 0 {
        let map = word(mem, word(mem, object + 4) + 20);
        let rc = api.remove_key(map, word(mem, sub + 4));
        if rc >= 0x80000000 && err < 0x80000000 {
            err = rc;
        }
    }
    api.free(word(mem, env + 4), object);
    err
}

/// Iterator and insertion methods owned by the caller.
pub trait AudioPromptOperations: PromptServices {
    fn frequency(&mut self, properties: u32, value: &mut u32) -> u32;
    fn iterator(&mut self, map: u32, value: &mut u32) -> u32;
    fn next(&mut self, mem: &mut [u8], iterator: u32, key: &mut u32, state: &mut u32) -> u32;
    fn rate(&mut self, interface: u32, payload: u32, value: &mut u16) -> u32;
    fn erase(&mut self, iterator: u32) -> u32;
    fn end(&mut self, iterator: u32) -> u32;
    fn resize(&mut self, heap: u32, old: u32, bytes: u32) -> u32;
    fn buffer(&mut self, handle: [u32; 2], out: &mut u32, bytes: u32) -> u32;
    fn commit(&mut self, handle: [u32; 2], bytes: u32) -> u32;
    fn read(&mut self, handle: [u32; 2], remaining: u32, dest: u32, count: &mut u32) -> u32;
    fn volume(&mut self, dest: u32, count: u32, volume: u32);
    fn convert(
        &mut self,
        state: u32,
        first: u32,
        rate: i32,
        dest: u32,
        count: u32,
        heap: u32,
        chunk: &mut u32,
        interface: u32,
        handle: [u32; 2],
    );
}

/// fn_22e6c0 / EU fn_22e87c / KR fn_22e928.
pub fn reopen_audio_fetch(
    mem: &mut [u8],
    object: u32,
    tag: u32,
    api: &mut impl AudioPromptOperations,
) -> u32 {
    let mut rc = api.validate(object, tag, 0x2be, 44);
    if rc >= 0x80000000 {
        return rc;
    }
    let env = word(mem, object + 8);
    let props = word(mem, env + 20);
    let mut wanted = 0;
    if props != 0 {
        let mut tmp = 0;
        rc = api.frequency(props, &mut tmp);
        if rc < 0x80000000 {
            wanted = tmp as u16;
        }
    }
    let mut iterator = 0;
    let mut key = 0;
    let mut state = 0;
    let mut current = 0;
    rc = api.iterator(word(mem, object + 24), &mut iterator);
    if rc < 0x80000000 {
        loop {
            if rc >= 0x80000000 {
                break;
            }
            rc = api.next(mem, iterator, &mut key, &mut state);
            if rc >= 0x80000000 {
                break;
            }
            rc = api.rate(word(mem, state + 24), word(mem, state + 28), &mut current);
            if rc < 0x80000000 && current == wanted {
                continue;
            }
            rc = api.erase(iterator);
        }
        let end = api.end(iterator);
        if end >= 0x80000000 && rc & 0x1fff == 23 {
            rc = end;
        }
    }
    if rc & 0x1fff == 23 {
        0
    } else {
        rc
    }
}

/// fn_22c85c / EU fn_22ca18 / KR fn_22cac4.
pub fn insert_audio_samples(
    mem: &mut [u8],
    object: u32,
    api: &mut impl AudioPromptOperations,
) -> u32 {
    let total = word(mem, object + 92);
    let avail = word(mem, object + 64);
    let n = avail.min(total);
    let handle = [word(mem, object + 24), word(mem, object + 28)];
    let source = [word(mem, object + 80), word(mem, object + 84)];
    let mut rc = 0;
    let mut done = 0u32;
    let mut count = 0;
    let mut chunk = 1024;
    if mem[(object + 101) as usize] != 0 {
        let state = word(mem, object + 104);
        let first = u32::from(avail >= total);
        if n > 1024 {
            let p = api.resize(
                word(mem, state + 256),
                word(mem, object + 112),
                n.wrapping_mul(2),
            );
            store(mem, object + 112, p);
        }
        let mut accumulated = 0u32;
        while n > done {
            let dest = word(mem, object + 112).wrapping_add(done.wrapping_mul(2));
            rc = api.read(source, n.wrapping_sub(done), dest, &mut count);
            if rc >= 0x80000000 {
                api.commit(handle, 0);
                return rc;
            }
            if mem[(object + 100) as usize] != 0 {
                api.volume(dest, count, half(mem, object + 96) as u32);
            }
            api.convert(
                state,
                first,
                half(mem, object + 98) as i16 as i32,
                dest,
                count,
                word(mem, object + 108),
                &mut chunk,
                word(mem, object + 12),
                handle,
            );
            done = done.wrapping_add(count);
            accumulated = accumulated.wrapping_add(chunk);
        }
        store(
            mem,
            object + 68,
            word(mem, object + 68).wrapping_add(accumulated),
        );
    } else {
        let mut buffer = 0;
        rc = api.buffer(handle, &mut buffer, n.wrapping_mul(2));
        if rc >= 0x80000000 {
            api.error(word(mem, word(mem, object + 8) + 16), 0xa12);
            return rc;
        }
        while n > done {
            let dest = buffer.wrapping_add(done.wrapping_mul(2));
            rc = api.read(source, n.wrapping_sub(done), dest, &mut count);
            if rc >= 0x80000000 {
                api.commit(handle, 0);
                return rc;
            }
            if mem[(object + 100) as usize] != 0 {
                api.volume(dest, count, half(mem, object + 96) as u32);
            }
            done = done.wrapping_add(count);
        }
        rc = api.commit(handle, done.wrapping_mul(2));
        if rc >= 0x80000000 {
            api.error(word(mem, word(mem, object + 8) + 16), 0xa12);
            return rc;
        }
        store(mem, object + 68, word(mem, object + 68).wrapping_add(done));
    }
    store(mem, object + 92, word(mem, object + 92).wrapping_sub(done));
    rc
}

/// Audio insertion registration and resource ownership.
pub trait AudioInsertionClose: PromptServices {
    /// Restore the interface's default check/changed callbacks for this key.
    fn reset_parameter(&mut self, parameters: u32, name: &[u8], object: u32);
    fn release_audio_resource(&mut self, registry: u32, name: &[u8]) -> u32;
    fn destroy_converter(&mut self, slot: u32);
}

/// fn_22cb4c / EU fn_22cd08 / KR fn_22cdb4.
pub fn close_audio_insertion(
    mem: &mut [u8],
    object: u32,
    tag: u32,
    api: &mut impl AudioInsertionClose,
) -> u32 {
    let mut rc = api.validate(object, tag, 0x34ea, 0x74);
    if rc >= 0x80000000 {
        return 0x8b902008;
    }
    let context = word(mem, object + 8);
    for name in [
        b"frequencyhz".as_slice(),
        b"audiooutputbufsamples",
        b"volume",
        b"audioinsertervolumeadjust",
    ] {
        api.reset_parameter(word(mem, context + 20), name, object);
    }
    if word(mem, object + 12) != 0 {
        let r = api.release_audio_resource(word(mem, context + 24), b"SYNTHSTREAM");
        if r >= 0x80000000 {
            rc = r;
        }
    }
    if word(mem, object + 48) != 0 {
        let r = api.release_audio_resource(word(mem, context + 24), b"AUDIOFETCHER");
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    let state = word(mem, object + 104);
    if state != 0 {
        api.free(word(mem, state + 256), word(mem, object + 108));
        api.free(word(mem, state + 256), word(mem, object + 112));
        api.destroy_converter(object + 104);
        store(mem, object + 104, 0);
    }
    api.free(word(mem, context + 4), object);
    rc
}

/// Decoder callbacks for caller-owned audio buffers.
pub trait AudioDecoderTransfer: PromptServices {
    fn read_audio_sink(&mut self, mem: &mut [u8], sink: u32, count: &mut u32, dst: u32) -> u32;
    fn decode_audio(
        &mut self,
        mem: &mut [u8],
        handle: [u32; 2],
        input: u32,
        available: &mut u16,
        dst: u32,
        capacity: &mut u16,
        scratch: &mut u16,
    ) -> u32;
}

/// fn_230028 / EU fn_2301e4 / KR fn_230290.
/// Follows the ROM's ten-argument decoder call and count-checked loop.
pub fn read_audio_decoder(
    mem: &mut [u8],
    object: u32,
    tag: u32,
    mut count: u32,
    dst: u32,
    out: u32,
    api: &mut impl AudioDecoderTransfer,
) -> u32 {
    store(mem, out, 0);
    let mut rc = api.validate(object, tag, 0x2bf, 28);
    if rc >= 0x80000000 {
        return rc;
    }
    if count == 0 || dst == 0 {
        return 0x87402007;
    }
    let env = word(mem, object + 8);
    let mut pos = word(mem, object + 24);
    if word(mem, env + 8) <= pos {
        return 0x87402003;
    }
    count = count.min(word(mem, env + 8) - pos);
    if word(mem, env + 20) != 0 {
        store(mem, out, count);
        rc = api.read_audio_sink(mem, word(mem, env + 20), &mut count, dst);
        store(mem, out, count);
        if rc < 0x80000000 {
            store(mem, object + 24, pos.wrapping_add(count));
        } else {
            api.error(word(mem, word(mem, object) + 16), 158);
        }
        return rc;
    }
    if word(mem, object + 12) == 0 {
        let src = word(mem, env + 28).wrapping_add(pos.wrapping_mul(2));
        let end = src.wrapping_add(count.wrapping_mul(2));
        mem.copy_within(src as usize..end as usize, dst as usize);
        store(mem, object + 24, pos.wrapping_add(count));
        store(mem, out, count);
        return rc;
    }
    while count != 0 {
        let rate = half(mem, env + 38) as u32;
        let product = rate.wrapping_mul(count);
        let mut available = if product < 0x10000 {
            product as u16
        } else {
            0xffff
        };
        let shifted = count.wrapping_shl(1);
        let mut capacity = if shifted >= 0x10000 {
            0xffff
        } else {
            shifted as u16
        };
        let mut scratch = 0;
        let input = word(mem, env + 28).wrapping_add(pos.wrapping_mul(rate));
        let output = dst.wrapping_add(word(mem, out).wrapping_mul(2));
        rc = api.decode_audio(
            mem,
            [word(mem, object + 16), word(mem, object + 20)],
            input,
            &mut available,
            output,
            &mut capacity,
            &mut scratch,
        );
        if rc < 0x80000000 && available != 0 {
            let consumed = (capacity >> 1) as u32;
            count = count.wrapping_sub(consumed);
            pos = pos.wrapping_add(consumed);
            store(mem, object + 24, pos);
            store(mem, out, word(mem, out).wrapping_add(consumed));
        } else {
            api.error(word(mem, word(mem, object) + 16), 163);
        }
        if rc >= 0x80000000 {
            break;
        }
    }
    rc
}

/// Map construction and INET resource operations owned by the caller.
pub trait AudioFetchConstructor: PromptServices {
    fn inet_resource(&mut self, registry: u32, resource: &mut u32) -> u32;
    /// Create either the request map or the speechbase map with its fixed ROM callbacks.
    fn create_audio_map(
        &mut self,
        mem: &mut [u8],
        heap: u32,
        speechbases: bool,
        out: &mut u32,
    ) -> u32;
    fn close_audio_map(&mut self, map: u32);
}

/// fn_22e378 / EU fn_22e534 / KR fn_22e5e0.
pub fn open_audio_fetch(
    mem: &mut [u8],
    component: u32,
    tag: u32,
    session: [u32; 2],
    out: u32,
    api: &mut impl AudioFetchConstructor,
) -> u32 {
    store(mem, out, 0);
    store(mem, out + 4, 0);
    let mut env = 0;
    let mut rc = api.resolve(session, &mut env);
    if rc >= 0x80000000 {
        return rc;
    }
    rc = api.validate(component, tag, 0x2bd, 32);
    if rc >= 0x80000000 {
        return 0x87402008;
    }
    let heap = word(mem, env + 4);
    let object = api.allocate(heap, 44);
    if object == 0 {
        api.error(word(mem, env + 16), 100);
        return 0x8740200a;
    }
    store(mem, object, session[0]);
    store(mem, object + 4, session[1]);
    store(mem, object + 8, env);
    store(mem, object + 12, component);
    store(mem, object + 16, 1);
    mem[object as usize + 28] = 1;
    if word(mem, object + 32) == 0 {
        let mut resource = 0;
        rc = api.inet_resource(word(mem, env + 24), &mut resource);
        if rc < 0x80000000 && resource != 0 {
            let interface = word(mem, resource + 4);
            if interface != 0 && mem[resource as usize] == 1 {
                store(mem, object + 32, interface);
                store(mem, object + 36, word(mem, resource + 8));
                store(mem, object + 40, word(mem, resource + 12));
            } else {
                api.error(word(mem, word(mem, object + 8) + 16), 129);
                api.release_resource(word(mem, word(mem, object + 8) + 24), b"INET");
            }
            mem[object as usize + 28] = 0;
        }
    }
    let mut first = word(mem, object + 20);
    let first_heap = word(mem, word(mem, object + 8) + 4);
    rc = api.create_audio_map(mem, first_heap, false, &mut first);
    store(mem, object + 20, first);
    if rc < 0x80000000 {
        let mut second = word(mem, object + 24);
        let second_heap = word(mem, word(mem, object + 8) + 4);
        rc = api.create_audio_map(mem, second_heap, true, &mut second);
        store(mem, object + 24, second);
        if rc < 0x80000000 {
            store(mem, out, object);
            store(mem, out + 4, 0x2be);
            return rc;
        }
    }
    api.error(word(mem, word(mem, object + 8) + 16), 130);
    store(mem, out, object);
    store(mem, out + 4, 0x2be);
    let close_status = api.validate(object, 0x2be, 0x2be, 44);
    if close_status < 0x80000000 {
        if word(mem, object + 24) != 0 {
            api.close_audio_map(word(mem, object + 24));
        }
        if word(mem, object + 20) != 0 {
            api.close_audio_map(word(mem, object + 20));
        }
        if word(mem, object + 32) != 0 {
            api.release_resource(word(mem, word(mem, object + 8) + 24), b"INET");
        }
        api.free(word(mem, word(mem, object + 8) + 4), object);
    }
    store(mem, out, 0);
    store(mem, out + 4, 0);
    rc
}

/// RIFF chunk operations supplied by the caller.
pub trait RiffServices: PromptServices {
    fn open_riff(
        &mut self,
        handle: [u32; 2],
        url: u32,
        input: u32,
        length: u32,
        riff: &mut u32,
    ) -> u32;
    fn riff_chunk(
        &mut self,
        mem: &mut [u8],
        riff: u32,
        tag: &mut [u8; 8],
        size: &mut u32,
        entry: &mut u32,
    ) -> u32;
    fn advance_riff(&mut self, mem: &mut [u8], riff: u32) -> u32;
    fn close_riff(&mut self, riff: u32);
}
fn append_text(mem: &mut [u8], dst: u32, s: &[u8]) {
    let n = text(mem, dst).len() as u32;
    write_text(mem, dst + n, s);
}
fn append_signed(mem: &mut [u8], dst: u32, value: i32) {
    if value < 0 {
        append_text(mem, dst, b"-");
    }
    let mut buf = [0; 11];
    append_text(mem, dst, decimal(value.unsigned_abs(), &mut buf));
}

/// fn_2afdc4 / EU fn_2aff80 / KR fn_2b002c.
pub fn load_wave_audio(
    mem: &mut [u8],
    handle: [u32; 2],
    url: u32,
    input: u32,
    length: u32,
    mime: u32,
    data: u32,
    total: u32,
    api: &mut impl RiffServices,
) -> u32 {
    if url == 0
        || mem[url as usize] == 0
        || input == 0
        || length == 0
        || mime == 0
        || data == 0
        || total == 0
    {
        return 0x87402007;
    }
    mem[mime as usize] = 0;
    store(mem, data, 0);
    store(mem, total, 0);
    let mut res = 0;
    let mut rc = api.resolve(handle, &mut res);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut riff = 0;
    rc = api.open_riff(handle, url, input, length, &mut riff);
    if rc >= 0x80000000 {
        return rc;
    }
    let log = word(mem, res + 16);
    let mut chunks = [(0u32, 0u32); 100];
    let mut count = 0;
    let mut bytes = 0u32;
    let mut tag = [0u8; 8];
    let mut size = 0;
    let mut entry = 0;
    let mut early = false;
    loop {
        rc = api.riff_chunk(mem, riff, &mut tag, &mut size, &mut entry);
        if rc >= 0x80000000 {
            break;
        }
        let tag = &tag[..tag.iter().position(|&b| b == 0).unwrap_or(8)];
        if tag == b"fmt " {
            let mut code = 0;
            if size < 16 {
                code = 143;
            } else {
                let format = half(mem, entry) as i16;
                let channels = half(mem, entry + 2) as i16;
                let rate = word(mem, entry + 4) as i32;
                let byte_rate = word(mem, entry + 8) as i32;
                let block = half(mem, entry + 12) as i16;
                let bits = half(mem, entry + 14) as i16;
                if channels != 1 {
                    code = 144;
                } else {
                    match format {
                        1 => {
                            write_text(mem, mime, b"audio/L");
                            append_signed(mem, mime, bits as i32);
                            append_text(mem, mime, b";endian=little;rate=");
                            append_signed(mem, mime, rate);
                            if bits != 16 {
                                code = 145;
                            }
                        }
                        6 | 7 => {
                            if rate == 8000 && byte_rate == 8000 && block == 1 && bits == 8 {
                                write_text(
                                    mem,
                                    mime,
                                    if format == 6 {
                                        b"audio/x-alaw-basic"
                                    } else {
                                        b"audio/basic"
                                    },
                                );
                            } else {
                                code = if format == 6 { 146 } else { 147 };
                            }
                        }
                        _ => code = 148,
                    }
                }
            }
            if code != 0 {
                api.error(log, code);
                rc = 0x87402004;
                early = true;
                break;
            }
        } else if tag == b"data" {
            chunks[count] = (entry, size);
            count += 1;
            bytes = bytes.wrapping_add(size);
        }
        rc = api.advance_riff(mem, riff);
        if rc >= 0x80000000 {
            break;
        }
    }
    if !early {
        if rc.wrapping_shl(19) == 0xa00000 {
            rc = 0;
        }
        if mem[mime as usize] == 0 {
            api.error(log, 149);
            rc = 0x87402004;
        } else if count == 0 {
            api.error(log, 150);
            rc = 0x87402004;
        } else {
            let out = api.allocate(word(mem, res + 4), bytes);
            store(mem, data, out);
            if out == 0 {
                api.error(log, 151);
                rc = 0x8740200a;
            } else {
                let mut dst = out;
                for &(p, n) in &chunks[..count] {
                    mem.copy_within(p as usize..p.wrapping_add(n) as usize, dst as usize);
                    dst = dst.wrapping_add(n);
                }
                store(mem, total, bytes);
            }
        }
    }
    if riff != 0 {
        api.close_riff(riff);
    }
    rc
}

fn nist_delimiter(c: u8) -> bool {
    matches!(c, b'\n' | b'\r' | b' ' | b'\t')
}
fn nist_value(mem: &[u8], mut line: u32, end: u32, dst: &mut [u8; 64]) -> Result<(), u32> {
    while line < end && !matches!(mem[line as usize], b' ' | b'\t') {
        line += 1;
    }
    while line < end && matches!(mem[line as usize], b' ' | b'\t') {
        line += 1;
    }
    if line >= end {
        return Err(124);
    }
    let s = &mem[line as usize..];
    if s[0] != b'-' || s[1] != b's' || !s[2].is_ascii_digit() {
        return Err(125);
    }
    line += 2;
    let n = parse_decimal(&mem[line as usize..]).min(63);
    while line < end && mem[line as usize].is_ascii_digit() {
        line += 1;
    }
    while line < end && matches!(mem[line as usize], b' ' | b'\t') {
        line += 1;
    }
    if end < line + n {
        return Err(126);
    }
    // fn_195e38 is strncpy, including padding when a value contains a NUL.
    let mut nul = false;
    for i in 0..n as usize {
        nul |= mem[line as usize + i] == 0;
        dst[i] = if nul { 0 } else { mem[line as usize + i] };
    }
    dst[n as usize] = 0;
    Ok(())
}

/// fn_2aec6c / EU fn_2aee28 / KR fn_2aeed4.
/// Decode a caller-provided NIST header and copy its payload using caller services.
pub fn load_nist_audio(
    mem: &mut [u8],
    handle: [u32; 2],
    url: u32,
    input: u32,
    len: u32,
    mime: u32,
    data: u32,
    size: u32,
    api: &mut impl PromptServices,
) -> u32 {
    if url == 0
        || mem[url as usize] == 0
        || input == 0
        || len == 0
        || mime == 0
        || data == 0
        || size == 0
    {
        return 0x87402007;
    }
    mem[mime as usize] = 0;
    store(mem, data, 0);
    store(mem, size, 0);
    let mut res = 0;
    let mut rc = api.resolve(handle, &mut res);
    if rc >= 0x80000000 {
        return rc;
    }
    let log = word(mem, res + 16);
    let fail = |api: &mut dyn PromptServices, code| {
        api.error(log, code);
        0x87402004
    };
    let mut fields = [0u32; 6];
    let mut fmt = [0u8; 64];
    fmt[0] = b'N';
    let mut coding = [0u8; 64];
    coding[..3].copy_from_slice(b"pcm");
    let end = input.wrapping_add(len);
    let mut p = input;
    let mut state = 0;
    let mut offset = 0;
    loop {
        let mut line = p;
        while p < end && !matches!(mem[p as usize], b'\n' | b'\r') {
            p += 1;
        }
        if p >= end {
            return fail(api, 109);
        }
        if state == 0 {
            if p - line != 7 || &mem[line as usize..p as usize] != b"NIST_1A" {
                return fail(api, 110);
            }
            state = 1;
        } else if state == 1 {
            while line < p && matches!(mem[line as usize], b' ' | b'\t') {
                line += 1;
            }
            offset = parse_mime_decimal(&mem[line as usize..]);
            if offset == 0 || len <= offset {
                return fail(api, 111);
            }
            state = 2;
        } else {
            if p - input >= offset {
                return fail(api, 109);
            }
            let keys: [&[u8]; 9] = [
                b"end_head",
                b"sample_rate",
                b"sample_count",
                b"sample_sig_bits",
                b"sample_n_bytes",
                b"channel_count",
                b"sample_byte_format",
                b"sample_coding",
                b"",
            ];
            for (i, key) in keys[..8].iter().enumerate() {
                let n = key.len() as u32;
                if n > p - line
                    || &mem[line as usize..(line + n) as usize] != *key
                    || !nist_delimiter(mem[(line + n) as usize])
                {
                    continue;
                }
                if i == 0 {
                    state = 3;
                } else if i < 6 {
                    // Reuse the parser without exposing its temporary output in the arena.
                    let mut q = line;
                    while q < p && !matches!(mem[q as usize], b' ' | b'\t') {
                        q += 1;
                    }
                    while q < p && matches!(mem[q as usize], b' ' | b'\t') {
                        q += 1;
                    }
                    if q >= p {
                        return fail(api, 121);
                    }
                    if p - q < 2
                        || &mem[q as usize..q as usize + 2] != b"-i"
                        || !nist_delimiter(mem[q as usize + 2])
                    {
                        return fail(api, 122);
                    }
                    q += 2;
                    while q < p && matches!(mem[q as usize], b' ' | b'\t') {
                        q += 1;
                    }
                    if p <= q {
                        return fail(api, 123);
                    }
                    fields[[0, 4, 0, 2, 1, 5][i]] = parse_decimal(&mem[q as usize..]);
                } else {
                    let dst = if i == 6 { &mut fmt } else { &mut coding };
                    if let Err(code) = nist_value(mem, line, p, dst) {
                        return fail(api, code);
                    }
                    rc = 0;
                }
                if i > 0 && i < 6 {
                    rc = 0;
                }
                break;
            }
        }
        while p < end && nist_delimiter(mem[p as usize]) {
            p += 1;
        }
        if state == 3 {
            break;
        }
    }
    if fields[5] != 1 {
        return fail(api, 112);
    }
    if (fields[0] as i32) < 0 {
        return fail(api, 119);
    }
    if (fields[1] as i32) <= 0 {
        return fail(api, 120);
    }
    let cod = &coding[..coding.iter().position(|&c| c == 0).unwrap_or(64)];
    let format = &fmt[..fmt.iter().position(|&c| c == 0).unwrap_or(64)];
    let ulaw = cod == b"ulaw" || (cod != b"alaw" && cod != b"pcm" && format == b"mu-law");
    let alaw = cod == b"alaw" || (cod != b"ulaw" && cod != b"pcm" && format == b"a-law");
    if ulaw || alaw {
        if !matches!(fields[4], 0 | 8000) || fields[1] != 1 {
            return fail(api, if ulaw { 114 } else { 113 });
        }
        write_text(
            mem,
            mime,
            if ulaw {
                b"audio/basic"
            } else {
                b"audio/x-alaw-basic"
            },
        );
    } else if cod == b"pcm" {
        if (fields[4] as i32) < 1 || fields[1] != 2 {
            return fail(api, 115);
        }
        let prefix: &[u8] = match format {
            b"01" | b"N" | b"1" => b"audio/L16;endian=little;rate=",
            b"10" => b"audio/L16;endian=big;rate=",
            _ => return fail(api, 115),
        };
        write_text(mem, mime, prefix);
        let mut digits = [0; 11];
        write_text(
            mem,
            mime + prefix.len() as u32,
            decimal(fields[4], &mut digits),
        );
    } else {
        return fail(api, 115);
    }
    let bytes = fields[0].wrapping_mul(fields[1]);
    if bytes == 0 {
        return fail(api, 116);
    }
    if offset.wrapping_add(bytes) > len {
        return fail(api, 117);
    }
    let dst = api.allocate(word(mem, res + 4), bytes);
    store(mem, data, dst);
    if dst == 0 {
        api.error(log, 118);
        return 0x8740200a;
    }
    mem.copy_within(
        (input + offset) as usize..(input + offset + bytes) as usize,
        dst as usize,
    );
    store(mem, size, bytes);
    rc
}

/// Caller-owned audio insertion construction services.
pub trait AudioInsertionConstructor: AudioInsertionClose {
    fn resource(&mut self, registry: u32, name: &[u8], resource: &mut u32) -> u32;
    fn parameter(&mut self, properties: u32, name: &[u8], value: &mut u32) -> u32;
    fn set_parameter(&mut self, properties: u32, name: &[u8], value: u32) -> u32;
    fn converter(&mut self, context: u32, frequency: u16) -> u32;
    fn reset_converter(&mut self, state: u32);
    fn converter_rate(&mut self, state: u32, rate: u32);
    fn allocate_buffer(&mut self, heap: u32, bytes: u32) -> u32;
    fn register_parameter(&mut self, properties: u32, name: &[u8], object: u32) -> u32;
}

/// fn_22cd30 / EU fn_22ceec / KR fn_22cf98.
pub fn open_audio_insertion(
    mem: &mut [u8],
    session: [u32; 2],
    out: u32,
    api: &mut impl AudioInsertionConstructor,
) -> u32 {
    if out == 0 {
        return 0x8b902007;
    }
    let mut context = 0;
    let mut rc = api.resolve(session, &mut context);
    if rc >= 0x80000000 {
        return rc;
    }
    store(mem, out, 0);
    store(mem, out + 4, 0);
    let object = api.allocate(word(mem, context + 4), 0x74);
    if object == 0 {
        api.error(word(mem, context + 16), 0xa11);
        return 0x8b90200a;
    }
    store(mem, object, session[0]);
    store(mem, object + 4, session[1]);
    store(mem, object + 8, context);
    let registry = word(mem, context + 24);
    let properties = word(mem, context + 20);
    let mut resource = 0;
    rc = api.resource(registry, b"SYNTHSTREAM", &mut resource);
    if rc >= 0x80000000 {
        api.error(word(mem, context + 16), 0xa0e);
    } else {
        store(mem, object + 12, word(mem, resource + 4));
        rc = api.resource(registry, b"AUDIOFETCHER", &mut resource);
        if rc >= 0x80000000 {
            api.error(word(mem, context + 16), 0xa00);
        } else if mem[resource as usize] != 1 || word(mem, resource + 4) == 0 {
            api.release_audio_resource(registry, b"AUDIOFETCHER");
            api.error(word(mem, context + 16), 0xa00);
        } else {
            store(mem, object + 48, word(mem, resource + 4));
            store(mem, object + 52, word(mem, resource + 8));
            store(mem, object + 56, word(mem, resource + 12));
        }
        let keys: [&[u8]; 6] = [
            b"frequencyhz",
            b"audiooutputbufsamples",
            b"volume",
            b"rate",
            b"audioinsertervolumeadjust",
            b"audioinserterrateadjust",
        ];
        let mut tmp = 0;
        let mut failed = false;
        for (i, key) in keys.iter().enumerate() {
            if i < 2 {
                tmp = word(mem, object + 60 + i as u32 * 4);
            }
            rc = api.parameter(properties, key, &mut tmp);
            if i < 2 {
                store(mem, object + 60 + i as u32 * 4, tmp);
            }
            if rc >= 0x80000000 {
                tmp = [8000, 0x400, 80, 100, 1, 1][i];
                if i < 2 {
                    store(mem, object + 60 + i as u32 * 4, tmp);
                }
                rc = api.set_parameter(properties, key, tmp);
                if rc >= 0x80000000 {
                    api.error(word(mem, context + 16), 0xa13);
                    failed = true;
                    break;
                }
            }
            match i {
                2 => store_half(mem, object + 96, tmp as u16),
                3 => store_half(mem, object + 98, tmp as u16),
                4 => mem[(object + 100) as usize] = u8::from(tmp != 0),
                5 => mem[(object + 101) as usize] = u8::from(tmp != 0),
                _ => {}
            }
        }
        if !failed {
            let state = api.converter(context, word(mem, object + 60) as u16);
            store(mem, object + 104, state);
            if state == 0 {
                rc = 0x8b90200a;
            } else {
                api.reset_converter(state);
                api.converter_rate(state, 100);
                let heap = word(mem, state + 256);
                let first = api.allocate_buffer(heap, 2048);
                store(mem, object + 108, first);
                if first == 0 {
                    rc = 0x8b90200a;
                } else {
                    let second = api.allocate_buffer(word(mem, state + 256), 2048);
                    store(mem, object + 112, second);
                    if second == 0 {
                        rc = 0x8b90200a;
                    } else {
                        for key in keys {
                            rc = api.register_parameter(properties, key, object);
                            if rc >= 0x80000000 {
                                break;
                            }
                        }
                        if rc < 0x80000000 {
                            store(mem, out, object);
                            store(mem, out + 4, 0x34ea);
                            return rc;
                        }
                    }
                }
            }
        }
    }
    api.error(word(mem, context + 16), 0xa14);
    store(mem, out, object);
    store(mem, out + 4, 0x34ea);
    close_audio_insertion(mem, object, 0x34ea, api);
    store(mem, out, 0);
    store(mem, out + 4, 0);
    rc
}

/// Caller-supplied request lookup and decoder construction.
pub trait AudioDecoderConstructor: PromptMethods {
    fn audio_request(&mut self, map: u32, key: u32, request: &mut u32) -> u32;
    fn prepare_audio(&mut self, interface: u32, payload: u32, state: u32, data: u32) -> u32;
    fn decoder_interface(&mut self, registry: u32, name: &[u8], interface: &mut u32) -> u32;
    fn open_decoder(
        &mut self,
        interface: u32,
        encoding: u32,
        session: [u32; 2],
        handle: &mut [u32; 2],
    ) -> u32;
}

/// fn_22fcb8 / EU fn_22fe74 / KR fn_22ff20.
/// The format flag is supplied by the owner's hardware configuration.
pub fn open_audio_decoder(
    mem: &mut [u8],
    object: u32,
    tag: u32,
    key: u32,
    data: u32,
    size: u32,
    out: u32,
    format_flag: u8,
    api: &mut impl AudioDecoderConstructor,
) -> u32 {
    store(mem, data, 0);
    store(mem, size, 0);
    store(mem, out, 0);
    store(mem, out + 4, 0);
    let mut rc = api.validate(object, tag, 0x2be, 44);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut request = 0;
    rc = api.audio_request(word(mem, object + 20), key, &mut request);
    if rc >= 0x80000000 {
        return rc;
    }
    let env = word(mem, object + 8);
    let decoder = api.allocate(word(mem, env + 4), 28);
    if decoder == 0 {
        api.error(word(mem, env + 16), 100);
        return 0x8740200a;
    }
    store(mem, decoder, env);
    store(mem, decoder + 4, object);
    store(mem, decoder + 8, request);
    let payload = word(mem, request + 20);
    let mut ready = false;
    if payload != 0 {
        rc = api.prepare_audio(
            word(mem, request + 16),
            payload,
            word(mem, request + 24),
            word(mem, request + 8),
        );
        if rc < 0x80000000 {
            ready = true;
        } else {
            api.error(word(mem, word(mem, object + 8) + 16), 157);
        }
    } else {
        let kind = mem[(request + 40) as usize];
        if (kind == 3 && format_flag != 0) || (kind == 4 && format_flag == 0) {
            ready = true;
        } else {
            let descriptor: Option<(&[u8], u32)> = match kind {
                1 => Some((b"decoder/g711alaw", 0x11110)),
                2 => Some((b"decoder/g711ulaw", 0x11108)),
                3 => Some((b"decoder/l16le", 0x11102)),
                4 => Some((b"decoder/l16be", 0x11102)),
                _ => None,
            };
            if let Some((name, encoding)) = descriptor {
                let mut interface = word(mem, decoder + 12);
                rc = api.decoder_interface(
                    word(mem, word(mem, object + 8) + 12),
                    name,
                    &mut interface,
                );
                store(mem, decoder + 12, interface);
                if rc < 0x80000000 {
                    let mut handle = [word(mem, decoder + 16), word(mem, decoder + 20)];
                    rc = api.open_decoder(
                        interface,
                        encoding,
                        [word(mem, object), word(mem, object + 4)],
                        &mut handle,
                    );
                    store(mem, decoder + 16, handle[0]);
                    store(mem, decoder + 20, handle[1]);
                    if rc < 0x80000000 {
                        ready = true;
                    } else {
                        api.error(word(mem, word(mem, object + 8) + 16), 161);
                    }
                } else {
                    api.error(word(mem, word(mem, object + 8) + 16), 160);
                }
            } else {
                api.error(word(mem, word(mem, object + 8) + 16), 162);
                rc = 0x87402001;
            }
        }
    }
    if ready {
        store(mem, data, word(mem, request + 12));
        store(mem, size, word(mem, request + 8));
        store(mem, out, decoder);
        store(mem, out + 4, 0x2bf);
    } else {
        store(mem, out, decoder);
        store(mem, out + 4, 0x2bf);
        close_audio_decoder(mem, decoder, 0x2bf, 0, api);
        store(mem, out, 0);
        store(mem, out + 4, 0);
    }
    rc
}

/// A decoded APDB record. Addresses refer to the caller's arena.
#[derive(Clone, Copy, Default)]
pub struct ExpansionRecord {
    pub domain: u32,
    pub kind: u8,
    pub data: u32,
    pub flags: [u8; 4],
    pub text: u32,
    pub auxiliary: u32,
    pub entries: u32,
    pub count: u16,
    pub suffix: u32,
}
/// Expansion leaves and optional diagnostics owned by the caller.
pub trait PromptExpansionServices {
    fn initialize(&mut self, mem: &mut [u8], context: u32);
    fn trace_enabled(&mut self, logger: u32) -> bool;
    /// Begin/unsupported diagnostics have no status; the final diagnostic carries the exact return code.
    fn diagnostic(
        &mut self,
        mem: &[u8],
        context: u32,
        logger: u32,
        event: u32,
        status: Option<u32>,
    );
    fn dump(&mut self, logger: u32, event: u32, markers: bool, data: u32, bytes: u32);
    fn expand(
        &mut self,
        mem: &mut [u8],
        record: ExpansionRecord,
        name: Option<u32>,
        text: u32,
        entries: u32,
        count: u32,
        node: u32,
        mode: [u32; 2],
        pad: &mut u16,
    ) -> u32;
}
fn expansion_slot(mem: &[u8], table: u32, index: u32) -> Option<u32> {
    if table == 0 || index >= word(mem, table + 20) {
        None
    } else {
        Some(word(mem, table).wrapping_add(index.wrapping_mul(half(mem, table + 4) as u32)))
    }
}
fn expansion_decode(mem: &[u8], context: u32, entry: u32, index: u32) -> ExpansionRecord {
    let p = word(mem, entry + 12).wrapping_add(index.wrapping_mul(20));
    let data = word(mem, context + 108).wrapping_add(word(mem, p));
    let address = |offset: u32| {
        let n = half(mem, p + offset);
        if n == 0xffff {
            0
        } else {
            data.wrapping_add(n as u32)
        }
    };
    ExpansionRecord {
        domain: context,
        kind: mem[entry as usize],
        data,
        flags: mem[p as usize + 4..p as usize + 8].try_into().unwrap(),
        text: data.wrapping_add(half(mem, p + 8) as u32),
        auxiliary: address(10),
        entries: word(mem, context + 112).wrapping_add(word(mem, p + 12).wrapping_mul(4)),
        count: half(mem, p + 16),
        suffix: address(18),
    }
}
fn expansion_name(mem: &[u8], entry: u32) -> bool {
    let p = word(mem, entry + 4);
    crate::frontend_resources::contains_name(
        if p == 0 {
            None
        } else {
            Some(&mem[p as usize..])
        },
        Some(b"*\0"),
    ) != 0
}
fn expansion_permitted(mem: &[u8], entry: u32, entries: u32, count: u32) -> bool {
    if expansion_name(mem, entry) {
        return true;
    }
    let name = word(mem, entry + 4);
    for i in 0..half(mem, count) as u32 {
        let marker = word(mem, entries).wrapping_add(i * 24);
        if half(mem, marker) != 21 {
            continue;
        }
        let p = word(mem, marker + 20);
        let mut wide = [0u16; 65];
        let mut n = 0usize;
        if p == 0 {
            continue;
        }
        while n < 64 && half(mem, p + n as u32 * 2) != 0 {
            wide[n] = half(mem, p + n as u32 * 2);
            n += 1;
        }
        if n == 64 {
            continue;
        }
        if crate::frontend_resources::contains_wide_name(Some(&mem[name as usize..]), Some(&wide))
            != 0
        {
            return true;
        }
    }
    false
}
fn expansion_before(a: ExpansionRecord, b: ExpansionRecord, mem: &[u8]) -> bool {
    let length = |p: u32| mem[p as usize..].iter().position(|&b| b == 0).unwrap();
    (length(b.text), b.flags[2], b.flags[3]) > (length(a.text), a.flags[2], a.flags[3])
}
fn expansion_dump(
    mem: &[u8],
    context: u32,
    text: u32,
    entries: u32,
    count: u32,
    head: u32,
    event: u32,
    api: &mut impl PromptExpansionServices,
) {
    let logger = word(mem, word(mem, word(mem, context) + 8) + 16);
    let mut last = head;
    while last != 0 && word(mem, last + 24) != 0 {
        last = word(mem, last + 24);
    }
    let n = if count == 0 {
        0
    } else {
        half(mem, count) as u32
    };
    let span = if n == 0 {
        0
    } else {
        word(mem, last + 8)
            .wrapping_add(word(mem, last + 4))
            .wrapping_sub(word(mem, word(mem, entries) + 12))
    };
    api.dump(logger, event, false, word(mem, text), span);
    api.dump(logger, event, true, word(mem, entries), n.wrapping_mul(24));
}
fn expand_prompt_records(
    mem: &mut [u8],
    context: u32,
    text: u32,
    entries: u32,
    count: u32,
    head: u32,
    mode: Option<[u32; 2]>,
    api: &mut impl PromptExpansionServices,
) -> u32 {
    if mem[context as usize + 13] == 0 {
        api.initialize(mem, context);
    }
    if mem[context as usize + 13] != 2
        || mem[context as usize + 12] == 0
        || (mode.is_none() && mem[context as usize + 14] == 0)
    {
        return 0;
    }
    let logger = word(mem, word(mem, word(mem, context) + 8) + 16);
    let event = if mode.is_some() { 273 } else { 213 };
    if api.trace_enabled(logger) {
        api.diagnostic(mem, context, logger, event, None);
        expansion_dump(mem, context, text, entries, count, head, event + 1, api);
    }
    let mut node = head;
    let mut rc = 0;
    'nodes: while node != 0 {
        rc = 0;
        if mem[node as usize] != 0 {
            let mut pad = 0;
            let mut extra = 0;
            if let Some(a) = expansion_slot(mem, word(mem, context + 116), 0) {
                if mem[a as usize] == 1 {
                    if let Some(b) = expansion_slot(mem, word(mem, context + 116), 1) {
                        if mem[b as usize] == 2 {
                            extra = 2;
                            let permitted = if mode.is_some() {
                                expansion_permitted(mem, a, entries, count)
                                    & expansion_permitted(mem, b, entries, count)
                            } else {
                                expansion_name(mem, a) & expansion_name(mem, b)
                            };
                            if !permitted {
                                api.diagnostic(mem, context, logger, event + 3, None);
                            } else {
                                let mut ai = 0;
                                let mut bi = 0;
                                let mut da = expansion_decode(mem, context, a, 0);
                                let mut db = expansion_decode(mem, context, b, 0);
                                while mem[node as usize] == 1
                                    && (ai < word(mem, a + 8) || bi < word(mem, b + 8))
                                {
                                    let take_b = if bi >= word(mem, b + 8) {
                                        false
                                    } else if ai == word(mem, a + 8) {
                                        true
                                    } else {
                                        expansion_before(da, db, mem)
                                    };
                                    let (record, entry) = if take_b { (db, b) } else { (da, a) };
                                    let name = mode.map(|_| word(mem, entry + 4));
                                    rc = api.expand(
                                        mem,
                                        record,
                                        name,
                                        text,
                                        entries,
                                        count,
                                        node,
                                        mode.unwrap_or([0; 2]),
                                        &mut pad,
                                    );
                                    if rc >= 0x80000000 {
                                        break 'nodes;
                                    }
                                    if take_b {
                                        bi += 1;
                                        if bi < word(mem, b + 8) {
                                            db = expansion_decode(mem, context, b, bi);
                                        }
                                    } else {
                                        ai += 1;
                                        if ai < word(mem, a + 8) {
                                            da = expansion_decode(mem, context, a, ai);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            while let Some(entry) = expansion_slot(mem, word(mem, context + 116), extra) {
                let permitted = if mode.is_some() {
                    expansion_permitted(mem, entry, entries, count)
                } else {
                    expansion_name(mem, entry)
                };
                if permitted {
                    let mut index = 0;
                    while mem[node as usize] == 1 && index < word(mem, entry + 8) {
                        let record = expansion_decode(mem, context, entry, index);
                        let name = mode.map(|_| word(mem, entry + 4));
                        rc = api.expand(
                            mem,
                            record,
                            name,
                            text,
                            entries,
                            count,
                            node,
                            mode.unwrap_or([0; 2]),
                            &mut pad,
                        );
                        if rc >= 0x80000000 {
                            break 'nodes;
                        }
                        index += 1;
                    }
                } else if mode.is_some() {
                    api.diagnostic(mem, context, logger, event, None);
                }
                extra += 1;
            }
        }
        node = word(mem, node + 24);
    }
    if api.trace_enabled(logger) {
        expansion_dump(mem, context, text, entries, count, head, event + 2, api);
    }
    api.diagnostic(mem, context, logger, event, Some(rc));
    rc
}
/// fn_27f7cc / EU fn_27f988 / KR fn_27fa34.
pub fn expand_original_prompts(
    mem: &mut [u8],
    context: u32,
    text: u32,
    entries: u32,
    count: u32,
    head: u32,
    api: &mut impl PromptExpansionServices,
) -> u32 {
    expand_prompt_records(mem, context, text, entries, count, head, None, api)
}
/// fn_27feb4 / EU fn_280070 / KR fn_28011c.
pub fn expand_normalized_prompts(
    mem: &mut [u8],
    context: u32,
    text: u32,
    entries: u32,
    count: u32,
    mode: [u32; 2],
    head: u32,
    api: &mut impl PromptExpansionServices,
) -> u32 {
    expand_prompt_records(mem, context, text, entries, count, head, Some(mode), api)
}

/// String construction and diagnostics for prompt database inspection.
pub trait PromptInspectionServices: PromptExpansionServices {
    fn create_string(&mut self, heap: u32) -> u32;
    fn append_name(&mut self, string: u32, name: u32);
    fn append_separator(&mut self, string: u32, separator: u8);
    fn string_contents(&mut self, string: u32) -> u32;
    fn destroy_string(&mut self, string: u32);
    fn context_diagnostic(&mut self, mem: &[u8], context: u32, description: u32);
    fn record_diagnostic(&mut self, logger: u32, record: ExpansionRecord, begin: &[u8], end: &[u8]);
}
fn inspect_expansion_record(
    mem: &[u8],
    context: u32,
    record: ExpansionRecord,
    api: &mut impl PromptInspectionServices,
) {
    let labels: [&[u8]; 6] = [b"[*]", b"[S]", b"[P]", b"[S|P]", b"[...]", b"[p]"];
    let mut begin = [0u8; 12];
    let mut end = [0u8; 12];
    let a = labels[record.flags[2] as usize];
    let b = labels[record.flags[3] as usize];
    begin[..a.len()].copy_from_slice(a);
    end[..b.len()].copy_from_slice(b);
    if record.flags[2] == 5 {
        begin[1] = record.flags[0];
    }
    if record.flags[3] == 5 {
        end[1] = record.flags[1];
    }
    let logger = word(mem, word(mem, word(mem, context) + 8) + 16);
    api.record_diagnostic(logger, record, &begin[..a.len()], &end[..b.len()]);
    api.dump(
        logger,
        204,
        false,
        record.entries,
        (record.count as u32) * 4,
    );
}
/// fn_2807c0 / EU fn_28097c / KR fn_280a28.
/// Inspect paired records first, then all remaining entries, starting at zero when unpaired.
pub fn inspect_prompt_database(
    mem: &mut [u8],
    context: u32,
    api: &mut impl PromptInspectionServices,
) {
    let heap = word(mem, word(mem, word(mem, context) + 8) + 4);
    let builder = api.create_string(heap);
    if builder != 0 {
        if mem[context as usize + 13] == 0 {
            api.initialize(mem, context);
        }
        if mem[context as usize + 13] == 2 {
            let mut index = 0;
            while let Some(entry) = expansion_slot(mem, word(mem, context + 116), index) {
                let name = word(mem, entry + 4);
                if name != 0 && mem[name as usize] != 0 {
                    api.append_name(builder, name);
                    api.append_separator(builder, b'|');
                }
                index += 1;
            }
        }
    }
    let description = if builder == 0 {
        0
    } else {
        api.string_contents(builder)
    };
    api.context_diagnostic(mem, context, description);
    if builder != 0 {
        api.destroy_string(builder);
    }
    let mut index = 0;
    if let Some(a) = expansion_slot(mem, word(mem, context + 116), 0) {
        if mem[a as usize] == 1 {
            if let Some(b) = expansion_slot(mem, word(mem, context + 116), 1) {
                if mem[b as usize] == 2 {
                    let mut ai = 0;
                    let mut bi = 0;
                    let mut da = expansion_decode(mem, context, a, 0);
                    let mut db = expansion_decode(mem, context, b, 0);
                    while ai < word(mem, a + 8) || bi < word(mem, b + 8) {
                        let take_b = if bi >= word(mem, b + 8) {
                            false
                        } else if ai == word(mem, a + 8) {
                            true
                        } else {
                            expansion_before(da, db, mem)
                        };
                        inspect_expansion_record(mem, context, if take_b { db } else { da }, api);
                        if take_b {
                            bi += 1;
                            if bi < word(mem, b + 8) {
                                db = expansion_decode(mem, context, b, bi);
                            }
                        } else {
                            ai += 1;
                            if ai < word(mem, a + 8) {
                                da = expansion_decode(mem, context, a, ai);
                            }
                        }
                    }
                    index = 2;
                }
            }
        }
    }
    while let Some(entry) = expansion_slot(mem, word(mem, context + 116), index) {
        for i in 0..word(mem, entry + 8) {
            inspect_expansion_record(mem, context, expansion_decode(mem, context, entry, i), api);
        }
        index += 1;
    }
    let logger = word(mem, word(mem, word(mem, context) + 8) + 16);
    api.diagnostic(mem, context, logger, 203, None);
}

/// Host operations used by the APDB resource loader. All addresses are caller-relative.
pub trait ApdbServices {
    fn resolve_apdb(&mut self, handle: [u32; 2], context: &mut u32) -> u32;
    fn open_apdb(
        &mut self,
        handle: [u32; 2],
        name: &[u8],
        mode: u32,
        flags: u32,
        version: u32,
        entry: &mut u32,
    ) -> u32;
    fn calloc_apdb(&mut self, mem: &mut [u8], heap: u32, count: u32, size: u32) -> u32;
    fn alloc_apdb(&mut self, heap: u32, size: u32) -> u32;
    fn realloc_apdb(&mut self, heap: u32, old: u32, size: u32) -> u32;
    fn free_apdb(&mut self, heap: u32, object: u32);
    fn copy_apdb_string(&mut self, mem: &mut [u8], heap: u32, value: &[u8], out: u32) -> u32;
    fn create_apdb_table(&mut self, mem: &mut [u8], heap: u32, defaults: [u32; 3], out: u32)
        -> u32;
    fn apdb_header(&mut self, mem: &mut [u8], entry: u32, out: u32, index: &mut u32) -> u32;
    fn next_apdb_record(&mut self, entry: u32) -> u32;
    fn apdb_record(
        &mut self,
        entry: u32,
        kind: &mut [u8; 8],
        count: &mut u32,
        index: &mut u32,
    ) -> u32;
    fn apdb_field(
        &mut self,
        entry: u32,
        index: u32,
        count: u32,
        offset: u32,
        buf: &mut [u8],
        size: &mut u32,
    ) -> u32;
    fn decode_apdb_integer(
        &mut self,
        index: u32,
        count: u32,
        offset: &mut u32,
        value: &mut u32,
    ) -> u32;
    fn insert_apdb_record(&mut self, table: u32, record: [u32; 4], tn_type: Option<&[u8]>) -> u32;
    fn apdb_table_entry(&mut self, table: u32, index: u32, out: &mut u32) -> u32;
    fn apdb_alias_matches(&mut self, mem: &[u8], text: u32, list: u32) -> bool;
    fn apdb_frequency(&mut self, props: u32, value: &mut u32) -> u32;
    fn apdb_stream(&mut self, context: u32, path: &[u8], value: &mut u32) -> u32;
    fn close_apdb_stream(&mut self, context: u32, value: u32);
    fn apdb_broker(&mut self, context: u32, name: &[u8], value: &mut u32) -> u32;
    fn close_apdb_broker(&mut self, context: u32, value: u32);
    fn close_apdb_entry(&mut self, entry: u32) -> u32;
    fn destroy_apdb(&mut self, mem: &mut [u8], object: u32);
    fn apdb_logging(&mut self, log: u32) -> bool;
    fn inspect_apdb(&mut self, mem: &mut [u8], object: u32);
    fn apdb_error(&mut self, log: u32, code: u32);
}

fn terminated(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())]
}
fn apdb_copy_fixed(mem: &mut [u8], dst: u32, value: &[u8], size: usize) {
    let n = value.len().min(size - 1);
    mem[dst as usize..dst as usize + n].copy_from_slice(&value[..n]);
    mem[dst as usize + n] = 0;
}
fn apdb_uri_value(
    mem: &mut [u8],
    api: &mut impl ApdbServices,
    heap: u32,
    log: u32,
    uri: u32,
    key: &[u8],
) -> (u32, u32) {
    let mut p = uri;
    let mut result = 0;
    let mut rc = 0x88e02014;
    loop {
        let Some(n) = text(mem, p).iter().position(|&b| b == b';') else {
            break;
        };
        p = p.wrapping_add(n as u32 + 1);
        if !text(mem, p).starts_with(key) || mem[p as usize + key.len()] != b'=' {
            continue;
        }
        p += key.len() as u32;
        rc = 0;
        let start = p + 1;
        let len = text(mem, start)
            .iter()
            .position(|&b| b == b';')
            .unwrap_or_else(|| text(mem, start).len());
        result = api.alloc_apdb(heap, len as u32 + 1);
        if result == 0 {
            api.apdb_error(log, 0xb798);
            return (0, 0x88e0200a);
        }
        mem.copy_within(start as usize..start as usize + len, result as usize);
        mem[result as usize + len] = 0;
    }
    (result, rc)
}

/// fn_27e4a4 / EU fn_27e660 / KR fn_27e70c.
/// Load an APDB resource into caller-owned memory, including attributes and recipe aliases.
pub fn load_prompt_database(
    mem: &mut [u8],
    owner: u32,
    uri: u32,
    name: u32,
    mode: u32,
    flags: u32,
    weight: u32,
    output: u32,
    defaults: &[u8; 108],
    api: &mut impl ApdbServices,
) -> u32 {
    if output == 0 {
        return 0x88e02007;
    }
    store(mem, output, 0);
    let mut filename = [0u8; 256];
    let s = if name == 0 || mem[name as usize] == 0 {
        b"in-memory APDB".as_slice()
    } else {
        text(mem, name)
    };
    filename[..s.len()].copy_from_slice(s);
    let filename = terminated(&filename);
    let mut eng = 0;
    let handle = [word(mem, owner), word(mem, owner + 4)];
    let mut rc = api.resolve_apdb(handle, &mut eng);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut entry = 0;
    rc = api.open_apdb(
        handle,
        filename,
        mode,
        flags,
        u32::from_le_bytes(defaults[..4].try_into().unwrap()) & 0xffffff,
        &mut entry,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let heap = word(mem, eng + 4);
    let log = word(mem, eng + 16);
    let object = api.calloc_apdb(mem, heap, 1, 128);
    store(mem, output, object);
    let mut index = 0;
    let mut count = 0;
    let mut recipe = false;
    let mut tn_type = [0u8; 16];
    let mut table_defaults = [word(defaults, 64), word(defaults, 68), word(defaults, 72)];
    rc = 'load: {
        if object == 0 {
            api.apdb_error(log, 0xb798);
            break 'load 0x88e0200a;
        }
        store(mem, object, owner);
        mem[object as usize + 12] = 1;
        mem[object as usize + 124] = 0;
        mem[object as usize + 14] = 0;
        rc = api.copy_apdb_string(mem, heap, filename, object + 20);
        if rc >= 0x80000000 {
            api.apdb_error(log, 0xb798);
            break 'load 0x88e0200a;
        }
        if api.create_apdb_table(mem, heap, table_defaults, object + 116) == 0 {
            api.apdb_error(log, 0xb798);
            break 'load 0x88e0200a;
        }
        rc = api.apdb_header(mem, entry, object + 4, &mut index);
        if rc >= 0x80000000 {
            break 'load rc;
        }
        let mut first = true;
        'records: loop {
            if !first {
                if rc >= 0x80000000 {
                    break;
                }
                rc = api.next_apdb_record(entry);
                if rc >= 0x80000000 {
                    break;
                }
            }
            first = false;
            let mut kind = [0u8; 8];
            rc = api.apdb_record(entry, &mut kind, &mut count, &mut index);
            if rc >= 0x80000000 {
                break;
            }
            match terminated(&kind) {
                b"ATTR" | b"PHGP" => {
                    let phgp = kind[0] == b'P';
                    if phgp {
                        mem[object as usize + 124] = 1;
                        let p = api.calloc_apdb(mem, heap, 100, 4);
                        store(mem, object + 120, p);
                    }
                    let mut i = 0u32;
                    while (i as i32) < count as i32 && rc < 0x80000000 {
                        let mut key = [0u8; 64];
                        let mut value = [0u8; 256];
                        let mut nf = 64;
                        rc = api.apdb_field(entry, index, count, i, &mut key, &mut nf);
                        if rc >= 0x80000000 {
                            break 'records;
                        }
                        i = i.wrapping_add(nf);
                        nf = 256;
                        rc = api.apdb_field(entry, index, count, i, &mut value, &mut nf);
                        if rc >= 0x80000000 {
                            break 'records;
                        }
                        i = i.wrapping_add(nf);
                        let key = terminated(&key);
                        let value = terminated(&value);
                        if phgp {
                            if key.len() == 4
                                && &key[..2] == b"PG"
                                && key[2].is_ascii_digit()
                                && key[3].is_ascii_digit()
                            {
                                let slot =
                                    ((key[2] - b'0') as u32 * 10 + (key[3] - b'0') as u32) * 4;
                                let at = word(mem, object + 120) + slot;
                                let mut dst = word(mem, at);
                                if dst == 0 {
                                    dst = api.calloc_apdb(mem, heap, value.len() as u32 + 1, 1);
                                    store(mem, at, dst);
                                }
                                write_text(mem, dst, value);
                            }
                            continue;
                        }
                        let fixed = match key {
                            b"Lng" => Some((36, 8)),
                            b"RawDataVersion" => Some((48, 8)),
                            b"Reduction" => Some((56, 12)),
                            b"ReductionVersion" => Some((68, 8)),
                            b"UnitIDType" => Some((76, 12)),
                            b"Mode" => Some((24, 12)),
                            _ => None,
                        };
                        if let Some((at, size)) = fixed {
                            apdb_copy_fixed(mem, object + at, value, size);
                        } else {
                            let pointer = match key {
                                b"Voice" => Some(44),
                                b"Domain" => Some(88),
                                b"PhrasePunctuation" => Some(100),
                                b"AllPunctuation" => Some(104),
                                b"URISuffix" => Some(96),
                                _ => None,
                            };
                            if let Some(at) = pointer {
                                rc = api.copy_apdb_string(mem, heap, value, object + at);
                            } else if key == b"TNType" {
                                tn_type.fill(0);
                                tn_type[..value.len()].copy_from_slice(value);
                            } else if key == b"OriOrth" {
                                mem[object as usize + 14] = 1;
                            } else if key == b"UnitIDSize" && parse_decimal(value) != 4 {
                                api.apdb_error(log, 0xb798);
                                rc = 0x88e02001;
                            }
                        }
                    }
                }
                b"STRP" => store(mem, object + 108, index),
                b"UIDP" => store(mem, object + 112, index),
                b"APPR" | b"RECP" => {
                    let recp = kind[0] == b'R';
                    recipe |= recp;
                    let at = if recp { 92 } else { 76 };
                    let mut record = [
                        word(defaults, at),
                        word(defaults, at + 4),
                        word(defaults, at + 8),
                        word(defaults, at + 12),
                    ];
                    let mut offset = 0;
                    let mut n = 0;
                    if recp {
                        rc = api.decode_apdb_integer(index, count, &mut offset, &mut n);
                        if tn_type[0] == 0 && n != u32::MAX {
                            record[1] = word(mem, object + 108).wrapping_add(n);
                        }
                        if rc >= 0x80000000 {
                            break;
                        }
                    }
                    rc = api.decode_apdb_integer(index, count, &mut offset, &mut record[2]);
                    if rc >= 0x80000000 {
                        break;
                    }
                    record[3] = index.wrapping_add(offset);
                    if api.insert_apdb_record(
                        word(mem, object + 116),
                        record,
                        if tn_type[0] != 0 {
                            Some(terminated(&tn_type))
                        } else {
                            None
                        },
                    ) == 0
                    {
                        api.apdb_error(log, 0xb798);
                        rc = 0x88e0200a;
                        break;
                    }
                }
                b"TNAL" => {
                    let mut offset = 0;
                    let mut n = 0;
                    rc = api.decode_apdb_integer(index, count, &mut offset, &mut n);
                    table_defaults[0] = offset;
                    if rc >= 0x80000000 {
                        break;
                    }
                    let base = word(mem, object + 108).wrapping_add(n);
                    rc = api.decode_apdb_integer(index, count, &mut offset, &mut n);
                    table_defaults[0] = offset;
                    if rc >= 0x80000000 {
                        break;
                    }
                    let list = word(mem, object + 108).wrapping_add(n);
                    let mut i = 0;
                    loop {
                        if api.apdb_table_entry(word(mem, object + 116), i, &mut entry) == 0 {
                            break;
                        }
                        let s = word(mem, entry + 4);
                        if api.apdb_alias_matches(mem, s, list) {
                            let len = text(mem, s).len() + text(mem, base).len() + 2;
                            let dst = api.realloc_apdb(heap, s, len as u32);
                            if dst == 0 {
                                api.apdb_error(log, 0xb798);
                                rc = 0x88e0200a;
                                break 'records;
                            }
                            store(mem, entry + 4, dst);
                            let end = dst + text(mem, dst).len() as u32;
                            mem[end as usize] = b'/';
                            let len = text(mem, base).len();
                            mem.copy_within(base as usize..base as usize + len, end as usize + 1);
                            mem[end as usize + 1 + len] = 0;
                            continue 'records;
                        }
                        i += 1;
                        if api.apdb_table_entry(word(mem, object + 116), i, &mut entry) == 0 {
                            break;
                        }
                    }
                    api.apdb_error(log, 0xb7a9);
                    rc = 0x88e02004;
                    break;
                }
                _ => {}
            }
        }
        if rc.wrapping_shl(19) == 0x00a00000 {
            rc = 0;
        } else if rc >= 0x80000000 {
            break 'load rc;
        }
        if !recipe {
            break 'load rc;
        }
        let eng = word(mem, word(mem, object) + 8);
        let heap = word(mem, eng + 4);
        let log = word(mem, eng + 16);
        if uri != 0 && mem[uri as usize] != 0 {
            let (v, status) = apdb_uri_value(mem, api, heap, log, uri, b"urisuffix");
            if status < 0x80000000 {
                let old = word(mem, object + 96);
                if old != 0 {
                    api.free_apdb(heap, old);
                }
                store(mem, object + 96, v);
            }
            let (v, status) = apdb_uri_value(mem, api, heap, log, uri, b"uriprefix");
            if status < 0x80000000 {
                mem[object as usize + 15] = 2;
                let old = word(mem, object + 92);
                if old != 0 {
                    api.free_apdb(heap, old);
                }
                store(mem, object + 92, v);
                break 'load 0;
            }
        }
        let context = word(mem, eng + 12);
        let mut frequency = 0;
        rc = api.apdb_frequency(word(mem, eng + 20), &mut frequency);
        if rc < 0x80000000 {
            let mut path = [0u8; 128];
            let mut len = 0;
            for s in [
                b"apdb_cs/".as_slice(),
                text(mem, word(mem, object + 44)),
                b"/",
                text(mem, word(mem, object + 88)),
                b"/f",
            ] {
                path[len..len + s.len()].copy_from_slice(s);
                len += s.len();
            }
            let mut number = [0u8; 11];
            let signed = frequency as i32 / 1000;
            let digits = decimal(signed.unsigned_abs(), &mut number);
            if signed < 0 {
                path[len] = b'-';
                len += 1;
            }
            path[len..len + digits.len()].copy_from_slice(digits);
            len += digits.len();
            let mut stream = 0;
            rc = api.apdb_stream(context, &path[..len], &mut stream);
            if rc < 0x80000000 {
                api.close_apdb_stream(context, stream);
                mem[object as usize + 15] = 1;
                let old = word(mem, object + 92);
                if old != 0 {
                    api.free_apdb(heap, old);
                }
                let dst = api.alloc_apdb(heap, len as u32 + 27);
                store(mem, object + 92, dst);
                if dst == 0 {
                    api.apdb_error(log, 0xb798);
                    break 'load 0x88e0200a;
                }
                let prefix = b"x-vocalizer-speechbase://";
                mem[dst as usize..dst as usize + prefix.len()].copy_from_slice(prefix);
                mem[dst as usize + prefix.len()..dst as usize + prefix.len() + len]
                    .copy_from_slice(&path[..len]);
                mem[dst as usize + prefix.len() + len] = 0;
                break 'load rc;
            }
        }
        let mut broker = 0;
        rc = api.apdb_broker(context, filename, &mut broker);
        if rc >= 0x80000000 {
            mem[object as usize + 15] = 2;
            rc = api.copy_apdb_string(mem, heap, filename, object + 16);
            break 'load rc;
        }
        api.close_apdb_broker(context, broker);
        api.apdb_error(log, 0xb7aa);
        0x88e02014
    };
    if entry != 0 {
        let close = api.close_apdb_entry(entry);
        if close >= 0x80000000 && rc < 0x80000000 {
            rc = close;
        }
    }
    if rc >= 0x80000000 {
        if object != 0 {
            api.destroy_apdb(mem, object);
            store(mem, output, 0);
        }
    } else {
        store(mem, object + 8, weight);
        let eng = word(mem, word(mem, object) + 8);
        if api.apdb_logging(word(mem, eng + 16)) {
            api.inspect_apdb(mem, object);
        }
    }
    rc
}

/// Caller-owned prompt tree and heap services. Node operations use the original
/// interface slot numbers: 9=create, 10=after, 11=before, 13=first, 15=next,
/// 19=count. Field sizes count elements, not bytes.
pub trait PromptInsertionServices {
    fn allocate(&mut self, heap: u32, size: u32) -> u32;
    fn resize(&mut self, heap: u32, old: u32, size: u32) -> u32;
    fn free(&mut self, heap: u32, buffer: u32);
    fn error(&mut self, log: u32, code: u32);
    fn skipped(&mut self, log: u32);
    fn node(&mut self, slot: u32, handle: [u32; 2], kind: u32, parent: u32, out: &mut u16) -> u32;
    fn get(&mut self, handle: [u32; 2], node: u16, field: u32, out: &mut [u8]) -> u32;
    fn set(&mut self, handle: [u32; 2], node: u16, field: u32, count: u16, value: &[u8]) -> u32;
    fn pointer(&mut self, handle: [u32; 2], node: u16, field: u32, out: &mut u32) -> u32;
    fn delete(&mut self, handle: [u32; 2], node: u16) -> u32;
}

fn find_byte(m: &[u8], p: u32, c: u8) -> Option<u32> {
    // strchr also finds the terminating zero.
    let n = text(m, p).len();
    m[p as usize..=p as usize + n]
        .iter()
        .position(|&b| b == c)
        .map(|i| p.wrapping_add(i as u32))
}

fn boundary_token(m: &mut [u8], p: u32, position: u32) {
    store_half(m, p, 1);
    for off in [4, 8, 16, 20] {
        store(m, p + off, 0);
    }
    store(m, p + 12, position);
}

/// fn_2784c8 / EU fn_278684 / KR fn_278730.
/// Inserts prompt text, boundary tokens, and tree spans into caller-owned memory.
pub fn insert_prompt_text(
    mem: &mut [u8],
    prompt: u32,
    relative: u16,
    start: i32,
    output: u32,
    tokens: u32,
    count: u32,
    index: u32,
    handle: [u32; 2],
    inserted: u32,
    api: &mut impl PromptInsertionServices,
) -> u32 {
    let owner = word(mem, word(mem, prompt));
    let mgr = word(mem, owner + 8);
    let heap = word(mem, mgr + 4);
    let log = word(mem, mgr + 16);
    let source = word(mem, prompt + 16);
    let delimiter = find_byte(mem, source, 17);
    let original_len = delimiter.map_or_else(
        || text(mem, source).len() as u16,
        |p| p.wrapping_sub(source) as u16,
    );
    let buffer = if delimiter.is_some() {
        let p = api.allocate(heap, original_len as u32 + 1);
        if p == 0 {
            api.error(log, 0xb798);
            return 0x88e0200a;
        }
        mem.copy_within(
            source as usize..source as usize + original_len as usize,
            p as usize,
        );
        mem[p as usize + original_len as usize] = 0;
        p
    } else {
        source
    };
    let mut skip_cleanup = false;
    let rc = (|| {
        macro_rules! check {
            ($v:expr) => {{
                let rc = $v;
                if rc >= 0x80000000 {
                    return rc;
                }
                rc
            }};
        }
        macro_rules! node {
            ($slot:expr,$kind:expr,$parent:expr,$out:expr) => {
                check!(api.node($slot, handle, $kind, $parent, $out))
            };
        }
        macro_rules! get16 {
            ($id:expr,$field:expr,$out:ident) => {{
                let mut b = $out.to_le_bytes();
                check!(api.get(handle, $id, $field, &mut b));
                $out = u16::from_le_bytes(b);
            }};
        }
        macro_rules! set16 {
            ($id:expr,$field:expr,$v:expr) => {
                check!(api.set(handle, $id, $field, 1, &($v as u16).to_le_bytes()))
            };
        }
        macro_rules! set8 {
            ($id:expr,$v:expr) => {
                check!(api.set(handle, $id, 0, 1, &[$v]))
            };
        }
        if half(mem, prompt + 28) == 0 || original_len == 0 {
            return 0x88e02007;
        }
        let old = word(mem, output);
        let punctuation = word(mem, word(mem, prompt) + 104);
        let mut end = relative;
        while (text(mem, old).len() as u16) > end
            && find_byte(mem, punctuation, mem[(old + end as u32) as usize]).is_some()
        {
            end = end.wrapping_add(1);
        }
        let mut length = original_len;
        let mut suffix = 0u32;
        if text(mem, old).len() as u16 != end {
            if mem[(old + end as u32) as usize] != b' ' {
                length = length.wrapping_add(1);
                suffix = 1;
            }
            if suffix == 1 {
                end = relative;
            } else {
                end = end.wrapping_add(1);
            }
        }
        let mut begin = relative.saturating_sub(1);
        let mut prefix = 0u32;
        if relative != 0 {
            while begin != 0
                && find_byte(mem, punctuation, mem[(old + begin as u32) as usize]).is_some()
            {
                begin = begin.wrapping_sub(1);
            }
            if mem[(old + begin as u32) as usize] != b' ' {
                length = length.wrapping_add(1);
                prefix = 1;
            }
            begin = if prefix != 0 {
                relative.wrapping_add(1)
            } else {
                begin.wrapping_add(1)
            };
        }
        let mut end = end.wrapping_add(length);
        if prefix == 1 && suffix == 1 {
            store_half(
                mem,
                word(mem, tokens).wrapping_add(index.wrapping_mul(24)),
                0x4000,
            );
            api.skipped(log);
            skip_cleanup = true;
            return 0;
        }
        let size = (text(mem, old).len() as u32)
            .wrapping_add(length as u32)
            .wrapping_add(2);
        let out = api.resize(heap, old, size);
        if out == 0 {
            api.error(log, 0xb798);
            return 0x88e0200a;
        }
        store(mem, output, out);
        let len = text(mem, out).len() as u32;
        let n = len.wrapping_add(2).wrapping_sub(relative as u32);
        if (n as i32) > 0 {
            for i in (0..n).rev() {
                mem[(out + relative as u32 + length as u32 + i) as usize] =
                    mem[(out + relative as u32 + i) as usize];
            }
        }
        if prefix == 1 {
            mem[(out + relative as u32) as usize] = b' ';
        }
        for i in 0..original_len as u32 {
            mem[(out + relative as u32 + prefix + i) as usize] = mem[(buffer + i) as usize];
        }
        if suffix == 1 {
            mem[(out + relative as u32 + length as u32 - 1) as usize] = b' ';
        }
        store_half(mem, inserted, half(mem, inserted).wrapping_add(length));
        let base = word(mem, tokens);
        let original_position = word(
            mem,
            base.wrapping_add(index.wrapping_mul(24)).wrapping_add(12),
        );
        if prefix != 0 && (start as u32).wrapping_add(relative as u32) == original_position {
            store(
                mem,
                base.wrapping_add(index.wrapping_mul(24)).wrapping_add(12),
                original_position.wrapping_add(1),
            );
        }
        let cnt = half(mem, count);
        let mut k = index.wrapping_add(1) as u16;
        while k < cnt {
            let p = base + k as u32 * 24 + 12;
            store(mem, p, word(mem, p).wrapping_add(length as u32));
            k = k.wrapping_add(1);
        }
        let left_position = (start as u32).wrapping_add(begin as u32);
        let right_position = (start as u32).wrapping_add(end as u32);
        let has_boundary = |position| {
            (0..cnt).any(|k| {
                let p = base + k as u32 * 24;
                half(mem, p) == 1 && word(mem, p + 12) == position
            })
        };
        let left = !has_boundary(left_position);
        let right = !has_boundary(right_position) && text(mem, out).len() as u16 != end;
        let mut at = index;
        while (cnt as u32) > at && word(mem, base + at * 24 + 12) == original_position {
            at = (at as u16).wrapping_add(1) as u32;
        }
        if prefix != 0 && (start as u32).wrapping_add(relative as u32) == original_position {
            at = (at as u16).wrapping_add(1) as u32;
        }
        let spaces = text(mem, buffer).iter().filter(|&&c| c == b' ').count() as u16;
        let added = spaces.wrapping_add(left as u16).wrapping_add(right as u16);
        let new_base = api.resize(heap, base, (cnt as u32 + added as u32).wrapping_mul(24));
        if new_base == 0 {
            api.error(log, 0xb798);
            return 0x88e0200a;
        }
        store(mem, tokens, new_base);
        let tail = (cnt as u32).wrapping_sub(at).wrapping_mul(24);
        mem.copy_within(
            (new_base + at * 24) as usize..(new_base + at * 24 + tail) as usize,
            (new_base + (at + added as u32) * 24) as usize,
        );
        if left {
            boundary_token(mem, new_base + at * 24, left_position);
            at = (at as u16).wrapping_add(1) as u32;
        }
        for i in 0..text(mem, buffer).len() as u32 {
            if mem[(buffer + i) as usize] == b' ' {
                boundary_token(
                    mem,
                    new_base + at * 24,
                    i.wrapping_add(prefix)
                        .wrapping_add(original_position)
                        .wrapping_add(1),
                );
                at = (at as u16).wrapping_add(1) as u32;
            }
        }
        if right {
            boundary_token(mem, new_base + at * 24, right_position);
        }
        store_half(mem, count, cnt.wrapping_add(added));
        let mut root = 0u16;
        let mut total = 0u16;
        let mut cur = 0u16;
        let mut lo = 0u16;
        let mut hi = 0u16;
        let mut kind = 5u8;
        // First pass shifts all existing spans.
        node!(13, 1, 0, &mut root);
        let rc = api.node(19, handle, 3, root as u32, &mut total);
        if rc < 0x80000000 && total != 0 {
            node!(13, 3, root as u32, &mut cur);
            while cur != 0 {
                get16!(cur, 1, lo);
                get16!(cur, 2, hi);
                if lo >= relative {
                    lo = lo.wrapping_add(length);
                    set16!(cur, 1, lo);
                }
                if relative < hi {
                    hi = hi.wrapping_add(length);
                    set16!(cur, 2, hi);
                }
                let id = cur;
                node!(15, id as u32, 0, &mut cur);
            }
        }
        let mut first = 0u16;
        let mut before = 0u16;
        let mut after = 0u16;
        let mut saved_node = 0u16;
        let mut saved_lo = 0u16;
        let mut saved_hi = 0u16;
        let mut saved_text = 0u32;
        let mut last = 0u16;
        node!(13, 1, 0, &mut root);
        let rc = api.node(19, handle, 3, root as u32, &mut total);
        if rc < 0x80000000 && total != 0 {
            node!(13, 3, root as u32, &mut cur);
            while cur != 0 {
                get16!(cur, 1, lo);
                get16!(cur, 2, hi);
                check!(api.get(handle, cur, 0, core::slice::from_mut(&mut kind)));
                if kind == 3 && lo < relative && relative < hi {
                    set16!(cur, 2, relative);
                    let id = cur;
                    node!(10, id as u32, 0, &mut cur);
                    set8!(cur, kind);
                    lo = relative.wrapping_add(length);
                    set16!(cur, 1, lo);
                    set16!(cur, 2, hi);
                    lo = relative;
                }
                if kind == 5 {
                    saved_node = cur;
                    check!(api.pointer(handle, cur, 4, &mut saved_text));
                    saved_lo = lo;
                    saved_hi = hi;
                }
                last = cur;
                if first == 0 && lo >= relative {
                    first = cur;
                }
                if before == 0 && lo >= begin {
                    before = cur;
                }
                if after == 0 && lo >= end {
                    after = cur;
                    break;
                }
                let id = cur;
                node!(15, id as u32, 0, &mut cur);
            }
        } else {
            cur = 0;
        }
        if last == 0 {
            node!(9, 3, root as u32, &mut first);
        } else if cur == 0 {
            node!(10, last as u32, 0, &mut first);
        } else {
            let id = first;
            node!(11, id as u32, 0, &mut first);
        }
        if after != 0 {
            node!(15, after as u32, 0, &mut cur);
            if cur != 0 {
                check!(api.get(handle, cur, 0, core::slice::from_mut(&mut kind)));
                if kind == 5 {
                    after = 0;
                }
            }
            if after != 0 {
                let id = after;
                node!(10, id as u32, 0, &mut after);
            }
        }
        if prefix != 0 {
            set8!(first, 3);
            set16!(first, 1, relative);
            set16!(first, 2, relative.wrapping_add(1));
            let id = first;
            node!(10, id as u32, 0, &mut first);
        }
        set8!(first, 6);
        set16!(first, 1, relative.wrapping_add(prefix as u16));
        set16!(
            first,
            2,
            relative.wrapping_add(length).wrapping_sub(suffix as u16)
        );
        check!(api.set(handle, first, 4, 5, b"_PR_\0"));
        check!(api.set(
            handle,
            first,
            3,
            original_len.wrapping_add(1),
            &mem[buffer as usize..buffer as usize + original_len as usize + 1]
        ));
        lo = begin;
        hi = end;
        while mem[(out + end as u32) as usize] == b' ' {
            end = end.wrapping_add(1);
        }
        if relative as u32 + prefix == begin as u32 {
            let id = first;
            node!(10, id as u32, 0, &mut first);
            before = first;
        } else {
            let id = before;
            node!(10, id as u32, 0, &mut before);
        }
        set8!(before, 5);
        set16!(before, 1, lo);
        set16!(before, 2, hi);
        let mut status = check!(api.set(handle, before, 4, 7, b"prompt\0"));
        if suffix == 1 {
            let id = first;
            node!(10, id as u32, 0, &mut first);
            lo = relative.wrapping_add(length).wrapping_sub(1);
            hi = relative.wrapping_add(length);
            set8!(first, 3);
            set16!(first, 1, lo);
            status = set16!(first, 2, hi);
        }
        if saved_text != 0 {
            if begin > saved_lo {
                status = set16!(saved_node, 2, begin);
            } else {
                status = check!(api.delete(handle, saved_node));
            }
        }
        if after == 0 || saved_text == 0 {
            return status;
        }
        lo = end;
        hi = saved_hi;
        set8!(after, 5);
        set16!(after, 1, lo);
        set16!(after, 2, hi);
        let bytes = text(mem, saved_text).len() + 1;
        api.set(
            handle,
            after,
            4,
            bytes as u16,
            &mem[saved_text as usize..saved_text as usize + bytes],
        )
    })();
    if !skip_cleanup && delimiter.is_some() && buffer != 0 {
        api.free(heap, buffer);
    }
    rc
}

/// Services needed when applying a prompt's orthography and pronunciation.
pub trait PromptApplicationServices: PromptInsertionServices {
    fn voice_text(&mut self, handle: [u32; 2], wide: u32, condition: u32, out: &mut u32) -> u32;
    fn remember(&mut self, map: u32, value: u32) -> u32;
    fn create_span(
        &mut self,
        heap: u32,
        kind: u32,
        position: u32,
        length: u32,
        name: u32,
        count: u32,
        items: u32,
    ) -> u32;
    fn update_span(
        &mut self,
        heap: u32,
        node: u32,
        kind: u32,
        position: u32,
        length: u32,
        name: u32,
        count: u32,
        items: u32,
    ) -> u32;
    /// Builds and logs owner-name + "::" + prompt-name, retaining the builder's
    /// return/error handling. Its result does not replace the prompt operation's result.
    fn application_log(
        &mut self,
        mem: &[u8],
        heap: u32,
        log: u32,
        owner_name: u32,
        prompt_name: u32,
    ) -> u32;
}

fn segment_length(m: &[u8], p: u32) -> u16 {
    find_byte(m, p, 17).map_or(text(m, p).len() as u16, |q| q.wrapping_sub(p) as u16)
}
fn word_separator(m: &[u8], p: u32) -> u16 {
    let n = segment_length(m, p);
    (0..n)
        .find(|&i| matches!(m[(p + i as u32) as usize], b' ' | b'_'))
        .unwrap_or(u16::MAX)
}
fn pronunciation_separator(m: &[u8], p: u32) -> u16 {
    let n = text(m, p).len() as u16;
    (0..n)
        .find(|&i| {
            let c = m[(p + i as u32) as usize];
            c == b'#'
                || (c == b'-' && m[(p + i as u32 + 1) as usize] == b'*')
                || (c == b'%' && m[(p + i as u32 + 1) as usize] == b'#')
        })
        .unwrap_or(u16::MAX)
}
fn widen_text(m: &mut [u8], dst: u32, src: u32) {
    let n = text(m, src).len() as u32;
    for i in 0..=n {
        store_half(m, dst + i * 2, m[(src + i) as usize] as u16);
    }
}

/// fn_276c50 / EU fn_276e0c / KR fn_276eb8.
/// Applies one prompt and adjusts text, domain entries, span nodes, and tree
/// pronunciation records without owning an allocator.
pub fn apply_prompt(
    mem: &mut [u8],
    prompt: u32,
    pos: u16,
    requested_length: u32,
    start: u32,
    output: u32,
    entries: u32,
    count: u32,
    index: u32,
    mut span: u32,
    handle: [u32; 2],
    offset: u32,
    api: &mut impl PromptApplicationServices,
) -> u32 {
    let owner = word(mem, word(mem, prompt));
    let object = word(mem, prompt);
    let ctx = word(mem, owner + 8);
    let heap = word(mem, ctx + 4);
    let log = word(mem, ctx + 16);
    let mode = mem[(prompt + 4) as usize];
    let orth = word(mem, prompt + 16);
    let pronunciation = word(mem, prompt + 20);
    let name = word(mem, prompt + 8);
    let ids = word(mem, prompt + 24);
    let id_count = half(mem, prompt + 28) as u32;
    let startpos = start.wrapping_add(pos as u32);
    let len = find_byte(mem, orth, 17).map_or_else(
        || {
            if requested_length != 0 {
                requested_length
            } else {
                (text(mem, orth).len() as u16).wrapping_add(1) as u32
            }
        },
        |q| q.wrapping_sub(orth).wrapping_add(1) as u16 as u32,
    );
    let mut temp = 0u32;
    let mut cleanup = false;
    let mut stored_buffer = 0u32;
    let rc = (|| {
        macro_rules! check {
            ($e:expr) => {{
                let rc = $e;
                if rc >= 0x80000000 {
                    return rc;
                }
                rc
            }};
        }
        macro_rules! node {
            ($s:expr,$k:expr,$p:expr,$o:expr) => {
                check!(api.node($s, handle, $k, $p, $o))
            };
        }
        macro_rules! get16 {
            ($id:expr,$f:expr,$v:ident) => {{
                let mut b = $v.to_le_bytes();
                let rc = check!(api.get(handle, $id, $f, &mut b));
                $v = u16::from_le_bytes(b);
                rc
            }};
        }
        macro_rules! set16 {
            ($id:expr,$f:expr,$v:expr) => {
                check!(api.set(handle, $id, $f, 1, &($v as u16).to_le_bytes()))
            };
        }
        macro_rules! nomem {
            () => {{
                api.error(log, 0xb798);
                return 0x88e0200a;
            }};
        }
        if mode == 2 {
            if mem[(object + 15) as usize] == 1 {
                let prefix = word(mem, object + 92);
                if prefix == 0 {
                    return 0;
                }
                let mut suffix = [0u8; 64];
                let mut n = 0usize;
                for i in 0..id_count {
                    let mut digits = [0u8; 11];
                    let d = decimal(word(mem, ids + i * 4), &mut digits);
                    suffix[n] = b'#';
                    n += 1;
                    suffix[n..n + d.len()].copy_from_slice(d);
                    n += d.len();
                }
                let a = text(mem, prefix).len() as u32;
                let p = api.allocate(heap, (a + n as u32 + 1).wrapping_mul(2));
                if p == 0 {
                    nomem!();
                }
                widen_text(mem, p, prefix);
                for (i, &c) in suffix[..=n].iter().enumerate() {
                    store_half(mem, p + (a + i as u32) * 2, c as u16);
                }
                let map = word(mem, owner + 44);
                if api.remember(map, p) != 0 {
                    stored_buffer = p;
                } else {
                    api.free(heap, p);
                    nomem!();
                }
            } else {
                if word(mem, owner + 32) == 0 {
                    return 0;
                }
                let prefix = word(mem, object + 92);
                let suffix = word(mem, object + 96);
                let a = if prefix == 0 {
                    0
                } else {
                    text(mem, prefix).len() as u32
                };
                let b = if suffix == 0 {
                    0
                } else {
                    text(mem, suffix).len() as u32
                };
                let p = api.allocate(
                    heap,
                    (a + b + text(mem, name).len() as u32 + 1).wrapping_mul(2),
                );
                if p == 0 {
                    nomem!();
                }
                if prefix != 0 {
                    widen_text(mem, p, prefix);
                }
                widen_text(mem, p + a * 2, name);
                if suffix != 0 {
                    let mut n = 0;
                    while half(mem, p + n * 2) != 0 {
                        n += 1;
                    }
                    widen_text(mem, p + n * 2, suffix);
                }
                let condition = if prefix != 0 {
                    0
                } else {
                    word(mem, object + 16)
                };
                let rc = api.voice_text(
                    [word(mem, owner + 36), word(mem, owner + 40)],
                    p,
                    condition,
                    &mut stored_buffer,
                );
                api.free(heap, p);
                if rc >= 0x80000000 {
                    return 0;
                }
            }
        }
        let mut root = 0u16;
        let mut cur = 0u16;
        let mut mode_byte = 11u8;
        let mut begin = 0u16;
        let mut end = 0u16;
        let mut release_pitch = 0u16;
        let mut first_changed = true;
        node!(13, 1, 0, &mut root);
        node!(13, 2, root as u32, &mut cur);
        while cur != 0 {
            check!(api.get(handle, cur, 0, core::slice::from_mut(&mut mode_byte)));
            get16!(cur, 1, begin);
            let mut remove = 0u16;
            if matches!(mode_byte, 4 | 9 | 10) && !(begin < pos || pos as u32 + len <= begin as u32)
            {
                if mode == 2 {
                    mode_byte = if first_changed {
                        first_changed = false;
                        14
                    } else {
                        12
                    };
                    check!(api.set(handle, cur, 0, 1, &[mode_byte]));
                } else {
                    remove = cur;
                }
            }
            let id = cur;
            node!(15, id as u32, 0, &mut cur);
            if remove != 0 {
                get16!(remove, 8, release_pitch);
                check!(api.delete(handle, remove));
            }
        }
        let mut words = 1u16;
        let mut phonemes = 1u16;
        let mut delta = 0u16;
        if mode == 1 {
            let mut at = 0u16;
            let mut sep = word_separator(mem, orth);
            while sep != u16::MAX {
                words = words.wrapping_add(1);
                at = at.wrapping_add(sep).wrapping_add(1);
                sep = word_separator(mem, orth + at as u32);
            }
            let mut at = 0u16;
            loop {
                let sep = pronunciation_separator(mem, pronunciation + at as u32);
                if sep == u16::MAX {
                    break;
                }
                at = at.wrapping_add(sep).wrapping_add(1);
                phonemes = phonemes.wrapping_add(1);
                while b"-*%#".contains(&mem[(pronunciation + at as u32) as usize]) {
                    at = at.wrapping_add(1);
                }
            }
            if phonemes > words {
                delta = phonemes.wrapping_sub(words).wrapping_mul(2);
                temp = api.allocate(heap, delta as u32 + 1);
                if temp == 0 {
                    nomem!();
                }
                mem[temp as usize] = 0;
                // Each inserted separator is the ROM's two-byte "\x15 " string.
                let mut k = 0u16;
                while k < phonemes - words {
                    let n = text(mem, temp).len() as u32;
                    write_text(mem, temp + n, b"\x15 ");
                    k = k.wrapping_add(1);
                }
            }
            if temp != 0 {
                cleanup = true;
                let old = word(mem, output);
                let n = text(mem, old).len() as u32;
                let extra = text(mem, temp).len() as u32;
                let out = api.resize(heap, old, n.wrapping_add(extra).wrapping_add(2));
                if out == 0 {
                    nomem!();
                }
                store(mem, output, out);
                let mut at = text(mem, out).len() as u32 + 1;
                if pos as u32 <= at {
                    loop {
                        let b = mem[(out + at) as usize];
                        at = at.wrapping_sub(1);
                        mem[(out + extra + at + 1) as usize] = b;
                        if (pos as i32) > (at as i32) {
                            break;
                        }
                    }
                }
                for i in 0..extra {
                    mem[(out + pos as u32 + i) as usize] = mem[(temp + i) as usize];
                }
                store_half(mem, offset, half(mem, offset).wrapping_add(extra as u16));
                let base = word(mem, entries);
                let same = word(mem, base + index * 24 + 12);
                let cnt = half(mem, count);
                let mut i = index;
                while i < (cnt as u32) && word(mem, base + i * 24 + 12) == same {
                    i = (i as u16).wrapping_add(1) as u32;
                }
                while i < (cnt as u32) {
                    let p = base + i * 24 + 12;
                    store(mem, p, word(mem, p).wrapping_add(extra));
                    i = (i as u16).wrapping_add(1) as u32;
                }
                node!(13, 1, 0, &mut root);
                node!(13, 2, root as u32, &mut cur);
                while cur != 0 {
                    get16!(cur, 1, begin);
                    get16!(cur, 2, end);
                    if begin >= pos {
                        begin = begin.wrapping_add(extra as u16);
                        set16!(cur, 1, begin);
                    }
                    if pos <= end {
                        end = end.wrapping_add(extra as u16);
                        set16!(cur, 2, end);
                    }
                    let id = cur;
                    node!(15, id as u32, 0, &mut cur);
                }
                let mut n = word(mem, span + 24);
                while n != 0 {
                    store(mem, n + 4, word(mem, n + 4).wrapping_add(delta as u32));
                    n = word(mem, n + 24);
                }
            }
        }
        cleanup = true;
        let npos = word(mem, span + 4);
        let nlen = word(mem, span + 8);
        let kind = mem[span as usize] as u32;
        let next = word(mem, span + 24);
        let status;
        if startpos > npos {
            status = check!(api.update_span(
                heap,
                span,
                kind,
                npos,
                startpos.wrapping_sub(npos),
                0,
                0,
                0
            ));
            let n = api.create_span(
                heap,
                0,
                startpos,
                len.wrapping_add(delta as u32),
                if mode == 1 { pronunciation } else { 0 },
                if mode == 1 { id_count } else { 0 },
                if mode == 1 { ids } else { 0 },
            );
            if n == 0 {
                api.error(log, 0xb798);
                store(mem, span + 24, next);
                return 0x88e0200a;
            }
            store(mem, span + 24, n);
            span = n;
        } else {
            status = check!(api.update_span(
                heap,
                span,
                0,
                word(mem, span + 4),
                len.wrapping_add(delta as u32),
                if mode == 1 { pronunciation } else { 0 },
                if mode == 1 { id_count } else { 0 },
                if mode == 1 { ids } else { 0 }
            ));
        }
        if npos.wrapping_add(nlen) > startpos {
            let total = startpos.wrapping_add(len).wrapping_add(delta as u32);
            let n = api.create_span(
                heap,
                kind,
                total,
                npos.wrapping_add(nlen).wrapping_sub(total),
                0,
                0,
                0,
            );
            if n == 0 {
                api.error(log, 0xb798);
                store(mem, span + 24, next);
                return 0x88e0200a;
            }
            store(mem, span + 24, n);
            span = n;
        }
        store(mem, span + 24, next);
        if mode != 1 {
            let base = word(mem, entries);
            let cnt = half(mem, count);
            let mut at = index;
            while at < (cnt as u32) && word(mem, base + at * 24 + 12) <= startpos {
                at = (at as u16).wrapping_add(1) as u32;
            }
            let p = api.resize(heap, base, (cnt as u32 + 1) * 24);
            if p == 0 {
                nomem!();
            }
            store(mem, entries, p);
            if at < (cnt as u32) {
                mem.copy_within(
                    (p + at * 24) as usize..(p + cnt as u32 * 24) as usize,
                    (p + (at + 1) * 24) as usize,
                );
            }
            store_half(mem, count, cnt.wrapping_add(1));
            let e = p + at * 24;
            mem[e as usize..e as usize + 24].fill(0);
            store_half(mem, e, 26);
            if at != 0 {
                store(mem, e + 4, word(mem, e - 20));
            }
            store(mem, e + 12, startpos);
            store(mem, e + 20, stored_buffer);
            return status;
        }
        let out = word(mem, output);
        let mut at = pos;
        for _ in 0..words.saturating_sub(phonemes) {
            let sep = word_separator(mem, out + at as u32);
            if sep == u16::MAX {
                return 0x88e02007;
            }
            at = at.wrapping_add(sep);
            mem[(out + at as u32) as usize] = b'-';
            at = at.wrapping_add(1);
        }
        let mut previous = 0u16;
        node!(13, 1, 0, &mut root);
        node!(13, 2, root as u32, &mut cur);
        while cur != 0 {
            get16!(cur, 1, begin);
            if begin >= pos {
                break;
            }
            previous = cur;
            let id = cur;
            node!(15, id as u32, 0, &mut cur);
        }
        at = pos;
        let mut phonetic_at = 0u16;
        let mut status = 0;
        for part in 0..phonemes {
            let mut orth_len = word_separator(mem, out + at as u32);
            let mut pronunciation_len =
                pronunciation_separator(mem, pronunciation + phonetic_at as u32);
            if orth_len == u16::MAX {
                orth_len = text(mem, out + at as u32).len() as u16;
            }
            if pronunciation_len == u16::MAX {
                pronunciation_len = text(mem, pronunciation + phonetic_at as u32).len() as u16;
            }
            if previous == 0 {
                node!(9, 2, root as u32, &mut previous);
            } else {
                let id = previous;
                node!(10, id as u32, 0, &mut previous);
            }
            let old_phonetic = phonetic_at;
            let old_at = at;
            phonetic_at = phonetic_at.wrapping_add(pronunciation_len);
            at = at.wrapping_add(orth_len);
            mode_byte = if first_changed {
                first_changed = false;
                13
            } else {
                11
            };
            check!(api.set(handle, previous, 0, 1, &[mode_byte]));
            set16!(previous, 1, old_at);
            set16!(previous, 2, at);
            let n = (phonetic_at as u32).wrapping_sub(old_phonetic as u32) as usize;
            let mut buffer = [0u8; 64];
            buffer[..n].copy_from_slice(
                &mem[(pronunciation + old_phonetic as u32) as usize
                    ..(pronunciation + old_phonetic as u32) as usize + n],
            );
            while mem[(out + at as u32) as usize] != 0
                && b" _".contains(&mem[(out + at as u32) as usize])
            {
                at = at.wrapping_add(1);
            }
            while b"-*%#".contains(&mem[(pronunciation + phonetic_at as u32) as usize]) {
                phonetic_at = phonetic_at.wrapping_add(1);
            }
            let mut pitch = if phonetic_at != 0
                && mem[(pronunciation + phonetic_at as u32 - 1) as usize] == b'#'
            {
                200u16
            } else {
                0u16
            };
            if phonetic_at != 0
                && mem[(pronunciation + phonetic_at as u32 - 1) as usize] == b'#'
                && mem[(pronunciation + phonetic_at as u32) as usize] == b'['
            {
                let mut b = [0u8; 16];
                let mut n = 0;
                let mut p = phonetic_at.wrapping_add(1);
                while !matches!(mem[(pronunciation + p as u32) as usize], 0 | b']') && n < 10 {
                    b[n] = mem[(pronunciation + p as u32) as usize];
                    n += 1;
                    p = p.wrapping_add(1);
                }
                pitch = parse_decimal(&b[..n]) as u16;
                phonetic_at = p.wrapping_add(1);
            }
            check!(api.set(handle, previous, 3, (n + 1) as u16, &buffer[..n + 1]));
            check!(api.set(handle, previous, 5, 2, b"*\0"));
            check!(api.set(handle, previous, 6, 2, b"*\0"));
            if part == phonemes - 1 {
                pitch = release_pitch;
            }
            set16!(previous, 8, pitch);
            status = check!(api.set(handle, previous, 9, 1, &[0]));
            let mut next = previous;
            while next != 0 {
                status = get16!(next, 1, begin);
                if begin >= at {
                    break;
                }
                previous = next;
                let id = next;
                status = node!(15, id as u32, 0, &mut next);
            }
        }
        status
    })();
    if cleanup && temp != 0 {
        api.free(heap, temp);
    }
    if rc < 0x80000000 {
        api.application_log(mem, heap, log, word(mem, word(mem, prompt) + 88), name);
    }
    rc
}

/// INET's 24-byte tagged value. The tag occupies the low halfword.
#[derive(Clone, Copy, Default)]
pub struct FetchValue(pub [u32; 6]);

/// Caller-provided heap, resource, speechbase, and INET operations.
pub trait AudioFetchServices {
    fn validate(&mut self, object: u32, tag: u32) -> u32;
    fn allocate(&mut self, mem: &mut [u8], heap: u32, bytes: u32, zero: bool) -> u32;
    fn resize(&mut self, mem: &mut [u8], heap: u32, request: u32, bytes: u32) -> u32;
    fn free(&mut self, heap: u32, pointer: u32);
    fn error(&mut self, log: u32, code: u32);
    fn resource(&mut self, registry: u32, name: &[u8], out: &mut u32) -> u32;
    fn release_resource(&mut self, registry: u32, name: &[u8]);
    fn frequency(&mut self, properties: u32, out: &mut u32) -> u32;
    fn lookup(&mut self, mem: &[u8], map: u32, key: u32, out: &mut u32) -> u32;
    fn insert_key(&mut self, mem: &[u8], map: u32, key: u32, fields: [u32; 8]) -> u32;
    fn unit(&mut self, interface: u32, object: u32, id: u32, out: &mut [u32; 2]) -> u32;
    fn emit(&mut self, mem: &mut [u8], object: u32, request: u32, out: u32) -> u32;
    fn create_properties(&mut self, interface: u32, owner: [u32; 2], out: &mut [u32; 2]) -> u32;
    fn copy_properties(
        &mut self,
        interface: u32,
        owner: [u32; 2],
        source: [u32; 2],
        out: &mut [u32; 2],
    ) -> u32;
    fn begin_properties(&mut self, interface: u32, source: [u32; 2], out: &mut [u32; 2]) -> u32;
    fn next_property(
        &mut self,
        interface: u32,
        iterator: [u32; 2],
        name: &mut u32,
        value: &mut u32,
    ) -> u32;
    fn end_properties(&mut self, interface: u32, iterator: [u32; 2]) -> u32;
    fn close_properties(&mut self, interface: u32, properties: [u32; 2]) -> u32;
    fn set_property(
        &mut self,
        mem: &[u8],
        interface: u32,
        properties: [u32; 2],
        name: &[u8],
        value: FetchValue,
    ) -> u32;
    fn get_property(
        &mut self,
        interface: u32,
        properties: [u32; 2],
        name: &[u8],
        out: &mut FetchValue,
    ) -> u32;
    fn release_value(&mut self, interface: u32, properties: [u32; 2], value: FetchValue) -> u32;
    fn open(
        &mut self,
        interface: u32,
        owner: [u32; 2],
        uri: u32,
        flags: u32,
        properties: [u32; 2],
        result: [u32; 2],
        stream: &mut u32,
    ) -> u32;
    fn close(&mut self, interface: u32, owner: [u32; 2]) -> u32;
    fn read(
        &mut self,
        mem: &mut [u8],
        interface: u32,
        owner: [u32; 2],
        buffer: u32,
        count: &mut u32,
        stream: u32,
    ) -> u32;
    fn parse_format(
        &mut self,
        mime: &[u8],
        encoding: &mut u8,
        rate: &mut u16,
        channels: &mut u16,
        order: Option<&mut u8>,
    ) -> bool;
    fn decode(
        &mut self,
        mem: &mut [u8],
        encoding: u8,
        owner: [u32; 2],
        uri: u32,
        data: u32,
        count: u32,
        mime: &mut [u8; 64],
        out: &mut u32,
        size: &mut u32,
    ) -> u32;
}

fn fetch_release(mem: &mut [u8], heap: u32, request: u32, api: &mut impl AudioFetchServices) {
    let refs = half(mem, request);
    if refs > 1 {
        store_half(mem, request, refs - 1);
    } else {
        let p = word(mem, request + 4);
        if p != 0 {
            api.free(heap, p);
        }
        api.free(heap, request);
    }
}
fn fetch_copy_uri(mem: &mut [u8], dest: u32, src: u32) {
    let n = text(mem, src).len() + 1;
    mem.copy_within(src as usize..src as usize + n, dest as usize);
}

/// fn_22e8b4 / EU fn_22ea70 / KR fn_22eb1c.
/// Fetch speechbase or INET audio, decode containers, and submit a request.
/// Arena addresses and callback state belong to the caller.
pub fn fetch_audio(
    mem: &mut [u8],
    object: u32,
    tag: u32,
    wide_uri: u32,
    base_uri: u32,
    properties: [u32; 2],
    out: u32,
    api: &mut impl AudioFetchServices,
) -> u32 {
    const BAD: u32 = 0x87402004;
    const OOM: u32 = 0x8740200a;
    if wide_uri == 0 || half(mem, wide_uri) == 0 || out == 0 {
        return 0x87402007;
    }
    store(mem, out, 0);
    let rc = api.validate(object, tag);
    if rc >= 0x80000000 {
        return rc;
    }
    let env = word(mem, object + 8);
    let heap = word(mem, env + 4);
    let log = word(mem, env + 16);
    let registry = word(mem, env + 24);
    let mut len = 0u32;
    while half(mem, wide_uri + len * 2) != 0 {
        len += 1;
    }
    let uri = api.allocate(mem, heap, len.wrapping_add(1), false);
    if uri == 0 {
        api.error(log, 100);
        return OOM;
    }
    // fn_176460 substitutes '?' for code units outside the byte range.
    for i in 0..=len {
        let c = half(mem, wide_uri + i * 2);
        mem[(uri + i) as usize] = if c > 255 { b'?' } else { c as u8 };
    }
    let mut pa = [0; 2];
    let mut pb = [0; 2];
    let mut pc = [0; 2];
    let mut mime_value = FetchValue::default();
    let mut resource = 0;
    let mut request = 0;
    let mut stream = 0;
    let mut decoded = 0;
    let mut iface = word(mem, object + 32);
    let rc = (|| {
        let speech = {
            let prefix = b"x-vocalizer-speechbase://";
            let s = text(mem, uri);
            s.len() >= prefix.len() && &s[..prefix.len()] == prefix
        };
        if speech {
            let key = uri + 25;
            let hash = text(mem, uri)
                .iter()
                .position(|&b| b == b'#')
                .map(|i| uri + i as u32);
            let Some(hash) = hash else {
                api.error(log, 154);
                return 0x87402007;
            };
            mem[hash as usize] = 0;
            let s = text(mem, hash + 1);
            let mut at = 0;
            while at < s.len() && matches!(s[at], 9..=13 | 32) {
                at += 1;
            }
            let negative = s.get(at) == Some(&b'-');
            if negative || s.get(at) == Some(&b'+') {
                at += 1;
            }
            let mut id = 0u32;
            let mut overflow = false;
            while at < s.len() && s[at].is_ascii_digit() {
                if let Some(v) = id
                    .checked_mul(10)
                    .and_then(|v| v.checked_add((s[at] - b'0') as u32))
                {
                    id = v;
                } else {
                    overflow = true;
                }
                at += 1;
            }
            if overflow {
                id = u32::MAX;
            }
            if negative {
                id = id.wrapping_neg();
            }
            let map = word(mem, object + 24);
            let mut entry = 0;
            let mut rc = api.lookup(mem, map, key, &mut entry);
            if rc >= 0x80000000 {
                let mut rate = 0;
                let props = word(mem, env + 20);
                if props != 0 {
                    let mut v = 0;
                    if api.frequency(props, &mut v) < 0x80000000 {
                        rate = v as u16 as u32;
                    }
                }
                rc = api.insert_key(
                    mem,
                    map,
                    key,
                    [
                        rate,
                        key,
                        word(mem, object),
                        word(mem, object + 4),
                        env,
                        word(mem, object + 12),
                        0,
                        0,
                    ],
                );
                if rc >= 0x80000000 {
                    api.error(log, 155);
                } else {
                    rc = api.lookup(mem, map, key, &mut entry);
                }
            }
            mem[hash as usize] = b'#';
            if rc >= 0x80000000 || entry == 0 || word(mem, entry + 28) == 0 {
                return rc;
            }
            let req = api.allocate(
                mem,
                heap,
                (text(mem, uri).len() as u32).wrapping_add(45),
                true,
            );
            if req == 0 {
                api.error(log, 100);
                return OOM;
            }
            store(mem, req + 12, req + 44);
            store(mem, req + 28, req + 44);
            store_half(mem, req, 1);
            let unit_iface = word(mem, entry + 24);
            store(mem, req + 16, unit_iface);
            store(mem, req + 20, word(mem, entry + 28));
            let mut data = [0; 2];
            rc = api.unit(unit_iface, word(mem, entry + 28), id, &mut data);
            if rc >= 0x80000000 {
                api.error(log, 156);
            } else {
                store(mem, req + 24, data[0]);
                store(mem, req + 8, data[1]);
                fetch_copy_uri(mem, req + 44, uri);
                rc = api.emit(mem, object, req, out);
            }
            fetch_release(mem, heap, req, api);
            return rc;
        }
        if iface == 0 {
            if mem[(object + 28) as usize] == 0 {
                return 0x87402001;
            }
            let mut found = 0;
            let rc = api.resource(registry, b"INET", &mut found);
            if rc >= 0x80000000 || found == 0 {
                api.error(log, 128);
            } else if word(mem, found + 4) == 0 || mem[found as usize] != 1 {
                api.error(log, 129);
                api.release_resource(registry, b"INET");
            } else {
                iface = word(mem, found + 4);
                store(mem, object + 32, iface);
                store(mem, object + 36, word(mem, found + 8));
                store(mem, object + 40, word(mem, found + 12));
            }
            mem[(object + 28) as usize] = 0;
            if iface == 0 {
                return 0x87402001;
            }
        }
        let owner = [word(mem, object), word(mem, object + 4)];
        let inet_owner = [word(mem, object + 36), word(mem, object + 40)];
        macro_rules! properties_check {
            ($e:expr) => {{
                let rc = $e;
                if rc >= 0x80000000 {
                    api.error(log, 100);
                    return rc;
                }
                rc
            }};
        }
        properties_check!(api.create_properties(iface, owner, &mut pa));
        if api.resource(registry, b"INETPROPERTIES", &mut resource) < 0x80000000
            && properties[0] != 0
        {
            properties_check!(api.copy_properties(
                iface,
                owner,
                [word(mem, resource + 8), word(mem, resource + 12)],
                &mut pb
            ));
            properties_check!(api.begin_properties(iface, properties, &mut pc));
            loop {
                let mut name = 0;
                let mut value = 0;
                if api.next_property(iface, pc, &mut name, &mut value) != 0 {
                    break;
                }
                let v = FetchValue(core::array::from_fn(|i| word(mem, value + i as u32 * 4)));
                properties_check!(api.set_property(mem, iface, pb, text(mem, name), v));
            }
            api.end_properties(iface, pc);
        }
        if base_uri != 0 && mem[base_uri as usize] != 0 {
            if pb[0] == 0 {
                if resource != 0 && word(mem, resource + 8) != 0 {
                    properties_check!(api.copy_properties(
                        iface,
                        owner,
                        [word(mem, resource + 8), word(mem, resource + 12)],
                        &mut pb
                    ));
                } else if properties[0] == 0 {
                    properties_check!(api.create_properties(iface, owner, &mut pb));
                } else {
                    properties_check!(api.copy_properties(iface, owner, properties, &mut pb));
                }
            }
            properties_check!(api.set_property(
                mem,
                iface,
                pb,
                b"inet.urlBase",
                FetchValue([2, base_uri, 0, 0, 0, 0])
            ));
        }
        let selected = if pb[0] != 0 {
            pb
        } else if resource != 0 && word(mem, resource + 8) != 0 {
            [word(mem, resource + 8), word(mem, resource + 12)]
        } else {
            properties
        };
        let mut rc = api.open(iface, inet_owner, uri, 0, selected, pa, &mut stream);
        if rc >= 0x80000000 || stream == 0 {
            stream = 0;
            return rc;
        }
        if pb != [0, 0] {
            api.close_properties(iface, pb);
            pb = [0, 0];
        }
        let mut value = FetchValue::default();
        let mut capacity = 0u32;
        if api.get_property(iface, pa, b"inet.info.sizeBytes", &mut value) < 0x80000000 {
            if value.0[0] as u16 == 0 && value.0[1] != 0 {
                capacity = value.0[1].wrapping_add(1);
            }
            api.release_value(iface, pa, value);
        }
        if capacity == 0 {
            capacity = 64000;
        }
        let need = ((text(mem, uri).len() as u32) | 1).wrapping_add(1);
        if capacity > 0u32.wrapping_sub(45).wrapping_sub(need) {
            api.error(log, 131);
            return BAD;
        }
        rc = api.get_property(iface, pa, b"inet.info.mimeType", &mut mime_value);
        if rc >= 0x80000000
            || mime_value.0[0] as u16 != 2
            || mime_value.0[1] == 0
            || text(mem, mime_value.0[1]).len() > 64
        {
            api.error(log, 132);
            return BAD;
        }
        let mut encoding = 0;
        let mut rate = 0;
        let mut channels = 0;
        let mut order = 0;
        if !api.parse_format(
            text(mem, mime_value.0[1]),
            &mut encoding,
            &mut rate,
            &mut channels,
            Some(&mut order),
        ) {
            api.error(log, 133);
            return BAD;
        }
        let mut expected_rate = 0;
        if word(mem, env + 20) != 0 {
            let mut v = 0;
            if api.frequency(word(mem, env + 20), &mut v) < 0x80000000 {
                expected_rate = v as u16 as u32;
            }
        }
        if order == 0 && rate as u32 != expected_rate {
            api.error(log, 134);
            return BAD;
        }
        request = api.allocate(
            mem,
            heap,
            need.wrapping_add(capacity).wrapping_add(44),
            false,
        );
        if request == 0 {
            api.error(log, 100);
            return OOM;
        }
        mem[request as usize..request as usize + 44].fill(0);
        store(mem, request + 12, request + 44);
        store(
            mem,
            request + 28,
            request.wrapping_add(need).wrapping_add(44),
        );
        store_half(mem, request, 1);
        store_half(mem, request + 36, rate);
        store_half(mem, request + 38, channels);
        mem[(request + 40) as usize] = encoding;
        fetch_copy_uri(mem, request + 44, uri);
        let max = 0u32.wrapping_sub(need.wrapping_add(64045));
        let mut count = 0u32;
        loop {
            let mut n = capacity.wrapping_sub(count);
            rc = api.read(
                mem,
                iface,
                inet_owner,
                word(mem, request + 28).wrapping_add(count),
                &mut n,
                stream,
            );
            if rc < 0x80000000 {
                count = count.wrapping_add(n);
            }
            if rc == 0 {
                if count < capacity {
                    continue;
                }
                if capacity > max {
                    api.error(log, 131);
                    return BAD;
                }
                capacity = capacity.wrapping_add(64000);
                let resized = api.resize(
                    mem,
                    heap,
                    request,
                    need.wrapping_add(capacity).wrapping_add(44),
                );
                if resized == 0 {
                    api.error(log, 100);
                    return OOM;
                }
                request = resized;
                store(mem, request + 12, request + 44);
                store(
                    mem,
                    request + 28,
                    request.wrapping_add(need).wrapping_add(44),
                );
                continue;
            }
            if rc >= 0x80000000 || rc.wrapping_shl(19) != 0x180000 {
                return rc;
            }
            if count == 0 && (5..=7).contains(&encoding) {
                return BAD;
            }
            if capacity > count.wrapping_add(1) {
                let resized = api.resize(
                    mem,
                    heap,
                    request,
                    need.wrapping_add(count).wrapping_add(44),
                );
                if resized == 0 {
                    api.error(log, 100);
                    return OOM;
                }
                request = resized;
                store(mem, request + 12, request + 44);
                store(
                    mem,
                    request + 28,
                    request.wrapping_add(need).wrapping_add(44),
                );
            }
            if !(5..=7).contains(&encoding) {
                store(mem, request + 32, count);
                if channels != 0 {
                    if channels == 2 {
                        count >>= 1;
                    }
                    store(
                        mem,
                        request + 8,
                        if channels <= 2 {
                            count
                        } else {
                            count / channels as u32
                        },
                    );
                }
            } else {
                let mut mime = [0; 64];
                let mut size = 0;
                rc = api.decode(
                    mem,
                    encoding,
                    owner,
                    uri,
                    word(mem, request + 28),
                    count,
                    &mut mime,
                    &mut decoded,
                    &mut size,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                api.free(heap, request);
                request = api.allocate(mem, heap, size.wrapping_add(need).wrapping_add(44), false);
                if request == 0 {
                    api.error(log, 100);
                    return OOM;
                }
                mem[request as usize..request as usize + 44].fill(0);
                store(mem, request + 12, request + 44);
                store(
                    mem,
                    request + 28,
                    request.wrapping_add(need).wrapping_add(44),
                );
                let n = mime.iter().position(|&b| b == 0).unwrap_or(64);
                if !api.parse_format(&mime[..n], &mut encoding, &mut rate, &mut channels, None) {
                    api.error(log, 136);
                    return BAD;
                }
                if rate as u32 != expected_rate {
                    api.error(log, 137);
                    return BAD;
                }
                store_half(mem, request, 1);
                store_half(mem, request + 36, rate);
                store_half(mem, request + 38, channels);
                mem[(request + 40) as usize] = encoding;
                if channels != 0 {
                    store(mem, request + 8, size / channels as u32);
                }
                store(mem, request + 32, size);
                fetch_copy_uri(mem, request + 44, uri);
                let dest = word(mem, request + 28);
                mem.copy_within(
                    decoded as usize..decoded as usize + size as usize,
                    dest as usize,
                );
                api.free(heap, decoded);
                decoded = 0;
            }
            if word(mem, request + 32) == 0 {
                api.error(log, 138);
            }
            return api.emit(mem, object, request, out);
        }
    })();
    if resource != 0 {
        api.release_resource(registry, b"INETPROPERTIES");
    }
    if pb != [0, 0] {
        api.close_properties(iface, pb);
    }
    if pa != [0, 0] {
        if mime_value.0[1] != 0 {
            api.release_value(iface, pa, mime_value);
        }
        api.close_properties(iface, pa);
        if stream != 0 {
            api.close(iface, [word(mem, object + 36), word(mem, object + 40)]);
        }
    }
    if decoded != 0 {
        api.free(heap, decoded);
    }
    if request != 0 {
        fetch_release(mem, heap, request, api);
    }
    api.free(heap, uri);
    rc
}
