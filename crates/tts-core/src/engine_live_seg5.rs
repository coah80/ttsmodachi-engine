//! Live segment 5 of the US speech_1 scheduler run (lane seg5).
//!
//! Roots: the 0x1cca64 scheduler step just before 0x2104b0 (tick 6255063),
//! 0x2104b0 (tick 6255651), and 0x227d64 three times (ticks 6296175,
//! 6449623, 10638928). Captures: scratch/engine-stages/seg5-*.wire /
//! *.expected plus seg5-bounds.json (see scratch/seg5/cap_seg5.py).
//!
//! The host below wraps the live engine dispatcher
//! ([`crate::engine_live`] public items): 0x2104b0 and 0x227d64 ride
//! their certified ports, 0x1cca64 and every nested call ride
//! `engine_live::call_live_m`. The first entry the live engine cannot
//! serve is recorded in [`Seg5Host::first_unbound`] instead of being
//! silently replayed.

use crate::{
    boot::Memory, component_live_a, component_live_c, engine, engine_live, frontend,
};

/// Live host for the seg5 stretch. `platform` is the caller's heap
/// platform (thread id / arbiter); `first_unbound` is the first entry
/// the live engine refused ([`engine::Error::UnsupportedStage`]).
pub struct Seg5Host<'a> {
    platform: &'a mut dyn engine::HeapPlatform,
    pub first_unbound: Option<u32>,
    /// First nested live-dispatcher failure: (entry, rc). Reported to
    /// the test; the lib itself stays silent (no_std, no printing).
    pub first_nested_error: Option<(u32, u32)>,
    /// First entry the live dispatcher refused but a certified lane-c
    /// core port served (gap fill). Reported to the test.
    pub gap_filled: Option<u32>,
}

impl<'a> Seg5Host<'a> {
    pub fn new(platform: &'a mut dyn engine::HeapPlatform) -> Self {
        Seg5Host {
            platform,
            first_unbound: None,
            first_nested_error: None,
            gap_filled: None,
        }
    }

    /// Run one seg5 root (`0x1cca64`, `0x2104b0` or `0x227d64`) on the
    /// caller-owned arena. `args` is the wire arg vector (registers
    /// first, then guest stack words): 0x2104b0 takes
    /// `[p, tag, in0, in1, active]`, 0x227d64 takes all seven,
    /// 0x1cca64 takes the first four.
    pub fn call_root(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        match entry {
            0x1cca64 => {
                let a = [args[0], args[1], args[2], args[3]];
                match engine_live::call_live_m(m, entry, &a, self.platform) {
                    Ok(rc) => rc,
                    Err(engine::Error::UnsupportedStage { entry: f }) => {
                        if self.first_unbound.is_none() {
                            self.first_unbound = Some(f);
                        }
                        0x80002001
                    }
                    Err(_) => 0x80002001,
                }
            }
            0x2104b0 => {
                // Certified lane-c port, same calling convention as
                // tests/component_live_c.rs: pending/active is the fifth
                // (stack) arg, scratch is the disjoint guest window
                // 0xe000000, all other workspace is Rust locals.
                // c3 r29: the saved frame sizes the RewriterLive staging
                // window (capacity = len-16). ARM passes heap pointers
                // with no staging cap; 4096 was just enough for speech_1
                // but us-corpus:182 stages 4152 event bytes in one
                // append. Size to the largest generatable datum (the
                // 64 KiB text buffer) so the cap never binds.
                let mut saved = [0u8; 65536];
                let mut events = [frontend::TextEvent::default(); 512];
                let mut event_bytes = [0u8; 512 * 24];
                let mut text = [0u8; 65536];
                match component_live_c::fn_2104b0_m(
                    m,
                    args[0],
                    args[1],
                    [args[2], args[3]],
                    args[4],
                    0xe000000,
                    &mut saved,
                    &mut events,
                    &mut event_bytes,
                    &mut text,
                    self,
                ) {
                    Ok(rc) => rc,
                    Err(_) => 0x80002001,
                }
            }
            0x227d64 => {
                // Certified lane-a port, same convention as
                // tests/component_live_a.rs and the live dispatcher's
                // selection_root_m: SP-relative stack scratch
                // (SCRATCH_227D64), caller-owned nodes
                // and feature workspace, live heap platform.
                let mut a = [0u32; 7];
                a[..args.len().min(7)].copy_from_slice(&args[..args.len().min(7)]);
                let mut nodes =
                    [crate::selection::DecisionNode { question: 0, miss: 0, hit: 0 }; 65536];
                let mut features = component_live_a::FeatureWorkspace {
                    blob: [0; 131072],
                    bytes: [[0; 2053]; 60],
                    words: [[0; 1025]; 60],
                    ints: [[0; 16384]; 6],
                    pitch: [0; 4096],
                    contexts: [0; 9216],
                    modes: [0; 16384],
                    markers: [crate::prosody::ProsodyMarker {
                        kind: 0,
                        active: 0,
                        position: 0,
                    }; 1024],
                    carrier: [0; 4096],
                };
                match component_live_a::fn_227d64_m(
                    m,
                    a,
                    crate::component_live_a::SCRATCH_227D64,
                    &mut nodes,
                    self.platform,
                    &mut features,
                ) {
                    Ok(rc) => rc,
                    Err(engine::Error::UnsupportedStage { entry: f }) => {
                        if self.first_unbound.is_none() {
                            self.first_unbound = Some(f);
                        }
                        0x80002001
                    }
                    Err(_) => 0x80002001,
                }
            }
            _ => panic!("seg5 host does not own entry {entry:#x}"),
        }
    }
}

impl component_live_c::Host for Seg5Host<'_> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        // Gap fill (round 1, seg5): the lane-c clear port the live
        // dispatcher does not serve. Scratch 0xe0fed00 matches the
        // certified component_live_c test convention.
        if entry == 0x15b518 {
            let ab: [u32; 2] = match args.try_into() {
                Ok(ab) => ab,
                Err(_) => return 0x80002001,
            };
            return component_live_c::clear_db_live_m(m, ab, 0xe0fed00, self);
        }
        // Every other nested call rides the live engine dispatcher first.
        // Gap fill (round 1, seg5): entries the live dispatcher refuses
        // but the certified lane-c core ports serve (e.g. 0x15d594,
        // 0x15d6c4) fall back to core_call_m. Boundary roots nested
        // inside are a real stop and never fall back.
        match engine_live::call_live_m(m, entry, args, self.platform) {
            Ok(rc) => rc,
            Err(engine::Error::UnsupportedStage { entry: f }) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(f);
                }
                if self.first_nested_error.is_none() {
                    self.first_nested_error = Some((entry, 0x80002001));
                }
                0x80002001
            }
            Err(_) => match component_live_c::core_call_m(m, entry, args) {
                Ok(rc) => {
                    if self.gap_filled.is_none() {
                        self.gap_filled = Some(entry);
                    }
                    rc
                }
                Err(_) => {
                    if self.first_nested_error.is_none() {
                        self.first_nested_error = Some((entry, 0x80002001));
                    }
                    0x80002001
                }
            },
        }
    }

    fn thread_id_m(&mut self) -> u32 {
        self.platform.thread_id()
    }
}
