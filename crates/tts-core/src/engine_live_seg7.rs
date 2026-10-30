//! Live segment 7 of the US speech_1 scheduler run (lane seg7, round 2).
//!
//! Roots (scheduler order, lr 0x154ef8, sp 0xe0ff938):
//! 0x22d6dc (insertion pump), 0x1e3b14 (marker processor), 0x231ba0
//! (xcoder markers), 0x1530d0 (output pump). Captures:
//! scratch/engine-stages/seg7-<root>-entry.wire / seg7-<root>-ret.expected
//! (all r0 0). The epilogue hops are captured as seg7x-22a0a8-entry.wire
//! (post-pump scheduler root), seg7x-22d6dc2/1e3b142/231ba02 (second
//! iteration), seg7c1-1530d0-entry.wire / -ret.expected (chunk-1 pump),
//! and seg7-1a18c4-entry.wire, which is chunk-1's nested render entry
//! (proven identical to a re-capture; the 22a0a8 body between is seg6's).
//!
//! Dispatch: 0x22d6dc and 0x231ba0 already ride engine_live's Host, so the
//! arms forward to the public live dispatcher. 0x1e3b14 rides
//! engine_components::process with a bridge that forwards every nested
//! call to the same public dispatcher (single-region views only). 0x1530d0
//! rides emit_output_m with a live output side that mirrors the app's
//! own 0x18f6b8 call for call (local buffer, locks, tag map, two
//! callbacks); the callbacks run through lane back's certified BackHost
//! with a caller-owned backend workspace, PCM sink, tick and VFP
//! registers, so the nested render, capture hooks and publication all
//! match, including the app-domain job block writes around 0xAF340D.
//! No globals, no unsafe, no replayed results: every address read is
//! guest state the ARM reads at the same point.
//!
//! The scheduler step between roots is the taken path of fn_15429c
//! (src/fn_151914.c) through this stretch: aux-notify via 0x15daa8, the
//! "Process for %s returned" log, advance/loop-head bookkeeping
//! (pending++/anStage++), the "Calling Process for %s" log, and the next
//! root call with live-derived args (comp table, aux pair, extra+stage).
//! The finish_stage path (last stage) calls 0x1530d0 instead of a root.
//! After 0x1530d0 returns, step_seg7_post_pump replicates the finish
//! tail: store rc, rewind anStage to the newest nonzero extra byte,
//! rejoin the loop head (no aux-notify, no returned log on this path),
//! and call the next root. Every branch condition is evaluated live;
//! an untaken path returns UnsupportedStage naming its site instead of
//! silently diverging.
//!
//! Follow-ups (no persistent effect on this request, proven by the
//! boundary diffs): the live 0x15daa8 arm skips the 0x176f94 lock the ARM
//! takes inside fn_15daa8 (lr 0x15dac4); the voice-switch body behind
//! extra==3 is guarded, not ported.

use crate::{boot::Memory, engine};
use crate::engine_components::InternalCalls;
use crate::engine_live::NESTED_RESULT;
use crate::engine_live_back::BackHost;

/// Caller-owned render context for the 0x1530d0 nested render. Every
/// field is an explicit caller input: the backend workspace and PCM sink
/// the nested 0x1a18c4 render needs (lane back's certified shape), the
/// sink fill level, the host tick, and the VFP registers the ARM holds
/// at this lane's notify sites (s16 0 / s17 0 on the full run-up, proven
/// by the 1530d0 return image; never a baked constant).
pub struct RenderCtx<'w> {
    pub workspace: &'w mut crate::engine_live::BackendWorkspace<'w>,
    pub pcm: &'w mut [i16],
    pub written: usize,
    pub tick: u64,
    pub registers: engine::NotificationRegisters,
    /// Stop the nested dispatch at the 0x1a18c4 render entry (report it
    /// as unbound) instead of running the render. The arena then holds
    /// the live nested-entry state for boundary certification.
    pub stop_at_render: bool,
}

/// No-op sleeper for the nested render: the 0x166054 waits inside the
/// kind-0x10 dispatch have no guest effect, so they succeed at once.
struct NoSleep;
impl crate::engine_live::SleepPlatform for NoSleep {
    fn sleep_ns(&mut self, _nanoseconds: u64) -> Result<(), engine::Error> {
        Ok(())
    }
}
/// Excluded-stack frame (32 bytes) for the 0x1e3b14 process scratch.
/// Inside 0x0e000000..0x0e100000, never compared; below the caller SP.
const SEG7_FRAME_1E3B14: u32 = 0x0e0fdd00;
/// Excluded-stack slots for the scheduler step's aux out/status byte.
const SEG7_STEP_SLOT: u32 = 0x0e0fdd40;
/// Excluded-stack varargs (3 words) for the scheduler step's logs.
const SEG7_STEP_VARARGS: u32 = 0x0e0fdd48;
/// Excluded-stack SAMP varargs (4 words) and params (20 bytes).
const SEG7_SAMP_VARARGS: u32 = 0x0e0fddc0;
const SEG7_SAMP_PARAMS: u32 = 0x0e0fddd0;
/// Excluded-stack app request block (28 bytes: kind, flags, result
/// pointer, padding) for the 0x18f6b8-equivalent callback.
const SEG7_REQ: u32 = 0x0e0fdd60;
/// Guest scratch the application side uses (same convention as the
/// engine's own output side): local buffer, request, out-params.
const SEG7_OUT_LOCAL: u32 = 0x1f00;
const SEG7_OUT_REQUEST: u32 = 0x1f20;
const SEG7_OUT_SCRATCH: u32 = 0x1f40;

/// Format strings both scheduler logs use (ROM immediates at their sites).
const SEG7_FMT_RETURNED: u32 = 0x1551c4;
const SEG7_FMT_CALLING: u32 = 0x154630;
/// Log module both scheduler logs use.
const SEG7_LOG_MODULE: u32 = 0x8d7994;
/// Name table the "Process for" log indexes with the extra byte.
const SEG7_VNAME_TABLE: u32 = 0x8d7a84;

/// Bridge from the 0x1e3b14 port to the public live dispatcher. Holds no
/// arena; every nested call runs on the caller's arena through a fresh
/// live dispatch with the same platform, runtime and snapshot.
pub struct Seg7Bridge<'p, 'r> {
    pub platform: &'p mut dyn engine::HeapPlatform,
    pub runtime: &'r mut dyn crate::engine_live::RuntimeServicesM,
    pub snap: &'r [u8],
}

impl InternalCalls for Seg7Bridge<'_, '_> {
    fn call(
        &mut self,
        m: &mut engine::Memory<'_, '_>,
        entry: u32,
        args: &[u32],
    ) -> Result<u32, engine::Error> {
        if m.regions.len() != 1 {
            return Err(engine::Error::InvalidSnapshot);
        }
        let base = m.regions[0].base;
        let bytes = &mut *m.regions[0].bytes;
        let mut bm = Memory { base, bytes };
        crate::engine_live::call_live_with_runtime_m(
            &mut bm,
            entry,
            args,
            self.platform,
            self.runtime,
            self.snap,
        )
    }
}

/// The application's output side over the live guest, mirroring the
/// engine's own output side call for call. `synth` is the 0x1530d0 `p`.
/// The recorded stream observations let the marker-array growth compute
/// the pump's own count exactly (single growth on this request; a second
/// firing errors instead of guessing).
pub struct Seg7OutputSide<'p, 'r, 'x> {
    pub platform: &'p mut dyn engine::HeapPlatform,
    pub runtime: &'r mut dyn crate::engine_live::RuntimeServicesM,
    pub snap: &'r [u8],
    pub synth: u32,
    pub render: Option<RenderCtx<'x>>,
    pub put_calls: u32,
    pub pcm_avail: u32,
    pub mk_buf: u32,
    pub mk_len: u32,
    pub pos_calls: u32,
    pub pos_vals: [u32; 8],
    pub eos: u8,
    pub growths: u32,
    pub first_unbound: Option<u32>,
}

impl<'x> Seg7OutputSide<'_, '_, 'x> {
    fn live(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        match crate::engine_live::call_live_with_runtime_m(
            m,
            entry,
            args,
            self.platform,
            self.runtime,
            self.snap,
        ) {
            Ok(rc) => rc,
            Err(e) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(match e {
                        engine::Error::UnsupportedStage { entry: f }
                        | engine::Error::MissingStreamAdapter { entry: f } => f,
                        _ => entry,
                    });
                }
                0x80002001
            }
        }
    }
    fn context(&self, m: &Memory<'_>) -> u32 {
        m.word(self.synth.wrapping_add(0x84))
    }
    /// fn_1738e8 with the engine's module and a live level. Varargs ride
    /// the logger scratch at 0x500, saved and restored like the engine.
    fn log(&mut self, m: &mut Memory<'_>, level: u32, fmt: u32, values: &[u32]) {
        let logger = m.word(self.context(m).wrapping_add(16));
        let s = 0x500;
        let mut saved = [0u32; 12];
        for (i, v) in saved.iter_mut().enumerate() {
            *v = m.word(s + i as u32 * 4);
        }
        for (i, v) in values.iter().enumerate() {
            m.put_word(s + i as u32 * 4, *v);
        }
        self.live(m, 0x1738e8, &[logger, 0x8d7994, level, fmt, s]);
        for (i, v) in saved.iter().enumerate() {
            m.put_word(s + i as u32 * 4, *v);
        }
    }
    fn vt_entry(&self, m: &Memory<'_>, off: u32) -> u32 {
        m.word(m.word(self.synth.wrapping_add(0x40)).wrapping_add(off))
    }
    /// One application callback through lane back's certified BackHost:
    /// the 0x163a98 render event with the live-derived 5 args
    /// (callback_arg, user, object_id, context, request). The inner
    /// runtime carries the caller's backend workspace, PCM sink, tick
    /// and VFP registers; a nested fault is recorded and reported as
    /// the hook error instead of guessed.
    fn render_163a98(&mut self, m: &mut Memory<'_>, user: u32, event: u32) -> Result<u32, ()> {
        let render = match self.render.as_mut() {
            Some(r) => r,
            None => {
                self.first_unbound.get_or_insert(0x1a18c4);
                return Err(());
            }
        };
        let args5 = [
            m.word(user.wrapping_add(0x184)),
            user,
            0x684f424a,
            m.word(user.wrapping_add(0x16c)),
            event,
        ];
        let mut nap = NoSleep;
        let mut inner = crate::engine_live::RuntimeM {
            backend: Some(&mut *render.workspace),
            pcm: &mut *render.pcm,
            written: render.written,
            tick: render.tick,
            registers: render.registers,
            sleeper: &mut nap,
        };
        if render.stop_at_render {
            let mut stopper = StopAtRender { inner: &mut inner, stopped: false };
            match BackHost::call_entry(m, 0x163a98, &args5, self.platform, &mut stopper, self.snap) {
                Ok(status) => {
                    render.written = inner.written;
                    Ok(status)
                }
                Err(e) => {
                    self.note_fault(e);
                    Err(())
                }
            }
        } else {
            match BackHost::call_entry(m, 0x163a98, &args5, self.platform, &mut inner, self.snap) {
                Ok(status) => {
                    render.written = inner.written;
                    Ok(status)
                }
                Err(e) => {
                    self.note_fault(e);
                    Err(())
                }
            }
        }
    }
    fn note_fault(&mut self, e: engine::Error) {
        self.first_unbound.get_or_insert(match e {
            engine::Error::UnsupportedStage { entry: f }
            | engine::Error::MissingStreamAdapter { entry: f } => f,
            _ => 0x163a98,
        });
    }
    /// The finish-path direct 18f6b8 flush (fn_1561b8 +0x39c: putOutput
    /// user, flags=2, sptr, cptr, tags, tcptr). ROM fn_18f6b8 with the
    /// app path (user buffer null) and flags&2: local AudioBuffer setup
    /// (samples = *sptr, bytes = 2 * *cptr, tags = p->tags), check_state
    /// via the live lock, tag map, then callback 1 (deliver + render)
    /// through BackHost and done. No logs (fn_18f6b8 logs nothing), no
    /// callback 2 (pop), no have_output. request_flags rides 0xffff
    /// for the delivery (flags&2) and is 2 afterwards, exactly like the
    /// ROM. *sptr/*cptr are zeroed on the way out. Returns the 18f6b8
    /// return code (0 on the certified runs).
    /// The finish-path direct 18f6b8 flush (flags&2): callback 1 only.
    pub fn flush_m(
        &mut self,
        mut m: &mut Memory<'_>,
        user: u32,
        sptr: u32,
        cptr: u32,
        tags: u32,
        tcptr: u32,
    ) -> u32 {
        self.app_pump_m(m, user, 2, sptr, cptr, tags, tcptr)
    }
    /// ROM fn_18f6b8 on the app path (user buffer null): local AudioBuffer
    /// setup (samples = *sptr, bytes = 2 * *cptr, tags = p->tags),
    /// check_state via the live lock, tag map, then callback 1 (deliver +
    /// render) through BackHost. flags&2 presets request_flags (0xffff)
    /// and ends after callback 1 (*sptr/*cptr zeroed). Otherwise callback
    /// 2 (pop) follows with have_output (*sptr/*cptr/*tcptr and the user
    /// tag words follow the popped local buffer). No logs (fn_18f6b8
    /// logs nothing). Returns the 18f6b8 return code (0 on the certified
    /// runs). `tags`/`tcptr` select the marker source/count; the mapped
    /// records land in the app records (p->tags) through the live 18f490.
    pub fn app_pump_m(
        &mut self,
        mut m: &mut Memory<'_>,
        user: u32,
        flags: u32,
        sptr: u32,
        cptr: u32,
        tags: u32,
        tcptr: u32,
    ) -> u32 {
        let _ = tcptr;
        // Local AudioBuffer setup (C locals): samples = *sptr,
        // bytes = 2 * *cptr, tag_count = *tcptr, tags = p->tags.
        let local_samples = m.word(sptr);
        let local_bytes = m.word(cptr).wrapping_mul(2);
        let local_tags = m.word(user.wrapping_add(0x18c));
        // check_state via the live lock.
        let mutex = m.word(user.wrapping_add(0x24));
        let rc = self.live(&mut m, 0x176f2c, &[mutex]);
        if rc >= 0x80000000 {
            return rc;
        }
        let stopped = m.byte(user.wrapping_add(0x28)) == 6;
        let rc = self.live(&mut m, 0x176f5c, &[mutex]);
        if rc >= 0x80000000 {
            return rc;
        }
        let mut mapped = 0u32;
        if !stopped {
            // fn_18f490 tag map into the app records (p->tags).
            let rc = self.live(&mut m, 0x18f490, &[user, NESTED_RESULT, tags, 0]);
            if rc >= 0x80000000 {
                return rc;
            }
            let _ = mapped;
        }
        let _ = (local_samples, local_bytes, local_tags, mapped);
        // Local AudioBuffer on the ARM's own stack (C local): the result
        // struct lives at the nested 1a18c4 SP-relative slot, where the
        // chunk re-drive reads it (see NESTED_RESULT).
        m.put_word(NESTED_RESULT, 0);
        m.put_word(NESTED_RESULT.wrapping_add(4), local_bytes);
        m.put_word(NESTED_RESULT.wrapping_add(8), local_samples);
        m.put_word(NESTED_RESULT.wrapping_add(12), 0);
        m.put_word(NESTED_RESULT.wrapping_add(16), local_tags);
        // flags&2 presets request_flags; otherwise the request carries
        // the live request_flags word unchanged (C order).
        if flags & 2 != 0 {
            m.put_word(user.wrapping_add(0x150), 0xffff);
        }
        // Request block (C stack): command 0x10 (deliver + render),
        // request_flags, &local. The callback is 0x132124, which
        // rotates into 0x163a98; BackHost serves the rotation target.
        m.put_word(SEG7_REQ, 0x10);
        m.put_word(SEG7_REQ.wrapping_add(4), m.word(user.wrapping_add(0x150)));
        m.put_word(SEG7_REQ.wrapping_add(8), NESTED_RESULT);
        // Callback 1 (deliver + render) through lane back's BackHost.
        let status = match self.render_163a98(&mut m, user, SEG7_REQ) {
            Ok(s) => s,
            Err(()) => {
                return 0x80002001;
            }
        };
        let rc = if status != 0 {
            (status & 0x1fff) | 0x80002000
        } else {
            0
        };
        // request_flags = 2 (C order, before the flags&2 check).
        m.put_word(user.wrapping_add(0x150), 2);
        if rc >= 0x80000000 {
            if self.first_unbound.is_none() {
                self.first_unbound = Some(0x18f340);
            }
            return 0x80002001;
        }
        if flags & 2 != 0 {
            // flags&2: *samples = 0, *count = 0, done. No callback 2.
            m.put_word(sptr, 0);
            m.put_word(cptr, 0);
            return rc;
        }
        // check_state again (C order, live locks).
        let rc2 = self.live(&mut m, 0x176f2c, &[mutex]);
        if rc2 >= 0x80000000 {
            return rc2;
        }
        let stopped2 = m.byte(user.wrapping_add(0x28)) == 6;
        let rc2 = self.live(&mut m, 0x176f5c, &[mutex]);
        if rc2 >= 0x80000000 {
            return rc2;
        }
        if stopped2 {
            m.put_word(sptr, 0);
            m.put_word(cptr, 0);
            m.put_word(user.wrapping_add(0x18c), 0);
            m.put_word(user.wrapping_add(0x188), 0);
            m.put_word(tcptr, 0);
            return rc;
        }
        // Request block reused: command 8 (pop), flags 0, same local.
        m.put_word(SEG7_REQ, 8);
        m.put_word(SEG7_REQ.wrapping_add(4), 0);
        // Callback 2 (pop) through lane back's BackHost.
        let status2 = match self.render_163a98(&mut m, user, SEG7_REQ) {
            Ok(s) => s,
            Err(()) => {
                return 0x80002001;
            }
        };
        let rc = if status2 != 0 {
            (status2 & 0x1fff) | 0x80002000
        } else {
            0
        };
        // have_output (C order): *samples = local.samples, *count =
        // local.bytes/2, p->tags = local.tags, p->tag_count =
        // local.tag_count/40, *tag_count = same.
        let got_samples = m.word(NESTED_RESULT.wrapping_add(8));
        let got_count = m.word(NESTED_RESULT.wrapping_add(4)) >> 1;
        let got_tags = m.word(NESTED_RESULT.wrapping_add(16));
        let got_nmark = m.word(NESTED_RESULT.wrapping_add(12)) / 40;
        if rc >= 0x80000000 {
            return rc;
        }
        m.put_word(sptr, got_samples);
        m.put_word(cptr, got_count);
        m.put_word(user.wrapping_add(0x18c), got_tags);
        m.put_word(user.wrapping_add(0x188), got_nmark);
        m.put_word(tcptr, got_nmark);
        rc
    }
}

impl crate::engine_live::EmitHooksM for Seg7OutputSide<'_, '_, '_> {
    fn put_output_m(
        &mut self,
        user: u32,
        msg: u32,
        len_in: u32,
        n_in: u32,
        markers: (u32, u32),
        mem: &mut [u8],
    ) -> (u32, Option<u32>, u32, u32) {
        self.put_calls = self.put_calls.wrapping_add(1);
        let mut m = Memory { base: 0, bytes: mem };
        let o = self.synth.wrapping_add(0x84);
        let mut samples = m.word(o.wrapping_add(0x60));
        let mut count = len_in;
        let mut tag_count = n_in;
        let first = msg == 1;
        if first {
            self.log(&mut m, 4, 0x1534d0, &[]);
        }
        let buffer = m.word(user.wrapping_add(0x160));
        if buffer != 0 {
            if count != 0 || tag_count != 0 {
                m.put_word(user.wrapping_add(0x15c), 16);
                m.put_word(buffer.wrapping_add(4), count.wrapping_mul(2));
                m.put_word(buffer.wrapping_add(0xc), tag_count);
                let rc = self.live(&mut m, 0x18f490, &[user, buffer, markers.0, tag_count]);
                if rc >= 0x80000000 {
                    return (rc, None, 0, 0);
                }
                if msg & 2 != 0 {
                    return (0, None, 0, 0);
                }
            } else if msg & 2 != 0 {
                m.put_word(user.wrapping_add(0x15c), 2);
                return (0, Some(0), 0, 0);
            }
            let buf = m.word(buffer.wrapping_add(8));
            let bytes = m.word(user.wrapping_add(0x164));
            let tags = m.word(user.wrapping_add(0x168)) / 40;
            m.put_word(user.wrapping_add(0x18c), m.word(buffer.wrapping_add(16)));
            m.put_word(user.wrapping_add(0x188), tags);
            return (0, Some(buf), bytes / 2, tags);
        }
        // App path (ROM fn_18f6b8 else-branch: user buffer is null).
        // Replicates the ROM wrapper's sibling calls: Send/PUT-OUTPUT logs,
        // 18f6b8 setup inlined (local struct, lock, tag map), then the
        // 163a98 dispatch on the request kind (live 0x183258 notify + kind
        // path). Only the first delivery (kind 0x10) is reachable.
        // seg7 r8: speech_2/custom_2 deliver twice per interlude
        // (put_calls 2); process each delivery with its own args.
        let _ = self.put_calls;
        // Send / PUT OUTPUT logs (C order, before the app callback).
        // out->len/nmark were committed by emit before this hook ran.
        let out_len = m.word(o.wrapping_add(0x64));
        let out_nmark = m.word(o.wrapping_add(0x68));
        self.log(&mut m, 4, 0x153ad0, &[msg, out_len, out_nmark]);
        self.log(&mut m, 99, 0x153b00, &[out_len, out_nmark]);
        // SAMP log (C order, gated by out->len and the logger).
        // out->restart == 1 selects level 8 and is cleared.
        let ctx = m.word(self.synth.wrapping_add(0x84));
        let logger = m.word(ctx.wrapping_add(16));
        if out_len != 0 && logger != 0 {
            let mut resumed = 9u32;
            if m.byte(o.wrapping_add(0x4c)) == 1 {
                m.put_byte(o.wrapping_add(0x4c), 0);
                resumed = 8;
            }
            let freq = m.word(self.synth.wrapping_add(0x88));
            // varargs: SAMP, len, FREQ, freq/1000.
            m.put_word(SEG7_SAMP_VARARGS, 0x153b30);
            m.put_word(SEG7_SAMP_VARARGS.wrapping_add(4), out_len);
            m.put_word(SEG7_SAMP_VARARGS.wrapping_add(8), 0x153b28);
            m.put_word(SEG7_SAMP_VARARGS.wrapping_add(12), freq / 1000);
            let mut fhost = Seg7FmtHost {
                platform: &mut *self.platform,
                runtime: &mut *self.runtime,
                snap: self.snap,
            };
            crate::boot_part_b::fn_173fb4(
                &mut m,
                logger,
                resumed,
                0x153b38,
                SEG7_SAMP_VARARGS,
                SEG7_SAMP_PARAMS,
                0,
                &mut fhost,
            );
            let _ = resumed;
        }
        // Local AudioBuffer setup (C local): samples = out buf,
        // bytes = 2 * count, tag_count = n, tags = p->tags.
        let local_samples = samples;
        let local_bytes = count.wrapping_mul(2);
        let local_tags = m.word(user.wrapping_add(0x18c));
        // check_state via the live lock.
        let mutex = m.word(user.wrapping_add(0x24));
        let rc = self.live(&mut m, 0x176f2c, &[mutex]);
        if rc >= 0x80000000 {
            return (rc, None, 0, 0);
        }
        let stopped = m.byte(user.wrapping_add(0x28)) == 6;
        let rc = self.live(&mut m, 0x176f5c, &[mutex]);
        if rc >= 0x80000000 {
            return (rc, None, 0, 0);
        }
        let mut mapped = 0u32;
        if !stopped {
            // fn_18f490 (src/fn_18f490.c) inlined 1:1 on guest memory.
            // The app records (p->tags) are the output array; the
            // incoming markers are the input array. No fixed cap: the
            // ARM loops over all ntags (fresh-361 carries more than 8).
            let o_tags = m.word(user.wrapping_add(0x18c));
            let n = n_in;
            let mode = m.bytes[user as usize + 0x35];
            let mut limit_b = [0u8; 4];
            limit_b.copy_from_slice(&m.bytes[user as usize + 0x14c..user as usize + 0x150]);
            let limit = u32::from_le_bytes(limit_b);
            let mut j: u32 = 0;
            let mut k: u32 = 0;
            let mut tag_err = false;
            while k < n {
                let ta = markers.0.wrapping_add(k.wrapping_mul(24)) as usize;
                let oa = o_tags.wrapping_add(j.wrapping_mul(40)) as usize;
                // resetOutput(ob, j): f00, f18, f1c, f20.
                m.bytes[oa..oa + 4].copy_from_slice(&0u32.to_le_bytes());
                m.bytes[oa + 24..oa + 26].copy_from_slice(&0u16.to_le_bytes());
                m.bytes[oa + 28..oa + 32].copy_from_slice(&0u32.to_le_bytes());
                m.bytes[oa + 32..oa + 36].copy_from_slice(&0u32.to_le_bytes());
                let tag = u16::from_le_bytes([m.bytes[ta], m.bytes[ta + 1]]);
                let f04 = u32::from_le_bytes([
                    m.bytes[ta + 4],
                    m.bytes[ta + 5],
                    m.bytes[ta + 6],
                    m.bytes[ta + 7],
                ]);
                let f08 = u32::from_le_bytes([
                    m.bytes[ta + 8],
                    m.bytes[ta + 9],
                    m.bytes[ta + 10],
                    m.bytes[ta + 11],
                ]);
                let f0c = u32::from_le_bytes([
                    m.bytes[ta + 12],
                    m.bytes[ta + 13],
                    m.bytes[ta + 14],
                    m.bytes[ta + 15],
                ]);
                let f10 = u32::from_le_bytes([
                    m.bytes[ta + 16],
                    m.bytes[ta + 17],
                    m.bytes[ta + 18],
                    m.bytes[ta + 19],
                ]);
                let f14 = u32::from_le_bytes([
                    m.bytes[ta + 20],
                    m.bytes[ta + 21],
                    m.bytes[ta + 22],
                    m.bytes[ta + 23],
                ]);
                let flags: u16 = match tag {
                    0x4000 => {
                        if f08 == 0 {
                            k = k.wrapping_add(1);
                            continue;
                        }
                        1
                    }
                    0x28 => {
                        if crate::frontend_runtime::utf16_to_ascii_in_place(
                            &mut m.bytes[..],
                            f14 as usize,
                        )
                        .is_err()
                        {
                            tag_err = true;
                            break;
                        }
                        m.bytes[oa + 36..oa + 40].copy_from_slice(&f14.to_le_bytes());
                        0x400
                    }
                    0x21 => {
                        m.bytes[oa + 24..oa + 26].copy_from_slice(&(f14 as u16).to_le_bytes());
                        4
                    }
                    0x1d => {
                        let v = crate::frontend_runtime::parse_ascii_int_in_place(
                            &m.bytes[..],
                            f14 as usize,
                        ) as u32;
                        m.bytes[oa + 28..oa + 32].copy_from_slice(&v.to_le_bytes());
                        8
                    }
                    1 => 2,
                    8 => {
                        m.bytes[oa + 32..oa + 36].copy_from_slice(&f14.to_le_bytes());
                        0x10
                    }
                    0x19 => {
                        m.bytes[oa + 32..oa + 36].copy_from_slice(&f14.to_le_bytes());
                        0x20
                    }
                    _ => {
                        k = k.wrapping_add(1);
                        continue;
                    }
                };
                m.bytes[oa + 4..oa + 6].copy_from_slice(&flags.to_le_bytes());
                let mut wf08 = f04;
                let mut wf0c = f08;
                if mode == 2 {
                    if flags == 1 && wf08 < limit {
                        let d = limit - wf08;
                        wf0c = if d < wf0c { wf0c - d } else { 0 };
                    }
                    wf08 = if wf08 > limit { wf08 - limit } else { 0 };
                }
                m.bytes[oa + 8..oa + 12].copy_from_slice(&wf08.to_le_bytes());
                m.bytes[oa + 12..oa + 16].copy_from_slice(&wf0c.to_le_bytes());
                m.bytes[oa + 16..oa + 20].copy_from_slice(&f0c.to_le_bytes());
                m.bytes[oa + 20..oa + 24].copy_from_slice(&f10.to_le_bytes());
                j = j.wrapping_add(1);
                k = k.wrapping_add(1);
            }
            if tag_err {
                return (0x80002001, None, 0, 0);
            }
            mapped = j;
            let _ = (local_samples, local_bytes, local_tags, mapped);
        }
        // Local AudioBuffer on the ARM's own stack (C local): kind@+0,
        // bytes@+4, samples@+8, tag_count@+12, tags@+16, at the nested
        // 1a18c4 SP-relative slot (see NESTED_RESULT). Stopped
        // (state 6) zeroes bytes/tag_count but still delivers.
        m.put_word(NESTED_RESULT, 0);
        m.put_word(
            NESTED_RESULT.wrapping_add(4),
            if stopped { 0 } else { local_bytes },
        );
        m.put_word(NESTED_RESULT.wrapping_add(8), local_samples);
        m.put_word(
            NESTED_RESULT.wrapping_add(12),
            if stopped { 0 } else { mapped },
        );
        m.put_word(NESTED_RESULT.wrapping_add(16), local_tags);
        // flags&2 presets request_flags (dead on this request: msg 0x20).
        if msg & 2 != 0 {
            m.put_word(user.wrapping_add(0x150), 0xffff);
        }
        // Request block (C stack): command 0x10 (deliver + render),
        // request_flags, &local. The callback is 0x132124, which
        // rotates into 0x163a98; BackHost serves the rotation target.
        m.put_word(SEG7_REQ, 0x10);
        m.put_word(
            SEG7_REQ.wrapping_add(4),
            m.word(user.wrapping_add(0x150)),
        );
        m.put_word(SEG7_REQ.wrapping_add(8), NESTED_RESULT);
        // Callback 1 (deliver + render) through lane back's BackHost.
        // A stop/fault leaves the out words as the ARM had them: emit
        // stores this tuple back, so None/0 here would trash them.
        let status = match self.render_163a98(&mut m, user, SEG7_REQ) {
            Ok(s) => s,
            Err(()) => {
                return (
                    0x80002001,
                    Some(m.word(o.wrapping_add(0x60))),
                    m.word(o.wrapping_add(0x64)),
                    m.word(o.wrapping_add(0x68)),
                )
            }
        };
        let mut rc = if status != 0 {
            (status & 0x1fff) | 0x80002000
        } else {
            0
        };
        // request_flags = 2 (C order, before the flags&2 check).
        m.put_word(user.wrapping_add(0x150), 2);
        if rc >= 0x80000000 {
            // Engine-stop path (status 0x40380000 class): never taken
            // on this request; the stop entry is unbound live.
            if self.first_unbound.is_none() {
                self.first_unbound = Some(0x18f340);
            }
            return (0x80002001, None, 0, 0);
        }
        if msg & 2 != 0 {
            m.put_word(o.wrapping_add(0x60), 0);
            m.put_word(o.wrapping_add(0x64), 0);
            return (0, Some(0), 0, m.word(o.wrapping_add(0x68)));
        }
        // check_state again (C order, live locks).
        let rc2 = self.live(&mut m, 0x176f2c, &[mutex]);
        if rc2 >= 0x80000000 {
            return (rc2, None, 0, 0);
        }
        let stopped2 = m.byte(user.wrapping_add(0x28)) == 6;
        let rc2 = self.live(&mut m, 0x176f5c, &[mutex]);
        if rc2 >= 0x80000000 {
            return (rc2, None, 0, 0);
        }
        if stopped2 {
            // Never taken here: zero samples/count/tags/tag_count.
            m.put_word(o.wrapping_add(0x60), 0);
            m.put_word(o.wrapping_add(0x64), 0);
            m.put_word(user.wrapping_add(0x18c), 0);
            m.put_word(user.wrapping_add(0x188), 0);
            return (0, Some(0), 0, 0);
        }
        // Request block reused: command 8 (pop), flags 0, same local.
        m.put_word(SEG7_REQ, 8);
        m.put_word(SEG7_REQ.wrapping_add(4), 0);
        // Callback 2 (pop) through lane back's BackHost.
        let status2 = match self.render_163a98(&mut m, user, SEG7_REQ) {
            Ok(s) => s,
            Err(()) => {
                return (
                    0x80002001,
                    Some(m.word(o.wrapping_add(0x60))),
                    m.word(o.wrapping_add(0x64)),
                    m.word(o.wrapping_add(0x68)),
                )
            }
        };
        rc = if status2 != 0 {
            (status2 & 0x1fff) | 0x80002000
        } else {
            0
        };
        // have_output (C order): *samples = local.samples, *count =
        // local.bytes/2, p->tags = local.tags, p->tag_count =
        // local.tag_count/40, *tag_count = same. The *samples/**count
        // words alias out->60/64, which emit stores from this tuple.
        let got_samples = m.word(NESTED_RESULT.wrapping_add(8));
        let got_count = m.word(NESTED_RESULT.wrapping_add(4)) >> 1;
        let got_tags = m.word(NESTED_RESULT.wrapping_add(16));
        let got_nmark = m.word(NESTED_RESULT.wrapping_add(12)) / 40;
        if rc >= 0x80000000 {
            // Never taken here: p->tags stays, out words updated.
            return (rc, Some(got_samples), got_count, m.word(o.wrapping_add(0x68)));
        }
        m.put_word(user.wrapping_add(0x18c), got_tags);
        m.put_word(user.wrapping_add(0x188), got_nmark);
        return (0, Some(got_samples), got_count, got_nmark);
    }
    fn get_avail_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> (u32, u32, u8) {
        let mut m = Memory { base: 0, bytes: mem };
        let entry = self.vt_entry(&m, 0x48);
        m.put_word(SEG7_OUT_SCRATCH, 0);
        m.put_byte(SEG7_OUT_SCRATCH + 4, 0);
        let rc = self.live(&mut m, entry, &[a, b, SEG7_OUT_SCRATCH, SEG7_OUT_SCRATCH + 4]);
        let avail = m.word(SEG7_OUT_SCRATCH);
        let eos = m.byte(SEG7_OUT_SCRATCH + 4);
        self.eos = eos;
        (rc, avail, eos)
    }
    fn get_buf_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> (u32, Option<u32>, u32) {
        let mut m = Memory { base: 0, bytes: mem };
        let o = self.synth.wrapping_add(0x84);
        let entry = self.vt_entry(&m, 0x2c);
        m.put_word(SEG7_OUT_SCRATCH + 8, 0);
        m.put_word(SEG7_OUT_SCRATCH + 12, 0);
        let rc = self.live(&mut m, entry, &[a, b, SEG7_OUT_SCRATCH + 8, SEG7_OUT_SCRATCH + 12]);
        let buf = m.word(SEG7_OUT_SCRATCH + 8);
        let len = m.word(SEG7_OUT_SCRATCH + 12);
        if a == m.word(o.wrapping_add(0x44)) && b == m.word(o.wrapping_add(0x48)) {
            self.mk_buf = buf;
            self.mk_len = len;
        } else {
            self.pcm_avail = len;
        }
        (
            rc,
            if buf == 0 { None } else { Some(buf) },
            len,
        )
    }
    fn get_pos_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> (u32, u32) {
        let mut m = Memory { base: 0, bytes: mem };
        let entry = self.vt_entry(&m, 0x44);
        m.put_word(SEG7_OUT_SCRATCH + 16, 0);
        let rc = self.live(&mut m, entry, &[a, b, SEG7_OUT_SCRATCH + 16]);
        let pos = m.word(SEG7_OUT_SCRATCH + 16);
        if (self.pos_calls as usize) < self.pos_vals.len() {
            self.pos_vals[self.pos_calls as usize] = pos;
        }
        self.pos_calls = self.pos_calls.wrapping_add(1);
        (rc, pos)
    }
    fn consume_m(&mut self, a: u32, b: u32, n: u32, mem: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes: mem };
        let entry = self.vt_entry(&m, 0x30);
        self.live(&mut m, entry, &[a, b, n])
    }
    fn realloc_markers_m(&mut self, mem: &mut [u8]) -> Option<u32> {
        // Single growth on this request (the ARM grows 0 -> 6 markers
        // once via fn_1963e0). The count below is the pump's own first
        // iteration count, recomputed from the recorded stream
        // observations; anything else errors instead of guessing.
        if self.growths != 0 || self.put_calls != 0 || self.pos_calls != 2 {
            return None;
        }
        self.growths = 1;
        let mut m = Memory { base: 0, bytes: mem };
        let o = self.synth.wrapping_add(0x84);
        let bits = m.word(o.wrapping_add(8));
        let out_len = m.word(o.wrapping_add(0x64));
        let nmark = m.word(o.wrapping_add(0x68));
        let flags = m.word(self.synth.wrapping_add(0x34));
        let eos = self.eos;
        let mk = self.mk_buf;
        let mk_end = mk.wrapping_add(self.mk_len);
        let mut copy_len = self.pcm_avail;
        let cap = out_len.wrapping_mul(bits) >> 3;
        if copy_len >= cap {
            copy_len = cap;
        }
        let base = self.pos_vals[1];
        let end = if bits == 16 {
            base.wrapping_add(copy_len) >> 1
        } else if bits == 8 {
            base.wrapping_add(copy_len)
        } else {
            crate::frontend::divmod_u32(base.wrapping_add(copy_len) << 3, bits).0
        };
        let pos = self.pos_vals[0];
        let mut min_pos = if bits == 16 {
            pos >> 1
        } else if bits == 8 {
            pos
        } else {
            crate::frontend::divmod_u32(pos << 3, bits).0
        };
        let mut n = 0u32;
        let mut cur = mk;
        let mut max_pos = min_pos;
        let mut cnt = 0u32;
        let mut mk_hit = 0u32;
        let mut all = false;
        let bytes = &m.bytes;
        loop {
            mk_hit = crate::frontend::filter_markers(bytes, flags, cur, mk_end, Some(&mut cnt));
            if mk_hit == 0 {
                all = true;
                break;
            }
            let at = mk_hit.wrapping_add(12) as usize;
            let mpos = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            if cnt.wrapping_add(n) > nmark || (eos == 0 && mpos >= end) {
                break;
            }
            n = n.wrapping_add(1);
            cur = mk_hit.wrapping_add(24);
            if mpos > max_pos {
                max_pos = mpos;
            }
        }
        if !all {
            let at = mk_hit.wrapping_add(12) as usize;
            let mpos = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            if mpos < end {
                if mpos >= max_pos {
                    max_pos = mpos;
                }
                let t = if bits == 16 {
                    max_pos.wrapping_mul(2).wrapping_sub(base) as i32
                } else if bits == 8 {
                    max_pos.wrapping_sub(base) as i32
                } else {
                    bits.wrapping_mul(max_pos).wrapping_sub(base) as i32
                };
                if t <= 0 && nmark < cnt {
                    loop {
                        mk_hit = crate::frontend::filter_markers(bytes, flags, cur, mk_end, None);
                        if mk_hit == 0 {
                            all = true;
                            break;
                        }
                        if nmark <= n {
                            break;
                        }
                        n = n.wrapping_add(1);
                        cur = mk_hit.wrapping_add(24);
                    }
                }
            }
        }
        if n == 0 {
            return None;
        }
        let heap = m.word(m.word(self.synth.wrapping_add(0x84)).wrapping_add(4));
        let old = m.word(o.wrapping_add(0x78));
        let size = n.wrapping_mul(24);
        match crate::engine_live::call_live_with_runtime_m(
            &mut m,
            0x1963e0,
            &[heap, old, size],
            &mut *self.platform,
            &mut *self.runtime,
            self.snap,
        ) {
            Ok(rc) if rc != 0 && rc < 0x80000000 => Some(rc),
            _ => None,
        }
    }
}

/// The 0x183258 event-queue append. Shares the at-capacity behaviour with
/// engine_live through boot_part_b2::fn_183258_live (one implementation:
/// count >= reserved with unowned growth silently drops, returning 0 with
/// no write and no end advance, exactly the ARM's +0x54/0x68 path whose
/// void return fn_163a98 ignores). The tick rides the last queue record,
/// matching the ARM's constant-tick run at every green 1530d0 boundary.
fn seg7_notify_event(
    m: &mut Memory<'_>,
    p: u32,
    kind: u32,
    vfp: engine::NotificationRegisters,
) -> u32 {
    let vector = p.wrapping_add(0x61cc);
    let begin = m.word(vector.wrapping_add(4));
    let end = m.word(vector.wrapping_add(8));
    let tick = if end != 0 {
        let last = end.wrapping_sub(32);
        if last >= begin {
            m.word(last.wrapping_add(8)) as u64
                | ((m.word(last.wrapping_add(12)) as u64) << 32)
        } else {
            0
        }
    } else {
        0
    };
    crate::boot_part_b2::fn_183258_live(m, p, kind, vfp.s16, vfp.s17, tick)
}

/// Terminal nested runtime: the capture hook's guest effects ride
/// engine_live's own 0xc72018 arm; the harness PCM sink behind it has
/// no guest effect, so it answers 0.
struct Seg7NestedSink {
    registers: engine::NotificationRegisters,
}

impl crate::engine_live::RuntimeServicesM for Seg7NestedSink {
    fn invoke_m(
        &mut self,
        m: &mut Memory<'_>,
        entry: u32,
        a: &[u32],
    ) -> Result<u32, engine::Error> {
        match entry {
            0x183258 => Ok(seg7_notify_event(m, a[0], a[1], self.registers)),
            0xc72018 => Ok(0),
            _ => Err(engine::Error::UnsupportedStage { entry }),
        }
    }
}

/// Format host for the SAMP log: every service rides the live dispatcher.
struct Seg7FmtHost<'p, 'r> {
    platform: &'p mut dyn engine::HeapPlatform,
    runtime: &'r mut dyn crate::engine_live::RuntimeServicesM,
    snap: &'r [u8],
}
impl crate::boot_part_b::Host for Seg7FmtHost<'_, '_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        match crate::engine_live::call_live_with_runtime_m(
            m,
            f,
            a,
            &mut *self.platform,
            &mut *self.runtime,
            self.snap,
        ) {
            Ok(rc) => rc,
            Err(_) => 0x80002001,
        }
    }
    fn thread_id(&mut self) -> u32 {
        self.platform.thread_id()
    }
    fn real_log_args(&mut self) -> bool {
        true
    }
}
impl crate::boot_part_b::FormatHost for Seg7FmtHost<'_, '_> {}

/// Runtime services for the segment: the 0x183258 event-queue append
/// with the caller's VFP registers, and the 0xc72018 capture hook
/// through engine_live's own arm with a terminal sink. The VFP values
/// are per-notify-site caller inputs: the scheduler's own notifies
/// carry s16 0x47dd / s17 0 (round-22 wire record), while this lane's
/// 1530d0 app-path notifies carry s16 0 / s17 0 on the full run-up
/// (the 1530d0 return image; the emulator driver presets are loader
/// state that VFP writes before the nested notify already clobbered).
pub struct Seg7Runtime<'p, 'r> {
    pub registers: engine::NotificationRegisters,
    pub platform: &'p mut dyn engine::HeapPlatform,
    pub snap: &'r [u8],
}

/// Runtime wrapper that stops the BackHost-routed nested dispatch at
/// the 0x1a18c4 render entry: everything nests normally except the
/// render itself, which reports unbound. The guest arena at the stop is
/// the live nested-entry state (notify, ready check and scratch words
/// already applied; no render writes yet).
pub struct StopAtRender<'a> {
    pub inner: &'a mut dyn crate::engine_live::RuntimeServicesM,
    pub stopped: bool,
}

impl crate::engine_live::RuntimeServicesM for StopAtRender<'_> {
    fn invoke_m(
        &mut self,
        m: &mut Memory<'_>,
        entry: u32,
        args: &[u32],
    ) -> Result<u32, engine::Error> {
        if entry == 0x1a18c4 {
            self.stopped = true;
            return Err(engine::Error::UnsupportedStage { entry });
        }
        self.inner.invoke_m(m, entry, args)
    }
}

impl crate::engine_live::RuntimeServicesM for Seg7Runtime<'_, '_> {
    fn invoke_m(
        &mut self,
        m: &mut Memory<'_>,
        entry: u32,
        a: &[u32],
    ) -> Result<u32, engine::Error> {
        match entry {
            0x183258 => Ok(seg7_notify_event(m, a[0], a[1], self.registers)),
            0xc72018 => {
                let mut sink = Seg7NestedSink {
                    registers: self.registers,
                };
                crate::engine_live::call_live_with_runtime_m(
                    m,
                    entry,
                    a,
                    &mut *self.platform,
                    &mut sink,
                    self.snap,
                )
            }
            _ => Err(engine::Error::UnsupportedStage { entry }),
        }
    }
}

/// One live segment-7 root call. `args` carries the registers plus the
/// stack words exactly like the scheduler passes them (7 entries). The
/// coordinator merges these arms into engine_live::Host; until then they
/// run here over the same public dispatcher. The 0x1530d0 arm needs the
/// caller's render context for its nested render; without one it reports
/// the nested render entry as unbound.
pub fn call_seg7(
    m: &mut Memory<'_>,
    entry: u32,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn crate::engine_live::RuntimeServicesM,
    snap: &[u8],
) -> Result<u32, engine::Error> {
    call_seg7_with_render(m, entry, args, platform, runtime, snap, None)
}

/// One live segment-7 root call with the caller's render context for
/// the 0x1530d0 nested render (lane us's chain test keeps the 6-arg
/// form, which runs without a render context and reports the nested
/// render entry as unbound, as in round 1).
pub fn call_seg7_with_render(
    m: &mut Memory<'_>,
    entry: u32,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn crate::engine_live::RuntimeServicesM,
    snap: &[u8],
    render: Option<RenderCtx<'_>>,
) -> Result<u32, engine::Error> {
    if m.base != 0 {
        return Err(engine::Error::InvalidSnapshot);
    }
    match entry {
        0x22d6dc | 0x231ba0 => crate::engine_live::call_live_with_runtime_m(
            m, entry, args, platform, runtime, snap,
        ),
        0x1e3b14 => {
            let mut a5 = [0u32; 5];
            let n = args.len().min(5);
            a5[..n].copy_from_slice(&args[..n]);
            let mut regions = [engine::Region {
                base: m.base,
                bytes: &mut *m.bytes,
            }];
            let mut sparse =
                engine::Memory::new(&mut regions).map_err(|_| engine::Error::InvalidSnapshot)?;
            let mut bridge = Seg7Bridge {
                platform,
                runtime,
                snap,
            };
            crate::engine_components::process(&mut sparse, entry, a5, SEG7_FRAME_1E3B14, &mut bridge)
        }
        0x18f6b8 => {
            // seg7 r11: the finish-path direct flush (fn_1561b8 +0x39c).
            // Six args (regs plus stack tags/tcptr): user, flags, sptr,
            // cptr, tags, tcptr. Only the app path (user buffer null)
            // with flags&2 is bound here; anything else reports the
            // entry as unbound so the caller falls back to the back-lane
            // port. The nested render needs the caller's render context;
            // without one it reports the nested render entry as unbound.
            if args.len() < 6 {
                return Err(engine::Error::InvalidSnapshot);
            }
            let (user, flags, sptr, cptr, tags, tcptr) =
                (args[0], args[1], args[2], args[3], args[4], args[5]);
            if m.word(user.wrapping_add(0x160)) != 0 {
                return Err(engine::Error::UnsupportedStage { entry });
            }
            let mut side = Seg7OutputSide {
                platform,
                runtime,
                snap,
                synth: 0,
                render,
                put_calls: 0,
                pcm_avail: 0,
                mk_buf: 0,
                mk_len: 0,
                pos_calls: 0,
                pos_vals: [0; 8],
                eos: 0,
                growths: 0,
                first_unbound: None,
            };
            let rc = side.app_pump_m(m, user, flags, sptr, cptr, tags, tcptr);
            if rc == 0x80002001 {
                if let Some(f) = side.first_unbound {
                    return Err(engine::Error::UnsupportedStage { entry: f });
                }
            }
            if rc < 0x80000000 {
                if let Some(r) = side.render.as_ref() {
                    let n = r.written;
                    match side.runtime.emit_pcm_m(&r.pcm[..n]) {
                        Ok(()) => {}
                        Err(engine::Error::UnsupportedStage { .. })
                        | Err(engine::Error::MissingStreamAdapter { .. }) => {}
                        Err(e) => return Err(e),
                    }
                }
            }
            Ok(rc)
        }
        0x1530d0 => {
            let p = args.first().copied().unwrap_or(0);
            let sent = match args.get(1).copied().unwrap_or(0) {
                0 => None,
                s => Some(s),
            };
            let mut side = Seg7OutputSide {
                platform,
                runtime,
                snap,
                synth: p,
                render,
                put_calls: 0,
                pcm_avail: 0,
                mk_buf: 0,
                mk_len: 0,
                pos_calls: 0,
                pos_vals: [0; 8],
                eos: 0,
                growths: 0,
                first_unbound: None,
            };
            let rc = crate::engine_live::emit_output_m(m.bytes, p, sent, &mut side);
            if rc == 0x80002001 {
                if let Some(f) = side.first_unbound {
                    return Err(engine::Error::UnsupportedStage { entry: f });
                }
            }
            // wire-a r26: thread the nested render's PCM to the caller's
            // sink. The count stays inside this composition
            // (render.written); only the samples cross, through the
            // runtime trait. Runtimes without a caller sink keep the
            // trait default, which reports unbound and drops the samples
            // here, so the certified arena images are unchanged. A
            // too-small caller buffer fails the pump with Capacity.
            if rc < 0x80000000 {
                if let Some(r) = side.render.as_ref() {
                    let n = r.written;
                    match side.runtime.emit_pcm_m(&r.pcm[..n]) {
                        Ok(()) => {}
                        Err(engine::Error::UnsupportedStage { .. })
                        | Err(engine::Error::MissingStreamAdapter { .. }) => {}
                        Err(e) => return Err(e),
                    }
                }
            }
            Ok(rc)
        }
        _ => Err(engine::Error::UnsupportedStage { entry }),
    }
}

/// The next call the scheduler makes. Either the next component root with
/// its live-derived args, or the 0x1530d0 finish pump.
pub struct Seg7Next {
    pub entry: u32,
    pub args: [u32; 7],
}

/// The scheduler step after a root returns (taken path of fn_15429c).
/// Reads the scheduler's own state live (comp table, aux pair, extra
/// bytes, counters, saved stage) and issues the same live calls the ARM
/// issues: aux-notify, the "returned" log, bookkeeping, the "Calling"
/// log. Returns the next call with live-derived args.
pub fn step_seg7(
    m: &mut Memory<'_>,
    rc: u32,
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn crate::engine_live::RuntimeServicesM,
    snap: &[u8],
) -> Result<Seg7Next, engine::Error> {
    if m.base != 0 {
        return Err(engine::Error::InvalidSnapshot);
    }
    // sched_sp is the scheduler's frame sp: fn_15429c already pushed 12
    // regs and subbed 0x440, so every call it makes records this sp.
    // Its saved r0 (p) sits at frame+0x440, r1 (flush) at +0x444.
    let sched_sp: u32 = 0xe0ff938;
    let p = m.word(sched_sp.wrapping_add(0x440));
    let ctx = m.word(p.wrapping_add(0x84));
    let logger = m.word(ctx.wrapping_add(16));
    let cs = m.word(p.wrapping_add(0x7c));
    let count = m.half(cs.wrapping_add(0xc)) as u32;
    let cur = m.half(p.wrapping_add(0xd8)) as u32;
    let extra_base = m.word(p.wrapping_add(0x84).wrapping_add(0x58));
    let extra = m.byte(extra_base.wrapping_add(cur));
    let live = |m: &mut Memory<'_>,
                platform: &mut dyn engine::HeapPlatform,
                runtime: &mut dyn crate::engine_live::RuntimeServicesM,
                entry: u32,
                args: &[u32]|
     -> Result<u32, engine::Error> {
        crate::engine_live::call_live_with_runtime_m(m, entry, args, platform, runtime, snap)
    };
    // procCb (p+0x20) must be null on this stretch.
    if m.word(p.wrapping_add(0x20)) != 0 {
        return Err(engine::Error::UnsupportedStage { entry: 0x154f78 });
    }
    // Aux-notify slot (p+0x54 -> +0x64/+0x68).
    let t = m.word(p.wrapping_add(0x54));
    if m.word(t.wrapping_add(0x64)) != 0 {
        let target = m.word(t.wrapping_add(0x68));
        if target != 0x15daa8 {
            return Err(engine::Error::UnsupportedStage { entry: target });
        }
        let aux0 = m.word(p.wrapping_add(0xa0));
        let aux1 = m.word(p.wrapping_add(0xa4));
        m.put_byte(SEG7_STEP_SLOT, 0);
        let arc = live(m, &mut *platform, &mut *runtime, target, &[aux0, aux1, SEG7_STEP_SLOT])?;
        if arc >= 0x80000000 {
            return Err(engine::Error::EngineStatus(arc));
        }
        // Status byte clear skips the observer path on this stretch.
        if m.byte(SEG7_STEP_SLOT) != 0 {
            return Err(engine::Error::UnsupportedStage { entry: 0x154fe4 });
        }
    }
    if rc >= 0x80000000 {
        return Err(engine::Error::EngineStatus(rc));
    }
    // "Process for %s returned %s %x".
    let comps = m.word(cs.wrapping_add(8));
    let comp = m.word(comps.wrapping_add(cur.wrapping_mul(4)));
    let name = m.word(comp.wrapping_add(4));
    let vname = m.word(SEG7_VNAME_TABLE.wrapping_add((extra as u32).wrapping_mul(4)));
    m.put_word(SEG7_STEP_VARARGS, name);
    m.put_word(SEG7_STEP_VARARGS + 4, vname);
    m.put_word(SEG7_STEP_VARARGS + 8, rc);
    let lrc = live(
        m,
        &mut *platform,
        &mut *runtime,
        0x1738e8,
        &[logger, SEG7_LOG_MODULE, 4, 0, SEG7_FMT_RETURNED, SEG7_STEP_VARARGS],
    )?;
    if lrc >= 0x80000000 {
        return Err(engine::Error::EngineStatus(lrc));
    }
    // Voice-switch body behind extra==3 is not taken on this stretch.
    if extra == 3 && m.byte(p.wrapping_add(0x80)) != 2 {
        return Err(engine::Error::UnsupportedStage { entry: 0x1550bc });
    }
    if extra == 4 {
        return Err(engine::Error::UnsupportedStage { entry: 0x1554fc });
    }
    if extra != 0 {
        // Advance: anStage++ only while stages remain after it.
        if count > cur.wrapping_add(1) {
            m.put_half(p.wrapping_add(0xd8), (cur + 1) as u16);
        } else {
            return finish_check(m, p, cur, count, extra_base, platform, runtime, snap);
        }
    } else {
        return finish_check(m, p, cur, count, extra_base, platform, runtime, snap);
    }
    // Loop head.
    if m.byte(p.wrapping_add(0x30)) == 6 {
        return Err(engine::Error::UnsupportedStage { entry: 0x1542f0 });
    }
    m.put_word(p.wrapping_add(0xd4), m.word(p.wrapping_add(0xd4)).wrapping_add(1));
    let limit = m.word(p.wrapping_add(0x38));
    let pending = m.word(p.wrapping_add(0xd4));
    if limit != 0 && limit < pending {
        return Err(engine::Error::UnsupportedStage { entry: 0x154310 });
    }
    let cur2 = m.half(p.wrapping_add(0xd8)) as u32;
    // flush==1 && cur==saved re-enters the process block; not the case
    // past the first stage, and saved rides the scheduler frame.
    let saved = m.half(sched_sp.wrapping_add(0x414)) as u32;
    let flush = m.word(sched_sp.wrapping_add(0x444));
    if flush == 1 && cur2 == saved {
        return Err(engine::Error::UnsupportedStage { entry: 0x1543a0 });
    }
    // "Calling Process for %s"; the stage is fresh (extra already 0).
    m.put_byte(extra_base.wrapping_add(cur2), 0);
    let comp2 = m.word(comps.wrapping_add(cur2.wrapping_mul(4)));
    let name2 = m.word(comp2.wrapping_add(4));
    m.put_word(SEG7_STEP_VARARGS, name2);
    let lrc = live(
        m,
        &mut *platform,
        &mut *runtime,
        0x1738e8,
        &[logger, SEG7_LOG_MODULE, 4, 0, SEG7_FMT_CALLING, SEG7_STEP_VARARGS],
    )?;
    if lrc >= 0x80000000 {
        return Err(engine::Error::EngineStatus(lrc));
    }
    if m.word(cs.wrapping_add(0x10)) != 0 {
        return Err(engine::Error::UnsupportedStage { entry: 0x1543e0 });
    }
    if m.byte(p.wrapping_add(0x80)) == 2 {
        return Err(engine::Error::UnsupportedStage { entry: 0x154400 });
    }
    let h0 = m.word(comp2.wrapping_add(0xc));
    let h1 = m.word(comp2.wrapping_add(0x10));
    let aux0 = m.word(p.wrapping_add(0xa0));
    let aux1 = m.word(p.wrapping_add(0xa4));
    let active = extra_base.wrapping_add(cur2);
    Ok(Seg7Next {
        entry: m.word(m.word(comp2.wrapping_add(8)).wrapping_add(0x34)),
        args: [h0, h1, aux0, aux1, active, 0, 0],
    })
}

/// The finish_stage fork: last stage with output pending calls 0x1530d0.
/// The rewind loop past it belongs to the render preamble (lane back).
fn finish_check(
    m: &mut Memory<'_>,
    p: u32,
    cur: u32,
    count: u32,
    extra_base: u32,
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn crate::engine_live::RuntimeServicesM,
    snap: &[u8],
) -> Result<Seg7Next, engine::Error> {
    if cur.wrapping_add(1) != count {
        return Err(engine::Error::UnsupportedStage { entry: 0x155610 });
    }
    if m.byte(extra_base.wrapping_add(cur)) == 0 {
        return Err(engine::Error::UnsupportedStage { entry: 0x155654 });
    }
    if m.word(p.wrapping_add(0x84).wrapping_add(0x3c)) == 0 {
        return Err(engine::Error::UnsupportedStage { entry: 0x155660 });
    }
    Ok(Seg7Next {
        entry: 0x1530d0,
        args: [p, 0, 0, 0, 0, 0, 0],
    })
}

/// The scheduler tail after the 0x1530d0 finish pump returns (taken
/// path of fn_15429c from 0x155570 on): store rc, rewind anStage to
/// the newest nonzero extra byte, rejoin the loop head, and call the
/// next root with live-derived args. Unlike the between-roots step
/// there is no aux-notify and no "returned" log on this path: the
/// single 0x1738e8 the ARM issues here (lr 0x1543d8) is the "Calling"
/// log for the next root. Every branch condition is evaluated live;
/// an untaken path returns UnsupportedStage naming its site.
pub fn step_seg7_post_pump(
    m: &mut Memory<'_>,
    rc: u32,
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn crate::engine_live::RuntimeServicesM,
    snap: &[u8],
) -> Result<Seg7Next, engine::Error> {
    if m.base != 0 {
        return Err(engine::Error::InvalidSnapshot);
    }
    let sched_sp: u32 = 0xe0ff938;
    let p = m.word(sched_sp.wrapping_add(0x440));
    // 0x155570: an error rc leaves through 0x156044.
    if rc >= 0x80000000 {
        return Err(engine::Error::UnsupportedStage { entry: 0x156044 });
    }
    m.put_word(sched_sp.wrapping_add(0x28), rc);
    let o = p.wrapping_add(0x84);
    let extra_base = m.word(o.wrapping_add(0x58));
    // 0x155608 search: rewind anStage while its extra byte is 0.
    // A 2 or 3 (or anStage 0) ends the search; the flush==1 saved
    // comparison path is not taken on this request.
    let mut an = m.half(o.wrapping_add(0x54)) as u32;
    loop {
        let e = m.byte(extra_base.wrapping_add(an));
        if e == 2 || e == 3 || an == 0 {
            break;
        }
        if m.word(sched_sp.wrapping_add(0x444)) == 1 {
            return Err(engine::Error::UnsupportedStage { entry: 0x155640 });
        }
        an = an.wrapping_sub(1);
        m.put_half(o.wrapping_add(0x54), an as u16);
    }
    // after_switch (C fn_15429c, src/fn_151914.c): the rewind drained
    // to stage 0. extra[0]==2 means stage 0 (xcoder) still has output
    // pending and would be re-entered through the loop head (0x1542ec);
    // unobserved on this path, staged. Otherwise the scheduler checks
    // the text input and the all-processed flag: pp closed (pp.a == 0,
    // out+0x28) plus every extra byte == 1 falls into loop_check with
    // ready == 1 and returns rc (done). The live end-of-speech flush
    // hits exactly that (flush word 0, pp.a 0, all 26 extras 1), so it
    // leaves through 0x156044 with rc 0. Text still open or a stage
    // unmarked would re-enter the loop; both are staged, not guessed.
    if an == 0 {
        if m.byte(extra_base) == 2 {
            return Err(engine::Error::UnsupportedStage { entry: 0x1542ec });
        }
        if m.word(o.wrapping_add(0x28)) != 0 {
            return Err(engine::Error::UnsupportedStage { entry: 0x155d8c });
        }
        let cs0 = m.word(p.wrapping_add(0x7c));
        let n0 = m.half(cs0.wrapping_add(0x0c)) as u32;
        for i in 0..n0 {
            if m.byte(extra_base.wrapping_add(i)) != 1 {
                return Err(engine::Error::UnsupportedStage { entry: 0x1542ec });
            }
        }
        return Err(engine::Error::UnsupportedStage { entry: 0x156044 });
    }
    if m.byte(extra_base.wrapping_add(an)) == 3 {
        return Err(engine::Error::UnsupportedStage { entry: 0x155d74 });
    }
    // 0x156044: the flush==2 and aux-armed paths are not taken.
    if m.word(sched_sp.wrapping_add(0x444)) == 2 {
        return Err(engine::Error::UnsupportedStage { entry: 0x156060 });
    }
    if m.word(sched_sp.wrapping_add(0x42c)) != 0 {
        return Err(engine::Error::UnsupportedStage { entry: 0x1560d4 });
    }
    // Loop head 0x1542ec: state, pending++, limit.
    if m.byte(p.wrapping_add(0x30)) == 6 {
        return Err(engine::Error::UnsupportedStage { entry: 0x154350 });
    }
    m.put_word(
        p.wrapping_add(0xd4),
        m.word(p.wrapping_add(0xd4)).wrapping_add(1),
    );
    let limit = m.word(p.wrapping_add(0x38));
    let pending = m.word(p.wrapping_add(0xd4));
    if limit != 0 && limit < pending {
        return Err(engine::Error::UnsupportedStage { entry: 0x15431c });
    }
    // 0x15437c: the flush==1 saved comparison is not taken.
    if m.word(sched_sp.wrapping_add(0x444)) == 1 {
        return Err(engine::Error::UnsupportedStage { entry: 0x154398 });
    }
    let cs = m.word(p.wrapping_add(0x7c));
    let comps = m.word(cs.wrapping_add(8));
    let comp = m.word(comps.wrapping_add(an.wrapping_mul(4)));
    // Clear extra[anStage], stash the name at [sp,#4], then the
    // "Calling Process for %s" log (the stash is the log's stack arg;
    // the word rides the excluded scheduler-stack page).
    m.put_byte(extra_base.wrapping_add(an), 0);
    let name = m.word(comp.wrapping_add(4));
    m.put_word(sched_sp.wrapping_add(4), name);
    let logger = m.word(m.word(o).wrapping_add(16));
    m.put_word(SEG7_STEP_VARARGS, name);
    let live = |m: &mut Memory<'_>,
                platform: &mut dyn engine::HeapPlatform,
                runtime: &mut dyn crate::engine_live::RuntimeServicesM,
                entry: u32,
                args: &[u32]|
     -> Result<u32, engine::Error> {
        crate::engine_live::call_live_with_runtime_m(m, entry, args, platform, runtime, snap)
    };
    let lrc = live(
        m,
        &mut *platform,
        &mut *runtime,
        0x1738e8,
        &[logger, SEG7_LOG_MODULE, 4, 0, SEG7_FMT_CALLING, SEG7_STEP_VARARGS],
    )?;
    if lrc >= 0x80000000 {
        return Err(engine::Error::EngineStatus(lrc));
    }
    if m.word(cs.wrapping_add(0x10)) != 0 {
        return Err(engine::Error::UnsupportedStage { entry: 0x1543e0 });
    }
    if m.byte(p.wrapping_add(0x80)) == 2 {
        return Err(engine::Error::UnsupportedStage { entry: 0x154400 });
    }
    let h0 = m.word(comp.wrapping_add(0xc));
    let h1 = m.word(comp.wrapping_add(0x10));
    let aux0 = m.word(p.wrapping_add(0xa0));
    let aux1 = m.word(p.wrapping_add(0xa4));
    let active = extra_base.wrapping_add(an);
    // The call site (0x154ef4) writes only stack0; stack1/2 ride
    // whatever the scheduler frame holds. Copy them live.
    let st1 = m.word(sched_sp.wrapping_add(4));
    let st2 = m.word(sched_sp.wrapping_add(8));
    Ok(Seg7Next {
        entry: m.word(m.word(comp.wrapping_add(8)).wrapping_add(0x34)),
        args: [h0, h1, aux0, aux1, active, st1, st2],
    })
}
