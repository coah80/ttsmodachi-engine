

#[derive(Clone, Copy)]
pub struct NumericTables<'a> {
    constants: &'a [u8],
    cosine: &'a [u8],
}

impl<'a> NumericTables<'a> {
    pub const CONSTANT_BYTES: usize = 0xa224c0 - 0xa21fb0;
    pub const COSINE_BYTES: usize = 256 * 16;

    pub fn from_us_image(image: &'a [u8]) -> Option<Self> {
        Self::from_bytes(
            image.get(0xa21fb0 - 0x100000..0xa224c0 - 0x100000)?,
            image.get(0x8cbfe0 - 0x100000..0x8ccfe0 - 0x100000)?,
        )
    }

    pub fn from_bytes(constants: &'a [u8], cosine: &'a [u8]) -> Option<Self> {
        (constants.len() == Self::CONSTANT_BYTES && cosine.len() == Self::COSINE_BYTES)
            .then_some(Self { constants, cosine })
    }

    pub(crate) fn float(&self, address: usize, index: usize) -> f32 {
        let (bytes, offset) = if address == 0x8cbfe0 {
            (self.cosine, index * 4)
        } else {
            (self.constants, address - 0xa21fb0 + index * 4)
        };
        f32::from_bits(u32::from_le_bytes(
            bytes[offset..offset + 4].try_into().unwrap(),
        ))
    }

    pub(crate) fn double(&self, address: usize, index: usize) -> f64 {
        let offset = address - 0xa21fb0 + index * 8;
        f64::from_bits(u64::from_le_bytes(
            self.constants[offset..offset + 8].try_into().unwrap(),
        ))
    }
}
