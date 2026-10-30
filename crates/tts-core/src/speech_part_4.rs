//! ROM libc entries and RVCT continuation bindings for speech_1.
//! Guest addresses index caller-owned memory. No allocations or host libc.
use crate::{boot::Memory, fe_text};

/// fn_100560. Bounded printf output callback. The end is exclusive.
pub fn fn_100560(m: &mut Memory<'_>, byte: u32, state: u32) {
    let cursor = m.word(state);
    if cursor < m.word(state.wrapping_add(4)) {
        m.put_word(state, cursor.wrapping_add(1));
        let at = m.offset(cursor);
        m.bytes[at] = byte as u8;
    }
}
/// fn_2ffb34. Read one format byte and advance the guest cursor, including NUL.
pub fn fn_2ffb34(m: &mut Memory<'_>, state: u32) -> u32 {
    let p = m.word(state.wrapping_add(16));
    m.put_word(state.wrapping_add(16), p.wrapping_add(1));
    m.byte(p) as u32
}
/// fn_2ffdc2. RVCT unsigned hh/h argument narrowing, with hh taking priority.
pub fn fn_2ffdc2(m: &mut Memory<'_>, value: u32, state: u32) -> u32 {
    let flags = m.word(state);
    if flags & 1024 != 0 { value as u8 as u32 }
    else if flags & 256 != 0 { value as u16 as u32 }
    else { value }
}
/// fn_30208c. Unsigned decimal digit predicate, including out-of-byte inputs.
pub fn fn_30208c(_m: &mut Memory<'_>, value: u32) -> u32 {
    u32::from(value.wrapping_sub(48) < 10)
}
/// fn_1012cc. Bind the Thumb strchr to the existing byte-string port.
pub fn fn_1012cc(m: &mut Memory<'_>, text: u32, byte: u32) -> u32 {
    let p = fe_text::strchr_195d30(m.bytes, text - m.base, byte);
    if p == 0 { 0 } else { p + m.base }
}
/// fn_3003fc. RVCT unsigned divide-by-ten, quotient in r0 and remainder in r1.
pub fn fn_3003fc(_m: &mut Memory<'_>, value: u32) -> (u32, u32) {
    (value / 10, value % 10)
}

/// Platform callbacks receive the current memory, never a captured arena.
pub trait Host {
    /// Regional guest heap globals and platform synchronization methods.
    fn heap_table(&mut self, _: &mut Memory<'_>) -> u32 { panic!("guest heap table is required") }
    fn heap_optional_flag(&mut self, _: &mut Memory<'_>) -> u32 { panic!("optional heap flag is required") }
    fn heap_thread_id(&mut self) -> u32 { panic!("platform thread id is required") }
    fn heap_arbiter_address(&self) -> u32 { panic!("platform arbiter is required") }
    /// 32 disjoint caller-owned bytes, restored by conversion helpers.
    fn runtime_scratch(&mut self, _: &mut Memory<'_>) -> u32 { panic!("conversion scratch is required") }
    fn runtime_locale(&mut self, _: &mut Memory<'_>) -> u32 { panic!("conversion locale is required") }

    fn decimal_point(&mut self, _: &mut Memory<'_>) -> u8 { b'.' }
    /// A write through a NULL capture buffer reached guest code (see
    /// fn_c726a8). Live hosts fail the request with it.
    fn code_overwritten(&mut self, end: u32) { panic!("guest code overwritten up to {end:#x}") }
    /// A capture allocation of `size` bytes failed (heap exhausted). Hosts
    /// that serve only full renders fail the request; the ARM path continues.
    fn heap_exhausted(&mut self, _size: u32) {}
    fn call(&mut self, memory: &mut Memory<'_>, function: u32, arguments: &[u32]) -> u32;
}
fn emit(m: &mut Memory<'_>, host: &mut impl Host, state: u32, byte: u32) {
    let callback = m.word(state + 4);
    let object = m.word(state + 8);
    if callback & !1 == 0x100560 {
        fn_100560(m, byte, object);
    } else {
        host.call(m, callback, &[byte, object]);
    }
}
/// fn_100c22. Existing strncat port. Source and destination must not overlap.
pub fn fn_100c22(m: &mut Memory<'_>, dst: u32, src: u32, count: u32) -> u32 {
    fe_text::strncat_195dac(m.bytes, dst - m.base, src - m.base, count);
    dst
}
/// fn_302edc. Bind the RVCT word-at-a-time strlen to the existing byte model.
pub fn fn_302edc(m: &mut Memory<'_>, text: u32) -> u32 {
    crate::normalization::strlen8(m.bytes, text - m.base)
}
/// fn_2ffdd8. The runtime's ASCII-only wide lowercase wrapper.
/// global is the caller-resolved regional locale object, as in speech_part_3.
pub fn fn_2ffdd8(m: &mut Memory<'_>, character: u32, global: u32) -> u32 {
    if character <= 127 {
        let p = crate::speech_part_3::fn_302078(m, global);
        m.locale_lower(m.word(p), character)
    } else { character }
}
/// fn_2ffc14. Leading field padding, bit 0 suppresses it and bit 4 chooses zero.
pub fn fn_2ffc14(m: &mut Memory<'_>, host: &mut impl Host, state: u32) {
    let mut remaining = m.word(state + 24) as i32;
    let flags = m.word(state);
    let byte = if flags & 16 != 0 { 48 } else { 32 };
    if flags & 1 != 0 { return; }
    loop {
        remaining = remaining.wrapping_sub(1);
        if remaining < 0 { break; }
        emit(m, host, state, byte);
        m.put_word(state + 32, m.word(state + 32).wrapping_add(1));
    }
}
/// fn_2ffc40. Trailing space padding, enabled by bit 0.
pub fn fn_2ffc40(m: &mut Memory<'_>, host: &mut impl Host, state: u32) {
    let mut remaining = m.word(state + 24) as i32;
    if m.word(state) & 1 == 0 { return; }
    loop {
        remaining = remaining.wrapping_sub(1);
        if remaining < 0 { break; }
        emit(m, host, state, 32);
        m.put_word(state + 32, m.word(state + 32).wrapping_add(1));
    }
}
struct Release<'a,H> { host: &'a mut H, base: u32, method: u32 }
impl<H:Host> crate::frontend::HeapVtableM for Release<'_,H> {
    fn alloc_at_m(&mut self,_:u32,_:u32,_:&mut[u8])->u32 { panic!("unexpected allocation in release") }
    fn realloc_at_m(&mut self,_:u32,_:u32,_:u32,_:&mut[u8])->u32 { panic!("unexpected reallocation in release") }
    fn release_at_m(&mut self,object:u32,pointer:u32,bytes:&mut[u8]) {
        self.host.call(&mut Memory{base:self.base,bytes},self.method,&[object,pointer]);
    }
}
/// fn_196964. Conditional release with reversed pointer/heap arguments.
pub fn fn_196964(m: &mut Memory<'_>, host: &mut impl Host, pointer: u32, heap: u32) {
    if pointer == 0 || heap == 0 { return; }
    let method = m.word(m.word(heap) + 12);
    let mut release = Release{host,base:m.base,method};
    crate::frontend::heap_release_m(m.bytes, Some(heap - m.base), Some(pointer), &mut release);
}

struct NoDivisionTrap;
impl crate::boot_part_c::BootCHost for NoDivisionTrap {
    fn invoke(&mut self,_:&mut Memory<'_>,f:u32,_:&[u32])->[u32;3] {
        panic!("unexpected arithmetic trap {f:x}")
    }
}
fn continuation(m:&mut Memory<'_>,entry:crate::boot_part_c::DivisionEntry,n:u32,d:u32,q:u32,carry:bool)->(u32,u32) {
    crate::boot_part_c::signed_divide_continue(m,&mut NoDivisionTrap,entry,
        crate::boot_part_c::DivisionContinuation{remainder:n,divisor:d,quotient:q,signs:0,carry})
}
/// fn_3020f4. Internal label, not a C ABI: preserve live r0,r1,r2 and carry.
pub fn fn_3020f4(m:&mut Memory<'_>,n:u32,d:u32,q:u32,carry:bool)->(u32,u32) {
    continuation(m,crate::boot_part_c::DivisionEntry::Rescale8,n,d,q,carry)
}
/// fn_3020f8. Internal label consuming live partial division registers.
pub fn fn_3020f8(m:&mut Memory<'_>,n:u32,d:u32,q:u32,carry:bool)->(u32,u32) {
    continuation(m,crate::boot_part_c::DivisionEntry::Bits15,n,d,q,carry)
}
/// fn_302128. Internal label consuming live partial division registers.
pub fn fn_302128(m:&mut Memory<'_>,n:u32,d:u32,q:u32,carry:bool)->(u32,u32) {
    continuation(m,crate::boot_part_c::DivisionEntry::Bits11,n,d,q,carry)
}
/// fn_30215c. Internal label consuming live partial division registers.
pub fn fn_30215c(m:&mut Memory<'_>,n:u32,d:u32,q:u32,carry:bool)->(u32,u32) {
    continuation(m,crate::boot_part_c::DivisionEntry::Bits7,n,d,q,carry)
}
/// fn_302180. Internal label consuming live partial division registers.
pub fn fn_302180(m:&mut Memory<'_>,n:u32,d:u32,q:u32,carry:bool)->(u32,u32) {
    continuation(m,crate::boot_part_c::DivisionEntry::Bits4,n,d,q,carry)
}
/// fn_30218c. Internal label consuming live partial division registers.
pub fn fn_30218c(m:&mut Memory<'_>,n:u32,d:u32,q:u32,carry:bool)->(u32,u32) {
    continuation(m,crate::boot_part_c::DivisionEntry::Bits3,n,d,q,carry)
}

/// fn_100278. Byte-string printf conversion, including precision and alias order.
pub fn fn_100278(m: &mut Memory<'_>, host: &mut impl Host, state: u32, mut text: u32, mut count: u32) {
    let mut len = 1u32;
    if count != 1 {
        if m.word(state) & 32 != 0 { count = m.word(state + 28); }
        len = 0;
        while len < count && m.byte(text.wrapping_add(len)) != 0 { len += 1; }
    }
    let end = text.wrapping_add(len);
    m.put_word(state + 24, m.word(state + 24).wrapping_sub(len));
    m.put_word(state + 32, m.word(state + 32).wrapping_add(len));
    fn_2ffc14(m, host, state);
    while text < end {
        let byte = m.byte(text) as u32;
        text = text.wrapping_add(1);
        emit(m, host, state, byte);
    }
    fn_2ffc40(m, host, state);
}
/// fn_2ffcfc. Emit sign/prefix, precision zeros, reverse-order digits and padding.
pub fn fn_2ffcfc(m: &mut Memory<'_>, host: &mut impl Host, state: u32, mut digits: u32, prefix: u32, prefix_len: u32) -> u32 {
    let precision = if m.word(state) & 32 != 0 {
        m.put_word(state, m.word(state) & !16);
        m.word(state + 28) as i32
    } else { 1 };
    let zeros = if precision > digits as i32 { precision.wrapping_sub(digits as i32) } else { 0 };
    let total = (zeros as u32).wrapping_add(digits).wrapping_add(prefix_len);
    m.put_word(state + 24, m.word(state + 24).wrapping_sub(total));
    if m.word(state) & 16 == 0 { fn_2ffc14(m, host, state); }
    let mut i = 0u32;
    while (i as i32) < prefix_len as i32 {
        let byte = m.byte(prefix.wrapping_add(i)) as u32;
        emit(m, host, state, byte);
        m.put_word(state + 32, m.word(state + 32).wrapping_add(1));
        i = i.wrapping_add(1);
    }
    if m.word(state) & 16 != 0 { fn_2ffc14(m, host, state); }
    for _ in 0..zeros.max(0) {
        emit(m, host, state, 48);
        m.put_word(state + 32, m.word(state + 32).wrapping_add(1));
    }
    while (digits as i32) > 0 {
        digits = digits.wrapping_sub(1);
        let byte = m.byte(state.wrapping_add(36).wrapping_add(digits)) as u32;
        emit(m, host, state, byte);
        m.put_word(state + 32, m.word(state + 32).wrapping_add(1));
    }
    fn_2ffc40(m, host, state);
    if m.word(state) & 128 != 0 { 2 } else { 1 }
}
/// fn_129778. Aligned allocator wrapper. fill_flag is the active regional global byte.
pub fn fn_129778(m: &mut Memory<'_>, host: &mut impl Host, size: u32, fill_flag: u32) -> u32 {
    let table=host.heap_table(m);
    crate::speech_part_1::fn_11db30(m,size,0,32,fill_flag as u8 as u32,table,&mut HeapBinding(host))
}
struct Conversion<'a,H>(&'a mut H);
impl<H:Host> Host for Conversion<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 { self.0.call(m,f,a) }
    fn decimal_point(&mut self,m:&mut Memory<'_>)->u8 { self.0.decimal_point(m) }
}
impl<H:Host> crate::render_part_d2::LocaleHost for Conversion<'_,H> {
    fn locale_global(&mut self,m:&mut Memory<'_>)->u32 { self.0.runtime_locale(m) }
    fn decode(&mut self,m:&mut Memory<'_>,f:u32,d:u32,b:u32,s:u32)->u32 { self.0.call(m,f,&[d,b,s]) }
}
fn conversion(m:&mut Memory<'_>,h:&mut impl Host,f:u32,s:u32,c:u32,a:u32)->u32 {
    let scratch=h.runtime_scratch(m);
    crate::speech_part_6::replay_format(m,&mut Conversion(h),f,s,c,a,scratch)
}
struct HeapBinding<'a,H>(&'a mut H);
impl<H:Host> crate::speech_part_1::Host for HeapBinding<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
        if f==0x184f84 { crate::boot_part_b2::guest_184f84(m,a[0],a[1]);0 }
        else {self.0.call(m,f,a)}
    }
    fn get(&mut self,_:&mut Memory<'_>,_:u32,_:u32,_:u32,_:u32)->(u32,u32,u32) {
        panic!("unexpected stream call in heap binding")
    }
}
impl<H:Host> crate::speech_part_1::HeapHost for HeapBinding<'_,H> {
    fn thread_id(&mut self)->u32 {self.0.heap_thread_id()}
    fn arbiter_address(&self)->u32 {self.0.heap_arbiter_address()}
}
/// fn_103c2c. ARM conversion router bound to the real speech_part_6 entries.
pub fn fn_103c2c(m: &mut Memory<'_>, host: &mut impl Host, state: u32, mut spec: u32, args: u32) -> u32 {
    let direct = match spec { 110 => Some(0x100418), 112 => Some(0x1004c0), 102 | 101 | 103 => Some(0x1a7970), 97 => Some(0x1a7974), _ => None };
    if let Some(f) = direct { return conversion(m, host, f, state, spec, args); }
    if m.word(state) & 128 != 0 { spec |= 128; }
    let integer = match spec {
        0x69 | 0x64 | 0x75 => Some(0x100694), 0x6f => Some(0x100448), 0x78 => Some(0x4a7aa4),
        0xe9 | 0xe4 | 0xf5 => Some(0x10035c), 0xef => Some(0x4a7a9c), 0xf8 => Some(0x4a7a94), _ => None,
    };
    if let Some(f) = integer { return conversion(m, host, f, state, spec, args); }
    if m.word(state) & 64 != 0 { spec |= 128; }
    let target = match spec { 0x63 => 0x1007fc, 0x73 => 0x4a7a8c, 0xe3 => 0x100834, 0xf3 => 0x4a7a84, _ => return 0 };
    conversion(m, host, target, state, spec, args)
}
/// Caller-resolved US talkmodachi patch globals. Regional patches are not supplied.
#[derive(Clone,Copy)]
pub struct CaptureRuntime {
    pub slot: u32,
    pub fill_flag: u32,
    pub append_result: u32,
}
/// fn_c726f4. Forward byte copy, including the patch's overlapping-copy behavior.
/// wire-a r48: ARM no-null-check ABI. At exhaustion tmalloc returns NULL;
/// citra reads zeros through NULL and drops NULL writes, continuing to a
/// genuine short completion. Guest 0 is never a legitimate pointer, so:
/// writes to 0 are dropped (arena[0..] is never touched) and reads from 0
/// yield zeros. Bounded and fail-soft like memcpy_at; alloc-free.
pub fn fn_c726f4(m: &mut Memory<'_>, dst:u32, src:u32, count:u32) -> u32 {
    if count != 0 {
        if dst == 0 || src == 0 {
            if dst != 0 {
                let base = m.base as usize;
                let mut i = 0u32;
                while i != count {
                    if let Some(slot) = m.bytes.get_mut((dst as usize).wrapping_sub(base).wrapping_add(i as usize)) {
                        *slot = 0;
                    }
                    i = i.wrapping_add(1);
                }
            }
        } else {
            crate::normalization::memcpy_at(m.bytes,dst-m.base,src-m.base,count);
        }
    }
    dst
}
/// fn_c72690. Patch allocator veneer, bound to this module's real wrapper.
pub fn fn_c72690(m: &mut Memory<'_>, host:&mut impl Host, size:u32, fill_flag:u32)->u32 {
    fn_129778(m,host,size,fill_flag)
}
/// Guest code starts here. Nothing is mapped below it, so citra drops writes
/// through a NULL buffer until they reach this address.
const CODE_BASE: u32 = 0x100000;
/// fn_c726a8. Allocate/copy/free patch realloc, with the original no-null-check ABI.
/// At exhaustion the allocation is NULL and the copy writes guest
/// [0, old_size). Past CODE_BASE that overwrites the game's code, tfree
/// (0x11da30) included, which runs next and executes PCM: the guest never
/// finishes the request (citra times out, restarts and retries), so the
/// copy is reported as a fault instead.
pub fn fn_c726a8(m: &mut Memory<'_>, host:&mut impl Host, old:u32, old_size:u32, new_size:u32, fill_flag:u32)->u32 {
    let new = fn_129778(m,host,new_size,fill_flag);
    if new == 0 { host.heap_exhausted(new_size); }
    if new == 0 && old_size > CODE_BASE {
        host.code_overwritten(old_size);
        return new;
    }
    fn_c726f4(m,new,old,old_size);
    let table=host.heap_table(m);
    let optional=host.heap_optional_flag(m);
    crate::speech_part_1::fn_11da30(m,old,table,optional,&mut HeapBinding(host));
    new
}
/// fn_c725bc. Append captured PCM bytes, rereading the global after allocator calls.
pub fn fn_c725bc(m:&mut Memory<'_>,host:&mut impl Host,env:CaptureRuntime,source:u32,count:u32)->u32 {
    if count != 0 {
        let mut state=m.word(env.slot);
        let mut buffer=m.word(state+16);
        let mut len;
        if buffer==0 {
            let capacity=count.wrapping_shl(2);
            m.put_word(state+20,capacity);
            buffer=fn_c72690(m,host,capacity,env.fill_flag);
            if buffer == 0 { host.heap_exhausted(capacity); }
            m.put_word(state+16,buffer);
            state=m.word(env.slot);
            m.put_word(state+12,0);
            len=0;
        } else { len=m.word(state+12); }
        let needed=len.wrapping_add(count);
        let mut capacity=m.word(state+20);
        if (capacity as i32)<(needed as i32) {
            loop {
                capacity=capacity.wrapping_shl(1);
                if (capacity as i32)>=(needed as i32) { break; }
            }
            m.put_word(state+20,capacity);
            let old=m.word(state+16);
            buffer=fn_c726a8(m,host,old,len,capacity,env.fill_flag);
            m.put_word(state+16,buffer);
            state=m.word(env.slot);
            len=m.word(state+12);
        }
        // wire-a r48: if the buffer is still NULL (even the small retry
        // failed at extreme exhaustion), skip the append without advancing
        // the length. The ARM writes the chunk to guest `len` (benign low
        // memory on citra); the port arena maps that address to real data,
        // so writing would corrupt the image. Skipping keeps a clean short.
        if m.word(state+16) == 0 {
            // that write reaching guest code crashes it, as in fn_c726a8
            let end=len.wrapping_add(count);
            if end > CODE_BASE { host.code_overwritten(end); }
            return env.append_result;
        }
        let dst=m.word(state+16).wrapping_add(len);
        fn_c726f4(m,dst,source,count);
        state=m.word(env.slot);
        m.put_word(state+12,m.word(state+12).wrapping_add(count));
    }
    env.append_result
}
/// fn_c72018. Register-preserving capture hook. Only guest memory is changed.
pub fn fn_c72018(m:&mut Memory<'_>,host:&mut impl Host,env:CaptureRuntime,source:u32,count:u32)->(u32,u32) {
    fn_c725bc(m,host,env,source,count);
    (source,count)
}
