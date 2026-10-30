//! Live segment 6 of the US speech_1 scheduler run (lane seg6).
//!
//! See prompts/rust_seg_r1.md. This lane owns a single root:
//!
//! | tick | root | lr | sp | r0 | r1 | r2 | r3 | stack[0] |
//! |---|---|---|---|---|---|---|---|---|
//! | 13229494 | 0x22a0a8 | 0x154ef8 | 0xe0ff938 | 0x14d4c20c | 0x4e7 | 0x14c42964 | 0xb2c2 | 0x14c428ba |
//!
//! No 0x1cca64 scheduler step precedes this root in
//! wire-a-r21c-entries.json (the last logged step is at tick 6255063);
//! the scheduler calls 0x22a0a8 directly after 0x15daa8 plus log writes,
//! and it returns r0 0 at tick 19461965, just before 0x22d6dc.
//!
//! The root is the live synthesis pump
//! ([`crate::component_live_a2::fn_22a0a8_m`]), already bound in the live
//! engine dispatcher ([`crate::engine_live`] routes 0x22a0a8 through
//! `component_a2_m` with scratch root 0x4000). This host therefore forwards
//! every entry, including the seg6 root, to the live engine unchanged: no
//! entry is shadowed, no state is kept here, and region is rom_region only
//! (Rom::Us inside the live dispatcher).

use crate::{boot, engine, engine_live};

/// This lane's root entry.
pub const ROOT_22A0A8: u32 = 0x22a0a8;
/// ARM return value at the captured return boundary.
pub const RETURN_R0: u32 = 0;

/// Run any entry through the live engine (all entries are forwarded;
/// the seg6 root rides the certified port bound there).
pub fn call_m(
    m: &mut boot::Memory<'_>,
    entry: u32,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn engine_live::RuntimeServicesM,
    snap: &[u8],
) -> Result<u32, engine::Error> {
    engine_live::call_live_with_runtime_m(m, entry, args, platform, runtime, snap)
}

/// Run the seg6 root (0x22a0a8) through the live engine.
///
/// `args` is the 7-word captured call vector `[r0, r1, r2, r3, s0, s1, s2]`
/// (the fifth ARM argument, `state`, arrives on the guest stack at s0).
/// The capture guarantees `args[0] == 0x14d4c20c`, `args[1] == 0x4e7`,
/// `args[4] == 0x14c428ba`.
pub fn call_root_m(
    m: &mut boot::Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn engine_live::RuntimeServicesM,
    snap: &[u8],
) -> Result<u32, engine::Error> {
    debug_assert_eq!(args.len(), 7);
    call_m(m, ROOT_22A0A8, args, platform, runtime, snap)
}
