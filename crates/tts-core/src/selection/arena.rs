

pub struct Arena<'a> {
    pub(crate) base: u32,
    pub(crate) bytes: &'a mut [u8],
}

impl<'a> Arena<'a> {

    pub fn new(base: u32, bytes: &'a mut [u8]) -> Self {
        Self { base, bytes }
    }

    pub fn base(&self) -> u32 {
        self.base
    }

    fn offset(&self, address: u32) -> usize {
        address.wrapping_sub(self.base) as usize
    }

    pub fn u8(&self, address: u32) -> u8 {
        self.bytes[self.offset(address)]
    }

    pub fn u16(&self, address: u32) -> u16 {
        u16::from_le_bytes(
            self.bytes[self.offset(address)..self.offset(address) + 2]
                .try_into()
                .unwrap(),
        )
    }

    pub fn u32(&self, address: u32) -> u32 {
        u32::from_le_bytes(
            self.bytes[self.offset(address)..self.offset(address) + 4]
                .try_into()
                .unwrap(),
        )
    }

    pub fn slice(&self, address: u32, length: usize) -> &[u8] {
        let at = self.offset(address);
        &self.bytes[at..at + length]
    }

    pub fn set_u8(&mut self, address: u32, value: u8) {
        let at = self.offset(address);
        self.bytes[at] = value;
    }

    pub fn set_u16(&mut self, address: u32, value: u16) {
        let at = self.offset(address);
        self.bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn set_u32(&mut self, address: u32, value: u32) {
        let at = self.offset(address);
        self.bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    pub fn copy(&mut self, destination: u32, source: u32, length: u32) {
        let from = self.offset(source);
        let to = self.offset(destination);
        self.bytes.copy_within(from..from + length as usize, to);
    }
}

pub trait Host {

    fn call(&mut self, arena: &mut Arena<'_>, id: u32, args: &[u32]) -> (u32, u32);
}

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
