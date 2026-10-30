//! Caller-owned linguistic and domain glue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    Validate,
    Session,
    ReleaseResource,
    Free,
    DestroyList,
    DestroyMap,
    Lock,
    Unlock,
    Signal,
    ConvertError,
    Guard,
    Restore,
    Forward,
    Unload,
    CreateProperties,
    Begin,
    Next,
    Item,
    End,
    Refresh,
    Accumulate,
    PropertyValue,
    SetProperty,
    DestroyProperties,
    AllocateZeroed,
    AcquireResource,
    Lookup,
    RemoveIndex,
    LanguageList,
    ReleaseCallback,
    CloseVoice,
    DetachPool,
    CloseLog,
    CloseSynthStream,
    CloseLinguistic,
    CloseParameters,
    CloseSynthesis,
    CloseLingDb,
    CloseEngine,
    CloseProvider,
    CloseStream,
    ClosePool,
    ReleaseWorker,
    ResetHandle,
}
/// Host boundaries use caller-owned offsets, never native pointers.
/// The reply is status/result followed by up to two output words.
pub trait Host {
    fn call(&mut self, operation: Boundary, arguments: &[u32]) -> [u32; 3];
    /// Memory-aware boundary for callers whose host services mutate their graphs.
    fn call_memory(&mut self, operation: Boundary, _memory: &mut [u8], arguments: &[u32]) -> [u32; 3] {
        self.call(operation, arguments)
    }
}
fn word(m: &[u8], at: u32) -> u32 {
    u32::from_le_bytes(m[at as usize..at as usize + 4].try_into().unwrap())
}
fn call(h: &mut impl Host, op: Boundary, args: &[u32]) -> u32 {
    h.call(op, args)[0]
}
fn session(m: &[u8], p: u32, h: &mut impl Host) -> [u32; 3] {
    h.call(Boundary::Session, &[word(m, p), word(m, p + 4)])
}
fn validate(p: u32, tag: u32, expected: u32, size: u32, h: &mut impl Host) -> u32 {
    call(h, Boundary::Validate, &[p, tag, expected, size])
}
fn release(m: &[u8], p: u32, labels: &[u32], h: &mut impl Host) {
    let env = word(m, p);
    for label in labels {
        call(h, Boundary::ReleaseResource, &[word(m, env + 24), *label]);
    }
    call(h, Boundary::Free, &[word(m, env + 4), p]);
}

/// fn_200c18 / EU fn_200dd4 / KR fn_200e80.
pub fn fn_200c18(m: &[u8], p: u32, tag: u32, h: &mut impl Host) -> u32 {
    let rc = validate(p, tag, 62339, 32, h);
    if rc >= 0x80000000 {
        return 2343575560;
    }
    if p == 0 {
        return rc;
    }
    release(m, p, &[0, 2, 3], h);
    0
}

/// fn_1f3d24 / EU fn_1f3ee0 / KR fn_1f3f8c.
pub fn fn_1f3d24(m: &[u8], p: u32, tag: u32, h: &mut impl Host) -> u32 {
    let rc = validate(p, tag, 62338, 52, h);
    if rc >= 0x80000000 {
        return 2342526984;
    }
    if p == 0 {
        return rc;
    }
    release(m, p, &[0, 1, 2, 3], h);
    0
}

/// fn_200c88 / EU fn_200e44 / KR fn_200ef0.
pub fn fn_200c88(m: &[u8], p: u32, tag: u32, a: u32, h: &mut impl Host) -> u32 {
    let rc = validate(p, tag, 0xf383, 32, h);
    if rc >= 0x80000000 {
        return 0x8bb02008;
    }
    call(
        h,
        Boundary::Forward,
        &[72, word(m, p + 12), word(m, p + 16), a],
    )
}

/// fn_200d38 / EU fn_200ef4 / KR fn_200fa0.
pub fn fn_200d38(m: &[u8], p: u32, tag: u32, a: u32, b: u32, h: &mut impl Host) -> u32 {
    let rc = validate(p, tag, 0xf383, 32, h);
    if rc >= 0x80000000 {
        return 0x8bb02008;
    }
    call(
        h,
        Boundary::Forward,
        &[80, word(m, p + 12), word(m, p + 16), a, b],
    )
}

/// fn_200d80 / EU fn_200f3c / KR fn_200fe8.
pub fn fn_200d80(m: &[u8], p: u32, tag: u32, a: u32, b: u32, h: &mut impl Host) -> u32 {
    let rc = validate(p, tag, 0xf383, 32, h);
    if rc >= 0x80000000 {
        return 0x8bb02008;
    }
    call(
        h,
        Boundary::Forward,
        &[84, word(m, p + 12), word(m, p + 16), a, b],
    )
}

/// fn_200dc8 / EU fn_200f84 / KR fn_201030.
pub fn fn_200dc8(m: &[u8], p: u32, tag: u32, a: u32, b: u32, h: &mut impl Host) -> u32 {
    let rc = validate(p, tag, 0xf383, 32, h);
    if rc >= 0x80000000 {
        return 0x8bb02008;
    }
    call(
        h,
        Boundary::Forward,
        &[88, word(m, p + 12), word(m, p + 16), a, b],
    )
}

/// fn_1fd854 / EU fn_1fda10 / KR fn_1fdabc.
pub fn fn_1fd854(m: &[u8], p: u32, tag: u32, h: &mut impl Host) -> u32 {
    let rc = validate(p, tag, 0x11d, 72, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let [rc, env, _] = session(m, p, h);
    if rc >= 0x80000000 {
        return rc;
    }
    for (at, label) in [(32, 4), (20, 5), (16, 0), (12, 1)] {
        if word(m, p + at) != 0 {
            call(h, Boundary::ReleaseResource, &[word(m, env + 24), label]);
        }
    }
    for (at, op) in [
        (64, Boundary::DestroyList),
        (60, Boundary::DestroyMap),
        (44, Boundary::DestroyList),
    ] {
        let v = word(m, p + at);
        if v != 0 {
            call(h, op, &[v]);
        }
    }
    call(h, Boundary::Free, &[word(m, env + 4), p]);
    rc
}

fn object_valid(m: &[u8], p: u32, tag: u32, expected: u32) -> bool {
    tag == expected && p != 0 && word(m, p) == expected.wrapping_add(0x0b000000)
}
fn pause(m: &[u8], p: u32, tag: u32, operation: u32, h: &mut impl Host) -> u32 {
    if !object_valid(m, p, tag, 0x684f424a) {
        return 0x80000008;
    }
    let lock = word(m, p + 36);
    let mut rc = call(h, Boundary::Lock, &[lock]);
    let mut state = 0;
    if rc < 0x80000000 {
        state = m[p as usize + 40];
        rc = call(h, Boundary::Unlock, &[lock]);
    }
    if rc >= 0x80000000 {
        return call(h, Boundary::ConvertError, &[rc]);
    }
    if state != 3 && state != 4 {
        return 0x80000011;
    }
    rc = call(h, Boundary::Signal, &[p, operation]);
    call(h, Boundary::ConvertError, &[rc])
}

/// fn_194920 / EU fn_1949d0 / KR fn_194a0c.
pub fn fn_194920(m: &[u8], p: u32, tag: u32, h: &mut impl Host) -> u32 {
    pause(m, p, tag, 64, h)
}

/// fn_194a4c / EU fn_194afc / KR fn_194b38.
pub fn fn_194a4c(m: &[u8], p: u32, tag: u32, h: &mut impl Host) -> u32 {
    pause(m, p, tag, 128, h)
}

/// fn_194644 / EU fn_1946f4 / KR fn_194730.
pub fn fn_194644(m: &[u8], p: u32, tag: u32, resource: [u32; 2], h: &mut impl Host) -> u32 {
    if !object_valid(m, p, tag, 0x684f424a) {
        return 0x80000008;
    }
    let [rc, locked, _] = h.call(Boundary::Guard, &[p, 2]);
    if rc >= 0x80000000 {
        return call(h, Boundary::ConvertError, &[rc]);
    }
    let rc = call(
        h,
        Boundary::Unload,
        &[word(m, p + 44), word(m, p + 48), resource[0], resource[1]],
    );
    call(h, Boundary::Restore, &[p, locked as u8 as u32]);
    call(h, Boundary::ConvertError, &[rc])
}

/// fn_1954b8 / EU fn_195568 / KR fn_1955a4.
pub fn fn_1954b8(m: &[u8], p: u32, tag: u32, count: Option<&mut u16>) -> u32 {
    if !object_valid(m, p, tag, 0x68434c53) {
        return 0x80000008;
    }
    let Some(count) = count else {
        return 0x80000006;
    };
    *count = 0;
    0x80000001
}

/// fn_1fdd0c / EU fn_1fdec8 / KR fn_1fdf74.
pub fn fn_1fdd0c(m: &[u8], p: u32, h: &mut impl Host) -> u32 {
    let [rc, env, _] = session(m, p, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let [rc, properties, _] = h.call(Boundary::CreateProperties, &[word(m, env + 4)]);
    if rc >= 0x80000000 {
        return rc;
    }
    let [rc, iter, _] = h.call(Boundary::Begin, &[word(m, p + 60), 0, 0]);
    if rc < 0x80000000 {
        loop {
            let [rc, _, container] = h.call(Boundary::Next, &[iter]);
            if rc >= 0x80000000 {
                break;
            }
            let mut i = 0u32;
            loop {
                let [rc, item, _] = h.call(Boundary::Item, &[word(m, container + 8), i]);
                if rc == 0 {
                    break;
                }
                call(h, Boundary::Accumulate, &[word(m, item), properties]);
                i = i.wrapping_add(1);
            }
        }
        call(h, Boundary::End, &[iter]);
    }
    let v = call(h, Boundary::PropertyValue, &[properties]);
    let rc = call(h, Boundary::SetProperty, &[word(m, env + 20), v]);
    call(h, Boundary::DestroyProperties, &[properties]);
    rc
}

/// fn_1fdec0 / EU fn_1fe07c / KR fn_1fe128.
pub fn fn_1fdec0(m: &mut [u8], p: u32, tag: u32, h: &mut impl Host) -> u32 {
    let rc = validate(p, tag, 0x11d, 72, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let [rc, _, _] = session(m, p, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let [rc, iter, _] = h.call(Boundary::Begin, &[word(m, p + 60), 0, 0]);
    if rc < 0x80000000 {
        loop {
            let [rc, _, container] = h.call(Boundary::Next, &[iter]);
            if rc >= 0x80000000 {
                break;
            }
            let mut i = 0u32;
            loop {
                let [rc, item, _] = h.call(Boundary::Item, &[word(m, container + 8), i]);
                if rc == 0 {
                    break;
                }
                let at = word(m, item).wrapping_add(13);
                m[at as usize] = 0;
                i = i.wrapping_add(1);
            }
        }
        call(h, Boundary::End, &[iter]);
    }
    let rc = call(h, Boundary::Refresh, &[p]);
    m[p as usize + 68] = 0;
    rc
}

fn toggle(
    m: &mut [u8],
    p: u32,
    tag: u32,
    target: u32,
    target_tag: u32,
    on: bool,
    h: &mut impl Host,
) -> u32 {
    let rc = validate(p, tag, 0x11d, 72, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let rc = validate(target, target_tag, 0x13c, 128, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let rc = session(m, p, h)[0];
    if rc >= 0x80000000 {
        return rc;
    }
    let mut code = 0x88e0240du32;
    let [rc, iter, _] = h.call(Boundary::Begin, &[word(m, p + 60), 0, 0]);
    if rc < 0x80000000 {
        loop {
            let [rc, _, container] = h.call(Boundary::Next, &[iter]);
            if rc >= 0x80000000 {
                break;
            }
            let alt = 0x88e0241a;
            let mut i = 0u32;
            loop {
                let [rc, item, _] = h.call(Boundary::Item, &[word(m, container + 8), i]);
                if rc == 0 {
                    break;
                }
                if word(m, item) == target {
                    let flag = &mut m[target as usize + 12];
                    if on {
                        if *flag != 0 {
                            code = alt;
                        } else {
                            *flag = 1;
                            code = 0;
                        }
                    } else if *flag != 0 {
                        *flag = 0;
                        code = 0;
                    } else {
                        code = 0x88e0240d;
                    }
                    break;
                }
                i = i.wrapping_add(1);
            }
        }
        call(h, Boundary::End, &[iter]);
    }
    code
}

/// fn_200030 / EU fn_2001ec / KR fn_200298.
pub fn fn_200030(
    m: &mut [u8],
    p: u32,
    tag: u32,
    target: u32,
    target_tag: u32,
    h: &mut impl Host,
) -> u32 {
    toggle(m, p, tag, target, target_tag, true, h)
}

/// fn_20018c / EU fn_200348 / KR fn_2003f4.
pub fn fn_20018c(
    m: &mut [u8],
    p: u32,
    tag: u32,
    target: u32,
    target_tag: u32,
    h: &mut impl Host,
) -> u32 {
    toggle(m, p, tag, target, target_tag, false, h)
}

/// fn_1f4498 / EU fn_1f4654 / KR fn_1f4700.
pub fn fn_1f4498(m: &[u8], p: u32, tag: u32, h: &mut impl Host) -> u32 {
    let rc = validate(p, tag, 0xf424, 64, h);
    if rc >= 0x80000000 {
        return 0x8c802008;
    }
    if p == 0 {
        return rc;
    }
    let env = word(m, p);
    for (at, label) in [(4, 1), (8, 2), (28, 3)] {
        if word(m, p + at) != 0 {
            call(h, Boundary::ReleaseResource, &[word(m, env + 24), label]);
        }
    }
    call(h, Boundary::Free, &[word(m, env + 4), p]);
    0
}

/// fn_1f4558 / EU fn_1f4714 / KR fn_1f47c0.
pub fn fn_1f4558(
    m: &mut [u8],
    handle: [u32; 2],
    output: Option<&mut [u32; 2]>,
    h: &mut impl Host,
) -> u32 {
    let Some(output) = output else {
        return 0x8c802007;
    };
    *output = [0, 0];
    let [rc, env, _] = h.call_memory(Boundary::Session, m, &handle);
    if rc >= 0x80000000 {
        return rc;
    }
    let p = h.call_memory(Boundary::AllocateZeroed, m, &[word(m, env + 4), 1, 64])[0];
    if p == 0 {
        return 0x8c80200a;
    }
    m[p as usize..p as usize + 4].copy_from_slice(&env.to_le_bytes());
    let mut entries = [0; 3];
    let mut status = 0;
    for (i, label) in [1, 2, 3].iter().enumerate() {
        let [rc, entry, _] = h.call_memory(Boundary::AcquireResource, m, &[word(m, env + 24), *label]);
        status = rc;
        if rc >= 0x80000000 {
            break;
        }
        entries[i] = entry;
    }
    if status < 0x80000000 {
        for (dst, src) in [
            (4, entries[0] + 4),
            (8, entries[1] + 4),
            (12, entries[1] + 8),
            (16, entries[1] + 12),
            (28, entries[2] + 4),
            (20, entries[2] + 8),
            (24, entries[2] + 12),
        ] {
            let v = word(m, src);
            m[(p + dst) as usize..(p + dst + 4) as usize].copy_from_slice(&v.to_le_bytes());
        }
    }
    *output = [p, 0xf424];
    if status >= 0x80000000 {
        // The inlined close suppresses its own error mapping.
        if validate(p, 0xf424, 0xf424, 64, h) < 0x80000000 {
            let env = word(m, p);
            for (at, label) in [(4, 1), (8, 2), (28, 3)] {
                if word(m, p + at) != 0 {
                    call(h, Boundary::ReleaseResource, &[word(m, env + 24), label]);
                }
            }
            call(h, Boundary::Free, &[word(m, env + 4), p]);
        }
        *output = [0, 0];
    }
    status
}

/// fn_200cc8 / EU fn_200e84 / KR fn_200f30.
pub fn fn_200cc8(m: &[u8], p: u32, tag: u32, args: [u32; 7], h: &mut impl Host) -> u32 {
    if validate(p, tag, 0xf383, 32, h) >= 0x80000000 {
        return 0x8bb02008;
    }
    call(
        h,
        Boundary::Forward,
        &[
            76,
            word(m, p + 12),
            word(m, p + 16),
            args[0],
            args[1],
            args[2],
            args[3],
            args[4],
            args[5],
            args[6],
        ],
    )
}
fn store(m: &mut [u8], at: u32, value: u32) {
    m[at as usize..at as usize + 4].copy_from_slice(&value.to_le_bytes());
}
fn half(m: &[u8], at: u32) -> u16 {
    u16::from_le_bytes(m[at as usize..at as usize + 2].try_into().unwrap())
}
fn store_half(m: &mut [u8], at: u32, value: u16) {
    m[at as usize..at as usize + 2].copy_from_slice(&value.to_le_bytes());
}
fn widen(m: &mut [u8], src: u32, dst: u32, cap: u16) -> u32 {
    let mut len = 0;
    while m[(src + len) as usize] != 0 {
        len += 1;
    }
    if len > cap.wrapping_sub(1) as u32 {
        store_half(m, dst, 0);
        return 0x80000009;
    }
    for i in 0..=len {
        store_half(m, dst + i * 2, m[(src + i) as usize] as u16);
    }
    0
}

/// fn_194b7c / EU fn_194c2c / KR fn_194c68.
pub fn fn_194b7c(
    m: &mut [u8],
    p: u32,
    tag: u32,
    output: u32,
    count: u32,
    h: &mut impl Host,
) -> u32 {
    if !object_valid(m, p, tag, 0x68434c53) {
        return 0x80000008;
    }
    if count == 0 {
        return 0x80000006;
    }
    let env = word(m, p + 0xb0);
    let capacity = half(m, count) as u32;
    let entries = if capacity != 0 && output != 0 {
        let v = call(
            h,
            Boundary::AllocateZeroed,
            &[word(m, env + 4), capacity, 0x56],
        );
        if v == 0 {
            return call(h, Boundary::ConvertError, &[0x8000000a]);
        }
        v
    } else {
        0
    };
    let [mut rc, n, _] = h.call(
        Boundary::LanguageList,
        &[word(m, p + 0xcc), word(m, p + 0xd0), entries, capacity],
    );
    if (rc < 0x80000000 || rc & 0x1fff == 9) && entries != 0 {
        for i in 0..n.min(capacity) {
            for (src, dst, cap) in [(0, 0, 0x80), (0x40, 0x100, 4), (0x48, 0x108, 0x80)] {
                rc = widen(m, entries + i * 0x56 + src, output + i * 0x20a + dst, cap);
                if rc >= 0x80000000 {
                    return call(h, Boundary::ConvertError, &[rc]);
                }
            }
            let id = half(m, entries + i * 0x56 + 0x54);
            store_half(m, output + i * 0x20a + 0x208, id);
        }
    }
    store_half(m, count, n as u16);
    if entries != 0 {
        call(h, Boundary::Free, &[word(m, env + 4), entries]);
    }
    call(h, Boundary::ConvertError, &[rc])
}

/// Property writer for fn_195570. The host consumes the temporary filename
/// before returning; no pointer to the local buffer escapes.
pub trait FilenameHost: Host {
    fn set_filename(&mut self, properties: u32, filename: &[u8]) -> u32;
}
fn append_utf16(m: &[u8], src: u32, dst: &mut [u8]) -> u32 {
    let mut len = 0;
    while half(m, src + len * 2) != 0 {
        len += 1;
    }
    if len + 1 > dst.len() as u32 {
        dst[0] = 0;
        return 0x80000007;
    }
    for i in 0..len {
        let c = half(m, src + i * 2);
        dst[i as usize] = if c > 255 { b'?' } else { c as u8 };
    }
    dst[len as usize] = 0;
    0
}

/// fn_195570 / EU fn_195620 / KR fn_19565c.
pub fn fn_195570(
    m: &[u8],
    p: u32,
    tag: u32,
    name: u32,
    dir: u32,
    h: &mut impl FilenameHost,
) -> u32 {
    if !object_valid(m, p, tag, 0x684f424a) {
        return 0x80000008;
    }
    let [rc, env, _] = h.call(Boundary::Session, &[word(m, p + 8), word(m, p + 12)]);
    if rc >= 0x80000000 {
        return call(h, Boundary::ConvertError, &[rc]);
    }
    let [rc, flag, _] = h.call(Boundary::Guard, &[p, 2]);
    if rc >= 0x80000000 {
        return call(h, Boundary::ConvertError, &[rc]);
    }
    let mut buf = [0u8; 512];
    let mut len = 0;
    if dir != 0 && half(m, dir) != 0 {
        append_utf16(m, dir, &mut buf);
        len = buf.iter().position(|&b| b == 0).unwrap();
        // As in ROM, successful nonempty directory conversion supplies
        // the byte before the terminator.
        if len != 0 && buf[len - 1] != b'/' {
            buf[len] = b'/';
            len += 1;
            buf[len] = 0;
        }
    }
    if name != 0 && half(m, name) != 0 {
        append_utf16(m, name, &mut buf[len..]);
    }
    let len = buf.iter().position(|&b| b == 0).unwrap();
    let rc = h.set_filename(word(m, env + 20), &buf[..len]);
    call(h, Boundary::Restore, &[p, flag as u8 as u32]);
    call(h, Boundary::ConvertError, &[rc])
}

/// Formatter and callback boundaries for fn_18f430.
pub trait FormatHost {
    fn format(&mut self, input: u32, size: u32, output: &mut [u8; 2048]) -> u32;
    fn notify(&mut self, context: u32, arg: u32, text: &[u8; 2048]);
}

/// fn_18f430 / EU fn_18f4e0 / KR fn_18f51c.
pub fn fn_18f430(
    m: &[u8],
    callback: u32,
    arg: u32,
    input: u32,
    size: u32,
    h: &mut impl FormatHost,
) -> u32 {
    let mut buffer = [0u8; 2048];
    let length = h.format(input, size, &mut buffer);
    if length >= 0x7ff {
        buffer[0x7ff] = 0;
    }
    h.notify(word(m, callback + 8), arg, &buffer);
    0
}

/// fn_1906cc / EU fn_19077c / KR fn_1907b8.
pub fn fn_1906cc(m: &mut [u8], p: u32, tag: u32, h: &mut impl Host) -> u32 {
    if !object_valid(m, p, tag, 0x68434c53) {
        return 0x80000008;
    }
    if word(m, p + 0xdc) != 0 {
        return 0x80000011;
    }
    let [rc, env, _] = h.call(Boundary::Session, &[word(m, p + 0xa8), word(m, p + 0xac)]);
    if rc >= 0x80000000 {
        return 0x80000008;
    }
    let own_env = word(m, p + 0xb0);
    let pool = word(m, own_env + 4);
    let stream = word(m, own_env + 8);
    let provider = word(m, own_env + 0x1c);
    let user = word(m, p + 0x20);
    store(m, p, 0x8cbcb3ac);
    let mut status = 0;
    // Entries are in the ROM's destruction order. Failure restores only
    // the magic; resources already closed stay closed.
    for (at, op) in [
        (0xc0, Boundary::ReleaseCallback),
        (0xcc, Boundary::CloseVoice),
    ] {
        if word(m, p + at) != 0 {
            status = call(h, op, &[word(m, p + at), word(m, p + at + 4)]);
            if status >= 0x80000000 {
                store(m, p, 0x73434c53);
                return call(h, Boundary::ConvertError, &[status]);
            }
            let [_, a, b] = h.call(Boundary::ResetHandle, &[]);
            store(m, p + at, a);
            store(m, p + at + 4, b);
        }
    }
    for (at, op) in [
        (env + 0x10, Boundary::CloseLog),
        (p + 0xb4, Boundary::CloseSynthStream),
        (env + 0x18, Boundary::CloseLinguistic),
        (env + 0x14, Boundary::CloseParameters),
        (env + 0xc, Boundary::CloseSynthesis),
        (p + 0xb8, Boundary::CloseLingDb),
    ] {
        let resource = word(m, at);
        if resource != 0 {
            if op == Boundary::CloseLog {
                call(h, Boundary::DetachPool, &[pool, 0, 0]);
            }
            status = call(h, op, &[resource]);
            if status >= 0x80000000 {
                store(m, p, 0x73434c53);
                return call(h, Boundary::ConvertError, &[status]);
            }
            store(m, at, 0);
        }
    }
    if word(m, p + 0xa8) != 0 {
        status = call(
            h,
            Boundary::CloseEngine,
            &[word(m, p + 0xa8), word(m, p + 0xac)],
        );
        if status >= 0x80000000 {
            store(m, p, 0x73434c53);
            return call(h, Boundary::ConvertError, &[status]);
        }
        let [_, a, b] = h.call(Boundary::ResetHandle, &[]);
        store(m, p + 0xa8, a);
        store(m, p + 0xac, b);
    }
    for (v, op) in [
        (provider, Boundary::CloseProvider),
        (stream, Boundary::CloseStream),
        (pool, Boundary::ClosePool),
    ] {
        if v != 0 {
            status = call(h, op, &[v]);
            if status >= 0x80000000 {
                store(m, p, 0x73434c53);
                return call(h, Boundary::ConvertError, &[status]);
            }
        }
    }
    call(h, Boundary::ReleaseWorker, &[user, p]);
    call(h, Boundary::ConvertError, &[status])
}

/// fn_1ff79c / EU fn_1ff958 / KR fn_1ffa04.
/// Strict ROM behavior, including the three C corrections in 8368a40.
pub fn fn_1ff79c(
    m: &mut [u8],
    p: u32,
    tag: u32,
    target: u32,
    target_tag: u32,
    h: &mut impl Host,
) -> u32 {
    let rc = validate(p, tag, 0x11d, 72, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let rc = validate(target, target_tag, 0x13c, 128, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let rc = session(m, p, h)[0];
    if rc >= 0x80000000 {
        return rc;
    }
    let mut code = 0x88e0240d;
    let [rc, iter, _] = h.call(Boundary::Begin, &[word(m, p + 60), 0, 0]);
    if rc >= 0x80000000 {
        return code;
    }
    loop {
        let [rc, _, list] = h.call(Boundary::Next, &[iter]);
        if rc >= 0x80000000 {
            break;
        }
        let mut i = 0u32;
        loop {
            let [rc, item, _] = h.call(Boundary::Item, &[word(m, list + 8), i]);
            if rc == 0 {
                break;
            }
            if word(m, item) == target {
                let key = word(m, target + 88);
                code = validate(p, tag, 0x11d, 72, h);
                if code < 0x80000000 {
                    code = session(m, p, h)[0];
                }
                if code < 0x80000000 {
                    let [rc, value, _] = h.call(Boundary::Lookup, &[word(m, p + 60), key]);
                    code = rc;
                    if code < 0x80000000 && word(m, p + 52) != 0 {
                        let mut j = 0u32;
                        loop {
                            let [_, item2, _] = h.call(Boundary::Item, &[word(m, p + 64), j]);
                            if item2 != 0 && word(m, item2) == value {
                                call(h, Boundary::RemoveIndex, &[word(m, p + 64), j]);
                                let n = word(m, p + 52).wrapping_sub(1);
                                store(m, p + 52, n);
                                break;
                            }
                            j = j.wrapping_add(1);
                            if j >= word(m, p + 52) {
                                break;
                            }
                        }
                    }
                }
                if code >= 0x80000000 {
                    return code;
                }
                call(h, Boundary::RemoveIndex, &[word(m, list + 8), i]);
                code = 0;
                break;
            }
            i = i.wrapping_add(1);
        }
    }
    call(h, Boundary::End, &[iter]);
    if code < 0x80000000 {
        call(h, Boundary::Refresh, &[p]);
    }
    code
}

/// Caller-owned log record. All string and array offsets refer to the
/// caller's memory; the timestamp is local, fixed-size storage.
pub struct LogRecord {
    pub timestamp: [u8; 128],
    pub category: u32,
    pub component: u32,
    pub level: u32,
    pub count: u32,
    pub names: u32,
    pub values: u32,
}
pub trait LogHost: Host {
    fn time(&mut self, services: u32) -> u32;
    fn format_time(&mut self, services: u32, time: u32, output: &mut [u8; 128]);
    fn serialize(&mut self, record: &LogRecord, output: &mut [u8; 3072]) -> u32;
    fn write(&mut self, file: u32, text: &[u8]) -> u32;
}

/// fn_1ce428 / EU fn_1ce5e4 / KR fn_1ce690.
pub fn fn_1ce428(
    m: &[u8],
    component: u32,
    category: u32,
    level: u32,
    count: u32,
    names: u32,
    values: u32,
    h: &mut impl LogHost,
) -> u32 {
    if component == 0 || category == 0 {
        return 0x84b02006;
    }
    let context = word(m, component);
    if m[context as usize + 4] == 0 {
        return 0;
    }
    let services = word(m, word(m, context) + 28);
    let time = h.time(services);
    let mut record = LogRecord {
        timestamp: [0; 128],
        category,
        component,
        level,
        count,
        names,
        values,
    };
    h.format_time(services, time, &mut record.timestamp);
    let mut len = record.timestamp.iter().position(|&c| c == 0).unwrap();
    while len != 0 && (record.timestamp[len - 1] == 10 || record.timestamp[len - 1] == 13) {
        len -= 1;
        record.timestamp[len] = 0;
    }
    let mut output = [0; 3072];
    let mut rc = h.serialize(&record, &mut output);
    if rc < 0x80000000 {
        let lock = word(m, context + 0x190);
        rc = call(h, Boundary::Lock, &[lock]);
        if rc < 0x80000000 {
            let n = output.iter().position(|&c| c == 0).unwrap();
            rc = h.write(word(m, context + 0x194), &output[..n]);
            call(h, Boundary::Unlock, &[lock]);
        }
    }
    rc
}

/// Text callback buffer. Pointers are offsets in caller-owned memory.
pub struct TextBuffer {
    pub length: u32,
    pub text: u32,
    pub count: u32,
    pub records: u32,
}
pub trait TextNotify {
    fn notify(
        &mut self,
        state: u32,
        context: u32,
        command: u32,
        buffer: &mut TextBuffer,
        argument: u32,
    );
}
/// fn_18fbb8 / EU fn_18fc68 / KR fn_18fca4.
pub fn fn_18fbb8(
    m: &mut [u8],
    p: u32,
    mode: u32,
    encoding: u32,
    input: u32,
    length: u32,
    data: u32,
    count: u32,
    h: &mut impl TextNotify,
) -> u32 {
    if mode != 4 && mode != 8 {
        return 0;
    }
    let n = word(m, length);
    let records = word(m, count);
    let mut b = TextBuffer {
        length: if encoding == 1 {
            n.wrapping_sub(0x26)
        } else {
            n
        },
        text: 0,
        count: records,
        records: 0,
    };
    if mode == 4 {
        h.notify(p, word(m, p + 0x16c), 0x40000, &mut b, word(m, p + 0x184));
        store(m, p + 0x174, word(m, length));
        store(m, p + 0x17c, word(m, count));
        store(m, p + 0x178, b.text);
        store(m, p + 0x180, b.records);
    } else {
        let src = word(m, input) as usize;
        let dst = word(m, p + 0x178) as usize;
        if n != 0 && src != 0 {
            if encoding != 1 {
                m.copy_within(src..src + n as usize, dst);
            } else if (m[src] == 0xff && m[src + 1] == 0xfe)
                || (m[src] == 0xfe && m[src + 1] == 0xff)
            {
                m.copy_within(src..src + 2, dst);
                m.copy_within(src + 0x28..src + n as usize, dst + 2);
            } else {
                m.copy_within(src + 0x26..src + n as usize, dst);
            }
        }
        b.text = word(m, p + 0x178);
        b.records = word(m, p + 0x180);
        let start = data as usize;
        m.copy_within(
            start..start + records.wrapping_mul(0x2c) as usize,
            b.records as usize,
        );
        for i in 0..records {
            let at = b.records.wrapping_add(i.wrapping_mul(0x2c)).wrapping_add(4);
            let off = word(m, at);
            if off > 0x26 {
                store(m, at, off.wrapping_sub(0x26));
            }
        }
        h.notify(p, word(m, p + 0x16c), 0x80000, &mut b, word(m, p + 0x184));
    }
    0
}

#[derive(Clone, Copy)]
pub enum SpeechOp {
    Reset,
    Guard,
    Convert,
    Restore,
    GetType,
    Allocate,
    Free,
    Analyze,
    Process,
    Session,
    Create,
    Open,
    Buffer,
    Commit,
    Close,
    Destroy,
}
/// Speech integration returns status followed by two output words.
pub trait SpeechHost {
    fn speech(&mut self, op: SpeechOp, args: &[u32]) -> [u32; 3];
}
/// fn_192004 / EU fn_1920b4 / KR fn_1920f0.
pub fn fn_192004(m: &mut [u8], p: u32, tag: u32, request: u32, h: &mut impl SpeechHost) -> u32 {
    h.speech(SpeechOp::Reset, &[]);
    if tag != 0x684f424a || p == 0 || word(m, p) != 0x734f424a {
        return 0x80000008;
    }
    let guard = h.speech(SpeechOp::Guard, &[p, 3]);
    let locked = guard[1] as u8;
    if guard[0] >= 0x80000000 {
        return h.speech(SpeechOp::Convert, &[guard[0]])[0];
    }
    let mut rc = 0x80002007;
    let mut allocated = 0;
    if request != 0 && word(m, request + 8) != 0 && word(m, request + 4) != 0 {
        if m[request as usize] != 0 {
            rc = 0x80002300;
        } else {
            let manager = word(m, p + 16);
            let reply = h.speech(SpeechOp::GetType, &[word(m, manager + 20)]);
            rc = reply[0];
            if rc < 0x80000000 {
                let start = reply[1] as usize;
                let end = m[start..].iter().position(|&x| x == 0).unwrap() + start;
                let utf16 = &m[start..end] == b"text/plain;charset=utf-16";
                let utf8 = &m[start..end] == b"text/plain;charset=utf-8";
                if !utf16 && !utf8 {
                    rc = 0x80000007;
                } else {
                    let prefix = b"\x1b\\lang=ins_unknown\\";
                    let length = word(m, request + 4).wrapping_add(if utf16 { 0x26 } else { 0x13 });
                    allocated = h.speech(
                        SpeechOp::Allocate,
                        &[
                            word(m, manager + 4),
                            1,
                            length.wrapping_add(if utf16 { 2 } else { 1 }),
                        ],
                    )[0];
                    if allocated != 0 {
                        let dst = allocated as usize;
                        let src = word(m, request + 8) as usize;
                        let n = word(m, request + 4) as usize;
                        if utf16 {
                            m.copy_within(src..src + 2, dst);
                            for (i, &v) in prefix.iter().enumerate() {
                                m[dst + 2 + 2 * i] = v;
                                m[dst + 3 + 2 * i] = 0;
                            }
                            m.copy_within(src + 2..src + n, dst + 0x28);
                        } else {
                            m[dst..dst + prefix.len()].copy_from_slice(prefix);
                            m.copy_within(src..src + n, dst + 0x13);
                        }
                        if word(m, p + 0x170) == 0 {
                            h.speech(SpeechOp::Restore, &[p, locked as u32]);
                            return 0x80000880;
                        }
                        let output = h.speech(SpeechOp::Analyze, &[p, length, allocated]);
                        rc = output[0];
                        if rc < 0x80000000 {
                            rc = h.speech(SpeechOp::Process, &[p, 2, output[1], output[2]])[0];
                        }
                    }
                }
            }
        }
    }
    if allocated != 0 {
        h.speech(SpeechOp::Free, &[word(m, word(m, p + 16) + 4), allocated]);
    }
    if locked != 0 {
        h.speech(SpeechOp::Restore, &[p, locked as u32]);
    }
    h.speech(SpeechOp::Convert, &[rc])[0]
}

#[derive(Clone, Copy)]
pub enum ListOp {
    AllocateString,
    Acquire,
    Resolve,
    AllocateRecords,
    Fetch,
    Release,
    Free,
    Convert,
}
pub trait ListHost {
    fn list(&mut self, op: ListOp, args: &[u32], request: &mut [u8; 86]) -> [u32; 3];
}
fn narrow_string(m: &mut [u8], src: u32, dst: u32) {
    let mut i = 0;
    loop {
        let v = half(m, src + i * 2);
        m[(dst + i) as usize] = if v & 0xff00 != 0 { b'?' } else { v as u8 };
        if v == 0 {
            break;
        }
        i += 1;
    }
}
fn widen_string(m: &mut [u8], src: u32, dst: u32) -> u32 {
    let mut n = 0;
    while m[(src + n) as usize] != 0 {
        n += 1;
    }
    if n > 127 {
        store_half(m, dst, 0);
        return 0x80000009;
    }
    for i in 0..=n {
        store_half(m, dst + 2 * i, m[(src + i) as usize] as u16);
    }
    0
}
fn get_list(
    m: &mut [u8],
    p: u32,
    tag: u32,
    voice: u32,
    id: u32,
    filter: Option<u32>,
    output: u32,
    count: u32,
    h: &mut impl ListHost,
) -> u32 {
    if tag != 0x68434c53 || p == 0 || word(m, p) != 0x73434c53 {
        return 0x80000008;
    }
    if count == 0 || filter == Some(0) {
        return 0x80000006;
    }
    let heap = word(m, word(m, p + 0xb0) + 4);
    let mut request = [0u8; 86];
    let mut strings = [0u32; 2];
    let mut stream = 0;
    let mut records = 0;
    let mut resolved = id as u16;
    let rc = (|| {
        for (i, src) in [voice, filter.unwrap_or(0)].into_iter().enumerate() {
            if src == 0 {
                continue;
            }
            let mut n = 1u32;
            while half(m, src + (n - 1) * 2) != 0 {
                n += 1;
            }
            let size = ((n as u16) as u32) << 1;
            let dst = h.list(ListOp::AllocateString, &[heap, size], &mut request)[0];
            strings[i] = dst;
            if dst == 0 {
                return 0x8000000a;
            }
            // fn_190078 compares its full length against the narrowed capacity.
            if n > (n as u16) as u32 {
                m[dst as usize] = 0;
            } else {
                narrow_string(m, src, dst);
            }
        }
        let reply = h.list(ListOp::Acquire, &[word(m, p + 0xb8)], &mut request);
        stream = reply[1];
        if reply[0] >= 0x80000000 {
            return reply[0];
        }
        let reply = h.list(
            ListOp::Resolve,
            &[word(m, p + 0xb0), strings[0], resolved as u32],
            &mut request,
        );
        strings[0] = reply[1];
        resolved = reply[2] as u16;
        if reply[0] >= 0x80000000 {
            return reply[0];
        }
        let mut i = 0;
        loop {
            let c = m[strings[0] as usize + i];
            request[i] = c;
            if c == 0 {
                break;
            }
            i += 1;
        }
        let maximum = half(m, count) as u32;
        if maximum != 0 && output != 0 {
            records = h.list(
                ListOp::AllocateRecords,
                &[heap, maximum, if filter.is_some() { 0x8e } else { 0x88 }],
                &mut request,
            )[0];
            if records == 0 {
                return 0x8000000a;
            }
        }
        let reply = h.list(
            ListOp::Fetch,
            &[
                word(m, p + 0xcc),
                word(m, p + 0xd0),
                strings[1],
                records,
                maximum,
            ],
            &mut request,
        );
        let mut result = reply[0];
        let returned = reply[1];
        if (result < 0x80000000 || result & 0x1fff == 9) && records != 0 {
            for i in 0..returned.min(half(m, count) as u32) {
                let record = records + i * if filter.is_some() { 0x8e } else { 0x88 };
                let item = output + i * if filter.is_some() { 0x404 } else { 0x502 };
                let fields: &[(u32, u32)] = if filter.is_some() {
                    &[(0x40, 0x200), (0, 0x100), (0x60, 0), (0x6c, 0x300)]
                } else {
                    &[
                        (0, 0x200),
                        (0x20, 0x100),
                        (0x7c, 0),
                        (0x60, 0x300),
                        (0x6c, 0x400),
                    ]
                };
                for (j, &(src, dst)) in fields.iter().enumerate() {
                    result = widen_string(m, record + src, item + dst);
                    if result >= 0x80000000 {
                        return result;
                    }
                    if j == 2 {
                        store_half(
                            m,
                            item + if filter.is_some() { 0x402 } else { 0x500 },
                            resolved,
                        );
                    }
                }
                if filter.is_some() {
                    store_half(m, item + 0x400, half(m, record + 0x8c));
                }
            }
        }
        store_half(m, count, returned as u16);
        if records != 0 {
            h.list(ListOp::Free, &[heap, records], &mut request);
        }
        result
    })();
    if stream != 0 {
        h.list(ListOp::Release, &[stream], &mut request);
    }
    for s in strings {
        if s != 0 {
            h.list(ListOp::Free, &[heap, s], &mut request);
        }
    }
    h.list(ListOp::Convert, &[rc], &mut request)[0]
}
/// fn_194d88 / EU fn_194e38 / KR fn_194e74.
pub fn fn_194d88(
    m: &mut [u8],
    p: u32,
    tag: u32,
    voice: u32,
    id: u32,
    output: u32,
    count: u32,
    h: &mut impl ListHost,
) -> u32 {
    get_list(m, p, tag, voice, id, None, output, count, h)
}
/// fn_1950e8 / EU fn_195198 / KR fn_1951d4.
pub fn fn_1950e8(
    m: &mut [u8],
    p: u32,
    tag: u32,
    voice: u32,
    id: u32,
    filter: u32,
    output: u32,
    count: u32,
    h: &mut impl ListHost,
) -> u32 {
    get_list(m, p, tag, voice, id, Some(filter), output, count, h)
}

/// fn_191aa8 / EU fn_191b58 / KR fn_191b94.
pub fn fn_191aa8(m: &mut [u8], p: u32, tag: u32, request: u32, h: &mut impl SpeechHost) -> u32 {
    if tag != 0x684f424a || p == 0 || word(m, p) != 0x734f424a {
        return 0x80000008;
    }
    let mut rc = h.speech(SpeechOp::Session, &[word(m, p + 8), word(m, p + 12)])[0];
    if rc >= 0x80000000 {
        return h.speech(SpeechOp::Convert, &[rc])[0];
    }
    let guard = h.speech(SpeechOp::Guard, &[p, 3]);
    let locked = guard[1] as u8;
    if guard[0] >= 0x80000000 {
        return h.speech(SpeechOp::Convert, &[guard[0]])[0];
    }
    rc = 0x80002007;
    if request != 0 && word(m, request + 8) != 0 && word(m, request + 4) != 0 {
        if m[request as usize] != 0 {
            rc = 0x80002300;
        } else {
            if word(m, p + 0x170) == 0 {
                h.speech(SpeechOp::Restore, &[p, locked as u32]);
                return 0x80000880;
            }
            let initial = h.speech(SpeechOp::Reset, &[]);
            let mut stream = [initial[1], initial[2]];
            let initial = h.speech(SpeechOp::Reset, &[]);
            let mut text = [initial[1], initial[2]];
            let src = word(m, request + 8) as usize;
            let n = word(m, request + 4);
            rc = 0x80000007;
            if m[src] == b'#' && m[src + n as usize - 1] == b'#' {
                rc = h.speech(SpeechOp::Session, &[word(m, p + 8), word(m, p + 12)])[0];
                if rc < 0x80000000 {
                    let empty = h.speech(SpeechOp::Reset, &[]);
                    let reply = h.speech(
                        SpeechOp::Create,
                        &[empty[1], empty[2], word(m, p + 8), word(m, p + 12)],
                    );
                    rc = reply[0];
                    text = [reply[1], reply[2]];
                    if rc < 0x80000000 {
                        let reply = h.speech(SpeechOp::Open, &[text[0], text[1], 0xc0, 0, 2]);
                        rc = reply[0];
                        stream = [reply[1], reply[2]];
                        if rc < 0x80000000 {
                            let reply = h.speech(SpeechOp::Buffer, &[stream[0], stream[1], n]);
                            rc = reply[0];
                            if rc < 0x80000000 {
                                let dst = reply[1] as usize;
                                const TABLE: [u8; 128] = [
                                    32, 32, 44, 159, 34, 32, 32, 32, 94, 32, 83, 60, 69, 32, 32,
                                    32, 32, 96, 39, 34, 34, 32, 32, 32, 126, 32, 115, 62, 101, 32,
                                    32, 89, 32, 173, 155, 156, 32, 157, 124, 21, 32, 32, 166, 174,
                                    170, 45, 32, 32, 248, 241, 253, 32, 39, 230, 32, 250, 32, 32,
                                    167, 175, 172, 171, 32, 168, 65, 65, 65, 65, 142, 143, 146,
                                    128, 69, 144, 69, 69, 73, 73, 73, 73, 68, 165, 79, 79, 79, 79,
                                    153, 120, 32, 85, 85, 85, 154, 89, 32, 225, 133, 160, 131, 97,
                                    132, 134, 145, 135, 138, 130, 136, 137, 141, 161, 140, 139,
                                    100, 164, 149, 162, 147, 111, 148, 246, 32, 151, 163, 150, 129,
                                    121, 32, 152,
                                ];
                                // The ROM's do/while decrements a 16-bit counter.
                                let remaining = if n as u16 == 0 {
                                    65536
                                } else {
                                    n as u16 as usize
                                };
                                for i in 0..remaining {
                                    let v = m[src + i];
                                    m[dst + i] = if v < 128 {
                                        v
                                    } else {
                                        TABLE[(v - 128) as usize]
                                    };
                                }
                                rc = h.speech(SpeechOp::Commit, &[stream[0], stream[1], n])[0];
                            }
                        }
                    }
                    if stream[0] != 0 {
                        h.speech(SpeechOp::Close, &stream);
                    }
                    if rc >= 0x80000000 && text[0] != 0 {
                        h.speech(SpeechOp::Destroy, &text);
                        h.speech(SpeechOp::Reset, &[]);
                    }
                    if rc < 0x80000000 {
                        rc = h.speech(SpeechOp::Process, &[p, 1, text[0], text[1]])[0];
                    }
                }
            }
        }
    }
    if locked != 0 {
        h.speech(SpeechOp::Restore, &[p, locked as u32]);
    }
    h.speech(SpeechOp::Convert, &[rc])[0]
}

#[derive(Clone, Copy)]
pub enum DomainOpenOp {
    Allocate,
    List,
    Map,
    Resource,
}
pub trait DomainOpenHost: Host {
    fn open_domain(&mut self, op: DomainOpenOp, args: &[u32]) -> [u32; 3];
    fn open_domain_memory(&mut self, op: DomainOpenOp, _memory: &mut [u8], args: &[u32]) -> [u32; 3] {
        self.open_domain(op, args)
    }
}
/// fn_1fd9f8 / EU fn_1fdbb4 / KR fn_1fdc60.
pub fn fn_1fd9f8(
    m: &mut [u8],
    _a: u32,
    _b: u32,
    mgr: [u32; 2],
    out: u32,
    h: &mut impl DomainOpenHost,
) -> u32 {
    store(m, out, 0);
    store(m, out + 4, 0);
    let [mut rc, env, _] = h.call_memory(Boundary::Session, m, &mgr);
    if rc >= 0x80000000 {
        return rc;
    }
    let heap = word(m, env + 4);
    let n = h.open_domain_memory(DomainOpenOp::Allocate, m, &[heap, 1, 72])[0];
    if n == 0 {
        return 0x88e0200a;
    }
    store(m, out, n);
    store(m, out + 4, 0x11d);
    store(m, n, mgr[0]);
    store(m, n + 4, mgr[1]);
    store(m, n + 8, env);
    m[n as usize + 48] = 1;
    m[n as usize + 68] = 0;
    rc = (|| {
        let reply = h.open_domain_memory(DomainOpenOp::List, m, &[heap, 0, 1]);
        store(m, n + 44, reply[1]);
        if reply[0] == 0 {
            return 0x88e0200a;
        }
        let reply = h.open_domain_memory(DomainOpenOp::Map, m, &[heap, 0]);
        store(m, n + 60, reply[1]);
        if reply[0] >= 0x80000000 {
            return reply[0];
        }
        let reply = h.open_domain_memory(DomainOpenOp::List, m, &[heap, 1, 1]);
        store(m, n + 64, reply[1]);
        if reply[0] == 0 {
            return 0x88e0200a;
        }
        let mut status = 0;
        for (label, offset, required) in
            [(1, 12, true), (0, 16, true), (5, 20, false), (4, 32, false)]
        {
            let reply = h.open_domain_memory(DomainOpenOp::Resource, m, &[word(m, env + 24), label]);
            if required {
                status = reply[0];
                if status >= 0x80000000 {
                    return status;
                }
            }
            if reply[0] < 0x80000000 {
                store(m, n + offset, word(m, reply[1] + 4));
                if !required {
                    store(m, n + offset + 4, word(m, reply[1] + 8));
                    store(m, n + offset + 8, word(m, reply[1] + 12));
                }
            }
        }
        status
    })();
    if rc >= 0x80000000 {
        fn_1fd854(m, n, 0x11d, h);
        store(m, out, 0);
        store(m, out + 4, 0);
    }
    rc
}

#[derive(Clone, Copy)]
pub enum CoopOp {
    Session,
    Lock,
    Unlock,
    Analyze,
    Configure,
    Start,
    Dispatch,
    Stop,
    ResetState,
    Samples,
    ResetHandle,
    Destroy,
    Restore,
    Convert,
}
pub trait CoopHost {
    fn coop(&mut self, op: CoopOp, args: &[u32]) -> [u32; 3];
}
fn clear_output(m: &mut [u8], count: u32, status: u32, n: u32) {
    store(m, count, n);
    m[status as usize] = 0;
    store(m, status + 4, 0);
    store(m, status + 12, 0);
}
/// fn_192928 / EU fn_1929d8 / KR fn_192a14.
pub fn fn_192928(
    m: &mut [u8],
    p: u32,
    tag: u32,
    request: u32,
    count: u32,
    status: u32,
    h: &mut impl CoopHost,
) -> u32 {
    use CoopOp::*;
    if tag != 0x684f424a || p == 0 || word(m, p) != 0x734f424a {
        return 0x80000008;
    }
    let mut rc = h.coop(Session, &[word(m, p + 8), word(m, p + 12)])[0];
    if rc >= 0x80000000 {
        return h.coop(Convert, &[rc])[0];
    }
    rc = h.coop(Lock, &[word(m, p + 36)])[0];
    if rc >= 0x80000000 {
        return h.coop(Convert, &[rc])[0];
    }
    let old = m[p as usize + 40];
    if old == 1 {
        m[p as usize + 40] = 4;
    } else if !matches!(old, 4 | 5 | 6) || request != 0 {
        rc = 0x80002011;
    }
    let unlock = h.coop(Unlock, &[word(m, p + 36)])[0];
    if unlock >= 0x80000000 && rc < 0x80000000 {
        rc = unlock;
    }
    if rc >= 0x80000000 {
        return h.coop(Convert, &[rc])[0];
    }
    let mut flags = 0u16;
    let mut check = true;
    if old == 1 {
        clear_output(m, count, status, 1);
        rc = 0x80002007;
        if request != 0 && word(m, request + 8) != 0 && word(m, request + 4) != 0 {
            if m[request as usize] != 0 {
                rc = 0x80002300;
            } else {
                let reply = h.coop(Analyze, &[p, request]);
                rc = reply[0];
                store(m, p + 0x154, reply[1]);
                store(m, p + 0x158, reply[2]);
                if rc < 0x80000000 {
                    let configure = m[p as usize + 0x148];
                    let mut start = true;
                    if configure != 0 {
                        m[p as usize + 0x148] = 0;
                        rc = h.coop(Configure, &[p, 1, p + 0x36])[0];
                        if rc >= 0x80000000 {
                            m[p as usize + 0x148] = configure;
                            start = false;
                        }
                    } else if m[p as usize + 0x34] & 12 == 12 {
                        start = false;
                        check = false;
                    }
                    if start {
                        rc = h.coop(Start, &[word(m, p + 44), word(m, p + 48), p + 0xc8, 0])[0];
                    }
                }
            }
        }
    } else if word(m, status + 8) != 0 && word(m, status + 4) != 0 && old != 6 {
        store(m, p + 0x160, status);
        store(m, p + 0x15c, count);
        store(m, p + 0x164, word(m, status + 4));
        store(m, p + 0x168, word(m, status + 12));
        h.coop(
            Samples,
            &[word(m, word(m, p + 16) + 20), word(m, status + 4) >> 1],
        );
        clear_output(m, count, status, 0x8000);
        let empty = h.coop(ResetHandle, &[]);
        let reply = h.coop(
            Dispatch,
            &[
                word(m, p + 44),
                word(m, p + 48),
                word(m, p + 0x154),
                word(m, p + 0x158),
                empty[1],
                empty[2],
            ],
        );
        rc = reply[0];
        flags = reply[1] as u16;
        if rc < 0x80000000 {
            check = false;
        } else {
            store(m, status + 4, 0);
            store(m, status + 12, 0);
            if rc & 0x1fff == 0x807 {
                rc = h.coop(ResetState, &[p])[0];
            }
        }
    } else {
        clear_output(m, count, status, 0x20);
        if old != 6 {
            rc = h.coop(Stop, &[word(m, p + 44), word(m, p + 48)])[0];
            if rc < 0x80000000 {
                rc = h.coop(ResetState, &[p])[0];
            }
        } else {
            store(m, p + 0x15c, count);
            store(m, p + 0x160, status);
            store(m, p + 0x164, 0);
            store(m, p + 0x168, 0);
            let empty = h.coop(ResetHandle, &[]);
            let reply = h.coop(
                Dispatch,
                &[
                    word(m, p + 44),
                    word(m, p + 48),
                    word(m, p + 0x154),
                    word(m, p + 0x158),
                    empty[1],
                    empty[2],
                ],
            );
            rc = reply[0];
            flags = reply[1] as u16;
            if rc & 0x1fff == 0x807 {
                rc = h.coop(ResetState, &[p])[0];
            } else if rc < 0x80000000 {
                check = false;
            }
        }
    }
    if check && rc >= 0x80000000 {
        flags = 2;
    }
    if flags & 2 != 0 {
        store(m, count, 2);
        h.coop(Destroy, &[word(m, p + 0x154), word(m, p + 0x158)]);
        let empty = h.coop(ResetHandle, &[]);
        store(m, p + 0x154, empty[1]);
        store(m, p + 0x158, empty[2]);
        if m[p as usize + 0x34] & 12 != 12 {
            h.coop(Start, &[word(m, p + 44), word(m, p + 48), 0, 0]);
        }
        if rc < 0x80000000 && old == 6 {
            rc = 0x80002807;
        }
        h.coop(Restore, &[p, 1]);
    }
    h.coop(Convert, &[rc])[0]
}

#[derive(Clone, Copy)]
pub enum StartingOp {
    Reset,
    Guard,
    Convert,
    Restore,
    Get,
    Set,
    Override,
    GetType,
    SetType,
    Common,
    ReleaseType,
    Apply,
    Analyze,
    Process,
}
pub trait StartingHost {
    fn starting(&mut self, op: StartingOp, args: &[u32], text: &[u8]) -> [u32; 3];
}
fn zbytes(m: &[u8], p: u32) -> &[u8] {
    let start = p as usize;
    let n = m[start..].iter().position(|&v| v == 0).unwrap();
    &m[start..start + n]
}
/// fn_192330 / EU fn_1923e0 / KR fn_19241c.
pub fn fn_192330(
    m: &mut [u8],
    p: u32,
    tag: u32,
    request: u32,
    records: u32,
    index: u32,
    h: &mut impl StartingHost,
) -> u32 {
    use StartingOp::*;
    h.starting(Reset, &[], &[]);
    if tag != 0x684f424a || p == 0 || word(m, p) != 0x734f424a {
        return 0x80000008;
    }
    let guard = h.starting(Guard, &[p, 3], &[]);
    let locked = guard[1] as u8;
    if guard[0] >= 0x80000000 {
        return h.starting(Convert, &[guard[0]], &[])[0];
    }
    let parameters = word(m, word(m, p + 16) + 20);
    let mut enable = 0;
    let rc = (|| {
        if request == 0 || word(m, request + 8) == 0 || word(m, request + 4) == 0 {
            return 0x80002007;
        }
        if m[request as usize] != 0 {
            return 0x80002300;
        }
        if word(m, p + 0x170) == 0 {
            h.starting(Restore, &[p, locked as u32], &[]);
            return 0x80000880;
        }
        let reply = h.starting(Get, &[parameters, 0], &[]);
        enable = reply[1];
        if reply[0] >= 0x80000000 {
            return reply[0];
        }
        let rc = h.starting(Set, &[parameters, 0, 0], &[])[0];
        if rc >= 0x80000000 {
            return rc;
        }
        let record = records.wrapping_add(index.wrapping_mul(0x2c));
        if index > word(m, request + 4) || (word(m, record + 4) as i32) < 0 {
            return rc;
        }
        m[p as usize + 0x190] = m[request as usize];
        store(
            m,
            p + 0x194,
            word(m, request + 4).wrapping_sub(word(m, record + 4)),
        );
        store(
            m,
            p + 0x198,
            word(m, request + 8).wrapping_add(word(m, record + 4) & !1),
        );
        let detail = word(m, record + 8);
        let mut old = [u32::MAX; 5];
        for (i, v) in old.iter_mut().enumerate() {
            *v = h.starting(Get, &[parameters, i as u32 + 1], &[])[1];
        }
        let char_type = h.starting(GetType, &[parameters], &[])[1];
        let old_text = zbytes(m, char_type);
        let mut saved = [0u8; 64];
        saved[..old_text.len()].copy_from_slice(old_text);
        let saved_len = old_text.len();
        h.starting(SetType, &[parameters], b"text/plain;charset=utf-16");
        let common = 0u32;
        h.starting(Common, &[parameters, common], &common.to_le_bytes());
        if detail != 0
            && h.starting(Apply, &[word(m, p + 0x2c), word(m, p + 0x30), detail], &[])[0] == 1
        {
            let name = zbytes(m, detail + 28);
            if name == b"unknown" || name.len() == 7 {
                h.starting(Common, &[parameters, 0], b"unknown");
            } else if name.len() == 3 {
                h.starting(Common, &[parameters, 0], name);
            }
            for (i, off) in [4, 16, 8, 12, 20].into_iter().enumerate() {
                let v = word(m, detail + off);
                if v != 0x270f {
                    h.starting(Override, &[parameters, i as u32 + 1, v], &[]);
                }
            }
            let bias = word(m, detail + 24);
            if bias != 0 {
                h.starting(Override, &[parameters, 6, bias], &[]);
            }
        }
        let reply = h.starting(Analyze, &[p, word(m, p + 0x194), word(m, p + 0x198)], &[]);
        let mut rc = reply[0];
        if rc < 0x80000000 {
            rc = h.starting(Process, &[p, 0, reply[1], reply[2]], &[])[0];
        }
        h.starting(Common, &[parameters, common], &common.to_le_bytes());
        for (i, &v) in old.iter().enumerate() {
            h.starting(Set, &[parameters, i as u32 + 1, v], &[]);
        }
        h.starting(SetType, &[parameters], &saved[..saved_len]);
        h.starting(ReleaseType, &[parameters, char_type], &[]);
        rc
    })();
    // The inactive fast return bypasses all restoration and conversion.
    if rc == 0x80000880
        && request != 0
        && word(m, request + 8) != 0
        && word(m, request + 4) != 0
        && m[request as usize] == 0
        && word(m, p + 0x170) == 0
    {
        return rc;
    }
    h.starting(Set, &[parameters, 0, enable], &[]);
    if locked != 0 {
        h.starting(Restore, &[p, locked as u32], &[]);
    }
    h.starting(Convert, &[rc], &[])[0]
}

#[derive(Clone, Copy, Debug)]
pub enum PromptOp {
    Validate,
    Session,
    Clear,
    Allocate,
    Free,
    Enable,
    Count,
    Pop,
    CreateNode,
    UpdateNode,
    Consult,
    Write,
    DestroyNodes,
    Build,
    Insert,
    Iter,
    Next,
    Close,
    Mark,
    Marked,
    Slot,
    Expand,
}
/// External domain services. Addresses are offsets into caller-owned memory.
/// Local prompt descriptors remain in the fixed nine-word buffer.
pub trait PromptHost {
    fn prompt(
        &mut self,
        op: PromptOp,
        mem: &mut [u8],
        args: &[u32],
        descriptor: &mut [u32; 9],
    ) -> [u32; 3];
}
fn byte_length(m: &[u8], p: u32) -> u32 {
    m[p as usize..].iter().position(|&b| b == 0).unwrap() as u32
}
fn wide_length(m: &[u8], p: u32) -> u32 {
    let mut n = 0;
    while half(m, p.wrapping_add(2 * n)) != 0 {
        n += 1;
    }
    n
}
fn normal_name(m: &[u8], p: u32) -> bool {
    b"normal\0"
        .iter()
        .enumerate()
        .all(|(i, &b)| half(m, p + 2 * i as u32) == b as u16)
}
/// fn_200390 / EU fn_20054c / KR fn_2005f8.
/// Apply explicit prompt insertions, then traverse default domains in reverse.
pub fn fn_200390(
    m: &mut [u8],
    manager: u32,
    text: u32,
    entries: u32,
    count: u32,
    head: u32,
    context: [u32; 2],
    offset: u32,
    h: &mut impl PromptHost,
) -> u32 {
    use PromptOp::*;
    let heap = word(m, word(m, manager + 8) + 4);
    let start = word(m, word(m, entries) + 12);
    let mut descriptor = [0; 9];
    let mut rc = 0;
    let mut name = 0;
    let mut node = head;
    let mut index = 0u16;
    while index < half(m, count) {
        let e = word(m, entries) + 24 * index as u32;
        if half(m, e) == 40 {
            name = h.prompt(
                Allocate,
                m,
                &[heap, wide_length(m, word(m, e + 20)).wrapping_add(1)],
                &mut descriptor,
            )[0];
            if name == 0 {
                rc = 0x88e0200a;
                break;
            }
            narrow_string(m, word(m, e + 20), name);
            rc = h.prompt(Build, m, &[manager, name, 0], &mut descriptor)[0];
            if rc >= 0x80000000 {
                break;
            }
            h.prompt(Free, m, &[heap, name], &mut descriptor);
            name = 0;
            loop {
                if node == 0 {
                    break;
                }
                let position = word(m, node + 4);
                let end = position.wrapping_add(word(m, node + 8));
                let at = word(m, e + 12);
                if (position <= at && at < end)
                    || (at == end && byte_length(m, word(m, text)).wrapping_add(start) == at)
                {
                    break;
                }
                node = word(m, node + 24);
            }
            if node == 0 {
                rc = 0x88e02000;
                break;
            }
            rc = h.prompt(
                Insert,
                m,
                &[
                    0,
                    word(m, e + 12).wrapping_sub(start) as u16 as u32,
                    0,
                    start,
                    text,
                    entries,
                    count,
                    index as u32,
                    node,
                    context[0],
                    context[1],
                    offset,
                ],
                &mut descriptor,
            )[0];
            if rc >= 0x80000000 {
                break;
            }
        }
        index = index.wrapping_add(1);
    }
    if rc < 0x80000000 && m[(manager + 48) as usize] != 0 {
        let reply = h.prompt(Iter, m, &[word(m, manager + 60), 0, 0, 0], &mut descriptor);
        if reply[0] < 0x80000000 {
            loop {
                let next = h.prompt(Next, m, &[reply[1], 0, 0], &mut descriptor);
                if next[0] >= 0x80000000 {
                    break;
                }
                h.prompt(Mark, m, &[next[2], 0], &mut descriptor);
            }
            h.prompt(Close, m, &[reply[1]], &mut descriptor);
        }
        let domains = word(m, manager + 64);
        let mut i = h.prompt(Count, m, &[domains], &mut descriptor)[0];
        'domains: while i != 0 {
            let slot = h.prompt(Slot, m, &[domains, i - 1, 0], &mut descriptor);
            if slot[0] == 0 {
                break;
            }
            let domain = word(m, slot[1]);
            if h.prompt(Marked, m, &[domain], &mut descriptor)[0] == 0 {
                let prompts = word(m, domain + 8);
                let mut j = h.prompt(Count, m, &[prompts], &mut descriptor)[0];
                while j != 0 {
                    let item = h.prompt(Slot, m, &[prompts, j - 1, 0], &mut descriptor);
                    if item[0] == 0 {
                        break;
                    }
                    rc = h.prompt(
                        Expand,
                        m,
                        &[
                            word(m, item[1]),
                            text,
                            entries,
                            count,
                            context[0],
                            context[1],
                            head,
                        ],
                        &mut descriptor,
                    )[0];
                    if rc >= 0x80000000 {
                        break 'domains;
                    }
                    j -= 1;
                }
                h.prompt(Mark, m, &[domain, 1], &mut descriptor);
            }
            i -= 1;
        }
    }
    if name != 0 {
        h.prompt(Free, m, &[heap, name], &mut descriptor);
    }
    rc
}
/// fn_1fef0c / EU fn_1ff0c8 / KR fn_1ff174.
/// Split normalized text at domain boundaries and write completed prompt nodes.
pub fn fn_1fef0c(
    m: &mut [u8],
    manager: u32,
    tag: u32,
    text: u32,
    entries: u32,
    count: u32,
    context: [u32; 2],
    text_output: [u32; 2],
    item_output: [u32; 2],
    offset: u32,
    h: &mut impl PromptHost,
) -> u32 {
    use PromptOp::*;
    let mut descriptor = [0; 9];
    let mut rc = h.prompt(Validate, m, &[manager, tag, 0x11d, 0x48], &mut descriptor)[0];
    if rc >= 0x80000000 {
        return rc;
    }
    let session = h.prompt(
        Session,
        m,
        &[word(m, manager), word(m, manager + 4), 0],
        &mut descriptor,
    );
    rc = session[0];
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = session[1];
    let heap = word(m, ctx + 4);
    h.prompt(Clear, m, &[word(m, manager + 44)], &mut descriptor);
    let mut i = 0u16;
    // The ROM compares against the unsigned count-minus-one.
    while (i as u32) < (half(m, count) as u32).wrapping_sub(1) {
        let e = word(m, entries) + i as u32 * 24;
        if word(m, e + 12) > word(m, e + 36) {
            return 0x88e02007;
        }
        i = i.wrapping_add(1);
    }
    let start = word(m, word(m, entries) + 12).wrapping_add(half(m, offset) as i16 as i32 as u32);
    let head = h.prompt(
        CreateNode,
        m,
        &[heap, 1, start, byte_length(m, word(m, text)), 0, 0, 0],
        &mut descriptor,
    )[0];
    if head == 0 {
        return 0x88e0200a;
    }
    let mut node = head;
    rc = (|| {
        let mut i = 0u16;
        while i < half(m, count) {
            let e = word(m, entries) + 24 * i as u32;
            i = i.wrapping_add(1);
            let kind = half(m, e);
            if kind == 12 {
                let value = word(m, e + 20);
                if value != 0
                    && half(m, value) != 0
                    && !normal_name(m, value)
                    && m[(manager + 68) as usize] == 0
                {
                    let name = h.prompt(
                        Allocate,
                        m,
                        &[heap, wide_length(m, value).wrapping_add(1)],
                        &mut descriptor,
                    )[0];
                    if name == 0 {
                        return 0x88e0200a;
                    }
                    narrow_string(m, value, name);
                    h.prompt(Enable, m, &[manager, tag, name], &mut descriptor);
                    h.prompt(Free, m, &[heap, name], &mut descriptor);
                } else if m[(manager + 68) as usize] == 0 {
                    let list = word(m, manager + 64);
                    if h.prompt(Count, m, &[list], &mut descriptor)[0] > word(m, manager + 52) {
                        h.prompt(Pop, m, &[list], &mut descriptor);
                    }
                }
                continue;
            }
            if matches!(kind, 29 | 8 | 26) && word(m, e + 12) > word(m, node + 4) {
                let length = word(m, node + 8);
                let amount = word(m, e + 12).wrapping_sub(word(m, node + 4));
                if length.wrapping_sub(amount) as i16 > 0 {
                    let r = h.prompt(
                        UpdateNode,
                        m,
                        &[
                            heap,
                            node,
                            m[node as usize] as u32,
                            word(m, node + 4),
                            amount,
                            0,
                            0,
                            0,
                        ],
                        &mut descriptor,
                    )[0];
                    if r >= 0x80000000 {
                        return r;
                    }
                    let next = h.prompt(
                        CreateNode,
                        m,
                        &[
                            heap,
                            m[node as usize] as u32,
                            word(m, e + 12),
                            length.wrapping_sub(word(m, node + 8)),
                            0,
                            0,
                            0,
                        ],
                        &mut descriptor,
                    )[0];
                    if next == 0 {
                        return 0x88e0200a;
                    }
                    store(m, node + 24, next);
                    node = next;
                }
            }
        }
        if byte_length(m, word(m, text)) != 0 {
            rc = h.prompt(
                Consult,
                m,
                &[
                    manager, text, entries, count, head, context[0], context[1], offset,
                ],
                &mut descriptor,
            )[0];
            if rc >= 0x80000000 {
                return rc;
            }
        }
        let mut wrote = false;
        node = head;
        while node != 0 {
            if word(m, node + 12) != 0 && word(m, node + 20) != 0 {
                wrote = true;
                rc = h.prompt(
                    Write,
                    m,
                    &[
                        item_output[0],
                        item_output[1],
                        word(m, node + 20),
                        half(m, node + 16) as u32 * 4,
                        0,
                    ],
                    &mut descriptor,
                )[0];
                if rc >= 0x80000000 {
                    return rc;
                }
                rc = h.prompt(
                    Write,
                    m,
                    &[
                        text_output[0],
                        text_output[1],
                        word(m, node + 12),
                        byte_length(m, word(m, node + 12)),
                        0,
                    ],
                    &mut descriptor,
                )[0];
                if rc >= 0x80000000 {
                    return rc;
                }
                rc = h.prompt(
                    Write,
                    m,
                    &[text_output[0], text_output[1], 0, 1, 27],
                    &mut descriptor,
                )[0];
                if rc >= 0x80000000 {
                    return rc;
                }
            }
            node = word(m, node + 24);
        }
        if wrote {
            rc = h.prompt(
                Write,
                m,
                &[text_output[0], text_output[1], 0, 1, 0],
                &mut descriptor,
            )[0];
        }
        rc
    })();
    if ctx != 0 {
        h.prompt(DestroyNodes, m, &[head, heap], &mut descriptor);
    }
    rc
}

#[derive(Clone, Copy, Debug)]
pub enum WorkerOp {
    Allocate,
    Release,
    Engine,
    Resource,
    Provider,
    OpenEngine,
    Context,
    OpenStream,
    Stream,
    Linguistic,
    Parameters,
    Logger,
    Log,
    Callback,
    CallbackId,
    BindLog,
    VoiceApi,
    Empty,
    Attach,
    CallbackApi,
    Register,
    SynthStream,
    LingDb,
    Destroy,
    CloseProvider,
    CloseResource,
    CloseEngine,
    Convert,
}
/// Worker construction services. Output words use the caller's address space.
pub trait WorkerHost {
    fn worker(&mut self, op: WorkerOp, mem: &mut [u8], args: &[u32]) -> [u32; 3];
}
/// fn_190984 / EU fn_190a34 / KR fn_190a70.
/// Construct a worker; clean up each successfully acquired resource on failure.
pub fn fn_190984(m: &mut [u8], request: u32, out: u32, h: &mut impl WorkerHost) -> u32 {
    use WorkerOp::*;
    if request == 0 || out == 0 {
        return 0x80000006;
    }
    if half(m, request) != 0x520 || word(m, request + 8) == 0 || word(m, request + 24) == 0 {
        return h.worker(Convert, m, &[0x80002300])[0];
    }
    store(m, out, 0);
    store(m, out + 4, 0);
    let allocator = word(m, request + 8);
    let user = word(m, request + 12);
    let w = h.worker(Allocate, m, &[user, 1, 0xe0])[0];
    let mut engine = 0;
    let mut resource = 0;
    let mut provider = 0;
    let mut status = 0x8000000a;
    if w != 0 {
        store(m, w, 0x73434c53);
        for i in 0..4 {
            store(m, w + 4 + 4 * i, word(m, allocator + 4 * i));
        }
        for (src, dst, n) in [
            (word(m, request + 16), w + 0x24, 4),
            (word(m, request + 24), w + 0x34, 7),
            (word(m, request + 28), w + 0x5c, 5),
        ] {
            if src != 0 {
                for i in 0..n {
                    if dst != w + 0x34 || i != 5 {
                        store(m, dst + 4 * i, word(m, src + 4 * i));
                    }
                }
            }
        }
        let callback = word(m, request + 36);
        if callback != 0 {
            store(m, w + 0x9c, word(m, callback));
            store(m, w + 0xa0, word(m, callback + 4));
            store(m, w + 0xa4, word(m, request + 40));
        }
        store(m, w + 0x20, user);
        status = (|| {
            let reply = h.worker(Engine, m, &[w + 4, user, 0]);
            engine = reply[1];
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            let reply = h.worker(
                Resource,
                m,
                &[engine, user, w + 0x24, word(m, request + 20), 0],
            );
            resource = reply[1];
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            let reply = h.worker(
                Provider,
                m,
                &[engine, user, w + 0x34, word(m, request + 32), 0],
            );
            provider = reply[1];
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            let reply = h.worker(OpenEngine, m, &[engine, resource, 0, 0, w + 0xa8]);
            store(m, w + 0xa8, reply[1]);
            store(m, w + 0xac, reply[2]);
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            let reply = h.worker(
                Context,
                m,
                &[word(m, w + 0xa8), word(m, w + 0xac), w + 0xb0],
            );
            store(m, w + 0xb0, reply[1]);
            let ctx = reply[1];
            store(m, ctx + 28, provider);
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            let identifier = word(m, request + 4);
            if identifier == 0 {
                return 0x80000012;
            }
            let reply = h.worker(
                OpenStream,
                m,
                &[
                    word(m, w + 0xa8),
                    word(m, w + 0xac),
                    3,
                    identifier,
                    0,
                    w + 0xb8,
                ],
            );
            store(m, w + 0xb8, reply[1]);
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            for (op, args, dst) in [
                (Stream, [0, word(m, w + 0xb8), ctx + 12], ctx + 12),
                (
                    Linguistic,
                    [0, word(m, w + 0xa8), word(m, w + 0xac)],
                    ctx + 24,
                ),
                (
                    Parameters,
                    [0, word(m, w + 0xa8), word(m, w + 0xac)],
                    ctx + 20,
                ),
                (Logger, [word(m, w + 0xa8), word(m, w + 0xac), 0], w + 0xb4),
            ] {
                let reply = if matches!(op, Linguistic | Parameters) {
                    h.worker(op, m, &[args[0], args[1], args[2], dst])
                } else if matches!(op, Logger) {
                    h.worker(op, m, &[args[0], args[1], 0, dst])
                } else {
                    h.worker(op, m, &args)
                };
                store(m, dst, reply[1]);
                if reply[0] >= 0x80000000 {
                    return reply[0];
                }
            }
            let reply = h.worker(Log, m, &[word(m, w + 0xb4), ctx + 16]);
            store(m, ctx + 16, reply[1]);
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            if word(m, w + 0x9c) != 0 || word(m, w + 0xa0) != 0 {
                h.worker(Callback, m, &[word(m, ctx + 16), w + 0x9c]);
            }
            let reply = h.worker(CallbackId, m, &[1, 0]);
            if reply[0] < 0x80000000 {
                h.worker(BindLog, m, &[engine, reply[1], word(m, ctx + 16)]);
            }
            let reply = h.worker(VoiceApi, m, &[1, w + 0xc8]);
            store(m, w + 0xc8, reply[1]);
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            let empty = h.worker(Empty, m, &[0]);
            let reply = h.worker(
                Attach,
                m,
                &[
                    empty[1],
                    empty[2],
                    word(m, w + 0xa8),
                    word(m, w + 0xac),
                    w + 0xcc,
                ],
            );
            store(m, w + 0xcc, reply[1]);
            store(m, w + 0xd0, reply[2]);
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            let reply = h.worker(CallbackApi, m, &[3, w + 0xbc]);
            store(m, w + 0xbc, reply[1]);
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            let reply = h.worker(
                Register,
                m,
                &[word(m, w + 0xa8), word(m, w + 0xac), w + 0xc0],
            );
            store(m, w + 0xc0, reply[1]);
            store(m, w + 0xc4, reply[2]);
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            let reply = h.worker(SynthStream, m, &[1, w + 0xd4]);
            store(m, w + 0xd4, reply[1]);
            if reply[0] >= 0x80000000 {
                return reply[0];
            }
            let reply = h.worker(LingDb, m, &[1, w + 0xd8]);
            store(m, w + 0xd8, reply[1]);
            reply[0]
        })();
        if status < 0x80000000 {
            store(m, out, w);
            store(m, out + 4, 0x68434c53);
            return h.worker(Convert, m, &[status])[0];
        }
        if word(m, w + 0xb0) != 0 {
            h.worker(Destroy, m, &[w, 0x68434c53]);
            return h.worker(Convert, m, &[status])[0];
        }
    }
    if provider != 0 {
        h.worker(CloseProvider, m, &[provider]);
    }
    if resource != 0 {
        h.worker(CloseResource, m, &[resource]);
    }
    if engine != 0 {
        h.worker(CloseEngine, m, &[engine]);
    }
    if w != 0 {
        h.worker(Release, m, &[user, w]);
    }
    h.worker(Convert, m, &[status])[0]
}

#[derive(Clone, Copy, Debug)]
pub enum LoadOp {
    Validate,
    Session,
    Acquire,
    Release,
    Open,
    Find,
    Init,
    Add,
    Append,
    Enable,
    Notify,
    Destroy,
    Remove,
    DestroyList,
}
pub trait LoadHost {
    fn load(&mut self, op: LoadOp, mem: &mut [u8], args: &[u32], list: &mut [u32; 4]) -> [u32; 5];
}
fn contains_bytes(m: &[u8], p: u32, needle: &[u8]) -> bool {
    let bytes = &m[p as usize..(p + byte_length(m, p)) as usize];
    bytes.windows(needle.len()).any(|v| v == needle)
}
/// fn_1ffae8 / EU fn_1ffca4 / KR fn_1ffd50.
/// Load an APDB resource, retain it in the domain map, and unwind failures.
/// The floating-point broker argument is carried as bits, including sNaNs.
pub fn fn_1ffae8(
    m: &mut [u8],
    p: u32,
    tag: u32,
    mime: u32,
    name: u32,
    f: u32,
    g: u32,
    mode: u32,
    float_bits: u32,
    output: u32,
    h: &mut impl LoadHost,
) -> u32 {
    use LoadOp::*;
    let mut list = [0; 4];
    if mime == 0
        || output == 0
        || (name == 0 && (f == 0 || g == 0))
        || (name != 0 && f == 0 && g != 0)
    {
        return 0x88e02007;
    }
    let mut rc = h.load(Validate, m, &[p, tag, 0x11d, 0x48], &mut list)[0];
    if rc >= 0x80000000 {
        return rc;
    }
    rc = h.load(Session, m, &[word(m, p), word(m, p + 4), 0], &mut list)[0];
    if rc >= 0x80000000 {
        return rc;
    }
    store(m, output, 0);
    store(m, output + 4, 0);
    let mut release = 0;
    let mut x = f;
    let mut y = g;
    let mut aux = 0;
    let mut owned = true;
    rc = (|| {
        if f == 0 || g == 0 {
            x = 0;
            y = 0;
            if !contains_bytes(m, mime, b";loader=broker") {
                if word(m, p + 20) == 0 {
                    return 0x88e02001;
                }
                let reply = h.load(
                    Acquire,
                    m,
                    &[
                        word(m, p + 24),
                        word(m, p + 28),
                        name,
                        mode,
                        float_bits,
                        1,
                        0,
                        0,
                        0,
                        0,
                        0,
                    ],
                    &mut list,
                );
                release = reply[1];
                x = reply[3];
                y = reply[4];
                if reply[0] >= 0x80000000 {
                    return reply[0];
                }
            }
        }
        let reply = h.load(Open, m, &[0, p, mime, name, x, y, release, 0], &mut list);
        aux = reply[1];
        if reply[0] >= 0x80000000 {
            return reply[0];
        }
        let key = word(m, aux + 88);
        release = 0;
        let found = h.load(Find, m, &[word(m, p + 60), key, 0], &mut list);
        let mut entry = found[1];
        if found[0] >= 0x80000000 {
            let result = h.load(Init, m, &[0, word(m, p), word(m, p + 4), 0], &mut list)[0];
            if result >= 0x80000000 {
                return result;
            }
            let result = h.load(Add, m, &[word(m, p + 60), key, 0], &mut list)[0];
            if result >= 0x80000000 {
                h.load(DestroyList, m, &[0], &mut list);
                return result;
            }
            let found = h.load(Find, m, &[word(m, p + 60), key, 0], &mut list);
            entry = found[1];
            if found[0] >= 0x80000000 {
                h.load(DestroyList, m, &[0], &mut list);
                return found[0];
            }
        }
        let result = h.load(Append, m, &[entry, aux], &mut list)[0];
        if result >= 0x80000000 {
            return result;
        }
        owned = false;
        if contains_bytes(m, mime, b";mode=automatic")
            || (byte_length(m, aux + 24) == 9
                && &m[(aux + 24) as usize..(aux + 33) as usize] == b"automatic")
        {
            let result = h.load(Enable, m, &[p, tag, key], &mut list)[0];
            if result >= 0x80000000 {
                return result;
            }
            store(m, p + 52, word(m, p + 52).wrapping_add(1));
            rc = result;
        } else {
            rc = result;
        }
        h.load(Notify, m, &[p], &mut list);
        store(m, output, aux);
        store(m, output + 4, 0x13c);
        rc
    })();
    if rc >= 0x80000000 && aux != 0 {
        if owned {
            h.load(Destroy, m, &[], &mut list);
        } else {
            store(m, output, aux);
            store(m, output + 4, 0x13c);
            h.load(Remove, m, &[p, tag, aux, 0x13c], &mut list);
            store(m, output, 0);
            store(m, output + 4, 0);
        }
    }
    if release != 0 {
        h.load(Release, m, &[word(m, p + 24), word(m, p + 28)], &mut list);
    }
    rc
}
