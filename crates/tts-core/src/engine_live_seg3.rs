//! Live segment 3 of the US speech_1 scheduler run (lane seg3, prompts/rust_seg_r1.md).
//!
//! Roots `0x1f98c8`, `0x1f1440`, `0x1f3248`, `0x1df780` with the `0x1cca64`
//! scheduler step before each. The host below serves the nested calls of
//! the four roots: the certified lane-b dispatcher covers every landed
//! port, with the four `1f98c8`-subtree entries the component test binds
//! by hand (`0x1738e8` short-log padding, `0x1edabc` via lane c4,
//! `0x1ed954` and `0x1ec7cc` via lane b3). The POS roots (`0x1f1440`/
//! `0x1f3248`) run the certified `pos` stage ports over a live `StageIo`
//! that issues the LINGDB and FE_DEPES method-table calls through this
//! same host.
//!
//! Note: `engine_live::Host` is private to its module, so this host cannot
//! name it; the arms below are written to merge into `Host::call_inner`
//! directly (each root arm takes the entry's register/stack words, serves
//! nested entries through `dispatch`, and falls through to the lane-b
//! dispatcher otherwise).
use crate::{
    boot::Memory,
    component_live_b::{self, Host as BLaneHost},
    component_live_b3, component_live_b4, component_live_c,
    component_live_c4, engine, engine_live, pos,
};
use crate::frontend_types::{Error as StageError, TokenId};

/// Guest scratch window for the segment (the excluded guest-stack area the
/// component tests use). The POS adapter's out-param slots live at
/// `+0x1000` and its byte buffer at `+0x1100`; both avoid the lane-b
/// transient slots (`+0x100`/`+0x300`/`+0x400`), the dictionary frames
/// (`+0x800`/`+0xc00`) and the one-word local frames (`+0x2000`/`+0x3000`).
pub const SEG_SCRATCH: u32 = 0xe0fe000;
const POS_SLOT: u32 = SEG_SCRATCH + 0x1000;
const POS_DATA: u32 = SEG_SCRATCH + 0x1100;

/// Segment host: caller-owned platform, fault and scratch, no globals.
pub struct Host<'a> {
    pub platform: &'a mut dyn engine::HeapPlatform,
    pub fault: Option<engine::Error>,
    pub scratch: u32,
    /// First entry the live dispatch could not serve, if any.
    pub first_unbound: Option<u32>,
    /// Depes VM workspace, reused across the module runs of a root (the
    /// PhrasingCalls recipe; a fresh workspace per call is not the ROM's
    /// behaviour).
    vm: component_live_c::VmWorkspace,
}

impl<'a> Host<'a> {
    pub fn new(platform: &'a mut dyn engine::HeapPlatform, scratch: u32) -> Self {
        Host {
            platform,
            fault: None,
            scratch,
            first_unbound: None,
            vm: component_live_c::VmWorkspace::default(),
        }
    }

    fn live_call(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        let mut inner = component_live_b::LiveHost {
            platform: &mut *self.platform,
            fault: None,
            scratch: self.scratch,
        };
        let rc = inner.call_m(f, a, m);
        if inner.fault.is_some() {
            self.fault = inner.fault;
        }
        rc
    }

    /// Serve one nested guest call. Roots are driven through the
    /// `root_*` entry points; everything the roots reach lands here.
    pub fn dispatch(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch_inner(m, f, a)
    }

    fn dispatch_inner(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        // The b3 string leaves: lane b's LiveHost slices the terminator
        // off and scans for one, so unterminated inputs panic there;
        // they are served locally and fail closed (the component test's
        // RootHost recipe).
        if f == 0x195d90 {
            return match a.first() {
                Some(&s) => component_live_b3::fn_195d90_m(m, s).unwrap_or(0x80002001),
                None => {
                    self.fault =
                        Some(engine::Error::MissingStreamAdapter { entry: f });
                    0x80002001
                }
            };
        }
        if f == 0x195d58 {
            return match (a.first(), a.get(1)) {
                (Some(&x), Some(&y)) => {
                    component_live_b3::fn_195d58_m(m, x, y).unwrap_or(0x80002001)
                }
                _ => {
                    self.fault =
                        Some(engine::Error::MissingStreamAdapter { entry: f });
                    0x80002001
                }
            };
        }
        // The ROM logger is varargs; lane b spills a[5..] into scratch, so
        // short log calls are padded with zeros (the component test's
        // RootHost recipe).
        if f == 0x1738e8 && a.len() < 6 {
            let mut padded = [0u32; 6];
            padded[..a.len()].copy_from_slice(a);
            return self.live_call(f, &padded, m);
        }
        // The dictionary LookupMultiword entry: lane c4's certified port
        // over a flat view of this arena (the component test's recipe).
        if f == 0x1edabc {
            let Some(args) = a.get(..7) else {
                self.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                return 0x80002001;
            };
            let args = [args[0], args[1], args[2], args[3], args[4], args[5], args[6]];
            let frame = self.scratch;
            let mut regions = [engine::Region { base: 0, bytes: &mut *m.bytes }];
            let mut view = match engine::Memory::new(&mut regions) {
                Ok(v) => v,
                Err(e) => {
                    self.fault = Some(e);
                    return 0x80002001;
                }
            };
            return match component_live_c4::fn_1edabc_m(
                &mut view,
                args,
                frame,
                &mut *self.platform,
            ) {
                Ok(rc) => rc,
                Err(e) => {
                    self.fault = Some(e);
                    0x80002001
                }
            };
        }
        // The record previous-in-sentence link the phrasing pass walks:
        // lane c's direct port (the PhrasingCalls recipe; lane b leaves
        // this entry unbound).
        // The phonetic-input path (c3 r29): lane c's direct port of
        // fn_274564, served over a +0x4000 guest scratch window (clear
        // of the POS/dictionary/one-word slots above). The port takes
        // the caller's 12-arg vector verbatim; a nested unbound entry
        // is recorded by the shared dispatch and returned as-is.
        if f == 0x274564 {
            let Some(args) = a.get(..12) else {
                self.fault =
                    Some(engine::Error::MissingStreamAdapter { entry: f });
                return 0x80002001;
            };
            let args = [
                args[0], args[1], args[2], args[3], args[4], args[5], args[6],
                args[7], args[8], args[9], args[10], args[11],
            ];
            let scratch = self.scratch.wrapping_add(0x4000);
            // NOTE: args[2] is the caller's field-3 string; the C body
            // never reads its `phon` parameter (it re-derives the string
            // via getPtr), so it is not forwarded.
            return component_live_b3::fn_274564_m(
                m,
                self,
                scratch,
                args[0],
                args[1],
                args[3] as u16,
                args[4],
                args[5],
                args[6],
                args[7],
                args[8],
                args[9],
                args[10],
                args[11] as i32,
            );
        }
        if f == 0x15ca38 {
            let Ok(quad) = <[u32; 4]>::try_from(a.get(..4).unwrap_or(&[])) else {
                self.fault =
                    Some(engine::Error::MissingStreamAdapter { entry: f });
                return 0x80002001;
            };
            return match component_live_c::record_link_m(m, f, quad) {
                Ok(rc) => rc,
                Err(e) => {
                    self.fault = Some(e);
                    0x80002001
                }
            };
        }
        // The depes module run: the workspace persists across the calls of
        // a root, like the phrasing host's (a fresh workspace per call
        // diverges on the partofspeech run).
        if f == 0x2193a4 {
            let Some(args) = a.get(..3) else {
                self.fault =
                    Some(engine::Error::MissingStreamAdapter { entry: f });
                return 0x80002001;
            };
            let args = [args[0], args[1], args[2]];
            let mut vm =
                core::mem::replace(&mut self.vm, component_live_c::VmWorkspace::default());
            let mut adapter = DepesAdapter { host: self };
            let rc = component_live_c::depes_exec_module_m(
                m, args[0], args[1], args[2], &mut vm, &mut adapter,
            );
            adapter.host.vm = vm;
            return rc;
        }
        // The dictionary normalizeWord entry (the 275928 DictIf call).
        if f == 0x1ed954 {
            let Some(args) = a.get(..5) else {
                self.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                return 0x80002001;
            };
            let args = [args[0], args[1], args[2], args[3], args[4]];
            return component_live_b3::fn_1ed954_m(
                m, self, args[0], args[1], args[2], args[3], args[4],
            );
        }
        // The ROM's fn_1ec7cc with total == 1 tail-calls fn_1ea4e0 (the
        // component test's recipe: lane b3's wrapper over lane b4).
        if f == 0x1ec7cc {
            let Some(args) = a.get(..8) else {
                self.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                return 0x80002001;
            };
            let args =
                [args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7]];
            let scratch = self.scratch.wrapping_add(0x800).wrapping_add(0x400);
            return component_live_b3::fn_1ec7cc_m(
                m, self, scratch, args[0], args[1], args[2], args[3], args[4], args[5],
                args[6], args[7],
            );
        }
        // The fe_dctlkp canonical-token entry (seg1 r10): the certified
        // component_live_c port directly. The ARM calls it with
        // (ctx, handle, key, text); mode 1 folds case, other modes are the
        // port's own behavior.
        if f == 0x1ed1d0 {
            let Some(args) = a.get(..4) else {
                self.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                return 0x80002001;
            };
            let args = [args[0], args[1], args[2], args[3]];
            let r = crate::component_live_c::dctlkp_canonical_token_m(
                m, args[0], args[1], args[2], args[3], self,
            );
            return r;
        }
        // The morphology config for a new 275928 mode (pause r4): lane
        // b3's 1:1 port (src/fn_275204.c) over this host; the per-field
        // getString rides the vtable (0x1ea4e0 fast path inside the
        // port). Scratch is the caller-owned excluded window,
        // saved/restored by the port.
        if f == 0x275204 {
            let Some(args) = a.get(..2) else {
                self.fault =
                    Some(engine::Error::MissingStreamAdapter { entry: f });
                return 0x80002001;
            };
            return component_live_b3::fn_275204_m(
                m, self, args[0], args[1],
                component_live_b3::FE275204_SCRATCH,
            );
        }
        // The dictionary fecfg/getString leaf the depes expansion reaches
        // on numbers and dates (c3 round 19): lane b4's certified port
        // directly, the way the 1f98c8 prologue binds it. Takes no scratch.
        if f == 0x1ea4e0 {            let Some(args) = a.get(..7) else {
                self.fault = Some(engine::Error::MissingStreamAdapter { entry: f });
                return 0x80002001;
            };
            let args =
                [args[0], args[1], args[2], args[3], args[4], args[5], args[6]];
            return component_live_b4::fn_1ea4e0_m(
                m, self, args[0], args[1], args[2], args[3], args[4], args[5],
                args[6],
            );
        }
        self.live_call(f, a, m)
    }

    /// The `0x1cca64` scheduler step rides the shared live-engine port.
    pub fn step_1cca64(&mut self, m: &mut Memory<'_>, a: [u32; 4], flag: u32) -> u32 {
        // The engine arm reads the flag from the guest stack slot the ARM
        // used at the step entry; mirror it through the scratch-adjacent
        // slot the arm reads when the stack image is absent is not an
        // option, so the caller passes the entry image with its stack and
        // the flag word is written to the arm's slot first.
        let _ = flag;
        match engine_live::call_live_m(m, 0x1cca64, &a, &mut *self.platform) {
            Ok(rc) => rc,
            Err(e) => {
                self.fault = Some(e);
                0x80002001
            }
        }
    }
}

impl BLaneHost for Host<'_> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.dispatch(m, entry, args)
    }
}

impl component_live_c::Host for Host<'_> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.dispatch(m, entry, args)
    }
    fn thread_id_m(&mut self) -> u32 {
        self.platform.thread_id()
    }
}

/// `fn_1f98c8(fe, b, a, c, done)`: the live one-word root, direct port call
/// with the component test's scratch layout.
pub fn root_oneword(
    h: &mut Host<'_>,
    m: &mut Memory<'_>,
    p: u32,
    b: u32,
    a: u32,
    c: u32,
    done: u32,
) -> u32 {
    let scratch = h.scratch;
    component_live_b3::fn_1f98c8_m(m, h, scratch, p, b, a, c, done)
}

/// `fn_1df780(object, tag, handle..., active)`: the live FE_PHRASING root.
pub fn root_phrasing(
    h: &mut Host<'_>,
    m: &mut Memory<'_>,
    args: [u32; 4],
    active: u32,
    buffers: &mut crate::prosody::PhrasingBuffers<'_>,
    w: &mut component_live_c::VmWorkspace,
) -> u32 {
    component_live_c::fn_1df780_m(m, &args, active, buffers, w, h)
}

/// Adapter letting the depes VM reenter this host (nested calls ride
/// `dispatch`, like every other nested call of the segment).
struct DepesAdapter<'x, 'y> {
    host: &'x mut Host<'y>,
}

impl component_live_c::Host for DepesAdapter<'_, '_> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.host.dispatch(m, entry, args)
    }
    fn thread_id_m(&mut self) -> u32 {
        self.host.platform.thread_id()
    }
}

/// Live `StageIo` over the LINGDB (`ling`) and FE_DEPES (`depes`) method
/// tables the POS module holds. Method addresses are read from the tables;
/// out-params and byte payloads ride the caller-owned guest slots.
struct PosIo<'x, 'y> {
    h: &'x mut Host<'y>,
    m: &'x mut Memory<'y>,
    p: u32,
    ling: u32,
    hh: [u32; 2],
    depes: [u32; 2],
    depes_m: u32,
    /// Guest line buffers for `SetRule` (the `1f1440` heap allocs), else 0.
    lines: [u32; 3],
    /// Guest rule-output buffers by layer, captured at `GetRule` so the
    /// port's in-place token NULs can be committed to the arena.
    get_ptr: [u32; 3],
    get_count: [u32; 3],
}

impl PosIo<'_, '_> {
    /// Issue a LINGDB `text`-family call, returning the guest (ptr, count).
    fn node_text(&mut self, entry: u32, node: u32, index: u32) -> (u32, u32, u16) {
        let hh = self.hh;
        let h = &mut *self.h;
        let m = &mut *self.m;
        m.put_word(POS_SLOT.wrapping_add(8), 0);
        m.put_half(POS_SLOT.wrapping_add(4), 0);
        let rc = h.dispatch(
            m,
            entry,
            &[hh[0], hh[1], node, index, POS_SLOT.wrapping_add(8), POS_SLOT.wrapping_add(4)],
        );
        let m = &mut *self.m;
        (rc, m.word(POS_SLOT.wrapping_add(8)), m.half(POS_SLOT.wrapping_add(4)))
    }

    fn copy_from_guest(&mut self, ptr: u32, count: usize, out: &mut [u8]) -> usize {
        let n = count.min(out.len());
        for (i, b) in out[..n].iter_mut().enumerate() {
            *b = self.m.byte(ptr.wrapping_add(i as u32));
        }
        n
    }

    fn copy_to_data(&mut self, input: &[u8]) {
        for (i, b) in input.iter().enumerate() {
            self.m.put_byte(POS_DATA.wrapping_add(i as u32), *b);
        }
    }
}

impl pos::StageIo for PosIo<'_, '_> {
    fn call(
        &mut self,
        op: pos::StageOperation,
        args: [u32; 3],
        input: &[u8],
        output: &mut [u8],
    ) -> Result<pos::StageReply, StageError> {
        use pos::{StageOperation as Op, StageReply};
        match op {
            Op::Validate => {
                let p = self.p;
                let h = &mut *self.h;
                let m = &mut *self.m;
                let rc = h.dispatch(m, 0x176f94, &[p, args[0], args[0], args[1]]);
                Ok(StageReply { status: rc, count: 0, bytes: 0 })
            }
            Op::First => {
                let hh = self.hh;
                let ling = self.ling;
                let h = &mut *self.h;
                let m = &mut *self.m;
                let first = m.word(ling.wrapping_add(52));
                m.put_half(POS_SLOT, 0);
                let rc = h.dispatch(m, first, &[hh[0], hh[1], args[0], args[1], POS_SLOT]);
                let got = m.half(POS_SLOT);
                output[..2].copy_from_slice(&got.to_le_bytes());
                Ok(StageReply { status: rc, count: 0, bytes: 2 })
            }
            Op::Next => {
                let hh = self.hh;
                let ling = self.ling;
                let h = &mut *self.h;
                let m = &mut *self.m;
                let next = m.word(ling.wrapping_add(60));
                m.put_half(POS_SLOT, 0);
                let rc = h.dispatch(m, next, &[hh[0], hh[1], args[0], POS_SLOT]);
                let got = m.half(POS_SLOT);
                output[..2].copy_from_slice(&got.to_le_bytes());
                Ok(StageReply { status: rc, count: 0, bytes: 2 })
            }
            Op::Value => {
                let hh = self.hh;
                let ling = self.ling;
                let h = &mut *self.h;
                let m = &mut *self.m;
                let value = m.word(ling.wrapping_add(92));
                m.put_half(POS_SLOT, 0);
                let rc =
                    h.dispatch(m, value, &[hh[0], hh[1], args[0], args[1], POS_SLOT]);
                let got = m.half(POS_SLOT);
                output[..2].copy_from_slice(&got.to_le_bytes());
                Ok(StageReply { status: rc, count: 0, bytes: 2 })
            }
            Op::Read => {
                let hh = self.hh;
                let ling = self.ling;
                let h = &mut *self.h;
                let m = &mut *self.m;
                let read = m.word(ling.wrapping_add(84));
                let n = if args[1] == 0 { 1 } else { 2 };
                m.put_half(POS_SLOT.wrapping_add(4), 0);
                for i in 0..4 {
                    m.put_byte(POS_DATA.wrapping_add(i), 0);
                }
                let rc = h.dispatch(
                    m,
                    read,
                    &[hh[0], hh[1], args[0], args[1], 1, POS_DATA, POS_SLOT.wrapping_add(4)],
                );
                output[0] = 0;
                output[1] = 0;
                for i in 0..n.min(output.len()) {
                    output[i] = m.byte(POS_DATA.wrapping_add(i as u32));
                }
                Ok(StageReply { status: rc, count: 0, bytes: 2 })
            }
            Op::Text => {
                let ling = self.ling;
                let h = &mut *self.h;
                let m = &mut *self.m;
                let text = m.word(ling.wrapping_add(88));
                let _ = h;
                let _ = m;
                let (rc, ptr, count) = self.node_text(text, args[0], args[1]);
                let n = self.copy_from_guest(ptr, usize::from(count), output);
                Ok(StageReply { status: rc, count, bytes: n })
            }
            Op::Set => {
                let hh = self.hh;
                let ling = self.ling;
                let h = &mut *self.h;
                let m = &mut *self.m;
                let set = m.word(ling.wrapping_add(80));
                let _ = h;
                let _ = m;
                let data_len =
                    (args[2] as usize).wrapping_mul(if args[1] == 7 { 2 } else { 1 });
                self.copy_to_data(&input[..data_len.min(input.len())]);
                let h = &mut *self.h;
                let m = &mut *self.m;
                m.put_half(POS_SLOT.wrapping_add(12), 0);
                let rc = h.dispatch(
                    m,
                    set,
                    &[hh[0], hh[1], args[0], args[1], args[2], POS_DATA, POS_SLOT.wrapping_add(12)],
                );
                Ok(StageReply { status: rc, count: 0, bytes: 0 })
            }
            Op::Remove => {
                let hh = self.hh;
                let ling = self.ling;
                let h = &mut *self.h;
                let m = &mut *self.m;
                let remove = m.word(ling.wrapping_add(96));
                let rc = h.dispatch(m, remove, &[hh[0], hh[1], args[0]]);
                Ok(StageReply { status: rc, count: 0, bytes: 2 })
            }
            Op::SetRule => {
                let depes = self.depes;
                let depes_m = self.depes_m;
                let lines = self.lines;
                let line = args[0] as usize;
                let buf = if line < 3 && lines[line] != 0 { lines[line] } else { POS_DATA };
                if buf == POS_DATA {
                    self.copy_to_data(input);
                } else {
                    // The guest line holds the port's bytes already; mirror
                    // the ARM heap line for the depes call (content + NUL).
                    for (i, b) in input.iter().enumerate() {
                        self.m.put_byte(buf.wrapping_add(i as u32), *b);
                    }
                    self.m.put_byte(buf.wrapping_add(input.len() as u32), 0);
                }
                let h = &mut *self.h;
                let m = &mut *self.m;
                let set = m.word(depes_m.wrapping_add(36));
                let rc = h.dispatch(m, set, &[depes[0], depes[1], args[0], buf, args[1]]);
                Ok(StageReply { status: rc, count: 0, bytes: 0 })
            }
            Op::GetRule => {
                let depes = self.depes;
                let depes_m = self.depes_m;
                let h = &mut *self.h;
                let m = &mut *self.m;
                let get = m.word(depes_m.wrapping_add(40));
                m.put_word(POS_SLOT.wrapping_add(8), 0);
                m.put_half(POS_SLOT.wrapping_add(4), 0);
                let rc = h.dispatch(
                    m,
                    get,
                    &[depes[0], depes[1], args[0], POS_SLOT.wrapping_add(8), POS_SLOT.wrapping_add(4)],
                );
                let ptr = m.word(POS_SLOT.wrapping_add(8));
                let count = m.half(POS_SLOT.wrapping_add(4));
                let layer = args[0] as usize;
                if layer < 3 {
                    self.get_ptr[layer] = ptr;
                    self.get_count[layer] = u32::from(count);
                }
                let _ = h;
                let _ = m;
                let n = self.copy_from_guest(ptr, usize::from(count), output);
                Ok(StageReply { status: rc, count, bytes: n })
            }
            Op::ProcessRule => {
                let depes = self.depes;
                let depes_m = self.depes_m;
                self.copy_to_data(input);
                let h = &mut *self.h;
                let m = &mut *self.m;
                let process = m.word(depes_m.wrapping_add(44));
                let rc = h.dispatch(m, process, &[depes[0], depes[1], POS_DATA]);
                Ok(StageReply { status: rc, count: 0, bytes: 0 })
            }
            Op::Classify => {
                // The ROM classifies one byte: nonzero for the boundary
                // bytes the scans skip (the same set the port treats as
                // whitespace).
                let b = (args[0] & 0xff) as u8;
                let status =
                    u32::from(matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'_'));
                Ok(StageReply { status, count: 0, bytes: 0 })
            }
        }
    }

    fn truncate_text(&mut self, node: TokenId, field: u16) -> Result<(), StageError> {
        // Re-issue the text fetch for the same guest pointer the port saw
        // and cut the field at the first backslash, like the ROM's
        // truncate loop over the fetched pointer.
        let ling = self.ling;
        let h = &mut *self.h;
        let m = &mut *self.m;
        let text = m.word(ling.wrapping_add(88));
        let _ = h;
        let _ = m;
        let (_, ptr, count) = self.node_text(text, u32::from(node.0), u32::from(field));
        for i in 0..u32::from(count) {
            if self.m.byte(ptr.wrapping_add(i)) == b'\\' {
                self.m.put_byte(ptr.wrapping_add(i), 0);
                break;
            }
        }
        Ok(())
    }
}

/// Read the POS module header the scheduler passes: the LINGDB/FE_DEPES
/// handles and their method tables.
fn pos_module(m: &mut Memory<'_>, p: u32) -> (u32, u32, [u32; 2], u32) {
    let ling = m.word(p.wrapping_add(4));
    let depes = [m.word(p.wrapping_add(8)), m.word(p.wrapping_add(12))];
    let depes_m = m.word(p.wrapping_add(16));
    let env = m.word(p);
    (env, ling, depes, depes_m)
}

/// `fn_1f1440(p, tag, handle..., out)`: the live FE_POS root. The three
/// line buffers are guest-heap allocs (freed at cleanup), exactly like the
/// ROM; the port writes the line bodies through host buffers and the
/// adapter mirrors them into the guest allocs at each `SetRule`.
pub fn root_pos<'a>(
    h: &mut Host<'a>,
    m: &mut Memory<'a>,
    p: u32,
    tag: u32,
    h0: u32,
    h1: u32,
    out: u32,
    scratch: &mut pos::StageScratch<'_>,
) -> u32 {
    let _ = tag;
    let (env, ling, depes, depes_m) = pos_module(m, p);
    // The root marks its out byte live on entry (ARM 0x1f14bc strb);
    // every exit path below, including the early state-gate return,
    // leaves it set unless the final status overwrites it.
    m.put_byte(out, 1);
    let heap = m.word(env.wrapping_add(4));
    let first_fn = m.word(ling.wrapping_add(52));
    let text_fn = m.word(ling.wrapping_add(88));
    // First(input, 1, 0): the port's own first call; the root id is needed
    // before any alloc, so it is fetched here through the live methods.
    m.put_half(POS_SLOT, 0);
    let rc = h.dispatch(m, first_fn, &[h0, h1, 1, 0, POS_SLOT]);
    if rc >= 0x80000000 {
        return rc;
    }
    let root = m.half(POS_SLOT);
    // The state gate (ARM 0x1f150c, slot ling+92, 0x15d594 on this
    // path): unless the indexed field state is 1, the root returns the
    // First rc it already holds in r5 (0x1f1520 bne / 0x1f1514 bhs land
    // on 0x1f27dc, which returns r5, even when the state call errors).
    // The half slot is reused by the text call, which re-zeroes it.
    let state_fn = m.word(ling.wrapping_add(92));
    let state_rc =
        h.dispatch(m, state_fn, &[h0, h1, u32::from(root), 0, POS_SLOT.wrapping_add(4)]);
    if state_rc >= 0x80000000 || m.half(POS_SLOT.wrapping_add(4)) != 1 {
        return rc;
    }
    // text(root, 0): the sentence the line sizes derive from (ROM: strlen
    // + 129 per line).
    m.put_word(POS_SLOT.wrapping_add(8), 0);
    m.put_half(POS_SLOT.wrapping_add(4), 0);
    let rc = h.dispatch(
        m,
        text_fn,
        &[h0, h1, u32::from(root), 0, POS_SLOT.wrapping_add(8), POS_SLOT.wrapping_add(4)],
    );
    if rc >= 0x80000000 {
        return rc;
    }
    // ROM (fn_1f1440): a one-or-fewer count returns the text rc with no
    // line allocs and no POS walk. process_pos re-checks the same gate,
    // so behavior is identical either way; skipping the allocs keeps
    // later heap addresses ARM-identical on this path.
    if m.half(POS_SLOT.wrapping_add(4)) <= 1 {
        return rc;
    }
    let ptr = m.word(POS_SLOT.wrapping_add(8));
    let mut len = 0u32;
    while m.byte(ptr.wrapping_add(len)) != 0 {
        len = len.wrapping_add(1);
    }
    let size = len.wrapping_add(129);
    let mut lines = [0u32; 3];
    for (i, slot) in lines.iter_mut().enumerate() {
        let got = h.dispatch(m, 0x196394, &[heap, size]);
        if got == 0 {
            // Allocation failure mirrors the ROM: free what was taken and
            // report the alloc status.
            for done in lines[..i].iter() {
                h.dispatch(m, 0x196428, &[heap, *done]);
            }
            return 0x89e0200a;
        }
        *slot = got;
    }
    for (i, line) in lines.iter().enumerate() {
        m.put_word(p.wrapping_add(20).wrapping_add(i as u32 * 4), *line);
        m.put_byte(*line, 0);
    }
    let mut probe = PosIo {
        h,
        m,
        p,
        ling,
        hh: [h0, h1],
        depes,
        depes_m,
        lines,
        get_ptr: [0; 3],
        get_count: [0; 3],
    };
    let mut applied = 0u8;
    let result = pos::process_pos(&mut probe, &mut applied, scratch);
    // Commit the port's in-place rule-token NULs to the guest rule
    // buffers (the ROM tokenizes the depes outputs destructively).
    for layer in 1..=2usize {
        let ptr = probe.get_ptr[layer];
        if ptr != 0 {
            let n = (probe.get_count[layer] as usize)
                .wrapping_add(1)
                .min(scratch.rule_output[layer - 1].len());
            let m = &mut *probe.m;
            for (i, b) in scratch.rule_output[layer - 1][..n].iter().enumerate() {
                m.put_byte(ptr.wrapping_add(i as u32), *b);
            }
        }
    }
    let (h, m) = (&mut *probe.h, &mut *probe.m);
    // Cleanup mirrors the ROM: free the three lines in order, clear the
    // module words.
    for line in lines {
        h.dispatch(m, 0x196428, &[heap, line]);
    }
    for i in 0..3 {
        m.put_word(p.wrapping_add(20).wrapping_add(i * 4), 0);
    }
    match result {
        Ok(status) => {
            m.put_byte(out, applied);
            status
        }
        Err(StageError::Engine(status)) => {
            m.put_byte(out, applied);
            status
        }
        Err(_) => {
            h.fault = Some(engine::Error::MissingStreamAdapter { entry: 0x1f1440 });
            0x80002001
        }
    }
}

/// `fn_1f3248(filter, check, handle..., applied)`: the live FE_HMOGRPH
/// root. The record scan rides the module's limit/preserve words; the
/// validate lines stay in host buffers (the ROM keeps them on its stack).
pub fn root_homographs<'a>(
    h: &mut Host<'a>,
    m: &mut Memory<'a>,
    p: u32,
    check: u32,
    h0: u32,
    h1: u32,
    out: u32,
    scratch: &mut pos::StageScratch<'_>,
) -> u32 {
    let _ = check;
    let (_, ling, depes, depes_m) = pos_module(m, p);
    let limit = m.half(p.wrapping_add(42));
    let preserve = m.byte(p.wrapping_add(44)) != 0;
    m.put_half(p.wrapping_add(40), 0);
    let mut state = pos::HomographState { capacity: 0, limit, preserve };
    let mut probe = PosIo {
        h,
        m,
        p,
        ling,
        hh: [h0, h1],
        depes,
        depes_m,
        lines: [0; 3],
        get_ptr: [0; 3],
        get_count: [0; 3],
    };
    let mut applied = 0u8;
    let result = pos::process_homographs(&mut probe, &mut state, &mut applied, scratch);
    let (h, m) = (&mut *probe.h, &mut *probe.m);
    // Cleanup mirrors the ROM: free the text/record buffers when set.
    let env = m.word(p);
    let heap = m.word(env.wrapping_add(4));
    let text_buffer = m.word(p.wrapping_add(32));
    if text_buffer != 0 {
        h.dispatch(m, 0x196428, &[heap, text_buffer]);
        m.put_word(p.wrapping_add(32), 0);
    }
    let record_buffer = m.word(p.wrapping_add(36));
    if record_buffer != 0 {
        h.dispatch(m, 0x196428, &[heap, record_buffer]);
        m.put_word(p.wrapping_add(36), 0);
    }
    match result {
        Ok(status) => {
            m.put_byte(out, applied);
            status
        }
        Err(StageError::Engine(status)) => {
            m.put_byte(out, applied);
            status
        }
        Err(_) => {
            h.fault = Some(engine::Error::MissingStreamAdapter { entry: 0x1f3248 });
            0x80002001
        }
    }
}
