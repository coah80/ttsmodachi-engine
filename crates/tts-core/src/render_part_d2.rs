
use crate::{boot::Memory, normalization as n};

pub trait LocaleHost {

    fn locale_global(&mut self, m: &mut Memory<'_>) -> u32;
    fn decode(&mut self, m: &mut Memory<'_>, method: u32, dst: u32, byte: u32, state: u32) -> u32;
}

pub fn fn_2ffae8(m: &mut Memory<'_>, dst: u32, byte: u32, state: u32, h: &mut impl LocaleHost) -> u32 {
    let global = h.locale_global(m);
    let slot = crate::speech_part_3::fn_302078(m, global);
    let locale = m.word(slot);
    if m.byte(locale.wrapping_add(257)) == 1 {
        let field = locale.wrapping_add(259);
        let method = field.wrapping_add(m.word(field));
        return h.decode(m, method, dst, byte, state);
    }
    if m.byte(locale.wrapping_add(byte)) == 0 { return u32::MAX; }
    m.put_half(dst, byte as u16);
    1
}

pub trait MarkerHost: n::NormalizationHost + crate::frontend::HeapVtableM {}
impl<T: n::NormalizationHost + crate::frontend::HeapVtableM> MarkerHost for T {}

fn move_bytes(m: &mut Memory<'_>, dst: u32, src: u32, len: u32) {
    crate::fe_text::memmove_195b38(m.bytes, dst - m.base, src - m.base, len);
}
fn release(m: &mut Memory<'_>, ctx: u32, p: u32, h: &mut impl MarkerHost) {
    let heap = m.word(ctx + 4);
    assert_eq!(m.base, 0, "normalization ports use absolute arena indexes");
    crate::frontend::heap_release_m(m.bytes, if heap == 0 { None } else { Some(heap) },
        if p == 0 { None } else { Some(p) }, h);
}
fn marker(m: &Memory<'_>, rows: u32, i: u16) -> u32 { let _ = m; rows.wrapping_add(i as u32 * 40) }

pub fn fn_2f0894(m: &mut Memory<'_>, ctx: u32, guard1: u32, guard2: u32, object: u32, h: &mut impl MarkerHost) -> u32 {
    if guard1 == 0 || guard2 == 0 { return 0x81102007; }
    assert_eq!(m.base, 0, "normalization ports use absolute arena indexes");
    let mut header = 0;
    let rc = n::fetch_entry(m.bytes, object, 0x3fd, 1, Some(&mut header), h);
    if rc >= 0x80000000 { return rc; }
    let rows = m.word(header + 4);
    let mut count = m.half(header + 8);
    let mut i = 1u16;
    while i < count {
        let p = marker(m, rows, i);
        if m.half(p + 10) == 7 && m.word(p + 28) == 43 {
            let mut j = i.wrapping_add(1);
            while j < count && m.half(marker(m, rows, j) + 10) != 7 { j = j.wrapping_add(1); }
            let q = marker(m, rows, j);
            if j < count && m.half(q + 10) == 7 && m.word(q + 28) == 43 && m.word(p + 20) == m.word(q + 20) {
                let mut k = i.wrapping_add(1);
                while k <= j {
                    let r = marker(m, rows, k);
                    let ptr = m.word(r + 32);
                    if ptr != 0 { release(m, ctx, ptr, h); m.put_word(r + 32, 0); }
                    k = k.wrapping_add(1);
                }
                count = m.half(header + 8);
                move_bytes(m, q, q + 40, (count as u32).wrapping_sub(j as u32).wrapping_mul(40));
                count = count.wrapping_sub(1);
                m.put_half(header + 8, count);
                move_bytes(m, p, p + 40, (count as u32).wrapping_sub(i as u32).wrapping_mul(40));
                i = i.wrapping_sub(1);
                count = count.wrapping_sub(1);
                m.put_half(header + 8, count);
            }
        }
        i = i.wrapping_add(1);
    }
    i = 1;
    while i < count {
        let p = marker(m, rows, i);
        if m.half(p + 10) != 7 { i = i.wrapping_add(1); continue; }
        let mut j = i.wrapping_add(1);
        while j < count && m.half(marker(m, rows, j) + 10) != 7 { j = j.wrapping_add(1); }
        let kind = m.word(p + 28);
        if j >= count {
            if kind == 15 {
                m.put_word(p + 24, 0);
                if m.byte(p) == 0 { m.put_half(p + 10, 999); }
            } else {
                m.put_word(p + 24, m.word(rows + 20).wrapping_add(m.word(rows + 24)).wrapping_sub(m.word(p + 20)));
                if kind == 43 {
                    let end = m.word(rows + 12).wrapping_add(m.word(rows + 16));
                    if end >= m.word(p + 12) { m.put_word(p + 16, end.wrapping_sub(m.word(p + 12))); }
                }
            }
            i = i.wrapping_add(1);
            continue;
        }
        let q = marker(m, rows, j);
        let next = m.word(q + 28);
        let mut keep = true;
        if kind == 43 {
            if next == 43 {
                let ptr = m.word(q + 32);
                if ptr != 0 { release(m, ctx, ptr, h); m.put_word(q + 32, 0); }
                m.put_half(q + 10, 999);
            } else {
                let mut k = j;
                while k < count {
                    let r = marker(m, rows, k);
                    if matches!(m.half(r + 10), 7 | 31 | 36 | 999 | 21) {
                        let mut pos = m.word(r + 12).wrapping_sub(m.word(p + 16));
                        m.put_word(r + 12, pos);
                        if k > j && pos > m.word(q + 12) {
                            pos = pos.wrapping_sub(m.word(q + 16));
                            m.put_word(r + 12, pos);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            m.put_word(p + 16, m.word(q + 12).wrapping_sub(m.word(p + 12)));
        } else if kind == next && matches!(kind, 16 | 60) {
            if m.byte(q) == 0 { m.put_half(q + 10, 999); } else { keep = false; }
        } else if kind != next && m.word(p + 20) == m.word(q + 20) {
            m.put_word(p + 28, next);
            if m.byte(q) != 0 { keep = false; }
            else {
                if m.byte(p) == 0 {
                    m.put_half(p + 10, 999);
                    i = i.wrapping_add(1);
                    continue;
                }
                if (i as u32 + 1) < count as u32 {
                    move_bytes(m, p, p + 40, (count as u32).wrapping_sub(i as u32).wrapping_mul(40));
                }
                count = count.wrapping_sub(1);
                continue;
            }
        } else if kind != next {
            if kind == 15 && m.byte(p) == 0 {
                m.put_half(p + 10, 999);
                i = i.wrapping_add(1);
                continue;
            }
            m.put_word(p + 16, m.word(q + 12).wrapping_sub(m.word(p + 12).wrapping_add(m.word(p + 16))));
        }
        if m.half(p + 10) != 999 { m.put_word(p + 24, m.word(q + 20).wrapping_sub(m.word(p + 20))); }
        if !keep {
            if (j as u32 + 1) < count as u32 {
                move_bytes(m, q, q + 40, (count as u32).wrapping_sub(j as u32).wrapping_mul(40));
            }
            count = count.wrapping_sub(1);
        } else { i = i.wrapping_add(1); }
    }
    m.put_half(header + 8, count);
    0
}

pub trait FormatHost: crate::speech_part_4::Host + LocaleHost {

    fn conversion_scratch(&mut self, m: &mut Memory<'_>) -> u32;

    fn read_format(&mut self, m: &mut Memory<'_>, method: u32, state: u32) -> u32 {
        match method {
            0x2ffb34 => crate::speech_part_4::fn_2ffb34(m, state),
            0x2ffb70 => {
                let p = m.word(state + 16);
                m.put_word(state + 16, p.wrapping_add(2));
                m.half(p) as u32
            }
            _ => self.call(m, method, &[state]),
        }
    }
}
struct LiveConversions<'a, H>(&'a mut H);
impl<H: FormatHost> crate::speech_part_4::Host for LiveConversions<'_, H> {
    fn runtime_scratch(&mut self,m:&mut Memory<'_>)->u32 { self.0.conversion_scratch(m) }
    fn runtime_locale(&mut self,m:&mut Memory<'_>)->u32 { self.0.locale_global(m) }

    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if matches!(f, 0x100418 | 0x1004c0 | 0x1a7970 | 0x1a7974 | 0x100694 | 0x100448 |
            0x4a7aa4 | 0x10035c | 0x4a7a9c | 0x4a7a94 | 0x1007fc | 0x4a7a8c | 0x100834 | 0x4a7a84) {
            let scratch = self.0.conversion_scratch(m);
            crate::speech_part_6::replay_format(m, self, f, a[0], a[1], a[2], scratch)
        } else { self.0.call(m, f, a) }
    }
    fn decimal_point(&mut self, m: &mut Memory<'_>) -> u8 { self.0.decimal_point(m) }
}
impl<H: FormatHost> LocaleHost for LiveConversions<'_, H> {
    fn locale_global(&mut self, m: &mut Memory<'_>) -> u32 { self.0.locale_global(m) }
    fn decode(&mut self, m: &mut Memory<'_>, f: u32, dst: u32, byte: u32, state: u32) -> u32 {
        self.0.decode(m, f, dst, byte, state)
    }
}
fn next_format(m: &mut Memory<'_>, state: u32, h: &mut impl FormatHost) -> u32 {
    let method = m.word(state + 12);
    h.read_format(m, method, state)
}
fn output(m: &mut Memory<'_>, state: u32, c: u32, h: &mut impl FormatHost) {
    let method = m.word(state + 4);
    let context = m.word(state + 8);
    h.call(m, method, &[c, context]);
}

pub fn fn_301eac(m: &mut Memory<'_>, state: u32, mut args: u32, h: &mut impl FormatHost) -> u32 {
    m.put_word(state + 32, 0);
    loop {
        let mut c = next_format(m, state, h);
        if c == 0 { break; }
        if c != 37 {
            output(m, state, c, h);
            m.put_word(state + 32, m.word(state + 32).wrapping_add(1));
            continue;
        }
        let mut flags = 0u32;
        c = next_format(m, state, h);
        loop {
            let bit = match c { 45 => 1, 43 => 2, 32 => 4, 35 => 8, 48 => 16, _ => 0 };
            if bit == 0 { break; }
            flags |= bit;
            c = next_format(m, state, h);
        }
        if flags & 2 != 0 { flags &= !4; }
        m.put_word(state + 28, 0);
        m.put_word(state + 24, 0);
        for part in 0..2 {
            let field = state + 24 + part * 4;
            if c == 42 {
                let v = m.word(args);
                args = args.wrapping_add(4);
                m.put_word(field, v);
                if part == 1 && (v as i32) < 0 { flags &= !32; }
                c = next_format(m, state, h);
            } else {
                while crate::speech_part_4::fn_30208c(m, c) != 0 {
                    let v = m.word(field).wrapping_mul(10).wrapping_add(c.wrapping_sub(48));
                    m.put_word(field, v);
                    c = next_format(m, state, h);
                }
            }
            if part != 0 || c != 46 { break; }
            flags |= 32;
            c = next_format(m, state, h);
        }
        let width = m.word(state + 24) as i32;
        if width < 0 {
            m.put_word(state + 24, width.wrapping_neg() as u32);
            flags |= 1;
        }
        if flags & 1 != 0 { flags &= !16; }
        if c == 108 || c == 104 {
            let previous = c;
            c = next_format(m, state, h);
            flags |= if c == previous {
                c = next_format(m, state, h);
                if previous == 108 { 128 } else { 1024 }
            } else if previous == 108 { 64 } else { 256 };
        } else if matches!(c, 76 | 106 | 116 | 122) {
            if c == 106 { flags |= 128; }
            c = next_format(m, state, h);
        }
        if c == 0 { break; }
        if c.wrapping_sub(65) <= 25 { flags |= 2048; c += 32; }
        m.put_word(state, flags);
        let used = crate::speech_part_4::fn_103c2c(m, &mut LiveConversions(h), state, c, args);
        if used == 0 {
            output(m, state, c, h);
            m.put_word(state + 32, m.word(state + 32).wrapping_add(1));
        } else if used == 1 { args = args.wrapping_add(4); }
        else { args = (args.wrapping_add(7) & !7).wrapping_add(8); }
    }
    m.word(state + 32)
}
fn wrapper(m: &mut Memory<'_>, fmt: u32, context: u32, args: u32, sink: u32, scratch: u32, wide: bool, h: &mut impl FormatHost) -> u32 {
    let at = m.offset(scratch);
    let mut saved = [0u8; 60];
    saved.copy_from_slice(&m.bytes[at..at + 60]);
    m.put_word(scratch + 4, sink);
    m.put_word(scratch + 8, context);
    m.put_word(scratch + 12, if wide { 0x2ffb70 } else { 0x2ffb34 });
    m.put_word(scratch + 16, fmt);
    m.put_word(scratch + 20, wide as u32);
    let result = fn_301eac(m, scratch, args, h);
    m.bytes[at..at + 60].copy_from_slice(&saved);
    result
}

pub fn fn_2ffb3e(m: &mut Memory<'_>, fmt: u32, context: u32, args: u32, sink: u32, scratch: u32, h: &mut impl FormatHost) -> u32 {
    wrapper(m, fmt, context, args, sink, scratch, false, h)
}

pub fn fn_2ffb7a(m: &mut Memory<'_>, fmt: u32, context: u32, args: u32, sink: u32, scratch: u32, h: &mut impl FormatHost) -> u32 {
    wrapper(m, fmt, context, args, sink, scratch, true, h)
}

pub struct PipelineData<'a> {
    pub environment: &'a crate::render_part_c::Environment,
    pub dump: &'a crate::render_part_c::DumpData,
    pub constants: u32,
    pub names: [u32; 6],
    pub tags: [u32; 6],
    pub scratch: u32,
    pub stages: Option<&'a crate::leftovers_1::PipelineStageData<'a>>,
}
pub trait PipelineHost: MarkerHost + crate::render_part_c::Host + crate::log_live::LogMethodsM {}
impl<T: MarkerHost + crate::render_part_c::Host + crate::log_live::LogMethodsM> PipelineHost for T {}
fn pipeline_dump(m: &mut Memory<'_>, a: &[u32;14], ctx: u32, text: u32, events: u32, stage: usize,
    data: &PipelineData<'_>, h: &mut impl PipelineHost) {
    crate::render_part_c::fn_2b9770(m, a[3], ctx, text, events, data.tags[stage], data.dump, h);
}
fn pipeline_copy(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) {
    crate::fe_text::memcpy_195ae8(m.bytes, dst, src, n);
}
fn pipeline_ok(rc: u32) -> Result<(),u32> { if rc >= 0x80000000 { Err(rc) } else { Ok(()) } }
fn pipeline_current(m: &Memory<'_>, it: u32) -> Result<u32,u32> {
    crate::render_part_a3::entry(m.bytes, it, 0xffff)
}
fn pipeline_find(m: &mut Memory<'_>, it: u32, ids: u32) -> Result<Option<u32>,u32> {
    let rc = crate::render_part_a3::scan(m.bytes, it, 0x2b93c8, ids);
    if rc & 0x1fff == 0x14 { return Ok(None); }
    pipeline_ok(rc)?;
    Ok(Some(pipeline_current(m, it)?))
}

pub fn fn_2efd50(m: &mut Memory<'_>, a: [u32;14], data: &PipelineData<'_>, h: &mut impl PipelineHost) -> u32 {
    assert_eq!(m.base,0);
    if a[6]==0 { return 0x81102007; }
    let (rc,ctx)=crate::render_part_c::resolve(m,[a[4],a[5]],data.environment);
    if rc>=0x80000000 { return 0x81102007; }
    let at=m.offset(data.scratch);
    let mut saved=[0u8;512];saved.copy_from_slice(&m.bytes[at..at+512]);
    let mut text=0;let mut events=0;
    let result=(|| -> Result<u32,u32> {
        pipeline_ok(n::open_iterator(m.bytes,a[4],a[5],a[6],0x3fc,Some(&mut text),h))?;
        pipeline_ok(n::open_iterator(m.bytes,a[4],a[5],a[6],0x3fd,Some(&mut events),h))?;
        pipeline_dump(m,&a,ctx,text,events,0,data,h);
        pipeline_ok(fn_2f0894(m,ctx,text,events,a[6],h))?;
        pipeline_dump(m,&a,ctx,text,events,1,data,h);
        let rec=data.scratch;let ids=rec+40;let skip=rec+44;
        m.put_word(ids,m.word(data.constants));
        pipeline_copy(m,rec,data.constants+0xc8,40);
        if text==0||events==0 {return Err(0x81102007);}
        let mut textlen=0;
        pipeline_ok(crate::render_part_c::iterator_count(m,text,&mut textlen))?;
        let offsets=[64,80,96,136,164,180];
        for i in 0..6 {crate::fe_text::strcpy_widen_1763e4(m.bytes,rec+offsets[i],data.names[i]);}
        let mut table=0;
        pipeline_ok(n::fetch_entry(m.bytes,a[6],0x3f4,1,Some(&mut table),h))?;
        let names=m.word(table+4);
        pipeline_ok(crate::render_part_a3::index(m.bytes,events,0))?;
        let first=crate::render_part_a3::entry(m.bytes,events,0)?;
        let mut current=pipeline_find(m,events,ids)?;
        while let Some(cur)=current {
            let name=names.wrapping_add(m.word(cur+28).wrapping_mul(2));
            let eq=|i:usize| crate::fe_text::strcmp16_1765ac(m.bytes,name,rec+offsets[i])==0;
            let matched=eq(0)||n::u16_ncmp(m.bytes,name,rec+offsets[1],6)==0||eq(3)||eq(4)||eq(5);
            let major=matched&&(eq(2)||eq(3)||eq(4)||eq(5));
            m.put_half(skip,1);
            crate::render_part_a3::scan(m.bytes,events,0x2b94ac,skip);
            let next=pipeline_find(m,events,ids)?;
            if matched {
                let end=next.map_or(m.word(first+20).wrapping_add(textlen as u32*2),|p|m.word(p+20));
                let len=end.wrapping_sub(m.word(cur+20));
                m.put_word(rec+12,m.word(cur+12));m.put_word(rec+20,m.word(cur+20));
                m.put_word(rec+16,len);m.put_word(rec+24,len);m.put_word(rec+28,if major{44}else{14});
                if len!=0 {pipeline_ok(n::insert_ordered(m.bytes,events,rec,h))?;}
            }
            current=pipeline_find(m,events,ids)?;
        }
        pipeline_dump(m,&a,ctx,text,events,2,data,h);
        let stages=data.stages.expect("pipeline stages require caller-owned constants and workspace");
        pipeline_ok(crate::leftovers_1::fn_2f0d3c(m,
            [a[3],ctx,a[0],a[1],a[2],a[4],a[5],text,events,a[6],a[10],a[11],a[12],a[13]],stages,h))?;
        pipeline_dump(m,&a,ctx,text,events,3,data,h);
        pipeline_ok(crate::leftovers_1::fn_2f15bc(m,
            [a[4],a[5],text,events,a[6],a[7],a[8],a[9],0x3fd],stages,h))?;
        pipeline_dump(m,&a,ctx,text,events,4,data,h);
        m.put_word(ids,m.word(data.constants+8));
        pipeline_copy(m,rec,data.constants+0xf0,40);
        if text==0||events==0 {return Err(0x81102007);}
        pipeline_ok(crate::render_part_a3::index(m.bytes,events,0))?;
        let first=crate::render_part_a3::entry(m.bytes,events,0)?;
        let mut current=pipeline_find(m,events,ids)?;
        let start=m.word(first+12);let length=m.word(first+16);
        let end=m.word(first+20).wrapping_add(m.word(first+24));
        if current.map_or(true,|p|m.word(first+20)<m.word(p+20)) {
            m.put_word(rec+12,start);
            m.put_word(rec+16,current.map_or(length,|p|m.word(p+12).wrapping_sub(start)));
            m.put_word(rec+20,m.word(first+20));
            m.put_word(rec+24,current.map_or(m.word(first+24),|p|m.word(p+20).wrapping_sub(m.word(first+20))));
            m.put_word(rec+28,0);
            pipeline_ok(n::insert_ordered(m.bytes,events,rec,h))?;
            if current.is_some() {current=Some(pipeline_current(m,events)?);}
        }
        while let Some(cur)=current {
            m.put_half(skip,1);crate::render_part_a3::scan(m.bytes,events,0x2b94ac,skip);
            let mut after=pipeline_find(m,events,ids)?;
            let nextoff=after.map_or(end,|p|m.word(p+20));
            let nextpos=after.map_or(start.wrapping_add(length),|p|m.word(p+12));
            if m.word(cur+20).wrapping_add(m.word(cur+24))<nextoff {
                m.put_word(rec+12,m.word(cur+12).wrapping_add(m.word(cur+16)));
                m.put_word(rec+16,nextpos.wrapping_sub(m.word(cur+12)).wrapping_sub(m.word(cur+16)));
                m.put_word(rec+20,m.word(cur+20).wrapping_add(m.word(cur+24)));
                m.put_word(rec+24,nextoff.wrapping_sub(m.word(cur+20)).wrapping_sub(m.word(cur+24)));
                m.put_word(rec+28,0);
                pipeline_ok(n::insert_ordered(m.bytes,events,rec,h))?;
                if after.is_some() {after=Some(pipeline_current(m,events)?);}
            }
            current=after;
        }
        pipeline_dump(m,&a,ctx,text,events,5,data,h);
        let mut table=0;
        let rc=n::fetch_entry(m.bytes,a[6],0x3fd,1,Some(&mut table),h);
        pipeline_ok(rc)?;
        let rows=m.word(table+4);let count=m.half(table+8);
        if count==0||m.half(rows+10)!=0x4000 {return Err(0x81102000);}
        for i in 1..count {
            let p=rows+i as u32*40;
            if m.half(p+10)==7&&m.word(p+28)==43 {
                for j in i+1..count {
                    let q=rows+j as u32*40;
                    if m.word(q+20)<=m.word(p+20)||m.word(q+20)>=m.word(p+20).wrapping_add(m.word(p+24)){break;}
                    m.put_word(q+20,m.word(p+20));
                }
            }
        }
        let mut i=1u16;
        while i<count {
            let p=rows+i as u32*40;
            if i as u32+1<count as u32&&m.word(p+60)<m.word(p+20) {
                pipeline_copy(m,rec,p+40,40);pipeline_copy(m,p+40,p,40);pipeline_copy(m,p,rec,40);i=0;
            } else {i+=1;}
        }
        Ok(rc)
    })();
    if events!=0 {n::release_iterator(m.bytes,events,h);}
    if text!=0 {n::release_iterator(m.bytes,text,h);}
    m.bytes[at..at+512].copy_from_slice(&saved);
    match result {Ok(rc)|Err(rc)=>rc}
}
