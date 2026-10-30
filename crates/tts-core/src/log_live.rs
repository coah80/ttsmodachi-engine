//! Live fn_1738e8. Guest addresses use the caller's boot::Memory base.
//! No rule tables or recorded callback results are read.
use crate::boot::Memory;

/// Bind a guest method address to its real Rust implementation or platform call.
/// The dispatcher must not retain an arena borrow. Unknown addresses must fail
/// explicitly rather than returning a recorded or invented reply.
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

/// Fan out using vtable byte offset 24, preserving callback read/write order.
/// `varargs` points just past the incoming format word, as ARM's va_list does.
/// Each callback receives a copy of that address, not a shared advancing cursor.
/// This is not fn_1739fc, which does not suppress target 1 at levels above 1.
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
        // Re-read the pair and method after each call. A callback may replace
        // a later target or reenter logging on these same bytes.
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
