//! Raw-arena replay support for the slice 4b climb.
//!
//! The functions here are direct translations of the `src/` C that operate on
//! the caller's byte arena instead of typed views, so a single fixture format
//! can drive the original ARM and the port from the same memory image. Addresses
//! are arena addresses; the arena base is supplied by the caller and is only
//! subtracted to reach the borrowed bytes. Every call that leaves the ported set
//! goes through [`Host`], which records it and supplies the scripted reply.

/// A caller-owned byte arena addressed by 32-bit machine addresses.
pub struct Arena<'a> {
    pub(crate) base: u32,
    pub(crate) bytes: &'a mut [u8],
}

impl<'a> Arena<'a> {
    /// Wraps a borrowed arena whose first byte lives at `base`.
    pub fn new(base: u32, bytes: &'a mut [u8]) -> Self {
        Self { base, bytes }
    }

    /// The address of the first borrowed byte.
    pub fn base(&self) -> u32 {
        self.base
    }

    fn offset(&self, address: u32) -> usize {
        address.wrapping_sub(self.base) as usize
    }

    /// Reads one byte at an arena address.
    pub fn u8(&self, address: u32) -> u8 {
        self.bytes[self.offset(address)]
    }

    /// Reads a little-endian halfword at an arena address.
    pub fn u16(&self, address: u32) -> u16 {
        u16::from_le_bytes(
            self.bytes[self.offset(address)..self.offset(address) + 2]
                .try_into()
                .unwrap(),
        )
    }

    /// Reads a little-endian word at an arena address.
    pub fn u32(&self, address: u32) -> u32 {
        u32::from_le_bytes(
            self.bytes[self.offset(address)..self.offset(address) + 4]
                .try_into()
                .unwrap(),
        )
    }

    /// Borrows `length` bytes at an arena address.
    pub fn slice(&self, address: u32, length: usize) -> &[u8] {
        let at = self.offset(address);
        &self.bytes[at..at + length]
    }

    /// Writes one byte at an arena address.
    pub fn set_u8(&mut self, address: u32, value: u8) {
        let at = self.offset(address);
        self.bytes[at] = value;
    }

    /// Writes a little-endian halfword at an arena address.
    pub fn set_u16(&mut self, address: u32, value: u16) {
        let at = self.offset(address);
        self.bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }

    /// Writes a little-endian word at an arena address.
    pub fn set_u32(&mut self, address: u32, value: u32) {
        let at = self.offset(address);
        self.bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// Copies `length` bytes between two arena addresses.
    pub fn copy(&mut self, destination: u32, source: u32, length: u32) {
        let from = self.offset(source);
        let to = self.offset(destination);
        self.bytes.copy_within(from..from + length as usize, to);
    }
}

/// One intercepted call outside the ported set.
pub trait Host {
    /// Records the call and returns the scripted `(r0, r1)` reply. The host may
    /// apply the callee's memory effects through `arena`.
    fn call(&mut self, arena: &mut Arena<'_>, id: u32, args: &[u32]) -> (u32, u32);
}

/// fn_2da7b0 / EU fn_2da96c / KR fn_2daa18.
///
/// Reads `count` parameter pairs out of the voice data. The two PDF-table
/// callees (`fn_2f6f94` for the untimed path, `fn_2f7420` for the indexed
/// path) are host calls here; the original enters the indexed path when
/// `index` is nonzero and picks the group's start and the record offset first.
pub fn read_voice_pair(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    a: u32,
    b: u32,
    c: u32,
    index: u32,
    out: u32,
    v: u32,
) -> u32 {
    let mut rc = 0;
    let count = arena.u16(v.wrapping_add(0x1ac)) as u32;
    let first = arena.u32(out.wrapping_add(4));
    let second = arena.u32(out.wrapping_add(8));
    let mut i = 0;
    if index != 0 {
        let group = arena.u32(v.wrapping_add(0x1e4).wrapping_add(index.wrapping_mul(4)));
        let offset = index
            .wrapping_sub(
                arena.u16(v.wrapping_add(0x1b8).wrapping_add(group.wrapping_mul(2))) as u32,
            )
            .wrapping_sub(1);
        while i < count {
            let args = [
                a,
                v.wrapping_add(0x1a4),
                v.wrapping_add(0x254),
                b,
                c,
                group,
                offset,
                i,
                first.wrapping_add(i.wrapping_mul(4)),
                second.wrapping_add(i.wrapping_mul(4)),
                16,
            ];
            rc = host.call(arena, 0x2f7420, &args).0;
            if rc >= 0x8000_0000 {
                return rc;
            }
            i += 1;
        }
    } else {
        while i < count {
            let args = [
                a,
                v.wrapping_add(0x1a4),
                b,
                c,
                i,
                first.wrapping_add(i.wrapping_mul(4)),
                second.wrapping_add(i.wrapping_mul(4)),
                16,
            ];
            rc = host.call(arena, 0x2f6f94, &args).0;
            if rc >= 0x8000_0000 {
                return rc;
            }
            i += 1;
        }
    }
    rc
}
