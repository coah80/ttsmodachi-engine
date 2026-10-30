//! Language-engine constructor. Addresses and temporary storage belong to the caller.
use crate::boot::{Calls, Memory};
use crate::selection::{arena::Arena, arena_leaf as leaf};

/// Read-only regional data. Strings follow STRING_NAMES; scratch is 4096 disjoint
/// caller-owned bytes, restored on normal return. The ctype table is D_8dc30c.
pub struct Data {
    pub strings: [u32; 32],
    pub callback: [u32; 2],
    pub destroy_callback: [u32; 2],
    pub default_environment: u32,
    pub null_handle: [u8; 8],
    pub ctype: u32,
    pub category: u32,
    pub owner_defaults: [u32; 3],
    pub scratch: u32,
}
pub const STRING_NAMES: [&str;32] = [
    "SYNTHSTREAM","FE_DCTLKP","COMPQUERY","mlset","langcode",
    "D_1cfc08","yes","no","lidcfg","languages","",
    "lidscope","D_1cfbe4","D_8f86c5","D_8f86da","D_8f86ca",
    "lidmode","D_8f86e7","D_8f86f1","D_8f86ff","rejectparam1","rejectparam2",
    "numfallback","enx","fuzzythreshold","fuzzyfallbacklang",
    "setting fuzzy matching threshold to %7lu",
    "setting common fuzzy matching threshold to %7lu",
    "setting fuzzy matching fallback language to %s","","","",
];
fn bad(rc:u32)->bool {rc>=0x80000000}
fn nz(p:u32)->Option<u32>{(p!=0).then_some(p)}
fn strlen(m:&mut Memory<'_>,s:u32)->u32 {crate::boot_port_1::fn_195d90(m,s)}
fn copy(m:&mut Memory<'_>,d:u32,s:u32) {leaf::copy_string(&mut Arena::new(m.base,m.bytes),d,s);}
fn find(m:&mut Memory<'_>,s:u32,c:u32)->u32 {leaf::find_byte(&mut Arena::new(m.base,m.bytes),s,c)}
fn substr(m:&Memory<'_>,a:u32,b:u32)->u32 {
    let a0=m.offset(a);let b0=m.offset(b);
    crate::pronunciation::find_substring(&m.bytes[a0..],&m.bytes[b0..])
        .map_or(0,|i|a+i as u32)
}
fn equal(m:&Memory<'_>,a:u32,b:u32)->bool {
    !crate::pronunciation::caseless_differs(&m.bytes[m.offset(a)..],&m.bytes[m.offset(b)..])
}
fn widen(m:&mut Memory<'_>,d:u32,s:u32) {
    crate::normalization::u8_to_u16_copy(m.bytes,d,s);
}
fn number(m:&Memory<'_>,s:u32)->u32 {
    crate::normalization::atoi_scan(m.bytes,s)
}

/// fn_1cffa4 / EU 1d0160 / KR 1d020c. Indirect allocator, lock and
/// dictionary methods receive the live arena through Calls. Named engine
/// calls are bound below, not delegated as scripted replies.
pub fn fn_1cffa4(m:&mut Memory<'_>,_unused:[u32;2],input:[u32;2],out:u32,d:&Data,h:&mut impl Calls)->u32 {
    assert_eq!(m.base,0,"constructor uses absolute-index existing callees");
    let s=d.scratch;
    let saved:[u8;4096]=core::array::from_fn(|i|m.byte(s+i as u32));
    let mut b=Bindings {data:d,host:h,scratch:s+512};
    let rc=construct(m,input,out,d,&mut b);
    for (i,v) in saved.into_iter().enumerate(){m.put_byte(s+i as u32,v);}
    rc
}
fn construct(m:&mut Memory<'_>,input:[u32;2],out:u32,d:&Data,b:&mut impl Calls)->u32 {
    let s=d.scratch;let t=&d.strings;
    if out==0 {return 0x8bd02007;}
    m.put_word(out,0);m.put_word(out+4,0);
    for off in [0,4,12,16,20,24] {m.put_word(s+off,0);}
    m.put_half(s+8,0);m.put_byte(s+10,0);
    let mut rc=b.call(m,0x19611c,&[input[0],input[1],s]);
    if bad(rc) {return rc;}
    let env=m.word(s);
    let p=b.call(m,0x1963b8,&[m.word(env+4),1,0x358]);
    if p==0 {b.call(m,0x173dc0,&[m.word(env+16),d.category,0xbb80,0]);return 0x8bd0200a;}
    m.put_word(p,env);
    // A single fallible body gives all failure edges the original cleanup.
    rc=(|| {
        for (index,off) in [(0,16),(1,12),(2,28)] {
            let rc=b.call(m,0x17b058,&[m.word(env+24),t[index],s+4]);
            if bad(rc) {
                if index!=2 {return rc;}
                m.put_word(p+28,0);
                for (i,v) in d.null_handle.into_iter().enumerate(){m.put_byte(p+20+i as u32,v);}
            } else {
                let entry=m.word(s+4);m.put_word(p+off,m.word(entry+4));
                if index==1 {m.put_word(p+4,m.word(entry+8));m.put_word(p+8,m.word(entry+12));}
                if index==2 {m.put_word(p+20,m.word(entry+8));m.put_word(p+24,m.word(entry+12));}
            }
        }
        if bad(b.call(m,0x175a1c,&[m.word(env+20),t[3],s+12])) {
            let rc=b.call(m,0x175a1c,&[m.word(env+20),t[4],s+12]);
            if bad(rc){return logged(m,b,m.word(p),d,rc);}
        }
        let val=m.word(s+12);
        let size=strlen(m,val)+1;
        let original=b.call(m,0x196394,&[m.word(env+4),size]);
        m.put_word(p+0x34c,original);
        if original==0 {return 0x8bd0200a;}
        copy(m,original,val);
        let mut i=0u16;
        while (strlen(m,original) as i32)>i as i32 {
            let c=m.byte(original+i as u32);
            let lower=leaf::add_class_bit(&Arena::new(m.base,m.bytes),d.ctype,c as u32,1,0x20);
            m.put_byte(original+i as u32,lower as u8);i=i.wrapping_add(1);
        }
        m.put_byte(p+0x44,0);
        if !bad(b.call(m,0x175a1c,&[m.word(env+20),t[5],s+24])) {
            let prop=m.word(s+24);
            if equal(m,prop,t[6]) {m.put_byte(p+0x44,0);}
            else if equal(m,prop,t[7]) {m.put_byte(p+0x44,1);}
        } else {
            let key=if val!=0 && strlen(m,val)>3 {m.put_byte(p+0x44,1);t[7]} else {t[6]};
            let rc=b.call(m,0x1759a0,&[m.word(env+20),t[5],key]);
            if bad(rc){return logged(m,b,m.word(p),d,rc);}
        }
        let rc=register(m,b,m.word(env+20),t[5],p,d);
        if bad(rc){return logged(m,b,m.word(p),d,rc);}
        for (model,which) in [(p+0x48,0xffffu16),(p+0x60,1u16)] {
            m.put_half(s+8,which);
            let rc=identify(m,b,p,d,t[9]);
            if bad(rc) && which==0xffff {return rc;}
            if bad(rc) || m.half(s+8)==0 {
                m.put_half(model+2,0);m.put_word(model+4,0);continue;
            }
            let list=m.word(m.word(s+20));let comma=find(m,list,m.byte(s+10) as u32);
            if comma!=0 {m.put_byte(comma,0);}
            m.put_half(model+2,1);
            let mut comma=find(m,list,b',' as u32);
            while comma!=0 {
                m.put_half(model+2,m.half(model+2).wrapping_add(1));
                comma=find(m,comma+1,b',' as u32);
            }
            let a=b.call(m,0x1963b8,&[m.word(env+4),m.half(model+2) as u32,26]);
            m.put_word(model+4,a);
            if a==0 {return logged(m,b,m.word(p),d,0x8bd0200a);}
            let mut tok=list;let mut comma=find(m,tok,b',' as u32);
            if comma!=0 {m.put_byte(comma,0);}
            let mut i=0u16;
            while i<m.half(model+2) {
                let e=m.word(model+4)+i as u32*26;
                copy(m,e,tok);widen(m,e+8,tok);m.put_byte(e+24,0);m.put_byte(tok+2,0);
                let found=if val==0 {0}else{substr(m,val,tok)};
                m.put_byte(e+25,(val!=0 && found!=0) as u8);
                if val!=0 && found!=0 {crate::render_part_b::fn_176354(m,e+8,found,3);}
                if comma!=0 {
                    tok=comma+1;comma=find(m,tok,b',' as u32);
                    if comma!=0 {m.put_byte(comma,0);}
                }
                i=i.wrapping_add(1);
            }
        }
        if m.half(p+0x4a)==0 && m.half(p+0x62)==0 {return 0;}
        m.put_byte(p+0x43,0);m.put_word(p+0x98,0x10dd5);
        let n=core::cmp::max(m.half(p+0x4a),m.half(p+0x62)) as u32;
        let voices=b.call(m,0x1963b8,&[m.word(env+4),1,n*8]);m.put_word(p+0x340,voices);
        if voices==0 {return logged(m,b,m.word(p),d,0x8bd0200a);}
        let rc=b.call(m,0x245a08,&[p,t[8]]);
        if bad(rc){return logged(m,b,m.word(p),d,rc);}
        if m.word(p+28)!=0 {
            m.put_word(s+16,0);
            query(m,b,p,t[10],0,s+16);
            if m.word(s+16)!=0 {
                let qbuf=b.call(m,0x1963b8,&[m.word(env+4),m.word(s+16),86]);
                if qbuf==0 {
                    b.call(m,0x173dc0,&[m.word(env+16),d.category,0xbb80,0]);
                    return rc;
                }
                let rc=query(m,b,p,t[10],qbuf,s+16);
                if bad(rc){return rc;}
                let mut i=0u16;
                while (i as u32)<m.word(s+16) {
                    let name=qbuf+i as u32*86+64;
                    let mut j=0u16;
                    while m.byte(name+j as u32)!=0 {
                        m.put_byte(name+j as u32,crate::normalization::tolower_240ba0(m.byte(name+j as u32) as u32) as u8);
                        j=j.wrapping_add(1);
                    }
                    let mut e=b.call(m,0x1cfc18,&[p,name,1]);
                    if e==0 {copy(m,s+28,name);m.put_byte(s+30,b'x');e=b.call(m,0x1cfc18,&[p,s+28,1]);}
                    if e!=0 {m.put_byte(e+24,1);}
                    e=b.call(m,0x1cfc18,&[p,name,0]);
                    if e==0 {copy(m,s+28,name);m.put_byte(s+30,b'x');e=b.call(m,0x1cfc18,&[p,s+28,1]);}
                    if e!=0 {m.put_byte(e+24,1);}
                    i=i.wrapping_add(1);
                }
                b.call(m,0x196428,&[m.word(env+4),qbuf]);
            }
        }
        m.put_half(s+8,0xffff);
        let rc=identify(m,b,p,d,t[11]);if bad(rc){return rc;}
        if m.half(s+8)==0 {
            m.put_half(s+8,1);let rc=identify(m,b,p,d,t[11]);if bad(rc){return rc;}
        }
        if m.half(s+8)!=0 {m.put_byte(p+0x43,if substr(m,m.word(m.word(s+20)),t[14])!=0 {1}else{2});}
        if !bad(b.call(m,0x175a1c,&[m.word(env+20),t[12],s+24])) {
            let rc=b.call(m,0x1cfce8,&[p,m.word(s+24)]);if bad(rc){return rc;}
        } else {let rc=b.call(m,0x1759a0,&[m.word(env+20),t[12],t[10]]);if bad(rc){return logged(m,b,m.word(p),d,rc);}}
        let rc=register(m,b,m.word(env+20),t[12],p,d);if bad(rc){return logged(m,b,m.word(p),d,rc);}
        if !bad(b.call(m,0x175a1c,&[m.word(env+20),t[11],s+24])) {
            let prop=m.word(s+24);
            for (value,key) in [(0,13),(1,14),(2,15)] {if equal(m,prop,t[key]) {m.put_byte(p+0x43,value);break;}}
        } else {
            let key=match m.byte(p+0x43) {0=>13,1=>14,_=>15};
            let rc=b.call(m,0x1759a0,&[m.word(env+20),t[11],t[key]]);if bad(rc){return logged(m,b,m.word(p),d,rc);}
        }
        let rc=register(m,b,m.word(env+20),t[11],p,d);if bad(rc){return logged(m,b,m.word(p),d,rc);}
        let rc=b.call(m,0x1756d0,&[m.word(env+20),t[4],p+0x350,0]);if bad(rc){return rc;}
        for off in [0x8c,0x80,0xb6] {m.put_byte(p+off,0);}
        m.put_half(s+8,0xffff);let rc=identify(m,b,p,d,t[16]);if bad(rc){return rc;}
        if m.half(s+8)!=0 && substr(m,m.word(m.word(s+20)),t[17])!=0 {m.put_byte(p+0x80,1);}
        m.put_half(s+8,1);
        if !bad(identify(m,b,p,d,t[16])) {
            if m.half(s+8)!=0 && substr(m,m.word(m.word(s+20)),t[17])!=0 {m.put_byte(p+0x8c,1);}
            if m.half(s+8)!=0 && substr(m,m.word(m.word(s+20)),t[19])!=0 {m.put_byte(p+0xb6,1);}
        }
        if !bad(b.call(m,0x175a1c,&[m.word(env+20),t[16],s+24])) {
            let prop=m.word(s+24);
            for (key,r,bias) in [(17,1,0),(18,0,0),(19,0,1)] {
                if equal(m,prop,t[key]) {m.put_byte(p+0x80,r);m.put_byte(p+0x8c,r);m.put_byte(p+0xb6,bias);break;}
            }
        } else {
            let key=if m.byte(p+0x80)!=0 || m.byte(p+0x8c)!=0 {17}else if m.byte(p+0xb6)!=0 {19}else{18};
            let rc=b.call(m,0x1759a0,&[m.word(env+20),t[16],t[key]]);if bad(rc){return logged(m,b,m.word(p),d,rc);}
        }
        let rc=register(m,b,m.word(env+20),t[16],p,d);if bad(rc){return logged(m,b,m.word(p),d,rc);}
        for off in [0x80,0x8c] {m.put_word(p+off+8,50);m.put_word(p+off+4,0);}
        for (which,key,off,required) in [(0xffff,20,0x84,true),(0xffff,21,0x88,true),(1,20,0x90,false),(1,21,0x94,false)] {
            m.put_half(s+8,which);let rc=identify(m,b,p,d,t[key]);
            if bad(rc) {if required{return rc;}} else if m.half(s+8)!=0 {m.put_word(p+off,number(m,m.word(m.word(s+20))) as u16 as u32);}
        }
        m.put_byte(p+0xb5,0);m.put_byte(p+0xb4,0);
        for (which,off,required) in [(0xffff,0xb4,true),(1,0xb5,false)] {
            m.put_half(s+8,which);let rc=identify(m,b,p,d,t[22]);
            if bad(rc) {if required{return rc;}} else if m.half(s+8)!=0 {m.put_byte(p+off,1);}
        }
        for (enabled,threshold,lang) in [(0x9c,0xa0,0xa4),(0xa8,0xac,0xb0)] {
            m.put_byte(p+enabled,0);m.put_word(p+threshold,1000);
            leaf::copy_bytes(&mut Arena::new(m.base,m.bytes),p+lang,t[23],4);
        }
        for (which,enabled,threshold,format,required) in [(0xffff,0x9c,0xa0,26,true),(1,0xa8,0xac,27,false)] {
            m.put_half(s+8,which);let rc=identify(m,b,p,d,t[24]);
            if bad(rc) {if required{return rc;}} else if m.half(s+8)!=0 {
                m.put_byte(p+enabled,1);m.put_word(p+threshold,number(m,m.word(m.word(s+20))));
                b.call(m,0x1738e8,&[m.word(env+16),d.category,5,0,t[format],m.word(p+threshold)]);
            }
        }
        for (which,required) in [(0xffff,true),(1,false)] {
            m.put_half(s+8,which);let rc=identify(m,b,p,d,t[25]);
            if bad(rc) {if required{return rc;}} else if m.half(s+8)!=0 {
                let text=m.word(m.word(s+20));let n=strlen(m,text);
                // Both calls intentionally update the primary fields, as in ROM.
                m.put_byte(p+0x9c,1);leaf::copy_bytes(&mut Arena::new(m.base,m.bytes),p+0xa4,text,4);
                if n>3 {m.put_byte(p+0xa7,0);}
                b.call(m,0x1738e8,&[m.word(env+16),d.category,5,0,t[28],p+0xa4]);
            }
        }
        0
    })();
    m.put_word(out,p);m.put_word(out+4,0xf3c0);
    if bad(rc) {b.call(m,0x1cf9d0,&[p,0xf3c0]);m.put_word(out,0);m.put_word(out+4,0);}
    rc
}
fn logged(m:&mut Memory<'_>,b:&mut impl Calls,env:u32,d:&Data,rc:u32)->u32 {
    b.call(m,0x173dc0,&[m.word(env+16),d.category,0xbb80,0]);rc
}
fn register(m:&mut Memory<'_>,b:&mut impl Calls,props:u32,key:u32,p:u32,d:&Data)->u32 {
    b.call(m,0x174e74,&[props,key,p,d.callback[0],d.callback[1]])
}
fn identify(m:&mut Memory<'_>,b:&mut impl Calls,p:u32,d:&Data,key:u32)->u32 {
    b.call(m,m.word(m.word(p+12)+28),&[m.word(p+4),m.word(p+8),d.strings[8],key,d.scratch+20,d.scratch+8,d.scratch+10])
}
fn query(m:&mut Memory<'_>,b:&mut impl Calls,p:u32,name:u32,buf:u32,count:u32)->u32 {
    b.call(m,m.word(m.word(p+28)+32),&[m.word(p+20),m.word(p+24),name,buf,count])
}
struct Bindings<'a,H> {data:&'a Data,host:&'a mut H,scratch:u32}
impl<H:Calls> Calls for Bindings<'_,H> {
    fn call_wide(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u64 {self.host.call_wide(m,f,a)}
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
        let s=self.scratch;self.scratch+=512;
        let rc=match f {
            0x19611c=>crate::boot_port_1::fn_19611c(m,[a[0],a[1]],a[2],self.data.null_handle,self.data.default_environment),
            0x1963b8=>crate::boot_part_e::calloc(m,a[0],a[1],a[2],&mut Indirect(self)),
            0x1759a0|0x1966ac|0x198760|0x196980|0x176f2c|0x176f5c=>m.boot_parameter_call(self,f,a,s).expect("bound engine call"),
            0x196394=>crate::boot_part_e::alloc(m,a[0],a[1],&mut Indirect(self)),
            0x196428=>{crate::boot_part_e::release(m,a[0],a[1],&mut Indirect(self));0},
            0x1963e0=>crate::boot_part_e::fn_1963e0(m,a[0],a[1],a[2],&mut Indirect(self)),
            0x17b058=>crate::render_part_c::fn_17b058_m(m,a[0],a[1],a[2],&mut Indirect(self)),
            0x17b1c4=>crate::render_part_c::registry_release_m(m,a[0],a[1],s,&mut Indirect(self)),
            0x174f9c=>crate::boot_part_b::fn_174f9c(m,a[0],a[1],[a[2],a[3],a[4]],&mut Output{h:self,s}),
            0x1970ac=>crate::boot_part_e::fn_1970ac(m,a[0],a[1],a[2]),
            0x196e28=>{
                assert_eq!(m.word(a[0]+12),0,"use live destructor binding for active vectors");
                crate::frontend::remove_at(m.bytes,nz(a[0]),a[1],|_,_|unreachable!())
            }
            0x1cf868|0x245a90=>{
                let mut regions=[crate::engine::Region{base:m.base,bytes:m.bytes}];
                crate::engine_components2::read_call(&mut crate::engine::Memory::new(&mut regions).unwrap(),f,a,&mut Component(self)).unwrap()
            }
            0x175170=>crate::speech_part_1::parameter_call_m(m,f,a,s,&mut Indirect(self)),
            0x174e74=>crate::boot_live::register_owner_m(m,self,a[0],a[1],[a[2],a[3],a[4]],self.data.owner_defaults,s),
            0x175a1c|0x1756d0=>lookup(m,self,f,a),
            0x1cf9d0|0x1cfce8|0x1cfc18=>{let globals=self.data.destroy_callback;component(m,self,f,a,s,globals)},
            0x245a08=>models(m,self,a[0],a[1],s),
            0x173dc0=>{
                for (i,&v) in a[4..].iter().enumerate(){m.put_word(s+i as u32*4,v);}
                crate::render_part_a::error_log(m,&mut Output{h:self,s:s+32},a[0],a[1],a[2],a[3],s,s+64)
            }
            0x1738e8=>{
                for (i,&v) in a[5..].iter().enumerate(){m.put_word(s+i as u32*4,v);}
                crate::log_live::fn_1738e8(m,a[0],a[1],a[2],a[3],a[4],s,&mut Indirect(self))
            }
            0x195ae8=>leaf::copy_bytes(&mut Arena::new(m.base,m.bytes),a[0],a[1],a[2]),
            _=>self.host.call(m,f,a),
        };
        self.scratch=s;rc
    }
}
struct Lock<'a,'b,H>{h:&'a core::cell::RefCell<&'b mut H>,p:u32}
impl<H:Calls> crate::reentrant2::StateLockM for Lock<'_, '_,H> {
    fn acquire_m(&mut self,mem:&mut [u8])->u32{self.h.borrow_mut().call(&mut Memory{base:0,bytes:mem},0x176f2c,&[self.p])}
    fn release_m(&mut self,mem:&mut [u8])->u32{self.h.borrow_mut().call(&mut Memory{base:0,bytes:mem},0x176f5c,&[self.p])}
}
fn lookup(m:&mut Memory<'_>,h:&mut impl Calls,f:u32,a:&[u32])->u32 {
    let p=if a[0]==0 {0}else{m.word(a[0]+4)};
    let map=if a[0]==0 {0}else{m.word(a[0]+8)};
    let mut value=if a[2]==0 {0}else{m.word(a[2])};
    let mut size=if f==0x1756d0 && a[3]!=0 {m.word(a[3])}else{0};
    // The original clears guest outputs before taking the lock.
    if a[0]!=0 && a[1]!=0 && a[2]!=0 {
        m.put_word(a[2],0);
        if f==0x1756d0 && a[3]!=0 {m.put_word(a[3],0);}
    }
    let shared=core::cell::RefCell::new(h);
    let rc=crate::reentrant2::lookup_param_m(m.bytes,nz(a[0]),a[1],
        (a[2]!=0).then_some(&mut value),
        (f==0x1756d0 && a[3]!=0).then_some(&mut size),&mut Lock{h:&shared,p},
        |x,y,mem|{
            let mut m=Memory{base:0,bytes:mem};
            let cmp=m.word(m.word(map+60)+8);
            shared.borrow_mut().call(&mut m,cmp,&[x,y]) as i32
        });
    if a[2]!=0 {m.put_word(a[2],value);}
    if f==0x1756d0 && a[3]!=0 {m.put_word(a[3],size);}
    rc
}
struct Component<'a,H>(&'a mut H);
impl<H:Calls> crate::engine_components::InternalCalls for Component<'_,H> {
    fn call(&mut self,m:&mut crate::engine::Memory<'_, '_>,f:u32,a:&[u32])->Result<u32,crate::engine::Error> {
        let region=&mut m.regions[0];
        Ok(self.0.call(&mut Memory {base:region.base,bytes:region.bytes},f,a))
    }
}
fn component(m:&mut Memory<'_>,h:&mut impl Calls,f:u32,a:&[u32],s:u32,globals:[u32;2])->u32 {
    fn run(m:&mut crate::engine::Memory<'_, '_>,h:&mut impl Calls,f:u32,a:&[u32],s:u32)->u32 {
        if f==0x1cfc18 {
            use crate::engine_components::InternalCalls;
            crate::engine_components2::ReadCalls.call(m,f,a).unwrap()
        } else {crate::engine_components2::leaf(m,f,a,s,&mut Component(h)).unwrap()}
    }
    if m.base as u64<=0x8f884c && m.base as u64+m.bytes.len() as u64>=0x8f8854 {
        let mut regions=[crate::engine::Region{base:m.base,bytes:m.bytes}];
        return run(&mut crate::engine::Memory::new(&mut regions).unwrap(),h,f,a,s);
    }
    let mut global=[0u8;8];
    global[..4].copy_from_slice(&globals[0].to_le_bytes());global[4..].copy_from_slice(&globals[1].to_le_bytes());
    let mut regions=[crate::engine::Region{base:m.base,bytes:m.bytes},crate::engine::Region{base:0x8f884c,bytes:&mut global}];
    run(&mut crate::engine::Memory::new(&mut regions).unwrap(),h,f,a,s)
}

struct ModelHeap<'a,'b,H>{m:&'a mut Memory<'b>,h:&'a core::cell::RefCell<&'a mut H>,allocator:u32}
impl<H:Calls> crate::heap::Heap for ModelHeap<'_, '_,H> {
    fn allocate(&mut self,n:u32)->Option<u32>{nz(self.h.borrow_mut().call(self.m,0x196394,&[self.allocator,n]))}
    fn allocate_zeroed(&mut self,n:u32,z:u32)->Option<u32>{nz(self.h.borrow_mut().call(self.m,0x1963b8,&[self.allocator,n,z]))}
    fn reallocate(&mut self,p:u32,n:u32)->Option<u32>{nz(self.h.borrow_mut().call(self.m,0x1963e0,&[self.allocator,p,n]))}
    fn release(&mut self,p:u32){self.h.borrow_mut().call(self.m,0x196428,&[self.allocator,p]);}
    fn arena(&mut self)->&mut [u8]{self.m.bytes}
}
struct ModelLookup<'a,H>{h:&'a core::cell::RefCell<&'a mut H>,s:u32}
impl<H:Calls> crate::render_part_a::LanguageLog for ModelLookup<'_,H> {
    fn message(&mut self,_:&mut Arena<'_>,_:u32,_:crate::render_part_a::LanguageText<'_>){unreachable!("model initializer does not log messages")}
    fn value(&mut self,_:&mut Arena<'_>,_:u32,_:u32,_:u32){unreachable!("model initializer does not log values")}
    fn allocation_error(&mut self,a:&mut Arena<'_>,logger:u32) {
        self.h.borrow_mut().call(&mut Memory{base:a.base,bytes:a.bytes},0x173dc0,&[logger,0x8f87bc,0xbb80,0]);
    }
}
impl<H:Calls> crate::render_part_a::LanguageLookup for ModelLookup<'_,H> {
    fn identify(&mut self,a:&mut Arena<'_>,method:u32,instance:[u32;2],set:u32,key:&[u8],which:u16)->(u32,u32,u16,u8) {
        let s=self.s;
        let saved:[u8;128]=core::array::from_fn(|i|a.u8(s+i as u32));
        a.set_u32(s,0);a.set_u16(s+4,which);a.set_u8(s+6,0);
        for (i,&v) in key.iter().enumerate(){a.set_u8(s+8+i as u32,v);}
        a.set_u8(s+8+key.len() as u32,0);
        let rc=self.h.borrow_mut().call(&mut Memory{base:a.base,bytes:a.bytes},method,&[instance[0],instance[1],set,s+8,s,s+4,s+6]);
        let out=(rc,a.u32(s),a.u16(s+4),a.u8(s+6));
        for (i,v) in saved.into_iter().enumerate(){a.set_u8(s+i as u32,v);}
        out
    }
}
fn models(m:&mut Memory<'_>,h:&mut impl Calls,p:u32,set:u32,s:u32)->u32 {
    let allocator=m.word(m.word(p)+4);
    let shared=core::cell::RefCell::new(h);
    let mut lookup=ModelLookup {h:&shared,s};
    let mut heap=ModelHeap {m,h:&shared,allocator};
    crate::render_part_a::initialize_language_models(&mut heap,&mut lookup,0,p,set)
}
struct Indirect<'a,H>(&'a mut H);
impl<H:Calls> crate::boot_part_e::Host for Indirect<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.0.call(m,f,a)}
}

impl<H:Calls> crate::render_part_c::ParserMethodsM for Indirect<'_,H> {
    fn method_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32{self.0.call(m,f,a)}
}
struct Output<'a,H>{h:&'a mut H,s:u32}
impl<H:Calls> crate::boot_part_b::Host for Output<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.h.call(m,f,a)}
    fn thread_id(&mut self)->u32{unreachable!("unregistration has no CP15 reads")}
}
impl<H:Calls> crate::boot_part_b::OutputHost for Output<'_,H> {
    fn output(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->(u32,u32) {
        let saved=m.word(self.s);m.put_word(self.s,0);
        let rc=self.h.call(m,f,&[a[0],a[1],self.s]);let value=m.word(self.s);m.put_word(self.s,saved);(rc,value)
    }
}

impl<H:Calls> crate::speech_part_1::Host for Indirect<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.0.call(m,f,a)}
    fn get(&mut self,_:&mut Memory<'_>,f:u32,_:u32,_:u32,_:u32)->(u32,u32,u32){panic!("unexpected stream call {f:x} in property setter")}
}
impl<H:Calls> crate::boot_part_b::FormatHost for Output<'_,H> {}
impl<H:Calls> crate::log_live::LogMethodsM for Indirect<'_,H> {
    fn log_m(&mut self,f:u32,o:u32,c:u32,l:u32,a:u32,fmt:u32,va:u32,m:&mut Memory<'_>)->u32 {
        self.0.call(m,f,&[o,c,l,a,fmt,va])
    }
}
