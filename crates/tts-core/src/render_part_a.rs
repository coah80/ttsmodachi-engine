

use crate::selection::arena::{Arena, Host};
use crate::selection::arena_leaf;

pub fn read_chunk_string(
    arena: &mut Arena<'_>,
    reader: u32,
    source: u32,
    length: u32,
    offset: u32,
    destination: u32,
    out_length: u32,
) -> u32 {
    if reader == 0
        || source == 0
        || offset > length
        || out_length == 0
        || (arena.u32(out_length) != 0 && destination == 0)
    {
        return 0x8a20_2007;
    }
    let encoded = arena.u8(reader.wrapping_add(13)) == 1;
    let end = if encoded { 0xa9 } else { 0 };
    let start = source.wrapping_add(offset);
    let mut size = 0u32;
    while arena.u8(start.wrapping_add(size)) != end {
        size = size.wrapping_add(1);
        if offset.wrapping_add(size) > length {
            return 0x8a20_2004;
        }
    }
    let needed = size.wrapping_add(1);
    let mut rc = 0;
    if needed > arena.u32(out_length) {
        if arena.u32(out_length) != 0 {
            arena.set_u8(destination, 0);
        }
        rc = 0x8a20_2009;
    } else {
        arena_leaf::copy_bytes(arena, destination, start, size);
        arena.set_u8(destination.wrapping_add(size), 0);
        if encoded {
            for i in 0..size {
                let at = destination.wrapping_add(i);
                arena.set_u8(at, arena.u8(at) ^ 0xa9);
            }
        }
    }
    arena.set_u32(out_length, needed);
    rc
}

pub fn next_token(arena: &mut Arena<'_>, cursor: u32, delimiters: u32) -> u32 {
    if cursor == 0 {
        return 0;
    }
    let mut start = arena.u32(cursor);
    if start == 0 {
        return 0;
    }
    while arena.u8(start) != 0
        && arena_leaf::find_byte(arena, delimiters, arena.u8(start) as u32) != 0
    {
        start = start.wrapping_add(1);
    }
    if arena.u8(start) == 0 {
        return 0;
    }
    arena.set_u32(cursor, start);
    let mut end = start;
    while arena.u8(end) != 0 && arena_leaf::find_byte(arena, delimiters, arena.u8(end) as u32) == 0
    {
        end = end.wrapping_add(1);
        arena.set_u32(cursor, end);
    }
    if arena.u8(end) == 0 {
        arena.set_u32(cursor, 0);
    } else {
        arena.set_u8(end, 0);
        arena.set_u32(cursor, end.wrapping_add(1));
    }
    start
}

pub fn search_forward(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    context: u32,
    callback: u32,
    argument: u32,
    out: u32,
) -> u32 {
    if context == 0 || callback == 0 || out == 0 {
        return 0x8110_2007;
    }
    let start = arena.u16(context.wrapping_add(16));
    let inner = arena.u32(context.wrapping_add(12));
    arena.set_u16(out, start);
    let mut value = (start as u32)
        .wrapping_mul(arena.u16(inner.wrapping_add(2)) as u32)
        .wrapping_add(arena.u32(inner.wrapping_add(4)));
    while arena.u16(out) < arena.u16(inner.wrapping_add(8))
        && host.call(arena, callback, &[argument, value]).0 == 0
    {
        arena.set_u16(out, arena.u16(out).wrapping_add(1));
        value = value.wrapping_add(arena.u16(inner.wrapping_add(2)) as u32);
    }
    if arena.u16(out) >= arena.u16(inner.wrapping_add(8)) {
        0x8110_2014
    } else {
        0
    }
}

pub fn search_backward(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    context: u32,
    callback: u32,
    argument: u32,
    out: u32,
) -> u32 {
    if context == 0 || callback == 0 || out == 0 {
        return 0x8110_2007;
    }
    let start = arena.u16(context.wrapping_add(16));
    let inner = arena.u32(context.wrapping_add(12));
    arena.set_u16(out, start);
    let mut value = (start as u32)
        .wrapping_mul(arena.u16(inner.wrapping_add(2)) as u32)
        .wrapping_add(arena.u32(inner.wrapping_add(4)));
    while arena.u16(out) != 0 && host.call(arena, callback, &[argument, value]).0 == 0 {
        arena.set_u16(out, arena.u16(out).wrapping_sub(1));
        value = value.wrapping_sub(arena.u16(inner.wrapping_add(2)) as u32);
    }
    if arena.u16(out) != 0 || host.call(arena, callback, &[argument, value]).0 != 0 {
        0
    } else {
        0x8110_2014
    }
}

pub fn append_range(
    heap: &mut impl crate::heap::Heap,
    base: u32,
    list: u32,
    index: u16,
    low: u32,
    high: u32,
) -> u32 {
    let mut arena = Arena::new(base, heap.arena());
    let old = arena.u32(list.wrapping_add(4));
    let count = arena.u16(list.wrapping_add(2));
    let capacity = arena.u16(list);
    let mut failed = false;
    if old != 0 && (count as u32).wrapping_add(1) <= capacity as u32 {
        arena.set_u16(list.wrapping_add(2), count.wrapping_add(1));
    } else {
        let new_capacity;
        let pointer;
        if old != 0 {
            new_capacity = capacity.wrapping_add(10);
            pointer = heap.reallocate(old, new_capacity as u32 * 12).unwrap_or(0);
        } else {
            arena.set_u16(list, 0);
            arena.set_u16(list.wrapping_add(2), 0);
            new_capacity = 10;
            pointer = heap.allocate(120).unwrap_or(0);
        }
        arena = Arena::new(base, heap.arena());
        failed = pointer == 0;
        if !failed {
            arena.set_u32(list.wrapping_add(4), pointer);
            arena.set_u16(list, new_capacity);
            arena.set_u16(
                list.wrapping_add(2),
                arena.u16(list.wrapping_add(2)).wrapping_add(1),
            );
        }
    }

    let item = if failed {
        0
    } else {
        arena.u32(list.wrapping_add(4)).wrapping_add(
            (arena.u16(list.wrapping_add(2)) as u32)
                .wrapping_sub(1)
                .wrapping_mul(12),
        )
    };
    arena.set_u16(item.wrapping_add(8), index);
    arena.set_u32(item.wrapping_add(4), high.wrapping_add(1));
    arena.set_u32(item, low);
    if failed {
        0x8bd0_200a
    } else {
        0
    }
}

pub enum LanguageText<'a> {
    Address(u32),
    Local(&'a [u8]),
}

pub trait LanguageLog {

    fn message(&mut self, arena: &mut Arena<'_>, logger: u32, text: LanguageText<'_>);

    fn value(&mut self, arena: &mut Arena<'_>, logger: u32, name: u32, value: u32);

    fn allocation_error(&mut self, arena: &mut Arena<'_>, logger: u32);
}

pub fn log_language_values(
    arena: &mut Arena<'_>,
    host: &mut impl LanguageLog,
    engine: u32,
    text: LanguageText<'_>,
    count: u16,
    pairs: u32,
    values: u32,
) {
    let logger = arena.u32(arena.u32(engine).wrapping_add(16));
    host.message(arena, logger, text);
    let mut i = 0u16;
    while i < count {
        let pair = pairs.wrapping_add(i as u32 * 8);
        let name = arena.u32(pair);
        let value = arena.u32(if values == 0 {
            pair.wrapping_add(4)
        } else {
            values.wrapping_add(i as u32 * 4)
        });
        let logger = arena.u32(arena.u32(engine).wrapping_add(16));
        host.value(arena, logger, name, value);
        i = i.wrapping_add(1);
    }
}

pub fn release_language_models(heap: &mut impl crate::heap::Heap, base: u32, engine: u32) {
    let mut i = 0u16;
    loop {
        let arena = Arena::new(base, heap.arena());
        if i >= arena.u16(engine.wrapping_add(0x344)) {
            break;
        }
        let segments = arena.u32(engine.wrapping_add(0x348));
        let voices = arena.u32(segments.wrapping_add(i as u32 * 48).wrapping_add(12));
        heap.release(voices);
        i = i.wrapping_add(1);
    }
    let mut arena = Arena::new(base, heap.arena());
    let segments = arena.u32(engine.wrapping_add(0x348));
    if segments != 0 {
        heap.release(segments);
    }
    arena = Arena::new(base, heap.arena());
    arena.set_u32(engine.wrapping_add(0x348), 0);
    arena.set_u16(engine.wrapping_add(0x344), 0);
    arena.set_u16(engine.wrapping_add(0x346), 0);
    for db_offset in [0x48u32, 0x60] {
        i = 0;
        loop {
            let arena = Arena::new(base, heap.arena());
            let db = engine.wrapping_add(db_offset);
            if i >= arena.u16(db.wrapping_add(8)) {
                break;
            }
            let items = arena.u32(db.wrapping_add(12));
            let langs = arena.u32(items.wrapping_add(i as u32 * 528).wrapping_add(12));
            heap.release(langs);
            i = i.wrapping_add(1);
        }
        for offset in [12u32, 20] {
            let arena = Arena::new(base, heap.arena());
            let pointer = arena.u32(engine.wrapping_add(db_offset).wrapping_add(offset));
            if pointer != 0 {
                heap.release(pointer);
            }
        }
    }
}

pub fn append_language_segment(
    heap: &mut impl crate::heap::Heap,
    host: &mut impl LanguageLog,
    base: u32,
    engine: u32,
    out: u32,
) -> u32 {
    let mut arena = Arena::new(base, heap.arena());
    let cap = arena.u16(engine.wrapping_add(0x344));
    let old = arena.u32(engine.wrapping_add(0x348));
    let count = arena.u16(engine.wrapping_add(0x346));
    if old != 0 && count as u32 + 1 <= cap as u32 {
        arena.set_u16(engine.wrapping_add(0x346), count.wrapping_add(1));
    } else {
        let ncap;
        let pointer;
        if old != 0 {
            ncap = cap.wrapping_add(10);
            pointer = heap.reallocate(old, ncap as u32 * 48).unwrap_or(0);
        } else {
            arena.set_u16(engine.wrapping_add(0x344), 0);
            arena.set_u16(engine.wrapping_add(0x346), 0);
            ncap = 10;
            pointer = heap.allocate(480).unwrap_or(0);
        }
        if pointer == 0 {
            return 0x8bd0_200a;
        }
        arena = Arena::new(base, heap.arena());
        arena.set_u32(engine.wrapping_add(0x348), pointer);
        arena.set_u16(engine.wrapping_add(0x344), ncap);
        arena.set_u16(
            engine.wrapping_add(0x346),
            arena.u16(engine.wrapping_add(0x346)).wrapping_add(1),
        );
    }
    arena = Arena::new(base, heap.arena());
    if arena.u16(engine.wrapping_add(0x344)) > cap {
        let mut i = cap;
        loop {
            arena = Arena::new(base, heap.arena());
            if i >= arena.u16(engine.wrapping_add(0x344)) {
                break;
            }
            let mx = core::cmp::max(
                arena.u16(engine.wrapping_add(0x4a)),
                arena.u16(engine.wrapping_add(0x62)),
            ) as u32;
            let slot = arena
                .u32(engine.wrapping_add(0x348))
                .wrapping_add(i as u32 * 48);
            let voices = if mx == 0 {
                0
            } else {
                heap.allocate_zeroed(1, mx * 8).unwrap_or(0)
            };
            arena = Arena::new(base, heap.arena());
            arena.set_u32(slot.wrapping_add(12), voices);
            if voices == 0 {
                let logger = arena.u32(arena.u32(engine).wrapping_add(16));
                host.allocation_error(&mut arena, logger);
                return 0x8bd0_200a;
            }
            i = i.wrapping_add(1);
        }
    }
    arena = Arena::new(base, heap.arena());
    let segment = arena.u32(engine.wrapping_add(0x348)).wrapping_add(
        (arena.u16(engine.wrapping_add(0x346)) as u32)
            .wrapping_sub(1)
            .wrapping_mul(48),
    );
    arena.set_u32(out, segment);
    0
}

pub trait LanguageLookup: LanguageLog {
    fn identify(
        &mut self,
        arena: &mut Arena<'_>,
        method: u32,
        instance: [u32; 2],
        set: u32,
        key: &[u8],
        which: u16,
    ) -> (u32, u32, u16, u8);
}

pub fn score_language_trigrams(
    heap: &mut impl crate::heap::Heap,
    host: &mut impl LanguageLookup,
    base: u32,
    engine: u32,
    text: u32,
    set: u32,
    which: u16,
    count: u16,
    out: u32,
    pairs: u32,
) -> u32 {
    let mut arena = Arena::new(base, heap.arena());
    for i in 0..count {
        arena.set_u32(out.wrapping_add(i as u32 * 4), 0);
    }
    let len = arena_leaf::string_length(&arena, text).wrapping_add(4) as u16;
    let buf = heap.allocate(len as u32 + 1).unwrap_or(0);
    arena = Arena::new(base, heap.arena());
    if buf == 0 {
        let logger = arena.u32(arena.u32(engine).wrapping_add(16));
        host.allocation_error(&mut arena, logger);
        return 0x8bd0_200a;
    }
    arena.set_u8(buf, b'_');
    arena.set_u8(buf.wrapping_add(1), b'_');
    arena.set_u8(buf.wrapping_add(2), 0);
    arena_leaf::copy_string(&mut arena, buf.wrapping_add(2), text);
    let tail = buf.wrapping_add(len as u32).wrapping_sub(2);
    arena.set_u8(tail, b'_');
    arena.set_u8(tail.wrapping_add(1), b'_');
    arena.set_u8(tail.wrapping_add(2), 0);
    let limit = (len as u32).wrapping_sub(2);
    let mut i = 0u16;
    let mut key = [0u8; 28];
    let mut vals = [0u32; 130];
    while (i as u32) < limit {
        key[0] = 0;
        let mut offset = 0u32;
        let mut first_hex = false;
        for part in 0..3 {
            let source = buf.wrapping_add(i as u32).wrapping_add(offset);
            let byte = arena.u8(source);
            let hex = crate::resources::find_byte(b"0123456789ABCDEF\0", byte as u32).is_some();
            if part == 0 {
                first_hex = hex;
            }
            let amount = if hex { 4 } else { 1 };

            let available = arena_leaf::string_length(&arena, source).wrapping_add(1);
            crate::pronunciation::append_fixed(
                &mut key,
                arena.slice(source, core::cmp::min(available, amount) as usize),
                amount as i32,
            );
            offset = offset.wrapping_add(amount);
        }
        let method = arena.u32(arena.u32(engine.wrapping_add(12)).wrapping_add(28));
        let instance = [
            arena.u32(engine.wrapping_add(4)),
            arena.u32(engine.wrapping_add(8)),
        ];
        let (rc, value, flag, delimiter) =
            host.identify(&mut arena, method, instance, set, &key, which);
        if rc >= 0x8000_0000 {
            return rc;
        }
        for j in 0..count {
            vals[j as usize] = 0;
        }
        if flag != 0 {
            let mut token = arena.u32(value);
            let mut j = 0u16;
            let mut comma = arena_leaf::find_byte(&mut arena, token, delimiter as u32);
            while comma != 0 && j < count {
                if comma != token {
                    arena.set_u8(comma, 0);
                    let length = arena_leaf::string_length(&arena, token);
                    vals[j as usize] =
                        crate::resources::decimal(Some(arena.slice(token, length as usize + 1)));
                }
                j = j.wrapping_add(1);
                token = comma.wrapping_add(1);
                comma = arena_leaf::find_byte(&mut arena, token, delimiter as u32);
            }
            if arena.u8(token) != 0 && j < count {
                let length = arena_leaf::string_length(&arena, token);
                vals[j as usize] =
                    crate::resources::decimal(Some(arena.slice(token, length as usize + 1)));
            }
        }

        if count != 0 {
            for j in 0..count {
                let at = out.wrapping_add(j as u32 * 4);
                let value = vals[j as usize];
                arena.set_u32(
                    at,
                    arena.u32(at).wrapping_add(if value != 0 {
                        value
                    } else {
                        arena.u32(engine.wrapping_add(0x98))
                    }),
                );
            }
        }

        log_language_values(
            &mut arena,
            host,
            engine,
            LanguageText::Local(&key),
            count,
            pairs,
            out,
        );
        i = i.wrapping_add(if first_hex { 4 } else { 1 });
    }
    heap.release(buf);
    0
}

pub(crate) fn fn_173dc0(
    m: &mut crate::boot::Memory<'_>, host: &mut impl crate::boot_part_b::FormatHost,
    logger: u32, category: u32, code: u32, format: u32, args: u32, params: u32,
) -> u32 {
    if logger == 0 { return 0; }
    if category == 0 { return 0x84b0_2006; }
    crate::boot_part_b::fn_173c70(m, logger, category, code, format, args, params, host)
}

pub fn error_log(
    m: &mut crate::boot::Memory<'_>, host: &mut impl crate::boot_part_b::FormatHost,
    logger: u32, category: u32, code: u32, format: u32, args: u32, params: u32,
) -> u32 { fn_173dc0(m, host, logger, category, code, format, args, params) }

#[derive(Default)]
struct WordState {
    spaces: u16, punctuation: u16, start: u16,
    nondigit: bool, digit: bool, punct: bool, upper: bool,
}
impl WordState {
    fn mark(&self, a: &mut Arena<'_>, buffer: u32, out: u32, result: u32) {
        let length = out.wrapping_sub(buffer).wrapping_sub(self.start as u32)
            .wrapping_sub(self.spaces as u32).wrapping_sub(self.punctuation as u32) as i32;
        if (self.upper && length > 1 && length < 5)
            || ((!self.nondigit || self.upper) && (self.digit || self.punct)) {
            let index = a.u16(result + 8);
            a.set_u16(result + 8, index.wrapping_add(1));
            a.set_u16(result + 10 + index as u32 * 2, self.start);
        }
    }
    fn flush(&mut self, a: &mut Arena<'_>, buffer: u32, out: u32, result: u32, separator: bool) {
        self.mark(a, buffer, out, result);
        a.set_u16(result, a.u16(result).wrapping_add(1));
        if self.nondigit && !self.digit { a.set_u16(result+2, a.u16(result+2).wrapping_add(1)); }
        self.start = out.wrapping_sub(buffer) as u16;
        self.spaces=0; self.punctuation=0; self.nondigit=false; self.digit=false; self.upper=true;
        if separator { self.punct=false; }
    }
}

pub fn normalize_language_text(
    heap: &mut impl crate::heap::Heap, base: u32, text: u32, n: u32,
    result: u32, table: u32, table_b: u32,
) -> u32 {
    Arena::new(base,heap.arena()).set_u16(result+8,0);
    let Some(buffer)=heap.allocate(n.wrapping_mul(4).wrapping_add(1)) else { return 0; };
    let mut out=buffer;
    let mut state=WordState {upper:true,..WordState::default()};
    let mut i=0u32;
    while i<n {
        let c=Arena::new(base,heap.arena()).u16(text+i*2) as u32;
        if matches!(c,10|13|27|32|95) {
            Arena::new(base,heap.arena()).set_u8(out,b' '); out+=1;
            state.spaces=state.spaces.wrapping_add(1); i+=1; continue;
        }
        let separator=matches!(c,33..=47|58..=64|91..=94|96|123..=125|171|174|175|187);
        if state.spaces!=0 { state.flush(&mut Arena::new(base,heap.arena()),buffer,out,result,separator); }
        if separator {
            let mut member=|index:u32, set:u32| {
                let c=Arena::new(base,heap.arena()).u16(text+index*2);
                crate::normalization::u16chr(heap.arena(),set.wrapping_sub(base),c).is_some()
            };
            let previous=i==0 || member(i-1,table);
            let edge=i==n.wrapping_sub(1)
                || (i<n.wrapping_sub(1) && member(i+1,table))
                || (i==n.wrapping_sub(2) && member(i+1,table_b) && !member(i,table_b))
                || (i<n.wrapping_sub(2) && member(i+2,table) && member(i+1,table_b));
            if (previous && edge) || (i!=0 && !previous && !edge) {
                Arena::new(base,heap.arena()).set_u8(out,c as u8);out+=1;
                state.punctuation=state.punctuation.wrapping_add(1); state.punct=true;
            }
            i+=1;continue;
        }
        state.punct=false;
        let digit=c.wrapping_sub(48)<10;
        if digit {state.digit=true;} else {state.nondigit=true;}
        if c>255 || digit {
            state.upper=digit && state.upper;
            let mut hex=[0u8;5];
            crate::frontend::render_signed(c as i32,&mut hex,16).expect("u16 fits hexadecimal buffer");
            for b in &mut hex[..4] {if *b>96 {*b-=32;}}
            let len=hex.iter().position(|&b| b==0).unwrap();
            let mut a=Arena::new(base,heap.arena());
            for j in 0..4 {a.set_u8(out+j,b'0');}
            for j in 0..=len {a.set_u8(out+4-len as u32+j as u32,hex[j]);}
            out+=4;
        } else {
            let lower=crate::selection::lowercase_voice_byte(c&255);
            Arena::new(base,heap.arena()).set_u8(out,lower as u8);out+=1;
            state.upper=lower!=(c&255) && state.upper;
        }
        i+=1;
    }
    let mut a=Arena::new(base,heap.arena());
    state.mark(&mut a,buffer,out,result);
    if a.u16(text.wrapping_add(n.wrapping_mul(2)).wrapping_sub(2))&255!=32 {
        a.set_u16(result,a.u16(result).wrapping_add(1));
        if state.nondigit && !state.digit {a.set_u16(result+2,a.u16(result+2).wrapping_add(1));}
    }
    a.set_u8(out,0);
    buffer
}

fn literal(a: &mut Arena<'_>, dst: u32, bytes: &[u8]) {
    for (i,&b) in bytes.iter().enumerate() {a.set_u8(dst+i as u32,b);}
    a.set_u8(dst+bytes.len() as u32,0);
}
fn language_query(heap: &mut impl crate::heap::Heap, h: &mut impl LanguageLookup,
    base:u32, engine:u32, set:u32, key:&[u8], kind:u16) -> (u32,u32,u16,u8) {
    let mut a=Arena::new(base,heap.arena());
    let method=a.u32(a.u32(engine+12)+28);
    let instance=[a.u32(engine+4),a.u32(engine+8)];
    h.identify(&mut a,method,instance,set,key,kind)
}
fn language_allocation_error(heap:&mut impl crate::heap::Heap,h:&mut impl LanguageLog,base:u32,p:u32) {
    let mut a=Arena::new(base,heap.arena());let log=a.u32(a.u32(p)+16);
    h.allocation_error(&mut a,log);
}
fn terminated(a:&Arena<'_>,p:u32,out:&mut [u8])->usize {
    let n=arena_leaf::string_length(a,p) as usize;
    out[..n].copy_from_slice(a.slice(p,n));n
}
fn split_language_list(a:&mut Arena<'_>,s:u32,ch:u8)->u16 {
    let end=arena_leaf::find_byte(a,s,ch as u32);if end!=0 {a.set_u8(end,0);}
    let mut n=1u16;let mut comma=arena_leaf::find_byte(a,s,b',' as u32);
    while comma!=0 {n=n.wrapping_add(1);comma=arena_leaf::find_byte(a,comma+1,b',' as u32);}
    n
}
fn copy_language_tokens(a:&mut Arena<'_>,s:u32,n:u16,items:u32,stride:u32) {
    let mut token=s;let mut comma=arena_leaf::find_byte(a,s,b',' as u32);
    if comma!=0 {a.set_u8(comma,0);}
    let mut i=0u16;
    loop {
        arena_leaf::copy_string(a,items+i as u32*stride,token);
        if comma!=0 {token=comma+1;comma=arena_leaf::find_byte(a,token,b',' as u32);if comma!=0 {a.set_u8(comma,0);}}
        i=i.wrapping_add(1);if i>=n {break;}
    }
}

pub fn initialize_script_database(heap:&mut impl crate::heap::Heap,h:&mut impl LanguageLookup,
    base:u32,p:u32,db:u32,kind:u16,lidcfg:u32)->u32 {
    let mut a=Arena::new(base,heap.arena());
    a.set_u16(db,kind);a.set_u32(db+12,0);a.set_u16(db+8,0);
    let (mut rc,value,flag,ch)=language_query(heap,h,base,p,lidcfg,b"scripts",kind);
    if rc>=0x80000000 || flag==0 {return rc;}
    a=Arena::new(base,heap.arena());let s=a.u32(value);
    let count=split_language_list(&mut a,s,ch);a.set_u16(db+8,count);
    let items=heap.allocate_zeroed(count as u32,528).unwrap_or(0);
    a=Arena::new(base,heap.arena());a.set_u32(db+12,items);
    if items==0 {language_allocation_error(heap,h,base,p);return 0x8bd0200a;}
    copy_language_tokens(&mut a,s,count,items,528);
    for j in 0..count {
        a=Arena::new(base,heap.arena());let item=a.u32(db+12)+j as u32*528;
        let mut name=[0u8;8];let n=terminated(&a,item,&mut name);
        let mut key=[0u8;20];
        for (prefix,offset) in [(b"mertfrq_".as_slice(),16),(b"merwfrq_".as_slice(),272)] {
            key[..prefix.len()].copy_from_slice(prefix);key[prefix.len()..prefix.len()+n].copy_from_slice(&name[..n]);
            literal(&mut a,item+offset,&key[..prefix.len()+n]);
        }
        let prefix=b"languages.";key[..prefix.len()].copy_from_slice(prefix);key[prefix.len()..prefix.len()+n].copy_from_slice(&name[..n]);
        let (status,value,_flag,ch)=language_query(heap,h,base,p,lidcfg,&key[..prefix.len()+n],kind);rc=status;
        if rc>=0x80000000 {break;}
        a=Arena::new(base,heap.arena());let text=a.u32(value);
        let nlang=split_language_list(&mut a,text,ch);a.set_u16(item+8,nlang);
        let langs=heap.allocate_zeroed(nlang as u32,8).unwrap_or(0);
        a=Arena::new(base,heap.arena());a.set_u32(item+12,langs);
        if langs==0 {language_allocation_error(heap,h,base,p);return 0x8bd0200a;}
        copy_language_tokens(&mut a,text,nlang,langs,8);
    }
    if rc>=0x80000000 {return rc;}
    a=Arena::new(base,heap.arena());a.set_u32(db+20,0);
    let count=a.u16(db+8);
    for i in 0..count {
        a=Arena::new(base,heap.arena());let item=a.u32(db+12)+i as u32*528;
        let mut name=[0u8;8];let n=terminated(&a,item,&mut name);
        let ranges:&[(u32,u32)]=match &name[..n] {
            b"latn"=>&[(0,0x24f)],b"cyrl"=>&[(0x400,0x52f),(0x2de0,0x2dff),(0xa640,0xa69f)],
            b"deva"=>&[(0x900,0x97f)],b"hanx"=>&[(0x3400,0x4dbf),(0x4e00,0x9faf)],
            b"hrkt"=>&[(0x3040,0x30ff),(0xff65,0xff9f)],b"hang"=>&[(0x1100,0x11ff),(0xac00,0xd7af),(0xffa0,0xffdc)],
            b"arab"=>&[(0x600,0x6ff),(0x750,0x77f),(0xfb50,0xfdff),(0xfe70,0xfeff)],
            b"grek"=>&[(0x370,0x3ff),(0x1f00,0x1fff)],b"hebr"=>&[(0x590,0x5ff),(0xfb1d,0xfb40)],
            _=>return 0x8bd02007,
        };
        for &(lo,hi) in ranges {append_range(heap,base,db+16,i,lo,hi);}
    }
    0
}

pub fn initialize_language_models(heap:&mut impl crate::heap::Heap,h:&mut impl LanguageLookup,
    base:u32,p:u32,lidcfg:u32)->u32 {
    let mut a=Arena::new(base,heap.arena());a.set_u32(p+0x348,0);a.set_u16(p+0x344,0);
    literal(&mut a,p+0xc0,b"mertfrq");literal(&mut a,p+0x1c0,b"merwfrq");
    let rc=initialize_script_database(heap,h,base,p,p+0x48,0xffff,lidcfg);if rc>=0x80000000 {return rc;}
    let rc=initialize_script_database(heap,h,base,p,p+0x60,1,lidcfg);
    if rc>=0x80000000 && Arena::new(base,heap.arena()).u16(p+0x4a)==0 {rc} else {0}
}

pub struct ReaderRuntime {
    pub category:u32,
    pub messages:[u32;16],
    pub pair_format:u32,
    pub triple_format:u32,
    pub four_format:u32,
    pub file_key:u32,
    pub chunk_key:u32,
    pub parent_key:u32,
    pub offset_key:u32,
    pub scratch:u32,
}

pub trait ReaderHost: crate::boot_part_b::FormatHost + crate::normalization::NormalizationHost + crate::frontend::BlockHeap {
    fn reader_call(&mut self,_m:&mut crate::boot::Memory<'_>,_f:u32,_args:&[u32])->Option<u32> {None}
    fn array_release_method(&mut self, context:u32, pointer:u32);
    fn array_destructor(&mut self, method:u32, element:u32, context:u32);
    fn array_destructor_m(&mut self,_memory:&mut [u8],method:u32,element:u32,context:u32) { self.array_destructor(method,element,context); }
    fn array_initializer(&mut self,memory:&mut [u8],method:u32,source:u32,context:u32,destination:u32)->u32;
}
fn reader_info(m:&mut crate::boot::Memory<'_>,r:u32,k:&ReaderRuntime,level:u32,slot:usize) {
    let logger=m.word(m.word(r+8)+16);
    crate::normalization::log_fanout(m.bytes,logger,k.category,level,0,k.messages[slot]);
}
fn reader_error(m:&mut crate::boot::Memory<'_>,h:&mut impl ReaderHost,r:u32,k:&ReaderRuntime,code:u32,format:u32,args:&[u32]) {
    for (i,&v) in args.iter().enumerate(){m.put_word(k.scratch+0x100+i as u32*4,v);}
    fn_173dc0(m,h,m.word(m.word(r+8)+16),k.category,code,format,k.scratch+0x100,k.scratch);
}
fn reader_unmap(m:&mut crate::boot::Memory<'_>,h:&mut impl ReaderHost,r:u32,rec:u32,k:&ReaderRuntime)->u32 {
    reader_info(m,r,k,3,0);
    let mut rc=0;
    if m.byte(rec+4)==0 {
        if m.word(rec+8)!=0 {
            rc=crate::boot_part_b::Host::call(h,m,0x179208,&[m.word(m.word(r+8)+12),m.word(rec+8),m.word(rec+16)]);
        } else if m.word(rec+12)!=0 {
            let handle=m.word(rec+12);let data=m.word(rec+16);
            rc=crate::normalization::release_mapped(m.bytes,handle,data,h);
        }
    }
    if m.word(rec+8)!=0 || m.word(rec+12)!=0 {
        m.put_word(rec+16,0);m.put_word(rec+24,0);m.put_word(rec+20,0);
    }
    rc
}

pub fn map_reader_window(m:&mut crate::boot::Memory<'_>,h:&mut impl ReaderHost,r:u32,pos:u32,len:u32,rec:u32,k:&ReaderRuntime)->u32 {
    if let Some(rc)=h.reader_call(m,0x241E74,&[r,pos,len,rec]) {return rc;}
    assert_eq!(m.base,0);
    let max_slot=k.scratch+0x80;m.put_word(max_slot,len.max(32));
    let mut rc=0;
    if m.byte(rec+4)!=0 {
        for off in [8,12,16,20,24]{m.put_word(rec+off,0);}
        let at=m.offset(rec+4);m.bytes[at]=0;
    } else if m.word(rec+16)!=0 {
        rc=reader_unmap(m,h,r,rec,k);if rc>=0x80000000{return rc;}
    }
    reader_info(m,r,k,3,1);
    let mut done=false;
    match m.byte(r+12) {
        0=>{
            if m.word(rec+8)==0 {
                rc=crate::boot_part_b::Host::call(h,m,0x179f30,&[m.word(m.word(r+8)+12),m.word(r+16),rec+8]);
            }
            if rc<0x80000000 {
                rc=crate::boot_part_b::fn_179374(m,m.word(m.word(r+8)+12),m.word(rec+8),pos,max_slot,rec+16,h);
            } else {done=true;}
        },
        1=>{
            if m.word(rec+12)==0 {
                let env=m.word(r+8);
                let source=m.word(env+28);let heap=m.word(env+4);let name=m.word(r+16);
                rc=crate::normalization::open_mapped(m.bytes,source,heap,name,rec+12,h);
            }
            if rc<0x80000000 {
                let handle=m.word(rec+12);
                rc=crate::normalization::fetch_span(m.bytes,handle,pos,max_slot,rec+16,h);
            } else {done=true;}
        },
        4=>{if pos.wrapping_add(len)>m.word(rec+24){rc=0x8a202004;done=true;}},
        _=>{rc=0x8a202001;done=true;},
    }
    if !done && rc<0x80000000 && m.word(max_slot)<len {
        reader_unmap(m,h,r,rec,k);rc=0x8a202004;
    }
    if m.word(rec+8)!=0 || m.word(rec+12)!=0 {
        if rc<0x80000000 {m.put_word(rec+20,pos);m.put_word(rec+24,m.word(max_slot).wrapping_add(pos));}
        else {m.put_word(rec+16,0);m.put_word(rec+24,0);m.put_word(rec+20,0);}
    }
    rc
}
pub(crate) fn reader_release_array(m:&crate::boot::Memory<'_>,h:&mut impl ReaderHost,array:u32)->u32 {
    use core::cell::RefCell;
    let host=RefCell::new(h);
    struct V<'a,'b,H>(&'a RefCell<&'b mut H>);
    impl<H:ReaderHost> crate::frontend::HeapVtable for V<'_,'_,H> {
        fn alloc_at(&mut self,_:u32,_:u32)->u32{unreachable!("array destruction does not allocate")}
        fn realloc_at(&mut self,_:u32,_:u32,_:u32)->u32{unreachable!("array destruction does not reallocate")}
        fn release_at(&mut self,ctx:u32,p:u32){self.0.borrow_mut().array_release_method(ctx,p);}
    }
    let method=if array==0 {0}else{m.word(array+12)};
    crate::frontend::destroy_array(m.bytes,(array!=0).then_some(array),&mut V(&host),|element,context|{
        host.borrow_mut().array_destructor(method,element,context);
    })
}

pub fn close_reader(m:&mut crate::boot::Memory<'_>,h:&mut impl ReaderHost,r:u32,k:&ReaderRuntime)->u32 {
    if let Some(rc)=h.reader_call(m,0x2421B0,&[r]) {return rc;}
    assert_eq!(m.base,0);if r==0{return 0x8a202007;}
    let env=m.word(r+8);reader_info(m,r,k,4,2);
    let rc=crate::normalization::close_owner_pair(m.bytes,r+24,h);
    if m.word(r+16)!=0 {crate::boot_part_b::Host::call(h,m,0x196428,&[m.word(m.word(r+8)+4),m.word(r+16)]);}
    if m.word(r+20)!=0 {reader_release_array(m,h,m.word(r+20));}
    if rc<0x80000000 {crate::boot_part_b::Host::call(h,m,0x196428,&[m.word(m.word(r+8)+4),r]);}
    let logger=m.word(env+16);
    crate::normalization::log_fanout(m.bytes,logger,k.category,4,0,k.messages[3]);
    rc
}

pub fn close_reader_chunk(m:&mut crate::boot::Memory<'_>,h:&mut impl ReaderHost,r:u32,k:&ReaderRuntime)->u32 {
    if let Some(rc)=h.reader_call(m,0x2422AC,&[r]) {return rc;}
    assert_eq!(m.base,0);if r==0{return 0x8a202007;}
    let slot=k.scratch+0x80;m.put_word(slot,0);
    let array=m.word(r+20);
    if crate::normalization::chunk_last(m.bytes,array,slot)==0 {
        reader_error(m,h,r,k,0x6987,k.pair_format,&[k.file_key,m.word(r+16)]);return 0x8a202000;
    }
    reader_info(m,r,k,4,4);
    let size=m.word(m.word(slot)+12);m.put_word(r+52,size);
    let align=u16::from_le_bytes(m.bytes[(r+14)as usize..(r+16)as usize].try_into().unwrap())as u32;
    let quotient=crate::normalization::udiv(size,align);let rem=size.wrapping_sub(quotient.wrapping_mul(align));
    if rem!=0 {m.put_word(r+52,align.wrapping_sub(rem).wrapping_add(size));}
    let mut rc=0;
    let array=m.word(r+20);
    if crate::normalization::chunk_pop(m.bytes,array)==0 {
        reader_error(m,h,r,k,0x6988,k.pair_format,&[k.file_key,m.word(r+16)]);rc=0x8a202008;
    }
    reader_info(m,r,k,4,5);rc
}

pub fn reader_chunk_data(m:&mut crate::boot::Memory<'_>,h:&mut impl ReaderHost,r:u32,len:u32,out:u32,k:&ReaderRuntime)->u32 {
    if let Some(rc)=h.reader_call(m,0x242590,&[r,len,out]) {return rc;}
    assert_eq!(m.base,0);if r==0||len==0||out==0{return 0x8a202007;}
    if m.word(r+40)==0{return 0x8a202011;}
    reader_info(m,r,k,4,6);
    let mut rc=0;let pos=m.word(r+52);
    if m.word(r+40)==0||m.word(r+44)>pos||pos.wrapping_add(len)>m.word(r+48) {
        rc=map_reader_window(m,h,r,pos,len,r+24,k);
    }
    let data=if rc>=0x80000000 {0}else{m.word(r+40).wrapping_add(m.word(r+52).wrapping_sub(m.word(r+44)))};
    m.put_word(out,data);reader_info(m,r,k,4,7);rc
}

fn reader_append(m:&mut crate::boot::Memory<'_>,h:&mut impl ReaderHost,array:u32,source:u32)->u32 {
    use core::cell::RefCell;
    let mut bytes=[0u8;16];bytes.copy_from_slice(&m.bytes[source as usize..source as usize+16]);
    let mut block=crate::frontend::GrowBlock{
        data:m.word(array),elem_size:u16::from_le_bytes(m.bytes[array as usize+4..array as usize+6].try_into().unwrap()),
        callback:m.word(array+8),cap:m.word(array+16),count:m.word(array+20),
        grow:u16::from_le_bytes(m.bytes[array as usize+24..array as usize+26].try_into().unwrap()),heap:m.word(array+28),
    };
    let callback=block.callback;
    let shared=RefCell::new(h);
    struct B<'a,'b,H>(&'a RefCell<&'b mut H>);
    impl<H:ReaderHost> crate::frontend::BlockHeap for B<'_,'_,H>{
        fn alloc_block(&mut self,h:u32,n:u32)->u32{self.0.borrow_mut().alloc_block(h,n)}
        fn realloc_block(&mut self,h:u32,p:u32,n:u32)->u32{self.0.borrow_mut().realloc_block(h,p,n)}
        fn alloc_cells(&mut self,h:u32,n:u32,s:u32)->u32{self.0.borrow_mut().alloc_cells(h,n,s)}
    }
    let rc=crate::frontend::append_element(Some(&mut block),m.bytes,source,&bytes,&mut B(&shared),None::<fn(u32,u32,u32)->u32>);
    m.put_word(array,block.data);m.put_word(array+16,block.cap);m.put_word(array+20,block.count);
    if rc!=0&&callback!=0 {
        let destination=block.data.wrapping_add(block.count.wrapping_sub(1).wrapping_mul(block.elem_size as u32));

        if shared.borrow_mut().array_initializer(m.bytes,callback,source,block.heap,destination)==0 {
            m.put_word(array+20,m.word(array+20).wrapping_sub(1));return 0;
        }
    }
    rc
}

pub fn open_reader_chunk(m:&mut crate::boot::Memory<'_>,h:&mut impl ReaderHost,r:u32,name:u32,out_len:u32,out_data:u32,k:&ReaderRuntime)->u32 {
    if let Some(rc)=h.reader_call(m,0x24277C,&[r,name,out_len,out_data]) {return rc;}
    assert_eq!(m.base,0);if r==0||out_len==0{return 0x8a202007;}
    reader_info(m,r,k,4,8);let slot=k.scratch+0x80;m.put_word(slot,0);
    let array=m.word(r+20);let last=crate::normalization::chunk_last(m.bytes,array,slot);let parent=m.word(slot);
    let mut rc=0;
    if last!=0 {
        if m.word(r+52).wrapping_add(8)>m.word(parent+12){reader_info(m,r,k,3,9);rc=0x8a202014;}
        else if m.word(r+40)==0||m.word(r+44)>m.word(r+52)||m.word(r+52).wrapping_add(8)>m.word(r+48){
            rc=map_reader_window(m,h,r,m.word(r+52),8,r+24,k);
        }
    } else {
        rc=map_reader_window(m,h,r,0,8,r+24,k);m.put_word(r+52,0);
        if rc<0x80000000 {
            let c=m.byte(m.word(r+40));if matches!(c,0xe0|0xfb){let at=m.offset(r+13);m.bytes[at]=1;}
        }
    }
    if rc<0x80000000 {
        let off=m.word(r+52).wrapping_sub(m.word(r+44));let source=m.word(r+40);
        if source!=0&&off.wrapping_add(4)<=m.word(r+48)&&name!=0 {
            let mut a=Arena::new(0,m.bytes);arena_leaf::copy_bytes(&mut a,name,source.wrapping_add(off),4);a.set_u8(name+4,0);
            if a.u8(r+13)==1 {for i in 0..4 {a.set_u8(name+i,a.u8(name+i)^0xa9);}}
        }
        let mut a=Arena::new(0,m.bytes);arena_leaf::copy_bytes(&mut a,out_len,source.wrapping_add(off).wrapping_add(4),4);
        a.set_u32(r+52,a.u32(r+52).wrapping_add(8));
        for i in 0..4 {
            let c=a.u8(name+i);
            if c.is_ascii_alphanumeric()||c==b' ' {continue;}
            if parent!=0 {
                reader_error(m,h,r,k,0x6983,k.triple_format,&[k.file_key,m.word(r+16),k.parent_key,parent,k.offset_key,m.word(r+52).wrapping_sub(8)]);
            } else {reader_error(m,h,r,k,0x697c,k.pair_format,&[k.file_key,m.word(r+16)]);}
            rc=0x8a202004;break;
        }
        if rc<0x80000000&&parent!=0 {
            let len=m.word(out_len);let end=m.word(parent+12);
            if len>end||(m.word(r+52).wrapping_add(len)as i32)>(end as i32){
                reader_error(m,h,r,k,0x6984,k.four_format,&[k.file_key,m.word(r+16),k.chunk_key,name]);rc=0x8a202004;
            }
        }
        if rc<0x80000000&&out_data!=0 {
            let pos=m.word(r+52);let len=m.word(out_len);
            if m.word(r+40)==0||m.word(r+44)>pos||pos.wrapping_add(len)>m.word(r+48) {
                rc=map_reader_window(m,h,r,pos,len,r+24,k);
            }
        }
        if rc<0x80000000 {
            let element=k.scratch+0x40;for i in 0..16 {m.bytes[(element+i)as usize]=0;}
            let mut a=Arena::new(0,m.bytes);

            a.set_u32(element+4,r);arena_leaf::copy_string(&mut a,element,name);
            a.set_u32(element+8,a.u32(r+52));a.set_u32(element+12,a.u32(r+52).wrapping_add(a.u32(out_len)));
            if reader_append(m,h,m.word(r+20),element)!=0 {
                if out_data!=0 {m.put_word(out_data,m.word(r+40).wrapping_add(m.word(r+52).wrapping_sub(m.word(r+44))));}
            } else {reader_error(m,h,r,k,0x6978,0,&[]);rc=0x8a20200a;}
        }
    }
    if rc>=0x80000000 {m.bytes[name as usize]=0;m.put_word(out_len,0);if out_data!=0 {m.put_word(out_data,0);}}
    reader_info(m,r,k,4,10);rc
}

struct ReaderHeapCall { args: Option<(u32,u32)> }
impl crate::frontend::HeapVtable for ReaderHeapCall {
    fn alloc_at(&mut self,obj:u32,size:u32)->u32 { self.args=Some((obj,size));0 }
    fn realloc_at(&mut self,_:u32,_:u32,_:u32)->u32 { unreachable!() }
    fn release_at(&mut self,obj:u32,p:u32) { self.args=Some((obj,p)); }
}
struct ReaderMethods<'a,H>(&'a mut H);
impl<H:ReaderHost> crate::reentrant2::HooksM for ReaderMethods<'_,H> {
    fn call_m(&mut self,f:u32,a:&[u32],m:&mut crate::boot::Memory<'_>)->u32 {
        crate::boot_part_b::Host::call(self.0,m,f,a)
    }
}
struct ReaderCloneHooks<'a,H>(&'a mut H);
impl<H:ReaderHost> crate::reentrant2::HooksM for ReaderCloneHooks<'_,H> {
    fn call_m(&mut self,f:u32,a:&[u32],m:&mut crate::boot::Memory<'_>)->u32 {
        match f {
            0x1963b8=>crate::reentrant2::call_method_m(m,a[0],a[1],a[2],&mut ReaderMethods(self.0)),
            0x196394|0x196428=>{
                let mut call=ReaderHeapCall{args:None};
                if f==0x196394 {
                    crate::frontend::heap_alloc(m.bytes,(a[0]!=0).then_some(a[0]),a[1],&mut call);
                } else {
                    crate::frontend::heap_release(m.bytes,(a[0]!=0).then_some(a[0]),(a[1]!=0).then_some(a[1]),&mut call);
                }
                if let Some((object,value))=call.args {
                    let method=m.word(m.word(a[0])+if f==0x196394{0}else{12});
                    crate::boot_part_b::Host::call(self.0,m,method,&[object,value])
                } else {0}
            },
            _ if a.len()==3=>self.0.array_initializer(m.bytes,f,a[0],a[1],a[2]),
            _=>{self.0.array_destructor_m(m.bytes,f,a[0],a[1]);0},
        }
    }
}

pub fn find_reader_chunk(m:&mut crate::boot::Memory<'_>,h:&mut impl ReaderHost,r:u32,name:u32,mode:u32,out_len:u32,out_data:u32,k:&ReaderRuntime)->u32 {
    if let Some(rc)=h.reader_call(m,0x242C9C,&[r,name,mode,out_len,out_data]) {return rc;}
    assert_eq!(m.base,0);if r==0||out_len==0{return 0x8a202007;}
    let count=crate::normalization::idx5(m.bytes,m.word(r+20));if count==0{return 0x8a202007;}
    reader_info(m,r,k,4,11);
    let slot=k.scratch+0x84;let source=m.word(r+20);m.put_word(slot,0);
    if crate::reentrant2::clone_table_m(m,source,slot,&mut ReaderCloneHooks(h))==0 {
        reader_error(m,h,r,k,0x6978,0,&[]);return 0x8a20200a;
    }
    let clone=m.word(slot);let saved=m.word(r+52);let chunk_slot=k.scratch+0x88;m.put_word(chunk_slot,0);
    let mut cleanup=false;let mut rc=0;
    match mode{
        0=>{m.put_word(r+52,12);for _ in 1..count{let array=m.word(r+20);crate::normalization::chunk_pop(m.bytes,array);}},
        1=>if count!=1 {
            let array=m.word(r+20);
            if crate::normalization::span_at(m.bytes,array,1,chunk_slot)==0 {cleanup=true;}
            else {m.put_word(r+52,m.word(m.word(chunk_slot)+12));for _ in 1..count{let array=m.word(r+20);crate::normalization::chunk_pop(m.bytes,array);}}
        },
        2=>{
            let array=m.word(r+20);if crate::normalization::chunk_last(m.bytes,array,chunk_slot)==0 {cleanup=true;}
            else {m.put_word(r+52,m.word(m.word(chunk_slot)+8));}
        },
        3=>{},
        _=>{reader_error(m,h,r,k,0x6986,k.pair_format,&[k.chunk_key,name]);cleanup=true;},
    }
    if cleanup&&matches!(mode,1|2) {reader_error(m,h,r,k,0x6985,k.pair_format,&[k.chunk_key,name]);}
    if !cleanup {
        loop {
            let id=k.scratch+0xc0;rc=open_reader_chunk(m,h,r,id,out_len,0,k);
            if rc>=0x80000000 {cleanup=true;break;}
            let mut a=Arena::new(0,m.bytes);if arena_leaf::compare_strings(&mut a,id,name)==0 {break;}
            rc=close_reader_chunk(m,h,r,k);if rc>=0x80000000 {cleanup=true;break;}
        }
        if !cleanup&&out_data!=0 {rc=reader_chunk_data(m,h,r,m.word(out_len),out_data,k);if rc>=0x80000000{cleanup=true;}}
    }
    if cleanup {
        rc=0x8a202014;
        if clone!=0 {reader_release_array(m,h,m.word(r+20));m.put_word(r+20,clone);m.put_word(r+52,saved);}
        m.put_word(out_len,0);if out_data!=0 {m.put_word(out_data,0);}
    } else {reader_release_array(m,h,clone);}
    reader_info(m,r,k,4,12);rc
}

pub struct LanguageRuntime<'a>{
    pub key:&'a mut [u8],pub values:u32,pub fallback:u32,pub cursor:u32,pub params:u32,
    pub delimiters:u32,pub punctuation:u32,pub table:u32,pub table_b:u32,
    pub category:u32,pub special_format:u32,pub fuzzy_format:u32,pub summary:u32,
    pub http:u32,pub ftp:u32,
}
fn word_query(heap:&mut impl crate::heap::Heap,h:&mut impl LanguageLookup,p:u32,set:u32,tok:u32,kind:u16,k:&mut LanguageRuntime<'_>)->(u32,u32,u16,u8){
    let a=Arena::new(0,heap.arena());let n=terminated(&a,tok,k.key);
    language_query(heap,h,0,p,set,&k.key[..n],kind)
}
fn score_log(heap:&mut impl crate::heap::Heap,p:u32,k:&LanguageRuntime<'_>,fmt:u32){
    let a=Arena::new(0,heap.arena());let logger=a.u32(a.u32(p)+16);
    crate::normalization::log_fanout(heap.arena(),logger,k.category,5,0,fmt);
}
fn score_error(heap:&mut impl crate::heap::Heap,h:&mut impl crate::boot_part_b::FormatHost,p:u32,k:&LanguageRuntime<'_>){
    let a=Arena::new(0,heap.arena());let logger=a.u32(a.u32(p)+16);
    fn_173dc0(&mut crate::boot::Memory{base:0,bytes:heap.arena()},h,logger,k.category,0xbb80,0,0,k.params);
}
fn next_word(heap:&mut impl crate::heap::Heap,k:&LanguageRuntime<'_>)->u32{next_token(&mut Arena::new(0,heap.arena()),k.cursor,k.delimiters)}
fn special_word(heap:&mut impl crate::heap::Heap,buffer:u32,tok:u32,res:u32,k:&LanguageRuntime<'_>)->bool{
    let mut a=Arena::new(0,heap.arena());let at=arena_leaf::find_byte(&mut a,tok,b'@' as u32);
    if at!=0&&arena_leaf::find_byte(&mut a,at,b'.' as u32)!=0{return true;}
    if crate::frontend::find_substr(heap.arena(),tok,k.http)!=0||crate::frontend::find_substr(heap.arena(),tok,k.ftp)!=0{return true;}
    a=Arena::new(0,heap.arena());let off=tok.wrapping_sub(buffer)as u16;
    (0..a.u16(res+8)).any(|i|a.u16(res+10+i as u32*2)==off)
}
fn dictionary_values(a:&mut Arena<'_>,value:u32,ch:u8,count:u16,out:u32){
    let mut s=a.u32(value);let mut i=0u16;let mut comma=arena_leaf::find_byte(a,s,ch as u32);
    while comma!=0&&i<count{
        if comma!=s{a.set_u8(comma,0);let n=arena_leaf::string_length(a,s);let v=crate::resources::decimal(Some(a.slice(s,n as usize+1)));a.set_u32(out+i as u32*4,v);}
        i=i.wrapping_add(1);s=comma+1;comma=arena_leaf::find_byte(a,s,ch as u32);
    }
    if a.u8(s)!=0&&i<count{let n=arena_leaf::string_length(a,s);let v=crate::resources::decimal(Some(a.slice(s,n as usize+1)));a.set_u32(out+i as u32*4,v);}
}
fn compare_scores(memory:&[u8],a:u32,b:u32)->i32{
    let a=crate::pronunciation::read_offset(memory,a+4);let b=crate::pronunciation::read_offset(memory,b+4);a.wrapping_sub(b)as i32
}
fn finish_language(heap:&mut impl crate::heap::Heap,h:&mut impl LanguageLookup,p:u32,buffer:u32,count:u16,pairs:u32,k:&LanguageRuntime<'_>){
    crate::fe_text::qsort_2409d4(heap.arena(),pairs,count as u32,8,compare_scores);
    let mut a=Arena::new(0,heap.arena());let off=if a.u8(p+0x40)!=0{0xa8}else{0x9c};
    if a.u8(p+off)!=0&&count>1 {
        let diff=a.u32(pairs+4).wrapping_sub(a.u32(pairs+12));
        if (crate::render_part_b::fn_17c4d8(diff as i32)as u32)<a.u32(p+off+4)&&{let name=a.u32(pairs+8);arena_leaf::compare_strings(&mut a,name,p+off+8)==0} {
            score_log(heap,p,k,k.fuzzy_format);a=Arena::new(0,heap.arena());
            let mut tmp=[0u8;8];tmp.copy_from_slice(a.slice(pairs+8,8));a.copy(pairs+8,pairs,8);
            for(i,&b)in tmp.iter().enumerate(){a.set_u8(pairs+i as u32,b);}
        }
    }
    log_language_values(&mut a,h,p,LanguageText::Address(k.summary),count,pairs,0);
    heap.release(buffer);
}
fn identify_language_words<H:LanguageLookup+crate::boot_part_b::FormatHost>(
    heap:&mut impl crate::heap::Heap,h:&mut H,p:u32,text:u32,len:u32,key_a:u32,key_b:u32,kind:u16,count:u16,pairs:u32,res:u32,k:&mut LanguageRuntime<'_>,global:bool,
)->u32{
    let mut a=Arena::new(0,heap.arena());for off in [0,2,4,6]{a.set_u16(res+off,0);}
    for i in 0..count as u32{a.set_u32(k.values+i*4,0);a.set_u32(k.fallback+i*4,0);}
    let buffer=normalize_language_text(heap,0,text,len,res,k.table,k.table_b);
    if buffer==0{score_error(heap,h,p,k);return 0x8bd0200a;}
    a=Arena::new(0,heap.arena());a.set_u32(k.cursor,buffer);
    let mut tok=next_word(heap,k);let mut rc=0;
    while tok!=0 {
        let(status,_value,flag,_ch)=word_query(heap,h,p,key_b,tok,kind,k);rc=status;if rc>=0x80000000{return rc;}
        let next=next_word(heap,k);
        if flag==0 {
            if !global&&special_word(heap,buffer,tok,res,k){score_log(heap,p,k,k.special_format);}
            else{
                a=Arena::new(0,heap.arena());let mut q=tok;
                while a.u8(q)!=0{let c=a.u8(q);if arena_leaf::find_byte(&mut a,k.punctuation,c as u32)!=0{a.set_u8(q,b' ');}q+=1;}
            }
        }
        if next!=0{
            a=Arena::new(0,heap.arena());let mut e=next-1;
            while e>buffer{let c=a.u8(e);if c==0||arena_leaf::find_byte(&mut a,k.delimiters,c as u32)!=0{a.set_u8(e,b' ');e-=1;}else{break;}}
        }
        tok=next;
    }
    a=Arena::new(0,heap.arena());a.set_u32(k.cursor,buffer);tok=next_word(heap,k);
    while tok!=0 {
        if global {
            rc=score_language_trigrams(heap,h,0,p,tok,key_a,kind,count,k.values,pairs);if rc>=0x80000000{return rc;}
            log_language_values(&mut Arena::new(0,heap.arena()),h,p,LanguageText::Address(tok),count,pairs,k.values);
        }
        let(status,value,flag,ch)=word_query(heap,h,p,key_b,tok,kind,k);rc=status;if rc>=0x80000000{return rc;}
        let word_values=if global{k.fallback}else{k.values};
        a=Arena::new(0,heap.arena());for i in 0..count as u32{a.set_u32(word_values+i*4,0);}
        if flag!=0{a.set_u16(res+4,a.u16(res+4).wrapping_add(1));dictionary_values(&mut a,value,ch,count,word_values);}
        if !global {
            log_language_values(&mut a,h,p,LanguageText::Address(tok),count,pairs,k.values);
            if count!=0 {
                let mut any0=false;let mut anynz=false;
                for i in 0..count as u32{let v=a.u32(k.values+i*4);a.set_u32(k.fallback+i*4,0);any0|=v==0;anynz|=v!=0;}
                if any0 {
                    if !anynz&&special_word(heap,buffer,tok,res,k){
                        score_log(heap,p,k,k.special_format);a=Arena::new(0,heap.arena());
                        let name=if a.u8(p+0xb6)!=0{p+0xbc}else{a.u32(p+0x350)};
                        for i in 0..count as u32{let same=arena_leaf::compare_bounded(&a,a.u32(pairs+i*8),name,2)==0;a.set_u32(k.fallback+i*4,if same{10}else{100});}
                    } else {
                        rc=score_language_trigrams(heap,h,0,p,tok,key_a,kind,count,k.fallback,pairs);if rc>=0x80000000{return rc;}
                    }
                }
                log_language_values(&mut Arena::new(0,heap.arena()),h,p,LanguageText::Address(tok),count,pairs,k.fallback);
            }
        }
        a=Arena::new(0,heap.arena());
        for i in 0..count as u32{
            let v=a.u32(word_values+i*4);let other=if global{k.values}else{k.fallback};
            let v=if v==0{a.u32(other+i*4)}else{v};let out=pairs+i*8+4;a.set_u32(out,a.u32(out).wrapping_add(v));
        }
        log_language_values(&mut a,h,p,LanguageText::Address(tok),count,pairs,0);tok=next_word(heap,k);
    }
    a=Arena::new(0,heap.arena());if a.u16(res)!=0 {let q=crate::normalization::udiv(a.u16(res+4)as u32*100,a.u16(res)as u32);a.set_u16(res+6,q as u16);}
    if !global&&a.u8(p+0xb6)!=0&&count!=0{
        for i in 0..count as u32{
            if arena_leaf::compare_bounded(&a,a.u32(pairs+i*8),p+0xbc,2)==0{
                let out=pairs+i*8+4;
                let v=if a.u16(res+2)==0||a.u16(res+4)==0||a.u8(p+0x354)!=0{0}else{let adj=a.u32(p+0xb8).wrapping_mul(a.u16(res+2)as u32);let old=a.u32(out);if adj<old{old-adj}else{1}};
                a.set_u32(out,v);
            }
        }
    }
    finish_language(heap,h,p,buffer,count,pairs,k);rc
}

pub fn identify_language_segment<H:LanguageLookup+crate::boot_part_b::FormatHost>(heap:&mut impl crate::heap::Heap,h:&mut H,p:u32,text:u32,len:u32,key_a:u32,key_b:u32,kind:u16,count:u16,pairs:u32,res:u32,k:&mut LanguageRuntime<'_>)->u32{
    identify_language_words(heap,h,p,text,len,key_a,key_b,kind,count,pairs,res,k,false)
}

pub fn identify_language_text<H:LanguageLookup+crate::boot_part_b::FormatHost>(heap:&mut impl crate::heap::Heap,h:&mut H,p:u32,text:u32,len:u32,k:&mut LanguageRuntime<'_>)->u32{
    let mut a=Arena::new(0,heap.arena());let alt=a.u8(p+0x40)!=0;let db=p+if alt{0x60}else{0x48};let n=a.u16(db+2);let pairs=a.u32(p+0x340);
    for i in 0..n as u32{a.set_u32(pairs+i*8,a.u32(db+4)+i*26);a.set_u32(pairs+i*8+4,0);}
    identify_language_words(heap,h,p,text,len,p+0xc0,p+0x1c0,if alt{1}else{0xffff},n,pairs,p+0x322,k,true)
}

fn language_segment_slot(heap:&mut impl crate::heap::Heap,h:&mut impl LanguageLookup,p:u32,k:&LanguageRuntime<'_>)->Result<u32,u32>{
    let out=k.params+0x20;let rc=append_language_segment(heap,h,0,p,out);
    if rc>=0x80000000 {Err(rc)}else{Ok(Arena::new(0,heap.arena()).u32(out))}
}

pub fn identify_script_segments<H:LanguageLookup+crate::boot_part_b::FormatHost>(
    heap:&mut impl crate::heap::Heap,h:&mut H,p:u32,text:u32,count:u32,k:&mut LanguageRuntime<'_>,
)->u32{
    let mut a=Arena::new(0,heap.arena());a.set_u16(p+0x346,0);
    let db=p+if a.u8(p+0x40)!=0{0x60}else{0x48};
    if a.u16(db+8)==0 {
        let segment=match language_segment_slot(heap,h,p,k){Ok(v)=>v,Err(rc)=>return rc};
        a=Arena::new(0,heap.arena());a.set_u32(segment,0);a.set_u16(segment+8,0xffff);a.set_u16(segment+10,0);a.set_u32(segment+4,count);
    } else {
        let mut previous=-1i16;let mut segment=0;let mut previous_pos=u32::MAX;let mut last_space=u32::MAX;
        for pos in 0..count{
            a=Arena::new(0,heap.arena());let ch=a.u16(text+pos*2)as u32;let mut nonalpha=false;
            if ch<255 {
                let b=ch as u8;nonalpha=!b.is_ascii_alphabetic();if b==32||b==9 {last_space=pos;}
            }
            let ranges=a.u32(db+20);let n=a.u16(db+18);
            for i in 0..n as u32{
                let range=ranges+i*12;if ch<a.u32(range)||ch>=a.u32(range+4){continue;}
                let index=a.u16(range+8)as i16;if index<0||nonalpha {break;}
                if index!=previous{
                    if segment!=0 {a.set_u32(segment+4,previous_pos.wrapping_add(1));}
                    segment=match language_segment_slot(heap,h,p,k){Ok(v)=>v,Err(rc)=>return rc};
                    a=Arena::new(0,heap.arena());a.set_u16(segment+8,index as u16);
                    a.set_u32(segment,if (last_space as i32)>=(previous_pos as i32){last_space.wrapping_add(1)}else{pos});a.set_u16(segment+10,0);
                }
                previous=index;previous_pos=pos;break;
            }
        }
        if segment!=0 {Arena::new(0,heap.arena()).set_u32(segment+4,previous_pos.wrapping_add(1));}
    }
    a=Arena::new(0,heap.arena());let n=a.u16(p+0x346);let mut rc=0;
    for i in 0..n as u32 {
        a=Arena::new(0,heap.arena());let segment=a.u32(p+0x348)+i*48;let index=a.u16(segment+8)as i16;let pairs=a.u32(segment+12);let res=segment+16;
        let(key_a,key_b,cnt)=if index>=0 {
            let item=a.u32(db+12)+index as u32*528;let cnt=a.u16(item+8);
            for j in 0..cnt as u32{a.set_u32(pairs+j*8,a.u32(item+12)+j*8);a.set_u32(pairs+j*8+4,0);}
            (item+16,item+272,cnt)
        } else {
            let cnt=a.u16(db+2);for j in 0..cnt as u32{a.set_u32(pairs+j*8,a.u32(db+4)+j*26);a.set_u32(pairs+j*8+4,0);}
            (p+0xc0,p+0x1c0,cnt)
        };
        a.set_u16(segment+10,cnt);
        if cnt>1{
            let start=a.u32(segment);let len=a.u32(segment+4).wrapping_sub(start);let kind=a.u16(db);
            rc=identify_language_segment(heap,h,p,text+start*2,len,key_a,key_b,kind,cnt,pairs,res,k);if rc>=0x80000000{return rc;}
        } else if cnt==1 {for off in [0,2,4,6]{a.set_u16(res+off,0);}}
    }
    rc
}
