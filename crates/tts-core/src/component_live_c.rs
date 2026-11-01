

use crate::boot::Memory;

pub trait Host {
    fn call_m(&mut self, entry: u32, args: &[u32], memory: &mut Memory<'_>) -> u32;
    fn thread_id_m(&mut self)->u32{panic!("thread ID platform service is not bound")}
    fn rom(&self)->crate::rom_region::Rom{crate::rom_region::Rom::Us}
}

pub struct Calls<'a, C: crate::boot::Calls>(pub &'a mut C);
impl<C: crate::boot::Calls> Host for Calls<'_, C> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.call(m, entry, args)
    }
}

pub fn core_call_m(m:&mut Memory<'_>,entry:u32,args:&[u32])->Result<u32,crate::engine::Error> {
    core_call_m_rom(m,entry,args,crate::rom_region::Rom::Us)
}

pub fn core_call_m_rom(m:&mut Memory<'_>,entry:u32,args:&[u32],rom:crate::rom_region::Rom)->Result<u32,crate::engine::Error> {
    let descriptors=crate::rom_region::addr(rom,0x8d7ca0);
    if entry==0x18f488{return Ok(crate::frontend::finish_request());}
    if entry==0x1ce56c{return Ok(crate::frontend::streaming_mode((args[0]!=0).then(||(m.byte(m.word(args[0])+4),m.byte(m.word(args[0])+0x10c)))));}
    if entry==0x219554 {return Ok(fn_219554_m(m,args[0],args[1],args[2],args[3]));}
    if entry==0x21958c {
        let check=crate::normalization::quad_check(args[0],args[1],0xf37e,64);
        if check>=0x80000000{return Ok(0x89702008);}
        struct Validator{p:u32}
        impl crate::depes::HandleValidator for Validator{
            fn validate(&mut self,tag:u32,expected:u32,size:u32)->u32{crate::normalization::quad_check(self.p,tag,expected,size)}
        }
        let opts=crate::depes::ModuleOptions{first:m.word(args[0]+56)as i32,second:m.word(args[0]+60)as i32};
        let mut value=0;let rc=opts.get(args[1],&mut value,args[3]as i32,&mut Validator{p:args[0]});
        m.put_word(args[2],value as u32);return Ok(rc);
    }
    if [0x15c4b0,0x15c744,0x15c9a4,0x15c7d8,0x15ca38].contains(&entry){return record_link_m(m,entry,args.try_into().expect("record-link ABI"));}
    if entry==0x15d6c4{return enquire_child_m(m,args.try_into().expect("enquire-child ABI"),rom);}
    if entry==0x15d378 {return field_data_m(m,args.try_into().expect("field-data ABI"),descriptors);}
    let mut regions=[crate::engine::Region{base:m.base,bytes:m.bytes}];
    let mut sparse=crate::engine::Memory::new(&mut regions)?;
    if entry==0x15d594 {
        return crate::bridge::fn_15d594(&mut sparse,args.try_into().expect("field-state ABI"),descriptors);
    }
    use crate::engine_components::InternalCalls;
    if [0x15140c,0x151294,0x14fe9c,0x15017c,0x150878,0x150c0c,0x14f918].contains(&entry){
        return crate::engine::LiveComponentCalls.call(&mut sparse,entry,args);
    }
    crate::engine_components::CoreCalls.call(&mut sparse,entry,args)
}

pub fn heap_call_m(m:&mut Memory<'_>,entry:u32,args:&[u32],platform:&mut dyn crate::engine::HeapPlatform)->Result<u32,crate::engine::Error>{
    let mut host=crate::engine::SpeechBootHost{platform,fault:None};
    let result=HeapExtended(&mut host).dispatch(m,entry,args);
    if let Some(error)=host.fault{Err(error)}else{Ok(result)}
}

pub fn heap_call_m_rom(m:&mut Memory<'_>,entry:u32,args:&[u32],platform:&mut dyn crate::engine::HeapPlatform,rom:crate::rom_region::Rom)->Result<u32,crate::engine::Error>{
    let mut host=crate::engine::SpeechBootHost{platform,fault:None};
    let result=HeapExtendedRom(&mut host,rom).dispatch(m,entry,args);
    if let Some(error)=host.fault{Err(error)}else{Ok(result)}
}
struct HeapExtendedRom<'a,'p>(&'a mut crate::engine::SpeechBootHost<'p>,crate::rom_region::Rom);
impl HeapExtendedRom<'_, '_>{
 fn dispatch(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{
  let f=crate::rom_region::to_us(self.1,f);
  match f{
   0x1963e0=>crate::boot_part_e::fn_1963e0(m,a[0],a[1],a[2],self),
   0x4a79b8=>crate::speech_part_3::fn_4a79b8(m,a[0],a[1],a[2],self),
   _=>self.0.invoke(m,f,a),
  }
 }
}
impl crate::boot_part_e::Host for HeapExtendedRom<'_, '_>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.dispatch(m,f,a)}
}
impl crate::speech_part_3::Host for HeapExtendedRom<'_, '_>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.dispatch(m,f,a)}
}
impl crate::boot_part_b::Host for HeapExtendedRom<'_, '_>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.dispatch(m,f,a)}
 fn thread_id(&mut self)->u32{crate::boot_part_b::Host::thread_id(self.0)}
}
struct HeapExtended<'a,'p>(&'a mut crate::engine::SpeechBootHost<'p>);
impl HeapExtended<'_, '_>{
 fn dispatch(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{
  match f{
   0x1963e0=>crate::boot_part_e::fn_1963e0(m,a[0],a[1],a[2],self),
   0x4a79b8=>crate::speech_part_3::fn_4a79b8(m,a[0],a[1],a[2],self),
   _=>self.0.invoke(m,f,a),
  }
 }
}
impl crate::boot_part_e::Host for HeapExtended<'_, '_>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.dispatch(m,f,a)}
}
impl crate::speech_part_3::Host for HeapExtended<'_, '_>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.dispatch(m,f,a)}
}
impl crate::boot_part_b::Host for HeapExtended<'_, '_>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.dispatch(m,f,a)}
 fn thread_id(&mut self)->u32{crate::boot_part_b::Host::thread_id(self.0)}
}

pub fn field_data_m(m:&mut Memory<'_>,a:[u32;6],descriptors:u32)->Result<u32,crate::engine::Error>{
    use crate::linguistic::{Field,FieldDatabase,Record};
    if crate::normalization::quad_check(a[0],a[1],0xb2c2,0x38)>=0x80000000{return Ok(0x89202008);}
    let missing_log=crate::engine::Error::UnsupportedStage{entry:0x173dc0};
    if a[2]>=m.word(a[0]+32){return Err(missing_log);}
    let record=m.word(m.word(a[0]+48)+a[2]*4);
    if record==0{return Err(missing_log);}
    let kind=m.byte(record+2);let desc=descriptors+kind as u32*272;
    let nfields=u16::from_le_bytes(m.bytes[m.offset(desc+4)..m.offset(desc+4)+2].try_into().unwrap())as u32;
    if kind==0 || a[3]>=nfields{return Err(missing_log);}
    let f=desc+12+a[3]*20;let base=m.word(desc);
    let loc=record.wrapping_add(m.word(f).wrapping_sub(base));
    let length=m.word(f+4);
    let fixed=u16::from_le_bytes(m.bytes[m.offset(f+8)..m.offset(f+8)+2].try_into().unwrap());
    let target=if fixed==0{m.word(loc)}else{loc};
    let state=m.byte(record.wrapping_add(m.word(desc+8).wrapping_sub(base))+a[3]);
    let count=if length==0{None}else{
        let at=m.offset(record.wrapping_add(length.wrapping_sub(base)));
        Some(u16::from_le_bytes(m.bytes[at..at+2].try_into().unwrap()))
    };
    let records=[Some(Record{kind,..Default::default()})];
    let fields=[Field{data:if target==0{None}else{Some(&m.bytes[m.offset(target)..])},state,fixed_count:fixed,count,element_size:0}];
    let rows:[&[Field<'_>];1]=[&fields];
    let db=FieldDatabase{records:&records,fields:&rows,valid:true};
    let mut output=None;let mut count=0;
    let rc=db.field_data(0,0,&mut output,&mut count);
    if rc>=0x80000000{return Err(missing_log);}
    m.put_word(a[4],target);m.put_half(a[5],count);Ok(rc)
}

struct SpellingCalls<'a, H>(&'a mut H);
impl<H: Host> crate::speech_part_2::Host for SpellingCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        if entry == 0x219554 {
            fn_219554_m(m, args[0], args[1], args[2], args[3])
        } else {
            self.0.call_m(entry, args, m)
        }
    }
}

pub fn fn_219c8c_m(
    m: &mut Memory<'_>, p: u32, tag: u32, input: [u32; 2],
    e: &crate::speech_part_2::Environment, h: &mut impl Host,
) -> u32 {
    crate::speech_part_2::fn_219c8c(m, p, tag, input, e, &mut SpellingCalls(h))
}

fn half_at(m: &Memory<'_>, at: u32) -> u16 {
    let o = m.offset(at);
    u16::from_le_bytes(m.bytes[o..o + 2].try_into().unwrap())
}

const STRING_KINDS: [u16; 12] = [6, 12, 21, 50, 36, 39, 37, 34, 22, 40, 26, 29];

fn heap_release_m(m: &mut Memory<'_>, heap: u32, object: u32, h: &mut impl Host) {
    if heap == 0 || object == 0 {
        return;
    }
    let obj = m.word(heap + 4);
    let f = m.word(m.word(heap) + 12);
    h.call_m(f, &[obj, object], m);
}

fn heap_realloc_m(m: &mut Memory<'_>, heap: u32, old: u32, size: u32, h: &mut impl Host) -> u32 {
    if heap == 0 || size == 0 {
        return 0;
    }
    let obj = m.word(heap + 4);
    if old != 0 {
        let f = m.word(m.word(heap) + 8);
        h.call_m(f, &[obj, old, size], m)
    } else {
        let f = m.word(m.word(heap));
        h.call_m(f, &[obj, size], m)
    }
}

pub fn release_typed_buffer_m(m: &mut Memory<'_>, a: &[u32], h: &mut impl Host) {
    let (p, heap) = (a[0], a[1]);
    if p == 0 || !STRING_KINDS.contains(&half_at(m, p)) {
        return;
    }
    let buffer = m.word(p + 20);
    if buffer == 0 {
        return;
    }
    heap_release_m(m, heap, buffer, h);
    m.put_word(p + 20, 0);
}

fn compact_stream_buffer_m(m: &mut Memory<'_>, state: u32, required: u32, h: &mut impl Host) {
    let buffer = m.word(state + 60);
    if buffer == 0 || m.byte(state + 40) != 0 {
        return;
    }
    let write = m.word(state + 44);
    let mut minimum = write;
    let readers = m.word(state + 12);
    if readers != 0 {
        let base = m.word(readers);
        let size = half_at(m, readers + 4) as u32;
        let count = m.word(readers + 20);
        let mut index = 0;
        while index < count {
            let reader = m.word(base.wrapping_add(index.wrapping_mul(size)));
            let position = m.word(reader.wrapping_add(8));
            if position < minimum {
                minimum = position;
            }
            index += 1;
        }
    }
    let old = m.word(state + 56);
    let stride = half_at(m, state + 28) as u32;
    let mut discarded = minimum.wrapping_sub(old);
    if stride > 1 {
        discarded -= discarded % stride;
    }
    let destroy = m.word(state + 36);
    if destroy != 0 && discarded != 0 {
        let heap = m.word(m.word(state + 4) + 4);
        let end = buffer.wrapping_add(discarded);
        let mut element = m.word(state + 52).wrapping_sub(old).wrapping_add(buffer);
        let mut position = m.word(state + 52);
        while element < end {
            h.call_m(destroy, &[element, heap], m);
            element = element.wrapping_add(stride);
            position = position.wrapping_add(stride);
        }
        m.put_word(state + 52, position);
    }
    let config = m.word(state);
    let mut shrink = false;
    if required == 0 {
        if m.word(config + 24) <= discarded {
            shrink = true;
        } else {
            return;
        }
    } else if discarded < required && discarded < m.word(config + 20) {
        return;
    }
    let remaining = write.wrapping_sub(minimum);
    if remaining != 0 {
        crate::frontend::move_bytes(m.bytes, buffer, buffer.wrapping_add(discarded), remaining);
    }
    m.put_word(state + 56, old.wrapping_add(discarded));
    if shrink {
        let capacity = m.word(state + 64);
        let heap = m.word(m.word(state + 4) + 4);
        if capacity == discarded || write == minimum {
            heap_release_m(m, heap, buffer, h);
            m.put_word(state + 60, 0);
            m.put_word(state + 64, 0);
        } else {
            let newbuf = heap_realloc_m(m, heap, buffer, capacity.wrapping_sub(discarded), h);
            if newbuf != 0 {
                m.put_word(state + 60, newbuf);
                m.put_word(state + 64, capacity.wrapping_sub(discarded));
            }
        }
    }
}

pub fn fn_15017c_m(m: &mut Memory<'_>, a: &[u32], h: &mut impl Host) -> u32 {
    let rc = crate::frontend::validate_handle(a[0] != 0, a[1], 0x47de, 12);
    if rc >= 0x80000000 {
        return rc;
    }
    let base = a[0];
    if m.byte(base) != 0 {
        return 0x88f02007;
    }
    let state = m.word(base + 4);
    let position = m.word(base + 8);
    let available = m.word(state + 44).wrapping_sub(position);
    let (position, rc) = if a[2] > available {
        (position.wrapping_add(available), 0x88f02007)
    } else {
        (position.wrapping_add(a[2]), rc)
    };
    m.put_word(base + 8, position);
    if position > m.word(state + 48) {
        m.put_word(state + 48, position);
    }
    compact_stream_buffer_m(m, state, 0, h);
    rc
}

pub fn fn_1fe040_m(
    m: &mut Memory<'_>, p: u32, tag: u32, scratch: u32, h: &mut impl Host,
) -> u32 {
    let rc = crate::normalization::quad_check(p, tag, 0x11d, 0x48);
    if rc >= 0x80000000 { return rc; }
    let saved = m.word(scratch);
    m.put_word(scratch, 0);
    let rc = h.call_m(0x19611c, &[m.word(p), m.word(p + 4), scratch], m);
    m.put_word(scratch, saved);
    if rc >= 0x80000000 { return rc; }
    let count = h.call_m(0x1970a0, &[m.word(p + 0x40)], m);
    m.put_word(p + 0x38, count);
    rc
}

pub fn fn_200e2c_m(
    m: &mut Memory<'_>, p: u32, tag: u32, scratch: u32, h: &mut impl Host,
) -> u32 {
    if crate::normalization::quad_check(p, tag, 0xf383, 0x20) >= 0x80000000 {
        return 0x8bb02008;
    }
    let target = crate::rom_region::to_us_guest(h.rom(), m.word(m.word(p + 8) + 28));
    let a = m.word(p + 12);
    let b = m.word(p + 16);
    if target == 0x1fe040 { fn_1fe040_m(m, a, b, scratch, h) }
    else { h.call_m(target, &[a, b], m) }
}

pub fn fn_219554_m(m: &mut Memory<'_>, p: u32, tag: u32, value: u32, which: u32) -> u32 {
    struct Validator { p: u32 }
    impl crate::depes::HandleValidator for Validator {
        fn validate(&mut self, tag: u32, expected: u32, size: u32) -> u32 {
            crate::normalization::quad_check(self.p, tag, expected, size)
        }
    }
    let mut options = crate::depes::ModuleOptions::default();
    let rc = options.set(tag, value as i32, which as i32, &mut Validator { p });
    if rc < 0x80000000 {
        m.put_word(p + if which == 1 { 56 } else { 60 }, value);
    }
    rc
}

use crate::frontend::{strlen_bytes,copy_str,compare_str};
fn mem_word(m:&[u8],p:u32)->u32{u32::from_le_bytes(m[p as usize..p as usize+4].try_into().unwrap())}
fn mem_store(m:&mut[u8],p:u32,v:u32){m[p as usize..p as usize+4].copy_from_slice(&v.to_le_bytes());}
fn mem_store16(m:&mut[u8],p:u32,v:u16){m[p as usize..p as usize+2].copy_from_slice(&v.to_le_bytes());}

pub trait PromptOrthHooksM {

    fn check_m(&mut self, p: u32, check: u32, f1: u32, f2: u32, memory: &mut [u8]) -> u32;

    fn find_m(&mut self, h0: u32, h1: u32, f1: u32, f2: u32, id: &mut u16, memory: &mut [u8]) -> u32;

    fn data_type_m(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        ty: &mut u16, memory: &mut [u8]) -> u32;

    fn get_m(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        ptr: &mut u32,
        len: &mut u16, memory: &mut [u8]) -> u32;

    #[allow(clippy::too_many_arguments)]
    fn set_m(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        count: u32,
        ptr: u32,
        written: &mut u16, memory: &mut [u8]) -> u32;

    #[allow(clippy::too_many_arguments)]
    fn process_m(
        &mut self,
        t0: u32,
        t1: u32,
        orth: &mut Option<u32>,
        records: &mut Option<u32>,
        count: &mut u16,
        h0: u32,
        h1: u32,
        delta: &mut i16, memory: &mut [u8]) -> u32;

    fn get_position_m(&mut self, h0: u32, h1: u32, pos: &mut u32, memory: &mut [u8]) -> u32;

    fn set_position_m(&mut self, h0: u32, h1: u32, pos: u32, memory: &mut [u8]) -> u32;

    fn alloc_orth_m(&mut self, heap: u32, size: u32, memory: &mut [u8]) -> Option<u32>;

    fn alloc_records_m(&mut self, heap: u32, size: u32, memory: &mut [u8]) -> Option<u32>;

    fn free_buf_m(&mut self, heap: u32, ptr: u32, memory: &mut [u8]);

    fn log_no_memory_m(&mut self, log: u32, code: u32, flag: u32, memory: &mut [u8]);
}

pub fn fn_200e64_m<H: PromptOrthHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    p: u32,
    check: u32,
    h0: u32,
    h1: u32,
    active: u32,
) -> u32 {
    let mut result = hooks.check_m(p, check, 0xf383, 0x20, mem);
    if result >= 0x80000000 {
        return 0x8bb02008;
    }
    mem[active as usize] = 1;
    let ctx = mem_word(mem, p);
    let heap = mem_word(mem, ctx.wrapping_add(4));
    let log = mem_word(mem, ctx.wrapping_add(16));
    let mut id = 0u16;
    result = hooks.find_m(h0, h1, 1, 0, &mut id, mem);
    if result >= 0x80000000 {
        return result;
    }
    let mut ty = 0u16;
    let mut source_orth = 0u32;
    let mut source_rec = 0u32;
    let mut length = 0u16;
    let mut have_source = false;
    if hooks.data_type_m(h0, h1, id, 0, &mut ty, mem) < 0x80000000 && ty == 1 {
        result = hooks.get_m(h0, h1, id, 0, &mut source_orth, &mut length, mem);
        if result >= 0x80000000 {
            return result;
        }
        if length > 1 {
            have_source = true;
        } else if hooks.data_type_m(h0, h1, id, 1, &mut ty, mem) >= 0x80000000
            || ty != 1
        {
            return result;
        }
    } else if hooks.data_type_m(h0, h1, id, 1, &mut ty, mem) >= 0x80000000
        || ty != 1
    {
        return result;
    }
    result = hooks.get_m(h0, h1, id, 1, &mut source_rec, &mut length, mem);
    if result >= 0x80000000 {
        return result;
    }
    let mut count = length;

    let size = if have_source {
        strlen_bytes(&mem[source_orth as usize..]).wrapping_add(1)
    } else {
        1
    };
    let Some(buf) = hooks.alloc_orth_m(heap, size, mem) else {
        hooks.log_no_memory_m(log, 0xafc8, 0, mem);
        return 0x8bb0200a;
    };
    if have_source {
        copy_str(mem, buf, source_orth);
    } else {
        mem[buf as usize] = 0;
    }
    let mut orth = Some(buf);
    let bytes = (count as u32).wrapping_mul(24);
    let Some(rec) = hooks.alloc_records_m(heap, bytes, mem) else {
        hooks.log_no_memory_m(log, 0xafc8, 0, mem);
        hooks.free_buf_m(heap, buf, mem);
        return 0x8bb0200a;
    };
    if bytes != 0 {
        mem.copy_within(
            source_rec as usize..(source_rec.wrapping_add(bytes)) as usize,
            rec as usize,
        );
    }
    let mut records = Some(rec);
    let mut delta = 0i16;
    result = hooks.process_m(
        mem_word(mem, p.wrapping_add(12)),
        mem_word(mem, p.wrapping_add(16)),
        &mut orth,
        &mut records,
        &mut count,
        h0,
        h1,
        &mut delta, mem);
    if result >= 0x80000000 {
        if let Some(buf) = orth {
            hooks.free_buf_m(heap, buf, mem);
        }
        if let Some(rec) = records {
            hooks.free_buf_m(heap, rec, mem);
        }
        return result;
    }
    let mut written = 0u16;
    if let Some(buf) = orth {
        if compare_str(&mem[buf as usize..], b"\0") != 0 {
            let n = strlen_bytes(&mem[buf as usize..]).wrapping_add(1) as u16 as u32;
            result = hooks.set_m(h0, h1, id, 0, n, buf, &mut written, mem);
            if result >= 0x80000000 {
                hooks.free_buf_m(heap, buf, mem);
                if let Some(rec) = records {
                    hooks.free_buf_m(heap, rec, mem);
                }
                return result;
            }
        }
    }
    if let Some(rec) = records {
        if count != 0 {
            let pos = mem_word(mem, rec.wrapping_add(16));
            mem_store(mem, rec.wrapping_add(16), pos.wrapping_add(delta as u32));
            result = hooks.set_m(
                h0,
                h1,
                id,
                1,
                count as u32,
                rec,
                &mut written, mem);
            if result >= 0x80000000 {
                if let Some(buf) = orth {
                    hooks.free_buf_m(heap, buf, mem);
                }
                hooks.free_buf_m(heap, rec, mem);
                return result;
            }
        }
    }
    let mut position = 0u32;
    result = hooks.get_position_m(
        mem_word(mem, p.wrapping_add(24)),
        mem_word(mem, p.wrapping_add(28)),
        &mut position, mem);
    if result >= 0x80000000 {
        if let Some(buf) = orth {
            hooks.free_buf_m(heap, buf, mem);
        }
        if let Some(rec) = records {
            hooks.free_buf_m(heap, rec, mem);
        }
        return result;
    }
    position = position.wrapping_add(delta as u32);
    result = hooks.set_position_m(
        mem_word(mem, p.wrapping_add(24)),
        mem_word(mem, p.wrapping_add(28)),
        position, mem);
    if let Some(buf) = orth {
        hooks.free_buf_m(heap, buf, mem);
    }
    if let Some(rec) = records {
        hooks.free_buf_m(heap, rec, mem);
    }
    result
}

pub fn prompt_orth_live_m(
    m: &mut Memory<'_>, p:u32, tag:u32, input:[u32;2], active:u32,
    scratch:u32, h:&mut impl Host,
)->u32 {
    assert_eq!(m.base,0);
    let at=scratch as usize;
    let mut saved=[0;16];saved.copy_from_slice(&m.bytes[at..at+16]);
    let rc=fn_200e64_m(&mut OrthLive{p,s:scratch,h},m.bytes,p,tag,input[0],input[1],active);
    m.bytes[at..at+16].copy_from_slice(&saved);rc
}
struct OrthLive<'a,H>{p:u32,s:u32,h:&'a mut H}
impl<H:Host> OrthLive<'_,H>{
 fn invoke(&mut self,m:&mut[u8],f:u32,a:&[u32])->u32{self.h.call_m(f,a,&mut Memory{base:0,bytes:m})}
 fn method(&mut self,m:&mut[u8],table:u32,slot:u32,a:&[u32])->u32{
  let f=mem_word(m,mem_word(m,self.p+table)+slot);self.invoke(m,f,a)
 }
}
impl<H:Host> PromptOrthHooksM for OrthLive<'_,H> {
 fn check_m(&mut self,p:u32,tag:u32,expected:u32,size:u32,_:&mut[u8])->u32{
  crate::normalization::quad_check(p,tag,expected,size)
 }
 fn find_m(&mut self,a:u32,b:u32,kind:u32,root:u32,out:&mut u16,m:&mut[u8])->u32{
  mem_store16(m,self.s,*out);let rc=self.method(m,4,52,&[a,b,kind,root,self.s]);*out=mem_word(m,self.s)as u16;rc
 }
 fn data_type_m(&mut self,a:u32,b:u32,id:u16,flag:u32,out:&mut u16,m:&mut[u8])->u32{
  mem_store16(m,self.s,*out);let rc=self.method(m,4,92,&[a,b,id as u32,flag,self.s]);*out=mem_word(m,self.s)as u16;rc
 }
 fn get_m(&mut self,a:u32,b:u32,id:u16,flag:u32,out:&mut u32,len:&mut u16,m:&mut[u8])->u32{
  mem_store(m,self.s,*out);mem_store16(m,self.s+4,*len);
  let rc=self.method(m,4,88,&[a,b,id as u32,flag,self.s,self.s+4]);
  *out=mem_word(m,self.s);*len=mem_word(m,self.s+4)as u16;rc
 }
 fn set_m(&mut self,a:u32,b:u32,id:u16,flag:u32,count:u32,ptr:u32,out:&mut u16,m:&mut[u8])->u32{
  mem_store16(m,self.s,*out);let rc=self.method(m,4,80,&[a,b,id as u32,flag,count,ptr,self.s]);*out=mem_word(m,self.s)as u16;rc
 }
 fn process_m(&mut self,a:u32,b:u32,orth:&mut Option<u32>,records:&mut Option<u32>,count:&mut u16,h0:u32,h1:u32,delta:&mut i16,m:&mut[u8])->u32{
  mem_store(m,self.s,orth.unwrap_or(0));mem_store(m,self.s+4,records.unwrap_or(0));mem_store16(m,self.s+8,*count);mem_store16(m,self.s+12,*delta as u16);
  let rc=self.method(m,8,60,&[a,b,self.s,self.s+4,self.s+8,h0,h1,self.s+12]);
  let o=mem_word(m,self.s);let r=mem_word(m,self.s+4);
  *orth=if o==0{None}else{Some(o)};*records=if r==0{None}else{Some(r)};
  *count=mem_word(m,self.s+8)as u16;*delta=mem_word(m,self.s+12)as i16;rc
 }
 fn get_position_m(&mut self,a:u32,b:u32,out:&mut u32,m:&mut[u8])->u32{
  mem_store(m,self.s,*out);let rc=self.method(m,20,60,&[a,b,self.s,0]);*out=mem_word(m,self.s);rc
 }
 fn set_position_m(&mut self,a:u32,b:u32,value:u32,m:&mut[u8])->u32{self.method(m,20,56,&[a,b,value,0])}
 fn alloc_orth_m(&mut self,heap:u32,size:u32,m:&mut[u8])->Option<u32>{
  let p=self.invoke(m,0x196394,&[heap,size]);if p==0{None}else{Some(p)}
 }
 fn alloc_records_m(&mut self,heap:u32,size:u32,m:&mut[u8])->Option<u32>{
  let p=self.invoke(m,0x1963b8,&[heap,1,size]);if p==0{None}else{Some(p)}
 }
 fn free_buf_m(&mut self,heap:u32,p:u32,m:&mut[u8]){self.invoke(m,0x196428,&[heap,p]);}
 fn log_no_memory_m(&mut self,log:u32,code:u32,flag:u32,m:&mut[u8]){self.invoke(m,0x173dc0,&[log,0x8f96ac,code,flag]);}
}

use crate::frontend::u16_len;
use crate::fe_text::narrow_176460 as recode_176460;
fn mem_half(m:&[u8],p:u32)->u16{u16::from_le_bytes(m[p as usize..p as usize+2].try_into().unwrap())}

pub trait InsertPromptHooksM {

    fn check_m(&mut self, handle: u32, start: u32, memory: &mut [u8]) -> u32;

    fn resolve_m(&mut self, a: u32, b: u32, manager: &mut u32, memory: &mut [u8]) -> u32;

    fn alloc_text_m(&mut self, heap: u32, size: u32, memory: &mut [u8]) -> Option<u32>;

    fn build_prompt_m(
        &mut self,
        handle: u32,
        text: u32,
        prompt: &mut [u32; 9], memory: &mut [u8]) -> u32;

    fn log_insert_m(&mut self, log: u32, memory: &mut [u8]);

    fn free_text_m(&mut self, heap: u32, text: u32, memory: &mut [u8]);

    #[allow(clippy::too_many_arguments)]
    fn rewrite_prompt_m(
        &mut self,
        prompt: &[u32; 9],
        pos: u32,
        start: u32,
        output: &mut u32,
        tokens: &mut u32,
        count: &mut u16,
        index: u32,
        args_a: u32,
        args_b: u32,
        c: u32, memory: &mut [u8]) -> u32;

    fn log_rewrite_m(&mut self, log: u32, memory: &mut [u8]);

    fn log_no_memory_m(&mut self, log: u32, memory: &mut [u8]);
}

pub fn fn_1ff4fc_m<H: InsertPromptHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    handle: u32,
    check: u32,
    output: u32,
    tokens: u32,
    count_ptr: u32,
    args_a: u32,
    args_b: u32,
    c: u32,
) -> u32 {
    let mut rc = hooks.check_m(handle, check, mem);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut manager = 0u32;
    rc = hooks.resolve_m(
        mem_word(mem, handle),
        mem_word(mem, handle.wrapping_add(4)),
        &mut manager, mem);
    if rc >= 0x80000000 {
        return rc;
    }
    let heap = mem_word(mem, manager.wrapping_add(4));
    let log = mem_word(mem, manager.wrapping_add(16));
    let mut tokp = mem_word(mem, tokens);
    let start = mem_word(mem, tokp.wrapping_add(12));
    let mut i = 0u32;
    loop {
        let count = mem_half(mem, count_ptr) as u32;
        if i >= count {
            break;
        }
        let tok = tokp.wrapping_add(i.wrapping_mul(24));
        if mem_half(mem, tok) == 40 {
            let text_ptr = mem_word(mem, tok.wrapping_add(20));
            let size = u16_len(mem, text_ptr).wrapping_add(1);
            let Some(text) = hooks.alloc_text_m(heap, size, mem) else {
                hooks.log_no_memory_m(log, mem);
                rc = 0x88e0200a;
                break;
            };
            recode_176460(mem, text, text_ptr);
            let mut prompt = [0u32; 9];
            rc = hooks.build_prompt_m(handle, text, &mut prompt, mem);
            if rc >= 0x80000000 {
                hooks.free_text_m(heap, text, mem);
                break;
            }
            hooks.log_insert_m(log, mem);
            hooks.free_text_m(heap, text, mem);
            let pos =
                mem_word(mem, tok.wrapping_add(12)).wrapping_sub(start) as u16 as u32;
            let mut out_w = mem_word(mem, output);
            let mut tok_w = mem_word(mem, tokens);
            let mut count_w = mem_half(mem, count_ptr);
            rc = hooks.rewrite_prompt_m(
                &prompt, pos, start, &mut out_w, &mut tok_w, &mut count_w, i,
                args_a, args_b, c, mem);
            mem_store(mem, output, out_w);
            mem_store(mem, tokens, tok_w);
            mem_store16(mem, count_ptr, count_w);
            if rc >= 0x80000000 {
                break;
            }
            hooks.log_rewrite_m(log, mem);
            tokp = tok_w;
        }
        i = i.wrapping_add(1);
    }
    rc
}

pub fn write_field_live_m(m:&mut Memory<'_>,a:[u32;7],h:&mut impl Host)->u32{
 assert_eq!(m.base,0);
 let rom=h.rom();
 let anchor=crate::rom_region::addr(rom,0x8d7ca0);
 let mut descs=[0u8;7*272];descs.copy_from_slice(&m.bytes[anchor as usize..anchor as usize+7*272]);
 write_field_m(m.bytes,(a[0]!=0).then_some(a[0]),a[1],a[2],a[3],a[4],(a[5]!=0).then_some(a[5]),a[6],&descs,crate::rom_region::addr(rom,0x8d7c98),
  &mut |heap,size,bytes|h.call_m(0x1963b8,&[heap,1,size],&mut Memory{base:0,bytes}))
}

pub fn insert_prompt_live_m(m:&mut Memory<'_>,a:[u32;8],scratch:u32,h:&mut impl Host)->u32{
 assert_eq!(m.base,0);let at=scratch as usize;let mut saved=[0;112];saved.copy_from_slice(&m.bytes[at..at+112]);
 let rc=fn_1ff4fc_m(&mut InsertLive{s:scratch,h,text:0,output:0},m.bytes,a[0],a[1],a[2],a[3],a[4],a[5],a[6],a[7]);
 m.bytes[at..at+112].copy_from_slice(&saved);rc
}
struct InsertLive<'a,H>{s:u32,h:&'a mut H,text:u32,output:u32}
impl<H:Host> InsertLive<'_,H>{
 fn invoke(&mut self,m:&mut[u8],f:u32,a:&[u32])->u32{self.h.call_m(f,a,&mut Memory{base:0,bytes:m})}
 fn log(&mut self,m:&mut[u8],logger:u32,format:&[u8],arg:u32){
  let at=(self.s+64)as usize;m[at..at+format.len()].copy_from_slice(format);mem_store(m,self.s+56,arg);
  self.invoke(m,0x1738e8,&[logger,0x8f95d4,5,0,self.s+64,self.s+56]);
 }
}
impl<H:Host> InsertPromptHooksM for InsertLive<'_,H>{
 fn check_m(&mut self,p:u32,tag:u32,_:&mut[u8])->u32{crate::normalization::quad_check(p,tag,0x11d,0x48)}
 fn resolve_m(&mut self,a:u32,b:u32,out:&mut u32,m:&mut[u8])->u32{
  mem_store(m,self.s+48,*out);let rc=self.invoke(m,0x19611c,&[a,b,self.s+48]);*out=mem_word(m,self.s+48);rc
 }
 fn alloc_text_m(&mut self,heap:u32,size:u32,m:&mut[u8])->Option<u32>{
  let p=self.invoke(m,0x196394,&[heap,size]);self.text=p;if p==0{None}else{Some(p)}
 }
 fn build_prompt_m(&mut self,p:u32,text:u32,prompt:&mut[u32;9],m:&mut[u8])->u32{
  for(i,v)in prompt.iter().enumerate(){mem_store(m,self.s+i as u32*4,*v);}
  let rc=self.invoke(m,0x1fe418,&[p,text,self.s]);
  for(i,v)in prompt.iter_mut().enumerate(){*v=mem_word(m,self.s+i as u32*4);}rc
 }
 fn log_insert_m(&mut self,log:u32,m:&mut[u8]){self.log(m,log,b"    Do User Prompt Insertion for [%s]\0",self.text);}
 fn free_text_m(&mut self,heap:u32,p:u32,m:&mut[u8]){self.invoke(m,0x196428,&[heap,p]);}
 fn rewrite_prompt_m(&mut self,prompt:&[u32;9],pos:u32,start:u32,output:&mut u32,tokens:&mut u32,count:&mut u16,index:u32,a:u32,b:u32,c:u32,m:&mut[u8])->u32{
  for(i,v)in prompt.iter().enumerate(){mem_store(m,self.s+i as u32*4,*v);}
  mem_store16(m,self.s+36,*count);mem_store(m,self.s+40,*output);mem_store(m,self.s+44,*tokens);
  let rc=self.invoke(m,0x2784c8,&[self.s,pos,start,self.s+40,self.s+44,self.s+36,index,a,b,c]);
  *output=mem_word(m,self.s+40);*tokens=mem_word(m,self.s+44);*count=mem_half(m,self.s+36);self.output=*output;rc
 }
 fn log_rewrite_m(&mut self,log:u32,m:&mut[u8]){self.log(m,log,b"    [%s]\0",self.output);}
 fn log_no_memory_m(&mut self,log:u32,m:&mut[u8]){self.invoke(m,0x173dc0,&[log,0x8f95d4,0xb798,0]);}
}

use crate::frontend::{validate_handle,move_bytes,u16_copy};
fn field_alloc_m(mem:&mut[u8],arena:u32,size:u32,alloc:&mut impl FnMut(u32,u32,&mut[u8])->u32)->u32{
 struct Allocator<'a,F>(&'a mut F);

 impl<F> crate::db_live::PoolHooksM for Allocator<'_,F>{
  fn log_pool_m(&mut self,_:u32,_:u32,_:&mut Memory<'_>){unreachable!("allocator-only adapter")}
 }
 impl<F:FnMut(u32,u32,&mut[u8])->u32> crate::db_live::DbHooksM for Allocator<'_,F>{
  fn call_m(&mut self,entry:u32,a:&[u32],m:&mut Memory<'_>)->u32{
   assert_eq!(entry,0x1963b8);assert_eq!(a[1],1);self.0(a[0],a[2],m.bytes)
  }
 }
 crate::db_live::arena_alloc_m(&mut Allocator(alloc),&mut Memory{base:0,bytes:mem},arena,size)
}

pub fn write_field_m(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    record: u32,
    field: u32,
    count: u32,
    input: Option<u32>,
    written: u32,
    descs: &[u8],
    word_anchor: u32,
    alloc: &mut impl FnMut(u32, u32, &mut [u8]) -> u32,
) -> u32 {
    if validate_handle(object.is_some(), tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let db = object.unwrap();
    let table = mem_word(mem, db + 48);
    let data = if record < mem_word(mem, db + 32) {
        mem_word(mem, table.wrapping_add(record.wrapping_mul(4)))
    } else {
        0
    };
    if data == 0 {
        return 0x89203e11;
    }
    let ty = mem[data as usize + 2];
    let layout = match descs.get(ty as usize * 272..) {
        Some(layout) if layout.len() >= 272 => layout,
        _ => return 0x89203e0d,
    };
    let base = u32::from_le_bytes(layout[0..4].try_into().unwrap());
    let nfields = u16::from_le_bytes(layout[4..6].try_into().unwrap()) as u32;
    let states = u32::from_le_bytes(layout[8..12].try_into().unwrap());
    if ty == 0 || field >= nfields {
        return 0x89203e0d;
    }
    let field_off = 12 + field as usize * 20;
    let fdata = u32::from_le_bytes(layout[field_off..field_off + 4].try_into().unwrap());
    let fcount = u32::from_le_bytes(layout[field_off + 4..field_off + 8].try_into().unwrap());
    let flength = u16::from_le_bytes(layout[field_off + 8..field_off + 10].try_into().unwrap());
    let fsize = u16::from_le_bytes(layout[field_off + 10..field_off + 12].try_into().unwrap());
    let dest = data.wrapping_add(fdata.wrapping_sub(base));
    let cnt = if fcount != 0 {
        Some(data.wrapping_add(fcount.wrapping_sub(base)))
    } else {
        None
    };
    mem[data
        .wrapping_add(states.wrapping_sub(base))
        .wrapping_add(field) as usize] = 1;
    let src = input.unwrap_or(0);
    if flength == 0 {
        let bytes = (fsize as u32).wrapping_mul(count);
        let p = field_alloc_m(mem, db + 4, bytes, alloc);
        mem_store(mem, dest, p);
        if p == 0 {
            return 0x8920200a;
        }
        move_bytes(mem, p, src, bytes);
        if fdata == word_anchor {
            let mut i = 0u32;
            while i < count {
                let entry = src.wrapping_add(i.wrapping_mul(24));
                let etype = mem_half(mem, entry);
                if matches!(
                    etype,
                    6 | 12 | 21 | 50 | 36 | 39 | 37 | 34 | 22 | 40 | 26 | 29
                ) {
                    let str_ptr = mem_word(mem, entry + 20);
                    let n = if str_ptr == 0 {
                        0
                    } else {
                        u16_len(mem, str_ptr)
                    };
                    let d = field_alloc_m(
                        mem,
                        db + 4,
                        n.wrapping_mul(2).wrapping_add(2),
                        alloc,
                    );
                    if d == 0 {
                        return 0x8920200a;
                    }
                    if n == 0 {
                        mem_store16(mem, d, 0);
                    } else {
                        u16_copy(mem, d, str_ptr);
                    }
                    mem_store(mem, p.wrapping_add(i.wrapping_mul(24)).wrapping_add(20), d);
                }
                i = i.wrapping_add(1);
            }
        }
        if let Some(cnt) = cnt {
            mem_store16(mem, cnt, count as u16);
        }
        mem_store16(mem, written, count as u16);
    } else {
        let length = if flength as u32 > count {
            count
        } else {
            flength as u32
        };
        move_bytes(mem, dest, src, (fsize as u32).wrapping_mul(length));
        if let Some(cnt) = cnt {
            mem_store16(mem, cnt, length as u16);
        }
        mem_store16(mem, written, length as u16);
    }
    mem[db as usize + 52] = 1;
    0
}

pub fn fn_1d5ba8_m(m:&mut Memory<'_>,p:u32,tag:u32,db:[u32;2],pending:u32,e:&crate::speech_part_1::ProcessEnvironment,scratch:u32,h:&mut impl Host)->u32{
 crate::speech_part_1::fn_1d5ba8(m,p,tag,db,pending,e,scratch,&mut ProcessCalls{h,log_s:scratch+768})
}
struct ProcessCalls<'a,H>{h:&'a mut H,log_s:u32}
impl<H:Host> ProcessCalls<'_,H>{
 fn trace(&mut self,m:&mut Memory<'_>,logger:u32,category:u32,level:u32,format:Option<&[u8]>,guest_format:u32,values:&[u32]){
  assert!(values.len()<=16);
  let at=m.offset(self.log_s);let mut saved=[0;320];saved.copy_from_slice(&m.bytes[at..at+320]);
  let f=if let Some(text)=format{
   assert!(text.len()<256);m.bytes[at..at+text.len()].copy_from_slice(text);m.bytes[at+text.len()]=0;self.log_s
  }else{guest_format};
  for(i,v)in values.iter().enumerate(){m.put_word(self.log_s+256+i as u32*4,*v);}
  self.h.call_m(0x1738e8,&[logger,category,level,0,f,self.log_s+256],m);
  m.bytes[at..at+320].copy_from_slice(&saved);
 }
}
impl<H:Host> crate::speech_part_1::Host for ProcessCalls<'_,H>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.h.call_m(f,a,m)}
 fn get(&mut self,m:&mut Memory<'_>,f:u32,handle:u32,pos:u32,len:u32)->(u32,u32,u32){
  let saved=[m.word(self.log_s),m.word(self.log_s+4)];
  m.put_word(self.log_s,len);m.put_word(self.log_s+4,0);
  let rc=self.h.call_m(f,&[handle,pos,self.log_s,self.log_s+4],m);
  let n=m.word(self.log_s);let ptr=m.word(self.log_s+4);
  m.put_word(self.log_s,saved[0]);m.put_word(self.log_s+4,saved[1]);(rc,n,ptr)
 }
}
impl<H:Host> crate::speech_part_6::Host for ProcessCalls<'_,H>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.h.call_m(f,a,m)}
 fn log(&mut self,m:&mut Memory<'_>,logger:u32,category:u32,level:u32,format:&[u8],values:&[u32]){
  self.trace(m,logger,category,level,Some(format),0,values)
 }
 fn log_guest(&mut self,m:&mut Memory<'_>,logger:u32,category:u32,level:u32,format:u32,values:&[u32]){
  self.trace(m,logger,category,level,None,format,values)
 }
 fn error(&mut self,m:&mut Memory<'_>,logger:u32,category:u32,code:u32){self.h.call_m(0x173dc0,&[logger,category,code,0,0,0],m);}
}

pub fn process_environment_eu()->crate::speech_part_1::ProcessEnvironment{
 crate::speech_part_1::ProcessEnvironment{
  normal:0x8f9a99,
  phon:0x8f9a94,
  default16:0x8f9b2c,
  latin16:0x8f9b3a,
  user16:0x8f9b46,
  section:0x1d6460,
  esctn:0x1d6cbc,
  esclang:0x1d75ec,
  domain:0x1d6cc8,
  extra:0x1d6cdc,
  pipeline:0x1d6430,
  pause:0x1d6468,
  mapped_pause:0x1d6468,
  original_name:0x1d6ce8,
  base_name:0x1d6cf8,
  format2:0x1d6d08,
  format4:0x1d6d04,
  token:crate::speech_part_6::TokenConstants{
   category:0x8f9aac,
   langcode:0x1d580c,
   clm:0x1d5844,
   user:0x1d583c,
   end_format:0x1d581c,
   lid:0x1d583c,
   noclmset:0x1d5848,
   no:0x1d5854,
   yes:0x1d5c7c,
   marker:0x1d5840,
  },
 }
}

pub fn process_environment_kr()->crate::speech_part_1::ProcessEnvironment{
 crate::speech_part_1::ProcessEnvironment{
  normal:0x91bdf9,
  phon:0x91bdf4,
  default16:0x91be8c,
  latin16:0x91be9a,
  user16:0x91bea6,
  section:0x1d650c,
  esctn:0x1d6d68,
  esclang:0x1d7698,
  domain:0x1d6d74,
  extra:0x1d6d88,
  pipeline:0x1d64dc,
  pause:0x1d6514,
  mapped_pause:0x1d6514,
  original_name:0x1d6d94,
  base_name:0x1d6da4,
  format2:0x1d6db4,
  format4:0x1d6db0,
  token:crate::speech_part_6::TokenConstants{
   category:0x91be0c,
   langcode:0x1d58b8,
   clm:0x1d58f0,
   user:0x1d58e8,
   end_format:0x1d58c8,
   lid:0x1d58e8,
   noclmset:0x1d58f4,
   no:0x1d5900,
   yes:0x1d5d28,
   marker:0x1d58ec,
  },
 }
}

pub fn process_environment_us()->crate::speech_part_1::ProcessEnvironment{
 crate::speech_part_1::ProcessEnvironment{
  normal:0x8f8859,
  phon:0x8f8854,
  default16:0x8f88ec,
  latin16:0x8f88fa,
  user16:0x8f8906,
  section:0x1d4d4c,
  esctn:0x1d6b00,
  esclang:0x1d7430,
  domain:0x1d6b0c,
  extra:0x1d6b20,
  pipeline:0x1d6274,
  pause:0x1d62ac,
  mapped_pause:0x1d62ac,
  original_name:0x1d6b2c,
  base_name:0x1d6b3c,
  format2:0x1791c4,
  format4:0x1d6b48,
  token:crate::speech_part_6::TokenConstants{
   category:0x8f886c,
   langcode:0x1d4d1c,
   clm:0x1d5040,
   user:0x1d5680,
   end_format:0x1d5660,
   lid:0x1d5680,
   noclmset:0x1d568c,
   no:0x1d5698,
   yes:0x1d5ac0,
   marker:0x1d5684,
  },
 }
}

pub fn ld_reset_live_m(m:&mut Memory<'_>,p:u32,tag:u32,scratch:u32,h:&mut impl Host)->u32{
 let at=m.offset(scratch);let mut saved=[0;96];saved.copy_from_slice(&m.bytes[at..at+96]);
 let rom=h.rom();
 let data=crate::db_live::DbData{module:crate::rom_region::addr(rom,0x8d7b56),open_begin:0,open_end:0,open_detail:0,reset_begin:crate::rom_region::addr(rom,0x15bbdc),reset_end:crate::rom_region::addr(rom,0x15bbfc),pool_name_word:0,keys:[0;5]};
 let rc=crate::db_live::ld_reset_m(&mut DbCalls{h,s:scratch},m,p,tag,&data,scratch);
 m.bytes[at..at+96].copy_from_slice(&saved);rc
}
struct DbCalls<'a,H>{h:&'a mut H,s:u32}
impl<H:Host> crate::db_live::DbHooksM for DbCalls<'_,H>{
 fn call_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32{
  if f==0x1738e8{
   assert!(a.len()<=12);for(i,v)in a[5..].iter().enumerate(){m.put_word(self.s+16+i as u32*4,*v);}
   return self.h.call_m(f,&[a[0],a[1],a[2],a[3],a[4],self.s+16],m);
  }
  if f==0x173dc0 && a.len()==4{return self.h.call_m(f,&[a[0],a[1],a[2],a[3],0,0],m);}
  self.h.call_m(f,a,m)
 }

 fn literal_m(&self,us:u32)->u32{crate::rom_region::addr(self.h.rom(),us)}
}
impl<H:Host> crate::db_live::PoolHooksM for DbCalls<'_,H>{
 fn log_pool_m(&mut self,p:u32,blocks:u32,m:&mut Memory<'_>){
  let stage=b"before ResetPool\0";let at=m.offset(self.s+64);m.bytes[at..at+stage.len()].copy_from_slice(stage);
  for(i,v)in [m.word(p+4),self.s+64,m.word(p+24),m.word(p+20),m.word(p+12),m.word(p+16),blocks].iter().enumerate(){m.put_word(self.s+16+i as u32*4,*v);}
  self.h.call_m(0x1738e8,&[m.word(m.word(p)+16),0x8d7b56,5,0,0x8dc928,self.s+16],m);
 }
}

pub fn speech_literals_us()->crate::speech_part_2::SpeechLiterals{
 crate::speech_part_2::SpeechLiterals{
text:[0x24ed34,0x24ed18,0x24ed20,0x193c10,0x24ed4c,0x24ed64,0x24ed7c,0x24eda4,0x8f8f44,0x1ee188,0x1ee6bc,0x8f8fd8,0x24c858,0x24c880,0x24c8b8,0x914a38],
end:0x24c9d0,
errors:[0x24ea48,0x24ee48,0x1ec3f4,0x250d48,0x207740]
}
}

impl<H:Host> crate::speech_part_2::Host for ProcessCalls<'_,H>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.h.call_m(f,a,m)}
}
impl<H:Host> crate::boot_part_b::Host for ProcessCalls<'_,H>{
 fn real_log_args(&mut self)->bool{true}
 fn log_variadic(&mut self,m:&mut Memory<'_>,entry:u32,a:[u32;4],format:&[u8],varargs:&[u32])->bool{
  crate::log_live::variadic_m(m,entry,a,format,varargs,self.log_s,|m,f,a|self.h.call_m(f,a,m))
 }

 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.h.call_m(f,a,m)}
 fn thread_id(&mut self)->u32{self.h.thread_id_m()}
}
impl<H:Host> crate::boot_part_b::FormatHost for ProcessCalls<'_,H>{}
impl<H:Host> crate::log_live::LogMethodsM for ProcessCalls<'_,H>{
 fn log_m(&mut self,f:u32,p:u32,category:u32,level:u32,arg:u32,fmt:u32,va:u32,m:&mut Memory<'_>)->u32{
  self.h.call_m(f,&[p,category,level,arg,fmt,va],m)
 }
}

pub fn fn_1ede44_m(m:&mut Memory<'_>,a:[u32;6],t:&crate::speech_part_2::SpeechLiterals,scratch:u32,h:&mut impl Host)->u32{
 crate::speech_part_2::fn_1ede44(m,a[0],a[1],[a[2],a[3]],a[4],a[5],t,scratch,&mut ProcessCalls{h,log_s:scratch+84})
}

pub fn create_child_record_m(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    child_type: u32,
    index: u32,
    out: u32,
    scratch: u32,
    parent_table: &[u8],
    h: &mut impl Host,
    rom: crate::rom_region::Rom,
) -> u32 {
    if validate_handle(object.is_some(), tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let base = object.unwrap();
    if index >= mem_word(mem, base.wrapping_add(32)) {
        return 0x89203e11;
    }
    let records = mem_word(mem, base.wrapping_add(48));
    let parent = mem_word(mem, records.wrapping_add(index.wrapping_mul(4)));
    if parent == 0 {
        return 0x89203e11;
    }
    let parent_type = mem[parent.wrapping_add(2) as usize];
    if child_type < 2 {
        return 0x89203e02;
    }
    if parent_table[child_type as usize] != parent_type {
        return 0x89203e03;
    }
    let rc = h.call_m(0x15b758, &[base,child_type,scratch], &mut Memory{base:0,bytes:mem});
    if rc >= 0x80000000 {
        return rc;
    }
    let node = mem_word(mem, scratch);
    mem_store(mem, node.wrapping_add(4), parent);
    mem_store(mem, node.wrapping_add(16), 0);
    mem_store(mem, node.wrapping_add(12), 0);
    let mut found = false;
    let mut i = 0u32;
    while i < 2 && !found {
        let slot = parent.wrapping_add(16).wrapping_add(i.wrapping_mul(4));
        let child = mem_word(mem, slot);
        if child != 0 && u32::from(mem[child.wrapping_add(2) as usize]) == child_type {
            found = true;
            mem_store(mem, node.wrapping_add(8), child);
            mem_store(mem, child.wrapping_add(12), node);
            mem_store(mem, slot, node);
        }
        i += 1;
    }
    if !found {
        let mut j = 0u32;
        while j < 2
            && mem_word(
                mem,
                parent.wrapping_add(16).wrapping_add(j.wrapping_mul(4)),
            ) != 0
        {
            j += 1;
        }
        if j < 2 {
            mem_store(
                mem,
                parent.wrapping_add(16).wrapping_add(j.wrapping_mul(4)),
                node,
            );
            mem_store(mem, node.wrapping_add(8), 0);
        } else {
            return 0x89203e1b;
        }
    }
    mem_store16(mem, out, mem_half(mem, node));
    mem[base.wrapping_add(52) as usize] = 1;
    rc
}

pub fn create_record_live_m(m:&mut Memory<'_>,a:[u32;3],scratch:u32,h:&mut impl Host)->u32{
 let at=m.offset(scratch);let mut saved=[0;96];saved.copy_from_slice(&m.bytes[at..at+96]);
 let rom=h.rom();
 let rc=crate::db_live::create_record_m(&mut DbCalls{h,s:scratch},m,a[0],a[1],a[2],crate::rom_region::addr(rom,0x8d8274),crate::rom_region::addr(rom,0x8d7ca0));
 m.bytes[at..at+96].copy_from_slice(&saved);rc
}

pub fn create_child_live_m(m:&mut Memory<'_>,a:[u32;5],scratch:u32,h:&mut impl Host)->u32{
 assert_eq!(m.base,0);let saved=m.word(scratch);
 let pa=crate::rom_region::addr(h.rom(),0x8d7b50) as usize;
 let mut parents=[0u8;32];parents.copy_from_slice(&m.bytes[pa..pa+32]);
 let rc=create_child_record_m(m.bytes,(a[0]!=0).then_some(a[0]),a[1],a[2],a[3],a[4],scratch,&parents,h,h.rom());
 m.put_word(scratch,saved);rc
}

pub fn clone_record_m(
    mem: &mut [u8],
    db: Option<u32>,
    a: u32,
    index: u32,
    out: u32,
    scratch: u32,
    h: &mut impl Host,
) -> u32 {
    if validate_handle(db.is_some(), a, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let base = db.unwrap();
    if index >= mem_word(mem, base.wrapping_add(32)) {
        return 0x89203e11;
    }
    let records = mem_word(mem, base.wrapping_add(48));
    let record = mem_word(mem, records.wrapping_add(index.wrapping_mul(4)));
    if record == 0 {
        return 0x89203e11;
    }
    let rtype = mem[record.wrapping_add(2) as usize];
    if rtype == 1 {
        return 0x89203e04;
    }
    let rc = h.call_m(0x15b758, &[base,u32::from(rtype),scratch], &mut Memory{base:0,bytes:mem});
    if rc >= 0x80000000 {
        return rc;
    }
    let node = mem_word(mem, scratch);
    mem_store(mem, node.wrapping_add(4), mem_word(mem, record.wrapping_add(4)));
    mem_store(mem, node.wrapping_add(12), record);
    let next = mem_word(mem, record.wrapping_add(8));
    mem_store(mem, node.wrapping_add(8), next);
    mem_store(mem, record.wrapping_add(8), node);
    mem_store(mem, node.wrapping_add(16), 0);
    if next != 0 {
        mem_store(mem, next.wrapping_add(12), node);
    }
    mem_store16(mem, out, mem_half(mem, node));
    mem[base as usize + 52] = 1;
    rc
}

struct OtherCalls<'a,H>(&'a mut H);
impl<H:Host> crate::component_live_b::Host for OtherCalls<'_,H>{
 fn call_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32{self.0.call_m(f,a,m)}
}

pub fn stream_call_live_m(m:&mut Memory<'_>,f:u32,a:&[u32],h:&mut impl Host)->u32{
 crate::component_live_b::stream_call_m(m,f,a,&mut OtherCalls(h))
}

#[allow(clippy::too_many_arguments)]
pub fn fn_2104b0_m(
 m:&mut Memory<'_>,p:u32,tag:u32,input:[u32;2],active:u32,scratch:u32,saved:&mut[u8],
 events:&mut[crate::frontend::TextEvent],event_bytes:&mut[u8],text:&mut[u8],h:&mut impl Host,
)->Result<u32,crate::frontend_types::Error>{
 use crate::frontend::{TextRewriter,RewriterHandle};
 if crate::normalization::quad_check(p,tag,0xf381,0x34)>=0x80000000{return Ok(0x89a02008);}
 assert!(saved.len()>=32);
 let at=m.offset(scratch);saved.copy_from_slice(&m.bytes[at..at+saved.len()]);
 let mut owner=TextRewriter{
  outputs:core::array::from_fn(|i|RewriterHandle{first:m.word(p+24+i as u32*8),second:m.word(p+28+i as u32*8)}),
  position_handle:RewriterHandle{first:m.word(p+12),second:m.word(p+16)},
  preserve_stars:m.byte(p+48)!=0,
 };
 m.put_byte(active,1);let mut changed=1;
 let result=crate::frontend::rewrite_text(&mut owner,tag,RewriterHandle{first:input[0],second:input[1]},&mut changed,
  &mut RewriterLive{m,h,p,s:scratch,capacity:saved.len()},events,event_bytes,text);
 m.put_byte(active,changed);
 for(i,o)in owner.outputs.iter().enumerate(){m.put_word(p+24+i as u32*8,o.first);m.put_word(p+28+i as u32*8,o.second);}
 m.bytes[at..at+saved.len()].copy_from_slice(saved);result
}
struct RewriterLive<'a,'m,H>{m:&'a mut Memory<'m>,h:&'a mut H,p:u32,s:u32,capacity:usize}
impl<H:Host> RewriterLive<'_, '_,H>{
 fn method(&mut self,table:u32,slot:u32,a:&[u32])->u32{
  let f=self.m.word(self.m.word(self.p+table)+slot);self.h.call_m(f,a,self.m)
 }
 fn half(&self,p:u32)->u16{let at=self.m.offset(p);u16::from_le_bytes(self.m.bytes[at..at+2].try_into().unwrap())}
 fn child(&mut self,slot:u32,a:u32,b:u32,c:u32,d:u32,out:&mut u16)->u32{
  self.m.put_half(self.s,*out);let rc=self.method(4,slot,&[a,b,c,d,self.s]);*out=self.half(self.s);rc
 }
 fn scalar(&mut self,input:crate::frontend::RewriterHandle,node:u16,field:u32,index:u16,initial:u32,count:&mut u16)->(u32,u32){
  self.m.put_word(self.s+8,initial);self.m.put_half(self.s+4,*count);
  let rc=self.method(4,84,&[input.first,input.second,node as u32,field,index as u32,self.s+8,self.s+4]);
  *count=self.half(self.s+4);(rc,self.m.word(self.s+8))
 }
 fn data(&mut self,input:crate::frontend::RewriterHandle,node:u16,field:u32,count:&mut u16)->(u32,u32){
  self.m.put_word(self.s,0);self.m.put_half(self.s+4,*count);
  let rc=self.method(4,88,&[input.first,input.second,node as u32,field,self.s,self.s+4]);
  *count=self.half(self.s+4);(rc,self.m.word(self.s))
 }
 fn stage(&mut self,data:&[u8])->Result<u32,crate::frontend_types::Error>{
  let available=self.capacity-16;
  if data.len()>available{return Err(crate::frontend_types::Error::Capacity{required:data.len(),available});}
  let at=self.m.offset(self.s+16);self.m.bytes[at..at+data.len()].copy_from_slice(data);Ok(self.s+16)
 }
}
impl<H:Host> crate::frontend::RewriterText for RewriterLive<'_, '_,H>{
 fn first(&mut self,i:crate::frontend::RewriterHandle,table:u32,root:u16,out:&mut u16)->u32{self.child(52,i.first,i.second,table,root as u32,out)}
 fn next(&mut self,i:crate::frontend::RewriterHandle,node:u16,out:&mut u16)->u32{
  self.m.put_half(self.s,*out);let rc=self.method(4,60,&[i.first,i.second,node as u32,self.s]);*out=self.half(self.s);rc
 }
 fn has_children(&mut self,i:crate::frontend::RewriterHandle,table:u32,node:u16,out:&mut u16)->u32{self.child(76,i.first,i.second,table,node as u32,out)}
 fn has(&mut self,i:crate::frontend::RewriterHandle,node:u16,field:u32,out:&mut u16)->u32{self.child(92,i.first,i.second,node as u32,field,out)}
 fn get_u8(&mut self,i:crate::frontend::RewriterHandle,node:u16,field:u32,index:u16,out:&mut u8,count:&mut u16)->u32{
  let(rc,v)=self.scalar(i,node,field,index,*out as u32,count);*out=v as u8;rc
 }
 fn get_u16(&mut self,i:crate::frontend::RewriterHandle,node:u16,field:u32,index:u16,out:&mut u16,count:&mut u16)->u32{
  let(rc,v)=self.scalar(i,node,field,index,*out as u32,count);*out=v as u16;rc
 }
 fn get_text(&mut self,i:crate::frontend::RewriterHandle,node:u16,field:u32,scratch:&mut[u8],count:&mut u16)->Result<(u32,bool),crate::frontend_types::Error>{
  let(rc,ptr)=self.data(i,node,field,count);
  if rc>=0x80000000||ptr==0{return Ok((rc,false));}
  let n=*count as usize;if n>scratch.len(){return Err(crate::frontend_types::Error::Capacity{required:n,available:scratch.len()});}
  let at=self.m.offset(ptr);scratch[..n].copy_from_slice(&self.m.bytes[at..at+n]);Ok((rc,true))
 }
 fn get_events(&mut self,i:crate::frontend::RewriterHandle,node:u16,field:u32,scratch:&mut[crate::frontend::TextEvent],count:&mut u16)->Result<(u32,bool),crate::frontend_types::Error>{
  let(rc,ptr)=self.data(i,node,field,count);
  if rc>=0x80000000||ptr==0{return Ok((rc,false));}
  let n=*count as usize;if n>scratch.len(){return Err(crate::frontend_types::Error::Capacity{required:n,available:scratch.len()});}
  for(j,e)in scratch[..n].iter_mut().enumerate(){
   let p=ptr+j as u32*24;e.kind=self.half(p);e.flags=self.half(p+2);e.data=[self.m.word(p+4),self.m.word(p+8)];e.position=self.m.word(p+12)as i32;e.length=self.m.word(p+16)as i32;e.value=self.m.word(p+20)as i32;
  }
  Ok((rc,true))
 }
 fn set_text(&mut self,i:crate::frontend::RewriterHandle,node:u16,field:u32,count:u16,text:&[u8],out:&mut u16)->u32{
  let ptr=self.stage(text).expect("caller must reserve its text capacity in the guest frame");self.m.put_half(self.s,*out);
  let rc=self.method(4,80,&[i.first,i.second,node as u32,field,count as u32,ptr,self.s]);*out=self.half(self.s);rc
 }
 fn release(&mut self,i:crate::frontend::RewriterHandle)->u32{self.method(4,32,&[i.first,i.second])}
}
impl<H:Host> crate::frontend::RewriterBuffer for RewriterLive<'_, '_,H>{
 fn append(&mut self,i:crate::frontend::RewriterHandle,data:&[u8])->u32{
  let ptr=self.stage(data).expect("caller must reserve serialized-event capacity in the guest frame");
  self.method(8,52,&[i.first,i.second,ptr,data.len()as u32])
 }
 fn release(&mut self,i:crate::frontend::RewriterHandle)->u32{
  let rc=self.method(8,36,&[i.first,i.second]);
  for n in 0..3{
   let at=self.p+24+n*8;
   if self.m.word(at)==i.first&&self.m.word(at+4)==i.second{self.m.put_word(at,0);self.m.put_word(at+4,0);break;}
  }
  rc
 }
}
impl<H:Host> crate::frontend::RewriterPositions for RewriterLive<'_, '_,H>{
 fn get(&mut self,i:crate::frontend::RewriterHandle,out:&mut i32,which:u32)->u32{
  self.m.put_word(self.s,*out as u32);let rc=self.method(20,60,&[i.first,i.second,self.s,which]);*out=self.m.word(self.s)as i32;rc
 }
 fn set(&mut self,i:crate::frontend::RewriterHandle,value:i32,which:u32)->u32{self.method(20,56,&[i.first,i.second,value as u32,which])}
}

pub fn read_field_m(m:&mut Memory<'_>,a:[u32;7],descriptors:u32,frame:u32,workspace:&mut[u8])->Result<u32,crate::engine::Error>{
 use crate::linguistic::{Field,FieldDatabase,Record};
 m.put_half(a[6],0);
 let saved=[m.word(frame),m.word(frame+4)];
 let result=field_data_m(m,[a[0],a[1],a[2],a[3],frame,frame+4],descriptors);
 let pointer=m.word(frame);let count={let at=m.offset(frame+4);u16::from_le_bytes(m.bytes[at..at+2].try_into().unwrap())};
 m.put_word(frame,saved[0]);m.put_word(frame+4,saved[1]);
 let status=result?;if status>=0x80000000{return Ok(status);}
 let record=m.word(m.word(a[0]+48)+a[2]*4);let kind=m.byte(record+2);
 let at=m.offset(descriptors+kind as u32*272+22+a[3]*20);
 let size=u16::from_le_bytes(m.bytes[at..at+2].try_into().unwrap());
 let records=[Some(Record{kind,..Default::default()})];
 let fields=[Field{data:Some(&m.bytes[m.offset(pointer)..]),state:1,fixed_count:count,count:None,element_size:size}];
 let rows:[&[Field<'_>];1]=[&fields];let db=FieldDatabase{records:&records,fields:&rows,valid:true};
 let mut written=0;
 let rc=db.read_field(0,0,a[4],workspace,&mut written).map_err(|e|match e{
  crate::frontend_types::Error::Capacity{required,available}=>crate::engine::Error::Capacity{required,available},
  _=>crate::engine::Error::InvalidSnapshot,
 })?;
 let bytes=written as usize*size as usize;
 let out=m.offset(a[5]);
 crate::frontend::copy_bytes(&mut m.bytes[out..out+bytes],&workspace[..bytes],bytes as u32);
 m.put_half(a[6],written);Ok(rc)
}

impl<H:Host> crate::boot_part_e::Host for ProcessCalls<'_,H>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.h.call_m(f,a,m)}
}

pub fn trace_stream_write_m(m:&mut Memory<'_>,a:[u32;3],h:&mut impl Host)->Result<(),crate::engine::Error>{
 let logger=m.word(m.word(a[0]+4)+16);
 let level=crate::component_live_a::max_slot_level_m(m,logger,&mut ProcessCalls{h,log_s:0});
 if level>=6{return Err(crate::engine::Error::MissingStreamAdapter{entry:0x14e614});}
 crate::frontend::log_write(m.bytes,a[0],(a[1]!=0).then_some(a[1]),a[2],level);Ok(())
}

pub fn clone_record_live_m(m:&mut Memory<'_>,a:[u32;4],scratch:u32,h:&mut impl Host)->u32{
 assert_eq!(m.base,0);let saved=m.word(scratch);
 let rc=clone_record_m(m.bytes,(a[0]!=0).then_some(a[0]),a[1],a[2],a[3],scratch,h);
 m.put_word(scratch,saved);rc
}

pub fn enquire_child_m(m:&mut Memory<'_>,a:[u32;5],rom:crate::rom_region::Rom)->Result<u32,crate::engine::Error>{
 use crate::linguistic::{Database,Record};
 if crate::normalization::quad_check(a[0],a[1],0xb2c2,0x38)>=0x80000000{return Ok(0x89202008);}
 let mut records=[None;3];let mut parents=[0u8;256];
 if a[2]!=1{
  if a[2]>=parents.len()as u32{return Err(crate::engine::Error::InputTooLong);}
  let missing=crate::engine::Error::UnsupportedStage{entry:0x173dc0};
  if a[3]>=m.word(a[0]+32){return Err(missing);}
  let table=m.word(a[0]+48);let parent=m.word(table+a[3]*4);
  if parent==0{return Err(missing);}
  let mut record=Record{id:{let at=m.offset(parent);u16::from_le_bytes(m.bytes[at..at+2].try_into().unwrap())},kind:m.byte(parent+2),..Default::default()};
  parents[a[2]as usize]=m.byte(crate::rom_region::addr(rom,0x8d7b50)+a[2]);
  for i in 0..2{
   let child=m.word(parent+16+i as u32*4);if child==0{break;}
   let at=m.offset(child);records[i+1]=Some(Record{id:u16::from_le_bytes(m.bytes[at..at+2].try_into().unwrap()),kind:m.byte(child+2),..Default::default()});
   record.children[i]=Some(i+1);
  }
  records[0]=Some(record);
 }
 let db=Database{records:&records,parents:&parents,allocated:false,valid:true};
 let mut output=0;let rc=db.enquire_child(a[2],0,&mut output);
 if rc>=0x80000000{return Err(crate::engine::Error::UnsupportedStage{entry:0x173dc0});}
 m.put_half(a[4],output);Ok(rc)
}

pub fn record_link_m(m:&mut Memory<'_>,f:u32,a:[u32;4])->Result<u32,crate::engine::Error>{
 use crate::linguistic::{Database,Record};
 if crate::normalization::quad_check(a[0],a[1],0xb2c2,0x38)>=0x80000000{return Ok(0x89202008);}
 let missing=crate::engine::Error::UnsupportedStage{entry:0x173dc0};
 if a[2]>=m.word(a[0]+32){return Err(missing);}
 let record=m.word(m.word(a[0]+48)+a[2]*4);if record==0{return Err(missing);}
 let off=match f{0x15c4b0=>4,0x15c744|0x15c7d8=>8,0x15c9a4|0x15ca38=>12,_=>return Err(crate::engine::Error::UnsupportedStage{entry:f})};
 let ptr=m.word(record+off);
 let read_id=|m:&Memory<'_>,p:u32|{let at=m.offset(p);u16::from_le_bytes(m.bytes[at..at+2].try_into().unwrap())};
 let mut root=Record{id:read_id(m,record),kind:m.byte(record+2),..Default::default()};
 let link=(ptr!=0).then_some(1);
 match off{4=>root.parent=link,8=>root.next=link,_=>root.previous=link}
 let records=[Some(root),if ptr==0{None}else{Some(Record{id:read_id(m,ptr),kind:m.byte(ptr+2),..Default::default()})}];
 let db=Database{records:&records,parents:&[],allocated:true,valid:true};let mut output=0;
 let rc=match f{
  0x15c4b0=>db.parent(0,&mut output),0x15c744=>db.next(0,&mut output),0x15c9a4=>db.previous(0,&mut output),
  0x15c7d8=>db.next_in_sentence(0,&mut output),_=>db.previous_in_sentence(0,&mut output),
 };
 if rc>=0x80000000{return Err(missing);}
 m.put_half(a[3],output);Ok(rc)
}

struct HeapMethods<'a,H>{heap:u32,h:&'a mut H}
impl<H:Host> crate::frontend::HeapVtableM for HeapMethods<'_,H>{
 fn alloc_at_m(&mut self,obj:u32,n:u32,bytes:&mut[u8])->u32{
  let f=mem_word(bytes,mem_word(bytes,self.heap));self.h.call_m(f,&[obj,n],&mut Memory{base:0,bytes})
 }
 fn realloc_at_m(&mut self,obj:u32,old:u32,n:u32,bytes:&mut[u8])->u32{
  let f=mem_word(bytes,mem_word(bytes,self.heap)+8);self.h.call_m(f,&[obj,old,n],&mut Memory{base:0,bytes})
 }
 fn release_at_m(&mut self,obj:u32,p:u32,bytes:&mut[u8]){
  let f=mem_word(bytes,mem_word(bytes,self.heap)+12);self.h.call_m(f,&[obj,p],&mut Memory{base:0,bytes});
 }
}

pub fn typed_stream_element_m(m:&mut Memory<'_>,f:u32,a:&[u32],h:&mut impl Host)->u32{
 let mut methods=HeapMethods{heap:a[1],h};
 match f{
  0x151610=>crate::component_live_b::duplicate_typed_string_m(m.bytes,(a[0]!=0).then_some(a[0]),(a[1]!=0).then_some(a[1]),(a[2]!=0).then_some(a[2]),&mut methods),
  0x1516c8=>{crate::component_live_b::release_typed_buffer_m(m.bytes,(a[0]!=0).then_some(a[0]),a[1],&mut methods);0}
  _=>panic!("typed-element ABI"),
 }
}

pub trait DbClearHooksM{
 fn log_begin_m(&mut self,log:u32,memory:&mut[u8]);
 fn log_end_m(&mut self,log:u32,result:u32,memory:&mut[u8]);
 fn reset_pool_m(&mut self,at:u32,memory:&mut[u8]);
}

pub fn clear_db_m<H: DbClearHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
    a: u32,
) -> u32 {
    if a != 0xb2c2 || base.is_none() {
        return 0x89202008;
    }
    let base = base.unwrap();
    let ctx = mem_word(mem, base);
    let log = mem_word(mem, ctx.wrapping_add(16));
    hooks.log_begin_m(log,mem);
    mem[base.wrapping_add(52) as usize] = 0;
    if mem_word(mem, base.wrapping_add(48)) != 0 {
        hooks.reset_pool_m(base.wrapping_add(4),mem);
        mem_store(mem, base.wrapping_add(32), 1);
    }
    hooks.log_end_m(log,0,mem);
    0
}

struct ClearLive<'a,H>{h:&'a mut H,s:u32}
impl<H:Host> DbClearHooksM for ClearLive<'_,H>{
 fn log_begin_m(&mut self,log:u32,bytes:&mut[u8]){
  self.h.call_m(0x1738e8,&[log,0x8d7b56,4,0,0x15b5b8,self.s+16],&mut Memory{base:0,bytes});
 }
 fn log_end_m(&mut self,log:u32,rc:u32,bytes:&mut[u8]){
  mem_store(bytes,self.s+16,rc);
  self.h.call_m(0x1738e8,&[log,0x8d7b56,4,0,0x15b5d4,self.s+16],&mut Memory{base:0,bytes});
 }
 fn reset_pool_m(&mut self,p:u32,bytes:&mut[u8]){
  crate::db_live::fn_180ea0(&mut DbCalls{h:self.h,s:self.s},&mut Memory{base:0,bytes},p);
 }
}

pub fn clear_db_live_m(m:&mut Memory<'_>,a:[u32;2],scratch:u32,h:&mut impl Host)->u32{
 assert_eq!(m.base,0);let at=m.offset(scratch);let mut saved=[0;96];saved.copy_from_slice(&m.bytes[at..at+96]);
 let rc=clear_db_m(&mut ClearLive{h,s:scratch},m.bytes,(a[0]!=0).then_some(a[0]),a[1]);
 m.bytes[at..at+96].copy_from_slice(&saved);rc
}

use crate::heap::Heap;
use crate::pronunciation::{LookupContext,LookupHost,LookupDictionary,LookupDictionaryState,terminated_len,append_terminated,read_offset,write_offset,text_at,string_differs,render_number};
#[derive(Clone,Copy)]
enum LookupWhich{Main,System,Language}
fn lookup_selected(ctx:&LookupContext,w:LookupWhich)->LookupDictionary{
 match w{LookupWhich::Main=>ctx.main,LookupWhich::System=>ctx.system,LookupWhich::Language=>ctx.language}
}
fn lookup_store(ctx:&mut LookupContext,w:LookupWhich,d:LookupDictionary){
 match w{LookupWhich::Main=>ctx.main=d,LookupWhich::System=>ctx.system=d,LookupWhich::Language=>ctx.language=d}
}
fn lookup_prefix_differs(arena:&[u8],key:u32,prefix:&[u8],n:i32)->bool{
 crate::frontend::compare_str_n(text_at(arena,key),prefix,n)!=0
}

pub trait LookupDictionariesM{
 fn load_m(&mut self,slot:u8,state:&mut LookupDictionaryState,key:u32,size:u32,resource:u32,heap:&mut impl Heap)->u32;
 fn unload_m(&mut self,slot:u8,state:&mut LookupDictionaryState,resource:u32,heap:&mut impl Heap)->u32;
 fn result_capacity_m(&mut self,slot:u8,state:&mut LookupDictionaryState,heap:&mut impl Heap)->u16;
 fn separator_m(&mut self,slot:u8,state:&mut LookupDictionaryState,heap:&mut impl Heap)->u8;
 #[allow(clippy::too_many_arguments)]
 fn lookup_m(&mut self,slot:u8,state:&mut LookupDictionaryState,text:u32,start:&mut i16,end:&mut i16,fold:bool,length:&mut i16,heap:&mut impl Heap)->bool;
}
struct LookupCleanupM {
    dictionary: LookupDictionary,
    which: LookupWhich,
    pending: bool,
    lookup_key: u32,
}

impl LookupCleanupM {
    #[allow(clippy::too_many_arguments)]
    fn finish(
        &self,
        ctx: &mut LookupContext,
        heap: &mut impl Heap,
        host: &mut impl LookupHost,
        dictionaries: &mut impl LookupDictionariesM,
        count: u16,
        out: &mut u32,
        rc: u32,
    ) -> u32 {
        if self.lookup_key != 0 {
            heap.release(self.lookup_key);
        }
        if rc < 0x80000000 {
            if count != 0 {
                *out = ctx.results;
            }
            return rc;
        }
        if self.pending {
            let slot = u32::from(ctx.dictionary_count) * 4;
            if ctx.keys != 0 && read_offset(heap.arena(), ctx.keys + slot) != 0 {
                let pointer = read_offset(heap.arena(), ctx.keys + slot);
                heap.release(pointer);
                write_offset(heap.arena(), ctx.keys + slot, 0);
            }
            if ctx.tables != 0 && read_offset(heap.arena(), ctx.tables + slot) != 0 {
                if ctx.resources != 0 && read_offset(heap.arena(), ctx.resources + slot) != 0 {
                    let mut state = self.dictionary.state;
                    state.tables = read_offset(heap.arena(), ctx.tables + slot);
                    let resource = read_offset(heap.arena(), ctx.resources + slot);

                    let mut selected = lookup_selected(ctx,self.which);
                    selected.state.tables = state.tables;
                    lookup_store(ctx,self.which,selected);
                    dictionaries.unload_m(self.which as u8,&mut state,resource,heap);
                    selected.state.bit_byte = state.bit_byte;
                    selected.state.bit_index = state.bit_index;
                    selected.state.buffer = state.buffer;
                    selected.state.h0 = state.h0;
                    selected.state.h1 = state.h1;
                    lookup_store(ctx,self.which,selected);
                }
                let pointer = read_offset(heap.arena(), ctx.tables + slot);
                heap.release(pointer);
                write_offset(heap.arena(), ctx.tables + slot, 0);
            }
            if ctx.resources != 0 && read_offset(heap.arena(), ctx.resources + slot) != 0 {
                let resource = read_offset(heap.arena(), ctx.resources + slot);
                host.resource_close(heap, resource);
                write_offset(heap.arena(), ctx.resources + slot, 0);
            }
        }
        rc
    }
}
fn lookup_find_dictionary(ctx: &LookupContext, arena: &[u8], lookup_key: u32, key: u32) -> i16 {
    let mut i = 0u16;
    while i < ctx.dictionary_count {
        let candidate = text_at(arena, read_offset(arena, ctx.keys + u32::from(i) * 4));
        let target: &[u8] = if lookup_key != 0 {
            text_at(arena, lookup_key)
        } else {
            text_at(arena,key)
        };
        if !string_differs(candidate, target) {
            return i as i16;
        }
        i += 1;
    }
    -1
}

pub fn lookup_dictionary_m(
    ctx: &mut LookupContext,
    heap: &mut impl Heap,
    host: &mut impl LookupHost,
    dictionaries: &mut impl LookupDictionariesM,
    handle: u32,
    key: u32,
    text: u32,
    out: &mut u32,
    count: &mut u16,
    separator: &mut u8,
) -> u32 {
    let mut maximum = 0u32;
    let which;
    let mut lookup_key = 0u32;
    let mut rc;
    let mut number = [0u8; 8];
    let mut length = 0i16;
    let mut multiple = true;
    let mut start;
    let mut end;

    rc = host.check(handle, 0xf37f, 0x2a4);
    if rc >= 0x80000000 {
        return 0x89802008;
    }
    if key == 0 || text == 0 {
        return 0x89802007;
    }
    if *count == 1 {
        let size = terminated_len(text_at(heap.arena(),key)) + 2;
        let Some(pointer) = heap.allocate(size) else {
            return 0x8980200a;
        };
        lookup_key = pointer;
        let arena = heap.arena();
        let at = lookup_key as usize;
        crate::frontend::copy_str(arena,lookup_key,key);
        append_terminated(&mut arena[at..], b"C\0");
        if !string_differs(text_at(heap.arena(),key), b"lidcfg")
            || !lookup_prefix_differs(heap.arena(),key, b"mertfrq", 7)
            || !lookup_prefix_differs(heap.arena(),key, b"merwfrq", 7)
        {
            which = LookupWhich::Language;
            if ctx.language.size == 0 {
                let cleanup = LookupCleanupM {
                    dictionary: ctx.language,
                    which: LookupWhich::Language,
                    pending: false,
                    lookup_key,
                };
                return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x89802007);
            }
        } else {
            which = LookupWhich::System;
            if ctx.system.size == 0 {
                let cleanup = LookupCleanupM {
                    dictionary: ctx.system,
                    which: LookupWhich::System,
                    pending: false,
                    lookup_key,
                };
                return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x89802007);
            }
        }
    } else {
        which = LookupWhich::Main;
    }
    if *count == 0xffff || *count == 1 {
        multiple = false;
        *count = 0;
    }
    let mut dictionary = lookup_selected(ctx,which);
    let mut cleanup = LookupCleanupM {
        dictionary,
        which,
        pending: false,
        lookup_key,
    };
    let index = lookup_find_dictionary(ctx, heap.arena(), lookup_key, key);
    if index >= 0 {
        dictionary.state.tables = read_offset(heap.arena(), ctx.tables + (index as u32) * 4);
        lookup_store(ctx,which,dictionary);
        cleanup.dictionary = dictionary;
    } else {
        cleanup.pending = true;
        let bytes = (u32::from(ctx.dictionary_count) + 2) * 4;
        let slot = u32::from(ctx.dictionary_count) * 4;
        let Some(pointer) = heap.reallocate(ctx.resources, bytes) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        ctx.resources = pointer;
        let arena = heap.arena();
        write_offset(arena, ctx.resources + slot, 0);
        write_offset(arena, ctx.resources + slot + 4, 0);
        let name: &[u8] = match which {
            LookupWhich::System => b"sysdct",
            LookupWhich::Language => b"lid",
            LookupWhich::Main => &ctx.name,
        };
        let mut name_buffer = [0u8; 256];
        let name_length = terminated_len(name) as usize;
        name_buffer[..name_length].copy_from_slice(&name[..name_length]);
        let mut resource = 0u32;
        rc = host.resource_open(heap, &name_buffer[..name_length], &mut resource);
        write_offset(heap.arena(), ctx.resources + slot, resource);
        if rc >= 0x80000000 {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, rc);
        }
        let Some(pointer) = heap.reallocate(ctx.tables, bytes) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        ctx.tables = pointer;
        let arena = heap.arena();
        write_offset(arena, ctx.tables + slot, 0);
        write_offset(arena, ctx.tables + slot + 4, 0);
        let Some(pointer) = heap.allocate_zeroed(1, 0x4c) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        write_offset(heap.arena(), ctx.tables + slot, pointer);
        dictionary.state.tables = pointer;

        lookup_store(ctx,which,dictionary);
        cleanup.dictionary = dictionary;
        rc = dictionaries.load_m(which as u8,&mut dictionary.state,key,dictionary.size,resource,heap);

        lookup_store(ctx,which,dictionary);
        cleanup.dictionary = dictionary;
        if rc >= 0x80000000 {
            if rc == 0x89802014 {
                let tables = read_offset(heap.arena(), ctx.tables + slot);
                heap.release(tables);
                write_offset(heap.arena(), ctx.tables + slot, 0);
                host.resource_close(heap, resource);
                write_offset(heap.arena(), ctx.resources + slot, 0);
                cleanup.pending = false;
                rc = 0;
            }
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, rc);
        }
        let Some(pointer) = heap.reallocate(ctx.keys, bytes) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        ctx.keys = pointer;
        let arena = heap.arena();
        write_offset(arena, ctx.keys + slot, 0);
        write_offset(arena, ctx.keys + slot + 4, 0);

        let source = if lookup_key != 0 {lookup_key}else{key};
        let source_length = terminated_len(text_at(heap.arena(),source)) as usize + 1;
        let Some(pointer) = heap.allocate(source_length as u32) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        write_offset(heap.arena(), ctx.keys + slot, pointer);
        crate::frontend::copy_str(heap.arena(),pointer,source);
        maximum = u32::from(dictionaries.result_capacity_m(which as u8,&mut dictionary.state,heap));
        lookup_store(ctx,which,dictionary);
        cleanup.dictionary = dictionary;
        if maximum == 0 {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x89802007);
        }
        if u32::from(ctx.result_capacity) < maximum {
            let Some(pointer) = heap.reallocate(dictionary.state.output, maximum + 1) else {
                return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
            };
            dictionary.state.output = pointer;
            ctx.main.state.output = pointer;
            ctx.system.state.output = pointer;
            ctx.language.state.output = pointer;
        }
        lookup_store(ctx,which,dictionary);
        cleanup.dictionary = dictionary;
        cleanup.pending = false;
        ctx.dictionary_count += 1;
    }
    *separator = dictionaries.separator_m(which as u8,&mut dictionary.state,heap);
    lookup_store(ctx,which,dictionary);
    cleanup.dictionary = dictionary;
    let text_length = terminated_len(text_at(heap.arena(),text));
    if i32::from(ctx.query_capacity) < (text_length.wrapping_add(5) as u16 as i16) as i32 {
        let capacity = (text_length + 5) as u16;
        let Some(pointer) = heap.reallocate(ctx.query, u32::from(capacity)) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        ctx.query_capacity = capacity as i16;
        ctx.query = pointer;
    }
    *count = 0;
    *out = ctx.results;
    if u32::from(ctx.result_capacity) < maximum {
        let mut i = 0u16;
        while i32::from(i) < i32::from(ctx.result_blocks) * 5 - 1 {
            let pointer = read_offset(heap.arena(), ctx.results + u32::from(i) * 4);
            let Some(pointer) = heap.reallocate(pointer, maximum + 1) else {
                return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
            };
            write_offset(heap.arena(), ctx.results + u32::from(i) * 4, pointer);
            i += 1;
        }
        ctx.result_capacity = maximum as u16;
    }
    loop {
        if *count != 0 && !multiple {
            break;
        }
        start = 0;
        let at = ctx.query as usize;
        crate::frontend::copy_str(heap.arena(),ctx.query,text);
        if *count != 0 {
            append_terminated(&mut heap.arena()[at..], b"\x17\0");
            render_number(u32::from(*count), &mut number, 10);
            let rendered_length = terminated_len(&number) as usize + 1;
            let mut rendered = [0u8; 8];
            rendered[..rendered_length].copy_from_slice(&number[..rendered_length]);
            append_terminated(&mut heap.arena()[at..], &rendered[..rendered_length]);
        }
        end = terminated_len(text_at(heap.arena(), ctx.query)).wrapping_sub(1) as i16;
        if !dictionaries.lookup_m(which as u8,
            &mut dictionary.state,ctx.query,&mut start,&mut end,false,&mut length,heap,
        ) {
            lookup_store(ctx,which,dictionary);
            break;
        }
        lookup_store(ctx,which,dictionary);
        if u32::from(*count) + 2 > u32::from(ctx.result_blocks) * 5 {
            let blocks = ctx.result_blocks + 1;
            let Some(pointer) = heap.reallocate(ctx.results, u32::from(blocks) * 20) else {
                return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
            };
            ctx.results = pointer;
            let mut i = if ctx.result_blocks == 0 {
                0u16
            } else {
                ctx.result_blocks * 5 - 1
            };
            let at = ctx.results as usize + u32::from(i) as usize * 4;
            let filled = u32::from(blocks * 5 - i) as usize * 4;
            heap.arena()[at..at + filled].fill(0);
            while i < blocks * 5 - 1 {
                let Some(pointer) = heap.allocate(u32::from(ctx.result_capacity) + 1) else {
                    return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
                };
                write_offset(heap.arena(), ctx.results + u32::from(i) * 4, pointer);
                i += 1;
            }
            write_offset(heap.arena(), ctx.results + u32::from(i) * 4, 0);
            ctx.result_blocks += 1;
        }
        let arena = heap.arena();
        let destination = read_offset(arena, ctx.results + u32::from(*count) * 4) as usize;
        let source = dictionary.state.output as usize;
        let written = length.max(0) as usize;
        for j in 0..written {
            arena[destination + j] = arena[source + j];
        }
        arena[destination + written] = 0;
        *count += 1;
    }
    cleanup.finish(ctx, heap, host, dictionaries, *count, out, rc)
}

fn lookup_state_read(m:&Memory<'_>,p:u32)->LookupDictionaryState{
 LookupDictionaryState{bit_byte:m.word(p),bit_index:m.byte(p+4),buffer:m.word(p+8),tables:m.word(p+12),h0:m.word(p+16),h1:m.word(p+20),output:m.word(p+24)}
}
fn lookup_state_write(m:&mut Memory<'_>,p:u32,s:LookupDictionaryState){
 m.put_word(p,s.bit_byte);m.put_byte(p+4,s.bit_index);m.put_word(p+8,s.buffer);m.put_word(p+12,s.tables);m.put_word(p+16,s.h0);m.put_word(p+20,s.h1);m.put_word(p+24,s.output);
}
fn lookup_context_read(m:&Memory<'_>,p:u32)->LookupContext{
 let owner=m.word(p);let half=|o:u32|{let at=m.offset(p+o);u16::from_le_bytes(m.bytes[at..at+2].try_into().unwrap())};
 let mut c=LookupContext{
  owner:crate::pronunciation::LookupOwner{heap:m.word(owner+4),resources:m.word(owner+12),logger:m.word(owner+16)},
  main:LookupDictionary{size:m.word(p+4),state:lookup_state_read(m,p+8)},
  system:LookupDictionary{size:m.word(p+0x24),state:lookup_state_read(m,p+0x28)},
  language:LookupDictionary{size:m.word(p+0x44),state:lookup_state_read(m,p+0x48)},
  field64:m.word(p+0x64),keys:m.word(p+0x68),tables:m.word(p+0x6c),results:m.word(p+0x70),query:m.word(p+0x74),resources:m.word(p+0x78),
  dictionary_count:half(0x88),result_blocks:half(0x8a),result_capacity:half(0x8c),query_capacity:half(0x8e)as i16,
  table90:m.word(p+0x90),mode9c:half(0x9c),flag29e:m.byte(p+0x29e),table2a0:m.word(p+0x2a0),..Default::default()
 };
 let at=m.offset(p+0x9e);c.name.copy_from_slice(&m.bytes[at..at+256]);c.name2.copy_from_slice(&m.bytes[at+256..at+512]);c
}
fn lookup_context_write(m:&mut Memory<'_>,p:u32,c:&LookupContext){
 for(i,d)in [c.main,c.system,c.language].iter().enumerate(){lookup_state_write(m,p+8+i as u32*32,d.state);}
 for(o,v)in [(0x68,c.keys),(0x6c,c.tables),(0x70,c.results),(0x74,c.query),(0x78,c.resources)]{m.put_word(p+o,v);}
 for(o,v)in [(0x88,c.dictionary_count),(0x8a,c.result_blocks),(0x8c,c.result_capacity),(0x8e,c.query_capacity as u16)]{m.put_half(p+o,v);}
}
struct LookupHeap<'a,'m,'h,H>{m:&'a mut Memory<'m>,heap:u32,h:&'a core::cell::RefCell<&'h mut H>}
impl<H:Host> Heap for LookupHeap<'_, '_, '_,H>{
 fn allocate(&mut self,n:u32)->Option<u32>{let p=self.h.borrow_mut().call_m(0x196394,&[self.heap,n],self.m);(p!=0&&p<0x80000000).then_some(p)}
 fn allocate_zeroed(&mut self,n:u32,size:u32)->Option<u32>{let p=self.h.borrow_mut().call_m(0x1963b8,&[self.heap,n,size],self.m);(p!=0&&p<0x80000000).then_some(p)}
 fn reallocate(&mut self,p:u32,n:u32)->Option<u32>{let p=self.h.borrow_mut().call_m(0x1963e0,&[self.heap,p,n],self.m);(p!=0&&p<0x80000000).then_some(p)}
 fn release(&mut self,p:u32){self.h.borrow_mut().call_m(0x196428,&[self.heap,p],self.m);}
 fn arena(&mut self)->&mut[u8]{self.m.bytes}
}
struct LookupServices<'a,'h,H>{p:u32,resources:u32,s:u32,h:&'a core::cell::RefCell<&'h mut H>}
impl<H:Host> LookupHost for LookupServices<'_, '_,H>{
 fn check(&mut self,tag:u32,expected:u32,size:u32)->u32{crate::normalization::quad_check(self.p,tag,expected,size)}
 fn resource_open(&mut self,heap:&mut impl Heap,name:&[u8],out:&mut u32)->u32{
  let mut m=Memory{base:0,bytes:heap.arena()};assert!(name.len()<256);let at=m.offset(self.s+128);
  m.bytes[at..at+name.len()].copy_from_slice(name);m.bytes[at+name.len()]=0;m.put_word(self.s,0);
  let rc=self.h.borrow_mut().call_m(0x179f30,&[self.resources,self.s+128,self.s],&mut m);*out=m.word(self.s);rc
 }
 fn resource_close(&mut self,heap:&mut impl Heap,handle:u32){self.h.borrow_mut().call_m(0x179f3c,&[self.resources,handle],&mut Memory{base:0,bytes:heap.arena()});}
 fn resource_release(&mut self,_:u32,_:u32){panic!("lookup does not use resource_release")}
}
struct LookupDictionaryLive<'a,'h,H>{p:u32,s:u32,h:&'a core::cell::RefCell<&'h mut H>,fault:Option<u32>}
impl<H:Host> LookupDictionaryLive<'_, '_,H>{
 fn state_at(&self,slot:u8)->u32{assert!(slot<3);self.p+8+u32::from(slot)*32}
 fn header<'a>(&self,heap:&'a mut impl Heap,state:&LookupDictionaryState)->crate::morphology::DictionaryHeader<'a>{
  let bytes=heap.arena();let header=mem_word(bytes,state.tables)as usize;
  crate::morphology::DictionaryHeader{bytes:&bytes[header..header+76]}
 }
}
impl<H:Host> LookupDictionariesM for LookupDictionaryLive<'_, '_,H>{
 fn load_m(&mut self,slot:u8,state:&mut LookupDictionaryState,key:u32,size:u32,resource:u32,heap:&mut impl Heap)->u32{
  let p=self.state_at(slot);let mut m=Memory{base:0,bytes:heap.arena()};lookup_state_write(&mut m,p,*state);
  let rc=self.h.borrow_mut().call_m(0x26b7ec,&[p,key,size,resource],&mut m);*state=lookup_state_read(&m,p);
  if rc==0x80002001{self.fault=Some(0x26b7ec);}rc
 }
 fn unload_m(&mut self,slot:u8,state:&mut LookupDictionaryState,resource:u32,heap:&mut impl Heap)->u32{
  let p=self.state_at(slot);let mut m=Memory{base:0,bytes:heap.arena()};lookup_state_write(&mut m,p,*state);
  let rc=self.h.borrow_mut().call_m(0x26bb28,&[p,resource],&mut m);*state=lookup_state_read(&m,p);
  if rc==0x80002001{self.fault=Some(0x26bb28);}rc
 }
 fn result_capacity_m(&mut self,slot:u8,state:&mut LookupDictionaryState,heap:&mut impl Heap)->u16{
  self.state_at(slot);self.header(heap,state).result_capacity().expect("initialized dictionary header")
 }
 fn separator_m(&mut self,slot:u8,state:&mut LookupDictionaryState,heap:&mut impl Heap)->u8{
  self.state_at(slot);self.header(heap,state).separator().expect("initialized dictionary header")
 }
 fn lookup_m(&mut self,slot:u8,state:&mut LookupDictionaryState,text:u32,start:&mut i16,end:&mut i16,fold:bool,length:&mut i16,heap:&mut impl Heap)->bool{
  let p=self.state_at(slot);let mut m=Memory{base:0,bytes:heap.arena()};lookup_state_write(&mut m,p,*state);
  m.put_half(self.s+16,*start as u16);m.put_half(self.s+18,*end as u16);m.put_half(self.s+20,*length as u16);
  let rc=self.h.borrow_mut().call_m(0x26bb6c,&[p,text,self.s+16,self.s+18,u32::from(fold),self.s+20],&mut m);
  *state=lookup_state_read(&m,p);
  let half=|o:u32|{let at=m.offset(self.s+o);u16::from_le_bytes(m.bytes[at..at+2].try_into().unwrap())as i16};
  *start=half(16);*end=half(18);*length=half(20);
  if rc>1{self.fault=Some(0x26bb6c);false}else{rc!=0}
 }
}

pub fn lookup_dictionary_live_m(m:&mut Memory<'_>,a:[u32;7],scratch:u32,h:&mut impl Host)->Result<u32,crate::engine::Error>{
 assert_eq!(m.base,0);
 if crate::normalization::quad_check(a[0],a[1],0xf37f,0x2a4)>=0x80000000{return Ok(0x89802008);}
 if a[2..].contains(&0){return Err(crate::engine::Error::UnsupportedStage{entry:0x173dc0});}
 let at=m.offset(scratch);let mut saved=[0;512];saved.copy_from_slice(&m.bytes[at..at+512]);
 let mut c=lookup_context_read(m,a[0]);
 m.put_word(scratch+16,a[3]);m.put_word(scratch+20,a[2]);
 h.call_m(0x1738e8,&[c.owner.logger,0x8f8f44,5,0,0x1ea89c,scratch+16],m);
 let mut out=m.word(a[4]);let count_at=m.offset(a[5]);
 let mut count=u16::from_le_bytes(m.bytes[count_at..count_at+2].try_into().unwrap());let mut sep=m.byte(a[6]);
 let cell=core::cell::RefCell::new(h);
 let mut heap=LookupHeap{m,heap:c.owner.heap,h:&cell};
 let mut services=LookupServices{p:a[0],resources:c.owner.resources,s:scratch,h:&cell};
 let mut dictionaries=LookupDictionaryLive{p:a[0],s:scratch,h:&cell,fault:None};
 let rc=lookup_dictionary_m(&mut c,&mut heap,&mut services,&mut dictionaries,a[1],a[2],a[3],&mut out,&mut count,&mut sep);
 let fault=dictionaries.fault;
 lookup_context_write(m,a[0],&c);m.put_word(a[4],out);m.put_half(a[5],count);m.put_byte(a[6],sep);
 if rc==0x8980200a{cell.borrow_mut().call_m(0x173dc0,&[c.owner.logger,0x8f8f44,0xa028,0,0,0],m);}
 cell.borrow_mut().call_m(0x1738e8,&[c.owner.logger,0x8f8f44,4,0,0x1eae7c,scratch+16],m);
 m.bytes[at..at+512].copy_from_slice(&saved);
 if let Some(entry)=fault{Err(crate::engine::Error::UnsupportedStage{entry})}else{Ok(rc)}
}

const DICTIONARY_NODES:usize=512;

pub struct DictionaryWorkspace{
 trees:[[[u16;2];DICTIONARY_NODES];5],
 count_values:[u16;DICTIONARY_NODES],skip_values:[u16;DICTIONARY_NODES*2+2],offset_values:[u16;DICTIONARY_NODES*2+2],
 characters:[u8;DICTIONARY_NODES],deltas:[u8;DICTIONARY_NODES],
 alphabet:[u8;256],pairs:[[u8;2];256],initial_characters:[u8;256],initial_offsets:[u32;256],initial_indices:[u32;256],
 query:[u8;1024],output:[u8;1024],
}
impl Default for DictionaryWorkspace{
 fn default()->Self{Self{
  trees:[[[0;2];DICTIONARY_NODES];5],count_values:[0;DICTIONARY_NODES],skip_values:[0;DICTIONARY_NODES*2+2],offset_values:[0;DICTIONARY_NODES*2+2],
  characters:[0;DICTIONARY_NODES],deltas:[0;DICTIONARY_NODES],alphabet:[0;256],pairs:[[0;2];256],
  initial_characters:[0;256],initial_offsets:[0;256],initial_indices:[0;256],query:[0;1024],output:[0;1024],
 }}
}
fn dictionary_half(m:&Memory<'_>,p:u32)->u16{let at=m.offset(p);u16::from_le_bytes(m.bytes[at..at+2].try_into().unwrap())}
fn dictionary_tree(m:&Memory<'_>,ptr:u32,root:u16,out:&mut[[u16;2];DICTIONARY_NODES])->Result<(usize,usize),crate::engine::Error>{
 let mut visited=[false;DICTIONARY_NODES];let mut todo=[0u16;DICTIONARY_NODES];let mut used=1;todo[0]=root;let mut span=0;let mut leaves=0;
 while used!=0{
  used-=1;let index=todo[used]as usize;if index>=DICTIONARY_NODES{return Err(crate::engine::Error::InputTooLong);}if visited[index]{continue;}visited[index]=true;
  let node=[dictionary_half(m,ptr+index as u32*4),dictionary_half(m,ptr+index as u32*4+2)];out[index]=node;span=span.max(index+1);
  if node[0]==u16::MAX{leaves=leaves.max(index+1);}else{
   for child in node{if used==todo.len(){return Err(crate::engine::Error::InputTooLong);}todo[used]=child;used+=1;}
  }
 }
 Ok((span,leaves))
}
struct DictionaryBrokerPages<'a,'m,H>{m:&'a mut Memory<'m>,p:u32,h:&'a mut H,last:u32,fault:Option<u32>}
impl<H:Host> crate::morphology::DictionaryPages for DictionaryBrokerPages<'_, '_,H>{
 fn read(&mut self,offset:u32,n:u32)->Result<&[u8],crate::frontend_types::Error>{
  let args=[self.m.word(self.p+16),self.m.word(self.p+20),offset,n,self.p+8];
  let rc=self.h.call_m(0x17a1a4,&args,self.m);
  if rc>=0x80000000{if rc==0x80002001{self.fault=Some(0x17a1a4);}return Err(crate::frontend_types::Error::InvalidResource);}
  self.last=self.m.word(self.p+8);let at=self.m.offset(self.last);
  Ok(&self.m.bytes[at..at+n as usize])
 }
 fn release(&mut self){
  let args=[self.m.word(self.p+16),self.m.word(self.p+20),self.last];
  let rc=self.h.call_m(0x179208,&args,self.m);if rc==0x80002001{self.fault=Some(0x179208);}
 }
}

pub fn dictionary_lookup_entry_m(m:&mut Memory<'_>,a:[u32;6],w:&mut DictionaryWorkspace,h:&mut impl Host)->Result<u32,crate::engine::Error>{
 use crate::morphology::{BitCursor,TrieCursor,TrieTables,EntryTables,OffsetTable};
 assert_eq!(m.base,0);let p=a[0];let table=m.word(p+12);let header=m.word(table);
 let bh=|o|m.byte(header+o);let hh=|o|dictionary_half(m,header+o);let roots=[hh(26),hh(28),hh(30),hh(32),hh(34)];
 let mut spans=[0;5];let mut leaves=[0;5];
 for i in 0..5{let(a,b)=dictionary_tree(m,m.word(table+24+i as u32*4),roots[i],&mut w.trees[i])?;spans[i]=a;leaves[i]=b;}
 let copy_bytes=|dest:&mut[u8],ptr:u32,n:usize|{let at=m.offset(ptr);dest[..n].copy_from_slice(&m.bytes[at..at+n]);};
 let alphabet_len=256-usize::from(bh(4));copy_bytes(&mut w.alphabet,m.word(table+4),alphabet_len);
 let initial=hh(46)as usize;if initial>256{return Err(crate::engine::Error::InputTooLong);}
 copy_bytes(&mut w.initial_characters,m.word(table+12),initial);
 for i in 0..initial{w.initial_offsets[i]=m.word(m.word(table+16)+i as u32*4);w.initial_indices[i]=m.word(m.word(table+20)+i as u32*4);}
 for i in 0..leaves[0]{w.count_values[i]=dictionary_half(m,m.word(table+44)+i as u32*2);}
 copy_bytes(&mut w.characters,m.word(table+48),leaves[1]);copy_bytes(&mut w.deltas,m.word(table+52),leaves[2]);
 let skip_len=if bh(13)<=16{leaves[3]}else{(leaves[3]*usize::from(bh(13))+15)/16+2};
 let offset_len=(leaves[4]*usize::from(bh(12))+15)/16+2;
 if skip_len>w.skip_values.len()||offset_len>w.offset_values.len(){return Err(crate::engine::Error::InputTooLong);}
 for i in 0..skip_len{w.skip_values[i]=dictionary_half(m,m.word(table+56)+i as u32*2);}
 for i in 0..offset_len{w.offset_values[i]=dictionary_half(m,m.word(table+60)+i as u32*2);}
 let pairs=256-usize::from(bh(6));for i in 0..pairs{let ptr=m.word(table+8)+i as u32*2;w.pairs[i]=[m.byte(ptr),m.byte(ptr+1)];}
 let query_len=terminated_len(text_at(m.bytes,a[1]))as usize+1;
 if query_len>w.query.len(){return Err(crate::engine::Error::InputTooLong);}
 let at=m.offset(a[1]);w.query[..query_len].copy_from_slice(&m.bytes[at..at+query_len]);
 let output_capacity=usize::from(hh(48))+1;
 if output_capacity>w.output.len(){return Err(crate::engine::Error::InputTooLong);}
 let offsets=OffsetTable{tree:&w.trees[4][..spans[4]],root:roots[4],values:&w.offset_values[..offset_len],width:bh(12),inline_limit:bh(18),shift:bh(14)};
 let trie=TrieTables{
  alphabet:&w.alphabet[..alphabet_len],first_character:bh(4),initial_characters:&w.initial_characters[..initial],
  initial_offsets:&w.initial_offsets[..initial],initial_indices:&w.initial_indices[..initial],initial_position:m.word(header+56),initial_bit:bh(17),direct_count:hh(24),
  count_tree:&w.trees[0][..spans[0]],count_root:roots[0],counts:&w.count_values[..leaves[0]],
  character_tree:&w.trees[1][..spans[1]],character_root:roots[1],characters:&w.characters[..leaves[1]],
  delta_tree:&w.trees[2][..spans[2]],delta_root:roots[2],deltas:&w.deltas[..leaves[2]],
  skipped_tree:&w.trees[3][..spans[3]],skipped_root:roots[3],skipped_values:&w.skip_values[..skip_len],skipped_width:bh(13),offsets,
  terminator:bh(16),inline_limit:bh(18),alignment_mask:bh(15),data_offset:m.word(table+64),page_size:hh(50),fallback_bits:bh(12),fallback_shift:bh(14),
 };
 let entries=EntryTables{trie,pairs:&w.pairs[..pairs],pair_base:bh(6),input_min:bh(4),input_max:bh(5),separator:bh(8),prefix_min:bh(10),prefix_max:bh(11),
  max_length:hh(22)as i16,offset_bits:bh(19),offset_size:hh(54),offset_bytes:m.word(header+68),offsets_offset:m.word(table+72),record_size:hh(52),records_offset:m.word(table+68)};
 let mut cursor=TrieCursor{bits:BitCursor{position:m.word(p),bit:m.byte(p+4)},page_offset:None};
 let start=dictionary_half(m,a[2])as i16;let end=dictionary_half(m,a[3])as i16;let mut length=dictionary_half(m,a[5])as i16;
 let last=m.word(p+8);let mut pages=DictionaryBrokerPages{m,p,h,last,fault:None};
 let result=cursor.lookup_entry(&w.query[..query_len],start,end,a[4]!=0,entries,&mut pages,&mut w.output[..output_capacity],&mut length);
 let fault=pages.fault;
 m.put_word(p,cursor.bits.position);m.put_byte(p+4,cursor.bits.bit);
 if let Some(entry)=fault{return Err(crate::engine::Error::UnsupportedStage{entry});}
 let found=result.map_err(|_|crate::engine::Error::UnsupportedStage{entry:0x26bb6c})?;
 if found{let output=m.word(p+24);let at=m.offset(output);let n=usize::try_from(length).expect("nonnegative dictionary result")+1;m.bytes[at..at+n].copy_from_slice(&w.output[..n]);m.put_half(a[5],length as u16);}
 Ok(u32::from(found))
}

pub fn unmap_environment_us(scratch:u32)->crate::boot_part_e::UnmapEnvironment{
 unmap_environment_rom(scratch,crate::rom_region::Rom::Us)
}

pub fn unmap_environment_rom(scratch:u32,rom:crate::rom_region::Rom)->crate::boot_part_e::UnmapEnvironment{
 let addr=|us| crate::rom_region::addr(rom,us);
 crate::boot_part_e::UnmapEnvironment{category:addr(0x8d7668),begin_format:addr(0x179310),end_format:addr(0x17935c),error_format:addr(0x17934c),labels:[addr(0x179344),addr(0x17933c),addr(0x1791d8)],scratch}
}

pub fn broker_range_live_m(m:&mut Memory<'_>,f:u32,a:&[u32],scratch:u32,h:&mut impl Host)->u32{
 let env=unmap_environment_rom(scratch+16,h.rom());
 let mut calls=ProcessCalls{h,log_s:scratch};
 match f{
  0x17a1a4=>crate::leftovers_1::fn_17a1a4(m,a[0],a[1],a[2],a[3],a[4],scratch,&env,&mut calls),
  0x179208=>crate::boot_part_e::fn_179208(m,a[0],a[1],a[2],&env,&mut calls),
  0x17c9d8=>crate::boot_part_e::fn_17c9d8(m,a[0],a[1],a[2],a[3],a[4],&mut calls),
  0x179374=>crate::boot_part_b::fn_179374(m,a[0],a[1],a[2],a[3],a[4],&mut calls),
  0x1b2760=>crate::speech_part_1::fn_1b2760(m,a[0],a[1],&mut calls),
  0x172de0=>crate::speech_part_1::fn_172de0(m,a[0],a[1],a[2],&mut calls),
  0x172d30=>crate::speech_part_1::fn_172d30(m,a[0],a[1],a[2],a[3],&mut calls),
  0x4a794c=>crate::speech_part_3::fn_4a794c(m,a[0],a[1],a[2],a[3]),
  0x4a78f0=>crate::speech_part_3::fn_4a78f0(m,a[0],a[1],a[2],a[3]),
  0x1b254c=>crate::speech_part_1::fn_1b254c(m,a[0],a[1],a[2],scratch+84,&mut calls),
  0x1b2974=>crate::speech_part_1::fn_1b2974(m,a[0],a[1],a[2],a[3],&mut calls),
  _=>panic!("broker-range ABI"),
 }
}

impl<H:Host> crate::speech_part_1::StreamHost for ProcessCalls<'_,H>{
 fn read(&mut self,m:&mut Memory<'_>,method:u32,dst:Option<u32>,n:u32,stream:u32)->u32{
  let local=self.log_s+80;
  let saved=if dst.is_none(){assert_eq!(n,4);Some(m.word(local))}else{None};
  let method=crate::rom_region::to_us_guest(self.h.rom(),method);
  let rc=self.h.call_m(method,&[dst.unwrap_or(local),1,n,stream],m);
  if let Some(value)=saved{m.put_word(local,value);}
  rc
 }
}

pub fn fn_240b28(c:u32)->u32{
 if c.wrapping_sub(0x41)<=0x19{return (c+0x20)&0xff;}
 match c{
  0x80=>0x87,0x8e=>0x84,0x8f=>0x86,0x90=>0x82,
  0x92=>0x91,0x99=>0x94,0x9a=>0x81,0xa5=>0xa4,
  _=>c,
 }
}

pub fn fn_195dac_m(m:&mut Memory<'_>,dst:u32,mut src:u32,n:i32)->u32{
 let mut at=dst;
 while m.byte(at)!=0{at=at.wrapping_add(1);}
 let mut left=n;
 while left>0{
  left-=1;
  let value=m.byte(src);src=src.wrapping_add(1);
  m.bytes[m.offset(at)]=value;at=at.wrapping_add(1);
  if value==0{return dst;}
 }
 m.bytes[m.offset(at)]=0;
 dst
}

pub fn fn_1852d8_m(m:&mut Memory<'_>,p:u32,v:u16)->u16{
 let at=p+0x34;
 let old=u16::from_le_bytes(m.bytes[m.offset(at)..m.offset(at)+2].try_into().unwrap());
 m.put_half(at,v);
 old
}

pub fn fn_1853c4_m(m:&mut Memory<'_>,p:u32,v:i8)->i32{
 let at=p+0x38;
 let old=m.byte(at) as i8;
 m.bytes[m.offset(at)]=v as u8;
 old as i32
}

pub fn fn_26bb54_m(m:&mut Memory<'_>,p:u32)->u8{
 let inner=m.word(m.word(p+0xc));
 m.byte(inner+9)
}

use crate::depes;

pub const VM_LAYERS: usize = 5;
pub const VM_CODE_MAX: usize = 65536;
pub const VM_RULE_MAX: usize = 4096;
pub const VM_MODULE_MAX: usize = 128;

pub const VM_CLASS_MAX: usize = 162 * 256;

pub const VM_GROUP_MAX: usize = 162 * 64;
pub const VM_MATRIX_MAX: usize = 64 * 256;
pub const VM_KIND_MAX: usize = 256;
pub const VM_CAPTURE_TYPE_MAX: usize = 512;
pub const VM_CAPTURE_MAX: usize = 256;

pub const VM_ROW_MAX: usize = 30000;
pub const VM_FRAME_MAX: usize = 50;
pub const VM_SCANNER_MAX: usize = 64;

pub struct VmWorkspace {
    code: [u8; VM_CODE_MAX],
    rules: [usize; VM_RULE_MAX],
    offsets: [u8; VM_RULE_MAX],
    minimums: [u8; VM_RULE_MAX],
    lengths: [u8; VM_RULE_MAX],
    jumps: [u8; VM_RULE_MAX],
    groups: [u8; VM_RULE_MAX],
    required_first: [u8; VM_RULE_MAX],
    required_second: [u8; VM_RULE_MAX],
    module_starts: [u16; VM_MODULE_MAX],
    module_ends: [i16; VM_MODULE_MAX],
    kinds: [u8; VM_KIND_MAX],
    matrix: [u8; VM_MATRIX_MAX],
    classes: [u8; VM_CLASS_MAX],
    group_table: [u8; VM_GROUP_MAX],
    values: [u8; VM_GROUP_MAX],
    capture_types: [u8; VM_CAPTURE_TYPE_MAX],
    rows: [[u8; VM_ROW_MAX]; VM_LAYERS + 1],
    captures: [u8; VM_CAPTURE_MAX],
    frames: [depes::RepeatFrame; VM_FRAME_MAX],
    scanner: [depes::ScanFrame; VM_SCANNER_MAX],
}

impl Default for VmWorkspace {
    fn default() -> Self {
        Self {
            code: [0; VM_CODE_MAX],
            rules: [0; VM_RULE_MAX],
            offsets: [0; VM_RULE_MAX],
            minimums: [0; VM_RULE_MAX],
            lengths: [0; VM_RULE_MAX],
            jumps: [0; VM_RULE_MAX],
            groups: [0; VM_RULE_MAX],
            required_first: [0; VM_RULE_MAX],
            required_second: [0; VM_RULE_MAX],
            module_starts: [0; VM_MODULE_MAX],
            module_ends: [0; VM_MODULE_MAX],
            kinds: [0; VM_KIND_MAX],
            matrix: [0; VM_MATRIX_MAX],
            classes: [0; VM_CLASS_MAX],
            group_table: [0; VM_GROUP_MAX],
            values: [0; VM_GROUP_MAX],
            capture_types: [0; VM_CAPTURE_TYPE_MAX],
            rows: [[0; VM_ROW_MAX]; VM_LAYERS + 1],
            captures: [0; VM_CAPTURE_MAX],
            frames: [depes::RepeatFrame::default(); VM_FRAME_MAX],
            scanner: [depes::ScanFrame::default(); VM_SCANNER_MAX],
        }
    }
}

pub trait VmExpansion {

    fn post_expand(&mut self, _ends: &[i16], _m: &mut Memory<'_>) {}
    fn expand_m(
        &mut self,
        handle: u32,
        selector: u32,
        code: u8,
        text: &[u8],
        out: &mut [u8],
        ends: &mut [i16],
        m: &mut Memory<'_>,
    ) -> Result<(i16, u8), crate::frontend_types::Error>;
}

struct VmExpansionCall<'a, 'm, H> {
    m: &'a mut Memory<'m>,
    host: &'a mut H,
    handle: u32,
    selector: u32,
    out: [u8; VM_ROW_MAX],
    ends: [i16; VM_LAYERS + 1],
}

impl<H: VmExpansion> depes::ExpansionCallback for VmExpansionCall<'_, '_, H> {
    fn expand(
        &mut self,
        code: u8,
        text: &mut [u8],
    ) -> Result<depes::Expansion<'_>, crate::frontend_types::Error> {
        let (status, separator) = self.host.expand_m(
            self.handle,
            self.selector,
            code,
            text,
            &mut self.out,
            &mut self.ends,
            self.m,
        )?;
        Ok(depes::Expansion {
            status,
            separator,
            text: &mut self.out,
            ends: &mut self.ends,
        })
    }
}

impl<H: VmExpansion> VmExpansionCall<'_, '_, H> {

    fn expanded(&mut self) {
        let (host, ends, m) = (&mut self.host, &self.ends, &mut *self.m);
        host.post_expand(ends, m);
    }
}

fn vm_u16(m: &Memory<'_>, p: u32) -> u16 {
    let at = m.offset(p);
    u16::from_le_bytes(m.bytes[at..at + 2].try_into().unwrap())
}

fn vm_i16(m: &Memory<'_>, p: u32) -> i16 {
    vm_u16(m, p) as i16
}

fn vm_u8(m: &Memory<'_>, p: u32) -> u8 {
    m.bytes[m.offset(p)]
}

fn vm_put_u8(m: &mut Memory<'_>, p: u32, v: u8) {
    let at = m.offset(p);
    m.bytes[at] = v;
}

fn vm_put_u16(m: &mut Memory<'_>, p: u32, v: u16) {
    let at = m.offset(p);
    m.bytes[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

fn vm_put_u32(m: &mut Memory<'_>, p: u32, v: u32) {
    let at = m.offset(p);
    m.bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn vm_span(
    m: &Memory<'_>,
    ctx: u32,
    off: u32,
    next: u32,
) -> Result<usize, crate::frontend_types::Error> {
    let first = m.word(ctx + off);
    let second = m.word(ctx + next);
    if second < first {
        return Err(crate::frontend_types::Error::InvalidResource);
    }
    Ok((second - first) as usize)
}

fn vm_copy(m: &Memory<'_>, src: u32, dst: &mut [u8]) -> Result<(), crate::frontend_types::Error> {
    let at = m.offset(src);
    let bytes = m
        .bytes
        .get(at..at + dst.len())
        .ok_or(crate::frontend_types::Error::InvalidResource)?;
    dst.copy_from_slice(bytes);
    Ok(())
}

fn vm_copy_table(
    m: &Memory<'_>,
    ctx: u32,
    off: u32,
    next: u32,
    dst: &mut [u8],
) -> Result<usize, crate::frontend_types::Error> {
    let len = vm_span(m, ctx, off, next)?;
    if len > dst.len() {
        return Err(crate::frontend_types::Error::Capacity {
            required: len,
            available: dst.len(),
        });
    }
    vm_copy(m, m.word(ctx + off), &mut dst[..len])?;
    Ok(len)
}

fn vm_copy_out(
    m: &mut Memory<'_>,
    dst: u32,
    src: &[u8],
) -> Result<(), crate::frontend_types::Error> {
    let at = m.offset(dst);
    let target = m
        .bytes
        .get_mut(at..at + src.len())
        .ok_or(crate::frontend_types::Error::InvalidResource)?;
    target.copy_from_slice(src);
    Ok(())
}

fn vm_layer_index(layers: &[u32; VM_LAYERS], ptr: u32) -> Option<usize> {
    layers.iter().position(|value| *value == ptr)
}

fn vm_layer_ptr(layers: &[u32; VM_LAYERS], index: Option<usize>, original: u32) -> u32 {
    match index {
        Some(value) if value < layers.len() => layers[value],
        _ => original,
    }
}

pub fn vm_run_m(
    m: &mut Memory<'_>,
    ctx: u32,
    module: u32,
    start: i32,
    end_ptr: u32,
    w: &mut VmWorkspace,
    host: &mut impl VmExpansion,
) -> Result<u8, crate::frontend_types::Error> {
    if vm_u8(m, ctx + 0x37b) != 0 { return Ok(0); }
    let layer_count = vm_u16(m, ctx + 0x0c) as usize;
    if layer_count == 0 || layer_count > VM_LAYERS {
        return Err(crate::frontend_types::Error::InvalidResource);
    }
    let stride = m.word(ctx + 0x1c) as usize;
    let reset_value = vm_i16(m, ctx + 0x20);
    let capture_count = vm_u16(m, ctx + 0x22) as usize;
    let blob_size = m.word(ctx + 0x24) as usize;
    if blob_size > VM_CODE_MAX {
        return Err(crate::frontend_types::Error::Capacity {
            required: blob_size,
            available: VM_CODE_MAX,
        });
    }
    let blob = m.word(ctx + 0x68);

    let kinds_len = vm_copy_table(m, ctx, 0x28, 0x2c, &mut w.kinds)?;
    let module_bytes = vm_span(m, ctx, 0x2c, 0x30)?;
    let module_len = module_bytes / 2;
    if module_len > VM_MODULE_MAX {
        return Err(crate::frontend_types::Error::Capacity {
            required: module_len,
            available: VM_MODULE_MAX,
        });
    }
    let module_starts = m.word(ctx + 0x2c);
    let module_ends = m.word(ctx + 0x30);
    for index in 0..module_len {
        w.module_starts[index] = vm_u16(m, module_starts + 2 * index as u32);
        w.module_ends[index] = vm_i16(m, module_ends + 2 * index as u32);
    }
    let classes_len = vm_copy_table(m, ctx, 0x34, 0x38, &mut w.classes)?;
    let groups_len = vm_copy_table(m, ctx, 0x38, 0x3c, &mut w.group_table)?;
    let values_len = vm_copy_table(m, ctx, 0x3c, 0x40, &mut w.values)?;
    let matrix_len = vm_copy_table(m, ctx, 0x40, 0x44, &mut w.matrix)?;
    let capture_type_len = vm_copy_table(m, ctx, 0x44, 0x48, &mut w.capture_types)?;
    let rule_bytes = vm_span(m, ctx, 0x48, 0x4c)?;
    let rule_len = rule_bytes / 4;
    if rule_len > VM_RULE_MAX {
        return Err(crate::frontend_types::Error::Capacity {
            required: rule_len,
            available: VM_RULE_MAX,
        });
    }
    let rule_offsets = m.word(ctx + 0x48);
    for index in 0..rule_len {
        w.rules[index] = m.word(rule_offsets + 4 * index as u32) as usize;
    }
    let offsets_len = vm_copy_table(m, ctx, 0x4c, 0x50, &mut w.offsets)?;
    let minimums_len = vm_copy_table(m, ctx, 0x50, 0x54, &mut w.minimums)?;
    let lengths_len = vm_copy_table(m, ctx, 0x54, 0x58, &mut w.lengths)?;
    let jumps_len = vm_copy_table(m, ctx, 0x58, 0x5c, &mut w.jumps)?;
    let groups_rule_len = vm_copy_table(m, ctx, 0x5c, 0x60, &mut w.groups)?;
    let required_first_len = vm_copy_table(m, ctx, 0x60, 0x64, &mut w.required_first)?;
    let required_second_len = vm_copy_table(m, ctx, 0x64, 0x68, &mut w.required_second)?;
    let rule_len = rule_len
        .min(offsets_len)
        .min(minimums_len)
        .min(lengths_len)
        .min(jumps_len)
        .min(groups_rule_len)
        .min(required_first_len)
        .min(required_second_len);
    if rule_len == 0 {
        return Err(crate::frontend_types::Error::InvalidResource);
    }
    vm_copy(m, blob, &mut w.code[..blob_size])?;

    let default_start = vm_i16(m, m.word(ctx + 0x6c));
    let layer_array = m.word(ctx + 0x39c);
    let mut layers = [0u32; VM_LAYERS];
    for index in 0..layer_count {
        layers[index] = m.word(layer_array + 4 * index as u32);
    }

    let cap = (vm_u16(m, end_ptr.wrapping_sub(2)) as usize).min(VM_ROW_MAX);
    for index in 0..layer_count {
        vm_copy(m, layers[index], &mut w.rows[index][..cap])?;
    }
    vm_copy(m, m.word(ctx + 0x3b8), &mut w.rows[VM_LAYERS][..cap])?;
    let capture_len = capture_count + 1;
    if capture_len > VM_CAPTURE_MAX {
        return Err(crate::frontend_types::Error::Capacity {
            required: capture_len,
            available: VM_CAPTURE_MAX,
        });
    }
    vm_copy(m, m.word(ctx + 0x3a0), &mut w.captures[..capture_len])?;

    for index in 0..VM_FRAME_MAX {
        let position = m.word(ctx + 0xa4 + 4 * index as u32);
        let string = m.word(ctx + 0x298 + 4 * index as u32);        w.frames[index] = depes::RepeatFrame {
            repeats: vm_u8(m, ctx + 0x70 + index as u32),
            position: if position == 0 {
                None
            } else {
                Some((position - blob) as usize)
            },
            offset: vm_i16(m, ctx + 0x16c + 2 * index as u32),
            length: vm_i16(m, ctx + 0x1d0 + 2 * index as u32),
            command: vm_i16(m, ctx + 0x234 + 2 * index as u32),
            current: vm_layer_index(&layers, string),
        };
    }

    let cursor = m.word(ctx + 0x388);
    let original_current = m.word(ctx + 0x380);
    let original_output = m.word(ctx + 0x384);
    let original_prefix = m.word(ctx + 0x368);
    let original_saved = m.word(ctx + 0x36c);
    let mut state = depes::VmState {
        sequence: depes::SequenceState {
            predicate: depes::PredicateState {
                current: vm_layer_index(&layers, original_current).unwrap_or(0),
                saved: None,
                cursor: cursor.wrapping_sub(blob) as usize,
                character: vm_u8(m, ctx + 0x371),
                captured: vm_u8(m, ctx + 0x39a),
                value: vm_u8(m, ctx + 0x39b),
            },
            active: vm_u8(m, ctx + 0x360),
            saved_layer: vm_layer_index(&layers, original_saved),
            argument: vm_u8(m, ctx + 0x372),
            maximum: vm_i16(m, ctx + 0x374),
            depth: vm_i16(m, ctx + 0x38c),
            offset: vm_i16(m, ctx + 0x38e),
            step: vm_i16(m, ctx + 0x390),
            skip: vm_i16(m, ctx + 0x392),
            previous: vm_i16(m, ctx + 0x394),
        },
        prefix_saved: vm_layer_index(&layers, original_prefix),
        replacement: depes::ReplacementState {
            active: vm_u8(m, ctx + 0x370),
            tail: vm_i16(m, ctx + 0x378),
            error: vm_u8(m, ctx + 0x37b),
            depth: vm_i16(m, ctx + 0x37c),
            changed: vm_u8(m, ctx + 0x37e),
            output: vm_layer_index(&layers, original_output).unwrap_or(0),
            cursor: cursor.wrapping_sub(blob) as usize,
            span: vm_i16(m, ctx + 0x396),
            distance: vm_i16(m, ctx + 0x398),
            variant: vm_u8(m, ctx + 0x39b),
        },
        dispatch_type: vm_i16(m, ctx + 0x376),
    };

    let handle = m.word(ctx + 0x3ac);
    let selector = m.word(ctx + 0x3b0);
    let mut end = vm_i16(m, end_ptr);
    let mut ends = [0i16; VM_LAYERS + 1];    let VmWorkspace {
        code,
        rules: rule_table,
        offsets,
        minimums,
        lengths,
        jumps,
        groups,
        required_first,
        required_second,
        module_starts,
        module_ends,
        kinds,
        matrix,
        classes,
        group_table,
        values,
        capture_types,
        rows: row_buffers,
        captures: capture_slots,
        frames,
        scanner,
    } = w;
    let rules = depes::VmRules {
        code: &code[..blob_size],
        module_starts: &module_starts[..module_len],
        module_ends: &module_ends[..module_len],
        rules: &rule_table[..rule_len],
        offsets: &offsets[..rule_len],
        minimums: &minimums[..rule_len],
        jumps: &jumps[..rule_len],
        groups: &groups[..rule_len],
        required_first: &required_first[..rule_len],
        required_second: &required_second[..rule_len],
        header: 0,
        capture_count,
        replacement: depes::ReplacementRules {
            kinds: &kinds[..kinds_len],
            matrix: &matrix[..matrix_len],
            lengths: &lengths[..rule_len],
            stride,
            default_start,
            reset: reset_value,
        },
        characters: depes::CharacterTables {
            classes: &classes[..classes_len],
            groups: &group_table[..groups_len],
            values: &values[..values_len],
            capture_types: &capture_types[..capture_type_len],
        },
    };
    let (layer_buffers, marker_buffer) = row_buffers.split_at_mut(VM_LAYERS);
    let (row0, rest) = layer_buffers.split_at_mut(1);
    let (row1, rest) = rest.split_at_mut(1);
    let (row2, rest) = rest.split_at_mut(1);
    let (row3, rest) = rest.split_at_mut(1);
    let (row4, _) = rest.split_at_mut(1);
    let mut rows: [&mut [u8]; VM_LAYERS] = [
        &mut row0[0][..],
        &mut row1[0][..],
        &mut row2[0][..],
        &mut row3[0][..],
        &mut row4[0][..],
    ];
    let captures = &mut capture_slots[..capture_len];
    let mut scratch = depes::VmScratch {
        frames: &mut frames[..],
        scanner: &mut scanner[..],
    };
    let result = {
        let mut expansion = VmExpansionCall {
            m: &mut *m,
            host: &mut *host,
            handle,
            selector,
            out: [0; VM_ROW_MAX],
            ends,
        };
        let result = state.dispatch(
            module as usize,
            start,
            &mut end,
            &mut rows[..layer_count],
            &mut marker_buffer[0][..],
            captures,
            &rules,
            &mut scratch,
            &mut expansion,
        );

        expansion.expanded();
        result
    };
    let result = result?;

    let result = if state.replacement.error != 0 { 0 } else { 1 };
    vm_put_u16(m, end_ptr, end as u16);

    let layers = {
        let mut layers = [0u32; VM_LAYERS];
        for index in 0..layer_count {
            layers[index] = m.word(layer_array + 4 * index as u32);
        }
        layers
    };
    let cursor = blob.wrapping_add(state.sequence.predicate.cursor as u32);
    vm_put_u8(m, ctx + 0x360, state.sequence.active);
    vm_put_u32(
        m,
        ctx + 0x368,
        vm_layer_ptr(&layers, state.prefix_saved, original_prefix),
    );
    vm_put_u32(
        m,
        ctx + 0x36c,
        vm_layer_ptr(&layers, state.sequence.saved_layer, original_saved),
    );
    vm_put_u8(m, ctx + 0x370, state.replacement.active);
    vm_put_u8(m, ctx + 0x371, state.sequence.predicate.character);
    vm_put_u8(m, ctx + 0x372, state.sequence.argument);
    vm_put_u16(m, ctx + 0x374, state.sequence.maximum as u16);
    vm_put_u16(m, ctx + 0x376, state.dispatch_type as u16);
    vm_put_u16(m, ctx + 0x378, state.replacement.tail as u16);
    vm_put_u8(m, ctx + 0x37b, state.replacement.error);
    vm_put_u16(m, ctx + 0x37c, state.replacement.depth as u16);
    vm_put_u8(m, ctx + 0x37e, state.replacement.changed);
    vm_put_u32(
        m,
        ctx + 0x380,
        vm_layer_ptr(&layers, Some(state.sequence.predicate.current), original_current),
    );
    vm_put_u32(
        m,
        ctx + 0x384,
        vm_layer_ptr(&layers, Some(state.replacement.output), original_output),
    );
    vm_put_u32(m, ctx + 0x388, cursor);
    vm_put_u16(m, ctx + 0x38c, state.sequence.depth as u16);
    vm_put_u16(m, ctx + 0x38e, state.sequence.offset as u16);
    vm_put_u16(m, ctx + 0x390, state.sequence.step as u16);
    vm_put_u16(m, ctx + 0x392, state.sequence.skip as u16);
    vm_put_u16(m, ctx + 0x394, state.sequence.previous as u16);
    vm_put_u16(m, ctx + 0x396, state.replacement.span as u16);
    vm_put_u16(m, ctx + 0x398, state.replacement.distance as u16);
    vm_put_u8(m, ctx + 0x39a, state.sequence.predicate.captured);
    vm_put_u8(m, ctx + 0x39b, state.sequence.predicate.value);
    for index in 0..VM_FRAME_MAX {
        let frame = frames[index];
        vm_put_u8(m, ctx + 0x70 + index as u32, frame.repeats);
        let position = match frame.position {
            Some(value) => blob + value as u32,
            None => 0,
        };
        vm_put_u32(m, ctx + 0xa4 + 4 * index as u32, position);
        vm_put_u16(m, ctx + 0x16c + 2 * index as u32, frame.offset as u16);
        vm_put_u16(m, ctx + 0x1d0 + 2 * index as u32, frame.length as u16);
        vm_put_u16(m, ctx + 0x234 + 2 * index as u32, frame.command as u16);
        let string = match frame.current {
            Some(value) => layers[value],
            None => 0,
        };
        vm_put_u32(m, ctx + 0x298 + 4 * index as u32, string);
    }
    for index in 0..layer_count {
        vm_copy_out(m, layers[index], &layer_buffers[index][..cap])?;
    }
    vm_copy_out(m, m.word(ctx + 0x3b8), &marker_buffer[0][..cap])?;
    vm_copy_out(m, m.word(ctx + 0x3a0), &capture_slots[..capture_len])?;
    Ok(result)
}

const PHRASING_SCRATCH: u32 = 0x0e0ff700;

const PHRASING_DATA_MAX: usize = 16384;

const PHRASING_MARK_SIZE: usize = 24;

struct NoExpansion;
impl VmExpansion for NoExpansion {
    fn expand_m(
        &mut self,
        _handle: u32,
        _selector: u32,
        _code: u8,
        _text: &[u8],
        _out: &mut [u8],
        _ends: &mut [i16],
        _m: &mut Memory<'_>,
    ) -> Result<(i16, u8), crate::frontend_types::Error> {
        Err(crate::frontend_types::Error::InvalidResource)
    }
}

pub struct PhrasingLive<'a, 'm, 'w, H> {
    pub m: &'a mut Memory<'m>,
    pub h: &'a mut H,
    pub w: &'w mut VmWorkspace,
    pub env: u32,
    pub nodes: u32,
    pub engine: [u32; 2],
    pub engine_methods: u32,
    pub dictionary: [u32; 2],
    pub dictionary_methods: u32,
    pub text_block: u32,
    pub map_block: u32,
    data: [u8; PHRASING_DATA_MAX],

    pub input_addr: u32,
}

impl<'a, 'm, 'w, H: Host> PhrasingLive<'a, 'm, 'w, H> {

    fn call(&mut self, entry: u32, args: &[u32]) -> u32 {
        let m = &mut *self.m;
        let h = &mut *self.h;
        h.call_m(entry, args, m)
    }
    fn word(&self, at: u32) -> u32 {
        self.m.word(at)
    }
    fn half(&self, at: u32) -> u16 {
        let o = self.m.offset(at);
        u16::from_le_bytes(self.m.bytes[o..o + 2].try_into().unwrap())
    }
    fn byte(&self, at: u32) -> u8 {
        self.m.bytes[self.m.offset(at)]
    }
    fn put_byte(&mut self, at: u32, value: u8) {
        let o = self.m.offset(at);
        self.m.bytes[o] = value;
    }
    fn put_half(&mut self, at: u32, value: u16) {
        let o = self.m.offset(at);
        self.m.bytes[o..o + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn method(&self, table: u32, offset: u32) -> u32 {
        self.word(table + offset)
    }

    fn field_info(&self, object: u32, record: u32, field: u32) -> (u32, u32, u32) {
        if record >= self.word(object + 32) {
            return (0, 0, 0);
        }
        let entry = self.word(self.word(object + 48) + record * 4);
        if entry == 0 {
            return (0, 0, 0);
        }
        let kind = self.byte(entry + 2) as u32;
        let layout = crate::rom_region::addr(self.h.rom(), 0x8d7ca0) + kind * 272;
        let base = self.word(layout);
        let desc = layout + 12 + field * 20;
        let fixed = self.half(desc + 8) as u32;
        let mut address = entry.wrapping_add(self.word(desc).wrapping_sub(base));
        if fixed == 0 {
            address = self.word(address);
        }
        let raw = self.word(desc + 4);
        let count = if raw == 0 {
            0
        } else {
            u32::from(self.half(entry.wrapping_add(raw.wrapping_sub(base))))
        };
        let size = u32::from(self.half(desc + 10));
        (address, count, size)
    }

    fn copy_out(&mut self, from: u32, len: usize) -> usize {
        let mut copied = 0usize;
        while copied < len && copied < PHRASING_DATA_MAX {
            let Some(offset) = self.m.bytes.get(self.m.offset(from + copied as u32)) else {
                break;
            };
            let value = *offset;
            self.data[copied] = value;
            copied += 1;
        }
        copied
    }
}

impl<'a, 'm, 'w, H: Host> crate::prosody::PhrasingProcessHost for PhrasingLive<'a, 'm, 'w, H> {

    fn validate(&mut self, object: [u32; 2], tag: u32, size: u32) -> u32 {
        if object[1] == tag && (size == 0 || object[0] != 0) {
            0
        } else {
            0x81706008
        }
    }

    fn first_node(&mut self, handle: [u32; 2], kind: i32, sub: u32) -> (u32, u16) {
        let entry = self.method(self.nodes, 0x34);
        let out = PHRASING_SCRATCH;
        let rc = self.call(entry, &[handle[0], handle[1], kind as u32, sub, out]);
        (rc, self.half(out))
    }

    fn next_node(&mut self, handle: [u32; 2], node: u16) -> (u32, u16) {
        let entry = self.method(self.nodes, 0x3c);
        let out = PHRASING_SCRATCH;
        self.put_half(out, node);
        let rc = self.call(entry, &[handle[0], handle[1], u32::from(node), out]);
        (rc, self.half(out))
    }

    fn previous_node(&mut self, handle: [u32; 2], node: u16) -> (u32, u16) {
        let entry = self.method(self.nodes, 0x44);
        let out = PHRASING_SCRATCH;
        let rc = self.call(entry, &[handle[0], handle[1], u32::from(node), out]);
        (rc, self.half(out))
    }

    fn node_field(
        &mut self,
        handle: [u32; 2],
        node: u16,
        field: i32,
        count: i32,
    ) -> (u32, u32, u16) {
        let entry = self.method(self.nodes, 0x54);
        let out = PHRASING_SCRATCH + 8;
        let written = PHRASING_SCRATCH + 16;
        let rc = self.call(
            entry,
            &[
                handle[0],
                handle[1],
                u32::from(node),
                field as u32,
                count as u32,
                out,
                written,
            ],
        );
        let size = self.half(written);

        let (_, _, elem) = self.field_info(handle[0], u32::from(node), field as u32);
        let bytes = ((u32::from(size)).saturating_mul(elem.max(1))).min(4) as u16;
        let mut value = 0u32;
        let mut index = 0u16;
        while index < bytes {
            value |= u32::from(self.byte(out + u32::from(index))) << (8 * u32::from(index));
            index += 1;
        }
        (rc, value, size)
    }

    fn set_node_field(
        &mut self,
        handle: [u32; 2],
        node: u16,
        field: i32,
        count: u16,
        value: u32,
    ) -> (u32, u16) {
        let entry = self.method(self.nodes, 0x50);
        let value_at = PHRASING_SCRATCH + 24;
        let written = PHRASING_SCRATCH + 32;
        let (_, _, size) = self.field_info(handle[0], u32::from(node), field as u32);
        let mut index = 0u32;
        while index < size.min(4) {
            self.put_byte(value_at + index, (value >> (8 * index)) as u8);
            index += 1;
        }
        let rc = self.call(
            entry,
            &[
                handle[0],
                handle[1],
                u32::from(node),
                field as u32,
                u32::from(count),
                value_at,
                written,
            ],
        );
        (rc, self.half(written))
    }

    fn node_data(&mut self, handle: [u32; 2], node: u16, field: i32) -> (u32, &[u8], u16) {
        let entry = self.method(self.nodes, 0x58);
        let out = PHRASING_SCRATCH + 40;
        let count = PHRASING_SCRATCH + 48;
        let rc = self.call(
            entry,
            &[
                handle[0],
                handle[1],
                u32::from(node),
                field as u32,
                out,
                count,
            ],
        );
        if rc >= 0x80000000 {
            return (rc, &[], 0);
        }
        let elements = self.half(count);
        let address = self.word(out);
        if field == 0 {
            self.input_addr = address;
        }
        let (_, _, size) = self.field_info(handle[0], u32::from(node), field as u32);
        let mut len = usize::from(elements) * size.max(1) as usize;
        if field == 5 {
            let bytes = &self.m.bytes[self.m.offset(address)..];
            len = bytes.iter().position(|b| *b == 0).expect("unterminated node word") + 1;
        }
        let copied = self.copy_out(address, len);
        (rc, &self.data[..copied], elements)
    }

    fn node_count(&mut self, handle: [u32; 2], node: u16, field: i32) -> (u32, u16) {
        let entry = self.method(self.nodes, 0x5c);
        let out = PHRASING_SCRATCH + 56;
        let rc = self.call(
            entry,
            &[handle[0], handle[1], u32::from(node), field as u32, out],
        );
        (rc, self.half(out))
    }

    fn root_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16) {
        let (rc, data, len) = self.node_data(handle, node, 0);
        if rc >= 0x80000000 {
            return (rc, 0);
        }
        let mut index = 0usize;
        while index < out.len() && index < data.len() && data[index] != 0 {
            out[index] = data[index];
            index += 1;
        }
        if index < out.len() {
            out[index] = 0;
        }
        (rc, len)
    }

    fn commit_input_byte(&mut self, offset: u32, value: u8) {
        if self.input_addr != 0 {
            self.m.put_byte(self.input_addr.wrapping_add(offset), value);
        }
    }

    fn root_marks(
        &mut self,
        handle: [u32; 2],
        node: u16,
        out: &mut [crate::prosody::PhrasingMark],
    ) -> (u32, u16) {
        let (rc, data, len) = self.node_data(handle, node, 1);
        if rc >= 0x80000000 {
            return (rc, 0);
        }
        let count = usize::from(len);
        let mut index = 0usize;
        while index < count && index < out.len() {
            let at = index * PHRASING_MARK_SIZE;
            if at + PHRASING_MARK_SIZE > data.len() {
                break;
            }
            out[index].r#type = u16::from_le_bytes(data[at..at + 2].try_into().unwrap());
            out[index].position =
                u32::from_le_bytes(data[at + 0x0c..at + 0x10].try_into().unwrap());
            out[index].value = u32::from_le_bytes(data[at + 0x14..at + 0x18].try_into().unwrap());
            index += 1;
        }
        (rc, len)
    }

    fn mode_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16) {
        let (rc, data, len) = self.node_data(handle, node, 4);
        if rc >= 0x80000000 {
            return (rc, 0);
        }
        let mut index = 0usize;
        while index < out.len() && index < data.len() && data[index] != 0 {
            out[index] = data[index];
            index += 1;
        }
        if index < out.len() {
            out[index] = 0;
        }
        (rc, len)
    }

    fn merge_param(
        &mut self,
        dictionary: [u32; 2],
        key: &[u8],
        out: &mut [u8],
    ) -> (u32, u16, u8, usize) {
        let entry = self.method(self.dictionary_methods, 0x1c);
        let key_at = PHRASING_SCRATCH + 0x60;
        let values = PHRASING_SCRATCH + 0x40;
        let found = PHRASING_SCRATCH + 0x48;
        let separator = PHRASING_SCRATCH + 0x4c;
        let section = PHRASING_SCRATCH + 0x1e0;
        let mut index = 0usize;
        while index < key.len() && index < 128 {
            self.put_byte(key_at + index as u32, key[index]);
            index += 1;
        }
        self.put_byte(key_at + index as u32, 0);
        self.put_byte(section, b'f');
        self.put_byte(section + 1, b'e');
        self.put_byte(section + 2, b'c');
        self.put_byte(section + 3, b'f');
        self.put_byte(section + 4, b'g');
        self.put_byte(section + 5, 0);
        let rc = self.call(
            entry,
            &[
                dictionary[0],
                dictionary[1],
                section,
                key_at,
                values,
                found,
                separator,
            ],
        );
        let value = self.byte(separator);
        let flag = self.half(found);
        let pointer = self.word(values);
        let mut copied = 0usize;
        while copied < out.len() {
            let byte = self.byte(pointer + copied as u32);
            if byte == 0 {
                break;
            }
            out[copied] = byte;
            copied += 1;
        }
        (rc, flag, value, copied)
    }

    fn pause_weights(&mut self, env: u32, key: &[u8]) -> (u32, u32) {
        let key_at = PHRASING_SCRATCH + 0x60;
        let out = PHRASING_SCRATCH + 0x50;
        let mut index = 0usize;
        while index < key.len() && index < 128 {
            self.put_byte(key_at + index as u32, key[index]);
            index += 1;
        }
        self.put_byte(key_at + index as u32, 0);
        let config = self.word(env + 0x14);
        let rc = self.call(0x175944, &[config, key_at, out]);
        (rc, self.word(out))
    }

    fn set_engine_input(&mut self, handle: [u32; 2], index: i32, text: &[u8]) -> u32 {
        let block = if index == 0 { self.text_block } else { self.map_block };
        if block == 0 {
            return 0x89702007;
        }
        let mut at = 0usize;
        while at < text.len() {
            self.put_byte(block + at as u32, text[at]);
            at += 1;
        }

        self.put_byte(block + at as u32, 0);
        let m = &mut *self.m;
        let h = &mut *self.h;
        depes_set_layer_m(
            m,
            handle[0],
            handle[1],
            index as u32,
            block,
            text.len() as u32,
            h,
        )
    }

    fn engine_output(&mut self, handle: [u32; 2], index: i32, buf: &mut [u8]) -> (u32, usize) {
        let entry = self.method(self.engine_methods, 0x28);
        let out = PHRASING_SCRATCH + 0x58;
        let len_at = PHRASING_SCRATCH + 0x5c;
        let rc = self.call(
            entry,
            &[handle[0], handle[1], index as u32, out, len_at],
        );
        if rc >= 0x80000000 {
            return (rc, 0);
        }
        let length = self.half(len_at) as usize;
        let pointer = self.word(out);
        let mut copied = 0usize;
        while copied < buf.len() && copied < length {
            buf[copied] = self.byte(pointer + copied as u32);
            copied += 1;
        }

        self.put_byte(pointer + length as u32, 0);
        (rc, copied)
    }

    fn run_engine(&mut self, handle: [u32; 2], key: &[u8]) -> u32 {
        let key_at = PHRASING_SCRATCH + 0x60;
        let mut index = 0usize;
        while index < key.len() && index < 128 {
            self.put_byte(key_at + index as u32, key[index]);
            index += 1;
        }
        self.put_byte(key_at + index as u32, 0);
        let m = &mut *self.m;
        let h = &mut *self.h;
        let w = &mut *self.w;
        depes_exec_module_m(m, handle[0], handle[1], key_at, w, h)
    }

    fn prepare_dictionary(&mut self, handle: [u32; 2], mode: &[u8], text: &mut [u8]) -> u32 {
        let block = self.text_block;
        if block == 0 {
            return 0x89802007;
        }

        let length = terminated_length(text);
        let mut index = 0usize;
        while index < length {
            self.put_byte(block + index as u32, text[index]);
            index += 1;
        }
        let mode_at = PHRASING_SCRATCH + 0x60;
        let mut mode_index = 0usize;
        while mode_index < mode.len() && mode_index < 128 {
            self.put_byte(mode_at + mode_index as u32, mode[mode_index]);
            mode_index += 1;
        }
        self.put_byte(mode_at + mode_index as u32, 0);
        let m = &mut *self.m;
        let h = &mut *self.h;
        let rc = dctlkp_canonical_token_m(m, handle[0], handle[1], mode_at, block, h);
        let mut read = 0usize;
        while read < length {
            let value = m.byte(block + read as u32);
            text[read] = value;
            read += 1;
        }
        rc
    }

    fn log_engine(&mut self, env: u32, stage: u8, key: &[u8], text: &[u8]) {
        let log = self.word(env + 0x10);
        let key_at = PHRASING_SCRATCH + 0x60;
        let text_at = PHRASING_SCRATCH + 0xe0;
        let mut index = 0usize;
        while index < key.len() && index < 128 {
            self.put_byte(key_at + index as u32, key[index]);
            index += 1;
        }
        self.put_byte(key_at + index as u32, 0);
        let mut text_index = 0usize;
        while text_index < text.len() && text_index < 256 {
            self.put_byte(text_at + text_index as u32, text[text_index]);
            text_index += 1;
        }
        self.put_byte(text_at + text_index as u32, 0);
        let _ = stage;
        self.call(0x1738e8, &[log, 0x8f8a54, 5, 0, key_at, text_at]);
    }

    fn log_nomem(&mut self, env: u32) {
        let log = self.word(env + 0x10);
        self.call(0x173dc0, &[log, 0x8f8a54, 0x9088, 0]);
    }

    fn alloc_text(&mut self, env: u32, capacity: u16) -> u32 {
        let heap = self.word(env + 0x04);
        let block = self.call(0x196394, &[heap, u32::from(capacity)]);
        if block != 0 {
            self.put_byte(block, 0);
            if self.text_block == 0 {
                self.text_block = block;
            } else {
                self.map_block = block;
            }
        }
        block
    }

    fn free_text(&mut self, env: u32, addr: u32) {
        let heap = self.word(env + 0x04);
        self.call(0x196428, &[heap, addr]);
    }
}

pub fn depes_set_layer_m(
    m: &mut Memory<'_>,
    engine: u32,
    tag: u32,
    layer: u32,
    text: u32,
    length: u32,
    h: &mut impl Host,
) -> u32 {
    if engine == 0 || tag != 0xf37e {
        return 0x89702008;
    }
    let context = m.word(engine);
    let allocator = m.word(context + 0x04);
    let layer_count = m.half(engine + 0x30) as i16;
    if layer_count <= 0 || layer >= layer_count as u32 || text == 0 {
        return 0;
    }
    let length = length.min(10000);
    let new_capacity = length * 3;
    let old_capacity = u32::from(m.half(engine + 0x32));
    let layers = m.word(engine + 0x28);
    let flags = m.word(engine + 0x2c);
    if new_capacity > old_capacity {
        let mut index = 0u32;
        while index < layer_count as u32 {
            let row = m.word(layers + index * 4);
            let grown = h.call_m(0x1963e0, &[allocator, row, new_capacity], m);
            if grown == 0 {
                return 0x8970200a;
            }
            m.put_word(layers + index * 4, grown);
            if m.word(engine + 0x10) != 0 {
                let first = m.word(engine + 0x20);
                m.put_word(m.word(first + 0x39c) + index * 4, grown);
            }
            if m.word(engine + 0x14) != 0 {
                let second = m.word(engine + 0x24);
                m.put_word(m.word(second + 0x39c) + index * 4, grown);
            }
            let mut offset = old_capacity;
            while offset < new_capacity {
                m.put_byte(grown + offset, b' ');
                offset += 1;
            }
            index += 1;
        }
        let grown = h.call_m(0x1963e0, &[allocator, flags, new_capacity], m);
        if grown == 0 {
            return 0x8970200a;
        }
        m.put_word(engine + 0x2c, grown);
        if m.word(engine + 0x10) != 0 {
            let first = m.word(engine + 0x20);
            m.put_word(first + 0x3b8, grown);
        }
        if m.word(engine + 0x14) != 0 {
            let second = m.word(engine + 0x24);
            m.put_word(second + 0x3b8, grown);
        }
        m.put_half(engine + 0x32, new_capacity as u16);
    }
    let capacity = u32::from(m.half(engine + 0x32));
    let row = m.word(layers + layer * 4);
    let mut index = 0u32;
    while index < length {
        m.put_byte(row + index, m.byte(text + index));
        index += 1;
    }
    let mut offset = length;
    while offset < capacity {
        m.put_byte(row + offset, b' ');
        offset += 1;
    }
    if layer == 0 {
        let mut index = 1u32;
        while index < layer_count as u32 {
            let other = m.word(layers + index * 4);
            let mut offset = 0u32;
            while offset < capacity {
                m.put_byte(other + offset, b' ');
                offset += 1;
            }
            index += 1;
        }
        m.put_half(engine + 0x34, length as u16);
    } else {
        let current = m.half(engine + 0x34);
        if (length as i16) > (current as i16) {
            m.put_half(engine + 0x34, length as u16);
        }
    }
    0
}

pub fn depes_execute_module_m(
    m: &mut Memory<'_>,
    key: u32,
    ctx: u32,
    end_ptr: u32,
    w: &mut VmWorkspace,
    h: &mut impl Host,
) -> u32 {
    let size = m.word(ctx + 0x04);
    let data = m.word(ctx + 0x08);
    let mut offset = 0u32;
    while offset < size {
        let different = h.call_m(0x1769d0, &[data + offset, key], m);
        if different == 0 {
            break;
        }
        offset += arena_strlen(m, data + offset) + 1;
        while offset & 3 != 0 {
            offset += 1;
        }
        let length = m.word(data + offset);
        offset += 4 + length;
    }
    if size <= offset {
        return 0x89702004;
    }
    let mut position = offset + arena_strlen(m, data + offset) + 1;
    while position & 3 != 0 {
        position += 1;
    }
    let length = m.word(data + position);
    position += 4;
    let mut buffer = [0u8; 104];
    let mut index = 0u32;
    while index < length && index < 104 {
        buffer[index as usize] = m.byte(data + position + index);
        index += 1;
    }
    m.put_half(ctx + 0x37c, 0);
    m.put_byte(ctx + 0x37b, 0);
    m.put_half(ctx + 0x378, m.half(m.word(ctx + 0x6c)));
    let buffer_size = m.half(ctx + 0x22);
    let slots = m.word(ctx + 0x3a0);
    let mut offset = 0u32;
    while offset < u32::from(buffer_size) {
        m.put_byte(slots + offset, 0);
        offset += 1;
    }
    m.put_byte(ctx + 0x39a, 1);
    let mut index = 0u32;
    let mut expansion = NoExpansion;
    while index < length {

        if vm_run_m(
            m,
            ctx,
            u32::from(buffer[index as usize]),
            1,
            end_ptr,
            w,
            &mut expansion,
        )
        .is_err()
        {
            return 0x8970200a;
        }
        index += 1;
    }
    let status = m.byte(ctx + 0x37b);
    if status == 0x9b {
        0x8970200a
    } else if status == 0x9c {
        0x89702000
    } else {
        0
    }
}

pub fn depes_get_layer_m(
    m: &mut Memory<'_>,
    engine: u32,
    tag: u32,
    index: u32,
    out: u32,
    len_at: u32,
) -> u32 {
    if engine == 0 || tag != 0xf37e {
        return 0x89702008;
    }
    let count = m.half(engine + 0x30) as i16;
    if count <= 0 || index >= count as u32 || out == 0 {
        return 0x89702007;
    }
    m.put_half(len_at, m.half(engine + 0x34));
    let layers = m.word(engine + 0x28);
    m.put_word(out, m.word(layers + index * 4));
    0
}

pub fn depes_exec_module_m(
    m: &mut Memory<'_>,
    engine: u32,
    tag: u32,
    key: u32,
    w: &mut VmWorkspace,
    h: &mut impl Host,
) -> u32 {
    if engine == 0 || tag != 0xf37e {
        return 0x89702008;
    }
    let end_ptr = engine + 0x34;
    let mut result = 0x89702004u32;
    if m.word(engine + 0x14) != 0 {
        result = depes_execute_module_m(m, key, m.word(engine + 0x24), end_ptr, w, h);
    }
    if m.word(engine + 0x10) != 0 && result == 0x89702004 {
        result = depes_execute_module_m(m, key, m.word(engine + 0x20), end_ptr, w, h);
    }
    result
}

pub fn dctlkp_canonical_token_m(
    m: &mut Memory<'_>,
    object: u32,
    tag: u32,
    input: u32,
    text: u32,
    h: &mut impl Host,
) -> u32 {
    if object == 0 || tag != 0xf37f {
        return 0x89802008;
    }
    if text == 0 {
        return 0x89802007;
    }

    if m.byte(object + 0x29e) != 0 && !guest_str_eq(m, input, object + 0x19e) {
        let rc = h.call_m(0x1eb978, &[object, tag, input], m);
        if rc >= 0x80000000 {
            return rc;
        }
        guest_str_copy(m, object + 0x19e, input);
    }
    let mode = m.half(object + 0x9c);
    if mode == 0 {
        let table = m.word(object + 0x90);
        let cap = arena_strlen(m, text).wrapping_add(1);
        return canonical_replace_m(m, table, text, cap);
    }
    let length = arena_strlen(m, text);
    if mode != 2 && length > 1 {
        let mut index = 0u32;
        while index < length {
            let value = m.byte(text + index);
            m.put_byte(text + index, fn_240b28(u32::from(value)) as u8);
            index += 1;
        }
    }
    0
}

fn guest_str_eq(m: &Memory<'_>, a: u32, b: u32) -> bool {
    let mut off = 0u32;
    loop {
        let x = m.byte(a.wrapping_add(off));
        let y = m.byte(b.wrapping_add(off));
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
        off = off.wrapping_add(1);
    }
}

fn guest_str_copy(m: &mut Memory<'_>, dst: u32, src: u32) {
    let mut off = 0u32;
    loop {
        let b = m.byte(src.wrapping_add(off));
        m.put_byte(dst.wrapping_add(off), b);
        if b == 0 {
            break;
        }
        off = off.wrapping_add(1);
    }
}

fn canonical_replace_m(m: &mut Memory<'_>, table: u32, text: u32, cap: u32) -> u32 {
    let mut length = arena_strlen(m, text);
    let mut i = 0u32;
    while i < length {
        let b = m.byte(text.wrapping_add(i));
        let rep = if table != 0 {
            m.word(table.wrapping_add(u32::from(b).wrapping_mul(4)))
        } else {
            0
        };
        if table != 0 && rep != 0 {
            let extra = arena_strlen(m, rep);
            let total = length.wrapping_add(extra);
            if (total as u16 as u32) > cap {
                return 0x89802007;
            }
            let base = m.offset(text);
            let need = base.wrapping_add(length as usize).wrapping_add(extra as usize);
            if need > m.bytes.len() {
                return 0x89802007;
            }
            m.bytes.copy_within(base + i as usize + 1..base + length as usize + 1, base + i as usize + extra as usize);
            let rbase = m.offset(rep);
            let rlen = extra as usize;
            if rbase + rlen > m.bytes.len() {
                return 0x89802007;
            }
            for k in 0..rlen {
                m.bytes[base + i as usize + k] = m.bytes[rbase + k];
            }
            length = total.wrapping_sub(1);
        } else {
            m.put_byte(text.wrapping_add(i), fn_240b28(u32::from(b)) as u8);
        }
        i = i.wrapping_add(1);
    }
    0
}

fn arena_strlen(m: &Memory<'_>, at: u32) -> u32 {
    let mut length = 0u32;
    loop {
        let Some(offset) = m.offset(at + length).checked_sub(0).and_then(|v| {
            if v < m.bytes.len() {
                Some(v)
            } else {
                None
            }
        }) else {
            return length;
        };
        if m.bytes[offset] == 0 {
            return length;
        }
        length += 1;
    }
}

pub fn fn_1df780_m<'m>(
    m: &mut Memory<'m>,
    a: &[u32],
    active: u32,
    buffers: &mut crate::prosody::PhrasingBuffers<'_>,
    w: &mut VmWorkspace,
    h: &mut impl Host,
) -> u32 {
    if a.len() < 4 {
        return 0x8a002008;
    }
    let object = a[0];
    let env = m.word(object);
    let nodes = m.word(object + 0x04);
    let engine = [m.word(object + 0x08), m.word(object + 0x0c)];
    let engine_methods = m.word(object + 0x10);
    let dictionary = [m.word(object + 0x14), m.word(object + 0x18)];
    let dictionary_methods = m.word(object + 0x1c);
    let text_block = m.word(object + 0x20);
    let map_block = m.word(object + 0x24);
    let mut phrasing = crate::prosody::Phrasing {
        env,
        ling: nodes,
        depes: engine,
        depes_if: engine_methods,
        dct: dictionary,
        dct_if: dictionary_methods,
        z32: text_block,
        z36: map_block,
    };
    let mut flag = 0u8;
    let result = {
        let mut live = PhrasingLive {
            m,
            h,
            w,
            env,
            nodes,
            engine,
            engine_methods,
            dictionary,
            dictionary_methods,
            text_block: 0,
            map_block: 0,
            data: [0; PHRASING_DATA_MAX],
            input_addr: 0,
        };
        crate::prosody::process_phrasing_stage(
            &mut phrasing,
            [a[0], a[1]],
            [a[2], a[3]],
            &mut flag,
            buffers,
            &mut live,
        )
    };
    m.put_word(object + 0x00, phrasing.env);
    m.put_word(object + 0x04, phrasing.ling);
    m.put_word(object + 0x08, phrasing.depes[0]);
    m.put_word(object + 0x0c, phrasing.depes[1]);
    m.put_word(object + 0x10, phrasing.depes_if);
    m.put_word(object + 0x14, phrasing.dct[0]);
    m.put_word(object + 0x18, phrasing.dct[1]);
    m.put_word(object + 0x1c, phrasing.dct_if);
    m.put_word(object + 0x20, phrasing.z32);
    m.put_word(object + 0x24, phrasing.z36);
    if flag != 0 && active != 0 {
        m.put_byte(active, flag);
    }
    result
}

pub struct PhrasingPlatform;
impl crate::engine::HeapPlatform for PhrasingPlatform {
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
    fn arbiter_address(&self) -> u32 {
        0xa7bb7c
    }
    fn arbitrate(&mut self, _: &mut Memory<'_>, _: &[u32]) -> u32 {
        0
    }
}

pub struct PhrasingCalls<'a, P> {
    pub platform: &'a mut P,
    pub fault: Option<crate::engine::Error>,
    pub first_unbound: Option<u32>,
    pub calls: u32,
    pub trace: HeapTrace,

    pub live_rom: crate::rom_region::Rom,
}

impl<'a, P: crate::engine::HeapPlatform> PhrasingCalls<'a, P> {
    pub fn new(platform: &'a mut P) -> Self {
        PhrasingCalls {
            platform,
            fault: None,
            first_unbound: None,
            calls: 0,
            trace: HeapTrace::default(),
            live_rom: crate::rom_region::Rom::Us,
        }
    }
}

impl<P: crate::engine::HeapPlatform> Host for PhrasingCalls<'_, P> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.calls += 1;

        let entry = crate::rom_region::to_us_guest(self.live_rom, entry);
        if matches!(entry, 0x196394 | 0x1963b8 | 0x1963e0 | 0x196428) {
            let rc = self.heap_call(entry, args, m);
            if rc.is_some() {
                return rc.unwrap();
            }
        }
        if entry == 0x1769d0 {
            return crate::fe_text::strcmp_ci_1769d0(m.bytes, args[0], args[1]) as u32;
        }
        if entry == 0x2191c8 {
            let quad = match args {
                [a, b, c, d, e] => (*a, *b, *c, *d, *e),
                _ => {
                    self.first_unbound = Some(entry);
                    return 0x80002001;
                }
            };
            return depes_get_layer_m(m, quad.0, quad.1, quad.2, quad.3, quad.4);
        }
        if entry == 0x15ca38 {
            let quad = match args {
                [a, b, c, d] => [*a, *b, *c, *d],
                _ => {
                    self.first_unbound = Some(entry);
                    return 0x80002001;
                }
            };
            return match record_link_m(m, entry, quad) {
                Ok(rc) => rc,
                Err(e) => {
                    self.fault = Some(e);
                    0x80002001
                }
            };
        }
        let rom = self.live_rom;
        let mut host = crate::component_live_b::LiveHost {
            platform: &mut *self.platform,
            fault: None,
            scratch: 0x500,
            rom: crate::rom_region::Rom::Us,
        };
        let rc = host.call_m_with(entry, args, m, rom);
        if host.fault.is_some() {
            self.fault = host.fault;
            if self.first_unbound.is_none() {
                self.first_unbound = Some(entry);
            }
            return 0x80002001;
        }
        rc
    }
    fn rom(&self) -> crate::rom_region::Rom {
        self.live_rom
    }
}

#[derive(Clone, Copy)]
pub struct HeapCall {
    pub entry: u32,
    pub args: [u32; 4],
    pub ret: u32,

    pub lists: [u32; 4],

    pub block: [u32; 5],
}

pub struct HeapTrace {
    pub calls: [HeapCall; 32],
    pub len: usize,
}

impl Default for HeapTrace {
    fn default() -> Self {
        HeapTrace { calls: [HeapCall { entry: 0, args: [0; 4], ret: 0, lists: [0; 4], block: [0; 5] }; 32], len: 0 }
    }
}

fn heap_snapshot(m: &Memory<'_>, heap: u32, block: u32) -> ([u32; 4], [u32; 5]) {
    let lists = [m.word(heap + 0x3c), m.word(heap + 0x40), m.word(heap + 0x44), m.word(heap + 0x48)];
    let block = [
        u32::from(m.half(block)),
        u32::from(m.half(block + 2)),
        m.word(block + 4),
        m.word(block + 8),
        m.word(block + 12),
    ];
    (lists, block)
}

impl<P: crate::engine::HeapPlatform> PhrasingCalls<'_, P> {

    fn heap_call(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> Option<u32> {
        let wrapper = args[0];
        if wrapper == 0 || args.len() < 2 {
            return None;
        }
        let object = m.word(wrapper + 4);
        let heap = m.word(object + 4);
        if heap == 0 {
            return None;
        }
        let rc = self.dispatch_heap(entry, args, m);
        let touched = match entry {
            0x196394 | 0x1963b8 => rc.wrapping_sub(16),
            _ => args[1].wrapping_sub(16),
        };
        let (lists, block) = heap_snapshot(m, heap, touched);
        if self.trace.len < self.trace.calls.len() {
            let mut call_args = [0u32; 4];
            for (index, value) in args.iter().take(4).enumerate() {
                call_args[index] = *value;
            }
            self.trace.calls[self.trace.len] = HeapCall { entry, args: call_args, ret: rc, lists, block };
            self.trace.len += 1;
        }
        Some(rc)
    }

    fn dispatch_heap(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        let rom = self.live_rom;
        let mut host = crate::component_live_b::LiveHost {
            platform: &mut *self.platform,
            fault: None,
            scratch: 0x500,
            rom: crate::rom_region::Rom::Us,
        };
        let rc = host.call_m_with(entry, args, m, rom);
        if host.fault.is_some() {
            self.fault = host.fault;
            if self.first_unbound.is_none() {
                self.first_unbound = Some(entry);
            }
        }
        rc
    }
}

fn terminated_length(bytes: &[u8]) -> usize {
    match bytes.iter().position(|byte| *byte == 0) {
        Some(at) => at + 1,
        None => bytes.len(),
    }
}

const FE_TEXT_SCRATCH: u32 = 0x0e0ffd00;
const FE_TEXT_VALUE: u32 = FE_TEXT_SCRATCH + 0x00;
const FE_TEXT_WRITTEN: u32 = FE_TEXT_SCRATCH + 0x04;
const FE_TEXT_POINTER: u32 = FE_TEXT_SCRATCH + 0x08;
const FE_TEXT_LENGTH: u32 = FE_TEXT_SCRATCH + 0x0c;
const FE_TEXT_HALF: u32 = FE_TEXT_SCRATCH + 0x10;
const FE_TEXT_NAME: u32 = FE_TEXT_SCRATCH + 0x40;

const FE_TEXT_RULE: &[u8] = b"disambiguate_punctuation";

const FE_TEXT_UNBOUND: u32 = 0x8000_2001;

fn fe_text_byte(arena: &[u8], at: u32) -> u8 {
    arena.get(at as usize).copied().unwrap_or(0)
}
fn fe_text_half(arena: &[u8], at: u32) -> u16 {
    let index = at as usize;
    if index + 2 <= arena.len() {
        u16::from_le_bytes([arena[index], arena[index + 1]])
    } else {
        0
    }
}
fn fe_text_word(arena: &[u8], at: u32) -> u32 {
    let index = at as usize;
    if index + 4 <= arena.len() {
        u32::from_le_bytes([
            arena[index],
            arena[index + 1],
            arena[index + 2],
            arena[index + 3],
        ])
    } else {
        0
    }
}
fn fe_text_store_half(arena: &mut [u8], at: u32, value: u16) {
    let index = at as usize;
    if index + 2 <= arena.len() {
        arena[index..index + 2].copy_from_slice(&value.to_le_bytes());
    }
}
fn fe_text_store_word(arena: &mut [u8], at: u32, value: u32) {
    let index = at as usize;
    if index + 4 <= arena.len() {
        arena[index..index + 4].copy_from_slice(&value.to_le_bytes());
    }
}
fn fe_text_strlen(arena: &[u8], at: u32) -> u32 {
    let mut length = 0u32;
    while fe_text_byte(arena, at.wrapping_add(length)) != 0 {
        length = length.wrapping_add(1);
    }
    length
}
fn fe_text_equal(arena: &[u8], at: u32, literal: &[u8]) -> bool {
    let mut index = 0usize;
    while index < literal.len() {
        if fe_text_byte(arena, at.wrapping_add(index as u32)) != literal[index] {
            return false;
        }
        index += 1;
    }
    fe_text_byte(arena, at.wrapping_add(literal.len() as u32)) == 0
}

pub struct FeTextLive<'a, P> {
    pub platform: &'a mut P,
    pub fault: Option<crate::engine::Error>,
    pub first_unbound: Option<u32>,
    pub calls: u32,

    pub rom: crate::rom_region::Rom,

    pub workspace: Option<VmWorkspace>,

    pub mode: u32,

    pub rule: [u8; 132],
    pub rule_len: usize,
}

impl<'a, P: crate::engine::HeapPlatform> FeTextLive<'a, P> {
    pub fn new(platform: &'a mut P) -> Self {
        FeTextLive {
            platform,
            fault: None,
            first_unbound: None,
            calls: 0,
            rom: crate::rom_region::Rom::Us,
            workspace: None,
            mode: 0,
            rule: [0; 132],
            rule_len: 0,
        }
    }

    fn unbound(&mut self, entry: u32) -> u32 {
        if self.first_unbound.is_none() {
            self.first_unbound = Some(entry);
        }
        FE_TEXT_UNBOUND
    }

    fn build_rule(&mut self, arena: &[u8], full: bool) -> u32 {
        let mut length = 0usize;
        for byte in FE_TEXT_RULE {
            if length < 128 {
                self.rule[length] = *byte;
                length += 1;
            }
        }
        if full && self.mode != 0 {
            let mode = self.mode;
            if !fe_text_equal(arena, mode, b"normal") {
                let mode_length = fe_text_strlen(arena, mode);
                if length + mode_length as usize + 2 < 128 {
                    self.rule[length] = b'_';
                    length += 1;
                    let mut index = 0u32;
                    while index < mode_length {
                        self.rule[length] = fe_text_byte(arena, mode.wrapping_add(index));
                        length += 1;
                        index += 1;
                    }
                }
            }
        }
        self.rule[length] = 0;
        self.rule_len = length;
        length as u32
    }

    fn exec_rule(&mut self, arena: &mut [u8], engine: u32, tag: u32, length: u32) -> u32 {
        let mut index = 0u32;
        while index <= length {
            let byte = self.rule[index as usize];
            if let Some(slot) = arena.get_mut((FE_TEXT_NAME + index) as usize) {
                *slot = byte;
            }
            index += 1;
        }
        let mut workspace = self.workspace.take().unwrap_or_default();
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            depes_exec_module_m(
                &mut memory,
                engine,
                tag,
                FE_TEXT_NAME,
                &mut workspace,
                self,
            )
        };
        self.workspace = Some(workspace);
        status
    }

    fn set_option(&mut self, arena: &mut [u8], handle: u32, tag: u32, value: u32, which: u32) -> u32 {
        let mut memory = Memory { base: 0, bytes: &mut *arena };
        fn_219554_m(&mut memory, handle, tag, value, which)
    }

    fn get_option(
        &mut self,
        arena: &mut [u8],
        handle: u32,
        tag: u32,
        out: u32,
        which: u32,
    ) -> u32 {
        let mut memory = Memory { base: 0, bytes: &mut *arena };
        core_call_m(
            &mut memory,
            0x21958c,
            &[handle, tag, out, which],
        )
        .unwrap_or(FE_TEXT_UNBOUND)
    }
}

impl<P: crate::engine::HeapPlatform> Host for FeTextLive<'_, P> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.calls += 1;
        match entry {

            0x15c4b0 | 0x15c744 | 0x15c9a4 | 0x15c7d8 | 0x15ca38 => {
                let Ok(quad) = <[u32; 4]>::try_from(args) else {
                    return self.unbound(entry);
                };
                return match record_link_m(m, entry, quad) {
                    Ok(rc) => rc,
                    Err(e) => {
                        self.fault = Some(e);
                        self.unbound(entry)
                    }
                };
            }
            0x15d6c4 => {
                if args.len() < 5 {
                    return self.unbound(entry);
                }
                match enquire_child_m(m, [args[0], args[1], args[2], args[3], args[4]], self.rom) {
                    Ok(status) => status,
                    Err(error) => {
                        self.fault = Some(error);
                        self.unbound(entry)
                    }
                }
            }
            0x218f88 | 0x218ba4 => {
                if args.len() < 4 {
                    return self.unbound(entry);
                }
                let length = if args.len() > 4 { args[4] } else { 0 };
                depes_set_layer_m(m, args[0], args[1], args[2], args[3], length, self)
            }
            0x2191c8 => {
                if args.len() < 5 {
                    return self.unbound(entry);
                }
                depes_get_layer_m(m, args[0], args[1], args[2], args[3], args[4])
            }
            0x1963e0 => {

                match heap_call_m_rom(m, entry, args, &mut *self.platform, self.rom) {
                    Ok(status) => status,
                    Err(error) => {
                        self.fault = Some(error);
                        self.unbound(entry)
                    }
                }
            }
            0x2193a4 => {
                if args.len() < 3 {
                    return self.unbound(entry);
                }
                let mut workspace = self.workspace.take().unwrap_or_default();
                let status = depes_exec_module_m(
                    m,
                    args[0],
                    args[1],
                    args[2],
                    &mut workspace,
                    self,
                );
                self.workspace = Some(workspace);
                status
            }
            _ => {
                let rom = self.rom;
                let mut host = crate::component_live_b::LiveHost {
                    platform: &mut *self.platform,
                    fault: None,
                    scratch: 0x500,
            rom: crate::rom_region::Rom::Us,
                };
                let status = host.call_m_with(entry, args, m, rom);
                if host.fault.is_some() {
                    self.fault = host.fault;
                    return self.unbound(entry);
                }
                status
            }
        }
    }
    fn thread_id_m(&mut self) -> u32 {
        0x1ff82000
    }
    fn rom(&self) -> crate::rom_region::Rom {
        self.rom
    }
}

impl<P: crate::engine::HeapPlatform> crate::normalization::NormalizationHost for FeTextLive<'_, P> {

    fn calloc(&mut self, arena: &mut [u8], heap: u32, count: u32, size: u32) -> Option<u32> {
        let mut memory = Memory { base: 0, bytes: &mut *arena };
        let status = self.call_m(0x1963b8, &[heap, count, size], &mut memory);
        if status >= 0x8000_0000 || status == 0 {
            None
        } else {
            Some(status)
        }
    }
    fn alloc(&mut self, arena: &mut [u8], heap: u32, size: u32) -> Option<u32> {
        let mut memory = Memory { base: 0, bytes: &mut *arena };
        let status = self.call_m(0x196394, &[heap, size], &mut memory);
        if status >= 0x8000_0000 || status == 0 {
            None
        } else {
            Some(status)
        }
    }
    fn realloc(&mut self, arena: &mut [u8], heap: u32, ptr: u32, size: u32) -> Option<u32> {
        let mut memory = Memory { base: 0, bytes: &mut *arena };
        let status = self.call_m(0x1963e0, &[heap, ptr, size], &mut memory);
        if status >= 0x8000_0000 || status == 0 {
            None
        } else {
            Some(status)
        }
    }
    fn free(&mut self, arena: &mut [u8], heap: u32, ptr: u32) {
        let mut memory = Memory { base: 0, bytes: &mut *arena };
        self.call_m(0x196428, &[heap, ptr], &mut memory);
    }
    fn resolve(&mut self, _arena: &mut [u8], _h0: u32, _h1: u32) -> (u32, u32) {
        (self.unbound(0x19611c), 0)
    }
}

impl<P: crate::engine::HeapPlatform> crate::depes::HandleValidator for FeTextLive<'_, P> {

    fn validate(&mut self, handle: u32, tag: u32, _size: u32) -> u32 {
        if handle == tag {
            0
        } else {
            0x8170_6008
        }
    }
}

impl<P: crate::engine::HeapPlatform> crate::normalization::NormOutHost for FeTextLive<'_, P> {

    fn acquire(&mut self, _arena: &mut [u8], _resources: u32, _name: u32) -> (u32, u32) {
        (self.unbound(0x17b058), 0)
    }
    fn release_named(&mut self, _resources: u32, _name: u32) {}
    fn get_string(
        &mut self,
        _arena: &mut [u8],
        _ctlkp_h0: u32,
        _ctlkp_h1: u32,
        _out_value: u32,
        _out_len: u32,
        _out_sep: u32,
    ) -> u32 {
        self.unbound(0x17b1c4)
    }
}

impl<P: crate::engine::HeapPlatform> crate::fe_text::FeTextHost for FeTextLive<'_, P> {

    fn cmp_seq(&mut self, _arena: &mut [u8], _a: u32, _b: u32) -> i32 {
        0
    }
    fn lock_call(&mut self, _target: u32) -> u32 {
        self.unbound(0x176f2c)
    }

    fn lat_first_node(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        kind: u32,
        x: u32,
        node: &mut u16,
    ) -> u32 {
        fe_text_store_half(arena, FE_TEXT_HALF, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            self.call_m(0x15c544, &[h0, h1, kind, x, FE_TEXT_HALF], &mut memory)
        };
        if status < 0x8000_0000 {
            *node = fe_text_half(arena, FE_TEXT_HALF);
        }
        status
    }
    fn lat_next_node(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        next: &mut u16,
    ) -> u32 {
        fe_text_store_half(arena, FE_TEXT_HALF, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            self.call_m(0x15c7d8, &[h0, h1, node, FE_TEXT_HALF], &mut memory)
        };
        if status < 0x8000_0000 {
            *next = fe_text_half(arena, FE_TEXT_HALF);
        }
        status
    }

    fn lat_get_int(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        field: u32,
        n: u32,
        out: &mut u32,
        count: &mut u16,
    ) -> u32 {
        fe_text_store_word(arena, FE_TEXT_VALUE, 0);
        fe_text_store_half(arena, FE_TEXT_WRITTEN, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            self.call_m(
                0x15d0c4,
                &[h0, h1, node, field, n, FE_TEXT_VALUE, FE_TEXT_WRITTEN],
                &mut memory,
            )
        };
        if status < 0x8000_0000 {
            *out = fe_text_word(arena, FE_TEXT_VALUE);
            *count = fe_text_half(arena, FE_TEXT_WRITTEN);
        }
        status
    }
    fn lat_get_str(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        key: u32,
        out: &mut u32,
        count: &mut u16,
    ) -> u32 {
        fe_text_store_word(arena, FE_TEXT_POINTER, 0);
        fe_text_store_half(arena, FE_TEXT_LENGTH, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            self.call_m(
                0x15d378,
                &[h0, h1, node, key, FE_TEXT_POINTER, FE_TEXT_LENGTH],
                &mut memory,
            )
        };
        if status < 0x8000_0000 {
            *out = fe_text_word(arena, FE_TEXT_POINTER);
            *count = fe_text_half(arena, FE_TEXT_LENGTH);
            if key == 4 {
                self.mode = *out;
            }
        }
        status
    }
    fn lat_get_u16(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        key: u32,
        out: &mut u16,
    ) -> u32 {
        fe_text_store_half(arena, FE_TEXT_HALF, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            self.call_m(0x15d594, &[h0, h1, node, key, FE_TEXT_HALF], &mut memory)
        };
        if status < 0x8000_0000 {
            *out = fe_text_half(arena, FE_TEXT_HALF);
        }
        status
    }
    fn lat_set_field(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        key: u32,
        n: u32,
        data: u32,
        written: &mut u16,
    ) -> u32 {
        fe_text_store_half(arena, FE_TEXT_WRITTEN, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            self.call_m(
                0x15cdcc,
                &[h0, h1, node, key, n, data, FE_TEXT_WRITTEN],
                &mut memory,
            )
        };
        if status < 0x8000_0000 {
            *written = fe_text_half(arena, FE_TEXT_WRITTEN);
        }
        status
    }
    fn lat_set_field_bytes(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        key: u32,
        n: u32,
        data: &[u8],
        written: &mut u16,
    ) -> u32 {

        assert!(
            data.len() <= 4,
            "lat_set_field_bytes: {0} bytes exceed the 4-byte stage",
            data.len()
        );
        for (index, byte) in data.iter().enumerate() {
            if let Some(slot) = arena.get_mut((FE_TEXT_VALUE + index as u32) as usize) {
                *slot = *byte;
            }
        }
        for index in data.len()..4 {
            if let Some(slot) = arena.get_mut((FE_TEXT_VALUE + index as u32) as usize) {
                *slot = 0;
            }
        }
        fe_text_store_half(arena, FE_TEXT_WRITTEN, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            self.call_m(
                0x15cdcc,
                &[h0, h1, node, key, n, FE_TEXT_VALUE, FE_TEXT_WRITTEN],
                &mut memory,
            )
        };
        if status < 0x8000_0000 {
            *written = fe_text_half(arena, FE_TEXT_WRITTEN);
        }
        status
    }
    fn lat_count_nodes(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        kind: u32,
        x: u32,
        count: &mut u16,
    ) -> u32 {
        fe_text_store_half(arena, FE_TEXT_HALF, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            self.call_m(0x15d6c4, &[h0, h1, kind, x, FE_TEXT_HALF], &mut memory)
        };
        if status < 0x8000_0000 {
            *count = fe_text_half(arena, FE_TEXT_HALF);
        }
        status
    }
    fn lat_append(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        kind: u32,
        x: u32,
        node: &mut u16,
    ) -> u32 {
        fe_text_store_half(arena, FE_TEXT_HALF, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            self.call_m(0x15c744, &[h0, h1, kind, x, FE_TEXT_HALF], &mut memory)
        };
        if status < 0x8000_0000 {
            *node = fe_text_half(arena, FE_TEXT_HALF);
        }
        status
    }
    fn lat_insert_after(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        new: &mut u16,
    ) -> u32 {
        fe_text_store_half(arena, FE_TEXT_HALF, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };

            self.call_m(0x15c1e4, &[h0, h1, node, FE_TEXT_HALF], &mut memory)
        };
        if status < 0x8000_0000 {
            *new = fe_text_half(arena, FE_TEXT_HALF);
        }
        status
    }
    fn lat_insert_before(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        new: &mut u16,
    ) -> u32 {
        fe_text_store_half(arena, FE_TEXT_HALF, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            self.call_m(0x15c4b0, &[h0, h1, node, FE_TEXT_HALF], &mut memory)
        };
        if status < 0x8000_0000 {
            *new = fe_text_half(arena, FE_TEXT_HALF);
        }
        status
    }
    fn lat_remove(&mut self, arena: &mut [u8], h0: u32, h1: u32, node: u32) -> u32 {
        let mut memory = Memory { base: 0, bytes: &mut *arena };
        self.call_m(0x15da64, &[h0, h1, node], &mut memory)
    }
    fn lat_reset(&mut self, _arena: &mut [u8], _h0: u32, _h1: u32) -> u32 {
        self.unbound(0x15d038)
    }
    fn synth_convert(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _upper: u32,
        _phon: u32,
        _len: u32,
        _out: u32,
        _cap: u32,
    ) -> u32 {
        self.unbound(0x1ed6c4)
    }
    fn dict_lookup(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _table: u32,
        _key: u32,
        _entry: &mut u32,
        _found: &mut u16,
        _sep: &mut u8,
    ) -> u32 {
        self.unbound(0x198760)
    }
    fn dict_annotate(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _h2: u32,
        _h3: u32,
        _toks: u32,
        _count: u32,
    ) -> u32 {
        self.unbound(0x198b54)
    }
    fn res_available(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _avail: &mut u32,
        _end: &mut u8,
    ) -> u32 {
        self.unbound(0x196a64)
    }
    fn res_read(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _data: &mut u32,
        _bytes: &mut u32,
    ) -> u32 {
        self.unbound(0x196db0)
    }
    fn res_consume(&mut self, _arena: &mut [u8], _h0: u32, _h1: u32, _n: u32) -> u32 {
        self.unbound(0x19690c)
    }
    fn lat_type_name(&mut self, arena: &mut [u8], _h0: u32, _h1: u32, _ty: u32, out: &mut u32) {
        *out = 0;
        let _ = arena;
    }
    fn dct_get_param(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _sect: u32,
        _key: u32,
        _val: &mut u32,
        _found: &mut u16,
        _n: &mut i32,
    ) -> u32 {
        self.unbound(0x175944)
    }
    fn dct_get_flags(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _a: &mut u8,
        _b: &mut u8,
    ) -> u32 {
        self.unbound(0x1756d0)
    }

    fn depes_set_input(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        layer: u32,
        s: u32,
        n: u32,
    ) -> u32 {
        let mut memory = Memory { base: 0, bytes: &mut *arena };
        self.call_m(0x218f88, &[h0, h1, layer, s, n], &mut memory)
    }
    fn depes_get_output(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        layer: u32,
        out: &mut u32,
        len: &mut u16,
    ) -> u32 {
        fe_text_store_word(arena, FE_TEXT_POINTER, 0);
        fe_text_store_half(arena, FE_TEXT_LENGTH, 0);
        let status = {
            let mut memory = Memory { base: 0, bytes: &mut *arena };
            self.call_m(
                0x2191c8,
                &[h0, h1, layer, FE_TEXT_POINTER, FE_TEXT_LENGTH],
                &mut memory,
            )
        };
        if status < 0x8000_0000 {
            *out = fe_text_word(arena, FE_TEXT_POINTER);
            *len = fe_text_half(arena, FE_TEXT_LENGTH);
        }
        status
    }

    fn depes_set_rules(&mut self, arena: &mut [u8], h0: u32, h1: u32, _name: u32) -> u32 {
        let length = self.build_rule(arena, true);
        let status = self.exec_rule(arena, h0, h1, length);
        if status >= 0x8000_0000 {
            let length = self.build_rule(arena, false);
            return self.exec_rule(arena, h0, h1, length);
        }
        status
    }

    fn depes_set_weights(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        w: u32,
        n: u32,
    ) -> u32 {
        if h0 == 0 || h1 != 0xf37e {
            return 0x8970_2008;
        }
        let target = fe_text_word(arena, h0.wrapping_add(0x2c));
        if target == 0 {
            return 0x8970_200a;
        }
        let capacity = u32::from(fe_text_half(arena, h0.wrapping_add(0x32)));
        let length = n.min(capacity);
        let mut index = 0u32;
        while index < length {
            let byte = fe_text_byte(arena, w.wrapping_add(index));
            if let Some(slot) = arena.get_mut((target.wrapping_add(index)) as usize) {
                *slot = byte;
            }
            index += 1;
        }
        while index < capacity {
            if let Some(slot) = arena.get_mut((target.wrapping_add(index)) as usize) {
                *slot = 0;
            }
            index += 1;
        }
        0
    }
    fn depes_get_weights(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        out: &mut u32,
        len: &mut u16,
    ) -> u32 {
        if h0 == 0 || h1 != 0xf37e {
            return 0x8970_2008;
        }
        *out = fe_text_word(arena, h0.wrapping_add(0x2c));
        *len = fe_text_half(arena, h0.wrapping_add(0x34));
        0
    }

    fn prop_get(&mut self, arena: &mut [u8], h0: u32, h1: u32, pos: &mut i32, flag: u32) -> u32 {
        fe_text_store_word(arena, FE_TEXT_POINTER, 0);
        let status = self.get_option(arena, h0, h1, FE_TEXT_POINTER, flag);
        if status < 0x8000_0000 {
            *pos = fe_text_word(arena, FE_TEXT_POINTER) as i32;
        }
        status
    }
    fn prop_set(&mut self, arena: &mut [u8], h0: u32, h1: u32, val: u32, flag: u32) -> u32 {
        self.set_option(arena, h0, h1, val, flag)
    }

    fn char_punctuation(&mut self, arena: &mut [u8], h0: u32, h1: u32, c: u32) -> i32 {
        if h0 == 0 || h1 != 0xf37f || c > 0xff {
            return 0;
        }
        let class = fe_text_word(arena, h0.wrapping_add(100));
        if class == 0 {
            return 0;
        }
        let mut at = class;
        loop {
            let byte = fe_text_byte(arena, at);
            if byte == 0 {
                return 0;
            }
            if byte == c as u8 {
                return 1;
            }
            at = at.wrapping_add(1);
        }
    }
    fn log_level(&mut self, arena: &mut [u8], log: u32) -> u32 {
        let mut memory = Memory { base: 0, bytes: &mut *arena };
        self.call_m(0x173fd8, &[log], &mut memory)
    }

    fn dp_first_node(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _kind: u32,
        _x: u32,
        _node: &mut u16,
    ) -> u32 {
        self.unbound(0x15c544)
    }
    fn dp_next_node(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _node: u32,
        _next: &mut u16,
    ) -> u32 {
        self.unbound(0x15c7d8)
    }
    fn dp_get(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _node: u32,
        _field: u32,
        _n: u32,
        _out: &mut u32,
        _count: &mut u16,
    ) -> u32 {
        self.unbound(0x15d0c4)
    }
    fn dp_get_ptr(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _node: u32,
        _key: u32,
        _out: &mut u32,
        _count: &mut u16,
    ) -> u32 {
        self.unbound(0x15d378)
    }
    fn dp_get_type(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _node: u32,
        _key: u32,
        _out: &mut u16,
    ) -> u32 {
        self.unbound(0x15d594)
    }
    fn dp_set(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _node: u32,
        _key: u32,
        _n: u32,
        _data: u32,
        _written: &mut u16,
    ) -> u32 {
        self.unbound(0x15cdcc)
    }
    fn dp_remove(&mut self, _arena: &mut [u8], _h0: u32, _h1: u32, _node: u32) -> u32 {
        self.unbound(0x15da64)
    }

    fn out_get_pos(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _pos: &mut i32,
        _flag: u32,
    ) -> u32 {
        self.unbound(0x21958c)
    }
    fn out_set_layer(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _layer: u32,
        _s: u32,
        _n: u32,
    ) -> u32 {
        self.unbound(0x218f88)
    }
    fn out_set_layer_x(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _layer: u32,
        _s: u32,
        _n: u32,
    ) -> u32 {
        self.unbound(0x218ba4)
    }
    fn out_process(&mut self, _arena: &mut [u8], _h0: u32, _h1: u32, _name: u32) -> u32 {
        self.unbound(0x2193a4)
    }
    fn out_get_layer(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _layer: u32,
        _out: &mut u32,
        _len: &mut u16,
    ) -> u32 {
        self.unbound(0x2191c8)
    }
    fn out_set_pos(&mut self, _arena: &mut [u8], _h0: u32, _h1: u32, _pos: i32, _flag: u32) -> u32 {
        self.unbound(0x219554)
    }

    fn st_open(
        &mut self,
        _arena: &mut [u8],
        _ha: u32,
        _hb: u32,
        _ty: u32,
        _name: u32,
        _a: u32,
        _b: u32,
        _out_a: &mut u32,
        _out_b: &mut u32,
    ) -> u32 {
        self.unbound(0x17b0d4)
    }
    fn st_open4(
        &mut self,
        _arena: &mut [u8],
        _ha: u32,
        _hb: u32,
        _ty: u32,
        _name: u32,
        _a: u32,
        _b: u32,
        _out_a: &mut u32,
        _out_b: &mut u32,
    ) -> u32 {
        self.unbound(0x17b0d4)
    }
    fn st_close(&mut self, _arena: &mut [u8], _ha: u32, _hb: u32) -> u32 {
        self.unbound(0x17b098)
    }
    fn st_get_mode(&mut self, _arena: &mut [u8]) -> u32 {
        self.unbound(0x17b0c4)
    }
    fn st_read(
        &mut self,
        _arena: &mut [u8],
        _ha: u32,
        _hb: u32,
        _n: &mut i32,
        _fin: &mut u8,
    ) -> u32 {
        self.unbound(0x196df0)
    }
    fn st_gettext(
        &mut self,
        _arena: &mut [u8],
        _ha: u32,
        _hb: u32,
        _buf: &mut u32,
        _len: &mut i32,
    ) -> u32 {
        self.unbound(0x196e04)
    }
    fn st_write(&mut self, _arena: &mut [u8], _ha: u32, _hb: u32, _buf: u32, _len: u32) -> u32 {
        self.unbound(0x196e7c)
    }
    fn st_consume(&mut self, _arena: &mut [u8], _ha: u32, _hb: u32, _n: u32) -> u32 {
        self.unbound(0x196e94)
    }
    fn depes_get_pos(
        &mut self,
        _arena: &mut [u8],
        _h0: u32,
        _h1: u32,
        _pos: &mut i32,
        _flag: u32,
    ) -> u32 {
        self.unbound(0x21958c)
    }
    fn depes_set_pos(&mut self, _arena: &mut [u8], _h0: u32, _h1: u32, _pos: i32, _flag: u32) -> u32 {
        self.unbound(0x219554)
    }
    fn dict_count(&mut self, _arena: &mut [u8], _ha: u32, _hb: u32) -> u32 {
        self.unbound(0x198760)
    }
    fn suba1c(&mut self, _arena: &mut [u8], _reg: u32, _key: u32, _out: &mut u32) -> u32 {
        self.unbound(0x282af0)
    }
    fn tn_282af0(
        &mut self,
        _arena: &mut [u8],
        _ctx: u32,
        _dict: u32,
        _cb: u32,
        _ha: u32,
        _hb: u32,
        _all_transforms: u32,
        _input: u32,
        _content_type: u32,
        _output: u32,
        _capacity: u32,
        _lang: u32,
    ) -> u32 {
        self.unbound(0x282af0)
    }
}

pub fn fn_1dd914_m(
    m: &mut Memory<'_>,
    a: &[u32],
    changed: u32,
    host: &mut impl crate::fe_text::FeTextHost,
) -> u32 {
    if a.len() < 4 {
        return 0x89c0_2008;
    }
    crate::fe_text::fe_punctspat_1dd914(m.bytes, a[0], a[1], a[2], a[3], changed, host)
}
