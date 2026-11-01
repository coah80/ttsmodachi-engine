

use crate::jp::Engine;

pub fn mutex_lock(_eng: &Engine, _tag: u32) {}

pub fn mutex_lock_at(_mem: &[u8], _off: usize, _tag: u32) {}

pub fn mutex_unlock(_eng: &Engine) {}

pub fn mutex_unlock_at(_mem: &[u8], _off: usize) {}

pub fn current_tid(tid: u32) -> u32 {
    tid
}

pub fn owner_query(mem: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([mem[off], mem[off + 1], mem[off + 2], mem[off + 3]])
}

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

pub fn event_wait(mem: &mut [u8], off: usize, _timeout: i32) -> i32 {
    mem[off + 4] = 1;
    1
}

pub fn event_wait_signal(eng: &mut Engine, off: usize, _tag: u32) {
    if eng.mem[off + 4] == 0 {
        eng.mem[off + 4] = 1;
    }
    if eng.rds32(off) == 0 {
        return;
    }
    eng.wr32(off, 0);
}

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

#[derive(Clone, Copy, Debug)]
pub struct GuestSync {

    pub engine_base: u32,

    pub mutex_a: u32,
    pub mutex_b: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct JobSubmit {

    pub cb: u32,

    pub arg: u32,

    pub out_off: usize,
}

pub fn job_submit(mem: &mut [u8], obj: usize, sync: GuestSync, job: JobSubmit) -> u32 {
    let wr = |mem: &mut [u8], o: usize, v: u32| {
        mem[o..o + 4].copy_from_slice(&v.to_le_bytes());
    };

    let out_g = sync.engine_base.wrapping_add(job.out_off as u32);
    let r7 = out_g.wrapping_sub(4) & !7;
    let r = r7.wrapping_sub(20) & !7;
    let ro = r.wrapping_sub(sync.engine_base) as usize;
    wr(mem, ro, 0x169d00);
    wr(mem, ro + 4, 0x169cf4);
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
    wr(mem, obj, 0x14C548);
    mem[obj + 4] = 0;
    mem[obj + 5] = 0;
    0
}
