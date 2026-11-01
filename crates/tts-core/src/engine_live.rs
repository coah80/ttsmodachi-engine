
use crate::{boot::Memory, db_live::{DbHooksM, PoolHooksM}, engine};

#[derive(Clone, Copy, Debug)]
pub struct CallTrace { pub entry: u32, pub caller: u32, pub args: [u32; 4], pub nargs: usize, pub r0: u32, pub depth: u32 }

pub trait CallTraceSink { fn record_entry(&mut self, t: CallTrace) -> usize; fn record_return(&mut self, idx: usize, r0: u32); }
struct Host<'a> { boot: engine::SpeechBootHost<'a>, boundary:Option<engine::VoiceBoundary>, runtime:Option<&'a mut dyn RuntimeServicesM>, snap:&'a [u8], trace:Option<&'a mut dyn CallTraceSink>, until:Option<u32>, depth:u32, stack:[u32;64], stack_len:usize, rom:crate::rom_region::Rom }

struct ReleaseCalls<'a, 'b>(&'a mut Host<'b>);
impl crate::component_live_a2::Calls for ReleaseCalls<'_, '_> {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.0.call(m, entry, args)
    }
}

struct ParamBridge<'a, 'b> {
    host: &'a mut Host<'b>,
}

struct DepesBridge<'a, 'b> {
    host: &'a mut Host<'b>,
}

struct DictBridge<'a, 'b> {
    host: &'a mut Host<'b>,
}
impl crate::boot_part_f::Host for DictBridge<'_, '_> {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.host.call(m, method, args)
    }
    fn compare(&mut self, _m: &mut Memory<'_>, method: u32, _key: &mut crate::boot_part_f::Key, _value: &mut u32) -> u32 {
        self.host.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: method });
        0x80002001
    }
}
impl crate::log_live::LogMethodsM for DictBridge<'_, '_> {
    fn log_m(&mut self, f: u32, o: u32, c: u32, l: u32, a: u32, t: u32, v: u32, m: &mut Memory<'_>) -> u32 {
        self.host.call(m, f, &[o, c, l, a, t, v])
    }
}
impl crate::speech_part_5::Host for DictBridge<'_, '_> {
    fn scratch(&self) -> u32 {
        0x7a00
    }
    fn format(&self, callsite: u32) -> u32 {
        callsite
    }
    fn dictionary_entries(&mut self, _m: &mut Memory<'_>, dictionary: u32, entries: &mut u32, count: &mut u32) -> u32 {
        *entries = 0;
        *count = 0;
        self.host.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: dictionary });
        0x80002001
    }
    fn record(&mut self, _m: &mut Memory<'_>, record: u32, _arg: u32, _buffer: &mut crate::speech_part_5::RecordBuffer, found: &mut u8) -> u32 {
        *found = 0;
        self.host.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: record });
        0x80002001
    }
    fn entries(&mut self, m: &mut Memory<'_>, a: u32, b: u32, c: u32, array: &mut u32, count: &mut u32) -> u32 {

        *array = 0;
        *count = 0;
        crate::speech_part_5::fn_2f88d4_m(m, a, b, c, array, count, self) as u32
    }
}
impl crate::speech_part_3::SpeechHost for DictBridge<'_, '_> {}
impl crate::component_live_c2::C2DepesHost for DepesBridge<'_, '_> {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.host.call(m, entry, args)
    }
    fn rom(&self) -> crate::rom_region::Rom {
        self.host.rom
    }
}
impl crate::engine_components::InternalCalls for ParamBridge<'_, '_> {
    fn call(
        &mut self,
        m: &mut engine::Memory<'_, '_>,
        entry: u32,
        a: &[u32],
    ) -> Result<u32, engine::Error> {

        let entry = crate::rom_region::to_us_guest(self.host.rom, entry);
        if (entry == 0x175a1c || entry == 0x1795f0 || entry == 0x1ecf68 || entry == 0x14e440 || entry == 0x150878 || entry == 0x150c0c || entry == 0x1516c8 || entry == 0x15017c || entry == 0x1503b0 || entry == 0x1ea4e0 || entry == 0x17a1a4 || entry == 0x1963e0 || entry == 0x2409d4 || entry == 0x21a1fc) && m.regions.len() == 1 {
            let base = m.regions[0].base;
            let bytes = &mut *m.regions[0].bytes;
            let mut bm = Memory { base, bytes };
            let r0 = self.host.call(&mut bm, entry, a);

            if self.host.stop_is_until() {
                return Err(engine::Error::UnsupportedStage { entry });
            }
            return Ok(r0);
        }

        if self.host.stop_is_until() {
            return Err(engine::Error::UnsupportedStage { entry });
        }

        let heap_logged = matches!(entry, 0x196394 | 0x1963b8 | 0x1963e0 | 0x196428);
        let rc = crate::engine_components::InternalCalls::call(
            &mut engine::LiveComponentCalls,
            m,
            entry,
            a,
        );
        if heap_logged {
            if let Ok(r0) = rc {
                self.host.trace_heap(entry, a, r0);
            }
        }
        rc
    }
}

impl Host<'_> {

    pub(crate) fn stop_is_until(&self) -> bool {
        match (&self.boundary, self.until) {
            (Some(b), Some(u)) => b.entry == u,
            _ => false,
        }
    }
    pub(crate) fn trace_heap(&mut self, entry: u32, a: &[u32], r0: u32) {
        if let Some(t) = self.trace.as_mut() {
            let caller = if self.stack_len > 0 { self.stack[self.stack_len - 1] } else { 0 };
            let nargs = a.len().min(4);
            let mut args4 = [0u32; 4];
            args4[..nargs].copy_from_slice(&a[..nargs]);
            let idx = t.record_entry(CallTrace { entry, caller, args: args4, nargs, r0: 0xeeee_eeee, depth: self.depth });
            t.record_return(idx, r0);
        }
    }
}
impl Host<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {

        if self.depth > 60 {
            panic!("live call loop at {f:#x}");
        }
        let caller = if self.stack_len > 0 { self.stack[self.stack_len - 1] } else { 0 };
        let depth = self.depth;
        self.depth += 1;
        if (self.stack_len as usize) < self.stack.len() {
            self.stack[self.stack_len] = f;
            self.stack_len += 1;
        }
        let nargs = a.len().min(4);
        let mut args4 = [0u32; 4];
        args4[..nargs].copy_from_slice(&a[..nargs]);
        let idx = if let Some(t) = self.trace.as_mut() {
            Some(t.record_entry(CallTrace { entry: f, caller, args: args4, nargs, r0: 0xeeee_eeee, depth }))
        } else { None };
        let r = self.call_inner(m, f, a);
        if let (Some(t), Some(i)) = (self.trace.as_mut(), idx) {
            t.record_return(i, r);
        }
        if self.stack_len > 0 { self.stack_len -= 1; }
        self.depth -= 1;
        r
    }
    fn call_inner(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        let f = if self.rom == crate::rom_region::Rom::Us { f } else { crate::rom_region::to_us(self.rom, f) };
        if self.boundary.is_some(){return 0x80002001;}

        if self.until==Some(f){
            let mut args=[0;7];let n=a.len().min(7);args[..n].copy_from_slice(&a[..n]);
            self.boundary=Some(engine::VoiceBoundary{entry:f,args,event:None});return 0x80002001;
        }
        match f {
            0x2645b4 => {

                let snap = self.snap;
                crate::engine_live_render::run_block(self, m, a[0], a[1], a[2], snap)
            }
            0x26a3e4 => {

                let snap = self.snap;
                crate::engine_live_render::run_writer(
                    self, m, a[0], a[1], a[2], [a[3], a[4]], [a[5], a[6]], snap,
                )
            }
            0x252f88 => {

                let mut p = 0;
                let env = crate::render_part_c::Environment {
                    cached_handle: core::array::from_fn(|i| m.byte(crate::rom_region::addr(self.rom,0x8d7660) + i as u32)),
                    default_context: crate::rom_region::addr(self.rom,0x8d7640),
                };
                let rc = crate::normalization::fetch_entry(
                    m.bytes,
                    a[0],
                    a[1],
                    a[2],
                    Some(&mut p),
                    &mut crate::engine_live_render::LiveFetch { host: self, env: &env },
                );
                if rc < 0x80000000 {
                    m.put_word(a[3], p);
                }
                rc
            }
            0xffff0001=>{self.boot.fault=Some(engine::Error::MissingStreamAdapter{entry:f});0x80002001},
            0x1738e8|0x173dc0=>{

                if self.boot.fault.is_some(){return 0x80002001;}
                let mut args=[0u32;6];let n=a.len().min(6);args[..n].copy_from_slice(&a[..n]);
                if f==0x1738e8 {crate::log_live::fn_1738e8(m,args[0],args[1],args[2],args[3],args[4],args[5],self)}
                else {crate::render_part_a::error_log(m,self,args[0],args[1],args[2],args[3],args[4],args[5])}
            }
            0x1a18c4|0x166054|0x183258=>{
                if f == 0x1a18c4 && a.len() >= 2 && m.word(a[1] + 0x19c) == 0 {
                    let scratch = 0x0e0f7f00;
                    let saved = [m.word(scratch), m.word(scratch + 4)];
                    let rc = self.call(m, 0x19611c, &[m.word(a[1] + 8), m.word(a[1] + 12), scratch]);
                    let rc = if rc < 0x80000000 {
                        self.call(m, 0x17b058, &[m.word(m.word(scratch) + 24), crate::rom_region::addr(self.rom, 0x19592c), scratch + 4])
                    } else { rc };
                    if rc < 0x80000000 {
                        let entry = m.word(scratch + 4);
                        m.put_word(a[1] + 0x19c, m.word(entry + 4));
                        m.put_word(a[1] + 0x1a0, m.word(entry + 8));
                        m.put_word(a[1] + 0x1a4, m.word(entry + 12));
                    }
                    m.put_word(scratch, saved[0]);m.put_word(scratch + 4, saved[1]);
                    if rc >= 0x80000000 { return rc; }
                }
                let result=if let Some(runtime)=self.runtime.as_deref_mut(){runtime.invoke_m(m,f,a)}else{Err(engine::Error::UnsupportedStage{entry:f})};
                match result{Ok(rc)=>rc,Err(e)=>{self.boot.fault=Some(e);0x80002001}}
            }
            0xc72018=>{

                if self.runtime.as_deref().is_none_or(|r|r.guest_capture()) {
                let slot=crate::rom_region::capture_slot(self.rom);
                let env=crate::speech_part_4::CaptureRuntime{slot:m.word(slot),fill_flag:m.byte(crate::rom_region::addr(self.rom,0xa7b8e8))as u32,append_result:m.word(slot+4)};
                crate::speech_part_4::fn_c72018(m,self,env,a[0],a[1]);
                }
                if self.boot.fault.is_some(){return 0x80002001;}
                let result=if let Some(runtime)=self.runtime.as_deref_mut(){runtime.invoke_m(m,f,a)}else{Err(engine::Error::UnsupportedStage{entry:f})};
                match result{Ok(rc)=>rc,Err(e)=>{self.boot.fault=Some(e);0x80002001}}
            }
            0x230e1c => {
                let e=speech_environment(m,self.rom);
                let t=crate::speech_part_2::XcoderLiterals{input_key:crate::rom_region::addr(self.rom,0x2311b8),native_key:crate::rom_region::addr(self.rom,0x2311e8),utf16:crate::rom_region::addr(self.rom,0x231244),sync:crate::rom_region::addr(self.rom,0x2312cc),category:crate::rom_region::addr(self.rom,0x8fa93c),format:crate::rom_region::addr(self.rom,0x231238),content_type:crate::rom_region::addr(self.rom,0x23122c)};
                let classes=core::array::from_fn(|i| m.half(crate::rom_region::addr(self.rom,0x8dc30c)+i as u32*2));
                crate::speech_part_2::fn_230e1c(m,a[0],a[1],[a[2],a[3]],&e,&t,&classes,0x800,self)
            }
            0x22d460 => open_audio_m(&mut AudioHost{h:self,p:a[0],index:0},m.bytes,(a[0]!=0).then_some(a[0]),a[1],a[2],a[3]),
            0x22d6dc => pump_insertion_at_m(&mut BufferHost{h:self,p:a[0]},m.bytes,a[0],a[1],a[4]),
            0x227d64 => {
                let mut args=[0;7];args[..a.len()].copy_from_slice(a);
                match selection_root_m(m,args,self.boot.platform,self.rom){Ok(rc)=>rc,Err(e)=>{self.boot.fault=Some(e);0x80002001}}
            }
            0x1e57d0 => {
                let env=stream_environment(m,self.rom);
                let n=m.bytes.len().min(0x10000);
                let clean=m.bytes[..n].iter().all(|b|*b==0);
                let (rc,fault);
                {
                    let mut lh=crate::component_live_b::LiveHost{platform:&mut *self.boot.platform,fault:None,scratch:0x500,rom:self.rom};
                    rc=crate::component_live_b::fn_1e57d0_m(m,[a[0],a[1],a[2],a[3]],&env,0x600,&mut lh);
                    fault=lh.fault;
                }
                if clean{m.bytes[..n].fill(0);}
                if let Some(e)=fault{self.boot.fault=Some(e);}
                rc
            }
            0x1d135c => {
                let env=language_environment(m,self.rom);
                let n=m.bytes.len().min(0x10000);
                let clean=m.bytes[..n].iter().all(|b|*b==0);
                let (rc,fault);
                {
                    let mut lh=crate::component_live_b::LiveHost{platform:&mut *self.boot.platform,fault:None,scratch:0x500,rom:self.rom};
                    rc=crate::component_live_b::fn_1d135c_m(m,[a[0],a[1],a[2],a[3]],&env,0x600,&mut lh);
                    fault=lh.fault;
                }
                if clean{m.bytes[..n].fill(0);}
                if let Some(e)=fault{self.boot.fault=Some(e);}
                rc
            }
            0x1f3ea0 => {
                let n=m.bytes.len().min(0x10000);
                let clean=m.bytes[..n].iter().all(|b|*b==0);
                let (rc,fault);
                {
                    let mut lh=crate::component_live_b::LiveHost{platform:&mut *self.boot.platform,fault:None,scratch:0x500,rom:self.rom};
                    rc=crate::component_live_b::fn_1f3ea0_m(m,[a[0],a[1],a[2],a[3],a[4]],0xd00,&mut lh);
                    fault=lh.fault;
                }
                if clean{m.bytes[..n].fill(0);}
                if let Some(e)=fault{self.boot.fault=Some(e);}
                rc
            }
            0x202f9c => {

                let r={let p=&mut *self.boot.platform;let rom=self.rom;fn_202f9c_m(m,a[0],a[1],a[2],a[3],p,rom)};
                match r{Ok(rc)=>rc,Err(e)=>{self.boot.fault=Some(e);0x80002001}}
            }
            0x1f4724 => {
                let r={let p=&mut *self.boot.platform;let rom=self.rom;fn_1f4724_m(m,a[0],a[1],a[2],a[3],p,rom)};
                match r{Ok(rc)=>rc,Err(e)=>{self.boot.fault=Some(e);0x80002001}}
            }
            0x1ef464 => {
                let e=speech_types_environment(m,self.rom);
                crate::speech_part_2::fn_1ef464(m,a[0],a[1],[a[2],a[3]],&e,self)
            }
            0x219c8c => {
                let e=spelling_environment(m,self.rom);
                crate::component_live_c::fn_219c8c_m(m,a[0],a[1],[a[2],a[3]],&e,self)
            }
            0x1d513c => {
                let e=component_environment(m,self.rom);
                crate::speech_part_1::fn_1d513c(m,a[0],a[1],[a[2],a[3]],&e,self)
            }
            0x200e2c => crate::component_live_c::fn_200e2c_m(m,a[0],a[1],0x1e80,self),
            0x1970a0 => crate::normalization::idx5(m.bytes,a[0]),
            0x163a98 => callback_m(m,a[..5].try_into().unwrap(),0x1c00,0x989680,self),
            0x132124 => callback_m(m,[a[4],a[0],a[1],a[2],a[3]],0x1c00,0x989680,self),
            0x22c5f0 => {

                let p=crate::prosody_rules::InserterParameters{
                    input_active:m.word(a[0]+0x10),output_active:m.word(a[0]+0x18),..Default::default()};
                let mut valid=1u8;
                let rc=p.validate(terminated(m.bytes,a[1]),terminated(m.bytes,a[2]),&mut valid);
                m.put_byte(a[3],valid);rc
            }
            0x22c6dc => {

                let mut p=crate::prosody_rules::InserterParameters{
                    input_active:m.word(a[0]+0x10),output_active:m.word(a[0]+0x18),
                    frequency:m.word(a[0]+0x3c),buffer_samples:m.word(a[0]+0x40),
                    volume:m.half(a[0]+0x60),rate:m.half(a[0]+0x62),
                    adjust_volume:m.byte(a[0]+0x64),adjust_rate:m.byte(a[0]+0x65)};
                let rc=p.set(terminated(m.bytes,a[1]),terminated(m.bytes,a[2]));
                if p.frequency!=m.word(a[0]+0x3c) {m.put_word(a[0]+0x3c,p.frequency);}
                if p.buffer_samples!=m.word(a[0]+0x40) {m.put_word(a[0]+0x40,p.buffer_samples);}
                if p.volume!=m.half(a[0]+0x60) {m.put_half(a[0]+0x60,p.volume);}
                if p.rate!=m.half(a[0]+0x62) {m.put_half(a[0]+0x62,p.rate);}
                if p.adjust_volume!=m.byte(a[0]+0x64) {m.put_byte(a[0]+0x64,p.adjust_volume);}
                if p.adjust_rate!=m.byte(a[0]+0x65) {m.put_byte(a[0]+0x65,p.adjust_rate);}
                rc
            }
            0x2289b0 => {

                let mut valid=0u8;
                let rc=crate::frontend_output::validate_parameter(terminated(m.bytes,a[1]),terminated(m.bytes,a[2]),&mut valid);
                m.put_byte(a[3],valid);rc
            }
            0x151840 => {

                let table=core::array::from_fn(|i| m.half(crate::rom_region::addr(self.rom,0x8dc30c)+i as u32*2));
                let pair=core::array::from_fn(|i| m.byte(crate::rom_region::addr(self.rom,0x8d7660)+i as u32));
                let mut valid=0u8;
                let rc=crate::frontend::validate_param(m.bytes,&table,(a[0]!=0).then_some(a[0]),
                    terminated(m.bytes,a[1]),(a[2]!=0).then_some(a[2]),&mut valid,&pair,crate::rom_region::addr(self.rom,0x8d7640));
                m.put_byte(a[3],valid);rc
            }
            0x151df8=>{

                let p=a[0];let name=a[1];let value=a[2];
                if value==0||m.byte(value)==0||p==0 {return 0;}
                let saved=m.word(OUT_CTX);
                m.put_word(OUT_CTX,0);
                let rc=self.call(m,0x19611c,&[m.word(p),m.word(p+4),OUT_CTX]);
                if rc<0x80000000 {
                    let ctx=m.word(OUT_CTX);
                    let logger=m.word(ctx.wrapping_add(16));
                    let values=[name,value];
                    self.log_m(logger,0x8d7994,4,0x151f0c,&values,m);
                    if self.call(m,0x195d58,&[name,crate::rom_region::addr(self.rom,0x151f24)])==0 {
                        m.put_word(p+0x38,crate::normalization::atoi_scan(m.bytes,value));
                    } else if self.call(m,0x195d58,&[name,crate::rom_region::addr(self.rom,0x151f3c)])==0 {
                        m.put_word(p+0x34,crate::normalization::atoi_scan(m.bytes,value));
                    } else if self.call(m,0x195d58,&[name,crate::rom_region::addr(self.rom,0x151f48)])==0 {
                        m.put_word(p+0xe8,crate::normalization::atoi_scan(m.bytes,value));
                    } else if self.call(m,0x195d58,&[name,crate::rom_region::addr(self.rom,0x151f60)])==0 {

                        self.boot.fault=Some(engine::Error::MissingStreamAdapter{entry:0x15af9c});
                        m.put_word(OUT_CTX,saved);
                        return 0x80002001;
                    }
                }
                m.put_word(OUT_CTX,saved);
                rc
            }
            0x228aac=>{

                let p=a[0];let name=a[1];let value=a[2];
                if self.call(m,0x195d58,&[name,crate::rom_region::addr(self.rom,0x151f48)])!=0 {
                    self.boot.fault=Some(engine::Error::MissingStreamAdapter{entry:0x2a9608});
                    return 0x80002001;
                }
                let table=core::array::from_fn(|i|m.half(crate::rom_region::addr(self.rom,0x8dc30c)+i as u32*2));
                let parsed=crate::frontend::parse_int(&table,m.bytes,value) as u32;
                let frame=m.word(p+0x18);
                let mut buffer=if parsed>1 {parsed}else{1};
                if frame!=0 {
                    let remainder=parsed%frame;
                    if remainder!=0&&buffer>frame {buffer=buffer.wrapping_sub(remainder);}
                }
                if buffer>frame {m.put_word(p+0x14,buffer);} else {m.put_word(p+0x14,frame);}
                0
            }
            0x18f490=>{

                match fn_18f490_m(m,a[0],a[1],a[2],a[3]) {
                    Ok(rc)=>rc,
                    Err(e)=>{self.boot.fault=Some(e);0x80002001}
                }
            }
            0x183398 => {if finish_checked_m(m,a[0],a[1],a[2],0x1c40,self){0}else{0x80002001}}
            0x84c160 => fn_84c160_m(m,a[0],a[1],self),
            0x11da30 => {let rom=self.rom;fn_11da30_m(m,a[0],self,rom);0},
            0x1959dc => {

                if a.len() < 2 {self.boot.fault=Some(engine::Error::InvalidSnapshot);0x80002001}
                else {fn_1959dc_m(m,a[0],a[1],self)}
            },
            0x196fbc|0x174b14|0x19665c|0x19869c|0x198fb4|0x196428|0x196964 => {

                if a.is_empty() {self.boot.fault=Some(engine::Error::InvalidSnapshot);0x80002001}
                else {
                    match f {
                        0x196fbc=>fn_196fbc_m(m,a[0],*a.get(1).unwrap_or(&0),self),
                        0x174b14=>{fn_174b14_m(m,a[0],self);0},
                        0x19665c=>fn_19665c_m(m,a[0],self),
                        0x19869c=>{fn_19869c_m(m,a[0],self);0},
                        0x198fb4=>{if a.len()<3 {self.boot.fault=Some(engine::Error::InvalidSnapshot);0x80002001} else {fn_198fb4_m(m,a[0],a[1],a[2],self);0}},
                        0x196964=>{fn_196428_m(m,*a.get(1).unwrap_or(&0),a[0],self);0},
                        _=>{fn_196428_m(m,a[0],*a.get(1).unwrap_or(&0),self);0},
                    }
                }
            },
            0x129778 => {

                if a.is_empty() {self.boot.fault=Some(engine::Error::InvalidSnapshot);0x80002001}
                else {
                {let obj=m.word(crate::rom_region::addr(self.rom,0xb6f4ec));let mut b=m.word(obj.wrapping_add(0x3c));let mut k=0;while k<5{if b==0{break;}b=m.word(b.wrapping_add(12));k+=1;}}
                crate::speech_part_4::fn_129778(m,self,a[0],m.byte(crate::rom_region::addr(self.rom,0xa7b8e8))as u32)}
            },
            0x18f6b8 => {

                if a.len() >= 6 && m.word(a[0].wrapping_add(0x160)) == 0 {
                    seg7_flush_arm(self, m, f, a)
                }
                else if a.len() < 7 {self.boot.fault=Some(engine::Error::InvalidSnapshot);0x80002001}
                else if self.runtime.is_none() {self.boot.fault=Some(engine::Error::UnsupportedStage{entry:f});0x80002001}
                else {
                    let rt = self.runtime.as_deref_mut().unwrap();
                    let mut hooks = crate::engine_live_back::Hooks{platform:self.boot.platform,runtime:rt,snap:self.snap,fault:None,rom:self.rom};
                    let rc = crate::engine_live_back::fn_18f6b8_m(m,a[0],a[1],a[2],a[3],a[4],a[5],a[6],&mut hooks);
                    if let Some(e) = hooks.fault {self.boot.fault=Some(e);0x80002001}else{rc}
                }
            }
            0x1fe094 => clear_text_items_m(m,a[0],a[1],0x1c80,self),
            0x22b518|0x22a0a8|0x22b3bc|0x229de4|0x14f010|0x14f918|0x151610|0x1516c8|0x151744|0x196830|0x19690c|0x14df44|0x1749ac|0x174a04|0x15017c|0x150878|0x150c0c|0x174b38|0x175170|0x19668c|0x1966ac|0x176054|0x176104|0x175850|0x196980 => {

                if f == 0x14f010 {

                    let mut si = 0; while si < self.stack_len {  si += 1; }
                }
                if f == 0x22b3bc {

                }
                match component_a2_m(m,f,a,self.boot.platform,self.rom){Ok(rc)=>rc,Err(e)=>{self.boot.fault=Some(e);0x80002001}}
            }
            0x15140c|0x14fe9c|0x151294 => {
                let mut regions=[engine::Region{base:m.base,bytes:m.bytes}];
                let mut sparse=engine::Memory::new(&mut regions).expect("current arena");
                match crate::engine_components::InternalCalls::call(&mut engine::LiveComponentCalls,&mut sparse,f,a) {
                    Ok(rc)=>rc,Err(e)=>{self.boot.fault=Some(e);0x80002001}
                }
            }
            0x2312fc|0x231ba0 => {
                let mut args=[0;7];args[..a.len()].copy_from_slice(a);let call=engine::StageCall{entry:f,args};
                let mut regions=[engine::Region{base:m.base,bytes:m.bytes}];let mut sparse=engine::Memory::new(&mut regions).expect("current arena");
                let result=if f==0x2312fc {let mut scratch=[0u8;8192];engine::xcoder_text_stage(&mut sparse,call,&mut scratch)}else{engine::xcoder_markers_stage(&mut sparse,call)};
                match result{Ok(r)=>r.r0.expect("XCODER result"),Err(e)=>{self.boot.fault=Some(e);0x80002001}}
            }
            0x15daa8 => {
                let valid=validate_handle(a[0]!=0,a[1],0xb2c2,56)<0x80000000;
                if valid {let mut out=0;let rc=crate::linguistic::dirty(true,m.byte(a[0]+52),&mut out);m.put_byte(a[2],out);rc}
                else {crate::linguistic::dirty(false,0,&mut 0)}
            }
            0x15dad8 => {
                let valid=validate_handle(a[0]!=0,a[1],0xb2c2,56)<0x80000000;
                if valid {let mut out=m.byte(a[0]+52);let rc=crate::linguistic::clear_dirty(true,&mut out);m.put_byte(a[0]+52,out);rc}
                else {crate::linguistic::clear_dirty(false,&mut 0)}
            }
            0x15429c => {
                let mut activity=[0u8;256];let n=m.half(m.word(a[0]+0x7c)+12) as usize;
                if n>activity.len(){self.boot.fault=Some(engine::Error::Capacity{required:n,available:activity.len()});return 0x80002001;}
                process_text_m(m,a[0],a[1],&mut activity,0x1e00,self)
            }
            0x1e4a3c|0x1f48cc|0x1503b0=>{
                match component_b_m(m,f,a,self.boot.platform){Ok(rc)=>rc,Err(e)=>{self.boot.fault=Some(e);0x80002001}}
            }
            0x18cae8=>{let mut regions=[engine::Region{base:m.base,bytes:m.bytes}];let mut sparse=engine::Memory::new(&mut regions).expect("current arena");match engine::reset_synthesis_live(&mut sparse,a[0],self.rom){Ok(())=>0,Err(e)=>{self.boot.fault=Some(e);0x80002001}}},
            0x7f6820=>crate::speech_part_3::fn_7f6820(m,a[0]),
            0x18cc30=>crate::boot_part_b::fn_18cc30(m,a[0],self),
            0x18ac58=>crate::boot_part_b2::fn_18ac58(m,crate::rom_region::addr(self.rom,0xa7bf6c),crate::rom_region::addr(self.rom,0xb84078),self),
            0x1858e4=>crate::boot_part_b2::fn_1858e4(m,crate::rom_region::addr(self.rom,0xa7c90c),crate::rom_region::addr(self.rom,0xb9be08)),
            0x1840e8=>{crate::boot_part_b::fn_1840e8(m,self);0},
            0x185d40=>{crate::boot_part_b::fn_185d40(m,self);0},
            0x1b5b8c=>{notify_output_m(m,a[0],a[1],self);0},
            0x1a1b68=>{match publish_output_m(m,a[0],self.rom,self){Ok(())=>0,Err(e)=>{self.boot.fault=Some(e);0}}},
            0x19703c=>{
                if a[0]==0 || m.word(a[0]+12)==0 || m.word(a[0]+20)==0 {crate::normalization::run_dtors(m.bytes,a[0])}
                else {self.boot.fault=Some(engine::Error::MissingStreamAdapter{entry:f});0x80002001}
            }
            0x19869c=>{clear_tree_m(m,a[0],self);0}

            0x14e0b4=>{let mut h=ReleaseCalls(self);crate::component_live_a2::release_stream_state_m(m,a[0],a.get(1).copied().unwrap_or(0),&mut h);0},
            0x14dfc0=>{let p=a[0];let hp=a.get(1).copied().unwrap_or(0);if p!=0{let h=m.word(p);if h!=0{self.call(m,0x196428,&[hp,h]);}self.call(m,0x196428,&[hp,p]);}0},
            0x196c7c=>crate::reentrant2::destroy_array_m(m,a[0],self),
            0x198fb4=>{destroy_subtree_m(m,a[0],a[1],a[2],self);0}
            0x196770=>crate::frontend::clear_iterator((a[0]!=0).then_some(a[0]),&mut |tree|{self.call(m,0x19869c,&[tree]);}),
            0x230274=>pipeline_switch_m(&mut SwitchHost(self),m.bytes,a[0],a[1]),
            0x1757a8=>crate::frontend::check_handle(a[0]!=0),
            0x175944=>crate::db_live::get_uint(self,m,a[0],a[1],a[2]),
            0x150fbc=>{let s=0x1e40;let saved=[m.word(s),m.word(s+4)];
                let rc=write_copy_m(m.bytes,(a[0]!=0).then_some(a[0]),a[1],(a[2]!=0).then_some(a[2]),a[3],a[4],a[5],s,s+4,self);m.put_word(s,saved[0]);m.put_word(s+4,saved[1]);rc}
            0x2581ac => fn_2581ac_m(m,a[0],self),
            0x2ba6b4 => fn_2ba6b4_m(m,a[0],self),
            0x23ffd4 => pcre_deinit_m(m,a[0],a[1],0x1d00,self),
            0x264578 => fn_264578(m,a[0],self),
            0x1e5af0 => crate::speech_part_2::fn_1e5af0(m,a[0],a[1],self),

            0x205e58 => crate::speech_part_2::fn_205e58(m,a[0],a[1],self),
            0x1f1138 => crate::speech_part_2::fn_1f1138(m,a[0],a[1],self),
            0x21cd54 => crate::speech_part_2::fn_21cd54(m,a[0],a[1],self),
            0x1d7544 => crate::speech_part_1::fn_1d7544(m,a[0],a[1],self),

            0x1f4cb0 => fn_1f4cb0_m(m,a[0],a[1],self),

            0x1fd6d0 => fn_1fd6d0_m(m,a[0],a[1],self),

            0x1f27ec => fn_1f27ec_m(m,a[0],a[1],self),
            0x1f3a8c => fn_1f3a8c_m(m,a[0],a[1],self),
            0x1e03ac => fn_1e03ac_m(m,a[0],a[1],self),
            0x1ea328 => fn_1ea328_m(m,a[0],a[1],self),
            0x1f7e8c => fn_1f7e8c_m(m,a[0],a[1],self),

            0x1f43f0 => fn_1f43f0_m(m,a[0],a[1],self),

            0x211348 => fn_211348_m(m,a[0],a[1],self),

            0x1e3e28 => {
                let e=component_environment(m,self.rom);
                crate::speech_part_1::fn_1e3e28(m,a[0],a[1],&e,self)
            },

            0x2276dc => {
                let e=speech_environment(m,self.rom);
                crate::speech_part_2::fn_2276dc(m,a[0],a[1],&e,self)
            },

            0x229cc0 => fn_229cc0_m(m,a[0],a[1],self),

            0x19672c => fn_19672c_m(m,a[0],a[1],self),

            0x23220c => {
                let e=speech_environment(m,self.rom);
                crate::speech_part_2::fn_23220c(m,a[0],a[1],&e,self)
            },

            0x207130 => fn_207130_m(m,a[0],a[1],self),

            0x1de844 => fn_1de844_m(m,a[0],a[1],self),

            0x1d453c => crate::speech_part_1::fn_1d453c(m,a[0],a[1],0x1d170c,self),

            0x2318f0 => {
                let e=speech_environment(m,self.rom);
                crate::speech_part_2::fn_2318f0(m,a[0],a[1],&e,self)
            },

            0x22b238 => fn_22b238_m(m,a[0],a[1],self),

            0x2fdc84 => {
                if a.len() < 4 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::speech_part_5::fn_2fdc84_m(m,a[0],a[1],a[2],a[3],self)
            },
            0x1e502c => end_token_processing_m(m,a[0],a[1],self),
            0x201264 => {if validate_handle(a[0]!=0,a[1],0xf383,0x20)>=0x80000000 {return 0x8bb02008;}let entry=m.word(m.word(a[0]+8)+32);call_method8_m(&mut MethodHost{h:self,entry},m.bytes,a[0],a[1])}
            0x22e084 => reset_ctx_m(&mut ResetHost{h:self,p:a[0]},m.bytes,(a[0]!=0).then_some(a[0]),a[1]),
            0x175a1c => fetch_param_m(self,m,a[0],a[1],a[2]),
            0x1758ac=>{

                let s=OUT_PARAM;
                let at=m.offset(s);
                let mut saved=[0u8;36];
                saved.copy_from_slice(&m.bytes[at..at+36]);
                crate::boot_port_1::fn_176c58(m,a[2],s,10);
                let n=crate::boot_port_1::fn_195d90(m,s).wrapping_add(1);
                let rc=crate::speech_part_1::fn_175170(m,a[0],a[1],s,n,0,s+40,self);
                m.bytes[at..at+36].copy_from_slice(&saved);
                rc
            }
            0x1deb04 => crate::engine_live2::fn_1deb04_m(m,a[0],a[1]),
            0x1e772c => crate::engine_live2::fn_1e772c_m(m,a[0],a[1]),
            0x1f3dc0 => crate::engine_live2::fn_1f3dc0_m(m,a[0],a[1],[a[2],a[3]],self.rom,self),
            0x1f51dc => crate::engine_live2::fn_1f51dc_m(m,a[0],a[1]),
            0x210398 => crate::engine_live2::fn_210398_m(m,a[0],a[1],a[2],a[3],self.rom,self),
            0x2279b4 => crate::engine_live2::fn_2279b4_m(m,a[0],a[1],[a[2],a[3]],0x5000,self,self.rom),
            0x1e3938 => crate::engine_live2::fn_1e3938_m(m,a[0],a[1],[a[2],a[3]],0x5100,self,self.rom),
            0x2319f0 => crate::engine_live2::fn_2319f0_m(m,a[0],a[1],[a[2],a[3]],0x5200,self,self.rom),
            0x1e5bbc => {

                let mut args=[0u32;5];let n=a.len().min(5);args[..n].copy_from_slice(&a[..n]);
                match crate::component_live_b::fn_1e5bbc_m(m,args,0x5300,self) {
                    Ok(rc)=>rc,
                    Err(e)=>{self.boot.fault=Some(e);0x80002001}
                }
            }
            0x1d3314 => {

                let mut args = [0; 5];
                args[..a.len().min(5)].copy_from_slice(a);

                let s = PORT_FRAME_1D3314;
                let mut bridge = ParamBridge { host: self };
                let rc = {
                    let mut regions = [engine::Region { base: m.base, bytes: m.bytes }];
                    let mut sparse = engine::Memory::new(&mut regions).expect("current arena");
                    crate::engine_components2::process(
                        &mut sparse,
                        f,
                        args,
                        s,
                        &mut bridge,
                    )
                };
                match rc {
                    Ok(rc) => rc,
                    Err(e) => {
                        bridge.host.boot.fault = Some(e);
                        0x80002001
                    }
                }
            }
            0x1795f0 => {

                let broker = a.first().copied().unwrap_or(0);
                let name = a.get(1).copied().unwrap_or(0);
                let flags = a.get(2).copied().unwrap_or(0);
                let out = a.get(3).copied().unwrap_or(0);
                let globals = crate::boot_port_2c::BrokerGlobals {
                    module: crate::rom_region::addr(self.rom,0x8d7668),
                    brokerstring_key: crate::rom_region::addr(self.rom,0x177264),
                    add_item_format: crate::rom_region::addr(self.rom,0x177278),
                    broker_label: crate::rom_region::addr(self.rom,0x17729c),
                    fullpath_key: crate::rom_region::addr(self.rom,0x17823c),
                    ctype_table: crate::rom_region::addr(self.rom,0x8dc30c),
                    parameters_desc: crate::rom_region::addr(self.rom,0x8d7614),
                    linguistic_desc: crate::rom_region::addr(self.rom,0x8d773c),
                }.data_open(BROKER_DATA_SCRATCH);
                let mut files = crate::engine_components2::live_fs::FilePlatform {
                    env: crate::speech_part_1::ArchiveEnvironment {
                        dot: crate::rom_region::addr(self.rom,0x8cd208),
                        parent: crate::rom_region::addr(self.rom,0x8cd20c),
                        locale: 0,
                    },
                    scratch: BROKER_FILE_SCRATCH,
                    rb: crate::engine_components2::rb_literal(self.rom),
                };
                let rom = self.rom;
                let mut host = BrokerDataHost {
                    host: self,
                    rom,
                    lit_cursor: BROKER_LIT_BASE,
                };
                crate::engine_components2::fn_1795f0_live(
                    m, broker, name, flags, out, &globals, &mut files, &mut host,
                )
            }
            0x1ecf68 => {

                let p = a.first().copied().unwrap_or(0);
                let a1 = a.get(1).copied().unwrap_or(0);
                let a2 = a.get(2).copied().unwrap_or(0);
                let a3 = a.get(3).copied().unwrap_or(0);
                let out4 = a.get(4).copied().unwrap_or(0);
                let Some(s) = a.get(5).copied() else {
                    self.boot.fault = Some(engine::Error::UnsupportedStage { entry: 0x1ecf68 });
                    return 0x80002001;
                };
                if crate::normalization::quad_check(p, a1, 0xf37f, 0x2a4) >= 0x8000_0000 {
                    return 0x8980_2008;
                }
                if out4 != 0 {
                    m.put_word(out4, 0);
                }
                let env = m.word(p);
                let logger = m.word(env + 16);
                if a2 == 0 {
                    self.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa029, 0]);
                    return 0x8980_2007;
                }
                {
                    let text = b"Entering fe_dctlkp_QueryLidDictExists : %s?\0";
                    let at = m.offset(s + 16);
                    m.bytes[at..at + text.len()].copy_from_slice(text);
                }
                self.call(m, 0x1738e8, &[logger, 0x8f8f44, 4, 0, s + 16, a2]);
                let broker = m.word(env + 12);
                let rc = if a3 != 0 {
                    let at = m.offset(s + 8);
                    m.bytes[at..at + 4].copy_from_slice(b"lid\0");
                    self.call(m, 0x1795f0, &[broker, s + 8, 1, s])
                } else {

                    self.call(m, 0x179f30, &[broker, env.wrapping_add(0x9e), s])
                };
                if rc >= 0x8000_0000 {
                    return 0;
                }
                let handle = m.word(s);
                let base_off = if a3 != 0 { 0x48 } else { 8 };
                let size_off = if a3 != 0 { 0x44 } else { 4 };
                let rc = self.call(
                    m,
                    0x26b718,
                    &[p + base_off, a2, m.word(p + size_off), handle, out4],
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                self.call(m, 0x179f3c, &[broker, handle])
            }
            0x26b718 => {

                let mut inner = crate::component_live_c2::OpenHost {
                    query: BROKER_QUERY_SCRATCH,
                    pending_release: None,
                };
                let mut host = crate::component_live_c2::LidHost { inner: &mut inner };
                let found = a.get(4).copied().unwrap_or(0);
                crate::speech_part_2::fn_26b718(
                    m, a[0], a[1], a[2], found, BROKER_LID_SCRATCH, &mut host,
                )
            }
            0x179f30 => {

                let broker = a.first().copied().unwrap_or(0);
                let name = a.get(1).copied().unwrap_or(0);
                let out = a.get(2).copied().unwrap_or(0);
                let globals = crate::boot_port_2c::BrokerGlobals {
                    module: crate::rom_region::addr(self.rom,0x8d7668),
                    brokerstring_key: crate::rom_region::addr(self.rom,0x177264),
                    add_item_format: crate::rom_region::addr(self.rom,0x177278),
                    broker_label: crate::rom_region::addr(self.rom,0x17729c),
                    fullpath_key: crate::rom_region::addr(self.rom,0x17823c),
                    ctype_table: crate::rom_region::addr(self.rom,0x8dc30c),
                    parameters_desc: crate::rom_region::addr(self.rom,0x8d7614),
                    linguistic_desc: crate::rom_region::addr(self.rom,0x8d773c),
                }.data_open(BROKER_DATA_SCRATCH);
                let mut inner = crate::component_live_c2::OpenHost {
                    query: BROKER_QUERY_SCRATCH,
                    pending_release: None,
                };
                crate::leftovers_1::fn_179f30(m, broker, name, out, &globals, &mut inner)
            }
            0x179f3c => {

                let mut inner = crate::component_live_c2::OpenHost {
                    query: BROKER_QUERY_SCRATCH,
                    pending_release: None,
                };
                let mut host = BrokerCloseDirect { host: self, inner: &mut inner };
                crate::leftovers_1::fn_179f3c_m(m, a[0], a[1], &close_runtime_us(), &mut host)
            }
            0x14e440 => {
                fn_14e440_m(m, a[0], a[1], self);
                0
            }
            0x2031b0 => {

                let mut args = [0; 5];
                args[..a.len().min(5)].copy_from_slice(&a[..a.len().min(5)]);

                let mut bridge = DepesBridge { host: self };
                let rc = crate::component_live_c2::fn_2031b0_m(
                    m, DEPES_FRAME, args[0], args[1], args[4], &mut bridge,
                );
                if bridge.host.boot.fault.is_some() {
                    return 0x80002001;
                }
                rc
            }
            0x176f94 => crate::boot_port_1::fn_176f94(m, a[0], a[1], a[2], a[3]),

            0x176fd4 => {
                if a.is_empty() {
                    self.boot.fault = Some(engine::Error::InvalidSnapshot);
                    0x80002001
                } else {
                    crate::boot_part_d::fn_176fd4(
                        m,
                        a[0],
                        [m.word(crate::rom_region::addr(self.rom,0x8d7660)), m.word(crate::rom_region::addr(self.rom,0x8d7664))],
                    );
                    0
                }
            }
            0x17486c => {

                crate::boot_port_1::fn_17486c(m, a[0], a[1], a[2], a[3], DEPES_LOG_SCRATCH, self)
            }
            0x17497c => crate::component_live_c2::fn_17497c(m, a[0]) as u32,
            0x2fbe50 => {

                crate::speech_part_3::fn_2fbe50_m()
            }

            0x17c8c0 => crate::fe_text::fold_17c8c0(a[0]),
            0x1960a8 => crate::frontend::find_substr(m.bytes, a[0], a[1]),
            0x2b331c => {

                m.put_word(a[1], m.word(a[0].wrapping_add(0x44)));
                0
            }
            0x2b3608 => {

                let mut bridge = DictBridge { host: self };
                let rc = crate::speech_part_3::fn_2b3608(m, a[0], a[1], a[2], a[3], 0x7a00, &mut bridge);
                if bridge.host.boot.fault.is_some() {
                    return 0x80002001;
                }
                rc
            }
            0x300a10 => {

                if a.len() < 3 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let mut bridge = DictBridge { host: self };
                crate::speech_part_5::fn_300a10(m, a[0], a[1], a[2], &mut bridge);
                if bridge.host.boot.fault.is_some() {
                    return 0x80002001;
                }
                0
            }
            0x17bfc0 => {

                if a.is_empty() {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::component_live_c2::fn_17bfc0_m(m, a[0])
            }
            0x17bd74 => {

                if a.len() < 2 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::component_live_c2::fn_17bd74_m(m, a[0], a[1])
            }
            0x17bb44 => {

                if a.len() < 7 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let extra: [u8; 256] = core::array::from_fn(|i| m.byte(crate::rom_region::addr(self.rom, 0x8dc1f3) + i as u32));
                let mask: [u8; 16] = core::array::from_fn(|i| m.byte(crate::rom_region::addr(self.rom, 0x8dc1e0) + i as u32));
                let base: [u8; 1024] = core::array::from_fn(|i| m.byte(crate::rom_region::addr(self.rom, 0x8dc2f4) + i as u32));
                let t = crate::normalization::DecoderTables {
                    extra: &extra,
                    mask: &mask,
                    mask_base: 6,
                    base: &base,
                    base_base: 0,
                };
                crate::normalization::decode_units(
                    m.bytes, a[0], a[1], a[2], a[3], a[4], a[5], a[6], &t,
                )
            }
            0x2b4004 => {

                if a.len() < 2 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let mut bridge = DictBridge { host: self };
                let rc = crate::speech_part_5::fn_2b4004(m, a[0], a[1], &mut bridge);
                if bridge.host.boot.fault.is_some() {
                    return 0x80002001;
                }
                rc
            }
            0x173fd8 => crate::component_live_a::max_slot_level_m(m, a[0], self),
            0x1ce56c => crate::frontend::streaming_mode(
                (a[0] != 0).then(|| (m.byte(m.word(a[0]) + 4), m.byte(m.word(a[0]) + 0x10c))),
            ),
            0x18f488 => crate::frontend::finish_request(),

            0x219554 => crate::component_live_c::fn_219554_m(m, a[0], a[1], a[2], a[3]),
            0x1ea4e0 => {

                if a.len() < 7 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let rc = crate::component_live_b4::fn_1ea4e0_m(
                    m, self, a[0], a[1], a[2], a[3], a[4], a[5], a[6],
                );
                if self.boot.fault.is_some() {
                    return 0x80002001;
                }
                rc
            }

            0x17a1a4|0x179374|0x17c9d8|0x179208|0x1b2760|0x1b254c|0x1b2974|0x172de0|0x172d30 => {
                crate::component_live_c::broker_range_live_m(m, f, a, RANGE17_SCRATCH, self)
            }

            0x4a794c => {
                if a.len() < 4 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::speech_part_3::fn_4a794c(m, a[0], a[1], a[2], a[3])
            }
            0x4a78f0 => {
                if a.len() < 4 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::speech_part_3::fn_4a78f0(m, a[0], a[1], a[2], a[3])
            }

            0x26bb44 => {
                if a.is_empty() {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::component_live_b3::fn_26bb44_m(m, a[0])
            }

            0x1769d0 => {
                if a.len() < 2 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::fe_text::strcmp_ci_1769d0(m.bytes, a[0], a[1]) as u32
            }
            0x195d90 => {
                if a.is_empty() {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::fe_text::strlen_195d90(m.bytes, a[0]) as u32
            }

            0x196394 => {
                if a.len() < 2 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::boot_part_e::alloc(m, a[0], a[1], self)
            }

            0x1963b8 => {
                if a.len() < 3 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::boot_part_e::calloc(m, a[0], a[1], a[2], self)
            }

            0x1963e0 => {
                if a.len() < 3 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::boot_part_e::fn_1963e0(m, a[0], a[1], a[2], self)
            }

            0x26b7ec => {
                if a.len() < 4 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::component_live_b3::fn_26b7ec_m(m, self, a[0], a[1], a[2], a[3])
            }

            0x275204 => {
                if a.len() < 2 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::component_live_b3::fn_275204_m(
                    m,
                    self,
                    a[0],
                    a[1],
                    crate::component_live_b3::FE275204_SCRATCH,
                )
            }

            0x2409d4 => {
                if a.len() < 4 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::component_live_b3::fn_2409d4_m(m, self, a[0], a[1], a[2], a[3])
            }
            0x21a1fc => {
                if a.len() < 2 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::component_live_b3::fn_21a1fc_m(m, a[0], a[1])
            }
            0x1f0a90 => {

                let mut args = [0; 5];
                let n = a.len().min(5);
                args[..n].copy_from_slice(&a[..n]);

                let s = PORT_FRAME_1F0A90;
                let mut bridge = ParamBridge { host: self };
                let rc = {
                    let mut regions = [engine::Region { base: m.base, bytes: m.bytes }];
                    let mut sparse = engine::Memory::new(&mut regions).expect("current arena");
                    crate::engine_components::process(
                        &mut sparse,
                        f,
                        args,
                        s,
                        &mut bridge,
                    )
                };
                match rc {
                    Ok(rc) => rc,
                    Err(e) => {
                        bridge.host.boot.fault = Some(e);
                        0x80002001
                    }
                }
            }
            0x281aa4 => {

                let current = m.word(a[1]);
                let next = if current == 0 { m.word(a[0] + 4) } else { m.word(current + 8) };
                m.put_word(a[1], next);
                if next == 0 { 0 } else { m.word(next + 4) }
            }
            0x17bff4 => crate::component_live_c2::fn_17bff4_m(m, a[0]),
            0x17bf70 => crate::component_live_c2::fn_17bf70_m(m, a[0], a[1], self.rom),
            0x176314 => {
                crate::component_live_c2::fn_176314_m(m, a[0], a[1]);
                0
            }
            0x202348 => {

                if a.len() < 7 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                use crate::component_live_c2::{
                    C2_2B3608_SCRATCH, C2_DICT_SCRATCH, C2_LANGCODE_KEY, C2_MLSET_KEY,
                    C2_NUMBEROUTPUTTRANSCRIPTIONS,
                };
                let _ = (C2_DICT_SCRATCH, C2_2B3608_SCRATCH, C2_NUMBEROUTPUTTRANSCRIPTIONS);
                let mut tables = crate::component_live_c2::C2DictTables {
                    types: [[0u8; 48]; 3],
                    type_lens: [0; 3],
                    disabled: [0u8; 32],
                    disabled_len: 0,
                    classes: core::array::from_fn(|i| m.half(crate::rom_region::addr(self.rom,0x8dc30c) + i as u32 * 2)),
                };
                crate::component_live_c2::c2_dict_tables(m, &mut tables, self.rom);
                let t = crate::speech_part_2::DictionaryLiterals {
                    types: [
                        &tables.types[0][..tables.type_lens[0]],
                        &tables.types[1][..tables.type_lens[1]],
                        &tables.types[2][..tables.type_lens[2]],
                    ],
                    disabled_mode: &tables.disabled[..tables.disabled_len],
                    langcode: crate::rom_region::addr(self.rom, C2_LANGCODE_KEY),
                    mlset: crate::rom_region::addr(self.rom, C2_MLSET_KEY),
                    category: crate::rom_region::addr(self.rom, 0x8f9734),
                    invalid_format: crate::rom_region::addr(self.rom, 0x155ec0),
                    name: crate::rom_region::addr(self.rom, 0x14db07),
                    empty: crate::rom_region::addr(self.rom, 0x8f972c),
                    type_label: crate::rom_region::addr(self.rom, 0x201500),
                    trace_formats: [
                        crate::rom_region::addr(self.rom, 0x202778),
                        crate::rom_region::addr(self.rom, 0x2027a0),
                        crate::rom_region::addr(self.rom, 0x2027a5),
                        crate::rom_region::addr(self.rom, 0x1feedc),
                    ],
                    labels: [
                        crate::rom_region::addr(self.rom, 0x1fee78),
                        crate::rom_region::addr(self.rom, 0x14daf4),
                        crate::rom_region::addr(self.rom, 0x1cd7e8),
                        crate::rom_region::addr(self.rom, 0x1feed8),
                        crate::rom_region::addr(self.rom, 0x155ea0),
                        crate::rom_region::addr(self.rom, 0x1feed0),
                        crate::rom_region::addr(self.rom, 0x14da60),
                    ],
                };
                crate::speech_part_2::fn_202348(
                    m, a[0], a[1], a[2], a[3], a[4], a[5], a[6],
                    &tables.classes, &t, 0x7900, self,
                )
            }
            0x195ae8 => crate::frontend::move_bytes(&mut *m.bytes, a[0], a[1], a[2]),

            0x21a25c => {

                let mut args = [0; 5];
                let n = a.len().min(5);
                args[..n].copy_from_slice(&a[..n]);
                let s = PORT_FRAME_21A25C;
                let mut bridge = ParamBridge { host: self };
                let rc = {
                    let mut regions = [engine::Region { base: m.base, bytes: m.bytes }];
                    let mut sparse = engine::Memory::new(&mut regions).expect("current arena");
                    crate::engine_components2::process(
                        &mut sparse,
                        f,
                        args,
                        s,
                        &mut bridge,
                    )
                };
                match rc {
                    Ok(rc) => rc,
                    Err(e) => {
                        bridge.host.boot.fault = Some(e);
                        0x80002001
                    }
                }
            }
            0x1756d0 => {

                struct ParamLock<'a, 'b> { host: &'a mut Host<'b>, word: u32 }
                impl crate::reentrant2::StateLockM for ParamLock<'_, '_> {
                    fn acquire_m(&mut self, bytes: &mut [u8]) -> u32 {
                        let mut m = Memory { base: 0, bytes };
                        crate::speech_part_2::parameter_lock_m(&mut m, self.word, false, &mut *self.host)
                    }
                    fn release_m(&mut self, bytes: &mut [u8]) -> u32 {
                        let mut m = Memory { base: 0, bytes };
                        crate::speech_part_2::parameter_lock_m(&mut m, self.word, true, &mut *self.host)
                    }
                }
                fn param_compare(a: u32, b: u32, bytes: &mut [u8]) -> i32 {
                    let mut i = 0usize;
                    loop {
                        let x = bytes.get(a as usize + i).copied().unwrap_or(0);
                        let y = bytes.get(b as usize + i).copied().unwrap_or(0);
                        if x != y {
                            return if x < y { -1 } else { 1 };
                        }
                        if x == 0 {
                            return 0;
                        }
                        i += 1;
                    }
                }
                if a[0] != 0 && a[1] != 0 && a[2] != 0 {
                    m.put_word(a[2], 0);
                    if a[3] != 0 {
                        m.put_word(a[3], 0);
                    }
                }
                let p = if a[0] == 0 { 0 } else { m.word(a[0] + 4) };
                let mut value = if a[2] == 0 { 0 } else { m.word(a[2]) };
                let mut size = if a[3] == 0 { 0 } else { m.word(a[3]) };
                let mut lock = ParamLock { host: self, word: p };
                let rc = crate::reentrant2::lookup_param_m(
                    m.bytes,
                    (a[0] != 0).then_some(a[0]),
                    a[1],
                    (a[2] != 0).then_some(&mut value),
                    (a[3] != 0).then_some(&mut size),
                    &mut lock,
                    param_compare,
                );
                if a[2] != 0 {
                    m.put_word(a[2], value);
                }
                if a[3] != 0 {
                    m.put_word(a[3], size);
                }
                rc
            }
            0x21958c => {

                struct Validator { object: u32, key: u32 }
                impl crate::depes::HandleValidator for Validator {
                    fn validate(&mut self, _: u32, tag: u32, size: u32) -> u32 {
                        crate::normalization::quad_check(self.object, self.key, tag, size)
                    }
                }
                let rc = crate::normalization::quad_check(a[0], a[1], 0xf37e, 64);
                if rc >= 0x80000000 {
                    0x89702008
                } else {
                    let opts = crate::depes::ModuleOptions {
                        first: m.word(a[0] + 56) as i32,
                        second: m.word(a[0] + 60) as i32,
                    };
                    let mut out = 0;
                    let rc = opts.get(a[1], &mut out, a[3] as i32, &mut Validator { object: a[0], key: a[1] });
                    if rc < 0x80000000 {
                        m.put_word(a[2], out as u32);
                    }
                    rc
                }
            }
            0x206be4 => {

                if a.len() < 4 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let changed = if a.len() > 4 { a[4] } else { m.word(0xe0ff938) };
                m.bytes[SEG2_C4_SCRATCH as usize..SEG2_C4_SCRATCH as usize + SEG2_C4_LEN].fill(0);
                let mut plat = HostPlat(&mut *self.boot.platform);
                let mut regions = [engine::Region { base: m.base, bytes: m.bytes }];
                let mut sparse = match engine::Memory::new(&mut regions) {
                    Ok(v) => v,
                    Err(e) => { self.boot.fault = Some(e); return 0x80002001; }
                };
                let (rc, unbound) = crate::engine_live_seg2::run_206be4_rom(
                    &mut sparse, a[0], a[1], [a[2], a[3]], changed, SEG2_C4_SCRATCH, &mut plat, self.rom,
                );
                match rc {
                    Ok(rc) => {
                        if let Some(e) = unbound {
                            self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: e });
                            return 0x80002001;
                        }
                        rc
                    }
                    Err(e) => { self.boot.fault = Some(e); 0x80002001 }
                }
            }
            0x1dd914 => {

                if a.len() < 4 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let changed = if a.len() > 4 { a[4] } else { m.word(0xe0ff938) };
                let mut plat = HostPlat(&mut *self.boot.platform);
                let (rc, unbound) = crate::engine_live_seg2::run_1dd914_rom(m, a, changed, &mut plat, self.rom);
                if let Some(e) = unbound {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: e });
                    return 0x80002001;
                }
                rc
            }
            0x1f98c8 => seg3_oneword_arm(self, m, f, a),
            0x1f1440 | 0x1f3248 => seg3_pos_arm(self, m, f, a),
            0x1df780 => seg3_phrasing_arm(self, m, f, a),
            0x1e7908 => {

                if a.len() < 5 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let (rc, unbound) = crate::engine_live_seg4::run_1e7908_rom(m, a, &mut *self.boot.platform, self.rom);
                if let Some(e) = unbound {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: e });
                    return 0x80002001;
                }
                rc
            }
            0x1f51f8 => {

                if a.len() < 5 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let (rc, unbound) = crate::engine_live_seg4::run_1f51f8_rom(m, a, &mut *self.boot.platform, self.rom);
                if let Some(e) = unbound {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: e });
                    return 0x80002001;
                }
                rc
            }
            0x1e3b14 => seg7_marker_arm(self, m, f, a),

            0x2c1e2c => {
                if a.len() < 2 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let saved = m.word(0x500);
                let rc = crate::component_live_b::fn_2c1e2c_m(m, self, a[0], a[1], 0x500);
                m.put_word(0x500, saved);
                rc
            }
            0x1759a0 => {

                if a.len() < 3 || a[0] == 0 || a[1] == 0 || a[2] == 0 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let s = OUT_PARAM;
                let at = m.offset(s);
                let mut saved = [0u8; 36];
                saved.copy_from_slice(&m.bytes[at..at + 36]);
                let n = crate::boot_port_1::fn_195d90(m, a[2]).wrapping_add(1);
                let rc = crate::speech_part_1::fn_175170(m, a[0], a[1], a[2], n, 0, s, self);
                m.bytes[at..at + 36].copy_from_slice(&saved);
                rc
            }
            0x175a24 => {

                if a.len() < 2 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let saved = m.word(0x500);
                let rc = crate::component_live_b::fn_175a24_m(m, self, a[0], a[1], 0x500);
                m.put_word(0x500, saved);
                rc
            }
            0x1966ac => {

                return crate::speech_part_1::parameter_call_m(m, f, a, 0x1800, self);
            }
            0x1530d0 => seg7_pump_arm(self, m, f, a),
            0x84c160 => {

                if a.len() < 2 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::speech_part_3::fn_84c160(m, a[0], a[1], self)
            }
            0x2104b0 => {

                if a.len() < 5 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }

                let mut saved = [0u8; 65536];
                let mut events = [crate::frontend::TextEvent::default(); 512];
                let mut event_bytes = [0u8; 512 * 24];
                let mut text = [0u8; 65536];
                match crate::component_live_c::fn_2104b0_m(
                    m, a[0], a[1], [a[2], a[3]], a[4], 0xe000000,
                    &mut saved, &mut events, &mut event_bytes, &mut text, self,
                ) {
                    Ok(rc) => rc,
                    Err(e) => { self.boot.fault = Some(frontend_error(e)); 0x80002001 }
                }
            }
            0x2a762c => {

                if a.len() < 2 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let at = m.offset(SCRATCH_2A762C);
                let mut saved = [0u8; 24];
                saved.copy_from_slice(&m.bytes[at..at + 24]);
                let k = crate::boot_port_3b::DestructionEnvironment {
                    pool_file: 0,
                    pool_format: 0,
                    free_pool_label: 0,
                    scratch: SCRATCH_2A762C,
                };
                let rc = fn_2a762c_m(m, a[0], a[1], self, &k);
                m.bytes[at..at + 24].copy_from_slice(&saved);
                if self.boot.fault.is_some() {
                    return 0x80002001;
                }
                rc
            }
            0x1d513c | 0x1e4a3c => {
                let mut args=[0;7];args[..a.len()].copy_from_slice(a);
                self.boundary=Some(engine::VoiceBoundary{entry:f,args,event:None});0x80002001
            }
            0x741e8c => {

                if a.len() < 18 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::engine_sing::song_notes(m, &crate::engine_sing::SongNoteArgs {
                    text: a[0], events: a[1], event_count: a[2] as i32,
                    flags: a[3] as i32,
                    frequency: (m.half(a[1].wrapping_add(0x20c)) as i16) as f32,
                    pitch: f32::from_bits(m.word(a[1].wrapping_add(0x210))),
                    tail_pause: a[4] as i32, rate: a[5] as i32,
                    volume: a[6] as i32, use_voice: a[7] as i32, voice: a[8],
                    marker: a[9] as i32, pause: a[10], add_pause: a[11] as i32,
                    first_duration: a[12] as i32, second_duration: a[13] as i32,
                    penultimate_duration: a[14] as i32, last_duration: a[15] as i32,
                    out: a[16], capacity: a[17] as i32,
                })
            }
            0x24ac00 => {

                if a.len() < 3 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::speech_part_1::fn_24ac00(m, a[0], a[1], a[2], 0x4800, self)
            }
            0x24c470 => {

                if a.is_empty() {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let literals = crate::component_live_b::speech_literals(m, crate::rom_region::Rom::Us);
                crate::speech_part_2::fn_24c470(m, a[0], &literals, 0x5800, self)
            }
            0x24bbc8 => {

                if a.is_empty() {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let shared = crate::component_live_b::speech_literals(m, crate::rom_region::Rom::Us);
                let literals = crate::component_live_b::domain_literals(m, crate::rom_region::Rom::Us);
                crate::speech_part_2::fn_24bbc8(m, a[0], &shared, &literals, 0x5000, self)
            }
            0x249cec => {

                if a.is_empty() {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                crate::speech_part_1::fn_249cec(m, a[0], 0x4200, self)
            }
            0x1ff4fc => {

                let Ok(args) = a.try_into() else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                };
                crate::component_live_c::insert_prompt_live_m(m, args, 0xe0ff600, self)
            }
            0x1e4b7c => {

                let mut args5 = [0u32; 5];
                if a.len() >= 5 {
                    args5.copy_from_slice(&a[..5]);
                } else if a.len() == 4 {
                    args5[..4].copy_from_slice(&a[..4]);
                    args5[4] = m.word(0xe0ff938);
                } else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let end = source_literal(m, b"At End of Processing");
                let mut live = crate::component_live_b::LiveHost {
                    platform: self.boot.platform,
                    fault: None,
                    scratch: 0x600,
                    rom: self.rom,
                };

                let heap = m.word(m.word(m.word(args5[0])) + 4);
                let mut host = crate::component_live_b2::TokenHost { inner: live, heap };
                let rc = crate::component_live_b2::fn_1e4b7c_live(m, args5, end, 0x600, &mut host);
                if host.inner.fault.is_some() {
                    self.boot.fault = host.inner.fault;
                    return 0x80002001;
                }
                rc
            }
            0x200e64 => {

                let active = if a.len() > 4 {
                    a[4]
                } else if a.len() == 4 {
                    m.word(0xe0ff938)
                } else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                };
                let rc = crate::component_live_c::prompt_orth_live_m(
                    m,
                    a[0],
                    a[1],
                    [a[2], a[3]],
                    active,
                    0xe0ff700,
                    self,
                );
                if self.boot.fault.is_some() {
                    return 0x80002001;
                }
                rc
            }
            0x1d5ba8 => {

                let pending = if a.len() > 4 {
                    a[4]
                } else if a.len() == 4 {
                    m.word(0xe0ff938)
                } else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                };
                let env = match self.rom {
                    crate::rom_region::Rom::Eu => crate::component_live_c::process_environment_eu(),
                    crate::rom_region::Rom::Kr => crate::component_live_c::process_environment_kr(),
                    crate::rom_region::Rom::Us => crate::component_live_c::process_environment_us(),
                };
                let rc = crate::component_live_c::fn_1d5ba8_m(
                    m, a[0], a[1], [a[2], a[3]], pending,
                    &env,
                    0xe0ff080, self,
                );
                if self.boot.fault.is_some() {
                    return 0x80002001;
                }
                rc
            }
            0x15bab0 => {

                if a.len() < 2 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let rc = crate::component_live_c::ld_reset_live_m(m, a[0], a[1], 0xe0fef00, self);
                if self.boot.fault.is_some() {
                    return 0x80002001;
                }
                rc
            }
            0x15c544 => {

                let mut regions = [engine::Region { base: m.base, bytes: m.bytes }];
                let mut sparse = engine::Memory::new(&mut regions).expect("current arena");
                match crate::engine_components::InternalCalls::call(
                    &mut crate::engine_components::RegionalCalls { rom: self.rom },
                    &mut sparse,
                    f,
                    a,
                ) {
                    Ok(rc) => rc,
                    Err(e) => {
                        self.boot.fault = Some(e);
                        0x80002001
                    }
                }
            }
            0x15cdcc => {

                let Ok(args) = a.try_into() else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                };
                crate::component_live_c::write_field_live_m(m, args, self)
            }
            0x1ede44 => {

                let Ok(args) = a.try_into() else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                };
                crate::component_live_c::fn_1ede44_m(
                    m,
                    args,
                    &crate::component_live_c::speech_literals_us(),
                    0xe0fed00,
                    self,
                )
            }
            0x15d378 => {

                let Ok(args) = a.try_into() else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                };
                match crate::component_live_c::field_data_m(m, args, crate::rom_region::addr(self.rom, 0x8d7ca0)) {
                    Ok(rc) => rc,
                    Err(e) => {
                        self.boot.fault = Some(e);
                        0x80002001
                    }
                }
            }
            0x15bf74 => {

                let Ok(args) = a.try_into() else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                };
                crate::component_live_c::create_child_live_m(m, args, 0xe0fef80, self)
            }
            0x15c1e4 => {

                let Ok(args) = a.try_into() else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                };
                crate::component_live_c::clone_record_live_m(m, args, 0xe0fef80, self)
            }
            0x15db2c => {

                if a.len() < 4 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let at = crate::rom_region::addr(self.rom, 0x8d7bb8) + a[2] * 8;
                let table = [[m.word(at), m.word(at + 4)]];
                let mut out = 0u32;
                let rc = crate::linguistic::field_property(&table, 0, &mut out);
                m.put_word(a[3], out);
                rc
            }
            0x15d594 => {

                let mut args5 = [0u32; 5];
                if a.len() >= 5 {
                    args5.copy_from_slice(&a[..5]);
                } else if a.len() == 4 {
                    args5[..4].copy_from_slice(&a[..4]);
                    args5[4] = m.word(0xe0ff938);
                } else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                match crate::component_live_c::core_call_m_rom(m, f, &args5, self.rom) {
                    Ok(rc) => rc,
                    Err(e) => {
                        self.boot.fault = Some(e);
                        0x80002001
                    }
                }
            }
            0x15b518 => {

                let ab: [u32; 2] = match a.try_into() {
                    Ok(ab) => ab,
                    Err(_) => {
                        self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                        return 0x80002001;
                    }
                };
                crate::component_live_c::clear_db_live_m(m, ab, 0xe0fed00, self)
            }
            0x15d6c4 => {

                match crate::component_live_c::core_call_m_rom(m, f, a, self.rom) {
                    Ok(rc) => rc,
                    Err(e) => {
                        self.boot.fault = Some(e);
                        0x80002001
                    }
                }
            }
            0x15c7d8 => {

                match crate::component_live_c::core_call_m(m, f, a) {
                    Ok(rc) => rc,
                    Err(e) => {
                        self.boot.fault = Some(e);
                        0x80002001
                    }
                }
            }
            0x1cca64 => {

                if a.len() < 4 {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                }
                let rc = self.call(m, 0x176f94, &[a[0], a[1], 0xb2c2, 56]);
                if rc >= 0x80000000 {
                    return 0x89202008;
                }
                let level = self.call(m, 0x173fd8, &[m.word(m.word(a[0]) + 16)]);

                if level < 100 {
                    return 0;
                }

                let flag = if a.len() > 4 {
                    a[4]
                } else if a.len() == 4 {
                    m.word(0xe0ff938)
                } else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                };
                let mut template = [0u8; 66];
                template.copy_from_slice(&m.bytes[0x8d7b56..0x8d7b56 + 66]);
                let mut dirty = m.byte(a[0] + 52);
                let tables = crate::linguistic::DumpTables {
                    sent: crate::linguistic::DumpTable { count: 0, bytes: &[] },
                    tok: crate::linguistic::DumpTable { count: 0, bytes: &[] },
                    word: crate::linguistic::DumpTable { count: 0, bytes: &[] },
                };
                let mut first = |_: u32, _: u32, _: u32, _: u32| (1u32, 0u16);
                let mut group = |_: u32, _: u32, _: u16, _: u16| (1u32, 0u16);
                let mut next = |_: u32, _: u32, _: u16| 0u16;
                let mut emit = |_: u32, _: u32, _: u8, _: u8, _: u32, _: u16, _: u16| {};
                crate::linguistic::dump_sentence(
                    Some(a[0]),
                    a[1],
                    a[2],
                    a[3],
                    flag,
                    level,
                    &mut dirty,
                    &template,
                    &tables,
                    &mut first,
                    &mut group,
                    &mut next,
                    &mut emit,
                )
            }
            0x15d0c4 => {

                let Ok(args) = a.try_into() else {
                    self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                    return 0x80002001;
                };
                match crate::component_live_c::read_field_m(
                    m,
                    args,
                    crate::rom_region::addr(self.rom, 0x8d7ca0),
                    0xe0feb00,
                    &mut [0u8; 64],
                ) {
                    Ok(rc) => rc,
                    Err(e) => {
                        self.boot.fault = Some(e);
                        0x80002001
                    }
                }
            }
            0x206bc8 => fn_206bc8_m(a),
            0x1d8eac => fn_1d8eac_m(a),
            0x1f9310 => fn_1f9310_m(a),
            0x1f1414 => fn_1f1414_m(a),
            0x1f2bf0 => fn_1f2bf0_m(a),
            0x1985fc => crate::frontend::begin_stage_hook((a[0]!=0).then_some(a[0])),
            0x19860c => crate::frontend::end_stage_hook((a[0]!=0).then_some(a[0])),
            0x14ed4c => {
                let valid=crate::frontend::validate_handle(a[0]!=0,a[1],0x47dd,32);
                if valid>=0x80000000{return 0x81706008;}
                let pair=[m.word(a[0]),m.word(a[0]+4)];
                let saved=m.word(0x380);
                let rc=self.call(m,0x19611c,&[pair[0],pair[1],0x380]);
                let scripts=crate::frontend::OpenScripts{rc794:0,rc68c:0,rc_db0:0,rc_611c:rc,ctx_611c:m.word(0x380)};
                m.put_word(0x380,saved);
                crate::frontend::set_pipeline_pos(m.bytes,Some(a[0]),a[1],a[2],&scripts)
            }
            0x17b058 => m.boot_parameter_call(self, f, a, 0x0e0f7e00).expect("registry lookup"),
            0x19611c => {
                let mut pair = [0;8];
                for i in 0..8 { pair[i] = m.byte(crate::rom_region::addr(self.rom,0x8d7660)+i as u32); }
                let half = if a[0] == 0 { None } else { Some(m.half(a[0])) };
                let mut ctx=0;
                let rc=crate::frontend::resolve_engine_context(a[0],half,a[1],&pair,crate::rom_region::addr(self.rom,0x8d7640),&mut ctx);
                m.put_word(a[2],ctx); rc
            }
            0x176f2c | 0x176f5c => {

                let mut target = None;
                let p=(a[0]!=0).then_some(a[0]);
                let rc=if f==0x176f2c {
                    crate::frontend::acquire_lock(m.bytes,p,|t| {target=Some(t);0})
                } else {
                    crate::frontend::release_lock(m.bytes,p,|t| {target=Some(t);0})
                };
                if let Some(t)=target {
                    let table=m.word(m.word(a[0])+8);
                    let entry=m.word(table+if f==0x176f2c {8}else{12});
                    self.call(m,entry,&[t])
                } else { rc }
            }
            0x195d58 => crate::frontend::compare_str(&m.bytes[a[0] as usize..],&m.bytes[a[1] as usize..]) as u32,
            0x2c322c => {

                let mut args = [0; 7];
                args[..a.len().min(7)].copy_from_slice(&a[..a.len().min(7)]);
                self.boundary = Some(engine::VoiceBoundary { entry: f, args, event: None });
                0x80002001
            }
            0x2be984 => {

                let snap = self.snap;
                let eu = |us: u32| crate::rom_region::addr(self.rom, us) as usize;
                let mut escapes = [0i16; 75];
                for (i, slot) in escapes.iter_mut().enumerate() {
                    let at = eu(0x9140e4) + i * 2;
                    *slot = i16::from_le_bytes(snap[at..at + 2].try_into().unwrap());
                }
                let mut utf8_limits = [0i32; 6];
                for (i, slot) in utf8_limits.iter_mut().enumerate() {
                    let at = eu(0x9147ac) + i * 4;
                    *slot = i32::from_le_bytes(snap[at..at + 4].try_into().unwrap());
                }
                let mut utf8_first = [0i32; 6];
                for (i, slot) in utf8_first.iter_mut().enumerate() {
                    let at = eu(0x9147c4) + i * 4;
                    *slot = i32::from_le_bytes(snap[at..at + 4].try_into().unwrap());
                }
                let mut utf8_mask = [0u32; 6];
                for (i, slot) in utf8_mask.iter_mut().enumerate() {
                    let at = eu(0x9147dc) + i * 4;
                    *slot = u32::from_le_bytes(snap[at..at + 4].try_into().unwrap());
                }
                let tables = crate::regex::PcreTables {
                    op_lengths: snap[eu(0x914092)..eu(0x914092) + 0x100].try_into().unwrap(),
                    ctypes: snap[eu(0x91426c)..eu(0x91426c) + 0x100].try_into().unwrap(),
                    default_tables: snap[eu(0x91436c)..eu(0x91436c) + 0x440].try_into().unwrap(),
                    escapes: &escapes,
                    utf8_limits: &utf8_limits,
                    utf8_first: &utf8_first,
                    utf8_cont: snap[eu(0x9147f4)..eu(0x9147f4) + 0x40].try_into().unwrap(),
                    utf8_mask: &utf8_mask,
                    rep_min: snap[eu(0x914044)..eu(0x914044) + 6].try_into().unwrap(),
                    rep_max: snap[eu(0x91404a)..eu(0x91404a) + 6].try_into().unwrap(),
                    posix_lengths: &[],
                    posix_names: &[],
                    posix_maps: &[],
                    ctypes_overspill: &snap[eu(0x9147ac)..eu(0x9147ac) + 4096],
                };
                let mut program = [0u8; 2048];
                let mut subject = [0u8; 2048];
                let mut frames = [crate::regex::MatchFrame::default(); 2048];
                let mut tags = [0u8; 2048];
                let mut blocks = [crate::regex::FrameBlockState::default(); 64];
                let mut escape_scratch = crate::render_part_d::EscapeScratch {
                    program: &mut program,
                    subject: &mut subject,
                    frames: &mut frames,
                    tags: &mut tags,
                    blocks: &mut blocks,
                    stack: crate::engine_live_render::scratch::PCRE_STACK,
                    fallback: 0,
                    toi_names: [0xa09390, 0xa09394, 0xa093da],
                };
                let mut regex = crate::render_part_c::RegexData {
                    scratch: &mut escape_scratch,
                    tables: &tables,
                };
                let mut eos = [0u8; 40];
                eos.copy_from_slice(&snap[0x9c56f4..0x9c56f4 + 40]);
                let separators = crate::engine_live_render::separators_of(m,self.rom);
                let engine = crate::render_part_c::Environment {
                    cached_handle: core::array::from_fn(|i| m.byte(crate::rom_region::addr(self.rom,0x8d7660) + i as u32)),
                    default_context: crate::rom_region::addr(self.rom,0x8d7640),
                };
                let parser = crate::render_part_c::ParserData {
                    eos: &eos,
                    separators: &separators,
                    category: 0x9c5650,
                    formats: [0x9c5830, 0x9c5888, 0x2beed0, 0x9c57c4, 0x9c577c],
                    extra_key: 0,
                    scratch: crate::engine_live_render::scratch::SENTENCE,
                };
                let sentence = crate::render_part_c::SentenceData {
                    parser: &parser,
                    labels: [0x2beeb4, 0x2beec0],
                    nfa_format: 0x2bf130,
                    nfa_words: [0x2bf128, 0x2bf12c],
                    dictionary_name: 0x2bf120,
                };
                crate::render_part_c::fn_2be984(m, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8] as i32, &engine, &sentence, &mut regex, self)
            }
            0x1748a0 => {

                if a[0] == 0 || m.byte(a[0] + 64) == 0 {
                    0
                } else {
                    let mut r = 0;
                    for i in 0..5u32 {
                        let vt = m.word(a[0] + 12 + i * 8);
                        if vt != 0 {
                            let g = m.word(vt + 52);
                            if g != 0 {
                                let t = self.call(m, g, &[m.word(a[0] + 8 + i * 8), a[1], a[2], a[3], a[4], a[5]]);
                                if t >= 0x80000000 {
                                    r = t;
                                }
                            }
                        }
                    }
                    let vt = m.word(a[0] + 60);
                    if vt != 0 {
                        let g = m.word(vt + 52);
                        if g != 0 {
                            let t = self.call(m, g, &[m.word(a[0] + 56), a[1], a[2], a[3], a[4], a[5]]);
                            if t >= 0x80000000 {
                                r = t;
                            }
                        }
                    }
                    r
                }
            }

            0x4a779c => {
                let target = m.word(m.word(a[0]) + 8);
                self.call(m, target, &[a[0], a[1], 4])
            }
            0x4a7758 => m.fn_4a7758(self, [a[0], a[1], a[2]]),
            0x4a79b8 => crate::speech_part_3::fn_4a79b8(m, a[0], a[1], a[2], self),
            0x4a774c => crate::speech_part_3::fn_4a774c(m, a[0], a[1], self),
            0x39f5e8 => m.fn_39f5e8(self, [a[0], a[1], a[2]]),
            0x39f5b8 => { crate::speech_part_3::fn_39f5b8(m, a[0], a[1], self); 0 }
            0x163f20 => m.fn_163f20(self, [a[0], a[1], a[2], a[3]]),
            0x1840fc => { crate::boot_part_b::fn_1840fc(m, a[0], self); 0 }
            0x18423c => { crate::boot_part_b::fn_18423c(m, a[0], self); 0 }
            0x184f20 => m.boot_heap_allocate([a[0], a[1], a[2], a[3], a[4], a[5]]),
            0x184f84 => { m.boot_heap_release(a[0], a[1]); 0 }
            0x19b78c => { crate::boot_part_b2::fn_19b78c_guest(m, a[0], a[1]); 0 }
            0x1850c0 => crate::boot_port_1::fn_1850c0(m, a[0], a[1], a[2] as i32),
            0x1852e8 => crate::boot_port_1::fn_1852e8(m, a[0], a[1], a[2]),
            0x19b814 => crate::boot_port_1::fn_19b814(m, a[0], a[1], a[2], a[3], a[4]),
            0x183edc => {
                if m.word(a[0]) == 0 { self.boot.fault=Some(engine::Error::InvalidSnapshot); return 0x80002001; }
                let handle = self.boot.platform.arbiter_address();
                crate::boot_part_b2::fn_183edc(m, a[0], handle, self); 0
            }
            0x165d00 => {
                let g = crate::boot::Globals {
                    manager: 0, callbacks: 0, rules: 0, components: 0,
                    dictionary: 0, context: 0, mode: 0,
                    semaphore: self.boot.platform.arbiter_address(),
                    service: 0, command: 0,
                };
                m.fn_165d00(self, &g, a[0]); 0
            }
            0x15b758 => crate::db_live::create_record_m(self,m,a[0],a[1],a[2],crate::rom_region::addr(self.rom,0x8d8274),crate::rom_region::addr(self.rom,0x8d7ca0)),
            0x174aac => {
                let ticket=core::array::from_fn(|i|m.word(crate::rom_region::addr(self.rom,0x8d7614)+i as u32*4));
                crate::db_live::fn_174aac_m(self,m,a[0],a[1],a[2],ticket)
            }
            0x175c40 => crate::db_live::append_owner_m(self,m,a[0],a[1],0x340),

            0x15b5f0 => crate::db_live::close_db_m(self,m,a[0],a[1],&crate::db_live::DbData{module:0,open_begin:0,open_end:0,open_detail:0,reset_begin:0,reset_end:0,pool_name_word:0,keys:[0;5]}),
            _ => {
                if self.rom == crate::rom_region::Rom::Eu && f == 0x14e084 {
                    return self.call(m, 0x14dfc0, a);
                }
                self.boot.invoke(m,f,a)
            }
        }
    }
}
impl PoolHooksM for Host<'_> {
    fn log_pool_m(&mut self,p:u32,n:u32,m:&mut Memory<'_>) {
        self.call_m(0x1738e8,&[m.word(m.word(p)+16),0x8d7b56,5,0,0x8dc928,m.word(p+4),0x180f78,
            m.word(p+24),m.word(p+20),m.word(p+12),m.word(p+16),n],m);
    }
}
impl DbHooksM for Host<'_> {
    fn literal_m(&self,us:u32)->u32 {crate::rom_region::addr(self.rom,us)}
    fn call_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32 {
        if f==0x1738e8 {
            let mut saved=[0u32;12];
            for (i,s) in saved.iter_mut().enumerate() { *s=m.word(0x500+i as u32*4); }
            for (i,v) in a[5..].iter().enumerate() {m.put_word(0x500+i as u32*4,*v);}
            let rc=self.call(m,f,&[a[0],a[1],a[2],a[3],a[4],0x500]);
            for (i,s) in saved.iter().enumerate() {m.put_word(0x500+i as u32*4,*s);}
            rc
        } else { self.call(m,f,a) }
    }
}
struct ViewOnly;
impl crate::bridge::Platform for ViewOnly {
    fn thread_id(&mut self)->u32 {unreachable!()}
    fn arbiter_address(&self)->u32 {unreachable!()}
    fn arbitrate(&mut self,_:&mut Memory<'_>,_:&[u32])->u32 {unreachable!()}
}

pub fn database_return(
    m:&mut engine::Memory<'_,'_>,boundary:engine::VoiceBoundary,
    request_sp:u32, workspace:&mut [u8],platform:&mut dyn engine::HeapPlatform,
    rom:crate::rom_region::Rom,
)->Result<engine::VoiceBoundary,engine::Error> {
    if boundary.entry!=0x15bc18 {return Err(engine::Error::UnsupportedStage{entry:boundary.entry});}
    let mut os=ViewOnly;
    let mut flat=crate::bridge::Flat::new(m,workspace,&mut os,0)?;
    let mut arena=flat.memory();
    let mut host=Host{boot:engine::SpeechBootHost{platform,fault:None},boundary:None,runtime:None,snap:&[],trace:None,until:None,depth:0,stack:[0;64],stack_len:0,rom};
    let r=rom;
    let data=crate::db_live::DbData {
        module:arena.word(crate::rom_region::addr(r,0x15b5d0)),open_begin:crate::rom_region::addr(r,0x15be18),open_end:crate::rom_region::addr(r,0x15beb0),
        open_detail:arena.word(crate::rom_region::addr(r,0x15bea8)),reset_begin:crate::rom_region::addr(r,0x15bbdc),reset_end:crate::rom_region::addr(r,0x15bbfc),
        pool_name_word:arena.word(crate::rom_region::addr(r,0x15beac)),keys:[crate::rom_region::addr(r,0x15be30),crate::rom_region::addr(r,0x15be48),crate::rom_region::addr(r,0x15be60),crate::rom_region::addr(r,0x15be74),crate::rom_region::addr(r,0x15be90)],
    };
    let caller_pipe=arena.word(boundary.args[4]-12);
    let rc=crate::db_live::open_db_m(&mut host,&mut arena,boundary.args[..5].try_into().unwrap(),&data,0x300);

    let sp=request_sp-0x1f8;
    arena.put_word(sp-4,crate::rom_region::addr(r,0x156bf0));
    arena.put_word(sp-72,caller_pipe);
    arena.put_word(sp-68,crate::rom_region::addr(r,0x15bde8));
    arena.put_word(sp-64,data.open_end);
    flat.finish()?;
    if let Some(e)=host.boot.fault {return Err(e);}
    if rc>=0x80000000 {return Err(engine::Error::EngineStatus(rc));}
    Ok(engine::VoiceBoundary{entry:0x156bf0,args:[rc,0,0,0,0,0,0],event:None})
}

pub fn component_boundary(m:&mut engine::Memory<'_,'_>,synthesis:engine::VoiceBoundary,
    request_sp:u32, workspace:&mut [u8],platform:&mut dyn engine::HeapPlatform,rom:crate::rom_region::Rom)->Result<engine::VoiceBoundary,engine::Error> {
    let mut os=ViewOnly;
    let mut flat=crate::bridge::Flat::new(m,workspace,&mut os,0)?;
    let mut arena=flat.memory();
    let mut host=Host{boot:engine::SpeechBootHost{platform,fault:None},boundary:None,runtime:None,snap:&[],trace:None,until:None,depth:0,stack:[0;64],stack_len:0,rom};
    let synth=synthesis.args[1];let ctx=synthesis.args[0];let out=synth+0x84;

    if synthesis.args[4]!=0 || arena.word(out+52)!=0 {
        return Err(engine::Error::UnsupportedStage{entry:0x15668c});
    }
    arena.put_byte(out+36,1);
    let props=arena.word(ctx+20);
    let rc=host.call(&mut arena,0x175c40,&[props,out+48]);
    let count=arena.half(arena.word(synth+0x7c)+12) as u32;
    let sp=request_sp-0x1f8;
    arena.put_word(sp-4,crate::rom_region::addr(rom,0x156a78));
    arena.put_word(sp,count);
    arena.put_word(sp-72,arena.word(ctx+4));
    arena.put_word(sp-68,crate::rom_region::addr(rom,0x174b00));
    arena.put_word(sp-64,arena.word(crate::rom_region::addr(rom,0x8d7614)+8));
    arena.put_word(sp-60,arena.word(crate::rom_region::addr(rom,0x8d7614)+12));
    flat.finish()?;
    if let Some(e)=host.boot.fault {return Err(e);}
    if rc>=0x80000000 {return Err(engine::Error::EngineStatus(rc));}
    Ok(engine::VoiceBoundary{entry:0x153c5c,args:[synth,0,synthesis.args[6],0,count,0,0],event:None})
}

use crate::frontend::{compare_str,copy_str,fill_bytes};
fn mem_word(m:&[u8],p:u32)->u32 {u32::from_le_bytes(m[p as usize..p as usize+4].try_into().unwrap())}
fn mem_half(m:&[u8],p:u32)->u16 {u16::from_le_bytes(m[p as usize..p as usize+2].try_into().unwrap())}
fn mem_store(m:&mut [u8],p:u32,v:u32) {m[p as usize..p as usize+4].copy_from_slice(&v.to_le_bytes());}
fn mem_store16(m:&mut [u8],p:u32,v:u16) {m[p as usize..p as usize+2].copy_from_slice(&v.to_le_bytes());}
fn terminated(m:&[u8],p:u32)->&[u8] {let s=&m[p as usize..];&s[..s.iter().position(|v|*v==0).unwrap()+1]}

pub fn start_components_m(
    mem: &mut [u8],
    synth: u32,
    start: u32,
    end: u32,
    alloc: Option<u32>,
    cb_arg: u32,
    mut set_stage: impl FnMut(u32, u32, u32, &mut [u8]) -> u32,
    mut begin: impl FnMut(u32, u32, u32, &mut [u8]),
    mut open: impl FnMut(u32, u32, u32, &[u8], u32, u32, &mut [u8]) -> ([u32; 2], u32),
    mut process_start: impl FnMut(u32, u32, u32, u32, &mut [u8]) -> u32,
    mut end_stage: impl FnMut(u32, u32, u32, &mut [u8]),
    cb: Option<impl FnMut(u32, u32, u32, u32, &mut [u8]) -> u32>,
) -> u32 {
    const MARKERS: &[u8] = b"application/x-realspeak-markers-pp;version=4.0\0";
    const TEXT_UTF16: &[u8] = b"text/plain;charset=utf-16\0";
    const PP_TEXT: &[u8] = b"pp/text\0";
    const FE_LID: &[u8] = b"fe/fe_lid\0";
    let out = synth + 0x84;
    let pending = out + 148;
    let cs = mem_word(mem, synth + 0x7c);
    let comps = mem_word(mem, cs + 8);
    let count = mem_half(mem, cs + 12) as u32;
    let status = mem[(synth + 0x80) as usize];
    let obj = mem_word(mem, cs + 16);
    let arg = mem_word(mem, cs + 20);
    let pipe_a = mem_word(mem, out + 16);
    let pipe_b = mem_word(mem, out + 20);
    mem[pending as usize] = 0;
    mem_store16(mem, out + 180, 0);
    mem_store16(mem, out + 182, 0);
    let mut rc = 0u32;
    if status == 2 {
        let mut i = 0u32;
        while i < count {
            let comp = mem_word(mem, comps.wrapping_add(i.wrapping_mul(4)));
            if compare_str(terminated(mem, mem_word(mem, comp + 4)), PP_TEXT) == 0 {
                mem_store16(mem, out + 180, i as u16);
                mem_store16(mem, out + 182, i as u16);
                let comp1 =
                    mem_word(mem, comps.wrapping_add(i.wrapping_add(1).wrapping_mul(4)));
                if compare_str(terminated(mem, mem_word(mem, comp1 + 4)), FE_LID) == 0 {
                    mem_store16(mem, out + 182, i.wrapping_add(1) as u16);
                    let comp2 = mem_word(
                        mem,
                        comps.wrapping_add(i.wrapping_add(2).wrapping_mul(4)),
                    );
                    copy_str(mem, pending, mem_word(mem, comp2 + 4));
                } else {
                    copy_str(mem, pending, mem_word(mem, comp1 + 4));
                }
            }
            i += 1;
        }
    }
    let mut cb = cb;
    let mut s = start;
    while rc < 0x80000000 && s < end {
        let next = s.wrapping_add(1);

        let _ = set_stage(pipe_a, pipe_b, next & 0xffff, mem);
        let comp = mem_word(mem, comps.wrapping_add(s.wrapping_mul(4)));
        let name = mem_word(mem, comp + 4);
        if obj != 0 {
            begin(obj, arg, s.wrapping_add(2), mem);
        }
        if status == 2 {
            if compare_str(terminated(mem, name), terminated(mem, pending)) == 0 {
                let (pair, _) = open(pipe_a, pipe_b, 18, MARKERS, 0, 0, mem);
                mem_store(mem, synth + 0x4c, pair[0]);
                mem_store(mem, synth + 0x50, pair[1]);
            }
            if compare_str(terminated(mem, name), PP_TEXT) == 0 {
                let (pair, _) = open(pipe_a, pipe_b, 18, TEXT_UTF16, 0, 0, mem);
                mem_store(mem, synth + 0x44, pair[0]);
                mem_store(mem, synth + 0x48, pair[1]);
                mem_store(mem, out + 124, 0);
            }
        }
        let r2 = process_start(
            mem_word(mem, comp + 12),
            mem_word(mem, comp + 16),
            pipe_a,
            pipe_b,
            mem,
        );
        if obj != 0 {
            end_stage(obj, arg, s.wrapping_add(2), mem);
        }
        rc = r2;
        if r2 >= 0x80000000 {

        } else {
            mem_store16(mem, out + 78, next as u16);
            if let Some(cb) = cb.as_mut() {
                rc = cb(cb_arg, name, pipe_a, pipe_b, mem);
            }
        }
        s = next;
    }
    if status == 2 {
        mem_store(mem, out + 144, 0);
        let a = alloc.unwrap();
        mem_store(mem, out + 188, a);
        fill_bytes(mem, a, 0, 0x84);
        for k in 0..5 {
            mem_store(mem, a + 4 + k * 4, 9999);
        }
    }
    rc
}

pub fn first_component(m:&mut engine::Memory<'_,'_>,b:engine::VoiceBoundary,request_sp:u32,
    workspace:&mut [u8],platform:&mut dyn engine::HeapPlatform,rom:crate::rom_region::Rom)->Result<engine::VoiceBoundary,engine::Error> {
    let mut os=ViewOnly;
    let mut flat=crate::bridge::Flat::new(m,workspace,&mut os,0)?;
    let mut arena=flat.memory();
    let mut queue=QueueRuntime{registers:engine::NotificationRegisters{s16:0x47dd,s17:0}};
    let host=core::cell::RefCell::new(Host{boot:engine::SpeechBootHost{platform,fault:None},boundary:None,runtime:Some(&mut queue),snap:&[],trace:None,until:None,depth:0,stack:[0;64],stack_len:0,rom});
    let synth=b.args[0];
    if arena.byte(synth+0x80)==2 || b.args[1]!=0 {
        return Err(engine::Error::UnsupportedStage{entry:0x153c5c});
    }
    let vt=mem_word(arena.bytes,synth+0x40);
    let set=mem_word(arena.bytes,vt+28);
    start_components_m(arena.bytes,synth,b.args[3],b.args[4],None,b.args[2],
        |a,b,c,bytes| {let mut m=Memory{base:0,bytes};host.borrow_mut().call(&mut m,set,&[a,b,c])},
        |a,b,c,bytes| {let mut m=Memory{base:0,bytes};host.borrow_mut().call(&mut m,0x1985fc,&[a,b,c]);},
        |_,_,_,_,_,_,_| unreachable!("status-2 opens need live list allocation"),
        |a,b,c,d,bytes| {let mut m=Memory{base:0,bytes};

            let cs=m.word(synth+0x7c);let comps=m.word(cs+8);let comp=(0..m.half(cs+12) as u32).map(|i|m.word(comps+i*4)).find(|p|m.word(*p+12)==a && m.word(*p+16)==b).expect("component owner");let table=m.word(comp+8);
            let entry=m.word(table+48);
            host.borrow_mut().call(&mut m,entry,&[a,b,c,d])},
        |a,b,c,bytes| {let mut m=Memory{base:0,bytes};host.borrow_mut().call(&mut m,0x19860c,&[a,b,c]);},
        None::<fn(u32,u32,u32,u32,&mut [u8])->u32>);
    let sp=request_sp-0x1f8;
    let cs=arena.word(synth+0x7c);let comps=arena.word(cs+8);
    if host.borrow().boundary.is_none(){
        let count=arena.half(arena.word(synth+0x7c)+12) as u32;
        let out=synth+0x84;
        let ctx=arena.word(out);
        let params=arena.word(ctx+20);
        let pipe=[arena.word(out+0x10),arena.word(out+0x14)];
        let vt=arena.word(synth+0x40);
        let slot=0x2a00u32;
        arena.put_word(slot,0);

        let mut rc=0u32;
        if arena.word(synth+0x18)!=0 {

            let set_entry=arena.word(vt+0x1c);
            rc=host.borrow_mut().call(&mut arena,set_entry,&[pipe[0],pipe[1],count+1]);
            if rc<0x80000000 {
                let found=host.borrow_mut().call(&mut arena,0x175a1c,&[params,crate::rom_region::addr(rom,0x155ecc),slot]);
                let mime=arena.word(slot);
                if found<0x80000000 {
                    if mime!=0 {
                        if arena.byte(mime)!=0 {
                            let open=arena.word(vt+0x20);
                            let opened=host.borrow_mut().call(&mut arena,open,&[pipe[0],pipe[1],18,mime,0,0,out+0x3c]);
                            if opened<0x80000000 {
                                host.borrow_mut().call(&mut arena,open,&[pipe[0],pipe[1],18,crate::rom_region::addr(rom,0x154094),0,0,out+0x44]);
                            }
                        }
                        host.borrow_mut().call(&mut arena,0x1757a8,&[params,mime]);
                    }
                }
            }
        }
        arena.put_word(slot,0);

        if arena.byte(synth+0x80)!=2 {
            if rc>=0x80000000 { return Err(engine::Error::EngineStatus(rc)); }
            if arena.word(out+0x3c)!=0 {

                let mut side=OutputSide{host:&host,synth};
                rc=emit_output_m(&mut *arena.bytes,synth,None,&mut side);

                for i in 0..0x1c0 { arena.put_byte(OUT_LOCAL+i,0); }
                if rc>=0x80000000 { return Err(engine::Error::EngineStatus(rc)); }
            }
        }
        flat.finish()?;
        if let Some(e)=host.borrow_mut().boot.fault{return Err(e);}
        return Ok(engine::VoiceBoundary{entry:0x15429c,args:[synth,0,0,0,0,0,synth],event:None});
    }
    let owner=host.borrow().boundary.map(|b|b.args[0]);
    let comp=(0..arena.half(cs+12) as u32).map(|i|arena.word(comps+i*4)).find(|p|Some(arena.word(*p+12))==owner).unwrap_or(arena.word(comps));
    arena.put_word(sp-4,crate::rom_region::addr(rom,0x156e10));
    arena.put_word(sp-76,crate::rom_region::addr(rom,0x153d94));
    arena.put_word(sp-72,crate::rom_region::addr(rom,0x154074));
    arena.put_word(sp-68,arena.word(comp+4));
    flat.finish()?;
    let mut host=host.into_inner();
    if let Some(e)=host.boot.fault {return Err(e);}
    host.boundary.take().ok_or(engine::Error::EngineStatus(0x80002001))
}

fn speech_environment(m:&Memory<'_>,rom:crate::rom_region::Rom)->crate::speech_part_2::Environment {
    crate::speech_part_2::Environment{cached_handle:core::array::from_fn(|i|m.byte(crate::rom_region::addr(rom,0x8d7660)+i as u32)),default_context:crate::rom_region::addr(rom,0x8d7640),input_types:[0;2],output_types:[0;2]}
}

fn source_literal(m:&Memory<'_>,text:&[u8])->u32 {
    let end=m.bytes.len().min(0x500000);
    if end<=0x100000{return 0;}
    (0x100000+m.bytes[0x100000..end].windows(text.len()+1)
        .position(|w|&w[..text.len()]==text&&w[text.len()]==0).expect("mapped source literal")) as u32
}

fn speech_types_environment(m:&Memory<'_>,rom:crate::rom_region::Rom)->crate::speech_part_2::Environment {
    use crate::rom_region::addr;
    crate::speech_part_2::Environment{
        cached_handle:core::array::from_fn(|i|m.byte(addr(rom,0x8d7660)+i as u32)),
        default_context:addr(rom,0x8d7640),
        input_types:[m.word(addr(rom,0x8f902c)),m.word(addr(rom,0x8f902c)+4)],
        output_types:[m.word(addr(rom,0x8f9038)),m.word(addr(rom,0x8f9038)+4)],
    }
}

fn spelling_environment(m:&Memory<'_>,rom:crate::rom_region::Rom)->crate::speech_part_2::Environment {
    use crate::rom_region::addr;
    crate::speech_part_2::Environment{
        cached_handle:core::array::from_fn(|i|m.byte(addr(rom,0x8d7660)+i as u32)),
        default_context:addr(rom,0x8d7640),
        input_types:[m.word(addr(rom,0x8fa1f4)),m.word(addr(rom,0x8fa1f4)+4)],
        output_types:[m.word(addr(rom,0x8fa200)),m.word(addr(rom,0x8fa200)+4)],
    }
}

fn component_environment(m:&Memory<'_>,rom:crate::rom_region::Rom)->crate::speech_part_1::ComponentEnvironment {
    use crate::rom_region::addr;
    crate::speech_part_1::ComponentEnvironment{
        cached_handle:core::array::from_fn(|i|m.byte(addr(rom,0x8d7660)+i as u32)),
        input_types:[m.word(addr(rom,0x8f8860)),m.word(addr(rom,0x8f8860)+4)],
    }
}
fn stream_environment(m:&Memory<'_>,rom:crate::rom_region::Rom)->crate::speech_part_1::StreamEnvironment {
    use crate::rom_region::addr;
    crate::speech_part_1::StreamEnvironment{
        cached_handle:core::array::from_fn(|i|m.byte(addr(rom,0x8d7660)+i as u32)),
        default_context:addr(rom,0x8d7640),
        utf16:source_literal(m,b"text/plain;charset=utf-16"),
        depes:source_literal(m,b"text/plain;charset=depes"),
        markers:source_literal(m,b"application/x-realspeak-markers-pp;version=4.0"),
        input_mime:source_literal(m,b"inputmimetype"),
        category:addr(rom,0x8f8c6c),
        format:source_literal(m,b"%s%s"),
        content_name:source_literal(m,b"mimeContentType"),
        mime_name:source_literal(m,b"mimeType"),
        ids:core::array::from_fn(|i|m.half(addr(rom,0x8f8cae)+i as u32*6)),
    }
}
fn language_environment(m:&Memory<'_>,rom:crate::rom_region::Rom)->crate::speech_part_1::LanguageEnvironment {    use crate::rom_region::addr;
    crate::speech_part_1::LanguageEnvironment{
        cached_handle:core::array::from_fn(|i|m.byte(addr(rom,0x8d7660)+i as u32)),
        depes:source_literal(m,b"text/plain;charset=depes"),
        languages:[m.word(addr(rom,0x8f87a4)),m.word(addr(rom,0x8f87a8))],
        voices:[m.word(addr(rom,0x8f87b0)),m.word(addr(rom,0x8f87b4))],
        category:addr(rom,0x8f87bc),
        text_analysis:source_literal(m,b"textanalysison"),
        yes:source_literal(m,b"yes"),
        bias:source_literal(m,b"initcommonbiasvalue"),
        original_voice:source_literal(m,b"lidoriginalvoice"),
        voice:source_literal(m,b"voice"),
        lid_state:source_literal(m,b"lidstate"),
    }
}
impl crate::speech_part_2::Host for Host<'_> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.call(m,f,a)}
}
impl crate::boot_part_b::Host for Host<'_> {
    fn real_log_args(&mut self)->bool { true }
    fn log_variadic(&mut self,m:&mut Memory<'_>,entry:u32,a:[u32;4],format:&[u8],varargs:&[u32])->bool {
        crate::log_live::variadic_m(m,entry,a,format,varargs,0x0e0f7d00,|m,f,a|self.call(m,f,a))
    }

    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.call(m,f,a)}
    fn thread_id(&mut self)->u32 {self.boot.platform.thread_id()}
}
impl crate::boot_part_e::Host for Host<'_> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.call(m,f,a)}
}
impl crate::boot_part_b::FormatHost for Host<'_> {}
impl crate::log_live::LogMethodsM for Host<'_> {
    fn log_m(&mut self,f:u32,o:u32,c:u32,l:u32,a:u32,t:u32,v:u32,m:&mut Memory<'_>)->u32 {self.call(m,f,&[o,c,l,a,t,v])}
}

pub fn fetch_param_m(h:&mut impl DbHooksM,m:&mut Memory<'_>,obj:u32,key:u32,out:u32)->u32 {
    assert_eq!(m.base,0);
    if obj==0 || key==0 || out==0 {return 0x8b702007;}
    m.put_word(out,0);
    let rc=h.call_m(0x176f2c,&[m.word(obj+4)],m);
    if rc>=0x80000000 {return rc;}
    let mut ent=0;
    let tree=m.word(obj+8);
    let rc=crate::reentrant2::lookup_value_m(m.bytes,(tree!=0).then_some(tree),key,Some(&mut ent),|a,b,bytes| {
        let mut m=Memory{base:0,bytes};let tree=m.word(obj+8)+24;let target=m.word(m.word(tree+36)+8);
        h.call_m(target,&[a,b],&mut m) as i32
    });
    let mut result=if rc>=0x80000000 {(rc&0x1fff)|0x8b702000}else{0};
    if result==0 && ent!=0 {m.put_word(out,if m.word(ent+4)!=0 {m.word(ent)}else{0});}
    let rc=h.call_m(0x176f5c,&[m.word(obj+4)],m);
    if rc>=0x80000000 && result<0x80000000 {result=rc;}result
}

use crate::frontend::{validate_handle,render_unsigned,divmod_u32,INSERT_EVENT_LEN,filter_markers,copy_bytes};
use core::cell::RefCell;
pub trait AudioOpenHooksM {

    fn stream_open_m(
        &mut self,
        h0: u32,
        h1: u32,
        name: &[u8],
        x: u32,
        y: u32,
        out: &mut (u32, u32), mem: &mut [u8]) -> u32;

    fn query_marker_mode_m(&mut self, mem: &mut [u8]) -> u32;

    fn audio_open_failed_m(&mut self, logger: u32, mem: &mut [u8]);

    fn content_open_failed_m(&mut self, logger: u32, code: u32, mem: &mut [u8]);
}
pub trait CtxResetHooksM {

    fn release_pair_m(&mut self, a: u32, b: u32, flag: u32, mem: &mut [u8]) -> u32;

    fn free_pair_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> u32;

    fn detach_pair_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> u32;

    fn null_pair_m(&mut self, mem: &mut [u8]) -> (u32, u32);
}
pub trait VMethodHooksM {

    fn call_method_m(&mut self, c: u32, d: u32, mem: &mut [u8]) -> u32;
}
pub trait InserterPumpHooksM {

    fn buf_available_m(&mut self, h0: u32, h1: u32, count: &mut u32, end: &mut u8, mem: &mut [u8]) -> u32;

    fn buf_read_m(&mut self, h0: u32, h1: u32, data: &mut u32, count: &mut u32, mem: &mut [u8]) -> u32;

    fn buf_write_m(&mut self, h0: u32, h1: u32, data: &mut u32, count: u32, mem: &mut [u8]) -> u32;

    fn buf_consume_m(&mut self, h0: u32, h1: u32, n: u32, mem: &mut [u8]) -> u32;

    fn buf_close_m(&mut self, h0: u32, h1: u32, mem: &mut [u8]) -> u32;

    fn buf_copy_m(
        &mut self,
        d0: u32,
        d1: u32,
        s0: u32,
        s1: u32,
        n: u32,
        moved: &mut u32, mem: &mut [u8]) -> u32;

    fn buf_position_m(&mut self, h0: u32, h1: u32, pos: &mut u32, mem: &mut [u8]) -> u32;

    fn buf_commit_m(&mut self, h0: u32, h1: u32, n: u32, mem: &mut [u8]) -> u32;

    fn ins_resolve_m(
        &mut self,
        s0: u32,
        s1: u32,
        uri: u32,
        n0: u32,
        n1: u32,
        out: &mut u32, mem: &mut [u8]) -> u32;

    fn ins_open_m(
        &mut self,
        s0: u32,
        s1: u32,
        name: u32,
        pos: &mut u32,
        rem: &mut u32,
        ins: &mut (u32, u32), mem: &mut [u8]) -> u32;

    fn ins_close_m(&mut self, h0: u32, h1: u32, flag: u32, mem: &mut [u8]) -> u32;

    fn pump_remote_m(&mut self, mem: &mut [u8], base: u32) -> u32;

    fn null_pair_m(&mut self, mem: &mut [u8]) -> (u32, u32);
}
pub trait EmitHooksM {

    fn put_output_m(
        &mut self,
        user: u32,
        msg: u32,
        len_in: u32,
        n_in: u32,
        markers: (u32,u32), mem: &mut [u8]) -> (u32, Option<u32>, u32, u32);

    fn get_avail_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> (u32, u32, u8);

    fn get_buf_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> (u32, Option<u32>, u32);

    fn get_pos_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> (u32, u32);

    fn consume_m(&mut self, a: u32, b: u32, n: u32, mem: &mut [u8]) -> u32;

    fn realloc_markers_m(&mut self, mem: &mut [u8]) -> Option<u32>;
}

const OUT_LOCAL: u32 = 0x1f00;
const OUT_REQUEST: u32 = 0x1f20;
const OUT_SCRATCH: u32 = 0x1f40;
const OUT_CTX: u32 = 0x1f54;
const OUT_PARAM: u32 = 0x1f60;

fn fn_183258_m(m:&mut Memory<'_>,p:u32,kind:u32,vfp:engine::NotificationRegisters)->u32 {
    let vector=p.wrapping_add(0x61cc);
    let begin=m.word(vector.wrapping_add(4));
    let end=m.word(vector.wrapping_add(8));
    let capacity=m.word(vector.wrapping_add(12));
    let count=end.wrapping_sub(begin) as i32>>5;
    let reserved=capacity&0x3fffffff;

    let tick=if end!=0 {
        let last=end.wrapping_sub(32);
        if last>=begin {
            m.word(last.wrapping_add(8)) as u64|((m.word(last.wrapping_add(12)) as u64)<<32)
        } else {0}
    } else {0};
    if (reserved as i32)<=count {
        let grown=if reserved==0 {1} else {reserved.wrapping_mul(2)};
        if (reserved as i32)<(grown as i32) {

            return crate::boot_part_b2::fn_183258_live(m,p,kind,vfp.s16,vfp.s17,tick);
        }
    }
    if end!=0 {
        let first=vfp.s16;
        let tail=vfp.s17;
        for (i,v) in [kind,first,tick as u32,(tick>>32) as u32,0,0,0,tail].into_iter().enumerate(){
            m.put_word(end.wrapping_add(i as u32*4),v);
        }
    }
    m.put_word(vector.wrapping_add(8),end.wrapping_add(32));
    0
}

fn frontend_error(e:crate::frontend_types::Error)->engine::Error {
    match e {
        crate::frontend_types::Error::Engine(rc)=>engine::Error::EngineStatus(rc),
        crate::frontend_types::Error::Capacity{required,available}=>engine::Error::Capacity{required,available},
        crate::frontend_types::Error::InvalidResource=>engine::Error::InvalidSnapshot,
    }
}

pub fn fn_84c160_m(m:&mut Memory<'_>,p:u32,v:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    h.call_m(0x1840fc,&[p+88],m);
    let r=crate::speech_part_3::fn_7e715c(m,p,v);
    h.call_m(0x18423c,&[p+88],m);
    r
}

pub fn fn_1959dc_m(m:&mut Memory<'_>,p:u32,v:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    if v==0 {return p;}
    let rc=h.call_m(0x176f2c,&[m.word(p.wrapping_add(0x24))],m);
    if rc>=0x80000000 {return rc;}
    m.put_byte(p.wrapping_add(0x28),v as u8);
    h.call_m(0x176f5c,&[m.word(p.wrapping_add(0x24))],m)
}

pub fn fn_196428_m(m:&mut Memory<'_>,a:u32,o:u32,h:&mut impl crate::reentrant::HooksM) {
    if a==0||o==0 {return;}
    let tgt=m.word(m.word(a).wrapping_add(0xc));
    h.call_m(tgt,&[m.word(a.wrapping_add(4)),o],m);
}

pub fn fn_198fb4_m(m:&mut Memory<'_>,c:u32,n:u32,e:u32,h:&mut impl crate::reentrant::HooksM) {
    if n==e {return;}
    fn_198fb4_m(m,c,m.word(n),e,h);
    fn_198fb4_m(m,c,m.word(n.wrapping_add(4)),e,h);
    let av=m.word(n.wrapping_add(0x10));
    if av!=0 {let t=m.word(m.word(c.wrapping_add(0x24)).wrapping_add(4));h.call_m(t,&[av,m.word(c.wrapping_add(0x20))],m);}
    let bv=m.word(n.wrapping_add(0x14));
    if bv!=0 {let t=m.word(m.word(c.wrapping_add(0x24)).wrapping_add(0x10));h.call_m(t,&[bv,m.word(c.wrapping_add(0x20))],m);}
    fn_196428_m(m,m.word(c.wrapping_add(0x20)),n,h);
}

pub fn fn_19869c_m(m:&mut Memory<'_>,t:u32,h:&mut impl crate::reentrant::HooksM) {
    let n=m.word(t);
    if n!=t {
        fn_198fb4_m(m,t,m.word(n),t,h);
        fn_198fb4_m(m,t,m.word(n.wrapping_add(4)),t,h);
        let pv=m.word(n.wrapping_add(0x10));
        if pv!=0 {let f=m.word(m.word(t.wrapping_add(0x24)).wrapping_add(4));h.call_m(f,&[pv,m.word(t.wrapping_add(0x20))],m);}
        let qv=m.word(n.wrapping_add(0x14));
        if qv!=0 {let f=m.word(m.word(t.wrapping_add(0x24)).wrapping_add(0x10));h.call_m(f,&[qv,m.word(t.wrapping_add(0x20))],m);}
        fn_196428_m(m,m.word(t.wrapping_add(0x20)),n,h);
    }
    m.put_word(t.wrapping_add(0x18),0);
    m.put_word(t, t);
    m.put_word(t.wrapping_add(4), t);
}

pub fn fn_19665c_m(m:&mut Memory<'_>,p:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    if p==0 {return 0x89002007;}
    fn_19869c_m(m,p.wrapping_add(0x18),h);
    fn_196428_m(m,m.word(p.wrapping_add(0x14)),p,h);
    0
}

pub fn fn_174b14_m(m:&mut Memory<'_>,s:u32,h:&mut impl crate::reentrant::HooksM) {
    let q=m.word(s.wrapping_add(4));
    if q!=0 {fn_19665c_m(m,q,h);m.put_word(s.wrapping_add(4),0);}
}

pub fn fn_196fbc_m(m:&mut Memory<'_>,a:u32,e:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    if a==0 {return 0;}
    let data=m.word(a);let sz=m.word(a.wrapping_add(4)) as u16 as u32;let cnt=m.word(a.wrapping_add(0x14));
    let end=data.wrapping_add(cnt.wrapping_mul(sz));
    if e==0||data>e||e>end {return 0;}
    let dtor=m.word(a.wrapping_add(0xc));
    if dtor!=0 {h.call_m(dtor,&[e,m.word(a.wrapping_add(0x1c))],m);}
    let next=e.wrapping_add(sz);let n=end.wrapping_sub(next);
    if n>0 {m.bytes.copy_within(next as usize..(next as usize).wrapping_add(n as usize),e as usize);}
    m.put_word(a.wrapping_add(0x14),cnt.wrapping_sub(1));
    1
}

pub fn fn_11da30_m(m:&mut Memory<'_>,x:u32,h:&mut impl crate::reentrant::HooksM,rom:crate::rom_region::Rom) {
    for i in 0..3 {
        if i == 2 && m.byte(crate::rom_region::addr(rom,0xa7b8e8)+1) == 0 {break;}
        let o=m.word(crate::rom_region::addr(rom,0xb6f4ec)+i*4);
        if o == 0 {continue;}
        let accepts=m.word(m.word(o)+0x18);
        if h.call_m(accepts,&[o,x],m) == 0 {continue;}
        h.call_m(0x1840fc,&[o+88],m);
        h.call_m(0x184f84,&[o+24,x],m);
        m.put_word(o+84,m.word(o+84).wrapping_sub(1));
        h.call_m(0x18423c,&[o+88],m);
        break;
    }
}

pub fn fn_18f490_m(m:&mut Memory<'_>,st:u32,ob:u32,tags:u32,ntags:u32)->Result<u32,engine::Error> {
    let mode=m.byte(st+0x35);
    let limit=m.word(st+0x14c);
    let out=m.word(ob+0x10);
    let mut j=0u32;
    for i in 0..ntags {
        let base=tags.wrapping_add(i.wrapping_mul(24));
        let at=out.wrapping_add(j.wrapping_mul(40));
        if base.wrapping_add(24)>m.bytes.len() as u32||at.wrapping_add(40)>m.bytes.len() as u32 {
            return Err(engine::Error::Capacity{required:base.wrapping_add(24) as usize,available:m.bytes.len()});
        }
        let tag=m.half(base);
        let f04=m.word(base+4);
        let f08=m.word(base+8);
        let f0c=m.word(base+0xc);
        let f10=m.word(base+0x10);
        let f14=m.word(base+0x14);

        m.put_half(at+0x18,0);
        m.put_word(at+0x20,0);
        m.put_word(at+0x1c,0);
        m.put_word(at,0);
        let flags=match tag {
            0x4000=>{if f08==0 {continue;} 1}
            0x28=>{
                if f14 as usize>=m.bytes.len() {
                    return Err(engine::Error::Capacity{required:f14 as usize,available:m.bytes.len()});
                }
                let text=&mut m.bytes[f14 as usize..];
                crate::frontend_runtime::utf16_to_ascii_in_place(text,0).map_err(frontend_error)?;
                m.put_word(at+0x24,f14);
                0x400
            }
            0x21=>{m.put_half(at+0x18,f14 as u16);4}
            0x1d=>{
                if f14 as usize>=m.bytes.len() {
                    return Err(engine::Error::Capacity{required:f14 as usize,available:m.bytes.len()});
                }
                let value=crate::frontend_runtime::parse_ascii_int_in_place(&m.bytes[f14 as usize..],0);
                m.put_word(at+0x1c,value as u32);
                8
            }
            1=>2,
            8=>{m.put_word(at+0x20,f14);0x10}
            0x19=>{m.put_word(at+0x20,f14);0x20}
            _=>continue,
        };
        m.put_half(at+4,flags);
        m.put_word(at+8,f04);
        m.put_word(at+0xc,f08);
        if mode==2 {
            let mut value08=m.word(at+8);
            let mut value0c=m.word(at+0xc);

            if flags==1&&value08<limit {
                let distance=limit-value08;
                value0c=if distance<value0c {value0c-distance} else {0};
            }
            value08=if value08>limit {value08-limit} else {0};
            m.put_word(at+8,value08);
            m.put_word(at+0xc,value0c);
        }
        m.put_word(at+0x10,f0c);
        m.put_word(at+0x14,f10);
        j+=1;
    }
    m.put_word(ob+0xc,j);
    Ok(0)
}

struct QueueRuntime{registers:engine::NotificationRegisters}
impl RuntimeServicesM for QueueRuntime {
    fn invoke_m(&mut self,m:&mut Memory<'_>,entry:u32,a:&[u32])->Result<u32,engine::Error>{
        match entry {
            0x183258=>Ok(fn_183258_m(m,a[0],a[1],self.registers)),
            _=>Err(engine::Error::UnsupportedStage{entry}),
        }
    }
}

struct OutputSide<'a,'h> {
    host: &'a core::cell::RefCell<Host<'h>>,
    synth: u32,
}

impl OutputSide<'_, '_> {

    fn context(&self,m:&Memory<'_>)->u32 { m.word(self.synth.wrapping_add(0x84)) }

    fn log(&mut self,m:&mut Memory<'_>,fmt:u32,values:&[u32]) {
        let logger=m.word(self.context(m).wrapping_add(16));
        self.host.borrow_mut().log_m(logger,0x8d7994,4,fmt,values,m);
    }

    fn vt_entry(&self,m:&Memory<'_>,off:u32)->u32 {
        m.word(m.word(self.synth.wrapping_add(0x40)).wrapping_add(off))
    }
}

impl EmitHooksM for OutputSide<'_, '_> {

    fn put_output_m(&mut self,user:u32,msg:u32,len_in:u32,n_in:u32,markers:(u32,u32),mem:&mut [u8])->(u32,Option<u32>,u32,u32) {
        let mut m=Memory{base:0,bytes:mem};
        let o=self.synth.wrapping_add(0x84);
        let mut samples=m.word(o.wrapping_add(0x60));
        let mut count=len_in;
        let mut tag_count=n_in;
        let first=msg==1;
        if first { self.log(&mut m,0x1534d0,&[]); }
        let buffer=m.word(user.wrapping_add(0x160));
        if buffer!=0 {
            if count!=0||tag_count!=0 {

                m.put_word(user.wrapping_add(0x15c),16);
                m.put_word(buffer.wrapping_add(4),count.wrapping_mul(2));
                m.put_word(buffer.wrapping_add(0xc),tag_count);
                let rc=self.host.borrow_mut().call(&mut m,0x18f490,&[user,buffer,markers.0,tag_count]);
                if rc>=0x80000000 {return (rc,None,0,0);}
                if msg&2!=0 {return (0,None,0,0);}
            } else if msg&2!=0 {
                m.put_word(user.wrapping_add(0x15c),2);
                return (0,Some(0),0,0);
            }
            let buf=m.word(buffer.wrapping_add(8));
            let bytes=m.word(user.wrapping_add(0x164));
            let tags=m.word(user.wrapping_add(0x168))/40;
            m.put_word(user.wrapping_add(0x18c),m.word(buffer.wrapping_add(16)));
            m.put_word(user.wrapping_add(0x188),tags);
            return (0,Some(buf),bytes/2,tags);
        }

        let stopped=m.byte(user.wrapping_add(0x28))==6;
        if stopped {
            m.put_word(user.wrapping_add(0x18c),0);
            m.put_word(user.wrapping_add(0x188),0);
            samples=0;count=0;tag_count=0;
        } else {

            for i in 0..20 { m.put_byte(OUT_LOCAL+i,0); }
            m.put_word(OUT_REQUEST,8);
            m.put_word(OUT_REQUEST+4,0);
            m.put_word(OUT_REQUEST+8,OUT_LOCAL);
            let entry=m.word(user.wrapping_add(0x170));
            let context=m.word(user.wrapping_add(0x16c));
            let argument=m.word(user.wrapping_add(0x184));
            let status=self.host.borrow_mut().call(&mut m,entry,&[user,0x684f424a,context,OUT_REQUEST,argument]);
            let rc=if status!=0 {(status&0x1fff)|0x80002000} else {0};
            samples=m.word(OUT_LOCAL+8);
            count=m.word(OUT_LOCAL+4)/2;
            if samples!=0&&count!=0&&rc<0x80000000 {
                let tags=m.word(OUT_LOCAL+12)/40;
                m.put_word(user.wrapping_add(0x18c),m.word(OUT_LOCAL+16));
                m.put_word(user.wrapping_add(0x188),tags);
                tag_count=tags;
            } else {
                m.put_word(user.wrapping_add(0x18c),0);
                samples=0;count=0;tag_count=0;
            }
            if rc>=0x80000000 {
                if first { self.log(&mut m,0x1534f0,&[rc]); }
                return (rc,Some(samples),count,tag_count);
            }
        }
        if first {
            if samples==0||count==0 {
                self.log(&mut m,0x153540,&[]);
            } else {

                let params=m.word(self.context(&m).wrapping_add(0x14));
                let name=crate::rom_region::addr(self.host.borrow().rom,0x151f48);
                let rc=self.host.borrow_mut().call(&mut m,0x1758ac,&[params,name,count]);
                if rc>=0x80000000 {
                    return (rc,Some(samples),count,tag_count);
                }
                let values=[samples,count,tag_count];
                let logger=m.word(self.context(&m).wrapping_add(16));
                self.host.borrow_mut().log_m(logger,0x8d7994,4,0x153518,&values,&mut m);
            }
        }
        (0,Some(samples),count,tag_count)
    }
    fn get_avail_m(&mut self,a:u32,b:u32,mem:&mut [u8])->(u32,u32,u8) {
        let mut m=Memory{base:0,bytes:mem};
        let entry=self.vt_entry(&m,0x48);
        m.put_word(OUT_SCRATCH,0);m.put_byte(OUT_SCRATCH+4,0);
        let rc=self.host.borrow_mut().call(&mut m,entry,&[a,b,OUT_SCRATCH,OUT_SCRATCH+4]);
        (rc,m.word(OUT_SCRATCH),m.byte(OUT_SCRATCH+4))
    }
    fn get_buf_m(&mut self,a:u32,b:u32,mem:&mut [u8])->(u32,Option<u32>,u32) {
        let mut m=Memory{base:0,bytes:mem};
        let entry=self.vt_entry(&m,0x2c);
        m.put_word(OUT_SCRATCH+8,0);m.put_word(OUT_SCRATCH+12,0);
        let rc=self.host.borrow_mut().call(&mut m,entry,&[a,b,OUT_SCRATCH+8,OUT_SCRATCH+12]);
        let buf=m.word(OUT_SCRATCH+8);
        (rc,if buf==0 {None} else {Some(buf)},m.word(OUT_SCRATCH+12))
    }
    fn get_pos_m(&mut self,a:u32,b:u32,mem:&mut [u8])->(u32,u32) {
        let mut m=Memory{base:0,bytes:mem};
        let entry=self.vt_entry(&m,0x44);
        m.put_word(OUT_SCRATCH+16,0);
        let rc=self.host.borrow_mut().call(&mut m,entry,&[a,b,OUT_SCRATCH+16]);
        (rc,m.word(OUT_SCRATCH+16))
    }
    fn consume_m(&mut self,a:u32,b:u32,n:u32,mem:&mut [u8])->u32 {
        let mut m=Memory{base:0,bytes:mem};
        let entry=self.vt_entry(&m,0x30);
        self.host.borrow_mut().call(&mut m,entry,&[a,b,n])
    }
    fn realloc_markers_m(&mut self,_mem:&mut [u8])->Option<u32> {

        None
    }
}

pub trait SynthTeardownHooksM {

    fn close_stream_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> u32;

    fn process_end_m(&mut self, h0: u32, h1: u32, mem: &mut [u8]) -> u32;

    fn release_stream_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> u32;

    fn null_pair_m(&mut self, mem: &mut [u8]) -> (u32, u32);

    fn lex_close_m(&mut self, params: u32, lex: u32, mem: &mut [u8]) -> u32;

    fn lex_free_m(&mut self, params: u32, lex: u32, mem: &mut [u8]) -> u32;

    fn retire_pipeline_m(&mut self, cs: u32, mem: &mut [u8]) -> u32;

    fn reopen_pipeline_cs_m(&mut self, cs: u32, mem: &mut [u8]) -> u32;
}
pub fn open_audio_m<H: AudioOpenHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
    tag: u32,
    a: u32,
    b: u32,
) -> u32 {
    if tag != 0x34ea || base.is_none() {
        return 0x8b902008;
    }
    let base = base.unwrap();
    mem_store(mem, base.wrapping_add(68), 0);
    mem_store(mem, base.wrapping_add(72), 0);
    mem_store(mem, base.wrapping_add(76), 0);
    if mem_word(mem, base.wrapping_add(48)) == 0 {
        return 0;
    }
    let mut name = [0u8; 64];
    name[..15].copy_from_slice(b"audio/L16;rate=");
    let rate = mem_word(mem, base.wrapping_add(60));
    let _ = render_unsigned(rate, &mut name[15..], 10);
    let context = mem_word(mem, base.wrapping_add(8));
    let logger = mem_word(mem, context.wrapping_add(16));
    let mut h = (0u32, 0u32);
    let mut rc = hooks.stream_open_m(a, b, &name, 0, 0, &mut h, mem);
    mem_store(mem, base.wrapping_add(16), h.0);
    mem_store(mem, base.wrapping_add(20), h.1);
    if rc >= 0x80000000 {
        hooks.audio_open_failed_m(logger, mem);
    }
    rc = hooks.stream_open_m(a, b, &name, 0, 1, &mut h, mem);
    mem_store(mem, base.wrapping_add(24), h.0);
    mem_store(mem, base.wrapping_add(28), h.1);
    if rc >= 0x80000000 {
        hooks.content_open_failed_m(logger, 0xa10, mem);
        return rc;
    }
    const MARKERS: &[u8] = b"application/x-realspeak-markers-pp;version=4.0\0";
    rc = hooks.stream_open_m(a, b, MARKERS, 0, 0, &mut h, mem);
    mem_store(mem, base.wrapping_add(32), h.0);
    mem_store(mem, base.wrapping_add(36), h.1);
    if rc >= 0x80000000 {
        hooks.content_open_failed_m(logger, 0xa0f, mem);
        return rc;
    }
    let mode = hooks.query_marker_mode_m(mem);
    rc = hooks.stream_open_m(a, b, MARKERS, mode, 1, &mut h, mem);
    mem_store(mem, base.wrapping_add(40), h.0);
    mem_store(mem, base.wrapping_add(44), h.1);
    if rc >= 0x80000000 {
        hooks.content_open_failed_m(logger, 0xa10, mem);
    }
    rc
}
pub fn reset_ctx_m<H: CtxResetHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    obj: Option<u32>,
    tag: u32,
) -> u32 {
    let mut ret = validate_handle(obj.is_some(), tag, 0x34ea, 0x74);
    if ret >= 0x80000000 {
        return 0x8b902008;
    }
    let base = obj.unwrap();
    if mem_word(mem, base.wrapping_add(0x50)) != 0 {
        hooks.release_pair_m(
            mem_word(mem, base.wrapping_add(0x50)),
            mem_word(mem, base.wrapping_add(0x54)),
            1, mem);
        let null = hooks.null_pair_m(mem);
        mem_store(mem, base.wrapping_add(0x50), null.0);
        mem_store(mem, base.wrapping_add(0x54), null.1);
    }
    mem_store(mem, base.wrapping_add(0x58), 0);
    mem_store(mem, base.wrapping_add(0x5c), 0);
    for (i, at) in [0x10u32, 0x18, 0x20, 0x28].iter().enumerate() {
        let at = base.wrapping_add(*at);
        if mem_word(mem, at) != 0 {
            let e = hooks.free_pair_m(mem_word(mem, at), mem_word(mem, at.wrapping_add(4)), mem);
            if e >= 0x80000000 && (i == 0 || ret < 0x80000000) {
                ret = e;
            }
            let null = hooks.null_pair_m(mem);
            mem_store(mem, at, null.0);
            mem_store(mem, at.wrapping_add(4), null.1);
        }
    }
    if mem_word(mem, base.wrapping_add(0x30)) != 0 {
        hooks.detach_pair_m(
            mem_word(mem, base.wrapping_add(0x34)),
            mem_word(mem, base.wrapping_add(0x38)), mem);
    }
    ret
}
pub fn call_method8_m<H: VMethodHooksM>(hooks: &mut H, mem: &mut [u8], base: u32, q: u32) -> u32 {
    if validate_handle(true, q, 0xf383, 0x20) >= 0x80000000 {
        return 0x8bb02008;
    }
    hooks.call_method_m(
        mem_word(mem, base.wrapping_add(12)),
        mem_word(mem, base.wrapping_add(16)), mem)
}
trait StatusM {fn set_m(&mut self,value:u8,mem:&mut [u8]);}
struct LocalStatus<'a>(&'a mut u8);
impl StatusM for LocalStatus<'_>{fn set_m(&mut self,v:u8,_:&mut [u8]){*self.0=v;}}
struct GuestStatus(u32);
impl StatusM for GuestStatus{fn set_m(&mut self,v:u8,m:&mut [u8]){m[self.0 as usize]=v;}}
pub fn pump_insertion_m<H:InserterPumpHooksM>(h:&mut H,mem:&mut [u8],p:u32,key:u32,status:&mut u8)->u32 {
    pump_insertion_impl(h,mem,p,key,&mut LocalStatus(status))
}

pub fn pump_insertion_at_m<H:InserterPumpHooksM>(h:&mut H,mem:&mut [u8],p:u32,key:u32,status:u32)->u32 {
    pump_insertion_impl(h,mem,p,key,&mut GuestStatus(status))
}
fn pump_insertion_impl<H: InserterPumpHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    base: u32,
    check: u32,
    status: &mut impl StatusM,
) -> u32 {
    fn pair(mem: &[u8], at: u32) -> (u32, u32) {
        (mem_word(mem, at), mem_word(mem, at.wrapping_add(4)))
    }
    fn store_pair(mem: &mut [u8], at: u32, value: (u32, u32)) {
        mem_store(mem, at, value.0);
        mem_store(mem, at.wrapping_add(4), value.1);
    }
    if validate_handle(true, check, 0x34ea, 0x74) >= 0x80000000 {
        return 0x8b902008;
    }
    status.set_m(1,mem);
    if mem_word(mem, base.wrapping_add(48)) == 0 {
        return 0;
    }
    if mem_word(mem, base.wrapping_add(80)) != 0 {
        let stepped = hooks.pump_remote_m(mem, base);
        if stepped < 0x80000000 && mem_word(mem, base.wrapping_add(92)) == 0 {
            let mut result = 0;
            if mem_word(mem, base.wrapping_add(80)) != 0 {
                let (i0, i1) = pair(mem, base.wrapping_add(80));
                result = hooks.ins_close_m(i0, i1, 1, mem);
                let null=hooks.null_pair_m(mem);store_pair(mem, base.wrapping_add(80), null);
            }
            mem_store(mem, base.wrapping_add(88), 0);
            mem_store(mem, base.wrapping_add(92), 0);
            status.set_m(2,mem);
            return result;
        }
        status.set_m(2,mem);
        return stepped;
    }
    let (ein0, ein1) = pair(mem, base.wrapping_add(32));
    let mut count = 0u32;
    let mut events_end = 0u8;
    let mut rc = hooks.buf_available_m(ein0, ein1, &mut count, &mut events_end, mem);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut copy_audio = true;
    if count != 0 {
        let mut input = 0u32;
        rc = hooks.buf_read_m(ein0, ein1, &mut input, &mut count, mem);
        if rc >= 0x80000000 {
            return rc;
        }
        let (eout0, eout1) = pair(mem, base.wrapping_add(40));
        let mut output = 0u32;
        rc = hooks.buf_write_m(eout0, eout1, &mut output, count, mem);
        if rc >= 0x80000000 {
            hooks.buf_consume_m(ein0, ein1, 0, mem);
            return rc;
        }
        let events = count / INSERT_EVENT_LEN;
        let mut i = 0u32;
        let mut processed = 0u32;
        while i < events {
            let ie = input.wrapping_add(i.wrapping_mul(INSERT_EVENT_LEN));
            let oe = output.wrapping_add(i.wrapping_mul(INSERT_EVENT_LEN));
            if mem_word(mem, ie.wrapping_add(12)) != mem_word(mem, base.wrapping_add(72)) {
                mem_store(mem, base.wrapping_add(76), 0);
            }
            let mut tmp = [0u8; 24];
            tmp.copy_from_slice(&mem[ie as usize..ie as usize + 24]);
            mem[oe as usize..oe as usize + 24].copy_from_slice(&tmp);
            let pos = mem_word(mem, oe.wrapping_add(12))
                .wrapping_add(mem_word(mem, base.wrapping_add(68)));
            mem_store(mem, oe.wrapping_add(12), pos);
            if mem_half(mem, oe) != 0x21 {
                let pos = mem_word(mem, oe.wrapping_add(12))
                    .wrapping_add(mem_word(mem, base.wrapping_add(76)));
                mem_store(mem, oe.wrapping_add(12), pos);
            }
            processed += 1;
            let itype = mem_half(mem, ie);
            if itype == 8 {
                let ipos = mem_word(mem, ie.wrapping_add(12));
                mem_store(mem, base.wrapping_add(72), ipos);
                let add = mem_word(mem, ie.wrapping_add(20))
                    .wrapping_mul(mem_word(mem, base.wrapping_add(60)))
                    / 1000;
                let acc = mem_word(mem, base.wrapping_add(76)).wrapping_add(add);
                mem_store(mem, base.wrapping_add(76), acc);
            } else if itype == 0x1a {
                let mut position = 0u32;
                let mut available = 0u32;
                let mut audio_end = 0u8;
                if mem_word(mem, base.wrapping_add(16)) != 0 {
                    let (a0, a1) = pair(mem, base.wrapping_add(16));
                    hooks.buf_position_m(a0, a1, &mut position, mem);
                    rc = hooks.buf_available_m(a0, a1, &mut available, &mut audio_end, mem);
                    position >>= 1;
                    available >>= 1;
                } else {
                    audio_end = 1;
                }
                let target = mem_word(mem, base.wrapping_add(76))
                    .wrapping_add(mem_word(mem, ie.wrapping_add(12)));
                if rc >= 0x80000000
                    || (audio_end == 0 && target >= position.wrapping_add(available))
                {
                    processed = processed.wrapping_sub(1);
                    break;
                }
                copy_audio = false;
                if mem_word(mem, base.wrapping_add(16)) != 0 && position < target {
                    let (d0, d1) = pair(mem, base.wrapping_add(24));
                    let (s0, s1) = pair(mem, base.wrapping_add(16));
                    let n = target.wrapping_sub(position).wrapping_mul(2);
                    rc = hooks.buf_copy_m(d0, d1, s0, s1, n, &mut available, mem);
                }
                if audio_end != 0 {
                    let (a0, a1) = pair(mem, base.wrapping_add(24));
                    hooks.buf_position_m(a0, a1, &mut position, mem);
                    position >>= 1;
                    if rc < 0x80000000 && position < mem_word(mem, oe.wrapping_add(12)) {
                        let samples =
                            mem_word(mem, oe.wrapping_add(12)).wrapping_sub(position);
                        let mut silence = 0u32;
                        rc = hooks.buf_write_m(a0, a1, &mut silence, samples.wrapping_mul(2), mem);
                        if rc < 0x80000000 {
                            rc = hooks.buf_commit_m(a0, a1, samples.wrapping_mul(2), mem);
                        }
                    }
                }
                if rc < 0x80000000 {
                    let mut attempt = 1u32;
                    let uri = mem_word(mem, ie.wrapping_add(20));
                    let mut name = uri;
                    rc = 0x8b902014;
                    while rc & 0x1fff == 0x14 && attempt <= 2 {
                        if attempt == 2 {
                            let (s0, s1) = pair(mem, base.wrapping_add(52));
                            let null = hooks.null_pair_m(mem);
                            let mut resolved = 0u32;
                            rc = hooks.ins_resolve_m(s0, s1, uri, null.0, null.1, &mut resolved, mem);
                            if rc >= 0x80000000 {
                                break;
                            }
                            name = resolved;
                        }
                        let (s0, s1) = pair(mem, base.wrapping_add(52));
                        let mut opos = 0u32;
                        let mut orem = 0u32;
                        let mut oins = (0u32, 0u32);
                        rc = hooks.ins_open_m(s0, s1, name, &mut opos, &mut orem, &mut oins, mem);
                        mem_store(mem, base.wrapping_add(88), opos);
                        mem_store(mem, base.wrapping_add(92), orem);
                        store_pair(mem, base.wrapping_add(80), oins);
                        attempt += 1;
                    }
                    if rc < 0x80000000 {
                        rc = hooks.pump_remote_m(mem, base);
                        if rc < 0x80000000 && mem_word(mem, base.wrapping_add(92)) == 0 {
                            rc = 0;
                            if mem_word(mem, base.wrapping_add(80)) != 0 {
                                let (i0, i1) = pair(mem, base.wrapping_add(80));
                                rc = hooks.ins_close_m(i0, i1, 1, mem);
                                let null=hooks.null_pair_m(mem);store_pair(mem, base.wrapping_add(80), null);
                            }
                            mem_store(mem, base.wrapping_add(88), 0);
                            mem_store(mem, base.wrapping_add(92), 0);
                        }
                    }
                }
                status.set_m(2,mem);
                break;
            }
            i += 1;
        }
        if rc < 0x80000000 {
            if processed != 0 {
                loop {
                    let mut swapped = false;
                    let mut j = 0u32;
                    while j.wrapping_add(1) < processed {
                        let a = output.wrapping_add(j.wrapping_mul(INSERT_EVENT_LEN));
                        let b = output.wrapping_add(
                            j.wrapping_add(1).wrapping_mul(INSERT_EVENT_LEN),
                        );
                        if mem_word(mem, b.wrapping_add(12)) < mem_word(mem, a.wrapping_add(12))
                        {
                            let mut ta = [0u8; 24];
                            let mut tb = [0u8; 24];
                            ta.copy_from_slice(&mem[a as usize..a as usize + 24]);
                            tb.copy_from_slice(&mem[b as usize..b as usize + 24]);
                            mem[a as usize..a as usize + 24].copy_from_slice(&tb);
                            mem[b as usize..b as usize + 24].copy_from_slice(&ta);
                            swapped = true;
                        }
                        j += 1;
                    }
                    if !swapped {
                        break;
                    }
                }
            }
            let (e0, e1) = pair(mem, base.wrapping_add(40));
            rc = hooks.buf_commit_m(e0, e1, processed.wrapping_mul(INSERT_EVENT_LEN), mem);
            if rc >= 0x80000000 {
                hooks.buf_consume_m(ein0, ein1, 0, mem);
            } else {
                rc = hooks.buf_consume_m(ein0, ein1, processed.wrapping_mul(INSERT_EVENT_LEN), mem);
            }
        }
        if processed == events && events_end != 0 {
            status.set_m(2,mem);
        }
    } else {
        let mut available = 0u32;
        let mut audio_end = 0u8;
        let mut avail_err = false;
        if mem_word(mem, base.wrapping_add(16)) != 0 {
            let (a0, a1) = pair(mem, base.wrapping_add(16));
            rc = hooks.buf_available_m(a0, a1, &mut available, &mut audio_end, mem);
            if rc >= 0x80000000 {
                avail_err = true;
            }
        } else {
            audio_end = 1;
        }
        if !avail_err && events_end != 0 && audio_end != 0 {
            if mem_word(mem, base.wrapping_add(40)) != 0 {
                let (a0, a1) = pair(mem, base.wrapping_add(24));
                hooks.buf_close_m(a0, a1, mem);
                let null=hooks.null_pair_m(mem);store_pair(mem, base.wrapping_add(24), null);
                let (e0, e1) = pair(mem, base.wrapping_add(40));
                rc = hooks.buf_close_m(e0, e1, mem);
                let null=hooks.null_pair_m(mem);store_pair(mem, base.wrapping_add(40), null);
            }
            copy_audio = false;
        } else if available == 0 {
            copy_audio = false;
            status.set_m(0,mem);
        }
    }
    if rc < 0x80000000 && mem_word(mem, base.wrapping_add(16)) != 0 && copy_audio {
        let (d0, d1) = pair(mem, base.wrapping_add(24));
        let (s0, s1) = pair(mem, base.wrapping_add(16));
        let mut moved = 0u32;
        rc = hooks.buf_copy_m(d0, d1, s0, s1, 0xffffffff, &mut moved, mem);
    }
    rc
}
pub fn emit_output_m<H: EmitHooksM>(
    mem: &mut [u8],
    p: u32,
    sent: Option<u32>,
    hooks: &mut H,
) -> u32 {
    const OUT: u32 = 0x84;
    let o = p.wrapping_add(OUT);
    if let Some(s) = sent {
        mem[s as usize] = 0;
    }
    let put_output = mem_word(mem, p.wrapping_add(0x18));
    if put_output == 0 || mem[o.wrapping_add(0x5e) as usize] == 2 {
        return 0;
    }
    if mem[p.wrapping_add(0x80) as usize] == 2 {
        return 0;
    }
    let user = mem_word(mem, p.wrapping_add(0x28));
    let mut msg_type = 64u32;
    let mut rc;
    let mut out_buf = mem_word(mem, o.wrapping_add(0x60));
    if out_buf == 0 {
        mem[o.wrapping_add(0xc) as usize] = 1;
        mem_store(mem, o.wrapping_add(0x64), 0);
        mem_store(mem, o.wrapping_add(0x68), 0);
        let (r, buf, len, nmark) = hooks.put_output_m(user, 1, 0, 0, (0,0), mem);
        rc = r;
        mem_store(mem, o.wrapping_add(0x60), buf.unwrap_or(0));
        mem_store(mem, o.wrapping_add(0x64), len);
        mem_store(mem, o.wrapping_add(0x68), nmark);
        out_buf = buf.unwrap_or(0);
        if rc >= 0x80000000 {
            return rc;
        }
        if out_buf == 0 || mem_word(mem, o.wrapping_add(0x64)) == 0 {
            return 0x81202807;
        }
    }
    let bits = mem_word(mem, o.wrapping_add(8));
    let pcm_a = mem_word(mem, o.wrapping_add(0x3c));
    let pcm_b = mem_word(mem, o.wrapping_add(0x40));
    let (r, mut pcm_avail, eos) = hooks.get_avail_m(pcm_a, pcm_b, mem);
    rc = r;
    if rc >= 0x80000000 {
        return rc;
    }
    let mut pcm_buf = None;
    if pcm_avail != 0 {
        let (r, buf, len) = hooks.get_buf_m(pcm_a, pcm_b, mem);
        rc = r;
        if rc >= 0x80000000 {
            return rc;
        }
        pcm_buf = buf;
        pcm_avail = len;
    }
    let mk_a = mem_word(mem, o.wrapping_add(0x44));
    let mk_b = mem_word(mem, o.wrapping_add(0x48));
    let (r, mk_buf, mk_len) = hooks.get_buf_m(mk_a, mk_b, mem);
    rc = r;
    if rc >= 0x80000000 {
        return rc;
    }
    let marker_avail = mk_len / 24;
    let (_, pos) = hooks.get_pos_m(pcm_a, pcm_b, mem);
    let mut min_pos = if bits == 16 {
        pos >> 1
    } else if bits == 8 {
        pos
    } else {
        divmod_u32(pos << 3, bits).0
    };
    let flags = mem_word(mem, p.wrapping_add(0x34));
    let mut pcm_used = 0u32;
    let mut marker_used = 0u32;
    while mem[o.wrapping_add(0x5e) as usize] != 2
        && (pcm_avail > pcm_used || marker_avail > marker_used)
    {
        let out_len = mem_word(mem, o.wrapping_add(0x64));
        let mut copy_len = pcm_avail.wrapping_sub(pcm_used);
        let cap = out_len.wrapping_mul(bits) >> 3;
        if copy_len >= cap {
            copy_len = cap;
        }
        let first = match mk_buf {
            Some(mk) => mk.wrapping_add(marker_used.wrapping_mul(24)),
            None => 0,
        };
        let nmark = mem_word(mem, o.wrapping_add(0x68));
        let mut n = 0u32;
        if nmark == 0 || flags == 0 || mk_buf.is_none() || marker_used == marker_avail {
            marker_used = marker_avail;
        } else {
            let mk = mk_buf.unwrap();
            let (_, base) = hooks.get_pos_m(pcm_a, pcm_b, mem);
            let end = if bits == 16 {
                base.wrapping_add(pcm_used).wrapping_add(copy_len) >> 1
            } else if bits == 8 {
                base.wrapping_add(pcm_used).wrapping_add(copy_len)
            } else {
                divmod_u32(base.wrapping_add(pcm_used).wrapping_add(copy_len) << 3, bits).0
            };
            let mk_end = mk.wrapping_add(marker_avail.wrapping_mul(24));
            let mut cur = first;
            let mut max_pos = min_pos;
            let mut cnt = 0u32;
            let mut m = 0u32;
            let mut all = false;
            loop {
                m = filter_markers(mem, flags, cur, mk_end, Some(&mut cnt));
                if m == 0 {
                    all = true;
                    break;
                }
                if cnt.wrapping_add(n) > nmark
                    || (eos == 0 && mem_word(mem, m.wrapping_add(12)) >= end)
                {
                    break;
                }
                n = n.wrapping_add(1);
                cur = m.wrapping_add(24);
                let m_pos = mem_word(mem, m.wrapping_add(12));
                if m_pos > max_pos {
                    max_pos = m_pos;
                }
            }
            if !all {
                let m_pos = mem_word(mem, m.wrapping_add(12));
                if m_pos < end {
                    if m_pos >= max_pos {
                        max_pos = m_pos;
                    }
                    let t = if bits == 16 {
                        max_pos.wrapping_mul(2).wrapping_sub(base.wrapping_add(pcm_used)) as i32
                    } else if bits == 8 {
                        max_pos.wrapping_sub(base.wrapping_add(pcm_used)) as i32
                    } else {
                        (bits.wrapping_mul(max_pos) >> 3).wrapping_sub(base.wrapping_add(pcm_used))
                            as i32
                    };
                    copy_len = if t < 0 { 0 } else { t as u32 };
                    if t <= 0 && nmark < cnt {
                        loop {
                            m = filter_markers(mem, flags, cur, mk_end, None);
                            if m == 0 {
                                all = true;
                                break;
                            }
                            if nmark <= n {
                                break;
                            }
                            n = n.wrapping_add(1);
                            cur = m.wrapping_add(24);
                        }
                    }
                }
            }
            if all {
                marker_used = marker_avail;
            } else {
                marker_used = m.wrapping_sub(mk) / 24;
            }
        }
        if copy_len == 0 && n == 0 {
            if rc >= 0x80000000 {
                return rc;
            }
            break;
        }
        let cap_now = mem_word(mem, o.wrapping_add(0x74));
        if cap_now < n {
            match hooks.realloc_markers_m(mem) {
                Some(arr) => {
                    mem_store(mem, o.wrapping_add(0x78), arr);
                    mem_store(mem, o.wrapping_add(0x74), n);
                }
                None => return 0x8120200a,
            }
        }
        if n != 0 {
            let arr = mem_word(mem, o.wrapping_add(0x78));
            let pos_offset = mem_word(mem, o.wrapping_add(0x70));
            let mk = mk_buf.unwrap();
            let mk_end = mk.wrapping_add(marker_avail.wrapping_mul(24));
            let mut cur = first;
            let mut i = 0u32;
            while i < n {
                let m = filter_markers(mem, flags, cur, mk_end, None);
                if m == 0 {
                    break;
                }
                let dst = arr.wrapping_add(i.wrapping_mul(24)) as usize;
                mem.copy_within(m as usize..m as usize + 24, dst);
                let at = dst.wrapping_add(12);
                let v = u32::from_le_bytes(mem[at..at + 4].try_into().unwrap());
                let wpos = v.wrapping_add(pos_offset);
                mem[at..at + 4].copy_from_slice(&wpos.to_le_bytes());
                if wpos < min_pos {
                    mem[at..at + 4].copy_from_slice(&min_pos.to_le_bytes());
                } else {
                    min_pos = wpos;
                }
                cur = m.wrapping_add(24);
                i = i.wrapping_add(1);
            }
        }
        mem[o.wrapping_add(0xc) as usize] = 1;
        if let Some(s) = sent {
            mem[s as usize] = 1;
        }
        mem_store(mem, o.wrapping_add(0x50), 0);
        if copy_len != 0 {
            let src_us = pcm_buf.unwrap() as usize + pcm_used as usize;
            let dst_us = out_buf as usize;
            let count = copy_len as usize;
            if src_us < dst_us {
                let (head, tail) = mem.split_at_mut(dst_us);
                copy_bytes(&mut tail[..count], &head[src_us..src_us + count], copy_len);
            } else if src_us > dst_us {
                let (head, tail) = mem.split_at_mut(src_us);
                copy_bytes(&mut head[dst_us..dst_us + count], &tail[..count], copy_len);
            }
            pcm_used = pcm_used.wrapping_add(copy_len);
        }
        let new_len = if bits == 16 {
            copy_len >> 1
        } else if bits == 8 {
            copy_len
        } else {
            divmod_u32(copy_len << 3, bits).0
        };
        mem_store(mem, o.wrapping_add(0x64), new_len);
        mem_store(mem, o.wrapping_add(0x68), n);
        let restart = mem[o.wrapping_add(0x4c) as usize];
        if restart == 1 {
            mem[o.wrapping_add(0x4c) as usize] = 0;
        }
        if let Some(mk) = mk_buf {
            let mut j = first;

            let stop = mk.wrapping_add(marker_avail.wrapping_mul(24));
            while j != stop {
                let ty = u16::from_le_bytes(mem[j as usize..j as usize + 2].try_into().unwrap());
                let arg = mem_word(mem, j.wrapping_add(8));
                if ty == 0x4000 && arg != 0 {
                    msg_type = 32;
                    break;
                }
                j = j.wrapping_add(24);
            }
            if eos != 0 && pcm_used == pcm_avail {
                if msg_type == 64 {
                    msg_type = 128;
                } else {
                    msg_type |= 128;
                }
            }
        }
        let arr_now = mem_word(mem, o.wrapping_add(0x78));
        let n_now = mem_word(mem, o.wrapping_add(0x68));
        let take = (n_now as usize).saturating_mul(24);
        let (r, buf, len, nmark) = hooks.put_output_m(
            user,
            msg_type,
            mem_word(mem, o.wrapping_add(0x64)),
            n_now,
            (arr_now,take as u32), mem);
        rc = r;
        mem_store(mem, o.wrapping_add(0x60), buf.unwrap_or(0));
        mem_store(mem, o.wrapping_add(0x64), len);
        mem_store(mem, o.wrapping_add(0x68), nmark);
        out_buf = buf.unwrap_or(0);
        if rc >= 0x80000000 {

        } else if out_buf == 0 || mem_word(mem, o.wrapping_add(0x64)) == 0 {
            rc = 0x81202807;
        }
        if mem[o.wrapping_add(0x5e) as usize] == 1 {
            mem[o.wrapping_add(0x5e) as usize] = 2;
            mem_store(mem, o.wrapping_add(0x60), 0);
            out_buf = 0;
        }
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if pcm_used != 0 {
        rc = hooks.consume_m(pcm_a, pcm_b, pcm_used, mem);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if marker_used != 0 {
        rc = hooks.consume_m(mk_a, mk_b, marker_used.wrapping_mul(24), mem);
    }
    rc
}
pub fn teardown_synth_m<H: SynthTeardownHooksM + EmitHooksM>(
    mem: &mut [u8],
    p: u32,
    _err: u32,
    _in_len: u32,
    _dur: u32,
    hooks: &mut H,
) -> u32 {
    const OUT: u32 = 0x84;
    let o = p.wrapping_add(OUT);
    let bits = mem_word(mem, o.wrapping_add(8));
    let user = mem_word(mem, p.wrapping_add(0x28));
    let ctx = mem_word(mem, o);
    let cur = mem_half(mem, o.wrapping_add(0x4e)) as u32;
    let cell = RefCell::new(hooks);
    let mut rc = close_components_m(
        mem,
        p,
        0,
        cur,
        |_, _, _, _| {},
        |a, b, mem| {
            cell.borrow_mut().close_stream_m(a, b, mem);
        },
        |h0, h1, mem| cell.borrow_mut().process_end_m(h0, h1, mem),
        |_, _, _, _| {},
    );
    let hooks = cell.into_inner();
    let pp_a = mem_word(mem, o.wrapping_add(0x28));
    let pp_b = mem_word(mem, o.wrapping_add(0x2c));
    if pp_a != 0 {
        let rc2 = hooks.close_stream_m(pp_a, pp_b, mem);
        let (t0, t1) = hooks.null_pair_m(mem);
        mem_store(mem, o.wrapping_add(0x28), t0);
        mem_store(mem, o.wrapping_add(0x2c), t1);
        if rc < 0x80000000 && rc2 >= 0x80000000 {
            rc = rc2;
        }
    }
    let pcm_a = mem_word(mem, o.wrapping_add(0x3c));
    let pcm_b = mem_word(mem, o.wrapping_add(0x40));
    let mk_a = mem_word(mem, o.wrapping_add(0x44));
    let mk_b = mem_word(mem, o.wrapping_add(0x48));
    let mut rc2 = 0u32;
    if pcm_a != 0 {
        let r = emit_output_m(mem, p, None, &mut *hooks);
        if r >= 0x80000000 {
            rc2 = r;
        }
        let (r, pos) = hooks.get_pos_m(pcm_a, pcm_b, mem);
        if r < 0x80000000 {
            let off = mem_word(mem, o.wrapping_add(0x70));
            mem_store(
                mem,
                o.wrapping_add(0x70),
                off.wrapping_add(divmod_u32(pos << 3, bits).0),
            );
        } else if rc2 < 0x80000000 {
            rc2 = r;
        }
        let r = hooks.close_stream_m(pcm_a, pcm_b, mem);
        let (t0, t1) = hooks.null_pair_m(mem);
        mem_store(mem, o.wrapping_add(0x3c), t0);
        mem_store(mem, o.wrapping_add(0x40), t1);
        if rc2 < 0x80000000 && r >= 0x80000000 {
            rc2 = r;
        }
    }
    if mk_a != 0 {
        let r = hooks.close_stream_m(mk_a, mk_b, mem);
        let (t0, t1) = hooks.null_pair_m(mem);
        mem_store(mem, o.wrapping_add(0x44), t0);
        mem_store(mem, o.wrapping_add(0x48), t1);
        if rc2 < 0x80000000 && r >= 0x80000000 {
            rc2 = r;
        }
    }
    if rc < 0x80000000 && rc2 >= 0x80000000 {
        rc = rc2;
    }
    if rc < 0x80000000 || mem[o.wrapping_add(0xc) as usize] != 0 {
        mem_store(mem, o.wrapping_add(0x64), 0);
        mem_store(mem, o.wrapping_add(0x68), 0);
        let mut r = 0u32;
        if mem_word(mem, p.wrapping_add(0x18)) != 0 {
            let (rr, buf, len, nmark) = hooks.put_output_m(user, 2, 0, 0, (0,0), mem);
            r = rr;
            mem_store(mem, o.wrapping_add(0x60), buf.unwrap_or(0));
            mem_store(mem, o.wrapping_add(0x64), len);
            mem_store(mem, o.wrapping_add(0x68), nmark);
        }
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    if mem_word(mem, o.wrapping_add(0x78)) != 0 {
        mem_store(mem, o.wrapping_add(0x78), 0);
    }
    let new_cs = mem_word(mem, o.wrapping_add(0xb8));
    if new_cs != 0 {
        let r = hooks.retire_pipeline_m(mem_word(mem, p.wrapping_add(0x7c)), mem);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
        mem_store(mem, p.wrapping_add(0x7c), new_cs);
        mem_store(mem, o.wrapping_add(0xb8), 0);
        let r = hooks.reopen_pipeline_cs_m(new_cs, mem);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    if mem[o.wrapping_add(0x18) as usize] != 0 {
        let r = hooks.release_stream_m(
            mem_word(mem, o.wrapping_add(0x10)),
            mem_word(mem, o.wrapping_add(0x14)), mem);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    if mem[o.wrapping_add(0x24) as usize] != 0 {
        let r = hooks.release_stream_m(
            mem_word(mem, o.wrapping_add(0x1c)),
            mem_word(mem, o.wrapping_add(0x20)), mem);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    let lex = mem_word(mem, o.wrapping_add(0x30));
    if lex != 0 {
        let params = mem_word(mem, ctx.wrapping_add(20));
        let r = hooks.lex_close_m(params, lex, mem);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
        let r = hooks.lex_free_m(params, lex, mem);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    rc
}
pub fn close_components_m(
    mem: &mut [u8],
    synth: u32,
    start: u32,
    end: u32,
    mut begin: impl FnMut(u32, u32, u32, &mut [u8]),
    mut close: impl FnMut(u32, u32, &mut [u8]),
    mut process_end: impl FnMut(u32, u32, &mut [u8]) -> u32,
    mut end_stage: impl FnMut(u32, u32, u32, &mut [u8]),
) -> u32 {
    let at = (synth + 0xd2) as usize;
    mem[at..at + 2].copy_from_slice(&(start as u16).to_le_bytes());
    let cs = mem_word(mem, synth + 0x7c);
    let comps = mem_word(mem, cs + 8);
    let obj = mem_word(mem, cs + 16);
    let arg = mem_word(mem, cs + 20);
    let status = mem[(synth + 0x80) as usize];
    let pending = synth + 0x118;
    let mark_a = mem_word(mem, synth + 0x4c);
    let mark_b = mem_word(mem, synth + 0x50);
    let mut rc = 0u32;
    let mut s = start;
    while s < end {
        if obj != 0 {
            begin(obj, arg, s.wrapping_add(2), mem);
        }
        let comp = mem_word(mem, comps.wrapping_add(s.wrapping_mul(4)));
        if status == 2 && compare_str(terminated(mem, mem_word(mem, comp + 4)), terminated(mem,pending)) == 0 {
            close(mark_a, mark_b, mem);
        }
        let r2 = process_end(mem_word(mem, comp + 12), mem_word(mem, comp + 16), mem);
        if obj != 0 {
            end_stage(obj, arg, s.wrapping_add(2), mem);
        }
        if r2 >= 0x80000000 && rc < 0x80000000 {
            rc = r2;
        }
        s = s.wrapping_add(1);
    }
    if mem_word(mem, synth + 0x114) != 0 {
        mem_store(mem, synth + 0x114, 0);
    }
    rc
}

macro_rules! drain_validate_end {
    ($name:ident, $tag:expr, $size:expr, $err:expr) => {
        fn $name(m: &mut Memory<'_>, p: u32, tag: u32, _host: &mut Host<'_>) -> u32 {
            let r = crate::frontend::validate_handle(p != 0, tag, $tag, $size);
            if r >= 0x80000000 {
                return $err;
            }
            r
        }
    };
}
drain_validate_end!(fn_1f27ec_m, 0xf385, 0x20, 0x89e02008);
drain_validate_end!(fn_1f3a8c_m, 0xf386, 0x30, 0x89f02008);
drain_validate_end!(fn_1e03ac_m, 0xf387, 40, 0x8a002008);
drain_validate_end!(fn_1ea328_m, 0xf3b3, 60, 0x8b802008);
drain_validate_end!(fn_1f7e8c_m, 0xf388, 64, 0x8a102008);

fn fn_207130_m(m: &mut Memory<'_>, p: u32, tag: u32, _host: &mut Host<'_>) -> u32 {
    let r = crate::frontend::validate_handle(p != 0, tag, 0xf382, 0x2c);
    if r >= 0x80000000 {
        return 0x89b02008;
    }
    r
}

fn fn_1de844_m(m: &mut Memory<'_>, p: u32, tag: u32, _host: &mut Host<'_>) -> u32 {
    let r = crate::frontend::validate_handle(p != 0, tag, 0xf383, 56);
    if r >= 0x80000000 {
        return 0x89c02008;
    }
    r
}

fn fn_1fd6d0_m(m: &mut Memory<'_>, p: u32, slot: u32, host: &mut Host<'_>) -> u32 {
    let r = host.call(m, 0x176f94, &[p, slot, 0xf384, 0x4c8]);
    if r >= 0x80000000 {
        return 0x89b02008;
    }
    if r <= 1 {
        m.put_word(slot, m.word(crate::rom_region::addr(host.rom, 0x8f9590)));
        return 0;
    }
    0x89b02001
}

fn fn_1f4cb0_m(m: &mut Memory<'_>, p: u32, tag: u32, host: &mut Host<'_>) -> u32 {
    let rc = crate::frontend::validate_handle(p != 0, tag, 0xf424, 0x40);
    if rc >= 0x80000000 {
        return 0x8c802008;
    }
    let mut rc = rc;
    for start in [32u32, 48u32] {
        for i in 0..2u32 {
            let at = p.wrapping_add(start).wrapping_add(i * 8);
            if m.word(at) != 0 {
                let rel = m.word(m.word(p.wrapping_add(4)).wrapping_add(0x24));
                rc = host.call(m, rel, &[m.word(at), m.word(at.wrapping_add(4))]);
                m.put_word(at, 0);
                m.put_word(at.wrapping_add(4), 0);
            }
        }
    }
    rc
}

fn fn_22b238_m(m: &mut Memory<'_>, p: u32, arg: u32, host: &mut Host<'_>) -> u32 {
    let result = host.call(m, 0x176f94, &[p, arg, 0x10d, 0x38]);
    if result >= 0x80000000 {
        return result;
    }
    let mut result = result;
    let e0 = m.word(crate::rom_region::addr(host.rom,0x8d7660));
    let e1 = m.word(crate::rom_region::addr(host.rom,0x8d7664));
    let f0 = m.word(p.wrapping_add(0x24));
    let f1 = m.word(p.wrapping_add(0x28));
    if crate::frontend::match_halves(f0, f1, e0, e1) == 0 {
        let rel = m.word(m.word(p.wrapping_add(0x20)).wrapping_add(0x24));
        let status = host.call(m, rel, &[f0, f1]);
        if status >= 0x80000000 {
            result = status;
        }
    }
    m.put_word(p.wrapping_add(0x24), e0);
    m.put_word(p.wrapping_add(0x28), e1);
    let s0 = m.word(p.wrapping_add(0x2c));
    let s1 = m.word(p.wrapping_add(0x30));
    if crate::frontend::match_halves(s0, s1, e0, e1) == 0 {
        let rel = m.word(m.word(p.wrapping_add(0x20)).wrapping_add(0x24));
        let status = host.call(m, rel, &[s0, s1]);
        if status >= 0x80000000 {
            result = status;
        }
    }
    m.put_word(p.wrapping_add(0x2c), e0);
    m.put_word(p.wrapping_add(0x30), e1);
    result
}

fn fn_1f43f0_m(m: &mut Memory<'_>, p: u32, tag: u32, host: &mut Host<'_>) -> u32 {
    let rc = crate::frontend::validate_handle(p != 0, tag, 0xf382, 0x34);
    if rc >= 0x80000000 {
        return 0x8ba02008;
    }
    let mut rc = rc;
    let e0 = m.word(crate::rom_region::addr(host.rom,0x8d7660));
    let e1 = m.word(crate::rom_region::addr(host.rom,0x8d7664));
    for i in 0..2u32 {
        let at = p.wrapping_add(0x24).wrapping_add(i * 8);
        if m.word(at) != 0 {
            let rel = m.word(m.word(p.wrapping_add(8)).wrapping_add(0x24));
            rc = host.call(m, rel, &[m.word(at), m.word(at.wrapping_add(4))]);
            m.put_word(at, e0);
            m.put_word(at.wrapping_add(4), e1);
        }
    }
    rc
}

fn fn_211348_m(m: &mut Memory<'_>, p: u32, tag: u32, host: &mut Host<'_>) -> u32 {
    let rc = crate::frontend::validate_handle(p != 0, tag, 0xf381, 52);
    if rc >= 0x80000000 {
        return 0x89a02008;
    }
    let mut rc = rc;
    for i in 0..3u32 {
        let at = p.wrapping_add(24).wrapping_add(i * 8);
        if m.word(at) != 0 {
            let rel = m.word(m.word(p.wrapping_add(8)).wrapping_add(0x24));
            rc = host.call(m, rel, &[m.word(at), m.word(at.wrapping_add(4))]);
            m.put_word(at, 0);
            m.put_word(at.wrapping_add(4), 0);
        }
    }
    rc
}

fn fn_229cc0_m(m: &mut Memory<'_>, p: u32, tag: u32, host: &mut Host<'_>) -> u32 {
    let r = host.call(m, 0x176f94, &[p, tag, 0x4e7, 0x70]);
    if r >= 0x80000000 {
        return 0x84e02008;
    }
    let mut r = r;
    m.put_word(p.wrapping_add(0x68), 0);
    m.put_word(p.wrapping_add(0x6c), 0);
    let e0 = m.word(crate::rom_region::addr(host.rom,0x8d7660));
    let e1 = m.word(crate::rom_region::addr(host.rom,0x8d7664));
    for at in [0x40u32, 0x50, 0x48, 0x58] {
        let s = p.wrapping_add(at);
        if m.word(s) != 0 {
            let rel = m.word(m.word(p.wrapping_add(0x10)).wrapping_add(0x24));
            let t = host.call(m, rel, &[m.word(s), m.word(s.wrapping_add(4))]);
            if at == 0x40 {
                r = t;
            } else if t >= 0x80000000 && r < 0x80000000 {
                r = t;
            }
            m.put_word(s, e0);
            m.put_word(s.wrapping_add(4), e1);
        }
    }
    r
}

fn fn_19672c_m(m: &mut Memory<'_>, p: u32, key: u32, host: &mut Host<'_>) -> u32 {
    if p == 0 || key == 0 {
        return 0x89002007;
    }
    let o = p.wrapping_add(0x18);
    let cmp = m.word(m.word(o.wrapping_add(36)).wrapping_add(8));
    let mut n = m.word(o);
    let mut found = 0u32;
    while n != o {
        let r = host.call(m, cmp, &[key, m.word(n.wrapping_add(16))]) as i32;
        if r < 0 {
            n = m.word(n);
        } else if r > 0 {
            n = m.word(n.wrapping_add(4));
        } else {
            found = n;
            break;
        }
        if n == 0 {
            break;
        }
    }
    if found == 0 {
        return 0x89002014;
    }

    if m.byte(o.wrapping_add(40)) != 0 {
        let mut last = found;
        loop {
            let mut cur = m.word(last);
            while cur != o {
                if host.call(m, cmp, &[key, m.word(cur.wrapping_add(16))]) as i32 == 0 {
                    break;
                }
                cur = m.word(cur.wrapping_add(4));
            }
            if cur == o {
                break;
            }
            last = cur;
        }
        found = last;
    }
    crate::normalization::rb_erase(m.bytes, o, found);
    let heap = m.word(o.wrapping_add(0x20));
    let vt = m.word(o.wrapping_add(0x24));
    let k = m.word(found.wrapping_add(0x10));
    if k != 0 {
        host.call(m, m.word(vt.wrapping_add(4)), &[k, heap]);
    }
    let v = m.word(found.wrapping_add(0x14));
    if v != 0 {
        host.call(m, m.word(vt.wrapping_add(0x10)), &[v, heap]);
    }
    fn_196428_m(m, heap, found, host);
    0
}

pub fn synthesis_drain_m(
    m: &mut engine::Memory<'_, '_>,
    synth: u32,
    workspace: &mut [u8],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn RuntimeServicesM,
    snap: &[u8],
    rom: crate::rom_region::Rom,
) -> Result<u32, engine::Error> {
    let mut os = ViewOnly;
    let mut flat = crate::bridge::Flat::new(m, workspace, &mut os, 0)?;
    let mut arena = flat.memory();

    let out = synth.wrapping_add(0x84);
    let ctx = arena.word(out);
    let ctx_heap = if ctx == 0 { 0 } else { arena.word(ctx.wrapping_add(4)) };
    let arr = arena.word(out.wrapping_add(0x78));
    let extra = arena.word(out.wrapping_add(0x58));
    let host = RefCell::new(Host {
        boot: engine::SpeechBootHost { platform, fault: None },
        boundary: None,
        runtime: Some(runtime),
        snap,
        trace: None,
        until: None,
        depth: 0,
        stack: [0; 64],
        stack_len: 0,
        rom,
    });
    let rc = {
        let output = OutputSide { host: &host, synth };
        let mut hooks = DrainHooks { output, synth };
        teardown_synth_m(&mut arena.bytes, synth, 0, 0, 0, &mut hooks)
    };
    let rc = rc;

    {
        let mut arena = flat.memory();
        let mut host_ref = host.borrow_mut();
        if arr != 0 {
            crate::boot_part_e::release(&mut arena, ctx_heap, arr, &mut *host_ref);
            arena.put_word(out.wrapping_add(0x78), 0);
        }
        if extra != 0 {
            crate::boot_part_e::release(&mut arena, ctx_heap, extra, &mut *host_ref);
        }

        {
            let pipe = arena.word(out.wrapping_add(0x10));
            if pipe != 0 {
                let smap = arena.word(pipe.wrapping_add(12));
                if smap != 0 {
                    host_ref.call(&mut arena, 0x19665c, &[smap]);
                }
            }
        }
    }
    flat.finish()?;
    if let Some(e) = host.borrow_mut().boot.fault {
        return Err(e);
    }
    Ok(rc)
}

struct DrainDb<'a, 'h> {
    host: &'a core::cell::RefCell<Host<'h>>,
}
impl crate::boot::Calls for DrainDb<'_, '_> {
    fn call(&mut self, memory: &mut crate::boot::Memory<'_>, address: u32, args: &[u32]) -> u32 {
        self.host.borrow_mut().call(memory, address, args)
    }
    fn call_wide(&mut self, _memory: &mut crate::boot::Memory<'_>, _address: u32, _args: &[u32]) -> u64 {

        0
    }
}
impl crate::db_live::PoolHooksM for DrainDb<'_, '_> {
    fn log_pool_m(&mut self, _pool: u32, _blocks: u32, _mem: &mut crate::boot::Memory<'_>) {}
}
impl crate::db_live::DbHooksM for DrainDb<'_, '_> {
    fn call_m(&mut self, entry: u32, args: &[u32], mem: &mut crate::boot::Memory<'_>) -> u32 {
        self.host.borrow_mut().call(mem, entry, args)
    }
}

struct DrainHooks<'a, 'h> {
    output: OutputSide<'a, 'h>,
    synth: u32,
}

impl DrainHooks<'_, '_> {
    fn mem_word(mem: &[u8], at: u32) -> u32 {
        u32::from_le_bytes(mem[at as usize..at as usize + 4].try_into().unwrap())
    }

    fn mark(_mem: &mut [u8], _site: u32, _entry: u32) {}
    fn out(&self) -> u32 {
        self.synth.wrapping_add(0x84)
    }
    fn vt_entry(&self, mem: &[u8], off: u32) -> u32 {
        let vt = Self::mem_word(mem, self.synth.wrapping_add(0x40));
        Self::mem_word(mem, vt.wrapping_add(off))
    }
    fn call(&mut self, mem: &mut [u8], entry: u32, args: &[u32]) -> u32 {
        let mut m = Memory { base: 0, bytes: mem };
        self.output.host.borrow_mut().call(&mut m, entry, args)
    }
}

impl EmitHooksM for DrainHooks<'_, '_> {
    fn put_output_m(
        &mut self,
        user: u32,
        msg: u32,
        len_in: u32,
        n_in: u32,
        markers: (u32, u32),
        mem: &mut [u8],
    ) -> (u32, Option<u32>, u32, u32) {

        if msg == 2 && len_in == 0 && n_in == 0 {
            return (0, Some(Self::mem_word(mem, self.out().wrapping_add(0x60))), 0, 0);
        }
        self.output.put_output_m(user, msg, len_in, n_in, markers, mem)
    }
    fn get_avail_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> (u32, u32, u8) {
        self.output.get_avail_m(a, b, mem)
    }
    fn get_buf_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> (u32, Option<u32>, u32) {
        self.output.get_buf_m(a, b, mem)
    }
    fn get_pos_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> (u32, u32) {
        self.output.get_pos_m(a, b, mem)
    }
    fn consume_m(&mut self, a: u32, b: u32, n: u32, mem: &mut [u8]) -> u32 {
        self.output.consume_m(a, b, n, mem)
    }
    fn realloc_markers_m(&mut self, mem: &mut [u8]) -> Option<u32> {
        self.output.realloc_markers_m(mem)
    }
}

impl SynthTeardownHooksM for DrainHooks<'_, '_> {
    fn close_stream_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> u32 {
        let entry = Self::mem_word(mem, Self::mem_word(mem, self.synth.wrapping_add(0x40)).wrapping_add(0x24));
        Self::mark(mem, 1, entry);
        self.call(mem, entry, &[a, b])
    }
    fn process_end_m(&mut self, h0: u32, h1: u32, mem: &mut [u8]) -> u32 {

        let cs = Self::mem_word(mem, self.synth.wrapping_add(0x7c));
        let comps = Self::mem_word(mem, cs.wrapping_add(8));
        let count = u16::from_le_bytes(
            mem[cs.wrapping_add(12) as usize..cs.wrapping_add(14) as usize]
                .try_into()
                .unwrap(),
        ) as u32;
        let mut i = 0u32;
        while i < count {
            let comp = Self::mem_word(mem, comps.wrapping_add(i.wrapping_mul(4)));
            if Self::mem_word(mem, comp.wrapping_add(12)) == h0
                && Self::mem_word(mem, comp.wrapping_add(16)) == h1
            {
                let entry = Self::mem_word(mem, Self::mem_word(mem, comp.wrapping_add(8)).wrapping_add(0x38));
                Self::mark(mem, 2, entry);
                return self.call(mem, entry, &[h0, h1]);
            }
            i = i.wrapping_add(1);
        }
        0x80002001
    }
    fn release_stream_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> u32 {
        let o = self.out();
        let pipe = (
            Self::mem_word(mem, o.wrapping_add(0x10)),
            Self::mem_word(mem, o.wrapping_add(0x14)),
        );
        let aux = (
            Self::mem_word(mem, o.wrapping_add(0x1c)),
            Self::mem_word(mem, o.wrapping_add(0x20)),
        );
        let vt = if (a, b) == pipe {
            Self::mem_word(mem, self.synth.wrapping_add(0x40))
        } else if (a, b) == aux {

            let mut bm = Memory { base: 0, bytes: mem };
            let mut db = DrainDb { host: self.output.host };
            let data = crate::db_live::DbData {
                module: 0,
                open_begin: 0,
                open_end: 0,
                open_detail: 0,
                reset_begin: 0,
                reset_end: 0,
                pool_name_word: 0,
                keys: [0; 5],
            };
            return crate::db_live::close_db_m(&mut db, &mut bm, a, b, &data);
        } else {
            return 0x80002001;
        };
        let entry = Self::mem_word(mem, vt.wrapping_add(0x14));
        Self::mark(mem, 3, entry);
        self.call(mem, entry, &[a, b])
    }
    fn null_pair_m(&mut self, mem: &mut [u8]) -> (u32, u32) {
        let rom = self.output.host.borrow().rom;
        (
            Self::mem_word(mem, crate::rom_region::addr(rom, 0x8d7660)),
            Self::mem_word(mem, crate::rom_region::addr(rom, 0x8d7664)),
        )
    }
    fn lex_close_m(&mut self, params: u32, lex: u32, mem: &mut [u8]) -> u32 {

        if params == 0 || lex == 0 {
            return 0x8b702007;
        }
        let queue = Self::mem_word(mem, lex.wrapping_add(4));
        if queue == 0 {

            return 0;
        }
        let parameters = Self::mem_word(mem, lex);
        let mut rc = 0u32;

        const SCR: usize = OUT_SCRATCH as usize;
        let mut saved = [0u8; 12];
        saved.copy_from_slice(&mem[SCR..SCR + 12]);
        let iter_at: u32 = OUT_SCRATCH;
        let key_at: u32 = OUT_SCRATCH + 4;
        let entry_at: u32 = OUT_SCRATCH + 8;
        let node = crate::frontend::last_link(mem, queue.wrapping_add(24));
        let heap = Self::mem_word(mem, queue.wrapping_add(20));
        let object = if heap == 0 {
            0
        } else {
            let vt = Self::mem_word(mem, heap);
            let f = Self::mem_word(mem, vt.wrapping_add(4));
            Self::mark(mem, 4, f);
            self.call(mem, f, &[Self::mem_word(mem, heap.wrapping_add(4)), 1, 12])
        };
        if object == 0 {
            rc = 0x8900200a;
        } else {
            mem[iter_at as usize..iter_at as usize + 4].copy_from_slice(&object.to_le_bytes());
            mem[object as usize..object as usize + 4].copy_from_slice(&queue.to_le_bytes());
            mem[object as usize + 4..object as usize + 8].copy_from_slice(&0u32.to_le_bytes());
            mem[object as usize + 8..object as usize + 12].copy_from_slice(&node.to_le_bytes());

            let first = {
                let mut bm = Memory { base: 0, bytes: &mut *mem };
                crate::engine_components2::fn_196860(&mut bm, object, key_at, entry_at)
            };
            if first == 0 {
                loop {
                    let key = Self::mem_word(mem, key_at);
                    let entry = Self::mem_word(mem, entry_at);
                    let size = Self::mem_word(mem, entry.wrapping_add(4));
                    let mut t;
                    if size != 0 {
                        let data = Self::mem_word(mem, entry);
                        Self::mark(mem, 60, key);
                        t = self.call(mem, 0x175170, &[parameters, key, data, size, 0]);
                    } else {
                        t = self.lex_remove_m(parameters, key, mem);
                        let low = t & 0x1fff;
                        if low == 0x11 {
                            Self::mark(mem, 61, key);
                            t = self.call(mem, 0x175170, &[parameters, key, 0, 1, 0]);
                        } else if low == 0x14 {
                            t = 0;
                        }
                    }
                    if t >= 0x80000000 && rc < 0x80000000 {
                        rc = t;
                    }
                    let st = {
                        let mut bm = Memory { base: 0, bytes: &mut *mem };
                        crate::engine_components2::fn_196860(&mut bm, object, key_at, entry_at)
                    };
                    if st != 0 {
                        break;
                    }
                }
            }
            {
                let mut bm = Memory { base: 0, bytes: &mut *mem };
                let mut db = DrainDb { host: self.output.host };
                crate::boot_live::release_iterator_m(&mut bm, &mut db, object);
            }
        }
        mem[SCR..SCR + 12].copy_from_slice(&saved);
        rc
    }
    fn lex_free_m(&mut self, params: u32, lex: u32, mem: &mut [u8]) -> u32 {

        if params == 0 || lex == 0 {
            return 0x8b702007;
        }
        let lock = Self::mem_word(mem, params.wrapping_add(4));
        let mut rc = self.call(mem, 0x176f2c, &[lock]);
        if rc < 0x80000000 {
            let heap = Self::mem_word(mem, params.wrapping_add(16));
            if self.call(mem, 0x196fbc, &[heap, lex]) == 0 {
                rc = 0x8b702007;
            }
            let t = self.call(mem, 0x176f5c, &[lock]);
            if t >= 0x80000000 && rc < 0x80000000 {
                rc = t;
            }
        }
        rc
    }
    fn retire_pipeline_m(&mut self, cs: u32, mem: &mut [u8]) -> u32 {
        self.call(mem, 0x17ec38, &[cs])
    }
    fn reopen_pipeline_cs_m(&mut self, cs: u32, mem: &mut [u8]) -> u32 {
        self.call(mem, 0x1805f0, &[cs])
    }
}

impl DrainHooks<'_, '_> {

    fn lex_remove_m(&mut self, params: u32, key: u32, mem: &mut [u8]) -> u32 {
        if params == 0 || key == 0 {
            return 0x8b702007;
        }

        let lock = Self::mem_word(mem, params.wrapping_add(4));
        let values = Self::mem_word(mem, params.wrapping_add(8));
        Self::mark(mem, 50, 0x176f2c);
        let mut r = self.call(mem, 0x176f2c, &[lock]);
        if r >= 0x80000000 {
            return r;
        }

        const SCR: usize = OUT_SCRATCH as usize;
        let mut saved = [0u8; 16];
        saved.copy_from_slice(&mem[SCR..SCR + 16]);
        let value_at: u32 = OUT_SCRATCH + 12;
        Self::mark(mem, 51, 0x1966ac);
        r = self.call(mem, 0x1966ac, &[values, key, value_at]);
        if r < 0x80000000 {
            let value = Self::mem_word(mem, value_at);
            let object = if value == 0 {
                0
            } else {
                Self::mem_word(mem, value.wrapping_add(8))
            };
            if object != 0 {
                Self::mark(mem, 52, 0x1970a0);
                if self.call(mem, 0x1970a0, &[object]) != 0 {
                    r = 0x8b702011;
                }
            }
            if r < 0x80000000 {
                let (data, size) = if value == 0 {
                    (0, 0)
                } else {
                    let size = Self::mem_word(mem, value.wrapping_add(4));
                    let data = if size == 0 {
                        0
                    } else {
                        Self::mem_word(mem, value)
                    };
                    (data, size)
                };
                Self::mark(mem, 53, 0x174b38);
                r = self.call(mem, 0x174b38, &[params, key, data, size, 0]);
                if r < 0x80000000 {
                    Self::mark(mem, 54, 0x19672c);
                    r = self.call(mem, 0x19672c, &[values, key]);
                    if r < 0x80000000 {

                    } else {
                        r = (r & 0x1fff) | 0x8b702000;
                    }
                }
            }
        } else {
            r = (r & 0x1fff) | 0x8b702000;
        }
        Self::mark(mem, 55, 0x176f5c);
        let t = self.call(mem, 0x176f5c, &[lock]);
        mem[SCR..SCR + 16].copy_from_slice(&saved);
        if t >= 0x80000000 && r < 0x80000000 {
            r = t;
        }
        r
    }
}

pub trait SchedulerHooksM: crate::reentrant::HooksM {
    fn switch_m(&mut self, call: crate::frontend::PipelineCall, synth: u32,
        mem: &mut Memory<'_>) -> crate::frontend::PipelineReply;

    fn checkpoint_allowed(&mut self) -> bool {
        true
    }

    fn rom(&self) -> crate::rom_region::Rom {
        crate::rom_region::Rom::Us
    }

}

pub fn process_text_m(
    m: &mut Memory<'_>, p:u32, flush:u32, activity:&mut [u8], scratch:u32,
    hooks:&mut impl SchedulerHooksM,
)->u32 {
    use crate::frontend::{FrontendState,PipelineCall as C,PipelineReply,PipelineStages,SynthesisSink};
    fn load(s:&mut FrontendState<'_>,m:&Memory<'_>,p:u32) {
        let cs=m.word(p+0x7c);let vt=m.word(p+0x54);
        let act=m.word(p+0xdc) as usize;
        s.activity.copy_from_slice(&m.bytes[act..act+s.activity.len()]);
        s.stage=m.half(p+0xd8);s.pending=m.word(p+0xd4);s.loop_limit=m.word(p+0x38);
        s.request_state=m.byte(p+0x30);s.prepared_bytes=m.word(p+0xf0);
        s.input_open=m.word(p+0xac)!=0;s.output_open=m.word(p+0xc0)!=0;
        s.instrumented=m.word(cs+16)!=0;s.observer=m.word(p+0x20)!=0;
        s.auxiliary_notifications=m.word(vt+0x64)!=0;
    }
    fn store(s:&FrontendState<'_>,m:&mut Memory<'_>,p:u32) {
        let act=m.word(p+0xdc) as usize;
        m.bytes[act..act+s.activity.len()].copy_from_slice(s.activity);
        m.put_half(p+0xd8,s.stage);m.put_word(p+0xd4,s.pending);
        m.put_byte(p+0x30,s.request_state);m.put_word(p+0xf0,s.prepared_bytes);
    }
    struct NoSink;
    impl SynthesisSink for NoSink {
        fn emit(&mut self,_:crate::frontend::SynthesisPayload<'_>)->u32 {unreachable!("scheduler uses its output-pump call")}
    }
    struct Dispatch<'a,'b,H> {m:&'a mut Memory<'b>,p:u32,h:&'a mut H,scratch:u32}
    impl<H:SchedulerHooksM> PipelineStages for Dispatch<'_, '_, H> {
        fn call(&mut self,call:C,state:&mut FrontendState<'_>,_:&mut impl SynthesisSink)->PipelineReply {
            let p=self.p;let m=&mut *self.m;

            if !self.h.checkpoint_allowed() {
                return crate::frontend::PipelineReply { status: 0x80002001, value: 0, data: 0 };
            }
            store(state,m,p);
            let cs=m.word(p+0x7c);let vt=m.word(p+0x40);let vt2=m.word(p+0x54);
            let stage=state.stage as u32;let comp=m.word(m.word(cs+8)+stage*4);
            let pair=[m.word(p+0xa0),m.word(p+0xa4)];
            let mut r=PipelineReply::default();let s=self.scratch;
            match call {
                C::BeginStage(i)|C::EndStage(i)=>{
                    r.status=self.h.call_m(if matches!(call,C::BeginStage(_)){0x1985fc}else{0x19860c},
                        &[m.word(cs+16),m.word(cs+20),i as u32+2],m);
                }
                C::Process(i)=>{
                    let c=m.word(m.word(cs+8)+i as u32*4);
                    r.status=self.h.call_m(m.word(m.word(c+8)+52),
                        &[m.word(c+12),m.word(c+16),pair[0],pair[1],m.word(p+0xdc)+i as u32],m);
                }
                C::ObserveStage(_)=>{
                    r.status=self.h.call_m(m.word(p+0x20),
                        &[m.word(p+0x28),m.word(comp+4),pair[0],pair[1],m.word(p+0xdc)+stage],m);
                }
                C::QueryAuxiliary=>{
                    m.put_byte(s,0);
                    r.status=self.h.call_m(m.word(vt2+0x68),&[pair[0],pair[1],s],m);
                    r.value=m.byte(s) as u32;
                }
                C::NotifyAuxiliary(_)=>{
                    r.status=self.h.call_m(m.word(vt2+0x64),&[pair[0],pair[1],0,m.word(comp+4),1],m);
                }
                C::FlushOutput=>{r.status=self.h.call_m(0x1530d0,&[p,0],m);}
                C::Lock|C::Unlock=>{
                    r.status=self.h.call_m(if call==C::Lock {0x176f2c}else{0x176f5c},&[m.word(p+0x2c)],m);
                }
                C::ReadInput=>{
                    m.put_word(s,0x400);m.put_word(s+4,0);
                    r.status=self.h.call_m(m.word(p+0xb8),&[m.word(p+0xbc),s,s+4],m);
                    r.value=m.word(s);r.data=m.word(s+4);
                }
                C::CommitInput{bytes,data}=>{
                    m.put_word(s,data);
                    r.status=self.h.call_m(m.word(vt+0x34),&[m.word(p+0xac),m.word(p+0xb0),s,bytes],m);
                }
                C::CloseInput=>{
                    r.status=self.h.call_m(m.word(vt+0x24),&[m.word(p+0xac),m.word(p+0xb0)],m);
                    m.put_word(p+0xac,m.word(crate::rom_region::addr(self.h.rom(),0x8d7660)));m.put_word(p+0xb0,m.word(crate::rom_region::addr(self.h.rom(),0x8d7664)));
                }
                C::PrepareVoiceSwitch(_)|C::ExecuteVoiceSwitch(_)=>{r=self.h.switch_m(call,p,m);}
            }
            load(state,m,p);
            r
        }
    }
    assert_eq!(m.base,0);
    let n=m.half(m.word(p+0x7c)+12) as usize;assert!(n>0 && n<=activity.len());
    let mut state=FrontendState{activity:&mut activity[..n],stage:0,pending:0,loop_limit:0,request_state:0,
        prepared_bytes:0,input_open:false,output_open:false,instrumented:false,observer:false,auxiliary_notifications:false};
    load(&mut state,m,p);
    let saved=[m.word(scratch),m.word(scratch+4)];
    let mut d=Dispatch{m,p,h:hooks,scratch};let rc=crate::frontend::process_text(&mut state,flush,&mut d,&mut NoSink);

    if d.h.checkpoint_allowed() {
        store(&state,d.m,p);
    }
    d.m.put_word(scratch,saved[0]);d.m.put_word(scratch+4,saved[1]);rc
}

struct MethodHost<'h,'p> { h:&'h mut Host<'p>, entry:u32 }
impl VMethodHooksM for MethodHost<'_,'_> {
    fn call_method_m(&mut self,a:u32,b:u32,bytes:&mut [u8])->u32 {
        self.h.call(&mut Memory{base:0,bytes},self.entry,&[a,b])
    }
}
struct ResetHost<'h,'p>{h:&'h mut Host<'p>,p:u32}
impl CtxResetHooksM for ResetHost<'_,'_> {
    fn release_pair_m(&mut self,a:u32,b:u32,c:u32,bytes:&mut [u8])->u32 {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.p+48)+48);self.h.call(&mut m,f,&[a,b,c])
    }
    fn free_pair_m(&mut self,a:u32,b:u32,bytes:&mut [u8])->u32 {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.p+12)+36);self.h.call(&mut m,f,&[a,b])
    }
    fn detach_pair_m(&mut self,a:u32,b:u32,bytes:&mut [u8])->u32 {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.p+48)+28);self.h.call(&mut m,f,&[a,b])
    }
    fn null_pair_m(&mut self,bytes:&mut [u8])->(u32,u32) {
        (mem_word(bytes,crate::rom_region::addr(self.h.rom,0x8d7660)),mem_word(bytes,crate::rom_region::addr(self.h.rom,0x8d7664)))
    }
}

pub fn end_token_processing_m(m:&mut Memory<'_>,p:u32,tag:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    if validate_handle(p!=0,tag,0xe58c,8)>=0x80000000 {return 0x89602008;}
    let inner=m.word(p);let key=m.word(p+4);
    if validate_handle(inner!=0,key,0xe58d,0xf4)>=0x80000000 {return 0x89602008;}
    h.call_m(0x23ffd4,&[m.word(inner+48),m.word(inner+52)],m);0
}

struct HostPlat<'a>(&'a mut dyn engine::HeapPlatform);
impl engine::HeapPlatform for HostPlat<'_> {
    fn thread_id(&mut self) -> u32 { self.0.thread_id() }
    fn arbiter_address(&self) -> u32 { self.0.arbiter_address() }
    fn arbitrate(&mut self, m: &mut crate::boot::Memory<'_>, a: &[u32]) -> u32 { self.0.arbitrate(m, a) }
}

#[inline(never)]
fn seg3_oneword_arm(h: &mut Host, m: &mut Memory, f: u32, a: &[u32]) -> u32 {
    if a.len() < 4 {
        h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
        return 0x80002001;
    }
    let done = if a.len() > 4 { a[4] } else { m.word(0xe0ff938) };
    let mut shost = crate::engine_live_seg3::Host::new(&mut *h.boot.platform, crate::engine_live_seg3::SEG_SCRATCH).with_rom(h.rom);
    let rc = crate::engine_live_seg3::root_oneword(&mut shost, m, a[0], a[1], a[2], a[3], done);
    if let Some(e) = shost.first_unbound {
        h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: e });
        return 0x80002001;
    }
    if let Some(e) = shost.fault {
        h.boot.fault = Some(e);
        return 0x80002001;
    }
    rc
}

#[inline(never)]
fn seg3_pos_arm(h: &mut Host, m: &mut Memory, f: u32, a: &[u32]) -> u32 {
    if a.len() < 5 {
        h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
        return 0x80002001;
    }
    let mut t = [0u8; 65536];
    let mut mk = [0u8; 65536];
    let mut l0 = [0u8; 65536];
    let mut l1 = [0u8; 65536];
    let mut l2 = [0u8; 65536];
    let mut r0 = [0u8; 65536];
    let mut r1 = [0u8; 65536];
    let mut scratch = crate::pos::StageScratch { text: &mut t[..], markers: &mut mk[..], lines: [&mut l0[..], &mut l1[..], &mut l2[..]], rule_output: [&mut r0[..], &mut r1[..]] };
    let mut shost = crate::engine_live_seg3::Host::new(&mut *h.boot.platform, crate::engine_live_seg3::SEG_SCRATCH).with_rom(h.rom);
    let mut view = crate::boot::Memory { base: m.base, bytes: &mut m.bytes[..] };
    let rc = if f == 0x1f1440 {
        crate::engine_live_seg3::root_pos(&mut shost, &mut view, a[0], a[1], a[2], a[3], a[4], &mut scratch)
    } else {
        crate::engine_live_seg3::root_homographs(&mut shost, &mut view, a[0], a[1], a[2], a[3], a[4], &mut scratch)
    };
    if let Some(e) = shost.first_unbound {
        h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: e });
        return 0x80002001;
    }
    if let Some(e) = shost.fault {
        h.boot.fault = Some(e);
        return 0x80002001;
    }
    rc
}

#[inline(never)]
fn seg3_phrasing_arm(h: &mut Host, m: &mut Memory, f: u32, a: &[u32]) -> u32 {
    if a.len() < 5 {
        h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
        return 0x80002001;
    }
    let mut text = [0u8; 16384];
    let mut map = [0u8; 16384];
    let mut input = [0u8; 16384];
    let mut marks = [crate::prosody::PhrasingMark { r#type: 0, position: 0, value: 0 }; 1024];
    let mut mode = [0u8; 1024];
    let mut next_mode = [0u8; 1024];
    let mut merge_next = [0u8; 1024];
    let mut merge_current = [0u8; 1024];
    let mut output = [0u8; 16384];
    let mut flags = [0u8; 16384];
    let mut buffers = crate::prosody::PhrasingBuffers { text: &mut text[..], map: &mut map[..], input: &mut input[..], input_prefix: 0, marks: &mut marks[..], mode: &mut mode[..], next_mode: &mut next_mode[..], merge_next: &mut merge_next[..], merge_current: &mut merge_current[..], output: &mut output[..], flags: &mut flags[..] };
    let mut workspace = crate::component_live_c::VmWorkspace::default();
    let mut shost = crate::engine_live_seg3::Host::new(&mut *h.boot.platform, crate::engine_live_seg3::SEG_SCRATCH).with_rom(h.rom);
    let rc = crate::engine_live_seg3::root_phrasing(&mut shost, m, [a[0], a[1], a[2], a[3]], a[4], &mut buffers, &mut workspace);
    if let Some(e) = shost.first_unbound {
        h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: e });
        return 0x80002001;
    }
    if let Some(e) = shost.fault {
        h.boot.fault = Some(e);
        return 0x80002001;
    }
    rc
}

#[inline(never)]
fn seg7_marker_arm(h: &mut Host, m: &mut Memory, f: u32, a: &[u32]) -> u32 {
    let Some(rt) = h.runtime.as_deref_mut() else {
        h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
        return 0x80002001;
    };
    let snap = h.snap;
    match crate::engine_live_seg7::call_seg7_rom(m, f, a, &mut *h.boot.platform, rt, snap, h.rom) {
        Ok(rc) => rc,
        Err(engine::Error::UnsupportedStage { entry })
        | Err(engine::Error::MissingStreamAdapter { entry }) => {
            h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry });
            0x80002001
        }
        Err(e) => {
            h.boot.fault = Some(e);
            0x80002001
        }
    }
}

#[inline(never)]
fn seg7_flush_arm(h: &mut Host, m: &mut Memory, f: u32, a: &[u32]) -> u32 {
    let Some(rt) = h.runtime.as_deref_mut() else {
        h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
        return 0x80002001;
    };
    let snap = h.snap;
    let mut input = [0i16; 65536];
    let mut output = [0i32; 200000];
    let mut grain = [0i32; 16384];
    let mut window = [0f32; 16384];
    let mut modulation = [0f32; 1024];
    let mut normalization = [0f32; 32];
    let mut wave = [0i16; 1024];
    let mut vr0 = [0f32; 16384];
    let mut vr1 = [0f32; 16384];
    let mut vr2 = [0f32; 16384];
    let mut delay = [0u16; 65536];
    let mut audio = [0i16; crate::queue::AUDIO_CAPACITY];
    let mut samples = [0i16; 65536];
    let mut events = [crate::queue::SpeechEvent::default(); 16384];
    let mut first = [0i16; 0x25800];
    let mut second = [0i16; 0x25800];
    let mut pcm = [0i16; 200000];
    let mut workspace = BackendWorkspace {
        input: &mut input, output: &mut output, grain: &mut grain,
        window: &mut window, modulation: &mut modulation,
        normalization: &mut normalization, wave: &mut wave,
        voice_delays: [&mut vr0, &mut vr1, &mut vr2], sample_delay: &mut delay,
        audio: (&mut audio[..]).try_into().unwrap(), samples: &mut samples,
        events: &mut events, first: &mut first, second: &mut second,
        parameter_values: None,
        rom: h.rom,
    };
    let render = crate::engine_live_seg7::RenderCtx {
        workspace: &mut workspace, pcm: &mut pcm, written: 0,
        tick: 0x9abcdef0,
        registers: engine::NotificationRegisters { s16: 0, s17: 0 },
        stop_at_render: false,
    };
    match crate::engine_live_seg7::call_seg7_with_render_rom(
        m, f, a, &mut *h.boot.platform, rt, snap, Some(render), h.rom,
    ) {
        Ok(rc) => rc,
        Err(engine::Error::UnsupportedStage { entry })
        | Err(engine::Error::MissingStreamAdapter { entry }) => {
            h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry });
            0x80002001
        }
        Err(e) => {
            h.boot.fault = Some(e);
            0x80002001
        }
    }
}
#[inline(never)]
fn seg7_pump_arm(h: &mut Host, m: &mut Memory, f: u32, a: &[u32]) -> u32 {
    let Some(rt) = h.runtime.as_deref_mut() else {
        h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
        return 0x80002001;
    };
    let snap = h.snap;
    let mut input = [0i16; 65536];
    let mut output = [0i32; 200000];
    let mut grain = [0i32; 16384];
    let mut window = [0f32; 16384];
    let mut modulation = [0f32; 1024];
    let mut normalization = [0f32; 32];
    let mut wave = [0i16; 1024];
    let mut vr0 = [0f32; 16384];
    let mut vr1 = [0f32; 16384];
    let mut vr2 = [0f32; 16384];
    let mut delay = [0u16; 65536];
    let mut audio = [0i16; crate::queue::AUDIO_CAPACITY];
    let mut samples = [0i16; 65536];
    let mut events = [crate::queue::SpeechEvent::default(); 16384];
    let mut first = [0i16; 0x25800];
    let mut second = [0i16; 0x25800];
    let mut pcm = [0i16; 200000];
    let mut workspace = BackendWorkspace {
        input: &mut input, output: &mut output, grain: &mut grain,
        window: &mut window, modulation: &mut modulation,
        normalization: &mut normalization, wave: &mut wave,
        voice_delays: [&mut vr0, &mut vr1, &mut vr2], sample_delay: &mut delay,
        audio: (&mut audio[..]).try_into().unwrap(), samples: &mut samples,
        events: &mut events, first: &mut first, second: &mut second,
        parameter_values: None,
        rom: h.rom,
    };
    let render = crate::engine_live_seg7::RenderCtx {
        workspace: &mut workspace, pcm: &mut pcm, written: 0,
        tick: 0x9abcdef0,
        registers: engine::NotificationRegisters { s16: 0, s17: 0 },
        stop_at_render: false,
    };
    match crate::engine_live_seg7::call_seg7_with_render_rom(
        m, f, a, &mut *h.boot.platform, rt, snap, Some(render), h.rom,
    ) {
        Ok(rc) => rc,
        Err(engine::Error::UnsupportedStage { entry })
        | Err(engine::Error::MissingStreamAdapter { entry }) => {
            h.boot.fault = Some(engine::Error::MissingStreamAdapter { entry });
            0x80002001
        }
        Err(e) => {
            h.boot.fault = Some(e);
            0x80002001
        }
    }
}
pub fn call_live_m(m:&mut Memory<'_>,f:u32,a:&[u32],platform:&mut dyn engine::HeapPlatform)->Result<u32,engine::Error> {
    call_live_m_rom(m,f,a,platform,crate::rom_region::Rom::Us)
}
pub fn call_live_m_rom(m:&mut Memory<'_>,f:u32,a:&[u32],platform:&mut dyn engine::HeapPlatform,rom:crate::rom_region::Rom)->Result<u32,engine::Error> {
    let mut h=Host{boot:engine::SpeechBootHost{platform,fault:None},boundary:None,runtime:None,snap:&[],trace:None,until:None,depth:0,stack:[0;64],stack_len:0,rom};
    let rc=h.call(m,f,a);
    if let Some(e)=h.boot.fault {return Err(e);}
    if let Some(b)=h.boundary { return Err(engine::Error::UnsupportedStage{entry:b.entry});}
    Ok(rc)
}

struct BufferHost<'h,'p>{h:&'h mut Host<'p>,p:u32}
impl BufferHost<'_,'_> {
    fn buffer(&mut self,bytes:&mut [u8],off:u32,args:&[u32])->u32 {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.p+12)+off);
        self.h.call(&mut m,f,args)
    }
    fn inserter(&mut self,bytes:&mut [u8],off:u32,args:&[u32])->u32 {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.p+48)+off);
        self.h.call(&mut m,f,args)
    }
}
fn slots_m<R>(bytes:&mut [u8],values:[u32;4],f:impl FnOnce(&mut [u8],u32)->R)->(R,[u32;4]) {
    let s=0x1900;let saved=core::array::from_fn::<_,4,_>(|i|mem_word(bytes,s+i as u32*4));
    for (i,v) in values.iter().enumerate(){mem_store(bytes,s+i as u32*4,*v);}
    let r=f(bytes,s);let out=core::array::from_fn(|i|mem_word(bytes,s+i as u32*4));
    for (i,v) in saved.iter().enumerate(){mem_store(bytes,s+i as u32*4,*v);}
    (r,out)
}
impl InserterPumpHooksM for BufferHost<'_,'_> {
    fn buf_available_m(&mut self,a:u32,b:u32,count:&mut u32,end:&mut u8,bytes:&mut [u8])->u32 {
        let (rc,v)=slots_m(bytes,[*count,*end as u32,0,0],|m,s|self.buffer(m,72,&[a,b,s,s+4]));*count=v[0];*end=v[1] as u8;rc
    }
    fn buf_read_m(&mut self,a:u32,b:u32,data:&mut u32,count:&mut u32,bytes:&mut [u8])->u32 {
        let (rc,v)=slots_m(bytes,[*data,*count,0,0],|m,s|self.buffer(m,44,&[a,b,s,s+4]));*data=v[0];*count=v[1];rc
    }
    fn buf_write_m(&mut self,a:u32,b:u32,data:&mut u32,count:u32,bytes:&mut [u8])->u32 {
        let (rc,v)=slots_m(bytes,[*data,0,0,0],|m,s|self.buffer(m,56,&[a,b,s,count]));*data=v[0];rc
    }
    fn buf_consume_m(&mut self,a:u32,b:u32,n:u32,bytes:&mut [u8])->u32 {self.buffer(bytes,48,&[a,b,n])}
    fn buf_close_m(&mut self,a:u32,b:u32,bytes:&mut [u8])->u32 {self.buffer(bytes,36,&[a,b])}
    fn buf_copy_m(&mut self,a:u32,b:u32,c:u32,d:u32,n:u32,moved:&mut u32,bytes:&mut [u8])->u32 {
        let (rc,v)=slots_m(bytes,[*moved,0,0,0],|m,s|self.buffer(m,64,&[a,b,c,d,n,s]));*moved=v[0];rc
    }
    fn buf_position_m(&mut self,a:u32,b:u32,pos:&mut u32,bytes:&mut [u8])->u32 {
        let (rc,v)=slots_m(bytes,[*pos,0,0,0],|m,s|self.buffer(m,68,&[a,b,s]));*pos=v[0];rc
    }
    fn buf_commit_m(&mut self,a:u32,b:u32,n:u32,bytes:&mut [u8])->u32 {self.buffer(bytes,60,&[a,b,n])}
    fn ins_resolve_m(&mut self,a:u32,b:u32,uri:u32,c:u32,d:u32,out:&mut u32,bytes:&mut [u8])->u32 {
        let (rc,v)=slots_m(bytes,[*out,0,0,0],|m,s|self.inserter(m,32,&[a,b,0,uri,c,d,s]));*out=v[0];rc
    }
    fn ins_open_m(&mut self,a:u32,b:u32,name:u32,pos:&mut u32,rem:&mut u32,ins:&mut (u32,u32),bytes:&mut [u8])->u32 {
        let (rc,v)=slots_m(bytes,[*pos,*rem,ins.0,ins.1],|m,s|self.inserter(m,40,&[a,b,name,s,s+4,s+8]));*pos=v[0];*rem=v[1];*ins=(v[2],v[3]);rc
    }
    fn ins_close_m(&mut self,a:u32,b:u32,flag:u32,bytes:&mut [u8])->u32 {self.inserter(bytes,48,&[a,b,flag])}
    fn pump_remote_m(&mut self,bytes:&mut [u8],p:u32)->u32 {self.h.call(&mut Memory{base:0,bytes},0x22c85c,&[p])}
    fn null_pair_m(&mut self,bytes:&mut [u8])->(u32,u32) {(mem_word(bytes,crate::rom_region::addr(self.h.rom,0x8d7660)),mem_word(bytes,crate::rom_region::addr(self.h.rom,0x8d7664)))}
}
struct AudioHost<'h,'p>{h:&'h mut Host<'p>,p:u32,index:u32}
impl AudioOpenHooksM for AudioHost<'_,'_> {
    fn stream_open_m(&mut self,a:u32,b:u32,name:&[u8],x:u32,y:u32,out:&mut (u32,u32),bytes:&mut [u8])->u32 {
        let s=0x1a00usize;let mut saved=[0u8;128];saved.copy_from_slice(&bytes[s..s+128]);
        bytes[s..s+128].fill(0);let n=name.iter().position(|v|*v==0).unwrap();bytes[s..s+n].copy_from_slice(&name[..n]);
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.p+12)+32);let dst=self.p+16+self.index*8;
        self.index+=1;let rc=self.h.call(&mut m,f,&[a,b,0xb9,s as u32,x,y,dst]);
        *out=(m.word(dst),m.word(dst+4));m.bytes[s..s+128].copy_from_slice(&saved);rc
    }
    fn query_marker_mode_m(&mut self,bytes:&mut [u8])->u32 {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.p+12)+76);self.h.call(&mut m,f,&[])
    }
    fn audio_open_failed_m(&mut self,logger:u32,bytes:&mut [u8]) {

        let mut name=[0u8;64];name[..15].copy_from_slice(b"audio/L16;rate=");let _=render_unsigned(mem_word(bytes,self.p+60),&mut name[15..],10);
        let s=0x1a00usize;let mut saved=[0u8;64];saved.copy_from_slice(&bytes[s..s+64]);bytes[s..s+64].copy_from_slice(&name);
        let mut m=Memory{base:0,bytes};DbHooksM::call_m(self.h,0x1738e8,&[logger,0x8fa808,3,0,0x22d670,s as u32],&mut m);
        m.bytes[s..s+64].copy_from_slice(&saved);
    }
    fn content_open_failed_m(&mut self,logger:u32,code:u32,bytes:&mut [u8]) {
        let s=0x1a00usize;let mut saved=[0u8;64];saved.copy_from_slice(&bytes[s..s+64]);bytes[s..s+64].fill(0);
        if self.index==2 {bytes[s..s+15].copy_from_slice(b"audio/L16;rate=");let rate=mem_word(bytes,self.p+60);let _=render_unsigned(rate,&mut bytes[s+15..s+64],10);}
        else {bytes[s..s+43].copy_from_slice(b"application/x-realspeak-markers-pp;version=4.0");}
        let mut m=Memory{base:0,bytes};
        slots_m(m.bytes,[0x22d694,s as u32,0,0],|bytes,args|self.h.call(&mut Memory{base:0,bytes},0x173dc0,&[logger,0x8fa808,code,0x22d6a0,args,0x1940]));
        m.bytes[s..s+64].copy_from_slice(&saved);
    }
}

pub fn fn_264578(m:&mut Memory<'_>,p:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    if p==0 {return 0x81102007;}
    if m.byte(p+0x77)!=0 {return h.call_m(0x2ba6b4,&[m.word(p+0x3c)],m);}
    0
}

pub const NESTED_163A98_SP: u32 = 0xe0ff800;

pub const NESTED_CAP_S0: u32 = NESTED_163A98_SP + 0x18;

pub const NESTED_CAP_S1: u32 = NESTED_163A98_SP + 0x14;

pub const NESTED_RESULT: u32 = NESTED_163A98_SP + 0x64;
pub fn callback_m(m:&mut Memory<'_>,a:[u32;5],_scratch:u32,tick:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    let p=a[0];let event=a[4];
    h.call_m(0x183258,&[p,m.word(event)],m);
    match m.word(event) {
        1=>{h.call_m(0x18cae8,&[m.word(p+0x61b8)],m);}
        2=>{if h.call_m(0x183398,&[p,a[1],a[2]],m)==0x80002001{return 0x80002001;}m.put_byte(p+0x61b4,1);}
        8=>{
            let result=m.word(event+8);
            m.put_word(result+8,m.word(p+0x6194).wrapping_add(m.word(p+0x6198)&!1));
            m.put_word(result+4,m.word(p+0x6198));
            m.put_word(result+16,m.word(p+0x619c));m.put_word(result+12,4000);
        }
        16=>{
            loop {
                let ready=h.call_m(0x18cc30,&[p],m);
                if ready!=0 && (m.byte(p+0xa8)==0 || m.word(p+0x18)<=1 || m.word(p+0x20)!=0) {break;}
                if m.byte(p+0x61a1)!=0 {return 0;}
                h.call_m(0x166054,&[tick,0],m);
            }

            if !render_callback_m(m,p,a[1],a[2],m.word(event+8),NESTED_CAP_S0,false,h){return 0x80002001;}
            h.call_m(0x1840fc,&[p+0x110],m);
            let ready=h.call_m(m.word(m.word(p)+120),&[p],m);
            if ready!=0 && m.byte(p+0x61a4)==0 && m.byte(p+0xa8)==0 && m.byte(p+0x61a2)==0 {m.put_byte(p+0x61a2,1);}
            h.call_m(0x18423c,&[p+0x110],m);
            if m.byte(p+0x178)<16 {
                let delay=((m.word(p+0x61c4) as u64)<<32|m.word(p+0x61c0) as u64).wrapping_mul(1000);
                h.call_m(0x166054,&[delay as u32,(delay>>32) as u32],m);
            }
        }
        32=>m.put_byte(p+0x61a3,1),
        _=>{}
    }
    0
}

pub fn finish_m(m:&mut Memory<'_>,p:u32,a:u32,b:u32,scratch:u32,h:&mut impl crate::reentrant::HooksM) {let _=finish_checked_m(m,p,a,b,scratch,h);}
fn finish_checked_m(m:&mut Memory<'_>,p:u32,a:u32,b:u32,scratch:u32,h:&mut impl crate::reentrant::HooksM)->bool {
    let saved=core::array::from_fn::<_,7,_>(|i|m.word(scratch+i as u32*4));
    m.put_word(scratch+4,0);m.put_word(scratch+12,0);
    let ok=render_callback_m(m,p,a,b,scratch,scratch+20,true,h);
    for (i,v) in saved.iter().enumerate(){m.put_word(scratch+i as u32*4,*v);}
    ok
}
fn render_callback_m(m:&mut Memory<'_>,p:u32,a:u32,b:u32,result:u32,s:u32,flush:bool,h:&mut impl crate::reentrant::HooksM)->bool {
    let saved=[m.word(s),m.word(s+4)];
    let offset=m.word(p+0x28).wrapping_mul(2);
    let first=m.word(p+0x78).wrapping_add(offset);
    m.put_word(s,0x4b000u32.wrapping_sub(offset)>>1);
    m.put_word(s+4,if m.word(p+0x18)<2 {offset>>1}else{0x25800});

    if h.call_m(0x1a18c4,&[m.word(p+0x61b8),a,b,result,first,s,m.word(p+0x7c),s+4,flush as u32],m)==0x80002001 {
        return false;
    }
    let mut counts=[m.word(s),m.word(s+4)];
    m.put_word(p+0x80,m.word(p+0x80).wrapping_add(counts[0].wrapping_add(counts[1]).wrapping_mul(2)));
    if flush && m.word(p+0x80)<0x400 {
        let mut one=0x4b000i32.wrapping_sub(counts[0] as i32);
        let mut two=0x4b000i32.wrapping_sub(counts[1] as i32);
        if one>0x400 {one=0x400;two=0;}else{let left=0x400i32.wrapping_sub(two);if two>left {two=left;}}
        let pads=[one as u32,two as u32];
        crate::frontend::fill_bytes(m.bytes,first+counts[0]*2,0,pads[0]*2);
        crate::frontend::fill_bytes(m.bytes,m.word(p+0x7c)+counts[1]*2,0,pads[1]*2);
        counts[0]=counts[0].wrapping_add(pads[0]);counts[1]=counts[1].wrapping_add(pads[1]);
        m.put_word(p+0x80,m.word(p+0x80).wrapping_add(pads[0].wrapping_add(pads[1]).wrapping_mul(2)));
    }
    h.call_m(0xc72018,&[first,counts[0]*2],m);
    h.call_m(0xc72018,&[m.word(p+0x7c),counts[1]*2],m);
    h.call_m(0x1a1b68,&[p],m);
    m.put_word(s,saved[0]);m.put_word(s+4,saved[1]);true
}

pub fn clear_text_items_m(m:&mut Memory<'_>,p:u32,tag:u32,scratch:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    let rc=validate_handle(true,tag,0x11d,0x48);
    if rc>=0x80000000 {return rc;}
    let saved=m.word(scratch);m.put_word(scratch,0);
    let rc=h.call_m(0x19611c,&[m.word(p),m.word(p+4),scratch],m);
    m.put_word(scratch,saved);
    if rc>=0x80000000 {return rc;}
    h.call_m(0x19703c,&[m.word(p+0x2c)],m);rc
}

struct ReentrantBoot<'a,H>(&'a mut H);
impl<H:crate::reentrant::HooksM> crate::boot_part_e::Host for ReentrantBoot<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call_m(f,a,m)}
}

pub fn fn_2581ac_m(m:&mut Memory<'_>,p:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    if p==0 {return 0x81102007;}
    let method=m.word(m.word(p+48)+48);
    let rc=h.call_m(method,&[m.word(p+52),m.word(p+56)],m);
    if rc>=0x80000000 {return rc;}
    if m.word(p+12)!=0 {let _=crate::frontend::check_handle(m.word(m.word(p+8)+20)!=0);}
    let buffer=m.word(p+68);
    if buffer!=0 {
        crate::boot_part_e::release(m,m.word(m.word(p+8)+4),buffer,&mut ReentrantBoot(h));
        m.put_word(p+68,0);m.put_half(p+74,0);
    }
    rc
}

pub fn fn_2ba6b4_m(m:&mut Memory<'_>,p:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    if p==0 {return 0x81102007;}
    if m.word(p+64)!=0 {let _=crate::frontend::check_handle(m.word(m.word(p+8)+20)!=0);}
    let rc=fn_2581ac_m(m,m.word(p+12),h);
    if rc<0x80000000 {0}else{rc}
}

pub fn pcre_deinit_m(m:&mut Memory<'_>,p:u32,key:u32,scratch:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    let rc=validate_handle(p!=0,key,0xdd57,0x18);
    if rc>=0x80000000 {return rc;}
    let saved=m.word(scratch);m.put_word(scratch,0);
    let logger=m.word(m.word(p)+16);
    if logger!=0 {h.call_m(0x1738e8,&[logger,0x914050,4,0,0x240174,scratch],m);}
    let mut i=1u32;
    while i<m.half(p+8) as u32 {
        pcre_free_entry_m(m,p,i,false,h);i+=1;
    }
    m.put_half(p+8,1);
    for at in [16,18,20] {m.put_half(p+at,0);}
    if m.word(p+4)!=0 {
        pcre_free_entry_m(m,p,0,true,h);
        h.call_m(0x196428,&[m.word(m.word(p)+4),m.word(p+4)],m);
        m.put_word(p+4,0);
    }
    let logger=m.word(m.word(p)+16);
    if logger!=0 {h.call_m(0x1738e8,&[logger,0x914050,4,0,0x240188,scratch],m);}
    m.put_word(scratch,saved);0
}
fn pcre_free_entry_m(m:&mut Memory<'_>,p:u32,i:u32,head:bool,h:&mut impl crate::reentrant::HooksM) {
    let mut entry=m.word(m.word(p+4)+i*4);
    if entry!=0 {
        h.call_m(0x196428,&[m.word(m.word(p)+4),m.word(entry)],m);
        entry=m.word(m.word(p+4)+i*4);m.put_word(entry,0);
        h.call_m(0x196428,&[m.word(m.word(p)+4),m.word(entry+12)],m);
        entry=m.word(m.word(p+4)+i*4);m.put_word(entry+12,0);
    } else if head {return;}
    h.call_m(0x196428,&[m.word(m.word(p)+4),m.word(m.word(p+4)+i*4)],m);
    m.put_word(m.word(p+4)+i*4,0);
}

impl SchedulerHooksM for Host<'_> {
    fn switch_m(&mut self,_:crate::frontend::PipelineCall,_:u32,_:&mut Memory<'_>)->crate::frontend::PipelineReply {
        self.boot.fault=Some(engine::Error::UnsupportedStage{entry:crate::rom_region::addr(self.rom,0x15429c)});
        crate::frontend::PipelineReply{status:0x80002001,value:0,data:0}
    }
    fn rom(&self) -> crate::rom_region::Rom {
        self.rom
    }
    fn checkpoint_allowed(&mut self) -> bool {
        !self.stop_is_until()
    }
}

#[inline(never)]
fn selection_root_m(m:&mut Memory<'_>,args:[u32;7],platform:&mut dyn engine::HeapPlatform,rom:crate::rom_region::Rom)->Result<u32,engine::Error> {
                let mut nodes=[crate::selection::DecisionNode{question:0,miss:0,hit:0};65536];
                let mut features=crate::component_live_a::FeatureWorkspace{blob:[0;131072],bytes:[[0;2053];60],words:[[0;1025];60],ints:[[0;16384];6],pitch:[0;4096],contexts:[0;9216],modes:[0;16384],markers:[crate::prosody::ProsodyMarker{kind:0,active:0,position:0};1024],carrier:[0;4096]};

                crate::component_live_a::fn_227d64_m(m,args,crate::component_live_a::SCRATCH_227D64,&mut nodes,platform,&mut features,rom)
}

fn component_a2_m(m:&mut Memory<'_>,f:u32,a:&[u32],platform:&mut dyn engine::HeapPlatform,rom:crate::rom_region::Rom)->Result<u32,engine::Error> {
    let dependency=0x3000usize;let root=0x4000u32;
    let mut saved=[0u8;4352];saved.copy_from_slice(&m.bytes[dependency..dependency+4352]);
    let mut h=crate::component_live_a2::LiveCalls::new(platform,dependency as u32).with_rom(rom);
    let rc=match f {
        0x22b3bc=>{let (k,mi)=match rom{crate::rom_region::Rom::Eu=>(0x22b3e4,0x14f96c),_=>(0x22b228,0x14f8a8)};crate::component_live_a2::fn_22b3bc_m(m,a[0],a[1],a[2],a[3],k,mi,root,&mut h)},
        0x229de4=>{let (b,mk)=match rom{crate::rom_region::Rom::Eu=>(0x22a1e0,0x22a230),_=>(0x22a024,0x22a074)};crate::component_live_a2::fn_229de4_m(m,a[0],a[1],[a[2],a[3]],b,mk,root,&mut h)},
        0x22b518=>{let lt=match rom{crate::rom_region::Rom::Eu=>0x15e030,_=>0x15df6c};let dt=match rom{crate::rom_region::Rom::Eu=>0x8fb98c,_=>0x8fa74c};crate::component_live_a2::fn_22b518_m(m,a[0],a[1],a[4],lt,root,&mut h,dt)},
        0x22a0a8=>crate::component_live_a2::fn_22a0a8_m(m,a[0],a[1],a[4],root,&mut h),
        _=>crate::component_live_a2::Calls::call(&mut h,m,f,a),
    };
    let fault=h.fault();m.bytes[dependency..dependency+4352].copy_from_slice(&saved);
    if let Some(e)=fault{Err(e)}else{Ok(rc)}
}

pub fn destroy_subtree_m(m:&mut Memory<'_>,tree:u32,node:u32,end:u32,h:&mut impl crate::reentrant::HooksM) {
    if node==end{return;}
    destroy_subtree_m(m,tree,m.word(node),end,h);
    destroy_subtree_m(m,tree,m.word(node+4),end,h);
    destroy_node_m(m,tree,node,h);
}
fn destroy_node_m(m:&mut Memory<'_>,tree:u32,node:u32,h:&mut impl crate::reentrant::HooksM) {
    let p=m.word(node+16);
    if p!=0 {h.call_m(m.word(m.word(tree+36)+4),&[p,m.word(tree+32)],m);}
    let q=m.word(node+20);
    if q!=0 {h.call_m(m.word(m.word(tree+36)+16),&[q,m.word(tree+32)],m);}
    crate::boot_part_e::release(m,m.word(tree+32),node,&mut ReentrantBoot(h));
}

pub fn clear_tree_m(m:&mut Memory<'_>,tree:u32,h:&mut impl crate::reentrant::HooksM) {
    let node=m.word(tree);
    if node!=tree {
        destroy_subtree_m(m,tree,m.word(node),tree,h);
        destroy_subtree_m(m,tree,m.word(node+4),tree,h);
        destroy_node_m(m,tree,node,h);
    }
    m.put_word(tree+24,0);m.put_word(tree,tree);m.put_word(tree+4,tree);
}
pub trait SwitchHooksM {
    fn read_switch_m(&mut self,obj:u32,value:&mut u32,mem:&mut [u8])->u32;
    fn close_pipes_m(&mut self,p:u32,mem:&mut [u8])->u32;
}

pub fn pipeline_switch_m(h:&mut impl SwitchHooksM,mem:&mut [u8],p:u32,key:u32)->u32 {
    let mut value=0;
    let mut rc=validate_handle(true,key,0x2be,44);
    if rc<0x80000000 {
        let obj=mem_word(mem,mem_word(mem,p+8)+20);
        if obj!=0 {
            rc=h.read_switch_m(obj,&mut value,mem);
            if rc<0x80000000 && value==1 {return rc;}
        }
        rc=h.close_pipes_m(mem_word(mem,p+20),mem);
        mem_store(mem,p+16,1);
    }
    rc
}
struct SwitchHost<'h,'p>(&'h mut Host<'p>);
impl SwitchHooksM for SwitchHost<'_,'_> {
    fn read_switch_m(&mut self,obj:u32,value:&mut u32,bytes:&mut [u8])->u32 {
        let (rc,v)=slots_m(bytes,[0;4],|bytes,s|{
            let mut m=Memory{base:0,bytes};crate::db_live::get_uint(self.0,&mut m,obj,0x2302f0,s)
        });*value=v[0];rc
    }
    fn close_pipes_m(&mut self,p:u32,bytes:&mut [u8])->u32 {self.0.call(&mut Memory{base:0,bytes},0x196770,&[p])}
}

pub fn write_copy_m(bytes:&mut [u8],object:Option<u32>,tag:u32,source:Option<u32>,source_tag:u32,count:u32,written:u32,data:u32,available:u32,h:&mut impl crate::reentrant::HooksM)->u32 {
    let a=[object.unwrap_or(0),tag,source.unwrap_or(0),source_tag,count,written];
    let m=&mut Memory{base:0,bytes};
    let rc=validate_handle(object.is_some(),a[1],0x47de,12);
    if rc>=0x80000000{return rc;}
    if a[4]==0 || (m.byte(a[0])!=1 && m.byte(a[0])!=2) {
        m.put_word(a[5],0);return 0x88f02007;
    }
    m.put_word(a[5],0);
    let rc=h.call_m(0x14fe9c,&[a[2],a[3],data,available],m);
    if rc>=0x80000000 || m.word(available)==0 {return rc;}
    let n=m.word(available).min(a[4]);
    let rc=h.call_m(0x1503b0,&[a[0],a[1],m.word(data),n],m);
    if rc<0x80000000 {m.put_word(a[5],n);h.call_m(0x15017c,&[a[2],a[3],n],m);}
    else {h.call_m(0x15017c,&[a[2],a[3],0],m);}
    rc
}

const SCRATCH_2A762C: u32 = 0xe0fe000;

pub fn fn_2a3740_m(
    m: &mut Memory<'_>,
    s: u32,
    h: &mut Host,
    k: &crate::boot_port_3b::DestructionEnvironment,
) -> u32 {
    for i in 0..6 {
        m.put_word(s.wrapping_add(0x2c).wrapping_add(i * 4), 0);
    }
    crate::boot_port_3b::fn_2db4a0(m, h, s, k)
}

pub fn fn_2a3414_m(
    m: &mut Memory<'_>,
    p: u32,
    h: &mut Host,
    k: &crate::boot_port_3b::DestructionEnvironment,
) -> u32 {
    for &off in &[0x40u32, 0x44, 0x48, 0x50, 0x54, 0x4c, 0x38, 0x34, 0x3c] {
        m.put_word(p.wrapping_add(off), 0);
    }
    crate::boot_port_3b::fn_2db4a0(m, h, p, k);
    crate::boot_port_3b::fn_2db4a0(m, h, p.wrapping_add(24), k)
}

pub fn fn_2a762c_m(
    m: &mut Memory<'_>,
    o: u32,
    a: u32,
    h: &mut Host,
    k: &crate::boot_port_3b::DestructionEnvironment,
) -> u32 {
    fn_2a3740_m(m, o.wrapping_add(0x498), h, k);
    fn_2a3740_m(m, o.wrapping_add(0x4e0), h, k);
    fn_2a3414_m(m, o.wrapping_add(0x3d4), h, k);
    crate::boot_port_3b::fn_2db4a0(m, h, o, k);
    if m.word(a.wrapping_add(0x2c)) > 1 {
        let ctx = m.word(o.wrapping_add(0x18));
        crate::log_live::fn_1738e8(
            m,
            m.word(ctx.wrapping_add(0x10)),
            0x8fa510,
            4,
            0,
            0x2a7694,
            k.scratch,
            h,
        );
    }
    0
}
fn component_b_m(m:&mut Memory<'_>,f:u32,a:&[u32],platform:&mut dyn engine::HeapPlatform)->Result<u32,engine::Error> {
    let dependency=0x6000usize;let root=0x7000;
    let mut saved=[0u8;8192];saved.copy_from_slice(&m.bytes[dependency..dependency+8192]);
    let mut h=crate::component_live_b::LiveHost{platform,fault:None,scratch:dependency as u32,rom:crate::rom_region::Rom::Us};
    let rc=match f {
        0x1e4a3c=>crate::component_live_b::fn_1e4a3c_m(m,a[0],a[1],&mut h),
        0x1f48cc=>crate::component_live_b::fn_1f48cc_m(m,a[..5].try_into().unwrap(),root,&mut h),
        _=>crate::component_live_b::Host::call_m(&mut h,f,a,m),
    };
    let fault=h.fault;m.bytes[dependency..dependency+8192].copy_from_slice(&saved);
    if let Some(e)=fault{Err(e)}else{Ok(rc)}
}

mod backend_codec {
use crate::orchestration::SetupState;use crate::queue::SpeechFrame;
use crate::voice::{VoiceEvent, VoiceQueue, VoiceState, VoiceTransition};
pub fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
pub fn put(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
pub fn queue(bytes: &[u8]) -> VoiceQueue {
    assert!(word(bytes, 0) < 256 && word(bytes, 4) < 256);
    VoiceQueue {
        read: word(bytes, 0) as u8,
        write: word(bytes, 4) as u8,
        events: core::array::from_fn(|i| VoiceEvent {
            kind: word(bytes, 8 + i * 20) as i32,
            value: f32::from_bits(word(bytes, 12 + i * 20)),
            target: f32::from_bits(word(bytes, 16 + i * 20)),
            duration: f32::from_bits(word(bytes, 20 + i * 20)),
            delay: word(bytes, 24 + i * 20) as i32,
        }),
    }
}
pub fn write_queue(q: &VoiceQueue, bytes: &mut [u8]) {
    put(bytes, 0, q.read as u32);
    put(bytes, 4, q.write as u32);
    for (i, e) in q.events.iter().enumerate() {
        for (j, value) in [
            e.kind as u32,
            e.value.to_bits(),
            e.target.to_bits(),
            e.duration.to_bits(),
            e.delay as u32,
        ]
        .iter()
        .enumerate()
        {
            put(bytes, 8 + i * 20 + j * 4, *value);
        }
    }
}

const FLOAT_OFFSETS: [usize; 22] = [
    8, 0x5c, 0x6c, 0x70, 0x74, 0x78, 0x80, 0x84, 0x88, 0x8c, 0x90, 0x94, 0x98, 0x9c, 0xa0, 0xa4,
    0xb0, 0xc0, 0xc4, 0xc8, 0xcc, 0xd0,
];

pub fn state(bytes: &[u8]) -> VoiceState {
    let f = |o| f32::from_bits(word(bytes, o));
    VoiceState {
        sample_rate: f(8),
        enabled: bytes[0x48] != 0,
        position: f(0x5c),
        time: word(bytes, 0x64) as i32,
        value6c: f(0x6c),
        value70: f(0x70),
        gain: f(0x74),
        rate: f(0x78),
        period: f(0x80),
        value84: f(0x84),
        value88: f(0x88),
        base8c: f(0x8c),
        coefficient90: f(0x90),
        target94: f(0x94),
        base98: f(0x98),
        coefficient9c: f(0x9c),
        targeta0: f(0xa0),
        valuea4: f(0xa4),
        durationa8: word(bytes, 0xa8) as i32,
        valueb0: f(0xb0),
        durationb4: word(bytes, 0xb4) as i32,
        active: core::array::from_fn(|i| bytes[0xbc + i] != 0),
        targets: core::array::from_fn(|i| f(0xc0 + i * 4)),
        phases: core::array::from_fn(|i| f(0xcc + i * 4)),
        offsetd8: word(bytes, 0xd8) as i32,
        offsetdc: word(bytes, 0xdc) as i32,
        flag: bytes[0xe0] != 0,
        transitions: core::array::from_fn(|i| VoiceTransition {
            kind: word(bytes, 0x28f8 + i * 20) as i32,
            reciprocal: f(0x28fc + i * 20),
            remaining: word(bytes, 0x2908 + i * 20) as i32,
        }),
    }
}
pub fn write_state(s: &VoiceState, bytes: &mut [u8]) {
    let floats = [
        s.sample_rate,
        s.position,
        s.value6c,
        s.value70,
        s.gain,
        s.rate,
        s.period,
        s.value84,
        s.value88,
        s.base8c,
        s.coefficient90,
        s.target94,
        s.base98,
        s.coefficient9c,
        s.targeta0,
        s.valuea4,
        s.valueb0,
        s.targets[0],
        s.targets[1],
        s.targets[2],
        s.phases[0],
        s.phases[1],
    ];
    for (o, f) in FLOAT_OFFSETS.into_iter().zip(floats) {
        put(bytes, o, f.to_bits());
    }
    put(bytes, 0xd4, s.phases[2].to_bits());
    for (o, v) in [
        (0x64, s.time),
        (0xa8, s.durationa8),
        (0xb4, s.durationb4),
        (0xd8, s.offsetd8),
        (0xdc, s.offsetdc),
    ] {
        put(bytes, o, v as u32);
    }
    bytes[0x48] = s.enabled as u8;
    bytes[0xe0] = s.flag as u8;
    for i in 0..3 {
        bytes[0xbc + i] = s.active[i] as u8;
    }
    for (i, t) in s.transitions.iter().enumerate() {
        put(bytes, 0x28f8 + i * 20, t.kind as u32);
        put(bytes, 0x28fc + i * 20, t.reciprocal.to_bits());
        put(bytes, 0x2908 + i * 20, t.remaining as u32);
    }
}
use crate::{
    pitch::{PeriodTracker, PitchTracker},
    synthesis::SynthesisState,
};
pub fn synthesis_state(bytes: &[u8], tracker: &[u8]) -> SynthesisState {
    let f = |offset| f32::from_bits(word(bytes, offset));
    SynthesisState {
        voice: state(bytes),
        control_period: word(bytes, 0xc) as i32,
        input_window: word(bytes, 0x14) as usize,
        minimum_pitch: f(0x1c),
        history: [
            i16::from_le_bytes(bytes[0x34..0x36].try_into().unwrap()),
            i16::from_le_bytes(bytes[0x36..0x38].try_into().unwrap()),
        ],
        period: word(bytes, 0x4c) as i32,
        grain_length: word(bytes, 0x54) as usize,
        output_position: f(0x58),
        control_time: word(bytes, 0x60) as i32,
        output_remainder: word(bytes, 0x68) as i32,
        smoothed_period: f(0x7c),
        pitch_modulation_phase: word(bytes, 0xac) as i32,
        stretch_modulation_phase: word(bytes, 0xb8) as i32,
        tone_phase: f(0xe4),
        normalize: bytes[0x2920] != 0,
        tracker: PitchTracker::Crossings(PeriodTracker {
            min_period: word(tracker, 0) as i32,
            max_period: word(tracker, 4) as i32,
            period: word(tracker, 8) as i32,
            period_delta: word(tracker, 12) as i32,
            reset: tracker[16] as i8,
            stable_count: word(tracker, 20) as i32,
            polarity: word(tracker, 24) as i32,
        }),
    }
}

pub fn store_state(s: &SynthesisState, bytes: &mut [u8], tracker: &mut [u8]) {
    write_state(&s.voice, bytes);
    for (offset, value) in [
        (0x4c, s.period as u32),
        (0x54, s.grain_length as u32),
        (0x58, s.output_position.to_bits()),
        (0x60, s.control_time as u32),
        (0x68, s.output_remainder as u32),
        (0x7c, s.smoothed_period.to_bits()),
        (0xac, s.pitch_modulation_phase as u32),
        (0xb8, s.stretch_modulation_phase as u32),
        (0xe4, s.tone_phase.to_bits()),
    ] {
        put(bytes, offset, value);
    }
    bytes[0x34..0x36].copy_from_slice(&s.history[0].to_le_bytes());
    bytes[0x36..0x38].copy_from_slice(&s.history[1].to_le_bytes());
    let t = match &s.tracker {
        PitchTracker::Crossings(t) => t,
        PitchTracker::Resonator(e) => {
            let f = |v: f32| v.to_bits();
            for (offset, value) in [
                (28, f(e.previous)),
                (32, f(e.older)),
                (36, f(e.oldest)),
                (40, f(e.coefficient)),
                (44, f(e.sample_rate)),
            ] {
                put(tracker, offset, value);
            }
            &e.period
        }
    };
    for (offset, value) in [
        (0, t.min_period),
        (4, t.max_period),
        (8, t.period),
        (12, t.period_delta),
        (20, t.stable_count),
        (24, t.polarity),
    ] {
        put(tracker, offset, value as u32);
    }
    tracker[16] = t.reset as u8;
}

pub fn setup_state(b: &[u8]) -> SetupState {
    let f = |o| f32::from_bits(word(b, o));
    SetupState {
        sample_rate: f(0x4bc4),
        active: b[0x4bd0] != 0,
        polarity: f(0x4bd4),
        voice_enabled: b[0x4bd8] != 0,
        control_enabled: b[0x4bd9] != 0,
        state: word(b, 0x4bdc) as i32,
        gain: f(0x4be0),
        pitch: f(0x4be4),
        resampling: f(0x4be8),
        effects: core::array::from_fn(|i| f(0x4bec + 4 * i)),
        smoothing: f(0x4bf8),
        blend: f(0x4bfc),
        frequency: f(0x4c00),
        voice_gain: f(0x4c04),
        value4c08: f(0x4c08),
        timing_blend: f(0x4c0c),
        duration_scale: f(0x4c10),
        value4c14: f(0x4c14),
        value4c18: f(0x4c18),
        tempo: f(0x4c1c),
        modulation: [f(0x4c20), f(0x4c24)],
        stretch: [f(0x4c28), f(0x4c2c)],
        enabled: b[0x4c30] != 0,
        mode: word(b, 0x4c34) as i32,
        reset_mode: word(b, 0x4c40) as i32,
        reset_count: word(b, 0x4c6c) as i32,
    }
}
const FLOATS: [usize; 23] = [
    0x4bc4,
    0x4bd4,
    0x4be0,
    0x4be4,
    0x4be8,
    0x4bec,
    0x4bf0,
    0x4bf4,
    0x4bf8,
    0x4bfc,
    0x4c00,
    0x4c04,
    0x4c08,
    0x4c0c,
    0x4c10,
    0x4c14,
    0x4c18,
    0x4c1c,
    0x4c20,
    0x4c24,
    0x4c28,
    0x4c2c,
    0x31f4 + 0x20,
];
pub fn write_setup(s: &SetupState, b: &mut [u8]) {
    let floats = [
        s.sample_rate,
        s.polarity,
        s.gain,
        s.pitch,
        s.resampling,
        s.effects[0],
        s.effects[1],
        s.effects[2],
        s.smoothing,
        s.blend,
        s.frequency,
        s.voice_gain,
        s.value4c08,
        s.timing_blend,
        s.duration_scale,
        s.value4c14,
        s.value4c18,
        s.tempo,
        s.modulation[0],
        s.modulation[1],
        s.stretch[0],
        s.stretch[1],
    ];
    for (o, f) in FLOATS.into_iter().zip(floats) {
        put(b, o, f.to_bits());
    }
    for (o, v) in [
        (0x4bd0, s.active),
        (0x4bd8, s.voice_enabled),
        (0x4bd9, s.control_enabled),
        (0x4c30, s.enabled),
    ] {
        b[o] = v as u8;
    }
    for (o, v) in [
        (0x4bdc, s.state),
        (0x4c34, s.mode),
        (0x4c40, s.reset_mode),
        (0x4c6c, s.reset_count),
    ] {
        put(b, o, v as u32);
    }
}

pub fn decode(b: &[u8]) -> SpeechFrame {
    let mut s = SpeechFrame::default();
    s.started = b[4] != 0;
    s.suppress_boundary = b[0x1394] as i8;
    s.start_value = word(b, 8) as _;
    s.boundary_value = word(b, 12) as _;
    s.command_count = word(b, 20) as _;
    s.audio.count = word(b, 2076) as _;
    s.audio.written = word(b, 2080) as _;
    s.audio.pending = word(b, 2084) as _;
    s.character_count = word(b, 2088) as _;
    s.group_count = word(b, 2860) as _;
    s.output_count = word(b, 3888) as _;
    s.current_time = word(b, 4916) as _;
    s.before_time = word(b, 4920) as _;
    s.after_time = word(b, 4924) as _;
    s.pause_count = word(b, 4992) as _;
    s.pause_total = word(b, 4996) as _;
    s.position = word(b, 5000) as _;
    s.last_command = word(b, 5004) as _;
    s.last_pause = word(b, 5008) as _;
    s.commands = core::array::from_fn(|i| word(b, 24 + i * 4) as _);
    s.audio.offsets = core::array::from_fn(|i| word(b, 1052 + i * 4) as _);
    s.audio.lengths = core::array::from_fn(|i| word(b, 1564 + i * 4) as _);
    s.characters = core::array::from_fn(|i| {
        u16::from_le_bytes(b[2092 + i * 2..2092 + i * 2 + 2].try_into().unwrap())
    });
    s.character_offsets = core::array::from_fn(|i| word(b, 2348 + i * 4) as _);
    s.group_starts = core::array::from_fn(|i| word(b, 2864 + i * 4) as _);
    s.group_lengths = core::array::from_fn(|i| word(b, 3376 + i * 4) as _);
    s.output_starts = core::array::from_fn(|i| word(b, 3892 + i * 4) as _);
    s.output_lengths = core::array::from_fn(|i| word(b, 4404 + i * 4) as _);
    s.pauses = core::array::from_fn(|i| word(b, 4928 + i * 4) as _);
    s
}

pub fn encode(s: &SpeechFrame, b: &mut [u8]) {
    b[4] = s.started as u8;
    b[0x1394] = s.suppress_boundary as u8;
    put(b, 8, s.start_value as u32);
    put(b, 12, s.boundary_value as u32);
    put(b, 20, s.command_count as u32);
    put(b, 2076, s.audio.count as u32);
    put(b, 2080, s.audio.written as u32);
    put(b, 2084, s.audio.pending as u32);
    put(b, 2088, s.character_count as u32);
    put(b, 2860, s.group_count as u32);
    put(b, 3888, s.output_count as u32);
    put(b, 4916, s.current_time as u32);
    put(b, 4920, s.before_time as u32);
    put(b, 4924, s.after_time as u32);
    put(b, 4992, s.pause_count as u32);
    put(b, 4996, s.pause_total as u32);
    put(b, 5000, s.position as u32);
    put(b, 5004, s.last_command as u32);
    put(b, 5008, s.last_pause as u32);
    for i in 0..256 {
        put(b, 24 + i * 4, s.commands[i] as u32);
    }
    for i in 0..128 {
        put(b, 1052 + i * 4, s.audio.offsets[i] as u32);
    }
    for i in 0..128 {
        put(b, 1564 + i * 4, s.audio.lengths[i] as u32);
    }
    for i in 0..128 {
        b[2092 + i * 2..2092 + i * 2 + 2].copy_from_slice(&s.characters[i].to_le_bytes());
    }
    for i in 0..128 {
        put(b, 2348 + i * 4, s.character_offsets[i] as u32);
    }
    for i in 0..128 {
        put(b, 2864 + i * 4, s.group_starts[i] as u32);
    }
    for i in 0..128 {
        put(b, 3376 + i * 4, s.group_lengths[i] as u32);
    }
    for i in 0..128 {
        put(b, 3892 + i * 4, s.output_starts[i] as u32);
    }
    for i in 0..128 {
        put(b, 4404 + i * 4, s.output_lengths[i] as u32);
    }
    for i in 0..16 {
        put(b, 4928 + i * 4, s.pauses[i] as u32);
    }
}

}

pub struct BackendWorkspace<'a> {
    pub input:&'a mut [i16],pub output:&'a mut [i32],pub grain:&'a mut [i32],
    pub window:&'a mut [f32],pub modulation:&'a mut [f32;1024],
    pub normalization:&'a mut [f32;32],pub wave:&'a mut [i16;1024],
    pub voice_delays:[&'a mut [f32];3],pub sample_delay:&'a mut [u16],
    pub audio:&'a mut [i16;crate::queue::AUDIO_CAPACITY],
    pub samples:&'a mut [i16],pub events:&'a mut [crate::queue::SpeechEvent],
    pub first:&'a mut [i16],pub second:&'a mut [i16],

    pub parameter_values:Option<[&'a [u8];2]>,
    pub rom:crate::rom_region::Rom,
}
fn backend_capacity(required:usize,available:usize)->Result<(),engine::Error> {
    if required>available{Err(engine::Error::Capacity{required,available})}else{Ok(())}
}
fn read_shorts_m(m:&Memory<'_>,p:u32,out:&mut [i16]){for(i,v)in out.iter_mut().enumerate(){*v=m.half(p+i as u32*2)as i16;}}
fn read_floats_m(m:&Memory<'_>,p:u32,out:&mut [f32]){for(i,v)in out.iter_mut().enumerate(){*v=f32::from_bits(m.word(p+i as u32*4));}}
fn read_ints_m(m:&Memory<'_>,p:u32,out:&mut [i32]){for(i,v)in out.iter_mut().enumerate(){*v=m.word(p+i as u32*4)as i32;}}
fn parameter_span_m(m:&Memory<'_>,engine:u32,key:&[u8])->Result<(u32,usize),engine::Error> {
    let params=m.word(m.word(engine+8)+20);let tree=m.word(params+8)+24;let mut node=m.word(tree);
    while node!=tree {
        let stored=m.word(node+16);
        let order=crate::frontend::compare_str(key,terminated(m.bytes,stored));
        if order==0 {let entry=m.word(node+20);return Ok((m.word(entry),m.word(entry+4)as usize));}
        node=m.word(node+if order<0{0}else{4});
    }
    Err(engine::Error::EngineStatus(0x8b702014))
}

pub fn render_backend_m(m:&mut Memory<'_>,a:[u32;9],w:&mut BackendWorkspace<'_>)->Result<[usize;2],engine::Error> {
    use crate::{buffers::{RingBuffer,SampleFilter,VoiceFilter},effects::VoiceEffect,
        frames::{LipSync,PlannedSegments,TimingSegments},orchestration::Setup,
        output::SpeechTiming,queue::*,resources::*,tables::NumericTables};
    let p=a[0];let eng=a[1];let data=a[3];assert_eq!(m.base,0);
    let mut raw=[0u8;0x4d00];raw.copy_from_slice(&m.bytes[p as usize..p as usize+0x4d00]);
    let buffer_offsets=[0x20,0x24,0x2c,0x3c,0x44,0x50];
    let ptr: [u32;6]=core::array::from_fn(|i|m.word(p+buffer_offsets[i]));
    let sizes:[usize;6]=core::array::from_fn(|i|m.word(ptr[i]-12)as usize);
    for(size,width)in sizes.into_iter().zip([4,4,2,4,1,4]){if size%width!=0{return Err(engine::Error::InvalidSnapshot);}}
    backend_capacity(sizes[0]/4,w.modulation.len())?;backend_capacity(sizes[1]/4,w.window.len())?;
    backend_capacity(sizes[2]/2,w.input.len())?;backend_capacity(sizes[3]/4,w.output.len())?;
    backend_capacity(sizes[4],64)?;backend_capacity(sizes[5]/4,w.grain.len())?;
    let mut tracker=[0u8;64];tracker[..sizes[4]].copy_from_slice(&m.bytes[ptr[4]as usize..ptr[4]as usize+sizes[4]]);
    read_floats_m(m,ptr[0],&mut w.modulation[..sizes[0]/4]);read_floats_m(m,ptr[1],&mut w.window[..sizes[1]/4]);
    read_shorts_m(m,ptr[2],&mut w.input[..sizes[2]/2]);read_ints_m(m,ptr[3],&mut w.output[..sizes[3]/4]);read_ints_m(m,ptr[5],&mut w.grain[..sizes[5]/4]);
    let headers: [u32;4]=core::array::from_fn(|i|m.word(p+[0x31d8,0x31dc,0x31e0,0x3210][i]));
    let ring_data:[u32;4]=core::array::from_fn(|i|m.word(headers[i]+4));
    let ring_len:[usize;4]=core::array::from_fn(|i|m.word(headers[i]+8)as usize);
    let ring_pos:[u32;4]=core::array::from_fn(|i|m.word(headers[i]+12));
    for i in 0..3{backend_capacity(ring_len[i],w.voice_delays[i].len())?;read_floats_m(m,ring_data[i],&mut w.voice_delays[i][..ring_len[i]]);}
    backend_capacity(ring_len[3],w.sample_delay.len())?;
    for(i,v)in w.sample_delay[..ring_len[3]].iter_mut().enumerate(){*v=m.half(ring_data[3]+i as u32*2);}
    let audio_ptr=m.word(crate::rom_region::addr(w.rom,0xa7b794));read_shorts_m(m,audio_ptr,w.audio);

    for i in 0..128 {
        let at=0x3c44+i*4;let value=backend_codec::word(&raw,at);
        if audio_ptr<=value && value<=audio_ptr+crate::queue::AUDIO_CAPACITY as u32*2 {
            backend_codec::put(&mut raw,at,(value-audio_ptr)/2);
        }
    }
    read_shorts_m(m,m.word(p+0x31ec),w.wave);read_floats_m(m,crate::rom_region::addr(w.rom,0xa7b7f0),w.normalization);

    let samples=m.word(data+4)as usize/2;let events=m.word(data+12)as usize;
    backend_capacity(samples,w.samples.len())?;backend_capacity(events,w.events.len())?;
    read_shorts_m(m,m.word(data+8),&mut w.samples[..samples]);
    let event_ptr=m.word(data+16);
    for(i,e)in w.events[..events].iter_mut().enumerate(){
        let q=event_ptr+i as u32*40;*e=SpeechEvent{kind:m.half(q+4),value:m.word(q+12)as i32,position:m.word(q+16),character:m.half(q+24),command:m.word(q+28)};
    }
    let capacities=[m.word(a[5])as usize,m.word(a[7])as usize];
    backend_capacity(capacities[0],w.first.len())?;backend_capacity(capacities[1],w.second.len())?;
    let mut synth=backend_codec::synthesis_state(&raw,&tracker);let mut voice=synth.voice.clone();
    if w.rom==crate::rom_region::Rom::Eu{
        let g=|o:usize|f32::from_bits(u32::from_le_bytes(tracker[o..o+4].try_into().unwrap()));
        synth.tracker=crate::pitch::PitchTracker::Resonator(crate::pitch::EuTracker{period:crate::pitch::PeriodTracker{
            min_period:m.word(ptr[4])as i32,max_period:m.word(ptr[4]+4)as i32,
            period:m.word(ptr[4]+8)as i32,period_delta:m.word(ptr[4]+12)as i32,
            reset:tracker[16]as i8,stable_count:m.word(ptr[4]+20)as i32,
            polarity:m.word(ptr[4]+24)as i32},
            previous:g(28),older:g(32),oldest:g(36),coefficient:g(40),sample_rate:g(44)});
    }
    let mut input_events=backend_codec::queue(&raw[0xe8..0x14f0]);let mut commands=backend_codec::queue(&raw[0x14f0..0x28f8]);
    let [r0,r1,r2]=&mut w.voice_delays;
    let mut voice_filter=VoiceFilter{rings:[
        RingBuffer{samples:&mut r0[..ring_len[0]],position:ring_pos[0]},
        RingBuffer{samples:&mut r1[..ring_len[1]],position:ring_pos[1]},
        RingBuffer{samples:&mut r2[..ring_len[2]],position:ring_pos[2]},
    ],history:f32::from_bits(backend_codec::word(&raw,0x31f0))};
    let mut filter=SampleFilter{ring:RingBuffer{samples:&mut w.sample_delay[..ring_len[3]],position:ring_pos[3]},history:f32::from_bits(backend_codec::word(&raw,0x3214))};
    let pair=|off:usize|core::array::from_fn(|i|backend_codec::word(&raw,0x31a4+off+i*4)as i32);
    let mut effect=VoiceEffect{gain:backend_codec::word(&raw,0x31ac)as i32,feedforward:pair(12),feedback:pair(20),delay:pair(28),rate:pair(36),depth:pair(44),phase:pair(64)};
    let mut state=backend_codec::setup_state(&raw);
    let mut controls=crate::controls::FilterControls{values:core::array::from_fn(|i|backend_codec::word(&raw,0x31fc+i*4)as i32)};
    let mut setup=Setup{state:&mut state,voice_active:raw[0x48]!=0,voice:&mut voice,input:&mut input_events,commands:&mut commands,filter:&mut filter,voice_filter:&mut voice_filter,controls:&mut controls};
    let mut timing=SpeechTiming{phoneme:u16::from_le_bytes(raw[0x4c38..0x4c3a].try_into().unwrap()),pitch_duration:backend_codec::word(&raw,0x4c3c)as i32};
    let mut starts:[i32;128]=core::array::from_fn(|i|backend_codec::word(&raw,0x2924+i*4)as i32);
    let mut lengths:[i32;128]=core::array::from_fn(|i|backend_codec::word(&raw,0x2b24+i*4)as i32);
    let mut voiced:[u8;128]=core::array::from_fn(|i|raw[0x2d24+i]);
    let mut scales:[f32;128]=core::array::from_fn(|i|f32::from_bits(backend_codec::word(&raw,0x2da4+i*4)));
    let mut positions:[i32;128]=core::array::from_fn(|i|backend_codec::word(&raw,0x2fa4+i*4)as i32);
    let mut segments=PlannedSegments{starts:&mut starts,lengths:&mut lengths,voiced:&mut voiced,output:TimingSegments{scales:&mut scales,positions:&mut positions}};
    let mut lip_samples:[i16;0x301]=core::array::from_fn(|i|i16::from_le_bytes(raw[0x321a+i*2..0x321c+i*2].try_into().unwrap()));
    let mut lip=LipSync{samples:&mut lip_samples,count:backend_codec::word(&raw,0x381c)as i32,limit:backend_codec::word(&raw,0x3824)as i32};
    let mut frame=backend_codec::decode(&raw[0x3828..0x4bc4]);let mut position=backend_codec::word(&raw,0x4bc8)as i32;
    let boundary=m.word(p+0x4bc0);let voice_boundary=VoiceBoundary{enabled:m.byte(boundary+4)!=0,distance:m.word(boundary+20)as i32};
    let parameter_spans=if w.parameter_values.is_none(){Some([parameter_span_m(m,eng,b"rate\0")?,parameter_span_m(m,eng,b"voiceml\0")?])}else{None};
    let synth_ptr=m.word(eng+0x1a8);let resource_synth=[m.half(synth_ptr),m.half(synth_ptr+2)];
    let table=m.word(m.word(eng+0x1a0)+16);let dims=[m.half(table+20),m.half(table+18),m.half(table+32)];
    let rows=m.word(table+36);let row_len=(dims[0]as usize+dims[1]as usize)*dims[2]as usize;
    let made;
    let final_resource;
    let lang=match w.rom{
        crate::rom_region::Rom::Us=>crate::pitch::Language::UsEnglish,
        crate::rom_region::Rom::Eu=>crate::pitch::Language::UkEnglish,
        crate::rom_region::Rom::Kr=>crate::pitch::Language::Korean,
    };
    let mut pscratch=[0f32;1024];
    let freqs:[f32;5]=if w.rom==crate::rom_region::Rom::Eu{
        [f32::from_bits(m.word(0x8bf7dc)),f32::from_bits(m.word(0x8bf7e0)),
         f32::from_bits(m.word(0x8bf7e4)),f32::from_bits(m.word(0x8bf7e8)),
         f32::from_bits(m.word(0x8bf7ec))]
    }else{[0.;5]};
    {
        let bytes=&*m.bytes;
        let values=if let Some(v)=w.parameter_values {v}else{let [(rate,rate_len),(ml,ml_len)]=parameter_spans.unwrap();[&bytes[rate as usize..rate as usize+rate_len],&bytes[ml as usize..ml as usize+ml_len]]};
        let parameters=[ParameterEntry{key:b"rate\0",value:values[0]},ParameterEntry{key:b"voiceml\0",value:values[1]}];
        let resource=VoiceResource{bytes:&bytes[rows as usize..rows as usize+row_len],count:dims[2],key_width:dims[0],value_width:dims[1],disabled:false};
        let mut resources=EngineResources{valid:true,environment_status:0,parameters:Parameters{entries:&parameters},phonemes:PhonemeMaps{maps:[resource;4]},articulation:Articulation{symbols:&bytes[crate::rom_region::addr(w.rom,0x8d8458) as usize..crate::rom_region::addr(w.rom,0x8d8458) as usize+332*8],values:&bytes[crate::rom_region::addr(w.rom,0x8d8eb8) as usize..crate::rom_region::addr(w.rom,0x8d8eb8) as usize+332*18]},synth:resource_synth};
        let tables=NumericTables::from_bytes(&bytes[crate::rom_region::addr(w.rom,0xa21fb0) as usize..crate::rom_region::addr(w.rom,0xa21fb0) as usize+NumericTables::CONSTANT_BYTES],&bytes[crate::rom_region::addr(w.rom,0x8cbfe0) as usize..crate::rom_region::addr(w.rom,0x8cbfe0) as usize+NumericTables::COSINE_BYTES]).ok_or(engine::Error::InvalidSnapshot)?;
        let class_base=crate::rom_region::addr(w.rom,0xb6b300) as usize;let classification=bytes[class_base..class_base+256].try_into().unwrap();
        made=crate::engine_backend::render_backend(&mut setup,&mut synth,&mut frame,w.audio,&mut position,voice_boundary,classification,&mut timing,&mut segments,&mut lip,&mut resources,
            &mut w.input[..sizes[2]/2],&mut w.output[..sizes[3]/4],&mut w.grain[..sizes[5]/4],&w.window[..sizes[1]/4],w.modulation,w.normalization,&mut effect,w.wave,&tables,
            &w.samples[..samples],&w.events[..events],[&mut w.first[..capacities[0]],&mut w.second[..capacities[1]]],a[8]!=0,raw[0x4bd9]!=0,
            &mut crate::engine_backend::BackendRegion{language:lang,scratch:&mut pscratch,frequencies:&freqs,korean_timing:None});
        final_resource=resources.synth;
    }
    backend_codec::write_setup(setup.state,&mut raw);backend_codec::write_state(setup.voice,&mut raw);
    backend_codec::write_queue(setup.input,&mut raw[0xe8..0x14f0]);backend_codec::write_queue(setup.commands,&mut raw[0x14f0..0x28f8]);
    for i in 0..4{backend_codec::put(&mut raw,0x31fc+i*4,setup.controls.values[i]as u32);}
    synth.voice=setup.voice.clone();backend_codec::store_state(&synth,&mut raw,&mut tracker);
    for i in 0..128{
        backend_codec::put(&mut raw,0x2924+i*4,segments.starts[i]as u32);backend_codec::put(&mut raw,0x2b24+i*4,segments.lengths[i]as u32);raw[0x2d24+i]=segments.voiced[i];
        backend_codec::put(&mut raw,0x2da4+i*4,segments.output.scales[i].to_bits());backend_codec::put(&mut raw,0x2fa4+i*4,segments.output.positions[i]as u32);
    }
    for(i,v)in lip.samples.iter().enumerate(){raw[0x321a+i*2..0x321c+i*2].copy_from_slice(&v.to_le_bytes());}
    backend_codec::put(&mut raw,0x381c,lip.count as u32);raw[0x4c38..0x4c3a].copy_from_slice(&timing.phoneme.to_le_bytes());backend_codec::put(&mut raw,0x4c3c,timing.pitch_duration as u32);
    backend_codec::encode(&frame,&mut raw[0x3828..0x4bc4]);

    for i in 0..(frame.audio.count.max(0)as usize).min(128){backend_codec::put(&mut raw,0x3c44+i*4,audio_ptr+(frame.audio.offsets[i]as u32)*2);}
    backend_codec::put(&mut raw,0x4bc8,position as u32);backend_codec::put(&mut raw,0x31e4,effect.phase[0]as u32);backend_codec::put(&mut raw,0x31e8,effect.phase[1]as u32);
    backend_codec::put(&mut raw,0x31f0,setup.voice_filter.history.to_bits());
    m.bytes[p as usize..p as usize+raw.len()].copy_from_slice(&raw);
    for(i,v)in w.input[..sizes[2]/2].iter().enumerate(){m.put_half(ptr[2]+i as u32*2,*v as u16);}
    for(i,v)in w.output[..sizes[3]/4].iter().enumerate(){m.put_word(ptr[3]+i as u32*4,*v as u32);}
    for(i,v)in w.grain[..sizes[5]/4].iter().enumerate(){m.put_word(ptr[5]+i as u32*4,*v as u32);}
    m.bytes[ptr[4]as usize..ptr[4]as usize+sizes[4]].copy_from_slice(&tracker[..sizes[4]]);
    for i in 0..3{
        m.put_word(headers[i]+12,setup.voice_filter.rings[i].position);
        for(j,v)in setup.voice_filter.rings[i].samples.iter().enumerate(){m.put_word(ring_data[i]+j as u32*4,v.to_bits());}
    }
    m.put_word(headers[3]+12,setup.filter.ring.position);
    for(j,v)in setup.filter.ring.samples.iter().enumerate(){m.put_half(ring_data[3]+j as u32*2,*v);}
    for(i,v)in w.audio.iter().enumerate(){m.put_half(audio_ptr+i as u32*2,*v as u16);}
    for(i,v)in final_resource.iter().enumerate(){m.put_half(synth_ptr+i as u32*2,*v);}
    for(i,v)in w.first[..made[0]].iter().enumerate(){m.put_half(a[4]+i as u32*2,*v as u16);}
    for(i,v)in w.second[..made[1]].iter().enumerate(){m.put_half(a[6]+i as u32*2,*v as u16);}
    m.put_word(a[5],made[0]as u32);m.put_word(a[7],made[1]as u32);
    Ok(made)
}

pub fn notify_output_m(m:&mut Memory<'_>,outer:u32,node:u32,h:&mut impl crate::reentrant::HooksM){
    let inner=m.word(outer+0x68);h.call_m(0x1840fc,&[inner+0x70],m);
    let mut cursor=m.word(inner+0x30);
    while cursor!=0{
        if cursor==node{if m.byte(node+17)!=3{m.put_byte(inner+0x6f,m.byte(inner+0x6f)|2);}break;}
        cursor=m.word(cursor+20);
    }
    h.call_m(0x18423c,&[inner+0x70],m);
}

pub fn publish_output_m(m:&mut Memory<'_>,p:u32,rom:crate::rom_region::Rom,h:&mut impl crate::reentrant::HooksM)->Result<(),engine::Error>{
    h.call_m(0x1840fc,&[p+100],m);
    if m.byte(crate::rom_region::addr(rom,0xa7bf5c))!=0{h.call_m(0x1840e8,&[],m);}else{
        let global=h.call_m(0x18ac58,&[],m);h.call_m(0x1840fc,&[global+0x1c4],m);
    }
    let size=m.word(p+0x84);
    let [quotient,remainder]=crate::output::unsigned_division(m.word(p+0x80)>>1,size).map_err(|_|engine::Error::InvalidSnapshot)?;
    if m.word(p+0x94)<quotient{
        if m.word(p+0x18)>=2{
            let node=m.word(p+0x14)+m.word(p+0x1c)*24;m.put_word(node+4,size);
            h.call_m(0x1b5b8c,&[m.word(p+4),node],m);
            let next=m.word(p+0x20);m.put_word(p+0x1c,next);
            let count=m.word(p+0x18);if count==0{return Err(engine::Error::InvalidSnapshot);}
            m.put_word(p+0x20,next.wrapping_add(1)%count);
            m.put_word(p+0x78,m.word(m.word(p+0x90)+next*4));
            m.put_word(p+0x7c,m.word(m.word(p+0x90)+m.word(p+0x20)*4));
            m.put_word(p+0x28,remainder);
            m.put_word(m.word(p+0x14)+next*24,m.word(m.word(p+0x90)+next*4));
            m.put_word(m.word(p+0x14)+m.word(p+0x1c)*24+4,m.word(p+0x28));
            m.put_byte(m.word(p+0x14)+m.word(p+0x1c)*24+16,0);
            h.call_m(0x1b5b8c,&[m.word(p+4),m.word(p+0x14)+m.word(p+0x1c)*24],m);
            m.put_word(p+0x24,m.word(p+0x24).wrapping_add(1));
        }
    }else{
        m.put_word(p+0x28,remainder);m.put_word(m.word(p+0x14)+m.word(p+0x1c)*24+4,remainder);
        h.call_m(0x1b5b8c,&[m.word(p+4),m.word(p+0x14)+m.word(p+0x1c)*24],m);
    }
    m.put_word(p+0x94,quotient);
    if m.byte(crate::rom_region::addr(rom,0xa7bf5c))!=0{h.call_m(0x185d40,&[],m);}else{
        let global=h.call_m(0x18ac58,&[],m);h.call_m(0x18423c,&[global+0x1c4],m);
    }
    h.call_m(0x18423c,&[p+100],m);Ok(())
}

pub trait RuntimeServicesM {
    fn guest_capture(&self)->bool {true}
    fn invoke_m(&mut self,m:&mut Memory<'_>,entry:u32,args:&[u32])->Result<u32,engine::Error>;

    fn emit_pcm_m(&mut self,samples:&[i16])->Result<(),engine::Error>{
        let _=samples;
        Err(engine::Error::UnsupportedStage{entry:0xc72018})
    }
}
pub struct OutputRuntime<'a> {
    pub inner: &'a mut dyn RuntimeServicesM,
    pub guest_capture: bool,
}
impl RuntimeServicesM for OutputRuntime<'_> {
    fn guest_capture(&self)->bool {self.guest_capture}
    fn invoke_m(&mut self,m:&mut Memory<'_>,entry:u32,args:&[u32])->Result<u32,engine::Error> {self.inner.invoke_m(m,entry,args)}
    fn emit_pcm_m(&mut self,samples:&[i16])->Result<(),engine::Error> {self.inner.emit_pcm_m(samples)}
}
pub trait SleepPlatform {
    fn sleep_ns(&mut self,nanoseconds:u64)->Result<(),engine::Error>;
}

pub struct RuntimeM<'a,'w>{
    pub backend:Option<&'a mut BackendWorkspace<'w>>,
    pub pcm:&'a mut [i16],
    pub written:usize,
    pub tick:u64,
    pub registers:engine::NotificationRegisters,
    pub sleeper:&'a mut dyn SleepPlatform,
}
impl RuntimeServicesM for RuntimeM<'_,'_>{
    fn invoke_m(&mut self,m:&mut Memory<'_>,entry:u32,a:&[u32])->Result<u32,engine::Error>{
        match entry{
            0x1a18c4=>{
                let work=self.backend.as_deref_mut().ok_or(engine::Error::UnsupportedStage{entry})?;
                render_backend_m(m,a.try_into().map_err(|_|engine::Error::InvalidSnapshot)?,work)?;Ok(0)
            }
            0xc72018=>{
                if a[1]&1!=0{return Err(engine::Error::InvalidSnapshot);}
                let n=a[1]as usize/2;backend_capacity(self.written+n,self.pcm.len())?;
                for i in 0..n{self.pcm[self.written+i]=m.half(a[0]+i as u32*2)as i16;}
                self.written+=n;Ok(0)
            }
            0x166054=>{self.sleeper.sleep_ns((a[1]as u64)<<32|a[0]as u64)?;Ok(0)}
            0x183258=>{
                struct Clock(u64);impl engine::Platform for Clock{fn ticks(&mut self)->u64{self.0}}
                let mut regions=[engine::Region{base:m.base,bytes:m.bytes}];let mut sparse=engine::Memory::new(&mut regions)?;
                engine::notify_synthesis_live(&mut sparse,a[0],a[1],self.registers,&mut Clock(self.tick))?;Ok(0)
            }
            _=>Err(engine::Error::UnsupportedStage{entry}),
        }
    }
    fn emit_pcm_m(&mut self,samples:&[i16])->Result<(),engine::Error>{
        backend_capacity(self.written+samples.len(),self.pcm.len())?;
        for (i,s) in samples.iter().enumerate(){self.pcm[self.written+i]=*s;}
        self.written+=samples.len();Ok(())
    }
}

pub fn call_live_until_m(
    m:&mut Memory<'_>,entry:u32,args:&[u32],platform:&mut dyn engine::HeapPlatform,
    runtime:&mut dyn RuntimeServicesM,snap:&[u8],until:u32,
)->Result<u32,engine::Error>{
    let mut h=Host{boot:engine::SpeechBootHost{platform,fault:None},boundary:None,runtime:Some(runtime),snap,trace:None,until:Some(until),depth:0,stack:[0;64],stack_len:0,rom:crate::rom_region::Rom::Us};
    let rc=h.call(m,entry,args);
    if let Some(e)=h.boot.fault{return Err(e);}
    if let Some(b)=h.boundary{return Err(engine::Error::UnsupportedStage{entry:b.entry});}
    Ok(rc)
}

pub fn call_live_with_runtime_m(
    m:&mut Memory<'_>,entry:u32,args:&[u32],platform:&mut dyn engine::HeapPlatform,
    runtime:&mut dyn RuntimeServicesM,snap:&[u8],
)->Result<u32,engine::Error>{
    call_live_with_runtime_m_rom(m,entry,args,platform,runtime,snap,crate::rom_region::Rom::Us)
}

pub fn call_live_with_runtime_m_rom(
    m:&mut Memory<'_>,entry:u32,args:&[u32],platform:&mut dyn engine::HeapPlatform,
    runtime:&mut dyn RuntimeServicesM,snap:&[u8],rom:crate::rom_region::Rom,
)->Result<u32,engine::Error>{
    let mut h=Host{boot:engine::SpeechBootHost{platform,fault:None},boundary:None,runtime:Some(runtime),snap,trace:None,until:None,depth:0,stack:[0;64],stack_len:0,rom};
    let rc=h.call(m,entry,args);
    if let Some(e)=h.boot.fault{return Err(e);}
    if let Some(b)=h.boundary{return Err(engine::Error::UnsupportedStage{entry:b.entry});}
    Ok(rc)
}

pub fn call_live_until_m_traced(
    m:&mut Memory<'_>,entry:u32,args:&[u32],platform:&mut dyn engine::HeapPlatform,
    runtime:&mut dyn RuntimeServicesM,snap:&[u8],until:u32,trace:&mut dyn CallTraceSink,
)->Result<u32,engine::Error>{
    let mut h=Host{boot:engine::SpeechBootHost{platform,fault:None},boundary:None,runtime:Some(runtime),snap,trace:Some(trace),until:Some(until),depth:0,stack:[0;64],stack_len:0,rom:crate::rom_region::Rom::Us};
    let rc=h.call(m,entry,args);
    if let Some(e)=h.boot.fault{return Err(e);}
    if let Some(b)=h.boundary{return Err(engine::Error::UnsupportedStage{entry:b.entry});}
    Ok(rc)
}
pub fn call_live_with_runtime_m_traced(
    m:&mut Memory<'_>,entry:u32,args:&[u32],platform:&mut dyn engine::HeapPlatform,
    runtime:&mut dyn RuntimeServicesM,snap:&[u8],trace:&mut dyn CallTraceSink,rom:crate::rom_region::Rom,
)->Result<u32,engine::Error>{
    let mut h=Host{boot:engine::SpeechBootHost{platform,fault:None},boundary:None,runtime:Some(runtime),snap,trace:Some(trace),until:None,depth:0,stack:[0;64],stack_len:0,rom};
    let rc=h.call(m,entry,args);
    if let Some(e)=h.boot.fault{return Err(e);}
    if let Some(b)=h.boundary{return Err(engine::Error::UnsupportedStage{entry:b.entry});}
    Ok(rc)
}

impl crate::speech_part_4::Host for Host<'_>{
 fn heap_table(&mut self,_:&mut Memory<'_>)->u32{crate::rom_region::addr(self.rom,0xb6f4ec)}
 fn heap_optional_flag(&mut self,_:&mut Memory<'_>)->u32{crate::rom_region::addr(self.rom,0xa7b8e8)+1}
 fn heap_thread_id(&mut self)->u32{self.boot.platform.thread_id()}
 fn heap_arbiter_address(&self)->u32{self.boot.platform.arbiter_address()}
 fn code_overwritten(&mut self,end:u32){self.boot.fault.get_or_insert(engine::Error::CodeOverwritten{end});}
 fn heap_exhausted(&mut self,size:u32){self.boot.fault.get_or_insert(engine::Error::HeapExhausted{size});}
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.call(m,f,a)}
}
impl crate::component_live_c::Host for Host<'_>{
 fn call_m(&mut self,entry:u32,args:&[u32],m:&mut Memory<'_>)->u32{self.call(m,entry,args)}
 fn thread_id_m(&mut self)->u32{self.boot.platform.thread_id()}
 fn rom(&self)->crate::rom_region::Rom{self.rom}
}
impl crate::boot_port_3b::Host for Host<'_>{
 fn call(&mut self,m:&mut Memory<'_>,address:u32,args:&[u32])->u32{self.call(m,address,args)}
}
impl crate::component_live_b::Host for Host<'_>{
 fn call_m(&mut self,entry:u32,args:&[u32],m:&mut Memory<'_>)->u32{self.call(m,entry,args)}
}

impl crate::boot::Calls for Host<'_> {
    fn call(&mut self, memory: &mut crate::boot::Memory<'_>, address: u32, args: &[u32]) -> u32 {
        self.call(memory, address, args)
    }
    fn call_wide(&mut self, _: &mut crate::boot::Memory<'_>, f: u32, _: &[u32]) -> u64 {
        if self.boot.fault.is_none() {
            self.boot.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
        }
        0
    }
}
impl crate::speech_part_3::Host for Host<'_> {
    fn call(&mut self, memory: &mut crate::boot::Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.call(memory, method, args)
    }
}

impl crate::frontend::HeapVtable for Host<'_> {
    fn alloc_at(&mut self, _o: u32, _n: u32) -> u32 {
        panic!("the live sentence match reached the arena-blind allocator");
    }
    fn realloc_at(&mut self, _o: u32, _p: u32, _n: u32) -> u32 {
        panic!("the live sentence match reached the arena-blind reallocator");
    }
    fn release_at(&mut self, _o: u32, _p: u32) {
        panic!("the live sentence match reached the arena-blind releaser");
    }
}
impl crate::frontend::MethodTable for Host<'_> {
    fn call_at(&mut self, _o: u32, _a: u32, _b: u32) -> u32 {
        panic!("the live sentence match reached the arena-blind method table");
    }
}
impl crate::render_part_c::Host for Host<'_> {
    fn call(&mut self, memory: &mut crate::boot::Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.call(memory, entry, args)
    }
    fn output(&mut self, _m: &mut crate::boot::Memory<'_>, entry: u32, _args: &[u32]) -> (u32, u32) {
        panic!("the live sentence match reached the output method {entry:#x}");
    }
}

impl crate::engine_live_render::LiveDispatch for Host<'_>{
 fn dispatch(&mut self,m:&mut Memory<'_>,entry:u32,args:&[u32])->u32{self.call(m,entry,args)}
 fn thread_id(&mut self)->u32{self.boot.platform.thread_id()}
 fn rom(&self)->crate::rom_region::Rom{self.rom}

}
impl crate::speech_part_1::Host for Host<'_>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.call(m,f,a)}
 fn get(&mut self,m:&mut Memory<'_>,f:u32,handle:u32,pos:u32,len:u32)->(u32,u32,u32){
  let s=0x1ea0;
  let saved=[m.word(s),m.word(s+4)];
  m.put_word(s,0);m.put_word(s+4,0);
  let rc=self.call(m,f,&[handle,pos,len,s,s+4]);
  let got=(rc,m.word(s),m.word(s+4));
  m.put_word(s,saved[0]);m.put_word(s+4,saved[1]);
  got
 }
}
impl crate::speech_part_6::Host for Host<'_>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.call(m,f,a)}
 fn log(&mut self,m:&mut Memory<'_>,logger:u32,category:u32,level:u32,format:&[u8],values:&[u32]){
  let filtered=logger==0||level>m.byte(logger+48) as u32||(m.byte(logger+49)!=0&&level>4);
  let fmt=if filtered {0} else {source_literal(m,format)};
  self.log_m(logger,category,level,fmt,values,m);
 }
 fn log_guest(&mut self,m:&mut Memory<'_>,logger:u32,category:u32,level:u32,fmt:u32,values:&[u32]){
  self.log_m(logger,category,level,fmt,values,m);
 }
 fn error(&mut self,m:&mut Memory<'_>,logger:u32,category:u32,code:u32){
  self.call(m,0x173dc0,&[logger,category,code,0]);
 }
}
impl Host<'_>{

 fn log_m(&mut self,logger:u32,category:u32,level:u32,fmt:u32,values:&[u32],m:&mut Memory<'_>){
  let s=0x500;
  let mut saved=[0u32;12];
  for (i,v) in saved.iter_mut().enumerate(){*v=m.word(s+i as u32*4);}
  for (i,v) in values.iter().enumerate(){m.put_word(s+i as u32*4,*v);}
  self.call(m,0x1738e8,&[logger,category,level,0,fmt,s]);
  for (i,v) in saved.iter().enumerate(){m.put_word(s+i as u32*4,*v);}
 }
}

struct StreamOpen {

    iface: u32,

    tag: u32,

    size: u32,

    id: u32,

    inputs: u32,

    outputs: u32,

    in_slot: u32,

    out_slot: u32,

    error: u32,
}

fn open_streams_m(m:&mut Memory<'_>,p:u32,tag:u32,ha:u32,hb:u32,plan:StreamOpen,
    platform:&mut dyn engine::HeapPlatform,rom:crate::rom_region::Rom)->Result<u32,engine::Error>{
    { let v = crate::frontend::validate_handle(p!=0,tag,plan.tag,plan.size);     if v>=0x80000000{return Ok(plan.error);} }
    let dependency=0x3000usize;
    let mut saved=[0u8;4352];saved.copy_from_slice(&m.bytes[dependency..dependency+4352]);
    let mut h=crate::component_live_a2::LiveCalls::new(platform,dependency as u32).with_rom(rom);
    let inputs=crate::rom_region::addr(rom,plan.inputs);
    let outputs=crate::rom_region::addr(rom,plan.outputs);
    let mut rc=0u32;
    let mut i=0u32;
    while i<2 {
        let name=m.word(inputs+i*4);
        let out=p+plan.in_slot+i*8;
        let open=m.word(m.word(p+plan.iface)+0x20);
        rc=crate::component_live_a2::Calls::call(&mut h,m,open,&[ha,hb,plan.id,name,0,0,out]);
        if rc>=0x80000000{break;}
        i+=1;
    }
    let mut j=0u32;
    if rc<0x80000000 {
        while j<2 {
            let name=m.word(outputs+j*4);
            let out=p+plan.out_slot+j*8;
            let mode=if j!=1{0}else{
                let f=m.word(m.word(p+plan.iface)+0x4c);
                crate::component_live_a2::Calls::call(&mut h,m,f,&[])
            };
            let open=m.word(m.word(p+plan.iface)+0x20);
            rc=crate::component_live_a2::Calls::call(&mut h,m,open,&[ha,hb,plan.id,name,mode,1,out]);
            if rc>=0x80000000{break;}
            j+=1;
        }
    }
    if rc>=0x80000000 {
        let mut k=0u32;
        while k<i {
            let slot=p+plan.in_slot+k*8;
            let a=m.word(slot);
            if a!=0 {
                let b=m.word(slot+4);
                let f=m.word(m.word(p+plan.iface)+0x24);
                crate::component_live_a2::Calls::call(&mut h,m,f,&[a,b]);
                m.put_word(slot,0);
                m.put_word(slot+4,0);
            }
            k+=1;
        }
        k=0;
        while k<j {
            let slot=p+plan.out_slot+k*8;
            let a=m.word(slot);
            if a!=0 {
                let b=m.word(slot+4);
                let f=m.word(m.word(p+plan.iface)+0x24);
                crate::component_live_a2::Calls::call(&mut h,m,f,&[a,b]);
                m.put_word(slot,0);
                m.put_word(slot+4,0);
            }
            k+=1;
        }
    }
    let fault=h.fault();
    m.bytes[dependency..dependency+4352].copy_from_slice(&saved);
    if let Some(e)=fault{return Err(e);}
    Ok(rc)
}

pub fn fn_202f9c_m(m:&mut Memory<'_>,p:u32,tag:u32,ha:u32,hb:u32,
    platform:&mut dyn engine::HeapPlatform,rom:crate::rom_region::Rom)->Result<u32,engine::Error>{
    open_streams_m(m,p,tag,ha,hb,StreamOpen{iface:0x18,tag:0xf38a,size:0xb8,id:0xa8,
        inputs:0x8f9860,outputs:0x8f986c,in_slot:0x1c,out_slot:0x2c,error:0x8a802008},platform,rom)
}

fn process_start_m(a:&[u32],tag:u32,size:u32,error:u32)->u32 {
    let rc=crate::frontend::validate_handle(a[0]!=0,a[1],tag,size);
    if rc>=0x80000000 {error} else {rc}
}

const DEPES_FRAME: u32 = 0x0e0f_d600;
const DEPES_FRAME_LEN: usize = 0x400;
const PORT_FRAME_1D3314: u32 = 0x0e0f_da00;
const PORT_FRAME_1F0A90: u32 = 0x0e0f_db40;
const PORT_FRAME_21A25C: u32 = 0x0e0f_dc80;
const DEPES_LOG_SCRATCH: u32 = 0x7800;

const SEG2_C4_SCRATCH: u32 = 0x0e0f_8000;
const SEG2_C4_LEN: usize = 0x1000;

const MAP_WORD: u32 = 0x0e0f_d500;
const MAP_ENV: u32 = 0x0e0f_d540;

const RANGE17_SCRATCH: u32 = 0x0e0f_d400;
const BROKER_DATA_SCRATCH: u32 = 0x0e0f_d000;
const BROKER_QUERY_SCRATCH: u32 = 0x0e0f_d020;
const BROKER_FILE_SCRATCH: u32 = 0x0e0f_c000;
const BROKER_LIT_BASE: u32 = 0x0e0f_d100;
const BROKER_LIT_END: u32 = 0x0e0f_d200;
const BROKER_LID_SCRATCH: u32 = 0x0e0f_d200;
const BROKER_UNMAP_SCRATCH: u32 = 0x0e0f_d300;
const BROKER_CLOSE_SCRATCH: u32 = 0x0e0f_d340;

struct BrokerDataHost<'a, 'b> {
    host: &'a mut Host<'b>,
    rom: crate::rom_region::Rom,
    lit_cursor: u32,
}

impl BrokerDataHost<'_, '_> {
    fn translate(&self, entry: u32) -> u32 {
        crate::rom_region::to_us_guest(self.rom, entry)
    }
    fn forward(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        let f = self.translate(f);
        self.host.call(m, f, a)
    }
}

impl crate::boot_part_b::Host for BrokerDataHost<'_, '_> {
    fn log_variadic(&mut self,m:&mut Memory<'_>,entry:u32,a:[u32;4],format:&[u8],varargs:&[u32])->bool {
        crate::log_live::variadic_m(m,entry,a,format,varargs,0x0e0f7c00,|m,f,a|self.forward(m,f,a))
    }

    fn real_log_args(&mut self) -> bool {
        true
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.forward(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
    fn format_address(&mut self, format: &[u8]) -> Option<u32> {
        let at = (self.lit_cursor + 3) & !3;
        let end = at + format.len() as u32 + 1;
        if end > BROKER_LIT_END {
            return None;
        }
        self.lit_cursor = (end + 3) & !3;
        Some(at)
    }
}

impl crate::boot_part_b::OutputHost for BrokerDataHost<'_, '_> {
    fn output(&mut self, _m: &mut Memory<'_>, f: u32, _a: &[u32]) -> (u32, u32) {
        panic!("broker output requires live binding {f:#x}");
    }
}

impl crate::boot_part_b::PropertyHost for BrokerDataHost<'_, '_> {
    fn property(&mut self, _m: &mut Memory<'_>, _p: u32, _k: &[u8]) -> (u32, u32) {
        (0x8060_2001, 0)
    }
    fn insert(&mut self, _m: &mut Memory<'_>, _p: u32, _k: u32, _e: [u32; 3]) -> u32 {
        0x8150_6007
    }
}

impl crate::boot_part_b::QueryHost for BrokerDataHost<'_, '_> {
    fn query_output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        if f != 0x1772b0 {
            return (0, 0);
        }
        let scratch = BROKER_QUERY_SCRATCH;
        let out = scratch + 12;
        let rc = crate::boot_port_2c::fn_1772b0(m, a[0], a[1], out, scratch, self);
        (rc, m.word(out))
    }
    fn insert_record(&mut self, _m: &mut Memory<'_>, _db: u32, _k: u32, _r: [u32; 4]) -> u32 {
        0x8150_6007
    }
    fn open_pair(&mut self, _m: &mut Memory<'_>, _f: u32, _o: [u32; 2], _out: u32) -> u32 {
        0x8150_6007
    }
}

impl crate::boot_part_b::DataHost for BrokerDataHost<'_, '_> {
    fn data_output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32], out: u32) -> u32 {

        let _ = (m, f, a, out);
        0x8060_2001
    }
}

impl crate::component_live_a2::Calls for BrokerDataHost<'_, '_> {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.forward(m, entry, args)
    }
}

impl crate::boot_part_e::Host for BrokerDataHost<'_, '_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.forward(m, f, a)
    }
}

impl crate::boot_port_2c::BrokerQuery for BrokerDataHost<'_, '_> {
    fn db_lock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32 {
        self.forward(m, 0x176f2c, &[lock])
    }
    fn db_unlock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32 {
        self.forward(m, 0x176f5c, &[lock])
    }
    fn db_open(&mut self, m: &mut Memory<'_>, database: u32, name: u32, mode: u32, out: u32) -> u32 {
        crate::component_live_a2::alloc_handle_for_key_m(m, database, name, mode, out, self)
    }
    fn db_next(&mut self, m: &mut Memory<'_>, iterator: u32, key: u32, value: u32) -> u32 {
        crate::engine_components2::fn_196860(m, iterator, key, value)
    }
    fn db_close(&mut self, m: &mut Memory<'_>, iterator: u32) -> u32 {
        crate::component_live_a::release_owner_m(m, iterator, self)
    }
    fn db_compare(&mut self, m: &crate::boot::Memory<'_>, a: u32, b: u32) -> i32 {
        crate::fe_text::strcmp_ci_1769d0(m.bytes, a, b) as i32
    }
}

struct LidLookupHost<'a, 'b> {
    host: &'a mut Host<'b>,
    rom: crate::rom_region::Rom,
}

impl LidLookupHost<'_, '_> {
    fn translate(&self, entry: u32) -> u32 {
        crate::rom_region::to_us_guest(self.rom, entry)
    }
    fn forward(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        let f = self.translate(f);
        self.host.call(m, f, a)
    }
}

impl crate::speech_part_2::Host for LidLookupHost<'_, '_> {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.forward(m, method, args)
    }
}

impl crate::boot_part_b::Host for LidLookupHost<'_, '_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.forward(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}

impl crate::log_live::LogMethodsM for LidLookupHost<'_, '_> {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        self.forward(m, method, &[object, category, level, arg, format, varargs])
    }
}

impl crate::boot_part_b::FormatHost for LidLookupHost<'_, '_> {}

impl crate::reentrant::HooksM for LidLookupHost<'_, '_> {
    fn call_m(&mut self, address: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.forward(m, address, args)
    }
}

struct BrokerCloseDirect<'a, 'b, 'c> {
    host: &'a mut Host<'b>,
    inner: &'c mut crate::component_live_c2::OpenHost,
}

impl BrokerCloseDirect<'_, '_, '_> {
    fn forward_live(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.host.call(m, f, a)
    }
}

impl crate::frontend::HeapVtable for BrokerCloseDirect<'_, '_, '_> {
    fn alloc_at(&mut self, _obj: u32, _size: u32) -> u32 {
        0
    }
    fn realloc_at(&mut self, _obj: u32, _old: u32, _size: u32) -> u32 {
        0
    }
    fn release_at(&mut self, _obj: u32, _object: u32) {}
}

impl crate::frontend::MethodTable for BrokerCloseDirect<'_, '_, '_> {
    fn call_at(&mut self, _obj: u32, _a: u32, _b: u32) -> u32 {
        0
    }
}

impl crate::frontend::StateLock for BrokerCloseDirect<'_, '_, '_> {
    fn acquire(&mut self) -> u32 {
        0
    }
    fn release(&mut self) -> u32 {
        0
    }
}

impl crate::boot_part_b::FormatHost for BrokerCloseDirect<'_, '_, '_> {}

impl crate::boot_part_b::Host for BrokerCloseDirect<'_, '_, '_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.inner.dispatch(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}

impl crate::boot_part_e::Host for BrokerCloseDirect<'_, '_, '_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.forward_live(m, f, a)
    }
}

impl crate::log_live::LogMethodsM for BrokerCloseDirect<'_, '_, '_> {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        self.inner.dispatch(m, method, &[object, category, level, arg, format, varargs])
    }
}

impl crate::leftovers_1::CloseHost for BrokerCloseDirect<'_, '_, '_> {
    fn select_close_lock(&mut self, _lock: u32) {}
    fn compare_tree_keys(&mut self, _a: u32, _b: u32) -> i32 {
        1
    }
    fn indirect(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.inner.dispatch(m, method, args)
    }
}

pub fn fn_14e440_m(m: &mut Memory<'_>, state: u32, required: u32, h: &mut Host) {
    assert_eq!(m.base, 0);
    let buffer = m.word(state + 60);
    if buffer == 0 || m.byte(state + 40) != 0 {
        return;
    }
    let write = m.word(state + 44);
    let readers = m.word(state + 12);
    let mut minimum = write;
    if readers != 0 {
        let base = m.word(readers);
        let table = crate::frontend::ElementTable {
            data: &m.bytes[m.offset(base)..],
            elem_size: m.half(readers + 4),
            count: m.word(readers + 20),
        };
        let mut index = 0u32;
        loop {
            let mut off = 0u32;
            if crate::frontend::element_at(Some(&table), index, Some(&mut off)) != 1 {
                break;
            }
            let stream = m.word(base.wrapping_add(off));
            let rp = m.word(stream.wrapping_add(8));
            if rp < minimum {
                minimum = rp;
            }
            index = index.wrapping_add(1);
        }
    }
    let old = m.word(state + 56);
    let stride = m.half(state + 28) as u32;
    let mut discarded = minimum.wrapping_sub(old);
    if stride > 1 {
        let (_, rem) = crate::frontend::divmod_u32(minimum.wrapping_sub(old), stride);
        discarded = discarded.wrapping_sub(rem);
    }
    let dtor = m.word(state + 36);
    if dtor != 0 && discarded != 0 {
        let heap = m.word(m.word(state + 4) + 4);
        let end = buffer.wrapping_add(discarded);
        let mut element = m.word(state + 52).wrapping_sub(old).wrapping_add(buffer);
        let mut dpos = m.word(state + 52);
        let dtor = crate::rom_region::to_us_guest(crate::rom_region::Rom::Us, dtor);
        while element < end {
            h.call(m, dtor, &[element, heap]);
            element = element.wrapping_add(stride);
            dpos = dpos.wrapping_add(stride);
        }
        m.put_word(state + 52, dpos);
    }
    let config = m.word(state);
    if required == 0 {
        if m.word(config + 24) > discarded {
            return;
        }
    } else if discarded < required && discarded < m.word(config + 20) {
        return;
    }
    let remaining = write.wrapping_sub(minimum);
    if remaining != 0 {
        crate::frontend::move_bytes(
            &mut *m.bytes,
            buffer,
            buffer.wrapping_add(discarded),
            remaining,
        );
    }
    m.put_word(state + 56, old.wrapping_add(discarded));
    if required == 0 {
        let capacity = m.word(state + 64);
        let heap = m.word(m.word(state + 4) + 4);
        if capacity == discarded || write == minimum {
            crate::boot_part_e::release(m, heap, buffer, h);
            m.put_word(state + 60, 0);
            m.put_word(state + 64, 0);
        } else {
            let newbuf = crate::boot_part_e::fn_1963e0(
                m,
                heap,
                buffer,
                capacity.wrapping_sub(discarded),
                h,
            );
            if newbuf != 0 {
                m.put_word(state + 60, newbuf);
                m.put_word(state + 64, capacity.wrapping_sub(discarded));
            }
        }
    }
}

fn close_runtime_us() -> crate::leftovers_1::CloseRuntime {
    fn hash_text(bytes: &[u8]) -> u32 {
        bytes
            .iter()
            .fold(2166136261u32, |h, &b| (h ^ b as u32).wrapping_mul(16777619))
    }
    let mut fold = [0u16; 256];
    let mut i = 0usize;
    while i < 26 {
        fold[b'A' as usize + i] = 1;
        i += 1;
    }
    crate::leftovers_1::CloseRuntime {
        category: 0x8d7668,
        begin: 0x17a14c,
        end: 0x17a18c,
        error: 0x17a180,
        handle_key: hash_text(b"hData"),
        status_key: hash_text(b"lhError"),
        scratch: BROKER_CLOSE_SCRATCH,
        fold,
    }
}

pub fn fn_206bc8_m(a:&[u32])->u32 {process_start_m(a,0xf382,44,0x89b02008)}

pub fn fn_1d8eac_m(a:&[u32])->u32 {process_start_m(a,0xf383,56,0x89c02008)}

pub fn fn_1f9310_m(a:&[u32])->u32 {process_start_m(a,0xf384,0x4c8,0x89b02008)}

pub fn fn_1f1414_m(a:&[u32])->u32 {process_start_m(a,0xf385,32,0x89e02008)}

pub fn fn_1f2bf0_m(a:&[u32])->u32 {process_start_m(a,0xf386,0x30,0x89f02008)}

pub fn fn_1f4724_m(m:&mut Memory<'_>,p:u32,tag:u32,ha:u32,hb:u32,
    platform:&mut dyn engine::HeapPlatform,rom:crate::rom_region::Rom)->Result<u32,engine::Error>{
    open_streams_m(m,p,tag,ha,hb,StreamOpen{iface:0x04,tag:0xf424,size:0x40,id:0xc8,
        inputs:0x8f940c,outputs:0x8f9418,in_slot:0x20,out_slot:0x30,error:0x8c802008},platform,rom)
}
