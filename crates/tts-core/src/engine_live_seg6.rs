

use crate::{boot, engine, engine_live};

pub const ROOT_22A0A8: u32 = 0x22a0a8;

pub const RETURN_R0: u32 = 0;

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
