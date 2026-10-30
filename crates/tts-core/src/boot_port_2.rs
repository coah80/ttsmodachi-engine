//! Boot path lane two. Guest rodata addresses are supplied by the caller.
use crate::boot::Memory;
/// Return the module interface table D_8f88b0 supplied by the caller.
pub fn fn_1d75ac(m: &mut Memory<'_>, index: u32, out: u32, table: u32) -> u32 {
    if index <= 1 { m.put_word(out, table); 0 } else { 0x89902001 }
}
/// Return the module interface table D_8f8954 supplied by the caller.
pub fn fn_1d88c8(m: &mut Memory<'_>, index: u32, out: u32, table: u32) -> u32 {
    if index <= 1 { m.put_word(out, table); 0 } else { 0x8bf02001 }
}
/// Return the module interface table D_8f89c8 supplied by the caller.
pub fn fn_1de860(m: &mut Memory<'_>, index: u32, out: u32, table: u32) -> u32 {
    if index <= 1 { m.put_word(out, table); 0 } else { 0x89c02001 }
}
/// Return the module interface table D_8f8bb0 supplied by the caller.
pub fn fn_1e3ecc(m: &mut Memory<'_>, index: u32, out: u32, table: u32) -> u32 {
    if index <= 1 { m.put_word(out, table); 0 } else { 0x8a702001 }
}
/// Return the module interface table D_8f8c30 supplied by the caller.
pub fn fn_1e5110(m: &mut Memory<'_>, index: u32, out: u32, table: u32) -> u32 {
    if index <= 1 { m.put_word(out, table); 0 } else { 0x89602001 }
}
/// Return the module interface table D_8f8cd8 supplied by the caller.
pub fn fn_1e60cc(m: &mut Memory<'_>, index: u32, out: u32, table: u32) -> u32 {
    if index <= 1 { m.put_word(out, table); 0 } else { 0x81102001 }
}
/// Return the module interface table D_8f8ec0 supplied by the caller.
pub fn fn_1ea344(m: &mut Memory<'_>, index: u32, out: u32, table: u32) -> u32 {
    if index <= 1 { m.put_word(out, table); 0 } else { 0x8b802001 }
}
/// Return the module interface table D_8f8f88 supplied by the caller.
pub fn fn_1ee6d8(m: &mut Memory<'_>, index: u32, out: u32, table: u32) -> u32 {
    if index <= 1 { m.put_word(out, table); 0 } else { 0x89802001 }
}

/// Reentrant dictionary-interface result. Pointers remain guest addresses.
#[derive(Clone, Copy)]
pub struct Part { pub status:u32, pub result:u32, pub tag:u16, pub separator:u8 }
pub trait Parts {
    /// `tn` selects `nrpart_tn`, otherwise `nrpart_g2p`, in `fecfg`.
    /// `prior` preserves result/separator when the callback leaves an output untouched.
    fn get_part(&mut self,m:&mut Memory<'_>,method:u32,handle:[u32;2],tn:bool,prior:Part)->Part;
}
/// Read and parse both token-normalization switches through the engine vtable.
pub fn fn_1e512c(m:&mut Memory<'_>,o:u32,h:&mut impl Parts)->u32 {
    let mut rc=0;
    let mut prior=Part{status:0,result:0,tag:0xffff,separator:0};
    for (tn,off) in [(false,0xf0),(true,0xf1)] {
        prior.tag=0xffff;
        let p=h.get_part(m,m.word(m.word(o+24)+28),[m.word(o+28),m.word(o+32)],tn,prior);
        prior=p;
        rc=p.status;
        if rc>>31!=0 {return 0x89602000;}
        if p.tag==1 && p.result!=0 {
            let s=m.word(p.result);
            let hit=crate::fe_text::strchr_195d30(m.bytes,s-m.base,p.separator as u32);
            if hit!=0 {m.bytes[hit as usize]=0;}
            let s=m.word(p.result);
            let n=crate::pronunciation::parse_int(&m.bytes[m.offset(s)..]) as u8;
            let at=m.offset(o+off);m.bytes[at]=n;
        }
    }
    rc
}
/// Append a unique token name. All allocation uses the existing guest heap port.
pub fn fn_1e3ef8(m:&mut Memory<'_>,o:u32,name:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    let mut i=0u16;
    while i<m.half(o+0x6c) {
        if crate::normalization::strcmp_at(m.bytes,m.word(m.word(o+0x70)+i as u32*4)-m.base,name-m.base)==0 {return 0;}
        i=i.wrapping_add(1);
    }
    let ctx=m.word(o);let heap=m.word(ctx+4);
    let n=m.half(o+0x6c) as u32;
    let array=if n==0 {crate::boot_part_e::calloc(m,heap,1,4,h)} else {crate::boot_part_e::fn_1963e0(m,heap,m.word(o+0x70),n*4+4,h)};
    if array!=0 {
        m.put_word(o+0x70,array);
        let size=crate::fe_text::strlen_195d90(m.bytes,name-m.base) as u32+1;
        let heap=m.word(m.word(o)+4);
        let text=crate::boot_part_e::calloc(m,heap,1,size,h);
        m.put_word(m.word(o+0x70)+m.half(o+0x6c) as u32*4,text);
        if text!=0 {
            crate::normalization::strcpy_at(m.bytes,text-m.base,name-m.base);
            m.put_half(o+0x6c,m.half(o+0x6c).wrapping_add(1));return 0;
        }
    }
    h.call(m,0x173dc0,&[m.word(m.word(o)+16),m.word(o+0x48),0x5df0,0]);
    0x8960200a
}
/// Backend query, calling the landed lane-5 backend port.
pub fn fn_1e5f2c(m:&mut Memory<'_>,p:u32,tag:u32,out:u32,g:&ContextGlobals,h:&mut impl crate::boot_part_e::Host)->u32 {
    if crate::frontend::validate_handle(p!=0,tag,0x3815,0x30)>>31!=0 {return 0x81102008;}
    if p==0 {return 0x81102007;}
    let backend=m.word(p+0x28);
    crate::boot_port_5::fn_265b3c(m,&mut Backend{host:h,g},backend,out)
}

/// The pair sentinel and fallback context used by fn_19611c.
pub struct ContextGlobals {pub empty:[u8;8],pub fallback:u32}
fn context(m:&Memory<'_>,a:u32,b:u32,g:&ContextGlobals)->(u32,u32) {
    let mut p=0;
    let empty_a=u32::from_le_bytes(g.empty[..4].try_into().unwrap());
    let empty_b=u32::from_le_bytes(g.empty[4..].try_into().unwrap());
    // The real resolver compares the sentinel and validates the tag before
    // dereferencing the handle. Invalid tags may carry unmapped pointers.
    let pointed=if a==0 || b!=0x1e61 || (a==empty_a && b==empty_b) {None}else{Some(m.half(a))};
    let r=crate::frontend::resolve_engine_context(a,pointed,b,&g.empty,g.fallback,&mut p);
    (r,p)
}
struct PcreOpen<'a,'b,H> {m:&'a mut Memory<'b>,h:&'a mut H,g:&'a ContextGlobals,owner:u32}
impl<H:crate::boot_part_e::Host> crate::regex::PcreHost for PcreOpen<'_,'_,H> {
    fn validate(&mut self,p:u32,k:u32,t:u32,s:u32)->u32 {crate::frontend::validate_handle(p!=0,k,t,s)}
    fn resolve_owner(&mut self,p:u32,k:i32)->(u32,u32){let r=context(self.m,p,k as u32,self.g);self.owner=r.1;r}
    fn heap_of(&mut self,p:u32)->u32 {self.m.word(p+4)}
    fn alloc(&mut self,p:u32,n:u32,s:u32)->Option<u32> {let r=crate::boot_part_e::calloc(self.m,p,n,s,self.h);if r==0 {None}else{Some(r)}}
    fn free(&mut self,p:u32,v:u32){crate::boot_part_e::release(self.m,p,v,self.h)}
}
struct PcreInit<'a,H>(&'a mut H);
impl<H:crate::boot_part_e::Host> crate::component_live_b::Host for PcreInit<'_,H> {
    fn call_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32 {
        match f {
            0x23ffd4=>crate::component_live_b::pcre_deinit_m(m,a[0],a[1],self),
            0x1963b8=>crate::boot_part_e::calloc(m,a[0],a[1],a[2],self.0),
            0x196428=>{crate::boot_part_e::release(m,a[0],a[1],self.0);0},
            _=>self.0.call(m,f,a),
        }
    }
}
/// Open the synthesis setup and its PCRE workspace with the landed PCRE ports.
pub fn fn_1d79dc(m:&mut Memory<'_>,h0:u32,h1:u32,out:u32,g:&ContextGlobals,h:&mut impl crate::boot_part_e::Host)->u32 {
    if out==0 {return 0x8bf02007;}
    m.put_word(out,0);m.put_word(out+4,0);
    let (rc,ctx)=context(m,h0,h1,g);if rc>>31!=0 {return rc;}
    let mut slot=crate::regex::PcreHandleOut::default();
    let mut open=PcreOpen{m,h,g,owner:0};
    let rc=crate::regex::pcre_open(&mut open,h0,h1 as i32,Some(&mut slot));
    if rc>>31!=0 {return rc;}
    let regex=slot.object.expect("successful PCRE open");
    open.m.put_word(regex,open.owner);
    let rc=crate::component_live_b::pcre_init_m(open.m,regex,slot.key,10,10,&mut PcreInit(open.h));
    if rc>>31!=0 {return rc;}
    let buf=crate::boot_part_e::alloc(open.m,open.m.word(ctx+4),0x50,open.h);
    if buf==0 {open.h.call(open.m,0x173dc0,&[open.m.word(ctx+16),0x8f8910,0xc350,0]);return 0x8bf0200a;}
    crate::fe_text::memset_195bec(open.m.bytes,buf-open.m.base,0,0x50);
    open.m.put_word(buf,ctx);open.m.put_word(buf+0x40,regex);open.m.put_word(buf+0x44,slot.key);
    open.m.put_word(out+4,0xf618);open.m.put_word(out,buf);rc
}

/// Reset and load the six synthesis-setup arrays. The cursor scratch word is
/// caller-owned and restored on every return, including callee errors.
pub fn fn_1d7c3c(m:&mut Memory<'_>,o:u32,tag:u32,base:u32,limit:u32,scratch:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    let saved=m.word(scratch);m.put_word(scratch,14);
    let rc=setup(m,o,tag,base,limit,scratch,h);m.put_word(scratch,saved);rc
}
fn setup(m:&mut Memory<'_>,o:u32,tag:u32,base:u32,limit:u32,cursor:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    let rc0=crate::frontend::validate_handle(o!=0,tag,0xf618,0x50);
    if rc0>>31!=0 || o==0 {return 0x8bf02008;}
    if crate::frontend::validate_handle(o!=0,tag,0xf618,0x50)>>31!=0 {return 0x8bf03e81;}
    for off in [0x18,0x20,0x28,0x30,0x10,0x38] {
        let count=m.word(o+off+4);let base=m.base;
        crate::boot_port_4::fn_248210(m,o,o+off,count,&mut ForeignHeap{host:h,owner:o,base});m.put_word(o+off+4,0);
    }
    m.put_word(o+0x0c,0);
    let arena_base=m.base;
    crate::boot_port_4::fn_24752c(m,o,&mut ForeignHeap{host:h,owner:o,base:arena_base});
    crate::boot_port_4::fn_2475f8(m,o,&mut ForeignHeap{host:h,owner:o,base:arena_base});
    if limit==0 {return 0x8bf03e89;}
    if base==0 {return 0x8bf03e81;}
    if m.word(cursor).wrapping_add(5)>limit {
        h.call(m,0x173dc0,&[m.word(m.word(o)+16),0x8f8910,0xc353,0]);return rc0;
    }
    crate::frontend::copy_str_n(m.bytes,o+4-m.base,base+m.word(cursor)-m.base,3);
    let at=m.offset(o+7);m.bytes[at]=0;
    let v=m.byte(base+m.word(cursor)+3);let at=m.offset(o+8);m.bytes[at]=v;
    let v=m.byte(base+m.word(cursor)+4);let at=m.offset(o+9);m.bytes[at]=v;
    m.put_word(cursor,m.word(cursor)+5);
    for (stage,(off,mode)) in [(0x10,1),(0x18,0),(0x20,0),(0x28,0),(0x30,0),(0x38,2)].into_iter().enumerate() {
        let rc=h.call(m,0x248688,&[o,o+off,o+off+4,mode,base,limit,m.word(cursor)]);
        let err=if stage==0 {0x8bf03e83}else if stage==5 {0x8bf03e84}else {0x8bf03e82};
        if rc>>31!=0 {return if rc0&0x1fff!=0 {rc0}else {err};}
        if stage==0 && m.word(o+0x14)==0 {
            h.call(m,0x173dc0,&[m.word(m.word(o)+16),0x8f8910,0xc353,0]);return if rc0&0x1fff!=0 {rc0}else{err};
        }
        if stage<5 && crate::boot_port_4::fn_248a28(m,base,limit,cursor)>>31!=0 {
            return if stage==4 {0x8bf03e84}else{err};
        }
    }
    if h.call(m,0x247334,&[o])>>31!=0 {return 0x8bf03e87;}
    if h.call(m,0x247550,&[o])>>31!=0 {return 0x8bf03e87;}
    m.put_word(o+0x0c,1);0
}

/// Guest string addresses used by the config and dictionary path builders.
pub struct ConfigStrings {pub langcode:u32,pub voice:u32,pub fecfg:u32,pub prefix:u32,pub packaging:u32,pub clc:u32,pub clc_slash:u32,pub slash:u32,pub colon:u32,pub sysdct_slash:u32}
fn config(m:&mut Memory<'_>,o:u32,key:u32,slot:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    // Standard comparator and callback-free locks use the existing live bridge.
    // Nonstandard comparators/locks remain explicit engine dispatch, never a success fallback.
    if let Some(r)=m.boot_lookup_pure(0x175a1c,&[o,key,slot],0x196980) {r}
    else {h.call(m,0x175a1c,&[o,key,slot])}
}
fn append(m:&mut Memory<'_>,dst:u32,src:u32){crate::fe_text::strcat_195c90(m.bytes,dst-m.base,src-m.base);}
fn lower(m:&mut Memory<'_>,p:u32){
    let mut i=0u16;
    while (i as u32)<crate::fe_text::strlen_195d90(m.bytes,p-m.base) as u32 {
        let at=m.offset(p+i as u32);m.bytes[at]=crate::normalization::tolower_240ba0(m.bytes[at] as u32) as u8;i=i.wrapping_add(1);
    }
}
/// Add the language-family, language and voice token names. scratch is 128
/// disjoint caller-owned bytes; restoration includes every error path.
pub fn fn_1e5278(m:&mut Memory<'_>,p:u32,k:&ConfigStrings,scratch:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    let saved=core::array::from_fn::<_,128,_>(|i|m.byte(scratch+i as u32));
    let rc=language_tokens(m,p,k,scratch,h);
    for (i,v) in saved.into_iter().enumerate(){let at=m.offset(scratch+i as u32);m.bytes[at]=v;}rc
}
fn language_tokens(m:&mut Memory<'_>,p:u32,k:&ConfigStrings,s:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    m.put_word(s+104,0);
    let rc=config(m,m.word(m.word(p)+20),k.langcode,s+104,h);if rc>>31!=0{return rc;}
    let source=m.word(s+104);let len=crate::fe_text::strlen_195d90(m.bytes,source-m.base) as u32;
    for i in 0..len {let b=m.byte(source+i);let at=m.offset(s+i);m.bytes[at]=b;let at=m.offset(s+32+i);m.bytes[at]=b;}
    for (off,b) in [(2,b'x'),(3,0),(35,0)] {let at=m.offset(s+off);m.bytes[at]=b;}
    let rc=config(m,m.word(m.word(p)+20),k.voice,s+104,h);if rc>>31!=0{return rc;}
    let source=m.word(s+104);let len=crate::fe_text::strlen_195d90(m.bytes,source-m.base) as u32;
    if len>26{return 0x89602000;}
    for i in 0..=len {let b=m.byte(source+i);let at=m.offset(s+64+i);m.bytes[at]=b;}
    lower(m,s);lower(m,s+32);lower(m,s+64);
    let rc=fn_1e3ef8(m,p,s,h);if rc>>31!=0{return rc;}
    let at=m.offset(p+0x74);m.bytes[at]=m.half(p+0x6c) as u8;
    let rc=fn_1e3ef8(m,p,s+32,h);if rc>>31!=0{return rc;}
    fn_1e3ef8(m,p,s+64,h)
}
/// Construct the dictionary resource path, retaining the source's first lookup
/// status even when optional prefix/packaging queries fail.
pub fn fn_1ebe84(m:&mut Memory<'_>,owner:u32,dst:u32,flag:u32,k:&ConfigStrings,slot:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    let saved=m.word(slot);m.put_word(slot,0);
    let rc=path(m,owner,dst,flag,k,slot,h);m.put_word(slot,saved);rc
}
fn path(m:&mut Memory<'_>,o:u32,dst:u32,flag:u32,k:&ConfigStrings,slot:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    let cfg=m.word(o+20);let mut rc=config(m,cfg,k.langcode,slot,h);if rc>>31!=0{return rc;}
    let lang=m.word(slot);let mut dir=0;
    if flag==0 {rc=config(m,cfg,k.fecfg,slot,h);if rc>>31!=0{return rc;}dir=m.word(slot);}
    let at=m.offset(dst);m.bytes[at]=0;
    if config(m,cfg,k.prefix,slot,h)>>31==0 {let p=m.word(slot);if p!=0 && m.byte(p)!=0 {crate::normalization::strcpy_at(m.bytes,dst-m.base,p-m.base);}}
    if config(m,cfg,k.packaging,slot,h)>>31==0 {
        let p=m.word(slot);
        if p!=0 && crate::normalization::strcmp_at(m.bytes,p-m.base,k.clc-m.base)==0 {
            append(m,dst,k.clc_slash);append(m,dst,lang);append(m,dst,k.slash);
            if dir!=0 {append(m,dst,dir);}append(m,dst,k.colon);
        }
    }
    append(m,dst,k.sysdct_slash);append(m,dst,lang);
    if flag==0 {append(m,dst,k.slash);append(m,dst,dir);}
    let n=crate::fe_text::strlen_195d90(m.bytes,dst-m.base) as u16;
    for i in 0..n {let at=m.offset(dst+i as u32);m.bytes[at]=crate::normalization::tolower_240ba0(m.bytes[at] as u32) as u8;}rc
}

struct ForeignHeap<'a,H> {host:&'a mut H,owner:u32,base:u32}
impl<H:crate::boot_part_e::Host> crate::frontend::HeapVtableM for ForeignHeap<'_,H> {
    fn alloc_at_m(&mut self,o:u32,n:u32,b:&mut[u8])->u32 {let mut m=Memory{base:self.base,bytes:b};let f=m.word(m.word(m.word(m.word(self.owner)+4)));self.host.call(&mut m,f,&[o,n])}
    fn realloc_at_m(&mut self,o:u32,p:u32,n:u32,b:&mut[u8])->u32 {let mut m=Memory{base:self.base,bytes:b};let f=m.word(m.word(m.word(m.word(self.owner)+4))+8);self.host.call(&mut m,f,&[o,p,n])}
    fn release_at_m(&mut self,o:u32,p:u32,b:&mut[u8]){let mut m=Memory{base:self.base,bytes:b};let f=m.word(m.word(m.word(m.word(self.owner)+4))+12);self.host.call(&mut m,f,&[o,p]);}
}
struct Backend<'a,H>{host:&'a mut H,g:&'a ContextGlobals}
impl<H:crate::boot_part_e::Host> crate::boot_port_5::Host for Backend<'_,H>{fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.host.call(m,f,a)}}
impl<H:crate::boot_part_e::Host> crate::boot_port_5::ContextHost for Backend<'_,H>{
    fn context_pair(&mut self,_:&mut Memory<'_>)->[u8;8]{self.g.empty}
    fn default_context(&mut self,_:&mut Memory<'_>)->u32{self.g.fallback}
}

struct EngineCalls<'a,H>{host:&'a mut H,s:u32}
impl<H:crate::boot_part_e::Host> crate::boot::Calls for EngineCalls<'_,H>{
    fn call_wide(&mut self,_:&mut Memory<'_>,f:u32,_:&[u32])->u64 {panic!("unbound wide registry callee {f:x}")}
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
        if f==0x198760 && m.word(m.word(a[0]+36)+8)==0x196980 {
            let mem=&*m.bytes;
            return crate::frontend::find_node(mem,a[0],m.word(a[0]),m.byte(a[0]+40),a[1],|x,y| match (x,y) {
                (0,0)=>0,(0,_)=>-1,(_,0)=>1,_=>crate::normalization::strcmp_at(mem,x,y)
            }).unwrap_or(0);
        }
        if f==0x1963b8 {return crate::boot_part_e::calloc(m,a[0],a[1],a[2],self.host);}
        if let Some(r)=m.boot_round10_call(f,a){return r;}
        let frame=self.s;self.s+=128;
        let result=m.boot_parameter_call(self,f,a,frame);self.s=frame;
        result.unwrap_or_else(||self.host.call(m,f,a))
    }
}
impl<H:crate::boot_part_e::Host> crate::boot_part_b::Host for EngineCalls<'_,H>{
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {crate::boot::Calls::call(self,m,f,a)}
    fn thread_id(&mut self)->u32 {0}
}
fn invoke(m:&mut Memory<'_>,f:u32,a:&[u32],s:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    crate::boot::Calls::call(&mut EngineCalls{host:h,s},m,f,a)
}
fn registry_release(m:&mut Memory<'_>,reg:u32,name:u32,s:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    if reg==0 || name==0{return 0x8b602007;}
    let lock=m.word(reg+12);let rc=invoke(m,0x176f2c,&[lock],s+128,h);if rc>>31!=0{return rc;}
    let old=m.word(s);m.put_word(s,0);
    let find=invoke(m,0x1966ac,&[m.word(reg+16),name,s],s+128,h);let entry=m.word(s);m.put_word(s,old);
    let mut image=[0u8;40];let mut cb=0;let mut refs=0;
    if find>>31==0 {
        image[..32].copy_from_slice(&m.bytes[m.offset(entry)..m.offset(entry)+32]);
        refs=m.half(entry+28);cb=m.word(entry+24);
    }
    let remove=if find>>31==0 && refs==1 && cb!=0 {invoke(m,0x19672c,&[m.word(reg+16),name],s+128,h)}else{0};
    // Removal can reenter and replace the registry's handle halves.
    image[32..36].copy_from_slice(&m.word(reg).to_le_bytes());image[36..40].copy_from_slice(&m.word(reg+4).to_le_bytes());
    let scripts=crate::frontend::ReleaseScripts{lock:rc,find,found:if find>>31==0{Some(0)}else{None},remove,unlock:0};
    let callback=if cb==0 {None}else{Some(|ha,hb,tmp:&mut [u8;32]|{
        let saved=core::array::from_fn::<_,32,_>(|i|m.byte(s+i as u32));
        for (i,&v) in tmp.iter().enumerate(){let at=m.offset(s+i as u32);m.bytes[at]=v;}
        h.call(m,cb,&[ha,hb,s]);
        for (i,v) in saved.into_iter().enumerate(){let at=m.offset(s+i as u32);m.bytes[at]=v;}
    })};
    let mut status=crate::frontend::release_ling_entry(&mut image,Some(32),Some(name),&scripts,callback);
    if find>>31==0 && (refs!=1 || cb==0) {m.put_half(entry+28,u16::from_le_bytes([image[28],image[29]]));}
    let unlock=invoke(m,0x176f5c,&[m.word(reg+12)],s+128,h);if unlock>>31!=0 && status>>31==0 {status=unlock;}status
}
/// Open a phonmap mark wrapper. Resource registry and heap calls are live Rust.
/// scratch is 768 disjoint caller-owned bytes, restored by each nested bridge.
pub fn fn_1e36bc(m:&mut Memory<'_>,source_type:u32,source_name:u32,out:u32,g:&ContextGlobals,synth:u32,phon:u32,diagnostics:[u32;4],s:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    if out==0{return 0x8a702007;}
    let (rc,ctx)=context(m,source_type,source_name,g);if rc>>31!=0{return rc;}
    m.put_word(out,0);m.put_word(out+4,0);
    h.call(m,0x1738e8,&[m.word(ctx+16),0x8f8b6e,4,0,diagnostics[0]]);
    let p=crate::boot_part_e::calloc(m,m.word(ctx+4),1,0x2c,h);if p==0{return 0x8a70200a;}
    m.put_word(p+8,ctx);m.put_word(p+4,source_name);m.put_word(p,source_type);
    let saved=m.word(s);m.put_word(s,0);
    let mut rc=invoke(m,0x17b058,&[m.word(ctx+24),synth,s],s+128,h);
    let resource=m.word(s);m.put_word(s,saved);
    if rc>>31==0 {
        m.put_word(p+24,m.word(resource+4));m.put_word(s,0);
        rc=invoke(m,0x17b058,&[m.word(ctx+24),phon,s],s+128,h);
        let resource=m.word(s);m.put_word(s,saved);
        if rc>>31!=0 {h.call(m,0x173dc0,&[m.word(m.word(p+8)+16),0x8f8b6e,0xb02e,0]);rc=0;}
        else {m.put_word(p+12,m.word(resource+4));m.put_word(p+16,m.word(resource+8));m.put_word(p+20,m.word(resource+12));}
        m.put_word(out,p);m.put_word(out+4,0x72a5);
    } else {
        h.call(m,0x173dc0,&[m.word(m.word(p+8)+16),0x8f8b6e,0xb02d,0]);
        m.put_word(out,p);m.put_word(out+4,0x72a5);
        if crate::frontend::validate_handle(p!=0,0x72a5,0x72a5,0x2c)>>31==0 {
            let mut close=0;
            h.call(m,0x1738e8,&[m.word(ctx+16),0x8f8b6e,4,0,diagnostics[1]]);
            if m.word(p+24)!=0 {let r=registry_release(m,m.word(ctx+24),synth,s,h);if r>>31!=0 {close=r;}}
            if m.word(p+12)!=0 {let r=registry_release(m,m.word(ctx+24),phon,s,h);if close>>31==0 && r>>31!=0 {close=r;}}
            crate::boot_part_e::release(m,m.word(ctx+4),p,h);
            h.call(m,0x1738e8,&[m.word(ctx+16),0x8f8b6e,4,0,diagnostics[2],close]);
        }
        m.put_word(out,0);m.put_word(out+4,0);
    }
    h.call(m,0x1738e8,&[m.word(ctx+16),0x8f8b6e,4,0,diagnostics[3],rc]);rc
}

/// fn_1e0d14. Close a phonmap object: every reader, the word map and the
/// object itself. `scratch` is one caller-owned word outside the arena that
/// receives the map iterator handle.
pub fn fn_1e0d14<H: crate::boot_port_6b::Host>(m:&mut Memory<'_>,p:u32,tag:u32,scratch:u32,h:&mut H)->u32 {
    let mut rc=crate::frontend::validate_handle(p!=0,tag,0x72a4,0x238);
    if rc>>31!=0 {return rc;}
    let ctx=m.word(p+8);
    let log=m.word(ctx+0x10);
    h.call(m,0x1738e8,&[log,0x8f8ae0,4,0,0x1e0ed4]);
    for off in [0x0c,0x14,0x1c,0x24,0x10,0x18,0x20,0x28,0x22c,0x230] {
        let reader=m.word(p+off);
        if reader!=0 {rc=crate::boot_port_6b::fn_2412a0_m(m,h,reader);}
    }
    let dictionary=m.word(p+0x234);
    if dictionary!=0 {
        let open=crate::boot_live::alloc_iterator_m(m,dictionary,0,1,scratch,&mut EngineCalls{host:h,s:0});
        if open>>31==0 {
            let it=m.word(scratch);
            let mut key=0u32;
            let mut val=0u32;
            loop {
                let step=crate::frontend::step_iter(m.bytes,Some(it),Some(&mut key),Some(&mut val));
                if step>>31!=0 {break;}
                rc=crate::boot_port_6b::fn_2412a0_m(m,h,val);
            }
            crate::boot_live::release_iterator_m(m,&mut EngineCalls{host:h,s:0},it);
        }
        crate::boot_part_b2::guest_19665c_live(m,dictionary,&mut EngineCalls{host:h,s:0});
    }
    crate::boot_part_e::release(m,m.word(ctx+4),p,h);
    h.call(m,0x1738e8,&[log,0x8f8ae0,4,0,0x1e0ef4,rc]);
    rc
}

/// fn_1964e0. Create the map object: the five callback words, the owner and
/// the embedded tree. The callback words are the caller's canonical tokens;
/// the map factory stores them verbatim, exactly like the ARM copies the
/// ROM callback addresses into the object.
pub fn fn_1964e0(m:&mut Memory<'_>,ctx:u32,flag:u32,cb:[u32;5],out:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
    assert_eq!(m.base,0,"this port requires absolute arena indexes");
    if cb[0]==0||cb[1]==0||cb[2]==0||cb[3]==0||cb[4]==0||out==0 {return 0x89002007;}
    let o=crate::boot_part_e::calloc(m,ctx,1,0x44,h);
    m.put_word(out,o);
    if o==0 {return 0x8900200a;}
    for (i,v) in cb.iter().enumerate() {m.put_word(o+i as u32*4,*v);}
    m.put_word(o+20,ctx);
    crate::frontend::init_link(m.bytes,o+24,0xffffffff,o,ctx);
    if flag==1 {crate::frontend::set_link_flag(m.bytes,o+24);}
    0
}

/// fn_1ec8b4. The product version check of the dictionary context.
///
/// The dictionary lookup is the certified boot-port-2b port. `key` and
/// `text` are the guest strings the lookup uses and `required` is the word
/// D_8f8f40. `scratch` is five caller-owned words: the value pointer, the
/// count, the separator and the two end pointers of the version scan.
/// The logger is null in the certified scene, so the log calls carry the
/// module and the code but no format text.
pub fn fn_1ec8b4(
    m:&mut Memory<'_>,
    ctx:u32,
    handle:u32,
    out_a:u32,
    out_b:u32,
    key:u32,
    text:u32,
    required:u32,
    scratch:u32,
    h:&mut impl crate::boot_port_2b::Host,
    d:&mut impl crate::boot_port_2b::Dictionaries,
)->u32 {
    let rc=crate::boot_port_1::fn_176f94(m,ctx,handle,0xf37f,0x2a4);
    if rc>=0x80000000 {return 0x89802008;}
    let value=scratch;
    let count=scratch+4;
    let sep=scratch+8;
    let end1=scratch+12;
    let end2=scratch+16;
    m.put_byte(out_b,0);
    m.put_byte(out_a,0);
    m.put_half(count,0xffff);
    let owner=m.word(ctx+crate::boot_port_2b::ctx::OWNER);
    let logger=m.word(owner+crate::boot_port_2b::owner::LOGGER);
    let rc=crate::boot_port_2b::fn_1ea4e0(m,ctx,handle,key,text,value,count,sep,h,d);
    if m.half(count)==0 {return rc;}
    let list=m.word(value);
    let str_addr=m.word(list);
    let separator=m.byte(sep);
    let hit=crate::fe_text::strchr_195d30(m.bytes,str_addr-m.base,separator as u32);
    if hit!=0 {m.bytes[hit as usize]=0;}
    m.put_byte(out_b,1);
    let p1=crate::normalization::strtoul_scan(m.bytes,str_addr,end1,10,0) as i32;
    if p1<1 {h.call(m,0x173dc0,&[logger,0x8f8f44,0xa02e,0]);return 0x89802000;}
    let e1=m.word(end1);
    if e1==0||m.byte(e1)!=b'.' {h.call(m,0x173dc0,&[logger,0x8f8f44,0xa02e,0]);return 0x89802000;}
    let p2=crate::normalization::strtoul_scan(m.bytes,e1+1,end2,10,0) as i32;
    if p2<0 {h.call(m,0x173dc0,&[logger,0x8f8f44,0xa02e,0]);return 0x89802000;}
    let e2=m.word(end2);
    if e2==0||m.byte(e2)!=b'.' {h.call(m,0x173dc0,&[logger,0x8f8f44,0xa02e,0]);return 0x89802000;}
    let p3=crate::normalization::strtoul_scan(m.bytes,e2+1,end1,10,0) as i32;
    if p3<0 {h.call(m,0x173dc0,&[logger,0x8f8f44,0xa02e,0]);return 0x89802000;}
    let e3=m.word(end1);
    if e3==0 {h.call(m,0x173dc0,&[logger,0x8f8f44,0xa02e,0]);return 0x89802000;}
    if m.byte(e3)!=0&&m.byte(e3)!=b'.' {h.call(m,0x173dc0,&[logger,0x8f8f44,0xa02e,0]);return 0x89802000;}
    let r1=required&0xff;
    let r2=(required>>8)&0xff;
    let r3=(required>>16)&0xff;
    let low=(r1 as i32)<p1||((r1 as i32)==p1&&((r2 as i32)<p2||((r2 as i32)==p2&&(r3 as i32)<p3)));
    if low {h.call(m,0x1738e8,&[logger,0x8f8f44,5,0,0]);return rc;}
    if p1==1&&p2==0&&p3==0 {m.put_byte(out_a,0);} else {m.put_byte(out_a,1);}
    rc
}

/// fn_252d6c. Release the synthesis fetch table and every entry's f8 handle.
pub fn fn_252d6c(m:&mut Memory<'_>,e:u32,g:&ContextGlobals,h:&mut impl crate::boot_part_e::Host)->u32 {
    if e==0 {return 0x81102007;}
    let (rc,env)=context(m,m.word(e),m.word(e+4),g);
    if rc>=0x80000000 {return 0x81102007;}
    let heap=m.word(env+4);
    let count=m.half(e+8) as u32;
    let arr=m.word(e+12);
    for i in 0..count {
        let f8=m.word(arr+i*20+8);
        if f8!=0 {crate::boot_part_e::release(m,heap,f8,h);}
    }
    if arr!=0 {crate::boot_part_e::release(m,heap,arr,h);}
    crate::boot_part_e::release(m,heap,e,h);
    0
}

/// fn_1e0f10. Open a phonmap object for the handle pair `h0`/`h1` and store
/// the object and its tag in `out`. `scratch` is one caller-owned word outside
/// the arena that receives the voice string pointer; the map iterator handle
/// of the closing path uses the next word. On a successful voice lookup the
/// phonmap readers load through fn_1e0400_m and the word map publishes;
/// otherwise the object closes instead of publishing (src/fn_1e0d14.c).
/// Caller-owned inputs of the fn_1e0400 phonmap load: the region's
/// D_8f8ad4 word, the string-map/token-reader environment, and the five
/// word-map callbacks the ARM stores verbatim (copy, destroy, compare,
/// retain, release) in the region's own addresses.
pub struct PhonmapOpen<'a> {
    pub ver: u32,
    pub env: &'a crate::boot_port_6b::Env6b<'a>,
    pub callbacks: [u32; 5],
}

pub fn fn_1e0f10<H: crate::boot_port_6b::Host + crate::render_part_a::ReaderHost>(m:&mut Memory<'_>,h0:u32,h1:u32,out:u32,k:&ConfigStrings,g:&ContextGlobals,scratch:u32,v:&PhonmapOpen<'_>,h:&mut H)->u32 {
    if out==0 {return 0x8a702006;}
    m.put_word(out,0);m.put_word(out+4,0);
    let (rc,ctx)=context(m,h0,h1,g);if rc>>31!=0{return rc;}
    let log=m.word(ctx+0x10);
    crate::boot_part_e::Host::call(h,m,0x1738e8,&[log,0x8f8ae0,4,0,0x1e0f78]);
    let heap=m.word(ctx+4);
    let newobj=crate::boot_part_e::calloc(m,heap,1,568,h);
    let mut rc;
    if newobj==0 {
        crate::boot_part_e::Host::call(h,m,0x173dc0,&[log,0x8f8ae0,0x7148,0]);
        rc=0x8a70200a;
    } else {
        m.put_word(newobj,h0);m.put_word(newobj+4,h1);m.put_word(newobj+8,ctx);
        rc=config(m,m.word(ctx+0x14),k.voice,scratch,h);
        if rc>>31==0 {
            // src/fn_1e0d14.c fn_1e0f10: load the phonmap readers, then
            // release the voice string the config lookup returned.
            let val=m.word(scratch);
            rc=crate::boot_port_6b::fn_1e0400_m(m,h,newobj,val,v.ver,v.env);
            if val!=0 {crate::boot_part_e::Host::call(h,m,0x1757a8,&[m.word(ctx+0x14),val]);}
        }
        if rc>>31==0 {
            let heap=m.word(ctx+4);
            let cb=v.callbacks;
            rc=crate::boot_part_e::Host::call(h,m,0x1964e0,&[heap,0,cb[0],cb[1],cb[2],cb[3],cb[4],newobj+564]);
        }
        if rc>>31==0 {
            m.put_word(out,newobj);m.put_word(out+4,0x72a4);
        } else {
            m.put_word(out,newobj);m.put_word(out+4,0x72a4);
            fn_1e0d14(m,newobj,0x72a4,scratch+4,h);
            m.put_word(out,0);m.put_word(out+4,0);
        }
    }
    crate::boot_part_e::Host::call(h,m,0x1738e8,&[log,0x8f8ae0,4,0,0x1e1090]);
    rc
}

/// fn_195d58. The byte-string comparison of the key list.
fn cmp195d58(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    let (mut x, mut y) = (a, b);
    loop {
        let p = m.byte(x);
        let q = m.byte(y);
        if p == q && q != 0 {
            x += 1;
            y += 1;
            continue;
        }
        let d = p as i32 - q as i32;
        return if d < 0 { -1 } else if d > 0 { 1 } else { 0 };
    }
}

/// The broker host of fn_1ecd04. The certified scene reaches the allocator
/// seam and the two logger calls; the rest of the broker machine is
/// unreachable and reports the null-argument status.
pub struct Broker<'a, H>(pub &'a mut H);
impl<H: crate::boot_part_b::Host> crate::boot_part_b::Host for Broker<'_, H> {
    fn thread_id(&mut self) -> u32 {
        self.0.thread_id()
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
}
impl<H: crate::boot_part_b::Host> crate::boot_part_b::OutputHost for Broker<'_, H> {
    fn output(&mut self, _m: &mut Memory<'_>, _f: u32, _a: &[u32]) -> (u32, u32) {
        (0x89002007, 0)
    }
}
impl<H: crate::boot_part_b::Host> crate::boot_part_b::PropertyHost for Broker<'_, H> {
    fn property(&mut self, _m: &mut Memory<'_>, _d: u32, _k: &[u8]) -> (u32, u32) {
        (0x89002007, 0)
    }
    fn insert(&mut self, _m: &mut Memory<'_>, _d: u32, _k: u32, _e: [u32; 3]) -> u32 {
        0x89002007
    }
}
impl<H: crate::boot_part_b::Host> crate::boot_part_b::QueryHost for Broker<'_, H> {
    fn query_output(&mut self, _m: &mut Memory<'_>, _f: u32, _a: &[u32]) -> (u32, u32) {
        (0x89002007, 0)
    }
    fn insert_record(&mut self, _m: &mut Memory<'_>, _d: u32, _k: u32, _r: [u32; 4]) -> u32 {
        0x89002007
    }
    fn open_pair(&mut self, _m: &mut Memory<'_>, _f: u32, _o: [u32; 2], _out: u32) -> u32 {
        0x89002007
    }
}
impl<H: crate::boot_part_b::Host> crate::boot_part_b::DataHost for Broker<'_, H> {
    fn data_output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32], out: u32) -> u32 {
        if f == 0x173000 {
            return file_open(m, a[0], a[1], a[2], out, RB_MODE, &mut Broker(&mut *self.0));
        }
        0x89002007
    }
}
/// The guest "rb" mode string of the file service, supplied by the scene.
const RB_MODE: u32 = 0x10810;
/// fn_173000. The ROM file service boundary.
///
/// The handle object is allocated through the caller's heap, the source
/// vtable's `get` slot is tried first and its `open` slot is used when the
/// probe answers with one. A failing service releases the handle and clears
/// the caller's out word.
fn file_open(
    m: &mut Memory<'_>,
    src: u32,
    ctx: u32,
    path: u32,
    out: u32,
    rb: u32,
    h: &mut impl crate::boot_part_b::Host,
) -> u32 {
    if src == 0 || ctx == 0 || out == 0 || path == 0 {
        return 0x80602007;
    }
    let p = h.call(m, 0x1963b8, &[ctx, 1, 32]);
    m.put_word(out, p);
    if p == 0 {
        return 0x8060200a;
    }
    m.put_word(p, src);
    m.put_word(p + 4, ctx);
    let table = m.word(src + 8);
    let get = m.word(table + 40);
    let mut rc = if get == 0 {
        0x80602001
    } else {
        h.call(m, get, &[m.word(src + 12), m.word(src + 4), path, p + 12])
    };
    if rc == 1 {
        rc = h.call(
            m,
            m.word(table),
            &[m.word(src + 12), m.word(src + 4), path, rb, p + 16],
        );
    }
    if rc >= 0x80000000 {
        let vt = m.word(ctx);
        let method = m.word(vt + 12);
        let obj = m.word(ctx + 4);
        if method != 0 {
            h.call(m, method, &[obj, p]);
        }
        m.put_word(out, 0);
    }
    rc
}
struct DictHooks<'a, H>(&'a mut H);
impl<H: crate::boot_part_b::Host> crate::reentrant::HooksM for DictHooks<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.call(m, f, a)
    }
}

/// fn_1ecd04. The dictionary-exists query of the fe_dctlkp context.
///
/// The handle check is the certified boot-port-1 port, the key list is walked
/// with fn_195d58 and the resource is opened with the certified boot-part-b
/// broker port over the real broker scene. `scratch` is caller-owned words
/// outside the compared arena: the opened handle, the broker scratch and the
/// dictionary buffer.
pub fn fn_1ecd04(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    text: u32,
    found: u32,
    scratch: u32,
    h: &mut impl crate::boot_part_b::Host,
) -> u32 {
    let rc = crate::boot_port_1::fn_176f94(m, ctx, handle, 0xf37f, 0x2a4);
    if rc >= 0x80000000 {
        return 0x89802008;
    }
    m.put_byte(found, 0);
    let owner = m.word(ctx);
    let logger = m.word(owner + 0x10);
    if text == 0 {
        h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa029, 0]);
        return 0x89802007;
    }
    h.call(m, 0x1738e8, &[logger, 0x8f8f44, 4, 0, 0, text]);
    let count = m.half(ctx + 0x88) as u32;
    let keys = m.word(ctx + 0x68);
    let mut i = 0u32;
    while i < count {
        let key = m.word(keys + i * 4);
        if cmp195d58(m, text, key) == 0 {
            if (i as u16 as i16) < 0 {
                break;
            }
            m.put_byte(found, 1);
            return rc;
        }
        i += 1;
    }
    let resources = m.word(owner + 0x0c);
    let globals = crate::boot_part_b::DataOpenGlobals {
        module: 0,
        allocators: [0; 4],
        scratch: scratch + 8,
        fullpath_key: 0,
        add_item_format: 0,
        broker_label: 0,
        ctype_table: 0,
        brokerstring_key: 0,
    };
    // The fallback broker names live in the caller's scratch, outside the
    // compared arena: the ROM keeps them in its rodata.
    for (at, name) in [(scratch + 0x50, &b"sysdct\0"[..]), (scratch + 0x60, &b"lid\0"[..])] {
        let o = m.offset(at);
        m.bytes[o..o + name.len()].copy_from_slice(name);
    }
    let mut broker = Broker(h);
    let mut rc2 = 0u32;
    for i in 0..3u32 {
        let name = match i {
            0 => ctx + 0x9e,
            1 => scratch + 0x50,
            _ => scratch + 0x60,
        };
        let flags = if i == 0 { 0 } else { 1 };
        let rc = crate::boot_part_b::fn_1795f0(
            m,
            resources,
            name,
            flags,
            scratch,
            &globals,
            &mut broker,
        );
        if rc >= 0x80000000 {
            return if i == 0 { rc } else { 0 };
        }
        let mut dict = DictHooks(&mut broker);
        let rc3 = crate::reentrant::contains_m(
            m,
            &mut dict,
            ctx + 0x08 + i * 0x20,
            text,
            m.word(ctx + 0x04 + i * 0x20),
            found,
            scratch + 4,
        );
        if rc3 >= 0x80000000 {
            return rc3;
        }
        crate::boot_part_b::Host::call(&mut broker, m, 0x179f3c, &[resources, m.word(scratch)]);
        rc2 = rc3;
        if m.byte(found) != 0 {
            break;
        }
    }
    rc2
}

/// The guest strings of the fn_1ee808 row. The ROM keeps them in its own
/// rodata; the certified scene keeps the same text in the arena, so the
/// Rust host, the native oracle and the three ROM regions compare the same
/// bytes.
pub struct DestructorStrings {
    pub extraesclang: u32,
    pub extraesctn: u32,
    pub synthstream: u32,
    pub fedepes: u32,
    pub fedctlkp: u32,
    pub lingdb: u32,
}

/// The canonical guest address of D_8f902c. The native oracle links the
/// symbol to the same word and the three ROM regions read it through the
/// relocated literal pool of fn_1ee808.
const D_8F902C: u32 = 0x13800;
/// The canonical guest address of D_8d7660, the current handle pair. The
/// native oracle links the symbol to the same word and the three ROM regions
/// read it through the relocated literal pool of fn_176fd4.
const D_8D7660: u32 = 0x102c0;
/// The scratch layout of the fn_1ee808 row: the iterator handle, the value
/// slot of the map lookup, the entry slot of the registry lookup and the
/// 32-byte entry copy. The callback slot of the array lookup follows the
/// value slot inside fn_174f9c.
const DS_ITER: u32 = 0;
const DS_VALUE: u32 = 4;
const DS_ENTRY: u32 = 16;
const DS_COPY: u32 = 32;
const DS_TEARDOWN: u32 = 64;

/// fn_176f2c and fn_176f5c. The lock acquire and release. The inner table
/// holds the acquire at +8 and the release at +12, and the call takes the
/// lock's target word.
fn destructor_lock(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,p:u32,slot:u32)->u32 {
    if p==0 {return 0x81402007;}
    let f=m.word(m.word(m.word(p)+8)+slot);
    if f==0 {return 0;}
    h.call(m,f,&[m.word(p+8)])
}

/// fn_198760. The exact tree search. The comparator is the map's own
/// compare callback, so the scene's seam counts every comparison.
fn tree_find(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,tree:u32,key:u32)->u32 {
    let cmp=m.word(m.word(tree+36)+8);
    let mut n=m.word(tree);
    while n!=tree {
        let r=h.call(m,cmp,&[key,m.word(n+16)]) as i32;
        if r<0 {n=m.word(n);} else if r>0 {n=m.word(n+4);} else {
            if m.byte(tree+40)==0 {return n;}
            return tree_last(m,h,tree,n,key,cmp);
        }
        if n==0 {return 0;}
    }
    0
}

/// The flag walk of fn_198760: the last node that still compares equal.
fn tree_last(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,tree:u32,start:u32,key:u32,cmp:u32)->u32 {
    let mut last=start;
    loop {
        let mut cur=m.word(last);
        while cur!=tree {
            if h.call(m,cmp,&[key,m.word(cur+16)]) as i32==0 {break;}
            cur=m.word(cur+4);
        }
        if cur==tree {break;}
        last=cur;
    }
    last
}

/// fn_1987fc. The lower bound search of a tree whose flag is set.
fn tree_lower(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,tree:u32,key:u32)->u32 {
    let cmp=m.word(m.word(tree+36)+8);
    let mut cur=m.word(tree);
    let mut prev=0u32;
    while cur!=tree {
        let r=h.call(m,cmp,&[key,m.word(cur+16)]) as i32;
        if r>0 {cur=m.word(cur+4);} else if r<0||m.byte(tree+40)!=0 {
            prev=cur;cur=m.word(cur);
        } else {return cur;}
    }
    prev
}

/// fn_198f74. The first node of a tree, the leftmost node of the root.
fn tree_first(m:&mut Memory<'_>,tree:u32)->u32 {
    let mut n=m.word(tree);
    if n==tree {return 0;}
    let mut q=m.word(n);
    while q!=tree {n=q;q=m.word(q);}
    n
}

/// fn_1966ac. The map lookup: the tree search and the node's value word.
fn fn_1966ac(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,a:u32,b:u32,o:u32)->u32 {
    if a==0||b==0||o==0 {return 0x89002007;}
    let e=tree_find(m,h,a+24,b);
    if e!=0 {m.put_word(o,m.word(e+20));0} else {0x89002014}
}

/// fn_1970ac. The bounded fixed-stride item of the callback array.
fn fn_1970ac(m:&mut Memory<'_>,p:u32,i:u32,out:u32)->u32 {
    if p==0||out==0||i>=m.word(p+20) {return 0;}
    let size=m.half(p+4) as u32;
    m.put_word(out,m.word(p).wrapping_add(i.wrapping_mul(size)));
    1
}

/// fn_196e28. Remove one element of the callback array: the element
/// destructor runs, the tail moves down and the count drops.
fn fn_196e28(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,a:u32,idx:u32)->u32 {
    if a==0 {return 0;}
    let n=m.word(a+0x14);
    if n<=idx {return 0;}
    let size=m.half(a+4) as u32;
    let data=m.word(a);
    let dtor=m.word(a+0xc);
    let ctx=m.word(a+0x1c);
    if dtor!=0 {h.call(m,dtor,&[data.wrapping_add(idx.wrapping_mul(size)),ctx]);}
    let rest=n-idx-1;
    if rest!=0 {
        let from=data.wrapping_add((idx+1).wrapping_mul(size));
        let to=data.wrapping_add(idx.wrapping_mul(size));
        let len=(size*rest) as usize;
        m.bytes.copy_within(m.offset(from)..m.offset(from)+len,m.offset(to));
    }
    m.put_word(a+0x14,n-1);
    1
}

/// fn_174f9c. Remove the first callback whose three-word key matches. The
/// values tree answers the lookup and the callback array answers the
/// removal; the lock is taken for the whole search.
fn fn_174f9c(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,p:u32,key:u32,a:[u32;3],s:u32)->u32 {
    if p==0 {return 0x8b702007;}
    let rc=destructor_lock(m,h,m.word(p+4),8);
    if rc>>31!=0 {return rc;}
    let mut status=0u32;
    if key!=0&&m.byte(key)!=0 {
        let rc=fn_1966ac(m,h,m.word(p+8),key,s);
        if rc>>31!=0 {
            status=0x8b702000|((rc<<19)>>19);
        } else {
            let value=m.word(s);
            status=find_remove(m,h,m.word(value+8),a,s+4);
        }
    } else {
        status=find_remove(m,h,m.word(p+12),a,s+4);
    }
    let rc=destructor_lock(m,h,m.word(p+4),12);
    if rc>>31!=0&&status>>31==0 {status=rc;}
    status
}

/// The callback search of fn_174f9c.
fn find_remove(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,list:u32,a:[u32;3],s:u32)->u32 {
    if list==0 {return 0x8b702014;}
    let mut i=0u32;
    if fn_1970ac(m,list,i,s)==0 {return 0x8b702014;}
    loop {
        let cb=m.word(s);
        if m.word(cb)==a[0]&&m.word(cb+4)==a[1]&&m.word(cb+8)==a[2] {
            if fn_196e28(m,h,list,i)==0 {return 0x8b702100;}
            return 0;
        }
        i+=1;
        if fn_1970ac(m,list,i,s)==0 {return 0x8b702014;}
    }
}

/// fn_196794. Open an iterator over the map. A zero key starts at the
/// first node, a nonzero key searches the tree.
fn fn_196794(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,p:u32,key:u32,mode:u32,out:u32)->u32 {
    if p==0||out==0 {return 0x89002007;}
    let mut rc=0u32;
    let v=if key!=0 {
        let n=if mode!=0 {tree_find(m,h,p+24,key)} else {tree_lower(m,h,p+24,key)};
        if n==0 {rc=0x89002014;}
        n
    } else {
        tree_first(m,p+24)
    };
    if rc==0 {
        let o=crate::boot_part_e::calloc(m,m.word(p+0x14),1,12,h);
        m.put_word(out,o);
        if o==0 {return 0x8900200a;}
        m.put_word(o,p);
        m.put_word(o+4,0);
        m.put_word(o+8,v);
    }
    rc
}

/// fn_196830. The iterator close: the map's heap releases the handle.
fn fn_196830(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,p:u32)->u32 {
    if p==0 {return 0x89002007;}
    crate::boot_part_e::release(m,m.word(m.word(p)+0x14),p,h);
    0
}

/// fn_1968cc. The iterator step close: the current node leaves the tree
/// and the handle's reference word is cleared.
fn fn_1968cc(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,p:u32)->u32 {
    if p==0 {return 0x89002007;}
    let r=m.word(p+4);
    if r==0 {return 0x89002000;}
    fn_198f1c(m,h,m.word(p)+0x18,r);
    m.put_word(p+4,0);
    0
}

/// fn_198f1c. The tree node close: the node leaves the tree, the map's key
/// and value destructors run and the heap releases the node.
fn fn_198f1c(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,o:u32,a:u32) {
    crate::normalization::rb_erase(m.bytes,o,a);
    let heap=m.word(o+0x20);
    let vt=m.word(o+0x24);
    let key=m.word(a+0x10);
    if key!=0 {h.call(m,m.word(vt+4),&[key,heap]);}
    let value=m.word(a+0x14);
    if value!=0 {h.call(m,m.word(vt+0x10),&[value,heap]);}
    crate::boot_part_e::release(m,heap,a,h);
}

/// fn_198fb4. The tree destroy of one subtree.
fn fn_198fb4(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,c:u32,n:u32,end:u32) {
    if n==end {return;}
    fn_198fb4(m,h,c,m.word(n),end);
    fn_198fb4(m,h,c,m.word(n+4),end);
    let heap=m.word(c+0x20);
    let vt=m.word(c+0x24);
    let key=m.word(n+0x10);
    if key!=0 {h.call(m,m.word(vt+4),&[key,heap]);}
    let value=m.word(n+0x14);
    if value!=0 {h.call(m,m.word(vt+0x10),&[value,heap]);}
    crate::boot_part_e::release(m,heap,n,h);
}

/// fn_19869c. The tree destroy of the whole map.
fn fn_19869c(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,t:u32) {
    let n=m.word(t);
    if n!=t {
        fn_198fb4(m,h,t,m.word(n),t);
        fn_198fb4(m,h,t,m.word(n+4),t);
        let heap=m.word(t+0x20);
        let vt=m.word(t+0x24);
        let key=m.word(n+0x10);
        if key!=0 {h.call(m,m.word(vt+4),&[key,heap]);}
        let value=m.word(n+0x14);
        if value!=0 {h.call(m,m.word(vt+0x10),&[value,heap]);}
        crate::boot_part_e::release(m,heap,n,h);
    }
    m.put_word(t+0x18,0);
    m.put_word(t,t);
    m.put_word(t+4,t);
}

/// fn_19665c. The map close: the tree goes first, then the map itself.
fn fn_19665c(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,p:u32)->u32 {
    if p==0 {return 0x89002007;}
    fn_19869c(m,h,p+0x18);
    crate::boot_part_e::release(m,m.word(p+0x14),p,h);
    0
}

/// fn_19672c. Remove one key from the map.
fn fn_19672c(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,p:u32,q:u32)->u32 {
    if p==0||q==0 {return 0x89002007;}
    let t=tree_find(m,h,p+0x18,q);
    if t==0 {0x89002014} else {
        fn_198f1c(m,h,p+0x18,t);
        0
    }
}

/// fn_17b1c4. Close one named resource of the registry: the entry leaves
/// the map, its own destructor runs and the lock is released.
fn fn_17b1c4(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,o:u32,name:u32,s:u32)->u32 {
    if o==0||name==0 {return 0x8b602007;}
    let lock=m.word(o+0xc);
    let mut rc=destructor_lock(m,h,lock,8);
    if rc>>31!=0 {return rc;}
    let map=m.word(o+0x10);
    let find=fn_1966ac(m,h,map,name,s);
    if find>>31!=0 {
        rc=0x8b602000|((find<<19)>>19);
    } else {
        let e=m.word(s);
        let refs=m.half(e+0x1c);
        if refs==1 {
            let cb2=m.word(e+0x18);
            if cb2==0 {
                m.put_half(e+0x1c,0);
            } else {
                let mut image=[0u8;32];
                image.copy_from_slice(&m.bytes[m.offset(e)..m.offset(e)+32]);
                let at=m.offset(s+DS_COPY);
                m.bytes[at..at+32].copy_from_slice(&image);
                rc=fn_19672c(m,h,map,name);
                h.call(m,cb2,&[m.word(o),m.word(o+4),s+DS_COPY]);
            }
        } else if refs==0 {
            rc=0x8b602011;
        } else {
            m.put_half(e+0x1c,refs-1);
        }
    }
    let unlock=destructor_lock(m,h,lock,12);
    if unlock>>31!=0&&rc>>31==0 {rc=unlock;}
    rc
}

/// fn_26c858. The engine instance teardown.
///
/// The port is public so the boot-port-2c lane can bind it as the pipeline
/// release of fn_26caec: the ARM's fail path calls it with the object
/// fn_26caec allocated.
pub fn fn_26c858(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,p:u32,s:&DestructorStrings,scratch:u32)->u32 {
    let mut err=0u32;
    let tab=m.word(p+0xc);
    if tab!=0 {
        let count=m.half(tab+8) as u32;
        for i in 0..count {
            let slot=m.word(m.word(tab+4)+i*4);
            let vt=m.word(slot+8);
            let rc=h.call(m,m.word(vt+0x38),&[m.word(slot+0xc),m.word(slot+0x10)]);
            if rc>>31!=0&&err>>31==0 {err=rc;}
        }
    }
    if m.word(p+0x3c)!=0 {
        h.call(m,0x179f3c,&[m.word(m.word(p)+0xc)]);
        m.put_word(p+0x3c,0);
    }
    let current=(m.word(D_8D7660),m.word(D_8D7660+4));
    let pair=(m.word(p+0x2c),m.word(p+0x30));
    if pair!=current {
        let f14=m.word(p+0x14);
        let rc=h.call(m,m.word(m.word(f14)+0x24),&[pair.0,pair.1]);
        let now=(m.word(D_8D7660),m.word(D_8D7660+4));
        m.put_word(p+0x2c,now.0);
        m.put_word(p+0x30,now.1);
        if rc>>31!=0&&err>>31==0 {err=rc;}
    }
    if m.word(p+0x1c)!=0 {
        let f14=m.word(p+0x14);
        let rc=h.call(m,m.word(m.word(f14)+0x14),&[m.word(p+0x1c),m.word(p+0x20)]);
        if rc>>31!=0&&err>>31==0 {err=rc;}
    }
    if m.word(p+0x24)!=0 {
        let f18=m.word(p+0x18);
        let rc=h.call(m,m.word(m.word(f18)+0x14),&[m.word(p+0x24),m.word(p+0x28)]);
        if rc>>31!=0&&err>>31==0 {err=rc;}
    }
    if tab!=0 {
        let rc=h.call(m,0x2c66c8,&[tab]);
        if rc>>31!=0 {err=rc;}
        m.put_word(p+0xc,0);
    }
    let env=m.word(p);
    let res=m.word(env+0x18);
    if m.word(p+0x18)!=0 {
        let rc=fn_17b1c4(m,h,res,s.lingdb,scratch);
        if rc>>31!=0 {err=rc;}
        m.put_word(p+0x18,0);
    }
    if m.word(p+0x14)!=0 {
        let rc=fn_17b1c4(m,h,res,s.synthstream,scratch);
        if rc>>31!=0 {err=rc;}
        m.put_word(p+0x14,0);
    }
    if m.word(p+0x10)!=0 {
        let rc=h.call(m,0x2c7f24,&[m.word(p+4),m.word(p+8),m.word(p+0x10)]);
        if rc>>31!=0 {err=rc;}
        h.call(m,0x196c7c,&[m.word(p+0x10)]);
        m.put_word(p+0x10,0);
    }
    if res!=0 {
        let rc=h.call(m,0x17af38,&[res]);
        m.put_word(env+0x18,0);
        if rc>>31!=0&&err>>31==0 {err=rc;}
    }
    let owner=m.word(env+0x14);
    if owner!=0 {
        let rc=h.call(m,0x174e00,&[owner]);
        m.put_word(env+0x14,0);
        if rc>>31!=0&&err>>31==0 {err=rc;}
    }
    let heap=m.word(env+4);
    if m.word(p+4)!=0 {
        let rc=h.call(m,0x196238,&[m.word(p+4),m.word(p+8)]);
        if rc>>31!=0&&err>>31==0 {err=rc;}
    }
    if m.word(p+0x2d0)!=0 {
        crate::boot_part_e::release(m,heap,m.word(p+0x2d0),h);
    }
    crate::boot_part_e::release(m,heap,p,h);
    err
}

/// fn_1ee808. The parameters destructor of the synthesis object: the two
/// parameter callbacks leave the parameters block, the value map is emptied
/// and closed, the three registry entries are closed and the object itself
/// goes back to the heap.
pub fn fn_1ee808(
    m:&mut Memory<'_>,
    p:u32,
    key:u32,
    scratch:u32,
    s:&DestructorStrings,
    h:&mut impl crate::boot_part_e::Host,
)->u32 {
    let b=m.word(D_8F902C+0xbc);
    let f=m.word(D_8F902C+0xc0);
    let rc=crate::boot_port_1::fn_176f94(m,p,key,0xf488,0x54);
    if rc>>31!=0 {return 0x8c902008;}
    if p==0 {return rc;}
    let env=m.word(p);
    let heap=m.word(env+4);
    let res=m.word(env+0x18);
    let params=m.word(env+0x14);
    let _=fn_174f9c(m,h,params,s.extraesclang,[p,b,f],scratch+DS_VALUE);
    let _=fn_174f9c(m,h,params,s.extraesctn,[p,b,f],scratch+DS_VALUE);
    let map=m.word(p+0x4c);
    if map!=0 {
        let iter=scratch+DS_ITER;
        m.put_word(iter,0);
        let rc=fn_196794(m,h,map,0,1,iter);
        if rc>>31==0 {
            let mut ikey=0u32;
            let mut ival=0u32;
            loop {
                let step=crate::frontend::step_iter(m.bytes,Some(m.word(iter)),Some(&mut ikey),Some(&mut ival));
                if step>>31!=0 {break;}
                let _=fn_26c858(m,h,ival,s,scratch+DS_TEARDOWN);
                let _=fn_1968cc(m,h,m.word(iter));
            }
            let _=fn_196830(m,h,m.word(iter));
        }
        let _=fn_19665c(m,h,map);
    }
    if m.word(p+0xc)!=0 {let _=fn_17b1c4(m,h,res,s.synthstream,scratch+DS_ENTRY);}
    if m.word(p+0x18)!=0 {let _=fn_17b1c4(m,h,res,s.fedepes,scratch+DS_ENTRY);}
    if m.word(p+0x24)!=0 {let _=fn_17b1c4(m,h,res,s.fedctlkp,scratch+DS_ENTRY);}
    if m.word(p+0x48)!=0 {crate::boot_part_e::release(m,heap,m.word(p+0x48),h);}
    if m.word(p+0x50)!=0 {crate::boot_part_e::release(m,heap,m.word(p+0x50),h);}
    crate::boot_part_e::release(m,heap,p,h);
    0
}
