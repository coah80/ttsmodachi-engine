

use crate::{
    boot::Memory, component_live_a, component_live_c, engine, engine_live, frontend,
};

pub struct Seg5Host<'a> {
    platform: &'a mut dyn engine::HeapPlatform,
    pub first_unbound: Option<u32>,

    pub first_nested_error: Option<(u32, u32)>,

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
                    crate::rom_region::Rom::Us,
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

        if entry == 0x15b518 {
            let ab: [u32; 2] = match args.try_into() {
                Ok(ab) => ab,
                Err(_) => return 0x80002001,
            };
            return component_live_c::clear_db_live_m(m, ab, 0xe0fed00, self);
        }

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
