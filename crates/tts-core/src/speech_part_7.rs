

use crate::selection::{arena::{Arena, Host}, arena_mid as mid, arena_mid, arena_leaf};
const ROM_SENTINEL:u32=u32::MAX;
const ROM: u32 = u32::MAX;
fn at(p:u32,i:i32)->u32 {p.wrapping_add((i as u32).wrapping_mul(4))}
fn get(m:&Arena<'_>,p:u32)->i32 {m.u32(p) as i32}
fn cell(m:&Arena<'_>,table:u32,row:i32,col:i32)->u32 {at(m.u32(at(table,row)),col)}
fn call(m:&mut Arena<'_>,h:&mut impl Host,f:u32,a:&[u32])->u32 {h.call(m,f,a).0}
fn free(m:&mut Arena<'_>,h:&mut impl Host,owner:u32,p:u32) {call(m,h,0x196428,&[m.u32(owner+4),p]);}
fn local(m:&mut Arena<'_>,p:u32,body:impl FnOnce(&mut Arena<'_>)->u32)->u32 {
    let saved=[m.u32(p),m.u32(p+4),m.u32(p+8)];
    m.set_u32(p+4,0);m.set_u32(p+8,0);
    let rc=body(m);
    for (i,v) in saved.iter().enumerate(){m.set_u32(p+i as u32*4,*v);}
    rc
}

pub fn fn_2a8414(m:&mut Arena<'_>,h:&mut impl Host,ctx:u32,voice:u32,settings:u32,params:u32,weights:&[i32;8])->u32 {
    if m.u32(ctx+0x454)==0 {return 0;}
    local(m,params,|m| {
        let owner=m.u32(ctx+0x18);let models=ctx+0x440;let filter=ctx+0x498;
        let workspace=ctx+0x3d4;let result=ctx+0x4e0;let selection=ctx+0x26c;
        let mode3=m.u8(settings+0x113)==3;
        let mut overlap=0i32;let mut padding=0;
        m.set_u32(selection+0x18,m.u32(voice+0x204));
        let bank=m.u32(selection+0x28);
        let dimensions=get(m,selection+0x18);let streams=get(m,bank+0x18);
        let q=crate::runtime::signed_division(dimensions,streams).expect("zero stream divisor").quotient;
        m.set_u32(selection+0x1c,q.wrapping_sub(1) as u32);
        m.set_u32(filter+16,0);m.set_u32(filter+0x18,streams as u32);
        m.set_u32(filter+0x1c,0);m.set_u32(filter+0x28,bank);
        let mut frames=get(m,models+20);
        if m.u32(models+4)!=m.u32(ctx+0x460) {overlap=get(m,settings+0x50).wrapping_mul(2).min(frames>>1);}
        if m.u32(models)!=m.u32(ctx+0x45c) {frames=frames.wrapping_add(get(m,ctx+0x3a90)).wrapping_add(get(m,ctx+0x3a8c));}
        m.set_u32(result+16,0);
        m.set_u32(result+0x20,get(m,models+20).wrapping_add(get(m,ctx+0x3a90)).wrapping_sub(overlap>>1) as u32);
        m.set_u32(filter+0x20,frames as u32);
        m.set_u32(result+0x1c,m.u32(settings+0x3c).wrapping_sub(1));
        m.set_u32(result+0x28,ctx+0x528);m.set_u32(result+0x18,0);
        let mut rc;
        if !mode3 {
            m.set_u32(filter+0x34,0);m.set_u32(filter+0x30,0);m.set_u32(workspace+0x50,0);
            m.set_u32(filter+0x18,get(m,filter+0x18).min(2) as u32);padding=2;
            rc=call(m,h,0x2a76a4,&[m.u32(owner+4),m.u32(filter+0x18),frames as u32,padding,workspace+0x50]);
            if rc==0 {m.set_u32(workspace+0x3c,m.u32(filter+0x18));m.set_u32(workspace+0x4c,0);m.set_u32(workspace+0x38,frames as u32);}
        } else {rc=mid::prepare_gain_tables(m,h,owner,frames,get(m,bank+0x30).wrapping_mul(2).wrapping_add(1),get(m,filter+0x18),1,workspace);}
        'work: {
            if rc>=0x80000000 {break 'work;}
            rc=mid::prepare_filter_rows(m,h,owner,workspace,result,ROM);if rc>=0x80000000 {break 'work;}
            rc=mid::prepare_filter_rows(m,h,owner,workspace,filter,ROM);if rc>=0x80000000 {break 'work;}
            let mut model=m.u32(models);
            while model!=m.u32(models+4) {
                let p=call(m,h,0x2db1d0,&[ctx,m.u32(voice+0x1f8).wrapping_shl(1)]);
                m.set_u32(model+8,p);
                if p==0 {rc=0x84e0200a;break 'work;}
                m.set_u32(model+8,p.wrapping_sub(4));
                for index in 2..get(m,voice+0x1f8).wrapping_add(2) {
                    let value=call(m,h,0x2d9dc0,&[ctx+0x3c,m.u16(model+4) as i16 as i32 as u32,voice+0x440,0,(index-2) as u32]);
                    m.set_u16(m.u32(model+8).wrapping_add(index as u32*2),value as u16);
                    if get(m,settings+0x2c)>20 {call(m,h,0x1738e8,&[m.u32(owner+16),ROM,4,0,ROM,value]);}
                }
                model=m.u32(model+24);
            }
            let p=call(m,h,0x1963b8,&[m.u32(owner+4),m.u32(filter+0x18),4]);m.set_u32(params+4,p);
            if p==0 {rc=0x84e0200a;break 'work;}
            let p=call(m,h,0x1963b8,&[m.u32(owner+4),m.u32(filter+0x18),4]);m.set_u32(params+8,p);
            if p==0 {rc=0x84e0200a;break 'work;}
            for stream in 0..get(m,settings+0x3c) {
                let mut frame=0;
                if m.u32(models)!=m.u32(ctx+0x45c) {
                    while frame<get(m,ctx+0x3a90).wrapping_add(get(m,ctx+0x3a8c)) {
                        for j in 0..get(m,filter+0x18) {
                            let hist=ctx+0x56c+((get(m,ctx+0x3a90)+get(m,ctx+0x3a8c)-frame-1) as u32)*600;
                            let dst=cell(m,m.u32(filter+0x30),j,frame);m.set_u32(dst,m.u32(at(hist,stream*3+j)));
                            if mode3 {let dst=cell(m,m.u32(filter+0x34),j,frame);m.set_u32(dst,m.u32(at(hist+300,stream*3+j)));}
                        }
                        frame+=1;
                    }
                }
                let mut model=m.u32(models);
                while model!=m.u32(models+4) {
                    for index in 2..get(m,voice+0x1f8).wrapping_add(2) {
                        let id=m.u16(m.u32(model+8).wrapping_add(index as u32*2)) as u32;
                        rc=if !mode3 {call(m,h,0x2da8c8,&[m.u32(ctx+0x20),index as u32,id,stream as u32,m.u32(filter+0x18),params,voice+0x1e0])}
                        else {crate::selection::arena::read_voice_pair(m,h,m.u32(ctx+0x20),index as u32,id,stream as u32,params,voice+0x1e0)};
                        if rc>=0x80000000 {break 'work;}
                        for _ in 0..m.u16(m.u32(model+12)+index as u32*2) {
                            for j in 0..get(m,filter+0x18) {
                                let dst=cell(m,m.u32(filter+0x30),j,frame);let value=get(m,at(m.u32(params+4),j));
                                m.set_u32(dst,value.clamp(-0x7d00000,0x7d00000) as u32);
                                if mode3 {let dst=cell(m,m.u32(filter+0x34),j,frame);m.set_u32(dst,get(m,at(m.u32(params+8),j)).min(0x7d00000) as u32);}
                            }
                            frame+=1;
                        }
                    }
                    model=m.u32(model+24);
                }
                for i in 0..overlap {for j in 0..get(m,filter+0x18) {
                    let hist=ctx+0x56c+i as u32*600;
                    m.set_u32(at(hist,stream*3+j),m.u32(cell(m,m.u32(filter+0x30),j,frame-i-1)));
                    if mode3 {m.set_u32(at(hist+300,stream*3+j),m.u32(cell(m,m.u32(filter+0x34),j,frame-i-1)));}
                }}
                if mode3 {
                    if m.u32(voice+0x488)==0 {call(m,h,0x2a24f8,&[owner,filter]);}
                    else if stream<3 {mid::run_forward_stream(m,h,owner,filter);}
                    else {call(m,h,0x2daf68,&[filter,stream as u32,m.u32(ctx+0x3aa0),voice+0x484]);}
                } else {call(m,h,0x2a3b18,&[owner,filter]);}
                let mut i=0;let mut frame=if m.u32(models)!=m.u32(ctx+0x45c) {get(m,ctx+0x3a8c)} else {0};
                while i<get(m,ctx+0x3a90).min(4) {
                    let v=weights[i as usize+4].wrapping_mul(get(m,at(ctx+0x344c,stream*10+i)))
                        .wrapping_add(get(m,at(m.u32(filter+0x2c),frame)).wrapping_mul(weights[i as usize])).wrapping_add(2);
                    m.set_u32(cell(m,m.u32(result+0x2c),i,stream),(v/5) as u32);i+=1;frame+=1;
                }
                while i<get(m,result+0x20) {m.set_u32(cell(m,m.u32(result+0x2c),i,stream),m.u32(at(m.u32(filter+0x2c),frame)));i+=1;frame+=1;}
                for i in 0..(overlap>>1).min(4) {m.set_u32(at(ctx+0x344c,stream*10+i),m.u32(at(m.u32(filter+0x2c),frame)));frame+=1;}
            }
            m.set_u32(ctx+0x3a8c,overlap.wrapping_sub(overlap>>1) as u32);m.set_u32(ctx+0x3a90,(overlap>>1) as u32);
        }
        for p in [m.u32(params+4),m.u32(params+8)] {if p!=0 {free(m,h,owner,p);}}
        if !mode3 {
            let values=m.u32(workspace+0x50);
            if values!=0 {free(m,h,owner,m.u32(values).wrapping_sub(padding*4));free(m,h,owner,values);m.set_u32(workspace+0x50,0);}
            m.set_u32(filter+0x30,0);
        }
        mid::release_selector(m,h,workspace);mid::release_filter(m,h,filter);rc
    })
}

pub fn fn_2a6718(m:&mut Arena<'_>,h:&mut impl Host,s:u32,model:u32,cfg:u32,done:u32,params:u32,current:&[i32;7],previous:&[i32;7])->u32 {
    local(m,params,|m|synthesize_phones_body(m,h,s,model,cfg,done,params,current,previous))
}

pub fn fn_2a7740(m:&mut Arena<'_>,h:&mut impl Host,ctx:u32,config:u32,options:u32,output:u32,weights:&[i32;14])->u32 {
    if m.u8(options+0x46)==0 {return 0;}
    local(m,output,|m| {
        let owner=m.u32(ctx+0x18);let models=ctx+0x440;let workspace=ctx+0x3d4;let p=ctx+0x344;
        let paired=m.u8(options+0x112)==3;let mode=m.u8(options+0x112);
        let mut count=0i32;let mut padding=0;let mut remaining=0i32;
        let total=if mode==1 {1} else {m.u32(config+0x200)};
        m.set_u32(p+0x1c,0);m.set_u32(p+0x18,total);
        let total=get(m,models+20).wrapping_add(get(m,ctx+0x3a9c));
        let history_count=if m.u32(models+4)!=m.u32(ctx+0x460) {get(m,ctx+0x3a90).min(total)} else {0};
        m.set_u32(p+0x20,total as u32);
        let mut status;
        if mode==2 {
            m.set_u32(p+0x34,0);m.set_u32(p+0x18,get(m,p+0x18).min(2) as u32);m.set_u32(p+0x30,0);padding=2;
            status=call(m,h,0x2a76a4,&[m.u32(owner+4),m.u32(p+0x18),m.u32(p+0x20),padding,workspace+0x50]);
            if status==0 {m.set_u32(workspace+0x3c,m.u32(p+0x18));m.set_u32(workspace+0x4c,0);m.set_u32(workspace+0x38,m.u32(p+0x20));m.set_u32(p+0x30,m.u32(workspace+0x50));}
        } else {status=mid::prepare_input_tables(m,h,owner,p,workspace);}
        'work: {
            if status>=0x80000000 {break 'work;}
            let mut index=0i32;
            let ptr=call(m,h,0x1963b8,&[m.u32(owner+4),m.u32(p+0x18),4]);m.set_u32(output+4,ptr);
            if ptr==0 {status=0x84e0200a;break 'work;}
            m.set_u32(output+4,ptr.wrapping_sub(4));
            if paired {
                let ptr=call(m,h,0x1963b8,&[m.u32(owner+4),m.u32(p+0x18),4]);m.set_u32(output+8,ptr);
                if ptr==0 {status=0x84e0200a;break 'work;}
                m.set_u32(output+8,ptr.wrapping_sub(4));
            }
            if m.u32(models)!=m.u32(ctx+0x45c) {
                while index<get(m,ctx+0x3a9c) {
                    if m.u8(m.u32(ctx+0x430).wrapping_add(index as u32))&4!=0 {
                        for channel in 0..get(m,p+0x18) {
                            let hist=ctx+0x395c+(get(m,ctx+0x3a9c)-index) as u32*24;
                            m.set_u32(cell(m,m.u32(p+0x30),channel,count),m.u32(at(hist,channel)));
                            if paired {m.set_u32(cell(m,m.u32(p+0x34),channel,count),m.u32(at(hist+12,channel)));}
                        }
                        count+=1;
                    }
                    index+=1;
                }
            }
            let old_count=count;
            let mut model=m.u32(models);
            while model!=m.u32(models+4) {
                let position=m.u16(model+4) as i16 as i32;
                let boundary=position==0 || m.u8(m.u32(ctx+0x50).wrapping_add(position as u32))==1;
                for state in 2..get(m,config+0x1f8).wrapping_add(2) {
                    let id=call(m,h,0x2d9dc0,&[ctx+0x3c,position as u32,config+0x440,3,(state-2) as u32]);
                    if get(m,options+0x2c)>20 {call(m,h,0x1738e8,&[m.u32(owner+16),ROM,4,0,ROM,id]);}
                    m.set_u8(output,0);
                    status=if !paired {mid::select_band(m,h,m.u32(ctx+0x20),state as u32,id,m.u32(p+0x18) as i32,output,config+0x1e0,get(m,options+0x24),output+12)}
                    else {mid::select_spectral(m,h,m.u32(ctx+0x20),state as u32,id,output,config+0x1e0,get(m,options+0x24),output+12)};
                    if status>=0x80000000 {break 'work;}
                    if boundary && state!=2 && get(m,config+0x1f8).wrapping_add(1)!=state {m.set_u8(output,0);}
                    for _ in 0..m.u16(m.u32(model+12)+state as u32*2) {
                        let flag=m.u32(ctx+0x430).wrapping_add(index as u32);m.set_u8(flag,m.u8(flag)|m.u8(output));
                        if m.u8(flag)&4!=0 {
                            for channel in 0..get(m,p+0x18) {
                                let v=get(m,at(m.u32(output+4),channel+1)).clamp(-0x1f400000,0x1f400000);
                                m.set_u32(cell(m,m.u32(p+0x30),channel,count),v as u32);

                                if m.u8(model+20)&4!=0 && channel==0 && m.u8(options+0x42)==b'q' {
                                    let dst=cell(m,m.u32(p+0x30),0,count);m.set_u32(dst,get(m,dst).clamp(-0x1f400000,0x1f400000) as u32);
                                }
                                if paired {let v=get(m,at(m.u32(output+8),channel+1)).clamp(-0x1f400000,0x1f400000);m.set_u32(cell(m,m.u32(p+0x34),channel,count),v as u32);}
                            }
                            count+=1;
                        }
                        index+=1;
                    }
                }
                model=m.u32(model+24);
            }
            free(m,h,owner,m.u32(output+4).wrapping_add(4));m.set_u32(output+4,0);
            if paired {free(m,h,owner,m.u32(output+8).wrapping_add(4));m.set_u32(output+8,0);}
            m.set_u32(p+0x20,count as u32);remaining=count;
            for i in 0..history_count {
                let flag=m.u8(m.u32(ctx+0x430).wrapping_add((index-i-1) as u32));m.set_u8(ctx+0x384c+i as u32*28,flag);
                let hist=ctx+0x395c+(i+1) as u32*24;
                if flag&4!=0 {
                    remaining-=1;
                    for channel in 0..get(m,p+0x18) {
                        m.set_u32(at(hist,channel),m.u32(cell(m,m.u32(p+0x30),channel,remaining)));
                        if paired {m.set_u32(at(hist+12,channel),m.u32(cell(m,m.u32(p+0x34),channel,remaining)));}
                    }
                } else {for j in 0..6 {m.set_u32(hist+j*4,0);}}
            }
            if count>0 {
                count=0;
                for i in 0..index {
                    if m.u8(m.u32(ctx+0x430).wrapping_add(i as u32))&4!=0 {
                        for channel in 1..get(m,p+0x18) {
                            let window=m.u32(at(m.u32(m.u32(p+0x28)+0x1c),channel));let mut mask=4u8;
                            let mut j=get(m,window);let mut n=get(m,window+4).wrapping_sub(j).wrapping_add(1);
                            while n>0 {
                                let k=i.wrapping_add(j);
                                if k<0 || index<k {mask&=!4;} else {mask&=m.u8(m.u32(ctx+0x430).wrapping_add(k as u32));}
                                j=j.wrapping_add(1);n-=1;
                            }
                            if mask&4==0 {
                                if paired {m.set_u32(cell(m,m.u32(p+0x34),channel,count),0);}
                                m.set_u32(cell(m,m.u32(p+0x30),channel,count),0);
                            }
                        }
                        count+=1;
                    }
                }
                if count>0 {
                    m.set_u32(p+0x20,count as u32);m.set_u32(p+0x1c,0);
                    if mode==3 {status=mid::prepare_gain_tables(m,h,owner,count,get(m,m.u32(p+0x28)+0x30).wrapping_mul(2).wrapping_add(1),get(m,p+0x18),0,workspace);if status>=0x80000000 {break 'work;}}
                    status=mid::prepare_filter_rows(m,h,owner,workspace,p,ROM);if status>=0x80000000 {break 'work;}
                    call(m,h,if mode==1 {0x2a37f4} else if mode==3 {0x2a2be0} else {0x2a38d8},&[owner,p]);
                }
            }
            let mut i=0;
            for j in 0..7 {if i>=old_count {break;}if m.u8(m.u32(ctx+0x430)+j)&4!=0 {
                let dst=at(m.u32(p+0x2c),i);
                let v=(get(m,dst).wrapping_add(4)>>3).wrapping_mul(weights[j as usize])
                    .wrapping_add((get(m,at(ctx+0x3a64,i)).wrapping_add(4)>>3).wrapping_mul(weights[j as usize+7]));
                m.set_u32(dst,v as u32);i+=1;
            }}
            m.copy(ctx+0x3a64,at(m.u32(p+0x2c),remaining),count.wrapping_sub(remaining).wrapping_mul(4) as u32);
        }
        if m.u32(output+4)!=0 {free(m,h,owner,m.u32(output+4).wrapping_add(4));}
        if paired && m.u32(output+8)!=0 {free(m,h,owner,m.u32(output+8).wrapping_add(4));}
        if mode==2 {
            let values=m.u32(workspace+0x50);
            if values!=0 {free(m,h,owner,m.u32(values).wrapping_sub(padding*4));free(m,h,owner,values);m.set_u32(workspace+0x50,0);}
            m.set_u32(p+0x30,0);
        }
        mid::reset_workspace(m,h,p,workspace);m.set_u32(p+0x20,remaining as u32);status
    })
}

fn synthesize_phones_body(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    synth: u32,
    model: u32,
    config: u32,
    done: u32,
    values: u32,
    blend_current: &[i32],
    blend_previous: &[i32],
) -> u32 {
    let mut pad = 0i32;
    let output = synth.wrapping_add(0x26c);
    let start = arena.u32(synth.wrapping_add(0x494));
    let mut extra = 0i32;
    let mut history_frames = 0i32;
    let mut orders = arena.u32(config.wrapping_add(8)) as i32;
    let queue = synth.wrapping_add(0x440);
    let work = synth.wrapping_add(0x498);
    let previous = synth.wrapping_add(0x4e0);
    let mut overlap = 0i32;
    let input = synth.wrapping_add(0x3d4);
    let context = arena.u32(synth.wrapping_add(0x18));
    let heap = arena.u32(context.wrapping_add(4));
    let database = arena.u32(synth.wrapping_add(0x20));
    let model_count = arena.u32(model.wrapping_add(0x1f8)) as i32;
    let first_order = arena.u32(config.wrapping_add(0x3c)) as i32;
    let mut fast_order = arena.u16(model.wrapping_add(0x394)) as i32;
    if (arena.u32(output.wrapping_add(0x1c)) as i32) <= orders {
        orders = arena.u32(output.wrapping_add(0x1c)) as i32;
    }
    orders = orders.wrapping_add(1);
    if arena.u8(config.wrapping_add(0x114)) == 2 {
        if fast_order >= first_order {
            fast_order = first_order;
        }
        let configured = arena.u32(config.wrapping_add(12)) as i32;
        if configured != 99 && fast_order <= configured {
            fast_order = configured;
        }
    }
    let mut frames = 0i32;
    let mut phones = 0i32;
    arena.set_u8(done, 0);
    arena.set_u8(
        start.wrapping_add(20),
        arena.u8(start.wrapping_add(20)) & !2,
    );
    let mut phone = start;
    while arena.u8(phone.wrapping_add(20)) & 2 == 0 && arena.u32(queue.wrapping_add(4)) != phone {
        phones = phones.wrapping_add(1);
        frames = frames.wrapping_add(arena.u32(phone.wrapping_add(16)) as i32);
        phone = arena.u32(phone.wrapping_add(24));
    }
    if arena.u32(queue.wrapping_add(4)) == phone {
        arena.set_u8(done, 1);
    }
    arena.set_u32(synth.wrapping_add(0x494), phone);
    host.call(
        arena,
        0x1738e8,
        &[
            arena.u32(context.wrapping_add(16)),
            ROM_SENTINEL,
            4,
            0,
            ROM_SENTINEL,
            phones as u32,
            frames as u32,
        ],
    );
    let model_ptr = arena.u32(output.wrapping_add(0x28));
    arena.set_u32(
        work.wrapping_add(0x18),
        arena.u32(model_ptr.wrapping_add(0x18)),
    );
    let mut output_frames = frames;
    arena.set_u32(output.wrapping_add(0x20), output_frames as u32);
    if arena.u32(queue.wrapping_add(4)) != phone && phone != 0 {
        extra = 1;
        phones = phones.wrapping_add(1);
        frames = frames.wrapping_add(arena.u32(config.wrapping_add(0x58)) as i32);
    }
    if arena.u32(synth.wrapping_add(0x45c)) != start {
        let saved_overlap = arena.u32(synth.wrapping_add(0x3a98)) as i32;
        history_frames = arena.u32(config.wrapping_add(0x54)) as i32;
        if saved_overlap > history_frames {
            history_frames = saved_overlap;
        }
        frames = frames.wrapping_add(history_frames);
        output_frames = output_frames.wrapping_add(saved_overlap);
        arena.set_u32(output.wrapping_add(0x20), output_frames as u32);
    }
    let cursor = arena.u32(synth.wrapping_add(0x434)) as i32;
    if arena.u32(queue.wrapping_add(4)) != arena.u32(synth.wrapping_add(0x460))
        && (arena.u32(previous.wrapping_add(0x20)) as i32) < cursor.wrapping_add(output_frames)
    {
        let limit = arena.u32(synth.wrapping_add(0x3a90)) as i32;
        overlap = if frames < limit { frames } else { limit };
        output_frames = output_frames.wrapping_sub(overlap);
        arena.set_u32(output.wrapping_add(0x20), output_frames as u32);
    }
    arena.set_u32(work.wrapping_add(0x20), frames as u32);
    let old_channels = arena.u32(output.wrapping_add(0x18)) as i32;
    arena.set_u32(output.wrapping_add(0x18), 0);
    arena_mid::release_filter(arena, host, output);
    let mut rc = arena_mid::prepare_filter_rows(arena, host, context, 0, output, ROM_SENTINEL);
    if rc >= 0x8000_0000 {
        return cleanup(
            arena, host, synth, input, work, phone, done, fast_order, orders, pad, 0, 0, rc,
        );
    }
    arena.set_u32(output.wrapping_add(0x18), old_channels as u32);
    if first_order >= fast_order {
        arena.set_u32(work.wrapping_add(0x30), 0);
        arena.set_u32(work.wrapping_add(0x34), 0);
        let mut work_channels = arena.u32(work.wrapping_add(0x18)) as i32;
        if work_channels >= 2 {
            work_channels = 2;
            arena.set_u32(work.wrapping_add(0x18), 2);
        }
        pad = 2;
        rc = host
            .call(
                arena,
                0x2a76a4,
                &[
                    heap,
                    work_channels as u32,
                    arena.u32(work.wrapping_add(0x20)),
                    pad as u32,
                    input.wrapping_add(0x50),
                ],
            )
            .0;
        if rc == 0 {
            arena.set_u32(input.wrapping_add(0x3c), arena.u32(work.wrapping_add(0x18)));
            arena.set_u32(input.wrapping_add(0x4c), 0);
            arena.set_u32(input.wrapping_add(0x38), arena.u32(work.wrapping_add(0x20)));
        } else if rc >= 0x8000_0000 {
            return cleanup(
                arena, host, synth, input, work, phone, done, fast_order, orders, pad, 0, 0, rc,
            );
        }
    } else {
        rc = arena_mid::prepare_input_tables(arena, host, context, work, input);
        if rc >= 0x8000_0000 {
            return cleanup(
                arena, host, synth, input, work, phone, done, fast_order, orders, pad, 0, 0, rc,
            );
        }
    }
    if first_order < fast_order {
        let radius = arena.u32(arena.u32(work.wrapping_add(0x28)).wrapping_add(0x30)) as i32;
        rc = arena_mid::prepare_gain_tables(
            arena,
            host,
            context,
            arena.u32(work.wrapping_add(0x20)) as i32,
            radius.wrapping_mul(2).wrapping_add(1),
            arena.u32(work.wrapping_add(0x18)) as i32,
            0,
            input,
        );
        if rc >= 0x8000_0000 {
            return cleanup(
                arena, host, synth, input, work, phone, done, fast_order, orders, pad, 0, 0, rc,
            );
        }
    }
    rc = arena_mid::prepare_filter_rows(arena, host, context, input, work, ROM_SENTINEL);
    if rc >= 0x8000_0000 {
        return cleanup(
            arena, host, synth, input, work, phone, done, fast_order, orders, pad, 0, 0, rc,
        );
    }
    let means = host
        .call(
            arena,
            0x1963b8,
            &[heap, arena.u32(work.wrapping_add(0x18)), 4],
        )
        .0;
    if means == 0 {
        return cleanup(
            arena,
            host,
            synth,
            input,
            work,
            phone,
            done,
            fast_order,
            orders,
            pad,
            0,
            0,
            0x84e0_200a,
        );
    }
    let variances = host
        .call(
            arena,
            0x1963b8,
            &[heap, arena.u32(work.wrapping_add(0x18)), 4],
        )
        .0;
    if variances == 0 {
        return cleanup(
            arena,
            host,
            synth,
            input,
            work,
            phone,
            done,
            fast_order,
            orders,
            pad,
            means,
            0,
            0x84e0_200a,
        );
    }
    let mut order = first_order;
    while order < orders {
        if order == fast_order && first_order != order {
            arena_mid::reset_workspace(arena, host, work, input);
            arena.set_u32(work.wrapping_add(0x34), 0);
            arena.set_u32(work.wrapping_add(0x30), 0);
            let mut work_channels = arena.u32(work.wrapping_add(0x18)) as i32;
            if work_channels >= 2 {
                work_channels = 2;
                arena.set_u32(work.wrapping_add(0x18), 2);
            }
            pad = 2;
            rc = allocate_rows(
                arena,
                host,
                heap,
                work_channels,
                arena.u32(work.wrapping_add(0x20)) as i32,
                pad,
                input.wrapping_add(0x50),
            );
            if rc >= 0x8000_0000 {
                break;
            }
            arena.set_u32(input.wrapping_add(0x3c), arena.u32(work.wrapping_add(0x18)));
            arena.set_u32(input.wrapping_add(0x4c), 0);
            arena.set_u32(input.wrapping_add(0x38), arena.u32(work.wrapping_add(0x20)));
            arena.set_u32(work.wrapping_add(0x30), arena.u32(input.wrapping_add(0x50)));
        }
        let mut frame = 0i32;
        let mut n = 0i32;
        phone = start;
        let work_channels = arena.u32(work.wrapping_add(0x18)) as i32;
        if arena.u32(synth.wrapping_add(0x45c)) != start {
            while frame < history_frames {
                for i in 0..work_channels {
                    let row = arena.u32(
                        arena
                            .u32(work.wrapping_add(0x30))
                            .wrapping_add((i as u32).wrapping_mul(4)),
                    );
                    arena.set_u32(
                        row.wrapping_add((frame as u32).wrapping_mul(4)),
                        arena.u32(history_at(synth, history_frames - frame - 1, order, i)),
                    );
                    if order < fast_order {
                        let vrow = arena.u32(
                            arena
                                .u32(work.wrapping_add(0x34))
                                .wrapping_add((i as u32).wrapping_mul(4)),
                        );
                        arena.set_u32(
                            vrow.wrapping_add((frame as u32).wrapping_mul(4)),
                            arena.u32(history_at(synth, history_frames - frame - 1, order + 25, i)),
                        );
                    }
                }
                frame = frame.wrapping_add(1);
            }
        }
        let voice = model.wrapping_add(0x1e0);
        while n != phones {
            let models = arena.u32(phone.wrapping_add(8));
            let durations = arena.u32(phone.wrapping_add(12));
            let mut j = 2i32;
            while j < model_count.wrapping_add(2)
                && frame < arena.u32(work.wrapping_add(0x20)) as i32
            {
                arena.set_u32(values, 0);
                arena.set_u32(values.wrapping_add(4), means);
                arena.set_u32(values.wrapping_add(8), variances);
                if order >= fast_order {
                    rc = host
                        .call(
                            arena,
                            0x2da8c8,
                            &[
                                database,
                                j as u32,
                                arena.u16(models.wrapping_add((j as u32).wrapping_mul(2))) as u32,
                                order as u32,
                                work_channels as u32,
                                values,
                                voice,
                            ],
                        )
                        .0;
                } else {
                    rc = crate::selection::arena::read_voice_pair(
                        arena,
                        host,
                        database,
                        j as u32,
                        arena.u16(models.wrapping_add((j as u32).wrapping_mul(2))) as u32,
                        order as u32,
                        values,
                        voice,
                    );
                }
                if rc >= 0x8000_0000 {
                    break;
                }
                let mut k = 1i32;
                while k <= arena.u16(durations.wrapping_add((j as u32).wrapping_mul(2))) as i32
                    && frame < arena.u32(work.wrapping_add(0x20)) as i32
                {
                    for channel in 0..work_channels {
                        let row = arena.u32(
                            arena
                                .u32(work.wrapping_add(0x30))
                                .wrapping_add((channel as u32).wrapping_mul(4)),
                        );
                        let means_base = arena.u32(values.wrapping_add(4));
                        let value = arena
                            .u32(means_base.wrapping_add((channel as u32).wrapping_mul(4)))
                            as i32;
                        arena.set_u32(
                            row.wrapping_add((frame as u32).wrapping_mul(4)),
                            value as u32,
                        );
                        arena.set_u32(
                            row.wrapping_add((frame as u32).wrapping_mul(4)),
                            (if value > 0x7d00000 {
                                0x7d00000
                            } else if value <= -0x7d00000 {
                                -0x7d00000
                            } else {
                                value
                            }) as u32,
                        );
                        if order < fast_order {
                            let vrow = arena.u32(
                                arena
                                    .u32(work.wrapping_add(0x34))
                                    .wrapping_add((channel as u32).wrapping_mul(4)),
                            );
                            let variances_base = arena.u32(values.wrapping_add(8));
                            let mut v = arena
                                .u32(variances_base.wrapping_add((channel as u32).wrapping_mul(4)))
                                as i32;
                            arena.set_u32(
                                vrow.wrapping_add((frame as u32).wrapping_mul(4)),
                                v as u32,
                            );
                            if v > 0x7d00000 {
                                v = 0x7d00000;
                            }
                            arena.set_u32(
                                vrow.wrapping_add((frame as u32).wrapping_mul(4)),
                                v as u32,
                            );
                        }
                    }
                    frame = frame.wrapping_add(1);
                    k = k.wrapping_add(1);
                }
                j = j.wrapping_add(1);
            }
            if rc >= 0x8000_0000 {
                break;
            }
            n = n.wrapping_add(1);
            phone = arena.u32(phone.wrapping_add(24));
        }
        if rc >= 0x8000_0000 {
            break;
        }
        if extra != 0 {
            frame = frame.wrapping_sub(arena.u32(config.wrapping_add(0x58)) as i32);
        }
        let history_limit = arena.u32(config.wrapping_add(0x54)) as i32;
        let keep = if history_limit > overlap {
            history_limit
        } else {
            overlap
        };
        let mut i = 0i32;
        while i < keep {
            for j in 0..arena.u32(work.wrapping_add(0x18)) as i32 {
                let row = arena.u32(
                    arena
                        .u32(work.wrapping_add(0x30))
                        .wrapping_add((j as u32).wrapping_mul(4)),
                );
                let at = if i < frame { frame - i - 1 } else { 0 };
                arena.set_u32(
                    history_at(synth, i, order, j),
                    arena.u32(row.wrapping_add((at as u32).wrapping_mul(4))),
                );
                if order < fast_order {
                    let vrow = arena.u32(
                        arena
                            .u32(work.wrapping_add(0x34))
                            .wrapping_add((j as u32).wrapping_mul(4)),
                    );
                    arena.set_u32(
                        history_at(synth, i, order + 25, j),
                        arena.u32(vrow.wrapping_add((at as u32).wrapping_mul(4))),
                    );
                }
            }
            i = i.wrapping_add(1);
        }
        if order >= fast_order {
            host.call(arena, 0x2a3b18, &[context, work]);
        } else if arena.u32(model.wrapping_add(0x488)) == 0 {
            host.call(arena, 0x2a24f8, &[context, work]);
        } else if order < 3 {
            arena_mid::run_forward_stream(arena, host, context, work);
        } else {
            host.call(
                arena,
                0x2daf68,
                &[
                    work,
                    order as u32,
                    arena.u32(synth.wrapping_add(0x3aa0)),
                    model.wrapping_add(0x484),
                ],
            );
        }
        let linear = arena.u32(work.wrapping_add(0x2c));
        output_frames = arena.u32(output.wrapping_add(0x20)) as i32;

        let blend_bound = arena.u32(synth.wrapping_add(0x3a98)) as i32;
        let capped = if blend_bound > 7 { 7 } else { blend_bound };
        let mut i = 0i32;
        while i < capped {
            let line = arena.u32(linear.wrapping_add((i as u32).wrapping_mul(4))) as i32;
            let tail = arena.u32(tails_at(synth, order, i)) as i32;
            arena.set_u32(
                linear.wrapping_add((i as u32).wrapping_mul(4)),
                (((line.wrapping_add(4) >> 3).wrapping_mul(blend_value(blend_current, i)))
                    .wrapping_add(
                        (tail.wrapping_add(4) >> 3).wrapping_mul(blend_value(blend_previous, i)),
                    )) as u32,
            );
            i = i.wrapping_add(1);
        }
        if overlap != 0 {
            let mut offset = 0i32;
            if arena.u32(synth.wrapping_add(0x45c)) != start
                && (arena.u32(synth.wrapping_add(0x3a98)) as i32)
                    < (arena.u32(config.wrapping_add(0x54)) as i32)
            {
                offset = (arena.u32(config.wrapping_add(0x54)) as i32)
                    .wrapping_sub(arena.u32(synth.wrapping_add(0x3a98)) as i32);
            }
            arena_leaf::run(
                arena,
                0x195ae8,
                &[
                    tails_at(synth, order, 0),
                    linear
                        .wrapping_add((output_frames.wrapping_add(offset) as u32).wrapping_mul(4)),
                    (overlap as u32).wrapping_mul(4),
                ],
            );
        }
        let mut i = 0i32;
        if arena.u32(synth.wrapping_add(0x45c)) != start
            && (arena.u32(synth.wrapping_add(0x3a98)) as i32)
                < arena.u32(config.wrapping_add(0x54)) as i32
        {
            i = (arena.u32(config.wrapping_add(0x54)) as i32)
                .wrapping_sub(arena.u32(synth.wrapping_add(0x3a98)) as i32);
        }
        let mut j = 0i32;
        while j < output_frames {
            let row = arena.u32(
                arena
                    .u32(output.wrapping_add(0x2c))
                    .wrapping_add((j as u32).wrapping_mul(4)),
            );
            arena.set_u32(
                row.wrapping_add((order as u32).wrapping_mul(4)),
                arena.u32(linear.wrapping_add((i as u32).wrapping_mul(4))),
            );
            i = i.wrapping_add(1);
            j = j.wrapping_add(1);
        }
        order = order.wrapping_add(1);
    }

    if rc < 0x8000_0000 {
        arena.set_u32(synth.wrapping_add(0x3a98), overlap as u32);
    }
    let mut i = 0i32;
    while rc < 0x8000_0000 && i < first_order {
        let mut j = 0i32;
        while j < output_frames {
            let dst = arena.u32(
                arena
                    .u32(output.wrapping_add(0x2c))
                    .wrapping_add((j as u32).wrapping_mul(4)),
            );
            let src = arena.u32(
                arena
                    .u32(previous.wrapping_add(0x2c))
                    .wrapping_add(((cursor.wrapping_add(j)) as u32).wrapping_mul(4)),
            );
            arena.set_u32(
                dst.wrapping_add((i as u32).wrapping_mul(4)),
                arena.u32(src.wrapping_add((i as u32).wrapping_mul(4))),
            );
            j = j.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    cleanup(
        arena, host, synth, input, work, phone, done, fast_order, orders, pad, means, variances, rc,
    )
}

fn history_at(synth: u32, frame: i32, order: i32, channel: i32) -> u32 {
    synth.wrapping_add(0x56c).wrapping_add(
        (((frame as u32).wrapping_mul(50).wrapping_add(order as u32))
            .wrapping_mul(3)
            .wrapping_add(channel as u32))
        .wrapping_mul(4),
    )
}

fn tails_at(synth: u32, order: i32, index: i32) -> u32 {
    synth
        .wrapping_add(0x344c)
        .wrapping_add(((order as u32).wrapping_mul(10).wrapping_add(index as u32)).wrapping_mul(4))
}

fn blend_value(table: &[i32], index: i32) -> i32 {
    table[index as usize]
}

#[allow(clippy::too_many_arguments)]
fn cleanup(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    synth: u32,
    input: u32,
    work: u32,
    _phone: u32,
    done: u32,
    fast_order: i32,
    orders: i32,
    pad: i32,
    means: u32,
    variances: u32,
    rc: u32,
) -> u32 {
    let context = arena.u32(synth.wrapping_add(0x18));
    let heap = arena.u32(context.wrapping_add(4));
    if means != 0 {
        host.call(arena, 0x196428, &[heap, means]);
    }
    if variances != 0 {
        host.call(arena, 0x196428, &[heap, variances]);
    }
    if fast_order < orders {
        let data = arena.u32(input.wrapping_add(0x50));
        if data != 0 {
            host.call(
                arena,
                0x196428,
                &[
                    heap,
                    arena.u32(data).wrapping_sub((pad as u32).wrapping_mul(4)),
                ],
            );
            host.call(arena, 0x196428, &[heap, data]);
            arena.set_u32(input.wrapping_add(0x50), 0);
        }
        arena.set_u32(work.wrapping_add(0x30), 0);
    }
    arena_mid::reset_workspace(arena, host, work, input);
    arena_mid::release_filter(arena, host, work);
    if arena.u8(done) != 0 {
        arena_mid::release_filter(arena, host, synth.wrapping_add(0x4e0));
    }
    let model_ptr = arena.u32(synth.wrapping_add(0x26c).wrapping_add(0x28));
    arena.set_u32(
        work.wrapping_add(0x18),
        arena.u32(model_ptr.wrapping_add(0x18)),
    );
    rc
}

fn allocate_rows(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    heap: u32,
    rows: i32,
    columns: i32,
    pad: i32,
    out: u32,
) -> u32 {
    let array = host
        .call(arena, 0x196394, &[heap, (rows as u32).wrapping_shl(2)])
        .0;
    arena.set_u32(out, array);
    if array == 0 {
        return 0x84e0_200a;
    }
    let stride = columns.wrapping_add(pad.wrapping_mul(2));
    let mut data = host
        .call(
            arena,
            0x196394,
            &[heap, (stride.wrapping_mul(rows) as u32).wrapping_shl(2)],
        )
        .0;
    if data == 0 {
        host.call(arena, 0x196428, &[heap, arena.u32(out)]);
        arena.set_u32(out, 0);
        return 0x84e0_200a;
    }
    let mut i = 0i32;
    while i < rows {
        arena.set_u32(
            array.wrapping_add((i as u32).wrapping_mul(4)),
            data.wrapping_add((pad as u32).wrapping_mul(4)),
        );
        data = data.wrapping_add((stride as u32).wrapping_mul(4));
        i = i.wrapping_add(1);
    }
    0
}
