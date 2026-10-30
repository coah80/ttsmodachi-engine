//! fn_17f614, pipeline open. All addresses and storage belong to the caller.
use crate::{boot::Memory, frontend as fe, boot_part_b as b, boot_part_e as e};
/// The ROM module address the pipeline and broker log under. The regional
/// ROMs place D_8d7994 and D_8d7668 elsewhere; the value is a category that
/// the logger passes on, never guest state the arena comparison can see.
const MODULE:u32=0x8d7994;
/// The deepest pipeline dependency level. The five-key 0x1759a0 loop costs
/// three levels per key, and the live chain adds the broker query below it.
pub const DEPTH_LIMIT:u32=32;
/// The bytes one pipeline dependency level owns. fn_175170 uses its slot up
/// to 576 bytes, so a level's window overlaps the levels below it.
pub const DEPTH_SLOT_BYTES:u32=192;
/// The fixed slots, after the depth area.
const FIXED_OUT:u32=0;
const FIXED_NEXT:u32=8;
const FIXED_OPEN:u32=24;
const FIXED_KEY:u32=32;
const FIXED_RECORD:u32=104;
const FIXED_LOG:u32=128;
const FIXED_PAIR:u32=160;
const FIXED_BYTES:u32=192;
/// The scratch the caller must provide: the depth area first, then the fixed
/// slots. The caller also keeps a copy of the same size and hands it to
/// fn_17f614, which restores the region before it returns.
pub const SCRATCH_BYTES:u32=DEPTH_LIMIT*DEPTH_SLOT_BYTES+FIXED_BYTES;
/// Only indirect platform calls and explicitly owned pending ports cross this boundary.
pub trait Host {
    fn call(&mut self, m:&mut Memory<'_>, f:u32, a:&[u32])->u32;
    fn thread_id(&mut self)->u32 { 1 }
    /// The host owns the logger: the live boot binds fn_1738e8 and fn_173dc0
    /// to its own logging, so the port routes both to the host. The
    /// recorded-reply ports use the port's own fan-out.
    fn owns_logger(&mut self)->bool { false }
    /// The regional builtin component table fn_17dacc searches (D_8dc588 in
    /// the US ROM). The live boot maps the regional table into its arena, so
    /// the broker query finds the builtin entry the ARM found. The recorded
    /// reply ports have no table and leave it zero.
    fn broker_table(&mut self)->u32 { 0 }
}
/// Literal addresses are regional ROM inputs, mapped in the caller's arena.
/// scratch supplies SCRATCH_BYTES disjoint bytes, restored on return. fold is
/// the ROM case-fold table. null_pair/default_context are the fn_19611c
/// globals.
pub struct Environment {
    pub literals:[u32;42],
    pub scratch:u32,
    pub fold:[u16;256],
    pub null_pair:[u8;8],
    pub default_context:u32,
    /// The caller-owned state fn_17f278 needs from boot_port_1.
    pub component:ComponentBinding,
    /// The ROM-region addresses of the broker's format literals, in the order
    /// of crate::boot_part_b::BROKER_FORMATS. The recorded-reply ports pass
    /// the literal's hash instead and leave the table zeroed.
    pub broker_formats:[u32;BROKER_FORMAT_COUNT],
}
/// The number of broker format literals the host can address.
pub const BROKER_FORMAT_COUNT:usize=b::BROKER_FORMATS.len();
/// fn_17f278 opens a component in the registry at ctx+24. The registry is the
/// real boot_port_1 port, so its log categories, formats and names come from
/// the caller rather than from this module.
pub struct ComponentBinding {
    /// 1024 disjoint bytes for the registry open.
    pub scratch:u32,
    /// The kernel arbitration boundary used by the registry map insert.
    pub arbiter:u32,
    /// Broker, voice query, component and synth-stream log categories.
    pub categories:[u32;4],
    /// The log formats the registry uses.
    pub formats:[u32;12],
    /// The component error labels.
    pub error_labels:[u32;2],
    /// The regional names, including "phonmap".
    pub names:[u32;16],
}
pub const TEXTS:[&[u8];42]=[
 b"",b"mlset",b"voiceml",b"noclmset",b"extclccfg",b"fedatapackaging",
 b"COMPONENTS/*",b"COMPONENT",b"pipeline",b"newPipeline",b"Total",b"TTSEG input CB",
 b"TTSEG output CB",b"PARAMETERS/*",b"audiooutput",b"OBJECTS/*",b"RESOURCES/*",
 b"RESOURCE",b"content-type",b"component",b"lherror",b"uri",b"contentType",b"lhError",
 b"tag",b"Pipeline Open",b"Pipeline Close",b"pipeline_ObjOpen : Begin %s",
 b"pipeline_ObjOpen : End (%x)",b"%s%s",b"%s%s%s%s",b"%s%s%s%x",
 b"%s%s%s%s%s%s",b"%s%s%s%s%s%s%s%x",b"ResourceLoad: %s, %s",
 b"Reopen component: %s",b"Close component: %s",b"Open component: %s",
 b"PIPELINE",b"*",b"Reopen object: %s",b"phonmap",
];
fn nz(v:u32)->Option<u32>{if v==0 {None}else{Some(v)}}
fn pad(a:&[u32],n:usize)->[u32;6]{let mut v=[0u32;6];v[..a.len().min(n)].copy_from_slice(&a[..a.len().min(n)]);v}
struct Bound<'a,H>{h:&'a mut H,g:&'a Environment,depth:u32}
impl<'a,H> Bound<'a,H> {
 /// The depth area comes first, so a level never reaches a fixed slot.
 fn level(&self,depth:u32)->u32 {self.g.scratch+depth*DEPTH_SLOT_BYTES}
 /// The fixed slots follow the depth area.
 fn fixed(&self,slot:u32)->u32 {self.g.scratch+DEPTH_LIMIT*DEPTH_SLOT_BYTES+slot}
}
impl<H:Host> Bound<'_,H> {
 fn out(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->(u32,u32) {
    let p=self.fixed(FIXED_OUT);let old=m.word(p);m.put_word(p,0);
    let mut v=[0u32;12];v[..a.len()].copy_from_slice(a);v[a.len()]=p;
    let r=self.invoke(m,f,&v[..a.len()+1]);let got=m.word(p);m.put_word(p,old);(r,got)
 }
 fn invoke(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
    assert_eq!(m.base,0,"absolute frontend arena");
    let s=self.level(self.depth);
    self.depth+=1;assert!(self.depth<DEPTH_LIMIT,"pipeline dependency scratch depth");
    let rc=match f {
      0x19611c=>crate::boot_port_1::fn_19611c(m,[a[0],a[1]],a[2],self.g.null_pair,self.g.default_context),
      0x196394=>e::alloc(m,a[0],a[1],self),
      0x1963b8=>e::calloc(m,a[0],a[1],a[2],self),
      0x196428=>{e::release(m,a[0],a[1],self);0},
      0x195d90=>crate::boot_port_1::fn_195d90(m,a[0]),
      0x195cd0=>{let mut arena=crate::selection::arena::Arena::new(m.base,m.bytes);crate::selection::arena_leaf::copy_string(&mut arena,a[0],a[1])},
      0x195d58=>fe::compare_str(&m.bytes[a[0] as usize..],&m.bytes[a[1] as usize..]) as u32,
      0x195df8=>fe::compare_str_n(&m.bytes[a[0] as usize..],&m.bytes[a[1] as usize..],a[2] as i32) as u32,
      0x1769d0=>crate::fe_text::strcmp_ci_1769d0(m.bytes,a[0],a[1]) as u32,
      0x176fc0=>fe::match_halves(a[0],a[1],a[2],a[3]),
      0x176fd4=>{crate::boot_part_d::fn_176fd4(m,a[0],[u32::from_le_bytes(self.g.null_pair[..4].try_into().unwrap()),u32::from_le_bytes(self.g.null_pair[4..].try_into().unwrap())]);0},
      0x1759a0=>{let value=if a[2]==0 {self.g.literals[0]}else{a[2]};let n=self.invoke(m,0x195d90,&[value])+1;self.invoke(m,0x175170,&[a[0],a[1],value,n,0])},
      0x175170=>crate::speech_part_1::fn_175170(m,a[0],a[1],a[2],a[3],a[4],s,self),
      0x1738e8=>{if self.h.owns_logger(){self.h.call(m,f,a)}else{let v=pad(a,6);crate::log_live::fn_1738e8(m,v[0],v[1],v[2],v[3],v[4],v[5],self)}},
      0x173dc0=>{if self.h.owns_logger(){self.h.call(m,f,a)}else{let v=pad(a,5);crate::render_part_a::error_log(m,self,v[0],v[1],v[2],v[3],v[4],s)}},
      0x173fd8=>crate::component_live_a::max_slot_level_m(m,a[0],self),
      0x176f2c=>crate::boot_port_1::fn_176f2c(m,a[0],self),
      0x176f5c=>crate::boot_port_1::fn_176f5c(m,a[0],self),
      0x1772b0=>{let table=self.g.fold;let mut out=0;let r=crate::boot_live::lookup_keyed_value_m(m.bytes,nz(a[0]),nz(a[1]),&mut out,&table,s,self);m.put_word(a[2],out);r},
      0x196794=>crate::boot_live::alloc_iterator_m(m,a[0],a[1],a[2],a[3],self),
      0x196830=>crate::boot_live::release_iterator_m(m,self,a[0]),
      0x17a3b8=>{let table=self.g.fold;let mut out=m.word(a[4]);let r=crate::boot_live::open_tag_iter_m(m.bytes,nz(a[0]),nz(a[1]),nz(a[2]),a[3],Some(&mut out),s,&table,s+128,self);m.put_word(a[4],out);r},
      0x17a814=>{let mut tag=m.word(a[1]);let mut val=m.word(a[2]);let mut attrs=if a[3]==0 {0}else{m.word(a[3])};let r=fe::next_tag_entry(m.bytes,nz(a[0]),Some(&mut tag),Some(&mut val),if a[3]==0 {None}else{Some(&mut attrs)});m.put_word(a[1],tag);m.put_word(a[2],val);if a[3]!=0 {m.put_word(a[3],attrs);}r},
      0x17a2ec=>crate::boot_live::close_tag_iter_m(m.bytes,nz(a[0]),s,self),
      0x178510=>b::fn_178510(m,a[0],a[1],a[2],a[3],a[4],self),
      0x178530=>fe::release_broker_interface(nz(a[0]),nz(a[1])),
      0x19854c=>b::fn_19854c(m,[a[0],a[1]],a[2],a[3],self),
      0x1985fc=>fe::begin_stage_hook(nz(a[0])),
      0x19860c=>fe::end_stage_hook(nz(a[0])),
      0x19862c=>fe::begin_trace_hook(nz(a[0])),
      0x19863c=>fe::end_trace_hook(nz(a[0]),nz(a[2])),
      0x1985c4=>{let mut free=None;let r=fe::release_trace_entry(m.bytes,nz(a[0]),a[1],|heap,p|free=Some((heap,p)));if let Some((heap,p))=free {e::release(m,heap,p,self);}r},
      0x17ef1c=>b::fn_17ef1c(m,a[0],a[1],a[2],self),
      0x1966ec=>crate::boot_port_1::fn_1966ec(m,a[0],a[1],a[2],s,self),
      0x17ec38=>teardown_pipeline_m(m,a[0],self),
      0x177e0c=>{let table=self.h.broker_table();crate::boot_part_b::fn_177e0c(m,a[0],a[1],a[2],a[3],a[4],a[5],table,s,self)},
      0x17f278=>self.open_component(m,a),
      _=>{if f>=0x10000000 {self.h.call(m,f,a)}else if let Some(r)=m.boot_parameter_call(self,f,a,s) {r}else{self.h.call(m,f,a)}},
    };
    self.depth-=1;rc
 }
 fn literal(&self,i:usize)->u32 {self.g.literals[i]}
 /// The ROM-region address of a broker format literal, when the caller
 /// supplied one. The recorded-reply ports leave the table zeroed and the
 /// literal's hash travels instead.
 fn format_address(&self,format:&[u8])->Option<u32> {
    let i=b::BROKER_FORMATS.iter().position(|f|*f==format)?;
    let address=self.g.broker_formats[i];
    if address==0 {None}else{Some(address)}
 }
 /// fn_17f278. The component is opened in the registry at ctx+24. The
 /// registry map insert runs on the caller's arena, so the port needs no
 /// arena-sized bridge workspace.
 fn open_component(&mut self,m:&mut Memory<'_>,a:&[u32])->u32 {
    open_component_m(m,a,self)
 }
 fn log(&mut self,m:&mut Memory<'_>,ctx:u32,code:u32,text:usize,args:&[u32]) {
    let p=self.fixed(FIXED_LOG);
    for (i,v) in args.iter().enumerate(){m.put_word(p+i as u32*4,*v);}
    let logger=m.word(ctx+16);let category=MODULE;let fmt=self.literal(text);
    if code==0 {self.invoke(m,0x1738e8,&[logger,category,4,0,fmt,p]);}
    else {self.invoke(m,0x173dc0,&[logger,category,code,fmt,p]);}
 }
 fn duplicate(&mut self,m:&mut Memory<'_>,ctx:u32,value:u32)->u32 {
    let n=self.invoke(m,0x195d90,&[value]).wrapping_add(1);
    let p=self.invoke(m,0x196394,&[m.word(ctx+4),n]);
    if p!=0 {self.invoke(m,0x195cd0,&[p,value]);}p
 }
 fn next(&mut self,m:&mut Memory<'_>,it:u32,tag:&mut u32,val:&mut u32,attrs:Option<&mut u32>)->u32 {
    let s=self.fixed(FIXED_NEXT);let old=[m.word(s),m.word(s+4),m.word(s+8)];
    m.put_word(s,*tag);m.put_word(s+4,*val);m.put_word(s+8,attrs.as_ref().map_or(0,|v|**v));
    let r=self.invoke(m,0x17a814,&[it,s,s+4,if attrs.is_some(){s+8}else{0}]);
    *tag=m.word(s);*val=m.word(s+4);if let Some(v)=attrs {*v=m.word(s+8);}
    for(i,v)in old.into_iter().enumerate(){m.put_word(s+i as u32*4,v);}r
 }
 fn open(&mut self,m:&mut Memory<'_>,ctx:u32,name:u32,query:usize,flags:u32,it:&mut u32)->u32 {
    let s=self.fixed(FIXED_OPEN);let old=m.word(s);m.put_word(s,*it);
    let r=self.invoke(m,0x17a3b8,&[m.word(ctx+12),name,self.literal(query),flags,s]);*it=m.word(s);m.put_word(s,old);r
 }
}
impl<H:Host> crate::boot::Calls for Bound<'_,H> {
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.invoke(m,f,a)}
 fn call_wide(&mut self,_:&mut Memory<'_>,_:u32,_:&[u32])->u64 {panic!("unexpected wide pipeline call")}
}
impl<H:Host> e::Host for Bound<'_,H> {fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.invoke(m,f,a)}}
impl<H:Host> b::Host for Bound<'_,H> {fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.invoke(m,f,a)}fn thread_id(&mut self)->u32 {self.h.thread_id()}
 fn format_address(&mut self,format:&[u8])->Option<u32> {Bound::format_address(self,format)}}
impl<H:Host> b::FormatHost for Bound<'_,H> {}
impl<H:Host> b::ContextHost for Bound<'_,H> {fn context(&mut self,m:&mut Memory<'_>,a:[u32;2])->(u32,u32){self.out(m,0x19611c,&a)}}
impl<H:Host> b::OutputHost for Bound<'_,H> {fn output(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->(u32,u32){self.out(m,f,a)}}
impl<H:Host> crate::speech_part_1::Host for Bound<'_,H> {
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.invoke(m,f,a)}
 fn get(&mut self,_:&mut Memory<'_>,_:u32,_:u32,_:u32,_:u32)->(u32,u32,u32){panic!("unexpected stream get")}
}
impl<H:Host> b::PropertyHost for Bound<'_,H> {
 fn property(&mut self,m:&mut Memory<'_>,dictionary:u32,key:&[u8])->(u32,u32){
  let s=self.fixed(FIXED_KEY);
  for (i,b) in key.iter().enumerate(){m.put_byte(s+i as u32,*b);}
  m.put_byte(s+key.len() as u32,0);
  let (rc,v)=self.out(m,0x1966ac,&[dictionary,s]);
  if rc>=0x80000000 {(rc,0)} else {(rc,m.word(v))}
 }
 fn insert(&mut self,m:&mut Memory<'_>,dictionary:u32,key:u32,entry:[u32;3])->u32{
  let s=self.fixed(FIXED_RECORD);
  m.put_word(s,entry[0]);m.put_word(s+4,entry[1]);m.put_word(s+8,entry[2]);
  self.invoke(m,0x19668c,&[dictionary,key,s])
 }
}
impl<H:Host> b::QueryHost for Bound<'_,H> {
 fn query_output(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->(u32,u32){self.out(m,f,a)}
 fn insert_record(&mut self,m:&mut Memory<'_>,db:u32,key:u32,record:[u32;4])->u32{
  let s=self.fixed(FIXED_RECORD);
  for i in 0..4 {m.put_word(s+i as u32*4,record[i]);}
  self.invoke(m,0x19668c,&[db,key,s])
 }
 fn open_pair(&mut self,m:&mut Memory<'_>,f:u32,owner:[u32;2],out:u32)->u32{
  self.invoke(m,f,&[owner[0],owner[1],out])
 }
}
impl<H:Host> crate::boot_part_f::Host for Bound<'_,H> {
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.invoke(m,f,a)}
 fn compare(&mut self,_:&mut Memory<'_>,_:u32,_:&mut crate::boot_part_f::Key,_:&mut u32)->u32 {panic!("unexpected local comparator")}
}
impl<H:Host> crate::log_live::LogMethodsM for Bound<'_,H> {
 fn log_m(&mut self,f:u32,o:u32,c:u32,l:u32,a:u32,fmt:u32,v:u32,m:&mut Memory<'_>)->u32 {self.invoke(m,f,&[o,c,l,a,fmt,v])}
}
/// fn_17f614. a = handle words, name, source, prefix count, flags, out, changed.
/// Scratch must not alias arguments or any reachable object. The caller keeps
/// a copy of the scratch region in `save` and hands it over here; the port
/// restores the region from that copy before it returns, so the arena never
/// carries pipeline scratch. `save` needs SCRATCH_BYTES bytes.
pub fn fn_17f614(m:&mut Memory<'_>,a:[u32;8],g:&Environment,save:&mut [u8],h:&mut impl Host)->u32 {
 let n=SCRATCH_BYTES as usize;assert!(save.len()>=n,"pipeline scratch save buffer");
 let at=m.offset(g.scratch);
 save[..n].copy_from_slice(&m.bytes[at..at+n]);
 let rc=pipeline_open(m,a,&mut Bound{h,g,depth:0});
 m.bytes[at..at+n].copy_from_slice(&save[..n]);rc
}
fn pipeline_open<H:Host>(m:&mut Memory<'_>,a:[u32;8],b:&mut Bound<'_,H>)->u32 {
 let [ha,hb,name,src,n,f6,out,flag]=a;
 let (mut rc,ctx)=b.out(m,0x19611c,&[ha,hb]);
 if rc>=0x80000000 {return rc;}
 m.put_word(out,0);m.put_byte(flag,0);
 b.log(m,ctx,0,27,&[name]);
 for key in 1..=5 {b.invoke(m,0x1759a0,&[m.word(ctx+20),b.literal(key),b.literal(0)]);}
 let mut np=0;let mut it=0;let mut arr=0;let mut tag=0;let mut val=0;
 rc='run: {
  np=b.invoke(m,0x1963b8,&[m.word(ctx+4),1,28]);
  if np==0 {break 'run alloc_error(m,ctx,b);}
  m.put_word(np,ctx);let s=b.duplicate(m,ctx,name);m.put_word(np+4,s);
  if s==0 {break 'run alloc_error(m,ctx,b);}
  for mode in 1..=2 {
   rc=b.open(m,ctx,name,6,0,&mut it);
   if rc>=0x80000000 {b.log(m,ctx,0x3ec8,29,&[b.literal(8),name]);break 'run rc;}
   let mut j=0u16;
   loop {
    rc=b.next(m,it,&mut tag,&mut val,None);
    if rc>=0x80000000 {break;}
    if mode==1 {
     if tag==0 || b.invoke(m,0x195d58,&[tag,b.literal(7)])!=0 {
      b.log(m,ctx,0x3edb,30,&[b.literal(8),name,b.literal(24),if tag==0 {b.literal(0)}else{tag}]);break 'run 0x81202004;
     }
     if val==0 || m.byte(val)==0 {b.log(m,ctx,0x3edc,29,&[b.literal(8),name]);break 'run 0x81202004;}
     if src!=0 && n<0xffff && n>j as u32 {
      let comp=m.word(m.word(src+8)+j as u32*4);
      if b.invoke(m,0x1769d0,&[val,m.word(comp+4)])!=0 {
       b.log(m,ctx,0x3ef8,30,&[b.literal(8),m.word(src+4),b.literal(9),name]);break 'run 0x81202001;
      }
     }
    } else {
     let mut comp=0;
     if src!=0 && val!=0 && m.byte(val)!=0 {
      let mut i=0u16;
      while i<m.half(src+12) {
       let c=m.word(m.word(src+8)+i as u32*4);
       let same=b.invoke(m,0x195d58,&[m.word(c+4),val])==0;i=i.wrapping_add(1);
       if same && c!=0 {comp=c;break;}
      }
     }
     if comp!=0 {
      m.put_word(m.word(np+8)+j as u32*4,comp);m.put_half(comp,m.half(comp).wrapping_add(1));
      if m.word(np+16)!=0 {let s=b.duplicate(m,ctx,val);m.put_word(m.word(np+24)+8+j as u32*4,s);}
     } else {
      if m.word(np+16)!=0 {let s=b.duplicate(m,ctx,val);m.put_word(m.word(np+24)+8+j as u32*4,s);b.invoke(m,0x1985fc,&[m.word(np+16),m.word(np+20),j as u32+2]);}
      comp=b.invoke(m,0x1963b8,&[m.word(ctx+4),1,20]);
      if comp==0 {break 'run alloc_error(m,ctx,b);}
      m.put_word(m.word(np+8)+j as u32*4,comp);m.put_half(comp,1);
      let s=b.duplicate(m,ctx,val);m.put_word(comp+4,s);b.invoke(m,0x176fd4,&[comp+12]);
      if s==0 {break 'run alloc_error(m,ctx,b);}
      rc=b.invoke(m,0x178510,&[m.word(ctx+12),val,1,arr+j as u32*8,comp+8]);
      if rc>=0x80000000 {break 'run rc;}
      if m.word(np+16)!=0 {b.invoke(m,0x19860c,&[m.word(np+16),m.word(np+20),j as u32+2]);}
     }
    }
    j=j.wrapping_add(1);
   }
   b.invoke(m,0x17a2ec,&[it]);it=0;
   if mode==1 {
    let comps=b.invoke(m,0x1963b8,&[m.word(ctx+4),j as u32,4]);m.put_word(np+8,comps);
    if comps==0 {break 'run alloc_error(m,ctx,b);}
    arr=b.invoke(m,0x1963b8,&[m.word(ctx+4),j as u32,8]);
    if arr==0 {break 'run alloc_error(m,ctx,b);}
    m.put_half(np+12,j);
    if b.invoke(m,0x173fd8,&[m.word(ctx+16)])>=2 {
     b.invoke(m,0x19854c,&[ha,hb,j as u32+3,np+16]);b.invoke(m,0x1985fc,&[m.word(np+16),m.word(np+20),0]);
     if m.word(np+16)!=0 {
      let extra=b.invoke(m,0x1963b8,&[m.word(ctx+4),j as u32+3,4]);m.put_word(np+24,extra);
      if extra==0 {b.invoke(m,0x1985c4,&[m.word(np+16),m.word(np+20)]);b.invoke(m,0x176fd4,&[np+16]);}
      else {for (index,text) in [(0,10),(1,11),(j as u32+2,12)] {let s=b.duplicate(m,ctx,b.literal(text));m.put_word(extra+index*4,s);}}
     }
    }
   }
  }
  b.invoke(m,0x1759a0,&[m.word(ctx+20),b.literal(8),name]);
  rc=b.open(m,ctx,name,13,1,&mut it);
  if rc<0x80000000 {
   loop {
    rc=b.next(m,it,&mut tag,&mut val,None);if rc>=0x80000000 {break;}
    if tag==0 || m.byte(tag)==0 {continue;}
    if n!=0xffff && b.invoke(m,0x195df8,&[b.literal(14),tag,11])==0 {continue;}
    b.invoke(m,0x1759a0,&[m.word(ctx+20),tag,if val==0 {b.literal(0)}else{val}]);
   }
   b.invoke(m,0x17a2ec,&[it]);it=0;
  }
  rc=b.open(m,ctx,name,15,1,&mut it);
  if rc<0x80000000 {
   loop {
    rc=b.next(m,it,&mut tag,&mut val,None);
    if rc>=0x80000000 {b.invoke(m,0x17a2ec,&[it]);it=0;break;}
    if tag==0 || val==0 || m.byte(val)==0 {
     b.log(m,ctx,0x3ee3,30,&[b.literal(8),name,b.literal(24),if tag==0 {b.literal(0)}else{tag}]);break 'run 0x81202004;
    }
    rc=b.invoke(m,0x17f278,&[ha,hb,tag,val,f6,flag]);if rc>=0x80000000 {break 'run rc;}
   }
  }
  let mut i=0u16;
  while i<m.half(np+12) {
   let c=m.word(m.word(np+8)+i as u32*4);
   if c!=0 {
    if m.word(np+16)!=0 {b.invoke(m,0x1985fc,&[m.word(np+16),m.word(np+20),i as u32+2]);}
    let null=[u32::from_le_bytes(b.g.null_pair[..4].try_into().unwrap()),u32::from_le_bytes(b.g.null_pair[4..].try_into().unwrap())];
    if b.invoke(m,0x176fc0,&[m.word(c+12),m.word(c+16),null[0],null[1]])!=0 {
     b.log(m,ctx,0,37,&[m.word(c+4)]);
     rc=b.invoke(m,m.word(m.word(c+8)+16),&[m.word(arr+i as u32*8),m.word(arr+i as u32*8+4),ha,hb,c+12]);
    } else if m.word(m.word(c+8)+24)!=0 {
     b.log(m,ctx,0,35,&[m.word(c+4)]);m.put_byte(flag,1);rc=b.invoke(m,m.word(m.word(c+8)+24),&[m.word(c+12),m.word(c+16)]);
    }
    if m.word(np+16)!=0 {b.invoke(m,0x19860c,&[m.word(np+16),m.word(np+20),i as u32+2]);}
    if rc>=0x80000000 {b.log(m,ctx,0x3e81,31,&[b.literal(19),m.word(c+4),b.literal(20),rc]);break 'run rc;}
   }
   i=i.wrapping_add(1);
  }
  rc=b.open(m,ctx,name,16,1,&mut it);
  if rc>=0x80000000 {if rc&0x1fff!=13 && rc&0x1fff!=20 {break 'run rc;}rc=0;}
  let mut ct=0;let mut attrs=0;
  loop {
   let status=b.next(m,it,&mut tag,&mut val,Some(&mut attrs));if status>=0x80000000 {break;}
   if tag==0 || b.invoke(m,0x195d58,&[tag,b.literal(17)])!=0 {b.log(m,ctx,0x3edd,30,&[b.literal(8),name,b.literal(24),if tag==0 {b.literal(0)}else{tag}]);continue;}
   if val==0 || m.byte(val)==0 {b.log(m,ctx,0x3ede,29,&[b.literal(8),name]);continue;}
   let (attr_status,found)=if attrs==0 {(0x89002007,ct)}else{b.out(m,0x1966ec,&[attrs,b.literal(18)])};ct=found;
   if attrs==0 || attr_status>=0x80000000 || ct==0 || m.byte(ct)==0 {b.log(m,ctx,0x3edf,29,&[b.literal(8),name]);rc=0x81202004;continue;}
   b.log(m,ctx,0,34,&[val,ct]);
   let (_,c)=b.out(m,0x17ef1c,&[np,ct]);
   if c==0 {b.log(m,ctx,0x3ecd,30,&[b.literal(21),val,b.literal(22),ct]);continue;}
   let load=m.word(m.word(c+8)+32);
   if load==0 {b.log(m,ctx,0x3ea7,32,&[b.literal(19),m.word(c+4),b.literal(21),val,b.literal(22),ct]);continue;}
   let p=b.fixed(FIXED_PAIR);let old=[m.word(p),m.word(p+4)];b.invoke(m,0x176fd4,&[p]);
   let null=[u32::from_le_bytes(b.g.null_pair[..4].try_into().unwrap()),u32::from_le_bytes(b.g.null_pair[4..].try_into().unwrap())];
   let status=b.invoke(m,load,&[m.word(c+12),m.word(c+16),ct,val,0,0,null[0],null[1],p]);
   m.put_word(p,old[0]);m.put_word(p+4,old[1]);
   if status>=0x80000000 && status&0x1fff!=0x40c {b.log(m,ctx,0x3ea6,33,&[b.literal(19),m.word(c+4),b.literal(21),val,b.literal(22),ct,b.literal(23),status]);}
  }
  b.invoke(m,0x17a2ec,&[it]);it=0;
  if m.word(np+16)!=0 && m.word(np+24)!=0 {
   b.invoke(m,0x19860c,&[m.word(np+16),m.word(np+20),0]);
   b.invoke(m,0x19863c,&[m.word(np+16),m.word(np+20),m.word(ctx+16),b.literal(25),m.word(np+24),0,0]);
  }
  rc
 };
 if it!=0 {b.invoke(m,0x17a2ec,&[it]);}
 if arr!=0 {b.invoke(m,0x196428,&[m.word(ctx+4),arr]);}
 if rc<0x80000000 {m.put_word(out,np);}else if np!=0 {b.invoke(m,0x17ec38,&[np]);}
 b.log(m,ctx,0,28,&[rc]);rc
}
fn alloc_error<H:Host>(m:&mut Memory<'_>,ctx:u32,b:&mut Bound<'_,H>)->u32 {
 // A null format is meaningful to fn_173dc0, not the empty string.
 b.invoke(m,0x173dc0,&[m.word(ctx+16),b.literal(38),16000,0,b.fixed(FIXED_LOG)]);0x8120200a
}
/// Arena-passing conversion of frontend::teardown_pipeline. The host is Bound,
/// so every named callee is rebound to its real port before platform dispatch.
fn teardown_pipeline_m<H:Host>(m:&mut Memory<'_>,cs:u32,b:&mut Bound<'_,H>)->u32 {
 if cs==0 {return 0x81202006;}
 let ctx=m.word(cs);let mut result=0;
 if m.word(cs+8)!=0 {
  if m.word(cs+16)!=0 {b.invoke(m,0x19862c,&[m.word(cs+16),m.word(cs+20)]);b.invoke(m,0x1985fc,&[m.word(cs+16),m.word(cs+20),0]);}
  let mut i=m.half(cs+12);
  while i!=0 {
   i-=1;let c=m.word(m.word(cs+8)+i as u32*4);if c==0 {continue;}
   if m.half(c)>1 {m.put_half(c,m.half(c)-1);continue;}
   let null=[u32::from_le_bytes(b.g.null_pair[..4].try_into().unwrap()),u32::from_le_bytes(b.g.null_pair[4..].try_into().unwrap())];
   if b.invoke(m,0x176fc0,&[m.word(c+12),m.word(c+16),null[0],null[1]])==0 {
    b.log(m,ctx,0,36,&[m.word(c+4)]);
    if m.word(cs+16)!=0 {b.invoke(m,0x1985fc,&[m.word(cs+16),m.word(cs+20),i as u32+2]);}
    let r=b.invoke(m,m.word(m.word(c+8)+20),&[m.word(c+12),m.word(c+16)]);
    if m.word(cs+16)!=0 {b.invoke(m,0x19860c,&[m.word(cs+16),m.word(cs+20),i as u32+2]);}
    if r>=0x80000000 {b.log(m,ctx,0x3e82,31,&[b.literal(19),m.word(c+4),b.literal(20),r]);if result<0x80000000 {result=r;}}
   }
   if m.word(c+8)!=0 {let r=b.invoke(m,0x178530,&[m.word(ctx+12),m.word(c+8)]);if result<0x80000000 && r>=0x80000000 {result=r;}}
   b.invoke(m,0x196428,&[m.word(ctx+4),m.word(c+4)]);b.invoke(m,0x196428,&[m.word(ctx+4),c]);
  }
  if m.word(cs+16)!=0 {b.invoke(m,0x19860c,&[m.word(cs+16),m.word(cs+20),0]);b.invoke(m,0x19863c,&[m.word(cs+16),m.word(cs+20),m.word(ctx+16),b.literal(26),m.word(cs+24),0,0]);}
  b.invoke(m,0x196428,&[m.word(ctx+4),m.word(cs+8)]);
 }
 if m.word(cs+16)!=0 {b.invoke(m,0x1985c4,&[m.word(cs+16),m.word(cs+20)]);}
 if m.word(cs+24)!=0 {let mut i=0u16;while (i as u32)<m.half(cs+12) as u32+3 {let p=m.word(m.word(cs+24)+i as u32*4);if p!=0 {b.invoke(m,0x196428,&[m.word(ctx+4),p]);}i=i.wrapping_add(1);}b.invoke(m,0x196428,&[m.word(ctx+4),m.word(cs+24)]);}
 if m.word(cs+4)!=0 {b.invoke(m,0x196428,&[m.word(ctx+4),m.word(cs+4)]);}
 b.invoke(m,0x196428,&[m.word(ctx+4),cs]);result
}
/// The registry open runs the real boot_port_1 port, so this bound surface
/// has to satisfy that port's host trait. Only the registry path is live
/// here; the discovery, directory and close surfaces are not reached by
/// fn_17f614 and refuse instead of inventing a result.
impl<H:Host> crate::boot_part_b::DirectoryHost for Bound<'_,H> {
 fn join(&mut self,_:&mut Memory<'_>,_:u32,_:u32,_:u32,_:u32)->(u32,u32){panic!("unexpected directory join")}
 fn first(&mut self,_:&mut Memory<'_>,_:u32,_:u32,_:u32,_:u32)->(u32,u32,u32){panic!("unexpected directory first")}
 fn next(&mut self,_:&mut Memory<'_>,_:u32)->(u32,u32){panic!("unexpected directory next")}
}
impl<H:Host> crate::boot_part_b::ResourceHost for Bound<'_,H> {
 fn decode(&mut self,_:&mut Memory<'_>,_:[u32;4])->(u32,u32,u32){panic!("unexpected XML decode")}
}
impl<H:Host> crate::boot_part_b2::AttributeHost for Bound<'_,H> {
 fn create(&mut self,_:&mut Memory<'_>,_:u32,_:u32,_:[u32;5],_:u32)->u32{panic!("unexpected attribute create")}
 fn attribute(&mut self,_:&mut Memory<'_>,_:u32,_:&[u8],_:[u32;3])->u32{panic!("unexpected attribute")}
}
impl<H:Host> crate::boot_part_b::DiscoveryHost for Bound<'_,H> {
 fn remove_property(&mut self,_:&mut Memory<'_>,_:u32,_:&[u8])->u32{panic!("unexpected property remove")}
}
impl<H:Host> crate::boot_part_b::DataHost for Bound<'_,H> {
 fn data_output(&mut self,_:&mut Memory<'_>,_:u32,_:&[u32],_:u32)->u32{panic!("unexpected data output")}
}
impl<H:Host> crate::leftovers_1::CloseHost for Bound<'_,H> {
 fn select_close_lock(&mut self,_:u32){panic!("unexpected close lock")}
 fn compare_tree_keys(&mut self,_:u32,_:u32)->i32{panic!("unexpected detached comparison")}
 fn indirect(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.invoke(m,f,a)}
}
impl<H:Host> crate::frontend::HeapVtable for Bound<'_,H> {
 fn alloc_at(&mut self,_:u32,_:u32)->u32{panic!("unexpected detached allocation")}
 fn realloc_at(&mut self,_:u32,_:u32,_:u32)->u32{panic!("unexpected detached reallocation")}
 fn release_at(&mut self,_:u32,_:u32){panic!("unexpected detached release")}
}
impl<H:Host> crate::frontend::MethodTable for Bound<'_,H> {
 fn call_at(&mut self,_:u32,_:u32,_:u32)->u32{panic!("unexpected detached calloc")}
}
impl<H:Host> crate::frontend::StateLock for Bound<'_,H> {
 fn acquire(&mut self)->u32{panic!("unexpected detached lock")}
 fn release(&mut self)->u32{panic!("unexpected detached unlock")}
}
impl<H:Host> crate::boot_port_1::ClockHost for Bound<'_,H> {
 fn clock(&mut self,_:&mut Memory<'_>,_:u32,_:u32,_:&mut u32)->u32{panic!("unexpected clock")}
}

/// The registry's own method table. The registry calls back into the
/// guest's comparison and destruction methods, which the bound dispatcher
/// routes to the same ports the rest of the pipeline uses.
struct RegistryCalls<'a,'b,H>(&'a mut Bound<'b,H>);
impl<'a,'b,H:Host> crate::render_part_c::ParserMethodsM for RegistryCalls<'a,'b,H> {
 fn method_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32 {self.0.invoke(m,f,a)}
}
/// The owner list's database hooks. The registry never logs a pool.
struct RegistryDb<'a,'b,H>(&'a mut Bound<'b,H>);
impl<'a,'b,H:Host> crate::db_live::PoolHooksM for RegistryDb<'a,'b,H> {
 fn log_pool_m(&mut self,_:u32,_:u32,_:&mut Memory<'_>){unreachable!("the registry owner list does not log pools")}
}
impl<'a,'b,H:Host> crate::db_live::DbHooksM for RegistryDb<'a,'b,H> {
 fn call_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32 {self.0.invoke(m,f,a)}
}
/// The registry's log categories and formats. The caller owns both.
fn registry_info<'a,'b,H:Host>(m:&mut Memory<'_>,logger:u32,fmt:usize,args:&[u32],b:&'a mut Bound<'b,H>) {
 let e=&b.g.component;
 for (i,v) in args.iter().enumerate(){m.put_word(e.scratch+960+i as u32*4,*v);}
 let category=e.categories[match fmt {0..=2=>0,3..=5|11=>1,_=>2}];
 crate::log_live::fn_1738e8(m,logger,category,4,0,e.formats[fmt],e.scratch+960,b);
}
fn registry_error<'a,'b,H:Host>(m:&mut Memory<'_>,logger:u32,code:u32,fmt:usize,b:&'a mut Bound<'b,H>) {
 let e=&b.g.component;
 let category=e.categories[match fmt {1=>0,7|10=>2,_=>3}];
 let format=if fmt==6||fmt==10 {0}else{e.formats[fmt]};
 crate::render_part_a::error_log(m,b,logger,category,code,format,e.scratch+960,e.scratch+980);
}
fn registry_component_error<'a,'b,H:Host>(m:&mut Memory<'_>,ctx:u32,component:u32,rc:u32,b:&'a mut Bound<'b,H>) {
 let e=&b.g.component;
 for (i,v) in [e.error_labels[0],component,e.error_labels[1],rc].into_iter().enumerate(){m.put_word(e.scratch+960+i as u32*4,v);}
 registry_error(m,m.word(ctx+16),16001,7,b);
}
/// The registry lookup. The reference count of the found entry grows, as in
/// the ARM.
fn registry_lookup<'a,'b,H:Host>(m:&mut Memory<'_>,o:u32,name:u32,out:u32,b:&'a mut Bound<'b,H>)->u32 {
 crate::render_part_c::fn_17b058_m(m,o,name,out,&mut RegistryCalls(b))
}
/// The registry release of a looked-up entry.
fn registry_release<'a,'b,H:Host>(m:&mut Memory<'_>,o:u32,name:u32,s:u32,b:&'a mut Bound<'b,H>)->u32 {
 crate::render_part_c::registry_release_m(m,o,name,s,&mut RegistryCalls(b))
}
/// The registry's first-hit search, without the reference count.
fn registry_find<'a,'b,H:Host>(m:&mut Memory<'_>,map:u32,name:u32,b:&'a mut Bound<'b,H>)->(u32,u32) {
 if map==0||name==0 {return (0x89002007,0);}
 let node=crate::render_part_c::registry_find_m(m,map,name,&mut RegistryCalls(b));
 match node {Some(n)=>(0,m.word(n+20)),None=>(0x89002014,0)}
}
/// fn_19668c on the caller's arena. The bridge's staged copy is not needed:
/// the map, its tree, the node and the heap live in the arena the caller
/// owns, so the registry insert needs no arena-sized workspace.
fn registry_map_insert<'a,'b,H:Host>(m:&mut Memory<'_>,a:[u32;3],s:u32,b:&'a mut Bound<'b,H>)->u32 {
 if a.iter().any(|x|*x==0) {return 0x89002007;}
 let saved=m.word(s);m.put_word(s,0);
 let tree=a[0]+24;
 let heap=m.word(tree+32);
 let n=e::alloc(m,heap,24,b);
 if n==0 {m.put_word(s,saved);return 0x8900200a;}
 let mut rc=b.invoke(m,m.word(m.word(tree+36)+12),&[a[2],heap,s]);
 let tmp=m.word(s);
 if rc<0x80000000 {
  for off in [0,4,8,16] {m.put_word(n+off,0);}
  m.put_byte(n+12,0);
  m.put_word(n+20,tmp);
  rc=registry_tree_insert(m,tree,n,a[1],b);
  if rc<0x80000000 {m.put_word(s,saved);return 0;}
  if m.word(n+16)!=0 {b.invoke(m,m.word(m.word(tree+36)+4),&[m.word(n+16),heap]);}
  if m.word(n+20)!=0 {b.invoke(m,m.word(m.word(tree+36)+16),&[tmp,heap]);}
 }
 e::release(m,heap,n,b);
 m.put_word(s,saved);
 rc
}
/// The red-black insert of a fresh node. The comparison and the key copy
/// call the registry's own methods.
fn registry_tree_insert<'a,'b,H:Host>(m:&mut Memory<'_>,tree:u32,node:u32,key:u32,b:&'a mut Bound<'b,H>)->u32 {
 let mut parent=tree;
 let mut x=m.word(tree);
 let mut cmp=-1i32;
 while x!=tree {
  parent=x;
  cmp=b.invoke(m,m.word(m.word(tree+36)+8),&[key,m.word(x+16)]) as i32;
  if m.byte(tree+40)==0 && cmp==0 {return 0x89002002;}
  x=m.word(x+if cmp<0 {0} else {4});
 }
 let rc=b.invoke(m,m.word(m.word(tree+36)),&[key,m.word(tree+32),node+16]);
 if rc>=0x80000000 {return rc;}
 m.put_word(parent+if cmp<0 {0} else {4},node);
 m.put_word(node,tree);m.put_word(node+4,tree);m.put_word(node+8,parent);
 m.put_word(tree+24,m.word(tree+24).wrapping_add(1));
 m.put_byte(node+12,0);
 crate::frontend::rebalance(m.bytes,tree,node,parent);
 rc
}
/// The registry insert of one component entry.
fn registry_insert<'a,'b,H:Host>(m:&mut Memory<'_>,o:u32,name:u32,pair:u32,s:u32,b:&'a mut Bound<'b,H>)->u32 {
 if o==0||name==0||pair==0 {
  return crate::frontend::insert_ling_entry(m.bytes,None,None,None,&crate::frontend::InsertScripts{lock:0,find:0,found:None,insert:0,unlock:0});
 }
 let lock=crate::boot_port_1::fn_176f2c(m,m.word(o+12),b);
 if lock>=0x80000000 {return lock;}
 let (find,entry)=registry_find(m,m.word(o+16),name,b);
 let insert=if find>=0x80000000 {
  crate::frontend::fill_bytes(m.bytes,s,0,32);
  crate::normalization::memcpy_at(m.bytes,s,pair,20);
  registry_map_insert(m,[m.word(o+16),name,s],s+32,b)
 }else{0};
 let mut rc=crate::frontend::insert_ling_entry(m.bytes,Some(o),Some(name),Some(pair),
  &crate::frontend::InsertScripts{lock:0,find,found:if find<0x80000000{Some(entry)}else{None},insert,unlock:0});
 let status=crate::boot_port_1::fn_176f5c(m,m.word(o+12),b);
 if status>=0x80000000&&rc<0x80000000 {rc=status;}
 rc
}
/// fn_17f278. The component open in the registry at ctx+24, on the caller's
/// arena. The component scratch is restored on return.
fn open_component_m<'a,'b,H:Host>(m:&mut Memory<'_>,a:&[u32],b:&'a mut Bound<'b,H>)->u32 {
 let g=b.g;let e=&g.component;let s=e.scratch;
 let at=m.offset(s);let mut saved=[0u8;1024];saved.copy_from_slice(&m.bytes[at..at+1024]);
 let rc=open_component_inner(m,a,b);
 m.bytes[at..at+1024].copy_from_slice(&saved);rc
}
fn open_component_inner<'a,'b,H:Host>(m:&mut Memory<'_>,a:&[u32],b:&'a mut Bound<'b,H>)->u32 {
 let g=b.g;let e=&g.component;let s=e.scratch;
 let handle=[a[0],a[1]];let name=a[2];let component=a[3];let owner=a[4];let flag=a[5];
 m.put_word(s+400,0);m.put_word(s+404,0);
 let rc=crate::boot_port_1::fn_19611c(m,handle,s+400,g.null_pair,g.default_context);
 if rc>=0x80000000 {return rc;}
 let ctx=m.word(s+400);
 if registry_lookup(m,m.word(ctx+24),name,s+404,b)<0x80000000 {
  let entry=m.word(s+404);let object=m.word(entry+4);let mut rc=rc;
  if m.word(object+24)!=0 {
   registry_info(m,m.word(ctx+16),8,&[name],b);
   if flag!=0 {let at=m.offset(flag);m.bytes[at]=1;}
   rc=b.invoke(m,m.word(object+24),&[m.word(entry+8),m.word(entry+12)]);
   if rc>=0x80000000 {registry_component_error(m,ctx,component,rc,b);}
  }
  registry_release(m,m.word(ctx+24),name,s+480,b);
  return rc;
 }
 crate::frontend::fill_bytes(m.bytes,s+408,0,20);
 let kind=if crate::frontend::compare_folded(&g.fold,crate::frontend::terminated(m.bytes,component),crate::frontend::terminated(m.bytes,e.names[10]))==0 {2}else{1};
 m.bytes[(s+408) as usize]=kind;
 for i in 0..2 {m.put_word(s+432+i*4,u32::from_le_bytes(g.null_pair[i as usize*4..i as usize*4+4].try_into().unwrap()));}
 let rc=crate::boot_part_b::fn_178510(m,m.word(ctx+12),component,kind as u32,s+432,s+412,b);
 if rc>=0x80000000 {return rc;}
 registry_info(m,m.word(ctx+16),9,&[component],b);
 let interface=m.word(s+412);
 let mut rc=b.invoke(m,m.word(interface+16),&[m.word(s+432),m.word(s+436),handle[0],handle[1],s+416]);
 if rc>=0x80000000 {
  registry_component_error(m,ctx,component,rc,b);
  crate::frontend::release_broker_interface(if m.word(ctx+12)==0{None}else{Some(m.word(ctx+12))},Some(interface));
  return rc;
 }
 rc=registry_insert(m,m.word(ctx+24),name,s+408,s+448,b);
 if rc>=0x80000000 {registry_component_error(m,ctx,component,rc,b);}
 else if owner!=0 {
  m.put_word(s+440,name);
  if crate::db_live::append_element_m(&mut RegistryDb(&mut *b),m,owner,s+440,s+444)!=0 {return rc;}
  registry_error(m,m.word(ctx+16),16000,10,b);rc=0x8120200a;
 }else{return rc;}
 b.invoke(m,m.word(interface+20),&[m.word(s+416),m.word(s+420)]);
 crate::frontend::release_broker_interface(if m.word(ctx+12)==0{None}else{Some(m.word(ctx+12))},Some(interface));rc
}
