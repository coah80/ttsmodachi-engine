use crate::{
    boot_ports as cold, filesystem,
    image::{mask_patch, Dump},
};
use std::path::Path;
use tts_core::{boot, engine_us};

pub struct BootImage {
    pub arena: Vec<u8>,
    pub boot: engine_us::BootArgs,
}
impl BootImage {
    pub fn load(path: &Path) -> Result<Self, String> {
        let path = path.to_owned();
        std::thread::Builder::new()
            .name("rom-boot".into())
            .stack_size(512 * 1024 * 1024)
            .spawn(move || construct(&path))
            .map_err(|e| format!("cannot start ROM boot: {e}"))?
            .join()
            .map_err(|_| "ROM boot failed".to_owned())?
    }
}
struct RomRec;
impl boot::Calls for RomRec {
    fn real_log_args(&mut self) -> bool {
        true
    }
    fn call(&mut self, m: &mut boot::Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        cold::route_call(m, self, entry, args)
            .unwrap_or_else(|| panic!("unbound boot port {entry:#x}"))
    }
    fn call_wide(&mut self, _: &mut boot::Memory<'_>, entry: u32, _: &[u32]) -> u64 {
        panic!("unbound boot wide call {entry:#x}")
    }
}
fn r32(a: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(a[p as usize..p as usize + 4].try_into().unwrap())
}
fn w32(a: &mut [u8], p: u32, v: u32) {
    a[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn wide(a: &[u8], p: u32) -> Result<String, String> {
    let mut text = Vec::new();
    let mut p = p as usize;
    loop {
        let pair = a.get(p..p + 2).ok_or("invalid ROM archive path")?;
        let v = u16::from_le_bytes(pair.try_into().unwrap());
        if v == 0 {
            return String::from_utf16(&text).map_err(|_| "invalid ROM archive path".into());
        }
        text.push(v);
        p += 2;
        if text.len() > 4096 {
            return Err("invalid ROM archive path".into());
        }
    }
}
fn preload_archives(arena: &mut [u8], mgr: u32, dump: &Dump) -> Result<usize, String> {
    let image = dump.romfs()?;
    let ivfc = image
        .get(0..0x60)
        .ok_or("invalid RomFS")?
        .try_into()
        .unwrap();
    let base = filesystem::romfs_level3(0, ivfc)?;
    let files = filesystem::walk_romfs(image, base)?;
    let mut names: Vec<u32> = (0..7)
        .map(|i| r32(arena, cold::PATH_TABLE + i * 4))
        .collect();
    for table in [cold::RULES_NAMES, cold::COMPONENTS_NAMES, cold::DICT_NAMES] {
        names.push(r32(arena, table + 4));
    }
    let mut count = 0;
    for name in names {
        if name == 0 {
            continue;
        }
        let uri = wide(arena, name)?;
        let Some(relative) = uri
            .strip_prefix("rom:")
            .or_else(|| uri.strip_prefix("ext:"))
        else {
            continue;
        };
        let Some(file) = files.iter().find(|f| f.path == relative) else {
            continue;
        };
        let raw = image
            .get(file.range.0 as usize..(file.range.0 + file.range.1) as usize)
            .ok_or("invalid ROM archive range")?;
        let mut calls = RomRec;
        let buffer = cold::fn_129778(
            &mut boot::Memory {
                base: 0,
                bytes: arena,
            },
            &mut calls,
            raw.len() as u32,
        );
        if buffer == 0 {
            return Err("not enough memory to load ROM archives".into());
        }
        arena[buffer as usize..buffer as usize + raw.len()].copy_from_slice(raw);
        let entry = mgr + cold::MGR_REGISTRY + 4 + count * cold::REG_ENTRY;
        for (off, value) in [(0, name), (4, buffer), (8, raw.len() as u32)] {
            w32(arena, entry + off, value);
        }
        count += 1;
    }
    w32(arena, mgr + cold::MGR_REGISTRY, count);
    Ok(count as usize)
}
fn construct(path: &Path) -> Result<BootImage, String> {
    let dump = Dump::open(path)?;
    dump.ensure_decrypted()?;
    if dump.title_id != 0x000400000008c300 {
        return Err("wrong region: this engine needs a decrypted US Rev 1 dump".into());
    }
    let mut code = dump.code()?;
    dump.verify_us(&code)?;
    mask_patch(&mut code);
    const ARENA: usize = 0x1610_0000;
    const ENGINE: u32 = 0x1540_0000;
    let mut arena = vec![0u8; ARENA];
    arena[0x100000..0x100000 + code.len()].copy_from_slice(&code);
    arena[0xC72000..0xC72880].fill(0);
    for site in [0x104C84, 0x191860, 0x163C1C, 0x163C2C, 0x1834AC, 0x1834BC] {
        arena[site..site + 4].fill(0);
    }

    let tmp = ENGINE + 0x8000;
    let rscratch = ENGINE + 0x9000;
    let mgr = {
        let mut m = boot::Memory {
            base: 0,
            bytes: &mut arena,
        };
        let mut bh = RomRec;

        cold::bootstrap_heap0(&mut m, ENGINE);
        cold::startup_output_lock(&mut m, &mut bh);

        cold::bootstrap_slot1_free(&mut m);

        let mgr = cold::bootstrap_manager(&mut m, &mut bh);
        cold::bootstrap_archive_alloc(&mut m, &mut bh);

        if mgr == 0 {
            return Err("ROM boot could not construct the engine".into());
        }
        mgr
    };

    if mgr == 0 {
        return Err("ROM boot could not construct the engine".into());
    }
    let _narch = preload_archives(&mut arena, mgr, &dump)?;

    let vhandle = {
        let mut h = RomRec;
        let mut m = boot::Memory {
            base: 0,
            bytes: &mut arena,
        };
        let mut rt = cold::Rt { inner: &mut h };

        let vw = cold::fn_voice_setup(&mut m, &mut rt, ENGINE);

        if vw == 0 {
            return Err("ROM boot could not construct the engine".into());
        }

        vw
    };
    {
        let mut h = RomRec;
        let mut m = boot::Memory {
            base: 0,
            bytes: &mut arena,
        };
        let mut rt = cold::Rt { inner: &mut h };
        cold::fn_35aa40(&mut m, &mut rt, ENGINE, tmp);
    }

    {
        let mut h = RomRec;
        let mut m = boot::Memory {
            base: 0,
            bytes: &mut arena,
        };
        let mut rt = cold::Rt { inner: &mut h };
        let roots = [
            0x132124, 0xA7B69C, 0x8BF980, 0xA7B67C, 0x8BF9C0, 0xA7B70C, 0x8C016C, 0x8BFC24,
            0x8BFE64,
        ];
        cold::fn_11cc0c_full(&mut m, &mut rt, ENGINE, 1, &roots, rscratch);
    }

    let worker = r32(&arena, ENGINE + 0x124);
    let want = r32(&arena, 0x190E8C);

    if worker == 0 || want == 0 {
        return Err("ROM boot could not construct the engine".into());
    }
    let outcell = ENGINE + 0x11C;
    if r32(&arena, outcell) == 0 {
        return Err("ROM boot could not construct the engine".into());
    }

    {
        let mut m = boot::Memory {
            base: 0,
            bytes: &mut arena,
        };
        let mut h = RomRec;
        let mut rt = cold::Rt { inner: &mut h };
        cold::fn_35a958(&mut m, &mut rt, ENGINE);
    }

    let (req_obj, req_sp) = {
        let mut m = boot::Memory {
            base: 0,
            bytes: &mut arena,
        };
        let mut h = RomRec;
        let mut rt = cold::Rt { inner: &mut h };
        let q = cold::fn_129778(&mut m, &mut rt, 12);
        let s = cold::fn_129778(&mut m, &mut rt, 0x1000);

        (q, s)
    };

    let engine_obj = {
        let r = |o: u32| {
            u32::from_le_bytes(
                arena[(ENGINE + o) as usize..(ENGINE + o + 4) as usize]
                    .try_into()
                    .unwrap(),
            )
        };
        r(0x11C)
    };
    let boot = engine_us::BootArgs {
        engine: engine_obj,
        magic: 0x684F424A,
        request: req_obj,
        request_sp: req_sp,
        voice: vhandle,
        s16: 0,
        s17: 0,
        tick: 0,
    };
    Ok(BootImage { arena, boot })
}
