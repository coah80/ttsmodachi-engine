use sha2::{Digest, Sha256};
use std::{fs, path::Path};

pub struct Dump {
    bytes: Vec<u8>,
    base: usize,
    end: usize,
    pub title_id: u64,
    pub format: &'static str,
}

fn span(b: &[u8], p: usize, n: usize) -> Result<&[u8], String> {
    b.get(p..p.checked_add(n).ok_or("invalid dump: offset overflow")?)
        .ok_or_else(|| "invalid dump: truncated section".into())
}
fn le32(b: &[u8], p: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(span(b, p, 4)?.try_into().unwrap()))
}
fn le64(b: &[u8], p: usize) -> Result<u64, String> {
    Ok(u64::from_le_bytes(span(b, p, 8)?.try_into().unwrap()))
}
fn be16(b: &[u8], p: usize) -> Result<u16, String> {
    Ok(u16::from_be_bytes(span(b, p, 2)?.try_into().unwrap()))
}
fn be32(b: &[u8], p: usize) -> Result<u32, String> {
    Ok(u32::from_be_bytes(span(b, p, 4)?.try_into().unwrap()))
}
fn be64(b: &[u8], p: usize) -> Result<u64, String> {
    Ok(u64::from_be_bytes(span(b, p, 8)?.try_into().unwrap()))
}
fn aligned(n: usize) -> Result<usize, String> {
    n.checked_add(63)
        .map(|n| n & !63)
        .ok_or_else(|| "invalid dump: size overflow".into())
}
fn units(b: &[u8], p: usize) -> Result<usize, String> {
    (le32(b, p)? as usize)
        .checked_mul(512)
        .ok_or_else(|| "invalid dump: section size overflow".into())
}

impl Dump {
    pub fn open(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|e| format!("cannot read dump: {e}"))?;
        Self::from_bytes(bytes)
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let (base, end, format) = if bytes.get(0x100..0x104) == Some(b"NCCH") {
            (0, bytes.len(), "cxi")
        } else if bytes.get(0x100..0x104) == Some(b"NCSD") {
            let mut selected = None;
            for i in 0..8 {
                let off = units(&bytes, 0x120 + i * 8)?;
                let len = units(&bytes, 0x124 + i * 8)?;
                if off == 0 || len == 0 {
                    continue;
                }
                let part = span(&bytes, off, len)?;
                if part.get(0x100..0x104) == Some(b"NCCH") && units(part, 0x1a4)? > 0 {
                    selected = Some((off, off + len));
                    break;
                }
            }
            let (p, e) = selected.ok_or("invalid NCSD dump: no executable partition")?;
            (p, e, "ncsd")
        } else if bytes.len() >= 0x2020 && le32(&bytes, 0)? as usize >= 0x2020 {
            let header = le32(&bytes, 0)? as usize;
            span(&bytes, 0, header)?;
            let cert = le32(&bytes, 8)? as usize;
            let ticket = le32(&bytes, 12)? as usize;
            let tmd_len = le32(&bytes, 16)? as usize;
            let tmd = aligned(header)?
                .checked_add(aligned(cert)?)
                .and_then(|n| n.checked_add(aligned(ticket).ok()?))
                .ok_or("invalid CIA: size overflow")?;
            let metadata = span(&bytes, tmd, tmd_len)?;
            let sig = match be32(metadata, 0)? {
                0x10000 | 0x10003 => 0x240,
                0x10001 | 0x10004 => 0x140,
                0x10002 | 0x10005 => 0x80,
                _ => return Err("invalid CIA: unsupported signature header".into()),
            };
            let count = be16(metadata, sig + 0x9e)? as usize;
            let mut cursor = tmd
                .checked_add(aligned(tmd_len)?)
                .ok_or("invalid CIA: size overflow")?;
            let content_end = cursor
                .checked_add(
                    usize::try_from(le64(&bytes, 0x18)?)
                        .map_err(|_| "invalid CIA: oversized content")?,
                )
                .ok_or("invalid CIA: size overflow")?;
            span(&bytes, cursor, content_end - cursor)?;
            let mut selected = None;
            for i in 0..count {
                let c = sig + 0x9c4 + i * 48;
                let index = be16(metadata, c + 4)? as usize;
                if bytes[0x20 + index / 8] & (0x80 >> (index % 8)) == 0 {
                    continue;
                }
                let len = usize::try_from(be64(metadata, c + 8)?)
                    .map_err(|_| "invalid CIA: oversized content")?;
                let content = span(&bytes, cursor, len)?;
                if cursor + len > content_end {
                    return Err("invalid CIA: content runs past its section".into());
                }
                if content.get(0x100..0x104) == Some(b"NCCH") && units(content, 0x1a4)? > 0 {
                    selected = Some((cursor, cursor + len));
                    break;
                }
                if be16(metadata, c + 6)? & 1 != 0 && content.get(0x100..0x104) != Some(b"NCCH") {
                    return Err("encrypted dump: decrypt the CIA contents before running".into());
                }
                cursor = cursor
                    .checked_add(aligned(len)?)
                    .ok_or("invalid CIA: size overflow")?;
            }
            let (p, e) = selected.ok_or("invalid CIA: no executable content")?;
            (p, e, "cia")
        } else {
            return Err("unsupported dump: use a decrypted .cxi, .3ds, .cci or .cia".into());
        };
        let ncch = &bytes[base..end];
        span(ncch, 0, 0xa00)?;
        let title_id = le64(ncch, 0x118)?;
        let dump = Self {
            bytes,
            base,
            end,
            title_id,
            format,
        };
        dump.section(0x1a0, 0x1a4)?;
        dump.section(0x1b0, 0x1b4)?;
        Ok(dump)
    }
    pub fn ncch(&self) -> &[u8] {
        &self.bytes[self.base..self.end]
    }
    pub fn encrypted(&self) -> bool {
        self.ncch()[0x18f] & 4 == 0
    }
    pub fn ensure_decrypted(&self) -> Result<(), String> {
        if self.encrypted() {
            Err("encrypted dump: use the decrypted dump option and try again".into())
        } else {
            Ok(())
        }
    }
    fn section(&self, o: usize, n: usize) -> Result<&[u8], String> {
        let b = self.ncch();
        span(b, units(b, o)?, units(b, n)?)
    }
    pub fn romfs(&self) -> Result<&[u8], String> {
        self.ensure_decrypted()?;
        self.section(0x1b0, 0x1b4)
    }
    pub fn exefs_code(&self) -> Result<&[u8], String> {
        self.ensure_decrypted()?;
        let e = self.section(0x1a0, 0x1a4)?;
        span(e, 0, 512)?;
        for i in 0..10 {
            let p = i * 16;
            if span(e, p, 8)? == b".code\0\0\0" {
                return span(e, 512 + le32(e, p + 8)? as usize, le32(e, p + 12)? as usize);
            }
        }
        Err("invalid dump: ExeFS has no .code".into())
    }
    pub fn code(&self) -> Result<Vec<u8>, String> {
        let h = self.ncch();
        let packed = self.exefs_code()?;
        let mut expected = 0usize;
        for p in [0x214, 0x224, 0x234] {
            expected = expected
                .checked_add(
                    (le32(h, p)? as usize)
                        .checked_mul(4096)
                        .ok_or("invalid code size")?,
                )
                .ok_or("invalid code size")?;
        }
        if expected == 0 || expected > 64 * 1024 * 1024 {
            return Err("invalid dump: executable is too large".into());
        }
        let code = if packed.len() == expected {
            packed.to_vec()
        } else if h[0x20d] & 1 != 0 {
            decompress_blz(packed, expected)?
        } else {
            return Err("invalid dump: .code size disagrees with the executable header".into());
        };
        if code.len() != expected {
            return Err(
                "invalid dump: decompressed .code size disagrees with the executable header".into(),
            );
        }
        let mut mapped = vec![0u8; 0xb73000];
        let mut source = 0;
        for p in [0x210, 0x220, 0x230] {
            let va = le32(h, p)? as usize;
            let n = le32(h, p + 4)? as usize * 4096;
            let dest = va
                .checked_sub(0x100000)
                .ok_or("invalid dump: code address below executable base")?;
            let end = dest
                .checked_add(n)
                .ok_or("invalid dump: code range overflow")?;
            if end > 64 * 1024 * 1024 {
                return Err("invalid dump: executable address out of range".into());
            }
            if end > mapped.len() {
                mapped.resize(end, 0);
            }
            mapped[dest..end].copy_from_slice(span(&code, source, n)?);
            source += n;
        }
        Ok(mapped)
    }
    pub fn verify_us(&self, code: &[u8]) -> Result<(), String> {
        if self.title_id != 0x000400000008c300 {
            return Err("wrong region: this engine needs a decrypted US Rev 1 dump".into());
        }
        let mut canonical = code.to_vec();
        if canonical.len() != 0xb73000 {
            return Err("wrong revision: this engine needs US Rev 1".into());
        }
        mask_patch(&mut canonical);
        let digest = Sha256::digest(&canonical);
        if digest.as_slice() != US_CODE_HASH {
            return Err("wrong revision or modified code: this engine needs US Rev 1".into());
        }
        Ok(())
    }
}

pub fn mask_patch(code: &mut [u8]) {
    code[0xc72000 - 0x100000..0xc72880 - 0x100000].fill(0);
    for va in [0x104c84, 0x191860, 0x163c1c, 0x163c2c, 0x1834ac, 0x1834bc] {
        code[va - 0x100000..va - 0x100000 + 4].fill(0);
    }
}

pub fn decompress_blz(input: &[u8], limit: usize) -> Result<Vec<u8>, String> {
    let fail = || "invalid dump: malformed BLZ-compressed .code".to_owned();
    if input.len() < 8 {
        return Err(fail());
    }
    let footer = le32(input, input.len() - 8)?;
    let top = (footer & 0xffffff) as usize;
    let bottom = (footer >> 24) as usize;
    let size = input
        .len()
        .checked_add(le32(input, input.len() - 4)? as usize)
        .ok_or_else(fail)?;
    if !(8..=11).contains(&bottom) || top < bottom || top > input.len() || size > limit {
        return Err(fail());
    }
    let end = input.len() - top;
    let mut src = input.len() - bottom;
    let mut dst = size;
    let mut output = vec![0; size];
    output[..input.len()].copy_from_slice(input);
    while src > end {
        src -= 1;
        let flags = output[src];
        for bit in 0..8 {
            if src <= end {
                break;
            }
            if flags & (0x80 >> bit) == 0 {
                src -= 1;
                dst = dst.checked_sub(1).ok_or_else(fail)?;
                if dst < end {
                    return Err(fail());
                }
                output[dst] = output[src];
            } else {
                if src - end < 2 {
                    return Err(fail());
                }
                src -= 1;
                let hi = output[src] as usize;
                src -= 1;
                let distance = ((hi & 15) << 8 | output[src] as usize) + 3;
                let count = (hi >> 4) + 3;
                if count > dst - end || dst.checked_add(distance).is_none_or(|p| p > size) {
                    return Err(fail());
                }
                for _ in 0..count {
                    dst -= 1;
                    output[dst] = output[dst + distance];
                }
            }
        }
    }
    if dst != end {
        return Err(fail());
    }
    Ok(output)
}

const US_CODE_HASH: &[u8] = &[
    150, 174, 141, 101, 126, 152, 134, 213, 43, 137, 244, 188, 105, 254, 247, 178, 18, 62, 92, 52,
    105, 248, 65, 83, 123, 31, 183, 90, 66, 5, 138, 234,
];

#[cfg(test)]
mod tests {
    use super::*;
    fn word(b: &mut [u8], p: usize, v: u32) {
        b[p..p + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn ncch() -> Vec<u8> {
        let mut b = vec![0; 0xc00];
        b[0x100..0x104].copy_from_slice(b"NCCH");
        b[0x18f] = 4;
        b[0x118..0x120].copy_from_slice(&0x000400000008c300u64.to_le_bytes());
        word(&mut b, 0x1a0, 5);
        word(&mut b, 0x1a4, 1);
        b
    }
    #[test]
    fn cxi_and_ncsd_offsets() {
        let c = ncch();
        assert_eq!(
            Dump::from_bytes(c.clone()).unwrap().title_id,
            0x000400000008c300
        );
        let mut b = vec![0; 0x4000 + c.len()];
        b[0x100..0x104].copy_from_slice(b"NCSD");
        word(&mut b, 0x120, 32);
        word(&mut b, 0x124, 6);
        b[0x4000..].copy_from_slice(&c);
        let d = Dump::from_bytes(b).unwrap();
        assert_eq!(d.format, "ncsd");
        assert_eq!(d.ncch(), c);
    }
    #[test]
    fn cia_metadata_and_content_index() {
        let c = ncch();
        let tmd = 0x2040;
        let content = 0x2b80;
        let mut b = vec![0; content + c.len()];
        word(&mut b, 0, 0x2020);
        word(&mut b, 16, 0xb34);
        b[0x18..0x20].copy_from_slice(&(c.len() as u64).to_le_bytes());
        b[0x20] = 0x80;
        b[tmd..tmd + 4].copy_from_slice(&0x10004u32.to_be_bytes());
        b[tmd + 0x140 + 0x9e..tmd + 0x140 + 0xa0].copy_from_slice(&1u16.to_be_bytes());
        b[tmd + 0xb04 + 8..tmd + 0xb04 + 16].copy_from_slice(&(c.len() as u64).to_be_bytes());
        b[content..].copy_from_slice(&c);
        assert_eq!(Dump::from_bytes(b.clone()).unwrap().ncch(), c);
        b[0x20] = 0;
        assert!(Dump::from_bytes(b).is_err());
    }
    #[test]
    fn rejects_encrypted_and_truncated_sections() {
        let mut b = ncch();
        b[0x18f] = 0;
        assert!(Dump::from_bytes(b)
            .unwrap()
            .ensure_decrypted()
            .unwrap_err()
            .starts_with("encrypted dump:"));
        let mut b = ncch();
        word(&mut b, 0x1a4, 1000);
        assert!(Dump::from_bytes(b).is_err());
        assert!(Dump::from_bytes(vec![0; 1]).is_err());
    }
    #[test]
    fn blz_overlap_prefix_and_truncation() {
        let mut compressed = vec![1, 2, 3, 4, 0, 0, 0, 0xf0, 0, 0, 0, 0x18];
        compressed.extend_from_slice(&0x08000010u32.to_le_bytes());
        compressed.extend_from_slice(&8u32.to_le_bytes());
        let decoded = decompress_blz(&compressed, 28).unwrap();
        assert_eq!(&decoded[..4], &[1, 2, 3, 4]);
        assert!(decoded[4..].iter().all(|v| *v == 0));
        assert!(decompress_blz(&compressed, 27).is_err());
        for n in 0..compressed.len() {
            assert!(decompress_blz(&compressed[..n], 28).is_err());
        }
    }
}
