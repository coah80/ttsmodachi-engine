//! JP sync/OS adaptation layer (single-threaded, stubbed SVC).
//!
//! Every SVC answer is 0 with all other registers preserved (the harness
//! `UC_HOOK_INTR` handler only writes R0), so wrappers that store an SVC
//! out-word actually echo their entry R1 (`runtime.c`: `0x11af50`,
//! `0x1934b8` cases). Callers pass those words explicitly via [`GuestSync`]
//! (same pattern as the US `NotificationRegisters`): the engine never
//! invents guest addresses or register residue. Mutex lock/unlock and event
//! wait/signal always succeed; intermediate zero stores are kept so the
//! store trace matches the ROM exactly.
//!
//! The deferred-job submit (`job_submit`, fn_6b18b0 + the engine-visible
//! half of fn_1218bc) is a real caller-owned state machine: given the
//! submit arguments the call site already holds, it reproduces the SyncObj
//! words and the scheduler request struct bit-exactly, so no state word is
//! excluded from comparison.

use crate::jp::Engine;

/// fn_6b17b8: lock engine mutex (no state change on success).
pub fn mutex_lock(_eng: &Engine, _tag: u32) {}

/// fn_6b17c4: lock mutex object (no state change on success).
pub fn mutex_lock_at(_mem: &[u8], _off: usize, _tag: u32) {}

/// fn_6b17ec: unlock engine mutex (no state change on success).
pub fn mutex_unlock(_eng: &Engine) {}

/// fn_6b17f8: unlock mutex object (no state change on success).
pub fn mutex_unlock_at(_mem: &[u8], _off: usize) {}

/// fn_6b1868: current thread id. The ROM reads the `D_70f77c` word into R1
/// and `fn_11af50` echoes R1 back, so the result is the word the caller
/// lends (live guest word under the harness: `0xFFFF8000`).
pub fn current_tid(tid: u32) -> u32 {
    tid
}

/// fn_6b1890 (unreached): owner query; `fn_11af50` echoes entry R1, which
/// the ROM loads from `*p`, so this returns the stored word bit-exactly.
pub fn owner_query(mem: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([mem[off], mem[off + 1], mem[off + 2], mem[off + 3]])
}

/// fn_6b1968 (unreached): tick sleep. Ticks `>= 0x3477` take the `svc 0x0a`
/// path (stubbed 0). Smaller ticks run the `fn_11aeec` spin
/// (`subs r0,r0,#2; bgt`), whose residue has a closed form: 0 maps to -2,
/// other even mids to 0, odd mids below `0x80000000` to -1, `0x80000000`
/// to 0, `0x80000001` to -1, and larger mids to `mid - 2` (one trip).
pub fn sleep_ticks(lo: u32, hi: u32) -> i32 {
    let x = ((hi as u64) << 32) | lo as u64;
    if (x as i64) >= 0x3477 {
        return 0;
    }
    let a = lo as u64 * 0x44A2_FA85;
    let b = hi as u64 * 0x44A2_FA85;
    let mid = ((a >> 32) as u32).wrapping_add(b as u32);
    if mid <= 0x7FFF_FFFF {
        if mid == 0 {
            -2
        } else if mid & 1 == 0 {
            0
        } else {
            -1
        }
    } else if mid == 0x8000_0000 {
        0
    } else if mid == 0x8000_0001 {
        -1
    } else {
        mid.wrapping_sub(2) as i32
    }
}

/// fn_6b193c: event wait; sets the ready byte and returns 1.
pub fn event_wait(mem: &mut [u8], off: usize, _timeout: i32) -> i32 {
    mem[off + 4] = 1;
    1
}

/// fn_6b1818: event wait-then-signal; clears a nonzero handle word.
pub fn event_wait_signal(eng: &mut Engine, off: usize, _tag: u32) {
    if eng.mem[off + 4] == 0 {
        eng.mem[off + 4] = 1;
    }
    if eng.rds32(off) == 0 {
        return;
    }
    eng.wr32(off, 0);
}

/// fn_6b1760: mutex create. `None` models the null-pointer early return
/// (`bxeq lr` with R0 already 0). Otherwise the ROM stores 0, then the
/// entry R1 echoed by `fn_1934b8` (`str r1,[r2]` after the stubbed SVC),
/// and returns 0.
pub fn mutex_create(mem: &mut [u8], off: Option<usize>, entry_r1: u32) -> i32 {
    match off {
        None => 0,
        Some(o) => {
            mem[o..o + 4].copy_from_slice(&0u32.to_le_bytes());
            mem[o..o + 4].copy_from_slice(&entry_r1.to_le_bytes());
            0
        }
    }
}

/// Guest placement + stubbed-SVC echoes the engine cannot derive.
///
/// The engine stores guest pointers as engine-relative offsets and never
/// invents absolute addresses: the caller lends the guest mapping and the
/// register residue, exactly like the US `NotificationRegisters`.
#[derive(Clone, Copy, Debug)]
pub struct GuestSync {
    /// Guest address of the engine block. Places the fn_1218bc request
    /// struct (`R = ((out - kind) & !7 - 20) & !7` in guest coordinates).
    pub engine_base: u32,
    /// fn_6b1760 entry-R1 echoes stored as the mutex handles at 0xE6F8
    /// (`mutex_a`) and 0xE6FC (`mutex_b`). Both call sites pass flags 0,
    /// so both words are 0 under the harness.
    pub mutex_a: u32,
    pub mutex_b: u32,
}

/// fn_6b18b0 submit arguments that vary per call site. The rest of the
/// `JobReq` (kind 4, zero 0, a/b/c) is fixed by fn_6b18b0 itself; the
/// timeout pair lands on dead stack consumed only by the stubbed wait and
/// leaves no engine state.
#[derive(Clone, Copy, Debug)]
pub struct JobSubmit {
    /// Guest callback address stored at R+8 (`&D_6aebe8` from construct,
    /// fn_1a1d80 from dispatch). A ROM address, stored verbatim.
    pub cb: u32,
    /// Guest value copied to `*r7` (engine base from dispatch, 0 = null
    /// from construct). Null stays null; other values are stored as
    /// engine-relative offsets.
    pub arg: u32,
    /// `out` pointer as an engine offset (0x13584 at both sites).
    pub out_off: usize,
}

/// fn_6b18b0: deferred-job submit with the engine-visible half of
/// fn_1218bc (see `jp/native/runtime.c` `0x1218bc` case: `r7 =
/// (out - kind) & ~7` with kind 4, `R = (r7 - 20) & ~7`, the 5-word
/// request at R, `*r7 = arg`, `*obj` = stubbed-wait out-slot).
///
/// A null object faults on ARM (`1218bc` dereferences it), so the caller
/// must pass a real object. Returns 0.
pub fn job_submit(mem: &mut [u8], obj: usize, sync: GuestSync, job: JobSubmit) -> u32 {
    let wr = |mem: &mut [u8], o: usize, v: u32| {
        mem[o..o + 4].copy_from_slice(&v.to_le_bytes());
    };
    // Guest-coordinate placement, then rebased to engine offsets for
    // pointer-valued stores (constants compare exactly).
    let out_g = sync.engine_base.wrapping_add(job.out_off as u32);
    let r7 = out_g.wrapping_sub(4) & !7;
    let r = r7.wrapping_sub(20) & !7;
    let ro = r.wrapping_sub(sync.engine_base) as usize;
    wr(mem, ro, 0x169d00); // job.b = &D_169d00
    wr(mem, ro + 4, 0x169cf4); // job.c = &D_169cf4
    wr(mem, ro + 8, job.cb);
    wr(mem, ro + 12, r7);
    wr(mem, ro + 16, 0);
    let r7o = r7.wrapping_sub(sync.engine_base) as usize;
    wr(
        mem,
        r7o,
        if job.arg == 0 {
            0
        } else {
            job.arg.wrapping_sub(sync.engine_base)
        },
    );
    wr(mem, obj, 0x14C548); // stubbed-wait out-slot
    mem[obj + 4] = 0;
    mem[obj + 5] = 0;
    0
}
