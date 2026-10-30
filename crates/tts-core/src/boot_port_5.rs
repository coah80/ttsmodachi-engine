//! Boot-path ports for 11cc0c, lane 5.
use crate::boot::Memory;
/// Platform and explicitly unported foreign-lane calls. Every call receives the arena.
pub trait Host {
    fn call(&mut self, m: &mut Memory<'_>, address: u32, args: &[u32]) -> u32;
    /// Resolve a ROM literal for the active region. US is the default.
    fn literal(&mut self, _m:&mut Memory<'_>, us:u32)->u32 {us}
}
/// Initialize both Bet2 buffers through the landed lane-6 initializers.
pub fn fn_2a4180(m: &mut Memory<'_>, h: &mut impl Host, a: u32, b: u32) {
    let at=m.offset(b);
    crate::fe_text::memset_195bec(m.bytes, at as u32, 0, 0x48c);
    let name=h.literal(m,0x9c7f1c);
    crate::boot_port_6::fn_2da9c0(m,a,b+0x1e0,name);
    crate::boot_port_6::fn_2d9b88(m,a,b+0x440);
}

// The context cache and default token are ROM globals supplied by the embedder.
pub trait ContextHost: Host {
    fn context_pair(&mut self, m: &mut Memory<'_>) -> [u8;8];
    fn default_context(&mut self, m: &mut Memory<'_>) -> u32;
}
fn resolve(m: &mut Memory<'_>, h: &mut impl ContextHost, pair: [u32;2], out: &mut u32) -> u32 {
    let cache=h.context_pair(m);
    let default=h.default_context(m);
    let p=if pair[0]==0 {None}else{Some(u16::from_le_bytes(m.bytes[m.offset(pair[0])..m.offset(pair[0])+2].try_into().unwrap()))};
    crate::frontend::resolve_engine_context(pair[0],p,pair[1],&cache,default,out)
}
struct HeapAdapter<'a,H>{h:&'a mut H,base:u32,vt:u32}
impl<H:Host> crate::frontend::HeapVtableM for HeapAdapter<'_,H>{
 fn alloc_at_m(&mut self,obj:u32,n:u32,bytes:&mut [u8])->u32 {
  let mut m=Memory{base:self.base,bytes};let f=m.word(self.vt);
  self.h.call(&mut m,f,&[obj,n])
 }
 fn realloc_at_m(&mut self,obj:u32,old:u32,n:u32,bytes:&mut [u8])->u32 {
  let mut m=Memory{base:self.base,bytes};let f=m.word(self.vt+8);
  self.h.call(&mut m,f,&[obj,old,n])
 }
 fn release_at_m(&mut self,obj:u32,p:u32,bytes:&mut [u8]){
  let mut m=Memory{base:self.base,bytes};let f=m.word(self.vt+12);
  self.h.call(&mut m,f,&[obj,p]);
 }
}
fn alloc(m:&mut Memory<'_>,h:&mut impl Host,heap:u32,old:Option<u32>,n:u32)->u32 {
 let ptr=if heap==0{None}else{Some(heap-m.base)};
 let vt=if heap==0{0}else{m.word(heap)};
 let mut a=HeapAdapter{h,base:m.base,vt};
 match old {None=>crate::frontend::heap_alloc_m(m.bytes,ptr,n,&mut a),Some(p)=>crate::frontend::heap_realloc_m(m.bytes,ptr,p,n,&mut a)}
}
fn release(m:&mut Memory<'_>,h:&mut impl Host,heap:u32,p:u32) {
 let ptr=if heap==0{None}else{Some(heap-m.base)};
 let vt=if heap==0{0}else{m.word(heap)};
 let mut a=HeapAdapter{h,base:m.base,vt};
 crate::frontend::heap_release_m(m.bytes,ptr,if p==0{None}else{Some(p)},&mut a);
}
/// Append a tagged entry. Heap callbacks may mutate the same arena.
pub fn fn_252e1c(m:&mut Memory<'_>,h:&mut impl ContextHost,e:u32,key:u32,a2:u32,a3:u32)->u32 {
 if e==0||a2==0||a3==0{return 0x81102007;}
 let mut env=0;
 if resolve(m,h,[m.word(e),m.word(e+4)],&mut env)>=0x80000000{return 0x81102007;}
 let n=u16::from_le_bytes(m.bytes[m.offset(e+8)..m.offset(e+8)+2].try_into().unwrap());
 let mut i=0u16;
 while i<n {
  let at=m.word(e+12)+i as u32*20;
  let v=u16::from_le_bytes(m.bytes[m.offset(at+4)..m.offset(at+4)+2].try_into().unwrap());
  if v as u32==key{return 0x81102018;} i=i.wrapping_add(1);
 }
 let heap=m.word(env+4);let old=m.word(e+12);
 let arr=alloc(m,h,heap,if old==0{None}else{Some(old)},if old==0{20}else{(n as u32+1)*20});
 if arr==0{return 0x8110200a;}
 m.put_word(e+12,arr);
 let count=u16::from_le_bytes(m.bytes[m.offset(e+8)..m.offset(e+8)+2].try_into().unwrap()).wrapping_add(1);
 m.put_half(e+8,count);
 let slot=m.word(e+12)+(count as u32-1)*20;
 let at=m.offset(slot);crate::fe_text::memset_195bec(m.bytes,at as u32,0,20);
 let slot=m.word(e+12)+(u16::from_le_bytes(m.bytes[m.offset(e+8)..m.offset(e+8)+2].try_into().unwrap()) as u32-1)*20;
 m.put_half(slot+4,key as u16);m.put_half(slot+6,a2 as u16);m.put_half(slot,a3 as u16);0
}
/// Read an optional dictionary parameter through its live vtable.
pub fn fn_265b3c(m:&mut Memory<'_>,h:&mut impl ContextHost,p:u32,out:u32)->u32{
 if p==0{return 0x81102007;}
 let mut ctx=0;let rc=resolve(m,h,[m.word(p),m.word(p+4)],&mut ctx);
 if rc>=0x80000000{return 0x81102007;}
 if m.byte(p+0x77)==0{m.put_word(out,0);return rc;}
 let f=m.word(m.word(p+0x58)+28);
 h.call(m,f,&[m.word(p+0x5c),m.word(p+0x60),out])
}
struct LogAdapter<'a,H>(&'a mut H);
impl<H:Host> crate::log_live::LogMethodsM for LogAdapter<'_,H>{
 fn log_m(&mut self,f:u32,o:u32,c:u32,l:u32,a:u32,fmt:u32,va:u32,m:&mut Memory<'_>)->u32{
  self.0.call(m,f,&[o,c,l,a,fmt,va])
 }
}
/// The nine format addresses are caller-supplied ROM literals in check order.
pub fn fn_2a41e0(m:&mut Memory<'_>,h:&mut impl Host,c:u32,input:u32,out:u32,formats:[u32;9],category:u32)->u32{
 let mut bad=None;
 for (i,off) in [0xe8,0xc8,0xa8,0x48,8,0x28,0x148].into_iter().enumerate(){
  if m.word(input+off)==0{bad=Some(i);break;}
 }
 if bad.is_none(){
  if m.word(input+0x128)!=0&&m.word(input+0x88)!=0 {let at=m.offset(out+0x45);m.bytes[at]=1;}
  if m.word(input+0x108)!=0&&m.word(input+0x68)!=0 {let at=m.offset(out+0x46);m.bytes[at]=1;}
  if m.byte(out+0x43)!=0&&m.byte(out+0x44)!=0{bad=Some(7);}
  else if m.word(out+0x28) as i32>0&&u16::from_le_bytes(m.bytes[m.offset(out)..m.offset(out)+2].try_into().unwrap())!=0{bad=Some(8);}
 }
 if let Some(i)=bad{
  crate::log_live::fn_1738e8(m,m.word(c+16),category,0,0,formats[i],0,&mut LogAdapter(h));
  0x84e02004
 }else{0}
}
/// Query the embedded dictionary interfaces, forwarding unknown ids.
pub fn fn_2b3f04(m:&mut Memory<'_>,h:&mut impl Host,p:u32,id:u32,out:u32)->u32{
 let off=match id {
  0x18b53e23=>Some(36),0x18b54089=>Some(32),0x13e95e8d=>Some(48),
  0x13e96e46 if m.byte(p+72)==0=>Some(40),
  0x26f785cc if m.byte(p+72)==0=>Some(28),
  0x1f267872 if m.byte(p+72)==0=>Some(44),
  0x482d5ba8=>Some(52),_=>None
 };
 if let Some(off)=off{m.put_word(out,p+off);0}else{fn_2e4bf8(m,p,id,out)}
}
/// Reset the rate-dependent prediction state using the existing arithmetic ports.
pub fn fn_2a9f24(m:&mut Memory<'_>,h:&mut impl Host,p:u32)->u32{
 for off in [0x3c,0x120,0x12c,0x140,0x10c,0xe4,0x44,0x6c,0xac,0x9c,0x8c,0x7c]{m.put_word(p+off,0);}
 let mut active=m.word(p+0x34);crate::morphology::activate_dictionary(&mut active);m.put_word(p+0x34,active);
 let mut history=crate::selection::PredictionHistory{sample:m.word(p+0x54) as i32,first_length:m.word(p+0x18c) as i32,second_length:m.word(p+0x19c) as i32};
 history.reset();m.put_word(p+0x18c,history.first_length as u32);m.put_word(p+0x19c,history.second_length as u32);m.put_word(p+0x54,history.sample as u32);
 // Landed lane-6 reset, operating on the same live arena.
 crate::boot_port_6::fn_2dcf84(m,p);
 m.put_word(p+0x10,(crate::morphology::duration_frequency(m.word(p+4),0xaf00)>>6) as u32);
 m.put_word(p+0x14,(crate::morphology::duration_frequency(m.word(p+4),0xa00)>>6) as u32);
 m.put_half(p+0x30,(crate::morphology::duration_samples(m.word(p+4),4)>>6) as u16);
 m.put_word(p+0x154,0);m.put_word(p+0x18,0x4000);m.put_word(p+0x24,0x4000);
 let v=crate::selection::rounded_product(m.word(p+0x18) as i32,m.word(p+0x28).wrapping_shl(2) as i32,16);
 m.put_word(p+0x1c,v as u32);m.put_word(p+0x2c,0x4000);m.put_word(p+0x28,0x4000);
 let v=m.word(p+0x18).wrapping_mul(100)>>14;
 if v<=200{
  m.put_word(p+0x18,if v==100{0x4000}else{v.wrapping_mul(164)});
  let v=crate::selection::rounded_product(m.word(p+0x18) as i32,m.word(p+0x28).wrapping_shl(2) as i32,16);m.put_word(p+0x1c,v as u32);
 }
 m.put_half(p+0x20,0);0
}
/// Clone a compiled replacement rule. Character tables are borrowed from the caller.
pub fn fn_2aa91c(m:&mut Memory<'_>,h:&mut impl Host,source:u32,heap:u32,out:u32,tables:&crate::regex::PcreTables<'_>)->u32{
 if source==0||m.word(source)==0||m.word(source+4)==0||out==0{return 0;}
 let at=m.offset(m.word(source));
 let size=match crate::regex::pcre_info(&m.bytes[at..],None,1,tables){Ok(crate::regex::InfoValue::Word(n))=>n,_=>return 0};
 let p=alloc(m,h,heap,None,size);m.put_word(out,p);
 if m.word(out)==0{return 0;}
 let d=m.offset(m.word(out)) as u32;let s=m.offset(m.word(source)) as u32;
 crate::fe_text::memcpy_195ae8(m.bytes,d,s,size);
 let s=m.offset(m.word(source+4)) as u32;
 let n=crate::fe_text::strlen_195d90(m.bytes,s) as u32+1;
 let p=alloc(m,h,heap,None,n);m.put_word(out+4,p);
 if m.word(out+4)!=0{
  let d=m.offset(m.word(out+4)) as u32;let s=m.offset(m.word(source+4)) as u32;
  crate::frontend::copy_str(m.bytes,d,s);return 1;
 }
 release(m,h,heap,m.word(out));m.put_word(out,0);0
}

/// Base dictionary interface query, defined in src/fn_2e4ac0.c.
pub fn fn_2e4bf8(m:&mut Memory<'_>,p:u32,id:u32,out:u32)->u32 {
 m.put_word(out,if id==0x0f47e6ec {p.wrapping_add(12)} else {0});0
}

struct CallAdapter<'a,H>(&'a mut H);
impl<H:Host> crate::reentrant2::HooksM for CallAdapter<'_,H> {
 fn call_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32 {self.0.call(m,f,a)}
}
fn calloc(m:&mut Memory<'_>,h:&mut impl Host,heap:u32,n:u32,size:u32)->u32 {
 crate::reentrant2::call_method_m(m,heap,n,size,&mut CallAdapter(h))
}
/// Load the serialized sequence string. `scratch` is a disjoint caller-owned
/// word standing in for the ARM stack local and is restored before returning.
/// The actual ROM entry is 263b70; its C definition is named fn_263b80.
pub fn fn_263b70(m:&mut Memory<'_>,h:&mut impl ContextHost,o:u32,a2:u32,a3:u32,data:u32,scratch:u32)->u32 {
 if m.word(o+0x70)!=0{return 0;}
 let mut ctx=0;
 if resolve(m,h,[m.word(o),m.word(o+4)],&mut ctx)>=0x80000000{return 0x81102007;}
 let flag=m.byte(data);let at=m.offset(o+0x77);m.bytes[at]=flag;
 let size=m.word(data+2).wrapping_add(1);
 let p=calloc(m,h,m.word(ctx+4),1,size);m.put_word(o+0x70,p);
 if p==0{return 0x8110200a;}
 let offset=m.word(data+6).wrapping_add(10);let saved=m.word(scratch);m.put_word(scratch,size);
 let dest=m.word(o+0x70);
 let rc=crate::render_part_a::read_chunk_string(&mut crate::selection::arena::Arena::new(m.base,m.bytes),a2,data,a3,offset,dest,scratch);
 m.put_word(scratch,saved);
 if rc>=0x80000000||m.word(o+0x70)==0{0x81102007}else{rc}
}

struct FormatAdapter<'a,H>(&'a mut H);
impl<H:Host> crate::boot_part_b::Host for FormatAdapter<'_,H> {
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call(m,f,a)}
 fn thread_id(&mut self)->u32 {panic!("thread service not bound")}
}
impl<H:Host> crate::boot_part_b::FormatHost for FormatAdapter<'_,H> {}
/// Open an empty function-data block. `formats` contains begin/end ROM
/// literals; `scratch` is disjoint caller-owned error-formatter stack storage.
pub fn fn_2b645c(m:&mut Memory<'_>,h:&mut impl Host,o:u32,out:u32,formats:[u32;2],scratch:u32)->u32 {
 let ctx=m.word(o);let category=m.word(o+0x48);
 if out==0 {
  crate::render_part_a::error_log(m,&mut FormatAdapter(h),m.word(ctx+16),category,0x5df1,0,0,scratch);
  return crate::normalization::forward_error(7);
 }
 crate::log_live::fn_1738e8(m,m.word(ctx+16),category,4,0,formats[0],0,&mut LogAdapter(h));
 m.put_word(out,0);
 let p=calloc(m,h,m.word(ctx+4),1,8);
 if p==0 {
  crate::render_part_a::error_log(m,&mut FormatAdapter(h),m.word(ctx+16),category,0x5df0,0,0,scratch);
  return crate::normalization::forward_error(10);
 }
 let at=m.offset(p) as u32;crate::fe_text::memset_195bec(m.bytes,at,0,8);
 m.put_half(p,0);m.put_word(p+4,0);m.put_word(out,p);
 // The zero result is a local ARM va_arg represented by caller stack storage.
 let saved=m.word(scratch);m.put_word(scratch,0);
 crate::log_live::fn_1738e8(m,m.word(ctx+16),category,4,0,formats[1],scratch,&mut LogAdapter(h));
 m.put_word(scratch,saved);0
}

/// ROM strings and disjoint stack storage used by the existing private-heap ports.
#[derive(Clone,Copy)]
pub struct PoolLiterals {pub category:u32,pub format:u32,pub after_load:u32,pub before_free:u32,pub scratch:u32}
struct PoolAdapter<'a,'b,H>{m:Memory<'a>,h:&'b mut H,l:PoolLiterals,name:u32}
impl<H:Host> crate::selection::PoolHost for PoolAdapter<'_,'_,H>{
 fn block(&self,i:usize)->crate::selection::PoolBlock{
  let p=i as u32;let next=self.m.word(p);
  crate::selection::PoolBlock{next:if next==0{None}else{Some(next as usize)},base:self.m.word(p+4),current:self.m.word(p+8),end:self.m.word(p+12)}
 }
 fn set_block(&mut self,i:usize,b:crate::selection::PoolBlock){
  let p=i as u32;for (i,v) in [b.next.unwrap_or(0) as u32,b.base,b.current,b.end].into_iter().enumerate(){self.m.put_word(p+i as u32*4,v);}
 }
 fn allocate(&mut self,owner:u32,n:u32)->Option<(usize,u32)>{
  let heap=self.m.word(owner+4);let p=calloc(&mut self.m,self.h,heap,1,n);
  if p==0{None}else{Some((p as usize,p))}
 }
 fn free(&mut self,owner:u32,i:usize){let heap=self.m.word(owner+4);release(&mut self.m,self.h,heap,i as u32);}
 fn clear(&mut self,p:u32,n:u32){let at=self.m.offset(p);crate::fe_text::memset_195bec(self.m.bytes,at as u32,0,n);}
 fn report(&mut self,p:&crate::selection::PrivateHeap<'_>,label:&[u8],blocks:u32){
  let scratch=self.l.scratch;let at=self.m.offset(scratch);let mut saved=[0u8;24];saved.copy_from_slice(&self.m.bytes[at..at+24]);
  let label=if label==b"before FreePool"{self.l.before_free}else{self.l.after_load};
  for (i,v) in [self.name,label,p.allocated,p.pool,p.expected,blocks].into_iter().enumerate(){self.m.put_word(scratch+i as u32*4,v);}
  let logger=self.m.word(p.owner+16);
  crate::log_live::fn_1738e8(&mut self.m,logger,self.l.category,3,0,self.l.format,scratch,&mut LogAdapter(self.h));
  self.m.bytes[at..at+24].copy_from_slice(&saved);
 }
}
fn pool(m:&mut Memory<'_>,h:&mut impl Host,p:u32,op:u32,size:u32,l:PoolLiterals)->u32{
 if p==0{return 0;}
 let first=m.word(p+8);
 let mut heap=crate::selection::PrivateHeap{owner:m.word(p),name:b"",first_block:if first==0{None}else{Some(first as usize)},expected:m.word(p+12),pool:m.word(p+16),allocated:m.word(p+20)};
 let name=m.word(p+4);let mut host=PoolAdapter{m:Memory{base:m.base,bytes:m.bytes},h,l,name};
 let result=match op {0=>heap.allocate(size,&mut host).map_or(0,|a|a.address),1=>{heap.report(b"after load",&mut host);0},2=>{heap.free(&mut host);0},_=>unreachable!()};
 // report does not write private-heap fields, even if a logger edited them.
 if op!=1 {for (i,v) in [heap.first_block.unwrap_or(0) as u32,heap.expected,heap.pool,heap.allocated].into_iter().enumerate(){host.m.put_word(p+8+i as u32*4,v);}}
 result
}
struct Lane6<'a,H>(&'a mut H);
impl<H:Host> crate::boot_port_6::Host for Lane6<'_,H>{
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.0.call(m,f,a)}
}
/// Bind through the real lane-6 tree loader. 2da04c remains unported.
pub fn fn_2a4620(m:&mut Memory<'_>,h:&mut impl Host,a:u32,b:u32,c:u32,l:PoolLiterals,k:&crate::boot_port_6::Literals,names:u32)->u32{
 let total=[328,360,392,424,456].into_iter().fold(2000u32,|sum,off|sum.wrapping_add(m.word(b+off)));
 m.put_word(b+492,total);
 let mut status=0;
 for (key,off,flag) in [(2,224,None),(1,192,None),(0,160,None),(4,288,Some(69)),(3,256,Some(70))]{
  if flag.map_or(true,|off|m.byte(c+off)!=0) {status=crate::boot_port_6::fn_2d9c0c(m,&mut Lane6(h),b+1088,key,b+off,k,names);if status!=0{break;}}
 }
 if status==0{status=h.call(m,0x2da04c,&[a,b+480,c,b,b+320]);}
 pool(m,h,b+480,1,0,l);
 if status>=0x80000000{pool(m,h,b+480,2,0,l);}
 status
}

/// Allocate the dictionary wrapper. `methods` are the four allocator and four
/// stream entry addresses, resolved by the embedder for its ROM region.
pub fn fn_281c14(m:&mut Memory<'_>,h:&mut impl ContextHost,pair:[u32;2],out:u32,methods:[u32;8],category:u32,scratch:u32)->u32 {
 let mut ctx=0;let rc=resolve(m,h,pair,&mut ctx);
 if rc>=0x80000000{return rc;}
 let p=calloc(m,h,m.word(ctx+4),1,64);m.put_word(out,p);
 if m.word(out)==0{
  crate::render_part_a::error_log(m,&mut FormatAdapter(h),m.word(ctx+16),category,0x1130,0,0,scratch);
  return 0x8500200a;
 }
 let p=m.word(out);m.put_word(p,pair[0]);m.put_word(m.word(out)+4,pair[1]);
 for (i,value) in methods[..4].iter().enumerate(){m.put_word(m.word(out)+8+i as u32*4,*value);}
 for i in [0,1,3,2]{m.put_word(m.word(out)+44+i as u32*4,methods[4+i]);}
 let p=m.word(out);let at=m.offset(p+60);crate::fe_text::memset_195bec(m.bytes,at as u32,0,1);
 let p=m.word(out);let extra=m.word(ctx+4);
 let rc=crate::selection::arena_leaf::copy_quad(&mut crate::selection::arena::Arena::new(m.base,m.bytes),p+24,p+8,extra);
 if rc!=0{
  crate::render_part_a::error_log(m,&mut FormatAdapter(h),m.word(ctx+16),category,0x117d,0,0,scratch);
  rc.wrapping_add(0x1ea0)|0x85002000
 }else{rc}
}

fn half(m:&Memory<'_>,p:u32)->u16 {
 u16::from_le_bytes(m.bytes[m.offset(p)..m.offset(p)+2].try_into().unwrap())
}
fn chunk(m:&mut Memory<'_>,reader:u32,base:u32,size:u32,offset:u32,dest:u32,n:&mut u32,scratch:u32)->u32 {
 let saved=m.word(scratch);m.put_word(scratch,*n);
 let rc=crate::render_part_a::read_chunk_string(&mut crate::selection::arena::Arena::new(m.base,m.bytes),reader,base,size,offset,dest,scratch);
 *n=m.word(scratch);m.put_word(scratch,saved);rc
}
fn node_alloc_error(m:&mut Memory<'_>,h:&mut impl Host,o:u32,scratch:u32)->u32 {
 let ctx=m.word(o);let cat=m.word(o+0x48);
 crate::render_part_a::error_log(m,&mut FormatAdapter(h),m.word(ctx+16),cat,0x5df0,0,0,scratch);
 crate::normalization::forward_error(10)
}
/// Read a function-data node recursively. Tags are mapped ROM globals and
/// scratch is a disjoint word plus the error logger's 20-byte parameter area.
pub fn fn_2b6674(m:&mut Memory<'_>,h:&mut impl Host,o:u32,rd:u32,ctx:u32,node:u32,base:u32,cursor:u32,size:u32,tags:[u32;12],scratch:u32)->u32 {
 let parsed=half(m,base.wrapping_add(m.word(cursor)));m.put_word(cursor,m.word(cursor).wrapping_add(2));
 let mut rc=0;
 if parsed<12{m.put_word(node,tags[parsed as usize]);}
 if parsed==3{
  let cnt=half(m,base.wrapping_add(m.word(cursor))).wrapping_add(1);m.put_word(cursor,m.word(cursor).wrapping_add(2));
  let p=calloc(m,h,m.word(ctx+4),1,cnt as u32);m.put_word(node+24,p);
  if m.word(node+24)==0{return node_alloc_error(m,h,o,scratch);}
  let mut n=cnt as u32;
  rc=chunk(m,rd,base,size,m.word(cursor),m.word(node+24),&mut n,scratch);
  if rc>=0x80000000||n!=cnt as u32{return crate::normalization::forward_error(0);}
  m.put_word(cursor,m.word(cursor).wrapping_add(cnt as u32));
 }
 let mut v=half(m,base.wrapping_add(m.word(cursor)));m.put_word(cursor,m.word(cursor).wrapping_add(2));
 if v==0{m.put_word(node+20,0);}else{
  v=v.wrapping_add(1);
  let p=calloc(m,h,m.word(ctx+4),1,v as u32);m.put_word(node+20,p);
  if m.word(node+20)==0{return node_alloc_error(m,h,o,scratch);}
  let mut n=v as u32;rc=chunk(m,rd,base,size,m.word(cursor),m.word(node+20),&mut n,scratch);
  if rc>=0x80000000||n!=v as u32{return crate::normalization::forward_error(0);}
  m.put_word(cursor,m.word(cursor).wrapping_add(v as u32));
 }
 let value=half(m,base.wrapping_add(m.word(cursor)));m.put_half(node+4,value);m.put_word(cursor,m.word(cursor).wrapping_add(2));
 let value=half(m,base.wrapping_add(m.word(cursor)));let at=m.offset(node+12);m.bytes[at]=value as u8;m.put_word(cursor,m.word(cursor).wrapping_add(2));
 if m.byte(node+12)>0{
  let count=m.byte(node+12);let p=calloc(m,h,m.word(ctx+4),1,count as u32*4);m.put_word(node+8,p);
  if m.word(node+8)==0{return node_alloc_error(m,h,o,scratch);}
  let mut i=0u8;
  while i<count{
   let child=calloc(m,h,m.word(ctx+4),1,28);
   if child==0{return node_alloc_error(m,h,o,scratch);}
   m.put_word(m.word(node+8)+i as u32*4,child);
   m.put_word(child,tags[0]);let at=m.offset(child+12);m.bytes[at]=0;
   for off in [8,16,20,24]{m.put_word(child+off,0);}
   rc=fn_2b6674(m,h,o,rd,ctx,child,base,cursor,size,tags,scratch);
   if rc>=0x80000000{return rc;}
   i=i.wrapping_add(1);
  }
 }
 rc
}

/// The configuration vtable returns local ARM stack outputs by mutable value.
/// Keys and the configured section remain addresses in the caller's arena.
pub trait ConfigHost {
 fn string(&mut self,m:&mut Memory<'_>,method:u32,pair:[u32;2],section:u32,key:u32,value:&mut u32,len:&mut u16,sep:&mut u8)->u32;
}
/// Read morphology limits and flags. `keys` point to the eight underscore-
/// suffixed key prefixes. `scratch` is a disjoint 256-byte caller stack buffer,
/// restored on every return, and `table` is the ROM character classification.
pub fn fn_275204(m:&mut Memory<'_>,h:&mut impl ConfigHost,p:u32,mode:u32,keys:[u32;8],section:u32,scratch:u32,table:&[u16;256])->u32 {
 let at=m.offset(scratch);let mut saved=[0u8;256];saved.copy_from_slice(&m.bytes[at..at+256]);
 let result=(||{
  for (i,v) in [0,0,0,16,0,0,0,1].into_iter().enumerate(){let at=m.offset(p+0x6a+i as u32);m.bytes[at]=v;}
  let mut value=0;let mut sep=0;let mut rc=0;
  for (i,prefix) in keys.into_iter().enumerate(){
   let dst=m.offset(scratch) as u32;let src=m.offset(prefix) as u32;
   crate::frontend::copy_str(m.bytes,dst,src);
   let n=crate::fe_text::strlen_195d90(m.bytes,dst);
   let src=m.offset(mode) as u32;crate::frontend::append_str(m.bytes,dst,src);
   let mut len=0xffff;
   let method=m.word(m.word(p+0x18)+0x1c);let pair=[m.word(p+0x10),m.word(p+0x14)];
   rc=h.string(m,method,pair,section,scratch,&mut value,&mut len,&mut sep);
   if rc>=0x80000000{return rc;}
   if len==0 {
    let at=m.offset(scratch)+n as usize-1;m.bytes[at]=0;len=0xffff;
    let method=m.word(m.word(p+0x18)+0x1c);let pair=[m.word(p+0x10),m.word(p+0x14)];
    rc=h.string(m,method,pair,section,scratch,&mut value,&mut len,&mut sep);
    if rc>=0x80000000{return rc;}
   }
   if len!=0 {
    let s=m.word(value);let src=m.offset(s) as u32;
    let end=crate::fe_text::strchr_195d30(m.bytes,src,sep as u32);
    if end!=0||m.bytes[src as usize]==sep{m.bytes[end as usize]=0;}
    let field=match i {0=>0x6a,1=>0x6b,2=>0x6d,3=>0x6c,_=>0x6a+i as u32};
    let value=if i<3 {crate::frontend::parse_int(table,m.bytes,src) as u8}else{
     let expected:&[u8]=match i {3=>b"upper\0",4=>b"stemafx\0",_=>b"yes\0"};
     (crate::frontend::compare_str(&m.bytes[src as usize..],expected)==0) as u8
    };
    // mdtype's false branch deliberately leaves the existing field untouched.
    if i!=4||value!=0{let at=m.offset(p+field);m.bytes[at]=value;}
   }
  }
  rc
 })();
 m.bytes[at..at+256].copy_from_slice(&saved);result
}

/// Read the requested number of elements using the real cache-window and
/// private-heap ports. SpeechRound4Host binds its map service to fn_17a1a4.
pub fn fn_2a9274<H:Host+crate::speech_part_3::SpeechRound4Host>(m:&mut Memory<'_>,h:&mut H,arena:u32,out:u32,size:u32,mut count:u32,st:u32,l:PoolLiterals)->u32 {
 let pos=m.word(st+12);let limit=m.word(st+8);
 if size.wrapping_mul(count).wrapping_add(pos)>limit{count=crate::normalization::udiv(limit.wrapping_sub(pos),size);}
 if count!=0 {
  if m.byte(st)==2 {
   let mut total=size.wrapping_mul(count);
   let cache=m.word(st+28);
   if pos==0&&total<=limit&&(m.word(cache+12)==0||m.word(cache+8)!=m.word(st+4)){total=limit;}
   let off=m.word(st+4).wrapping_add(pos);
   let rc=crate::speech_part_3::sp4_cache(m,cache,off,total,st+20,h);
   if rc>=0x80000000{return 0;}
   m.put_word(st+16,m.word(st+4).wrapping_add(pos));m.put_word(st+24,total);
  }
  let pos=m.word(st+12);
  if m.byte(st)!=0||pos.wrapping_add(3)&!3!=pos {
   let total=size.wrapping_mul(count);let buf=pool(m,h,arena,0,total,l);m.put_word(out,buf);
   if buf==0{return 0;}
   let src=m.word(st+20).wrapping_add(m.word(st+4)).wrapping_add(m.word(st+12)).wrapping_sub(m.word(st+16));
   let d=m.offset(buf) as u32;let s=m.offset(src) as u32;crate::fe_text::memcpy_195ae8(m.bytes,d,s,total);
  }else{
   m.put_word(out,m.word(st+20).wrapping_add(m.word(st+4)).wrapping_add(pos).wrapping_sub(m.word(st+16)));
  }
  m.put_word(st+12,m.word(st+12).wrapping_add(size.wrapping_mul(count)));
 }
 count
}
fn window_pair(m:&Memory<'_>,p:u32,i:i32)->u32 {m.word(m.word(p+28).wrapping_add((i as u32).wrapping_mul(4)))}
/// Build the weighted-window bounds and pointer arrays. `name` is D_91721c.
pub fn fn_2a2f04<H:Host+crate::speech_part_3::SpeechRound4Host>(m:&mut Memory<'_>,h:&mut H,ctx:u32,p:u32,nw:i32,dim:u32,n:u32,wins:u32,name:u32,l:PoolLiterals)->u32 {
 let init=crate::selection::PrivateHeap::initialize(ctx,b"");
 for (i,v) in [init.owner,name,0,init.expected,init.pool,init.allocated].into_iter().enumerate(){m.put_word(p+i as u32*4,v);}
 let c=nw.wrapping_add(1);m.put_word(p+24,c as u32);m.put_word(p+12,(c as u32).wrapping_mul(28));
 let result=(||{
  let a=pool(m,h,p,0,m.word(p+24).wrapping_mul(4),l);m.put_word(p+28,a);if a==0{return false;}
  let mut i=0i32;
  while i<(m.word(p+24) as i32){let q=pool(m,h,p,0,8,l);m.put_word(m.word(p+28)+(i as u32)*4,q);if q==0{return false;}i+=1;}
  let q=pool(m,h,p,0,m.word(p+24).wrapping_mul(4),l);m.put_word(p+32,q);if q==0{return false;}
  let q=pool(m,h,p,0,m.word(p+24).wrapping_mul(4),l);m.put_word(p+36,q);if q==0{return false;}
  let pair=window_pair(m,p,0);m.put_word(pair+4,0);m.put_word(window_pair(m,p,0),0);
  let q=pool(m,h,p,0,4,l);m.put_word(m.word(p+32),q);if q==0{return false;}
  let q=pool(m,h,p,0,4,l);m.put_word(m.word(p+36),q);if q==0{return false;}
  let shift=dim.wrapping_sub(dim>>1)&255;m.put_word(m.word(m.word(p+32)),if shift>=32{0}else{1u32<<shift});
  let shift=n.wrapping_add(n>>1).wrapping_sub(dim)&255;m.put_word(m.word(m.word(p+36)),if shift>=32{0}else{1u32<<shift});
  i=1;
  while i<(m.word(p+24) as i32){
   let win=m.word(wins+(i as u32-1)*4);m.put_word(win+12,0);
   let out=m.word(p+32)+i as u32*4;let got=fn_2a9274(m,h,p,out,4,u32::MAX,win,l) as i32;
   let half=got/2;let q=half/2;
   let a=m.word(m.word(p+32)+i as u32*4);m.put_word(m.word(p+36)+i as u32*4,a.wrapping_add((half as u32)*4));
   let a=m.word(m.word(p+32)+i as u32*4);m.put_word(m.word(p+32)+i as u32*4,a.wrapping_add((q as u32)*4));
   let a=m.word(m.word(p+36)+i as u32*4);m.put_word(m.word(p+36)+i as u32*4,a.wrapping_add((q as u32)*4));
   let pair=window_pair(m,p,i);m.put_word(pair,q.wrapping_neg() as u32);m.put_word(window_pair(m,p,i)+4,if half&1==0{q.wrapping_sub(1)}else{q} as u32);i+=1;
  }
  true
 })();
 if !result {pool(m,h,p,2,0,l);return 0x84e0200a;}
 m.put_word(p+44,0);m.put_word(p+40,0);
 let count=m.word(p+24) as i32;let mut i=if count>0{count&1}else{0};
 if i==1 {let row=window_pair(m,p,0);m.put_word(p+40,(m.word(row) as i32).min(0) as u32);m.put_word(p+44,(m.word(row+4) as i32).max(0) as u32);}
 while i<count {
  let row=window_pair(m,p,i);let mut a=m.word(row) as i32;let b=(m.word(row+4) as i32).max(m.word(p+44) as i32);
  m.put_word(p+40,a.min(m.word(p+40) as i32) as u32);m.put_word(p+44,b as u32);
  a=a.min(m.word(window_pair(m,p,i+1)) as i32);m.put_word(p+40,a as u32);
  let b2=m.word(window_pair(m,p,i+1)+4) as i32;m.put_word(p+44,b.max(b2) as u32);i+=2;
 }
 m.put_word(p+48,(m.word(p+40) as i32).max(m.word(p+44) as i32) as u32);0
}

// Prediction-buffer destructors from the 2aa294 call tree. No callback sees
// cached heap/buffer fields after a preceding callback has mutated the arena.
fn close_prediction_buffer(m:&mut Memory<'_>,h:&mut impl Host,p:u32)->u32 {
 if p==0||m.word(p+4)==0{return u32::MAX;}
 release(m,h,m.word(p),m.word(p+4));
 m.put_word(p+4,0);m.put_word(p+8,0);m.put_word(p+12,0);0
}
fn close_prediction_group(m:&mut Memory<'_>,h:&mut impl Host,p:u32,offsets:&[u32])->u32 {
 let mut rc=0;for &off in offsets {if close_prediction_buffer(m,h,p+off)!=0{rc=u32::MAX;}}rc
}
/// Destroy the complete prediction object and clear its caller-owned pointer.
pub fn fn_2aa294(m:&mut Memory<'_>,h:&mut impl Host,pp:u32)->u32 {
 if pp==0{return u32::MAX;}
 let p=m.word(pp);let mut e1=0;
 if p!=0 {
  // fn_2dddf8
  for off in [0x1e8,0x1e0] {if m.word(p+off)!=0 {release(m,h,m.word(m.word(p)+4),m.word(p+off));m.put_word(p+off+4,0);}}
  // fn_2dd828
  if m.word(p+0x154)==0 {e1=u32::MAX;} else {
   if m.word(m.word(p+0x154))!=0 {release(m,h,m.word(m.word(p)+4),m.word(m.word(p+0x154)));}
   release(m,h,m.word(m.word(p)+4),m.word(p+0x154));
  }
  let mut e2=close_prediction_group(m,h,p,&[0x13c,0x128]);
  if m.byte(p+0xf8)<=m.byte(p+0x32)&&close_prediction_buffer(m,h,p+0xe0)!=0{e2=u32::MAX;}
  if m.byte(p+0x118)<=m.byte(p+0x32)&&close_prediction_buffer(m,h,p+0x108)!=0{e2=u32::MAX;}
  if close_prediction_group(m,h,p,&[0x50,0x188,0x198])!=0{e2=u32::MAX;}
  if close_prediction_group(m,h,p,&[0x40,0x68,0x78,0x88,0x98,0xa8])!=0{e2=u32::MAX;}
  if close_prediction_group(m,h,p,&[0xb8,0xc8,0x178])!=0||e2!=0{e1=u32::MAX;}
  let owner=m.word(p);if owner!=0 {release(m,h,m.word(owner+4),p);}
  m.put_word(pp,0);
 }e1
}

/// Boot constructor. Arguments retain the soft-float ABI's raw words, so
/// pointer-valued x/y arguments are not converted through host floating point.
/// frame reserves 144 bytes, disjoint from the reader runtime's 512 bytes.
pub fn fn_2aa3e4<H:Host+crate::boot_port_6::Host+crate::render_part_a::ReaderHost>(
 m:&mut Memory<'_>,h:&mut H,a:[u32;11],frame:u32,k:&crate::render_part_a::ReaderRuntime,
 tables:&[u32;7])->u32 {
 let [out,ctx,params,reader,duration,x,window,rate,mode,text,filter]=a;
 let p=calloc(m,h,m.word(ctx+4),1,500);m.put_word(out,p);
 if p==0{return u32::MAX;}
 m.put_word(p,ctx);
 for off in (0..32).step_by(4){m.put_word(p+16+off,m.word(params+off));}
 m.put_word(p+0x30,m.word(params+32));
 let at=m.offset(p+0x32);m.bytes[at]=mode as u8;
 m.put_word(p+0x60,x);m.put_word(p+0x64,x);
 let rc=(||{
  if ![8000,11025,16000,22050].contains(&rate){return u32::MAX;}
  crate::boot_port_6::fn_2ddf4c(m,p+4,rate as i32);
  if fn_2a9f24(m,h,p)!=0{return u32::MAX;}
  m.put_word(p+0x18,0x4000);
  m.put_word(p+0x1c,crate::selection::rounded_product(0x4000,m.word(p+0x28).wrapping_shl(2) as i32,16) as u32);
  m.put_word(p+0x24,0x4000);
  if crate::boot_port_6::fn_2dd870(m,p,rate)!=0{return u32::MAX;}
  if crate::boot_port_6::fn_2dd97c(m,h,p,filter)!=0{return u32::MAX;}
  if crate::boot_port_6::fn_2dd914(m,h,p,if rate==8000||rate==11025{64}else{128})!=0{return u32::MAX;}
  m.put_half(p+0x30,(crate::morphology::duration_samples(m.word(p+4),duration)>>6) as u16);
  if crate::boot_port_6::fn_2dd148(m,h,p,reader,frame,k)!=0{return u32::MAX;}
  m.put_word(p+0x11c,0);
  let freq=crate::morphology::duration_frequency(m.word(p+4),16000) as u32;
  m.put_half(p+0x100,freq as u16);
  m.put_word(p+0xfc,crate::morphology::duration_frequency(m.word(p+4),3840) as u32);
  let at=m.offset(p+0xf8);m.bytes[at]=3;
  if m.byte(p+0x32)>=3{
   if crate::boot_port_6::fn_2db9b0(m,h,m.word(m.word(p)+4),p+0xe0,4)!=0{return u32::MAX;}
   let count=m.word(p+0xec) as i32;
   if count>0 {
    let start=m.offset(m.word(p+0xe4));let value=freq as u16;
    // fill_samples is the existing 2db63c port.
    let mut cell=[0i16;1];crate::selection::fill_samples(&mut cell,value as i16,1).unwrap();
    for i in 0..count as usize{m.bytes[start+i*2..start+i*2+2].copy_from_slice(&cell[0].to_le_bytes());}
   }
   m.put_word(p+0xf0,0);let divisor=m.word(p+0xe8);m.put_word(p+0xf4,0x10000/divisor);
  }
  m.put_word(p+0x38,m.word(p+4));let at=m.offset(p+0x118);m.bytes[at]=3;
  if m.byte(p+0x32)>=3&&crate::boot_port_6::fn_2db9b0(m,h,m.word(m.word(p)+4),p+0x108,5)!=0{return u32::MAX;}
  if crate::boot_port_6::fn_2dd064(m,h,p)!=0{return u32::MAX;}
  if crate::boot_port_6::fn_2dba38(m,h,m.word(m.word(p)+4),p+0x40,m.word(p+0x60).wrapping_add(1))!=0{return u32::MAX;}
  m.put_half(p+0x106,0);m.put_word(p+0x14c,0);
  let n=u16::from_le_bytes(m.bytes[m.offset(p+0x30)..m.offset(p+0x30)+2].try_into().unwrap()) as u32;
  m.put_word(p+0xd8,n);
  let q=if n==0{0}else{(0x10000/n) as u16 as u32};m.put_word(p+0xdc,q);
  if q==0||n==0{return u32::MAX;}
  let duration=match m.byte(p+0x32){3|4=>1,2=>4,1=>11,0=>30,_=>return u32::MAX};
  if crate::boot_port_6::fn_2dba38(m,h,m.word(m.word(p)+4),p+0x68,m.word(p+0x60).wrapping_add(1))!=0{return u32::MAX;}
  m.put_word(p+0x174,(crate::morphology::duration_samples(m.word(p+4),duration)>>6) as u32);
  let mut settings=crate::selection::WindowSettings::default();
  let text=if text==0{None}else{Some(&m.bytes[m.offset(text)..])};
  if crate::selection::window_settings(m.word(p+0x60)!=0,m.byte(p+0x32) as i32,rate as i32,window,text,&mut settings).unwrap()!=0{return u32::MAX;}
  let at=m.offset(frame+128);let mut saved=[0u8;12];saved.copy_from_slice(&m.bytes[at..at+12]);
  m.put_word(frame+128,settings.value);
  for (i,b) in settings.parameters.into_iter().enumerate(){let at=m.offset(frame+132+i as u32);m.bytes[at]=b;}
  let args=[m.word(frame+128),m.word(frame+132),m.word(frame+136)];
  let at=m.offset(frame+128);m.bytes[at..at+12].copy_from_slice(&saved);
  if crate::boot_port_6::fn_2dc798(m,h,p,args[0],args[1],args[2],tables)!=0{return u32::MAX;}
  for off in [0x88,0x78,0x98]{
   let n=m.word(p+0x14).wrapping_add(1).wrapping_add(m.word(p+0x3c)).wrapping_add(m.word(p+0xd8));
   if crate::boot_port_6::fn_2db9b0(m,h,m.word(m.word(p)+4),p+off,n)!=0{return u32::MAX;}
  }
  if crate::boot_port_6::fn_2db9b0(m,h,m.word(m.word(p)+4),p+0xa8,m.word(p+0xd8).wrapping_mul(2))!=0{return u32::MAX;}
  m.put_half(p+0x138,freq as u16);m.put_half(p+0x13a,0);
  for off in [0x13c,0x128]{
   if crate::boot_port_6::fn_2dba38(m,h,m.word(m.word(p)+4),p+off,m.word(p+0x60).wrapping_add(1))!=0{return u32::MAX;}
  }
  // In-place coefficient update, preserving the original backwards traversal.
  // The existing typed 2dbb2c port is applied to each one-step recurrence.
  let ptr=m.word(p+0x140);let order=m.word(p+0x60);
  let start=m.offset(ptr);let mut input=[0i32;2];let mut output=[0i32;2];
  for i in (0..order).rev(){
   input[0]=m.word(ptr+i*4) as i32;input[1]=m.word(ptr+(i+1)*4) as i32;
   // The unclamped recurrence is needed for internal coefficients.
   let product=(input[1].wrapping_neg() as i64)*(m.word(p+0x1b0) as i32 as i64);
   m.put_word(ptr+i*4,((product>>24) as i32).wrapping_add(input[0]) as u32);
  }
  input[0]=(m.word(ptr) as i32).max(0);input[1]=m.word(ptr+4) as i32;
  crate::selection::prediction_coefficients(&input,&mut output,1,0).unwrap();
  m.bytes[start..start+4].copy_from_slice(&output[0].to_le_bytes());
  m.put_word(ptr+4,output[1] as u32);0
 })();
 if rc!=0 {
  let saved=m.word(frame);m.put_word(frame,p);fn_2aa294(m,h,frame);m.put_word(frame,saved);m.put_word(out,0);u32::MAX
 }else{0}
}

fn object_calloc(m:&mut Memory<'_>,h:&mut impl crate::boot_part_f::Host,heap:u32,n:u32,size:u32,err:&mut u8,scratch:u32)->u32{
 let at=m.offset(scratch);let saved=m.bytes[at];m.bytes[at]=*err;
 let p=crate::boot_port_1::fn_1b20dc(m,heap,n,size,scratch,h);
 let at=m.offset(scratch);*err=m.bytes[at];m.bytes[at]=saved;p
}
fn object_copy(m:&mut Memory<'_>,h:&mut impl crate::boot_part_f::Host,heap:u32,source:u32,out:u32)->u8{
 if source==0{m.put_word(out,0);return 0;}
 let at=m.offset(source) as u32;let n=crate::fe_text::strlen_195d90(m.bytes,at) as u32+1;
 let mut err=0;let p=crate::boot_part_f::fn_1b2140(m,heap,n,&mut err,h);m.put_word(out,p);
 if err==0{let d=m.offset(m.word(out)) as u32;let s=m.offset(source) as u32;crate::text_input::strncpy_at(m.bytes,d,s,n);}err
}
/// fn_30a2d4. Construct the map object of the ARM runtime library.
///
/// The body is reconstructed from the US ROM image at 0x30a2d4: the tree
/// header is allocated through the guest allocator object, grown to
/// `20 + 20 * limit` bytes and filled with the index table; the byte buffer
/// is grown to `limit` bytes. The two vtable words are the ROM code
/// addresses 0x30a2a8 and 0x30a2bc. `scratch` reserves four disjoint bytes
/// and is restored before returning.
pub fn fn_30a2d4<H:crate::boot_part_f::Host>(m:&mut Memory<'_>,h:&mut H,a:u32,b:u32,c:u32,d:u32,scratch:u32)->u32{
 let at=m.offset(scratch);let mut saved=[0u8;4];saved.copy_from_slice(&m.bytes[at..at+4]);
 let node=crate::boot_port_1::fn_1b20dc(m,b,1,20,scratch,h);
 let mut ptr=0u32;
 if m.bytes[at]==0 {
  m.put_word(node+12,c);m.put_word(node+16,b);
  let limit=m.word(node).wrapping_add(c);
  if m.word(node)<=limit {
   let size=20u32.wrapping_add(20u32.wrapping_mul(limit));
   let mut flag2=0u8;
   let block=crate::boot_part_f::fn_1b218c(m,b,node,size,&mut flag2,h);
   if flag2==0 {
    let mut i=m.word(block).wrapping_add(1);
    m.put_word(block+4,i);m.put_word(block,limit);
    while i<limit {m.put_word(block+8+20u32.wrapping_mul(i),i+1);i=i.wrapping_add(1);}
    let last=(block+20).wrapping_add(20u32.wrapping_mul(i));
    m.put_word(last.wrapping_sub(12),0);
    ptr=block;
   }
  }
 }
 m.put_word(a,ptr);
 if ptr==0 {m.bytes[at..at+4].copy_from_slice(&saved);return 1;}
 m.put_word(a+4,0x30a2a8);m.put_word(a+8,0x30a2bc);
 m.put_word(a+36,u32::MAX);m.put_word(a+40,b);
 let base=m.offset(a+12) as u32;
 crate::fe_text::memset_195bec(m.bytes,base,0,24);
 m.put_word(a+20,d);m.put_word(a+28,1);m.put_word(a+32,b);
 let mut failed=false;
 if m.word(a+24)<=m.word(a+12) {
  let end=m.word(a+24).wrapping_add(m.word(a+20));
  m.put_word(a+24,end);
  let size=end.wrapping_mul(m.word(a+28));
  let mut flag3=0u8;
  let block=crate::boot_part_f::fn_1b218c(m,m.word(a+32),m.word(a+16),size,&mut flag3,h);
  m.put_word(a+16,block);
  failed=flag3!=0;
 }
 if !failed {
  let stride=m.word(a+28);
  let dest=m.word(a+16).wrapping_add(m.word(a+12).wrapping_mul(stride));
  let d0=m.offset(dest) as u32;
  m.bytes[d0 as usize..(d0+stride) as usize].fill(0);
  let count=m.word(a+12);m.put_word(a+12,count+1);
 }
 m.bytes[at..at+4].copy_from_slice(&saved);
 if failed {1} else {0}
}
/// Append an option node, then initialize it. The map initializer 30a2d4
/// runs its real body through the guest allocator object.
pub fn fn_2e41d0<H:Host+crate::boot_part_f::Host>(m:&mut Memory<'_>,h:&mut H,p:u32,options:u32,scratch:u32)->u32{
 let mut last=m.word(p+40);let mut err=0;
 let node=object_calloc(m,h,m.word(p+36),1,80,&mut err,scratch);
 if err!=0{return err as u32;}
 if last==0{m.put_word(p+40,node);}else{
  while m.word(last+76)!=0{last=m.word(last+76);}
  m.put_word(last+76,node);
 }
 m.put_word(node+76,0);
 let heap=m.word(p+36);
 for off in [0,4,20,24,28]{m.put_word(node+off,0);}
 m.put_word(node+8,m.word(options));m.put_word(node+16,m.word(options+8));m.put_word(node+12,m.word(options+4));
 if m.word(options+12)!=0 {
  err=object_copy(m,h,heap,m.word(options+12),node+20);
  if err!=0{return err as u32;}
 }
 if m.word(options+16)!=0{
  let data=crate::boot_part_f::fn_1b2140(m,heap,129,&mut err,h);m.put_word(node+24,data);
  if err!=0{return err as u32;}
  let d=m.offset(m.word(node+24)) as u32;let s=m.offset(m.word(options+16)) as u32;
  crate::fe_text::memcpy_195ae8(m.bytes,d,s,128);
  let at=m.offset(m.word(node+24)+128);m.bytes[at]=0;
 }
 if m.word(options+20)!=0 {
  err=object_copy(m,h,heap,m.word(options+20),node+28);
  if err!=0{return err as u32;}
 }
 fn_30a2d4(m,h,node+32,heap,0x2710,0xc350,scratch)
}

pub struct DictionaryEnvironment<'a>{
 pub base_vtable:u32,pub interface_vtable:u32,pub dictionary_vtable:u32,
 pub collection_vtable:u32,pub token_vtable:u32,pub parameters:&'a crate::boot_part_f::Environment,
 pub encrypted:[u8;4],pub plain:[u8;4],
}
/// Construct the dictionary and its live option/collection call trees.
/// frame reserves 36 disjoint caller-owned bytes and is restored on return.
pub fn fn_2b2800<H:Host+crate::boot_port_6::Host+crate::boot_part_f::Host+crate::speech_part_5::Host>(
 m:&mut Memory<'_>,h:&mut H,p:u32,heap:u32,config:u32,image:u32,mode:u32,e:&DictionaryEnvironment<'_>,frame:u32)->u32{
 let mut status=crate::boot_port_6::fn_2e4afc(m,h,p,heap,e.base_vtable,e.interface_vtable,e.parameters) as u8;
 if status!=0{return status as u32;}
 let vt=e.dictionary_vtable;
 for (off,delta) in [(12,-96i32),(28,-60),(40,0),(44,12),(48,24),(52,32),(32,-48),(36,-20),(0,-120)]{
  m.put_word(p+off,vt.wrapping_add(delta as u32));
 }
 m.put_word(p+64,image);m.put_word(p+56,heap);let at=m.offset(p+72);m.bytes[at]=mode as u8;
 let cfg=object_calloc(m,h,heap,1,16,&mut status,frame+32);m.put_word(p+68,cfg);
 if status!=0{return status as u32;}
 if image==0&&config!=0 {
  for off in [0,4,8]{
   let heap=m.word(p+56);let out=m.word(p+68)+off;let source=m.word(config+off);
   status=object_copy(m,h,heap,source,out);if status!=0{return status as u32;}
   if off==0 {
    let at=m.offset(m.word(m.word(p+68))) as u32;
    let pos=crate::fe_text::strchr_195d30(m.bytes,at,b'|' as u32);if pos!=0{m.bytes[pos as usize]=0;}
   }
  }
 }
 status=crate::boot_port_6::fn_2e3ac0(m,h,m.word(p+56),m.word(p+64),mode,p+60,&e.encrypted,&e.plain) as u8;
 if status!=0{return status as u32;}
 if image==0&&config!=0 {
  let at=m.offset(frame);let mut saved=[0u8;32];saved.copy_from_slice(&m.bytes[at..at+32]);
  for (i,v) in [u32::MAX,u32::MAX,u32::MAX,0,0,m.word(m.word(p+68)+8),0,0].into_iter().enumerate(){m.put_word(frame+i as u32*4,v);}
  let source=m.word(m.word(p+68)+4);let mut err=0;
  if source!=0{
   let data=crate::boot_part_f::fn_1b2140(m,m.word(p+56),129,&mut err,h);m.put_word(frame+16,data);
   if err==0{
    let d=m.offset(m.word(frame+16)) as u32;crate::fe_text::memset_195bec(m.bytes,d,0,129);
    let s=m.offset(source) as u32;let n=crate::fe_text::strlen_195d90(m.bytes,s) as u32;crate::fe_text::memcpy_195ae8(m.bytes,d,s,n);
   }
  }
  status=err;
  if status==0{
   if m.word(m.word(p+68))!=0{m.put_word(frame+12,m.word(m.word(p+68)));}
   status=fn_2e41d0(m,h,m.word(p+60),frame,frame+32) as u8;
   if status==0&&m.word(frame+16)!=0{crate::boot_part_f::fn_1b2128(m,m.word(p+56),m.word(frame+16),h);}
  }
  let at=m.offset(frame);m.bytes[at..at+32].copy_from_slice(&saved);
  if status!=0{return status as u32;}
 }
 status=crate::speech_part_5::fn_2b24e8(m,p,&mut DictionaryMethods{host:h,frame}) as u8;

 if status!=0{return status as u32;}
 let rc4598=crate::boot_port_6::fn_2e4598(m,h,m.word(p+56),m.word(p+56),p+76,e.collection_vtable,e.token_vtable,e.parameters)&255;
 rc4598
}

struct DictionaryMethods<'a,H>{host:&'a mut H,frame:u32}
impl<H:crate::boot_port_6::Host+crate::speech_part_5::Host> crate::boot_part_f::Host for DictionaryMethods<'_,H>{
 fn compare(&mut self,m:&mut Memory<'_>,f:u32,k:&mut crate::boot_part_f::Key,v:&mut u32)->u32{crate::boot_part_f::Host::compare(self.host,m,f,k,v)}
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{
  if f==0x2e39ec{return crate::boot_port_6::fn_2e39ec(m,self.host,a[0],a[1],a[2]);}
  crate::boot_part_f::Host::call(self.host,m,f,a)
 }
}
impl<H:crate::speech_part_5::Host> crate::log_live::LogMethodsM for DictionaryMethods<'_,H>{
 fn log_m(&mut self,f:u32,o:u32,c:u32,l:u32,a:u32,fmt:u32,va:u32,m:&mut Memory<'_>)->u32{crate::log_live::LogMethodsM::log_m(self.host,f,o,c,l,a,fmt,va,m)}
}
impl<H:crate::boot_port_6::Host+crate::speech_part_5::Host> crate::speech_part_5::Host for DictionaryMethods<'_,H>{
 fn scratch(&self)->u32{self.host.scratch()}
 fn category(&self,us:u32)->u32{self.host.category(us)}
 fn format(&self,us:u32)->u32{self.host.format(us)}
 fn dictionary_entries(&mut self,m:&mut Memory<'_>,p:u32,out:&mut u32,n:&mut u32)->u32{
  let saved=[m.word(self.frame),m.word(self.frame+4)];
  let rc=crate::boot_port_6::fn_2e4098(m,self.host,p,self.frame,self.frame+4);
  *out=m.word(self.frame);*n=m.word(self.frame+4);m.put_word(self.frame,saved[0]);m.put_word(self.frame+4,saved[1]);rc
 }
 fn record(&mut self,m:&mut Memory<'_>,p:u32,a:u32,b:&mut crate::speech_part_5::RecordBuffer,f:&mut u8)->u32{self.host.record(m,p,a,b,f)}
 fn entries(&mut self,m:&mut Memory<'_>,a:u32,b:u32,c:u32,out:&mut u32,n:&mut u32)->u32{self.host.entries(m,a,b,c,out,n)}
}

fn adjusted(m:&Memory<'_>,p:u32,slot:u32)->u32{p.wrapping_sub(m.word(m.word(p)+slot))}
/// Load a buffer's payload and bind the dictionary. frame reserves 44 bytes.
pub fn fn_2b2b4c<H:Host+crate::boot_port_6::Host+crate::boot_part_f::Host+crate::speech_part_5::Host>(
 m:&mut Memory<'_>,h:&mut H,p:u32,heap:u32,config:u32,buffer:u32,mode:u32,e:&DictionaryEnvironment<'_>,frame:u32)->u32 {
 m.put_word(p+64,0);m.put_word(p+60,0);m.put_word(p+56,heap);let at=m.offset(p+72);m.bytes[at]=mode as u8;
 if buffer!=0{
  let f=m.word(m.word(buffer)+28);
  if Host::call(h,m,f,&[adjusted(m,buffer,104)])!=0x145cdefa{return crate::normalization::code_4(0);}
  let f=m.word(m.word(buffer)+32);let version=Host::call(h,m,f,&[adjusted(m,buffer,104)]);
  if crate::normalization::compare_packed(version,0x10000)==0{return 5;}
  let saved=[m.word(frame+36),m.word(frame+40)];let f=m.word(m.word(buffer)+4);
  let rc=Host::call(h,m,f,&[adjusted(m,buffer,104),0x18b3b204,0,frame+36,frame+40])&255;
  let payload=m.word(frame+36);let version=m.word(frame+40);
  m.put_word(frame+36,saved[0]);m.put_word(frame+40,saved[1]);
  if rc!=0{return rc;}
  if crate::normalization::compare_packed(version,0x10000)==0{return 5;}
  let f=m.word(m.word(payload)+12);
  let size=Host::call(h,m,f,&[adjusted(m,payload,24)]);
  let mut status=0;
  let data=crate::boot_part_f::fn_1b218c(m,m.word(p+56),m.word(p+64),size,&mut status,h);m.put_word(p+64,data);
  if status!=0{return status as u32;}
  let f=m.word(m.word(payload)+16);
  let rc=Host::call(h,m,f,&[adjusted(m,payload,24),m.word(p+64)])&255;if rc!=0{return rc;}
  let f=m.word(m.word(buffer)+12);
  let rc=Host::call(h,m,f,&[adjusted(m,buffer,104),payload])&255;if rc!=0{return rc;}
 }
 fn_2b2800(m,h,p,heap,config,m.word(p+64),mode,e,frame)
}
/// Allocate the dictionary interface and publish it only after construction.
/// frame reserves 48 bytes; archive scratch belongs to the supplied runtime.
pub fn fn_2b4010<H:Host+crate::boot_port_6::Host+crate::boot_part_f::Host+crate::speech_part_5::Host+crate::boot_part_e::ArchiveHost>(
 m:&mut Memory<'_>,h:&mut H,a:[u32;8],e:&DictionaryEnvironment<'_>,archive:&crate::boot_part_e::ArchiveEnvironment,frame:u32)->u32 {
 let [heap,a1,config,a3,key,c1,c2,out]=a;let mut flag=0;
 let obj=crate::boot_part_f::fn_1b2140(m,heap,92,&mut flag,h);
 if flag!=0{return flag as u32;}
 let mut iface=0;let mut inner=0;let mut rc=0;
 if key!=0{
  let saved=m.word(frame+44);m.put_word(frame+44,0);
  rc=crate::boot_part_e::fn_1aaa80(m,[a1,a1,a3,key,c1],frame+44,archive,h);
  iface=m.word(frame+44);m.put_word(frame+44,saved);
  if rc==0{inner=iface+16;}
 }
 if rc==0{
  rc=fn_2b2b4c(m,h,obj,a1,config,inner,c2,e,frame);
  if rc==0&&iface!=0{let f=m.word(m.word(iface)+16);rc=Host::call(h,m,f,&[iface]);}
 }
 flag=rc as u8;
 if rc==0{m.put_word(obj+8,heap);m.put_word(out,obj);}else{
  crate::boot_part_f::fn_1b2128(m,heap,obj,h);m.put_word(out,0);
 }
 flag as u32
}

/// `fn_252480`. The tokenizer close belongs to boot-port-2d, which landed
/// and certified the port. This binding forwards to it, so the lane no
/// longer offers a stub of its own. `fn_2520e4` never calls the close: it
/// is reached from `fn_1e4008`.
pub fn fn_252480(
 m:&mut Memory<'_>,o:u32,e:&crate::boot_port_2d::Environment,h:&mut impl crate::boot_port_2d::Host,
)->u32{
 crate::boot_port_2d::fn_252480(m,o,e,h)
}

/// `fn_281f34`. The dictionary attachment belongs to boot-port-5b, which
/// landed and certified the port. This binding forwards to it.
pub fn fn_281f34(
 m:&mut Memory<'_>,h:&mut impl crate::boot_port_5b::Host,a:u32,e:u32,l:&crate::boot_port_5b::Attach,
)->u32{
 crate::boot_port_5b::fn_281f34(m,h,a,e,l)
}

/// `fn_2da04c`. The model-file loader belongs to boot-port-6, which landed
/// and certified the port. This binding forwards to it. The `0x2a4620`
/// row of this lane still scripts the callee: the model-file scene the
/// loader needs is not part of this lane's arena, so the row keeps the
/// scripted service until that scene exists.
pub fn fn_2da04c(
 m:&mut Memory<'_>,h:&mut impl crate::boot_port_6::Host,ctx:u32,out:u32,mode:u32,files:u32,
 count:u32,k:&crate::boot_port_6::Literals,r:&crate::boot_port_6::StreamRuntime<'_>,
)->u32{
 crate::boot_port_6::fn_2da04c(m,h,ctx,out,mode,files,count,k,r)
}

/// The tokenizer's map environment.
///
/// The ROM keeps the map's five method words in `D_914a98` and the eight
/// check-function pairs in `D_914ad8`. Both tables are ROM data, so the
/// embedder copies them into the arena; the harness takes them from the
/// region's image and folds the region-specific words to their canonical
/// values. The comparator reads the first byte at the key address, so the
/// eight key bytes travel with the table.
pub struct TokenizerEnvironment {
 /// The five `D_914a98` words: key init, key free, comparator, value copy
 /// and value free.
 pub handlers: [u32;5],
 /// The eight `D_914ad8` pairs: the key address and the check function.
 pub entries: [(u32,u32);8],
 /// The eight comparator bytes, indexed by `key - key_base`.
 pub keys: [u8;8],
 /// The ROM address of the first key; entry `i` uses `key_base + i`.
 pub key_base: u32,
 /// A caller-owned word for the map handle. The port restores it.
 pub scratch: u32,
}
/// The map's method table: the manager's three allocator methods.
struct MapHeap<'a,H>{host:&'a mut H,mgr:u32,base:u32}
impl<H:Host> crate::frontend::HeapVtableM for MapHeap<'_,H>{
 fn alloc_at_m(&mut self,obj:u32,n:u32,bytes:&mut [u8])->u32{
  let mut m=Memory{base:self.base,bytes};let f=m.word(m.word(self.mgr));
  self.host.call(&mut m,f,&[obj,n])
 }
 fn realloc_at_m(&mut self,obj:u32,p:u32,n:u32,bytes:&mut [u8])->u32{
  let mut m=Memory{base:self.base,bytes};let f=m.word(m.word(self.mgr)+8);
  self.host.call(&mut m,f,&[obj,p,n])
 }
 fn release_at_m(&mut self,obj:u32,p:u32,bytes:&mut [u8]){
  let mut m=Memory{base:self.base,bytes};let f=m.word(m.word(self.mgr)+12);
  self.host.call(&mut m,f,&[obj,p]);
 }
}
/// The map's tree methods: the comparator reads one byte at each key
/// address, the key init keeps the key and the value copy keeps the value.
struct MapTree{keys:[u8;8],key_base:u32}
impl MapTree{
 fn byte(&self,key:u32)->u8{
  let i=key.wrapping_sub(self.key_base) as usize;
  if i<self.keys.len(){self.keys[i]}else{0}
 }
}
impl crate::frontend::TreeOps for MapTree{
 fn tree_cmp(&mut self,key:u32,node_key:u32)->i32{
  (self.byte(key) as i32)-(self.byte(node_key) as i32)
 }
 fn tree_init(&mut self,key:u32,heap:u32,_node:u32)->(u32,Option<u32>){
  if key==0||heap==0{return (crate::normalization::error_code(7),None);}
  (0,Some(key))
 }
 fn tree_alloc(&mut self,arg:u32,heap:u32)->(u32,u32){
  if arg==0||heap==0{return (crate::normalization::error_code(7),0);}
  (0,arg)
 }
 fn tree_free_node(&mut self,_p:u32,_heap:u32){}
 fn tree_free_handle(&mut self,_p:u32,_heap:u32){}
}
/// The manager's release method, `fn_196428`.
fn map_release(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,mgr:u32,p:u32){
 crate::boot_part_e::release(m,mgr,p,h);
}
/// `fn_19869c`. Free the map's tree in post order. The key and value free
/// callbacks of this map are empty, so only the node releases are visible.
fn map_destroy(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,map:u32,mgr:u32){
 fn walk(m:&mut Memory<'_>,h:&mut impl crate::boot_part_e::Host,tree:u32,node:u32,mgr:u32){
  if node==tree{return;}
  walk(m,h,tree,m.word(node),mgr);
  walk(m,h,tree,m.word(node+4),mgr);
  map_release(m,h,mgr,node);
 }
 let tree=map+24;
 walk(m,h,tree,m.word(tree),mgr);
 m.put_word(tree+24,0);m.put_word(tree,tree);m.put_word(tree+4,tree);
 map_release(m,h,mgr,map);
}
/// The tokenizer close, `fn_25251c`.
struct TokenTree<'a,H>{host:&'a mut H,mgr:u32,base:u32}
impl<H:Host+crate::boot_part_e::Host> crate::normalization::NormalizationHost for TokenTree<'_,H>{
 fn calloc(&mut self,arena:&mut [u8],heap:u32,n:u32,size:u32)->Option<u32>{
  let mut m=Memory{base:self.base,bytes:arena};
  let p=crate::boot_part_e::calloc(&mut m,heap,n,size,self.host);
  (p!=0).then_some(p)
 }
 fn alloc(&mut self,arena:&mut [u8],heap:u32,size:u32)->Option<u32>{
  let mut m=Memory{base:self.base,bytes:arena};
  let p=crate::boot_part_e::alloc(&mut m,heap,size,self.host);
  (p!=0).then_some(p)
 }
 fn realloc(&mut self,arena:&mut [u8],heap:u32,ptr:u32,size:u32)->Option<u32>{
  let mut m=Memory{base:self.base,bytes:arena};
  let p=crate::boot_part_e::fn_1963e0(&mut m,heap,ptr,size,self.host);
  (p!=0).then_some(p)
 }
 fn free(&mut self,arena:&mut [u8],heap:u32,ptr:u32){
  let mut m=Memory{base:self.base,bytes:arena};
  map_release(&mut m,self.host,heap,ptr);
 }
 fn resolve(&mut self,_arena:&mut [u8],_h0:u32,_h1:u32)->(u32,u32){unreachable!("the tokenizer close resolves no context")}
}
/// `fn_2520e4`. Open the tokenizer object of src/fn_2520e4.c.
///
/// The five parts are allocated through the guest allocator object, the
/// check-function map comes from `fn_24d84c` (the `fn_1964e0` object) and
/// the eight `D_914ad8` entries are inserted with `fn_19668c`. The
/// parameters the ROM reads are supplied by the caller: `e` holds the
/// `D_914a98` and `D_914ad8` images and the comparator bytes.
pub fn fn_2520e4<H:Host+crate::boot_part_e::Host>(
 m:&mut Memory<'_>,p:u32,e:&TokenizerEnvironment,h:&mut H)->u32{
 let owner=m.word(p);
 let logger=m.word(owner+0x10);
 let cat=m.word(p+0x48);
 let mgr=m.word(owner+4);
 let at=m.offset(e.scratch);
 let mut saved=[0u8;4];saved.copy_from_slice(&m.bytes[at..at+4]);
 crate::log_live::fn_1738e8(m,logger,cat,4,0,0,0,&mut LogAdapter(h));
 let tok=crate::boot_part_e::calloc(m,mgr,1,4,h);
 if tok==0{
  crate::render_part_a::error_log(m,&mut FormatAdapter(h),logger,cat,0x5df0,0,0,e.scratch);
  return crate::normalization::error_code(10);
 }
 m.put_word(p+0x40,tok);
 let a=crate::boot_part_e::calloc(m,mgr,1,36,h);
 m.put_word(tok,a);
 if a==0{return tokenizer_alloc_fail(m,h,p,e,logger,cat,mgr,tok);}
 let b=crate::boot_part_e::calloc(m,mgr,1,20,h);
 m.put_word(m.word(tok),b);
 if b==0{return tokenizer_alloc_fail(m,h,p,e,logger,cat,mgr,tok);}
 let c=crate::boot_part_e::calloc(m,mgr,1,32,h);
 m.put_word(m.word(tok)+4,c);
 if c==0{return tokenizer_alloc_fail(m,h,p,e,logger,cat,mgr,tok);}
 let d=crate::boot_part_e::calloc(m,mgr,1,4,h);
 m.put_word(m.word(m.word(tok)+4)+4,d);
 if d==0{return tokenizer_alloc_fail(m,h,p,e,logger,cat,mgr,tok);}
 m.put_half(d,0);m.put_half(d+2,0);
 let ee=crate::boot_part_e::calloc(m,mgr,1,92,h);
 m.put_word(m.word(m.word(tok)+4),ee);
 if ee==0{return tokenizer_alloc_fail(m,h,p,e,logger,cat,mgr,tok);}
 m.put_word(ee+84,0);
 let cc=m.word(m.word(tok)+4);
 m.put_word(cc+8,0);m.put_word(cc+0xc,0);m.put_half(cc+0x14,0);m.put_word(cc+0x18,0);
 let bb=m.word(m.word(tok));
 m.put_word(bb,m.word(bb)|4);
 m.put_word(bb+0xc,cc);
 let mut rc=0u32;
 let mut tmp=0u32;
 crate::log_live::fn_1738e8(m,logger,cat,4,0,0,0,&mut LogAdapter(h));
 {
  let slot=m.offset(e.scratch);
  m.put_word(e.scratch,0);
  rc=crate::boot_port_2::fn_1964e0(m,mgr,0,e.handlers,e.scratch,h);
  tmp=m.word(e.scratch);
  m.bytes[slot..slot+4].copy_from_slice(&saved);
 }
 if rc<0x80000000{
  let mut heap=MapHeap{host:h,mgr,base:m.base};
  let mut tree=MapTree{keys:e.keys,key_base:e.key_base};
  let mut i=0usize;
  while i<e.entries.len(){
   let (key,value)=e.entries[i];
   rc=crate::boot_part_b2::guest_19668c_m(m,tmp,key,value,&mut heap,&mut tree);
   if rc>=0x80000000{
    crate::render_part_a::error_log(m,&mut FormatAdapter(h),logger,cat,0x5213,0,0,e.scratch);
    break;
   }
   i+=1;
  }
  if rc<0x80000000{
   m.put_word(m.word(p+0x3c),tmp);
   rc=0;
  }
  if rc>=0x80000000&&tmp!=0{
   map_destroy(m,h,tmp,mgr);
  }
 }else{
  crate::render_part_a::error_log(m,&mut FormatAdapter(h),logger,cat,0x5de7,0,0,e.scratch);
  if tmp!=0{map_destroy(m,h,tmp,mgr);}
 }
 crate::log_live::fn_1738e8(m,logger,cat,4,0,0,0,&mut LogAdapter(h));
 if rc>=0x80000000{
  let mut tree=TokenTree{host:h,mgr,base:m.base};
  crate::normalization::close_token_tree(m.bytes,p,&mut tree);
 }
 crate::log_live::fn_1738e8(m,logger,cat,4,0,0,0,&mut LogAdapter(h));
 rc
}
/// The allocation-failure path of `fn_2520e4`.
fn tokenizer_alloc_fail<H:Host+crate::boot_part_e::Host>(
 m:&mut Memory<'_>,h:&mut H,p:u32,e:&TokenizerEnvironment,logger:u32,cat:u32,mgr:u32,tok:u32)->u32{
 crate::render_part_a::error_log(m,&mut FormatAdapter(h),logger,cat,0x5df0,0,0,e.scratch);
 let rc=crate::normalization::error_code(10);
 let mut tree=TokenTree{host:h,mgr,base:m.base};
 crate::normalization::close_token_tree(m.bytes,p,&mut tree);
 let _=tok;
 crate::log_live::fn_1738e8(m,logger,cat,4,0,0,0,&mut LogAdapter(h));
 rc
}

/// `fn_26402c`. Open the synthesis sequence of src/fn_263384.c.
///
// ------------------------------------------------- 26402c sequence constructor
/// The sequence constructor's arena environment.
///
/// `fn_26402c` (src/fn_263384.c) builds the sequence object. Its two render
/// callees, `fn_263c68` and `fn_263abc`, live in render_part_b.rs, so the
/// environments below are the render lane's, filled with this lane's arena
/// addresses. Nothing in render_part_b.rs changes for this lane.
pub struct SequenceEnvironment {
    /// The engine's cached handle and default context.
    pub engine: crate::render_part_c::Environment,
    /// The render lane's lifecycle environment.
    pub lifecycle: crate::render_part_b::LifecycleEnvironment,
    /// The resource service's environment.
    pub resources: crate::render_part_c2::ResourceEnvironment,
    /// The conversion environment.
    pub conversion: crate::render_part_c2::Environment<'static>,
    /// The dictionary environment.
    pub dictionaries: crate::render_part_d::DictionaryEnvironment,
    /// The user-dictionary parser's constructor data.
    pub words: crate::render_part_c::ConstructorData,
    /// The parser's constructor data.
    pub parser: crate::render_part_c::ParserConstructorDataM<'static>,
    /// The broker table the sequence's buffer registration uses.
    pub broker_table: u32,
    /// The break descriptor's tag.
    pub brkd_name: u32,
    /// The break descriptor's callback and destructor.
    pub brkd_callbacks: [u32;2],
    /// The two user-dictionary registry names.
    pub user_names: [u32;2],
    /// The category the sequence's diagnostics use.
    pub error_category: u32,
    /// The lifecycle environment's parameter pair and triple.
    pub parameter_pairs: u32,
    pub parameter_triple: u32,
    /// The ROM's endianness flag: zero selects the big-endian machine.
    pub endian_flag: u32,
    /// Caller-owned scratch: 512 disjoint bytes for the render callees.
    pub scratch: u32,
    /// The flag byte `fn_263c68` writes. It lives outside the compared
    /// window, like the constructor's stack flag in the C.
    pub flag: u32,
    /// The three parameter key strings, in the arena.
    pub parameter_keys: [u32;3],
}
impl SequenceEnvironment {
    /// The render lane's setup environment over this lane's arena.
    pub fn setup(&self) -> crate::render_part_b::SetupEnvironment<'_> {
        crate::render_part_b::SetupEnvironment {
            lifecycle: &self.lifecycle,
            resources: &self.resources,
            conversion: &self.conversion,
            dictionaries: &self.dictionaries,
            words: &self.words,
            parser: &self.parser,
            broker_table: self.broker_table,
            brkd_name: self.brkd_name,
            brkd_callbacks: self.brkd_callbacks,
            user_names: self.user_names,
            error_category: self.error_category,
        }
    }
    /// The lifecycle environment, with this lane's parameter pair and triple.
    pub fn lifecycle(&self) -> crate::render_part_b::LifecycleEnvironment {
        crate::render_part_b::LifecycleEnvironment {
            engine: crate::render_part_c::Environment {
                cached_handle: self.engine.cached_handle,
                default_context: self.engine.default_context,
            },
            parameter_pairs: self.parameter_pairs,
            parameter_triple: self.parameter_triple,
        }
    }
}
/// The host the sequence constructor needs: the allocator, parameter,
/// regex, engine and render ports the constructor and its two render
/// callees use.
pub trait SequenceHost:
    Host
    + crate::normalization::NormalizationHost
    + crate::regex::PcreHost
    + crate::component_live_b::Host
    + crate::render_part_b::SetupHost
{
}
impl<T> SequenceHost for T where
    T: Host
        + crate::normalization::NormalizationHost
        + crate::regex::PcreHost
        + crate::component_live_b::Host
        + crate::render_part_b::SetupHost
{
}
/// `fn_196394`: the allocator's allocate slot.
fn seq_alloc<H: Host>(m: &mut Memory<'_>, h: &mut H, heap: u32, size: u32) -> u32 {
    if heap == 0 {
        return 0;
    }
    let vt = m.word(heap);
    if vt == 0 {
        return 0;
    }
    h.call(m, vt, &[m.word(heap + 4), size])
}
/// `fn_263abc`: the sequence teardown.
fn seq_teardown<H: SequenceHost>(
    m: &mut Memory<'_>,
    h: &mut H,
    o: u32,
    e: &SequenceEnvironment,
    blocks: &mut [crate::regex::FrameBlockState],
) {
    crate::render_part_b::fn_263abc(m, o, &e.lifecycle(), e.scratch, blocks, h);
}
/// `fn_26402c`. Build the sequence object.
///
/// The sequence object is allocated from the resolved context, the two
/// parameter lookups fill the frame limits, the break configuration and the
/// PCRE handle are opened and `fn_263c68` builds the subcomponents. The
/// rollback precedence is the constructor's: a failing break configuration
/// or PCRE handle tears the object down, a failing setup tears it down
/// unless the setup itself already did.
pub fn fn_26402c<H: SequenceHost>(
    m: &mut Memory<'_>,
    h: &mut H,
    handle: [u32;2],
    vt8: u32,
    out: u32,
    e: &SequenceEnvironment,
    blocks: &mut [crate::regex::FrameBlockState],
    mut compare: impl FnMut(u32,u32)->i32,
    callback: &mut impl FnMut(u32,u32,&mut [u8;32]),
) -> u32 {
    if out == 0 {
        return 0x81102007;
    }
    let (rc, ctx) = crate::render_part_c::resolve(m, handle, &e.engine);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    m.put_word(out, 0);
    let o = seq_alloc(m, h, m.word(ctx + 4), 0x88);
    if o == 0 {
        return 0x8110200a;
    }
    crate::normalization::memset_at(m.bytes, o, 0, 0x88);
    m.put_word(o + 8, vt8);
    m.put_word(o, handle[0]);
    m.put_word(o + 4, handle[1]);
    m.put_byte(o + 0x77, 0);
    m.put_word(o + 0x7c, 0);
    m.put_word(o + 0x70, 0);
    m.put_word(o + 0x2c, 0);
    m.put_half(o + 0x30, 0);
    m.put_half(o + 0x82, 30);
    let params = m.word(ctx + 0x14);
    if crate::component_live_b::get_property_m(
        m,
        0x175944,
        &[params, e.parameter_keys[0], e.scratch],
        h,
    ) < 0x80000000
    {
        m.put_half(o + 0x82, m.word(e.scratch) as u16);
    }
    m.put_half(o + 0x84, 50);
    if crate::component_live_b::get_property_m(
        m,
        0x175944,
        &[params, e.parameter_keys[1], e.scratch],
        h,
    ) < 0x80000000
    {
        m.put_half(o + 0x84, m.word(e.scratch) as u16);
    }
    m.put_byte(o + 0x80, if e.endian_flag != 0 { 1 } else { 0 });
    let domain = h.literal(m, 0x915020);
    let logger = m.word(ctx + 0x10);
    crate::log_live::fn_1738e8(m, logger, domain, 4, 0, 0, 0, &mut LogAdapter(h));
    m.put_word(o + 0x4c, 0);
    m.put_word(o + 0x50, 0);
    m.put_word(o + 0x54, 0);
    m.put_word(o + 0x58, 0);
    m.put_word(o + 0x5c, 0);
    m.put_word(o + 0x60, 0);
    m.put_word(o + 0x64, 0);
    let pair = [
        u32::from_le_bytes(e.engine.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(e.engine.cached_handle[4..].try_into().unwrap()),
    ];
    crate::boot_part_d::fn_176fd4(m, o + 0x68, pair);
    let rc = crate::text_input::alloc_break_config(
        m.bytes,
        handle[0],
        handle[1],
        o + 0x7c,
        h,
    );
    if rc >= 0x80000000 {
        seq_teardown(m, h, o, e, blocks);
        return rc;
    }
    m.put_word(o + 0x44, 0);
    m.put_word(o + 0x48, 0);
    let mut slot = crate::regex::PcreHandleOut { object: None, key: 0 };
    let rc = crate::regex::pcre_open(h, handle[0], handle[1] as i32, Some(&mut slot));
    m.put_word(o + 0x44, slot.object.unwrap_or(0));
    m.put_word(o + 0x48, slot.key);
    if let Some(object) = slot.object {
        m.put_word(object, ctx);
    }
    if rc >= 0x80000000 {
        seq_teardown(m, h, o, e, blocks);
        return rc;
    }
    let rc = crate::component_live_b::pcre_init_m(
        m,
        o + 0x44,
        o + 0x48,
        m.half(o + 0x82) as i32,
        m.half(o + 0x84) as i32,
        h,
    );
    if rc >= 0x80000000 {
        seq_teardown(m, h, o, e, blocks);
        return rc;
    }
    m.put_byte(e.flag, 0);
    let setup = e.setup();
    let rc = crate::render_part_b::fn_263c68(
        m,
        o,
        e.flag,
        &setup,
        e.scratch,
        blocks,
        &mut compare,
        callback,
        h,
    );
    if rc >= 0x80000000 {
        if m.byte(e.flag) != 0 {
            return rc;
        }
        seq_teardown(m, h, o, e, blocks);
        return rc;
    }
    m.put_byte(o + 0x81, 0);
    if crate::component_live_b::get_property_m(
        m,
        0x175a1c,
        &[params, e.parameter_keys[2], e.scratch],
        h,
    ) < 0x80000000
    {
        let value = m.word(e.scratch);
        if value != 0 && m.byte(value) != 0 {
            m.put_byte(o + 0x81, 1);
        }
    }
    m.put_word(out, o);
    rc
}
