#![no_std]
#![forbid(unsafe_code)]

pub mod engine;
pub mod bridge;
pub mod engine_components;
pub mod engine_components2;
pub mod engine_backend;
pub mod engine_sing;
pub mod engine_jp;
pub mod render_part_a;
pub mod render_part_a2;
pub mod render_part_a3;
pub mod render_part_b;
pub mod render_part_b2;
pub mod render_part_c;
pub mod render_part_c2;
pub mod render_part_d;
pub mod render_part_d2;
pub mod boot;
pub mod boot_live;
pub mod boot_part_a;
pub mod boot_part_b;
pub mod boot_part_b2;
pub mod boot_part_c;
pub mod boot_part_d;
pub mod boot_part_e;
pub mod boot_part_f;
pub mod speech_part_1;
pub mod speech_part_2;
pub mod speech_part_3;
pub mod speech_part_4;
pub mod speech_part_5;
pub mod speech_part_6;
pub mod speech_part_7;
pub mod log_live;
pub mod db_live;
pub mod reentrant2;
pub mod leftovers_1;
pub mod engine_live;
pub mod engine_live_render;
/// Live backend from the render entry 0x1a18c4 to PCM (lane back, round 1).
pub mod engine_live_back;
/// Live scheduler segment 1 (lane seg1, prompts/rust_seg_r1.md).
pub mod engine_live_seg1;
/// Live scheduler segment 2 (lane seg2, prompts/rust_seg_r1.md).
pub mod engine_live_seg2;
/// Live scheduler segment 3 (lane seg3, prompts/rust_seg_r1.md).
pub mod engine_live_seg3;
/// Live scheduler segment 4 (lane seg4, prompts/rust_seg_r1.md).
pub mod engine_live_seg4;
/// Live scheduler segment 5 (lane seg5, prompts/rust_seg_r1.md).
pub mod engine_live_seg5;
/// Live scheduler segment 6 (lane seg6, prompts/rust_seg_r1.md).
pub mod engine_live_seg6;
/// Live scheduler segment 7 (lane seg7, prompts/rust_seg_r1.md).
pub mod engine_live_seg7;
/// The US engine end to end (lane us, prompts/rust_us_engine_r1.md).
pub mod engine_us;
/// live component roots 1deb04, 1e772c, 1f3dc0, 1f51dc, 210398, 2279b4, 1e3938 and 2319f0 (lane wire-a2)
pub mod engine_live2;
pub mod component_live_c;
/// live component roots 2031b0 and 1d3314 (split from component-live-c)
pub mod component_live_c2;
/// live component root 21a25c (split from component-live-c)
pub mod component_live_c3;
/// live component root 206be4 (split from component-live-c)
pub mod component_live_c4;
pub mod component_live_b;
/// live component roots 1e5bbc and 1e4b7c (split from component-live-b)
pub mod component_live_b2;
/// live ISTOKEN rule subtree reached from the 1e4b7c number/date validation
/// (seg1 r5): 0x2b8504/0x2b7fdc/0x2b7d80/0x2b79b4 plus buffer, canonical-word
/// and pure-leaf ports.
pub mod component_live_2b7d;
pub mod component_g2p;
pub mod component_live_b3;
pub mod component_live_b4;
pub mod component_live_a;
pub mod component_live_a2;
pub mod component_4a76e4;
pub mod boot_port_6;
/// Boot-path cluster for fn_1e0400 (lane 6b). See rust/evidence/boot-port-6-round8.progress.txt.
pub mod boot_port_6b;
pub mod boot_port_3b;
pub mod boot_port_4b;
pub mod boot_port_1b;
pub mod boot_port_1c;
pub mod boot_port_5b;
pub mod boot_port_2b;
pub mod boot_port_2c;
pub mod boot_port_2d;
pub mod boot_port_5c;
pub mod boot_port_5;
pub mod boot_port_4;
pub mod boot_port_3;
pub mod boot_port_2;
pub mod boot_port_1;
pub mod boot_fmt;
pub mod buffers;
pub mod controls;
pub mod math;
pub mod orchestration;
pub mod pitch;
pub mod rom_region;
pub mod runtime;
pub mod setup;
pub mod synthesis;
pub mod tables;
pub mod voice;

pub mod effects;
pub mod frames;

pub mod frames_pcm;
pub mod queue;

pub mod output;
pub mod resources;

pub mod frames_boundary;

pub mod frames_synthesis;

pub mod text;

pub mod frontend_types;

pub mod heap;

pub mod morphology;
pub mod pos;
pub mod pronunciation;

pub mod selection;

pub mod linguistic;

pub mod frontend_resources;

pub mod depes;

pub mod regex;

pub mod normalization;

pub mod text_input;

pub mod frontend;

pub mod frontend_runtime;

pub mod frontend_part_a;
pub mod frontend_part_b;
pub mod frontend_part_c;
pub mod frontend_part_d;

pub mod prosody;
pub mod prosody_rules;

pub mod frontend_output;

pub mod phonemes;

pub mod jp;

pub mod fe_text;

/// Additive live-memory hooks and alias-preserving guest entries.
pub mod reentrant;
