
use crate::boot::Memory;

pub trait Host {
    fn call(&mut self, memory: &mut Memory<'_>, method: u32, args: &[u32]) -> u32;
}

pub fn fn_7e715c(m: &mut Memory<'_>, p: u32, v: u32) -> u32 {
    u32::from(m.word(p + 0x30) <= v && v < m.word(p + 0x34))
}

pub fn fn_7f6820(m: &mut Memory<'_>, p: u32) -> u32 {
    u32::from(m.word(p + 0x80) >= m.word(p + 0x88))
}

pub fn fn_4a794c(m: &mut Memory<'_>, p: u32, count: u32, whence: u32, element: u32) -> u32 {
    if count > 0x7ffffff { return 0x80000104; }
    let off = count.wrapping_mul(element);
    let pos = match whence {
        0 => m.word(p).wrapping_add(off),
        1 => m.word(p + 4).wrapping_add(off),
        2 => m.word(p).wrapping_add(m.word(p + 8)).wrapping_sub(off),
        _ => return 0x80000007,
    };
    m.put_word(p + 4, pos);
    0
}

pub fn fn_4a774c(m: &mut Memory<'_>, p: u32, a: u32, h: &mut impl Host) -> u32 {
    let method = m.word(m.word(p) + 12);
    h.call(m, method, &[p, a])
}

pub fn fn_4a7aac(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    let cb = m.word(p + 12);
    let method = m.word(m.word(cb) + 12);
    h.call(m, method, &[cb, p]);
    0
}

pub fn fn_4a78f0(m: &mut Memory<'_>, dst: u32, a: u32, b: u32, p: u32) -> u32 {
    let mut n = a.wrapping_mul(b);
    let pos = m.word(p + 4);
    let end = m.word(p).wrapping_add(m.word(p + 8));
    if pos.wrapping_add(n) > end {
        n = end.wrapping_sub(pos);
        if (n as i32) < 0 { n = 0; }
    }
    m.rvct_copy(dst, pos, n);
    m.put_word(p + 4, m.word(p + 4).wrapping_add(n));
    n
}

pub fn fn_301ea0(m: &mut Memory<'_>, global: u32) -> u32 {
    let _ = m;
    global
}

pub fn fn_302078(m: &mut Memory<'_>, global: u32) -> u32 {
    fn_301ea0(m, global).wrapping_add(4)
}

pub fn fn_2a76a4(m: &mut Memory<'_>, ctx: u32, rows: u32, cols: u32,
    padding: u32, out: u32, h: &mut impl crate::frontend::HeapVtableM) -> u32 {
    assert_eq!(m.base, 0);
    let ctx = if ctx == 0 {None} else {Some(ctx)};
    let arr = crate::frontend::heap_alloc_m(m.bytes, ctx, rows.wrapping_mul(4), h);
    m.put_word(out, arr);
    if arr == 0 {return 0x84e0200a;}
    let stride = cols.wrapping_add(padding.wrapping_mul(2));
    let mut data = crate::frontend::heap_alloc_m(m.bytes, ctx,
        stride.wrapping_mul(rows).wrapping_mul(4), h);
    if data == 0 {
        let arr=m.word(out);
        crate::frontend::heap_release_m(m.bytes,ctx,if arr==0 {None} else {Some(arr)},h);
        m.put_word(out,0);
        return 0x84e0200a;
    }
    let mut i=0u32;
    while (i as i32) < (rows as i32) {
        m.put_word(m.word(out).wrapping_add(i.wrapping_mul(4)),data.wrapping_add(padding.wrapping_mul(4)));
        data=data.wrapping_add(stride.wrapping_mul(4));
        i=i.wrapping_add(1);
    }
    0
}

pub fn fn_84c160(m: &mut Memory<'_>, p: u32, v: u32,
    h: &mut impl crate::boot_part_b::Host) -> u32 {
    crate::boot_part_b::fn_1840fc(m,p.wrapping_add(88),h);
    let result=fn_7e715c(m,p,v);
    crate::boot_part_b::fn_18423c(m,p.wrapping_add(88),h);
    result
}

pub fn fn_39f5b8(m: &mut Memory<'_>, p: u32, pointer: u32,
    h: &mut impl crate::boot_part_b::Host) {
    let inner=m.word(p.wrapping_add(4));
    crate::boot_part_b::fn_1840fc(m,inner.wrapping_add(88),h);
    crate::boot_part_b2::fn_19b78c_guest(m,inner,pointer);
    crate::boot_part_b::fn_18423c(m,inner.wrapping_add(88),h);
}

pub fn fn_4a79b8(m: &mut Memory<'_>, p: u32, old: u32, size: u32,
    h: &mut (impl Host + crate::boot_part_b::Host)) -> u32 {
    assert_eq!(m.base,0);
    let heap=m.word(p.wrapping_add(4));
    crate::boot_part_b::fn_1840fc(m,heap.wrapping_add(88),h);
    let mut saved=[0u8;88];saved.copy_from_slice(&m.bytes[..88]);
    m.bytes.copy_within(heap as usize..heap as usize+88,0);
    let ok=crate::heap::EngineHeap::from_arena(m.bytes).block_resize(old,size).unwrap_or(0);
    m.bytes.copy_within(0..88,heap as usize);m.bytes[..88].copy_from_slice(&saved);
    crate::boot_part_b::fn_18423c(m,heap.wrapping_add(88),h);
    if ok!=0 {return old;}
    let method=m.word(m.word(p)+8);
    let new=Host::call(h,m,method,&[p,size,4]);
    if new!=0 && size!=0 {
        let at=m.offset(new);m.bytes[at..at+size as usize].fill(0);
    }
    let old_size=crate::heap::EngineHeap::from_arena(m.bytes).block_size(old);
    m.rvct_copy(new,old,old_size.min(size));
    let method=m.word(m.word(p)+12);
    Host::call(h,m,method,&[p,old]);
    new
}

pub fn fn_3015a4(m: &mut Memory<'_>, ctx: u32, crit: u32, list: u32,
    out_buf: u32, out_count: u32, h: &mut impl crate::boot_part_f::Host) -> u32 {
    fn cmp(m: &Memory<'_>, a: u32, b: u32) -> i32 {
        crate::frontend::compare_str(&m.bytes[m.offset(a)..], &m.bytes[m.offset(b)..])
    }
    fn copy(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) {
        crate::fe_text::memcpy_195ae8(m.bytes, dst.wrapping_sub(m.base), src.wrapping_sub(m.base), n);
    }
    let heap = m.word(list.wrapping_add(0x24));
    let mut node = m.word(list.wrapping_add(0x28));
    let mut total = 0u32;
    let mut out = 0;
    let mut oerr = 0u8;
    while node != 0 {
        let mut ierr = 0u8;
        let mut count = 0u32;
        let mut buf = 0u32;
        let head = m.word(node);
        let base = m.word(node.wrapping_add(20));
        let keys = m.word(node.wrapping_add(16));
        let mut lo = 0u32;
        let mut hi = m.word(head.wrapping_add(32)).wrapping_sub(1);
        let accept = if crit == 0 { true } else {
            let a = m.word(crit.wrapping_add(8));
            let b = m.word(crit.wrapping_add(16));
            let c = m.word(crit.wrapping_add(12));
            (a == 0 || (m.word(node+4) != 0 && cmp(m,a,m.word(node+4)) == 0))
                && (b == 0 || m.word(node+12) == 0 || cmp(m,b,m.word(node+12)) == 0)
                && (c == 0 || m.word(node+8) == 0 ||
                    crate::frontend::compare_bytes(&m.bytes[m.offset(c)..],
                        &m.bytes[m.offset(m.word(node+8))..],128) == 0)
                && (m.word(crit) == u32::MAX || m.word(crit) == m.word(head+8))
                && (m.word(crit+4) == u32::MAX || m.word(crit+4) == m.word(head+16))
        };
        if accept && m.word(head+32) != 0
            && cmp(m,ctx,m.word(keys).wrapping_add(base)) >= 0
            && cmp(m,ctx,m.word(keys.wrapping_add(hi.wrapping_mul(4))).wrapping_add(base)) <= 0 {
            loop {
                let mid = lo.wrapping_add(hi) >> 1;
                let r = cmp(m,ctx,m.word(keys.wrapping_add(mid.wrapping_mul(4))).wrapping_add(base));
                if r < 0 { hi = mid.wrapping_sub(1); }
                if r > 0 { lo = mid.wrapping_add(1); }
                if r == 0 {
                    let offsets = m.word(node+24);
                    let first = m.word(offsets.wrapping_add(mid.wrapping_mul(4)));
                    let mut second = m.word(offsets.wrapping_add(mid.wrapping_add(1).wrapping_mul(4)));
                    if (second as i32) < 0 { second = second.wrapping_neg(); }
                    if (first as i32) < 0 {
                        let blob = m.word(node+28);
                        let end = blob.wrapping_add(second);
                        let beg = blob.wrapping_sub(first);
                        let mut cur = beg.wrapping_add(beg & 1);
                        let mut data = cur.wrapping_add(2);
                        let vals = [m.word(head+8),m.word(head+12),m.word(head+16),
                            m.word(node+4),m.word(node+8),m.word(node+12)];
                        while data < end {
                            count = count.wrapping_add(1);
                            buf = crate::boot_part_f::fn_1b218c(m,heap,buf,count.wrapping_mul(32),&mut ierr,h);
                            if ierr != 0 { break; }
                            let rec = buf.wrapping_add(count.wrapping_sub(1).wrapping_mul(32));
                            for (i,v) in vals.iter().enumerate() { m.put_word(rec.wrapping_add(i as u32*4),*v); }
                            let n = m.half(cur) as u32;
                            m.put_word(rec+28,n);
                            if matches!(m.word(buf),1|3) {
                                let p = crate::boot_part_f::fn_1b2140(m,heap,n,&mut ierr,h);
                                if ierr != 0 { break; }
                                m.put_word(rec+24,p);
                                copy(m,p,data,m.word(rec+28) & !1);
                            } else { m.put_word(rec+24,data); }
                            let len = m.half(cur) as u32;
                            cur = data.wrapping_add(len).wrapping_add((len ^ data) & 1);
                            data = cur.wrapping_add(2);
                        }
                    } else {
                        buf = crate::boot_part_f::fn_1b2140(m,heap,32,&mut ierr,h);
                        if ierr == 0 {

                            let head = m.word(node);
                            for i in 0..3 { m.put_word(buf+i*4,m.word(head+8+i*4)); }
                            for i in 0..3 { m.put_word(buf+12+i*4,m.word(node+4+i*4)); }
                            m.put_word(buf+28,second.wrapping_sub(first));
                            if matches!(m.word(buf),1|3) {
                                let p = crate::boot_part_f::fn_1b2140(m,heap,m.word(buf+28),&mut ierr,h);
                                if ierr == 0 {
                                    m.put_word(buf+24,p);
                                    copy(m,p,m.word(node+28).wrapping_add(first),m.word(buf+28) & !1);
                                    count = 1;
                                }
                            } else {
                                m.put_word(buf+24,m.word(node+28).wrapping_add(first));
                                count = 1;
                            }
                        }
                    }
                    break;
                }
                if lo > hi { break; }
            }
        }
        oerr = ierr;
        if oerr != 0 { break; }
        if count != 0 {
            let t = total.wrapping_add(count);
            out = crate::boot_part_f::fn_1b218c(m,heap,out,t.wrapping_mul(32),&mut oerr,h);
            if oerr != 0 { break; }
            copy(m,out.wrapping_add(total.wrapping_mul(32)),buf,count.wrapping_mul(32));
            crate::boot_part_f::fn_1b2128(m,heap,buf,h);
            total = t;
        }
        node = m.word(node+32);
    }
    m.put_word(out_buf,out);
    m.put_word(out_count,total);
    oerr as u32
}

fn sp3_copy(m:&mut Memory<'_>,d:u32,s:u32,n:u32) {
    crate::fe_text::memcpy_195ae8(m.bytes,d-m.base,s-m.base,n);
}
fn sp3_len(m:&Memory<'_>,s:u32)->u32 {
    crate::fe_text::strlen_195d90(m.bytes,s-m.base) as u32
}
fn sp3_byte(m:&mut Memory<'_>,p:u32,v:u8) {let i=m.offset(p);m.bytes[i]=v;}
fn sp3_string(m:&mut Memory<'_>,p:u32,s:&[u8]) {let i=m.offset(p);m.bytes[i..i+s.len()].copy_from_slice(s);}

pub trait SpeechHost: crate::speech_part_5::Host {}

fn sp3_frame(m:&mut Memory<'_>,scratch:u32,body:impl FnOnce(&mut Memory<'_>)->u32)->u32 {
    let mut saved=[0u8;4096];
    let at=m.offset(scratch);
    saved.copy_from_slice(&m.bytes[at..at+4096]);
    let r=body(m);
    m.bytes[at..at+4096].copy_from_slice(&saved);
    r
}

pub fn fn_2e3e9c(m:&mut Memory<'_>,p:u32,a:u32,b:u32,o1:u32,o2:u32,h:&mut impl SpeechHost)->u32 {
    m.put_word(o1,0);m.put_word(o2,0);
    if m.byte(p+20)!=1 {
        crate::speech_part_5::fn_301b70(m,m.word(p+36),a,b,p,o1,o2,h)
    } else {fn_3015a4(m,a,b,p,o1,o2,h)}
}

pub fn fn_2e4260(m:&mut Memory<'_>,ctx:u32,source:u32,count:u32,out:u32,out_count:u32,scratch:u32,h:&mut impl SpeechHost)->u32 {
    sp3_frame(m,scratch,|m| {
        let mut status=0u8;let mut total=0u32;let mut items=0;
        let heap=m.word(ctx+4);
        if count!=0 {
            total=count;
            let mut allocation=crate::boot_part_f::fn_1b2140(m,heap,total.wrapping_mul(12).wrapping_add(4),&mut status,h);
            if status!=0 {return status as u32;}
            m.put_word(allocation,total);items=allocation+4;
            let mut written=0u32;
            for i in 0..count {
                let src=source+i*12;
                let mut expanded_count=0;
                let mut expanded=0;
                let phonetic=m.byte(src)==1;
                if phonetic {
                    let result=crate::speech_part_5::fn_2f91f8(m,m.word(ctx+8),m.word(ctx+4),m.word(src+4),scratch,scratch+4,h);
                    status=result as u8;
                    if result!=0 {return status as u32;}
                    expanded=m.word(scratch);expanded_count=m.word(scratch+4);
                }
                if !phonetic || expanded_count==0 {
                    let n=if m.byte(src)==1 {sp3_len(m,m.word(src+4)).wrapping_add(1)} else {m.word(src+8)};
                    let dest=items+written*12;
                    let data=crate::boot_part_f::fn_1b2140(m,m.word(ctx+4),n,&mut status,h);
                    m.put_word(dest+4,data);
                    if status!=0 {return status as u32;}
                    sp3_byte(m,dest,m.byte(src));
                    sp3_copy(m,m.word(dest+4),m.word(src+4),n);
                    m.put_word(dest+8,n);written+=1;
                }
                if phonetic {
                    if expanded_count>1 {
                        total=total.wrapping_add(expanded_count-1);
                        allocation=crate::boot_part_f::fn_1b218c(m,m.word(ctx+4),allocation,total.wrapping_mul(12).wrapping_add(4),&mut status,h);
                        if status!=0 {return status as u32;}
                        m.put_word(allocation,total);items=allocation+4;
                    }
                    for j in 0..expanded_count {
                        let dest=items+written*12;
                        sp3_byte(m,dest,m.byte(src));
                        m.put_word(dest+4,m.word(expanded+j*4));
                        let n=sp3_len(m,m.word(expanded+j*4)).wrapping_add(1);
                        m.put_word(dest+8,n);written+=1;
                    }
                    crate::boot_part_f::fn_1b2128(m,m.word(ctx+4),expanded,h);
                }
            }
        }
        m.put_word(out,items);m.put_word(out_count,total);status as u32
    })
}

pub fn fn_2fbe50_m() -> u32 {
    0
}

pub fn fn_2e19dc(m:&mut Memory<'_>,o:u32,a:u32,b:u32,e:u32,n:u32,scratch:u32,h:&mut impl SpeechHost)->u32 {
    sp3_frame(m,scratch,|m| {
        if e==0 || n==0 {return 0;}
        let vt=m.word(o);
        if h.call(m,m.word(vt+28),&[o.wrapping_sub(m.word(vt+40)),a])==0 {return 0;}
        let mut dst=scratch;let tmp=scratch+2047;
        let spec=scratch+2080;let non=scratch+2084;let space=scratch+2100;
        sp3_string(m,spec,b"s\0");sp3_string(m,non,b"<non-phonetic>\0");sp3_string(m,space,b" \0");
        for i in 0..n {
            let s=if m.byte(e+i*12)==1 {m.word(e+i*12+4)} else {non};
            dst=dst.wrapping_add(crate::speech_part_5::fn_2e1fc8(m,dst,tmp,spec,s,h));
            if i+1!=n {dst=dst.wrapping_add(crate::speech_part_5::fn_2e1fc8(m,dst,tmp,spec,space,h));}
        }
        sp3_byte(m,dst,0);
        let fmt=scratch+2110;sp3_string(m,fmt,b"Found %d transcriptions for '%s': %s\0");
        m.put_word(scratch+2160,n);
        let vt=m.word(o);
        h.call(m,m.word(vt+16),&[o.wrapping_sub(m.word(vt+40)),a,fmt,scratch+2160,b,scratch])
    })
}

pub fn fn_4a77ac(m:&mut Memory<'_>,archive:u32,allocator:u32,name:u32,mode:u32,result:u32,scratch:u32,env:&crate::speech_part_1::ArchiveEnvironment,h:&mut impl SpeechHost)->u32 {
    sp3_frame(m,scratch,|m| {
        let file=h.call(m,m.word(m.word(allocator)+8),&[allocator,16,4]);
        m.put_word(result,file);m.put_word(file+12,allocator);
        if name==0 || mode==0 || archive==0 {return 0x80000007;}
        if m.byte(mode)!=b'r' || crate::speech_part_4::fn_1012cc(m,name,46)!=0 {return 0x80000000;}
        let mut i=0;
        while m.byte(name+i)!=0 {
            let c=m.byte(name+i);sp3_byte(m,scratch+i,if c==b'/' {b'_'} else {c});i+=1;
        }
        sp3_byte(m,scratch+i,0);
        sp3_string(m,scratch+256,b".dat\0");
        let n=crate::speech_part_4::fn_302edc(m,scratch);
        let count=if n.wrapping_add(4)>=256 {255u32.wrapping_sub(n)} else {4};
        crate::speech_part_4::fn_100c22(m,scratch,scratch+256,count);
        sp3_byte(m,scratch+255,0);
        let data=crate::speech_part_1::fn_16b554(m,archive+8,0,scratch,file+8,scratch+512,env);
        m.put_word(file,data);
        if data!=0 {m.put_word(file+4,data);return 0;}
        h.call(m,m.word(m.word(allocator)+12),&[allocator,file]);
        0x80000014
    })
}
fn sp3_convert(m:&mut Memory<'_>,engine:u32,src:u32,dst:u32,h:&mut impl SpeechHost)->u8 {
    let kind=m.word(src);let format=m.word(src+4);
    let mapped=match (kind,format) {(0,0|2)=>Some(1),(1,1)=>Some(2),(3,1)=>Some(8),(2,3|2)=>Some(4),_=>None};
    let mut status=0;
    if let Some(v)=mapped {sp3_byte(m,dst,v)} else {status=crate::speech_part_5::fn_1b4238(m,0) as u8;}
    m.put_word(dst+4,m.word(src+24));m.put_word(dst+8,m.word(src+28));
    if kind==2 {
        if format==3 {status=crate::normalization::code_4(0) as u8;}
        else {
            m.put_word(dst+8,m.word(src+28));
            let p=crate::boot_part_f::fn_1b2140(m,m.word(engine+56),m.word(dst+8),&mut status,h);
            m.put_word(dst+4,p);
            if status==0 {sp3_copy(m,p,m.word(src+24),m.word(dst+8));}
        }
    } else if kind==0 {
        let mut text=m.word(src+24);let mut size=m.word(src+28);
        while m.byte(text)==b' ' {text+=1;size=size.wrapping_sub(1);}
        let n=if size>=3 && m.byte(text+size-2)==0 {size-1}
            else if size>=2 && m.byte(text+size-1)==0 {size} else {size.wrapping_add(1)};
        if n<=size {m.put_word(dst+8,n);}
        let copy=crate::boot_part_f::fn_1b2140(m,m.word(engine+56),n,&mut status,h);
        if status==0 {
            sp3_copy(m,copy,text,size.min(n));
            if n>size {sp3_byte(m,copy+size,0);}
            m.put_word(dst+4,copy);if n>size {m.put_word(dst+8,n);}
        }
    }
    status
}

pub fn fn_2b3608(m:&mut Memory<'_>,engine:u32,input:u32,out:u32,out_count:u32,scratch:u32,h:&mut impl SpeechHost)->u32 {
    sp3_frame(m,scratch,|m| {
        let mut config=m.word(engine+68);let mut status=0u8;
        if m.word(config)==0 {
            let r=crate::speech_part_5::fn_2b24e8(m,engine,h);if r!=0 {return r as u8 as u32;}
        }
        config=m.word(engine+68);
        let params=scratch+3000;
        for (i,v) in [u32::MAX,u32::MAX,0,0,m.word(config+8)].iter().enumerate() {m.put_word(params+i as u32*4,*v);}
        let name=m.word(config+4);
        if name!=0 {
            let p=crate::boot_part_f::fn_1b2140(m,m.word(engine+56),129,&mut status,h);
            m.put_word(params+12,p);
            if status!=0 {return status as u32;}
            let at=m.offset(p);m.bytes[at..at+129].fill(0);
            sp3_copy(m,p,name,sp3_len(m,name));
        }
        if m.word(config)!=0 {m.put_word(params+8,m.word(config));}
        let count_slot=scratch+3024;m.put_word(count_slot,0);
        let r=fn_2e3e9c(m,m.word(engine+60),input,params,engine+80,count_slot,h);
        if r!=0 {return r as u8 as u32;}
        let count=m.word(count_slot);let mut items=0;let mut i=0;
        if count!=0 {
            items=crate::boot_part_f::fn_1b2140(m,m.word(engine+56),count.wrapping_mul(12),&mut status,h);
            if status!=0 {return status as u32;}
            while i<count {
                status=sp3_convert(m,engine,m.word(engine+80)+i*32,items+i*12,h);
                if status!=0 {return status as u32;}i+=1;
            }
            m.put_word(engine+84,count);m.put_word(engine+88,i);
            if i==0 {crate::boot_part_f::fn_1b2128(m,m.word(engine+56),items,h);items=0;}
        }
        if m.word(params+12)!=0 {crate::boot_part_f::fn_1b2128(m,m.word(engine+56),m.word(params+12),h);}

        let r=fn_2e4260(m,m.word(engine+76),items,i,out,out_count,scratch,h);
        if r!=0 {return r;}
        if items!=0 {
            let mut j=0;
            while j<m.word(engine+88) {
                let item=items+j*12;
                if matches!(m.byte(item),1|4) {crate::boot_part_f::fn_1b2128(m,m.word(engine+56),m.word(item+4),h);}
                j+=1;
            }
            crate::boot_part_f::fn_1b2128(m,m.word(engine+56),items,h);
            let r=crate::speech_part_5::fn_2e3f2c(m,m.word(engine+60),m.word(engine+80),m.word(engine+84),h);
            if r!=0 {return r;}
            m.put_word(engine+80,0);m.put_word(engine+84,0);
        }
        fn_2e19dc(m,m.word(engine+20),0x3e62b2cd,input,m.word(out),m.word(out_count),scratch,h)
    })
}

pub type RegexEntry<H> = fn(&mut H,u32,u32,&mut crate::regex::FrameStore<'_>,&[u8],
    Option<&crate::regex::ExecExtra<'_>>,&[u8],usize,usize,i32,u32,Option<&mut[i32]>,i32,
    &crate::regex::PcreTables<'_>,Option<crate::regex::CalloutFn>)->i32;

pub trait SpeechRound4Host: SpeechHost + crate::frontend::HeapVtableM {
    type RegexHost: crate::regex::PcreHost;
    fn regex_exec(&mut self, m:&mut Memory<'_>, args:&[u32], offsets:&mut [i32;100],
        exec:RegexEntry<Self::RegexHost>,callout:fn(&mut Memory<'_>,u32,&mut Self)->u32)->i32 where Self:Sized;

    fn literal(&self, address:u32)->u32;

    fn pool_name(&self, pool:u32)->u32;
}

pub struct FilterWorkspace<'a> {
    pub nodes:&'a mut [crate::selection::DecisionNode],
    pub coefficients:&'a mut [i32],
    pub input:&'a mut [i32],
    pub output:&'a mut [i32],
}
fn sp3_half(m:&Memory<'_>,p:u32)->u16 {let i=m.offset(p);u16::from_le_bytes(m.bytes[i..i+2].try_into().unwrap())}
struct Round4Normalize<'a,H>(&'a mut H);
impl<H:SpeechRound4Host> crate::normalization::NormalizationHost for Round4Normalize<'_,H> {
    fn calloc(&mut self,b:&mut[u8],heap:u32,n:u32,size:u32)->Option<u32>{
        let r=sp4_alloc(&mut Memory{base:0,bytes:b},heap,n,size,self.0);
        if r==0 {None} else {Some(r)}
    }
    fn alloc(&mut self,b:&mut[u8],heap:u32,n:u32)->Option<u32>{
        let p=crate::frontend::heap_alloc_m(b,if heap==0 {None}else{Some(heap)},n,self.0);
        if p==0 {None}else{Some(p)}
    }
    fn realloc(&mut self,b:&mut[u8],heap:u32,p:u32,n:u32)->Option<u32>{
        let r=crate::frontend::heap_realloc_m(b,if heap==0 {None}else{Some(heap)},p,n,self.0);
        if r==0 {None}else{Some(r)}
    }
    fn free(&mut self,b:&mut[u8],heap:u32,p:u32){
        crate::frontend::heap_release_m(b,if heap==0{None}else{Some(heap)},if p==0{None}else{Some(p)},self.0);
    }
    fn resolve(&mut self,b:&mut[u8],a:u32,c:u32)->(u32,u32){
        let mut m=Memory{base:0,bytes:b};let out=self.0.scratch();
        let old=m.word(out);let r=self.0.call(&mut m,0x19611c,&[a,c,out]);let p=m.word(out);m.put_word(out,old);(r,p)
    }
}
fn sp4_alloc(m:&mut Memory<'_>,heap:u32,n:u32,size:u32,h:&mut impl SpeechRound4Host)->u32 {
    struct Method<'a,'b,H>{m:Memory<'a>,h:&'b mut H,method:u32}
    impl<H:SpeechRound4Host> crate::frontend::MethodTable for Method<'_,'_,H> {
        fn call_at(&mut self,obj:u32,n:u32,size:u32)->u32 {self.h.call(&mut self.m,self.method,&[obj,n,size])}
    }
    let mut local=[0u8;8];
    if heap!=0 {local[4..].copy_from_slice(&m.word(heap+4).to_le_bytes());}
    let method=if heap==0 {0}else{m.word(m.word(heap)+4)};
    crate::frontend::call_method(&local,if heap==0{None}else{Some(0)},n,size,
        &mut Method{m:Memory{base:m.base,bytes:m.bytes},h,method})
}
fn sp4_free(m:&mut Memory<'_>,heap:u32,p:u32,h:&mut impl SpeechRound4Host){
    assert_eq!(m.base,0);
    crate::frontend::heap_release_m(m.bytes,if heap==0{None}else{Some(heap)},if p==0{None}else{Some(p)},h);
}
struct Round4Format<'a,H>(&'a mut H);
impl<H:SpeechRound4Host> crate::boot_part_b::Host for Round4Format<'_,H>{
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call(m,f,a)}
    fn thread_id(&mut self)->u32 {0}
}
impl<H:SpeechRound4Host> crate::boot_part_b::FormatHost for Round4Format<'_,H>{}
fn sp4_log(m:&mut Memory<'_>,p:u32,level:u32,site:u32,args:&[u32],h:&mut impl SpeechRound4Host){
    let s=h.scratch();let at=m.offset(s);let mut saved=[0u8;160];saved.copy_from_slice(&m.bytes[at..at+160]);
    for (i,&v) in args.iter().enumerate(){m.put_word(s+32+i as u32*4,v);}
    let pitch=(0x2a5500..0x2a6000).contains(&site);
    let logger=m.word(m.word(p+if pitch {24}else{0})+16);
    let module=if pitch {h.literal(0x8fa510)}else{m.word(p+72)};
    if level==u32::MAX {
        let format=if matches!(site,0x5df0|0x5df1|0x5dc2) || (site==0x5dc1 && args.is_empty()) {0}else{h.format(site)};
        crate::render_part_a::error_log(m,&mut Round4Format(h),logger,module,site,format,s+32,s);
    } else {
        let fmt=h.format(site);
        crate::log_live::fn_1738e8(m,logger,module,level,0,fmt,s+32,h);
    }
    m.bytes[at..at+160].copy_from_slice(&saved);
}

pub fn fn_2b8f08(m:&mut Memory<'_>,p:u32,scratch:u32,h:&mut impl SpeechRound4Host)->u32 {
    assert_eq!(m.base,0,"normalization ports use absolute arena offsets");
    sp3_frame(m,scratch,|m|{
        let mut end=0u16;let mut position=0;let mut value=0;let mut rc=0;let mut index=0u16;
        m.put_word(scratch+4,0);m.put_word(scratch+8,0);
        if p==0 || m.word(p+0x5c)==0 {
            sp4_log(m,p,u32::MAX,0x5df1,&[],h);
            return crate::normalization::error_code(7);
        }
        if m.byte(p+0xc6)==1 && crate::normalization::item_value(m.bytes,p)!=0 {
            let label=h.literal(0x2b8f08);
            crate::speech_part_5::fn_24e004(m,p,label,0,&mut Round5Speech{h,scratch:scratch+256});
        }
        while index<sp3_half(m,p+0x60) {
            let list=m.word(p+0x44);
            crate::normalization::find_entry(m.bytes,m.word(list+4),sp3_half(m,list+8),index as i32,&mut position,&mut value);
            if position==0 || sp3_half(m,position)!=index {index=index.wrapping_add(1);}
            else {
                crate::normalization::lookup_default(m.bytes,m.word(list+4),sp3_half(m,list+8),sp3_half(m,position) as i16,sp3_half(m,position+2),&mut end);
                let start=sp3_half(m,position);
                m.put_half(scratch,start);m.put_half(scratch+2,end);
                crate::normalization::remove_range(m.bytes,list,sp3_half(m,list+8),m.word(list+4),start,end as i32);
                rc=crate::normalization::collect_matches(m.bytes,p,scratch,&mut Round4Normalize(h));
                if rc>=0x80000000 {return rc;}
                if sp3_half(m,p+0x86)>1 {
                    sp4_log(m,p,5,0x2b8f58,&[sp3_half(m,p+0x86) as u32,start as i16 as u32,end as i16 as u32],h);
                    if m.byte(p+0xc6)==1 {
                        index=0;
                        while index<sp3_half(m,p+0x86) {
                            let matched=m.word(m.word(p+0x80)+index as u32*4);
                            crate::normalization::block_field(m.bytes,p,m.word(matched+84),scratch+8);
                            sp4_log(m,p,5,0x2b8fb0,&[index as u32,m.word(scratch+8)],h);
                            index=index.wrapping_add(1);
                        }
                    }
                    rc=crate::normalization::resolve_priority(m.bytes,scratch+4,p,&mut Round4Normalize(h));
                    if rc>=0x80000000 {return rc;}
                    index=0;
                    while index<sp3_half(m,p+0x86) {
                        if index!=sp3_half(m,scratch+4) {
                            sp4_log(m,p,5,0x2b9000,&[index as u32],h);
                            let list=m.word(p+0x44);let matched=m.word(m.word(p+0x80)+index as u32*4);
                            crate::normalization::remove_entry(m.bytes,list,m.word(list+4),sp3_half(m,list+8) as i16,start as i16,end as i16,m.word(matched+84) as i32);
                        }
                        index=index.wrapping_add(1);
                    }
                }
                rc=crate::normalization::grow_slots_second(m.bytes,p,0,&mut Round4Normalize(h));
                if rc>=0x80000000 {return rc;}
                index=end;
            }
            position=0;
        }
        if m.byte(p+0xc6)==1 && crate::normalization::item_value(m.bytes,p)!=0 {
            let label=h.literal(0x2b9060);
            crate::speech_part_5::fn_24e004(m,p,label,0,&mut Round5Speech{h,scratch:scratch+256});
        }
        rc
    })
}

pub fn fn_2b7118(m:&mut Memory<'_>,p:u32,o:u32,scratch:u32,h:&mut impl SpeechRound4Host)->u32 {
    assert_eq!(m.base,0);
    sp3_frame(m,scratch,|m|{
        let mut ret=0;let end=sp3_half(m,o+20);let mut i=0i32;let mut buf=[0i32;100];
        sp3_byte(m,scratch,1);
        loop {
            if m.word(m.word(o+4)+24)!=0 {break;}
            i=h.regex_exec(m,&[m.word(p+48),m.word(p+52),m.word(o+12),m.word(o),m.word(p+92),sp3_half(m,o+18) as u32,end as u32,sp3_half(m,o+28) as u32],&mut buf,
                |host,o,k,store,program,extra,subject,start,len,offset,options,captures,count,tables,callout|
                    crate::regex::pcre_exec(host,o,k,store,program,extra,subject,start,len,offset,options,captures,count,tables,callout),
                |m,p,h|crate::speech_part_5::fn_2b6afc(m,p,h));
            if i<0 {break;}
        }
        if i<0 {
            ret=crate::normalization::err_code(i as u32);
            if ret>=0x80000000 && ((ret>>20)&0xfffff7ff)==0xa5 {
                let low=ret&0x1fff;
                if low==10 {sp4_log(m,p,u32::MAX,0x2afa,&[h.literal(0x2b7120),ret,h.literal(0x2b7124),m.word(o+8)],h);return ret;}
                if low!=20 {sp4_log(m,p,u32::MAX,0x2b13,&[h.literal(0x2b7120),ret,h.literal(0x2b7124),m.word(o+8)],h);return ret;}
                ret=0;
            }
        }
        let inner=m.word(o+4);let err=m.word(inner+24);
        if err>=0x80000000 {return err;}
        let ps=m.word(inner+4);
        if sp3_half(m,ps)!=0 || sp3_half(m,ps+2)!=0 {
            sp3_byte(m,scratch,1);
            if m.byte(inner+22)==1 {sp4_log(m,p,5,0x2b7190,&[m.word(o+8)],h);}
            else {
                ret=crate::speech_part_5::fn_2e5adc(m,p,m.word(inner),ps,m.word(p+92),m.word(inner+12),m.word(o+24),scratch,&mut Round5Speech{h,scratch:scratch+256});
                if ret>=0x80000000 {return ret;}
                if m.byte(scratch)!=1 {
                    let record=m.word(m.word(o+4));m.put_half(record+40,0);
                    sp4_log(m,p,5,0x2b71d0,&[m.word(o+8)],h);return ret;
                }
            }
            let inner=m.word(o+4);m.put_word(m.word(inner)+84,m.word(o+24));
            ret=crate::normalization::insert_node(m.bytes,p,m.word(inner+4),m.word(inner),&mut Round4Normalize(h));
            if ret>=0x80000000 {return ret;}
        }
        ret
    })
}
fn sp4_at(m:&Memory<'_>,table:u32,j:i32,i:i32)->u32 {m.word(table.wrapping_add((j as u32).wrapping_mul(4))).wrapping_add((i as u32).wrapping_mul(4))}
fn sp4_int(m:&Memory<'_>,p:u32)->i32 {m.word(p) as i32}
fn sp4_band(m:&mut Memory<'_>,p:u32,stage:u32,w:&mut FilterWorkspace<'_>)->bool {
    let n=sp4_int(m,p+32).max(0) as usize;let width=sp4_int(m,p+36).max(1) as usize;
    assert!(w.coefficients.len()>=n*width && w.input.len()>=n && w.output.len()>=n);
    let rows=m.word(p+60);
    for i in 0..n {for j in 0..width {w.coefficients[i*width+j]=sp4_int(m,m.word(rows+i as u32*4)+j as u32*4);}}
    if stage==0 {
        let ok=crate::selection::BandFactorization{coefficients:&mut w.coefficients[..n*width],count:n,stride:width}.factor_three(18).expect("factor input");
        for i in 0..n {for j in 0..width {m.put_word(m.word(rows+i as u32*4)+j as u32*4,w.coefficients[i*width+j] as u32);}}
        return ok;
    }
    let source=if stage==1||stage==3 {m.word(p+64)}else{m.word(p+56)};
    for i in 0..n {w.input[i]=sp4_int(m,source+i as u32*4);}
    let sys=crate::selection::BandSystem{coefficients:&w.coefficients[..n*width],stride:width,width,input:&w.input[..n]};
    match stage {
        1=>sys.forward_three(&mut w.output[..n]),
        2=>sys.backward_lattice(28,&mut w.output[..n]),
        3=>sys.forward(20,18,22,&mut w.output[..n]),
        4=>sys.backward(28,22,18,&mut w.output[..n]),
        _=>unreachable!(),
    }.expect("band solve");
    let dest=if stage==1||stage==3 {m.word(p+56)}else{m.word(p+44)};
    for i in 0..n {m.put_word(dest+i as u32*4,w.output[i] as u32);}
    true
}
fn sp4_factor(m:&mut Memory<'_>,context:u32,p:u32,w:&mut FilterWorkspace<'_>,h:&mut impl SpeechRound4Host){
    if !sp4_band(m,p,0,w) {
        let format=h.format(0x2a1c54);let category=h.literal(0x8fa510);
        crate::log_live::fn_1738e8(m,m.word(context+16),category,0,0,format,0,h);
    }
}

pub fn fn_2a27fc(m:&mut Memory<'_>,context:u32,p:u32,w:&mut FilterWorkspace<'_>,h:&mut impl SpeechRound4Host){
    if m.word(p+32)==0 {return;}
    m.put_word(p+56,m.word(m.word(p+48)));
    let mut out=m.word(p+44);m.put_word(p+64,out);
    if m.word(p+36)==3 {
        let mut i=0;
        while i<sp4_int(m,p+32) {
            let scale=sp4_int(m,sp4_at(m,m.word(p+52),0,i));
            let val=sp4_int(m,sp4_at(m,m.word(p+48),0,i));
            m.put_word(out,(scale>>10).wrapping_mul(val>>10) as u32);
            let row=m.word(m.word(p+60)+i as u32*4);
            m.put_word(row+8,0);m.put_word(row,(scale>>2) as u32);m.put_word(row+4,0);
            let mut j=1;
            while j<sp4_int(m,m.word(p+40)+24) {
                let bank=m.word(p+40);let weight=m.word(m.word(bank+32)+j as u32*4);
                for delta in [-1i32,0,1] {
                    let k=i+delta;let coefficient=sp4_int(m,weight.wrapping_sub((delta as u32).wrapping_mul(4)));
                    if coefficient!=0 && k>=0 && k<sp4_int(m,p+32) {
                        let product=(sp4_int(m,sp4_at(m,m.word(p+52),j,k))>>10).wrapping_mul(coefficient);
                        let value=product>>11;
                        let add=(product>>10).wrapping_mul(sp4_int(m,sp4_at(m,m.word(p+48),j,k))>>10);
                        m.put_word(out,m.word(out).wrapping_add(add as u32));
                        for col in 0..=(delta+1) {
                            if i+col<sp4_int(m,p+32) {
                                let c=sp4_int(m,sp4_at(m,m.word(bank+36),j,col-delta));
                                let addr=row+col as u32*4;
                                m.put_word(addr,m.word(addr).wrapping_add(value.wrapping_mul(c) as u32));
                            }
                        }
                    }
                }
                j+=1;
            }
            i+=1;out=out.wrapping_add(4);
        }
    } else {crate::speech_part_5::fn_2a178c(m,p,20,18,out);}
    if m.word(p+36)==3 {
        sp4_factor(m,context,p,w,h);sp4_band(m,p,1,w);sp4_band(m,p,2,w);
    } else {
        crate::speech_part_5::fn_2a19ac(m,context,p,18,h);
        sp4_band(m,p,3,w);sp4_band(m,p,4,w);
    }
}
struct Round4Pool<'a,'b,H>{m:Memory<'a>,h:&'b mut H,name:u32}
impl<H:SpeechRound4Host> crate::selection::PoolHost for Round4Pool<'_,'_,H>{
    fn block(&self,i:usize)->crate::selection::PoolBlock {
        let p=i as u32;let n=self.m.word(p);
        crate::selection::PoolBlock{next:if n==0{None}else{Some(n as usize)},base:self.m.word(p+4),current:self.m.word(p+8),end:self.m.word(p+12)}
    }
    fn set_block(&mut self,i:usize,b:crate::selection::PoolBlock){
        let p=i as u32;self.m.put_word(p,b.next.unwrap_or(0) as u32);self.m.put_word(p+4,b.base);self.m.put_word(p+8,b.current);self.m.put_word(p+12,b.end);
    }
    fn allocate(&mut self,owner:u32,n:u32)->Option<(usize,u32)> {
        let heap=self.m.word(owner+4);let p=sp4_alloc(&mut self.m,heap,1,n,self.h);
        if p==0 {None}else{Some((p as usize,p))}
    }
    fn free(&mut self,owner:u32,i:usize){let heap=self.m.word(owner+4);sp4_free(&mut self.m,heap,i as u32,self.h);}
    fn clear(&mut self,p:u32,n:u32){let at=self.m.offset(p);self.m.bytes[at..at+n as usize].fill(0);}
    fn report(&mut self,heap:&crate::selection::PrivateHeap<'_>,label:&[u8],blocks:u32){
        let logger=self.m.word(heap.owner+16);let format=self.h.format(0x2db308);
        let scratch=self.h.scratch();let at=self.m.offset(scratch);let mut saved=[0u8;24];
        saved.copy_from_slice(&self.m.bytes[at..at+24]);
        let site=if label==b"before ResetPool" {0x2db3c8}else{0x2db4a0};
        for (i,value) in [self.name,self.h.literal(site),heap.allocated,heap.pool,heap.expected,blocks].into_iter().enumerate(){
            self.m.put_word(scratch+i as u32*4,value);
        }
        crate::log_live::fn_1738e8(&mut self.m,logger,self.h.literal(0x917328),3,0,format,scratch,self.h);
        self.m.bytes[at..at+24].copy_from_slice(&saved);
    }
}
fn sp4_pool(m:&mut Memory<'_>,f:u32,p:u32,owner:u32,name:u32,n:u32,h:&mut impl SpeechRound4Host)->u32 {
    if f==0x2db19c {
        let name=if name==u32::MAX {h.pool_name(p)}else{name};
        let heap=crate::selection::PrivateHeap::initialize(owner,b"");
        for (off,v) in [(0,heap.owner),(4,name),(8,0),(12,heap.expected),(16,heap.pool),(20,heap.allocated)] {m.put_word(p+off,v);}
        return 0;
    }
    if p==0 {return 0;}
    let first=m.word(p+8);
    let mut heap=crate::selection::PrivateHeap{owner:m.word(p),name:b"",first_block:if first==0{None}else{Some(first as usize)},expected:m.word(p+12),pool:m.word(p+16),allocated:m.word(p+20)};
    let name=m.word(p+4);let mut adapter=Round4Pool{m:Memory{base:m.base,bytes:m.bytes},h,name};
    let result=match f {
        0x2db1d0=>heap.allocate(n,&mut adapter).map_or(0,|v|v.address),
        0x2db3c8=>{heap.reset(&mut adapter);0},
        0x2db4a0=>{heap.free(&mut adapter);0},
        _=>unreachable!(),
    };
    for (off,v) in [(8,heap.first_block.unwrap_or(0) as u32),(12,heap.expected),(16,heap.pool),(20,heap.allocated)] {adapter.m.put_word(p+off,v);}
    result
}
struct Round4Selection<'a,'b,'c,H>{h:&'a mut H,w:&'b mut FilterWorkspace<'c>,scratch:u32}
impl<H:SpeechRound4Host> crate::selection::arena::Host for Round4Selection<'_, '_, '_,H> {
    fn call(&mut self,a:&mut crate::selection::arena::Arena<'_>,f:u32,args:&[u32])->(u32,u32){
        (sp4_dispatch(&mut Memory{base:a.base,bytes:a.bytes},f,args,self.scratch,self.w,self.h),0)
    }
}

pub(crate) fn sp4_tree(m:&Memory<'_>,env:u32,pos:u32,rtab:u32,ridx:u32,ent:u32,w:&mut FilterWorkspace<'_>)->u32 {
    let table=rtab+ridx*4;let questions=m.word(table+16);let base=m.word(m.word(table+36)+ent*8+4);
    let mut count=1usize;let mut i=0;
    while i<count {
        assert!(i<w.nodes.len(),"caller decision-node workspace");
        let p=base+i as u32*6;
        let miss=sp3_half(m,p+2) as i16;let hit=sp3_half(m,p+4) as i16;
        w.nodes[i]=crate::selection::DecisionNode{question:sp3_half(m,p),miss,hit};
        if miss<0 {count=count.max(-(miss as i32) as usize+1);}
        if hit<0 {count=count.max(-(hit as i32) as usize+1);}
        i+=1;
    }
    let mut features=crate::selection::SelectionFeatures{count:sp3_half(m,env+6) as usize,columns:[None;60]};

    for j in 0..60 {let p=m.word(env+12+j*4);if p!=0 {features.columns[j as usize]=Some(&m.bytes[m.offset(p)..]);}}
    crate::selection::DecisionTree{questions:&m.bytes[m.offset(questions)..],nodes:&w.nodes[..count]}.select(features,pos as usize).expect("decision tree") as u32
}

impl<H:SpeechRound4Host> crate::boot_part_e::Host for Round4Format<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call(m,f,a)}
}

pub(crate) fn sp4_cache(m:&mut Memory<'_>,p:u32,offset:u32,len:u32,out:u32,h:&mut impl SpeechRound4Host)->u32 {
    if out!=0 {m.put_word(out,0);}
    let held=m.word(p+12);let pos=m.word(p+8);let size=m.word(p+16);
    if held!=0 && pos<=offset && offset.wrapping_add(len)<=pos.wrapping_add(size) {
        let at=m.offset(held);
        let mut cache=crate::selection::BlockCache{data:Some(&m.bytes[at..at+size as usize]),position:pos,length:size};
        cache.read(offset,len,|_|unreachable!(),|_,_|unreachable!()).expect("cached window");
        if out!=0 {m.put_word(out,held.wrapping_add(offset).wrapping_sub(pos));}
        return 0;
    }
    if held!=0 {
        let e=crate::boot_part_e::UnmapEnvironment{category:h.literal(0x179208),begin_format:h.literal(0x17920c),end_format:h.literal(0x179210),
            error_format:h.literal(0x179214),labels:[h.literal(0x179218),h.literal(0x17921c),h.literal(0x179220)],scratch:h.scratch()};
        let status=crate::boot_part_e::fn_179208(m,m.word(p),m.word(p+4),held,&e,&mut Round4Format(h));
        m.put_word(p+8,0);m.put_word(p+12,0);m.put_word(p+16,0);
        if status>=0x80000000 {return status;}
    }
    let status=h.call(m,0x17a1a4,&[m.word(p),m.word(p+4),offset,len,p+12]);
    if status>=0x80000000 {return status;}
    let data=m.word(p+12);let at=m.offset(data);
    let mut cache=crate::selection::BlockCache::default();
    cache.read(offset,len,|_|unreachable!(),|_,_|Ok(&m.bytes[at..at+len as usize])).expect("loaded window");
    let position=cache.position;let length=cache.length;
    m.put_word(p+8,position);m.put_word(p+16,length);
    if out!=0 {m.put_word(out,m.word(p+12));}
    status
}

fn sp4_acoustic(m:&mut Memory<'_>,a:&[u32],scratch:u32,h:&mut impl SpeechRound4Host)->u32 {
    let table=a[1];let pointer=m.word(table+112);
    let mut data=pointer;let mut status=0;
    if data==0 {
        let block=m.word(m.word(table+116)+a[2]*4);
        let offset=block.wrapping_add(a[3].wrapping_mul(sp3_half(m,table+16) as u32));
        status=sp4_cache(m,a[0],offset,sp3_half(m,table+16) as u32,scratch+80,h);
        if status>=0x80000000 {return status;}
        data=m.word(scratch+80);
    }
    let mut shifts=[[0u16;3];3];let mut scales=shifts;let mut offsets=[[0i32;3];3];
    for i in 0..3 {for j in 0..3 {
        shifts[i][j]=sp3_half(m,table+32+(i*3+j) as u32*2);
        scales[i][j]=sp3_half(m,table+50+(i*3+j) as u32*2);
        offsets[i][j]=sp4_int(m,table+68+(i*3+j) as u32*4);
    }}
    let block=if pointer==0 {0}else{m.word(m.word(table+116)+a[2]*4)};
    let blocks=[block];let mut out=[0i32;3];
    let table=crate::selection::AcousticTable{width:m.word(table+4),record_size:if pointer==0{0}else{sp3_half(m,table+16)},shifts,scales,offsets,offset_shift:sp3_half(m,table+104),data:Some(&m.bytes[m.offset(data)..]),data_origin:0,blocks:&blocks};
    table.read_three(0,if pointer==0 {0}else{a[3]},a[4] as usize,a[8],&mut out,|_,_|unreachable!()).expect("acoustic table");
    for i in 0..3 {m.put_word(a[5+i],out[i] as u32);}
    status
}
fn sp4_dispatch(m:&mut Memory<'_>,f:u32,a:&[u32],scratch:u32,w:&mut FilterWorkspace<'_>,h:&mut impl SpeechRound4Host)->u32 {
    use crate::selection::arena_mid as mid;
    if matches!(f,0x2db19c|0x2db1d0|0x2db3c8|0x2db4a0) {
        return sp4_pool(m,f,a[0],*a.get(1).unwrap_or(&0),*a.get(2).unwrap_or(&0),*a.get(1).unwrap_or(&0),h);
    }
    if f==0x2f850c {return sp4_acoustic(m,a,scratch,h);}
    if f==0x2d9dc0 {return sp4_tree(m,a[0],a[1],a[2],a[3],a[4],w);}
    if f==0x2da9e0 {fn_2da9e0(m,a[0]);return 0;}
    if f==0x2a27fc {fn_2a27fc(m,a[0],a[1],w,h);return 0;}
    if f==0x2a1c54 {sp4_factor(m,a[0],a[1],w,h);return 0;}
    if f==0x2a3d84 {sp4_band(m,a[0],1,w);return 0;}
    if f==0x2a23c4 {sp4_band(m,a[0],2,w);return 0;}
    if f==0x1738e8 {crate::log_live::fn_1738e8(m,a[0],a[1],a[2],a[3],a[4],0,h);return 0;}
    if f==0x196428 {sp4_free(m,a[0],a[1],h);return 0;}
    let mut arena=crate::selection::arena::Arena::new(m.base,m.bytes);
    let mut host=Round4Selection{h,w,scratch};
    match f {
        0x2a335c=>mid::prepare_input_tables(&mut arena,&mut host,a[0],a[1],a[2]),
        0x2a3214=>mid::prepare_gain_tables(&mut arena,&mut host,a[0],a[1] as i32,a[2] as i32,a[3] as i32,a[4],a[5]),
        0x2a4104=>mid::allocate_table(&mut arena,&mut host,a[0],a[1] as i32,a[2] as i32,a[3]),
        0x2a3498=>mid::prepare_filter_rows(&mut arena,&mut host,a[0],a[1],a[2],a[3]),
        0x2a3644=>mid::reset_workspace(&mut arena,&mut host,a[0],a[1]),
        0x2a2b88=>mid::run_backward_stream(&mut arena,&mut host,a[0],a[1]),
        0x2da518=>mid::select_voiced(&mut arena,&mut host,a[0],a[1],a[2],a[3],a[4],a[5] as i32,scratch+64),
        0x2da418=>mid::read_frame_params(&mut arena,&mut host,a[0],a[1],a[2],a[3],a[4],a[5] as i32,a[6],scratch+64),
        _=>host.h.call(&mut Memory{base:arena.base,bytes:arena.bytes},f,a),
    }
}
fn sp4_smooth(m:&mut Memory<'_>,stream:u32,w:&mut FilterWorkspace<'_>) {
    let n=sp4_int(m,stream+32).max(0) as usize;let mode=sp4_int(m,stream+28);
    if mode!=0 || n==0 {return;}
    assert!(w.input.len()>=n+4 && w.output.len()>=n && w.coefficients.len()>=n);
    let means=m.word(stream+48);let source=m.word(means).wrapping_sub(8);let middle=m.word(means+4);
    for i in 0..n+4 {w.input[i]=sp4_int(m,source+i as u32*4);}
    for i in 0..n {w.coefficients[i]=sp4_int(m,middle+i as u32*4);}
    crate::selection::smooth_features_wide(mode,&mut w.input[..n+4],&w.coefficients[..n],n,&mut w.output[..n]).expect("smooth features");
    for i in 0..n+4 {m.put_word(source+i as u32*4,w.input[i] as u32);}
    let dest=m.word(stream+44);
    for i in 0..n {m.put_word(dest+i as u32*4,w.output[i] as u32);}
}

pub fn fn_2a5578(m:&mut Memory<'_>,state:u32,model:u32,config:u32,scratch:u32,w:&mut FilterWorkspace<'_>,h:&mut impl SpeechRound4Host)->u32 {
    assert_eq!(m.base,0);
    sp3_frame(m,scratch,|m|{
        let stream=state+0x2b4;let work=state+0x3d4;let list=state+0x440;
        let offset=sp4_int(m,model+0x300);let offset_scaled=offset.wrapping_shl(8);
        let mut voiced=0i32;let mut pad=0u32;let mut remaining=0i32;
        let mut frame=0i32;let mut keep=0i32;let mut validity=1u8;
        let params=scratch+256;m.put_word(params+4,0);m.put_word(params+8,0);
        let full=m.byte(config+0x111)==3;
        m.put_word(stream+24,m.word(model+0x1fc));m.put_word(stream+28,0);
        let total=sp4_int(m,list+20).wrapping_add(sp4_int(m,state+0x3a9c));
        if m.word(list+4)!=m.word(state+0x460) {keep=sp4_int(m,state+0x3a90).min(total);}
        let context=m.word(state+24);let heap=m.word(context+4);
        for off in [0x430u32,0x42c] {let p=m.word(state+off);if p!=0 {sp4_free(m,heap,p,h);m.put_word(state+off,0);}}
        let flags=sp4_alloc(m,heap,total.wrapping_add(1) as u32,1,h);m.put_word(state+0x430,flags);
        let result=(||{
            if flags==0 {return 0x84e0200a;}
            let indices=sp4_alloc(m,heap,total.wrapping_add(1) as u32,4,h);m.put_word(state+0x42c,indices);
            if indices==0 {return 0x84e0200a;}
            m.put_word(stream+32,total as u32);
            let mut result;
            if full {result=sp4_dispatch(m,0x2a335c,&[context,stream,work],scratch,w,h);}
            else {
                m.put_word(stream+52,0);m.put_word(stream+48,0);
                if sp4_int(m,stream+24)>=2 {m.put_word(stream+24,2);}
                pad=2;
                result=fn_2a76a4(m,heap,m.word(stream+24),m.word(stream+32),pad,work+80,h);
                if result==0 {
                    m.put_word(work+60,m.word(stream+24));m.put_word(work+76,0);m.put_word(work+56,m.word(stream+32));
                    m.put_word(stream+48,m.word(work+80));
                }
            }
            if result>=0x80000000 {return result;}
            let means=sp4_alloc(m,heap,m.word(model+0x1fc),4,h);m.put_word(params+4,means);
            if means==0 {return 0x84e0200a;}
            m.put_word(params+4,means.wrapping_sub(4));
            let vars=sp4_alloc(m,heap,m.word(model+0x1fc),4,h);m.put_word(params+8,vars);
            if vars==0 {return 0x84e0200a;}
            m.put_word(params+8,vars.wrapping_sub(4));
            if m.word(list)!=m.word(state+0x45c) && sp4_int(m,state+0x3a9c)>0 {
                loop {
                    let hist=state+0x3818+(sp4_int(m,state+0x3a9c)-frame) as u32*28;
                    let flag=m.byte(hist+24);sp3_byte(m,m.word(state+0x430)+frame as u32,flag);m.put_word(m.word(state+0x42c)+frame as u32*4,0);
                    if flag&2!=0 {
                        for i in 0..sp4_int(m,stream+24) {
                            let to=sp4_at(m,m.word(stream+48),i,voiced);m.put_word(to,m.word(hist+i as u32*4));
                            if full {let to=sp4_at(m,m.word(stream+52),i,voiced);m.put_word(to,m.word(hist+12+i as u32*4));}
                        }
                        voiced+=1;
                    }
                    frame+=1;if frame>=sp4_int(m,state+0x3a9c) {break;}
                }
            }
            let prior=voiced;let mut node=m.word(list);
            while m.word(list+4)!=node {
                let boundary=sp3_half(m,node+4)==0 || m.byte(m.word(state+0x50)+(sp3_half(m,node+4) as i16 as i32) as u32)==1;
                let mut position=2i32;let mut j=0u32;
                while position<sp4_int(m,model+0x1f8).wrapping_add(2) {
                    let selected=sp4_dispatch(m,0x2d9dc0,&[state+0x3c,sp3_half(m,node+4) as i16 as u32,model+0x440,1,j],scratch,w,h);
                    if sp4_int(m,config+44)>20 {sp4_log(m,state,4,0x2a5578,&[selected],h);}
                    sp3_byte(m,params,0);
                    let mode=sp3_half(m,config+2) as i16;
                    let a=[m.word(state+32),position as u32,selected,params,model+0x1e0,m.word(config+32),mode as u32];
                    result=sp4_dispatch(m,if mode==0{0x2da518}else{0x2da418},&a,scratch,w,h);
                    if result>=0x80000000 {return result;}
                    if m.byte(node+20)&4!=0 && m.byte(params)&2!=0 && m.byte(config+0x42)==b'q' {
                        let p=m.word(params+4)+4;let mut pitch=sp4_int(m,p);let maximum=pitch.wrapping_add(pitch>>10);
                        let var=sp4_int(m,m.word(params+8)+4);
                        if var>0x400 {pitch=pitch.wrapping_add(crate::runtime::signed_division(0xa00000,var>>10).expect("pitch division").quotient.wrapping_mul(0x400));}
                        m.put_word(p,pitch.min(maximum) as u32);
                    }
                    let p=m.word(params+4)+4;let pitch=sp4_int(m,p);let maximum=h.literal(0x666660) as i32;
                    if pitch>maximum {sp4_log(m,state,3,0x2a56f0,&[],h);m.put_word(p,maximum.wrapping_sub(offset) as u32);}
                    else if pitch<offset {sp4_log(m,state,3,0x2a5700,&[],h);m.put_word(p,0);}
                    else {m.put_word(p,pitch.wrapping_sub(offset) as u32);}
                    if boundary && position!=2 && sp4_int(m,model+0x1f8)+1!=position {sp3_byte(m,params,0);}
                    for _ in 0..sp3_half(m,m.word(node+12)+position as u32*2) {
                        let flag=m.byte(params);sp3_byte(m,m.word(state+0x430)+frame as u32,flag);m.put_word(m.word(state+0x42c)+frame as u32*4,0);
                        if flag&2!=0 {
                            for i in 0..sp4_int(m,stream+24) {
                                let p=m.word(params+4)+(i as u32+1)*4;
                                let value=sp4_int(m,p).clamp(-0x2ee00000,0x2ee00000);m.put_word(p,value as u32);
                                let to=sp4_at(m,m.word(stream+48),i,voiced);m.put_word(to,value as u32);
                                if full {let to=sp4_at(m,m.word(stream+52),i,voiced);m.put_word(to,m.word(m.word(params+8)+(i as u32+1)*4));
                                    m.put_word(to,sp4_int(m,to).clamp(-0x2ee00000,0x2ee00000) as u32);}
                            }
                            voiced+=1;
                        }
                        frame+=1;
                    }
                    position+=1;j+=1;
                }
                node=m.word(node+24);
            }
            for off in [4,8] {let p=m.word(params+off).wrapping_add(4);sp4_free(m,heap,p,h);m.put_word(params+off,0);}
            remaining=voiced;
            for j in 0..keep {
                let hist=state+0x3818+(j as u32+1)*28;let flag=m.byte(m.word(state+0x430)+(frame-j-1) as u32);sp3_byte(m,hist+24,flag);
                if flag&2!=0 {
                    remaining-=1;
                    for i in 0..sp4_int(m,stream+24) {
                        m.put_word(hist+i as u32*4,m.word(sp4_at(m,m.word(stream+48),i,remaining)));
                        if full {m.put_word(hist+12+i as u32*4,m.word(sp4_at(m,m.word(stream+52),i,remaining)));}
                    }
                } else {let at=m.offset(hist);m.bytes[at..at+24].fill(0);}
            }
            if voiced>0 {
                voiced=0;
                for j in 0..frame {
                    if m.byte(m.word(state+0x430)+j as u32)&2!=0 {
                        for i in 1..sp4_int(m,stream+24) {
                            let model=m.word(stream+40);let window=m.word(m.word(model+28)+i as u32*4);validity|=2;
                            for n in sp4_int(m,window)..=sp4_int(m,window+4) {
                                let k=j.wrapping_add(n);
                                if k<0 || k>=frame {validity&=!2;} else {validity&=m.byte(m.word(state+0x430)+k as u32);}
                            }
                            if validity&2==0 {
                                if full {let p=sp4_at(m,m.word(stream+52),i,voiced);m.put_word(p,0);}
                                let p=sp4_at(m,m.word(stream+48),i,voiced);m.put_word(p,0);
                            }
                        }
                        voiced+=1;
                    }
                }
                if voiced>0 {
                    m.put_word(stream+32,voiced as u32);m.put_word(stream+28,0);
                    if full {
                        let radius=m.word(m.word(stream+40)+48).wrapping_mul(2).wrapping_add(1);
                        result=sp4_dispatch(m,0x2a3214,&[context,voiced as u32,radius,m.word(stream+24),0,work],scratch,w,h);
                        if result>=0x80000000 {return result;}
                    }
                    let label=h.literal(0x91724d);
                    result=sp4_dispatch(m,0x2a3498,&[context,work,stream,label],scratch,w,h);
                    if result>=0x80000000 {return result;}
                    if m.byte(config+0x111)==2 {sp4_smooth(m,stream,w);}
                    else {sp4_dispatch(m,if m.word(model+0x488)==0{0x2a27fc}else{0x2a2b88},&[context,stream],scratch,w,h);}
                    for j in (0..voiced).rev() {
                        let p=m.word(stream+44)+j as u32*4;let mut value=(offset_scaled as u32).wrapping_add(m.word(p));
                        if value>=0x66666001 {sp4_log(m,state,3,0x2a59a0,&[],h);value=0x66666000;}
                        else if (value as i32)<0x3999a000 {sp4_log(m,state,3,0x2a59a0,&[],h);value=0x3999a000;}
                        m.put_word(p,value);
                    }
                }
            }
            let mut i=0i32;let mut j=0;
            while i<prior && j<7 {
                if m.byte(m.word(state+0x430)+j)&2!=0 {
                    let p=m.word(stream+44)+i as u32*4;
                    let weights=h.literal(0x91728c);
                    let a=sp4_int(m,weights+(j+7)*4).wrapping_mul(sp4_int(m,state+0x394c+i as u32*4).wrapping_add(4)>>3);
                    let b=sp4_int(m,p).wrapping_add(4)>>3;
                    m.put_word(p,a.wrapping_add(b.wrapping_mul(sp4_int(m,weights+j*4))) as u32);i+=1;
                }
                j+=1;
            }
            sp3_copy(m,state+0x394c,m.word(stream+44).wrapping_add((remaining as u32).wrapping_mul(4)),(voiced-remaining).wrapping_mul(4) as u32);
            result
        })();
        for off in [4,8] {let p=m.word(params+off);if p!=0 {sp4_free(m,heap,p.wrapping_add(4),h);}}
        if m.byte(config+0x111)==1 {
            let data=m.word(work+80);
            if data!=0 {sp4_free(m,heap,m.word(data).wrapping_sub(pad*4),h);sp4_free(m,heap,data,h);m.put_word(work+80,0);}
            m.put_word(stream+48,0);
        }
        sp4_dispatch(m,0x2a3644,&[stream,work],scratch,w,h);
        m.put_word(stream+32,remaining as u32);
        result
    })
}

pub fn fn_2da9e0(m:&mut Memory<'_>,p:u32) {
    let k=m.word(p+28).wrapping_add(1);
    let a=m.word(p+48);let b=m.word(p+52);let rows=m.word(p+60);let out=m.word(p+64);
    let a0=m.word(a);let a1=m.word(a+k*4);let a2=m.word(a+k*8);
    let b0=m.word(b);let b2=m.word(b+k*4);let b3=m.word(b+k*8);
    let g=|m:&Memory<'_>,p:u32,i:u32|m.word(p+i*4) as i32;
    let v=g(m,a0,0).wrapping_sub(g(m,a1,1)>>1).wrapping_sub(g(m,a2,0).wrapping_mul(2)).wrapping_add(g(m,a2,1));
    m.put_word(out,v as u32);
    let row=m.word(rows);
    m.put_word(row,(g(m,b0,0).wrapping_add(g(m,b3,1)).wrapping_add(g(m,b3,0).wrapping_mul(4)).wrapping_add(g(m,b2,1)>>2)>>2) as u32);
    m.put_word(row+4,(g(m,b3,0).wrapping_add(g(m,b3,1)).wrapping_mul(-2)>>2) as u32);
    m.put_word(row+8,(g(m,b3,1).wrapping_sub(g(m,b2,1)>>2)>>2) as u32);
    let mut i=1u32;
    while (i as i32)<(m.word(p+32) as i32).wrapping_sub(1) {
        let v=g(m,a0,i).wrapping_add(g(m,a1,i-1)>>1).wrapping_sub(g(m,a1,i+1)>>1)
            .wrapping_add(g(m,a2,i-1)).wrapping_sub(g(m,a2,i).wrapping_mul(2)).wrapping_add(g(m,a2,i+1));
        m.put_word(out+i*4,v as u32);
        let row=m.word(rows+i*4);
        m.put_word(row,(g(m,b0,i).wrapping_add(g(m,b2,i-1)>>2).wrapping_add(g(m,b2,i+1)>>2)>>2) as u32);
        let v=g(m,b3,i-1).wrapping_add(g(m,b3,i).wrapping_mul(4)).wrapping_add(g(m,b3,i+1))>>2;
        m.put_word(row,m.word(row).wrapping_add(v as u32));
        m.put_word(row+4,(g(m,b3,i).wrapping_add(g(m,b3,i+1)).wrapping_mul(-2)>>2) as u32);
        m.put_word(row+8,(g(m,b3,i+1)>>2) as u32);
        m.put_word(row+8,m.word(row+8).wrapping_sub((g(m,b2,i+1)>>4) as u32));
        i+=1;
    }
    m.put_word(out+i*4,g(m,a0,i).wrapping_add(g(m,a1,i-1)>>1).wrapping_add(g(m,a2,i-1)).wrapping_add(g(m,a2,i).wrapping_mul(2)) as u32);
    let row=m.word(rows+i*4);
    m.put_word(row,(g(m,b0,i).wrapping_add(g(m,b3,i-1)).wrapping_add(g(m,b3,i).wrapping_mul(4)).wrapping_add(g(m,b2,i-1)>>2)>>2) as u32);
    m.put_word(row+4,0);m.put_word(row+8,0);
}

pub fn fn_24dcf0(m:&mut Memory<'_>,p:u32,block:u32,range:u32,mode:u32,scratch:u32,h:&mut impl SpeechRound4Host)->u32 {
    assert_eq!(m.base,0);
    let at=m.offset(scratch);let mut saved=[0u8;32];saved.copy_from_slice(&m.bytes[at..at+32]);
    let rc=if m.word(p+68)==0 {
        sp4_log(m,p,u32::MAX,0x5df1,&[],h);crate::normalization::error_code(7)
    } else {
        sp3_string(m,scratch+20,if mode==1 {b"<LOGFE> \0"}else{b"\0"});
        crate::normalization::strcpy_at(m.bytes,scratch,scratch+20);
        m.put_word(scratch+16,0);
        if crate::normalization::block_field(m.bytes,p,m.word(block+84),scratch+16)>=0x80000000 {sp4_log(m,p,u32::MAX,0x5df1,&[],h);}
        let copy=|m:&mut Memory<'_>,r:u32| {
            let start=sp3_half(m,r) as i16 as i32;let end=sp3_half(m,r+2) as i16 as i32;
            let n=end.wrapping_sub(start);
            crate::frontend::copy_str_n(m.bytes,m.word(p+200),m.word(p+92).wrapping_add(start as u32),n as u16 as u32);
            sp3_byte(m,m.word(p+200).wrapping_add(n as u32),0);
        };
        copy(m,range);
        sp4_log(m,p,5,0x24dcf0,&[scratch,m.word(p+200),sp3_half(m,range) as i16 as u32,sp3_half(m,range+2) as i16 as u32,m.word(p+200),sp3_half(m,block+88) as u32,sp3_half(m,block+90) as u32],h);
        let mut i=0u16;
        while i<(m.byte(block+82) as u16) {
            let r=block+42+i as u32*4;
            let null=sp3_half(m,r)==65535 && sp3_half(m,r+2)==65535;
            if !null {copy(m,r);}
            sp4_log(m,p,5,if null {0x24dd00}else{0x24dd04},&[h.literal(0x24dd00),i as u32+1,sp3_half(m,r) as i16 as u32,sp3_half(m,r+2) as i16 as u32,m.word(p+200)],h);
            i=i.wrapping_add(1);
        }
        i=0;
        while i<sp3_half(m,block+40) {
            sp4_log(m,p,5,0x24dd08,&[h.literal(0x24dd08),i as u32,m.word(p+172)+sp3_half(m,block+2+i as u32*4) as u32],h);
            i=i.wrapping_add(1);
        }
        0
    };
    m.bytes[at..at+32].copy_from_slice(&saved);rc
}

pub fn fn_2e564c(m:&mut Memory<'_>,p:u32,arg:u32,node:u32,result:u32,depth:u8,scratch:u32,h:&mut impl SpeechRound4Host)->u32 {
    assert_eq!(m.base,0);
    sp3_frame(m,scratch,|m|sp5_check(m,p,arg,node,result,depth,scratch,scratch+4096,h))
}
fn sp5_check(m:&mut Memory<'_>,p:u32,arg:u32,node:u32,result:u32,depth:u8,s:u32,end:u32,h:&mut impl SpeechRound4Host)->u32 {
    assert!(s+64<=end,"caller check frame exhausted");
    let child=s;let expanded=s+12;let output=s+24;let func=s+36;let flag=s+40;
    sp4_log(m,p,4,0x2e564c,&[depth as u32],h);
    let rc=sp5_alloc(m,p,child,h);
    if rc>=0x80000000 {return rc;}
    let mut rc=sp5_alloc(m,p,expanded,h);
    if rc<0x80000000 {
        rc=(|| {
            let mut i=0u16;
            while i<(m.byte(node+12) as u16) {
                sp4_log(m,p,5,0x2e5680,&[depth as u32,i as u32],h);
                let rc=sp5_check(m,p,arg,m.word(m.word(node+8)+i as u32*4),child,depth.wrapping_add(1),s+64,end,h);
                if rc>=0x80000000 {return rc;}
                if sp3_half(m,child+6)!=0 {sp4_log(m,p,5,0x2e5684,&[i as u32,m.word(child)],h);}
                let rc=sp5_concat(m,p,expanded,child,h);
                if rc>=0x80000000 {return rc;}
                if m.byte(node+12) as u16!=i+1 {
                    let old=m.byte(s+44);sp3_string(m,s+44,b" \0");
                    let rc=crate::normalization::push_string(m.bytes,p,expanded,s+44,&mut Round4Normalize(h));
                    sp5_buffer_error(m,p,rc,h);
                    sp3_byte(m,s+44,old);
                    if rc>=0x80000000 {return rc;}
                }
                m.put_half(child+6,0);sp3_byte(m,m.word(child),0);i+=1;
            }
            crate::normalization::release_buffer(m.bytes,m.word(p),child,&mut Round4Normalize(h));
            if sp3_half(m,expanded+6)!=0 {sp4_log(m,p,5,0x2e5688,&[m.word(expanded)],h);}
            let key=m.word(node);
            let function=sp5_lookup(m,p,key,func,h);
            if function==0 {
                let args=[h.literal(0x2e56c0),m.word(expanded)];sp4_log(m,p,u32::MAX,0x5dc1,if sp3_half(m,expanded+6)!=0 {&args}else{&[]},h);
                return 0;
            }
            let mut rc=sp5_alloc(m,p,output,h);
            if rc>=0x80000000 {return rc;}
            rc=(|| {
                sp3_byte(m,flag,0);
                let check=m.word(p+104);
                if matches!(m.byte(m.word(node)),8|9) {
                    m.put_word(check,m.word(expanded));m.put_word(check+4,output);m.put_word(check+16,0);m.put_half(check+20,sp3_half(m,node+4));
                } else {
                    let matched=m.word(check+12);let mut table=0;
                    let domain=sp3_half(m,matched+88) as u32;
                    let rc=crate::normalization::domain_value(m.bytes,p,domain,&mut table);
                    if rc>=0x80000000 {
                        let list=m.word(m.word(p+56)+8);
                        if list==0 {sp4_log(m,p,u32::MAX,0x5df1,&[],h);}
                        else {sp4_log(m,p,u32::MAX,0x5de9,&[h.literal(0x250c90),domain,h.literal(0x250c94),sp3_half(m,list) as u32],h);}
                        return rc;
                    }
                    let index=sp3_half(m,matched+90);
                    if index>sp3_half(m,table+12) {sp4_log(m,p,u32::MAX,0x5dc2,&[],h);return rc;}
                    let entry=m.word(table+8)+index as u32*28;
                    m.put_word(check,m.word(node+20));m.put_word(check+4,output);
                    m.put_word(check+16,if m.byte(entry+26)==0 {m.word(entry+20)+sp3_half(m,entry+2) as u32}else{m.word(p+84)});
                    m.put_half(check+20,sp3_half(m,node+4));
                }
                let rc=h.call(m,function,&[p,flag]);
                if rc>=0x80000000 {return rc;}
                sp5_concat(m,p,result,output,h)
            })();
            crate::normalization::release_buffer(m.bytes,m.word(p),output,&mut Round4Normalize(h));rc
        })();
    }
    crate::normalization::release_buffer(m.bytes,m.word(p),expanded,&mut Round4Normalize(h));
    crate::normalization::release_buffer(m.bytes,m.word(p),child,&mut Round4Normalize(h));
    sp4_log(m,p,4,0x2e56ff,&[],h);rc
}

fn sp5_lookup(m:&mut Memory<'_>,p:u32,key:u32,out:u32,h:&mut impl SpeechRound4Host)->u32 {
    let lookup=m.word(m.word(p+60));
    m.put_word(out,0);
    struct Lookup<'a,'b,H>{m:Memory<'a>,h:&'b mut H,out:u32}
    impl<H:SpeechRound4Host> crate::normalization::LookupHost for Lookup<'_,'_,H> {
        fn lookup(&mut self,p:u32,key:u32)->u32 {
            let mut value=0;
            let cmp=if p==0 {0}else{self.m.word(self.m.word(p+60)+8)};
            let rc=crate::reentrant2::lookup_value_m(self.m.bytes,if p==0{None}else{Some(p)},key,Some(&mut value),
                |a,b,bytes|self.h.call(&mut Memory{base:0,bytes},cmp,&[a,b]) as i32);

            if rc>=0x80000000 {self.m.word(self.out)}else{value}
        }
    }
    let mut descriptor=[0u8;72];
    descriptor[60..64].copy_from_slice(&64u32.to_le_bytes());
    descriptor[64..68].copy_from_slice(&lookup.to_le_bytes());
    crate::normalization::dispatch_lookup(&mut descriptor,0,0,key,68,&mut Lookup{m:Memory{base:0,bytes:m.bytes},h,out});
    let value=u32::from_le_bytes(descriptor[68..72].try_into().unwrap());
    m.put_word(out,value);value
}
struct Round5Speech<'a,H>{h:&'a mut H,scratch:u32}
impl<H:SpeechRound4Host> crate::boot_part_f::Host for Round5Speech<'_,H> {
    fn compare(&mut self,m:&mut Memory<'_>,f:u32,k:&mut crate::boot_part_f::Key,v:&mut u32)->u32{self.h.compare(m,f,k,v)}
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
        match f {
            0x24dcf0=>fn_24dcf0(m,a[0],a[1],a[2],a[3],self.scratch,self.h),
            0x2e564c=>fn_2e564c(m,a[0],a[1],a[2],a[3],a[4] as u8,self.scratch-256,self.h),
            _=>self.h.call(m,f,a),
        }
    }
}
impl<H:SpeechRound4Host> crate::log_live::LogMethodsM for Round5Speech<'_,H> {
    fn log_m(&mut self,a:u32,b:u32,c:u32,d:u32,e:u32,f:u32,g:u32,m:&mut Memory<'_>)->u32{self.h.log_m(a,b,c,d,e,f,g,m)}
}
impl<H:SpeechRound4Host> crate::speech_part_5::Host for Round5Speech<'_,H> {
    fn scratch(&self)->u32{self.h.scratch()}
    fn category(&self,p:u32)->u32{self.h.category(p)}
    fn thread_id(&mut self)->u32{self.h.thread_id()}
    fn format(&self,p:u32)->u32{self.h.format(p)}
    fn dictionary_entries(&mut self,m:&mut Memory<'_>,p:u32,a:&mut u32,b:&mut u32)->u32{self.h.dictionary_entries(m,p,a,b)}
    fn record(&mut self,m:&mut Memory<'_>,p:u32,a:u32,b:&mut crate::speech_part_5::RecordBuffer,f:&mut u8)->u32{self.h.record(m,p,a,b,f)}
    fn entries(&mut self,m:&mut Memory<'_>,p:u32,a:u32,b:u32,c:&mut u32,d:&mut u32)->u32{self.h.entries(m,p,a,b,c,d)}
}

fn sp5_buffer_error(m:&mut Memory<'_>,p:u32,rc:u32,h:&mut impl SpeechRound4Host) {
    if rc>=0x80000000 {sp4_log(m,p,u32::MAX,0x5df0,&[],h);}
}
fn sp5_alloc(m:&mut Memory<'_>,p:u32,out:u32,h:&mut impl SpeechRound4Host)->u32 {
    let rc=crate::normalization::alloc_destination(m.bytes,p,out,&mut Round4Normalize(h));
    sp5_buffer_error(m,p,rc,h);rc
}
fn sp5_concat(m:&mut Memory<'_>,p:u32,out:u32,source:u32,h:&mut impl SpeechRound4Host)->u32 {
    let rc=crate::normalization::concat_buffers(m.bytes,p,out,source,&mut Round4Normalize(h));
    sp5_buffer_error(m,p,rc,h);rc
}
