fn u32le(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}

fn u64le(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

pub fn romfs_level3(romfs_off: u64, ivfc: &[u8; 0x60]) -> Result<u64, String> {
    if &ivfc[0..4] != b"IVFC" {
        return Err("not IVFC".to_string());
    }
    let master = u32le(ivfc, 0x08) as u64;
    let block_shift = u32le(ivfc, 0x4C);
    if block_shift > 20 {
        return Err(format!("implausible IVFC block shift {block_shift}"));
    }
    let block = 1u64 << block_shift;
    Ok(romfs_off + ((0x60u64 + master + block - 1) / block) * block)
}

pub struct RomFsFile {
    pub path: String,
    pub range: (u64, u64),
}

pub fn walk_romfs(image: &[u8], l3: u64) -> Result<Vec<RomFsFile>, String> {
    let l3 = l3 as usize;
    if image.len() < l3 + 40 {
        return Err("image too small for RomFS header".to_string());
    }
    let h: Vec<u32> = (0..10).map(|i| u32le(image, l3 + i * 4)).collect();
    let (dir_meta, dir_size, file_meta, file_size, data_off) = (h[3], h[4], h[7], h[8], h[9]);
    let dirs = l3 + dir_meta as usize;
    let files = l3 + file_meta as usize;
    let data_base = l3 as u64 + data_off as u64;
    if image.len() < dirs + dir_size as usize || image.len() < files + file_size as usize {
        return Err("RomFS metadata runs past the image".to_string());
    }
    let mut out = Vec::new();
    walk_dir(image, dirs, files, data_base, 0, String::new(), &mut out)?;
    Ok(out)
}

fn utf16_name(image: &[u8], off: usize, len: usize) -> Result<String, String> {
    if image.len() < off + len {
        return Err("RomFS name runs past the image".to_string());
    }
    let units: Vec<u16> = (0..len / 2)
        .map(|i| u16::from_le_bytes(image[off + i * 2..off + i * 2 + 2].try_into().unwrap()))
        .collect();
    String::from_utf16(&units).map_err(|e| format!("bad RomFS name: {e}"))
}

fn walk_dir(
    image: &[u8],
    dirs: usize,
    files: usize,
    data_base: u64,
    d: usize,
    prefix: String,
    out: &mut Vec<RomFsFile>,
) -> Result<(), String> {
    let rd = |i: usize| u32le(image, dirs + d + i * 4);
    let (sib, child, fil, nlen) = (rd(1), rd(2), rd(3), rd(5) as usize);
    let name = if d == 0 && nlen == 0 {
        String::new()
    } else {
        utf16_name(image, dirs + d + 0x18, nlen)?
    };
    let path = if d == 0 {
        "/".to_string()
    } else {
        format!("{prefix}{name}/")
    };
    let mut f = fil;
    let mut steps = 0usize;
    let mut seen = std::collections::HashSet::new();
    while f != 0xFFFF_FFFF {
        steps += 1;
        if steps > 100_000 || !seen.insert(f) {
            return Err(format!("RomFS file chain cycles (dir off {d:#x})"));
        }
        let base = files + f as usize;
        if image.len() < base + 32 {
            return Err("RomFS file entry runs past the image".to_string());
        }
        let nxt = u32le(image, base + 4);
        let off = u64le(image, base + 8);
        let size = u64le(image, base + 16);
        let fnl = u32le(image, base + 28) as usize;
        let fname = utf16_name(image, files + f as usize + 0x20, fnl)?;
        out.push(RomFsFile {
            path: format!("{path}{fname}"),
            range: (data_base + off, size),
        });
        f = nxt;
    }
    if child != 0xFFFF_FFFF {
        walk_dir(
            image,
            dirs,
            files,
            data_base,
            child as usize,
            path.clone(),
            out,
        )?;
    }
    if sib != 0xFFFF_FFFF && d != 0 {
        walk_dir(image, dirs, files, data_base, sib as usize, prefix, out)?;
    }
    Ok(())
}
