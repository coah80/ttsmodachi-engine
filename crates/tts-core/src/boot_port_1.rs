//! Boot-path ports for 11cc0c, lane 1.
use crate::boot::Memory;

/// Store the supplied value through the output pointer.
pub fn fn_177028(m: &mut Memory<'_>, value: u32, _unused: u32, out: u32) -> u32 {
    m.put_word(out, value);
    0
}

pub fn fn_177038(m: &mut Memory<'_>, value: u32, _unused: u32, out: u32) -> u32 {
    m.put_word(out, value); 0
}
pub fn fn_1852d8(m: &mut Memory<'_>, p: u32, value: u32, _unused: u32) -> u32 {
    let at=p.wrapping_add(0x34); let old=half(m,at); m.put_half(at,value as u16); old
}
pub fn fn_1852e8(m: &mut Memory<'_>, p: u32, value: u32, _unused: u32) -> u32 {
    let at=p.wrapping_add(0x36); let old=half(m,at); m.put_half(at,((old&!1)|(value&1)) as u16); old&1
}
pub fn fn_1853c4(m: &mut Memory<'_>, p: u32, value: u32, _unused: u32) -> u32 {
    let at=m.offset(p.wrapping_add(0x38)); let old=m.bytes[at] as i8 as i32 as u32; m.bytes[at]=value as u8; old
}
pub fn fn_1aa838(m: &mut Memory<'_>, p: u32, _a: u32, _b: u32) -> u32 { m.word(p.wrapping_add(0x18)) }
pub fn fn_1aa840(m: &mut Memory<'_>, p: u32, _a: u32, _b: u32) -> u32 { m.word(p.wrapping_add(0x1c)) }
pub fn fn_1b3138(m: &mut Memory<'_>, p: u32, _a: u32, _b: u32) -> u32 { p }
pub fn fn_1b3210(m: &mut Memory<'_>, p: u32, _a: u32, _b: u32) -> u32 { m.word(p.wrapping_add(0x28)) }
pub fn fn_1b4528(m: &mut Memory<'_>, p: u32, _a: u32, _b: u32) {let at=p.wrapping_add(4);m.put_word(at,m.word(at).wrapping_add(1));}
fn half(m:&Memory<'_>,p:u32)->u32 {let at=m.offset(p);u16::from_le_bytes(m.bytes[at..at+2].try_into().unwrap()) as u32}

/// The stream destructor's empty base has no state.
pub fn fn_1ba3d0(m: &mut Memory<'_>, _object: u32) -> u32 { let _ = m; 0 }
pub fn fn_1b44c8(m: &mut Memory<'_>) -> u32 { fn_1ba3d0(m, 0) }
/// Return the regional interface table for supported versions.
pub fn fn_1cf83c(m: &mut Memory<'_>, version:u32, out:u32, table:u32) -> u32 {
    if version<=1 {m.put_word(out,table);0} else {0x84f02001}
}
pub fn fn_1d4990(m: &mut Memory<'_>, version:u32, out:u32, table:u32) -> u32 {
    if version<=1 {m.put_word(out,table);0} else {0x8bd02001}
}
/// The ROM's fn_1b20dc (0x1b20dc): call the method of the object at p with
/// (context, flag, arg) and write the error flag.  The flag is a byte of the
/// caller's own memory in the ROM, so this form keeps it out of the guest
/// arena; fn_1b20dc below keeps the guest address form.
pub fn fn_1b20dc_flag(m: &mut Memory<'_>,p:u32,flag:u32,arg:u32,err:&mut u8,h:&mut impl crate::boot_part_f::Host)->u32 {
    if flag==0 {*err=9;return 0;}
    let rc=h.call(m,m.word(p.wrapping_add(4)),&[m.word(p.wrapping_add(16)),flag,arg]);
    *err=u8::from(rc==0);rc
}
pub fn fn_1b20dc(m: &mut Memory<'_>,p:u32,flag:u32,arg:u32,out:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    let mut err=0u8;
    let rc=fn_1b20dc_flag(m,p,flag,arg,&mut err,h);
    let at=m.offset(out);m.bytes[at]=err;rc
}
pub fn fn_1aa9e0(m: &mut Memory<'_>,p:u32,obj:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    let at=m.offset(p.wrapping_add(0x48));m.bytes[at]=0;
    let vt=m.word(obj);let a=h.call(m,m.word(vt.wrapping_add(0x14)),&[obj.wrapping_sub(m.word(vt.wrapping_add(0x18)))]);
    let vt=m.word(obj);let b=h.call(m,m.word(vt.wrapping_add(0x14)),&[obj.wrapping_sub(m.word(vt.wrapping_add(0x18)))]);
    h.call(m,m.word(m.word(b).wrapping_add(0x10)),&[a])
}
pub fn fn_1ba384(m: &mut Memory<'_>,a:u32,b:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    let rc=h.call(m,m.word(m.word(b).wrapping_add(8)),&[b]);
    if rc==0 {crate::boot_part_f::fn_1b2128(m,a,b,h);}rc
}
pub fn fn_1ba82c(m: &mut Memory<'_>,p:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    let data=m.word(p.wrapping_add(4));
    if data!=0 {crate::boot_part_f::fn_1b2128(m,m.word(p.wrapping_add(0x14)),data,h);m.put_word(p.wrapping_add(4),0);}
    fn_1ba3d0(m,p)
}
pub fn fn_1b2cdc(m: &mut Memory<'_>,p:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    crate::boot_part_f::fn_1b2128(m,m.word(p.wrapping_add(0x44)),m.word(p.wrapping_add(0x18)),h);
    fn_1b44c8(m)
}
pub fn fn_1b350c(m: &mut Memory<'_>,p:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    let mut index=0u32;
    while index<m.word(m.word(p.wrapping_add(8)).wrapping_add(8)) {
        let table=m.word(p.wrapping_add(8));
        assert_eq!(m.base,0,"table_entry uses zero-based guest arenas");
        let buffer=crate::normalization::table_entry(m.bytes,table,index);
        crate::boot_part_f::fn_1b2128(m,m.word(p.wrapping_add(4)),buffer,h);
        index=index.wrapping_add(1);
    }
    let rc=fn_1ba384(m,m.word(p.wrapping_add(4)),m.word(p.wrapping_add(8)),h);
    if rc==0 {fn_1ba3d0(m,p)} else {rc}
}

struct Allocator<'a,H> { method:u32, host:&'a mut H }
impl<H:crate::boot_part_f::Host> crate::frontend::HeapVtableM for Allocator<'_,H> {
    fn alloc_at_m(&mut self,obj:u32,size:u32,bytes:&mut [u8])->u32 {
        self.host.call(&mut Memory{base:0,bytes},self.method,&[obj,size])
    }
    fn realloc_at_m(&mut self,_:u32,_:u32,_:u32,_:&mut [u8])->u32 {panic!("unexpected realloc")}
    fn release_at_m(&mut self,_:u32,_:u32,_:&mut [u8]) {panic!("unexpected release")}
}
fn allocate(m:&mut Memory<'_>,heap:u32,n:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    assert_eq!(m.base,0,"heap_alloc_m uses zero-based guest arenas");
    let method=if heap==0 || n==0 {0} else {m.word(m.word(heap))};
    crate::frontend::heap_alloc_m(m.bytes,if heap==0 {None}else{Some(heap)},n,&mut Allocator{method,host:h})
}
pub fn fn_151774(m:&mut Memory<'_>,src:u32,heap:u32,out:u32,empty:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    let s=m.word(src);
    let n=if s==0 {1}else{crate::normalization::strlen8(m.bytes,s).wrapping_add(1)};
    let p=allocate(m,heap,n,h);m.put_word(out,p);
    if p==0 {return 0;}
    crate::normalization::strcpy_at(m.bytes,p,if s==0 {empty}else{s});1
}
pub fn fn_177080(m:&mut Memory<'_>,src:u32,heap:u32,out:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    let p=allocate(m,heap,12,h);if p==0 {return 0x8150600a;}
    crate::normalization::memcpy_at(m.bytes,p,src,12);
    m.put_word(src,0);m.put_word(src.wrapping_add(8),0);m.put_word(out,p);0
}
pub fn fn_1773ec(m:&mut Memory<'_>,src:u32,heap:u32,out:u32,pair:[u32;2],h:&mut impl crate::boot_part_f::Host)->u32 {
    let p=allocate(m,heap,16,h);if p==0 {return 0x8150600a;}
    crate::normalization::memcpy_at(m.bytes,p,src,16);m.put_word(src,0);
    crate::boot_part_d::fn_176fd4(m,src.wrapping_add(8),pair);m.put_word(out,p);0
}
pub fn fn_1850c0(m:&mut Memory<'_>,heap:u32,requested:u32,alignment:i32)->u32 {
    let requested=if requested==0 {1}else{requested};let size=requested.wrapping_add(3)&!3;
    let first_fit=half(m,heap.wrapping_add(0x36))&1==0;
    let list=heap.wrapping_add(0x24);let mut best=0;let mut address=0;let mut best_size=u32::MAX;
    let mut block=m.word(list.wrapping_add(if alignment<0 {4}else{0}));
    let align=if alignment<0 {alignment.wrapping_neg() as u32}else{alignment as u32};let mask=align.wrapping_sub(1);
    while block!=0 {
        let start=block.wrapping_add(16);let available=m.word(block.wrapping_add(4));
        let aligned=if alignment<0 {available.wrapping_add(start).wrapping_sub(size)&!mask}else{start.wrapping_add(align).wrapping_sub(1)&!mask};
        let fits=if alignment<0 {(aligned.wrapping_sub(start) as i32)>=0}else{available>=aligned.wrapping_sub(start).wrapping_add(size)};
        if fits && available<best_size {best=block;best_size=available;address=aligned;if first_fit||best_size==size {break;}}
        block=m.word(block.wrapping_add(if alignment<0 {8}else{12}));
    }
    if best==0 {0}else{fn_19b814(m,list,best,address,size,u32::from(alignment<0))}
}
fn block_init(m:&mut Memory<'_>,p:u32,tag:u16,size:u32) {
    m.put_half(p,tag);m.put_half(p.wrapping_add(2),0);m.put_word(p.wrapping_add(12),0);m.put_word(p.wrapping_add(8),0);m.put_word(p.wrapping_add(4),size);
}
fn block_unlink(m:&mut Memory<'_>,p:u32,list:u32) {
    let prev=m.word(p.wrapping_add(8));let next=m.word(p.wrapping_add(12));
    if prev!=0 {m.put_word(prev.wrapping_add(12),next)}else{m.put_word(list,next)}
    if next!=0 {m.put_word(next.wrapping_add(8),prev)}else{m.put_word(list.wrapping_add(4),prev)}
}
fn block_link(m:&mut Memory<'_>,p:u32,list:u32,before:u32) {
    m.put_word(p.wrapping_add(8),before);
    let next=if before!=0 {let next=m.word(before.wrapping_add(12));m.put_word(before.wrapping_add(12),p);next}else{let next=m.word(list);m.put_word(list,p);next};
    m.put_word(p.wrapping_add(12),next);
    if next!=0 {m.put_word(next.wrapping_add(8),p)}else{m.put_word(list.wrapping_add(4),p)}
}
pub fn fn_19b814(m:&mut Memory<'_>,list:u32,block:u32,address:u32,size:u32,which:u32)->u32 {
    let hdr=address.wrapping_sub(16);let mut effective=hdr;
    let end=block.wrapping_add(16).wrapping_add(m.word(block.wrapping_add(4)));let mut alloc_end=address.wrapping_add(size);
    let mut prev=m.word(block.wrapping_add(8));let start=block.wrapping_sub((half(m,block.wrapping_add(2))<<17)>>25);
    block_unlink(m,block,list);
    let gap=effective.wrapping_sub(start);
    if gap>=20 && (which!=0 || m.byte(list.wrapping_add(0x14))!=0) {block_init(m,start,0x4652,gap.wrapping_sub(16));block_link(m,start,list,prev);prev=start;}else{effective=start;}
    let gap=end.wrapping_sub(alloc_end);
    if gap>=20 && (which!=1 || m.byte(list.wrapping_add(0x14))!=0) {block_init(m,alloc_end,0x4652,gap.wrapping_sub(16));block_link(m,alloc_end,list,prev);}else{alloc_end=end;}
    let gap=alloc_end.wrapping_sub(effective);
    if m.word(list.wrapping_sub(4))&1!=0 {for i in 0..gap/4 {m.put_word(effective.wrapping_add(i.wrapping_mul(4)),0);}}
    block_init(m,hdr,0x5544,alloc_end.wrapping_sub(hdr).wrapping_sub(16));
    let mut flags=half(m,hdr.wrapping_add(2));flags|=which<<15;
    flags=(flags&!0x7f00)|((hdr.wrapping_sub(effective)&0x7f)<<8);m.put_half(hdr.wrapping_add(2),flags as u16);
    flags=(flags&!0xff)|(half(m,list.wrapping_add(0x10))&0xff);m.put_half(hdr.wrapping_add(2),flags as u16);
    let tail=m.word(list.wrapping_add(12));m.put_word(hdr.wrapping_add(8),tail);block_link(m,hdr,list.wrapping_add(8),tail);address
}

pub fn fn_1a8f1c(m:&mut Memory<'_>,p:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    let heap=m.word(p.wrapping_add(68));let rc=fn_1ba384(m,heap,m.word(p.wrapping_add(76)),h);if rc!=0 {return rc;}
    let data=m.word(p.wrapping_add(60));if data!=0 {crate::boot_part_f::fn_1b2128(m,heap,data,h);}
    let sub=m.word(p.wrapping_add(48));if sub!=0 {
        if m.word(p.wrapping_add(64))==0 && m.byte(p.wrapping_add(80))!=0 {
            let rc=h.call(m,m.word(sub.wrapping_add(12)),&[m.word(p.wrapping_add(52))]);
            if rc!=0 {return crate::normalization::code_6(0);}
        }
        crate::boot_part_f::fn_1b2128(m,heap,m.word(p.wrapping_add(48)),h);
    }
    let data=m.word(p.wrapping_add(56));if data!=0 {crate::boot_part_f::fn_1b2128(m,heap,data,h);}
    fn_1b44c8(m)
}

struct Calls<'a,H>(&'a mut H);
impl<H:crate::boot_part_f::Host> crate::boot::Calls for Calls<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,address:u32,args:&[u32])->u32 {self.0.call(m,address,args)}
    fn call_wide(&mut self,_:&mut Memory<'_>,_:u32,_:&[u32])->u64 {panic!("unexpected wide call")}
}
pub fn fn_1966ec(m:&mut Memory<'_>,base:u32,key:u32,out:u32,scratch:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    if base==0 || key==0 || out==0 {return 0x89002007;}
    let tree=base.wrapping_add(24);
    // The boot bridge executes the existing first-hit fn_198760 port.
    // Its duplicate-key continuation is completed here using the same live arena.
    let mut node=m.boot_parameter_call(&mut Calls(h),0x198760,&[tree,key],scratch).expect("bound tree search");
    if node==0 {return 0x89002014;}
    if m.byte(tree.wrapping_add(40))!=0 {
        loop {
            let last=node;node=m.word(last);
            while node!=tree {
                let method=m.word(m.word(tree.wrapping_add(36)).wrapping_add(8));
                if h.call(m,method,&[key,m.word(node.wrapping_add(16))])==0 {break;}
                node=m.word(node.wrapping_add(4));
            }
            if node==tree {node=last;break;}
        }
    }
    m.put_word(out,m.word(node.wrapping_add(20)));0
}

/// Regional stream vtables and their seek entry, supplied by the caller.
pub struct Streams {
    pub parameters: crate::boot_part_f::Environment,
    pub plain: u32,
    pub encoded: u32,
    pub seek: u32,
    pub seek_adjustment: u32,
}
pub fn fn_1b2258(m:&mut Memory<'_>,a:[u32;9],e:&Streams,h:&mut impl crate::boot_part_f::Host)->u32 {
    let p=a[0];let rc=crate::boot_part_f::fn_1b449c(m,p,1,&e.parameters);if rc!=0 {return rc&255;}
    m.put_word(p+16,e.plain.wrapping_add(32));m.put_word(p+12,e.plain);
    m.put_word(p+36,a[2]);m.put_word(p+40,a[3]);m.put_word(p+44,a[4]);m.put_half(p+24,a[7] as u16);
    m.put_word(p+20,a[5]);m.put_word(p+32,a[6]);m.put_word(p+56,a[1]);let at=m.offset(p+60);m.bytes[at]=a[8] as u8;
    m.put_word(p,e.plain.wrapping_sub(24));
    let len=if a[7]==0 {a[6]}else{a[6].wrapping_sub(4u32.wrapping_mul(crate::normalization::udiv(a[6].wrapping_add(a[7]).wrapping_add(3),a[7].wrapping_add(4))))};
    m.put_word(p+28,len);m.put_word(p+48,len);m.put_word(p+52,a[7].min(len));
    crate::speech_part_1::fn_1b2760(m,p.wrapping_add(12).wrapping_sub(e.seek_adjustment),0,&mut Speech(h))&255
}
pub fn fn_1b23bc(m:&mut Memory<'_>,a:[u32;9],e:&Streams,h:&mut impl crate::boot_part_f::Host)->u32 {
    let mut error=0;let p=crate::boot_part_f::fn_1b2140(m,a[0],64,&mut error,h);
    if error==0 {
        error=fn_1b2258(m,[p,a[1],a[2],a[3],a[4],a[5],a[6],a[7],0],e,h) as u8;
        if error==0 {m.put_word(p+8,a[0]);m.put_word(a[8],p);}else{crate::boot_part_f::fn_1b2128(m,a[0],p,h);m.put_word(a[8],0);}
    }
    error as u32
}
pub fn fn_1b3218(m:&mut Memory<'_>,a:[u32;9],e:&Streams,h:&mut impl crate::boot_part_f::Host)->u32 {
    let mut status=0;let p=crate::boot_part_f::fn_1b2140(m,a[0],76,&mut status,h);
    if status==0 {
        let error=crate::boot_part_f::fn_1b449c(m,p,1,&e.parameters);
        if error==0 {
            m.put_word(p,e.encoded.wrapping_sub(24));m.put_word(p+12,e.encoded);m.put_word(p+16,e.encoded.wrapping_add(28));
            m.put_word(p+48,a[2]);m.put_word(p+52,a[3]);let at=m.offset(p+20);m.bytes[at]=a[6] as u8;
            m.put_word(p+24,0);m.put_word(p+28,0x4000);m.put_half(p+36,a[7] as u16);m.put_word(p+32,a[4]);m.put_word(p+44,a[5]);m.put_word(p+68,a[1]);let at=m.offset(p+72);m.bytes[at]=0;
            let block=a[7]&0xffff;
            let len=if block==0 {a[5]}else{
                let len=a[5].wrapping_sub(4u32.wrapping_mul(crate::normalization::udiv(block.wrapping_add(a[5]).wrapping_add(3),block+4)));
                m.put_word(p+64,crate::normalization::adler32(m.bytes,0,0,0));len
            };
            m.put_word(p+40,len);m.put_word(p+56,len);m.put_word(p+60,(half(m,p+36)).min(len));
        }
        status=error as u8;
        if error!=0 {crate::boot_part_f::fn_1b2128(m,a[0],p,h);m.put_word(a[8],0);}else{m.put_word(p+8,a[0]);m.put_word(a[8],p);}
    }
    status as u32
}
struct Speech<'a,H>(&'a mut H);
impl<H:crate::boot_part_f::Host> crate::speech_part_1::Host for Speech<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call(m,f,a)}
    fn get(&mut self,_:&mut Memory<'_>,_:u32,_:u32,_:u32,_:u32)->(u32,u32,u32) {panic!("unexpected stream get")}
}
/// Temporary slots are disjoint from the stream and buffer. Workspace has at least max(length,4) bytes.
pub fn fn_1b313c(m:&mut Memory<'_>,p:u32,buffer:u32,scratch:u32,workspace:&mut [u8],h:&mut impl crate::boot_part_e::ArchiveHost)->u32 {
    let saved=[m.word(scratch),m.word(scratch+4)];
    let rc=crate::boot_live::stream_read_m(m,h,p,buffer,m.word(p+40),scratch,scratch+4,scratch+5,workspace);
    let crc=m.byte(scratch+4);let eof=m.byte(scratch+5);m.put_word(scratch,saved[0]);m.put_word(scratch+4,saved[1]);
    if rc!=0 {return rc;}if crc==1 || eof!=0 {return crate::normalization::code_4(0);}rc
}
pub fn fn_1a9a60(m:&mut Memory<'_>,a:[u32;7],scratch:u32,e:&Streams,h:&mut impl crate::boot_part_f::Host)->u32 {
    let p=a[0];let n=m.word(p+40);let mut occurrence=0;let mut i=0;
    while i<n {
        let entry=m.word(p+60).wrapping_add(i.wrapping_mul(12));
        if m.word(entry)==a[1] {
            if occurrence==a[2] {
                m.put_word(a[3],m.word(entry+4));let at=m.offset(a[6]);m.bytes[at]=0;
                let vt=m.word(p+48);
                let rc=h.call(m,m.word(vt+4),&[m.word(p+52),m.word(p+64).wrapping_add(m.word(entry+8)),0]);
                if rc!=0 {return crate::normalization::code_6(0);}
                let saved=[m.word(scratch),m.word(scratch+4),m.word(scratch+8)];
                let rc=h.call(m,m.word(m.word(p+48)),&[scratch,1,12,m.word(p+52)]);
                let buf=[m.word(scratch),m.word(scratch+4),m.word(scratch+8)];
                for (j,v) in saved.into_iter().enumerate(){m.put_word(scratch+j as u32*4,v);}
                if rc==12 {
                    m.put_word(a[4],buf[2]);
                    let table=m.word(p+60).wrapping_add(i.wrapping_mul(12));
                    if m.word(table+4)!=buf[1] || m.word(table)!=buf[0] {return crate::normalization::code_4(0);}
                    m.put_word(a[5],m.word(p+64).wrapping_add(m.word(table+8)).wrapping_add(12));m.put_word(a[3],buf[1]);return if m.word(p+40)==i {crate::normalization::code_4(0)}else{0};
                }
                let rc=h.call(m,m.word(m.word(p+48)+8),&[m.word(p+52)]);
                if rc!=0 {return crate::normalization::code_6(0);}
                if crate::boot_part_f::fn_1b3b3c(m,m.word(p+76),0x3ad83a01,&e.parameters,h)&255==2 {let at=m.offset(a[6]);m.bytes[at]=1;return if m.word(p+40)==i {crate::normalization::code_4(0)}else{0};}
                return crate::normalization::code_4(0);
            }
            occurrence+=1;
        }
        i+=1;
    }
    crate::normalization::code_4(0)
}
pub fn fn_1aa1f0(m:&mut Memory<'_>,a:[u32;5],scratch:u32,e:&Streams,h:&mut impl crate::boot_part_f::Host)->u32 {
    let p=a[0];let heap=m.word(p+68);
    let saved=core::array::from_fn::<_,8,_>(|i|m.word(scratch+i as u32*4));
    let rc=fn_1a9a60(m,[p,a[1],a[2],scratch,scratch+4,scratch+8,scratch+12],scratch+16,e,h);
    let values=[m.word(scratch),m.word(scratch+4),m.word(scratch+8),m.byte(scratch+12) as u32];
    for (i,v) in saved.into_iter().enumerate(){m.put_word(scratch+i as u32*4,v);}
    if rc!=0 {return rc;}
    if values[3]==1 || values[1]==u32::MAX {return crate::normalization::code_4(0);}
    let saved=m.word(scratch);let rc=fn_1b3218(m,[heap,heap,m.word(p+48),m.word(p+52),values[2],values[0],m.byte(p+38) as u32,half(m,p+44),scratch],e,h);
    let stream=m.word(scratch);m.put_word(scratch,saved);if rc!=0 {return rc;}
    m.put_word(a[3],stream.wrapping_add(12));m.put_word(a[4],values[1]);let at=m.offset(p+72);m.bytes[at]=1;rc
}

/// fn_176c58, using the existing integer conversion body.
pub fn fn_176c58(m:&mut Memory<'_>,value:u32,buf:u32,base:u32)->u32 {
    assert_eq!(m.base,0);
    crate::normalization::udiv_itoa(m.bytes,value,buf,base)
}
pub fn fn_195d90(m:&mut Memory<'_>,s:u32)->u32 {
    assert_eq!(m.base,0);
    crate::normalization::strlen8(m.bytes,s)
}
pub fn fn_176f94(m:&mut Memory<'_>,a:u32,b:u32,c:u32,d:u32)->u32 {
    let _=m;
    crate::frontend::validate_handle(a!=0,b,c,d)
}
/// Platform clock callback. The temporary output is a real local, not guest state.
pub trait ClockHost {
    fn clock(&mut self,m:&mut Memory<'_>,method:u32,ctx:u32,out:&mut u32)->u32;
}
pub fn fn_173558(m:&mut Memory<'_>,p:u32,out:u32,h:&mut impl ClockHost)->u32 {
    let mut v=0;
    if p!=0 {
        let f=m.word(m.word(p.wrapping_add(8)).wrapping_add(64));
        if f!=0 {v=h.clock(m,f,m.word(p.wrapping_add(12)),&mut v);}
    }
    if out!=0 {m.put_word(out,v);}
    v
}
pub fn fn_176f2c(m:&mut Memory<'_>,p:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    assert_eq!(m.base,0);
    let mut target=None;
    let rc=crate::frontend::acquire_lock(m.bytes,if p==0 {None}else{Some(p)},|ctx| {target=Some(ctx);0});
    if let Some(ctx)=target {
        let f=m.word(m.word(m.word(p).wrapping_add(8)).wrapping_add(8));
        h.call(m,f,&[ctx])
    } else {rc}
}
pub fn fn_176f5c(m:&mut Memory<'_>,p:u32,h:&mut impl crate::boot_part_f::Host)->u32 {
    assert_eq!(m.base,0);
    let mut target=None;
    let rc=crate::frontend::release_lock(m.bytes,if p==0 {None}else{Some(p)},|ctx| {target=Some(ctx);0});
    if let Some(ctx)=target {
        let f=m.word(m.word(m.word(p).wrapping_add(8)).wrapping_add(12));
        h.call(m,f,&[ctx])
    } else {rc}
}

/// Resolve through the existing handle body. Globals are immutable regional data.
pub fn fn_19611c(m:&mut Memory<'_>,handle:[u32;2],out:u32,pair:[u8;8],default:u32)->u32 {
    let [p,tag]=handle;
    let cached=[u32::from_le_bytes(pair[..4].try_into().unwrap()),u32::from_le_bytes(pair[4..].try_into().unwrap())];
    let valid=crate::frontend::validate_handle(p!=0,tag,0x1e61,32)==0;
    let p_word=if p==0 {None} else if handle!=cached && valid {
        // The C output store precedes its tag read, including when they alias.
        m.put_word(out,p);
        Some(half(m,p) as u16)
    } else {Some(0)};
    let mut value=0;
    let rc=crate::frontend::resolve_engine_context(p,p_word,tag,&pair,default,&mut value);
    m.put_word(out,value);
    rc
}

/// Flags-zero broker open, bound to the existing live wrapper.
pub fn fn_179f30(m:&mut Memory<'_>,broker:u32,name:u32,out:u32,g:&crate::boot_part_b::DataOpenGlobals,h:&mut impl crate::boot_part_b::DataHost)->u32 {
    crate::leftovers_1::fn_179f30(m,broker,name,out,g,h)
}
/// Require the full requested map, including the real short-map unmap path.
pub fn fn_17a1a4(m:&mut Memory<'_>,broker:u32,handle:u32,offset:u32,n:u32,out:u32,scratch:u32,e:&crate::boot_part_e::UnmapEnvironment,h:&mut impl crate::boot_part_b::Host)->u32 {
    crate::leftovers_1::fn_17a1a4(m,broker,handle,offset,n,out,scratch,e,h)
}

struct UIntCalls<'a,H>(&'a mut H);
impl<H:crate::speech_part_1::Host> crate::boot_part_f::Host for UIntCalls<'_,H> {
 fn compare(&mut self,_:&mut Memory<'_>,_:u32,_:&mut crate::boot_part_f::Key,_:&mut u32)->u32 {panic!("mutex entry cannot compare keys")}
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call(m,f,a)}
}
impl<H:crate::speech_part_1::Host> crate::speech_part_1::Host for UIntCalls<'_,H> {
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
  match f {0x176f2c=>fn_176f2c(m,a[0],self),0x176f5c=>fn_176f5c(m,a[0],self),_=>self.0.call(m,f,a)}
 }
 fn get(&mut self,m:&mut Memory<'_>,f:u32,p:u32,pos:u32,n:u32)->(u32,u32,u32) {self.0.get(m,f,p,pos,n)}
}
/// The original 36-byte automatic buffer is caller-owned scratch, restored.
pub fn fn_1758ac(m:&mut Memory<'_>,p:u32,key:u32,value:u32,s:u32,h:&mut impl crate::speech_part_1::Host)->u32 {
 let at=m.offset(s);let mut saved=[0u8;36];saved.copy_from_slice(&m.bytes[at..at+36]);
 fn_176c58(m,value,s,10);
 let n=fn_195d90(m,s).wrapping_add(1);
 let rc=crate::speech_part_1::fn_175170(m,p,key,s,n,0,s.wrapping_add(40),&mut UIntCalls(h));
 m.bytes[at..at+36].copy_from_slice(&saved);rc
}
/// Variadic logger gate. va is the caller's ARM argument cursor.
pub fn fn_17486c(m:&mut Memory<'_>,p:u32,b:u32,fmt:u32,va:u32,s:u32,h:&mut impl crate::boot_part_e::Host)->u32 {
 if p==0 || m.byte(p.wrapping_add(64))==0 {return 0;}
 let at=m.offset(s);let mut saved=[0u8;160];saved.copy_from_slice(&m.bytes[at..at+160]);
 let rc=crate::render_part_b::fn_1741dc(m,p,b,fmt,va,s,h);
 m.bytes[at..at+160].copy_from_slice(&saved);rc
}

struct ContextCalls<'a,H>(&'a mut H);
impl<H:crate::boot_part_b::Host> crate::boot_part_e::Host for ContextCalls<'_,H> {
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call(m,f,a)}
}
/// Create the internet component with the real context resolver and allocator.
/// scratch is 24 disjoint automatic bytes, restored even on allocation failure.
pub fn fn_1cede4(m:&mut Memory<'_>,_unused:[u32;2],handle:[u32;2],out:u32,pair:[u8;8],default:u32,module:u32,s:u32,h:&mut impl crate::boot_part_b::FormatHost)->u32 {
 let at=m.offset(s);let mut saved=[0u8;24];saved.copy_from_slice(&m.bytes[at..at+24]);m.put_word(s,0);
 let rc=fn_19611c(m,handle,s,pair,default);
 let result=if rc>=0x80000000 {rc} else {
  let context=m.word(s);
  let p=crate::boot_part_e::calloc(m,m.word(context.wrapping_add(4)),1,4,&mut ContextCalls(h));
  if p!=0 {m.put_word(p,context);m.put_word(out,p);m.put_word(out.wrapping_add(4),0x642);rc}
  else {crate::render_part_a::error_log(m,h,m.word(context.wrapping_add(16)),module,0x20d,0,0,s.wrapping_add(4));0x84f0200a}
 };
 m.bytes[at..at+24].copy_from_slice(&saved);result
}

 
/// Constants stay caller-owned, including regional query keys and log strings.
/// scratch reserves 1024 disjoint bytes and is restored after each public entry.
pub struct Round3<'w> {
 /// Absolute-index bridge workspace, at least as long as the arena.
 pub workspace:core::cell::RefCell<&'w mut [u8]>,
 pub arbiter:u32,
 /// Broker, voice query, component, and synth-stream log categories.
 pub categories:[u32;4],
 /// Update begin/error/end, query begin/end, rescan, null, component error,
 /// reopen, open, null, matcher begin. Slots 6 and 10 are always null.
 pub scratch:u32, pub formats:[u32;12],
 pub component_error_labels:[u32;2],
 pub query_keys:[u32;9], pub pipeline:u32, pub priority:u32,
 pub params_query:u32, pub es_us:u32, pub es_mx:u32, pub segment:u32,
 pub ctype:[u16;256], pub star:u32,
 pub discovery:crate::boot_part_b::DiscoveryGlobals,
 pub null_pair:[u8;8], pub default_context:u32,
 /// langcode, fedataprefix, fedatapackaging, fecfg, clc, CLM, LINGDB,
 /// SYNTHSTREAM, FE_DCTLKP, clm, phonmap, clc/, /, :, clm/, empty.
 pub names:[u32;16],
 pub open:crate::boot_part_b::DataOpenGlobals,
 pub unmap:crate::boot_part_e::UnmapEnvironment,
 pub close:crate::leftovers_1::CloseRuntime,
}
pub trait Round3Host: crate::boot_part_b::DataHost + crate::boot_part_b::FormatHost
 + crate::boot_part_b::DiscoveryHost + crate::log_live::LogMethodsM
 + crate::speech_part_1::Host + crate::boot_part_f::Host + crate::leftovers_1::CloseHost + ClockHost {}
impl<T> Round3Host for T where T:crate::boot_part_b::DataHost+crate::boot_part_b::FormatHost
 +crate::boot_part_b::DiscoveryHost+crate::log_live::LogMethodsM
 +crate::speech_part_1::Host+crate::boot_part_f::Host+crate::leftovers_1::CloseHost+ClockHost {}
fn r3_info(m:&mut Memory<'_>,logger:u32,fmt:usize,args:&[u32],e:&Round3,h:&mut impl Round3Host) {
 for (i,v) in args.iter().enumerate(){m.put_word(e.scratch+960+i as u32*4,*v);}
 let category=e.categories[match fmt {0..=2=>0,3..=5|11=>1,_=>2}];
 crate::log_live::fn_1738e8(m,logger,category,4,0,e.formats[fmt],e.scratch+960,h);
}
fn r3_error(m:&mut Memory<'_>,logger:u32,code:u32,fmt:usize,e:&Round3,h:&mut impl Round3Host) {
 let category=e.categories[match fmt {1=>0,7|10=>2,_=>3}];
 let format=if fmt==6||fmt==10{0}else{e.formats[fmt]};
 crate::render_part_a::error_log(m,h,logger,category,code,format,e.scratch+960,e.scratch+980);
}
struct UpdateCalls<'a,'w,H>(&'a mut H,&'a Round3<'w>);
impl<H:Round3Host> crate::boot_part_b::Host for UpdateCalls<'_,'_,H> {
 fn thread_id(&mut self)->u32 {crate::boot_part_b::Host::thread_id(self.0)}
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
  match f {
   0x17dd30=>crate::boot_part_b::fn_17dd30(m,a[0],a[1],self.1.star,self),
   0x17e0bc=>crate::boot_part_b::fn_17e0bc(m,a[0],a[1],&self.1.discovery,self.0),
   _=>crate::boot_part_b::Host::call(self.0,m,f,a),
  }
 }
}
impl<H:Round3Host> crate::boot_part_b::DirectoryHost for UpdateCalls<'_,'_,H> {
 fn join(&mut self,m:&mut Memory<'_>,a:u32,b:u32,c:u32,d:u32)->(u32,u32) {crate::boot_part_b::DirectoryHost::join(self.0,m,a,b,c,d)}
 fn first(&mut self,m:&mut Memory<'_>,a:u32,b:u32,c:u32,d:u32)->(u32,u32,u32) {crate::boot_part_b::DirectoryHost::first(self.0,m,a,b,c,d)}
 fn next(&mut self,m:&mut Memory<'_>,a:u32)->(u32,u32) {crate::boot_part_b::DirectoryHost::next(self.0,m,a)}
}
pub fn fn_177a98(m:&mut Memory<'_>,p:u32,e:&Round3,h:&mut impl Round3Host)->u32 {
 if p==0 {return 0x81506007;}
 let at=m.offset(e.scratch);let mut saved=[0u8;1024];saved.copy_from_slice(&m.bytes[at..at+1024]);
 r3_info(m,m.word(m.word(p+8)+16),0,&[],e,h);
 let rc=crate::boot_part_b::fn_17e06c(m,p,m.word(p+24),&mut UpdateCalls(h,e));
 if rc>=0x80000000 {r3_error(m,m.word(m.word(p+8)+16),rc,1,e,h);}
 r3_info(m,m.word(m.word(p+8)+16),2,&[rc],e,h);
 m.bytes[at..at+1024].copy_from_slice(&saved);rc
}
struct ParamLock<'a,H>{host:&'a core::cell::RefCell<&'a mut H>,p:u32}
impl<H:Round3Host> crate::reentrant2::StateLockM for ParamLock<'_,H> {
 fn acquire_m(&mut self,bytes:&mut[u8])->u32 {fn_176f2c(&mut Memory{base:0,bytes},self.p,&mut **self.host.borrow_mut())}
 fn release_m(&mut self,bytes:&mut[u8])->u32 {fn_176f5c(&mut Memory{base:0,bytes},self.p,&mut **self.host.borrow_mut())}
}
fn r3_param(m:&mut Memory<'_>,p:u32,key:u32,out:u32,numeric:bool,h:&mut impl Round3Host)->u32 {
 let lock=if p==0 {0}else{m.word(p+4)};
 let host=core::cell::RefCell::new(h);let mut lock=ParamLock{host:&host,p:lock};
 let cmp=|a,b,bytes:&mut[u8]| {let mut m=Memory{base:0,bytes};let method=m.word(m.word(m.word(p+8)+24+36)+8);crate::boot_part_b::Host::call(&mut **host.borrow_mut(),&mut m,method,&[a,b]) as i32};
 if numeric {
  let mut value=0;let mut size=0;
  let rc=crate::reentrant2::lookup_param_m(m.bytes,if p==0 {None}else{Some(p)},key,Some(&mut value),Some(&mut size),&mut lock,cmp);
  let value=if rc<0x80000000 && size>1 {crate::frontend::parse_uint(m.bytes,if value==0 {None}else{Some(value)}) as u32}else{0};
  m.put_word(out,value);rc
 }else{
  crate::speech_part_2::fetch_parameter_m(m,p,key,out,&mut lock,cmp)
 }
}
struct VoiceCalls<'a,H>(&'a mut H);
impl<H:Round3Host> crate::boot::Calls for VoiceCalls<'_,H> {
 fn call_wide(&mut self,_:&mut Memory<'_>,f:u32,_:&[u32])->u64 {panic!("voice query cannot call a wide service {f:x}")}
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
  match f {
  0x1789b0=>{let mut out=m.word(a[2]);let rc=crate::boot_live::open_component_iter_m(m.bytes,if a[0]==0{None}else{Some(a[0])},if a[1]==0{None}else{Some(a[1])},Some(&mut out),a[2]+4,self);m.put_word(a[2],out);rc}
  0x175a1c|0x175944=>r3_param(m,a[0],a[1],a[2],f==0x175944,self.0),
  _=>crate::boot_part_b::Host::call(self.0,m,f,a)}
 }
}
struct VoiceHooks<'a,'w,H>(&'a mut H,&'a Round3<'w>);
impl<H:Round3Host> crate::boot_live::VoiceQueryHooksM for VoiceHooks<'_,'_,H> {
 fn log_begin_m(&mut self,logger:u32,bytes:&mut[u8]) {r3_info(&mut Memory{base:0,bytes},logger,3,&[],self.1,self.0);}
 fn log_end_m(&mut self,logger:u32,rc:u32,bytes:&mut[u8]) {r3_info(&mut Memory{base:0,bytes},logger,4,&[rc],self.1,self.0);}
 fn match_voice_m(&mut self,hdr:&[u8;32],rec:&mut[u8;296],flags:u32,bytes:&mut[u8])->u32 {
  let e=self.1;let s=e.scratch as usize;
  r3_info(&mut Memory{base:0,bytes},u32::from_le_bytes(hdr[16..20].try_into().unwrap()),11,&[],e,self.0);
  bytes[s..s+32].copy_from_slice(hdr);bytes[s+32..s+328].copy_from_slice(rec);
  let rc=crate::boot_live::match_voice_m(bytes,e.scratch,e.scratch+32,flags,&e.query_keys,e.pipeline,e.priority,e.params_query,e.es_us,e.es_mx,e.segment,e.scratch+336,&e.ctype,e.scratch+400,&mut VoiceCalls(self.0));
  rec.copy_from_slice(&bytes[s+32..s+328]);rc
 }
}
pub fn fn_15eef0(m:&mut Memory<'_>,a:[u32;6],e:&Round3,h:&mut impl Round3Host)->u32 {
 assert_eq!(m.base,0);
 let at=m.offset(e.scratch);let mut saved=[0u8;1024];saved.copy_from_slice(&m.bytes[at..at+1024]);
 let rc=crate::boot_live::find_voice_m(&mut VoiceHooks(h,e),m.bytes,if a[0]==0 {None}else{Some(a[0])},a[1],if a[2]==0 {None}else{Some(a[2])},a[3],if a[4]==0 {None}else{Some(a[4])},a[5]);
 m.bytes[at..at+1024].copy_from_slice(&saved);rc
}
pub fn fn_160604(m:&mut Memory<'_>,a:[u32;5],e:&Round3,h:&mut impl Round3Host)->u32 {
 let mut rc=fn_15eef0(m,[a[0],a[1],a[2],a[3],a[4],1],e,h);
 if rc<0x80000000 {return rc;}
 let p=a[0];
 if fn_176f94(m,p,a[1],0x15c,32)>=0x80000000 {return 0x89502008;}
 if m.word(p+20)==0 {return rc;}
 let arc=fn_176f2c(m,m.word(p+28),h);if arc>=0x80000000 {return arc;}
 let s=e.scratch;let saved=m.word(s);let stamp=fn_173558(m,m.word(m.word(p+8)+28),s,h);m.put_word(s,saved);
 if (m.word(p+20) as i32)<(stamp.wrapping_sub(m.word(p+24)) as i32) {
  let at=m.offset(s);let mut saved=[0;1024];saved.copy_from_slice(&m.bytes[at..at+1024]);
  r3_info(m,m.word(m.word(p+8)+16),5,&[stamp.wrapping_sub(m.word(p+24))],e,h);
  m.bytes[at..at+1024].copy_from_slice(&saved);
  if fn_177a98(m,m.word(p+12),e,h)<0x80000000 {
   rc=fn_15eef0(m,[a[0],a[1],a[2],a[3],a[4],1],e,h);m.put_word(p+24,stamp);
  }
 }
 let arc=fn_176f5c(m,m.word(p+28),h);
 if arc>=0x80000000 && rc<0x80000000 {rc=arc;}rc
}

struct RegistryCalls<'a,H>(&'a mut H);
impl<H:Round3Host> crate::render_part_c::ParserMethodsM for RegistryCalls<'_,H> {
 fn method_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32 {crate::boot_part_b::Host::call(self.0,m,f,a)}
}
fn r3_lookup(m:&mut Memory<'_>,o:u32,name:u32,out:u32,h:&mut impl Round3Host)->u32 {
 crate::render_part_c::fn_17b058_m(m,o,name,out,&mut RegistryCalls(h))
}
fn r3_release(m:&mut Memory<'_>,o:u32,name:u32,s:u32,h:&mut impl Round3Host)->u32 {
 // Existing reentrant adapter for frontend::release_ling_entry, including
 // the snapshot/remove/callback case that the script-only API cannot reborrow.
 crate::render_part_c::registry_release_m(m,o,name,s,&mut RegistryCalls(h))
}
fn r3_find(m:&mut Memory<'_>,map:u32,name:u32,h:&mut impl Round3Host)->(u32,u32) {
 if map==0||name==0 {return (0x89002007,0);}
 let node=crate::render_part_c::registry_find_m(m,map,name,&mut RegistryCalls(h));
 match node {Some(n)=>(0,m.word(n+20)),None=>(0x89002014,0)}
}
fn r3_insert(m:&mut Memory<'_>,o:u32,name:u32,pair:u32,s:u32,e:&Round3,h:&mut impl Round3Host)->u32 {
 if o==0||name==0||pair==0 {return crate::frontend::insert_ling_entry(m.bytes,None,None,None,&crate::frontend::InsertScripts{lock:0,find:0,found:None,insert:0,unlock:0});}
 let lock=fn_176f2c(m,m.word(o+12),h);if lock>=0x80000000 {return lock;}
 let (find,entry)=r3_find(m,m.word(o+16),name,h);
 let insert=if find>=0x80000000 {
  crate::frontend::fill_bytes(m.bytes,s,0,32);crate::normalization::memcpy_at(m.bytes,s,pair,20);
  r3_map_insert(m,[m.word(o+16),name,s],s+32,e,h)
 }else{0};
 let mut rc=crate::frontend::insert_ling_entry(m.bytes,Some(o),Some(name),Some(pair),&crate::frontend::InsertScripts{lock:0,find,found:if find<0x80000000{Some(entry)}else{None},insert,unlock:0});
 let status=fn_176f5c(m,m.word(o+12),h);if status>=0x80000000&&rc<0x80000000{rc=status;}rc
}
fn r3_drop(m:&mut Memory<'_>,o:u32,name:u32,h:&mut impl Round3Host)->u32 {
 if o==0||name==0 {return 0x8b602007;}
 let lock=fn_176f2c(m,m.word(o+12),h);if lock>=0x80000000{return lock;}
 let (find,entry)=if m.word(o+16)==0 {(lock,0)}else{r3_find(m,m.word(o+16),name,h)};
 let remove=entry!=0&&half(m,entry+28)==0;
 let mut rc=crate::frontend::drop_ling_entry(m.bytes,Some(o),Some(name),&crate::frontend::ReleaseScripts{lock,find,found:if entry!=0{Some(entry)}else{None},remove:0,unlock:0});
 if rc<0x80000000 && remove {rc=r3_map_remove(m,m.word(o+16),name,h);}
 let status=fn_176f5c(m,m.word(o+12),h);if status>=0x80000000&&rc<0x80000000 {rc=status;}rc
}
fn r3_clear(m:&mut Memory<'_>,p:u32) {m.put_word(p+52,0);m.put_word(p+56,0);m.put_word(p+60,0);}
fn r3_unmap(m:&mut Memory<'_>,p:u32,e:&Round3,h:&mut impl Round3Host) {
 crate::boot_part_e::fn_179208(m,m.word(m.word(p)+12),m.word(p+52),m.word(p+56),&e.unmap,&mut ContextCalls(h));
}
fn r3_close(m:&mut Memory<'_>,p:u32,e:&Round3,h:&mut impl Round3Host) {
 crate::leftovers_1::fn_179f3c(m,m.word(m.word(p)+12),m.word(p+52),&e.close,h);
}
pub fn fn_1d4ad8(m:&mut Memory<'_>,p:u32,e:&Round3,h:&mut impl Round3Host)->u32 {
 let at=m.offset(e.scratch);let mut saved=[0u8;1024];saved.copy_from_slice(&m.bytes[at..at+1024]);
 let rc=r3_load_stream(m,p,e,h);m.bytes[at..at+1024].copy_from_slice(&saved);rc
}
fn r3_load_stream(m:&mut Memory<'_>,p:u32,e:&Round3,h:&mut impl Round3Host)->u32 {
 let s=e.scratch;crate::frontend::fill_bytes(m.bytes,s,0,272);
 let properties=m.word(m.word(p)+20);
 let rc=r3_param(m,properties,e.names[0],s+256,false,h);if rc>=0x80000000 {return rc;}
 let lang=m.word(s+256);
 let rc=r3_param(m,properties,e.names[1],s+260,false,h);
 if rc<0x80000000 {let prefix=m.word(s+260);if prefix!=0&&m.byte(prefix)!=0 {crate::frontend::copy_str(m.bytes,s,prefix);}}
 let rc=r3_param(m,properties,e.names[2],s+264,false,h);
 let packaging=m.word(s+264);
 if rc<0x80000000&&packaging!=0&&crate::frontend::compare_str(crate::frontend::terminated(m.bytes,packaging),crate::frontend::terminated(m.bytes,e.names[4]))==0 {
  r3_param(m,properties,e.names[3],s+268,false,h);
  for a in [e.names[11],lang,e.names[12],if m.word(s+268)!=0{m.word(s+268)}else{e.names[15]},e.names[13]] {crate::frontend::append_str(m.bytes,s,a);}
 }
 for a in [e.names[14],lang] {crate::frontend::append_str(m.bytes,s,a);}
 let mut i=0u16;
 while i<(fn_195d90(m,s) as u16) {let at=m.offset(s+i as u32);m.bytes[at]=crate::normalization::tolower_240ba0(m.bytes[at] as u32) as u8;i=i.wrapping_add(1);}
 let rc=crate::boot_part_b::fn_1795f0(m,m.word(m.word(p)+12),s,1,p+52,&e.open,h);
 if rc>=0x80000000 {r3_clear(m,p);return 0;}
 m.put_word(p+60,0);
 let mut rc=crate::boot_part_b::fn_179374(m,m.word(m.word(p)+12),m.word(p+52),0,p+60,p+56,h);
 if rc<0x80000000 {
  rc=crate::boot_part_b::Host::call(h,m,m.word(m.word(p+72)+28),&[m.word(p+64),m.word(p+68),m.word(p+56),m.word(p+60)]);
  if rc<0x80000000 {return rc;}
  r3_unmap(m,p,e,h);
 }
 r3_close(m,p,e,h);r3_clear(m,p);rc
}
pub fn fn_1d49b4(m:&mut Memory<'_>,p:u32,tag:u32,e:&Round3,h:&mut impl Round3Host)->u32 {
 let mut rc=fn_176f94(m,p,tag,0xf380,76);if rc>=0x80000000 {return 0x89902008;}
 let at=m.offset(e.scratch);let mut saved=[0u8;1024];saved.copy_from_slice(&m.bytes[at..at+1024]);
 for name in [e.names[6],e.names[7],e.names[8]] {r3_release(m,m.word(m.word(p)+24),name,e.scratch+480,h);}
 if m.word(p+56)!=0 {
  crate::boot_part_b::Host::call(h,m,m.word(m.word(p+72)+32),&[m.word(p+64),m.word(p+68)]);
  r3_unmap(m,p,e,h);r3_close(m,p,e,h);r3_clear(m,p);
 }
 if m.word(p+72)!=0 {
  rc=crate::boot_part_b::Host::call(h,m,m.word(m.word(p+72)+20),&[m.word(p+64),m.word(p+68)]);
  if rc<0x80000000 {r3_drop(m,m.word(m.word(p)+24),e.names[5],h);}
 }
 crate::boot_part_e::release(m,m.word(m.word(p)+4),p,&mut ContextCalls(h));
 m.bytes[at..at+1024].copy_from_slice(&saved);rc
}
pub fn fn_1d4d6c(m:&mut Memory<'_>,unused:[u32;2],source:[u32;2],out:u32,e:&Round3,h:&mut impl Round3Host)->u32 {
 let _=unused;
 if out==0 {return 0x89902007;}
 m.put_word(out,0);m.put_word(out+4,0);
 let s=e.scratch;let at=m.offset(s);let mut saved=[0u8;1024];saved.copy_from_slice(&m.bytes[at..at+1024]);m.put_word(s+400,0);
 let rc=r3_create_stream(m,source,out,e,h);
 m.bytes[at..at+1024].copy_from_slice(&saved);rc
}
fn r3_create_stream(m:&mut Memory<'_>,source:[u32;2],out:u32,e:&Round3,h:&mut impl Round3Host)->u32 {
 let s=e.scratch;
 let rc=fn_19611c(m,source,s+400,e.null_pair,e.default_context);if rc>=0x80000000{return rc;}
 let ctx=m.word(s+400);
 let mut entries=[0;3];
 for i in 0..3 {
  m.put_word(s+404,0);
  let rc=r3_lookup(m,m.word(ctx+24),e.names[6+i],s+404,h);
  if rc>=0x80000000 {for j in 0..i {r3_release(m,m.word(ctx+24),e.names[6+j],s+480,h);}return rc;}
  entries[i]=m.word(s+404);
 }
 let p=crate::boot_part_e::calloc(m,m.word(ctx+4),1,76,&mut ContextCalls(h));
 if p==0 {
  r3_error(m,m.word(ctx+16),0x7530,6,e,h);
  for name in [e.names[6],e.names[7],e.names[8]]{r3_release(m,m.word(ctx+24),name,s+480,h);}
  return 0x8990200a;
 }
 m.put_word(out+4,0xf380);m.put_word(out,p);m.put_word(p,ctx);
 for (off,value) in [(24,m.word(entries[0]+4)),(28,m.word(entries[1]+4)),(20,m.word(entries[2]+4)),(12,m.word(entries[2]+8)),(16,m.word(entries[2]+12)),(4,source[0]),(8,source[1])]{m.put_word(p+off,value);}
 crate::frontend::fill_bytes(m.bytes,s+408,0,20);m.bytes[(s+408)as usize]=1;
 for i in 0..2 {m.put_word(s+432+i*4,u32::from_le_bytes(e.null_pair[i as usize*4..i as usize*4+4].try_into().unwrap()));}
 let mut rc=crate::boot_part_b::fn_178510(m,m.word(ctx+12),e.names[9],1,s+432,s+412,h);
 if rc<0x80000000 {
  m.put_word(p+72,m.word(s+412));m.bytes[(s+440)as usize]=0;m.bytes[(s+441)as usize]=0;
  rc=crate::boot_part_b::Host::call(h,m,m.word(m.word(p+20)+76),&[m.word(p+12),m.word(p+16),s+440,s+441]);
  if rc<0x80000000 {
   m.bytes[(p+48)as usize]=u8::from(m.byte(s+440)==1&&m.byte(s+441)==1);
   let null=[u32::from_le_bytes(e.null_pair[..4].try_into().unwrap()),u32::from_le_bytes(e.null_pair[4..].try_into().unwrap())];
   rc=crate::boot_part_b::Host::call(h,m,m.word(m.word(p+72)+16),&[null[0],null[1],m.word(p+4),m.word(p+8),p+64]);
   if rc<0x80000000 {
    m.put_word(s+416,m.word(p+64));m.put_word(s+420,m.word(p+68));
    rc=r3_insert(m,m.word(ctx+24),e.names[5],s+408,s+448,e,h);
    if rc<0x80000000 {rc=fn_1d4ad8(m,p,e,h);}
   }
  }
 }
 if rc>=0x80000000 {fn_1d49b4(m,m.word(out),m.word(out+4),e,h);m.put_word(out,0);m.put_word(out+4,0);}rc
}
pub fn fn_17f278(m:&mut Memory<'_>,handle:[u32;2],name:u32,component:u32,owner:u32,flag:u32,e:&Round3,h:&mut impl Round3Host)->u32 {
 let s=e.scratch;let at=m.offset(s);let mut saved=[0u8;1024];saved.copy_from_slice(&m.bytes[at..at+1024]);
 let rc=r3_open_component(m,handle,name,component,owner,flag,e,h);
 m.bytes[at..at+1024].copy_from_slice(&saved);rc
}
fn r3_open_component(m:&mut Memory<'_>,handle:[u32;2],name:u32,component:u32,owner:u32,flag:u32,e:&Round3,h:&mut impl Round3Host)->u32 {
 let s=e.scratch;m.put_word(s+400,0);m.put_word(s+404,0);
 let rc=fn_19611c(m,handle,s+400,e.null_pair,e.default_context);if rc>=0x80000000 {return rc;}
 let ctx=m.word(s+400);
 if r3_lookup(m,m.word(ctx+24),name,s+404,h)<0x80000000 {
  let entry=m.word(s+404);let object=m.word(entry+4);let mut rc=rc;
  if m.word(object+24)!=0 {
   r3_info(m,m.word(ctx+16),8,&[name],e,h);
   if flag!=0 {let at=m.offset(flag);m.bytes[at]=1;}
   rc=crate::boot_part_b::Host::call(h,m,m.word(object+24),&[m.word(entry+8),m.word(entry+12)]);
   if rc>=0x80000000 {r3_component_error(m,ctx,component,rc,e,h);}
  }
  r3_release(m,m.word(ctx+24),name,s+480,h);return rc;
 }
 crate::frontend::fill_bytes(m.bytes,s+408,0,20);
 let kind=if crate::frontend::compare_folded(&e.ctype,crate::frontend::terminated(m.bytes,component),crate::frontend::terminated(m.bytes,e.names[10]))==0 {2}else{1};
 m.bytes[(s+408)as usize]=kind;
 for i in 0..2 {m.put_word(s+432+i*4,u32::from_le_bytes(e.null_pair[i as usize*4..i as usize*4+4].try_into().unwrap()));}
 let rc=crate::boot_part_b::fn_178510(m,m.word(ctx+12),component,kind as u32,s+432,s+412,h);
 if rc>=0x80000000 {return rc;}
 r3_info(m,m.word(ctx+16),9,&[component],e,h);
 let interface=m.word(s+412);
 let mut rc=crate::boot_part_b::Host::call(h,m,m.word(interface+16),&[m.word(s+432),m.word(s+436),handle[0],handle[1],s+416]);
 if rc>=0x80000000 {
  r3_component_error(m,ctx,component,rc,e,h);
  crate::frontend::release_broker_interface(if m.word(ctx+12)==0{None}else{Some(m.word(ctx+12))},Some(interface));return rc;
 }
 rc=r3_insert(m,m.word(ctx+24),name,s+408,s+448,e,h);
 if rc>=0x80000000 {r3_component_error(m,ctx,component,rc,e,h);}
 else if owner!=0 {
  struct Db<'a,H>(&'a mut H);
  impl<H:Round3Host> crate::db_live::PoolHooksM for Db<'_,H> {fn log_pool_m(&mut self,_:u32,_:u32,_:&mut Memory<'_>){unreachable!()}}
  impl<H:Round3Host> crate::db_live::DbHooksM for Db<'_,H> {fn call_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32 {crate::boot_part_b::Host::call(self.0,m,f,a)}}
  m.put_word(s+440,name);
  if crate::db_live::append_element_m(&mut Db(h),m,owner,s+440,s+444)!=0 {return rc;}
  r3_error(m,m.word(ctx+16),16000,10,e,h);rc=0x8120200a;
 }else{return rc;}
 crate::boot_part_b::Host::call(h,m,m.word(interface+20),&[m.word(s+416),m.word(s+420)]);
 crate::frontend::release_broker_interface(if m.word(ctx+12)==0{None}else{Some(m.word(ctx+12))},Some(interface));rc
}

struct MapPlatform<'a,'h,H>{host:&'a core::cell::RefCell<&'h mut H>,arbiter:u32}
impl<H:Round3Host> crate::bridge::Platform for MapPlatform<'_,'_,H> {
 fn thread_id(&mut self)->u32 {crate::boot_part_b::Host::thread_id(&mut **self.host.borrow_mut())}
 fn arbiter_address(&self)->u32 {self.arbiter}
 fn arbitrate(&mut self,m:&mut Memory<'_>,a:&[u32])->u32 {crate::boot_part_b::Host::call(&mut **self.host.borrow_mut(),m,0x172d18,a)}
}
struct MapLive<'a,'h,H>(&'a core::cell::RefCell<&'h mut H>);
impl<H:Round3Host> crate::bridge::Live for MapLive<'_,'_,H> {
 fn call<P:crate::bridge::Platform>(&mut self,b:&mut crate::bridge::Flat<'_,'_,'_,'_,P>,f:u32,a:&[u32])->u32 {
  crate::boot_part_b::Host::call(&mut **self.0.borrow_mut(),&mut b.memory(),f,a)
 }
 fn log<P:crate::bridge::Platform>(&mut self,_:&mut crate::bridge::Flat<'_,'_,'_,'_,P>,_:u32,_:u32,_:&[u32]) {unreachable!("map insertion has no logger")}
}
fn r3_map_insert(m:&mut Memory<'_>,a:[u32;3],s:u32,e:&Round3,h:&mut impl Round3Host)->u32 {
 assert_eq!(m.base,0);
 let host=core::cell::RefCell::new(h);
 let mut platform=MapPlatform{host:&host,arbiter:e.arbiter};
 let mut regions=[crate::engine::Region{base:0,bytes:m.bytes}];
 let mut memory=crate::engine::Memory::new(&mut regions).expect("single arena region");
 let mut workspace=e.workspace.borrow_mut();
 let mut flat=crate::bridge::Flat::new(&mut memory,&mut workspace,&mut platform,0).expect("caller bridge workspace too small");
 let rc=flat.fn_19668c(a,s,&mut MapLive(&host));
 flat.finish().expect("map insertion reached an unbound allocator");rc
}
struct NodeCalls<'a,H>(&'a mut H);
impl<H:Round3Host> crate::render_part_d::DictionaryHost for NodeCalls<'_,H> {
 fn heap_method(&mut self,bytes:&mut[u8],f:u32,obj:u32,args:&[u32])->u32 {
  let mut a=[0;5];a[0]=obj;a[1..1+args.len()].copy_from_slice(args);
  crate::boot_part_b::Host::call(self.0,&mut Memory{base:0,bytes},f,&a[..1+args.len()])
 }
 fn tree_callback(&mut self,bytes:&mut[u8],f:u32,a:&[u32]) {crate::boot_part_b::Host::call(self.0,&mut Memory{base:0,bytes},f,a);}
 fn remove_node(&mut self,bytes:&mut[u8],tree:u32,node:u32) {crate::render_part_d::dictionary_release_node(bytes,tree,node,self);}
 fn lock_method(&mut self,_:&mut[u8],_:u32,_:u32)->u32 {unreachable!("node destruction cannot lock")}
 fn compare_key(&mut self,_:&[u8],_:u32,_:u32,_:u32)->i32 {unreachable!("node destruction cannot compare")}
 fn release_callback(&mut self,_:&mut[u8],_:[u32;2],_:&mut[u8;32]) {unreachable!("node destruction cannot release a linguistic entry")}
 fn open_dictionary(&mut self,_:&mut[u8],_:u32,_:[u32;2],_:u32,_:&mut u8)->u32 {unreachable!("node destruction cannot open")}
 fn match_dictionary(&mut self,_:&mut[u8],_:u32,_:[u32;2],_:u32,_:&[u8],_:&mut u32,_:&mut u16,_:&mut u8)->u32 {unreachable!("node destruction cannot match")}
}
fn r3_map_remove(m:&mut Memory<'_>,map:u32,key:u32,h:&mut impl Round3Host)->u32 {
 if map==0||key==0 {return 0x89002007;}
 if let Some(node)=crate::render_part_c::registry_find_m(m,map,key,&mut RegistryCalls(h)) {
  crate::render_part_d::dictionary_release_node(m.bytes,map+24,node,&mut NodeCalls(h));0
 }else{0x89002014}
}

fn r3_component_error(m:&mut Memory<'_>,ctx:u32,component:u32,rc:u32,e:&Round3,h:&mut impl Round3Host) {
 for (i,v) in [e.component_error_labels[0],component,e.component_error_labels[1],rc].into_iter().enumerate(){m.put_word(e.scratch+960+i as u32*4,v);}
 r3_error(m,m.word(ctx+16),16001,7,e,h);
}
