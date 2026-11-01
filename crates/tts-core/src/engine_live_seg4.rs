

use crate::{boot::Memory, engine, engine_live};

pub fn run_step(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn engine_live::RuntimeServicesM,
    snap: &[u8],
) -> Result<u32, engine::Error> {
    engine_live::call_live_with_runtime_m(m, 0x1cca64, args, platform, runtime, snap)
}

fn fault_entry(e: &engine::Error) -> u32 {
    match *e {
        engine::Error::UnsupportedStage { entry } => entry,
        engine::Error::MissingStreamAdapter { entry } => entry,
        _ => 0,
    }
}

fn fault_report(fault: &Option<engine::Error>, rc: u32, root: u32) -> Option<u32> {
    match fault {
        None => {
            if rc >= 0x80000000 {
                Some(root)
            } else {
                None
            }
        }
        Some(e) => {
            let en = fault_entry(e);
            Some(if en == 0 { root } else { en })
        }
    }
}

pub fn run_1e7908(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
) -> (u32, Option<u32>) {
    run_1e7908_rom(m, args, platform, crate::rom_region::Rom::Us)
}

pub fn run_1e7908_rom(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    rom: crate::rom_region::Rom,
) -> (u32, Option<u32>) {
    let mut host = crate::component_live_b::LiveHost {
        platform,
        fault: None,
        scratch: 0x600,
            rom,
    };
    let rc = crate::component_live_b::fn_1e7908_m(
        m, args[0], args[1], args[2], args[3], args[4], 0x600, &mut host,
    );
    (rc, fault_report(&host.fault, rc, 0x1e7908))
}

pub fn run_1f3ea0(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
) -> (u32, Option<u32>) {
    let mut host = crate::component_live_b::LiveHost {
        platform,
        fault: None,
        scratch: 0x500,
            rom: crate::rom_region::Rom::Us,
    };
    let a = [args[0], args[1], args[2], args[3], args[4]];
    let rc = crate::component_live_b::fn_1f3ea0_m(m, a, 0xd00, &mut host);
    (rc, fault_report(&host.fault, rc, 0x1f3ea0))
}

const G_SCRATCH: u32 = 0x2000;
const G_SCRATCH_LEN: usize = 0x2000;

const G_TEXT: u32 = 0x2400;
const G_TEXT_LEN: usize = 0x1c00;

struct DepesFwd<'a> {
    platform: &'a mut dyn engine::HeapPlatform,
    rom: crate::rom_region::Rom,
    first_unbound: Option<u32>,
}

impl crate::component_live_c::Host for DepesFwd<'_> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        let entry = crate::rom_region::to_us_guest(self.rom, entry);

        if entry == 0x1769d0 && args.len() >= 2 {
            return crate::fe_text::strcmp_ci_1769d0(m.bytes, args[0], args[1]) as u32;
        }
        match engine_live::call_live_m_rom(m, entry, args, &mut *self.platform, self.rom) {
            Ok(rc) => rc,
            Err(engine::Error::UnsupportedStage { entry: e })
            | Err(engine::Error::MissingStreamAdapter { entry: e }) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(e);
                }
                0x80002001
            }
            Err(_) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(entry);
                }
                0x80002001
            }
        }
    }
}

pub struct LiveGlobalHost<'a, 'm> {
    arena: &'m mut [u8],
    platform: &'a mut dyn engine::HeapPlatform,
    m34: u32,
    m3c: u32,
    m50: u32,
    m54: u32,
    m58: u32,
    m5c: u32,
    d24: u32,
    d28: u32,
    d2c: u32,
    d40: u32,
    t20: u32,
    heap: u32,
    log: u32,
    params: u32,
    rom: crate::rom_region::Rom,
    first_unbound: Option<u32>,
    mode_buf: [u8; 4096],
    mode_len: usize,
    foreign_buf: [u8; 256],
    foreign_len: Option<usize>,

    ostr_cur: u32,

    layer_bufs: [u32; 7],
    nlayer_bufs: usize,
}

fn arena_word(arena: &[u8], addr: u32) -> u32 {
    let at = addr as usize;
    u32::from_le_bytes(arena[at..at + 4].try_into().unwrap())
}

impl<'a, 'm> LiveGlobalHost<'a, 'm> {

    pub fn new(
        arena: &'m mut [u8],
        platform: &'a mut dyn engine::HeapPlatform,
        p: u32,
        rom: crate::rom_region::Rom,
    ) -> Self {
        let env = arena_word(arena, p);
        let ling_vt = arena_word(arena, p + 4);
        let depes_vt = arena_word(arena, p + 0x10);
        let dct_vt = arena_word(arena, p + 0x1c);
        let heap = arena_word(arena, env + 4);
        let log = arena_word(arena, env + 0x10);
        let params = arena_word(arena, env + 0x14);
        let m34 = arena_word(arena, ling_vt + 0x34);
        let m3c = arena_word(arena, ling_vt + 0x3c);
        let m50 = arena_word(arena, ling_vt + 0x50);
        let m54 = arena_word(arena, ling_vt + 0x54);
        let m58 = arena_word(arena, ling_vt + 0x58);
        let m5c = arena_word(arena, ling_vt + 0x5c);
        let d24 = arena_word(arena, depes_vt + 0x24);
        let d28 = arena_word(arena, depes_vt + 0x28);
        let d2c = arena_word(arena, depes_vt + 0x2c);
        let d40 = arena_word(arena, depes_vt + 0x40);
        let t20 = arena_word(arena, dct_vt + 0x20);
        Self {
            arena,
            platform,
            m34,
            m3c,
            m50,
            m54,
            m58,
            m5c,
            d24,
            d28,
            d2c,
            d40,
            t20,
            rom,
            heap,
            log,
            params,
            first_unbound: None,
            mode_buf: [0; 4096],
            mode_len: 0,
            foreign_buf: [0; 256],
            foreign_len: None,
            ostr_cur: 0,
            layer_bufs: [0; 7],
            nlayer_bufs: 0,
        }
    }

    pub fn first_unbound(&self) -> Option<u32> {
        self.first_unbound
    }

    fn call(&mut self, entry: u32, args: &[u32]) -> u32 {
        let entry = crate::rom_region::to_us_guest(self.rom, entry);

        {
            let mut m = Memory {
                base: 0,
                bytes: &mut *self.arena,
            };
            match engine_live::call_live_m_rom(&mut m, entry, args, &mut *self.platform, self.rom) {
                Ok(rc) => return rc,
                Err(_) => {}
            }
        }

        {
            let mut m = Memory {
                base: 0,
                bytes: &mut *self.arena,
            };
            match crate::component_live_c::core_call_m_rom(&mut m, entry, args, self.rom) {
                Ok(rc) => return rc,
                Err(engine::Error::UnsupportedStage { .. })
                | Err(engine::Error::MissingStreamAdapter { .. }) => {}
                Err(_) => {}
            }
        }

        {
            let mut m = Memory {
                base: 0,
                bytes: &mut *self.arena,
            };
            let mut fwd = DepesFwd {
                platform: &mut *self.platform,
                rom: self.rom,
                first_unbound: None,
            };
            let done = match entry {
                0x218f88 | 0x218ba4 => {
                    if args.len() < 5 {
                        None
                    } else {
                        Some(crate::component_live_c::depes_set_layer_m(
                            &mut m, args[0], args[1], args[2], args[3], args[4], &mut fwd,
                        ))
                    }
                }
                0x2191c8 => {
                    if args.len() < 5 {
                        None
                    } else {
                        Some(crate::component_live_c::depes_get_layer_m(
                            &mut m, args[0], args[1], args[2], args[3], args[4],
                        ))
                    }
                }
                0x2193a4 => {
                    if args.len() < 3 {
                        None
                    } else {
                        let mut ws = crate::component_live_c::VmWorkspace::default();
                        Some(crate::component_live_c::depes_exec_module_m(
                            &mut m, args[0], args[1], args[2], &mut ws, &mut fwd,
                        ))
                    }
                }
                0x1ed1d0 => {
                    if args.len() < 4 {
                        None
                    } else {
                        Some(crate::component_live_c::dctlkp_canonical_token_m(
                            &mut m, args[0], args[1], args[2], args[3], &mut fwd,
                        ))
                    }
                }
                _ => None,
            };
            if let Some(rc) = done {
                if let Some(e) = fwd.first_unbound {
                    if self.first_unbound.is_none() {
                        self.first_unbound = Some(e);
                    }
                }
                if rc == 0x80002001 && self.first_unbound.is_none() {
                    self.first_unbound = Some(entry);
                }
                return rc;
            }
            if let Some(e) = fwd.first_unbound {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(e);
                }
            }
        }
        if self.first_unbound.is_none() {
            self.first_unbound = Some(entry);
        }
        0x80002001
    }

    fn scratch_cstr(&mut self, s: &[u8]) -> u32 {
        let at = G_SCRATCH as usize;
        assert!(s.len() + 1 <= 0x400, "cstr too long");
        self.arena[at..at + s.len()].copy_from_slice(s);
        self.arena[at + s.len()] = 0;
        G_SCRATCH
    }

    fn scratch_bytes(&mut self, s: &[u8]) -> u32 {
        let at = (G_SCRATCH + 0x400) as usize;

        debug_assert!(s.len() <= 0x400);
        self.arena[at..at + s.len()].copy_from_slice(s);
        G_SCRATCH + 0x400
    }

    fn copy_m58_text(&mut self, handle: [u32; 2], node: u16, mode: u32, out: &mut [u8]) -> (u32, u16) {

        let out_slot = G_SCRATCH + 0x100;
        let aux = G_SCRATCH + 0x108;
        let rc = self.call(
            self.m58,
            &[handle[0], handle[1], u32::from(node), mode, out_slot, aux],
        );
        if rc >= 0x80000000 {
            return (rc, 0);
        }
        let ptr = arena_word(self.arena, out_slot) as usize;
        let len = arena_word(self.arena, aux) as usize;
        let take = len.min(out.len());
        out[..take].copy_from_slice(&self.arena[ptr..ptr + take]);
        (rc, len as u16)
    }
}

impl crate::prosody::GlobalProcessHost for LiveGlobalHost<'_, '_> {
    fn validate(&mut self, object: u32, b: u32, tag: u32, size: u32) -> u32 {
        self.call(0x176f94, &[object, b, tag, size])
    }
    fn foreign_param(&mut self, _env: u32) -> (u32, Option<&[u8]>) {
        let key = self.scratch_cstr(b"foreignpron");
        let out_ptr = G_SCRATCH + 0x100;

        {
            let at = out_ptr as usize;
            self.arena[at..at + 4].copy_from_slice(&[0; 4]);
        }
        let rc = self.call(0x175a1c, &[self.params, key, out_ptr]);
        if rc >= 0x80000000 {
            return (rc, None);
        }
        let mode_ptr = arena_word(self.arena, out_ptr);
        if mode_ptr == 0 {
            return (rc, None);
        }

        let mut len = 0;
        while len < self.foreign_buf.len() {
            let b = self.arena[mode_ptr as usize + len];
            if b == 0 {
                break;
            }
            len += 1;
        }
        self.foreign_buf[..len].copy_from_slice(&self.arena[mode_ptr as usize..mode_ptr as usize + len]);
        self.foreign_len = Some(len);
        let out: &[u8] = &self.foreign_buf[..len];
        (rc, Some(out))
    }
    fn first_node(&mut self, handle: [u32; 2], kind: i32, sub: u32) -> (u32, u16) {
        let out = G_SCRATCH + 0x100;
        let rc = self.call(self.m34, &[handle[0], handle[1], kind as u32, sub, out]);
        if rc >= 0x80000000 {
            return (rc, 0);
        }
        (rc, arena_word(self.arena, out) as u16)
    }
    fn next_node(&mut self, handle: [u32; 2], node: u16) -> (u32, u16) {
        let out = G_SCRATCH + 0x100;
        let rc = self.call(self.m3c, &[handle[0], handle[1], u32::from(node), out]);
        if rc >= 0x80000000 {
            return (rc, 0);
        }
        (rc, arena_word(self.arena, out) as u16)
    }
    fn node_field(
        &mut self,
        handle: [u32; 2],
        node: u16,
        field: i32,
        count: i32,
    ) -> (u32, u32, u16) {
        let out = G_SCRATCH + 0x100;
        let aux = G_SCRATCH + 0x108;
        {
            let at = out as usize;
            self.arena[at..at + 4].copy_from_slice(&[0; 4]);
            let at = aux as usize;
            self.arena[at..at + 4].copy_from_slice(&[0; 4]);
        }
        let rc = self.call(
            self.m54,
            &[
                handle[0],
                handle[1],
                u32::from(node),
                field as u32,
                count as u32,
                out,
                aux,
            ],
        );
        if rc >= 0x80000000 {
            return (rc, 0, 0);
        }
        (
            rc,
            arena_word(self.arena, out),
            arena_word(self.arena, aux) as u16,
        )
    }
    fn node_count(&mut self, handle: [u32; 2], node: u16, field: i32) -> (u32, u16) {
        let out = G_SCRATCH + 0x100;
        let rc = self.call(
            self.m5c,
            &[handle[0], handle[1], u32::from(node), field as u32, out],
        );
        if rc >= 0x80000000 {
            return (rc, 0);
        }
        (rc, arena_word(self.arena, out) as u16)
    }
    fn input_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16) {
        self.copy_m58_text(handle, node, 0, out)
    }
    fn word_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16) {
        self.copy_m58_text(handle, node, 3, out)
    }
    fn alt_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16) {
        self.copy_m58_text(handle, node, 10, out)
    }
    fn tag_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16) {
        self.copy_m58_text(handle, node, 4, out)
    }
    fn layer_byte(&mut self, handle: [u32; 2], node: u16, field: i32) -> (u32, u8, u16) {

        let out_slot = G_SCRATCH + 0x100;
        let aux = G_SCRATCH + 0x108;
        let rc = self.call(
            self.m58,
            &[handle[0], handle[1], u32::from(node), field as u32, out_slot, aux],
        );
        if rc >= 0x80000000 {
            return (rc, 0, 0);
        }
        let ptr = arena_word(self.arena, out_slot) as usize;
        (rc, self.arena[ptr], arena_word(self.arena, aux) as u16)
    }
    fn mode_text(&mut self, handle: [u32; 2], node: u16) -> (u32, &[u8], u16) {
        let out_slot = G_SCRATCH + 0x100;
        let aux = G_SCRATCH + 0x108;
        let rc = self.call(
            self.m58,
            &[handle[0], handle[1], u32::from(node), 4, out_slot, aux],
        );
        if rc >= 0x80000000 {
            return (rc, &[], 0);
        }
        let ptr = arena_word(self.arena, out_slot) as usize;
        let len = arena_word(self.arena, aux) as usize;
        let take = len.min(self.mode_buf.len());
        self.mode_buf[..take].copy_from_slice(&self.arena[ptr..ptr + take]);
        self.mode_len = take;
        let out: &[u8] = &self.mode_buf[..take];
        (rc, out, len as u16)
    }
    fn set_pred(&mut self, handle: [u32; 2], node: u16, len: u32, src: &[u8]) -> u32 {

        let s = self.scratch_bytes(&src[..len as usize]);
        let out = G_SCRATCH + 0x100;
        self.call(self.m50, &[handle[0], handle[1], u32::from(node), 3, len, s, out])
    }
    fn set_layer_input(&mut self, handle: [u32; 2], index: i32, text: &[u8]) -> u32 {

        let at = G_TEXT as usize;
        self.arena[at..at + text.len()].copy_from_slice(text);
        self.arena[at + text.len()] = 0;
        self.call(self.d24, &[handle[0], handle[1], index as u32, G_TEXT, text.len() as u32])
    }
    fn set_layer_input2(&mut self, handle: [u32; 2], index: i32, text: &[u8]) -> u32 {

        let at = G_TEXT as usize;
        self.arena[at..at + text.len()].copy_from_slice(text);
        self.arena[at + text.len()] = 0;
        self.call(self.d40, &[handle[0], handle[1], index as u32, G_TEXT, text.len() as u32])
    }
    fn layer_output(&mut self, handle: [u32; 2], index: i32, buf: &mut [u8]) -> (u32, u16) {
        let buf_ptr = G_SCRATCH + 0x110;
        let len_ptr = G_SCRATCH + 0x108;
        {
            let at = G_TEXT as usize;
            self.arena[at..at + G_TEXT_LEN].fill(0);
            let at = buf_ptr as usize;
            self.arena[at..at + 4].copy_from_slice(&[0; 4]);
        }

        let rc = self.call(
            self.d28,
            &[handle[0], handle[1], index as u32, buf_ptr, len_ptr],
        );
        if rc >= 0x80000000 {
            return (rc, 0);
        }
        let src = arena_word(self.arena, buf_ptr);
        let len = arena_word(self.arena, len_ptr) as usize;

        if index == 0 {
            self.ostr_cur = src;
        }
        let take = len.min(buf.len());
        buf[..take].copy_from_slice(&self.arena[src as usize..src as usize + take]);
        (rc, len as u16)
    }
    fn run_layers(&mut self, handle: [u32; 2], key: &[u8]) -> u32 {
        let k = self.scratch_cstr(key);
        self.call(self.d2c, &[handle[0], handle[1], k])
    }
    fn prepare_layer(&mut self, handle: [u32; 2], tag: &[u8], text: &mut [u8]) -> u32 {
        let t = self.scratch_cstr(tag);

        let z = text.iter().position(|&b| b == 0).map(|i| i + 1).unwrap_or(text.len());
        let at = G_TEXT as usize;
        self.arena[at..at + z].copy_from_slice(&text[..z]);
        let rc = self.call(self.t20, &[handle[0], handle[1], t, G_TEXT]);
        if rc >= 0x80000000 {
            return rc;
        }
        let len = z;

        let mut i = 0;
        while i < len {
            text[i] = self.arena[at + i];
            i += 1;
        }
        rc
    }
    fn log_layer(&mut self, _env: u32, _stage: u8, _key: &[u8], _text: &[u8]) {}
    fn log_nomem(&mut self, _env: u32) {}
    fn alloc_layer(&mut self, _env: u32, size: u16) -> u32 {
        let addr = self.call(0x196394, &[self.heap, u32::from(size)]);
        if self.nlayer_bufs < self.layer_bufs.len() {
            self.layer_bufs[self.nlayer_bufs] = addr;
            self.nlayer_bufs += 1;
        }
        addr
    }
    fn free_layer(&mut self, _env: u32, addr: u32) {
        let _ = self.call(0x196428, &[self.heap, addr]);
    }

    fn guest_read_byte(&self, addr: u32) -> u8 {
        self.arena.get(addr as usize).copied().unwrap_or(0)
    }
    fn guest_write_byte(&mut self, addr: u32, val: u8) {
        if (addr as usize) < self.arena.len() {
            self.arena[addr as usize] = val;
        }
    }
    fn guest_write_bytes(&mut self, addr: u32, data: &[u8]) {
        let at = addr as usize;
        if at.checked_add(data.len()).is_some_and(|end| end <= self.arena.len()) {
            self.arena[at..at + data.len()].copy_from_slice(data);
        }
    }
    fn token_cursor(&self) -> u32 {
        self.ostr_cur
    }
    fn set_token_cursor(&mut self, cur: u32) {
        self.ostr_cur = cur;
    }
    fn staging_buffer(&self, index: i32) -> u32 {
        if index < 0 || index >= self.layer_bufs.len() as i32 {
            return 0;
        }
        self.layer_bufs[index as usize]
    }
    fn terminate_layer_text(&mut self, len: u16) {

        if self.ostr_cur != 0 {
            let end = self.ostr_cur as usize + len as usize;
            if end < self.arena.len() {
                self.arena[end] = 0;
            }
        }
    }
}

pub fn run_1f51f8(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
) -> (u32, Option<u32>) {
    run_1f51f8_rom(m, args, platform, crate::rom_region::Rom::Us)
}

pub fn run_1f51f8_rom(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    rom: crate::rom_region::Rom,
) -> (u32, Option<u32>) {
    let p = args[0];
    let b = args[1];
    let handle = [args[2], args[3]];
    let done_addr = args[4];

    let mut words = [0u32; 16];
    {
        let at = p as usize;
        for (i, w) in words.iter_mut().enumerate() {
            *w = u32::from_le_bytes(m.bytes[at + i * 4..at + i * 4 + 4].try_into().unwrap());
        }
    }
    let mut global = crate::prosody::Global {
        id: p,
        env: words[0],
        ling_if: words[1],
        depes: [words[2], words[3]],
        depes_if: words[4],
        dct: [words[5], words[6]],
        dct_if: words[7],
        f20: words[8],
        f24: words[9],
        f28: words[10],
        f2c: words[11],
        f30: words[12],
        f34: words[13],
        f38: words[14],
        use_tn_tag: words[15] as u8,
        clc_pipeline: (words[15] >> 8) as u8,
        foreign: (words[15] >> 16) as u8,
    };
    let mut done: u8 = 0;

    let mut l0 = [0u8; 65536];
    let mut l1 = [0u8; 65536];
    let mut l2 = [0u8; 65536];
    let mut l3 = [0u8; 65536];
    let mut l4 = [0u8; 65536];
    let mut l5 = [0u8; 65536];
    let mut l6 = [0u8; 65536];
    let mut input = [0u8; 8192];
    let mut word = [0u8; 8192];
    let mut w80 = [0u8; 8192];
    let mut lastw = [0u8; 8192];
    let mut output = [0u8; 8192];

    let s_at = G_SCRATCH as usize;
    let mut saved = [0u8; G_SCRATCH_LEN];
    saved.copy_from_slice(&m.bytes[s_at..s_at + G_SCRATCH_LEN]);
    let (rc, first) = {
        let mut buffers = crate::prosody::GlobalBuffers {
            layers: [&mut l0[..], &mut l1[..], &mut l2[..], &mut l3[..], &mut l4[..], &mut l5[..], &mut l6[..]],
            input: &mut input[..],
            input_prefix: 0,
            word: &mut word[..],
            w80: &mut w80[..],
            lastw: &mut lastw[..],
            output: &mut output[..],
        };
        let mut host = LiveGlobalHost::new(m.bytes, platform, p, rom);
        let rc = crate::prosody::process_global_stage(&mut global, b, handle, &mut done, &mut buffers, &mut host);
        (rc, host.first_unbound())
    };

    let back = global.words();
    {
        let at = p as usize;
        for (i, w) in back.iter().enumerate() {
            m.bytes[at + i * 4..at + i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        m.bytes[done_addr as usize] = done;
    }
    m.bytes[s_at..s_at + G_SCRATCH_LEN].copy_from_slice(&saved);
    (rc, first)
}
