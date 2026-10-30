//! Caller-owned byte arena modelling the engine heap and C string helpers
//! used by the prosody rule interpreter (fn_196394 / fn_1963e0 / fn_196428 and
//! the fn_195xxx string family). Pointers are arena offsets; 0 is null.

pub const ARENA_BLOCKS: usize = 256;
const SCRATCH: usize = 4096;

/// Bump allocator over a caller-owned byte span. `alloc` mirrors fn_196394,
/// `realloc` mirrors fn_1963e0 (contents preserved), `free` mirrors fn_196428.
/// The offset 0 is reserved for null.
pub struct Arena<'a> {
    bytes: &'a mut [u8],
    starts: [u32; ARENA_BLOCKS],
    sizes: [u32; ARENA_BLOCKS],
    count: usize,
    bump: u32,
}

impl<'a> Arena<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Arena {
            bytes,
            starts: [0; ARENA_BLOCKS],
            sizes: [0; ARENA_BLOCKS],
            count: 0,
            bump: 0x100,
        }
    }

    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes
    }

    fn end(&self) -> usize {
        self.bytes.len()
    }

    fn block(&self, offset: u32) -> Option<usize> {
        self.starts[..self.count]
            .iter()
            .position(|&start| start == offset)
    }

    /// fn_196394: allocate `size` bytes, returning an offset or 0 on failure.
    pub fn alloc(&mut self, size: u32) -> u32 {
        if size == 0 || self.count >= ARENA_BLOCKS {
            return 0;
        }
        let aligned = (size + 15) & !15;
        let offset = self.bump;
        if offset as usize + aligned as usize > self.end() {
            return 0;
        }
        self.bytes[offset as usize..(offset + aligned) as usize].fill(0);
        self.starts[self.count] = offset;
        self.sizes[self.count] = size;
        self.count += 1;
        self.bump = offset + aligned;
        offset
    }

    /// fn_1963e0: grow `offset` to `size`, preserving the old contents.
    pub fn realloc(&mut self, offset: u32, size: u32) -> u32 {
        if offset == 0 {
            return self.alloc(size);
        }
        let Some(index) = self.block(offset) else {
            return 0;
        };
        let old = self.sizes[index];
        let fresh = self.alloc(size);
        if fresh == 0 {
            return 0;
        }
        let copy = old.min(size) as usize;
        self.bytes
            .copy_within(offset as usize..offset as usize + copy, fresh as usize);
        fresh
    }

    /// fn_196428: release `offset`. The bump allocator does not reclaim space.
    pub fn free(&mut self, offset: u32) {
        if let Some(index) = self.block(offset) {
            self.sizes[index] = 0;
        }
    }

    fn start(&self, offset: u32) -> usize {
        (offset as usize).min(self.end())
    }

    /// fn_195d90: length of the C string at `offset`.
    pub fn len(&self, offset: u32) -> usize {
        if offset == 0 {
            return 0;
        }
        let start = self.start(offset);
        self.bytes[start..]
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(self.end() - start)
    }

    /// Borrow the C string at `offset`.
    pub fn str(&self, offset: u32) -> &[u8] {
        let start = self.start(offset);
        &self.bytes[start..start + self.len(offset)]
    }

    pub fn slice(&self, offset: u32, length: usize) -> &[u8] {
        let start = self.start(offset);
        let end = (start + length).min(self.end());
        &self.bytes[start..end]
    }

    pub fn byte(&self, offset: u32) -> u8 {
        let index = self.start(offset);
        if index < self.end() {
            self.bytes[index]
        } else {
            0
        }
    }

    fn gather(&self, offset: u32, count: usize) -> ([u8; SCRATCH], usize) {
        let count = count.min(SCRATCH);
        let mut buffer = [0u8; SCRATCH];
        let end = (self.start(offset) + count).min(self.end());
        let start = self.start(offset).min(end);
        buffer[..end - start].copy_from_slice(&self.bytes[start..end]);
        (buffer, end - start)
    }

    /// fn_195cd0: copy the C string at `source` into `destination`.
    pub fn copy_str(&mut self, destination: u32, source: u32) {
        let (buffer, count) = self.gather(source, self.len(source));
        self.put(destination, &buffer[..count]);
    }

    /// fn_195cd0 with an explicit byte string.
    pub fn put(&mut self, offset: u32, text: &[u8]) {
        if offset == 0 {
            return;
        }
        let start = self.start(offset);
        if start >= self.end() {
            return;
        }
        let count = text.len().min(self.end() - start - 1);
        self.bytes[start..start + count].copy_from_slice(&text[..count]);
        self.bytes[start + count] = 0;
    }

    pub fn put_byte(&mut self, offset: u32, byte: u8) {
        let index = self.start(offset);
        if index < self.end() {
            self.bytes[index] = byte;
        }
    }

    /// fn_195c90: append the C string at `source` to `destination`.
    pub fn cat_from(&mut self, destination: u32, source: u32) {
        let (buffer, count) = self.gather(source, self.len(source));
        self.cat(destination, &buffer[..count]);
    }

    /// fn_195c90 with an explicit byte string.
    pub fn cat(&mut self, offset: u32, text: &[u8]) {
        if offset == 0 {
            return;
        }
        let start = self.start(offset) + self.len(offset);
        if start >= self.end() {
            return;
        }
        let count = text.len().min(self.end() - start - 1);
        self.bytes[start..start + count].copy_from_slice(&text[..count]);
        self.bytes[start + count] = 0;
    }

    /// fn_195dac: append at most `count` bytes of the C string at `source`.
    pub fn cat_n(&mut self, destination: u32, source: u32, count: usize) {
        let (buffer, count) = self.gather(source, count);
        self.cat(destination, &buffer[..count]);
    }

    /// fn_195d58 / fn_1769d0: compare the C strings at `left` and `right`.
    pub fn eq(&self, left: u32, right: u32) -> bool {
        self.str(left) == self.str(right)
    }

    pub fn eq_bytes(&self, offset: u32, text: &[u8]) -> bool {
        self.str(offset) == text
    }

    /// fn_195df8: compare at most `count` bytes of two C strings.
    pub fn eq_n(&self, left: u32, right: &[u8], count: usize) -> bool {
        let left = self.str(left);
        let left = &left[..left.len().min(count)];
        let right = &right[..right.len().min(count)];
        left == right
    }

    /// fn_195d30: find `byte` in the C string at `offset`.
    pub fn find_byte(&self, offset: u32, byte: u8) -> Option<u32> {
        self.str(offset)
            .iter()
            .position(|&value| value == byte)
            .map(|index| offset + index as u32)
    }

    /// fn_1960a8: locate `needle` in the C string at `offset`.
    pub fn find(&self, offset: u32, needle: &[u8]) -> Option<u32> {
        let haystack = self.str(offset);
        if needle.is_empty() || needle.len() > haystack.len() {
            return None;
        }
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|index| offset + index as u32)
    }

    /// fn_195ae8: copy `count` bytes from `source` to `destination`.
    pub fn copy(&mut self, destination: u32, source: u32, count: usize) {
        let (buffer, count) = self.gather(source, count);
        let start = self.start(destination);
        let count = count.min(self.end().saturating_sub(start));
        self.bytes[start..start + count].copy_from_slice(&buffer[..count]);
    }

    /// fn_195bec: fill `count` bytes at `offset` with `value`.
    pub fn fill(&mut self, offset: u32, value: u8, count: usize) {
        let start = self.start(offset);
        let end = (start + count).min(self.end());
        self.bytes[start..end].fill(value);
    }
}
