
use crate::boot::Memory;

pub trait Host: crate::boot_part_f::Host {
    fn context_pair(&mut self, _m:&Memory<'_>)->[u8;8] {[0;8]}
    fn default_context(&mut self, _m:&Memory<'_>)->u32 {0}
    fn literal(&mut self, _m: &Memory<'_>, us: u32) -> u32 { us }

    fn base_vtable(&self) -> u32 { 0 }

    fn interface_vtable(&self) -> u32 { 0 }

    fn ctor_vtable(&self) -> u32 { 0 }

    fn parameters(&self) -> crate::boot_part_f::Environment {
        crate::boot_part_f::Environment {
            stream_base_vtable: 0,
            stream_vtable: 0,
            vector_vtable: 0,
            compare: 0,
            accept: 0,
        }
    }

    fn ctor_names(&self) -> Names {
        Names { entry: 0, first: 0, second: 0 }
    }
}

pub fn fn_2b1d0c(m: &mut Memory<'_>, h: &mut impl Host, p: u32) {
    let allocator = m.word(p.wrapping_add(0x34));
    let collection = m.word(p.wrapping_add(0x5c));
    if crate::boot_port_1::fn_1ba384(m, allocator, collection, h) != 0 { return; }
    let record = m.word(p.wrapping_add(0x58));
    if record != 0 {
        crate::boot_part_f::fn_1b2128(m, allocator, record, h);
    }
    fn_2de278(m, h, p);
}

pub fn fn_2de278(m: &mut Memory<'_>, h: &mut impl Host, p: u32) {
    let allocator = m.word(p.wrapping_add(0x34));
    crate::boot_part_f::fn_1b2128(m, allocator, m.word(p.wrapping_add(0x44)), h);
    crate::boot_part_f::fn_1b2128(m, allocator, m.word(p.wrapping_add(0x3c)), h);
    let allocator = m.word(p.wrapping_add(0x34));
    if crate::boot_port_1::fn_1ba384(m, allocator, m.word(p.wrapping_add(0x40)), h) != 0 { return; }
    let allocator = m.word(p.wrapping_add(0x34));
    if crate::boot_port_1::fn_1ba384(m, allocator, m.word(p.wrapping_add(0x38)), h) != 0 { return; }
    fn_2de8ac(m, h, p);
}

pub fn fn_2de8ac(m: &mut Memory<'_>, h: &mut impl Host, p: u32) -> u32 {
    crate::boot_part_f::fn_1b2128(m, m.word(p.wrapping_add(0x34)), m.word(p.wrapping_add(0x2c)), h);
    m.put_word(p.wrapping_add(0x2c), 0);
    let slot = m.word(p.wrapping_add(0x30));
    if slot != 0 {
        let allocator = m.word(p.wrapping_add(0x34));
        let rc = crate::boot_port_1::fn_1ba384(m, allocator, slot, h);
        if rc != 0 { return rc; }
        m.put_word(p.wrapping_add(0x30), 0);
    }
    crate::boot_part_f::fn_1b2128(m, m.word(p.wrapping_add(0x34)), m.word(p.wrapping_add(0x24)), h);
    m.put_word(p.wrapping_add(0x24), 0);
    let slot = m.word(p.wrapping_add(0x1c));
    if slot != 0 {
        let allocator = m.word(p.wrapping_add(0x34));
        let rc = crate::boot_port_1::fn_1ba384(m, allocator, slot, h);
        if rc != 0 { return rc; }
        m.put_word(p.wrapping_add(0x1c), 0);
    }
    fn_2e4b58(m, h, p)
}

pub const IFACE_RELEASE_TOKEN: u32 = 0x9000_0040;

struct IfaceHost<'a, H>(&'a mut H);
impl<H: Host> crate::boot_part_e::Host for IfaceHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 { self.0.call(m, f, a) }
}
impl<H: Host> crate::boot_part_e::ArchiveHost for IfaceHost<'_, H> {
    fn service(&mut self, _: &mut Memory<'_>, _: u32, _: &[u32], _: &mut [u32; 4]) -> u32 {
        panic!("interface release adapter")
    }
    fn read(&mut self, _: &mut Memory<'_>, _: u32, _: u32, _: &mut [u8]) -> u32 {
        panic!("interface release adapter")
    }
}

pub fn fn_2e4b58(m: &mut Memory<'_>, h: &mut impl Host, p: u32) -> u32 {
    let iface = m.word(p.wrapping_add(0x14));
    if iface != 0 {
        let vt = m.word(iface);
        let method = m.word(vt.wrapping_add(0x20));
        let adj = m.word(vt.wrapping_add(0x28));
        let rc = match method {
            IFACE_RELEASE_TOKEN => {
                let mut a = IfaceHost(h);
                crate::boot_part_e::fn_1b44cc(m, iface.wrapping_sub(adj), &mut a)
            }
            other => panic!("unbound interface release token {other:x}"),
        };
        if rc != 0 { return rc; }
    }
    crate::boot_port_1::fn_1b44c8(m)
}

pub fn fn_2de1bc(m: &mut Memory<'_>, h: &mut impl Host, p: u32, allocator: u32,
    methods: u32, entry: u32, arg5: u32, t: &Tables) -> u32 {
    let saved = m.word(t.slot);
    m.put_word(t.slot, 0);

    let s = h.call(m, 0x1aaa80, &[allocator, allocator, methods, entry, arg5]);
    let stream = m.word(t.slot);
    m.put_word(t.slot, saved);
    if s != 0 { return s; }
    let s = h.call(m, 0x2de614, &[p, allocator, stream.wrapping_add(16)]);
    if s != 0 { return s; }
    m.put_word(p, t.vtable);
    let s = h.call(m, 0x2ddfe4, &[p, stream.wrapping_add(16)]);
    if s != 0 { return s; }
    let table = m.word(stream);
    h.call(m, m.word(table.wrapping_add(16)), &[stream])
}

pub fn fn_2e40d0(m: &mut Memory<'_>, h: &mut impl Host, a: u32) -> u32 {
    if m.word(a.wrapping_add(0x18)) != 0 {

        return crate::speech_part_5::fn_1b4238(m, 0x2e3f04);
    }
    let flag = m.byte(a.wrapping_add(0x14));
    if flag == 1 { return fn_300530(m, h, a); }
    fn_30047c(m, h, a)
}

pub fn fn_30047c(m: &mut Memory<'_>, h: &mut impl Host, a: u32) -> u32 {
    let allocator = m.word(a.wrapping_add(0x24));
    let block = m.word(a);
    if block != 0 { crate::boot_part_f::fn_1b2128(m, allocator, block, h); }
    let block = m.word(a.wrapping_add(4));
    if block != 0 { crate::boot_part_f::fn_1b2128(m, allocator, block, h); }
    let mut node = m.word(a.wrapping_add(0x28));
    while node != 0 {
        let next = m.word(node.wrapping_add(0x4c));
        let rc = fn_30a388(m, h, node.wrapping_add(0x20));
        if rc != 0 { return rc; }
        for off in [0x14u32, 0x18, 0x1c] {
            let value = m.word(node.wrapping_add(off));
            if value != 0 { crate::boot_part_f::fn_1b2128(m, allocator, value, h); }
        }
        crate::boot_part_f::fn_1b2128(m, allocator, node, h);
        node = next;
    }
    crate::boot_part_f::fn_1b2128(m, allocator, a, h);
    0
}

pub fn fn_300530(m: &mut Memory<'_>, h: &mut impl Host, a: u32) -> u32 {
    let allocator = m.word(a.wrapping_add(0x24));
    let node = m.word(a.wrapping_add(0x28));
    let mut rc = 0;
    if node != 0 {
        if m.word(node.wrapping_add(0x20)) != 0 {
            rc = fn_301e5c(m, h, allocator, node);
        }
        if rc == 0 { crate::boot_part_f::fn_1b2128(m, allocator, node, h); }
        if rc != 0 { return rc; }
    }
    crate::boot_part_f::fn_1b2128(m, allocator, a, h);
    rc
}

fn fn_301e5c(m: &mut Memory<'_>, h: &mut impl Host, allocator: u32, node: u32) -> u32 {
    if node == 0 { return 0; }
    let next = m.word(node.wrapping_add(0x20));
    let mut rc = 0;
    if next != 0 {
        rc = fn_301e5c(m, h, allocator, next);
        if rc != 0 { return rc; }
    }
    crate::boot_part_f::fn_1b2128(m, allocator, node, h);
    0
}

fn fn_30a388(m: &mut Memory<'_>, h: &mut impl Host, p: u32) -> u32 {
    let pool = m.word(p);
    fn_309fc4(m, h, pool);
    fn_308ed0(m, h, p.wrapping_add(0xc));
    0
}

fn fn_309fc4(m: &mut Memory<'_>, h: &mut impl Host, p: u32) {
    let count = m.word(p.wrapping_add(8));
    if count != 0 {
        let node = p.wrapping_add(count.wrapping_mul(20));
        let left = m.word(node.wrapping_add(8));
        if left != 0 { fn_309ca8(m, h, p, left); }
        let right = m.word(node.wrapping_add(12));
        if right != 0 { fn_309ca8(m, h, p, right); }
        let head = m.word(p.wrapping_add(4));
        m.put_word(node.wrapping_add(8), head);
        m.put_word(p.wrapping_add(4), count);
        m.put_word(p.wrapping_add(4), 0);
    }
    let allocator = m.word(p.wrapping_add(0x10));
    crate::boot_part_f::fn_1b2128(m, allocator, p, h);
}

fn fn_309ca8(m: &mut Memory<'_>, h: &mut impl Host, owner: u32, index: u32) {
    let node = owner.wrapping_add(index.wrapping_mul(20));
    let left = m.word(node.wrapping_add(8));
    if left != 0 { fn_309ca8(m, h, owner, left); }
    let right = m.word(node.wrapping_add(12));
    if right != 0 { fn_309ca8(m, h, owner, right); }
    let head = m.word(owner.wrapping_add(4));
    m.put_word(node.wrapping_add(8), head);
    m.put_word(owner.wrapping_add(4), index);
}

fn fn_308ed0(m: &mut Memory<'_>, h: &mut impl Host, x: u32) {
    let allocator = m.word(x.wrapping_add(0x14));
    if allocator != 0 {
        let block = m.word(x.wrapping_add(4));
        crate::boot_part_f::fn_1b2128(m, allocator, block, h);
    }
    m.put_word(x, 0);
    m.put_word(x.wrapping_add(4), 0);
}

pub fn fn_2b2de8(m: &mut Memory<'_>, h: &mut impl Host, p: u32) {
    let allocator = m.word(p.wrapping_add(0x38));
    if crate::boot_port_1::fn_1ba384(m, allocator, m.word(p.wrapping_add(0x4c)), h) != 0 { return; }
    if fn_2e40d0(m, h, m.word(p.wrapping_add(0x3c))) != 0 { return; }
    let slot = m.word(p.wrapping_add(0x40));
    if slot != 0 {
        crate::boot_part_f::fn_1b2128(m, allocator, slot, h);
    }
    for index in 0..3u32 {
        let slots = m.word(p.wrapping_add(0x44));
        let value = m.word(slots.wrapping_add(index * 4));
        if value != 0 {
            crate::boot_part_f::fn_1b2128(m, allocator, value, h);
        }
    }
    let slots = m.word(p.wrapping_add(0x44));
    crate::boot_part_f::fn_1b2128(m, allocator, slots, h);

    fn_2e4b58(m, h, p);
}

pub fn fn_2b2210(m: &mut Memory<'_>, p: u32, id: u32, out: u32) -> u32 {
    let offset = match id { 0x18b53e23 => 0x50, 0x18b54089 => 0x4c,
        0x482d5ba8 => 0x54, _ => return crate::boot_port_5::fn_2e4bf8(m,p,id,out) };
    m.put_word(out,p.wrapping_add(offset));0
}

pub struct Tables {
    pub dictionary: u32,
    pub collection: u32,
    pub token: u32,
    pub base: u32,
    pub scratch: u32,
    pub vtable: u32,
    pub slot: u32,
}
struct Lane6<'a,H>(&'a mut H);
impl<H:Host> crate::boot_port_6::Host for Lane6<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call(m,f,a)}
}

pub fn fn_2b22b0(m:&mut Memory<'_>,h:&mut impl Host,heap:u32,allocator:u32,methods:u32,entry:u32,context:u32,out:u32,t:&Tables)->u32 {
    let mut status=0;
    let p=crate::boot_part_f::fn_1b2140(m,heap,0x60,&mut status,h);
    if status!=0 {return status as u32;}

    status=fn_2de1bc(m,h,p,allocator,methods,entry,context,t) as u8;
    if status==0 {
        m.put_word(p+0xc,t.dictionary.wrapping_sub(0x24));
        m.put_word(p,t.dictionary.wrapping_sub(0x3c));
        m.put_word(p+0x54,t.dictionary.wrapping_add(0x30));
        m.put_word(p+0x50,t.dictionary.wrapping_add(0x1c));
        m.put_word(p+0x4c,t.dictionary);
        let object=p.wrapping_add(0x48);
        if object==0 {m.put_word(p+0x58,0);} else {
            let saved=m.byte(t.scratch);
            let record=crate::boot_port_1::fn_1b20dc(m,allocator,1,16,t.scratch,h);
            status=m.byte(t.scratch);
            let at=m.offset(t.scratch);m.bytes[at]=saved;
            m.put_word(p+0x58,record);
            if status==0 {m.put_word(record,object);}
        }
        if status==0 {
            let e=crate::boot_part_f::Environment{stream_base_vtable:t.base,stream_vtable:0,vector_vtable:0,compare:0,accept:0};
            status=crate::boot_port_6::fn_2e4598(m,&mut Lane6(h),allocator,allocator,p+0x5c,t.collection,t.token,&e) as u8;
        }
    }
    if status==0 {m.put_word(p+8,heap);m.put_word(out,p);}
    else {crate::boot_part_f::fn_1b2128(m,heap,p,h);m.put_word(out,0);}
    status as u32
}

struct Lane5<'a,H>(&'a mut H);
impl<H:Host> crate::boot_port_5::Host for Lane5<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call(m,f,a)}
}
struct Format<'a,H>(&'a mut H);
impl<H:Host> crate::boot_part_b::Host for Format<'_,H> {
    fn thread_id(&mut self)->u32 {panic!("logging thread service not bound")}
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call(m,f,a)}
}
impl<H:Host> crate::boot_part_b::FormatHost for Format<'_,H> {}
struct Release<'a,H>{host:&'a mut H,base:u32,method:u32}
impl<H:Host> crate::frontend::HeapVtableM for Release<'_,H> {
    fn alloc_at_m(&mut self,_:u32,_:u32,_:&mut[u8])->u32 {panic!("release-only adapter")}
    fn realloc_at_m(&mut self,_:u32,_:u32,_:u32,_:&mut[u8])->u32 {panic!("release-only adapter")}
    fn release_at_m(&mut self,object:u32,p:u32,bytes:&mut[u8]) {
        self.host.call(&mut Memory{base:self.base,bytes},self.method,&[object,p]);
    }
}
fn engine_free(m:&mut Memory<'_>,h:&mut impl Host,heap:u32,p:u32) {
    let method=if heap==0||p==0 {0}else{m.word(m.word(heap)+12)};
    let mut vt=Release{host:h,base:m.base,method};
    crate::frontend::heap_release_m(m.bytes,if heap==0{None}else{Some(heap-m.base)},if p==0{None}else{Some(p)},&mut vt);
}

pub struct Attach {
    pub types: [u32;3],
    pub category: u32,
    pub pair_format: u32,
    pub code_format: u32,
    pub name: u32,
    pub code: u32,
    pub content_type: u32,
    pub scratch: u32,
    pub tables: Tables,
}
fn context(m:&mut Memory<'_>,h:&mut impl Host,a:u32,scratch:u32)->(u32,u32) {
    let saved=m.word(scratch);
    let status=crate::boot_port_1::fn_19611c(m,[m.word(a),m.word(a+4)],scratch,h.context_pair(m),h.default_context(m));
    let ctx=m.word(scratch);m.put_word(scratch,saved);(status,ctx)
}
fn equal(m:&mut Memory<'_>,a:u32,b:u32)->bool {
    crate::selection::arena_leaf::compare_strings(&mut crate::selection::arena::Arena::new(m.base,m.bytes),a,b)==0
}
fn report(m:&mut Memory<'_>,h:&mut impl Host,l:&Attach,ctx:u32,code:u32,kind:u32,value:u32,status:Option<u32>) {
    let va=l.scratch+24;let mut saved=[0u8;16];let at=m.offset(va);saved.copy_from_slice(&m.bytes[at..at+16]);
    m.put_word(va,kind);m.put_word(va+4,value);
    let format=if let Some(status)=status {m.put_word(va+8,l.code);m.put_word(va+12,status);l.code_format}else{l.pair_format};
    let mut params=[0u8;20];let pat=m.offset(l.scratch+40);params.copy_from_slice(&m.bytes[pat..pat+20]);
    crate::render_part_a::error_log(m,&mut Format(h),m.word(ctx+16),l.category,code,format,va,l.scratch+40);
    m.bytes[at..at+16].copy_from_slice(&saved);m.bytes[pat..pat+20].copy_from_slice(&params);
}
fn cleanup(m:&mut Memory<'_>,h:&mut impl Host,a:u32,e:u32,l:&Attach) {
    if m.word(e+32)==0&&m.word(e+36)==0 {return;}
    let t=m.word(e+8);
    let (status,ctx)=context(m,h,a,l.scratch);
    if status>=0x80000000 {return;}
    if equal(m,t,l.types[0]) {
        let p=m.word(e+32);
        if p!=0 {
            fn_2b2de8(m,h,p);
            engine_free(m,h,m.word(ctx+4),m.word(e+32));m.put_word(e+32,0);
        }
    } else if equal(m,t,l.types[1]) {
        let p=m.word(e+36);
        if p!=0 {
            fn_2b1d0c(m,h,p);
            engine_free(m,h,m.word(ctx+4),m.word(e+36));m.put_word(e+36,0);
        }
    } else if !equal(m,t,l.types[2]) {
        report(m,h,l,ctx,0x1134,l.content_type,t,None);
    }
}

pub fn fn_281f34(m:&mut Memory<'_>,h:&mut impl Host,a:u32,e:u32,l:&Attach)->u32 {
    let t=m.word(e+8);
    let (mut status,ctx)=context(m,h,a,l.scratch);
    if status>=0x80000000 {return status;}
    m.put_word(e+28,0);
    let mut clean=false;
    if equal(m,t,l.types[0]) {
        let at=m.offset(l.scratch+8);let mut saved=[0u8;16];saved.copy_from_slice(&m.bytes[at..at+16]);

        status=h.call(m,0x2b4010,&[a+24,a+24,l.scratch+8,a+44,e,a+60,1,e+32]) as u8 as u32;
        m.bytes[at..at+16].copy_from_slice(&saved);
        if status!=0 {
            report(m,h,l,ctx,0x117e,l.name,m.word(e+4),Some(status));
            status=status.wrapping_add(0x1ea0)|0x85002000;m.put_word(e+40,0);clean=status>=0x80000000;
        } else if m.word(e+20)!=0&&m.byte(e+17)==0 {
            report(m,h,l,ctx,0x117f,l.name,m.word(e+4),None);
            status=0x85002019;clean=true;
        } else {
            status=crate::boot_port_5::fn_2b3f04(m,&mut Lane5(h),m.word(e+32),0x18b54089,e+40);
            if status!=0 {
                report(m,h,l,ctx,0x1180,l.name,m.word(e+4),Some(status));
                status=status.wrapping_add(0x1ea0)|0x85002000;m.put_word(e+40,0);clean=status>=0x80000000;
            }
        }
    } else if equal(m,t,l.types[1]) {
        status=fn_2b22b0(m,h,a+24,a+24,a+44,e,0,e+36,&l.tables);
        if status!=0 {
            report(m,h,l,ctx,0x1181,l.name,m.word(e+4),Some(status));
            status=status.wrapping_add(0x1ea0)|0x85002000;m.put_word(e+40,0);clean=status>=0x80000000;
        } else {
            status=fn_2b2210(m,m.word(e+36),0x18b54089,e+40);
            if status!=0 {
                report(m,h,l,ctx,0x1182,l.name,m.word(e+4),Some(status));
                status=status.wrapping_add(0x1ea0)|0x85002000;m.put_word(e+40,0);clean=status>=0x80000000;
            }
        }
    } else if equal(m,t,l.types[2]) {
        m.put_word(e+40,m.word(e+20));
    } else {
        report(m,h,l,ctx,0x1133,l.content_type,t,None);status=0x85002416;clean=true;
    }
    if clean {cleanup(m,h,a,e,l);}
    status
}

pub const STREAM_MAGIC: u32 = 0x9000_0260;
pub const STREAM_VERSION: u32 = 0x9000_0280;
pub const STREAM_SECTION: u32 = 0x9000_02a0;
pub const STREAM_RELEASE: u32 = 0x9000_02c0;
pub const STREAM_READ: u32 = 0x9000_02e0;
pub const STREAM_OPEN: u32 = 0x9000_0300;
pub const STREAM_CALL: u32 = 0x9000_0320;

pub const SECTION_READ: u32 = 0x9000_0340;

#[derive(Clone, Copy)]
pub struct Names {
    pub entry: u32,
    pub first: u32,
    pub second: u32,
}

pub trait StreamHost: Host + crate::boot_port_5c::Host {

    fn section_lookup(&mut self, _m: &mut Memory<'_>, _a: [u32; 7], _scratch: u32) -> u32 {
        crate::normalization::code_4(0)
    }

    fn stream_init(&mut self, _m: &mut Memory<'_>, _a: [u32; 6]) -> u32 {
        crate::normalization::code_4(0)
    }

    fn stream_validate(&mut self, _m: &mut Memory<'_>, _a: [u32; 4]) -> u32 {
        crate::normalization::code_4(0)
    }

    fn handle_check(&mut self, _m: &mut Memory<'_>, _handle: u32) -> u32 {
        0
    }

    fn section_open(&mut self, _m: &mut Memory<'_>, _object: u32) -> u32 {
        0
    }

    fn section_call(&mut self, _m: &mut Memory<'_>, _object: u32, _arg: u32) -> u32 {
        0
    }

    fn section_read(&mut self, _m: &mut Memory<'_>, _a: &[u32]) -> u32 {
        crate::normalization::code_4(0)
    }
}

pub fn fn_1a9a60(
    m: &mut Memory<'_>,
    args: [u32; 7],
    scratch: u32,
    e: &crate::boot_part_e::ArchiveEnvironment,
    h: &mut impl crate::boot_part_e::ArchiveHost,
) -> u32 {
    let p = &e.parameters;
    let streams = crate::boot_port_1::Streams {
        parameters: crate::boot_part_f::Environment {
            stream_base_vtable: p.stream_base_vtable,
            stream_vtable: p.stream_vtable,
            vector_vtable: p.vector_vtable,
            compare: p.compare,
            accept: p.accept,
        },
        plain: 0,
        encoded: 0,
        seek: 0,
        seek_adjustment: 0,
    };
    crate::boot_port_1::fn_1a9a60(
        m,
        args,
        scratch,
        &streams,
        &mut crate::boot_part_e::ArchiveCalls {
            h,
            e,
            wrapper: 0,
        },
    )
}

pub fn fn_1a86a0(
    m: &mut Memory<'_>,
    args: [u32; 4],
    e: &crate::boot_part_e::ArchiveEnvironment,
    h: &mut impl crate::boot_part_e::ArchiveHost,
) -> u32 {
    m.validate_stream(
        &mut crate::boot_part_e::ArchiveCalls {
            h,
            e,
            wrapper: 0,
        },
        args[0],
        args[1],
        args[2],
        args[3],
    )
}

pub fn fn_1a90c0(
    m: &mut Memory<'_>,
    h: &mut impl StreamHost,
    state: u32,
    a: u32,
    b: u32,
    c: u32,
    version: u32,
    flag: u32,
    mode: u32,
    out: u32,
    status: u32,
    flag_out: u32,
) -> u32 {
    let _ = mode;
    let mut rc = 0u8;
    let res = crate::boot_part_f::fn_1b2140(m, state, 84, &mut rc, h);
    if rc != 0 {
        return rc as u32;
    }
    rc = h.stream_init(m, [res, state, a, b, c, mode]) as u8;
    let mut handle = 0;
    if rc == 0 {
        handle = res;
        m.put_word(res.wrapping_add(8), state);
    } else {
        crate::boot_part_f::fn_1b2128(m, state, res, h);
    }
    if rc != 0 {
        return rc as u32;
    }
    m.put_word(out, handle);
    m.put_word(handle.wrapping_add(0x40), version);
    rc = h.stream_validate(m, [handle, flag, status, flag_out]) as u8;
    if rc != 0 {
        return rc as u32;
    }
    if m.byte(flag_out) == 0 {
        return 0;
    }
    let r = h.handle_check(m, handle);
    if r == 0 {
        m.put_word(out, 0);
    }
    r
}

pub fn fn_1a9c50(
    m: &mut Memory<'_>,
    h: &mut impl StreamHost,
    p: u32,
    a1: u32,
    a2: u32,
    out: u32,
    scratch: u32,
) -> u32 {
    let saved = [
        m.word(scratch),
        m.word(scratch.wrapping_add(4)),
        m.word(scratch.wrapping_add(8)),
        m.word(scratch.wrapping_add(12)),
        m.word(scratch.wrapping_add(16)),
        m.word(scratch.wrapping_add(20)),
        m.word(scratch.wrapping_add(24)),
    ];
    let r = fn_1a9c50_frame(m, h, p, a1, a2, out, scratch);
    for (i, v) in saved.into_iter().enumerate() {
        m.put_word(scratch.wrapping_add(i as u32 * 4), v);
    }
    r
}
fn fn_1a9c50_frame(
    m: &mut Memory<'_>,
    h: &mut impl StreamHost,
    p: u32,
    a1: u32,
    a2: u32,
    out: u32,
    scratch: u32,
) -> u32 {
    let rc = h.section_lookup(
        m,
        [
            p,
            a1,
            a2,
            scratch,
            scratch.wrapping_add(4),
            scratch.wrapping_add(8),
            scratch.wrapping_add(12),
        ],
        scratch.wrapping_add(16),
    );
    if rc != 0 {
        return rc;
    }
    if m.byte(scratch.wrapping_add(12)) == 1 {
        return crate::normalization::code_4(0);
    }
    if m.word(scratch.wrapping_add(4)) != u32::MAX {
        return crate::normalization::code_4(0);
    }
    let rc = fn_1a90c0(
        m,
        h,
        m.word(p.wrapping_add(0x44)),
        m.word(p.wrapping_add(0x30)),
        m.word(p.wrapping_add(0x34)),
        m.word(p.wrapping_add(0x38)),
        m.word(scratch.wrapping_add(8)),
        0,
        1,
        scratch.wrapping_add(20),
        scratch.wrapping_add(24),
        scratch.wrapping_add(12),
    );
    if rc != 0 {
        return rc;
    }
    if m.byte(scratch.wrapping_add(12)) == 1 {
        return crate::normalization::code_4(0);
    }
    if m.byte(scratch.wrapping_add(24)) == 1 {
        return crate::normalization::code_4(0);
    }
    m.put_word(out, m.word(scratch.wrapping_add(20)).wrapping_add(16));
    let at = m.offset(p.wrapping_add(0x48));
    m.bytes[at] = 1;
    0
}

pub fn fn_1aa8fc(m: &mut Memory<'_>, h: &mut impl StreamHost, this: u32, p: u32) -> u32 {
    let at = m.offset(this.wrapping_add(0x48));
    m.bytes[at] = 0;
    let mut open = |m: &mut Memory<'_>| {
        let vt = m.word(p);
        let method = m.word(vt.wrapping_add(0x40));
        let adj = m.word(vt.wrapping_add(0x68));
        (method, p.wrapping_sub(adj))
    };
    let (method, object) = open(m);
    let a = match method {
        STREAM_OPEN => h.section_open(m, object),
        other => panic!("unbound section open token {other:x}"),
    };
    let (method, object) = open(m);
    let b = match method {
        STREAM_OPEN => h.section_open(m, object),
        other => panic!("unbound section open token {other:x}"),
    };
    let vt = m.word(b);
    let method = m.word(vt.wrapping_add(0x10));
    match method {
        STREAM_CALL => h.section_call(m, b, a),
        other => panic!("unbound section call token {other:x}"),
    }
}

pub fn fn_1aa3cc(
    m: &mut Memory<'_>,
    h: &mut impl StreamHost,
    p: u32,
    a1: u32,
    a2: u32,
    a3: u32,
    f: [u32; 3],
    flag: u32,
) -> u32 {
    let saved = m.byte(flag);
    let r = fn_1aa3cc_frame(m, h, p, a1, a2, a3, f, flag);
    m.bytes[m.offset(flag)] = saved;
    r
}
fn fn_1aa3cc_frame(
    m: &mut Memory<'_>,
    h: &mut impl StreamHost,
    p: u32,
    a1: u32,
    a2: u32,
    a3: u32,
    f: [u32; 3],
    flag: u32,
) -> u32 {
    let q = m.word(p.wrapping_add(0x14));
    let method = m.word(q.wrapping_add(0x18));
    let base = m.word(q.wrapping_add(0x5c));
    let this = p.wrapping_sub(base).wrapping_add(0x14);
    let rc = match method {
        SECTION_READ => h.section_read(m, &[this, a1, a2, a3, f[0], f[1], f[2], flag]),
        other => panic!("unbound section read token {other:x}"),
    };
    if rc != 0 {
        return rc;
    }
    if m.byte(flag) != 0 {
        return crate::normalization::code_4(0);
    }
    0
}

pub fn fn_2fd9a0(
    m: &mut Memory<'_>,
    h: &mut impl StreamHost,
    t: &crate::boot_port_5c::Tables,
    heap: u32,
    allocator: u32,
    stream: u32,
    out: u32,
) -> u32 {
    crate::boot_port_5c::fn_2fd9a0(m, h, t, heap, allocator, stream, out)
}

pub fn fn_2fac04(
    m: &mut Memory<'_>,
    h: &mut impl StreamHost,
    t: &crate::boot_port_5c::Tables,
    allocator: u32,
    value: u32,
    size: u32,
    out: u32,
) -> u32 {
    crate::boot_port_5c::fn_2fac04(m, h, t, allocator, value, size, out)
}

pub fn fn_302e38(
    m: &mut Memory<'_>,
    h: &mut impl StreamHost,
    t: &crate::boot_port_5c::Tables,
    allocator: u32,
    data: u32,
    out: u32,
) -> u32 {
    crate::boot_port_5c::fn_302e38(m, h, t, allocator, data, out)
}

fn stream_table(m: &Memory<'_>, stream: u32) -> (u32, u32) {
    let vt = m.word(stream);
    let adj = m.word(vt.wrapping_add(0x68));
    (vt, stream.wrapping_sub(adj))
}

pub fn fn_2de614(
    m: &mut Memory<'_>,
    h: &mut impl StreamHost,
    t: &crate::boot_port_5c::Tables,
    obj: u32,
    allocator: u32,
    stream: u32,
    names: &Names,
    scratch: u32,
) -> u32 {
    let e = h.parameters();
    let base = h.base_vtable();
    let interface = h.interface_vtable();
    let ctor = h.ctor_vtable();
    {
        let mut lane = Lane6(&mut *h);
        let rc = crate::boot_port_6::fn_2e4afc(m, &mut lane, obj, allocator, base, interface, &e);
        if rc != 0 {
            return rc;
        }
    }
    m.put_word(obj, ctor);
    m.put_word(obj.wrapping_add(0x34), allocator);
    let (vt, this) = stream_table(m, stream);
    let magic = match m.word(vt.wrapping_add(0x1c)) {
        STREAM_MAGIC => crate::boot_port_1::fn_1aa838(m, this, 0, 0),
        other => panic!("unbound stream magic token {other:x}"),
    };
    if magic != 0x17271bc2 {
        return crate::normalization::code_4(0);
    }
    let version = match m.word(vt.wrapping_add(0x20)) {
        STREAM_VERSION => crate::boot_port_1::fn_1aa840(m, this, 0, 0),
        other => panic!("unbound stream version token {other:x}"),
    };
    if crate::normalization::compare_packed(version, 0x10000) == 0 {
        return 5;
    }
    let rc = h.call(m, STREAM_SECTION, &[this, names.entry, 0, scratch]);
    if rc != 0 {
        return rc;
    }
    let value = m.word(scratch);
    let rc = fn_2fd9a0(
        m,
        h,
        t,
        m.word(obj.wrapping_add(0x34)),
        m.word(obj.wrapping_add(0x34)),
        value,
        obj.wrapping_add(0x1c),
    );
    if rc != 0 {
        return rc;
    }
    let rc = h.call(m, STREAM_RELEASE, &[this, value]);
    if rc != 0 {
        return rc;
    }
    let rc = h.call(
        m,
        STREAM_READ,
        &[
            this,
            m.word(obj.wrapping_add(0x34)),
            names.first,
            0,
            0x10000,
            scratch.wrapping_add(4),
            scratch.wrapping_add(8),
        ],
    );
    if rc != 0 {
        return rc;
    }
    let out1 = m.word(scratch.wrapping_add(4));
    let out2 = m.word(scratch.wrapping_add(8));
    let count = out2 >> 2;
    m.put_word(obj.wrapping_add(0x20), count);
    m.put_word(obj.wrapping_add(0x24), out1);
    m.put_word(obj.wrapping_add(0x28), out1.wrapping_add(count.wrapping_mul(2)));
    let rc = h.call(
        m,
        STREAM_READ,
        &[
            this,
            m.word(obj.wrapping_add(0x34)),
            names.second,
            0,
            0x10000,
            obj.wrapping_add(0x2c),
            scratch.wrapping_add(8),
        ],
    );
    if rc != 0 {
        return rc;
    }
    fn_2fac04(
        m,
        h,
        t,
        m.word(obj.wrapping_add(0x34)),
        m.word(obj.wrapping_add(0x2c)),
        m.word(scratch.wrapping_add(8)),
        obj.wrapping_add(0x30),
    )
}

pub fn fn_2ddfe4(
    m: &mut Memory<'_>,
    h: &mut impl StreamHost,
    t: &crate::boot_port_5c::Tables,
    obj: u32,
    stream: u32,
    names: &Names,
    scratch: u32,
) -> u32 {
    let (vt, this) = stream_table(m, stream);
    let magic = match m.word(vt.wrapping_add(0x1c)) {
        STREAM_MAGIC => crate::boot_port_1::fn_1aa838(m, this, 0, 0),
        other => panic!("unbound stream magic token {other:x}"),
    };
    if magic != 0x17271bc2 {
        return crate::normalization::code_4(0);
    }
    let version = match m.word(vt.wrapping_add(0x20)) {
        STREAM_VERSION => crate::boot_port_1::fn_1aa840(m, this, 0, 0),
        other => panic!("unbound stream version token {other:x}"),
    };
    if crate::normalization::compare_packed(version, 0x10000) == 0 {
        return 5;
    }
    let rc = h.call(m, STREAM_SECTION, &[this, names.entry, 0, scratch]);
    if rc != 0 {
        return rc;
    }
    let value = m.word(scratch);
    let rc = fn_2fd9a0(
        m,
        h,
        t,
        m.word(obj.wrapping_add(0x34)),
        m.word(obj.wrapping_add(0x34)),
        value,
        obj.wrapping_add(0x38),
    );
    if rc != 0 {
        return rc;
    }
    let rc = h.call(m, STREAM_RELEASE, &[this, value]);
    if rc != 0 {
        return rc;
    }
    let rc = h.call(
        m,
        STREAM_READ,
        &[
            this,
            m.word(obj.wrapping_add(0x34)),
            names.first,
            0,
            0x10000,
            obj.wrapping_add(0x3c),
            scratch.wrapping_add(8),
        ],
    );
    if rc != 0 {
        return rc;
    }
    let rc = fn_302e38(
        m,
        h,
        t,
        m.word(obj.wrapping_add(0x34)),
        m.word(obj.wrapping_add(0x3c)),
        obj.wrapping_add(0x40),
    );
    if rc != 0 {
        return rc;
    }
    let rc = h.call(
        m,
        STREAM_READ,
        &[
            this,
            m.word(obj.wrapping_add(0x34)),
            names.second,
            0,
            0x10000,
            scratch.wrapping_add(4),
            scratch.wrapping_add(8),
        ],
    );
    if rc != 0 {
        return rc;
    }
    m.put_word(obj.wrapping_add(0x44), m.word(scratch.wrapping_add(4)));
    let src = m.offset(m.word(m.word(obj.wrapping_add(0x30)).wrapping_add(4)).wrapping_add(18));
    let dst = m.offset(obj.wrapping_add(0x48));
    for i in 0..4 {
        m.bytes[dst + i] = m.bytes[src + i];
    }
    m.bytes[dst + 3] = 0;
    0
}
