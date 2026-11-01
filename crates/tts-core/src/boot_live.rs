
use crate::boot::{Calls, Memory};

struct Db<'a, H>(&'a mut H);
impl<H: Calls> crate::db_live::PoolHooksM for Db<'_, H> {
    fn log_pool_m(&mut self, _: u32, _: u32, _: &mut Memory<'_>) {
        unreachable!("vector adapters do not log pools")
    }
}
impl<H: Calls> crate::db_live::DbHooksM for Db<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.call(m, f, a)
    }
}

pub fn register_owner_m(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, key: u32,
    owner: [u32;3], def: [u32;3], scratch: u32) -> u32 {
    if p == 0 || owner[2] == 0 { return 0x8b702007; }
    let mut result=h.call(m,0x176f2c,&[m.word(p+4)]);
    if result>=0x80000000 {return result;}
    let saved: [u32;6]=core::array::from_fn(|i|m.word(scratch+i as u32*4));
    m.put_word(scratch,0);
    for (i,v) in owner.into_iter().enumerate() {m.put_word(scratch+4+i as u32*4,v);}
    let mut target=0;
    if key!=0 && m.bytes[m.offset(key)]!=0 {
        result=h.call(m,0x1966ac,&[m.word(p+8),key,scratch]);
        if result>=0x80000000 {
            result=(result&0x1fff)|0x8b702000;
        } else {
            let value=m.word(scratch);
            if m.word(value+8)==0 {
                let ok=h.call(m,0x196a64,&[m.word(p),def[0],def[1],def[2],1,value+8]);
                if ok==0 {result=0x8b70200a;} else {target=m.word(value+8);}
            } else {target=m.word(value+8);}
        }
    } else {target=m.word(p+12);}
    if result<0x80000000 {

        if crate::db_live::append_element_m(&mut Db(h),m,target,scratch+4,scratch+16)==0 {result=0x8b70200a;}
    }
    for (i,v) in saved.into_iter().enumerate() {m.put_word(scratch+i as u32*4,v);}
    let rc=h.call(m,0x176f5c,&[m.word(p+4)]);
    if rc>=0x80000000 && result<0x80000000 {result=rc;}
    result
}

pub fn alloc_object_m(m:&mut Memory<'_>,h:&mut impl Calls,heap:u32,out:u32)->u32 {
    if out==0 {return 0x87f66006;}
    let p=h.call(m,0x196394,&[heap,8]);
    if p==0 {return 0x87f6600a;}
    m.put_word(p+4,heap);m.put_word(p,0);m.put_word(out,p);0
}

pub fn release_guard_m(m:&mut Memory<'_>,h:&mut impl Calls,p:u32)->u32 {
    if p==0 {return 0x81402007;}
    let target=m.word(m.word(m.word(p)+8)+4);
    let rc=if target==0 {0}else{h.call(m,target,&[m.word(p+8)])};
    if rc<0x80000000 {h.call(m,0x196428,&[m.word(p+4),p]);}
    rc
}

pub fn release_iterator_m(m:&mut Memory<'_>,h:&mut impl Calls,p:u32)->u32 {
    if p==0 {return 0x89002007;}
    h.call(m,0x196428,&[m.word(m.word(p)+20),p]);0
}

pub fn dispatch(m:&mut Memory<'_>,h:&mut impl Calls,f:u32,a:&[u32],def:[u32;3],scratch:u32)->Option<u32> {
    Some(match f {
        0x175170=>set_parameter_m(m,h,a[0],a[1],a[2],a[3],a[4],scratch),
        0x174b38=>update_attr_m(m,h,a[0],a[1],a[2],a[3],a[4],scratch),
        0x176054=>store_sized_m(m,h,a[0],a[1],a[2],a[3]),
        0x176104=>element_callbacks_m(m,h,a[0],a[1],a[2],a[3],scratch),
        0x17eb90=>license_token_m(m,h,a[0],a[1],a[2],scratch),
        0x174e74=>register_owner_m(m,h,a[0],a[1],[a[2],a[3],a[4]],def,scratch),
        0x160b0c=>alloc_object_m(m,h,a[0],a[1]),
        0x176edc=>release_guard_m(m,h,a[0]),
        0x196830=>release_iterator_m(m,h,a[0]),
        0x196db0=>crate::db_live::append_element_m(&mut Db(h),m,a[0],a[1],scratch),
        0x196cfc=>crate::db_live::append_slot_m(&mut Db(h),m,a[0],a[1]),
        _=>return None,
    })
}
fn put_byte(m:&mut Memory<'_>,p:u32,v:u8) {let i=m.offset(p);m.bytes[i]=v;}
fn half(m:&Memory<'_>,p:u32)->u32 {let i=m.offset(p);u16::from_le_bytes(m.bytes[i..i+2].try_into().unwrap()) as u32}
fn read_guest(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::ArchiveHost,
    f:u32,ctx:u32,dst:u32,n:u32,workspace:&mut[u8])->u32 {

    let at=m.offset(dst);let n=n as usize;
    workspace[..n].copy_from_slice(&m.bytes[at..at+n]);
    let got=h.read(m,f,ctx,&mut workspace[..n]);
    m.bytes[at..at+n].copy_from_slice(&workspace[..n]);got
}

pub fn stream_read_m(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::ArchiveHost,
    p:u32,buf:u32,len:u32,out:u32,a5:u32,a6:u32,workspace:&mut[u8])->u32 {
    put_byte(m,a5,if half(m,p+0x24)!=0 {0}else{2});put_byte(m,a6,0);
    if m.word(p+0x38)==0 {m.put_word(out,0);return 0;}
    if half(m,p+0x24)==0 {
        let count=m.word(p+0x38).min(len);
        let s=m.word(p+0x30);
        let got=read_guest(m,h,m.word(s),m.word(p+0x34),buf,count,workspace);
        if got!=count {
            if crate::boot_part_e::Host::call(h,m,m.word(s+8),&[m.word(p+0x34)])!=0 {
                return crate::normalization::code_6(0);
            }
            put_byte(m,a6,1);
        }
        m.put_word(p+0x38,m.word(p+0x38).wrapping_sub(got));m.put_word(out,got);return 0;
    }
    let mut count=m.word(p+0x38).min(len);let mut total=0u32;let mut b=buf;
    while count!=0 {
        let s=m.word(p+0x30);let n=m.word(p+0x3c).min(count);
        let got=read_guest(m,h,m.word(s),m.word(p+0x34),b,n,workspace);
        if got!=n {
            if crate::boot_part_e::Host::call(h,m,m.word(s+8),&[m.word(p+0x34)])!=0 {
                return crate::normalization::code_6(0);
            }
            put_byte(m,a6,1);break;
        }
        if m.bytes[m.offset(p+0x14)]!=0 {
            let hash=crate::normalization::adler32(m.bytes,m.word(p+0x40),b.wrapping_sub(m.base),got);
            m.put_word(p+0x40,hash);
        }
        m.put_word(p+0x3c,m.word(p+0x3c).wrapping_sub(got));
        if m.word(p+0x3c)==0 {
            let s=m.word(p+0x30);let mut hdr=[0u8;4];
            if h.read(m,m.word(s),m.word(p+0x34),&mut hdr)!=4 {
                if crate::boot_part_e::Host::call(h,m,m.word(s+8),&[m.word(p+0x34)])!=0 {
                    return crate::normalization::code_6(0);
                }
                put_byte(m,a6,1);break;
            }
            if m.bytes[m.offset(p+0x14)]!=0 {
                if m.word(p+0x40)!=u32::from_le_bytes(hdr) {put_byte(m,a5,1);return 0;}
                let hash=crate::normalization::adler32(m.bytes,0,0,0);m.put_word(p+0x40,hash);
            }
            let n=m.word(p+0x38).wrapping_sub(total).wrapping_sub(got);
            m.put_word(p+0x3c,half(m,p+0x24).min(n));
        }
        count=count.wrapping_sub(got);b=b.wrapping_add(got);total=total.wrapping_add(got);
    }
    m.put_word(p+0x38,m.word(p+0x38).wrapping_sub(total));m.put_word(out,total);0
}

pub fn license_token_m(m:&mut Memory<'_>,h:&mut impl Calls,object:u32,key:u32,out:u32,scratch:u32)->u32 {
    let saved=m.word(scratch);m.put_word(scratch,0);
    let result=h.call(m,0x19611c,&[object,key,scratch]);
    let context=if result>=0x80000000 {0}else{m.word(scratch)};
    m.put_word(scratch,saved);
    if context==0 || m.word(context+4)==0 {7}
    else if out==0 {6}
    else {m.put_word(out,0);0}
}

pub struct RootData { pub words:[u32;9] }

pub fn root_tail_m(m:&mut Memory<'_>,h:&mut impl Calls,engine:u32,mode:u32,delta:u32,d:&RootData,s:u32) {
    let saved=core::array::from_fn::<_,0xc30,_>(|i|m.bytes[m.offset(s+i as u32)]);
    let w=d.words;
    m.put_word(s+0xc18,0);m.put_word(s+0xc1c,w[0]);
    h.call(m,0x191948,&[m.word(engine+0x11c+delta),m.word(engine+0x120+delta),s+0xc18]);
    for i in 0..10u32 {
        put_byte(m,s+i*0x102,m.bytes[m.offset(w[1]+i*4)]);
        let value=match i {0=>m.word(w[2]+mode*4),2=>m.word(w[2]+32+mode*4),3=>16,_=>half(m,w[1]+i*4+2)};
        let at=m.offset(s+i*0x102+2);m.bytes[at..at+2].copy_from_slice(&(value as u16).to_le_bytes());
    }
    for i in 0..2u32 {
        let dst=s+(10+i)*0x102;
        put_byte(m,dst,m.bytes[m.offset(w[4]+i*0x102)]);
        let mut src=if i==0 {m.word(w[3]+mode*4)}else{w[4]+i*0x102+2};
        let mut end=dst+2;
        for _ in 0..127 {
            let v=half(m,src) as u16;let at=m.offset(end);m.bytes[at..at+2].copy_from_slice(&v.to_le_bytes());
            src+=2;end+=2;if half(m,src)==0 {break;}
        }
        let at=m.offset(end);m.bytes[at..at+2].fill(0);
    }
    h.call(m,0x19310c,&[m.word(engine+0x11c+delta),m.word(engine+0x120+delta),s,12]);
    let buffer=engine+0x61fc+delta;let size=engine+0x69fc+delta;
    h.call(m,0x3023b0,&[buffer,0x800]);
    let src=h.call(m,0x16b540,&[m.word(engine+0x61bc+delta),m.word(w[5]+mode*4),size]);
    let n=m.word(size).min(0x7ff);let mut i=0;
    while i<n {
        let b=m.bytes[m.offset(src+i)];put_byte(m,buffer+i,b);i+=1;
        if m.bytes[m.offset(src+i)]==0 {break;}
    }
    put_byte(m,buffer+i,0);
    h.call(m,0x194454,&[m.word(engine+0x11c+delta),m.word(engine+0x120+delta),w[6],m.word(size),buffer,engine+0x61f4+delta]);
    for i in 0..18 {m.put_word(w[3]+72+i*4,m.word(w[7]+mode*72+i*4));}
    for j in 0..4 {for i in 0..5 {

        m.put_word(engine+0xac+delta+i*16+j*4,m.word(w[8]+mode*80+j*20+i*4));
    }}
    for (i,b) in saved.into_iter().enumerate(){put_byte(m,s+i as u32,b);}
}

fn mem_word(mem:&[u8],p:u32)->u32 {u32::from_le_bytes(mem[p as usize..p as usize+4].try_into().unwrap())}
fn mem_store(mem:&mut[u8],p:u32,v:u32){mem[p as usize..p as usize+4].copy_from_slice(&v.to_le_bytes());}
use crate::frontend::classify_code;
pub trait SetOutDeviceHooksM {
 fn log_begin_m(&mut self,mem:&mut[u8],log:u32);
 fn resolve_ctx_m(&mut self,mem:&mut[u8],a:u32,b:u32,out:&mut u32)->u32;
 fn set_state_m(&mut self,mem:&mut[u8],obj:u32,locked:&mut u8)->u32;
 fn flag_state_m(&mut self,mem:&mut[u8],obj:u32,locked:u8);
 fn log_end_m(&mut self,mem:&mut[u8],log:u32);
}
impl<T:crate::frontend::SetOutDeviceHooks> SetOutDeviceHooksM for T {
 fn log_begin_m(&mut self,_:&mut[u8],log:u32){self.log_begin(log)}
 fn resolve_ctx_m(&mut self,_:&mut[u8],a:u32,b:u32,out:&mut u32)->u32{self.resolve_ctx(a,b,out)}
 fn set_state_m(&mut self,_:&mut[u8],obj:u32,locked:&mut u8)->u32{self.set_state(obj,locked)}
 fn flag_state_m(&mut self,_:&mut[u8],obj:u32,locked:u8){self.flag_state(obj,locked)}
 fn log_end_m(&mut self,_:&mut[u8],log:u32){self.log_end(log)}
}
pub fn set_out_device_m<H: SetOutDeviceHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    handle: Option<u32>,
    tag: u32,
    device: Option<u32>,
) -> u32 {
    let Some(object) = handle else {
        return 0x80000008;
    };
    if tag != 0x684f424a || mem_word(mem, object) != 0x734f424a {
        return 0x80000008;
    }
    let manager = mem_word(mem, object.wrapping_add(0x10));
    hooks.log_begin_m(mem, mem_word(mem, manager.wrapping_add(0x10)));
    let mut ctx = 0u32;
    let mut result = hooks.resolve_ctx_m(mem,
        mem_word(mem, object.wrapping_add(8)),
        mem_word(mem, object.wrapping_add(0xc)),
        &mut ctx,
    );
    if result >= 0x80000000 {
        return classify_code(result);
    }
    let Some(dev) = device else {
        return 0x80000006;
    };
    let second = mem_word(mem, dev.wrapping_add(4));
    if second == 0 {
        return 0x80000012;
    }
    let mut locked = 0u8;
    result = hooks.set_state_m(mem, object, &mut locked);
    if result >= 0x80000000 {
        return classify_code(result);
    }
    mem_store(mem, object.wrapping_add(0x170), second);
    mem_store(mem, object.wrapping_add(0x16c), mem_word(mem, dev));
    hooks.flag_state_m(mem, object, locked);
    let manager = mem_word(mem, object.wrapping_add(0x10));
    hooks.log_end_m(mem, mem_word(mem, manager.wrapping_add(0x10)));
    classify_code(result)
}

pub trait StateLockM {
 fn acquire_m(&mut self,mem:&mut[u8])->u32;
 fn release_m(&mut self,mem:&mut[u8])->u32;
}
impl<T:crate::frontend::StateLock> StateLockM for T {
 fn acquire_m(&mut self,_:&mut[u8])->u32{self.acquire()}
 fn release_m(&mut self,_:&mut[u8])->u32{self.release()}
}

pub fn set_lh_state_m(
    mem: &mut [u8],
    p: u32,
    _name: u32,
    val: u32,
    out: u32,
    lock: &mut impl StateLockM,
) -> u32 {
    let mut rc = lock.acquire_m(mem);
    if rc >= 0x80000000 {
        return rc;
    }
    if mem[p as usize + 40] == 1 {
        mem[out as usize] = mem[p as usize + 40];
        mem[p as usize + 40] = val as u8;
    } else {
        rc = 0x80002011;
    }
    let rc2 = lock.release_m(mem);
    if rc2 >= 0x80000000 && rc < 0x80000000 {
        rc = rc2;
    }
    rc
}

pub fn flag_lh_error_m(mem: &mut [u8], p: u32, v: u32, lock: &mut impl StateLockM) {
    if v == 0 {
        return;
    }
    if lock.acquire_m(mem) >= 0x80000000 {
        return;
    }
    mem[p as usize + 40] = v as u8;
    lock.release_m(mem);
}

use crate::frontend::{copy_bytes,copy_str,append_str,fill_bytes,copy_utf16_as_ascii,strlen_bytes,resolve_engine_context};
fn mem_half(mem:&[u8],p:u32)->u16{u16::from_le_bytes(mem[p as usize..p as usize+2].try_into().unwrap())}
fn nul_len(s:&[u8])->usize{s.iter().position(|v|*v==0).unwrap_or(s.len())}
pub trait ParamHooksM {
 fn state_acquire_m(&mut self,mem:&mut[u8],object:u32)->u32;
 fn state_release_m(&mut self,mem:&mut[u8],object:u32)->u32;
 fn configure_m(&mut self,a:u32,b:u32,resource:u32,synthesis:u32,converted:u32,mem:&mut[u8])->(u32,[u8;0x128]);
 fn query_m(&mut self,resource:u32,spec:u32,mem:&mut[u8])->(u32,u32);
 fn notify_m(&mut self,resource:u32,value:u32,mem:&mut[u8]);
 fn set_num_m(&mut self,params:u32,key:&[u8],value:u32,mem:&mut[u8])->u32;
 fn set_str_m(&mut self,params:u32,key:&[u8],value:&[u8],mem:&mut[u8])->u32;
 fn resolve_m(&mut self,manager:u32,key:&[u8],mem:&mut[u8])->(u32,u32,u16);
 fn release_m(&mut self,heap:u32,value:u32,mem:&mut[u8]);
 fn set_voice_m(&mut self,a:u32,b:u32,spec:Option<&[u8]>,flags:u32,mem:&mut[u8])->u32;
}
fn enter_request_live(mem:&mut[u8],object:u32,next:u8,previous:&mut u8,h:&mut impl ParamHooksM)->u32{
 let mut rc=h.state_acquire_m(mem,object);if rc>=0x80000000{return rc;}
 if mem[object as usize+40]==1{*previous=mem[object as usize+40];mem[object as usize+40]=next;}
 else{rc=0x80002011;}
 let unlock=h.state_release_m(mem,object);if unlock>=0x80000000 && rc<0x80000000{rc=unlock;}rc
}
fn restore_request_live(mem:&mut[u8],object:u32,previous:u32,h:&mut impl ParamHooksM){
 if previous!=0 && h.state_acquire_m(mem,object)<0x80000000{
  mem[object as usize+40]=previous as u8;h.state_release_m(mem,object);
 }
}

pub fn configure_voice_m(
    mem: &mut [u8],
    object: u32,
    mode: u32,
    voice: u32,
    converted: u32,
    freq_buf: u32,
    lit_pcm: u32,
    lit_pipeline: u32,
    lit_slash: u32,
    lit_format_pcm: u32,
    lit_phon: u32,
    pair: &[u8; 8],
    default_ctx: u32,
    h: &mut impl ParamHooksM,
) -> u32 {
    let spec = voice + 0x92;
    let engine = mem_word(mem, object + 8);
    let p_word = if engine == 0 {
        None
    } else {
        Some(mem_half(mem, engine))
    };
    let mut context = 0u32;
    let result = resolve_engine_context(
        engine,
        p_word,
        mem_word(mem, object + 12),
        pair,
        default_ctx,
        &mut context,
    );
    if result >= 0x80000000 {
        return result;
    }
    let resource = mem_word(mem, context + 0x0c);
    let synthesis = mem_word(mem, context + 0x14);
    if mode == 0 {
        fill_bytes(mem, converted, 0, 0x128);
        copy_str(mem, converted, voice);
        copy_str(mem, converted + 0x88, voice + 0x40);
        let freq = mem_half(mem, voice + 0x60);
        let converted_freq = match freq {
            11 => 0x2b11,
            22 => 0x5622,
            _ => u32::from(freq) * 1000,
        };
        mem_store(mem, converted + 0xb8, converted_freq);
        copy_str(mem, converted + 0xbc, voice + 0x62);

        copy_str(mem, converted + 0xf4, voice + 0x82);
        copy_str(mem, converted + 0x104, lit_pcm);
        let methods = mem_word(mem, object + 4);
        let (rc, script) = h.configure_m(
            mem_word(mem, methods + 0xcc),
            mem_word(mem, methods + 0xd0),
            resource,
            synthesis, converted, mem);
        if rc >= 0x80000000 {
            return rc;
        }
        mem[converted as usize..converted as usize + 0x128].copy_from_slice(&script);
        copy_str(mem, voice, converted);
        copy_str(mem, voice + 0x40, converted + 0x88);
        let back = (mem_word(mem, converted + 0xb8) / 1000) as u16;
        mem[voice as usize + 0x60..voice as usize + 0x62]
            .copy_from_slice(&back.to_le_bytes());
        copy_str(mem, voice + 0x62, converted + 0xbc);
        copy_str(mem, voice + 0x82, converted + 0xf4);
    }
    let freq = mem_half(mem, voice + 0x60);
    let _ = crate::frontend::render_unsigned(
        u32::from(freq),
        &mut mem[freq_buf as usize..freq_buf as usize + 32],
        10,
    );
    copy_str(mem, spec, lit_pipeline);
    append_str(mem, spec, voice);
    append_str(mem, spec, lit_slash);
    append_str(mem, spec, voice + 0x40);
    append_str(mem, spec, lit_slash);
    append_str(mem, spec, freq_buf);
    if mem[voice as usize + 0x62] != 0 {
        let v = &mem[voice as usize + 0x62..voice as usize + 0x67];
        let full = (v[0] == b'f' || v[0] == b'F')
            && (v[1] == b'u' || v[1] == b'U')
            && (v[2] == b'l' || v[2] == b'L')
            && (v[3] == b'l' || v[3] == b'L')
            && v[4] == 0;
        if !full {
            append_str(mem, spec, lit_slash);
            append_str(mem, spec, voice + 0x62);
        }
    }
    match mem[voice as usize + 0x112] {
        0 | 2 => {
            append_str(mem, spec, lit_slash);
            append_str(mem, spec, voice + 0x82);
            append_str(mem, spec, lit_format_pcm);
        }
        1 => {
            append_str(mem, spec, lit_phon);
        }
        _ => return 0x80002001,
    }
    let (qrc, value) = h.query_m(resource, spec, mem);
    if qrc >= 0x80000000 {
        0x80002000
    } else {
        h.notify_m(resource, value, mem);
        result
    }
}

fn finish_param_list_m(
    mem: &mut [u8],
    object: u32,
    voice: u32,
    mode: u8,
    input: u8,
    status: u32,
    changed: u32,
    acquired: u32,
    locked: u8,
    h: &mut impl ParamHooksM,
) -> u32 {
    if changed & 2 != 0 {
        mem[object as usize + 0x34] = mode;
        mem[object as usize + 0x35] = input;
        if changed & 1 != 0 {
            let params_base = object + 0x36;
            mem.copy_within(voice as usize..voice as usize+0x114,params_base as usize);
        }
    }
    if acquired != 0 && locked != 0 {
        restore_request_live(mem,object,u32::from(locked),h);
    }
    classify_code(status)
}

pub fn set_param_list_m(
    mem: &mut [u8],
    handle: Option<u32>,
    tag: u32,
    params: Option<u32>,
    count: u32,
    voice: u32,
    buffer: u32,
    converted: u32,
    freq_buf: u32,
    lit_pcm: u32,
    lit_pipeline: u32,
    lit_slash: u32,
    lit_format_pcm: u32,
    lit_phon: u32,
    pair: &[u8; 8],
    default_ctx: u32,
    h: &mut impl ParamHooksM,
) -> u32 {
    let Some(object) = handle else {
        return 0x80000008;
    };
    if tag != 0x684f424a || mem_word(mem, object) != 0x734f424a {
        return 0x80000008;
    }
    let Some(params) = params else {
        return 0x80000006;
    };
    let engine_first = mem_word(mem, object + 8);
    let p_word = if engine_first == 0 {
        None
    } else {
        Some(mem_half(mem, engine_first))
    };
    let mut context = 0u32;
    let status = resolve_engine_context(
        engine_first,
        p_word,
        mem_word(mem, object + 12),
        pair,
        default_ctx,
        &mut context,
    );
    if status >= 0x80000000 {
        return classify_code(status);
    }
    let params_base = object + 0x36;
    mem.copy_within(params_base as usize..params_base as usize+0x114,voice as usize);
    let mut input = mem[object as usize + 0x35];
    let mut mode = mem[object as usize + 0x34];
    let mut changed = 0u32;
    let mut frequency_set = 0u32;
    let mut variant_set = 0u32;
    let mut voice_id = 0u16;
    let mut resolved = 0u32;
    let mut acquired = 0u32;
    let mut locked = 0u8;
    let mut status = status;
    if !((mode & 8 != 0) && (mode & 4 != 0)) && (count > 1 || mem[params as usize] != 13) {
        return classify_code(0x80002011);
    }
    if mem[voice as usize + 0x92] == 0 {
        for i in 0..count {
            let id = mem[params as usize + i as usize * 0x102];
            if id == 1 || id == 7 || id == 15 || id == 8 || id == 19 {
                changed = 1;
                break;
            }
        }
        if changed == 0 {
            status = 0x80002011;
            return finish_param_list_m(
                mem, object, voice, mode, input, status, changed, acquired, locked, h,
            );
        }
    }
    for i in 0..count {
        let id = mem[params as usize + i as usize * 0x102];
        if id != 3 && id != 4 && id != 2 && id != 5 {
            acquired = 1;
            status = enter_request_live(mem, object, 2, &mut locked, h);
            if status >= 0x80000000 {
                locked = 0;
                return finish_param_list_m(
                    mem, object, voice, mode, input, status, changed, acquired, locked, h,
                );
            }
            break;
        }
    }
    let engine = mem_word(mem, context + 0x14);
    let manager = mem_word(mem, object + 0x10);
    let zero_key = [0u8; 4];
    for i in 0..count {
        let base = params + i * 0x102;
        let id = mem[base as usize];
        let number = mem_half(mem, base + 2);
        let text = base + 2;
        match id {
            3 => status = h.set_num_m(engine, b"rate", u32::from(number), mem),
            4 => status = h.set_num_m(engine, b"pitch", u32::from(number), mem),
            1 => {
                if number != 8 && number != 11 && number != 16 && number != 22 {
                    status = 0x80002007;
                    return finish_param_list_m(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, h,
                    );
                }
                changed = 1;
                mem[voice as usize + 0x60..voice as usize + 0x62]
                    .copy_from_slice(&number.to_le_bytes());
                frequency_set = 1;
            }
            12 => {
                if number == 0 {
                    status = h.set_num_m(engine, b"markermode", 0, mem);
                } else if number == 1 {
                    status = h.set_num_m(engine, b"markermode", 0x4af, mem);
                } else {
                    status = 0x80002007;
                    return finish_param_list_m(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, h,
                    );
                }
            }
            2 => status = h.set_num_m(engine, b"volume", u32::from(number), mem),
            5 => status = h.set_num_m(engine, b"waitfactor", u32::from(number), mem),
            6 => {
                status = h.set_num_m(engine, b"readmode", u32::from(number), mem);
                if status >= 0x80000000 {
                    return finish_param_list_m(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, h,
                    );
                }
            }
            13 => {
                if number == 3 || number == 12 {
                    mode = number as u8;
                } else {
                    status = 0x8000200f;
                    return finish_param_list_m(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, h,
                    );
                }
            }
            14 => {
                if number == 1 || number == 2 {
                    input = number as u8;
                } else {
                    status = 0x8000200f;
                    return finish_param_list_m(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, h,
                    );
                }
            }
            7 => {
                copy_utf16_as_ascii(mem, Some(text), voice, 64);
                voice_id = 0;
                changed = 1;
                if strlen_bytes(&mem[voice as usize..]) == 3 {
                    let (rc, res, id) =
                        {let key: [u8;64]=mem[voice as usize..voice as usize+64].try_into().unwrap(); h.resolve_m(manager,&key, mem)};
                    if rc < 0x80000000 {
                        resolved = res;
                        voice_id = id;
                        copy_str(mem, voice, resolved);
                        h.release_m(mem_word(mem, manager + 4), resolved, mem);
                        resolved = 0;
                    }
                }
            }
            15 => {
                voice_id = number;
                let (rc, res, id) = h.resolve_m(manager, &zero_key, mem);
                status = rc;
                if status >= 0x80000000 {
                    return finish_param_list_m(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, h,
                    );
                }
                resolved = res;
                voice_id = id;
                copy_str(mem, voice, resolved);
                h.release_m(mem_word(mem, manager + 4), resolved, mem);
                resolved = 0;
                changed = 1;
                continue;
            }
            8 => {
                copy_utf16_as_ascii(mem, Some(text), voice + 0x40, 32);
                changed = 1;
            }
            9 => {
                copy_utf16_as_ascii(mem, Some(text), voice + 0x82, 16);
            }
            19 => {
                copy_utf16_as_ascii(mem, Some(text), voice + 0x62, 32);
                changed = 1;
                variant_set = 1;
            }
            16 => {
                if number > 1024 {
                    status = 0x8000200f;
                    return finish_param_list_m(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, h,
                    );
                }
                status = h.set_num_m(engine, b"promptlexsize", u32::from(number), mem);
            }
            17 => status = h.set_num_m(engine, b"promptlexmsgs", u32::from(number), mem),
            18 => {
                if (i32::from(number).wrapping_sub(25) as u32) >= 0x9ac {
                    status = 0x8000200f;
                    return finish_param_list_m(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, h,
                    );
                }
                status = h.set_num_m(engine, b"maxinputlength", u32::from(number), mem);
            }
            11 => {
                if number == 1 {
                    status = h.set_str_m(engine, b"inputmimetype", b"text/plain;charset=utf-16", mem);
                } else if number == 2 {
                    status = h.set_str_m(engine, b"inputmimetype", b"text/plain;charset=utf-8", mem);
                } else {
                    status = 0x8000200f;
                    return finish_param_list_m(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, h,
                    );
                }
            }
            20 => match number {
                0 => status = h.set_str_m(engine, b"lidscope", b"none", mem),
                1 => status = h.set_str_m(engine, b"lidscope", b"user-defined", mem),
                2 => status = h.set_str_m(engine, b"lidscope", b"message", mem),
                _ => {
                    status = 0x8000200f;
                    return finish_param_list_m(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, h,
                    );
                }
            },
            21 => {
                if number == 0 {
                    status = h.set_str_m(engine, b"lidvoiceswitch", b"no", mem);
                } else if number == 1 {
                    status = h.set_str_m(engine, b"lidvoiceswitch", b"yes", mem);
                } else {
                    status = 0x8000200f;
                    return finish_param_list_m(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, h,
                    );
                }
            }
            22 => {
                copy_utf16_as_ascii(mem, Some(text), buffer, 128);
                let end = nul_len(&mem[buffer as usize..buffer as usize + 128]);
                let text: [u8;128]=mem[buffer as usize..buffer as usize+128].try_into().unwrap();
                status = h.set_str_m(engine,b"extraesclang",&text[..end], mem);
            }
            23 => {
                copy_utf16_as_ascii(mem, Some(text), buffer, 128);
                let end = nul_len(&mem[buffer as usize..buffer as usize + 128]);
                let text: [u8;128]=mem[buffer as usize..buffer as usize+128].try_into().unwrap();
                status = h.set_str_m(engine,b"extraesctn",&text[..end], mem);
            }
            _ => {
                status = 0x80002007;
                return finish_param_list_m(
                    mem, object, voice, mode, input, status, changed, acquired, locked, h,
                );
            }
        }
        if status >= 0x80000000 {
            return finish_param_list_m(
                mem, object, voice, mode, input, status, changed, acquired, locked, h,
            );
        }
    }
    if changed != 0 {
        status = configure_voice_m(
            mem, object, 0, voice, converted, freq_buf, lit_pcm, lit_pipeline, lit_slash,
            lit_format_pcm, lit_phon, pair, default_ctx, h,
        );
        if status >= 0x80000000 {
            if mem[voice as usize] != 0 || mem[voice as usize + 0x40] != 0 {
                if frequency_set == 0 || variant_set == 0 {
                    if frequency_set == 0 {
                        mem[voice as usize + 0x60..voice as usize + 0x62]
                            .copy_from_slice(&0u16.to_le_bytes());
                    }
                    if variant_set == 0 {
                        mem[voice as usize + 0x62] = 0;
                    }
                    configure_voice_m(
                        mem, object, 0, voice, converted, freq_buf, lit_pcm, lit_pipeline,
                        lit_slash, lit_format_pcm, lit_phon, pair, default_ctx, h,
                    );
                }
            }
            return finish_param_list_m(
                mem, object, voice, mode, input, status, changed, acquired, locked, h,
            );
        }
    } else if status >= 0x80000000 {
        return finish_param_list_m(
            mem, object, voice, mode, input, status, changed, acquired, locked, h,
        );
    }
    let mode_dual = (mode & 8 != 0) && (mode & 4 != 0);
    let object_dual = (mem[object as usize + 0x34] & 8 != 0) && (mem[object as usize + 0x34] & 4 != 0);
    if mode_dual != object_dual {
        if mode_dual {
            changed = 1;
        } else {
            status = h.set_voice_m(
                mem_word(mem, object + 0x2c),
                mem_word(mem, object + 0x30),
                None,
                0, mem);
            if status >= 0x80000000 {
                return finish_param_list_m(
                    mem, object, voice, mode, input, status, changed, acquired, locked, h,
                );
            }
        }
    }
    if changed == 1 && mode_dual {
        let end = nul_len(&mem[voice as usize + 0x92..voice as usize + 0x92 + 128]);
        let spec = voice as usize + 0x92;
        let text: [u8;128]=mem[spec..spec+128].try_into().unwrap();
        status = h.set_voice_m(
            mem_word(mem, object + 0x2c),
            mem_word(mem, object + 0x30),
            Some(&text[..end]),
            0, mem);
        if status >= 0x80000000 {
            return finish_param_list_m(
                mem, object, voice, mode, input, status, changed, acquired, locked, h,
            );
        }
    }
    let _ = resolved;
    let _ = voice_id;
    finish_param_list_m(
        mem, object, voice, mode, input, status, changed | 2, acquired, locked, h,
    )
}

pub struct LegacyParamHooks<'a,L,C,Q,N,SN,SS,R,F,V> {
 pub lock:&'a mut L,pub configure:&'a mut C,pub query:&'a mut Q,pub notify:&'a mut N,
 pub set_num:&'a mut SN,pub set_str:&'a mut SS,pub resolve:&'a mut R,pub release:&'a mut F,pub set_voice:&'a mut V,
}
impl<L,C,Q,N,SN,SS,R,F,V> ParamHooksM for LegacyParamHooks<'_,L,C,Q,N,SN,SS,R,F,V>
where L:crate::frontend::StateLock,
 C:FnMut(u32,u32,u32,u32)->(u32,[u8;0x128]),Q:FnMut(u32,u32)->(u32,u32),N:FnMut(u32,u32),
 SN:FnMut(u32,&[u8],u32)->u32,SS:FnMut(u32,&[u8],&[u8])->u32,R:FnMut(u32,&[u8])->(u32,u32,u16),F:FnMut(u32,u32),V:FnMut(u32,u32,Option<&[u8]>,u32)->u32 {
 fn state_acquire_m(&mut self,_:&mut[u8],_:u32)->u32{self.lock.acquire()}
 fn state_release_m(&mut self,_:&mut[u8],_:u32)->u32{self.lock.release()}
 fn configure_m(&mut self,a:u32,b:u32,r:u32,s:u32,_:u32,_:&mut[u8])->(u32,[u8;0x128]){(self.configure)(a,b,r,s)}
 fn query_m(&mut self,r:u32,s:u32,_:&mut[u8])->(u32,u32){(self.query)(r,s)}
 fn notify_m(&mut self,r:u32,v:u32,_:&mut[u8]){(self.notify)(r,v)}
 fn set_num_m(&mut self,p:u32,k:&[u8],v:u32,_:&mut[u8])->u32{(self.set_num)(p,k,v)}
 fn set_str_m(&mut self,p:u32,k:&[u8],v:&[u8],_:&mut[u8])->u32{(self.set_str)(p,k,v)}
 fn resolve_m(&mut self,p:u32,k:&[u8],_:&mut[u8])->(u32,u32,u16){(self.resolve)(p,k)}
 fn release_m(&mut self,h:u32,p:u32,_:&mut[u8]){(self.release)(h,p)}
 fn set_voice_m(&mut self,a:u32,b:u32,s:Option<&[u8]>,f:u32,_:&mut[u8])->u32{(self.set_voice)(a,b,s,f)}
}

pub fn store_sized_m(m:&mut Memory<'_>,h:&mut impl Calls,slot:u32,src:u32,n:u32,heap:u32)->u32 {
 if slot==0{return 0x8b702007;}
 if n==0 {
  if m.word(slot)!=0 && m.word(slot+4)!=0{h.call(m,0x196428,&[heap,m.word(slot)]);}
  m.put_word(slot+4,0);m.put_word(slot,src);
 }else{
  let size=m.word(slot+4);
  if size==0{let p=h.call(m,0x196394,&[heap,n]);m.put_word(slot,p);if p==0{return 0x8b70200a;}}
  else if size<n{let p=h.call(m,0x1963e0,&[heap,m.word(slot),n]);if p==0{return 0x8b70200a;}m.put_word(slot,p);}
  m.boot_copy_forward(m.word(slot),src,n);m.put_word(slot+4,n);
 }0
}

pub fn element_callbacks_m(m:&mut Memory<'_>,h:&mut impl Calls,p:u32,key:u32,value:u32,data:u32,s:u32)->u32{
 if p==0 || key==0 || value==0 || data==0{return 0;}
 let saved=m.word(s);
 let rc=(||{
  for pass in [1,2] {
   let list=if pass==1{m.word(value+8)}else{m.word(p+12)};
   if list!=0 {
    let count=h.call(m,0x1970a0,&[list]);
    for i in 0..count{
     if h.call(m,0x1970ac,&[list,i,s])==0{return 0x8b702100;}
     let e=m.word(s);let rc=h.call(m,m.word(e+8),&[m.word(e),key,data]);
     if rc>=0x80000000{return rc;}
    }
   }
  }0
 })();m.put_word(s,saved);rc
}
fn before_live(m:&mut Memory<'_>,h:&mut impl Calls,p:u32,key:u32,value:u32,data:u32,s:u32)->u32{
 if p==0 || key==0 || value==0 || data==0{return 0;}
 let mut rc=0;m.put_word(s,0);put_byte(m,s+4,1);
 for pass in [1,2]{
  let list=if pass==1{m.word(value+8)}else{m.word(p+12)};
  if list!=0{
   let count=h.call(m,0x1970a0,&[list]);let mut i=0;
   while rc<0x80000000 && i<count{
    if h.call(m,0x1970ac,&[list,i,s])==0{return 0x8b702100;}
    let e=m.word(s);let f=m.word(e+4);
    if f!=0{rc=h.call(m,f,&[m.word(e),key,data,s+4]);if rc<0x80000000 && m.bytes[m.offset(s+4)]==0{rc=0x8b70200f;break;}}
    i+=1;
   }
  }
 }rc
}

pub fn update_attr_m(m:&mut Memory<'_>,h:&mut impl Calls,o:u32,key:u32,data:u32,size:u32,update:u32,s:u32)->u32{
 let saved=core::array::from_fn::<_,5,_>(|i|m.word(s+i as u32*4));
 m.put_word(s,data);m.put_word(s+4,size);m.put_word(s+8,0);
 let mut rc=0;let mut i=0;
 loop{
  if h.call(m,0x1970ac,&[m.word(o+16),i,s+12])==0{break;}
  let ent=m.word(s+12);rc=h.call(m,0x19668c,&[m.word(ent+4),key,s]);
  if rc&0x1fff==2{
   if update==0{rc=0;}else{
    rc=h.call(m,0x1966ac,&[m.word(ent+4),key,s+16]);
    if rc>=0x80000000{rc=(rc&0x1fff)|0x8b702000;}
    else{rc=h.call(m,0x176054,&[m.word(s+16),data,size,m.word(m.word(ent))]);}
   }
  }
  i+=1;if rc>=0x80000000{break;}
 }
 for (i,v) in saved.into_iter().enumerate(){m.put_word(s+i as u32*4,v);}rc
}

pub fn set_parameter_m(m:&mut Memory<'_>,h:&mut impl Calls,p:u32,key:u32,data:u32,size:u32,update:u32,s:u32)->u32{
 if p==0 || key==0 || data==0{return 0x8b702007;}
 let mut rc=h.call(m,0x176f2c,&[m.word(p+4)]);if rc>=0x80000000{return rc;}
 let saved=core::array::from_fn::<_,8,_>(|i|m.word(s+i as u32*4));m.put_word(s,0);
 let lookup=h.call(m,0x1966ac,&[m.word(p+8),key,s]);let value=m.word(s);
 let old_size=if lookup<0x80000000 && value!=0{m.word(value+4)}else{0};
 let old_data=if old_size!=0{m.word(value)}else{0};
 let changed=lookup>=0x80000000 || old_size!=size || {
  let a=m.offset(old_data);let b=m.offset(data);crate::frontend::compare_bytes(&m.bytes[a..a+size as usize],&m.bytes[b..b+size as usize],size)!=0
 };
 if changed{
  let incoming=if lookup<0x80000000{value}else{m.put_word(s+4,data);m.put_word(s+8,size);m.put_word(s+12,0);s+4};
  rc=before_live(m,h,p,key,incoming,data,s+16);
  if rc<0x80000000{
   rc=h.call(m,0x174b38,&[p,key,if update!=0{data}else{old_data},if update!=0{size}else{old_size},update]);
   if rc<0x80000000{
    rc=if lookup<0x80000000{h.call(m,0x176054,&[value,data,size,m.word(p)])}else{h.call(m,0x19668c,&[m.word(p+8),key,incoming])};
    if rc>=0x80000000 && lookup>=0x80000000{rc=(rc&0x1fff)|0x8b702000;}
    if rc<0x80000000{rc=h.call(m,0x176104,&[p,key,incoming,data]);if rc>=0x80000000 && lookup>=0x80000000{rc=(rc&0x1fff)|0x8b702000;}}
   }
  }
 }
 for (i,v) in saved.into_iter().enumerate(){m.put_word(s+i as u32*4,v);}
 let unlock=h.call(m,0x176f5c,&[m.word(p+4)]);if unlock>=0x80000000 && rc<0x80000000{unlock}else{rc}
}

use crate::frontend::{parse_uint,compare_str};
pub trait SetParamHooksM {
 fn resolve_ctx_m(&mut self,mem:&mut[u8],a:u32,b:u32,out:&mut u32)->u32;
 fn log_change_m(&mut self,mem:&mut[u8],log:u32);
 fn apply_esc_m(&mut self,mem:&mut[u8],ctx:u32,p:u32,value:u32)->u32;
}
impl<T:crate::frontend::SetParamHooks> SetParamHooksM for T{
 fn resolve_ctx_m(&mut self,_:&mut[u8],a:u32,b:u32,out:&mut u32)->u32{self.resolve_ctx(a,b,out)}
 fn log_change_m(&mut self,_:&mut[u8],log:u32){self.log_change(log)}
 fn apply_esc_m(&mut self,_:&mut[u8],ctx:u32,p:u32,v:u32)->u32{self.apply_esc(ctx,p,v)}
}
pub fn set_synth_param_m<H: SetParamHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    p: Option<u32>,
    name: Option<u32>,
    value: Option<u32>,
) -> u32 {
    let mut r = 0u32;
    let (Some(p), Some(v)) = (p, value) else {
        return r;
    };
    if mem[v as usize] == 0 {
        return r;
    }
    let mut ctx = 0u32;
    if hooks.resolve_ctx_m(mem, mem_word(mem, p), mem_word(mem, p.wrapping_add(4)), &mut ctx)
        >= 0x80000000
    {
        return r;
    }
    hooks.log_change_m(mem, mem_word(mem, ctx.wrapping_add(0x10)));
    let Some(name) = name else {
        return r;
    };
    if compare_str(&mem[name as usize..], b"infiniteloopdetectlimit\0") == 0 {
        mem_store(mem, p.wrapping_add(0x38), parse_uint(mem, Some(v)) as u32);
    } else if compare_str(&mem[name as usize..], b"markermode\0") == 0 {
        mem_store(mem, p.wrapping_add(0x34), parse_uint(mem, Some(v)) as u32);
    } else if compare_str(&mem[name as usize..], b"audiooutputbufsamples\0") == 0 {
        mem_store(mem, p.wrapping_add(0xe8), parse_uint(mem, Some(v)) as u32);
    } else if compare_str(&mem[name as usize..], b"__NUAN_alternative_esc_seq__\0") == 0 {
        r = hooks.apply_esc_m(mem, ctx, p, v);
    }
    r
}

use crate::frontend::{terminated,derive_langgroup,derive_langgroup_candidate,rec_field_matches,candidate_matches,score_candidate,copy_str_or_empty,char_class,compare_folded,compare_folded_bounded,find_byte,find_substr,copy_str_n,step_iter,next_tag_entry};

fn broker_call(mem:&mut [u8],h:&mut impl Calls,f:u32,a:&[u32])->u32 {
    h.call(&mut Memory {base:0,bytes:mem},f,a)
}
fn broker_out(mem:&mut [u8],h:&mut impl Calls,f:u32,a:&[u32],out:Option<&mut u32>,io:u32)->u32 {
    let saved=mem_word(mem,io);let mut args=[0;8];args[..a.len()].copy_from_slice(a);
    if let Some(out)=out {
        mem_store(mem,io,*out);args[a.len()]=io;
        let rc=broker_call(mem,h,f,&args[..a.len()+1]);*out=mem_word(mem,io);
        mem_store(mem,io,saved);rc
    } else {args[a.len()]=0;broker_call(mem,h,f,&args[..a.len()+1])}
}
fn broker_two_out(mem:&mut [u8],h:&mut impl Calls,f:u32,a:&[u32],out:Option<&mut u32>,second:Option<&mut u32>,third_null:bool,io:u32)->u32 {
    let saved=[mem_word(mem,io),mem_word(mem,io+4)];let mut args=[0;8];args[..a.len()].copy_from_slice(a);
    if let Some(v)=out.as_ref() {mem_store(mem,io,**v);args[a.len()]=io;}
    if let Some(v)=second.as_ref() {mem_store(mem,io+4,**v);args[a.len()+1]=io+4;}
    let rc=broker_call(mem,h,f,&args[..a.len()+2+usize::from(third_null)]);
    if let Some(v)=out {*v=mem_word(mem,io);}if let Some(v)=second {*v=mem_word(mem,io+4);}
    mem_store(mem,io,saved[0]);mem_store(mem,io+4,saved[1]);rc
}
fn nz(p:u32)->Option<u32> {(p!=0).then_some(p)}

pub fn copy_heap_string_m(
    mem: &mut [u8],
    heap: Option<u32>,
    name: Option<u32>,
    out: &mut u32,
    io:u32,
    h:&mut impl Calls,
) -> u32 {
    *out = 0;
    let Some(name) = name else {
        return 0x81506000;
    };
    let o = broker_call(mem,h,0x196394,&[(heap).unwrap_or(0),strlen_bytes(&mem[name as usize..]) + 1]);
    *out = o;
    if o == 0 {
        return 0x8150600a;
    }
    copy_str(mem, o, name);
    0
}

pub fn match_wildcard_m(
    mem: &mut [u8],
    heap: Option<u32>,
    pattern: u32,
    key: u32,
    matched: &mut u8,
    io:u32,
    h:&mut impl Calls,
) -> u32 {
    let copy = broker_call(mem,h,0x196394,&[(heap).unwrap_or(0),strlen_bytes(&mem[pattern as usize..]) + 1]);
    if copy == 0 {
        return 0x8150600a;
    }
    *matched = 1;
    let mut cursor = pattern;
    let mut search = key;
    while strlen_bytes(&mem[pattern as usize..]) > cursor.wrapping_sub(pattern) {
        let star = find_byte(mem, cursor, '*' as u32);
        let length = if star != 0 {
            star.wrapping_sub(cursor)
        } else {
            strlen_bytes(&mem[cursor as usize..])
        };
        copy_str_n(mem, copy, cursor, length);
        mem[(copy + length) as usize] = 0;
        let found = find_substr(mem, search, copy);
        if found == 0 {
            *matched = 0;
            break;
        }
        cursor = cursor.wrapping_add(length).wrapping_add(1);
        search = found.wrapping_add(length);
        if length != 0
            && mem[(copy + length - 1) as usize] != b'/'
            && strlen_bytes(&mem[key as usize..]) != search.wrapping_sub(key)
        {
            *matched = 0;
            break;
        }
    }
    {broker_call(mem,h,0x196428,&[(heap).unwrap_or(0),(nz(copy)).unwrap_or(0)]);};
    0
}

pub fn lookup_keyed_value_m(
    mem: &mut [u8],
    p: Option<u32>,
    name: Option<u32>,
    out: &mut u32,
    table: &[u16; 256],
    io:u32,
    h:&mut impl Calls,
) -> u32 {
    *out = 0;
    let (Some(p), Some(name)) = (p, name) else {
        return 0x8150600d;
    };
    let ctx = mem_word(mem, p + 4);
    let result = broker_call(mem,h,0x176f2c,&[mem_word(mem,ctx+28)]);
    if result >= 0x80000000 {
        return result;
    }
    let mut iterator = 0u32;
    let mut result = broker_out(mem,h,0x196794,&[(nz(mem_word(mem, ctx + 16))).unwrap_or(0),name,1],Some(&mut iterator),io);
    if result < 0x80000000 {
        let mut key = 0u32;
        let mut value = 0u32;
        let mut status = step_iter(mem, nz(iterator), Some(&mut key), Some(&mut value));
        if status < 0x80000000 {
            loop {
                let order = compare_folded(table, terminated(mem, name), terminated(mem, key));
                if order == 0 {
                    *out = value;
                    break;
                }
                if order > 0 {
                    break;
                }
                status = step_iter(mem, nz(iterator), Some(&mut key), Some(&mut value));
                if status >= 0x80000000 {
                    break;
                }
            }
        }
        broker_call(mem,h,0x196830,&[(nz(iterator)).unwrap_or(0)]);
    } else if result & 0x1fff == 0x14 {
        result = 0x8150600d;
    }
    let status = broker_call(mem,h,0x176f5c,&[mem_word(mem,ctx+28)]);
    if status >= 0x80000000 {
        if result < 0x80000000 {
            result = status;
        } else {
            return result;
        }
    }
    if result < 0x80000000 && *out == 0 {
        return 0x8150600d;
    }
    result
}

pub fn query_tag_word_m(
    mem: &mut [u8],
    broker: Option<u32>,
    component: Option<u32>,
    key: Option<u32>,
    flags: u32,
    out: Option<&mut u32>,
    table: &[u16; 256],
    io:u32,
    h:&mut impl Calls,
) -> u32 {
    let _ = flags;
    let (Some(broker), Some(component), Some(key), Some(out)) = (broker, component, key, out)
    else {
        return 0x81506007;
    };
    if mem_word(mem, broker + 4) == 0 {
        return 0x81506007;
    }
    *out = 0;
    let mut entry = 0u32;
    let rc = broker_out(mem,h,0x1772b0,&[(Some(broker)).unwrap_or(0),(Some(component)).unwrap_or(0)],Some(&mut entry),io);
    if rc >= 0x80000000 {
        return 0x8150600d;
    }
    let mut value = 0u32;
    let rc = broker_out(mem,h,0x1966ac,&[(nz(mem_word(mem, entry))).unwrap_or(0),key],Some(&mut value),io);
    if rc >= 0x80000000 {
        return 0x81506014;
    }
    if value != 0 {
        *out = mem_word(mem, value);
    }
    0
}

pub fn open_component_iter_m(
    mem: &mut [u8],
    broker: Option<u32>,
    name: Option<u32>,
    out: Option<&mut u32>,
    io:u32,
    h:&mut impl Calls,
) -> u32 {
    let (Some(broker), Some(name), Some(out)) = (broker, name, out) else {
        return 0x81506007;
    };
    let context = mem_word(mem, broker + 4);
    if context == 0 {
        return 0x81506007;
    }
    let runtime = mem_word(mem, context + 8);
    let heap = nz(mem_word(mem, runtime + 4));
    let o = broker_call(mem,h,0x1963b8,&[(heap).unwrap_or(0),1,36]);
    *out = o;
    if o == 0 {
        return 0x8150600a;
    }
    mem_store(mem, o, runtime);
    mem_store(mem, o + 4, broker);
    let mut oname = 0u32;
    let mut rc = copy_heap_string_m(mem,heap,Some(name),&mut oname,io,h);
    mem_store(mem, o + 8, oname);
    if rc < 0x80000000 {
        rc = broker_call(mem,h,0x176f2c,&[mem_word(mem,context+28)]);
        if rc < 0x80000000 {
            let mut it = mem_word(mem, o + 12);
            rc = broker_out(mem,h,0x196794,&[(nz(mem_word(mem, context + 16))).unwrap_or(0),name,0],Some(&mut it),io);
            mem_store(mem, o + 12, it);
            if rc >= 0x80000000 && rc & 0x1fff == 0x14 {
                rc = 0;
            }
        }
        let status = broker_call(mem,h,0x176f5c,&[mem_word(mem,context+28)]);
        if status >= 0x80000000 && rc < 0x80000000 {
            rc = status;
        }
    }
    if rc >= 0x80000000 {
        let obj = *out;
        let ictx = mem_word(mem, obj);
        let key = mem_word(mem, obj + 8);
        if key != 0 {
            {broker_call(mem,h,0x196428,&[(nz(mem_word(mem, ictx + 4))).unwrap_or(0),(nz(key)).unwrap_or(0)]);};
            mem_store(mem, obj + 8, 0);
        }
        if mem_word(mem, obj + 12) != 0 {
            broker_call(mem,h,0x196830,&[(nz(mem_word(mem, obj + 12))).unwrap_or(0)]);
        }
        {broker_call(mem,h,0x196428,&[(nz(mem_word(mem, ictx + 4))).unwrap_or(0),(nz(obj)).unwrap_or(0)]);};
        *out = 0;
    }
    rc
}

pub fn next_component_entry_m(
    mem: &mut [u8],
    iterator: Option<u32>,
    mut key_out: Option<&mut u32>,
    mut attrs_out: Option<&mut u32>,
    table: &[u16; 256],
    attr_keys:[u32;3],
    io:u32,
    h:&mut impl Calls,
) -> u32 {

    let Some(iterator) = iterator else {
        return 0x81506007;
    };
    if let Some(k) = key_out.as_mut() {
        **k = 0;
    }
    if let Some(a) = attrs_out.as_mut() {
        **a = 0;
    }
    let runtime = mem_word(mem, iterator);
    let heap = nz(mem_word(mem, runtime + 4));
    let pattern = mem_word(mem, iterator + 8);
    let star = find_byte(mem, pattern, '*' as u32);
    let prefix = if star != 0 {
        star.wrapping_sub(pattern)
    } else {
        strlen_bytes(&mem[pattern as usize..])
    };
    let mut key = 0u32;
    let mut entry = 0u32;
    let mut rc = step_iter(
        mem,
        nz(mem_word(mem, iterator + 12)),
        Some(&mut key),
        Some(&mut entry),
    );
    while rc < 0x80000000 {
        if compare_folded_bounded(
            table,
            &mem[pattern as usize..],
            &mem[key as usize..],
            prefix as i32,
        ) < 0
        {
            rc = 0x81506017;
            break;
        }
        let mut matched = 0u8;
        rc = match_wildcard_m(mem,heap,pattern,key,&mut matched,io,h);
        if rc >= 0x80000000 {
            break;
        }
        if matched == 1 {
            if let Some(k) = key_out.as_mut() {
                **k = key;
            }
            if attrs_out.is_some() {
                mem_store(mem, iterator + 20, 0);
                mem_store(mem, iterator + 24, 0);
                mem_store(mem, iterator + 28, 0);
                mem_store(mem, iterator + 32, 0);
                mem_store(mem, iterator + 16, key);
                let base = mem_word(mem, entry);
                for (off,attr_key) in [20, 28, 24].into_iter().zip(attr_keys) {
 assert_ne!(attr_key,0,"regional attribute key input");
                    let mut attr = 0u32;
                    rc = broker_out(mem,h,0x1966ac,&[(nz(base)).unwrap_or(0),attr_key],Some(&mut attr),io);
                    if rc >= 0x80000000 {
                        rc = 0x81506014;
                    } else {
                        rc = 0;
                        if attr != 0 {
                            mem_store(mem, iterator + off, mem_word(mem, attr));
                        }
                    }
                    if rc >= 0x80000000 && rc != 0x81506014 {
                        break;
                    }
                }
                if rc >= 0x80000000 && rc != 0x81506014 {
                    break;
                }
                if let Some(a) = attrs_out.as_mut() {
                    **a = iterator + 16;
                }
                rc = 0;
            }
            break;
        }
        rc = step_iter(
            mem,
            nz(mem_word(mem, iterator + 12)),
            Some(&mut key),
            Some(&mut entry),
        );
    }
    rc
}

pub fn open_tag_iter_m(
    mem: &mut [u8],
    broker: Option<u32>,
    component: Option<u32>,
    query: Option<u32>,
    flags: u32,
    out: Option<&mut u32>,
    segment: u32,
    table: &[u16; 256],
    io:u32,
    h:&mut impl Calls,
) -> u32 {
    let _ = flags;
    let (Some(broker), Some(component), Some(query), Some(out)) = (broker, component, query, out)
    else {
        return 0x81506007;
    };
    if mem[component as usize] == 0 || mem[query as usize] == 0 {
        return 0x81506007;
    }
    let context = mem_word(mem, broker + 4);
    let runtime = mem_word(mem, context + 8);
    *out = 0;
    let mut entry = 0u32;
    let mut rc = broker_out(mem,h,0x1772b0,&[(Some(broker)).unwrap_or(0),(Some(component)).unwrap_or(0)],Some(&mut entry),io);
    let mut database = 0u32;
    if (if rc < 0x80000000 { entry } else { 0 }) == 0 && rc & 0x1fff == 0x0d {
        rc = 0x8150600d;
    } else if rc < 0x80000000 {
        database = mem_word(mem, entry);
    }
    if rc >= 0x80000000 {
        return failed_tag_iter_m(mem,&mut *out,rc,io,h);
    }
    let mut cursor = query;
    let mut node = 0u32;
    loop {
        let slash = find_byte(mem, cursor, '/' as u32);
        if slash == 0 && compare_str(terminated(mem, cursor), b"*\0") == 0 {
            break;
        }
        if node != 0 {
            database = mem_word(mem, node + 8);
        }
        if database == 0 {
            rc = 0x8150600d;
            return failed_tag_iter_m(mem,&mut *out,rc,io,h);
        }
        if slash != 0 && slash.wrapping_sub(cursor) < 128 {
            let length = slash.wrapping_sub(cursor);
            copy_str_n(mem, segment, cursor, length);
            mem[(segment + length) as usize] = 0;
            let mut found = 0u32;
            rc = broker_out(mem,h,0x1966ac,&[(nz(database)).unwrap_or(0),segment],Some(&mut found),io);
            node = found;
            if rc >= 0x80000000 {
                rc = 0x8150600d;
                return failed_tag_iter_m(mem,&mut *out,rc,io,h);
            }
            rc = 0;
        } else {
            let mut found = 0u32;
            rc = broker_out(mem,h,0x1966ac,&[(nz(database)).unwrap_or(0),cursor],Some(&mut found),io);
            node = found;
            if rc >= 0x80000000 {
                rc = 0x8150600d;
                return failed_tag_iter_m(mem,&mut *out,rc,io,h);
            }
            rc = 0;
            if slash == 0 {
                break;
            }
        }
        cursor = slash.wrapping_add(1);
    }
    if rc >= 0x80000000 {
        return failed_tag_iter_m(mem,&mut *out,rc,io,h);
    }
    let heap = nz(mem_word(mem, runtime + 4));
    let o = broker_call(mem,h,0x1963b8,&[(heap).unwrap_or(0),1,12]);
    *out = o;
    if o == 0 {
        return 0x8150600a;
    }
    if compare_str(terminated(mem, cursor), b"*\0") == 0 {
        if node != 0 {
            database = mem_word(mem, node + 8);
        }
        let mut it = mem_word(mem, o + 8);
        rc = broker_out(mem,h,0x196794,&[(nz(database)).unwrap_or(0),0,1],Some(&mut it),io);
        mem_store(mem, o + 8, it);
    } else {
        let mut it = mem_word(mem, o + 8);
        rc = broker_out(mem,h,0x196794,&[(nz(database)).unwrap_or(0),cursor,1],Some(&mut it),io);
        mem_store(mem, o + 8, it);
        if rc >= 0x80000000 {
            return failed_tag_iter_m(mem,&mut *out,rc,io,h);
        }
        let mut oname = mem_word(mem, o + 4);
        rc = copy_heap_string_m(mem,heap,Some(cursor),&mut oname,io,h);
        mem_store(mem, o + 4, oname);
    }
    mem_store(mem, o, runtime);
    if rc < 0x80000000 {
        return rc;
    }
    failed_tag_iter_m(mem,&mut *out,rc,io,h)
}

pub fn failed_tag_iter_m(
    mem: &mut [u8],
    out: &mut u32,
    rc: u32,
    io:u32,
    h:&mut impl Calls,
) -> u32 {
    let iterator = *out;
    if iterator != 0 {
        let owner = mem_word(mem, iterator);
        if mem_word(mem, iterator + 4) != 0 {
            {broker_call(mem,h,0x196428,&[(nz(mem_word(mem, owner + 4))).unwrap_or(0),(nz(mem_word(mem, iterator + 4))).unwrap_or(0)]);};
        }
        if mem_word(mem, iterator + 8) != 0 {
            broker_call(mem,h,0x196830,&[(nz(mem_word(mem, iterator + 8))).unwrap_or(0)]);
        }
        {broker_call(mem,h,0x196428,&[(nz(mem_word(mem, owner + 4))).unwrap_or(0),(nz(iterator)).unwrap_or(0)]);};
    }
    *out = 0;
    rc
}

pub fn close_tag_iter_m(
    mem: &mut [u8],
    p: Option<u32>,
    io:u32,
    h:&mut impl Calls,
) -> u32 {
    let Some(p) = p else {
        return 0x81506007;
    };
    let ctx = mem_word(mem, p);
    let heap = nz(mem_word(mem, ctx + 4));
    let entry = mem_word(mem, p + 4);
    if entry != 0 {
        {broker_call(mem,h,0x196428,&[(heap).unwrap_or(0),(Some(entry)).unwrap_or(0)]);};
    }
    let iterator = mem_word(mem, p + 8);
    if iterator != 0 {
        broker_call(mem,h,0x196830,&[(Some(iterator)).unwrap_or(0)]);
    }
    {broker_call(mem,h,0x196428,&[(heap).unwrap_or(0),(Some(p)).unwrap_or(0)]);};
    0
}

pub fn close_component_iter_m(
    mem: &mut [u8],
    p: Option<u32>,
    io:u32,
    h:&mut impl Calls,
) -> u32 {
    let Some(p) = p else {
        return 0x81506007;
    };
    let ctx = mem_word(mem, p);
    let heap = nz(mem_word(mem, ctx + 4));
    if mem_word(mem, p + 8) != 0 {
        {broker_call(mem,h,0x196428,&[(heap).unwrap_or(0),(Some(mem_word(mem, p + 8))).unwrap_or(0)]);};
        mem_store(mem, p + 8, 0);
    }
    let iterator = mem_word(mem, p + 12);
    if iterator != 0 {
        broker_call(mem,h,0x196830,&[(Some(iterator)).unwrap_or(0)]);
    }
    {broker_call(mem,h,0x196428,&[(heap).unwrap_or(0),(Some(p)).unwrap_or(0)]);};
    0
}

pub fn match_voice_m(
    mem: &mut [u8],
    hdr: u32,
    rec: u32,
    flags: u32,
    dict_keys: &[u32; 9],
    pipeline: u32,
    priority: u32,
    params_query: u32,
    es_us: u32,
    es_mx: u32,
    segment: u32,
    scratch: u32,
    table: &[u16; 256],
    io:u32,
    h:&mut impl Calls,
) -> u32 {
    let _ = flags;
    let iter = mem_word(mem, hdr + 12);
    let dict = mem_word(mem, hdr + 20);
    let mut qv = [0u32; 11];
    let mut cur = [0u32; 11];
    let mut best = [0u32; 11];
    if strlen_bytes(&mem[rec as usize..]) == 5
        && char_class(table, mem[rec as usize] as u32) != 0
        && char_class(table, mem[rec as usize + 1] as u32) != 0
        && mem[rec as usize + 2] == b'-'
        && char_class(table, mem[rec as usize + 3] as u32) != 0
        && char_class(table, mem[rec as usize + 4] as u32) != 0
    {
        copy_str(mem, rec + 0x48, rec);
        fill_bytes(mem, rec, 0, 0x40);
    }
    for (index, (slot, key)) in qv.iter_mut().take(9).zip(dict_keys.iter()).enumerate() {
        let rc = if index == 7 {
            let rc = broker_call(mem,h,0x175944,&[(nz(dict)).unwrap_or(0),*key,scratch]);
            *slot = mem_word(mem, scratch);
            rc
        } else {
            broker_out(mem,h,0x175a1c,&[(nz(dict)).unwrap_or(0),*key],Some(slot),io)
        };
        if dict != 0 && rc >= 0x80000000 && rc.wrapping_shl(19) != 0xa00000 {
            return rc;
        }
    }
    derive_langgroup(mem, table, &mut qv, scratch + 4);
    if rec_field_matches(mem, table, rec, &qv, es_us, es_mx) {
        best = qv;
    } else {
        let mut params = 0u32;
        let rc = broker_out(mem,h,0x1789b0,&[(nz(iter)).unwrap_or(0),(Some(pipeline)).unwrap_or(0)],Some(&mut params),io);
        if rc >= 0x80000000 {
            return rc;
        }
        let mut best_flag = 0u32;
        let mut best_score = 0u32;
        let mut best_pri = 0u32;
        let mut key = 0u32;
        while broker_two_out(mem,h,0x178c24,&[(nz(params)).unwrap_or(0)],Some(&mut key),None,false,io) < 0x80000000
        {
            cur = [0u32; 11];
            let mut it = 0u32;
            let rc = broker_out(mem,h,0x17a3b8,&[(nz(iter)).unwrap_or(0),(nz(key)).unwrap_or(0),(Some(params_query)).unwrap_or(0),1],Some(&mut it),io);
            if rc >= 0x80000000 {
                continue;
            }
            let mut name = 0u32;
            let mut val = 0u32;
            loop {
                if next_tag_entry(mem, nz(it), Some(&mut name), Some(&mut val), None)
                    >= 0x80000000
                {
                    break;
                }
                if name == 0 {
                    continue;
                }
                let tag = terminated(mem, name);
                if compare_str(tag, b"language\0") == 0 {
                    cur[0] = val;
                } else if compare_str(tag, b"langcode\0") == 0 {
                    cur[1] = val;
                } else if compare_str(tag, b"langietf\0") == 0 {
                    cur[2] = val;
                } else if compare_str(tag, b"langgroup\0") == 0 {
                    cur[3] = val;
                } else if compare_str(tag, b"voice\0") == 0 {
                    cur[4] = val;
                } else if compare_str(tag, b"gender\0") == 0 {
                    cur[5] = val;
                } else if compare_str(tag, b"age\0") == 0 {
                    cur[6] = val;
                } else if compare_str(tag, b"frequencyhz\0") == 0 {
                    cur[7] = parse_uint(mem, nz(val)) as u32;
                } else if compare_str(tag, b"voicemodel\0") == 0 {
                    cur[8] = val;
                }
            }
            broker_call(mem,h,0x17a2ec,&[(nz(it)).unwrap_or(0)]);
            derive_langgroup_candidate(mem, table, &mut cur, scratch + 8);
            if !candidate_matches(mem, table, rec, &cur, es_us, es_mx) {
                continue;
            }
            let score = score_candidate(mem, table, &qv, &cur);
            if best_flag != 0 && best_score > score {
                continue;
            }
            let mut bval = 0u32;
            let mut pri = 0u32;
            let rc = broker_out(mem,h,0x1785c8,&[(nz(iter)).unwrap_or(0),(nz(key)).unwrap_or(0),(Some(priority)).unwrap_or(0),1],Some(&mut bval),io);
            if rc < 0x80000000 && bval != 0 {
                pri = parse_uint(mem, nz(bval)) as u32 & 0xffff;
                broker_call(mem,h,0x178818,&[(nz(iter)).unwrap_or(0),(nz(bval)).unwrap_or(0)]);
            }
            if best_flag == 0 || score > best_score || (pri as u16) > (best_pri as u16) {
                best = cur;
                best_flag = 1;
                best_score = score;
                best_pri = pri;
            }
        }
        broker_call(mem,h,0x1788c0,&[(nz(params)).unwrap_or(0)]);
        if best_flag == 0 {
            return 0x8950200d;
        }
    }
    copy_str_or_empty(mem, rec, best[0]);
    copy_str_or_empty(mem, rec + 0x40, best[1]);
    copy_str_or_empty(mem, rec + 0x48, best[2]);
    copy_str_or_empty(mem, rec + 0x120, best[3]);
    copy_str_or_empty(mem, rec + 0x88, best[4]);
    copy_str_or_empty(mem, rec + 0xa8, best[5]);
    copy_str_or_empty(mem, rec + 0x114, best[6]);
    mem_store(mem, rec + 0xb8, best[7]);
    copy_str_or_empty(mem, rec + 0xbc, best[8]);
    0
}

pub trait VoiceQueryHooksM {
 fn log_begin_m(&mut self,logger:u32,mem:&mut[u8]);
 fn log_end_m(&mut self,logger:u32,rc:u32,mem:&mut[u8]);
 fn match_voice_m(&mut self,hdr:&[u8;32],vrec:&mut[u8;296],flags:u32,mem:&mut[u8])->u32;
}
impl<T:crate::frontend::VoiceQueryHooks+?Sized> VoiceQueryHooksM for T {
 fn log_begin_m(&mut self,logger:u32,_mem:&mut[u8]) {self.log_begin(logger)}
 fn log_end_m(&mut self,logger:u32,rc:u32,_mem:&mut[u8]) {self.log_end(logger,rc)}
 fn match_voice_m(&mut self,hdr:&[u8;32],vrec:&mut[u8;296],flags:u32,_mem:&mut[u8])->u32 {self.match_voice(hdr,vrec,flags)}
}

pub fn find_voice_m<H: VoiceQueryHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
    a1: u32,
    name: Option<u32>,
    lang: u32,
    rec: Option<u32>,
    flags: u32,
) -> u32 {
    if a1 != 0x15c || base.is_none() {
        return 0x89502008;
    }
    let base = base.unwrap();
    if rec.is_none() {
        return 0x89502007;
    }
    let rec = rec.unwrap();
    let inner = mem_word(mem, base.wrapping_add(8));
    let mut hdr = [0u8; 32];
    hdr.copy_from_slice(&mem[inner as usize..inner as usize + 32]);
    if let Some(n) = name {
        hdr[12..16].copy_from_slice(&n.to_le_bytes());
    }
    if lang != 0 {
        hdr[20..24].copy_from_slice(&lang.to_le_bytes());
    }
    let mut vrec = [0u8; 296];
    vrec.copy_from_slice(&mem[rec as usize..rec as usize + 296]);
    let logger = u32::from_le_bytes(hdr[16..20].try_into().unwrap());
    hooks.log_begin_m(logger,mem);
    let rc = hooks.match_voice_m(&hdr, &mut vrec, flags,mem);
    if rc < 0x80000000 {
        if vrec[0xf4] == 0 {
            vrec[0xf4..0xf9].copy_from_slice(b"text\0");
        }
        if vrec[0x104] == 0 {
            vrec[0x104..0x108].copy_from_slice(b"pcm\0");
        }
        mem[rec as usize..rec as usize + 296].copy_from_slice(&vrec);
    }
    hooks.log_end_m(logger, rc,mem);
    rc
}

struct ComponentCalls<'a,H>(&'a mut H);
impl<H:Calls> crate::component_live_b::Host for ComponentCalls<'_,H> {
 fn call_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32 {self.0.call(m,f,a)}
}

pub fn alloc_iterator_m(m:&mut Memory<'_>,p:u32,key:u32,ordered:u32,out:u32,h:&mut impl Calls)->u32 {
 crate::component_live_b::alloc_handle_for_key_live(m,p,key,ordered,out,&mut ComponentCalls(h))
}

use crate::frontend::{PIPE_KIND_MAP,pipe_cmp};

pub trait PipeHooksM {
 fn check_open_m(&mut self, obj: u32, b: u32, mem:&mut[u8]) -> u32;
 fn resolve_ctx_m(&mut self, a: u32, b: u32, mem:&mut[u8]) -> (u32, u32);
 fn set_textanalysis_m(&mut self, params: u32, on: bool, mem:&mut[u8]) ;
 fn lock_m(&mut self, mutex: u32, mem:&mut[u8]) -> u32;
 fn unlock_m(&mut self, mutex: u32, mem:&mut[u8]) -> u32;
 fn query_pending_m(&mut self, params: u32, mem:&mut[u8]) -> (u32, u32);
 fn trace_pipe_m(
        &mut self,
        ha: u32,
        hb: u32,
        name: u32,
        cs: u32,
        stream: u32, mem:&mut[u8]) -> (u32, u32, u8);
 fn query_param_m(&mut self, params: u32, key: &[u8], mem:&mut[u8]) -> (u32, u32);
 fn start_object_m(&mut self, ctx: u32, obj: u32, mem:&mut[u8]) -> u32;
 fn lic_count_m(&mut self, feat: u32, mem:&mut[u8]) -> u32;
 fn lic_feat_m(&mut self, feat: u32, mem:&mut[u8]) -> u32;
 fn release_token_m(&mut self, token: u32, mem:&mut[u8]) ;
 fn notify_lic_m(&mut self, ctx: u32, channel: u32, kind: u32, mode: u32, mem:&mut[u8]) ;
 fn release_pipe_m(&mut self, pipe: u32, mem:&mut[u8]) -> u32;
 fn rollback_pending_m(&mut self, params: u32, pending: u32, mem:&mut[u8]) -> u32;
 fn release_pending_m(&mut self, params: u32, pending: u32, mem:&mut[u8]) -> u32;
 fn reactivate_m(&mut self, cs: u32, mem:&mut[u8]) ;
 fn license_kind_m(&mut self,ctx:u32,obj:u32,trace:u32,mem:&mut[u8])->u32;
}

impl<T:crate::frontend::PipeHooks> PipeHooksM for T {
 fn check_open_m(&mut self, obj: u32, b: u32, mem:&mut[u8]) -> u32 { let _=&mem;crate::frontend::PipeHooks::check_open(self,obj,b) }
 fn resolve_ctx_m(&mut self, a: u32, b: u32, mem:&mut[u8]) -> (u32, u32) { let _=&mem;crate::frontend::PipeHooks::resolve_ctx(self,a,b) }
 fn set_textanalysis_m(&mut self, params: u32, on: bool, mem:&mut[u8])  { let _=&mem;crate::frontend::PipeHooks::set_textanalysis(self,params,on) }
 fn lock_m(&mut self, mutex: u32, mem:&mut[u8]) -> u32 { let _=&mem;crate::frontend::PipeHooks::lock(self,mutex) }
 fn unlock_m(&mut self, mutex: u32, mem:&mut[u8]) -> u32 { let _=&mem;crate::frontend::PipeHooks::unlock(self,mutex) }
 fn query_pending_m(&mut self, params: u32, mem:&mut[u8]) -> (u32, u32) { let _=&mem;crate::frontend::PipeHooks::query_pending(self,params) }
 fn trace_pipe_m(
        &mut self,
        ha: u32,
        hb: u32,
        name: u32,
        cs: u32,
        stream: u32, mem:&mut[u8]) -> (u32, u32, u8) { let _=&mem;crate::frontend::PipeHooks::trace_pipe(self,ha,hb,crate::frontend::terminated(mem,name),cs,stream) }
 fn query_param_m(&mut self, params: u32, key: &[u8], mem:&mut[u8]) -> (u32, u32) { let _=&mem;crate::frontend::PipeHooks::query_param(self,params,key) }
 fn start_object_m(&mut self, ctx: u32, obj: u32, mem:&mut[u8]) -> u32 { let _=&mem;crate::frontend::PipeHooks::start_object(self,ctx,obj) }
 fn lic_count_m(&mut self, feat: u32, mem:&mut[u8]) -> u32 { let _=&mem;crate::frontend::PipeHooks::lic_count(self,feat) }
 fn lic_feat_m(&mut self, feat: u32, mem:&mut[u8]) -> u32 { let _=&mem;crate::frontend::PipeHooks::lic_feat(self,feat) }
 fn release_token_m(&mut self, token: u32, mem:&mut[u8])  { let _=&mem;crate::frontend::PipeHooks::release_token(self,token) }
 fn notify_lic_m(&mut self, ctx: u32, channel: u32, kind: u32, mode: u32, mem:&mut[u8])  { let _=&mem;crate::frontend::PipeHooks::notify_lic(self,ctx,channel,kind,mode) }
 fn release_pipe_m(&mut self, pipe: u32, mem:&mut[u8]) -> u32 { let _=&mem;crate::frontend::PipeHooks::release_pipe(self,pipe) }
 fn rollback_pending_m(&mut self, params: u32, pending: u32, mem:&mut[u8]) -> u32 { let _=&mem;crate::frontend::PipeHooks::rollback_pending(self,params,pending) }
 fn release_pending_m(&mut self, params: u32, pending: u32, mem:&mut[u8]) -> u32 { let _=&mem;crate::frontend::PipeHooks::release_pending(self,params,pending) }
 fn reactivate_m(&mut self, cs: u32, mem:&mut[u8])  { let _=&mem;crate::frontend::PipeHooks::reactivate(self,cs) }
 fn license_kind_m(&mut self,ctx:u32,obj:u32,trace:u32,mem:&mut[u8])->u32 {crate::frontend::pipe_license_kind(self,mem,ctx,obj,trace)}
}

pub fn enter_pipeline_m(mem:&mut[u8],state:u32,next:u8,previous:&mut u8,lock:&mut impl crate::reentrant2::StateLockM)->u32 {
 let mut rc=lock.acquire_m(mem);if rc>=0x80000000{return rc;}
 if mem[state as usize]==1 {*previous=mem[state as usize];mem[state as usize]=next;}else {rc=0x81202011;}
 let release=lock.release_m(mem);if release>=0x80000000 && rc<0x80000000 {rc=release;}rc
}

pub fn restore_pipeline_m(mem:&mut[u8],state:u32,previous:u32,lock:&mut impl crate::reentrant2::StateLockM) {
 if previous!=0 && lock.acquire_m(mem)<0x80000000 {mem[state as usize]=previous as u8;lock.release_m(mem);}
}
struct PipeLockM<'a,H:PipeHooksM> {hooks:&'a mut H,mutex:u32}
impl<H:PipeHooksM> crate::reentrant2::StateLockM for PipeLockM<'_,H> {
 fn acquire_m(&mut self,mem:&mut[u8])->u32 {self.hooks.lock_m(self.mutex,mem)}
 fn release_m(&mut self,mem:&mut[u8])->u32 {self.hooks.unlock_m(self.mutex,mem)}
}

pub fn open_pipeline_ctx_m<H: PipeHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    a: u32,
    b: u32,
    ctx: &mut u32,
    obj: &mut u32,
) -> u32 {
    *obj = 0;
    let rc = hooks.check_open_m(a, b, mem);
    if rc >= 0x80000000 {
        if rc & 0x1fff == 8 {
            return 0x81202008;
        }
        return rc;
    }
    let (rc, fresh) = hooks.resolve_ctx_m(mem_word(mem, a), mem_word(mem, a.wrapping_add(4)), mem);
    *ctx = fresh;
    if rc < 0x80000000 {
        *obj = a;
    }
    rc
}

pub fn set_pipeline_m<H: PipeHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    a: u32,
    b: u32,
    name: u32,
    d: u32,
) -> u32 {
    let mut ctx = 0;
    let mut object = 0;
    let rc = open_pipeline_ctx_m(hooks, mem, a, b, &mut ctx, &mut object);
    if rc >= 0x80000000 {
        return rc;
    }
    let o = object;
    mem[o as usize + 0x80] = d as u8;
    let params = mem_word(mem, ctx.wrapping_add(20));
    hooks.set_textanalysis_m(params, d == 2, mem);
    let mut state = 0;
    let omutex = mem_word(mem, o.wrapping_add(0x2c));
    let rc = enter_pipeline_m(mem, o.wrapping_add(0x30),2,&mut state,&mut PipeLockM {
            hooks: &mut *hooks,
            mutex: omutex,
        });
    if rc >= 0x80000000 {
        return rc;
    }
    let mut pipeline = 0;
    let mut reactivate = 0;
    let mut rc = rc;
    if name != 0 && mem[name as usize] != 0 {
        let cs = mem_word(mem, o.wrapping_add(0x7c));
        if cs != 0
            && pipe_cmp(
                terminated(mem, name),
                terminated(mem, mem_word(mem, cs.wrapping_add(4))),
            ) == 0
        {
            restore_pipeline_m(mem, o.wrapping_add(0x30),u32::from(state),&mut PipeLockM {
                    hooks: &mut *hooks,
                    mutex: omutex,
                });
            return rc;
        }
        let (rc2, pending) = hooks.query_pending_m(params, mem);
        rc = rc2;
        if rc >= 0x80000000 {
            restore_pipeline_m(mem, o.wrapping_add(0x30),u32::from(state),&mut PipeLockM {
                    hooks: &mut *hooks,
                    mutex: omutex,
                });
            return rc;
        }
        let (rc2, fresh, flag) = hooks.trace_pipe_m(mem_word(mem, o),
            mem_word(mem, o.wrapping_add(4)),
            name,
            cs,
            mem_word(mem, o.wrapping_add(0x3c)), mem);
        pipeline = fresh;
        reactivate = flag;
        rc = rc2;
        if rc < 0x80000000 {
            let kind = hooks.license_kind_m(ctx,o,pipeline,mem);
            if mem[o as usize + 0x6a] == 0 {
                mem[o as usize + 0x69] = kind as u8;
                rc = hooks.start_object_m(ctx, o, mem);
            } else {
                let old = u32::from(mem[o as usize + 0x69]);
                if old != kind {
                    let channel = mem_word(mem, o.wrapping_add(0x10));
                    rc = hooks.lock_m(mem_word(mem, channel.wrapping_add(0x90)), mem);
                    if rc < 0x80000000 {
                        let usage =
                            channel.wrapping_add(0x30).wrapping_add(kind.wrapping_mul(24));
                        let feat = if kind < 4 {
                            u32::from(PIPE_KIND_MAP[kind as usize])
                        } else {
                            6
                        };
                        if mem_word(mem, usage.wrapping_add(12))
                            < mem_word(mem, usage.wrapping_add(4))
                        {
                            mem_store(
                                mem,
                                usage.wrapping_add(12),
                                mem_word(mem, usage.wrapping_add(12)).wrapping_add(1),
                            );
                            pipe_switch_m(
                                hooks, mem, ctx, channel, o, old, kind, 1, usage, feat,
                            );
                        } else if hooks.lic_count_m(feat, mem)
                            < mem_word(mem, usage.wrapping_add(16))
                            && hooks.lic_feat_m(feat, mem) == 0
                        {
                            pipe_switch_m(
                                hooks, mem, ctx, channel, o, old, kind, 2, usage, feat,
                            );
                        } else {
                            rc = 0x81203d06;
                            hooks.notify_lic_m(ctx, channel, kind, 7, mem);
                            if old == 0 {
                                rc = 0;
                            }
                            hooks.unlock_m(mem_word(mem, channel.wrapping_add(0x90)), mem);
                        }
                    }
                }
            }
            if rc >= 0x80000000 {
                hooks.release_pipe_m(pipeline, mem);
                hooks.rollback_pending_m(params, pending, mem);
            }
        } else {
            hooks.rollback_pending_m(params, pending, mem);
        }
        let status = hooks.release_pending_m(params, pending, mem);
        if status >= 0x80000000 && rc < 0x80000000 {
            rc = status;
        }
        if rc >= 0x80000000 {
            if reactivate != 0 && mem_word(mem, o.wrapping_add(0x7c)) != 0 {
                hooks.reactivate_m(mem_word(mem, o.wrapping_add(0x7c)), mem);
            }
            restore_pipeline_m(mem, o.wrapping_add(0x30),u32::from(state),&mut PipeLockM {
                    hooks: &mut *hooks,
                    mutex: omutex,
                });
            return rc;
        }
    }
    if mem_word(mem, o.wrapping_add(0x7c)) != 0 {
        rc = hooks.release_pipe_m(mem_word(mem, o.wrapping_add(0x7c)), mem);
    }
    mem_store(mem, o.wrapping_add(0x7c), pipeline);
    restore_pipeline_m(mem, o.wrapping_add(0x30),u32::from(state),&mut PipeLockM {
            hooks: &mut *hooks,
            mutex: omutex,
        });
    rc
}
fn pipe_switch_m<H: PipeHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    ctx: u32,
    channel: u32,
    o: u32,
    old: u32,
    kind: u32,
    license_state: u8,
    usage: u32,
    feat: u32,
) {
    if mem[o as usize + 0x6a] == 1 {
        let previous = channel.wrapping_add(0x30).wrapping_add(old.wrapping_mul(24));
        if mem_word(mem, previous.wrapping_add(12)) != 0 {
            mem_store(
                mem,
                previous.wrapping_add(12),
                mem_word(mem, previous.wrapping_add(12)).wrapping_sub(1),
            );
        }
    } else {
        hooks.release_token_m(mem_word(mem, o.wrapping_add(0x6c)), mem);
    }
    hooks.notify_lic_m(ctx, channel, old, 6, mem);
    mem[o as usize + 0x6a] = license_state;
    mem[o as usize + 0x69] = kind as u8;
    if hooks.lic_count_m(feat, mem).wrapping_add(mem_word(mem, usage.wrapping_add(12)))
        > mem_word(mem, usage.wrapping_add(8))
    {
        hooks.lic_count_m(feat, mem);
    }
    hooks.notify_lic_m(ctx, channel, kind, 5, mem);
    hooks.unlock_m(mem_word(mem, channel.wrapping_add(0x90)), mem);
}

pub fn remove_element_ptr_m(m:&mut Memory<'_>,h:&mut impl Calls,a:u32,elem:u32)->u32 {
 if a==0 {return 0;}
 let data=m.word(a);let size=m.half(a+4) as u32;
 let end=data.wrapping_add(m.word(a+20).wrapping_mul(size));
 if elem==0 || data>elem || elem>end {return 0;}
 let callback=m.word(a+12);
 if callback!=0 {h.call(m,callback,&[elem,m.word(a+28)]);}
 let next=elem.wrapping_add(m.half(a+4) as u32);
 let dst=m.offset(elem) as u32;let src=m.offset(next) as u32;
 crate::frontend::move_bytes(m.bytes,dst,src,end.wrapping_sub(next));
 m.put_word(a+20,m.word(a+20).wrapping_sub(1));1
}

pub fn remove_guarded_m(m:&mut Memory<'_>,h:&mut impl Calls,p:u32,elem:u32)->u32 {
 if p==0 || elem==0 {return 0x8b702007;}
 let mut rc=h.call(m,0x176f2c,&[m.word(p+4)]);
 if rc<0x80000000 {
  if h.call(m,0x196fbc,&[m.word(p+16),elem])==0 {rc=0x8b702007;}
  let release=h.call(m,0x176f5c,&[m.word(p+4)]);
  if release>=0x80000000 && rc<0x80000000 {rc=release;}
 }
 rc
}

pub trait SchedulerLicenseHooksM {
 fn acquire_m(&mut self,mutex:u32,mem:&mut[u8])->u32;
 fn release_m(&mut self,mutex:u32,mem:&mut[u8])->u32;
 fn ports_m(&mut self,feature:u32,mem:&mut[u8])->u32;
 fn token_m(&mut self,feature:u32,mem:&mut[u8])->u32;
 fn notify_m(&mut self,context:u32,channel:u32,kind:u32,level:u32,mem:&mut[u8]);
 fn log_usage_m(&mut self,logger:u32,level:u32,used:u32,limit:u32,threshold:u32,description:&[u8],mem:&mut[u8]);
}

pub struct SchedulerAdapterM<'a,L,C> {pub locks:&'a mut L,pub licensing:&'a mut C}
impl<L:crate::frontend_part_a::SchedulerHooks,C:crate::frontend_part_a::SchedulerLicenseHooks> SchedulerLicenseHooksM for SchedulerAdapterM<'_,L,C> {
 fn acquire_m(&mut self,p:u32,_:&mut[u8])->u32 {self.locks.acquire(p)}
 fn release_m(&mut self,p:u32,_:&mut[u8])->u32 {self.locks.release(p)}
 fn ports_m(&mut self,p:u32,_:&mut[u8])->u32 {self.licensing.ports(p)}
 fn token_m(&mut self,p:u32,_:&mut[u8])->u32 {self.licensing.token(p)}
 fn notify_m(&mut self,a:u32,b:u32,c:u32,d:u32,_:&mut[u8]) {self.licensing.notify(a,b,c,d)}
 fn log_usage_m(&mut self,a:u32,b:u32,c:u32,d:u32,e:u32,f:&[u8],_:&mut[u8]) {self.licensing.log_usage(a,b,c,d,e,f)}
}

pub fn scheduler_license_usage_m(
    mem: &mut [u8],
    context: u32,
    channel: u32,
    kind: u32,
    level: u32,
    hooks: &mut impl SchedulerLicenseHooksM,
) {
    if kind > 1 {
        return;
    }
    let mut text = [0u8; 20];
    text[..3].copy_from_slice(if kind == 0 { b"tts" } else { b"cpr" });
    let len = if mem[channel.wrapping_add(0x94) as usize] == 0 {
        text[3..15].copy_from_slice(b",unthrottled");
        15
    } else {
        3
    };
    let base = channel.wrapping_add(0x30).wrapping_add(kind * 24);
    let used = hooks.ports_m(if kind == 0 { 0 } else { 3 }, mem)
        .wrapping_add(mem_word(mem, base.wrapping_add(12)));
    hooks.log_usage_m(mem_word(mem, context.wrapping_add(0x10)),
        level,
        used,
        mem_word(mem, base.wrapping_add(4)).wrapping_add(mem_word(mem, base.wrapping_add(16))),
        mem_word(mem, base.wrapping_add(8)),
        &text[..len + 1], mem);
}

pub fn increase_scheduler_license_m(
    mem: &mut [u8],
    context: u32,
    object: u32,
    hooks: &mut impl SchedulerLicenseHooksM,
) -> u32 {
    if mem[object.wrapping_add(0x6a) as usize] != 0 {
        return 0x81202011;
    }
    let mut kind = u32::from(mem[object.wrapping_add(0x69) as usize]);
    let channel = mem_word(mem, object.wrapping_add(0x10));
    let mut rc = hooks.acquire_m(mem_word(mem, channel.wrapping_add(0x90)), mem);
    if rc >= 0x80000000 {
        return rc;
    }
    if kind == 2 {
        let base = channel.wrapping_add(0x60);
        if mem_word(mem, base.wrapping_add(12)) < mem_word(mem, base.wrapping_add(4)) {
            mem_store(
                mem,
                base.wrapping_add(12),
                mem_word(mem, base.wrapping_add(12)).wrapping_add(1),
            );
            mem[object.wrapping_add(0x6a) as usize] = 1;
        } else {
            rc = 0x81203d06;
        }
        hooks.notify_m(context, channel, kind, if rc == 0x81203d06 { 7 } else { 5 }, mem);
    } else if kind <= 1 {
        if kind == 1
            && mem_word(mem, channel.wrapping_add(0x54)) >= mem_word(mem, channel.wrapping_add(0x4c))
            && hooks.ports_m(3, mem) >= mem_word(mem, channel.wrapping_add(0x58))
            && (mem_word(mem, channel.wrapping_add(0x3c)) < mem_word(mem, channel.wrapping_add(0x34))
                || hooks.ports_m(0, mem) < mem_word(mem, channel.wrapping_add(0x58)))
        {
            hooks.notify_m(context, channel, kind, 7, mem);
            kind = 0;
            mem[object.wrapping_add(0x69) as usize] = 0;
        }
        let base = channel.wrapping_add(0x30).wrapping_add(kind * 24);
        let feature = if kind == 0 { 0 } else { 3 };
        if mem_word(mem, base.wrapping_add(4)) > mem_word(mem, base.wrapping_add(12)) {
            mem_store(
                mem,
                base.wrapping_add(12),
                mem_word(mem, base.wrapping_add(12)).wrapping_add(1),
            );
            mem[object.wrapping_add(0x6a) as usize] = 1;
        } else if hooks.ports_m(feature, mem) < mem_word(mem, base.wrapping_add(16)) {
            let token = hooks.token_m(feature, mem);
            mem_store(mem, object.wrapping_add(0x6c), token);
            if token != u32::MAX {
                mem[object.wrapping_add(0x6a) as usize] = 2;
            }
        }
        let level = if mem[object.wrapping_add(0x6a) as usize] == 0 {
            rc = 0x81203d06;
            7
        } else {
            if hooks.ports_m(feature,mem)
                .wrapping_add(mem_word(mem, base.wrapping_add(12)))
                > mem_word(mem, base.wrapping_add(8))
            {

                hooks.ports_m(feature, mem);
            }
            5
        };
        hooks.notify_m(context, channel, kind, level, mem);
    }
    hooks.release_m(mem_word(
        mem,
        mem_word(mem, object.wrapping_add(0x10)).wrapping_add(0x90),
    ), mem);
    rc
}
