#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ByteSpan {
    pub start: u32,
    pub length: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TokenId(pub u16);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhonemeId(pub u16);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarkerId(pub u32);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Token {
    pub id: TokenId,
    pub bytes: ByteSpan,
    pub source: ByteSpan,
    pub flags: u32,
    pub expansion: Option<TokenId>,
    pub marker: Option<MarkerId>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Phoneme {
    pub id: PhonemeId,
    pub token: TokenId,
    pub alternative: u16,
    pub stress: u8,
    pub syllable_boundary: u8,
    pub pos: u16,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextMarker {
    pub id: MarkerId,
    pub kind: u32,
    pub original_offset: u32,
    pub current_offset: u32,
    pub payload: [u32; 6],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub consumed: u32,
    pub produced: u32,
    pub eof: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Engine(u32),
    Capacity { required: usize, available: usize },
    InvalidResource,
}

#[derive(Clone, Copy)]
pub struct TokenView<'a> {
    pub bytes: &'a [u8],
    pub tokens: &'a [Token],
    pub source_map: &'a [u32],
    pub markers: &'a [TextMarker],
}

pub struct TokenBuffer<'a> {
    pub bytes: &'a mut [u8],
    pub tokens: &'a mut [Token],
    pub source_map: &'a mut [u32],
    pub markers: &'a mut [TextMarker],
    pub byte_count: usize,
    pub token_count: usize,
    pub marker_count: usize,
}

#[derive(Clone, Copy)]
pub struct PronunciationView<'a> {
    pub tokens: TokenView<'a>,
    pub phonemes: &'a [Phoneme],
}

pub struct PronunciationBuffer<'a> {
    pub phonemes: &'a mut [Phoneme],
    pub count: usize,
}
