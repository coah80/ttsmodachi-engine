

use crate::{
    boot::Memory,
    engine, engine_live,
    rom_region::{addr, to_us, Rom},
};

pub fn run_step(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn engine_live::RuntimeServicesM,
    snap: &[u8],
) -> Result<u32, engine::Error> {
    engine_live::call_live_with_runtime_m(m, 0x1cca64, args, platform, runtime, snap)
}

pub struct Probe1d5ba8<'a> {
    pub first_unbound: Option<u32>,
    platform: &'a mut dyn engine::HeapPlatform,
    rom: Rom,
}

impl<'a> Probe1d5ba8<'a> {
    pub fn new(platform: &'a mut dyn engine::HeapPlatform) -> Self {
        Probe1d5ba8::new_rom(platform, Rom::Us)
    }
    pub fn new_rom(platform: &'a mut dyn engine::HeapPlatform, rom: Rom) -> Self {
        Probe1d5ba8 {
            first_unbound: None,
            platform,
            rom,
        }
    }
    fn unbound(&mut self, f: u32) -> u32 {
        if self.first_unbound.is_none() {
            self.first_unbound = Some(f);
        }
        0x80002001
    }
}

fn process_environment_rom(rom: Rom) -> crate::speech_part_1::ProcessEnvironment {
    match rom {
        Rom::Us => crate::component_live_c::process_environment_us(),
        Rom::Eu => crate::component_live_c::process_environment_eu(),
        Rom::Kr => crate::component_live_c::process_environment_kr(),
    }
}

fn speech_literals_rom(rom: Rom) -> crate::speech_part_2::SpeechLiterals {
    let u = crate::component_live_c::speech_literals_us();
    crate::speech_part_2::SpeechLiterals {
        text: core::array::from_fn(|i| addr(rom, u.text[i])),
        end: addr(rom, u.end),
        errors: core::array::from_fn(|i| addr(rom, u.errors[i])),
    }
}

impl crate::component_live_b::Host for Probe1d5ba8<'_> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        <Self as crate::component_live_c::Host>::call_m(self, f, a, m)
    }
    fn rom(&self) -> Rom {
        self.rom
    }
}

impl crate::component_live_c::Host for Probe1d5ba8<'_> {
    fn rom(&self) -> Rom {
        self.rom
    }
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {

        let f = to_us(self.rom, f);
        if f == 0x15017c {
            return crate::component_live_c::fn_15017c_m(m, a, self);
        }
        if f == 0x1516c8 {
            crate::component_live_c::release_typed_buffer_m(m, a, self);
            return 0;
        }
        if f == 0x15c1e4 {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::clone_record_live_m(m, abi, 0xe0fef80, self);
        }
        if f == 0x15c1c8 && self.rom != Rom::Us {

            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::clone_record_live_m(m, abi, 0xe0fef80, self);
        }
        if f == 0xffff0001 {
            return self.unbound(f);
        }
        if [
            0x17a1a4, 0x179374, 0x179208, 0x17c9d8, 0x1b2760, 0x1b254c, 0x1b2974, 0x172de0,
            0x172d30, 0x4a794c, 0x4a78f0,
        ]
        .contains(&f)
        {
            return crate::component_live_c::broker_range_live_m(m, f, a, 0xe0fcf00, self);
        }
        if f == 0x26bb6c {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            let mut workspace = crate::component_live_c::DictionaryWorkspace::default();
            match crate::component_live_c::dictionary_lookup_entry_m(m, abi, &mut workspace, self)
            {
                Ok(value) => return value,
                Err(_) => return self.unbound(f),
            }
        }
        if f == 0x1ea4e0 {
            let Ok(abi): Result<[u32; 7], _> = a.try_into() else {
                return self.unbound(f);
            };
            match crate::component_live_c::lookup_dictionary_live_m(m, abi, 0xe0fd000, self) {
                Ok(rc) => return rc,
                Err(_) => return self.unbound(f),
            }
        }
        if f == 0x1963e0 {

            match crate::component_live_c::heap_call_m_rom(m, f, a, &mut *self.platform, self.rom) {
                Ok(p) => return p,
                Err(_) => {
                    if self.first_unbound.is_none() {
                        self.first_unbound = Some(f);
                    }
                    return 0;
                }
            }
        }
        if f == 0x15d0c4 {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            match crate::component_live_c::read_field_m(m, abi, addr(self.rom, 0x8d7ca0), 0xe0feb00, &mut [0u8; 64])
            {
                Ok(rc) => return rc,
                Err(_) => return self.unbound(f),
            }
        }
        if f == 0x15bf74 {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::create_child_live_m(m, abi, 0xe0fef80, self);
        }
        if f == 0x15b758 {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::create_record_live_m(m, abi, 0xe0fee00, self);
        }
        if f == 0x1ede44 {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::fn_1ede44_m(
                m,
                abi,
                &speech_literals_rom(self.rom),
                0xe0fed00,
                self,
            );
        }
        if f == 0x15bab0 {
            if a.len() < 2 {
                return self.unbound(f);
            }
            return crate::component_live_c::ld_reset_live_m(m, a[0], a[1], 0xe0fef00, self);
        }
        if f == 0x15cdcc {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::write_field_live_m(m, abi, self);
        }
        if f == 0x15d378 {

            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            match crate::component_live_c::field_data_m(m, abi, addr(self.rom, 0x8d7ca0)) {
                Ok(rc) => return rc,
                Err(_) => return self.unbound(f),
            }
        }
        if f == 0x15d6c4 {

            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            match crate::component_live_c::enquire_child_m(m, abi, self.rom) {
                Ok(rc) => return rc,
                Err(_) => return self.unbound(f),
            }
        }
        if f == 0x176f2c || f == 0x176f5c {

            if a.is_empty() {
                return self.unbound(f);
            }
            struct LockHost<'x, 'y> {
                probe: &'x mut Probe1d5ba8<'y>,
            }
            impl crate::boot_part_f::Host for LockHost<'_, '_> {
                fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
                    <Probe1d5ba8<'_> as crate::component_live_c::Host>::call_m(
                        self.probe, method, args, m,
                    )
                }
                fn compare(
                    &mut self,
                    _: &mut Memory<'_>,
                    _: u32,
                    _: &mut crate::boot_part_f::Key,
                    _: &mut u32,
                ) -> u32 {

                    0x80002001
                }
            }
            let mut host = LockHost { probe: self };
            return if f == 0x176f2c {
                crate::boot_port_1::fn_176f2c(m, a[0], &mut host)
            } else {
                crate::boot_port_1::fn_176f5c(m, a[0], &mut host)
            };
        }
        if f == 0x195d90 {

            if a.is_empty() || (a[0] as usize) >= m.bytes.len() {
                return self.unbound(f);
            }
            return crate::frontend::strlen_bytes(&m.bytes[a[0] as usize..]);
        }
        if f == 0x1769d0 {

            if a.len() < 2 {
                return self.unbound(f);
            }
            return crate::fe_text::strcmp_ci_1769d0(m.bytes, a[0], a[1]) as u32;
        }
        if f == 0x1770f8 || f == 0x1771a8 || f == 0x177154 {

            if a.len() < 2 {
                return self.unbound(f);
            }
            return crate::speech_part_1::fn_1770f8(m, a[0], a[1]);
        }

        if f == 0x26b7ec && a.len() >= 4 {
            return crate::component_live_b3::fn_26b7ec_m(m, self, a[0], a[1], a[2], a[3]);
        }
        if f == 0x26bb28 && a.len() >= 2 {
            return crate::component_live_b3::fn_26bb28_m(m, self, a[0], a[1]);
        }
        if f == 0x26bb44 && a.len() >= 1 {
            return crate::component_live_b3::fn_26bb44_m(m, a[0]);
        }
        if f == 0x179f3c {

            if a.len() < 2 {
                return self.unbound(f);
            }
            let mut inner = crate::component_live_c2::OpenHost {
                query: 0xe0fc800,
                pending_release: None,
            };
            let rc = {
                let mut host = crate::component_live_c2::CloseHost { inner: &mut inner };
                crate::leftovers_1::fn_179f3c(m, a[0], a[1], &close_runtime_rom(self.rom), &mut host)
            };
            if let Some(object) = inner.pending_release.take() {

                let rd = |at: u32| -> Option<u32> {
                    m.bytes
                        .get(at as usize..at as usize + 4)
                        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                };
                match rd(a[0] + 4).and_then(|p| rd(p + 8)).and_then(|b| rd(b + 4)) {
                    Some(heap) => crate::boot_part_e::release(m, heap, object, &mut inner),
                    None => return self.unbound(f),
                }
            }
            return rc;
        }
        if f == 0x179f30 {

            if a.len() < 3 {
                return self.unbound(f);
            }
            return crate::component_live_b3::broker_179f30_live(m, a[0], a[1], a[2], self);
        }
        if f == 0x15c544 {

            let mut host = crate::component_live_b::LiveHost {
                platform: &mut *self.platform,
                fault: None,
                scratch: 0xe0fcf00,
            rom: crate::rom_region::Rom::Us,
            };
            let rc = host.call_m_with(f, a, m, self.rom);
            if host.fault.is_some() {
                return self.unbound(f);
            }
            return rc;
        }
        let result = if [
            0x196394, 0x1963b8, 0x196428, 0x4a779c, 0x4a7758, 0x4a774c, 0x39f5e8, 0x39f5b8,
        ]
        .contains(&f)
        {
            crate::component_live_c::heap_call_m(m, f, a, &mut *self.platform)
        } else {
            crate::component_live_c::core_call_m_rom(m, f, a, self.rom)
        };
        match result {
            Ok(rc) => rc,
            Err(_) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(f);
                }
                if [0x196394, 0x1963b8, 0x1963e0].contains(&f) {
                    0
                } else {
                    0x80002001
                }
            }
        }
    }
}

fn close_runtime_rom(rom: Rom) -> crate::leftovers_1::CloseRuntime {
    let addr = |us| crate::rom_region::addr(rom, us);
    let hash = |bytes: &[u8]| {
        bytes
            .iter()
            .fold(2166136261u32, |h, &b| (h ^ b as u32).wrapping_mul(16777619))
    };
    crate::leftovers_1::CloseRuntime {
        category: addr(0x8d7668),
        begin: addr(0x17a14c),
        end: addr(0x17a18c),
        error: addr(0x17a180),
        handle_key: hash(b"hData"),
        status_key: hash(b"lhError"),
        scratch: 0xe0fc840,
        fold: crate::component_live_c2::ascii_fold_classes(),
    }
}

pub fn run_1d5ba8(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
) -> (u32, Option<u32>) {
    run_1d5ba8_rom(m, args, platform, Rom::Us)
}

pub fn run_1d5ba8_rom(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    rom: Rom,
) -> (u32, Option<u32>) {
    let mut host = Probe1d5ba8::new_rom(platform, rom);
    let env = process_environment_rom(rom);
    let rc = crate::component_live_c::fn_1d5ba8_m(
        m,
        args[0],
        args[1],
        [args[2], args[3]],
        args[4],
        &env,
        0xe0ff080,
        &mut host,
    );
    (rc, host.first_unbound)
}

pub struct Probe200e64<'a> {
    pub first_unbound: Option<u32>,
    platform: &'a mut dyn engine::HeapPlatform,
    rom: Rom,
}

impl<'a> Probe200e64<'a> {
    pub fn new(platform: &'a mut dyn engine::HeapPlatform) -> Self {
        Probe200e64::new_rom(platform, Rom::Us)
    }
    pub fn new_rom(platform: &'a mut dyn engine::HeapPlatform, rom: Rom) -> Self {
        Probe200e64 {
            first_unbound: None,
            platform,
            rom,
        }
    }
    fn unbound(&mut self, f: u32) -> u32 {
        if self.first_unbound.is_none() {
            self.first_unbound = Some(f);
        }
        0x80002001
    }
}

impl crate::component_live_c::Host for Probe200e64<'_> {
    fn rom(&self) -> Rom {
        self.rom
    }
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {

        let f = to_us(self.rom, f);
        if f == 0x173dc0 {

            return 0;
        }
        if f == 0x15cdcc {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::write_field_live_m(m, abi, self);
        }
        if f == 0x1ff4fc {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::insert_prompt_live_m(m, abi, 0xe0ff600, self);
        }
        let result = if [0x196394, 0x1963b8, 0x196428].contains(&f) {
            crate::component_live_c::heap_call_m(m, f, a, &mut *self.platform)
        } else {
            crate::component_live_c::core_call_m_rom(m, f, a, self.rom)
        };
        match result {
            Ok(rc) => rc,
            Err(e) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(f);
                }
                let _ = e;
                0x80002001
            }
        }
    }
}

pub fn run_200e64(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
) -> (u32, Option<u32>) {
    run_200e64_rom(m, args, platform, Rom::Us)
}

pub fn run_200e64_rom(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    rom: Rom,
) -> (u32, Option<u32>) {
    let mut host = Probe200e64::new_rom(platform, rom);
    let rc = crate::component_live_c::prompt_orth_live_m(
        m,
        args[0],
        args[1],
        [args[2], args[3]],
        args[4],
        0xe0ff700,
        &mut host,
    );
    (rc, host.first_unbound)
}

fn source_literal(bytes: &[u8], text: &[u8]) -> u32 {
    let end = bytes.len().min(0x500000);
    if end <= 0x100000 {
        return 0;
    }
    (0x100000
        + bytes[0x100000..end]
            .windows(text.len() + 1)
            .position(|w| &w[..text.len()] == text && w[text.len()] == 0)
            .expect("mapped source literal")) as u32
}

fn fault_entry(e: &engine::Error) -> u32 {
    match *e {
        engine::Error::UnsupportedStage { entry } => entry,
        engine::Error::MissingStreamAdapter { entry } => entry,
        _ => 0,
    }
}

pub fn run_1e4b7c(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
) -> (u32, Option<u32>) {
    run_1e4b7c_rom(m, args, platform, Rom::Us)
}

pub fn run_1e4b7c_rom(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    _rom: Rom,
) -> (u32, Option<u32>) {
    let mut host = crate::component_live_b2::TokenHost {
        inner: crate::component_live_b::LiveHost {
            platform,
            fault: None,
            scratch: 0x500,
            rom: _rom,
        },
        heap: 0,
    };

    host.heap = m.word(m.word(m.word(args[0])) + 4);
    let end = source_literal(m.bytes, b"At End of Processing");
    let a5 = [args[0], args[1], args[2], args[3], args[4]];
    let rc = crate::component_live_b::fn_1e4b7c_m(m, a5, end, 0x600, &mut host);
    let unbound = match &host.inner.fault {
        None => {
            if rc >= 0x80000000 {
                Some(0x1e4b7c)
            } else {
                None
            }
        }
        Some(e) => {
            let en = fault_entry(e);
            Some(if en == 0 { 0x1e4b7c } else { en })
        }
    };
    (rc, unbound)
}

pub fn scheduler_advance_after_step(arena: &mut [u8]) {
    let w = u32::from_le_bytes(arena[0x14c403e4..0x14c403e8].try_into().unwrap());
    arena[0x14c403e4..0x14c403e8].copy_from_slice(&w.wrapping_add(1).to_le_bytes());
    let h = u16::from_le_bytes(arena[0x14c403e8..0x14c403ea].try_into().unwrap());
    arena[0x14c403e8..0x14c403ea].copy_from_slice(&h.wrapping_add(1).to_le_bytes());
}
