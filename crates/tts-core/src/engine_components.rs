

use crate::engine::{Error, Memory};
use crate::rom_region::{Rom, addr, to_us};

pub trait InternalCalls {
    fn call(&mut self, memory: &mut Memory<'_, '_>, entry: u32, args: &[u32])
    -> Result<u32, Error>;

    fn rom(&self) -> Rom {
        Rom::Us
    }
}

pub struct CoreCalls;
impl InternalCalls for CoreCalls {
    fn call(&mut self, m: &mut Memory<'_, '_>, entry: u32, a: &[u32]) -> Result<u32, Error> {
        RegionalCalls {
            rom: self.rom().or(Rom::from_entry(entry)),
        }
        .call(m, entry, a)
    }
}

pub struct RomHost<'a> {
    pub inner: &'a mut dyn InternalCalls,
    pub rom: Rom,
}
impl InternalCalls for RomHost<'_> {
    fn call(
        &mut self,
        m: &mut Memory<'_, '_>,
        entry: u32,
        a: &[u32],
    ) -> Result<u32, Error> {
        self.inner.call(m, entry, a)
    }
    fn rom(&self) -> Rom {
        self.rom
    }
}

pub struct RegionalCalls {
    pub rom: Rom,
}
impl InternalCalls for RegionalCalls {
    fn rom(&self) -> Rom {
        self.rom
    }
    fn call(&mut self, m: &mut Memory<'_, '_>, entry: u32, a: &[u32]) -> Result<u32, Error> {
        let entry = to_us(self.rom, entry);
        match entry {
            0x19611c => {
                let (rc, context) = crate::engine::resolve_environment(m, [a[0], a[1]])?;
                m.put_word(a[2], context)?;
                Ok(rc)
            }
            0x15140c => remaining_binding(m, a, self.rom),
            0x15c544 => child_binding(m, a, self.rom),
            0x1738e8 => log_binding(m, a),
            0x173dc0 => {
                if a[0] == 0 {
                    Ok(0)
                } else if a[1] == 0 {
                    Ok(0x84b0_2006)
                } else {
                    Err(Error::UnsupportedStage { entry: 0x173c70 })
                }
            }
            0x17486c => {
                if a[0] == 0 || byte(m, a[0] + 64)? == 0 {
                    Ok(0)
                } else {
                    Err(Error::UnsupportedStage { entry: 0x1741dc })
                }
            }
            _ => Err(Error::UnsupportedStage { entry }),
        }
    }
}

fn log_binding(m: &Memory<'_, '_>, a: &[u32]) -> Result<u32, Error> {
    let p = a[0];
    let mut arena = [0u8; 128];
    let local = if p == 0 { 0 } else { 64 };
    if p != 0 {
        arena[64 + 0x30] = byte(m, p + 0x30)?;
        if a[2] <= arena[64 + 0x30] as u32 {
            arena[64 + 0x31] = byte(m, p + 0x31)?;
            if (arena[64 + 0x31] == 0 || a[2] <= 4) && a[4] != 0 && a[1] != 0 {
                for i in 0..6 {

                    let slot = if i == 5 { 0x38 } else { 12 + i * 8 };
                    let methods = m.word(p + slot)?;
                    if methods != 0 {
                        let method = m.word(methods + 24)?;
                        if method != 0 && (i != 1 || a[2] <= 1) {
                            return Err(Error::UnsupportedStage { entry: method });
                        }
                    }
                }
            }
        }
    }
    Ok(crate::normalization::log_fanout(
        &mut arena, local, a[1], a[2], a[3], a[4],
    ))
}

fn remaining_binding(m: &mut Memory<'_, '_>, a: &[u32], rom: Rom) -> Result<u32, Error> {
    let validation = crate::normalization::quad_check(a[0], a[1], 0x47de, 12);
    if bad(validation) {
        return Ok(validation);
    }
    let state = m.word(a[0] + 4)?;
    let logger = m.word(m.word(state + 4)? + 16)?;
    log_binding(m, &[logger, addr(rom, 0x8d7764), 4, 0, 1])?;
    let kind = byte(m, a[0])?;

    if (a[2] == 0 || kind != 0) && logger != 0 {
        return Err(Error::UnsupportedStage { entry: 0x173c70 });
    }
    let position = crate::frontend::StreamPosition {
        kind,
        read: if kind == 0 && a[2] != 0 {
            m.word(a[0] + 8)?
        } else {
            0
        },
        written: if kind == 0 && a[2] != 0 {
            m.word(state + 44)?
        } else {
            0
        },
        eof: 0,
    };
    let mut output = 0;
    let mut finished = 0;
    let rc = position.remaining(
        validation,
        if a[2] != 0 { Some(&mut output) } else { None },
        if a[3] != 0 { Some(&mut finished) } else { None },
    );
    if a[2] != 0 {
        m.put_word(a[2], output)?;
    }
    if a[3] != 0 {

        if kind == 0 && a[2] != 0 {
            finished = byte(m, state + 68)?;
        }
        m.write(a[3], &[finished])?;
    }
    log_binding(m, &[logger, addr(rom, 0x8d7764), 4, 0, 1])?;
    Ok(rc)
}

fn child_binding(m: &mut Memory<'_, '_>, a: &[u32], rom: Rom) -> Result<u32, Error> {
    use crate::linguistic::{Database, Record};
    let p = a[0];
    let valid = !bad(crate::normalization::quad_check(p, a[1], 0xb2c2, 0x38));
    let mut records = [None; 3];
    let mut parents = [0u8; 256];
    let mut allocated = false;
    if valid {
        let table = m.word(p + 48)?;
        allocated = table != 0;
        if a[2] != 1 {
            if a[2] >= parents.len() as u32 {
                return Err(Error::UnsupportedStage { entry: 0x15c544 });
            }
            let count = m.word(p + 32)?;
            let parent = if a[3] < count {
                m.word(table + a[3] * 4)?
            } else {
                0
            };
            if parent != 0 {
                let mut record = Record {
                    id: half(m, parent)?,
                    kind: byte(m, parent + 2)?,
                    ..Record::default()
                };
                parents[a[2] as usize] = byte(m, addr(rom, 0x8d7b50) + a[2])?;
                if record.kind == parents[a[2] as usize] {
                    for i in 0..2 {
                        let child = m.word(parent + 16 + i as u32 * 4)?;
                        if child == 0 {
                            break;
                        }
                        let kind = byte(m, child + 2)?;
                        let id = if kind as u32 == a[2] {
                            half(m, child)?
                        } else {
                            0
                        };
                        records[i + 1] = Some(Record {
                            id,
                            kind,
                            ..Record::default()
                        });
                        record.children[i] = Some(i + 1);
                        if kind as u32 == a[2] && id != 0 {
                            break;
                        }
                    }
                }
                records[0] = Some(record);
            }
        }
    }
    let db = Database {
        records: &records,
        parents: &parents,
        allocated,
        valid,
    };
    let mut output = 0;
    let rc = db.find_child(a[2], 0, &mut output);
    if bad(rc) {

        if valid && m.word(m.word(p)? + 16)? != 0 {
            return Err(Error::UnsupportedStage { entry: 0x173c70 });
        }
    } else {
        put_half(m, a[4], output)?;
    }
    Ok(rc)
}

pub struct Unwired;
impl InternalCalls for Unwired {
    fn call(&mut self, _: &mut Memory<'_, '_>, entry: u32, _: &[u32]) -> Result<u32, Error> {
        Err(Error::UnsupportedStage { entry })
    }
}

fn byte(m: &Memory<'_, '_>, a: u32) -> Result<u8, Error> {
    let mut v = [0];
    m.read(a, &mut v)?;
    Ok(v[0])
}
fn half(m: &Memory<'_, '_>, a: u32) -> Result<u16, Error> {
    let mut v = [0; 2];
    m.read(a, &mut v)?;
    Ok(u16::from_le_bytes(v))
}
fn put_half(m: &mut Memory<'_, '_>, a: u32, v: u16) -> Result<(), Error> {
    m.write(a, &v.to_le_bytes())
}
fn copy(m: &mut Memory<'_, '_>, dst: u32, src: u32, n: u32) -> Result<(), Error> {

    for i in 0..n {
        let v = byte(m, src + i)?;
        m.write(dst + i, &[v])?;
    }
    Ok(())
}
fn pair(m: &Memory<'_, '_>, a: u32) -> Result<[u32; 2], Error> {
    Ok([m.word(a)?, m.word(a + 4)?])
}
fn bad(rc: u32) -> bool {
    rc >= 0x8000_0000
}
fn invoke(
    h: &mut impl InternalCalls,
    m: &mut Memory<'_, '_>,
    table: u32,
    slot: u32,
    args: &[u32],
) -> Result<u32, Error> {
    let entry = m.word(table + slot)?;
    h.call(m, entry, args)
}
fn log(
    h: &mut impl InternalCalls,
    m: &mut Memory<'_, '_>,
    logger: u32,
    module: u32,
    code: u32,
    rc: u32,
) -> Result<(), Error> {

    let rom = h.rom();
    h.call(
        m,
        0x173dc0,
        &[logger, module, code, addr(rom, 0x1e3df8), addr(rom, 0x1e3df0), rc],
    )?;
    Ok(())
}

pub(crate) fn process_1e3b14(
    m: &mut Memory<'_, '_>,
    p: u32,
    key: u32,
    active: u32,
    s: u32,
    h: &mut impl InternalCalls,
) -> Result<u32, Error> {
    let mut rc = crate::normalization::quad_check(p, key, 0x72a5, 0x2c);
    if bad(rc) {
        return Ok(0x8a70_2008);
    }
    m.write(active, &[1])?;
    let input = pair(m, p + 28)?;
    if input[0] == 0 {
        return Ok(rc);
    }
    m.write(s, &[0; 32])?;
    let streams = m.word(p + 24)?;
    let output = pair(m, p + 36)?;
    rc = invoke(h, m, streams, 72, &[input[0], input[1], s, s + 4])?;
    if bad(rc) {
        log(h, m, m.word(m.word(p + 8)? + 16)?, addr(h.rom(), 0x8f8b6e), 0xb031, rc)?;
        return Ok(rc);
    }
    if m.word(s)? == 0 {
        if byte(m, s + 4)? == 0 {
            m.write(active, &[0])?;
        } else if output[0] != 0 {
            rc = invoke(h, m, streams, 36, &output)?;
            let mut null_pair = [0; 8];
            let mut value = [0; 8];
            m.read(addr(h.rom(), 0x8d7660), &mut value)?;
            crate::frontend::load_pair(&mut null_pair, &value);
            m.write(p + 36, &null_pair)?;
        }
        return Ok(rc);
    }
    rc = invoke(h, m, streams, 44, &[input[0], input[1], s + 8, s])?;
    if bad(rc) {
        log(h, m, m.word(m.word(p + 8)? + 16)?, addr(h.rom(), 0x8f8b6e), 0xb031, rc)?;
        return Ok(rc);
    }
    rc = invoke(
        h,
        m,
        streams,
        56,
        &[output[0], output[1], s + 12, m.word(s)?],
    )?;
    if !bad(rc) {
        let count = m.word(s)? / 24;
        let src = m.word(s + 8)?;
        let dst = m.word(s + 12)?;
        for i in 0..count {
            let a = src + i * 24;
            let b = dst + i * 24;
            copy(m, b, a, 24)?;
            if half(m, a)? == 0x21 {
                put_half(m, b, 0x26)?;
                let phoneme = m.word(a + 20)?;
                m.write(s + 16, &[phoneme as u8, 0])?;
                m.put_word(s + 20, 0)?;
                let phonemes = pair(m, p + 16)?;
                rc = invoke(
                    h,
                    m,
                    m.word(p + 12)?,
                    32,
                    &[phonemes[0], phonemes[1], 0, s + 16, s + 20, 0],
                )?;
                if bad(rc) {

                    h.call(
                        m,
                        0x173dc0,
                        &[
                            m.word(m.word(p + 8)? + 16)?,
                            addr(h.rom(), 0x8f8b6e),
                            0xb033,
                            addr(h.rom(), 0x1e3e10),
                            addr(h.rom(), 0x1e3e04),
                            phoneme,
                            addr(h.rom(), 0x1e3df0),
                            rc,
                        ],
                    )?;
                    rc = 0;
                    m.put_word(s + 20, addr(h.rom(), 0x8f8b6c))?;
                }
                m.put_word(b + 20, m.word(s + 20)?)?;
            }
        }
        rc = invoke(h, m, streams, 60, &[output[0], output[1], count * 24])?;
        if !bad(rc) {
            rc = invoke(h, m, streams, 48, &[input[0], input[1], count * 24])?;
            if bad(rc) {
                log(h, m, m.word(m.word(p + 8)? + 16)?, addr(h.rom(), 0x8f8b6e), 0xb031, rc)?;
            }
            return Ok(rc);
        }
    }
    log(h, m, m.word(m.word(p + 8)? + 16)?, addr(h.rom(), 0x8f8b6e), 0xb032, rc)?;
    invoke(h, m, streams, 48, &[input[0], input[1], 0])?;
    Ok(rc)
}

fn free_buffers(m: &mut Memory<'_, '_>, p: u32, h: &mut impl InternalCalls) -> Result<u32, Error> {
    for off in [28, 20] {
        let b = pair(m, p + off)?;
        if b[0] != 0 {
            let rc = invoke(h, m, m.word(p + 8)?, 36, &b)?;
            if bad(rc) {
                return Ok(rc);
            }
            m.write(p + off, &[0; 8])?;
        }
    }
    Ok(0)
}

pub(crate) fn process_1e5bbc(
    m: &mut Memory<'_, '_>,
    p: u32,
    key: u32,
    state: u32,
    s: u32,
    h: &mut impl InternalCalls,
) -> Result<u32, Error> {
    if bad(crate::normalization::quad_check(p, key, 0x3815, 0x30)) {
        return Ok(0x8110_2008);
    }
    if m.word(p + 12)? == 0 {
        return Ok(0x8110_2011);
    }
    m.write(s, &[0; 16])?;
    let owner = pair(m, p)?;
    if bad(h.call(m, 0x19611c, &[owner[0], owner[1], s])?) {
        return Ok(0x8110_2007);
    }
    let ctx = m.word(s)?;
    let logger = m.word(ctx + 16)?;
    if logger != 0 {
        h.call(m, 0x1738e8, &[logger, addr(h.rom(), 0x8f8c6c), 4, 0, addr(h.rom(), 0x1e5ee4)])?;
    }
    m.write(state, &[2])?;
    let fe = m.word(p + 36)?;
    let be = m.word(p + 40)?;
    let mut rc = h.call(m, 0x2645b4, &[be, fe, s + 4])?;
    if bad(rc) {
        match rc & 0x1fff {
            0x17 => {
                m.write(state, &[1])?;
                rc = free_buffers(m, p, h)?;
            }
            0x800 => {
                m.write(state, &[0])?;
                if byte(m, p + 44)? == 0 {
                    rc = 0;
                }
            }
            _ => {
                free_buffers(m, p, h)?;
                m.write(state, &[1])?;
            }
        }
        return Ok(rc);
    }
    if byte(m, s + 4)? == 1 {
        rc = h.call(
            m,
            0x1759a0,
            &[m.word(ctx + 20)?, addr(h.rom(), 0x1e5f04), addr(h.rom(), 0x1e5efc)],
        )?;
        m.write(state, &[3])?;
        return Ok(rc);
    }
    let a = pair(m, p + 20)?;
    let b = pair(m, p + 28)?;
    rc = h.call(m, 0x26a3e4, &[ctx, be, fe, a[0], a[1], b[0], b[1]])?;
    if bad(rc) {
        return Ok(rc);
    }
    m.put_word(s + 8, 0)?;
    rc = h.call(m, 0x252f88, &[fe, 0x3fd, 1, s + 8])?;
    if bad(rc) {
        return Ok(rc);
    }
    put_half(m, m.word(s + 8)? + 8, 0)?;
    m.put_word(s + 8, 0)?;
    let mut leave = false;
    if byte(m, p + 44)? == 0 {
        let status = h.call(m, 0x252f88, &[fe, 0x3fc, 1, s + 8])?;
        if !bad(status) && byte(m, m.word(s + 8)? + 12)? == 0 {
            leave = true;
        }
    }
    if !leave {
        m.write(state, &[1])?;
        if byte(m, p + 44)? == 0 {
            rc = free_buffers(m, p, h)?;
        }
        if bad(rc) {
            return Ok(rc);
        }
    }
    if logger != 0 {
        h.call(m, 0x1738e8, &[logger, addr(h.rom(), 0x8f8c6c), 4, 0, addr(h.rom(), 0x1e5f14)])?;
    }
    Ok(rc)
}

pub(crate) fn process_206be4(
    m: &mut Memory<'_, '_>,
    p: u32,
    key: u32,
    handle: [u32; 2],
    changed: u32,
    s: u32,
    h: &mut impl InternalCalls,
) -> Result<u32, Error> {
    let mut rc = crate::normalization::quad_check(p, key, 0xf382, 0x2c);
    if bad(rc) {
        return Ok(0x89b0_2008);
    }
    m.write(changed, &[1])?;
    m.write(s, &[0; 32])?;
    let vt = m.word(p + 4)?;
    let [a, b] = handle;
    rc = invoke(h, m, vt, 52, &[a, b, 1, 0, s])?;
    if bad(rc) {
        return Ok(rc);
    }
    let root = half(m, s)? as u32;
    let has = invoke(h, m, vt, 92, &[a, b, root, 0, s + 2])?;
    if bad(has) || half(m, s + 2)? != 1 {
        return Ok(rc);
    }
    rc = invoke(h, m, vt, 88, &[a, b, root, 0, s + 4, s + 8])?;
    if bad(rc) || half(m, s + 8)? <= 1 {
        return Ok(rc);
    }
    let text = m.word(s + 4)?;
    rc = invoke(h, m, vt, 52, &[a, b, 3, root, s + 10])?;
    if bad(rc) {
        return Ok(rc);
    }
    let mut current = half(m, s + 10)? as u32;
    while half(m, s + 10)? != 0 && current != 0 {
        rc = invoke(h, m, vt, 84, &[a, b, current, 0, 1, s + 12, s + 8])?;
        if bad(rc) {
            break;
        }
        let mut expand = false;
        if byte(m, s + 12)? == 5 {
            rc = invoke(h, m, vt, 84, &[a, b, current, 1, 1, s + 14, s + 8])?;
            if bad(rc) {
                break;
            }
            rc = invoke(h, m, vt, 84, &[a, b, current, 2, 1, s + 16, s + 8])?;
            if bad(rc) {
                break;
            }
            rc = invoke(h, m, vt, 88, &[a, b, current, 4, s + 20, s + 8])?;
            if bad(rc) {
                break;
            }
            let name = m.word(s + 20)?;
            expand = !contains(m, name, b"spell")?
                && !equals(m, name, b"phon")?
                && !contains(m, name, b"raw")?
                && !contains(m, name, b"RAW")?;
            if expand {
                rc = h.call(
                    m,
                    0x284bc0,
                    &[
                        p,
                        text,
                        half(m, s + 14)? as u32,
                        half(m, s + 16)? as u32,
                        name,
                        a,
                        b,
                        s + 10,
                    ],
                )?;
                if bad(rc) {
                    break;
                }
                current = half(m, s + 10)? as u32;
            }
        }
        if !expand {
            put_half(m, s + 18, current as u16)?;
            rc = invoke(h, m, vt, 60, &[a, b, current, s + 18])?;
            if bad(rc) {
                break;
            }
            current = half(m, s + 18)? as u32;
        }
    }
    for off in [32, 36] {
        let ptr = m.word(p + off)?;
        if ptr != 0 {
            h.call(m, 0x196428, &[m.word(m.word(p)? + 4)?, ptr])?;
        }
        m.put_word(p + off, 0)?;
    }
    if bad(rc) {
        return Ok(rc);
    }
    rc = invoke(h, m, vt, 52, &[a, b, 3, root, s + 10])?;
    if bad(rc) {
        return Ok(rc);
    }
    let mut node = half(m, s + 10)? as u32;
    let mut previous = 0;
    let mut previous_node = 0;
    let mut last_start = 0;
    let mut last_end = 0;
    while node != 0 {
        rc = invoke(h, m, vt, 84, &[a, b, node, 0, 1, s + 12, s + 8])?;
        if bad(rc) {
            return Ok(rc);
        }
        let kind = byte(m, s + 12)?;
        if kind == 4 || kind == 6 {
            rc = invoke(h, m, vt, 84, &[a, b, node, 1, 1, s + 14, s + 8])?;
            if bad(rc) {
                return Ok(rc);
            }
            rc = invoke(h, m, vt, 84, &[a, b, node, 2, 1, s + 16, s + 8])?;
            if bad(rc) {
                return Ok(rc);
            }
            let start = half(m, s + 14)?;
            let end = half(m, s + 16)?;
            if start >= last_end {
                previous_node = node;
                last_start = start;
                last_end = end;
            } else if end < last_end
                || start > last_start
                || (end == last_end && start == last_start)
            {
                rc = invoke(h, m, vt, 96, &[a, b, node])?;
                if bad(rc) {
                    return Ok(rc);
                }
                node = previous;
            } else if previous_node != 0 {
                rc = invoke(h, m, vt, 96, &[a, b, previous_node])?;
                if bad(rc) {
                    return Ok(rc);
                }
            }
        } else if kind <= 3 {
            rc = invoke(h, m, vt, 96, &[a, b, node])?;
            if bad(rc) {
                return Ok(rc);
            }
            node = previous;
        }
        if node == 0 {
            rc = invoke(h, m, vt, 52, &[a, b, 3, root, s + 10])?;
        } else {
            previous = node;
            rc = invoke(h, m, vt, 60, &[a, b, node, s + 10])?;
        }
        if bad(rc) {
            return Ok(rc);
        }
        node = half(m, s + 10)? as u32;
    }
    Ok(rc)
}

pub(crate) fn process_1f0a90(
    m: &mut Memory<'_, '_>,
    p: u32,
    key: u32,
    active: u32,
    s: u32,
    h: &mut impl InternalCalls,
) -> Result<u32, Error> {
    if bad(crate::normalization::quad_check(p, key, 0xf488, 0x54)) {
        return Ok(0x8c90_2008);
    }
    m.write(active, &[1])?;
    m.write(s, &[0; 128])?;
    let streams = m.word(p + 12)?;
    let input0 = pair(m, p + 40)?;
    let input1 = pair(m, p + 48)?;
    let output0 = pair(m, p + 56)?;
    let output1 = pair(m, p + 64)?;
    let mut rc = invoke(h, m, streams, 72, &[input0[0], input0[1], s, s + 8])?;
    if bad(rc) {
        return Ok(rc);
    }
    rc = invoke(h, m, streams, 72, &[input1[0], input1[1], s + 4, s + 8])?;
    if bad(rc) {
        return Ok(rc);
    }
    if m.word(s)? == 0 {
        if m.word(s + 4)? | byte(m, s + 8)? as u32 == 0 {
            m.write(active, &[0])?;
        } else {
            if m.word(s + 4)? != 0 {
                rc = invoke(
                    h,
                    m,
                    streams,
                    64,
                    &[
                        output1[0],
                        output1[1],
                        input1[0],
                        input1[1],
                        u32::MAX,
                        s + 12,
                    ],
                )?;
                if bad(rc) {
                    return Ok(rc);
                }
            }
            if byte(m, s + 8)? != 0 {
                for off in [56, 64] {
                    let output = pair(m, p + off)?;
                    if output[0] != 0 {
                        rc = invoke(h, m, streams, 36, &output)?;
                        m.write(p + off, &[0; 8])?;
                    }
                }
            }
        }
        return Ok(rc);
    }
    let heap = m.word(m.word(p)? + 4)?;

    let mut text_length = 0;
    let mut original_count = 0;
    let mut event_count = 0;
    let mut text = 0;
    let mut events = 0;
    let mut owned = 0;

    let result = (|| -> Result<u32, Error> {
        rc = invoke(h, m, streams, 44, &[input0[0], input0[1], s + 12, s + 20])?;
        if bad(rc) {
            return Ok(rc);
        }
        text_length = m.word(s + 20)? >> 1;
        text = h.call(
            m,
            0x196394,
            &[heap, text_length.wrapping_add(1).wrapping_mul(2)],
        )?;
        if text == 0 {
            return allocation_failure(m, p, h);
        }
        copy(m, text, m.word(s + 12)?, text_length * 2)?;
        put_half(m, text + text_length * 2, 0)?;
        m.put_word(s + 16, text)?;
        rc = invoke(h, m, streams, 44, &[input1[0], input1[1], s + 24, s + 28])?;
        if bad(rc) {
            return Ok(rc);
        }
        original_count = m.word(s + 28)? / 24;
        event_count = original_count;
        events = h.call(m, 0x1963b8, &[heap, 1, original_count * 24])?;
        if events == 0 {
            return allocation_failure(m, p, h);
        }
        owned = h.call(m, 0x1963b8, &[heap, 1, original_count])?;
        if owned == 0 {
            return allocation_failure(m, p, h);
        }
        let input_events = m.word(s + 24)?;
        let base = m.word(input_events + 12)?;
        copy(m, events, input_events, original_count * 24)?;
        m.put_word(s + 32, events)?;
        m.put_word(s + 36, owned)?;
        m.put_word(s + 40, event_count)?;
        let positions = m.word(p + 24)?;
        let position_pair = pair(m, p + 16)?;
        rc = invoke(
            h,
            m,
            positions,
            60,
            &[position_pair[0], position_pair[1], s + 44, 0],
        )?;
        if bad(rc) {
            return Ok(rc);
        }
        if m.word(p + 80)? != 0 {
            let mut voice = 0;
            let mut language = addr(h.rom(), 0x8f90f0);
            let mut position = 0;
            let mut skip = false;
            let mut group_start = 0;
            let mut i = 0;
            while i < event_count {
                let kind = half(m, events + i * 24)?;
                if matches!(kind, 0x24 | 0x15 | 7 | 0x22) {
                    let end = m.word(events + i * 24 + 12)?;
                    m.put_word(s + 48, 0)?;
                    if position < end && !skip {
                        rc = h.call(m, 0x1ef60c, &[p, voice, language, s + 48])?;
                        let config = m.word(s + 48)?;
                        if !bad(rc) && config != 0 {
                            rc = h.call(
                                m,
                                0x1ef960,
                                &[
                                    p,
                                    config,
                                    s + 16,
                                    position.wrapping_sub(base),
                                    end.wrapping_sub(base),
                                    s + 32,
                                    s + 36,
                                    group_start,
                                    i,
                                    s + 40,
                                    voice,
                                    language,
                                ],
                            )?;
                            text = m.word(s + 16)?;
                            events = m.word(s + 32)?;
                            owned = m.word(s + 36)?;
                            event_count = m.word(s + 40)?;
                            if bad(rc) {
                                return Ok(rc);
                            }
                            if byte(m, m.word(config + 52)?)? != 0 {
                                i += 2;
                            }
                        }
                    }
                    position = m.word(events + i * 24 + 12)?;
                    while i < event_count && m.word(events + i * 24 + 12)? == position {
                        let kind = half(m, events + i * 24)?;
                        if kind == 0x24 {
                            voice = m.word(events + i * 24 + 20)?;
                        } else if kind == 0x15 {
                            language = m.word(events + i * 24 + 20)?;
                        }
                        if (kind == 7 || kind == 0x22) && !skip {
                            skip = true;
                        } else if (kind == 7 || kind == 0x15) && skip {
                            skip = false;
                        }
                        i += 1;
                    }
                    group_start = i;
                } else {
                    i += 1;
                }
            }
            m.put_word(s + 48, 0)?;
            let length = u16_len(m, text)?;
            position = position.wrapping_sub(base);
            if position < length * 2 && !skip {
                rc = h.call(m, 0x1ef60c, &[p, voice, language, s + 48])?;
                let config = m.word(s + 48)?;
                if !bad(rc) && config != 0 {
                    rc = h.call(
                        m,
                        0x1ef960,
                        &[
                            p,
                            config,
                            s + 16,
                            position,
                            length * 2,
                            s + 32,
                            s + 36,
                            group_start,
                            i,
                            s + 40,
                            voice,
                            language,
                        ],
                    )?;
                    text = m.word(s + 16)?;
                    events = m.word(s + 32)?;
                    owned = m.word(s + 36)?;
                    event_count = m.word(s + 40)?;
                    if bad(rc) {
                        return Ok(rc);
                    }
                }
            }
        }
        let length = u16_len(m, text)?;
        if length != 0 {
            rc = invoke(
                h,
                m,
                streams,
                52,
                &[output0[0], output0[1], text, length * 2],
            )?;
            if bad(rc) {
                return Ok(rc);
            }
        }
        rc = invoke(h, m, streams, 48, &[input0[0], input0[1], text_length * 2])?;
        if bad(rc) {
            return Ok(rc);
        }
        rc = invoke(
            h,
            m,
            streams,
            52,
            &[output1[0], output1[1], events, event_count * 24],
        )?;
        if bad(rc) {
            return Ok(rc);
        }
        rc = invoke(
            h,
            m,
            streams,
            48,
            &[input1[0], input1[1], original_count * 24],
        )?;
        if bad(rc) {
            return Ok(rc);
        }
        let position = m
            .word(s + 44)?
            .wrapping_add(u16_len(m, text)?.wrapping_sub(text_length).wrapping_mul(2));
        invoke(
            h,
            m,
            positions,
            56,
            &[position_pair[0], position_pair[1], position, 0],
        )
    })()?;
    if text != 0 {
        h.call(m, 0x196428, &[heap, text])?;
    }
    for i in 0..event_count {
        if owned != 0 && byte(m, owned + i)? != 0 {
            let value = m.word(events + i * 24 + 20)?;
            if value != 0 {
                h.call(m, 0x196428, &[heap, value])?;
            }
        }
    }
    if events != 0 {
        h.call(m, 0x196428, &[heap, events])?;
    }
    if owned != 0 {
        h.call(m, 0x196428, &[heap, owned])?;
    }
    Ok(result)
}
fn u16_len(m: &Memory<'_, '_>, a: u32) -> Result<u32, Error> {
    let mut n = 0;
    while half(m, a + n * 2)? != 0 {
        n += 1;
    }
    Ok(n)
}
fn allocation_failure(
    m: &mut Memory<'_, '_>,
    p: u32,
    h: &mut impl InternalCalls,
) -> Result<u32, Error> {
    h.call(
        m,
        0x173dc0,
        &[m.word(m.word(p)? + 16)?, addr(h.rom(), 0x8f905a), 60000, 0],
    )?;
    Ok(0x8c90_200a)
}

fn contains(m: &Memory<'_, '_>, a: u32, needle: &[u8]) -> Result<bool, Error> {
    let mut i = 0;
    while byte(m, a + i)? != 0 {
        let mut j = 0;
        while j < needle.len() && byte(m, a + i + j as u32)? == needle[j] {
            j += 1;
        }
        if j == needle.len() {
            return Ok(true);
        }
        i += 1;
    }
    Ok(false)
}
fn equals(m: &Memory<'_, '_>, a: u32, text: &[u8]) -> Result<bool, Error> {
    for (i, &v) in text.iter().enumerate() {
        if byte(m, a + i as u32)? != v {
            return Ok(false);
        }
    }
    Ok(byte(m, a + text.len() as u32)? == 0)
}

pub fn process(
    m: &mut Memory<'_, '_>,
    entry: u32,
    args: [u32; 5],
    scratch: u32,
    host: &mut impl InternalCalls,
) -> Result<u32, Error> {

    let entry = to_us(host.rom().or(Rom::from_entry(entry)), entry);
    m.check(scratch, if entry == 0x1f0a90 { 128 } else { 32 })?;
    match entry {
        0x1e3b14 => process_1e3b14(m, args[0], args[1], args[4], scratch, host),
        0x1e5bbc => process_1e5bbc(m, args[0], args[1], args[4], scratch, host),
        0x1f0a90 => process_1f0a90(m, args[0], args[1], args[4], scratch, host),
        0x206be4 => process_206be4(
            m,
            args[0],
            args[1],
            [args[2], args[3]],
            args[4],
            scratch,
            host,
        ),
        _ => Err(Error::UnsupportedStage { entry }),
    }
}
