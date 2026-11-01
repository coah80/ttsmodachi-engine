

pub const ARENA_BLOCKS: usize = 256;
const SCRATCH: usize = 4096;

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

    pub fn free(&mut self, offset: u32) {
        if let Some(index) = self.block(offset) {
            self.sizes[index] = 0;
        }
    }

    fn start(&self, offset: u32) -> usize {
        (offset as usize).min(self.end())
    }

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

    pub fn copy_str(&mut self, destination: u32, source: u32) {
        let (buffer, count) = self.gather(source, self.len(source));
        self.put(destination, &buffer[..count]);
    }

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

    pub fn cat_from(&mut self, destination: u32, source: u32) {
        let (buffer, count) = self.gather(source, self.len(source));
        self.cat(destination, &buffer[..count]);
    }

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

    pub fn cat_n(&mut self, destination: u32, source: u32, count: usize) {
        let (buffer, count) = self.gather(source, count);
        self.cat(destination, &buffer[..count]);
    }

    pub fn eq(&self, left: u32, right: u32) -> bool {
        self.str(left) == self.str(right)
    }

    pub fn eq_bytes(&self, offset: u32, text: &[u8]) -> bool {
        self.str(offset) == text
    }

    pub fn eq_n(&self, left: u32, right: &[u8], count: usize) -> bool {
        let left = self.str(left);
        let left = &left[..left.len().min(count)];
        let right = &right[..right.len().min(count)];
        left == right
    }

    pub fn find_byte(&self, offset: u32, byte: u8) -> Option<u32> {
        self.str(offset)
            .iter()
            .position(|&value| value == byte)
            .map(|index| offset + index as u32)
    }

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

    pub fn copy(&mut self, destination: u32, source: u32, count: usize) {
        let (buffer, count) = self.gather(source, count);
        let start = self.start(destination);
        let count = count.min(self.end().saturating_sub(start));
        self.bytes[start..start + count].copy_from_slice(&buffer[..count]);
    }

    pub fn fill(&mut self, offset: u32, value: u8, count: usize) {
        let start = self.start(offset);
        let end = (start + count).min(self.end());
        self.bytes[start..end].fill(value);
    }
}
