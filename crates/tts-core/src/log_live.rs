

use crate::boot::Memory;

pub trait LogMethodsM {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        mem: &mut Memory<'_>,
    ) -> u32;
}

pub fn fn_1738e8(
    mem: &mut Memory<'_>,
    logger: u32,
    category: u32,
    level: u32,
    arg: u32,
    format: u32,
    varargs: u32,
    host: &mut impl LogMethodsM,
) -> u32 {
    if logger == 0
        || level > mem.byte(logger.wrapping_add(48)) as u32
        || (mem.byte(logger.wrapping_add(49)) != 0 && level > 4)
        || format == 0
    {
        return 0;
    }
    if category == 0 {
        return 0x84b0_2006;
    }
    let mut result = 0;
    for index in 0..6 {
        let offset = if index == 5 { 52 } else { 8 + index * 8 };
        let pair = logger.wrapping_add(offset);

        let methods = mem.word(pair.wrapping_add(4));
        if methods != 0 {
            let method = mem.word(methods.wrapping_add(24));
            if method != 0 && (index != 1 || level <= 1) {
                let status = host.log_m(
                    method,
                    mem.word(pair),
                    category,
                    level,
                    arg,
                    format,
                    varargs,
                    mem,
                );
                if status >= 0x8000_0000 {
                    result = status;
                }
            }
        }
    }
    result
}

pub fn variadic_m(
    m: &mut Memory<'_>, entry: u32, a: [u32; 4], format: &[u8],
    varargs: &[u32], scratch: u32,
    mut call: impl FnMut(&mut Memory<'_>, u32, &[u32]) -> u32,
) -> bool {
    assert!(matches!(entry, 0x1738e8 | 0x173dc0));
    if a[0] == 0 { return true; }
    if entry == 0x1738e8 && (a[2] > m.byte(a[0] + 48) as u32
        || (m.byte(a[0] + 49) != 0 && a[2] > 4)) { return true; }
    let format = format.strip_suffix(&[0]).unwrap_or(format);
    let end = m.bytes.len().min(0xa70000);
    let literal = (0x100000 + m.bytes[0x100000..end].windows(format.len() + 1)
        .position(|w| &w[..format.len()] == format && w[format.len()] == 0)
        .expect("ROM logging format")) as u32;
    assert!(varargs.len() <= 32);
    let at = m.offset(scratch);
    let saved: [u8; 160] = m.bytes[at..at + 160].try_into().unwrap();
    for (i, value) in varargs.iter().enumerate() { m.put_word(scratch + i as u32 * 4, *value); }
    if entry == 0x1738e8 { call(m, entry, &[a[0], a[1], a[2], a[3], literal, scratch]); }
    else { call(m, entry, &[a[0], a[1], a[2], literal, scratch, scratch + 128]); }
    m.bytes[at..at + 160].copy_from_slice(&saved);
    true
}
