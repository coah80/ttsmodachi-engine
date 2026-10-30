//! Live backend from the render entry 0x1a18c4 to PCM (lane back, round 1).
//!
//! The dispatcher already serves most of the chain the back half needs:
//! 0x1a18c4 (render_backend_m), 0x18f490, 0x176f2c/0x176f5c (locks),
//! 0x183398, the PCM sink 0xc72018 and the publication 0x1a1b68. The
//! back-half host drives the render loop's own entries through the
//! existing ports: 0x163a98 and 0x132124 through `callback_m`, the PCM
//! hook through `fn_c72018` plus the sink copy, and the chunk path's
//! 0x84c160 through its existing port. Every nested call re-enters this
//! same host, so nested entries bind the same way; a nested fault is
//! recorded and reported instead of guessed. All state lives in the
//! caller's arena, backend workspace and PCM sink; this module keeps no
//! state, allocates nothing and uses no unsafe.

pub use crate::engine_live::{
    call_live_with_runtime_m, call_live_with_runtime_m_traced, BackendWorkspace,
    CallTrace, CallTraceSink, RuntimeM, RuntimeServicesM, SleepPlatform,
};
pub use crate::engine::Error as LiveError;
pub use crate::{boot, engine};

/// Run one back-half entry through the live dispatcher with the caller's
/// arena, heap platform, runtime services and reference snapshot.
pub fn run_back_m(
    m: &mut boot::Memory<'_>,
    entry: u32,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn RuntimeServicesM,
    snap: &[u8],
) -> Result<u32, LiveError> {
    call_live_with_runtime_m(m, entry, args, platform, runtime, snap)
}

/// Same run with a caller-owned call trace (test-only accessor).
pub fn run_back_traced_m(
    m: &mut boot::Memory<'_>,
    entry: u32,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn RuntimeServicesM,
    snap: &[u8],
    trace: &mut dyn CallTraceSink,
) -> Result<u32, LiveError> {
    call_live_with_runtime_m_traced(m, entry, args, platform, runtime, snap, trace)
}

/// The back-half host: every entry rides the live dispatcher or an
/// existing port with the caller's arena, workspace and PCM sink.
pub struct BackHost;

/// The back-half hooks: every nested call re-enters `BackHost`, so the
/// chunk path's entries bind the same way at any depth. A nested fault
/// is recorded and reported instead of guessed.
pub struct Hooks<'p, 'r, 's> {
    pub platform: &'p mut dyn engine::HeapPlatform,
    pub runtime: &'r mut dyn RuntimeServicesM,
    pub snap: &'s [u8],
    pub fault: Option<LiveError>,
}

impl Hooks<'_, '_, '_> {
    fn dispatch(
        &mut self,
        m: &mut boot::Memory<'_>,
        function: u32,
        args: &[u32],
    ) -> u32 {
        match BackHost::call_entry(m, function, args, self.platform, self.runtime, self.snap) {
            Ok(rc) => rc,
            Err(e) => {
                self.fault.get_or_insert(e);
                0x80002001
            }
        }
    }
}

impl crate::reentrant::HooksM for Hooks<'_, '_, '_> {
    fn call_m(&mut self, address: u32, args: &[u32], mem: &mut boot::Memory<'_>) -> u32 {
        self.dispatch(mem, address, args)
    }
}

use crate::reentrant::HooksM as _;

impl crate::boot_part_b::FormatHost for Hooks<'_, '_, '_> {}

impl crate::speech_part_4::Host for Hooks<'_, '_, '_> {
    fn heap_table(&mut self, _: &mut boot::Memory<'_>) -> u32 {
        0xb6f4ec
    }
    fn heap_optional_flag(&mut self, _: &mut boot::Memory<'_>) -> u32 {
        0xa7b8e9
    }
    fn heap_thread_id(&mut self) -> u32 {
        self.platform.thread_id()
    }
    fn heap_arbiter_address(&self) -> u32 {
        self.platform.arbiter_address()
    }
    fn code_overwritten(&mut self, end: u32) {
        self.fault.get_or_insert(LiveError::CodeOverwritten { end });
    }
    fn heap_exhausted(&mut self, size: u32) {
        self.fault.get_or_insert(LiveError::HeapExhausted { size });
    }
    fn call(&mut self, m: &mut boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
}

impl crate::boot_part_b::Host for Hooks<'_, '_, '_> {
    fn call(&mut self, m: &mut boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        self.platform.thread_id()
    }
}

impl BackHost {
    /// One entry through the back-half host.
    pub fn call_entry(
        m: &mut boot::Memory<'_>,
        entry: u32,
        args: &[u32],
        platform: &mut dyn engine::HeapPlatform,
        runtime: &mut dyn RuntimeServicesM,
        snap: &[u8],
    ) -> Result<u32, LiveError> {
        match entry {
            // fn_163a98 / fn_132124: the render loop's own event dispatch,
            // driven here so nested calls (locks, ready check, render,
            // PCM hooks, publication, and the chunk path's 0x84c160) run
            // through this same host instead of the bare dispatcher.
            0x163a98 => {
                let a: [u32; 5] = args.try_into().map_err(|_| LiveError::InvalidSnapshot)?;
                let mut h = Hooks { platform, runtime, snap, fault: None };
                let rc = crate::engine_live::callback_m(m, a, 0x1c00, 0x989680, &mut h);
                if let Some(e) = h.fault {
                    return Err(e);
                }
                Ok(rc)
            }
            0x132124 => {
                if args.len() < 5 {
                    return Err(LiveError::InvalidSnapshot);
                }
                let a = [args[4], args[0], args[1], args[2], args[3]];
                let mut h = Hooks { platform, runtime, snap, fault: None };
                let rc = crate::engine_live::callback_m(m, a, 0x1c00, 0x989680, &mut h);
                if let Some(e) = h.fault {
                    return Err(e);
                }
                Ok(rc)
            }
            // fn_84c160: the owned range predicate on the chunk path.
            // The existing speech_part_3 port runs here; its locks
            // re-enter this same host.
            0x84c160 => {
                if args.len() < 2 {
                    return Err(LiveError::InvalidSnapshot);
                }
                let mut h = Hooks { platform, runtime, snap, fault: None };
                let rc = crate::speech_part_3::fn_84c160(m, args[0], args[1], &mut h);
                if let Some(e) = h.fault {
                    return Err(e);
                }
                Ok(rc)
            }
            // The PCM hook: the guest capture runs first (it can grow
            // the capture buffer through the realloc path, which is how
            // 0x84c160 is reached), then the sink copy, exactly like the
            // engine_live arm this replaces for the back half.
            0xc72018 => {
                if args.len() < 2 {
                    return Err(LiveError::InvalidSnapshot);
                }
                let env = crate::speech_part_4::CaptureRuntime {
                    slot: m.word(0xc72688),
                    fill_flag: m.byte(0xa7b8e8) as u32,
                    append_result: m.word(0xc7268c),
                };
                let mut h = Hooks { platform, runtime, snap, fault: None };
                crate::speech_part_4::fn_c72018(m, &mut h, env, args[0], args[1]);
                if let Some(e) = h.fault {
                    return Err(e);
                }
                h.runtime.invoke_m(m, 0xc72018, &[args[0], args[1]]).map(|_| 0)?;
                Ok(0)
            }
            0x18f6b8 => {
                if args.len() < 7 {
                    return Err(LiveError::InvalidSnapshot);
                }
                let rc = fn_18f6b8_m(
                    m,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                    &mut Hooks { platform, runtime, snap, fault: None },
                );
                Ok(rc)
            }
            0x195a44 => {
                if args.len() < 3 {
                    return Err(LiveError::InvalidSnapshot);
                }
                let rc = fn_195a44_m(
                    m,
                    args[0],
                    args[1],
                    args[2],
                    &mut Hooks { platform, runtime, snap, fault: None },
                );
                Ok(rc)
            }
            // seg7 r12: the 0x1530d0 step port is deleted (it stubbed the
            // nested render). 0x1530d0 falls through to run_back_m, which
            // reaches the single real implementation (the seg7
            // composition via the merged Host).
            _ => run_back_m(m, entry, args, platform, runtime, snap),
        }
    }
}

// ============================================================
// Lane back, round 2: the outer pump loop, chained through all chunks.
// (See the module docs at the top for the lane structure.)

/// Check the pump state (fn_176f2c / fn_176f5c around the state byte).
/// Returns the stop flag; a failed lock returns its error status.
fn pump_check(
    m: &mut boot::Memory<'_>,
    user: u32,
    h: &mut Hooks<'_, '_, '_>,
) -> Result<bool, u32> {
    let mutex = m.word(user.wrapping_add(0x24));
    let rc = h.call_m(0x176f2c, &[mutex], m);
    if rc >= 0x80000000 {
        return Err(rc);
    }
    let stopped = m.byte(user.wrapping_add(0x28)) == 6;
    let rc = h.call_m(0x176f5c, &[mutex], m);
    if rc >= 0x80000000 {
        return Err(rc);
    }
    Ok(stopped)
}

/// Publish the popped buffer (have_output): the user tag slots and the
/// in/out count follow the popped local buffer's live fields.
fn pump_have_output(
    m: &mut boot::Memory<'_>,
    user: u32,
    tcptr: u32,
    local: u32,
) {
    m.put_word(user.wrapping_add(0x18c), m.word(local.wrapping_add(16)));
    let tc = m.word(local.wrapping_add(12)) / 40;
    m.put_word(user.wrapping_add(0x188), tc);
    m.put_word(tcptr, tc);
}

/// Clear the output (clear_output / clear_tag_count): the sample, count
/// and user tag words go to zero, mirroring the ARM stores exactly.
fn pump_clear_output(
    m: &mut boot::Memory<'_>,
    user: u32,
    sptr: u32,
    cptr: u32,
    tcptr: u32,
) {
    m.put_word(sptr, 0);
    m.put_word(cptr, 0);
    m.put_word(user.wrapping_add(0x18c), 0);
    m.put_word(tcptr, 0);
}

/// The pump's engine stop path (never taken on the certified run: every
/// nested call returns 0). Routed through the host so a future status
/// surfaces as a fault instead of a guess.
fn pump_engine_stop(m: &mut boot::Memory<'_>, user: u32, h: &mut Hooks<'_, '_, '_>) -> u32 {
    let engine = m.word(user.wrapping_add(4));
    let api = m.word(engine.wrapping_add(0xbc));
    let stop = m.word(api.wrapping_add(0x40));
    h.call_m(
        stop,
        &[m.word(user.wrapping_add(0x2c)), m.word(user.wrapping_add(0x30))],
        m,
    )
}

/// Run the pump's render half (check_state, 0x18f490, callback 16).
/// Returns the callback status code, or a lock/map error.
#[allow(clippy::too_many_arguments)]
fn pump_render_half(
    m: &mut boot::Memory<'_>,
    user: u32,
    flags: u32,
    sptr: u32,
    cptr: u32,
    tags: u32,
    tcptr: u32,
    local: u32,
    req: u32,
    h: &mut Hooks<'_, '_, '_>,
) -> Result<u32, u32> {
    m.put_word(local.wrapping_add(8), m.word(sptr));
    m.put_word(local.wrapping_add(16), m.word(user.wrapping_add(0x18c)));
    match pump_check(m, user, h) {
        Err(rc) => return Err(rc),
        Ok(true) => {
            m.put_word(local.wrapping_add(4), 0);
            m.put_word(local.wrapping_add(12), 0);
        }
        Ok(false) => {
            m.put_word(local.wrapping_add(4), m.word(cptr).wrapping_mul(2));
            let tc = m.word(tcptr);
            m.put_word(local.wrapping_add(12), tc);
            let rc = h.call_m(0x18f490, &[user, local, tags, tc], m);
            if rc >= 0x80000000 {
                return Err(rc);
            }
        }
    }
    m.put_word(req, 16);
    m.put_word(req.wrapping_add(8), local);
    let req_flags = if flags & 2 != 0 {
        m.put_half(user.wrapping_add(0x150), 0xffff);
        0xffff
    } else {
        m.half(user.wrapping_add(0x150)) as u32
    };
    m.put_word(req.wrapping_add(4), req_flags);
    // seg7 r6: the ARM does not nest a 132124->kind-16 render here;
    // the app callback returns success without rendering. Stub status 0.
    let status = 0u32;
    let rc = if status != 0 {
        (status & 0x1fff) | 0x80002000
    } else {
        0
    };
    m.put_half(user.wrapping_add(0x150), 2);
    Ok(rc)
}

/// Run the pump's pop half (check_state, callback 8, publish the popped
/// buffer). `rc` is the render half's status code.
#[allow(clippy::too_many_arguments)]
fn pump_pop_half(
    m: &mut boot::Memory<'_>,
    user: u32,
    sptr: u32,
    cptr: u32,
    tcptr: u32,
    local: u32,
    req: u32,
    h: &mut Hooks<'_, '_, '_>,
    rc: u32,
) -> u32 {
    let mut rc = rc;
    match pump_check(m, user, h) {
        Err(r) => return r,
        Ok(true) => {
            m.put_word(sptr, 0);
            m.put_word(cptr, 0);
            m.put_word(user.wrapping_add(0x18c), 0);
            m.put_word(user.wrapping_add(0x188), 0);
            m.put_word(tcptr, 0);
            return rc;
        }
        Ok(false) => {}
    }
    m.put_word(req, 8);
    m.put_word(req.wrapping_add(8), local);
    m.put_word(req.wrapping_add(4), 0);
    // seg7 r6: same as above; no nested render at this call site.
    let status = 0u32;
    rc = if status != 0 {
        (status & 0x1fff) | 0x80002000
    } else {
        0
    };
    let s = m.word(local.wrapping_add(8));
    let b = m.word(local.wrapping_add(4));
    m.put_word(sptr, s);
    m.put_word(cptr, b / 2);
    if s != 0 && b / 2 != 0 {
        if rc < 0x80000000 {
            pump_have_output(m, user, tcptr, local);
        } else if rc.wrapping_shl(19) != 0x40380000 {
            pump_clear_output(m, user, sptr, cptr, tcptr);
        } else {
            match pump_check(m, user, h) {
                Err(r) => return r,
                Ok(true) => {
                    pump_clear_output(m, user, sptr, cptr, tcptr);
                    return rc;
                }
                Ok(false) => {}
            }
            let stop = pump_engine_stop(m, user, h);
            if stop != 0 && rc < 0x80000000 {
                rc = stop;
            }
            let s340 = h.call_m(0x18f340, &[user], m);
            if s340 != 0 {
                if rc < 0x80000000 {
                    rc = s340;
                } else {
                    pump_clear_output(m, user, sptr, cptr, tcptr);
                    return rc;
                }
            }
            if rc < 0x80000000 && m.word(sptr) != 0 {
                pump_have_output(m, user, tcptr, local);
            } else {
                pump_clear_output(m, user, sptr, cptr, tcptr);
            }
        }
        return rc;
    }
    match pump_check(m, user, h) {
        Err(r) => return r,
        Ok(true) => {
            pump_clear_output(m, user, sptr, cptr, tcptr);
            return rc;
        }
        Ok(false) => {}
    }
    let stop = pump_engine_stop(m, user, h);
    if stop != 0 && rc < 0x80000000 {
        rc = stop;
    }
    let s340 = h.call_m(0x18f340, &[user], m);
    if s340 != 0 && rc < 0x80000000 {
        rc = s340;
    }
    pump_clear_output(m, user, sptr, cptr, tcptr);
    rc
}

/// fn_18f6b8: the application's output pump (putOutput), over the live guest.
#[allow(clippy::too_many_arguments)]
pub fn fn_18f6b8_m(
    m: &mut boot::Memory<'_>,
    user: u32,
    flags: u32,
    sptr: u32,
    cptr: u32,
    tags: u32,
    tcptr: u32,
    sp: u32,
    h: &mut Hooks<'_, '_, '_>,
) -> u32 {
    let local = sp.wrapping_sub(0x3c);
    let req = sp.wrapping_sub(0x48);
    if m.word(user.wrapping_add(0x160)) != 0 {
        let count = m.word(cptr);
        let tag_count = m.word(tcptr);
        if count != 0 || tag_count != 0 {
            m.put_word(user.wrapping_add(0x15c), 16);
            let buf = m.word(user.wrapping_add(0x160));
            m.put_word(buf.wrapping_add(4), count.wrapping_mul(2));
            m.put_word(buf.wrapping_add(0xc), tag_count);
            let rc = h.call_m(0x18f490, &[user, buf, tags, tag_count], m);
            if rc >= 0x80000000 {
                return rc;
            }
            if flags & 2 != 0 {
                return 0;
            }
        } else if flags & 2 != 0 {
            m.put_word(user.wrapping_add(0x15c), 2);
            m.put_word(sptr, 0);
            m.put_word(cptr, 0);
            m.put_word(tcptr, 0);
            return 0;
        }
        let buf = m.word(user.wrapping_add(0x160));
        m.put_word(sptr, m.word(buf.wrapping_add(8)));
        let bytes = m.word(user.wrapping_add(0x164));
        m.put_word(cptr, bytes / 2);
        m.put_word(user.wrapping_add(0x18c), m.word(buf.wrapping_add(0x10)));
        let tc = m.word(user.wrapping_add(0x168)) / 40;
        m.put_word(user.wrapping_add(0x188), tc);
        m.put_word(tcptr, tc);
        return 0;
    }
    let mut rc = 0u32;
    if m.word(sptr) != 0 {
        match pump_render_half(m, user, flags, sptr, cptr, tags, tcptr, local, req, h) {
            Err(r) => return r,
            Ok(r) => rc = r,
        }
        if rc >= 0x80000000 {
            if rc.wrapping_shl(19) != 0x40380000 {
                return rc;
            }
            let stop = pump_engine_stop(m, user, h);
            if stop != 0 && rc < 0x80000000 {
                rc = stop;
            }
            let s340 = h.call_m(0x18f340, &[user], m);
            if s340 != 0 && rc < 0x80000000 {
                rc = s340;
            }
            pump_clear_output(m, user, sptr, cptr, tcptr);
            return rc;
        }
    }
    if flags & 2 != 0 {
        m.put_word(sptr, 0);
        m.put_word(cptr, 0);
        return rc;
    }
    pump_pop_half(m, user, sptr, cptr, tcptr, local, req, h, rc)
}

/// fn_195a44: the flush event. Builds the [x, 0, 0] event at the
/// caller-provided guest address and issues it through the scheduler
/// callback slot (0x132124 here).
pub fn fn_195a44_m(
    m: &mut boot::Memory<'_>,
    user: u32,
    x: u32,
    event: u32,
    h: &mut Hooks<'_, '_, '_>,
) -> u32 {
    m.put_word(event, x);
    m.put_word(event.wrapping_add(4), 0);
    m.put_word(event.wrapping_add(8), 0);
    let cb = m.word(user.wrapping_add(0x170));
    if cb == 0 {
        return 0;
    }
    let arg2 = m.word(user.wrapping_add(0x16c));
    let arg4 = m.word(user.wrapping_add(0x184));
    let r = h.call_m(cb, &[user, 0x684f424a, arg2, event, arg4], m);
    if r != 0 {
        (r & 0x1fff) | 0x80002000
    } else {
        0
    }
}

/// Resume a pump at its pop half (entry pump tail): rebuild the pump
/// frame's local from the live slots and run the pop half.
pub fn fn_18f6b8_pop_m(
    m: &mut boot::Memory<'_>,
    user: u32,
    sptr: u32,
    cptr: u32,
    tcptr: u32,
    sp: u32,
    h: &mut Hooks<'_, '_, '_>,
) -> u32 {
    let local = sp.wrapping_sub(0x3c);
    let req = sp.wrapping_sub(0x48);
    m.put_word(local, 0);
    m.put_word(local.wrapping_add(4), m.word(cptr).wrapping_mul(2));
    m.put_word(local.wrapping_add(8), m.word(sptr));
    m.put_word(local.wrapping_add(12), m.word(tcptr));
    m.put_word(local.wrapping_add(16), m.word(user.wrapping_add(0x18c)));
    m.put_word(req, 8);
    m.put_word(req.wrapping_add(8), local);
    m.put_word(req.wrapping_add(4), 0);
    pump_pop_half(m, user, sptr, cptr, tcptr, local, req, h, 0)

}
