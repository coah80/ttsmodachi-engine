//! Language/spelling processor prefixes and reserved component leaves.
//!
//! These are not complete call-tree implementations. Recorded language and
//! spelling stages run through live properties, heap growth and stream methods,
//! or fail closed at the first unwired resource service. Untested marker-mode
//! paths remain explicit instruction boundaries. No recorded outputs enter here.
use crate::engine::{Error, Memory};
use crate::engine_components::{CoreCalls, InternalCalls, RomHost};
use crate::rom_region::{Rom, addr, to_us};

fn bad(rc: u32) -> bool {
    rc >= 0x8000_0000
}
fn byte(m: &Memory<'_, '_>, a: u32) -> Result<u8, Error> {
    let mut b = [0];
    m.read(a, &mut b)?;
    Ok(b[0])
}
fn invoke(
    m: &mut Memory<'_, '_>,
    h: &mut impl InternalCalls,
    table: u32,
    slot: u32,
    pair: u32,
    rest: &[u32],
) -> Result<u32, Error> {
    let mut args = [0; 8];
    args[0] = m.word(pair)?;
    args[1] = m.word(pair + 4)?;
    args[2..2 + rest.len()].copy_from_slice(rest);
    // The slot holds a guest address: normalize region-first so a twin
    // that collides with another region's US key still routes to the
    // running region's function (KR 0x1ed1d0 names the lid query, US
    // 0x1ed1d0 the canonicalizer). Port-code literals never pass through
    // here, so the US-first rule for those is unaffected.
    let raw = m.word(table + slot)?;
    let entry = crate::rom_region::to_us_guest(h.rom(), raw);
    h.call(m, entry, &args[..2 + rest.len()])
}

/// Live stream/depes query bindings and reserved component leaves.
/// Growth helpers dispatch through the real frontend heap wrappers. Concrete
/// allocator methods not wired here still fail closed through CoreCalls.
/// The local read_direct_start image contains only scalar input state. Its
/// returned guest pointer is preserved, never used as a local offset.
pub struct ReadCalls;
impl InternalCalls for ReadCalls {
    fn call(&mut self, m: &mut Memory<'_, '_>, entry: u32, a: &[u32]) -> Result<u32, Error> {
        let rom = Rom::from_entry(entry);
        let mut h = RomHost { inner: self, rom };
        read_call(m, to_us(rom, entry), a, &mut h)
    }
}
/// Region-aware form of [`ReadCalls`]. The caller builds it with the live
/// region; nested ports read it through [`InternalCalls::rom`].
pub struct RegionalReads {
    pub rom: Rom,
}
impl InternalCalls for RegionalReads {
    fn rom(&self) -> Rom {
        self.rom
    }
    fn call(&mut self, m: &mut Memory<'_, '_>, entry: u32, a: &[u32]) -> Result<u32, Error> {
        let rom = self.rom.or(Rom::from_entry(entry));
        let mut h = RomHost { inner: self, rom };
        read_call(m, to_us(rom, entry), a, &mut h)
    }
}
pub(crate) fn read_call(m: &mut Memory<'_, '_>, entry:u32, a:&[u32], h:&mut impl InternalCalls)->Result<u32,Error>{
    match entry {
            0x15140c => {
                let rc = crate::normalization::quad_check(a[0], a[1], 0x47de, 12);
                if bad(rc) {
                    return Ok(rc);
                }
                let state = m.word(a[0] + 4)?;
                let p = crate::frontend::StreamPosition {
                    kind: byte(m, a[0])?,
                    read: m.word(a[0] + 8)?,
                    written: m.word(state + 44)?,
                    eof: byte(m, state + 68)?,
                };
                let mut output = 0;
                let mut ended = 0;
                let rc = p.remaining(
                    rc,
                    if a[2] == 0 { None } else { Some(&mut output) },
                    if a[3] == 0 { None } else { Some(&mut ended) },
                );
                if a[2] != 0 {
                    m.put_word(a[2], output)?;
                }
                if a[3] != 0 {
                    m.write(a[3], &[ended])?;
                }
                Ok(rc)
            }
            0x14fe9c => {
                let rc = crate::normalization::quad_check(a[0], a[1], 0x47de, 12);
                if bad(rc) {
                    return Ok(rc);
                }
                let mut local = [0u8; 128];
                // Stream at 4, state at 16; outputs at 112/116. Preserve the
                // original address arithmetic by choosing the input buffer
                // word so the port produces the actual guest pointer.
                m.read(a[0], &mut local[4..16])?;
                let state = m.word(a[0] + 4)?;
                local[8..12].copy_from_slice(&16u32.to_le_bytes());
                for off in [44u32, 56, 60] {
                    local[(16 + off) as usize..(20 + off) as usize]
                        .copy_from_slice(&m.word(state + off)?.to_le_bytes());
                }
                let rc = crate::frontend::read_direct_start(
                    &mut local,
                    Some(4),
                    a[1],
                    if a[2] == 0 { None } else { Some(112) },
                    if a[3] == 0 { None } else { Some(116) },
                );
                if a[2] != 0 {
                    m.put_word(
                        a[2],
                        u32::from_le_bytes(local[112..116].try_into().unwrap()),
                    )?;
                }
                if a[3] != 0 {
                    m.put_word(
                        a[3],
                        u32::from_le_bytes(local[116..120].try_into().unwrap()),
                    )?;
                }
                Ok(rc)
            }
            0x219554 => {
                struct Validator {object:u32,key:u32}
                impl crate::depes::HandleValidator for Validator {
                    fn validate(&mut self,_:u32,tag:u32,size:u32)->u32 {
                        crate::normalization::quad_check(self.object,self.key,tag,size)
                    }
                }
                let mut opts=crate::depes::ModuleOptions::default();
                let rc=opts.set(a[1],a[2] as i32,a[3] as i32,&mut Validator{object:a[0],key:a[1]});
                if !bad(rc) {
                    let value=if a[3]==1 {opts.first} else {opts.second};
                    m.put_word(a[0]+if a[3]==1 {56} else {60},value as u32)?; // negative-control: depes set
                }
                Ok(rc)
            }
            0x21958c => {
                struct Validator {
                    object: u32,
                    key: u32,
                }
                impl crate::depes::HandleValidator for Validator {
                    fn validate(&mut self, _: u32, tag: u32, size: u32) -> u32 {
                        crate::normalization::quad_check(self.object, self.key, tag, size)
                    }
                }
                let rc = crate::normalization::quad_check(a[0], a[1], 0xf37e, 64);
                if bad(rc) {
                    return Ok(0x8970_2008);
                }
                let opts = crate::depes::ModuleOptions {
                    first: m.word(a[0] + 56)? as i32,
                    second: m.word(a[0] + 60)? as i32,
                };
                let mut out = 0;
                let rc = opts.get(
                    a[1],
                    &mut out,
                    a[3] as i32,
                    &mut Validator {
                        object: a[0],
                        key: a[1],
                    },
                );
                m.put_word(a[2], out as u32)?;
                Ok(rc)
            }
            0x1963b8 if a[0]==0 || a[1]==0 || a[2]==0 => null_calloc(a),
            0x1ecf68 => lid_query(m,a,h),
            0x1ea4e0 => lookup_dictionary_prefix(m,a,h),
            0x176f94 => Ok(crate::normalization::quad_check(a[0],a[1],a[2],a[3])),
            0x175a1c if a[0]==0 || a[1]==0 || a[2]==0 => {
                struct Lock;
                impl crate::frontend::StateLock for Lock {
                    fn acquire(&mut self)->u32 { unreachable!() }
                    fn release(&mut self)->u32 { unreachable!() }
                }
                Ok(crate::frontend::fetch_param(&[],None,a[1],None,&mut Lock,|_,_|unreachable!()))
            }
            0x17b1c4 if a[0]==0 || a[1]==0 => {
                Ok(crate::frontend::release_ling_entry(&mut [],None,None,
                    &crate::frontend::ReleaseScripts {lock:0,find:0,found:None,remove:0,unlock:0},None::<fn(u32,u32,&mut [u8;32])>))
            }
            0x174f9c if a[0]==0 => Ok(0x8b702007),
            0x245a90 => destroy_models(m,a[0],h),
            0x1cf868 => save_events(m, a[0], a[1], a[2], a[3], h),
            0x219790 => destroy_spelling(m, a[0], a[1], h),
            0x219f74 => intern_pair(m, a[0], a[1], a[2], a[3], a[4], h),
            0x1cfc18 => find_language(m, a[0], a[1], a[2]),
            0x1d1808 => grow_array(m, a[0], a[1], a[2], a[3], a[4], a[5], h),
            0x219e80 => reserve_marks(m, a[0], a[1], a[2], a[3], a[4], h),
            0x196394 | 0x1963e0 | 0x196428 => heap_binding(m, entry, a, h),
            0x21a1fc => compare_events(m, a[0], a[1]),
            0x151294 => {m.put_word(a[2],m.word(a[0]+8)?)?;Ok(0)}
            _ => CoreCalls.call(m, entry, a),
    }
}

/// fn_1d3314 ABI: (LanguageEngine*, key=0xf3c0, unused, unused, active*) -> rc.
/// s is a disjoint caller-owned 2048-byte local frame.
/// Complete idle/empty-EOS paths; text and marker transformation remain unwired.
/// Calls the real handle/stream Rust ports via ReadCalls on the live path.
pub(crate) fn process_1d3314(
    m: &mut Memory<'_, '_>,
    p: u32,
    key: u32,
    active: u32,
    s: u32,
    h: &mut impl InternalCalls,
) -> Result<u32, Error> {
    let mut rc = crate::normalization::quad_check(p, key, 0xf3c0, 0x358);
    if bad(rc) {
        return Ok(0x8bd0_2008);
    }
    m.write(active, &[1])?; // negative-control: language active write
    if m.word(p + 32)? == 0 {
        return Ok(rc);
    }
    m.write(s, &[0; 64])?;
    let table = m.word(p + 16)?;
    rc = invoke(m, h, table, 72, p + 32, &[s, s + 8])?;
    if bad(rc) {
        return Ok(rc);
    }
    rc = invoke(m, h, table, 72, p + 40, &[s + 4, s + 8])?;
    if bad(rc) {
        return Ok(rc);
    }
    let text_bytes = m.word(s)?;
    let event_bytes = m.word(s + 4)?;
    let ended = byte(m, s + 8)?;
    if text_bytes == 0 {
        if (u32::from(ended) | event_bytes) == 0 {
            m.write(active, &[0])?;
        } else {
            if event_bytes != 0 {
                return prepare_events(m,&[p,s+20,s+24,s+28,s+32,s+36],s+64,h)
                    .and_then(|rc| if bad(rc) {Ok(rc)} else {Err(Error::UnsupportedStage {entry:0x1d3314})});
            }
            if ended != 0 {
                for slot in [48, 56] {
                    if m.word(p + slot)? != 0 {
                        rc = invoke(m, h, table, 36, p + slot, &[])?;
                        m.put_word(p + slot, 0)?;
                        m.put_word(p + slot + 4, 0)?;
                    }
                }
            }
        }
        return Ok(rc);
    }
    if byte(m, p + 68)? == 1 {
        let voice = m.word(p + 0x34c)?;
        if voice != 0 {
            // Only the >3 predicate is needed. The fifth byte is our sentinel;
            // the real strlen port computes the predicate on these inputs.
            let mut prefix = [0; 5];
            for i in 0..4 {
                prefix[i] = byte(m, voice + i as u32)?;
                if prefix[i] == 0 {
                    break;
                }
            }
            if crate::normalization::strlen8(&prefix, 0) > 3 {
                return language_text(m,p,s+64,h);
            }
        }
    }
    rc = invoke(m, h, table, 44, p + 32, &[s + 12, s + 16])?;
    if bad(rc) {
        return Ok(rc);
    }
    rc=prepare_events(m,&[p,s+20,s+24,s+28,s+32,s+36],s+64,h)?;
    if bad(rc) {return Ok(rc);}

    let input=m.word(s+20)?;let output=m.word(s+24)?;let n=m.word(s+32)?;
    if half(m,output)?==0x4000 {m.put_word(s+40,m.word(output+12)?)?;}
    else {invoke(m,h,table,68,p+32,&[s+40])?;}
    for i in 1..n {
        let e=output+i*24;
        if half(m,e)?==36 && half(m,e-24)?==36 {put_half(m,e-24,0x4000)?;}
        if byte(m,p+65)?!=0 {
            if half(m,e)?==36 && half(m,e+24)?==36
                && wide_equal(m,m.word(e+20)?,addr(h.rom(), 0x8f8732))? && wide_equal(m,m.word(e+44)?,addr(h.rom(), 0x8f8722))? {
                string_copy(m,m.word(e+20)?,addr(h.rom(), 0x8f8722),true)?;
                literal(m,s+180,b"yes\0")?;
                h.call(m,0x1759a0,&[m.word(m.word(p)?+20)?,0x1d24dc,s+180])?;
            }
            if byte(m,p+65)?!=0 && half(m,input)?==36 && wide_equal(m,m.word(input+20)?,addr(h.rom(), 0x8f8732))? {
                string_copy(m,m.word(input+20)?,addr(h.rom(), 0x8f8722),true)?;
            }
        }
    }
    literal(m,s+128,b"lidstate\0")?;m.put_word(s+44,0)?;
    let props=m.word(m.word(p)?+20)?;
    let property_rc=h.call(m,0x175a1c,&[props,s+128,s+44])?;
    let state=m.word(s+44)?;
    if !bad(property_rc) && state!=0 && byte(m,state)?!=0 {
        if compare_strings(m,state,addr(h.rom(), 0x8f870b))?!=0 {
            literal(m,s+160,b"pipelineswitchvoice\0")?;
            h.call(m,0x1759a0,&[props,s+160,state])?;
            h.call(m,0x1759a0,&[props,s+128,addr(h.rom(), 0x8f870b)])?;
            return Err(Error::UnsupportedStage{entry:0x1d3314});
        }
        h.call(m,0x175a24,&[props,s+128])?;
        return Err(Error::UnsupportedStage{entry:0x1d3314});
    }
    literal(m,p+0xc0,b"mertfrq\0")?;
    literal(m,p+0x1c0,b"merwfrq\0")?;
    m.write(s+48,&[0])?;
    let query=invoke(m,h,m.word(p+12)?,68,p+4,&[p+0xc0,1,s+48,s+256])?;
    m.write(p+64,&[u8::from(!bad(query) && byte(m,s+48)?!=0)])?;
    if byte(m,p+67)?==2 {
        h.call(m,0x245bf0,&[p,m.word(s+12)?,m.word(s+16)?>>1])?;
        return Err(Error::UnsupportedStage{entry:0x1d3314});
    }
    publish_tail(m,p,s,h)
}

/// The publish tail of fn_1d3314.
///
/// The ARM consumes the text into the input stream, publishes the events,
/// writes the text, saves the retained events and writes the events.  Each
/// call's arguments come from the frame the root body filled.
fn publish_tail(m:&mut Memory<'_, '_>,p:u32,s:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    if m.word(s+0x00)?!=0 {
        let rc=consume_text(m,m.word(p+0x30)?,m.word(s+0x00)?,m.word(s+0x0c)?,h)?;
        if rc>=0x8000_0000 {return Ok(rc);}
    }
    let events=m.word(s+0x1c)?;
    let rc=h.call(m,0x150c0c,&[m.word(p+0x38)?,m.word(p+0x3c)?,events.wrapping_mul(24)])?;
    if rc>=0x8000_0000 {return Ok(rc);}
    if m.word(s+0x10)?!=0 {
        let rc=h.call(m,0x15017c,&[m.word(p+0x20)?,m.word(p+0x24)?,m.word(s+0x10)?])?;
        if rc>=0x8000_0000 {return Ok(rc);}
    }
    let kept=m.word(s+0x20)?;
    if kept!=0 {
        let rc=h.call(m,0x1cf868,&[p,1,0,0])?;
        if rc>=0x8000_0000 {return Ok(rc);}
        let rc=h.call(m,0x15017c,&[m.word(p+0x28)?,m.word(p+0x2c)?,kept.wrapping_mul(24)])?;
        if rc>=0x8000_0000 {return Ok(rc);}
    }
    Ok(0)
}

/// The ARM's fn_1503b0: the stream's text consume.
///
/// The list is compacted when the buffer cannot hold the text, the text
/// buffer is grown only when the compaction does not make room, the text is
/// copied into the buffer at the stream's position and the stream's written
/// word advances.  The event writer the ARM calls next writes no persistent
/// byte.
fn consume_text(m:&mut Memory<'_, '_>,stream:u32,count:u32,src:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    let state=m.word(stream+4)?;
    let written=m.word(state+44)?;
    let base=m.word(state+56)?;
    if written<base {return Ok(0x88f0_2005);}
    let mut need=written.wrapping_sub(base).wrapping_add(count);
    let mut copy=true;
    if m.word(state+64)?<need {
        let readers=m.word(state+12)?;
        let reader_count=if readers==0 {0} else {m.word(readers+20)?};
        if byte(m,state+40)?!=0 || reader_count!=0 {
            h.call(m,0x14e440,&[state,need.wrapping_sub(m.word(state+64)?)])?;
            need=m.word(state+44)?.wrapping_sub(m.word(state+56)?).wrapping_add(count);
            if m.word(state+64)?<need {
                need=need.max(m.word(m.word(state)?+0x10)?);
                let heap=m.word(m.word(state+4)?+4)?;
                let old=m.word(state+60)?;
                let n=if old!=0 {
                    h.call(m,0x1963e0,&[heap,old,need])?
                } else {
                    h.call(m,0x196394,&[heap,need])?
                };
                if n==0 {return Ok(0x88f0_200a);}
                m.put_word(state+60,n)?;
                m.put_word(state+64,need)?;
            }
        } else {
            // The ROM advances the base and copies nothing.
            m.put_word(state+56,base.wrapping_add(count))?;
            copy=false;
        }
    }
    if copy {
        let position=m.word(state+44)?.wrapping_sub(m.word(state+56)?);
        let dst=m.word(state+60)?.wrapping_add(position);
        for i in 0..count {
            let mut b=[0u8;1];
            m.read(src+i,&mut b)?;
            m.write(dst+i,&b)?;
        }
    }
    m.put_word(state+44,m.word(state+44)?.wrapping_add(count))?;
    Ok(0)
}

/// fn_21a25c ABI: (SpellingProcessor*, key=0xf389, unused, unused, active*) -> rc.
/// s is a disjoint caller-owned 2048-byte local frame.
/// Idle/drain/EOS, plain ASCII text, language-run rebuilding, allocation and
/// cleanup are live. Dictionary expansion currently reaches the resource-read
/// boundary; special marker modes beyond that boundary remain untranslated.
pub(crate) fn process_21a25c(
    m: &mut Memory<'_, '_>,
    p: u32,
    key: u32,
    active: u32,
    s: u32,
    h: &mut impl InternalCalls,
) -> Result<u32, Error> {
    let mut rc = crate::normalization::quad_check(p, key, 0xf389, 0x4c);
    if bad(rc) {
        return Ok(0x8a60_2008);
    }
    m.write(active, &[1])?; // negative-control: spelling active write
    m.write(s, &[0; 64])?;
    let table = m.word(p + 28)?;
    rc = invoke(m, h, table, 72, p + 32, &[s, s + 8])?;
    if bad(rc) {
        return Ok(rc);
    }
    rc = invoke(m, h, table, 72, p + 40, &[s + 4, s + 8])?;
    if bad(rc) {
        return Ok(rc);
    }
    if m.word(s)? == 0 {
        if u32::from(byte(m, s + 8)?) | m.word(s + 4)? != 0 {
            if m.word(s + 4)? != 0 {
                rc = invoke(m, h, table, 44, p + 40, &[s + 12, s + 16])?;
                if bad(rc) {
                    return Ok(rc);
                }
                rc = invoke(m, h, table, 52, p + 56, &[m.word(s + 12)?, m.word(s + 16)?])?;
                if bad(rc) {
                    return Ok(rc);
                }
                rc = invoke(m, h, table, 48, p + 40, &[m.word(s + 16)?])?;
                if bad(rc) {
                    return Ok(rc);
                }
            }
            if byte(m, s + 8)? != 0 {
                for slot in [48, 56] {
                    if m.word(p + slot)? != 0 {
                        rc = invoke(m, h, table, 36, p + slot, &[])?;
                        m.put_word(p + slot, 0)?;
                        m.put_word(p + slot + 4, 0)?;
                    }
                }
            }
            return Ok(rc);
        }
        // ROM property-key pointer is supplied in the literal pool. The
        // dependency remains unwired rather than guessing a property value.
        return Err(Error::UnsupportedStage { entry: 0x175a1c });
    }
    let depes = m.word(p + 24)?;
    rc = invoke(m, h, depes, 60, p + 16, &[s + 20, 1])?;
    if bad(rc) {
        return Ok(rc);
    }
    rc = invoke(m, h, depes, 60, p + 16, &[s + 24, 0])?;
    if bad(rc) {
        return Ok(rc);
    }
    rc = invoke(m, h, table, 44, p + 32, &[s + 12, s + 16])?;
    if bad(rc) {
        return Ok(rc);
    }
    rc = invoke(m, h, table, 44, p + 40, &[s + 28, s + 32])?;
    if bad(rc) {
        return Ok(rc);
    }
    let count=(m.word(s+32)?/24) as u16 as u32;
    let heap=allocator(m,p)?;
    let records=h.call(m,0x1963b8,&[heap,1,1+count*24])?;
    if records==0 {
        h.call(m,0x173dc0,&[m.word(m.word(p)?+16)?,addr(h.rom(), 0x8fa20c),0x9858,0])?;
        return Ok(0x8a60200a);
    }
    copy_bytes(m,records,m.word(s+28)?,m.word(s+32)?)?;
    for i in 0..count {
        let e=records+i*24;
        if half(m,e)?==1 {m.put_word(e+16,0)?;}
    }
    let owned=h.call(m,0x196394,&[heap,count<<2])?;
    if owned==0 {
        allocation_log(m,p,true,h)?;
        free(m,p,records,h)?;
        return Ok(0x8a60200a);
    }
    zero(m,owned,count*4)?;
    if m.word(records+12)?==0 {
        rc=invoke(m,h,depes,56,p+16,&[0,1])?;
        if bad(rc) {return Ok(rc);}
    }
    let capacity=((m.word(s+16)?/2) as u16).wrapping_add(32);
    let marks=h.call(m,0x1963b8,&[heap,1,capacity as u32*4+1])?;
    if marks==0 {
        allocation_log(m,p,true,h)?;
        free(m,p,owned,h)?;free(m,p,records,h)?;
        return Ok(0x8a60200a);
    }
    for i in 0..capacity as u32 {put_half(m,marks+i*4,9999)?;}

    let text=m.word(s+12)?;
    let mut text_len=(m.word(s+16)?/2) as u16;
    let base=m.word(records+12)?;
    // The old prefix didn't publish inputOffset=0 on this branch.
    if base==0 {m.put_word(s+20,0)?;}
    let logger=m.word(m.word(p)?+16)?;
    literal(m,s+100,b" \0")?;
    h.call(m,0x1738e8,&[logger,addr(h.rom(), 0x8fa20c),5,0,s+100])?;
    let mut last=(count as u16).wrapping_sub(1);
    while last!=0 && half(m,records+last as u32*24)?==0x4000 {last=last.wrapping_sub(1);}
    let mut trailing_nulls=0u16;
    while half(m,text+(text_len as u32).wrapping_mul(2).wrapping_sub(2))?==0 {
        text_len=text_len.wrapping_sub(1);trailing_nulls=trailing_nulls.wrapping_add(1);
    }
    let mut leading=0u32;let mut trailing=0u32;let mut single=false;
    if m.word(records+last as u32*24+12)?==base {
        let mut i=0u16;
        while i<text_len && half(m,text+i as u32*2)?==32 {i=i.wrapping_add(1);}
        leading=(i&255) as u32;
        while i<text_len && half(m,text+i as u32*2)?!=32 {i=i.wrapping_add(1);}
        if (i as u32).wrapping_sub(leading)==1 {
            while i<text_len && half(m,text+i as u32*2)?==32 {i=i.wrapping_add(1);}
            if i==text_len {trailing=(i as u32).wrapping_sub(1).wrapping_sub(leading)&255;single=true;}
        }
    }
    // Logging is dispatched, not interpreted as a fabricated dependency reply.
    literal(m,s+100,b"Markers IN\0")?;
    h.call(m,0x1738e8,&[logger,addr(h.rom(), 0x8fa20c),5,0,s+100])?;
    literal(m,s+128,b"latin\0normal\0")?;
    literal(m,s+160,b"n\0o\0r\0m\0a\0l\0\0\0")?;
    for i in 0..count {
        let e=records+i*24;let kind=half(m,e)?;
        literal(m,s+220,b"Marker [type=%5u] Ref (%u,%u) Cur(%u,%u)\0")?;
        h.call(m,0x1738e8,&[logger,addr(h.rom(), 0x8fa20c),5,0,s+220,kind as u32,m.word(e+4)?,m.word(e+8)?,m.word(e+12)?,m.word(e+16)?])?;
        if kind==0x28 || kind==0x22 {single=false;}
        else if kind==0x15 && !wide_equal(m,m.word(e+20)?,s+160)? && m.word(e+12)?<=leading.wrapping_add(base) {single=false;}
    }
    intern_pair(m,p,p+68,p+72,s+128,s+64,h)?;
    intern_pair(m,p,p+68,p+72,s+134,s+64,h)?;
    spelling_loop(m,p,s,records,owned,marks,capacity,count,text,text_len,base,leading,trailing,trailing_nulls,single,h)

}

/// Dispatches only the two owned processors. UnsupportedStage is never success.
pub fn process(
    m: &mut Memory<'_, '_>,
    entry: u32,
    args: [u32; 5],
    scratch: u32,
    h: &mut impl InternalCalls,
) -> Result<u32, Error> {
    // The regional ROMs relocate the two processor entries. The region is
    // caller-owned: it rides the wrapped host, never a global.
    let rom = h.rom().or(Rom::from_entry(entry));
    let mut h = RomHost { inner: h, rom };
    let h = &mut h;
    match to_us(rom, entry) {
        0x1d3314 => process_1d3314(m, args[0], args[1], args[4], scratch, h),
        0x21a25c => process_21a25c(m, args[0], args[1], args[4], scratch, h),
        _ => Err(Error::UnsupportedStage { entry }),
    }
}

fn half(m: &Memory<'_, '_>, a: u32) -> Result<u16, Error> {
    let mut bytes = [0; 2];
    m.read(a, &mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

/// Adapt disjoint guest strings to the existing byte-string comparison port.
/// Each step uses live input bytes; no maximum string length is imposed.
fn compare_strings(m: &Memory<'_, '_>, mut a: u32, mut b: u32) -> Result<i32, Error> {
    loop {
        let x = byte(m, a)?;
        let y = byte(m, b)?;
        let order = crate::normalization::strcmp_at(&[x, 0, y, 0], 0, 2);
        if order != 0 || x == 0 {
            return Ok(order);
        }
        a = a.wrapping_add(1);
        b = b.wrapping_add(1);
    }
}

/// fn_1cfc18 / EU fn_1cfdd4 / KR fn_1cfe80.
/// ABI: (LanguageEngine*, const char* name, u32 alternate) -> LangEntry*.
/// Searches the selected 26-byte-entry model in caller-owned guest memory.
pub(crate) fn find_language(
    m: &Memory<'_, '_>,
    p: u32,
    name: u32,
    alternate: u32,
) -> Result<u32, Error> {
    let model = p.wrapping_add(if alternate != 0 { 0x60 } else { 0x48 });
    let count = half(m, model + 2)? as u32;
    if count == 0 {
        return Ok(0);
    }
    let list = m.word(model + 4)?;
    for i in 0..count {
        let entry = list.wrapping_add(i.wrapping_mul(26));
        if compare_strings(m, name, entry)? == 0 {
            return Ok(entry);
        }
    }
    Ok(0)
}

/// fn_21a1fc / EU fn_21a3b8 / KR fn_21a464.
/// ABI: (const TextEvent*, const TextEvent*) -> i32, returned as r0 bits.
/// ARM subtraction wraps, and language markers precede a same-position 0x22.
pub(crate) fn compare_events(m: &Memory<'_, '_>, a: u32, b: u32) -> Result<u32, Error> {
    let af = m.word(a.wrapping_add(12))?;
    let bf = m.word(b.wrapping_add(12))?;
    if af != bf {
        return Ok(af.wrapping_sub(bf));
    }
    let ta = half(m, a)?;
    if (ta == 0x24 || ta == 0x15) && half(m, b)? == 0x22 {
        return Ok((-100i32) as u32);
    }
    let tb = half(m, b)?;
    if (tb == 0x24 || tb == 0x15) && ta == 0x22 {
        return Ok(100);
    }
    Ok(m.word(a.wrapping_add(4))?
        .wrapping_sub(m.word(b.wrapping_add(4))?))
}

/// Adapt the existing frontend heap wrappers to live guest vtable calls.
/// The local projection contains only the allocator object's context pointer.
/// Callback errors are returned, never converted into allocation refusal.
fn heap_binding(
    m: &mut Memory<'_, '_>,
    entry: u32,
    a: &[u32],
    h: &mut impl InternalCalls,
) -> Result<u32, Error> {
    struct Calls<'a, 'b, 'c, 'd, H> {
        m: &'a mut Memory<'b, 'c>,
        h: &'d mut H,
        table: u32,
        error: Option<Error>,
    }
    impl<H: InternalCalls> Calls<'_, '_, '_, '_, H> {
        fn run(&mut self, slot: u32, args: &[u32]) -> u32 {
            let result = self
                .m
                .word(self.table.wrapping_add(slot))
                .and_then(|entry| self.h.call(self.m, entry, args));
            match result {
                Ok(v) => v,
                Err(e) => {
                    self.error = Some(e);
                    0
                }
            }
        }
    }
    impl<H: InternalCalls> crate::frontend::HeapVtable for Calls<'_, '_, '_, '_, H> {
        fn alloc_at(&mut self, obj: u32, n: u32) -> u32 {
            self.run(0, &[obj, n])
        }
        fn realloc_at(&mut self, obj: u32, old: u32, n: u32) -> u32 {
            self.run(8, &[obj, old, n])
        }
        fn release_at(&mut self, obj: u32, value: u32) {
            self.run(12, &[obj, value]);
        }
    }
    let heap = a[0];
    // A realloc with old=null still dispatches allocate if size != 0.
    let calls_method = heap != 0
        && if entry == 0x1963e0 {
            a[2] != 0
        } else {
            a[1] != 0
        };
    let mut local = [0; 8];
    let table = if calls_method {
        local[4..8].copy_from_slice(&m.word(heap + 4)?.to_le_bytes());
        m.word(heap)?
    } else {
        0
    };
    let mut calls = Calls {
        m,
        h,
        table,
        error: None,
    };
    let heap = if heap == 0 { None } else { Some(0) };
    let rc = match entry {
        0x196394 => crate::frontend::heap_alloc(&local, heap, a[1], &mut calls),
        0x1963e0 => crate::frontend::heap_realloc(&local, heap, a[1], a[2], &mut calls),
        _ => {
            crate::frontend::heap_release(
                &local,
                heap,
                if a[1] == 0 { None } else { Some(a[1]) },
                &mut calls,
            );
            0
        }
    };
    if let Some(e) = calls.error {
        Err(e)
    } else {
        Ok(rc)
    }
}

/// fn_1d1808 / EU fn_1d19c4 / KR fn_1d1a70.
/// ABI: (LanguageEngine*, void** buffer, u32* capacity, u32* count,
///       u32 element_size, u32 growth) -> rc.
/// Allocation/reallocation goes through the live dependency dispatcher.
#[allow(clippy::too_many_arguments)]
pub(crate) fn grow_array(
    m: &mut Memory<'_, '_>,
    p: u32,
    buffer: u32,
    cap: u32,
    count: u32,
    size: u32,
    extra: u32,
    h: &mut impl InternalCalls,
) -> Result<u32, Error> {
    if buffer == 0 {
        return Ok(0x8bd0_2007);
    }
    let old = m.word(buffer)?;
    if old != 0 && m.word(count)?.wrapping_add(1) <= m.word(cap)? {
        m.put_word(count, m.word(count)?.wrapping_add(1))?;
        return Ok(0);
    }
    if old == 0 {
        m.put_word(count, 0)?;
        m.put_word(cap, 0)?;
    }
    let capacity = m.word(cap)?.wrapping_add(extra);
    let heap = m.word(m.word(p)? + 4)?;
    let allocated = if old != 0 {
        h.call(m, 0x1963e0, &[heap, old, size.wrapping_mul(capacity)])?
    } else {
        h.call(m, 0x196394, &[heap, size.wrapping_mul(capacity)])?
    };
    if allocated == 0 {
        return Ok(0x8bd0_200a);
    }
    m.put_word(buffer, allocated)?;
    m.put_word(cap, capacity)?;
    m.put_word(count, m.word(count)?.wrapping_add(1))?;
    Ok(0)
}

/// fn_219e80 / EU fn_21a03c / KR fn_21a0e8.
/// ABI: (SpellingProcessor*, Mark** array, u16* capacity, u32 need,
///       u32 extra) -> rc. Padding bytes of each four-byte mark are preserved.
pub(crate) fn reserve_marks(
    m: &mut Memory<'_, '_>,
    p: u32,
    array: u32,
    count: u32,
    need: u32,
    extra: u32,
    h: &mut impl InternalCalls,
) -> Result<u32, Error> {
    let total = need.wrapping_add(extra);
    let old_cap = half(m, count)?;
    if total >= u32::from(old_cap) {
        let mut cap = old_cap.wrapping_add(32);
        if total > u32::from(cap) {
            cap = total.wrapping_add(32) as u16;
        }
        let context = m.word(p)?;
        let heap = m.word(context + 4)?;
        let old = m.word(array)?;
        let new = h.call(m, 0x1963e0, &[heap, old, u32::from(cap) * 4])?;
        if new == 0 {
            let logger = m.word(m.word(p)? + 16)?;
            h.call(m, 0x173dc0, &[logger, addr(h.rom(), 0x8fa20c), 40000, 0])?;
            return Ok(0x8a60_200a);
        }
        m.put_word(array, new)?;
        if need < u32::from(cap) {
            for i in 0..u32::from(cap).wrapping_sub(need) {
                let mark = m
                    .word(array)?
                    .wrapping_add(need.wrapping_add(i).wrapping_mul(4));
                m.write(mark, &9999u16.to_le_bytes())?;
                m.write(mark + 2, &[0])?;
            }
        }
        m.write(count, &cap.to_le_bytes())?;
    }
    Ok(0)
}

fn put_half(m: &mut Memory<'_, '_>, a: u32, v: u16) -> Result<(), Error> {
    m.write(a, &v.to_le_bytes())
}
fn copy_bytes(m: &mut Memory<'_, '_>, d: u32, s: u32, n: u32) -> Result<(), Error> {
    // memmove permits overlap in either direction.
    if d > s && d.wrapping_sub(s) < n {
        for i in (0..n).rev() { m.write(d.wrapping_add(i), &[byte(m,s.wrapping_add(i))?])?; }
    } else {
        for i in 0..n { m.write(d.wrapping_add(i), &[byte(m,s.wrapping_add(i))?])?; }
    }
    Ok(())
}
fn zero(m: &mut Memory<'_, '_>, d: u32, n: u32) -> Result<(), Error> {
    for i in 0..n { m.write(d.wrapping_add(i), &[0])?; }
    Ok(())
}
fn string_len(m: &Memory<'_, '_>, s: u32, wide: bool) -> Result<u32, Error> {
    let mut n = 0u32;
    loop {
        let c = if wide { half(m,s.wrapping_add(n.wrapping_mul(2)))? as u32 }
                else { byte(m,s.wrapping_add(n))? as u32 };
        if c == 0 { return Ok(n); }
        n=n.wrapping_add(1);
    }
}
fn string_copy(m: &mut Memory<'_, '_>, d: u32, s: u32, wide: bool) -> Result<(), Error> {
    copy_bytes(m,d,s,string_len(m,s,wide)?.wrapping_add(1).wrapping_mul(if wide {2} else {1}))
}
fn widen(m: &mut Memory<'_, '_>, d: u32, s: u32) -> Result<(), Error> {
    let mut i=0u32;
    loop {
        let c=byte(m,s.wrapping_add(i))?;
        put_half(m,d.wrapping_add(i.wrapping_mul(2)),c as u16)?;
        if c==0 {return Ok(());}
        i=i.wrapping_add(1);
    }
}
fn wide_equal(m: &Memory<'_, '_>, a: u32, b: u32) -> Result<bool, Error> {
    let mut i=0u32;
    loop {
        let x=half(m,a.wrapping_add(i))?;
        if x != half(m,b.wrapping_add(i))? {return Ok(false);}
        if x==0 {return Ok(true);}
        i=i.wrapping_add(2);
    }
}
fn allocator(m: &Memory<'_, '_>, p: u32) -> Result<u32, Error> { m.word(m.word(p)?+4) }
fn free(m: &mut Memory<'_, '_>, p:u32, x:u32, h:&mut impl InternalCalls) -> Result<(),Error> {
    h.call(m,0x196428,&[allocator(m,p)?,x])?; Ok(())
}
fn alloc(m: &mut Memory<'_, '_>, p:u32, n:u32, h:&mut impl InternalCalls) -> Result<u32,Error> {
    h.call(m,0x196394,&[allocator(m,p)?,n])
}
fn allocation_log(m: &mut Memory<'_, '_>, p:u32, spelling:bool, h:&mut impl InternalCalls) -> Result<(),Error> {
    h.call(m,0x173dc0,&[m.word(m.word(p)?+16)?,if spelling {addr(h.rom(), 0x8fa20c)} else {addr(h.rom(), 0x8f87bc)},
        if spelling {40000} else {0xbb80},0])?; Ok(())
}

/// fn_195d30. First `c` (low byte) in the NUL-terminated guest string,
/// or 0 on a miss. A zero `c` matches the terminator, as the ROM does.
fn find_separator(m: &Memory<'_, '_>, s: u32, c: u32) -> Result<u32, Error> {
    let c = c as u8;
    let mut p = s;
    loop {
        let v = byte(m, p)?;
        if v == c { return Ok(p); }
        if v == 0 { return Ok(0); }
        p = p.wrapping_add(1);
    }
}

/// fn_1cf868. Retained events are copied from live input, and the retained
/// UTF-16 payload is independently allocated and copied, never borrowed.
pub(crate) fn save_events(m:&mut Memory<'_, '_>,p:u32,clear:u32,events:u32,n:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    let saved=p+0x2c0;
    if clear!=0 {
        if half(m,saved+48)?==0x15 && m.word(saved+68)?!=0 {free(m,p,m.word(saved+68)?,h)?;}
        zero(m,saved,72)?; // negative-control: retained-event clear
    }
    if events!=0 && byte(m,p+0x43)?==1 {
        for i in 0..n {
            let e=events.wrapping_add(i.wrapping_mul(24));
            match half(m,e)? {
                7=>copy_bytes(m,saved,e,24)?,
                0x1f=>copy_bytes(m,saved+24,e,24)?,
                0x15=>{
                    if m.word(saved+68)?!=0 {free(m,p,m.word(saved+68)?,h)?;}
                    let source=m.word(e+20)?;
                    if wide_equal(m,source,addr(h.rom(), 0x8f8776))? {zero(m,saved+48,24)?;}
                    else {
                        copy_bytes(m,saved+48,e,24)?;
                        let dst=alloc(m,p,string_len(m,source,true)?.wrapping_add(1).wrapping_mul(2),h)?;
                        m.put_word(saved+68,dst)?;
                        if dst==0 {allocation_log(m,p,false,h)?;return Ok(0x8bd0200a);}
                        string_copy(m,dst,source,true)?;
                    }
                }
                _=>{}
            }
        }
    }
    Ok(0)
}

/// fn_219f74. Fresh and grown arrays have deliberately different failure
/// ownership, matching the ROM, including its nine-byte first allocation.
pub(crate) fn intern_pair(m:&mut Memory<'_, '_>,p:u32,array:u32,count:u32,s:u32,index:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    let n=half(m,count)? as u32;
    if n!=0 {
        for i in 0..n {
            if compare_strings(m,m.word(m.word(array)?+i*8)?,s)?==0 {
                put_half(m,index,i as u16)?; // negative-control: interned index
                return Ok(0);
            }
        }
    }
    let a=if n!=0 {
        let a=h.call(m,0x1963e0,&[allocator(m,p)?,m.word(array)?,n*8+9])?;
        if a!=0 {m.put_word(array,a)?;}
        a
    } else {h.call(m,0x1963b8,&[allocator(m,p)?,1,9])?};
    if a==0 {allocation_log(m,p,true,h)?;return Ok(0x8a60200a);}
    let entry=a+half(m,count)? as u32*8;
    let x=h.call(m,0x1963b8,&[allocator(m,p)?,1,string_len(m,s,false)?.wrapping_add(1)])?;
    m.put_word(entry,x)?;
    if x==0 {
        allocation_log(m,p,true,h)?;
        if n==0 {free(m,p,a,h)?;}
        return Ok(0x8a60200a);
    }
    let y=h.call(m,0x1963b8,&[allocator(m,p)?,1,string_len(m,s,false)?.wrapping_add(1).wrapping_mul(2)])?;
    m.put_word(a+half(m,count)? as u32*8+4,y)?;
    if y==0 {
        allocation_log(m,p,true,h)?;
        free(m,p,m.word(a+half(m,count)? as u32*8)?,h)?;
        if n==0 {free(m,p,a,h)?;}
        return Ok(0x8a60200a);
    }
    string_copy(m,m.word(a+half(m,count)? as u32*8)?,s,false)?;
    widen(m,m.word(a+half(m,count)? as u32*8+4)?,s)?;
    put_half(m,index,half(m,count)?)?;
    put_half(m,count,half(m,count)?.wrapping_add(1))?;
    m.put_word(array,a)?;
    Ok(0)
}

/// fn_219790. Registry methods remain real dependency calls, not success stubs.
/// Resource names use the original in-function literal addresses.
pub(crate) fn destroy_spelling(m:&mut Memory<'_, '_>,p:u32,key:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    let rc=crate::normalization::quad_check(p,key,0xf389,0x4c);
    if bad(rc) {return Ok(0x8a602008);}
    if p==0 {return Ok(rc);}
    if m.word(p+68)?!=0 {
        let mut i=0u16;
        while i<half(m,p+72)? {
            let a=m.word(p+68)?+i as u32*8;
            free(m,p,m.word(a)?,h)?;
            m.put_word(m.word(p+68)?+i as u32*8,0)?; // negative-control: freed pair
            free(m,p,m.word(m.word(p+68)?+i as u32*8+4)?,h)?;
            m.put_word(m.word(p+68)?+i as u32*8+4,0)?;
            i=i.wrapping_add(1);
        }
        free(m,p,m.word(p+68)?,h)?;
        m.put_word(p+68,0)?;
    }
    // These are original literal addresses, not local synthetic registry data.
    // The dispatcher may fail closed until the resource adapter is available.
    for name in [0x219890,0x21989c,0x2198a8] {
        h.call(m,0x17b1c4,&[m.word(m.word(p)?+24)?,name])?;
    }
    free(m,p,p,h)?;
    Ok(0)
}

/// Caller-owned 2048-byte local frame for callbacks taking stack pointers.
/// Its contents are temporary, not part of the processor's persistent state.
pub fn leaf(m:&mut Memory<'_, '_>,entry:u32,a:&[u32],s:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    // The region is caller-owned: it rides the wrapped host, never a global.
    let rom = h.rom().or(Rom::from_entry(entry));
    let mut h = RomHost { inner: h, rom };
    let h = &mut h;
    match to_us(rom, entry) {
        0x1cf9d0=>destroy_language(m,a[0],a[1],s,h),
        0x1cfce8=>parse_codes(m,a[0],a[1],s,h),
        0x1d18cc=>choose_voice(m,a[0],a[1],a[2],a[3],a[4],s,h),
        0x1d1be8=>segment_events(m,a,s,h),
        0x1d2098=>language_text(m,a[0],s,h),
        0x1d2fb0=>prepare_events(m,a,s,h),
        _=>h.call(m,entry,a),
    }
}

fn literal(m:&mut Memory<'_, '_>,s:u32,text:&[u8])->Result<u32,Error>{m.write(s,text)?;Ok(s)}

/// fn_1cf9d0. Finalization, releases, and property unregistration.
fn destroy_language(m:&mut Memory<'_, '_>,p:u32,key:u32,s:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    let v=m.word(addr(h.rom(), 0x8f884c))?; let w=m.word(addr(h.rom(), 0x8f8850))?;
    let rc=crate::normalization::quad_check(p,key,0xf3c0,0x358);
    if bad(rc) {return Ok(0x8bd02008);}
    if p==0 {return Ok(rc);}
    save_events(m,p,1,0,0,h)?;
    for (off,name) in [(16,&b"SYNTHSTREAM\0"[..]),(12,&b"FE_DCTLKP\0"[..]),(28,&b"COMPQUERY\0"[..])] {
        if m.word(p+off)?!=0 {
            literal(m,s,name)?;
            h.call(m,0x17b1c4,&[m.word(m.word(p)?+24)?,s])?;
        }
    }
    for name in [&b"lidlanguages\0"[..],&b"lidscope\0"[..],&b"lidmode\0"[..],&b"lidvoiceswitch\0"[..]] {
        literal(m,s,name)?;
        h.call(m,0x174f9c,&[m.word(m.word(p)?+20)?,s,p,v,w])?;
    }
    for off in [0x340,0x4c,0x64,0x7c,0x34c] {
        if m.word(p+off)?!=0 {
            free(m,p,m.word(p+off)?,h)?;
            m.put_word(p+off,0)?;
            if off==0x7c {put_half(m,p+0x78,0)?;}
        }
    }
    h.call(m,0x245a90,&[p])?;
    free(m,p,p,h)?;
    Ok(0)
}

/// fn_1cfce8. Three-letter language list parser and alternate-model fallback.
fn parse_codes(m:&mut Memory<'_, '_>,p:u32,codes:u32,s:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    if m.word(p+0x7c)?!=0 {
        free(m,p,m.word(p+0x7c)?,h)?;
        m.put_word(p+0x7c,0)?;
        put_half(m,p+0x78,0)?;
    }
    if codes==0 || byte(m,codes)?==0 {return Ok(0);}
    let mut n=1u32;
    for i in 0..string_len(m,codes,false)? {if byte(m,codes+i)?==b',' {n=n.wrapping_add(1);}}
    let a=h.call(m,0x1963b8,&[allocator(m,p)?,n,16])?;
    m.put_word(p+0x7c,a)?;
    if a==0 {allocation_log(m,p,false,h)?;return Ok(0x8bd0200a);}
    zero(m,s,8)?;
    let mut j=0u32; let mut overflow=false; let mut i=0u32;
    loop {
        let c=byte(m,codes.wrapping_add(i))?;
        if c==b',' || c==0 {
            if j!=3 {overflow=true;} else {
                string_copy(m,s+4,s,false)?;
                m.write(s+6,b"x")?;
                let mut e=find_language(m,p,s,1)?;
                if e==0 {e=find_language(m,p,s+4,1)?;}
                let mut add=e!=0 && (byte(m,e+24)?!=0 || byte(m,e+25)?!=0);
                if !add {
                    let mut l=find_language(m,p,s,0)?;
                    if l==0 {l=find_language(m,p,s+4,0)?;}
                    add=l!=0 && (byte(m,l+24)?!=0 || byte(m,l+25)?!=0);
                    if l==0 && e==0 {
                        literal(m,s+16,b"%s%s\0languageCode\0")?;
                        h.call(m,0x173dc0,&[m.word(m.word(p)?+16)?,addr(h.rom(), 0x8f87bc),0xbb84,s+16,s+21,s])?;
                    }
                }
                if add {
                    let d=m.word(p+0x7c)?+half(m,p+0x78)? as u32*16;
                    string_copy(m,d,s,false)?;
                    string_copy(m,d+8,s+4,false)?;
                    put_half(m,p+0x78,half(m,p+0x78)?.wrapping_add(1))?;
                }
            }
            j=0;
        } else {
            if j<3 {m.write(s+j,&[crate::normalization::tolower_240ba0(c as u32) as u8])?;}
            else {overflow=true;}
            j=j.wrapping_add(1);
        }
        i=i.wrapping_add(1);
        if string_len(m,codes,false)?<i {break;}
    }
    if overflow {
        literal(m,s+16,b"%s%s\0languageCodeList\0")?;
        h.call(m,0x173dc0,&[m.word(m.word(p)?+16)?,addr(h.rom(), 0x8f87bc),0xbb85,s+16,s+21,codes])?;
    }
    Ok(0)
}

/// fn_1d18cc. Select voice and try the two live COMPQUERY methods.
fn choose_voice(m:&mut Memory<'_, '_>,p:u32,count:u32,voices:u32,index:u32,out:u32,s:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    m.put_word(s,0)?;
    literal(m,s+4,b"lidoriginalvoice\0")?;
    let rc=h.call(m,0x175a1c,&[m.word(m.word(p)?+20)?,s+4,s])?;
    if bad(rc) {m.put_word(s,0)?;}
    put_half(m,index,0)?; // negative-control: selected voice reset
    m.write(out,&[0])?;
    while (half(m,index)? as u32)<count {
        let name=m.word(voices+half(m,index)? as u32*8)?;
        let mut matched=false;
        for which in [1,0] {
            let e=find_language(m,p,name,which)?;
            let mode=byte(m,p+0x44)?;
            if e!=0 && ((byte(m,e+24)?!=0 && mode==0)||(byte(m,e+25)?!=0 && mode==1)) {matched=true;break;}
        }
        if matched {
            zero(m,s+32,0x128)?;
            let x=byte(m,name+2)?==b'x';
            let codes=m.word(p+0x7c)?;
            let iterations=if codes==0 {1} else {half(m,p+0x78)? as u32};
            for i in 0..iterations {
                let code=if codes==0 {name} else {codes+i*16};
                if codes!=0 && compare_strings(m,name,code+if x {8} else {0})?!=0 {continue;}
                if byte(m,p+0x44)?==1 || m.word(p+28)?==0 {m.write(out,&[0])?;return Ok(0);}
                string_copy(m,s+32+if codes==0 && x {0x120} else {64},code,false)?;
                if m.word(s)?!=0 {string_copy(m,s+168,m.word(s)?,false)?;}
                let table=m.word(p+28)?;
                let rest=[m.word(m.word(p)?+12)?,m.word(m.word(p)?+20)?,s+32];
                let mut q=invoke(m,h,table,56,p+20,&rest)?;
                if bad(q) {
                    m.write(s+168,&[0])?;
                    q=invoke(m,h,table,48,p+20,&rest)?;
                }
                if !bad(q) {string_copy(m,out,s+168,false)?;return Ok(0);}
            }
        }
        put_half(m,index,half(m,index)?.wrapping_add(1))?;
    }
    Ok(rc)
}

/// fn_1d1be8. Segment rejection, bias updates and allocated language events.
fn segment_events(m:&mut Memory<'_, '_>,a:&[u32],s:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    let (p,out,cap,count,owned,owcap,owcount,base)=(a[0],a[1],a[2],a[3],a[4],a[5],a[6],a[7]);
    let segs=half(m,p+0x346)?; let mut i=0u16; let mut rc=0;
    while i<segs {
        let seg=m.word(p+0x348)?+i as u32*48;
        put_half(m,s,0)?;zero(m,s+4,32)?;
        rc=choose_voice(m,p,half(m,seg+10)? as u32,m.word(seg+12)?,s,s+4,s+64,h)?;
        if bad(rc) {return Ok(rc);}
        if half(m,s)? >= half(m,seg+10)? {put_half(m,s,0)?;}
        let voice=m.word(seg+12)?+half(m,s)? as u32*8;
        let name=m.word(voice)?;
        let alternate=byte(m,p+64)?;
        let entry=find_language(m,p,name,alternate as u32)?;
        let rp=p+if alternate!=0 {0x8c} else {0x80};
        if alternate!=0 && m.word(voice+4)?!=0 {
            let divisor=half(m,seg+16)? as u32;
            if divisor==0 {return Err(Error::UnsupportedStage{entry:0x30209c});}
            let r=crate::normalization::udiv(m.word(voice+4)?,divisor);
            let high=m.word(rp+8)?;let low=m.word(rp+4)?;
            m.put_word(p+0xb8,if high<=r {low} else {low.wrapping_add(high-r)})?;
            copy_bytes(m,p+0xbc,name,4)?;
            literal(m,s+700,b"Identified language for segment %u: %s\0")?;
            h.call(m,0x1738e8,&[m.word(m.word(p)?+16)?,addr(h.rom(), 0x8f87bc),5,0,s+700,m.word(p+0xb8)?,name])?;
        }
        let rejected=byte(m,rp)?!=0 && (half(m,seg+18)?==0 ||
            ((half(m,seg+18)? as u32)>=m.word(rp+4)? && m.word(rp+8)?>=half(m,seg+22)? as u32));
        if rejected {
            literal(m,s+700,b"Rejection for message: %u %u, %u %u, %u %u\0")?;
            h.call(m,0x1738e8,&[m.word(m.word(p)?+16)?,addr(h.rom(), 0x8f87bc),5,0,s+700,half(m,s)? as u32,half(m,p+0x4a)? as u32,half(m,seg+18)? as u32,m.word(p+0x84)?,half(m,seg+22)? as u32,m.word(p+0x88)?])?;
        } else {
            rc=grow_array(m,p,out,cap,count,24,10,h)?;
            if bad(rc) {return Ok(rc);}
            let event=m.word(out)?+m.word(count)?.wrapping_sub(1).wrapping_mul(24);
            if byte(m,p+65)?!=0 {
                rc=grow_array(m,p,out,cap,count,24,10,h)?;
                if bad(rc) {return Ok(rc);}
                let bias_event=m.word(out)?+m.word(count)?.wrapping_sub(1).wrapping_mul(24);
                put_half(m,bias_event,42)?;
                m.put_word(bias_event+4,0)?;m.put_word(bias_event+8,0)?;
                m.put_word(bias_event+12,m.word(seg)?.wrapping_add(base).wrapping_mul(2))?;
                m.put_word(bias_event+16,m.word(seg+4)?.wrapping_sub(m.word(seg)?).wrapping_mul(2))?;
                m.put_word(bias_event+20,m.word(p+0xb8)?)?;
            }
            literal(m,s+700,b"Setting bias %u: %s\0")?;
            h.call(m,0x1738e8,&[m.word(m.word(p)?+16)?,addr(h.rom(), 0x8f87bc),5,0,s+700,i as u32,name])?;
            if entry==0 {
                h.call(m,0x173dc0,&[m.word(m.word(p)?+16)?,addr(h.rom(), 0x8f87bc),0xbb81,0])?;
                return Ok(0x8bd02007);
            }
            let wbuf=alloc(m,p,string_len(m,entry+8,true)?.wrapping_add(5).wrapping_mul(2),h)?;
            if wbuf==0 {allocation_log(m,p,false,h)?;return Ok(0x8bd0200a);}
            string_copy(m,wbuf,entry+8,true)?;
            let end=wbuf+string_len(m,wbuf,true)?*2;
            string_copy(m,end,addr(h.rom(), 0x8f8784),true)?;
            m.put_word(event+20,wbuf)?;
            put_half(m,event,36)?;
            m.put_word(event+4,0)?;m.put_word(event+8,0)?;
            m.put_word(event+12,m.word(seg)?.wrapping_add(base).wrapping_mul(2))?;
            m.put_word(event+16,m.word(seg+4)?.wrapping_sub(m.word(seg)?).wrapping_mul(2))?;
            rc=grow_array(m,p,owned,owcap,owcount,4,10,h)?;
            if bad(rc) {return Ok(rc);}
            m.put_word(m.word(owned)?+m.word(owcount)?.wrapping_sub(1).wrapping_mul(4),
                m.word(count)?.wrapping_sub(if byte(m,p+65)?!=0 {2} else {1}))?;
        }
        i=i.wrapping_add(1);
    }
    Ok(rc)
}

/// fn_1d2fb0. Reserve and prepend pending language events without filling
/// untouched event fields. All stream writes go through the supplied methods.
fn prepare_events(m:&mut Memory<'_, '_>,a:&[u32],s:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    let (p,input,output,ic,oc,allocated)=(a[0],a[1],a[2],a[3],a[4],a[5]);
    m.put_word(oc,0)?;m.put_word(ic,0)?; // negative-control: event counts
    let table=m.word(p+16)?;
    let rc=invoke(m,h,table,44,p+40,&[input,ic])?;
    if bad(rc) {return Ok(rc);}
    m.put_word(ic,m.word(ic)?/24)?;
    m.put_word(s,0)?;
    literal(m,s+4,b"lidon\0")?;
    let mut state=0;
    let props=m.word(m.word(p)?+20)?;
    if !bad(h.call(m,0x175a1c,&[props,s+4,s])?) {
        let property=m.word(s)?;
        // The ROM copies this property into a local eight-byte voice buffer.
        string_copy(m,s+16,property,false)?;
        if compare_strings(m,property,addr(h.rom(), 0x8f86d2))?==0 {state=1;}
        if string_len(m,property,false)?==3 {state=2;}
        let events=m.word(input)?;
        if state!=0 && m.word(ic)?>1 && half(m,events)?==0x4000 && half(m,events+24)?==36 {
            if !wide_equal(m,m.word(events+44)?,addr(h.rom(), 0x8f8722))? {
                h.call(m,0x1759a0,&[props,s+4,property])?;
                if half(m,events+48)?==36 && wide_equal(m,m.word(events+68)?,addr(h.rom(), 0x8f8722))? {
                    h.call(m,0x1759a0,&[props,s+4,addr(h.rom(), 0x8f86d2)])?;
                }
            } else {h.call(m,0x1759a0,&[props,s+4,addr(h.rom(), 0x8f86d2)])?;}
            state=0;
        }
    }
    let result=invoke(m,h,table,56,p+56,&[output,m.word(ic)?.wrapping_add(5).wrapping_mul(24)])?;
    if bad(result) {return Ok(result);}
    if m.word(ic)?!=0 {
        let src=m.word(input)?;let dst=m.word(output)?;
        let pending=half(m,p+0x308)?==36 && half(m,src)?==0x4000;
        if pending || state!=0 {
            copy_bytes(m,dst,src,24)?;
            put_half(m,dst+24,if pending {half(m,p+0x308)?} else {36})?;
            if pending {
                m.put_word(dst+44,m.word(p+0x31c)?)?;
                m.put_word(dst+36,m.word(src+12)?)?;
                zero(m,p+0x308,24)?;
            } else {
                let value=if state==1 {addr(h.rom(), 0x8f8722)} else {
                    let value=alloc(m,p,string_len(m,addr(h.rom(), 0x8f8722),true)?.wrapping_add(1).wrapping_mul(2),h)?;
                    m.put_word(dst+44,value)?;
                    if value==0 {return Ok(0x8bd0200a);}
                    widen(m,value,m.word(s)?)?;
                    m.put_word(allocated,value)?;value
                };
                m.put_word(dst+44,value)?;
                m.put_word(dst+36,m.word(src+12)?)?;
            }
            if m.word(ic)?>1 {copy_bytes(m,dst+48,src+24,m.word(ic)?.wrapping_sub(1).wrapping_mul(24))?;}
            m.put_word(oc,m.word(ic)?.wrapping_add(1))?;
        } else {
            copy_bytes(m,dst,src,m.word(ic)?.wrapping_mul(24))?;
            m.put_word(oc,m.word(ic)?)?;
        }
    }
    Ok(result)
}

fn shift_indices(m:&mut Memory<'_, '_>,owned:u32,count:u32,at:u32,insert:bool)->Result<(),Error>{
    for k in 0..count {
        let a=owned+k*4;let v=m.word(a)?;
        if if insert {v>=at} else {v>at} {m.put_word(a,if insert {v.wrapping_add(1)} else {v.wrapping_sub(1)})?;}
    }
    Ok(())
}
fn insert_event(m:&mut Memory<'_, '_>,p:u32,s:u32,event:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    let rc=grow_array(m,p,s,s+4,s+8,24,10,h)?;
    if bad(rc) {return Ok(rc);}
    let output=m.word(s)?;
    let mut k=m.word(s+8)?.wrapping_sub(2) as i32;
    let mut moved=0u16;
    while k > -1 {
        if m.word(output+k as u32*24+12)? <= m.word(event+12)? {break;}
        moved=moved.wrapping_add(1);k-=1;
    }
    let at=(k+1) as u32;
    if moved!=0 {
        copy_bytes(m,output+(at+1)*24,output+at*24,moved as u32*24)?;
        shift_indices(m,m.word(s+12)?,m.word(s+20)?,at,true)?;
    }
    m.put_word(s+60,output+at*24)?;
    Ok(rc)
}
fn analyze_segment(m:&mut Memory<'_, '_>,p:u32,text:u32,start:u32,length:u32,base:u32,s:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    let rc=h.call(m,0x246274,&[p,text.wrapping_add(start.wrapping_mul(2)),length])?;
    if bad(rc) {return Ok(rc);}
    segment_events(m,&[p,s,s+4,s+8,s+12,s+16,s+20,start.wrapping_add(base>>1)],s+256,h)
}

/// fn_1d2098. Language marker transformation. The caller supplies a disjoint
/// 2048-byte local frame. LID analysis, property and stream methods are real
/// dependencies through InternalCalls; an unwired method always blocks.
fn language_text(m:&mut Memory<'_, '_>,p:u32,s:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    // Local pointer/count cells, not persistent engine state.
    zero(m,s,64)?;
    let mut inserted=0u32;let mut number=0u32;
    let table=m.word(p+16)?;
    let props=m.word(m.word(p)?+20)?;
    let mut rc=0u32;
    let result=(||->Result<u32,Error>{
        m.write(p+64,&[1])?; // negative-control: alternate-model selection
        literal(m,s+80,b"mertfrq\0mertfrq_latn\0")?;
        let lid=m.word(p+12)?;
        let mut query=invoke(m,h,lid,68,p+4,&[s+80,0,s+64,s+1400])?;
        if !(byte(m,s+64)?!=0 && !bad(query)) {
            query=invoke(m,h,lid,68,p+4,&[s+88,0,s+64,s+1400])?;
        }
        let detect=if !bad(query) && byte(m,s+64)?!=0 {
            m.write(p+64,&[0])?;byte(m,p+0xb4)?
        } else {m.write(p+64,&[1])?;byte(m,p+0xb5)?};
        rc=invoke(m,h,table,44,p+32,&[s+24,s+28])?;
        if bad(rc) {return Ok(rc);}
        m.put_word(s+28,m.word(s+28)?>>1)?;
        let text=m.word(s+24)?;let text_count=m.word(s+28)?;
        rc=invoke(m,h,table,44,p+40,&[s+32,s+36])?;
        if bad(rc) {return Ok(rc);}
        m.put_word(s+36,m.word(s+36)?/24)?;
        let events=m.word(s+32)?;let event_count=m.word(s+36)?;
        rc=grow_array(m,p,s,s+4,s+8,24,event_count.wrapping_add(10),h)?;
        if bad(rc) {return Ok(rc);}
        m.put_word(s+8,0)?;
        if half(m,events)?==0x4000 {m.put_word(s+40,m.word(events+12)?)?;}
        else {invoke(m,h,table,68,p+32,&[s+40])?;}
        let base=m.word(s+40)?;
        string_copy(m,s+112,addr(h.rom(), 0x8f86c4),false)?;
        literal(m,s+72,b"lidon\0")?;m.put_word(s+44,0)?;
        let mut lid_state=0u32;
        if !bad(h.call(m,0x175a1c,&[props,s+72,s+44])?) {
            let property=m.word(s+44)?;
            string_copy(m,s+112,property,false)?;
            if compare_strings(m,property,addr(h.rom(), 0x8f86d2))?==0 {lid_state=1;}
            if string_len(m,property,false)?==3 {lid_state=2;}
        }
        let mut final_state=lid_state;let original_state=lid_state;
        let mut user_unknown=false;
        let mut i=0u16;
        while (i as u32)<event_count && !user_unknown {
            let e=events+i as u32*24;
            if half(m,e)?==36 && !wide_equal(m,m.word(e+20)?,addr(h.rom(), 0x8f874a))? &&
                !wide_equal(m,m.word(e+20)?,addr(h.rom(), 0x8f8776))? {user_unknown=true;}
            if byte(m,p+65)?!=0 && half(m,e)?==36 {
                if half(m,e+24)?==36 && wide_equal(m,m.word(e+20)?,addr(h.rom(), 0x8f8732))? &&
                    wide_equal(m,m.word(e+44)?,addr(h.rom(), 0x8f8722))? {
                    string_copy(m,m.word(e+20)?,addr(h.rom(), 0x8f8722),true)?;
                    literal(m,s+128,b"hasuserunknowninserted\0yes\0")?;
                    h.call(m,0x1759a0,&[props,s+128,s+151])?;
                }
                if byte(m,p+65)?!=0 && half(m,e)?==36 && wide_equal(m,m.word(e+20)?,addr(h.rom(), 0x8f8732))? {
                    string_copy(m,m.word(e+20)?,addr(h.rom(), 0x8f8722),true)?;
                }
            }
            i=i.wrapping_add(1);
        }
        let mut j=0u16;
        while (j as u32)<event_count {
            let e=events+j as u32*24;
            let mut process=false;let mut start=0;let mut length=0;let mut copy=true;
            if byte(m,p+67)?==2 && m.word(e+12)?!=base {length=text_count;process=true;}
            if half(m,e)?==36 {
                let value=m.word(e+20)?;
                if original_state!=0 {
                    if wide_equal(m,value,addr(h.rom(), 0x8f8722))? {lid_state=1;}
                    if string_len(m,value,true)?==3 {lid_state=2;}
                    final_state=lid_state;
                    let mut k=0u32;
                    loop {let c=half(m,value+k*2)?;m.write(s+112+k,&[c as u8])?;if c==0 {break;} k=k.wrapping_add(1);}
                }
                if half(m,e)?==36 && ((wide_equal(m,value,addr(h.rom(), 0x8f874a))? && !user_unknown) ||
                    wide_equal(m,value,addr(h.rom(), 0x8f8722))? || wide_equal(m,value,addr(h.rom(), 0x8f8760))?) {
                    m.write(p+0x354,&[0])?;
                    if wide_equal(m,value,addr(h.rom(), 0x8f8760))? {m.write(p+0x354,&[1])?;}
                    start=m.word(e+12)?.wrapping_sub(base)>>1;length=text_count.wrapping_sub(start);
                    copy=false;process=true;lid_state=0;
                    let mut k=j.wrapping_add(1);
                    while (k as u32)<event_count {
                        let next=events+k as u32*24;
                        if half(m,next)?==36 || half(m,next)?==7 {length=m.word(next+12)?.wrapping_sub(m.word(e+12)?)>>1;break;}
                        k=k.wrapping_add(1);
                    }
                }
            }
            if process && length!=0 {
                rc=analyze_segment(m,p,text,start,length,base,s,h)?;if bad(rc) {return Ok(rc);}
            }
            if copy {
                rc=insert_event(m,p,s,e,h)?;if bad(rc) {return Ok(rc);}
                copy_bytes(m,m.word(s+60)?,e,24)?;
            }
            if half(m,e)?==0x4000 {
                if lid_state==1 && (j as u32)<event_count.wrapping_sub(1) && half(m,e+24)?!=36 {
                    lid_state=0;
                    let start=m.word(e+12)?.wrapping_sub(base)>>1;
                    let mut length=text_count.wrapping_sub(start);
                    let mut k=j.wrapping_add(1);
                    while (k as u32)<event_count {
                        let next=events+k as u32*24;
                        if half(m,next)?==36 || half(m,next)?==7 {length=m.word(next+12)?.wrapping_sub(m.word(e+12)?)>>1;break;}
                        k=k.wrapping_add(1);
                    }
                    if length!=0 {rc=analyze_segment(m,p,text,start,length,base,s,h)?;if bad(rc) {return Ok(rc);}}
                }
                if half(m,e)?==0x4000 && lid_state==2 && (j as u32)<event_count.wrapping_sub(1) && half(m,e+24)?!=36 {
                    lid_state=0;
                    rc=insert_event(m,p,s,e,h)?;if bad(rc) {return Ok(rc);}
                    inserted=m.word(s+60)?;
                    copy_bytes(m,inserted,inserted.wrapping_sub(24),24)?;
                    put_half(m,inserted,36)?;
                    let value=alloc(m,p,string_len(m,addr(h.rom(), 0x8f8722),true)?.wrapping_add(1).wrapping_mul(2),h)?;
                    m.put_word(inserted+20,value)?;
                    if value==0 {return Ok(rc);}
                    widen(m,value,s+112)?;
                }
            }
            j=j.wrapping_add(1);
        }
        h.call(m,0x1759a0,&[props,s+72,match final_state {1=>addr(h.rom(), 0x8f86d2),2=>s+112,_=>addr(h.rom(), 0x8f86c4)}])?;
        let mut current_voice=addr(h.rom(), 0x8f8776);let mut previous_unknown=false;let mut escaped=false;
        i=0;
        while (i as u32)<m.word(s+8)? {
            let e=m.word(s)?+i as u32*24;
            match half(m,e)? {
                36=>{
                    let value=m.word(e+20)?;
                    if !wide_equal(m,value,addr(h.rom(), 0x8f8722))? && !wide_equal(m,value,addr(h.rom(), 0x8f874a))? && !wide_equal(m,value,addr(h.rom(), 0x8f8760))? {
                        current_voice=value;previous_unknown=false;i=i.wrapping_add(1);
                    } else {
                        m.put_word(s+8,m.word(s+8)?.wrapping_sub(1))?;
                        if (i as u32)<m.word(s+8)? {
                            copy_bytes(m,e,e+24,m.word(s+8)?.wrapping_sub(i as u32).wrapping_mul(24))?;
                            shift_indices(m,m.word(s+12)?,m.word(s+20)?,i as u32,false)?;
                        }
                    }
                    continue;
                }
                34=>escaped=true,
                21=>escaped=false,
                1=>if detect!=0 {
                    let mut unknown=false;
                    let offset=m.word(e+12)?.wrapping_sub(base);
                    if !escaped {
                        let start=offset>>1;let length=m.word(e+16)?>>1;let mut found=false;
                        for k in 0..length {
                            let c=half(m,text+start.wrapping_add(k).wrapping_mul(2))?;
                            let mut q=0;
                            loop {
                                let v=half(m,addr(h.rom(), 0x8f878e)+q*2)?;
                                if v==c {found=true;break;}
                                if v==0 {break;}
                                q+=1;
                            }
                            if found {break;}
                        }
                        if found {
                            number=h.call(m,0x1963e0,&[allocator(m,p)?,number,length.wrapping_add(7)])?;
                            if number==0 {allocation_log(m,p,false,h)?;return Ok(0x8bd0200a);}
                            literal(m,number,b"mlnum_\0")?;
                            let mut k=0u16;
                            while (k as u32)<length && !unknown {
                                let c=half(m,text+(k as u32).wrapping_add(offset>>1).wrapping_mul(2))?;
                                if c>=255 {unknown=true;} else {m.write(number+k as u32+6,&[c as u8])?;}
                                k=k.wrapping_add(1);
                            }
                            if !unknown {
                                m.write(number+k as u32+6,&[0])?;
                                put_half(m,s+48,if byte(m,p+64)?!=0 {1} else {0xffff})?;
                                literal(m,s+80,b"lidcfg\0")?;
                                m.put_word(s+56,lid_state)?;
                                rc=invoke(m,h,lid,28,p+4,&[s+80,number,s+52,s+48,s+56])?;
                                if bad(rc) {return Ok(rc);}
                                lid_state=m.word(s+56)?;
                                if half(m,s+48)?==0 {unknown=true;}
                            }
                        }
                    }
                    if previous_unknown!=unknown && !wide_equal(m,current_voice,addr(h.rom(), 0x8f8776))? {
                        rc=grow_array(m,p,s,s+4,s+8,24,10,h)?;if bad(rc) {return Ok(rc);}
                        let e=m.word(s)?+i as u32*24;
                        copy_bytes(m,e+24,e,m.word(s+8)?.wrapping_sub(i as u32+1).wrapping_mul(24))?;
                        shift_indices(m,m.word(s+12)?,m.word(s+20)?,i as u32,true)?;
                        put_half(m,e,36)?;m.put_word(e+20,if unknown {addr(h.rom(), 0x8f8776)} else {current_voice})?;
                        m.put_word(e+4,0)?;m.put_word(e+8,0)?;m.put_word(e+12,m.word(e+36)?)?;m.put_word(e+16,0)?;
                        i=i.wrapping_add(1);
                    }
                    previous_unknown=unknown;
                },
                _=>{}
            }
            i=i.wrapping_add(1);
        }
        if text_count!=0 {
            rc=invoke(m,h,table,52,p+48,&[text,text_count.wrapping_mul(2)])?;
            if bad(rc) {return Ok(rc);}
        }
        rc=invoke(m,h,table,52,p+56,&[m.word(s)?,m.word(s+8)?.wrapping_mul(24)])?;
        if bad(rc) {return Ok(rc);}
        if text_count!=0 {
            rc=invoke(m,h,table,48,p+32,&[text_count.wrapping_mul(2)])?;
            if bad(rc) {return Ok(rc);}
        }
        invoke(m,h,table,48,p+40,&[event_count.wrapping_mul(24)])
    })();
    // Do not run speculative cleanup after an unwired dependency. Native error
    // returns, including OOM, do execute the ROM's cleanup sequence.
    let rc=result?;
    for k in 0..m.word(s+20)? {
        let event=m.word(s)?+m.word(m.word(s+12)?+k*4)?.wrapping_mul(24);
        if m.word(event+20)?!=0 {free(m,p,m.word(event+20)?,h)?;}
    }
    if inserted!=0 && m.word(inserted+20)?!=0 {free(m,p,m.word(inserted+20)?,h)?;}
    for value in [m.word(s)?,m.word(s+12)?,number] {if value!=0 {free(m,p,value,h)?;}}
    Ok(rc)
}

fn destroy_models(m:&mut Memory<'_, '_>,p:u32,h:&mut impl InternalCalls)->Result<u32,Error>{
    let mut i=0u16;
    while i<half(m,p+0x344)? {
        free(m,p,m.word(m.word(p+0x348)?+i as u32*48+12)?,h)?;
        i=i.wrapping_add(1);
    }
    if m.word(p+0x348)?!=0 {free(m,p,m.word(p+0x348)?,h)?;}
    m.put_word(p+0x348,0)?;put_half(m,p+0x344,0)?;put_half(m,p+0x346,0)?;
    for model in [p+0x48,p+0x60] {
        let mut i=0u16;
        while i<half(m,model+8)? {
            free(m,p,m.word(m.word(model+12)?+i as u32*528+12)?,h)?;
            i=i.wrapping_add(1);
        }
        for off in [12,20] {if m.word(model+off)?!=0 {free(m,p,m.word(model+off)?,h)?;}}
    }
    Ok(0)
}

/// Platform operations are the CP15 identity and kernel address arbitration
/// only. Allocator and stream callbacks always execute the bridge's Rust ports.
pub trait ComponentPlatform {
    fn thread_id(&mut self) -> u32;
    fn arbiter_address(&self) -> u32;
    fn arbitrate(&mut self, memory:&mut crate::boot::Memory<'_>,args:&[u32])->u32;
}
struct PlatformAdapter<'a,P>(&'a mut P);
impl<P:ComponentPlatform> crate::bridge::Platform for PlatformAdapter<'_,P> {
    fn thread_id(&mut self)->u32 {self.0.thread_id()}
    fn arbiter_address(&self)->u32 {self.0.arbiter_address()}
    fn arbitrate(&mut self,m:&mut crate::boot::Memory<'_>,a:&[u32])->u32 {self.0.arbitrate(m,a)}
}

/// Live bridge bindings with caller-owned absolute-index workspace. It must
/// reach the end of the highest guest mapping. No storage is allocated here.
/// ReadCalls remains a fail-closed fallback for callers without workspace.
pub struct BridgeCalls<'a,P> {
    pub workspace:&'a mut [u8],
    pub platform:&'a mut P,
    /// The ROM whose guest memory is live. The caller sets it; US keeps the
    /// old behavior exactly. Regional entries are also honored per call.
    pub rom: Rom,
}
impl<P:ComponentPlatform> InternalCalls for BridgeCalls<'_,P> {
    fn rom(&self) -> Rom {
        self.rom
    }
    fn call(&mut self,m:&mut Memory<'_, '_>,entry:u32,a:&[u32])->Result<u32,Error>{
        let rom = self.rom.or(Rom::from_entry(entry));
        let entry = to_us(rom, entry);
        // Null heap calloc reads no guest memory. Use the exact bridge entry
        // on an empty input view rather than requiring a giant unused workspace.
        if entry==0x1963b8 && (a[0]==0 || a[1]==0 || a[2]==0) {
            return null_calloc(a);
        }
        match entry {
            0x1795f0 => open_broker_data(m, self.workspace, a, rom),
            0x179f3c => {
                // The broker's close, over the flat arena.
                let rc = flat_stage(m, self.workspace, |flat| {
                    let mut host = broker_host(rom);
                    let rc = {
                        let mut close = CloseHost { inner: &mut host };
                        crate::leftovers_1::fn_179f3c(flat, a[0], a[1], &close_runtime(rom), &mut close)
                    };
                    if let Some(object) = host.pending_release.take() {
                        let heap = flat.word(flat.word(flat.word(a[0] + 4) + 8) + 4);
                        crate::boot_part_e::release(flat, heap, object, &mut host);
                    }
                    rc
                })?;
                Ok(rc)
            }
            0x26b718 => {
                // The opened module's key lookup, over the flat arena.
                let rc = flat_stage(m, self.workspace, |flat| {
                    let mut host = broker_host(rom);
                    crate::speech_part_2::fn_26b718(
                        flat,
                        a[0],
                        a[1],
                        a[2],
                        a.get(4).copied().unwrap_or(0),
                        LID_SCRATCH,
                        &mut host,
                    )
                })?;
                Ok(rc)
            }
            0x17a1a4 => {
                // The archive range path, over the flat arena.
                let rc = flat_stage(m, self.workspace, |flat| {
                    let mut host = broker_host(rom);
                    crate::component_live_c::broker_range_live_m(
                        flat,
                        0x17a1a4,
                        a,
                        RANGE_SCRATCH,
                        &mut host,
                    )
                })?;
                Ok(rc)
            }
            0x196394|0x1963b8|0x196428|0x14f918|0x175a1c|0x150878|0x4a79b8|0x1503b0|0x15017c|0x150c0c|0x14e440|0x2409d4|0x17a1a4|
            0x184f20|0x184f84|0x163f20|0x39f5e8|0x4a7758|
            0x4a779c|0x4a774c|0x39f5b8|0x19b78c=>{
                let mut platform=PlatformAdapter(self.platform);
                let allocator=if matches!(entry,0x196394|0x1963b8|0x196428) {a[0]} else {0};
                let mut f=crate::bridge::Flat::new(m,self.workspace,&mut platform,allocator)?;
                let rc=match entry {
                    0x196394=>f.fn_196394(a[0],a[1]),
                    0x1963b8=>f.fn_1963b8(a[0],a[1],a[2]),
                    0x196428=>{f.fn_196428(a[0],a[1]);0}
                    0x2409d4=>f.fn_2409d4([a[0],a[1],a[2],a[3]]),
                    0x1503b0=>f.fn_1503b0_quiet([a[0],a[1],a[2],a[3]]),
                    0x15017c=>f.fn_15017c([a[0],a[1],a[2]]),
                    0x150c0c=>f.fn_150c0c([a[0],a[1],a[2]]),
                    0x14e440=>{f.fn_14e440(a[0],a[1]);0}
                    0x150878=>f.fn_150878([a[0],a[1],a[2],a[3]]),
                    0x4a79b8=>f.fn_4a79b8_speech([a[0],a[1],a[2]]),
                    0x175a1c=>f.fn_175a1c([a[0],a[1],a[2]],self.rom),
                    0x14f918=>f.fn_14f918_shared_quiet([a[0],a[1]]),
                    0x17a1a4=>f.fn_17a1a4([a[0],a[1],a[2],a[3],a[4],a[5]]),
                    _=>f.allocator_method(entry,a),
                };
                f.finish()?;
                Ok(rc)
            }
            _=>read_call(m,entry,a,self),
        }
    }
}

/// The live broker's host over the caller's scratch areas.
fn broker_host(rom: Rom) -> LiveBrokerHost {
    LiveBrokerHost {
        query: QUERY_SCRATCH,
        data: DATA_SCRATCH,
        files: FILE_SCRATCH,
        lid: LID_SCRATCH,
        unmap: UNMAP_SCRATCH,
        close: CLOSE_SCRATCH,
        pending_release: None,
        rom,
    }
}

/// The broker's DataOpen, over the flat view of the caller's arena.
///
/// The broker's own services are the database query (fn_1772b0 over the
/// map's iterator ports), the allocator, the property service and the
/// logger.  The file interface is the live file platform over the archive
/// the real cartridge files filled.
fn open_broker_data(
    m: &mut Memory<'_, '_>,
    workspace: &mut [u8],
    a: &[u32],
    rom: Rom,
) -> Result<u32, Error> {
    let broker = a.first().copied().unwrap_or(0);
    let name = a.get(1).copied().unwrap_or(0);
    let flags = a.get(2).copied().unwrap_or(0);
    let out = a.get(3).copied().unwrap_or(0);
    flat_stage(m, workspace, |flat| {
        let mut host = broker_host(rom);
        let globals = broker_for(rom).data_open(DATA_SCRATCH);
        let mut files = live_fs::FilePlatform {
            env: archive_environment(rom),
            scratch: FILE_SCRATCH,
            rb: rb_literal(rom),
        };
        let mut open = DataOpenHost { inner: &mut host, platform: &mut files };
        crate::boot_part_b::fn_1795f0(flat, broker, name, flags, out, &globals, &mut open)
    })
}
struct NoPlatform;
impl crate::bridge::Platform for NoPlatform {
    fn thread_id(&mut self)->u32 {unreachable!("null allocator cannot lock")}
    fn arbiter_address(&self)->u32 {unreachable!("null allocator cannot arbitrate")}
    fn arbitrate(&mut self,_:&mut crate::boot::Memory<'_>,_:&[u32])->u32 {unreachable!()}
}
fn null_calloc(a:&[u32])->Result<u32,Error>{
    let mut regions=[];let mut memory=Memory::new(&mut regions)?;
    let mut platform=NoPlatform;let mut bytes=[];
    let mut f=crate::bridge::Flat::new(&mut memory,&mut bytes,&mut platform,0)?;
    let rc=f.fn_1963b8(a[0],a[1],a[2]);f.finish()?;Ok(rc)
}

/// fn_1765ac. Wide-string compare, 0 on equal.
fn wide_cmp(m:&Memory<'_, '_>,mut a:u32,mut b:u32)->Result<i32,Error>{
    loop {
        let x=half(m,a)?;let y=half(m,b)?;
        if x==0 || x!=y {return Ok(x as i32-y as i32);}
        a=a.wrapping_add(2);b=b.wrapping_add(2);
    }
}
/// fn_1765dc. Wide prefix compare over n units, 0 on match.
fn wide_ncmp(m:&Memory<'_, '_>,mut a:u32,mut b:u32,mut n:u32)->Result<i32,Error>{
    while n!=0 {
        let x=half(m,a)?;let y=half(m,b)?;
        if x==0 || x!=y {return Ok(x as i32-y as i32);}
        n-=1;a=a.wrapping_add(2);b=b.wrapping_add(2);
    }
    Ok(0)
}
/// fn_176624. Wide strlen.
fn wide_len(m:&Memory<'_, '_>,mut s:u32)->Result<u32,Error>{
    let mut n=0u32;
    while half(m,s)?!=0 {s=s.wrapping_add(2);n=n.wrapping_add(1);}
    Ok(n)
}
/// fn_176514. Locate wide char c at/after s, 0 on miss.
fn wide_chr(m:&Memory<'_, '_>,mut s:u32,c:u32)->Result<u32,Error>{
    loop {
        let x=half(m,s)? as u32;
        if x==c {return Ok(s);}
        if x==0 {return Ok(0);}
        s=s.wrapping_add(2);
    }
}
/// fn_176214. Wide copy with terminator.
fn wide_copy(m:&mut Memory<'_, '_>,mut d:u32,mut s:u32)->Result<(),Error>{
    loop {
        let x=half(m,s)?;
        put_half(m,d,x)?;
        if x==0 {return Ok(());}
        d=d.wrapping_add(2);s=s.wrapping_add(2);
    }
}
#[allow(clippy::too_many_arguments)]
fn spelling_loop(m:&mut Memory<'_, '_>,p:u32,s:u32,mut records:u32,owned:u32,marks:u32,capacity:u16,count:u32,
    text:u32,text_len:u16,mut base:u32,leading:u32,trailing:u32,trailing_nulls:u16,single:bool,h:&mut impl InternalCalls)->Result<u32,Error> {
    let table=m.word(p+28)?;
    let logger=m.word(m.word(p)?+16)?;
    m.put_word(s+68,marks)?;put_half(m,s+72,capacity)?;
    let mut mark_count=0u16;let mut record_index=0u16;let mut delta=0i16;
    let mut spell_mode=0;let mut in_marker=0;
    // Mode-name records for the 0x15/0x22 dispatch (src/fn_219790.c):
    // guest wide strings in the caller-owned frame past the s+450 slot.
    let spell_nm=s+460;let spell_pre=s+472;let spell_alnum=s+488;
    let alnum_nm=s+528;let digits_nm=s+556;let chars_nm=s+572;let normal_nm=s+588;
    let mut copy_marker=0u32;let mut saved_mode=normal_nm;let mut alnum_mode=0u32;
    let mut default_lang=9999u32;
    literal(m,s+380,b"spell\0")?;widen(m,spell_nm,s+380)?;
    literal(m,s+380,b"spell:\0")?;widen(m,spell_pre,s+380)?;
    literal(m,s+380,b"spell:alphanumeric\0")?;widen(m,spell_alnum,s+380)?;
    literal(m,s+380,b"alphanumeric\0")?;widen(m,alnum_nm,s+380)?;
    literal(m,s+380,b"digits\0")?;widen(m,digits_nm,s+380)?;
    literal(m,s+380,b"characters\0")?;widen(m,chars_nm,s+380)?;
    literal(m,s+380,b"normal\0")?;widen(m,normal_nm,s+380)?;
    let mut previous_language=2;
    let mut skip_space=true;
    let mut rc=0;
    // The ROM keeps the split spelling and language names across characters:
    // only translation, the current language and the language index reset
    // per character (src/fn_219790.c lookup_character, check_translation).
    let mut spelling_addr=0u32;
    let mut language_name=0u32;
    // Saved mode strings and per-character temporaries are ABI locals.
    let result=(||->Result<u32,Error>{
        let mut text_index=0u16;
        while text_index<text_len {
            while (record_index as u32)<count {
                let e=records+record_index as u32*24;
                if m.word(e+12)?.wrapping_sub(base)&65535 != (text_index as u32*2)&65535 {break;}
                let position=m.word(e+12)?.wrapping_add(m.word(s+20)?).wrapping_add(delta as i32 as u32) as i32;
                m.put_word(e+12,position.max(0) as u32)?;
                match half(m,e)? {
                    8=>{
                        // fn_219790.c type-8: skip unless in spell/marker text,
                        // else ensure a preceding space (space_before_marker).
                        if spell_mode==0 && in_marker==0 && text_index!=0 {
                            let prev=half(m,text+(text_index as u32-1)*2)?;
                            if prev!=0x20 {
                                literal(m,s+360,b" \0")?;
                                rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                                rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                                m.write(m.word(s+68)?+mark_count as u32*4+2,&[1])?;mark_count=mark_count.wrapping_add(1);
                                delta=delta.wrapping_add(1);
                                m.put_word(e+12,m.word(e+12)?.wrapping_add(1))?;
                            }
                        }
                    }
                    0x15=>{
                        // src/fn_219790.c mode record: leave spell mode on a
                        // full spell-mode record, remember the mode, pad with a
                        // space, then run the mode dispatch (type is 0x15).
                        let f20=m.word(e+20)?;
                        if spell_mode==1
                            && wide_cmp(m,f20,spell_nm)?==0 && wide_ncmp(m,f20,spell_pre,6)?==0
                            && wide_cmp(m,f20,alnum_nm)?==0 && wide_cmp(m,f20,digits_nm)?==0
                            && wide_cmp(m,f20,chars_nm)?==0 {
                            spell_mode=0;
                        }
                        saved_mode=f20;
                        if text_index!=0 {
                            let prev=half(m,text+(text_index as u32-1)*2)?;
                            if prev!=0x20 {
                                literal(m,s+360,b" \0")?;
                                rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                                rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                                m.write(m.word(s+68)?+mark_count as u32*4+2,&[1])?;mark_count=mark_count.wrapping_add(1);
                                delta=delta.wrapping_add(1);
                                m.put_word(e+12,m.word(e+12)?.wrapping_add(1))?;
                            }
                        }
                        in_marker=0;copy_marker=0;
                        if wide_cmp(m,f20,spell_nm)?==0 || wide_ncmp(m,f20,spell_pre,6)?==0
                            || wide_cmp(m,f20,alnum_nm)?==0 || wide_cmp(m,f20,digits_nm)?==0
                            || wide_cmp(m,f20,chars_nm)?==0 {
                            spell_mode=1;
                            if wide_cmp(m,f20,spell_alnum)?==0 || wide_cmp(m,f20,alnum_nm)?==0
                                || wide_cmp(m,f20,digits_nm)?==0 || wide_cmp(m,f20,chars_nm)?==0 {
                                alnum_mode=1;
                            } else {
                                alnum_mode=0;
                            }
                        } else if spell_mode!=0 {
                            spell_mode=0;
                            alnum_mode=0;
                        }
                    }
                    0x24=>{
                        // src/fn_219790.c language record: pad like
                        // space_before_marker, then resolve the ASCII language
                        // name and select it as the default language.
                        if text_index!=0 {
                            let prev=half(m,text+(text_index as u32-1)*2)?;
                            if prev!=0x20 {
                                literal(m,s+360,b" \0")?;
                                rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                                rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                                m.write(m.word(s+68)?+mark_count as u32*4+2,&[1])?;mark_count=mark_count.wrapping_add(1);
                                delta=delta.wrapping_add(1);
                                m.put_word(e+12,m.word(e+12)?.wrapping_add(1))?;
                            }
                        }
                        {
                            let ws=m.word(e+20)?;
                            let wl=wide_len(m,ws)?;
                            let tmp=h.call(m,0x196394,&[allocator(m,p)?,wl.wrapping_add(1)])?;
                            if tmp==0 {
                                h.call(m,0x173dc0,&[logger,addr(h.rom(),0x8fa20c),30000,0])?;
                                return Ok(0x8a60200a);
                            }
                            let mut li=0u32;
                            while half(m,ws+li.wrapping_mul(2))?!=0 {
                                let c=half(m,ws+li.wrapping_mul(2))? as u32;
                                m.write(tmp+li,&[crate::normalization::tolower_240ba0(c & 0xff) as u8])?;
                                li=li.wrapping_add(1);
                            }
                            m.write(tmp+li,&[0])?;
                            literal(m,s+380,b"normal\0")?;
                            if compare_strings(m,tmp,s+380)?==0 {
                                default_lang=9999;
                            } else {
                                rc=intern_pair(m,p,p+68,p+72,tmp,s+76,h)?;
                                if bad(rc) {free(m,p,tmp,h)?;return Ok(rc);}
                                default_lang=half(m,s+76)? as u32;
                            }
                            free(m,p,tmp,h)?;
                            put_half(m,e,0x4000)?;m.put_word(e+16,0)?;m.put_word(e+8,0)?;
                        }
                    }
                    0x22=>{
                        // src/fn_219790.c text_marker: following marker text
                        // is copied raw. (C also tests type==0x15 for
                        // check_mode here, unreachable: type is 0x22.)
                        let q=wide_chr(m,m.word(e+20)?,0x14)?;
                        in_marker=1;
                        if q!=0 {copy_marker=1;put_half(m,q,0)?;}
                    }
                    7=>{
                        // src/fn_219790.c check_span_marker: only '+' spans
                        // run spelling_span; anything else is next.
                        if m.word(e+20)?==0x2b {
                            let mut span_hash=0u32;let mut span_start=1u32;let mut span_space=0u32;
                            let span_marker=record_index as u32;
                            let mut span_end=text_index;let mut span_state=0u32;
                            if text_index!=0 {
                                let prev=half(m,text+(text_index as u32-1)*2)?;
                                if prev!=0x20 {
                                    literal(m,s+360,b" \0")?;
                                    rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                                    rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                                    m.write(m.word(s+68)?+mark_count as u32*4+2,&[1])?;mark_count=mark_count.wrapping_add(1);
                                    delta=delta.wrapping_add(1);
                                    m.put_word(e+12,m.word(e+12)?.wrapping_add(1))?;
                                }
                            }
                            record_index=record_index.wrapping_add(1);
                            while ((record_index as u32)<count
                                && (half(m,records+record_index as u32*24)?!=7
                                    || m.word(records+record_index as u32*24+12)?.wrapping_sub(base) as u16>span_end.wrapping_mul(2)))
                                || ((record_index as u32)==count && span_end<text_len) {
                                {
                                    let position=span_end.wrapping_mul(2);
                                    while (record_index as u32)<count {
                                        let re=records+record_index as u32*24;
                                        if m.word(re+12)?.wrapping_sub(base) as u16!=position {break;}
                                        if half(m,re)?==7 {break;}
                                        let t=half(m,re)?;
                                        if t==0x15 || t==0x24 {
                                            put_half(m,re,0x4000)?;m.put_word(re+16,0)?;m.put_word(re+8,0)?;
                                        }
                                        let d=m.word(re+12)?.wrapping_add(m.word(s+20)?).wrapping_add(delta as i32 as u32) as i32;
                                        record_index=record_index.wrapping_add(1);
                                        m.put_word(re+12,d.max(0) as u32)?;
                                        if (record_index as u32)<count
                                            && m.word(records+record_index as u32*24+12)?.wrapping_sub(base) as u16==position
                                            && half(m,records+record_index as u32*24)?==7 {
                                            span_state=1;
                                        }
                                    }
                                    let ch=half(m,text+span_end as u32*2)?;
                                    if ch!=0x20 {
                                        if span_state==0 {span_hash=0;}
                                        if (ch==0x5f || ch==0x23) && span_end!=text_index
                                            && half(m,text+(span_end.wrapping_sub(1) as u32)*2)?!=0x20
                                            && half(m,text+(span_end.wrapping_sub(1) as u32)*2)?!=0x5f
                                            && half(m,text+(span_end.wrapping_sub(1) as u32)*2)?!=0x23
                                            && (span_end<4
                                                || half(m,text+(span_end.wrapping_sub(1) as u32)*2)?!=0x5c
                                                || (half(m,text+(span_end.wrapping_sub(3) as u32)*2)?!=0x74
                                                    && half(m,text+(span_end.wrapping_sub(3) as u32)*2)?!=0x54)
                                                || half(m,text+(span_end.wrapping_sub(4) as u32)*2)?!=0x5c) {
                                            if ch==0x23 {span_hash=1;}
                                            if span_start==0 {
                                                literal(m,s+360,b" \0")?;
                                                rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                                                rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                                                put_half(m,m.word(s+68)?+mark_count as u32*4,default_lang as u16)?;
                                                m.write(m.word(s+68)?+mark_count as u32*4+2,&[1])?;
                                                mark_count=mark_count.wrapping_add(1);
                                                delta=delta.wrapping_add(1);
                                            }
                                            literal(m,s+360,&[0x15,0])?;
                                            rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                                            skip_space=false;
                                            if bad(rc) {return Ok(rc);}
                                            rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                                            delta=delta.wrapping_add(1);
                                            put_half(m,m.word(s+68)?+mark_count as u32*4,default_lang as u16)?;
                                            mark_count=mark_count.wrapping_add(1);
                                            span_start=0;
                                        }
                                    }
                                    if span_state==0 {
                                        span_end=span_end.wrapping_add(1);
                                        delta=delta.wrapping_sub(2);
                                    }
                                }
                            }
                            if (record_index as u32)>=count && ((record_index as u32)!=count || span_end!=text_len) {
                                // next_marker: fall through to the common increment.
                            } else {
                                let span_len=span_end.wrapping_sub(text_index) as u32;
                                let span_bytes=span_len.wrapping_mul(2);
                                let tmp=h.call(m,0x196394,&[allocator(m,p)?,span_bytes.wrapping_add(2)])?;
                                if tmp==0 {
                                    h.call(m,0x173dc0,&[logger,addr(h.rom(),0x8fa20c),40000,0])?;
                                    return Ok(0x8a60200a);
                                }
                                copy_bytes(m,tmp,text+text_index as u32*2,span_bytes)?;
                                put_half(m,tmp+span_len.wrapping_mul(2),0)?;
                                {
                                    let se2=records+span_marker.wrapping_mul(24);
                                    put_half(m,se2,0x22)?;m.put_word(se2+20,tmp)?;
                                }
                                m.put_word(owned+span_marker.wrapping_mul(4),tmp)?;
                                text_index=span_end;
                                if span_hash==0 {
                                    if span_start==0 {
                                        literal(m,s+360,b" \0")?;
                                        rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                                        rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                                        delta=delta.wrapping_add(1);
                                        put_half(m,m.word(s+68)?+mark_count as u32*4,default_lang as u16)?;
                                        m.write(m.word(s+68)?+mark_count as u32*4+2,&[1])?;
                                        mark_count=mark_count.wrapping_add(1);
                                    }
                                    literal(m,s+360,&[0x15,0])?;
                                    rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                                    skip_space=false;
                                    if bad(rc) {return Ok(rc);}
                                    rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                                    delta=delta.wrapping_add(1);
                                    put_half(m,m.word(s+68)?+mark_count as u32*4,default_lang as u16)?;
                                    mark_count=mark_count.wrapping_add(1);
                                }
                                if (record_index as u32)!=count {
                                    let wlen=wide_len(m,saved_mode)?;
                                    let tmp2=h.call(m,0x196394,&[allocator(m,p)?,wlen.wrapping_add(1).wrapping_mul(2)])?;
                                    if tmp2==0 {
                                        h.call(m,0x173dc0,&[logger,addr(h.rom(),0x8fa20c),40000,0])?;
                                        return Ok(0x8a60200a);
                                    }
                                    wide_copy(m,tmp2,saved_mode)?;
                                    {
                                        let re2=records+record_index as u32*24;
                                        put_half(m,re2,0x15)?;m.put_word(re2+20,tmp2)?;
                                        let d2=m.word(re2+12)?.wrapping_add(m.word(s+20)?).wrapping_add(delta as i32 as u32) as i32;
                                        m.put_word(re2+12,d2.max(0) as u32)?;
                                    }
                                }
                                if span_end<text_len && half(m,text+span_end as u32*2)? & 0xff==0x20 {
                                    span_space=1;
                                }
                                while span_end<text_len && half(m,text+span_end as u32*2)? & 0xff==0x20 {
                                    span_end=span_end.wrapping_add(1);
                                }
                                if span_end as u32*2+2 != m.word(records+16)? {
                                    if span_space==0 {
                                        literal(m,s+360,b" \0")?;
                                        rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                                        rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                                        delta=delta.wrapping_add(1);
                                        m.write(m.word(s+68)?+mark_count as u32*4+2,&[1])?;mark_count=mark_count.wrapping_add(1);
                                    }
                                } else {
                                    let ch2=half(m,text+span_end as u32*2)?;
                                    let lc=ch2 & 0xff;
                                    if lc!=0x2e && lc!=0x3f && lc!=0x3b && lc!=0x21 {
                                        if span_space==0 {
                                            literal(m,s+360,b" \0")?;
                                            rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                                            rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                                            delta=delta.wrapping_add(1);
                                            m.write(m.word(s+68)?+mark_count as u32*4+2,&[1])?;mark_count=mark_count.wrapping_add(1);
                                        }
                                    } else {
                                        m.write(s+380,&[ch2 as u8,0])?;
                                        rc=invoke(m,h,table,52,p+48,&[s+380,1])?;if bad(rc) {return Ok(rc);}
                                        rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                                        if byte(m,s+380)?==b' ' {
                                            m.write(m.word(s+68)?+mark_count as u32*4+2,&[1])?;
                                        }
                                        text_index=span_end.wrapping_add(1);
                                        mark_count=mark_count.wrapping_add(1);
                                        m.put_word(records+record_index as u32*24+12,m.word(records+record_index as u32*24+12)?.wrapping_add(1))?;
                                        {
                                            let marker_f12=m.word(records+record_index as u32*24+12)?;
                                            while (record_index as u32)<count {
                                                m.put_word(records+record_index as u32*24+12,marker_f12)?;
                                                record_index=record_index.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _=>{}
                }
                record_index=record_index.wrapping_add(1);
            }
            put_half(m,s+64,9999)?;
            language_name=0;
            let ch=half(m,text+text_index as u32*2)?;
            if ch==32 && skip_space {
                delta=delta.wrapping_sub(2);
                text_index=text_index.wrapping_add(1);
                continue;
            }
            let ascii=ch==32 || ch.wrapping_sub(48)<=9 || ch.wrapping_sub(65)<=25 || ch.wrapping_sub(97)<=25;
            let lookup=if copy_marker!=0 {false} else {match byte(m,p+65)? {0=>!ascii,1=>single || !ascii,_=>true}};
            if lookup || copy_marker!=0 {
                let mut translation:u32;
                let mut language=2u32;
                if copy_marker!=0 {
                    // src/fn_219790.c copyMarkerText path: raw char copy
                    // straight to translation_ready.
                    m.write(s+320,&[ch as u8,0])?;
                    spelling_addr=s+320;
                    translation=s+320;
                } else if in_marker!=0 && ch==0x15 {
                    // src/fn_219790.c lookup_character in-marker 0x15 path:
                    // fixed 0x15 translation with no dictionary lookup.
                    m.write(s+320,&[0x15,0])?;
                    spelling_addr=s+320;
                    translation=s+320;
                } else {
                let mut local=[0u8;40];
                crate::normalization::itoa(&mut local,ch as u32,0,16);
                let n=local.iter().position(|x| *x==0).unwrap();
                m.write(s+320,&local[..n+1])?;
                put_half(m,s+340,0xffff)?;
                literal(m,s+352,b"unixlit\0")?;
                rc=invoke(m,h,m.word(p+12)?,28,p+4,&[s+352,s+320,s+344,s+340,s+342,s+450])?;
                if bad(rc) {return Ok(rc);}
                // ROM continuation (src/fn_219790.c lookup_character tail):
                // split translation[sep]spelling[sep]language, resolve the
                // language, then check/ready/switch/emit like the ROM.
                let translations=m.word(s+344)?;
                let translation_count=half(m,s+340)? as i16;
                let separator=byte(m,s+342)? as u32;
                if translation_count==0 {
                    // translation is still null (reset for every character).
                    delta=delta.wrapping_sub(2);
                    text_index=text_index.wrapping_add(1);
                    continue;
                }
                translation=m.word(translations)?;
                if byte(m,translation)?==0xb0 {m.write(translation,&[0x20])?;}
                let f=find_separator(m,translation,separator)?;
                if f!=0 {
                    if f!=translation && byte(m,f.wrapping_sub(1))?==0xb0 {
                        m.write(f.wrapping_sub(1),&[0x20])?;
                    }
                    spelling_addr=f.wrapping_add(1);
                    m.write(f,&[0])?;
                    let g=find_separator(m,spelling_addr,separator)?;
                    if g!=0 {
                        language_name=g;
                        m.write(g,&[0])?;
                        if byte(m,translation)?!=0 {
                            language_name=g.wrapping_add(1);
                            if byte(m,language_name)?==0 {language_name=0;}
                            if language_name!=0 {
                                rc=intern_pair(m,p,p+68,p+72,language_name,s+64,h)?;
                                if bad(rc) {return Ok(rc);}
                                if compare_strings(m,language_name,s+134)?==0 {
                                    language=0;
                                } else if compare_strings(m,language_name,s+128)?==0 {
                                    language=1;
                                }
                            }
                        }
                    }
                }
                }
                // check_translation: a null or empty translation skips the
                // character; a missing spelling defaults to the translation.
                if translation==0 {delta=delta.wrapping_sub(2);text_index=text_index.wrapping_add(1);continue;}
                if byte(m,translation)?==0 {delta=delta.wrapping_sub(2);text_index=text_index.wrapping_add(1);continue;}
                if spelling_addr==0 || byte(m,spelling_addr)?==0 {spelling_addr=translation;}
                // translation_ready.
                literal(m,s+360,b" \0")?;
                if compare_strings(m,translation,s+360)?==0 {
                    let t2=delta as i32+text_index as i32*2;
                    if t2==0 && !single {delta=delta.wrapping_sub(2);text_index=text_index.wrapping_add(1);continue;}
                }
                // switch_language.
                if (previous_language==1 && language==0) || (previous_language==0 && language==1) {
                    literal(m,s+360,b" \0")?;
                    rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                    rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                    m.write(m.word(s+68)?+mark_count as u32*4+2,&[1])?;mark_count=mark_count.wrapping_add(1);
                }
                // emit_translation.
                previous_language=language;
                let spell_path=copy_marker==0 && (single || spell_mode!=0);
                let mut do_spell=false;
                if spell_path {
                    do_spell=true;
                    if alnum_mode!=0 {
                        let chc=half(m,text+text_index as u32*2)? as u32;
                        do_spell=chc>=14 && chc!=0x20 && chc.wrapping_sub(0x20)>15
                            && chc.wrapping_sub(0x3a)>6 && chc.wrapping_sub(0x5b)>5 && chc.wrapping_sub(0x7b)>4;
                    }
                }
                if do_spell {
                    // src/fn_219790.c write_spelling: spell the name out.
                    let slen=string_len(m,spelling_addr,false)?;
                    if slen>1 {
                        literal(m,s+360,&[0x11,0])?;
                        rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                        rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                        mark_count=mark_count.wrapping_add(1);
                        delta=delta.wrapping_add(1);
                    }
                    rc=invoke(m,h,table,52,p+48,&[spelling_addr,slen])?;
                    skip_space=false;
                    if bad(rc) {return Ok(rc);}
                    rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,slen,h)?;if bad(rc) {return Ok(rc);}
                    if slen!=0 {
                        let spell_marks=m.word(s+68)?;
                        for k in 0..slen {
                            if byte(m,spelling_addr+k)?==b' ' {
                                m.write(spell_marks+mark_count as u32*4+k*4+2,&[1])?;
                            }
                        }
                        mark_count=mark_count.wrapping_add(slen as u16);
                    }
                    if bad(rc) {return Ok(rc);}
                    delta=delta.wrapping_add(slen as i16).wrapping_sub(2);
                    if slen>1 {
                        literal(m,s+360,&[0x11,0])?;
                        rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                        rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                        mark_count=mark_count.wrapping_add(1);
                        delta=delta.wrapping_add(1);
                    }
                    text_index=text_index.wrapping_add(1);
                    continue;
                }
                if spell_path {
                    delta=delta.wrapping_sub(2);
                    text_index=text_index.wrapping_add(1);
                    continue;
                }
                // Shared translation write.
                if in_marker!=0 {
                    // src/fn_219790.c: dash runs inside marker text.
                    let dash_len=string_len(m,translation,false)?;
                    if dash_len!=1
                        && (find_separator(m,translation,0x20)?!=0 || find_separator(m,translation,0x5f)?!=0) {
                        for k in 0..dash_len {
                            if byte(m,translation+k)?==b' ' || byte(m,translation+k)?==b'_' {
                                m.write(translation+k,&[b'-'])?;
                            }
                        }
                    }
                }
                skip_space=false;
                let tlen=string_len(m,translation,false)?;
                rc=invoke(m,h,table,52,p+48,&[translation,tlen])?;
                if bad(rc) {return Ok(rc);}
                if compare_strings(m,translation,s+360)?==0 {
                    let r=reserve_marks(m,p,s+68,s+72,mark_count as u32,tlen,h)?;if bad(r) {return Ok(r);}
                    let mi=m.word(s+68)?+mark_count as u32*4;
                    if mark_count!=0 {put_half(m,mi,half(m,mi.wrapping_sub(4))?)?;}
                    m.write(mi+2,&[1])?;mark_count=mark_count.wrapping_add(1);
                    delta=delta.wrapping_add(tlen as i16).wrapping_sub(2);
                    text_index=text_index.wrapping_add(1);
                    continue;
                }
                if byte(m,translation)?==0x15 && byte(m,translation.wrapping_add(1))?==0 {
                    let r=reserve_marks(m,p,s+68,s+72,mark_count as u32,tlen,h)?;if bad(r) {return Ok(r);}
                    let mi=m.word(s+68)?+mark_count as u32*4;
                    if default_lang!=9999 {
                        put_half(m,mi,default_lang as u16)?;
                        mark_count=mark_count.wrapping_add(1);
                    } else {
                        if mark_count!=0 {put_half(m,mi,half(m,mi.wrapping_sub(4))?)?;}
                        m.write(mi+2,&[1])?;mark_count=mark_count.wrapping_add(1);
                    }
                    delta=delta.wrapping_add(tlen as i16).wrapping_sub(2);
                    text_index=text_index.wrapping_add(1);
                    continue;
                }
                {
                    // state stays 9999 here (span runs before spelling:);
                    // defaultLanguage follows the 0x24 language markers.
                    let lang_index=half(m,s+64)?;
                    let same=lang_index==9999 && default_lang==9999;
                    if language_name==0 && same {
                        let r=reserve_marks(m,p,s+68,s+72,mark_count as u32,tlen,h)?;if bad(r) {return Ok(r);}
                        if tlen!=0 {
                            let tmarks=m.word(s+68)?;
                            for k in 0..tlen {
                                if byte(m,translation+k)?==b' ' {
                                    m.write(tmarks+mark_count as u32*4+k*4+2,&[1])?;
                                }
                            }
                            mark_count=mark_count.wrapping_add(tlen as u16);
                        }
                    } else if !same {
                        let r=reserve_marks(m,p,s+68,s+72,mark_count as u32,tlen,h)?;if bad(r) {return Ok(r);}
                        let v=if default_lang==9999 {lang_index} else {default_lang as u16};
                        let tmarks=m.word(s+68)?;
                        for k in 0..tlen {
                            put_half(m,tmarks+mark_count as u32*4+k*4,v)?;
                            if byte(m,translation+k)?==b' ' {
                                m.write(tmarks+mark_count as u32*4+k*4+2,&[1])?;
                            }
                        }
                        mark_count=mark_count.wrapping_add(tlen as u16);
                    }
                    // language_name set with a matching index writes no
                    // marks (check_write).
                }
                if bad(rc) {return Ok(rc);}
                delta=delta.wrapping_add(tlen as i16).wrapping_sub(2);
                text_index=text_index.wrapping_add(1);
                continue;
            }
            m.write(s+320,&[ch as u8,0])?;
            let mut language=2;
            if byte(m,p+64)?==1 && (ch.wrapping_sub(65)<=25 || ch.wrapping_sub(97)<=25) {
                rc=intern_pair(m,p,p+68,p+72,s+128,s+64,h)?;if bad(rc) {return Ok(rc);}
                language=1;
            }
            if (previous_language==1 && language==0) || (previous_language==0 && language==1) {
                literal(m,s+360,b" \0")?;
                rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
                rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
                m.write(m.word(s+68)?+mark_count as u32*4+2,&[1])?;mark_count=mark_count.wrapping_add(1);
            }
            previous_language=language;
            skip_space=false;
            rc=invoke(m,h,table,52,p+48,&[s+320,1])?;
            let mi=m.word(s+68)?+mark_count as u32*4;
            if ch==32 {
                let reserve=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(reserve) {return Ok(reserve);}
                if mark_count!=0 {put_half(m,mi,half(m,mi-4)?)?;}
                m.write(mi+2,&[1])?;mark_count=mark_count.wrapping_add(1);
            } else if half(m,s+64)?!=9999 {
                let reserve=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(reserve) {return Ok(reserve);}
                put_half(m,mi,half(m,s+64)?)?;mark_count=mark_count.wrapping_add(1);
                if bad(rc) {return Ok(rc);}
            } else {
                let reserve=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(reserve) {return Ok(reserve);}
                mark_count=mark_count.wrapping_add(1);
                if bad(rc) {return Ok(rc);}
            }
            delta=delta.wrapping_sub(1);
            text_index=text_index.wrapping_add(1);
        }
        while (record_index as u32)<count {
            let e=records+record_index as u32*24;
            let position=m.word(e+12)?.wrapping_add(m.word(s+20)?).wrapping_add(delta as i32 as u32) as i32;
            m.put_word(e+12,position.max(0) as u32)?;record_index=record_index.wrapping_add(1);
        }
        literal(m,s+360,b"\0")?;
        rc=invoke(m,h,table,52,p+48,&[s+360,1])?;if bad(rc) {return Ok(rc);}
        rc=reserve_marks(m,p,s+68,s+72,mark_count as u32,1,h)?;if bad(rc) {return Ok(rc);}
        rc=invoke(m,h,table,48,p+32,&[m.word(s+16)?])?;if bad(rc) {return Ok(rc);}
        let marks=m.word(s+68)?;
        let total=mark_count.wrapping_add(1);
        m.put_word(records+16,m.word(records+16)?.wrapping_add(delta as i32 as u32).wrapping_sub(trailing_nulls as u32*2))?;
        let mut start=0u16;
        while start<total {
            let mut boundary=start;
            while boundary<total && byte(m,marks+boundary as u32*4+2)?!=1 {boundary=boundary.wrapping_add(1);}
            let mut end=boundary.wrapping_add(1);
            while end<total && byte(m,marks+end as u32*4+2)?==1 {end=end.wrapping_add(1);}
            let mut scan=start;
            while scan<boundary && scan<total {
                if half(m,marks+scan as u32*4)?==0 {
                    for k in start..end {put_half(m,marks+k as u32*4,0)?;}
                    break;
                }
                scan=scan.wrapping_add(1);
            }
            start=end;
        }
        for i in 0..total {if half(m,marks+i as u32*4)?==9999 {put_half(m,marks+i as u32*4,1)?;}}
        let mut runs=0u16;let mut scan=0u16;
        while scan<total {
            let value=half(m,marks+scan as u32*4)?;
            if value==9999 {if scan==0 {runs=runs.wrapping_add(1);}scan=scan.wrapping_add(1);}
            else {scan=scan.wrapping_add(1);while scan<total && half(m,marks+scan as u32*4)?==value {scan=scan.wrapping_add(1);}runs=runs.wrapping_add(1);}
        }
        if runs!=0 {
            let new=h.call(m,0x1963e0,&[allocator(m,p)?,records,(runs as u32+count)*24+1])?;
            if new==0 {allocation_log(m,p,true,h)?;return Ok(0x8a60200a);}
            records=new;
            let mut original=0u16;let mut inserted=0u16;let mut last_text=0u32;
            let mut start=0u16;
            while start<total {
                let mut end=start.wrapping_add(1);
                while end<total && half(m,marks+start as u32*4)?==half(m,marks+end as u32*4)? {end=end.wrapping_add(1);}
                while (original as u32)<count && m.word(records+(original as u32+inserted as u32)*24+12)?<=m.word(records+12)?.wrapping_add(start as u32) {
                    let e=records+(original as u32+inserted as u32)*24;
                    if half(m,e)?==0x22 {last_text=e;} else if half(m,e)?==0x15 {last_text=0;}
                    original=original.wrapping_add(1);
                }
                if (original as u32)!=count {
                    let mut remaining=count.wrapping_sub(original as u32);
                    while remaining!=0 {
                        let dst=records+(original as u32+inserted as u32+remaining)*24;
                        copy_bytes(m,dst,dst-24,24)?;remaining-=1;
                    }
                }
                let e=records+(original as u32+inserted as u32)*24;
                put_half(m,e,0x24)?;
                let value=half(m,marks+start as u32*4)?;
                m.put_word(e+20,m.word(m.word(p+68)?+if value==9999 {12} else {value as u32*8+4})?)?;
                m.put_word(e+4,0)?;m.put_word(e+8,0)?;m.put_word(e+12,m.word(records+12)?.wrapping_add(start as u32))?;
                if last_text!=0 {
                    let mut pos=m.word(last_text+12)?;
                    while byte(m,marks+pos.wrapping_sub(m.word(records+12)?)*4+2)?!=0 && pos<m.word(records+12)?.wrapping_add(start as u32) {pos=pos.wrapping_add(1);}
                    m.put_word(last_text+12,pos)?;
                }
                m.put_word(e+16,0)?;inserted=inserted.wrapping_add(1);start=end;
            }
        }
        h.call(m,0x2409d4,&[records+24,(count+runs as u32).wrapping_sub(1),24,0x21a1fc])?;
        rc=invoke(m,h,table,52,p+56,&[records,(count+runs as u32)*24])?;if bad(rc) {return Ok(rc);}
        rc=invoke(m,h,table,48,p+40,&[m.word(s+32)?])?;if bad(rc) {return Ok(rc);}
        literal(m,s+380,b"clcpppipelinemode\0")?;m.put_word(s+400,0)?;
        rc=h.call(m,0x175a1c,&[m.word(m.word(p)?+20)?,s+380,s+400])?;
        literal(m,s+420,b"internal\0")?;
        let pipeline=m.word(s+400)?;
        if rc!=0 || pipeline==0 || compare_strings(m,pipeline,s+420)?!=0 {
            // src/fn_219790.c single-character synthetic modes: emit two
            // 0x15 records around the lone character, then update_offset.
            let mut cur_lang=0u32;
            if single {cur_lang=m.word(records+16)?;}
            if single && cur_lang!=0 {
                let heap=allocator(m,p)?;
                let tmp=h.call(m,0x1963b8,&[heap,1,24])?;
                if tmp==0 {allocation_log(m,p,true,h)?;return Ok(0x8a60200a);}
                m.put_word(tmp+20,spell_nm)?;
                put_half(m,tmp,0x15)?;
                {
                    let d=(leading as i32).wrapping_add(base as i32).wrapping_add(m.word(s+20)? as i32);
                    m.put_word(tmp+4,d as u32)?;
                    m.put_word(tmp+8,0)?;m.put_word(tmp+12,(d.max(0)) as u32)?;m.put_word(tmp+16,0)?;
                }
                rc=invoke(m,h,table,52,p+56,&[tmp,24])?;
                free(m,p,tmp,h)?;
                if bad(rc) {return Ok(rc);}
                let tmp2=h.call(m,0x1963b8,&[heap,1,24])?;
                if tmp2==0 {
                    allocation_log(m,p,true,h)?;
                    return Ok(0x8a60200a);
                }
                m.put_word(tmp2+20,normal_nm)?;
                put_half(m,tmp2,0x15)?;
                base=(trailing as i32).wrapping_add(m.word(s+20)? as i32).wrapping_add(base as i32).wrapping_add(delta as i32).wrapping_add((leading as i32).wrapping_mul(2)) as u32;
                {
                    let d2=(base as i32).wrapping_add(2);
                    m.put_word(tmp2+4,d2 as u32)?;
                    if d2<0 {base=0;} else {base=(base as i32).wrapping_add(2) as u32;}
                }
                m.put_word(tmp2+8,0)?;m.put_word(tmp2+12,base)?;m.put_word(tmp2+16,0)?;
                rc=invoke(m,h,table,52,p+56,&[tmp2,24])?;
                free(m,p,tmp2,h)?;
                if bad(rc) {return Ok(rc);}
            }
        }
        invoke(m,h,m.word(p+24)?,56,p+16,&[m.word(s+24)?.wrapping_add(delta as i32 as u32),0])
    })();
    let rc=result?;
    free(m,p,records,h)?;
    if m.word(s+68)?!=0 {free(m,p,m.word(s+68)?,h)?;}
    if count>1 {for i in 1..count {let value=m.word(owned+i*4)?;if value!=0 {free(m,p,value,h)?;}}}
    free(m,p,owned,h)?;
    let _=logger;
    Ok(rc)
}

/// fn_196860 / EU fn_196910 / KR fn_19694c.
///
/// The map iterator's step.  `it` is the ROM's three-word iterator: the map
/// object, the current node and the pending node.  The step publishes the
/// pending node's key and value and stores the node that follows it in the
/// tree's order.  A null iterator or a null output reports the ROM's
/// 0x89002007; an exhausted iterator zeroes both outputs and reports
/// 0x89002017.
///
/// The order of the stores is the ARM's: the current node, then the key
/// output, then the value output read back through the current node, then
/// the pending node.  An output that aliases the iterator therefore sees
/// the same bytes the ARM leaves behind.
pub fn fn_196860(m:&mut crate::boot::Memory<'_>,it:u32,outa:u32,outb:u32)->u32 {
    if it==0||outa==0||outb==0 {return 0x8900_2007;}
    let next=m.word(it+8);
    if next==0 {
        m.put_word(outa,0);
        m.put_word(outb,0);
        return 0x8900_2017;
    }
    m.put_word(it+4,next);
    let key=m.word(next+0x10);
    m.put_word(outa,key);
    let cur=m.word(it+4);
    let value=m.word(cur+0x14);
    m.put_word(outb,value);
    let base=m.word(it);
    let following=crate::frontend::successor_node(m.bytes,base.wrapping_add(0x18),cur);
    m.put_word(it+8,following);
    0
}

/// The negative control of fn_196860: the same step with the node's value
/// read from the key's word instead of the value's word.  The engine never
/// calls it; the certification harness runs it beside the port so the
/// comparison can be shown to reject a broken port.  The walk is the ROM's,
/// so the variant always terminates.
pub fn fn_196860_negative(m:&mut crate::boot::Memory<'_>,it:u32,outa:u32,outb:u32)->u32 {
    if it==0||outa==0||outb==0 {return 0x8900_2007;}
    let next=m.word(it+8);
    if next==0 {
        m.put_word(outa,0);
        m.put_word(outb,0);
        return 0x8900_2017;
    }
    m.put_word(it+4,next);
    let key=m.word(next+0x10);
    m.put_word(outa,key);
    let cur=m.word(it+4);
    let value=m.word(cur+0x10);
    m.put_word(outb,value);
    let base=m.word(it);
    let following=crate::frontend::successor_node(m.bytes,base.wrapping_add(0x18),cur);
    m.put_word(it+8,following);
    0
}

/// The live broker's database services, over the caller's arena.
///
/// `fn_1772b0` reaches the database through six services: the database lock
/// and its release, the map iterator's open, step and close, and the
/// case-insensitive key compare.  Every one of them is a real port over the
/// arena the caller owns: the map's own iterator ports publish the handle
/// and step it, the map's heap releases it, and the compare folds the two
/// keys.  Nothing is invented and no reply is fabricated.
pub struct LiveBroker<'a, H> {
    /// The engine services the map's ports reach.
    pub host: &'a mut H,
}

impl<H: crate::boot_part_b::Host + crate::boot_part_e::Host + crate::component_live_a2::Calls
    + crate::boot_part_f::Host> crate::boot_port_2c::BrokerQuery for LiveBroker<'_, H> {
    fn db_lock(&mut self, m: &mut crate::boot::Memory<'_>, lock: u32) -> u32 {
        crate::boot_port_1::fn_176f2c(m, lock, self.host)
    }
    fn db_unlock(&mut self, m: &mut crate::boot::Memory<'_>, lock: u32) -> u32 {
        crate::boot_port_1::fn_176f5c(m, lock, self.host)
    }
    fn db_open(
        &mut self,
        m: &mut crate::boot::Memory<'_>,
        database: u32,
        name: u32,
        mode: u32,
        out: u32,
    ) -> u32 {
        crate::component_live_a2::alloc_handle_for_key_m(m, database, name, mode, out, self.host)
    }
    fn db_next(&mut self, m: &mut crate::boot::Memory<'_>, iterator: u32, key: u32, value: u32) -> u32 {
        fn_196860(m, iterator, key, value)
    }
    fn db_close(&mut self, m: &mut crate::boot::Memory<'_>, iterator: u32) -> u32 {
        crate::component_live_a::release_owner_m(m, iterator, self.host)
    }
    fn db_compare(&mut self, m: &crate::boot::Memory<'_>, a: u32, b: u32) -> i32 {
        crate::fe_text::strcmp_ci_1769d0(m.bytes, a, b) as i32
    }
}

/// The broker's regional constants for `rom`, from the derived scenes.
/// US behavior is unchanged; EU/KR resolve their own data twins.
pub fn broker_for(rom: Rom) -> crate::boot_port_2c::BrokerGlobals {
    crate::boot_port_2c::BrokerGlobals {
        module: addr(rom, 0x8d7668),
        brokerstring_key: 0x177264,
        add_item_format: 0x177278,
        broker_label: 0x17729c,
        fullpath_key: 0x17823c,
        ctype_table: addr(rom, 0x8dc30c),
        parameters_desc: addr(rom, 0x8d7614),
        linguistic_desc: addr(rom, 0x8d773c),
    }
}
/// The US broker's constants. Regional callers use [`broker_for`].
pub fn us_broker() -> crate::boot_port_2c::BrokerGlobals {
    broker_for(Rom::Us)
}

/// The live broker's host: the engine's allocator, the broker database
/// query, the file interface and the logger, all over the caller's arena.
///
/// The scratch areas are the caller's, disjoint from the arena and from
/// every other area.  The host owns no storage of its own.
pub struct LiveBrokerHost {
    /// The broker query's 16 byte scratch.
    pub query: u32,
    /// The DataOpen globals' 24 byte scratch.
    pub data: u32,
    /// The file platform's 4608 byte scratch.
    pub files: u32,
    /// The lid lookup's 80 byte scratch.
    pub lid: u32,
    /// The broker unmap environment's 64 byte scratch.
    pub unmap: u32,
    /// The broker close's 64 byte scratch.
    pub close: u32,
    /// The block release the close's heap vtable recorded.  The port's heap
    /// vtable has no arena, so the caller applies the release.
    pub pending_release: Option<u32>,
    /// The ROM whose guest memory is live. The caller sets it via
    /// `broker_host(rom)`; every data literal in the broker path follows it.
    pub rom: Rom,
}
impl LiveBrokerHost {
    /// The engine services the broker's open reaches.
    fn dispatch(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        // Guest vtable slots carry the running region's twins (EU 0x4a7f30
        // for US 0x4a7758); US-first keeps every port literal identical
        // while twin slots land on the US arm below.
        let f = to_us(self.rom, f);
        match f {
            // The varargs hash of the logger has no guest effect.
            0xffff0001 => 0,
            0x196394 => crate::boot_part_e::alloc(m, a[0], a[1], self),
            0x1963b8 => crate::boot_part_e::calloc(m, a[0], a[1], a[2], self),
            0x196428 => {
                crate::boot_part_e::release(m, a[0], a[1], self);
                0
            }
            0x39f5e8 => self.locked_allocate(m, a[0], a[1], a[2], 0),
            0x39f5b8 => {
                let inner = m.word(a[0] + 4);
                self.lock(m, inner + 88);
                self.dispatch(m, 0x19b78c, &[inner, a[1]]);
                self.unlock(m, inner + 88);
                0
            }
            0x19b78c => {
                m.boot_heap_release(a[0] + 24, a[1]);
                let count = m.word(a[0] + 84).wrapping_sub(1);
                m.put_word(a[0] + 84, count);
                0
            }
            0x184f20 => {
                let args: [u32; 6] = core::array::from_fn(|i| a.get(i).copied().unwrap_or(0));
                m.boot_heap_allocate(args)
            }
            0x184f84 => {
                m.boot_heap_release(a[0], a[1]);
                0
            }
            0x163f20 => self.locked_allocate(m, a[0], a[1], a[2], a[3]),
            0x4a7758 => {
                let n = a[1].wrapping_mul(a[2]);
                let target = m.word(m.word(a[0]) + 8);
                let p = self.dispatch(m, target, &[a[0], n, 4, 4]);
                if p != 0 {
                    let at = m.offset(p);
                    m.bytes[at..at + n as usize].fill(0);
                }
                p
            }
            0x4a779c => {
                let target = m.word(m.word(a[0]) + 8);
                self.dispatch(m, target, &[a[0], a[1], 4])
            }
            0x4a774c => {
                let target = m.word(m.word(a[0]) + 12);
                self.dispatch(m, target, &[a[0], a[1]])
            }
            // The arbiter's thread identity.
            0x183edc => 0x1ff82000,
            0x172d18 => 0,
            0x1738e8 => {
                let varargs = a.get(5).copied().unwrap_or(0);
                crate::log_live::fn_1738e8(m, a[0], a[1], a[2], a[3], a[4], varargs, self)
            }
            0x173dc0 => 0,
            // The broker database query's own ports.
            0x196794 => {
                crate::component_live_a2::alloc_handle_for_key_m(m, a[0], a[1], a[2], a[3], self)
            }
            0x196860 => fn_196860(m, a[0], a[1], a[2]),
            0x196830 => crate::component_live_a::release_owner_m(m, a[0], self),
            0x176f2c | 0x176f5c => 0,
            // The broker's mapped key lookup and its unmap.
            0x17a1a4 => crate::leftovers_1::fn_17a1a4(
                m,
                a[0],
                a[1],
                a[2],
                a[3],
                a[4],
                self.unmap,
                &unmap_environment(self.unmap, self.rom),
                self,
            ),
            0x179208 => {
                crate::boot_part_e::fn_179208(m, a[0], a[1], a[2], &unmap_environment(self.unmap, self.rom), self)
            }
            0x179f3c => {
                let rom = self.rom;
                let rc = {
                    let mut host = CloseHost { inner: self };
                    crate::leftovers_1::fn_179f3c(m, a[0], a[1], &close_runtime(rom), &mut host)
                };
                // The port's heap vtable has no arena, so the block release
                // it recorded is applied here with the arena the ROM
                // released from.
                if let Some(object) = self.pending_release.take() {
                    let heap = m.word(m.word(m.word(a[0] + 4) + 8) + 4);
                    crate::boot_part_e::release(m, heap, object, self);
                }
                rc
            }
            0x1731c4 => crate::boot_part_d::fn_1731c4(m, a[0], a[1], a[2], a[3], &mut DataHost(self)),
            0x1730fc => crate::boot_part_d::fn_1730fc(m, a[0], &mut DataHost(self)),
            0x17ca94 => crate::boot_part_b::fn_17ca94(m, a[0], self),
            // The case-insensitive byte compare and the string helpers.
            0x1769d0 => crate::fe_text::strcmp_ci_1769d0(m.bytes, a[0], a[1]) as u32,
            0x195d90 => crate::fe_text::strlen_195d90(m.bytes, a[0]) as u32,
            0x195d58 => crate::normalization::strcmp_at(m.bytes, a[0], a[1]) as u32,
            0x17c8c0 => crate::fe_text::fold_17c8c0(a[0]),
            // The file interface vtable thunks.
            0x4a794c => crate::speech_part_3::fn_4a794c(m, a[0], a[1], a[2], a[3]),
            0x4a78f0 => crate::speech_part_3::fn_4a78f0(m, a[0], a[1], a[2], a[3]),
            0x4a7aac => crate::speech_part_3::fn_4a7aac(m, a[0], self),
            // The archive range path: the broker map, the archive stream's
            // seek and read, and the archive's own file reader.
            0x17a1a4 | 0x179208 | 0x17c9d8 | 0x179374 | 0x1b2760 | 0x1b254c | 0x1b2974
            | 0x172d30 | 0x172de0 => {
                crate::component_live_c::broker_range_live_m(m, f, a, RANGE_SCRATCH, self)
            }
            // The broker DB comparator (EU 0x1771a8 lands here via the
            // to_us normalization above; KR 0x177154 likewise).
            0x1770f8 => crate::speech_part_1::fn_1770f8(m, a[0], a[1]),
            _ => 0x8000_2001,
        }
    }

    /// The block heap's allocate, under the block lock.
    fn locked_allocate(
        &mut self,
        m: &mut crate::boot::Memory<'_>,
        object: u32,
        size: u32,
        align: u32,
        pool: u32,
    ) -> u32 {
        let inner = m.word(object + 4);
        self.lock(m, inner + 88);
        let r = m.boot_heap_allocate([inner, size, align, pool, 0, 0]);
        self.unlock(m, inner + 88);
        r
    }

    /// fn_1840fc: the block lock.
    fn lock(&mut self, m: &mut crate::boot::Memory<'_>, address: u32) {
        crate::boot_part_b::fn_1840fc(m, address, &mut LockHost);
    }

    /// fn_18423c: the block unlock.
    fn unlock(&mut self, m: &mut crate::boot::Memory<'_>, address: u32) {
        crate::boot_part_b::fn_18423c(m, address, &mut LockHost);
    }
}

/// The block lock's platform: the thread identity is the arbiter's.
struct LockHost;
impl crate::boot_part_b::Host for LockHost {
    fn call(&mut self, _m: &mut crate::boot::Memory<'_>, _f: u32, _a: &[u32]) -> u32 {
        0
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}

/// The US broker's unmap environment: the log category, the begin, end and
/// error format literals and the three error labels.
pub(crate) fn unmap_environment(scratch: u32, rom: Rom) -> crate::boot_part_e::UnmapEnvironment {
    crate::boot_part_e::UnmapEnvironment {
        category: addr(rom, 0x8d7668),
        begin_format: 0x179310,
        end_format: 0x17935c,
        error_format: 0x17934c,
        labels: [0x179344, 0x17933c, 0x1791d8],
        scratch,
    }
}

/// The US broker's close runtime: the log category, the begin and end format
/// literals, the two error keys and the case-folding table.
fn close_runtime(rom: Rom) -> crate::leftovers_1::CloseRuntime {
    crate::leftovers_1::CloseRuntime {
        category: addr(rom, 0x8d7668),
        begin: 0x17a14c,
        end: 0x17a18c,
        error: 0x17a180,
        handle_key: hash_text(b"hData"),
        status_key: hash_text(b"lhError"),
        scratch: CLOSE_SCRATCH,
        fold: ascii_fold_table(),
    }
}

/// The FNV-1a hash the broker's format keys use.
fn hash_text(bytes: &[u8]) -> u32 {
    bytes.iter().fold(2166136261u32, |h, &b| (h ^ b as u32).wrapping_mul(16777619))
}

/// The US character class table's fold bits: bit zero marks A-Z.
fn ascii_fold_table() -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < 26 {
        table[b'A' as usize + i] = 1;
        i += 1;
    }
    table
}

/// The broker close's host: the allocator, the tree lookup and the logger.
///
/// The port's heap vtable has no arena, so the block release is recorded by
/// `release_at` and applied by the caller with the arena the ROM released
/// from.
struct CloseHost<'a> {
    inner: &'a mut LiveBrokerHost,
}

impl crate::frontend::HeapVtable for CloseHost<'_> {
    fn alloc_at(&mut self, _obj: u32, _size: u32) -> u32 {
        0
    }
    fn realloc_at(&mut self, _obj: u32, _old: u32, _size: u32) -> u32 {
        0
    }
    fn release_at(&mut self, _obj: u32, object: u32) {
        self.inner.pending_release = Some(object);
    }
}

impl crate::frontend::MethodTable for CloseHost<'_> {
    fn call_at(&mut self, _obj: u32, _a: u32, _b: u32) -> u32 {
        0
    }
}

impl crate::frontend::StateLock for CloseHost<'_> {
    fn acquire(&mut self) -> u32 {
        0
    }
    fn release(&mut self) -> u32 {
        0
    }
}

impl crate::boot_part_b::FormatHost for CloseHost<'_> {}

impl crate::boot_part_b::Host for CloseHost<'_> {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.inner.dispatch(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}

impl crate::log_live::LogMethodsM for CloseHost<'_> {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        m: &mut crate::boot::Memory<'_>,
    ) -> u32 {
        self.inner
            .dispatch(m, method, &[object, category, level, arg, format, varargs])
    }
}

impl crate::leftovers_1::CloseHost for CloseHost<'_> {
    fn select_close_lock(&mut self, _lock: u32) {}
    /// The tree comparison needs the arena, which the port's signature does
    /// not carry.  It is reached only on the name-lookup path of a close
    /// whose data block is still live; reporting a fixed order fails closed
    /// there instead of matching a wrong key.
    fn compare_tree_keys(&mut self, _a: u32, _b: u32) -> i32 {
        1
    }
    fn indirect(&mut self, m: &mut crate::boot::Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.inner.dispatch(m, method, args)
    }
}

impl crate::speech_part_2::Host for LiveBrokerHost {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
}
impl crate::component_live_c::Host for LiveBrokerHost {
    fn call_m(&mut self, entry: u32, args: &[u32], memory: &mut crate::boot::Memory<'_>) -> u32 {
        self.dispatch(memory, entry, args)
    }
    fn thread_id_m(&mut self) -> u32 {
        0x1ff82000
    }
}
impl crate::boot_part_b::FormatHost for LiveBrokerHost {}
impl crate::reentrant::HooksM for LiveBrokerHost {
    fn call_m(&mut self, address: u32, args: &[u32], m: &mut crate::boot::Memory<'_>) -> u32 {
        self.dispatch(m, address, args)
    }
}

impl crate::boot_part_b::Host for LiveBrokerHost {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}
impl crate::boot_part_e::Host for LiveBrokerHost {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
}
impl crate::component_live_a2::Calls for LiveBrokerHost {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
}
impl crate::boot_part_f::Host for LiveBrokerHost {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
    fn compare(
        &mut self,
        m: &mut crate::boot::Memory<'_>,
        _f: u32,
        key: &mut crate::boot_part_f::Key,
        value: &mut u32,
    ) -> u32 {
        crate::boot_part_f::compare_parameter(m, *key, *value)
    }
}
impl crate::log_live::LogMethodsM for LiveBrokerHost {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        m: &mut crate::boot::Memory<'_>,
    ) -> u32 {
        self.dispatch(m, method, &[object, category, level, arg, format, varargs])
    }
}
impl crate::speech_part_3::Host for LiveBrokerHost {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
}
impl crate::boot_part_b::OutputHost for LiveBrokerHost {
    fn output(&mut self, _m: &mut crate::boot::Memory<'_>, _f: u32, _a: &[u32]) -> (u32, u32) {
        (0, 0)
    }
}
impl crate::boot_part_b::PropertyHost for LiveBrokerHost {
    fn property(&mut self, _m: &mut crate::boot::Memory<'_>, _p: u32, _k: &[u8]) -> (u32, u32) {
        (0, 0)
    }
    fn insert(&mut self, _m: &mut crate::boot::Memory<'_>, _p: u32, _k: u32, _e: [u32; 3]) -> u32 {
        0
    }
}
impl crate::boot_part_b::QueryHost for LiveBrokerHost {
    fn query_output(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        if f != 0x1772b0 {
            return (0, 0);
        }
        let scratch = self.query;
        let out = scratch + 12;
        let broker = a.first().copied().unwrap_or(0);
        let name = a.get(1).copied().unwrap_or(0);
        let mut query = LiveBroker { host: self };
        let rc = crate::boot_port_2c::fn_1772b0(m, broker, name, out, scratch, &mut query);
        (rc, m.word(out))
    }
    fn insert_record(
        &mut self,
        _m: &mut crate::boot::Memory<'_>,
        _db: u32,
        _k: u32,
        _record: [u32; 4],
    ) -> u32 {
        0x8150_6007
    }
    fn open_pair(
        &mut self,
        _m: &mut crate::boot::Memory<'_>,
        _f: u32,
        _owner: [u32; 2],
        _out: u32,
    ) -> u32 {
        0x8150_6007
    }
}
impl crate::boot_part_b::DataHost for LiveBrokerHost {
    fn data_output(&mut self, _m: &mut crate::boot::Memory<'_>, _f: u32, _a: &[u32], _out: u32) -> u32 {
        0x8060_2001
    }
}

/// The data host adapter the broker's file interface needs.
struct DataHost<'a>(&'a mut LiveBrokerHost);

impl crate::boot_part_d::Host for DataHost<'_> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut crate::boot::Memory<'_>) -> u32 {
        self.0.dispatch(m, f, a)
    }
    fn params_call_m(
        &mut self,
        _q: u32,
        _params: &mut [u32; 4],
        _p: u32,
        _out: &mut u32,
        _flags: u32,
        _m: &mut crate::boot::Memory<'_>,
    ) -> u32 {
        0x8060_2011
    }
}


/// The live broker's scratch areas, in the machine stack window.
///
/// The window is above the ROM's stack pointer and below every mapping the
/// recorded stages carry, so the areas are disjoint from the arena, from
/// the processor's own frame and from each other.  They are scratch: the
/// recorded comparison ignores the whole stack window.
const DATA_SCRATCH: u32 = 0x0e0f_0000;
const QUERY_SCRATCH: u32 = 0x0e0f_0020;
const LID_SCRATCH: u32 = 0x0e0f_0040;
const UNMAP_SCRATCH: u32 = 0x0e0f_00a0;
const CLOSE_SCRATCH: u32 = 0x0e0f_00e0;
const FILE_SCRATCH: u32 = 0x0e0f_0100;
/// The broker range path's 88 disjoint restored scratch bytes.
const RANGE_SCRATCH: u32 = 0x0e0f_0200;
/// The window the scratch areas live in.
const SCRATCH_WINDOW: (u32, u32) = (0x0e0f_0000, 0x0e10_0000);

/// The archive environment the file platform resolves paths with.
fn archive_environment(rom: Rom) -> crate::speech_part_1::ArchiveEnvironment {
    crate::speech_part_1::ArchiveEnvironment {
        dot: addr(rom, 0x8cd208),
        parent: addr(rom, 0x8cd20c),
        locale: 0,
    }
}

/// The ROM's "rb" file-mode literal.  It sits 0xf8 past the 0x173000
/// function entry in every region (EU 0x1731a8, KR 0x173154, both read
/// back as "rb"), so the port resolves it through the region table
/// instead of the US address the file open checks for 'r'.
pub(crate) fn rb_literal(rom: Rom) -> u32 {
    addr(rom, 0x173000) + 0xf8
}

/// The flat view of the caller's guest mappings.
///
/// The engine's ports address one arena from zero.  The mappings are staged
/// into the caller's workspace, the port runs over the flat bytes and the
/// bytes are written back.  A byte the port wrote outside the mappings is
/// an error, never a silent write; the scratch window is exempt.
fn flat_stage(
    m: &mut Memory<'_, '_>,
    workspace: &mut [u8],
    run: impl FnOnce(&mut crate::boot::Memory<'_>) -> u32,
) -> Result<u32, Error> {
    let required = m.regions.iter().map(|r| r.base as usize + r.bytes.len()).max().unwrap_or(0);
    if workspace.len() < required {
        return Err(Error::Capacity { required, available: workspace.len() });
    }
    let bytes = &mut workspace[..required];
    bytes.fill(0);
    for r in m.regions.iter() {
        bytes[r.base as usize..r.base as usize + r.bytes.len()].copy_from_slice(r.bytes);
    }
    let mut flat = crate::boot::Memory { base: 0, bytes };
    let rc = run(&mut flat);
    let window = (SCRATCH_WINDOW.0 as usize, SCRATCH_WINDOW.1 as usize);
    let mut end = 0usize;
    loop {
        let next = m
            .regions
            .iter()
            .filter(|r| r.bytes.len() != 0 && r.base as usize >= end)
            .min_by_key(|r| r.base);
        let Some(region) = next else { break };
        if let Some(at) = first_dirty(&flat.bytes, end, region.base as usize, window) {
            return Err(Error::UnmappedMemory { address: at as u32, bytes: 1 });
        }
        end = region.base as usize + region.bytes.len();
    }
    for r in m.regions.iter_mut() {
        r.bytes.copy_from_slice(&flat.bytes[r.base as usize..r.base as usize + r.bytes.len()]);
    }
    Ok(rc)
}

/// The first byte the port wrote in `[from,to)` that is not scratch.
fn first_dirty(bytes: &[u8], from: usize, to: usize, window: (usize, usize)) -> Option<usize> {
    let mut at = from;
    while at < to {
        if window.0 > at && window.0 < to {
            at = window.0;
        }
        if at >= window.0 && at < window.1 {
            at = window.1.min(to);
            continue;
        }
        let end = to.min(window.0).max(at);
        if end <= at {
            break;
        }
        if let Some(k) = bytes[at..end].iter().position(|b| *b != 0) {
            return Some(at + k);
        }
        at = end;
    }
    None
}

fn lid_query(m:&mut Memory<'_, '_>,a:&[u32],h:&mut impl InternalCalls)->Result<u32,Error> {
    let p=a[0];
    if bad(crate::normalization::quad_check(p,a[1],0xf37f,0x2a4)) {return Ok(0x89802008);}
    m.write(a[4],&[0])?;
    let env=m.word(p)?;let logger=m.word(env+16)?;
    if a[2]==0 {h.call(m,0x173dc0,&[logger,addr(h.rom(), 0x8f8f44),0xa029,0])?;return Ok(0x89802007);}
    let s=*a.get(5).ok_or(Error::UnsupportedStage{entry:0x1ecf68})?;
    literal(m,s+16,b"Entering fe_dctlkp_QueryLidDictExists : %s?\0")?;
    h.call(m,0x1738e8,&[logger,addr(h.rom(), 0x8f8f44),4,0,s+16,a[2]])?;
    let broker=m.word(env+12)?;
    let rc=if a[3]!=0 {
        literal(m,s+8,b"lid\0")?;
        h.call(m,0x1795f0,&[broker,s+8,1,s])?
    } else {h.call(m,0x179f30,&[broker,p+0x9e,s])?};
    if bad(rc) {return Ok(if a[3]!=0 {0} else {rc});}
    let handle=m.word(s)?;
    let rc=h.call(m,0x26b718,&[p+if a[3]!=0 {0x48} else {8},a[2],m.word(p+if a[3]!=0 {0x44} else {4})?,handle,a[4]])?;
    if bad(rc) {return Ok(rc);}
    h.call(m,0x179f3c,&[broker,handle])?;
    Ok(rc)
}

/// ABI bridge up to the first missing resource open. The typed pronunciation
/// lookup owns a borrowed arena across error cleanup; invoking it with an
/// invented host error would incorrectly run cleanup past the missing call.
/// This branch publishes only the ARM prefix, then calls the real dependency.
pub(crate) fn lookup_dictionary_prefix(m:&mut Memory<'_, '_>,a:&[u32],h:&mut impl InternalCalls)->Result<u32,Error> {
    let p=a[0];
    if bad(crate::normalization::quad_check(p,a[1],0xf37f,0x2a4)) {return Ok(0x89802008);}
    let env=m.word(p)?;
    let scratch=*a.get(7).ok_or(Error::UnsupportedStage{entry:0x1ea4e0})?;
    literal(m,scratch,b"Entering fe_dctlkp_Lookup: searching %s in %s\0")?;
    h.call(m,0x1738e8,&[m.word(env+16)?,addr(h.rom(), 0x8f8f44),5,0,scratch,a[3],a[2]])?;
    if a[2]==0 || a[3]==0 || a[4]==0 || a[5]==0 || a[6]==0 {
        return Err(Error::UnsupportedStage{entry:0x1ea4e0});
    }
    if half(m,a[5])?==1 {return Err(Error::UnsupportedStage{entry:0x1ea4e0});}
    if half(m,a[5])?==0xffff {put_half(m,a[5],0)?;}
    let n=half(m,p+0x88)? as u32;
    for i in 0..n {
        if compare_strings(m,a[2],m.word(m.word(p+0x68)?+i*4)?)?==0 {
            m.put_word(p+0x14,m.word(m.word(p+0x6c)?+i*4)?)?;
            return lookup_cached_prefix(m,a,h);
        }
    }
    let resources=h.call(m,0x1963e0,&[m.word(env+4)?,m.word(p+0x78)?,(n+2)*4])?;
    if resources==0 {return Err(Error::UnsupportedStage{entry:0x1ea4e0});}
    m.put_word(p+0x78,resources)?;
    m.put_word(resources+n*4,0)?;m.put_word(resources+(n+1)*4,0)?;
    h.call(m,0x179f30,&[m.word(env+12)?,p+0x9e,resources+n*4])?;
    Err(Error::UnsupportedStage{entry:0x1ea4e0})
}

fn lookup_cached_prefix(m:&mut Memory<'_, '_>,a:&[u32],h:&mut impl InternalCalls)->Result<u32,Error> {
    let p=a[0];let state=p+8;let tables=m.word(state+12)?;let header=m.word(tables)?;
    let mut raw=[0u8;10];m.read(header,&mut raw)?;
    let separator=crate::morphology::DictionaryHeader{bytes:&raw}.separator().map_err(|_|Error::InvalidSnapshot)?;
    m.write(a[6],&[separator])?;
    let required=string_len(m,a[3],false)?.wrapping_add(5) as u16;
    if (half(m,p+0x8e)? as i16)<(required as i16) {
        let query=h.call(m,0x1963e0,&[m.word(m.word(p)?+4)?,m.word(p+0x74)?,required as u32])?;
        if query==0 {return Err(Error::UnsupportedStage{entry:0x1ea4e0});}
        put_half(m,p+0x8e,required)?;m.put_word(p+0x74,query)?;
    }
    put_half(m,a[5],0)?;m.put_word(a[4],m.word(p+0x70)?)?;
    let query=m.word(p+0x74)?;
    string_copy(m,query,a[3],false)?;
    let length=string_len(m,query,false)?;
    if length==0 || (length as i16)>(half(m,header+0x16)? as i16) {
        return Err(Error::UnsupportedStage{entry:0x26bb6c});
    }
    for i in 0..length {
        let c=byte(m,query+i)?;
        if c<byte(m,header+4)? || c>byte(m,header+5)? {
            return Err(Error::UnsupportedStage{entry:0x26bb6c});
        }
    }
    let wanted=byte(m,m.word(tables+4)?.wrapping_add((byte(m,query)? as u32).wrapping_sub(byte(m,header+4)? as u32)))?;
    let count=half(m,header+0x2e)? as u32;
    let chars=m.word(tables+12)?;
    let mut index=0;
    while index<count && byte(m,chars+index)?<wanted {index+=1;}
    if index==count || byte(m,chars+index)?!=wanted {return Err(Error::UnsupportedStage{entry:0x26c1c0});}
    let mut position=m.word(m.word(tables+16)?+index*4)?;
    if position<=byte(m,header+0x12)? as u32 {
        m.put_word(state,m.word(header+0x38)?)?;
        m.write(state+4,&[byte(m,header+0x11)?])?;
        position=m.word(state)?.wrapping_mul(8).wrapping_add(byte(m,state+4)? as u32);
    }
    // The map read needs one disjoint word plus the unmap environment's 64
    // bytes. Both live past the caller's literal string in its own scratch.
    let map_scratch=a.get(7).copied().ok_or(Error::UnsupportedStage{entry:0x17a1a4})?
        .wrapping_add(64);
    // The ROM's map read clears the caller's buffer before it maps the
    // archive range. The mapping service is the next dependency.
    m.put_word(state+8,0)?;
    h.call(m,0x17a1a4,&[m.word(state+16)?,m.word(state+20)?,m.word(tables+0x40)?.wrapping_add(position>>3),half(m,header+0x32)? as u32,state+8,map_scratch])?;
    Err(Error::UnsupportedStage{entry:0x26c1c0})
}


/// The live file boundary: the ROM's own file code at 0x132ab8, and the
/// file platform the broker's DataOpen reaches through it.
///
/// The ARM emulator's file layer serves one real cartridge file per resource
/// slot. The slot's UTF-16 name is reduced to its base name, the file's bytes
/// come from the cartridge, the buffer comes from the ROM's block heap
/// (fn_163f20), and the slot records the buffer, the file's size and the
/// loaded state. A name the cartridge does not have fails with 0x80000014,
/// the ROM's own "no such file" status.
///
/// The core never reads a filesystem and never synthesises a file: the caller
/// supplies the real cartridge bytes. One boundary serves the boot lanes that
/// load the TTS library's modules and the broker's DataOpen that reads them
/// back.
pub mod live_fs {
    use crate::boot::Memory;

    /// The real cartridge files behind the ROM's resource names.
    pub trait GameFiles {
        /// The bytes of one real cartridge file, named by the base name the
        /// ROM's resource slot carries. `None` when the cartridge has none.
        fn file(&mut self, name: &[u8]) -> Option<&[u8]>;
    }

    /// The ROM's block heap. The file layer allocates its buffer here.
    pub trait BlockHeap {
        /// fn_163f20: allocate `length` bytes for `allocator` and return the
        /// guest pointer, or zero when the allocation fails.
        fn allocate(&mut self, m: &mut Memory<'_>, a: [u32; 4]) -> u32;
    }

    /// The ROM's own "no such file" status.
    pub const NO_FILE: u32 = 0x8000_0014;
    /// The ROM's own "no memory" status.
    pub const NO_MEMORY: u32 = 0x8060_200a;
    /// The loaded state the ROM's file layer writes into the resource slot.
    pub const LOADED: u8 = 2;
    /// The units the ARM's file layer reads from the slot's name.
    pub const NAME_UNITS: u32 = 128;

    fn unit(m: &Memory<'_>, address: u32) -> u16 {
        let at = m.offset(address);
        u16::from_le_bytes([m.bytes[at], m.bytes[at + 1]])
    }

    fn put(m: &mut Memory<'_>, address: u32, value: u8) {
        let at = m.offset(address);
        m.bytes[at] = value;
    }

    /// The ROM's file layer at 0x132ab8, over the real cartridge files.
    pub struct LiveFileService<'a, F> {
        /// The real cartridge files.
        pub files: &'a mut F,
    }

    impl<'a, F: GameFiles> LiveFileService<'a, F> {
        /// The service over the caller's real cartridge files.
        pub fn new(files: &'a mut F) -> Self {
            Self { files }
        }

        /// The ROM's resource name: the slot's UTF-16 name reduced to its
        /// base name, the part after the last '/' or '\'. The ARM's file
        /// layer reads 128 units. Returns the length written to `name`.
        pub fn slot_name(m: &Memory<'_>, slot: u32, name: &mut [u8]) -> u32 {
            let mut units = 0u32;
            while units < NAME_UNITS {
                if unit(m, slot + units * 2) == 0 {
                    break;
                }
                units += 1;
            }
            let mut start = 0u32;
            for i in 0..units {
                let c = (unit(m, slot + i * 2) & 0xff) as u8;
                if c == b'/' || c == b'\\' {
                    start = i + 1;
                }
            }
            let mut len = 0u32;
            for i in start..units {
                if len as usize >= name.len() {
                    break;
                }
                name[len as usize] = (unit(m, slot + i * 2) & 0xff) as u8;
                len += 1;
            }
            len
        }

        /// fn_132ab8's boundary: serve the slot's real file.
        ///
        /// `name` is a caller-owned buffer of at least 128 bytes for the
        /// resource's base name. `heap` is the ROM's block heap. On success
        /// the slot records the buffer, the file's size and the loaded state
        /// and the status is zero. A name the cartridge does not have fails
        /// with NO_FILE and leaves the slot alone.
        pub fn serve_slot(
            &mut self,
            m: &mut Memory<'_>,
            slot: u32,
            name: &mut [u8],
            heap: &mut impl BlockHeap,
        ) -> u32 {
            let len = Self::slot_name(m, slot, name) as usize;
            if len > NAME_UNITS as usize {
                return NO_FILE;
            }
            let negative = m.byte(slot + 0xe6) != 0;
            let allocator = m.word(slot + if negative { 0xdc } else { 0xd8 });
            let arg = m.word(slot + 0xe0);
            let arg = if negative { arg.wrapping_neg() } else { arg };
            let mode = m.byte(slot + 0xe5) as u32;
            let mut key = [0u8; NAME_UNITS as usize];
            key[..len].copy_from_slice(&name[..len]);
            let Some(data) = self.files.file(&key[..len]) else {
                return NO_FILE;
            };
            let length = data.len() as u32;
            let buffer = heap.allocate(m, [allocator, length, arg, mode]);
            if buffer == 0 {
                return NO_MEMORY;
            }
            let at = m.offset(buffer);
            m.bytes[at..at + data.len()].copy_from_slice(data);
            m.put_word(slot + 0xcc, length);
            m.put_word(slot + 0xc0, buffer);
            put(m, slot + 0xe4, LOADED);
            0
        }
    }

    /// The TTS library's file platform: the source object's own file code.
    ///
    /// The source object names an archive and an allocator. Its `open` slot
    /// is the ROM's fn_4a77ac: it normalises the path, appends ".dat" and
    /// looks the entry up in the archive. The archive is the one the live
    /// file service filled with the real cartridge files. A path the archive
    /// does not have fails with the ROM's NO_FILE.
    pub struct FilePlatform {
        /// The archive environment: the dot, parent and locale literals the
        /// archive's path resolution needs.
        pub env: crate::speech_part_1::ArchiveEnvironment,
        /// The caller's scratch: 4096 bytes, disjoint from the archive, the
        /// handle object and every nested allocation.
        pub scratch: u32,
        /// The guest "rb" mode string the ROM passes to the open slot.
        pub rb: u32,
    }

    impl FilePlatform {
        /// fn_4a77ac: the source object's open slot.
        ///
        /// `src` is the source object, `path` the requested name and `out`
        /// the caller's handle word. The ROM allocates the file object
        /// before it checks the name, so the handle is written first.
        pub fn open_source(
            &mut self,
            m: &mut Memory<'_>,
            src: u32,
            path: u32,
            out: u32,
            h: &mut impl crate::boot_part_b::DataHost,
        ) -> u32 {
            let allocator = m.word(src + 4);
            let archive = m.word(src + 0xc);
            let allocate = m.word(m.word(allocator) + 8);
            let file = h.call(m, allocate, &[allocator, 16, 4]);
            m.put_word(out, file);
            if file == 0 {
                return NO_MEMORY;
            }
            m.put_word(file + 12, allocator);
            if path == 0 || self.rb == 0 || archive == 0 {
                return 0x8000_0007;
            }
            if m.byte(self.rb) != b'r' || crate::speech_part_4::fn_1012cc(m, path, 46) != 0 {
                return 0x8000_0000;
            }
            let mut i = 0u32;
            while m.byte(path + i) != 0 {
                let c = m.byte(path + i);
                put(m, self.scratch + i, if c == b'/' { b'_' } else { c });
                i += 1;
            }
            put(m, self.scratch + i, 0);
            let suffix = self.scratch + 256;
            let at = m.offset(suffix);
            m.bytes[at..at + 5].copy_from_slice(b".dat\0");
            let n = crate::speech_part_4::fn_302edc(m, self.scratch);
            let count = if n.wrapping_add(4) >= 256 { 255u32.wrapping_sub(n) } else { 4 };
            crate::speech_part_4::fn_100c22(m, self.scratch, suffix, count);
            put(m, self.scratch + 255, 0);
            let data = crate::speech_part_1::fn_16b554(
                m,
                archive + 8,
                0,
                self.scratch,
                file + 8,
                self.scratch + 512,
                &self.env,
            );
            m.put_word(file, data);
            if data != 0 {
                m.put_word(file + 4, data);
                return 0;
            }
            let release = m.word(m.word(allocator) + 12);
            h.call(m, release, &[allocator, file]);
            NO_FILE
        }
    }
}

/// The broker's DataOpen with the live file platform.
///
/// `h` owns the engine's allocator, the broker database query, the property
/// service and the logging. This binding owns the file platform: the source
/// object's own file code, over the archive the live file service filled with
/// the real cartridge files.
pub fn fn_1795f0_live(
    m: &mut crate::boot::Memory<'_>,
    broker: u32,
    name: u32,
    flags: u32,
    out: u32,
    globals: &crate::boot_part_b::DataOpenGlobals,
    platform: &mut live_fs::FilePlatform,
    h: &mut impl crate::boot_part_b::DataHost,
) -> u32 {
    let mut host = DataOpenHost { inner: h, platform };
    crate::boot_part_b::fn_1795f0(m, broker, name, flags, out, globals, &mut host)
}
struct DataOpenHost<'a, H> {
    inner: &'a mut H,
    platform: &'a mut live_fs::FilePlatform,
}
impl<'a, H: crate::boot_part_b::DataHost> crate::boot_part_b::Host for DataOpenHost<'a, H> {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.inner.call(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        self.inner.thread_id()
    }
}
impl<'a, H: crate::boot_part_b::DataHost> crate::boot_part_b::OutputHost for DataOpenHost<'a, H> {
    fn output(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        self.inner.output(m, f, a)
    }
}
impl<'a, H: crate::boot_part_b::DataHost> crate::boot_part_b::PropertyHost for DataOpenHost<'a, H> {
    fn property(&mut self, m: &mut crate::boot::Memory<'_>, p: u32, k: &[u8]) -> (u32, u32) {
        self.inner.property(m, p, k)
    }
    fn insert(&mut self, m: &mut crate::boot::Memory<'_>, p: u32, k: u32, e: [u32; 3]) -> u32 {
        self.inner.insert(m, p, k, e)
    }
}
impl<'a, H: crate::boot_part_b::DataHost> crate::boot_part_b::QueryHost for DataOpenHost<'a, H> {
    fn query_output(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        self.inner.query_output(m, f, a)
    }
    fn insert_record(&mut self, m: &mut crate::boot::Memory<'_>, db: u32, k: u32, r: [u32; 4]) -> u32 {
        self.inner.insert_record(m, db, k, r)
    }
    fn open_pair(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, o: [u32; 2], out: u32) -> u32 {
        self.inner.open_pair(m, f, o, out)
    }
}
impl<'a, H: crate::boot_part_b::DataHost> crate::boot_part_b::DataHost for DataOpenHost<'a, H> {
    fn data_output(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32], out: u32) -> u32 {
        if f == 0x173000 {
            let src = a[0];
            let ctx = a[1];
            let path = a[2];
            if src == 0 || ctx == 0 || out == 0 || path == 0 {
                return 0x8060_2007;
            }
            let p = self.inner.call(m, 0x1963b8, &[ctx, 1, 32]);
            m.put_word(out, p);
            if p == 0 {
                return 0x8060_200a;
            }
            m.put_word(p, src);
            m.put_word(p + 4, ctx);
            let table = m.word(src + 8);
            let get = m.word(table + 40);
            let mut rc = if get == 0 {
                0x8060_2001
            } else {
                self.inner.call(
                    m,
                    get,
                    &[m.word(src + 0xc), m.word(src + 4), path, p + 12],
                )
            };
            if rc & 0x1fff == 1 {
                rc = self.platform.open_source(m, src, path, p + 16, self.inner);
            }
            if rc >= 0x8000_0000 {
                self.inner.call(m, 0x196428, &[ctx, p]);
                m.put_word(out, 0);
            }
            return rc;
        }
        self.inner.data_output(m, f, a, out)
    }
}
