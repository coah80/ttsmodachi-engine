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

pub mod cold_boot;
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

pub mod engine_live_back;

pub mod engine_live_seg1;

pub mod engine_live_seg2;

pub mod engine_live_seg3;

pub mod engine_live_seg4;

pub mod engine_live_seg5;

pub mod engine_live_seg6;

pub mod engine_live_seg7;

pub mod engine_us;

pub mod engine_live2;
pub mod component_live_c;

pub mod component_live_c2;

pub mod component_live_c3;

pub mod component_live_c4;
pub mod component_live_b;

pub mod component_live_b2;

pub mod component_live_2b7d;
pub mod component_g2p;
pub mod component_live_b3;
pub mod component_live_b4;
pub mod component_live_a;
pub mod component_live_a2;
pub mod component_4a76e4;
pub mod boot_port_6;

pub mod boot_port_6b;
pub mod boot_port_3b;
pub mod boot_port_4b;
pub mod boot_port_1b;
pub mod boot_port_1c;
pub mod boot_port_5b;
pub mod boot_port_2b;
pub mod boot_port_2c;
pub mod boot_port_2d;

pub mod boot_port_r2;
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

pub mod reentrant;
