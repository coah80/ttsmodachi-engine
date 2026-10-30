//! Live memory-passing hooks for component group a.
//! Original typed entries remain unchanged. Every callback gets the live arena.
use crate::{boot::Memory, boot_part_e, frontend};

/// fn_17b654. Re-read the owner after releasing the extra block, as ARM does.
pub fn release_handle_m(m: &mut Memory<'_>, p: u32, h: &mut impl boot_part_e::Host) -> u32 {
    if p == 0 { return 0x8a402007; }
    let extra = m.word(p + 12);
    if extra != 0 { boot_part_e::release(m, m.word(p), extra, h); }
    boot_part_e::release(m, m.word(p), p, h);
    0
}

/// fn_196830. Resolve the owner through the object's vtable.
pub fn release_owner_m(m: &mut Memory<'_>, p: u32, h: &mut impl boot_part_e::Host) -> u32 {
    if p == 0 { return 0x89002007; }
    boot_part_e::release(m, m.word(m.word(p) + 20), p, h);
    0
}

/// fn_19690c. The allocation callback may change source bytes before the copy.
pub fn clone_str_m(m: &mut Memory<'_>, src: u32, heap: u32, out: u32, h: &mut impl boot_part_e::Host) -> u32 {
    if src == 0 || heap == 0 || out == 0 { return 0x89002007; }
    let size = frontend::strlen_bytes(&m.bytes[m.offset(src)..]).wrapping_add(1);
    let copy = boot_part_e::alloc(m, heap, size, h);
    if copy == 0 { return 0x8900200a; }
    let dst = m.offset(copy) as u32;
    let source = m.offset(src) as u32;
    frontend::copy_str(m.bytes, dst, source);
    m.put_word(out, copy);
    0
}

/// fn_14df44. No arena view remains borrowed across allocation or cleanup.
pub fn clone_labeled_word_m(m: &mut Memory<'_>, input: u32, heap: u32, out: u32, h: &mut impl boot_part_e::Host) -> u32 {
    if input == 0 || heap == 0 || out == 0 { return 0x88f02007; }
    let object = boot_part_e::calloc(m, heap, 1, 8, h);
    if object == 0 { return 0x88f0200a; }
    let second = m.half(input + 4);
    m.put_half(object + 4, second);
    let rc = clone_str_m(m, m.word(input), heap, object, h);
    if rc < 0x80000000 {
        m.put_word(out, object);
    } else {
        boot_part_e::release(m, heap, object, h);
    }
    rc
}

struct LiveHost<'a> { boot: crate::engine::SpeechBootHost<'a>, features: Option<&'a mut FeatureWorkspace>, synth:u32 }
/// Caller stack base for the 0x2a651c synthesis passes (round 3,
/// seg5). The ARM caller keeps a single 64-byte frame
/// (`push {r0-r3,r4-r11,lr}; sub sp,#12`) across the whole
/// 2a8414/2a5578/2a7740 sequence, so its body SP is one constant:
/// 0xe0ff888 (scratch/seg5/probe_2a8414.py measures it at the 2a8414
/// entry; wire-a-r21c-entries.json logs the same 227d64 entry SP
/// 0xe0ff938 on all three calls and the single 2a651c entry SP
/// 0xe0ff8c8 in call #2's window, whose 64-byte frame leaves 0xe0ff888).
/// 2a8414/2a5578/2a7740 execute once per speech_1 run, in call #2
/// (state 1): calls #1/#3 enter 227d64 in states 0/3 (setup/emit) and
/// never reach 0x2a651c, which has exactly one row in the 180k-entry
/// log and one static caller (0x2a6604).
/// The callee frames below land in the captured stack page the seg5
/// test excludes from the image compare, exactly like the ARM's own
/// dirty stack slots: nothing is zeroed or restored in the arms (the
/// ARM never zeroes its frame; the shared ports init what the C inits
/// and keep the fixture-pinned save/restore inside themselves).
/// Frame base for fn_227d64_m's 284-byte port scratch (round 12, seg5).
/// The ARM keeps these C-stack locals in its own frames below the
/// scheduler's SP; the port threads them as one explicit window instead.
/// Measured: the scheduler calls 227d64 (and every other root on every
/// workload) with SP 0xe0ff938 (scratch/seg1+bounds-*.json; stack args at
/// [sp] in all three seg5 entry captures), and the ARM's own frames reach
/// down to 0xe0ff6e0 (deepest stack write across calls 1-3). 0xe0fe200 sits
/// 0x1ae4 below the caller SP in the ARM's stack red zone: same excluded
/// stack page, ARM-untouched zeros on entry and return (0 diffs, all three
/// calls), clear of every fixed stack scratch in the tree (2a762c +0x0,
/// seg3 slots +0x100, 0xe0feb00, 0xe0fed00, 0xe0fee00, 0xe0fef00/0xf20/0xf80).
/// All slots are transient out-words/workspace; no guest pointer persists.
/// Replaces the 0x1000 disjoint window, which had no ARM counterpart.
pub const SCRATCH_227D64: u32 = 0xe0fe200;
const SYN_CALLER_SP: u32 = 0xe0ff888;
impl crate::selection::arena::Host for LiveHost<'_> {
    fn call(&mut self,a:&mut crate::selection::arena::Arena<'_>,f:u32,v:&[u32])->(u32,u32) {
        let mut m=Memory{base:a.base,bytes:a.bytes};
        // fn_176fd4 with no out-word (the selection stage's
        // fresh-null-pair convention, c3 r30): return the live
        // D_8d7660 words as the pair. 1:1 with src/fn_176fd4.c
        // (ldm r2,{r1,r2} / stm r0,{r1,r2}).
        if f==0x176fd4 && v.is_empty() {
            return (m.word(0x8d7660),m.word(0x8d7664));
        }
        let rc=match f {
            0x176f94=>frontend::validate_handle(v[0]!=0,v[1],v[2],v[3]),
            0x2902d8=>{let w=self.features.take().expect("nonrecursive feature binding");let rc=features_binding(&mut m,v[0],v[1],v[2],w,self);self.features=Some(w);rc},
            0x17c550=>crate::pronunciation::parse_int(&m.bytes[m.offset(v[0])..]) as u32,
            0x24197c=>resource_lookup_binding(&mut m,v[0],v[1],v[2]),
            0x17b824=>frontend::deref_slot2(m.bytes,(v[0]!=0).then_some(v[0])),
            0x1985fc=>frontend::begin_stage_hook((v[0]!=0).then_some(v[0])),
            0x19860c=>frontend::end_stage_hook((v[0]!=0).then_some(v[0])),
            0x173fd8=>max_slot_level_m(&mut m,v[0],self),
            0x2db19c=>{
                // The fixed callers above pass the C ROM name directly
                // (round 1, seg5); otherwise resolve the heap name from
                // the state offset, mirroring the certified
                // speech_part_7 Engine map (0x24 is the model workspace;
                // anything else keeps the VStream fallback).
                let name=if v.len()>=3 && v[2]!=u32::MAX {v[2]} else {heap_name(self.synth,v[0])};
                pool_binding(&mut m,f,&[v[0],v[1],name],&mut self.boot)},
            0x2db1d0|0x2db308|0x2db4a0|0x2db3c8=>pool_binding(&mut m,f,v,&mut self.boot),
            0x2f7f90=>duration_pair_binding(&mut m,v),
            0x17b500=>create_handle_m(&mut m,v[0],v[1],&mut self.boot),
            0x17b53c|0x17b9d4=>{let heap=m.word(v[0]);let mut h=HeapAdapter(&mut self.boot,heap);if f==0x17b53c {crate::render_part_b::fn_17b53c_m(&mut m,v[0],v[1] as i32,&mut h)}else{crate::speech_part_1::fn_17b9d4(&mut m,v[0],v[1],&mut h)}},
            0x17b654=>release_handle_m(&mut m,v[0],&mut self.boot),
            0x2a4740=>{
                self.synth=v[1];
                let blocks: &[u8;0x104]=m.bytes[v[2] as usize..v[2] as usize+0x104].try_into().unwrap();
                let frame=crate::selection::initialize_frame(v[0],blocks);
                let owner=frame.owner;let version=frame.version;
                m.bytes[v[1] as usize..v[1] as usize+0x3aa4].fill(0);
                m.put_word(v[1]+0x18,owner);m.put_half(v[1]+0x1c,version);
                for (i,off) in [0x294,0x2dc,0x324,0x36c,0x3b4].into_iter().enumerate(){m.put_word(v[1]+off,v[2]+i as u32*0x34);}0
            },
            0x2a4510=>crate::selection::validate_window_counts(m.word(m.word(v[1]+0x2dc)+24) as i32,m.word(v[2]+0x1fc) as i32,
                m.word(m.word(v[1]+0x294)+24) as i32,m.word(v[2]+0x204) as i32).unwrap_or_else(|_|0x84e02004),
            0x15140c=>{
                let valid=crate::normalization::quad_check(v[0],v[1],0x47de,12);
                if valid>=0x80000000 {return (valid,0);}
                let state=m.word(v[0]+4);
                let position=frontend::StreamPosition{kind:m.byte(v[0]),read:m.word(v[0]+8),written:m.word(state+44),eof:m.byte(state+68)};
                let mut n=0;let mut done=0;
                let rc=position.remaining(valid,Some(&mut n),Some(&mut done));
                m.put_word(v[2],n);if v[3]!=0 {m.put_byte(v[3],done);}rc
            },
            0x15017c=>{
                // Stream close with the live heap (round 27c, wire-a):
                // the destructor releases through the live dispatcher
                // like component_live_b's stream_call_m, instead of
                // panicking. Certified paths never reach the destructor,
                // so their images are unchanged.
                let state=m.word(v[0].wrapping_add(4));
                let heap=m.word(m.word(state.wrapping_add(4)).wrapping_add(4));
                let cell=core::cell::RefCell::new(&mut *self);
                let mut vt=LiveStreamHeap{cell:&cell,heap};
                let mut destroy=|p:u32,heap:u32,mem:&mut [u8]|{
                    let f=crate::frontend::mem_word(mem,state.wrapping_add(36));
                    let mut h=cell.borrow_mut();
                    crate::reentrant::HooksM::call_m(&mut **h,f,&[p,heap],&mut Memory{base:0,bytes:mem});
                };
                crate::component_live_b::read_direct_end_m(m.bytes,Some(v[0]),v[1],v[2],&mut vt,&mut destroy)
            },
            0x14fe9c=>frontend::read_direct_start(m.bytes,Some(v[0]),v[1],Some(v[2]),Some(v[3])),
            // fn_2d9dc0 (round 1, seg5): the decision-tree walk the
            // synthesis passes run. Pure guest reads plus the
            // caller-owned decision-node workspace; no host calls.
            0x2d9dc0=>{
                let mut nodes=[crate::selection::DecisionNode{question:0,miss:0,hit:0};65536];
                let mut w=crate::speech_part_3::FilterWorkspace{nodes:&mut nodes,coefficients:&mut [],input:&mut [],output:&mut []};
                crate::speech_part_3::sp4_tree(&m,v[0],v[1],v[2],v[3],v[4],&mut w)
            },
            // fn_2f7420 / fn_2f6f94 (round 1, seg5): the indexed and
            // untimed acoustic-model reads. Views are built from the
            // guest tables exactly like the speech_part_5 voice
            // fixture; resident blocks serve without the loader, and
            // the loader fails closed (a miss returns an error
            // record, which the seg5 0-diff certifies never happens
            // on these states).
            0x2f7420=>voice_indexed(&mut m,v),
            0x2f6f94=>voice_untimed(&mut m,v),
            // fn_2f86e0 (round 1, seg5): the outer acoustic-table read.
            // The table view mirrors sp4_acoustic; a cold table fails
            // closed like the voice loader above.
            0x2f86e0=>voice_outer(&mut m,v),
            // fn_2a38d8 (round 1, seg5): the wide smoother. Mirrors
            // the speech_part_7 test adapter; Rust scratch mirrors the
            // C's guest buffers exactly.
            0x2a38d8=>{
                let p=v[1];
                let mode=m.word(p+28) as i32;
                let n=m.word(p+32) as usize;
                if mode!=0 || n==0 {0} else {
                    assert!(n+4<=4096,"live smooth window");
                    let samples=m.word(m.word(p+48));
                    let output=m.word(p+44);
                    let mut x=[0i32;4096];
                    for i in 0..n+4 {x[i]=m.word(samples.wrapping_sub(8).wrapping_add(i as u32*4)) as i32;}
                    let middle=if n>1 {m.word(m.word(p+48)+4)} else {samples};
                    let mut y=[0i32;4096];
                    for i in 0..n {y[i]=m.word(middle.wrapping_add(i as u32*4)) as i32;}
                    let mut out=[0i32;4096];
                    crate::selection::smooth_features(mode,&mut x[..n+4],&y[..n],n,&mut out[..n])
                        .expect("live smooth");
                    for (i,&val) in x[..n+4].iter().enumerate() {
                        m.put_word(samples.wrapping_sub(8).wrapping_add(i as u32*4),val as u32);
                    }
                    for (i,&val) in out[..n].iter().enumerate() {
                        m.put_word(output.wrapping_add(i as u32*4),val as u32);
                    }
                    0
                }
            },
            0x2a76a4=>{
                // fn_2a76a4 (round 1, seg5): the workspace row-pointer
                // fan-out. Heap traffic runs on v[0] through the heap
                // adapter, like the C's fn_196394/fn_196428 calls.
                // v[0] here is the allocator heap, not the synth
                // struct, so it must not re-anchor the pool-name map.
                let heap=v[0];
                let mut hadapter=HeapAdapter(&mut self.boot,heap);
                crate::speech_part_3::fn_2a76a4(&mut m,v[0],v[1],v[2],v[3],v[4],&mut hadapter)
            },
            // fn_2a24f8 (round 1, seg5): the normal-equation assembly
            // the synthesis passes run. Lattice hooks ride the pure
            // ports below; the Cholesky/dictionary/logger paths fail
            // closed and never fire on these states (certified by the
            // seg5 0-diff).
            0x2a24f8=>{crate::speech_part_2::fn_2a24f8(&mut m,v[0],v[1],self);0},
            0x2a7740=>{
                // fn_2a7740 (round 3, seg5): the paired spectral pass.
                // output is the C's 12-byte ModelOutput frame at the
                // ARM's own slot: `sub sp,#64` past the 48-byte push
                // puts the frame base at own-sp+16, i.e. caller-SP-96
                // (values at +4, variances at +8, exactly like the C).
                // v[0] is the synth struct (pool-name anchor, see above).
                self.synth=v[0];
                let weights:[i32;14]=core::array::from_fn(|i|m.word(0x91728c+i as u32*4) as i32);
                crate::speech_part_7::fn_2a7740(a,self,v[0],v[1],v[2],SYN_CALLER_SP-96,&weights)
            },
            0x2a5578=>{
                // fn_2a5578 (round 3, seg5): pitch trajectory
                // construction. Workspaces mirror the certified
                // speech_part_3 fixture shape, sized up for the live
                // voice data. The port addresses its 12-byte
                // FrameParameters at scratch+256; the ARM's own slot
                // is `sub sp,#120` past the 48-byte push, i.e.
                // own-sp+60 = caller-SP-108, so scratch sits 256 below
                // that, in the same captured stack page. v[0] is the
                // synth struct (pool-name anchor, see above).
                self.synth=v[0];
                let scratch=SYN_CALLER_SP-108-256;
                let mut nodes=[crate::selection::DecisionNode{question:0,miss:0,hit:0};65536];
                let mut coef=[0i32;16384];
                let mut input=[0i32;4096];
                let mut output=[0i32;4096];
                let mut w=crate::speech_part_3::FilterWorkspace{nodes:&mut nodes,
                    coefficients:&mut coef,input:&mut input,output:&mut output};
                crate::speech_part_3::fn_2a5578(&mut m,v[0],v[1],v[2],scratch,&mut w,self)
            },
            // fn_2a4104 (round 1, seg5): the row-pointer table build the
            // gain/input table setup calls. The two 0x2db1d0 builds ride
            // the pool arm above; no extra workspace is needed.
            0x2a4104=>crate::selection::arena_mid::allocate_table(a,self,v[0],v[1] as i32,v[2] as i32,v[3]),
            0x2a8414=>{
                // fn_2a8414 (round 3, seg5): the forward synthesis pass.
                // params is the C's 12-byte Parameters frame at the
                // ARM's own slot: `sub sp,#112` past the 48-byte push
                // puts the frame base at own-sp+80, i.e. caller-SP-80
                // (values at +4, scales at +8; the C never reads ctx+4
                // here, so no heap is allocated for the frame itself).
                // Buffer allocs still go through the live guest heap
                // (0x1963b8/0x196428 on the owner allocator), exactly
                // like the ARM. weights are the eight D_91726c words
                // (US ROM, mapped in the live arena).
                self.synth=v[0];
                let weights:[i32;8]=core::array::from_fn(|i|m.word(0x91726c+i as u32*4) as i32);
                crate::speech_part_7::fn_2a8414(a,self,v[0],v[1],v[2],SYN_CALLER_SP-80,&weights)
            },
            // Anything else rides the boot dispatcher first (round 1,
            // seg5); what the boot dispatcher refuses rides the live
            // engine dispatcher the way Seg5Host does. A live refusal
            // still faults loudly with the entry and args below.
            _=>{
                let rc=self.boot.invoke(&mut m,f,v);
                if self.boot.fault.is_none() {rc} else {
                    self.boot.fault=None;
                    match crate::engine_live::call_live_m(&mut m,f,v,&mut *self.boot.platform) {
                        Ok(rc)=>rc,
                        Err(e)=>{self.boot.fault=Some(e);0x80002001}
                    }
                }
            },
        };if let Some(ref e)=self.boot.fault {panic!("unwired call {f:x} args {v:x?}: {e:?}");}(rc,0)
    }
}
/// First live US SELECT_BET2 Process return, certified from component startup.
/// Reuses selection::stage::process_stage without changing its body or Host API.
/// Memory must be flat (base zero). `scratch` must be the SP-relative window
/// [`SCRATCH_227D64`] (284 bytes: pipe out-words, model-step/frame flags,
/// feature-setup, duration and mean/variance slots); the
/// diagnostic adapter temporarily borrows and restores 24 bytes at 0x800.
/// Nodes must cover the largest reachable duration tree. The live fixture uses
/// 65536 slots. FeatureWorkspace is caller-owned and allocation-free.
/// Cold duration loads, voice callbacks, active feature dumps and stream-buffer
/// destruction are outside this startup capture and fail closed, never replay.
/// This is not certification of later Process states or other regional literals.
pub fn fn_227d64_m(m:&mut Memory<'_>,args:[u32;7],scratch:u32,nodes:&mut [crate::selection::DecisionNode],
    platform:&mut dyn crate::engine::HeapPlatform,features:&mut FeatureWorkspace)->Result<u32,crate::engine::Error> {
    assert_eq!(m.base,0,"flat live arena required");
    // Anchor the pool-name map (round 1, seg5): the synth struct the
    // roots work on persists across scheduler roots at engine+0x7d8
    // (selection ENG_SYNTH); state 0 roots allocate it fresh and the
    // arms below re-anchor as its children run. Without this, roots
    // that create pools before any synth-root arm runs (227d64 #3)
    // would resolve SENTINEL names against synth 0.
    let mut h=LiveHost{boot:crate::engine::SpeechBootHost{platform,fault:None},features:Some(features),synth:m.word(args[0].wrapping_add(0x7d8))};
    let mut a=crate::selection::arena::Arena::new(m.base,m.bytes);
    let rc=crate::selection::stage::process_stage(&mut a,&mut h,args[0],args[1],args[2],args[3],args[4],scratch,nodes);
    if let Some(e)=h.boot.fault {Err(e)}else{Ok(rc)}
}
/// Live-dispatcher forwarding for the synthesis bridge (round 1,
/// seg5). Inline-handled lattice entries never reach here; a nested
/// boundary root is a real stop and fails loudly, everything else
/// returns the live result or 0x80002001.
fn live_forward_m(m:&mut Memory<'_>,f:u32,a:&[u32],h:&mut LiveHost<'_>)->u32 {
    let platform:&mut dyn crate::engine::HeapPlatform=&mut *h.boot.platform;
    match crate::engine_live::call_live_m(m,f,a,platform) {
        Ok(rc)=>rc,
        Err(crate::engine::Error::UnsupportedStage{entry})=>panic!("synthesis reached boundary {entry:#x}"),
        Err(_)=>0x80002001,
    }
}
/// Heap-name resolution for SENTINEL-named 0x2db19c pool creation
/// (round 1, seg5): the state offset within the synth struct selects
/// the C ROM name, mirroring the certified speech_part_7 Engine map
/// (0x24 is the model workspace; anything else keeps the VStream
/// fallback). Both the live 0x2db19c arm and pool_name below share it.
fn heap_name(synth: u32, state: u32) -> u32 {
    match state.wrapping_sub(synth) {
        0x24=>0x2a0e30,0x26c=>0x91725b,0x2b4=>0x91724d,0x344=>0x917254,
        0x3d4=>0x2a3340,0x3ec=>0x2a334c,0x498=>0x917246,0x4e0=>0x917240,
        _=>0x917263
    }
}
impl crate::reentrant::HooksM for LiveHost<'_> {
    fn call_m(&mut self,address:u32,args:&[u32],mem:&mut Memory<'_>)->u32 {
        match address {
            // Pure lattice arithmetic; no host state.
            0x29b570=>crate::selection::prediction_square_root(args[0]) as u32,
            0x30209c=>crate::component_live_b::fn_30209c_m(args[0] as i32,args[1] as i32) as u32,
            _=>live_forward_m(mem,address,args,self),
        }
    }
}
impl crate::speech_part_2::Host for LiveHost<'_> {
    fn call(&mut self,memory:&mut Memory<'_>,method:u32,args:&[u32])->u32 {
        live_forward_m(memory,method,args,self)
    }
}
impl crate::boot_part_b::Host for LiveHost<'_> {
    fn call(&mut self,memory:&mut Memory<'_>,function:u32,args:&[u32])->u32 {
        live_forward_m(memory,function,args,self)
    }
    fn thread_id(&mut self)->u32 {self.boot.platform.thread_id()}
}
impl crate::boot_part_b::FormatHost for LiveHost<'_> {}
impl crate::boot_part_f::Host for LiveHost<'_> {
    fn call(&mut self,m:&mut Memory<'_>,method:u32,args:&[u32])->u32 {
        live_forward_m(m,method,args,self)
    }
    fn compare(&mut self,_:&mut Memory<'_>,_:u32,_:&mut crate::boot_part_f::Key,_:&mut u32)->u32 {
        // Dictionary comparison never fires on the synthesis success path (round 1, seg5).
        0x80002001
    }
}
impl crate::log_live::LogMethodsM for LiveHost<'_> {
    fn log_m(&mut self,method:u32,object:u32,category:u32,level:u32,arg:u32,
        format:u32,varargs:u32,m:&mut Memory<'_>)->u32 {
        live_forward_m(m,method,&[object,category,level,arg,format,varargs],self)
    }
}
impl crate::frontend::HeapVtableM for LiveHost<'_> {
    fn alloc_at_m(&mut self,_obj:u32,_size:u32,_mem:&mut [u8])->u32 {
        panic!("HeapVtableM needs a per-heap HeapAt wrapper (round 1, seg5)")
    }
    fn realloc_at_m(&mut self,_obj:u32,_old:u32,_size:u32,_mem:&mut [u8])->u32 {
        panic!("HeapVtableM needs a per-heap HeapAt wrapper (round 1, seg5)")
    }
    fn release_at_m(&mut self,obj:u32,object:u32,mem:&mut [u8]) {
        // Slot 3 is the release method in the shared heap layout
        // (round 1, seg5); unlike alloc, it is consistent across the
        // heaps this path touches. A miss fails loudly below.
        let mut m=Memory{base:0,bytes:mem};
        let f=m.word(m.word(obj)+12);
        live_forward_m(&mut m,f,&[obj,object],self);
    }
}
impl crate::speech_part_3::SpeechHost for LiveHost<'_> {}
impl crate::speech_part_3::SpeechRound4Host for LiveHost<'_> {
    type RegexHost=crate::component_live_b2::PcreGuest;
    // The C passes ROM addresses and immediates directly; echo them back
    // (round 1, seg5). This supplies filter pool names like D_91724d.
    fn literal(&self,address:u32)->u32 {address}
    fn pool_name(&self, p: u32) -> u32 { heap_name(self.synth, p) }
    fn regex_exec(&mut self,_:&mut Memory<'_>,_:&[u32],_:&mut [i32;100],_:crate::speech_part_3::RegexEntry<Self::RegexHost>,_:fn(&mut Memory<'_>,u32,&mut Self)->u32)->i32 {
        // No synthesis path on these states reaches the regex engine (round 1, seg5).
        -1
    }
}
impl crate::speech_part_5::Host for LiveHost<'_> {
    fn scratch(&self)->u32 {0x500}
    fn format(&self,callsite:u32)->u32 {callsite}
    fn dictionary_entries(&mut self,_:&mut Memory<'_>,_:u32,_:&mut u32,_:&mut u32)->u32 {0x80002001}
    fn record(&mut self,_:&mut Memory<'_>,_:u32,_:u32,_:&mut crate::speech_part_5::RecordBuffer,_:&mut u8)->u32 {0x80002001}
    fn entries(&mut self,_:&mut Memory<'_>,_:u32,_:u32,_:u32,_:&mut u32,_:&mut u32)->u32 {0x80002001}
    fn thread_id(&mut self)->u32 {self.boot.platform.thread_id()}
}
/// fn_2f7420 for a live host: builds the quantized voice views from the
/// guest tables (speech, codebook set) like the speech_part_5 voice
/// fixture and serves the indexed pair. See the 0x2f7420 arm above.
fn voice_indexed(m:&mut Memory<'_>,v:&[u32])->u32 {
    use crate::selection::{AcousticRecord,Codebook,PdfParameter,QuantizedPdfTable,VoiceBlock};
    let (sp,set)=(v[1],v[2]);
    let books=m.word(set+8);
    let blockp=m.word(sp+0xac);
    let block=v[3] as usize;
    assert!(block<8,"live voice block window");
    let channels=m.half(sp+8) as u32;
    let stride=m.half(sp+12) as u32;
    // The codebook array has no stored count; size the view to cover
    // exactly what this call decodes (the ARM indexes the same range).
    let nbooks=(channels.wrapping_mul(stride).wrapping_add(stride.wrapping_mul(v[7]))
        .wrapping_add(v[5])+1) as usize;
    assert!(nbooks<=64,"live voice codebook window");
    let dummy_param=PdfParameter{bias:0,bias_bits:0,sample_bits:0,multiplier:0};
    let params:[[PdfParameter;12];64]=core::array::from_fn(|i|{if i>=nbooks {return [dummy_param;12];}
        let p=books+i as u32*112;
        core::array::from_fn(|j|{let j=j as u32;PdfParameter{bias:m.word(p+60+j*4) as i32,
            bias_bits:m.half(p+56),sample_bits:m.half(p+8+j*2),multiplier:m.half(p+32+j*2)}})});
    let blocks:[VoiceBlock;8]=core::array::from_fn(|i|{let base=m.word(blockp+i as u32*8);
        let off=m.word(blockp+i as u32*8+4);
        let resident=base!=0&&(base as usize)<m.bytes.len();
        VoiceBlock{data:if resident{Some(&m.bytes[base as usize..])}else{None},data_origin:0,offset:off}});
    let codebooks:[Codebook;64]=core::array::from_fn(|i|{if i>=nbooks {
            return Codebook{dimensions:0,width:0,parameters:&params[0],data:&[],data_origin:0};}
        let p=books+i as u32*112;
        let doff=m.word(p+108) as usize;
        Codebook{dimensions:m.half(p),width:m.half(p+4),parameters:&params[i],
            data:if doff<m.bytes.len(){&m.bytes[doff..]}else{&[]},data_origin:0}});
    let quant=QuantizedPdfTable{dimensions:m.half(sp+4),extra:m.half(sp+6),channels:m.half(sp+8),
        stride:m.half(sp+12),blocks:&blocks[..block+1],codebooks:&codebooks[..nbooks]};
    let load=|_,_|AcousticRecord{status:0x84e0200a,bytes:&[]};
    let mut out=[0i32;2];
    let rc=quant.read_pair(v[3],v[4],v[5],v[6] as usize,v[7],v[10],&mut out,load)
        .expect("live voice record decodes");
    if rc==0 {m.put_word(v[8],out[0] as u32);m.put_word(v[9],out[1] as u32);}
    rc
}
/// fn_2f6f94 for a live host: the untimed acoustic-model read over the
/// same guest views. See the 0x2f6f94 arm above.
fn voice_untimed(m:&mut Memory<'_>,v:&[u32])->u32 {
    use crate::selection::{AcousticRecord,Codebook,PdfParameter,PdfTable,VoiceBlock};
    let sp=v[1];
    let blockp=m.word(sp+0xac);
    let block=v[2] as usize;
    assert!(block<8,"live voice block window");
    let params:[PdfParameter;6]=core::array::from_fn(|i|{let p=m.word(sp+0xa4)+i as u32*200;
        PdfParameter{bias:m.word(p+4) as i32,bias_bits:m.half(p+100),
            sample_bits:m.half(p+102),multiplier:m.half(p+150)}});
    let blocks:[VoiceBlock;8]=core::array::from_fn(|i|{let base=m.word(blockp+i as u32*8);
        let off=m.word(blockp+i as u32*8+4);
        let resident=base!=0&&(base as usize)<m.bytes.len();
        VoiceBlock{data:if resident{Some(&m.bytes[base as usize..])}else{None},data_origin:0,offset:off}});
    let pdf=PdfTable{width:m.half(sp+4),extra:m.half(sp+6),dimensions:m.half(sp+8),
        parameters:&params,blocks:&blocks[..block+1]};
    let load=|_,_|AcousticRecord{status:0x84e0200a,bytes:&[]};
    let (mut o0,mut o1)=(0i32,0i32);
    let second=if v[6]!=0 {Some(&mut o1)} else {None};
    let rc=pdf.read_pair(v[2],v[3],v[4],v[7],&mut o0,second,load)
        .expect("live voice record decodes");
    if rc==0 {m.put_word(v[5],o0 as u32);if v[6]!=0 {m.put_word(v[6],o1 as u32);}}
    rc
}
/// fn_2f86e0 for a live host: the outer acoustic-table read over the
/// guest engine table. See the 0x2f86e0 arm above.
fn voice_outer(m:&mut Memory<'_>,v:&[u32])->u32 {
    use crate::selection::{AcousticRecord,AcousticTable};
    let t=v[1];
    let block=v[2] as usize;
    let row=v[3];
    let column=v[4] as usize;
    let precision=v[7];
    let records=m.word(t+116);
    let rec_base=m.word(records+block as u32*4);
    let data_base=m.word(t+112);
    let resident=data_base!=0;
    let base=rec_base;
    let mut shifts=[[0u16;3];3];
    let mut scales=shifts;
    let mut offsets=[[0i32;3];3];
    for i in 0..3 {for j in 0..3 {
        shifts[i][j]=m.half(t+32+(i*3+j) as u32*2);
        scales[i][j]=m.half(t+50+(i*3+j) as u32*2);
        offsets[i][j]=m.word(t+68+(i*3+j) as u32*4) as i32;
    }}
    let resident=resident && (data_base as usize)<m.bytes.len();
    let data=if resident {Some(&m.bytes[m.offset(data_base)..])} else {None};
    let blocks=[base];
    let table=AcousticTable{width:m.word(t+4),record_size:m.half(t+16),
        shifts,scales,offsets,offset_shift:m.half(t+104),data,data_origin:0,blocks:&blocks};
    let load=|_,_|AcousticRecord{status:0x84e0200a,bytes:&[]};
    let mut out=[0i32;2];
    let rc=table.read_outer(0,row,column,precision,&mut out,load)
        .expect("live outer record decodes");
    if rc==0 {m.put_word(v[5],out[0] as u32);m.put_word(v[6],out[1] as u32);}
    rc
}
/// Live-heap vtable for the 0x15017c close (round 27c, wire-a): heap
/// methods ride the live dispatcher through the arena host, the way
/// component_live_b's StreamHeap does for its stream calls (the boot
/// dispatcher cannot serve them). The destructor below resolves through
/// the same host.
struct LiveStreamHeap<'a, 'h, H> {
    cell: &'a core::cell::RefCell<&'h mut H>,
    heap: u32,
}
impl<H: crate::reentrant::HooksM> crate::frontend::HeapVtableM for LiveStreamHeap<'_, '_, H> {
    fn alloc_at_m(&mut self, obj: u32, n: u32, mem: &mut [u8]) -> u32 {
        let f = crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap));
        let mut h = self.cell.borrow_mut();
        h.call_m(f, &[obj, n], &mut Memory { base: 0, bytes: mem })
    }
    fn realloc_at_m(&mut self, obj: u32, p: u32, n: u32, mem: &mut [u8]) -> u32 {
        let f = crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap) + 8);
        let mut h = self.cell.borrow_mut();
        h.call_m(f, &[obj, p, n], &mut Memory { base: 0, bytes: mem })
    }
    fn release_at_m(&mut self, obj: u32, p: u32, mem: &mut [u8]) {
        let f = crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap) + 12);
        let mut h = self.cell.borrow_mut();
        h.call_m(f, &[obj, p], &mut Memory { base: 0, bytes: mem });
    }
}
struct HeapAdapter<'a,'b>(&'a mut crate::engine::SpeechBootHost<'b>,u32);
impl frontend::HeapVtableM for HeapAdapter<'_,'_> {
    fn alloc_at_m(&mut self,obj:u32,size:u32,bytes:&mut [u8])->u32 {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.1));let r=self.0.invoke(&mut m,f,&[obj,size]);assert!(self.0.fault.is_none(),"heap {:?}",self.0.fault);r
    }
    fn realloc_at_m(&mut self,obj:u32,old:u32,size:u32,bytes:&mut [u8])->u32 {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.1)+8);self.0.invoke(&mut m,f,&[obj,old,size])
    }
    fn release_at_m(&mut self,obj:u32,p:u32,bytes:&mut [u8]) {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.1)+12);self.0.invoke(&mut m,f,&[obj,p]);
    }
}
/// fn_17b500. Only the allocator callback interface changes.
pub fn create_handle_m(m:&mut Memory<'_>,heap:u32,out:u32,h:&mut impl boot_part_e::Host)->u32 {
    if out==0 {return 0x8a402007;}
    let p=boot_part_e::calloc(m,heap,1,16,h);
    m.put_word(out,p);
    if p!=0 {m.put_word(p,heap);0}else{0x8a40200a}
}
struct PoolAdapter<'a,'b,'c>{m:Memory<'a>,h:&'b mut crate::engine::SpeechBootHost<'c>,name:u32,label:u32}
impl crate::selection::PoolHost for PoolAdapter<'_,'_,'_> {
    fn block(&self,i:usize)->crate::selection::PoolBlock {
        let p=i as u32;let n=self.m.word(p);
        crate::selection::PoolBlock{next:(n!=0).then_some(n as usize),base:self.m.word(p+4),current:self.m.word(p+8),end:self.m.word(p+12)}
    }
    fn set_block(&mut self,i:usize,b:crate::selection::PoolBlock){
        let p=i as u32;for (i,v) in [b.next.unwrap_or(0) as u32,b.base,b.current,b.end].into_iter().enumerate(){self.m.put_word(p+i as u32*4,v);}
    }
    fn allocate(&mut self,owner:u32,n:u32)->Option<(usize,u32)> {
        let heap=self.m.word(owner+4);let p=boot_part_e::calloc(&mut self.m,heap,1,n,self.h);
        (p!=0).then_some((p as usize,p))
    }
    fn free(&mut self,owner:u32,i:usize){let heap=self.m.word(owner+4);boot_part_e::release(&mut self.m,heap,i as u32,self.h);}
    fn clear(&mut self,p:u32,n:u32){let at=self.m.offset(p);self.m.bytes[at..at+n as usize].fill(0);}
    fn report(&mut self,p:&crate::selection::PrivateHeap<'_>,_:&[u8],blocks:u32) {
        let logger=self.m.word(p.owner+16);
        let mut saved=[0u32;6];for (i,x) in saved.iter_mut().enumerate(){*x=self.m.word(0x800+i as u32*4);}
        for (i,x) in [self.name,self.label,p.allocated,p.pool,p.expected,blocks].into_iter().enumerate(){self.m.put_word(0x800+i as u32*4,x);}
        self.h.invoke(&mut self.m,0x1738e8,&[logger,0x917328,3,0,0x2db384,0x800]);
        for (i,x) in saved.into_iter().enumerate(){self.m.put_word(0x800+i as u32*4,x);}
    }
}
fn pool_binding(m:&mut Memory<'_>,f:u32,a:&[u32],h:&mut crate::engine::SpeechBootHost<'_>)->u32 {
    let p=a[0];
    if f==0x2db19c {
        let x=crate::selection::PrivateHeap::initialize(a[1],b"");
        // Distinct names for feature storage and the model workspace.
        let name=a[2];
        for (i,v) in [x.owner,name,0,x.expected,x.pool,x.allocated].into_iter().enumerate(){m.put_word(p+i as u32*4,v);}
        return 0;
    }
    if p==0 {return 0;}
    let first=m.word(p+8);
    let mut x=crate::selection::PrivateHeap{owner:m.word(p),name:b"",first_block:(first!=0).then_some(first as usize),expected:m.word(p+12),pool:m.word(p+16),allocated:m.word(p+20)};
    let name=m.word(p+4);let mut host=PoolAdapter{m:Memory{base:m.base,bytes:m.bytes},h,name,label:if f==0x2db308 {0x2a52bc}else{0}};
    let rc=match f {0x2db1d0=>x.allocate(a[1],&mut host).map_or(0,|v|v.address),0x2db308=>{x.report(b"",&mut host);0},0x2db4a0=>{x.free(&mut host);0},0x2db3c8=>{x.reset(&mut host);0},_=>unreachable!()};
    for (i,v) in [x.first_block.unwrap_or(0) as u32,x.expected,x.pool,x.allocated].into_iter().enumerate(){host.m.put_word(p+8+i as u32*4,v);}
    rc
}

/// fn_173fd8. Re-read each slot and the clamp after live callbacks.
pub fn max_slot_level_m(m:&mut Memory<'_>,p:u32,h:&mut impl boot_part_e::Host)->u32 {
    if p==0 {return 0;}
    let mut max=1;
    for i in 0..6 {let slot=if i==5 {p+0x34}else{p+8+i*8};let vt=m.word(slot+4);
        if vt!=0 && m.word(vt+40)!=0 {let method=m.word(vt+40);let obj=m.word(slot);max=max.max(h.call(m,method,&[obj]));}
    }
    if m.byte(p+0x31)!=0 && max>4 {max=4;}max
}

impl boot_part_e::Host for LiveHost<'_> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
        match f {0x18f488=>frontend::finish_request(),0x1ce56c=>frontend::streaming_mode((a[0]!=0).then(|| (m.byte(m.word(a[0])+4),m.byte(m.word(a[0])+0x10c)))),_=>self.boot.invoke(m,f,a)}
    }
}
/// Caller-owned typed storage for the certified prosody port. Supports 1024
/// phone records and 16 coefficients per record, with explicit bounds checks.
/// (Lane us r23: the ARM handles 1500+ char requests; windows reach ~390
/// phones / ~776 raw units, past the old 256/511 caps.)
pub struct FeatureWorkspace {
    pub blob:[u8;131072],pub bytes:[[u8;2053];60],pub words:[[u16;1025];60],
    pub ints:[[i32;16384];6],pub pitch:[u16;4096],pub contexts:[u8;9216],
    pub modes:[u8;16384],pub markers:[crate::prosody::ProsodyMarker;1024],pub carrier:[u8;4096],
}
fn snapshot(m:&Memory<'_>,blob:&mut [u8],used:&mut usize,p:u32,n:usize)->usize {
    let at=*used;assert!(at+n<=blob.len(),"voice snapshot capacity");
    if p!=0 {blob[at..at+n].copy_from_slice(&m.bytes[m.offset(p)..m.offset(p)+n]);}
    else {blob[at..at+n].fill(0);}
    *used+=n;at
}
fn cstrlen(m:&Memory<'_>,p:u32)->usize {if p==0 {0}else{frontend::strlen_bytes(&m.bytes[m.offset(p)..]) as usize+1}}
struct ProsodyHost<'a,'b,'c>{m:Memory<'a>,h:&'b mut LiveHost<'c>,level:u32}
impl crate::prosody::TimingHost for ProsodyHost<'_,'_,'_> {
    fn alloc(&mut self,heap:u32,n:u32)->u32 {boot_part_e::alloc(&mut self.m,heap,n,&mut self.h.boot)}
    fn free(&mut self,heap:u32,p:u32){boot_part_e::release(&mut self.m,heap,p,&mut self.h.boot);}
    fn logging(&mut self)->bool {self.level>3}
    fn log(&mut self,_:u32){assert!(self.level<=3,"active timing log");}
    fn error(&mut self,_:u32,_:u32){panic!("timing error diagnostic requires binding");}
    fn log_error(&mut self,_:u32){panic!("timing error diagnostic requires binding");}
}
impl crate::prosody::ProsodyHost for ProsodyHost<'_,'_,'_> {
    fn compute_features(&mut self,_:bool,_:&[u8],_:&[u32],_:i32,_:&crate::prosody::ProsodyFeatures<'_>,_:&mut u8,_:&mut u8){panic!("voice feature callback requires binding");}
    fn log_level(&mut self)->u32 {self.level}
    fn write_output(&mut self,_:u32,_:u32,_:&[u8],_:&[u8],_:crate::prosody::DumpBytes<'_>){panic!("active feature dump requires binding");}
}
fn features_binding(m:&mut Memory<'_>,context:u32,request:u32,p:u32,w:&mut FeatureWorkspace,h:&mut LiveHost<'_>)->u32 {
    use crate::prosody::{ProsodyVoice,ProsodyFeatures,ProsodyRules};
    let v=m.word(p);let n=m.half(p+6) as usize;let raw=m.half(p+8) as usize;
    assert!(n<=1024 && raw<=2048);
    let bp:[u32;60]=core::array::from_fn(|i|m.word(p+12+i as u32*4));
    let wp:[u32;60]=core::array::from_fn(|i|m.word(p+252+i as u32*4));
    let bl:[usize;60]=core::array::from_fn(|i|if i==0 {raw+2}else if i==1 {raw+5}else if i==29 {raw+1}else{n});
    for i in 0..60 {
        if bp[i]!=0 {let guard=if i==0 {1}else if i==1 {4}else{0};let start=bp[i]-guard;
            w.bytes[i][..bl[i]].copy_from_slice(&m.bytes[m.offset(start)..m.offset(start)+bl[i]]);}
        if wp[i]!=0 {for j in 0..n+usize::from(i==3){w.words[i][j]=m.half(wp[i]+j as u32*2);}}
    }
    let ac=m.half(v+0x98c) as usize;let dc=m.half(v+0xa32) as usize;assert!(ac<=16 && dc<=16);
    let ptrs:[u32;9]=core::array::from_fn(|i|m.word(p+0x1fc+i as u32*4));
    let sizes=[n*4,n*9,n*ac,n*dc,n*dc,n*ac,n*2,0,0];
    if ptrs[0]!=0 {for i in 0..sizes[0]{w.pitch[i]=m.half(ptrs[0]+i as u32*2);}}
    if ptrs[1]!=0 {w.contexts[..sizes[1]].copy_from_slice(&m.bytes[m.offset(ptrs[1])..m.offset(ptrs[1])+sizes[1]]);}
    for (k,slot) in [(0,2),(1,3),(2,4),(3,6)] {if ptrs[slot]!=0 {for j in 0..sizes[slot]{w.ints[k][j]=m.word(ptrs[slot]+j as u32*4) as i32;}}}
    if ptrs[5]!=0 {w.modes[..sizes[5]].copy_from_slice(&m.bytes[m.offset(ptrs[5])..m.offset(ptrs[5])+sizes[5]]);}
    let carrier_n=m.word(p+0x21c) as usize;assert!(carrier_n<=4096);
    if ptrs[7]!=0 {for j in 0..carrier_n {w.ints[4][j]=m.word(ptrs[7]+j as u32*4) as i32;}}
    let marker_n=m.word(p+0x1f0) as usize;assert!(marker_n<=1024);
    let marker=m.word(p+0x1ec);
    for i in 0..marker_n {let q=marker+i as u32*24;w.markers[i]=crate::prosody::ProsodyMarker{kind:m.half(q),active:m.word(q+8),position:m.word(q+12)};}
    let mut used=0;
    let vb=snapshot(m,&mut w.blob,&mut used,v,0x3b40);
    let mut names=[(0,0);256];let names_ptr=m.word(v+0x880);
    if names_ptr!=0 {for i in 0..256 {let q=m.word(names_ptr+i as u32*4);if q==0 {break;}let len=cstrlen(m,q);names[i]=(snapshot(m,&mut w.blob,&mut used,q,len),len);}}
    let mut rules=[(0,0,0);5];let mut maxp=0usize;let mut maxv=0usize;
    for i in 0..5 {let q=v+0xa68+i as u32*8;let count=m.half(q) as usize;let stride=m.half(q+2) as usize;let len=count*stride;
        let at=snapshot(m,&mut w.blob,&mut used,m.word(q+4),len);rules[i]=(count,stride,at);
        for j in 0..count {for k in 0..4 {maxp=maxp.max(w.blob[at+j*stride+k] as usize);}for k in 4..6 {maxv=maxv.max(w.blob[at+j*stride+k] as usize);}}
    }
    let mut psets=[(0,0);256];let mut vsets=[(0,0);256];
    for (dest,table,count) in [(&mut psets,m.word(v+0xa60),maxp+1),(&mut vsets,m.word(v+0xa64),maxv+1)] {
        if table!=0 {for i in 0..count {let q=m.word(table+i as u32*4);let len=cstrlen(m,q);dest[i]=(snapshot(m,&mut w.blob,&mut used,q,len),len);}}
    }
    let extra_offsets=[0x990,0xa34,0xa90,0xa94,0xa98];
    let extra_sizes=[ac*96,dc*96,512,256,256];
    let mut extra=[0usize;5];
    for i in 0..5 {extra[i]=snapshot(m,&mut w.blob,&mut used,m.word(v+extra_offsets[i]),extra_sizes[i]);}
    let blob=&w.blob;
    let at=|off:usize,len:usize| &blob[vb+off..vb+off+len];
    let wi=|off:usize|u32::from_le_bytes(at(off,4).try_into().unwrap()) as i32;
    let name_refs=names.map(|(a,n)|if n==0{None}else{Some(&blob[a..a+n])});
    let pset_refs=psets.map(|(a,n)|if n==0{None}else{Some(&blob[a..a+n])});
    let vset_refs=vsets.map(|(a,n)|if n==0{None}else{Some(&blob[a..a+n])});
    let rule=|i:usize|{let(c,s,a)=rules[i];ProsodyRules{count:c as u16,stride:s as u16,entries:&blob[a..a+c*s]}};
    let voice=ProsodyVoice {
        categories:at(0x23b,257),vowel_types:at(0x33c,257),language_classes:at(0x43d,256).try_into().unwrap(),
        phoneme_classes:at(0x740,256).try_into().unwrap(),modifiers:at(0x840,64),voice_names:&name_refs,
        markup:wi(0x8a0),context_enabled:wi(0x8cc),language:wi(0x8f8),feature_widths:at(0x950,60).try_into().unwrap(),
        acoustic_count:ac as u16,acoustic_defaults:&blob[extra[0]..extra[0]+extra_sizes[0]],
        category_feature:wi(0x9cc),phoneme_feature:wi(0x9d0),voice_feature:wi(0x9e4),modifier_feature:wi(0x9f0),
        number_feature:wi(0xa00),alternate_number_feature:wi(0xa0c),stress_feature:wi(0xa10),phrase_feature:wi(0xa14),
        position_feature:wi(0xa18),length_feature:wi(0xa1c),duration_count:dc as u16,duration_defaults:&blob[extra[1]..extra[1]+extra_sizes[1]],
        pitch_feature:wi(0xa48),alternate_pitch_feature:wi(0xa4c),phoneme_sets:&pset_refs,voice_sets:&vset_refs,
        acoustic_rules:rule(0),secondary_acoustic_rules:rule(1),duration_rules:rule(2),alternate_duration_rules:rule(3),mode_rules:rule(4),
        pitch_ranges:&blob[extra[2]..extra[2]+extra_sizes[2]],modifier_pitch_rules:&blob[extra[3]..extra[3]+extra_sizes[3]],
        phrase_pitch_rules:&blob[extra[4]..extra[4]+extra_sizes[4]],category_context:at(0x3b04,12),category_table:wi(0x3b10) as u32,
        phoneme_context:at(0x3b14,12),phoneme_table:&[],
    };
    let mut bytes=core::array::from_fn(|_|None);
    for (i,(out,b)) in bytes.iter_mut().zip(w.bytes.iter_mut()).enumerate(){if bp[i]!=0 {*out=Some(&mut b[..bl[i]]);}}
    let mut words=core::array::from_fn(|_|None);
    for (i,(out,b)) in words.iter_mut().zip(w.words.iter_mut()).enumerate(){if wp[i]!=0 {*out=Some(&mut b[..n+usize::from(i==3)]);}}
    let [acoustic,duration,alternate,timings,carrier_timings,_]=&mut w.ints;
    let mut data=ProsodyFeatures {
        bytes,words,markers:&w.markers[..marker_n],marker_count:marker_n as u32,count:m.half(p+6) as i16,length:raw as i16,timing_count:m.half(p+0x1f4) as i16,
        pitch:if ptrs[0]==0{None}else{Some(&mut w.pitch[..sizes[0]])},
        contexts:if ptrs[1]==0{None}else{Some(&mut w.contexts[..sizes[1]])},
        acoustic:if ptrs[2]==0{None}else{Some(&mut acoustic[..sizes[2]])},
        duration:if ptrs[3]==0{None}else{Some(&mut duration[..sizes[3]])},
        alternate_duration:if ptrs[4]==0{None}else{Some(&mut alternate[..sizes[4]])},
        modes:if ptrs[5]==0{None}else{Some(&mut w.modes[..sizes[5]])},
        timings:if ptrs[6]==0{None}else{Some(&mut timings[..sizes[6]])},
        carrier_timings:&carrier_timings[..carrier_n],carrier_scratch:&mut w.carrier,
    };
    let logger=m.word(context+16);let heap=m.word(context+4);
    let level=max_slot_level_m(m,logger,h);
    let mut host=ProsodyHost{m:Memory{base:m.base,bytes:m.bytes},h,level};
    let rc=crate::prosody::build_phone_features(heap,logger,request,&voice,&mut data,&mut host);
    host.m.put_half(p+6,data.count as u16);host.m.put_half(p+8,data.length as u16);host.m.put_half(p+0x1f4,data.timing_count as u16);
    for i in 0..60 {
        if let Some(b)=&data.bytes[i] {let guard=if i==0{1}else if i==1{4}else{0};let a=host.m.offset(bp[i]-guard);host.m.bytes[a..a+bl[i]].copy_from_slice(b);}
        if let Some(b)=&data.words[i] {for (j,x) in b.iter().enumerate(){host.m.put_half(wp[i]+j as u32*2,*x);}}
    }
    if let Some(b)=data.pitch {for (i,x) in b.iter().enumerate(){host.m.put_half(ptrs[0]+i as u32*2,*x);}}
    if let Some(b)=data.contexts {let a=host.m.offset(ptrs[1]);host.m.bytes[a..a+b.len()].copy_from_slice(b);}
    for (slot,b) in [(2,data.acoustic),(3,data.duration),(4,data.alternate_duration),(6,data.timings)] {if let Some(b)=b {for(i,x) in b.iter().enumerate(){host.m.put_word(ptrs[slot]+i as u32*4,*x as u32);}}}
    if let Some(b)=data.modes {let a=host.m.offset(ptrs[5]);host.m.bytes[a..a+b.len()].copy_from_slice(b);}
    rc
}

fn resource_lookup_binding(m:&mut Memory<'_>,p:u32,key:u32,out:u32)->u32 {
    if p==0 || key==0 || out==0 {return 0x8a202007;}
    m.put_word(out,0);let table=m.word(p+24);let disabled=m.byte(p+16)!=0;
    let kw=m.half(p+18);let vw=m.half(p+20);let count=m.half(p+22);
    let size=count as usize*kw.wrapping_add(vw) as usize;
    let bytes=if table==0 {&[][..]}else{&m.bytes[m.offset(table)..m.offset(table)+size]};
    let r=crate::resources::VoiceResource{bytes,count,key_width:kw,value_width:vw,disabled};
    match r.lookup(&m.bytes[m.offset(key)..]) {Ok(b)=>{let result=table+(size-b.len()) as u32;m.put_word(out,result);0},Err(e)=>e}
}

fn duration_pair_binding(m:&mut Memory<'_>,a:&[u32])->u32 {
    let p=a[1];let base=m.word(p+132);
    assert!(base!=0,"cold duration table needs cache binding");
    let t=crate::selection::DurationTable{width:m.word(p+4),count:m.word(p+8),offsets:core::array::from_fn(|i|m.word(p+16+i as u32*4)),
        offset_shift:m.half(p+72),shifts:core::array::from_fn(|i|m.half(p+74+i as u32*2)),scales:core::array::from_fn(|i|m.half(p+102+i as u32*2)),
        data:Some(&m.bytes[m.offset(base)..]),data_origin:0,offset:m.word(p+136)};
    let mut out=[0;2];let rc=t.read_pair(a[2],a[3],a[6],&mut out,|_,_|unreachable!()).expect("duration table");
    m.put_word(a[4],out[0] as u32);m.put_word(a[5],out[1] as u32);rc
}

