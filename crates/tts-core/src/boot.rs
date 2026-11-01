

pub struct Memory<'a> {
    pub base: u32,
    pub bytes: &'a mut [u8],
}

impl Memory<'_> {
    pub(crate) fn offset(&self, address: u32) -> usize {
        address.checked_sub(self.base).expect("address below arena") as usize
    }

    pub(crate) fn word(&self, address: u32) -> u32 {
        let at = self.offset(address);
        u32::from_le_bytes(self.bytes[at..at + 4].try_into().unwrap())
    }

    pub(crate) fn put_word(&mut self, address: u32, value: u32) {
        let at = self.offset(address);
        self.bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn byte(&self, address: u32) -> u8 {
        self.bytes[self.offset(address)]
    }

    pub(crate) fn put_half(&mut self, address: u32, value: u16) {
        let at = self.offset(address);
        self.bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn open_archive(&mut self, blob: u32, archive: u32) -> u32 {

        self.put_word(archive, blob);
        let names = blob.wrapping_add(self.word(blob.wrapping_add(16)));
        self.put_word(archive.wrapping_add(4), names);
        let data = blob.wrapping_add(self.word(blob.wrapping_add(24)));
        self.put_word(archive.wrapping_add(8), data);
        let count = self.word(names.wrapping_add(8));
        self.put_word(
            archive.wrapping_add(16),
            names.wrapping_add(count.wrapping_mul(12)),
        );
        self.put_word(archive.wrapping_add(12), count);
        let size = self.word(blob.wrapping_add(20));
        self.put_word(archive.wrapping_add(20), size);
        self.put_word(archive.wrapping_add(24), 0);
        1
    }

    pub fn open_node(&mut self, node: u32, blob: u32, name: u32) -> u32 {
        if self.open_archive(blob, node.wrapping_add(8)) == 0 {
            return 0;
        }
        let mut i = 0u32;
        while i < 63 {
            let c = self.byte(name.wrapping_add(i)) as i8;
            if c == 0 {
                break;
            }
            self.put_half(node.wrapping_add(36 + i * 2), c as i16 as u16);
            i += 1;
        }
        self.put_half(node.wrapping_add(36 + i * 2), 0);
        1
    }

    pub fn append_node(&mut self, count: u32, head: u32, node: u32) -> u32 {
        let previous = self.word(head.wrapping_add(4));
        self.put_word(node, head);
        self.put_word(node.wrapping_add(4), previous);
        self.put_word(head.wrapping_add(4), node);
        self.put_word(previous, node);
        let n = self.word(count).wrapping_add(1);
        self.put_word(count, n);
        node
    }

    pub fn append_archive(&mut self, list: u32, node: u32) -> u32 {
        self.append_node(list.wrapping_add(4), list.wrapping_add(8), node)
    }

    fn mount(&mut self, loader: u32, manager: u32, request: u32, node_offset: u32) {
        let slots = self.word(manager.wrapping_add(0x14));
        let index = self.word(loader.wrapping_add(request));
        let slot = slots
            .wrapping_add(index.wrapping_mul(0xe8))
            .wrapping_add(0xc0);
        let blob = self.word(slot);
        let node = loader.wrapping_add(node_offset);
        self.open_archive(blob, node.wrapping_add(8));

        self.put_half(node.wrapping_add(36), 0);

        self.append_archive(loader.wrapping_add(8), node);
    }

    pub fn mount_components(&mut self, loader: u32, manager: u32) {
        self.mount(loader, manager, 0x348, 0x154);
    }

    pub fn mount_dictionary(&mut self, loader: u32, manager: u32) {
        self.mount(loader, manager, 0x34c, 0x1f8);
    }

    pub fn mount_rules(&mut self, loader: u32, manager: u32) {
        self.mount(loader, manager, 0x350, 0x29c);
    }
}

pub trait Calls {
    fn format_address_m(&mut self,_m:&Memory<'_>,_format:&[u8])->Option<u32> {None}
    fn real_log_args(&mut self)->bool {false}
    fn log_variadic(&mut self,_m:&mut Memory<'_>,_entry:u32,_a:[u32;4],_format:&[u8],_varargs:&[u32])->bool {false}
    fn call(&mut self, memory: &mut Memory<'_>, address: u32, args: &[u32]) -> u32;
    fn call_wide(&mut self, memory: &mut Memory<'_>, address: u32, args: &[u32]) -> u64;
}

pub struct Globals {
    pub manager: u32,
    pub callbacks: u32,
    pub rules: u32,
    pub components: u32,
    pub dictionary: u32,
    pub context: u32,
    pub mode: u32,
    pub semaphore: u32,
    pub service: u32,
    pub command: u32,
}

impl Memory<'_> {
    pub(crate) fn put_byte(&mut self, address: u32, value: u8) {
        let at = self.offset(address);
        self.bytes[at] = value;
    }

    pub(crate) fn half(&self, address: u32) -> u16 {
        let at = self.offset(address);
        u16::from_le_bytes(self.bytes[at..at + 2].try_into().unwrap())
    }

    pub fn fn_132124(&mut self, h: &mut impl Calls, a: [u32; 5]) -> u32 {
        h.call(self, 0x163a98, &[a[4], a[0], a[1], a[2], a[3]])
    }

    pub fn fn_132c90(&mut self, h: &mut impl Calls, obj: u32, ptr: u32) {
        let inner = self.word(obj + 4);
        let lock = inner.wrapping_add(0x58);
        h.call(self, 0x1840fc, &[lock]);
        h.call(self, 0x19b78c, &[inner, ptr]);
        h.call(self, 0x18423c, &[lock]);
    }

    fn request(
        &mut self,
        h: &mut impl Calls,
        g: &Globals,
        obj: u32,
        index: u32,
        table: u32,
        at: u32,
    ) {
        let callback = self.word(g.callbacks + 4);
        let engine = self.word(g.manager);
        let name = self.word(table.wrapping_add(index.wrapping_mul(4)));
        let result = h.call(self, 0x163d34, &[engine, name, obj, callback, 4]);
        self.put_word(obj + at, result);
    }

    pub fn fn_1343b8(&mut self, h: &mut impl Calls, g: &Globals, obj: u32, index: u32) {
        self.request(h, g, obj, index, g.rules, 0x350);
    }

    pub fn fn_1343fc(&mut self, h: &mut impl Calls, g: &Globals, obj: u32, index: u32) {
        self.request(h, g, obj, index, g.components, 0x348);
    }

    pub fn fn_134440(&mut self, h: &mut impl Calls, g: &Globals, obj: u32, index: u32) {
        self.request(h, g, obj, index, g.dictionary, 0x34c);
    }

    pub fn fn_136ccc(&mut self, h: &mut impl Calls, a: [u32; 4]) {
        let bit = h.call(self, 0x1674b0, &a) >> 31;
        if bit != 0 {
            h.call(self, 0x17177c, &[bit]);
        }
    }

    pub fn fn_136ce8(&mut self, h: &mut impl Calls, scratch: u32) -> u32 {
        let saved = self.word(scratch);
        self.put_word(scratch, 0);
        h.call(self, 0x1674fc, &[scratch]);
        let result = self.word(scratch);
        self.put_word(scratch, saved);
        result
    }

    pub fn fn_13792c(&mut self, h: &mut impl Calls, g: &Globals, a: u32, b: u32) {
        self.put_byte(g.context + 1, 0);
        self.put_half(g.context + 2, 0xd798);
        self.put_byte(g.context, 0);
        self.put_word(g.context + 8, b);
        self.put_word(g.context + 4, a);
        h.call(self, 0x13795c, &[g.context]);
    }

    pub fn fn_137afc(&mut self, h: &mut impl Calls, out: u32, input: u32) {
        let a = [
            out,
            self.word(input),
            self.byte(input + 4) as i8 as u32,
            self.byte(input + 5) as i8 as u32,
            self.byte(input + 7) as i8 as u32,
            self.byte(input + 8) as i8 as u32,
            self.byte(input + 9) as i8 as u32,
            self.half(input + 10) as i16 as u32,
        ];
        h.call(self, 0x1abd98, &a);
    }

    pub fn fn_13e4d0(&mut self, p: u32) -> u32 {
        for at in [0, 4, 8, 12, 20, 16, 432, 428] {
            self.put_word(p + at, 0);
        }
        for at in 24..428 {
            self.put_byte(p + at, 0);
        }
        self.put_byte(p + 436, 0);
        self.put_word(p + 452, 0);
        self.put_word(p + 460, u32::MAX);
        self.put_word(p + 456, 0);
        self.put_word(p + 468, p + 468);
        self.put_word(p + 464, 0);
        self.put_word(p + 472, p + 468);
        self.put_word(p + 480, p + 480);
        self.put_word(p + 476, 0);
        self.put_word(p + 484, p + 480);
        self.put_word(p + 496, 0);
        for at in 500..504 {
            self.put_byte(p + at, 0);
        }
        for at in [504, 508, 512, 516] {
            self.put_word(p + at, 0);
        }
        p
    }

    pub fn fn_160aa8(&mut self, id: u32, out: u32, blob: u32) -> u32 {
        if id <= 1 {
            self.put_word(out, blob);
            0
        } else {
            0x89502001
        }
    }

    pub fn fn_163d34(&mut self, h: &mut impl Calls, g: &Globals, a: [u32; 5]) -> u32 {
        let mode = self.byte(g.mode) as u32;
        h.call(self, 0x183524, &[a[0], a[1], a[2], a[3], a[4], mode])
    }

    pub fn fn_163f20(&mut self, h: &mut impl Calls, a: [u32; 4]) -> u32 {
        let inner = self.word(a[0] + 4);
        let lock = inner.wrapping_add(0x58);
        h.call(self, 0x1840fc, &[lock]);
        let result = h.call(self, 0x184f20, &[inner, a[1], a[2], a[3], 0, 0]);
        h.call(self, 0x18423c, &[lock]);
        result
    }

    pub fn fn_16537c(&mut self, h: &mut impl Calls, a: [u32; 6]) -> u32 {
        if a[1] == 0 || a[4] == 0 {
            return 0xe0e046bc;
        }
        h.call(self, 0x19b1d0, &[1]);
        let method = self.word(self.word(a[1]));
        h.call(self, method, &[a[1], a[0], a[2], a[3], a[4], a[5]])
    }

    pub fn fn_1653dc(&mut self, h: &mut impl Calls, a: u32, b: u32) -> u32 {
        if a == 0 || b == 0 {
            return 0xe0e046bc;
        }
        let method = self.word(self.word(b) + 12);
        h.call(self, method, &[b, a])
    }

    pub fn fn_165c84(&mut self, p: u32) {
        self.put_word(p, 1);
    }

    pub fn fn_165b88(&mut self, h: &mut impl Calls, p: u32, v: u32) {
        h.call(self, 0x165c84, &[p + 4]);
        self.put_word(p, if v != 0 { u32::MAX - 1 } else { u32::MAX });
    }

    pub fn fn_165bbc(&mut self, h: &mut impl Calls, g: &Globals, p: u32) {
        loop {
            match self.word(p) as i32 {
                -2 => {
                    let s = self.word(g.semaphore);
                    h.call(self, 0x172d18, &[s, p, 1, 0, 0, 0]);
                    return;
                }
                0 => {
                    self.put_word(p, u32::MAX);
                    return;
                }
                1 => return,
                _ => {
                    let s = self.word(g.semaphore);
                    h.call(self, 0x172d18, &[s, p, 1, 0, 0, 0]);
                }
            }
        }
    }

    pub fn fn_165d00(&mut self, h: &mut impl Calls, g: &Globals, p: u32) {
        let value = self.word(p).wrapping_neg();
        self.put_word(p, value);
        if (value as i32) > 1 {
            let s = self.word(g.semaphore);
            h.call(self, 0x172d18, &[s, p, 0, 1, 0, 0]);
        }
    }

    pub fn fn_165c9c(&mut self, h: &mut impl Calls, p: u32) {
        match self.word(p) {
            1 => {
                h.call(self, 0x183edc, &[p + 4]);
                self.put_word(p, u32::MAX - 1);
                h.call(self, 0x165d00, &[p + 4]);
            }
            0 => self.put_word(p, u32::MAX),
            _ => {}
        }
    }

    pub fn fn_166054(&mut self, h: &mut impl Calls, lo: u32, hi: u32) {
        let ns = (((hi as u64) << 32) | lo as u64) as i64;
        if ns >= 0x3477 {
            h.call(self, 0x11bcac, &[lo, hi]);
        } else {
            let high = ((lo as u64 * 0x44a2fa85u64) >> 32) as u32;
            let q = high.wrapping_add(hi.wrapping_mul(0x44a2fa85));
            h.call(self, 0x11bc00, &[q]);
        }
    }

    pub fn fn_1670e4(&mut self, h: &mut impl Calls, a: [u32; 8], scratch: u32) -> u32 {
        let saved = [self.word(scratch), self.word(scratch + 4)];
        h.call(
            self,
            0x1abd98,
            &[scratch, a[1], a[2], a[3], a[4], a[5], a[6], a[7]],
        );
        let value = [self.word(scratch), self.word(scratch + 4)];
        self.put_word(scratch, saved[0]);
        self.put_word(scratch + 4, saved[1]);
        self.put_word(a[0], value[0]);
        self.put_word(a[0] + 4, value[1]);
        a[0]
    }

    pub fn fn_1674b0(&mut self, h: &mut impl Calls, g: &Globals, a: [u32; 4]) -> u32 {
        let q = g.command;
        self.put_word(q, 0xb00c2);
        self.put_word(q + 4, a[1]);
        self.put_word(q + 8, a[2]);
        self.put_word(q + 12, a[3]);
        self.put_word(q + 20, a[0]);
        let service = self.word(g.service);
        self.put_word(q + 16, 12 | a[1].wrapping_shl(5));
        let rc = h.call(self, 0xffff0032, &[service]);
        if rc >> 31 == 0 {
            self.word(q + 4)
        } else {
            rc
        }
    }

    pub fn fn_1674fc(&mut self, h: &mut impl Calls, g: &Globals, out: u32) -> u32 {
        self.put_word(g.command, 0xc0000);
        let service = self.word(g.service);
        let rc = h.call(self, 0xffff0032, &[service]);
        if rc >> 31 != 0 {
            return rc;
        }
        let value = self.word(g.command + 8);
        self.put_word(out, value);
        self.word(g.command + 4)
    }

    pub fn fn_166f00(&mut self, h: &mut impl Calls, g: &Globals, ph: u32, src: u32) -> u32 {
        self.put_word(g.command, 0x10800);

        for i in 0..128 {
            let b = self.byte(src + i);
            self.put_byte(g.command + 4 + i, b);
        }
        let service = self.word(ph);
        let rc = h.call(self, 0xffff0032, &[service]);
        if rc >> 31 == 0 {
            self.word(g.command + 4)
        } else {
            rc
        }
    }

    pub fn fn_140808(
        &mut self,
        h: &mut impl Calls,
        p: u32,
        _x: u32,
        lo: u32,
        hi: u32,
        shift: u32,
    ) -> u64 {
        let a = (self.word(p) as u64) | ((self.word(p + 4) as u64) << 32);
        let m = (lo as u64) | ((hi as u64) << 32);
        let signed = a as i64 as i128;
        let high = (signed.wrapping_mul(m as i128) >> 64) as u64;

        let shifted = h.call_wide(self, 0x161178, &[high as u32, (high >> 32) as u32, shift]);
        shifted.wrapping_add(a >> 63)
    }

    pub fn fn_163de0(&mut self, h: &mut impl Calls, mgr: u32, slot: u32) {
        h.call(self, 0x1840fc, &[mgr + 0x2c]);
        h.call(self, 0x132ab8, &[mgr, slot]);
        if self.byte(slot + 0xe6) != 0 {
            let data = self.word(slot + 0xc0);
            self.put_word(slot + 0xc4, data);
            let size = self.word(slot + 0xcc);
            self.put_word(slot + 0xd0, size);
            h.call(self, 0x1840fc, &[mgr + 0x20]);
            self.put_byte(slot + 0xe4, 3);
            h.call(self, 0x18423c, &[mgr + 0x20]);
            let out = self.word(slot + 0xc8);
            if out != 0 {
                let raw = self.word(slot + 0xc4);
                let alloc = self.word(slot + 0xdc);
                let size = h.call(self, 0x19af64, &[raw]);
                let raw = self.word(slot + 0xc4);
                h.call(self, 0x19ae08, &[raw, out]);
                let raw = self.word(slot + 0xc4);
                h.call(self, 0x132c90, &[alloc, raw]);
                self.put_word(slot + 0xc4, 0);
                let out = self.word(slot + 0xc8);
                self.put_word(slot + 0xc0, out);
                self.put_word(slot + 0xcc, size);
            } else {
                let mode = self.byte(slot + 0xe5) as u32;
                let alloc = self.word(slot + 0xd8);
                let alloc2 = self.word(slot + 0xdc);
                let arg = self.word(slot + 0xe0);
                let raw = self.word(slot + 0xc4);
                let size = h.call(self, 0x19af64, &[raw]);
                let buf = h.call(self, 0x163f20, &[alloc, size, arg, mode]);
                self.put_word(slot + 0xc0, buf);
                let raw = self.word(slot + 0xc4);
                h.call(self, 0x19ae08, &[raw, buf]);
                let raw = self.word(slot + 0xc4);
                h.call(self, 0x132c90, &[alloc2, raw]);
                self.put_word(slot + 0xcc, size);
                self.put_word(slot + 0xc4, 0);
            }
        }
        h.call(self, 0x1840fc, &[mgr + 0x20]);
        self.put_byte(slot + 0xe4, 4);
        h.call(self, 0x18423c, &[mgr + 0x20]);
        h.call(self, 0x18423c, &[mgr + 0x2c]);
    }
}

pub trait Kernel: Calls {
    fn svc(&mut self, memory: &mut Memory<'_>, number: u32, registers: [u32; 4]) -> [u32; 4];
}

pub fn fn_161178(lo: u32, hi: u32, shift: u32) -> u64 {
    fn lsl(x: u32, n: u32) -> u32 { if n & 255 >= 32 { 0 } else { x << (n & 255) } }
    fn lsr(x: u32, n: u32) -> u32 { if n & 255 >= 32 { 0 } else { x >> (n & 255) } }
    fn asr(x: u32, n: u32) -> u32 { ((x as i32) >> (n & 255).min(31)) as u32 }

    let (low, high) = if (shift as i32) >= 32 {
        (asr(hi, shift.wrapping_sub(32)), ((hi as i32) >> 31) as u32)
    } else {
        (lsr(lo, shift) | lsl(hi, 32u32.wrapping_sub(shift)), asr(hi, shift))
    };
    low as u64 | ((high as u64) << 32)
}

impl Memory<'_> {

    pub fn fn_1003dc(&self, mut p: u32) -> u32 {
        let mut count = 0u32;
        while self.half(p) != 0 {
            count = count.wrapping_add(1);
            p = p.wrapping_add(2);
        }
        count
    }

    pub fn fn_13095c(&mut self, h: &mut impl Kernel, out: u32, value: u32) -> u32 {
        let r = h.svc(self, 0x2d, [out, value, 0, 0]);
        self.put_word(out, r[1]);
        r[0]
    }

    pub fn fn_14d6a0(&mut self, h: &mut impl Kernel, out: u32, value: u32) -> u32 {
        let r = h.svc(self, 0x35, [out, value, 0, 0]);
        self.put_word(out, r[1]);
        r[0]
    }

    pub fn fn_137b30(&mut self, acc: u32, input: u32) {
        let lo = self.word(input);
        let hi = self.word(input + 4);
        let p0 = (lo as u64) * 0xd7b634db;
        let p1 = (hi as i32 as i64).wrapping_mul(0xd7b634db);
        let (middle, carry1) = (p1 as u32).overflowing_add((p0 >> 32) as u32);
        let upper1 = ((p1 >> 32) as u32).wrapping_add(carry1 as u32);
        let p2 = (lo as u64) * 0x431bde82;
        let (_, carry2) = (p2 as u32).overflowing_add(middle);
        let upper2 = ((p2 >> 32) as u32).wrapping_add(carry2 as u32);
        let p3 = (hi as i32 as i64).wrapping_mul(0x431bde82);
        let combined = (upper2 as i32 as i64)
            .wrapping_add(upper1 as i32 as i64).wrapping_add(p3);
        let delta = (combined >> 18).wrapping_add((hi >> 31) as i64) as u64;
        let old = self.word(acc) as u64 | ((self.word(acc + 4) as u64) << 32);
        let value = old.wrapping_add(delta);
        self.put_word(acc, value as u32);
        self.put_word(acc + 4, (value >> 32) as u32);
    }

    pub fn fn_16b540(&mut self, h: &mut impl Calls, loader: u32, name: u32, size: u32) -> u32 {
        h.call(self, 0x16b554, &[loader.wrapping_add(8), 0, name, size])
    }

    pub fn fn_14d6dc(&mut self, h: &mut impl Calls, logger: u32, scratch: u32) -> u32 {
        if logger == 0 { return 0; }
        for i in 0..5 {
            let slot = logger + 20 + 20 * i;
            let sub = self.word(slot);
            if sub != 0 {
                let broker = self.word(self.word(logger + 8) + 12);
                let rc = h.call(self, 0x178530, &[broker, sub]);
                if rc >= 0x80000000 { return rc; }
                self.put_word(slot, 0);
            }
        }
        let saved = self.word(scratch);
        let rc = h.call(self, 0x19611c, &[self.word(logger), self.word(logger + 4), scratch]);
        let ctx = self.word(scratch);
        self.put_word(scratch, saved);
        if rc >= 0x80000000 { return 0x84b02008; }
        h.call(self, 0x196428, &[self.word(ctx + 4), logger]);
        rc
    }

    pub fn fn_14dc30(&mut self, h: &mut impl Calls, log: u32) -> u32 {
        if log == 0 { return 0; }
        for i in 0..5 {
            let slot = log + 8 + 8 * i;
            let sub = self.word(slot + 4);
            if sub != 0 {
                let rel = self.word(sub + 20);
                if rel != 0 && self.word(slot) != 0 {
                    let rc = h.call(self, rel, &[self.word(slot)]);
                    if rc >= 0x80000000 { return rc; }
                }
            }
        }
        h.call(self, 0x196428, &[self.word(log), log]);
        0
    }

    pub fn fn_14dc94(&mut self, h: &mut impl Calls, logger: u32, out: u32) -> u32 {
        if logger == 0 { self.put_word(out, 0); return 0; }
        if out == 0 { return 0x84b02006; }
        self.put_word(out, 0);
        let ctx = self.word(logger + 8);
        let log = h.call(self, 0x1963b8, &[self.word(ctx + 4), 1, 68]);
        if log == 0 { return 0x84b0200a; }
        h.call(self, 0x195bec, &[log, 0, 68]);
        self.put_word(log, self.word(self.word(logger + 8) + 4));
        for i in 0..5 {
            let source = logger + 12 + 20 * i;
            let slot = log + 8 + 8 * i;
            self.put_word(slot + 4, self.word(source + 8));
            let sub = self.word(source + 8);
            if sub != 0 {
                let rc = h.call(self, self.word(sub + 16), &[self.word(source + 12), self.word(source + 16), slot]);
                if rc >= 0x80000000 {
                    self.fn_14dc30(h, log);
                    return rc;
                }
            }
        }
        self.put_byte(log + 48, 1);
        for i in 0..5 {
            let slot = log + 8 + 8 * i;
            let sub = self.word(slot + 4);
            if sub != 0 && self.word(sub + 40) != 0 {
                let r = h.call(self, self.word(sub + 40), &[self.word(slot)]);
                if (self.byte(log + 48) as u32) < r { self.put_byte(log + 48, r as u8); }
            }
        }
        self.put_byte(log + 64, 0);
        for i in 0..5 {
            let sub = self.word(log + 12 + 8 * i);
            if sub != 0 && self.word(sub + 48) != 0 && self.word(sub + 52) != 0 {
                self.put_byte(log + 64, 1);
                break;
            }
        }
        self.put_word(out, log);
        0
    }
}

pub struct RequestGlobals {
    pub current: u32,
    pub descriptor: u32,
    pub lock: u32,
    pub retry_notice: u32,
}

impl Memory<'_> {

    pub fn fn_13795c(&mut self, h: &mut impl Kernel, g: &RequestGlobals, p: u32, scratch: u32) {
        let saved = [self.word(scratch), self.word(scratch + 4)];
        loop {
            h.call(self, 0x1840fc, &[g.lock]);
            let rc = if self.word(g.current) == 0 {
                let rc = h.call(self, 0x13095c, &[g.current, self.word(g.descriptor + 8)]);
                if (rc as i32) < 0 { rc } else { 0 }
            } else { 0 };
            if (rc as i32) < 0 {
                if rc == 0xd0401834 && self.byte(p) == 2 {
                    h.call(self, 0x166054, &[g.retry_notice, 0]);
                    h.call(self, 0x18423c, &[g.lock]);
                    continue;
                }
            } else {
                h.call(self, 0x14d6a0, &[p + 12, self.word(g.descriptor)]);
                self.put_word(scratch, self.word(g.current));
                h.call(self, 0x166f00, &[scratch, p]);
                if self.word(g.current) != 0 {
                    h.svc(self, 0x23, [0; 4]);
                    self.put_word(g.current, self.word(g.descriptor + 4));
                }
            }
            h.call(self, 0x18423c, &[g.lock]);
            if self.byte(p) != 2 && self.byte(p) != 5 {
                self.put_word(scratch, 0);
                self.put_word(scratch + 4, 0);
                h.call(self, 0x165b88, &[scratch, 1]);
                h.call(self, 0x165bbc, &[scratch]);
            }
            self.put_word(scratch, saved[0]);
            self.put_word(scratch + 4, saved[1]);
            return;
        }
    }
}

impl Memory<'_> {

    pub fn boot_request_prefix(
        &mut self, h: &mut impl Calls, g: &Globals,
        engine: u32, mode: u8, engine_delta: u32,
    ) {
        self.put_word(engine + 0x61ac + engine_delta, 0);
        self.put_word(engine + 0x61b0 + engine_delta, 0);
        self.put_byte(engine + 0x17a + engine_delta, mode);
        let loader = self.word(engine + 0x61bc + engine_delta);
        self.fn_1343fc(h, g, loader, mode as u32);
    }
}

impl Memory<'_> {

    pub fn fn_122364(&mut self, h: &mut impl Calls, p: u32, scratch: u32) {
        let saved: [u32; 32] = core::array::from_fn(|i| self.word(scratch + i as u32 * 4));
        self.update_calendar_rate(h, p, scratch);
        for (i, v) in saved.into_iter().enumerate() {
            self.put_word(scratch + i as u32 * 4, v);
        }
    }

    fn update_calendar_rate(&mut self, h: &mut impl Calls, p: u32, s: u32) {
        h.call(self, 0x19ba10, &[s]);
        self.put_word(s + 8, self.word(s));
        self.put_word(s + 12, self.word(s + 4));
        h.call(self, 0x1a1d18, &[s + 16, s + 8]);
        self.put_word(s + 32, self.word(s + 16));
        self.put_word(s + 36, self.word(s + 20) & 0xffffff);
        self.put_word(s + 40, 0);
        h.call(self, 0x137afc, &[s + 16, s + 32]);
        let a = self.word(s + 16) as u64 | ((self.word(s + 20) as u64) << 32);
        h.call(self, 0x1abd98, &[
            s + 48, self.half(p + 16) as i16 as i32 as u32,
            self.byte(p + 18) as i8 as i32 as u32,
            self.byte(p + 19) as i8 as i32 as u32,
        ]);
        let mut b = self.word(s + 48) as u64 | ((self.word(s + 52) as u64) << 32);
        let initial_b = b;
        let r = h.call(self, 0x136ce8, &[]);
        if (a as i64) < (b as i64) || self.word(p + 8) > r
            || self.half(p + 4) >= 300 || (a == b && self.half(p + 6) >= 10)
        {
            if a != b { self.put_half(p + 6, 0); }
            self.put_word(p + 8, r);
            let total = self.calendar_sum(h, s + 64, a);
            self.put_word(p + 12, total);
        } else {
            if r.wrapping_sub(self.word(p + 8)) < 100 { return; }
            while (a as i64) >= (b as i64) {
                let total = self.calendar_sum(h, s + 64, b);

                let first = b == initial_b;
                let base = if first { self.word(p + 12) & 0xffff } else { 0 };
                let phase = if first { self.half(p + 6) as u32 } else { 0 };
                let amount = total.saturating_sub(base);
                let q = amount / 100;
                let mut rem = amount % 100;
                let mut increment = q as u16 as u32;
                if phase + q >= 10 {
                    increment = 10u32.saturating_sub(phase) as u16 as u32;
                    rem = 0;
                }
                let rate = self.half(p + 4) as u32;
                if rate + increment >= 300 {
                    increment = 300u32.saturating_sub(rate) as u16 as u32;
                    rem = 0;
                }
                if a == b {
                    self.put_half(p + 6, phase.wrapping_add(increment) as u16);
                    self.put_word(p + 8, r.saturating_sub(rem));
                    self.put_word(p + 12, total.saturating_sub(rem));
                }
                self.put_half(p + 4, rate.wrapping_add(increment) as u16);
                b = b.wrapping_add(86_400_000);
            }
        }
        self.put_half(p + 16, self.half(s + 32));
        self.put_byte(p + 18, self.byte(s + 36));
        self.put_byte(p + 19, self.byte(s + 37));
        if self.half(p + 4) > 300 { self.put_half(p + 4, 300); }
    }

    fn calendar_sum(&mut self, h: &mut impl Calls, scratch: u32, value: u64) -> u32 {
        h.call(self, 0x136ccc, &[scratch, 24, value as u32, (value >> 32) as u32]);
        let mut sum = 0u32;
        for i in 0..24 { sum = sum.wrapping_add(self.half(scratch + i * 2) as u32); }
        sum
    }
}

pub struct TickGlobals { pub baseline: u32, pub sequence: u32, pub banks: u32 }
impl Memory<'_> {

    pub fn fn_1391a4(&mut self, h: &mut impl Kernel, g: &TickGlobals, s: u32) -> u64 {
        let saved:[u32;8]=core::array::from_fn(|i|self.word(s+i as u32*4));
        let pa=h.call(self,0x1670e4,&[s,2000,1,1,0,0,0,0]);
        let value=self.word(pa) as u64|((self.word(pa+4) as u64)<<32);
        let baseline=self.word(g.baseline) as u64|((self.word(g.baseline+4) as u64)<<32);
        let product=value.wrapping_sub(baseline).wrapping_mul(1_000_000);
        self.put_word(s+16,product as u32);self.put_word(s+20,(product>>32) as u32);
        let s1=self.fn_140808(h,s+16,(product>>32) as u32,0xd7b634db,0x431bde82,18);
        let pa=h.call(self,0x1670e4,&[s,2100,1,1,0,0,0,0]);
        let pb=h.call(self,0x1670e4,&[s+8,2000,1,1,0,0,0,0]);
        let av=self.word(pa) as u64|((self.word(pa+4) as u64)<<32);
        let bv=self.word(pb) as u64|((self.word(pb+4) as u64)<<32);
        let product=av.wrapping_sub(bv).wrapping_mul(1_000_000);
        self.put_word(s+16,product as u32);self.put_word(s+20,(product>>32) as u32);
        let s2=self.fn_140808(h,s+16,(product>>32) as u32,0xd7b634db,0x431bde82,18);

        let tick=h.svc(self,0x28,[0;4]);
        let bank=g.banks+((self.word(g.sequence)&1)<<5);
        let b0=self.word(bank);let b1=self.word(bank+4);
        let b2=self.word(bank+8);let b3=self.word(bank+12);
        let frequency=self.word(bank+16) as i32 as i64;
        let correction=self.word(bank+24) as u64|((self.word(bank+28) as u64)<<32);
        let time=tick[0] as u64|((tick[1] as u64)<<32);
        let delta=time.wrapping_sub(b2 as u64|((b3 as u64)<<32));
        let divide=|n:u64,d:u64| -> (u64,u64) {
            crate::boot_part_b::fn_1992f8(n as i64,d as i64,|n,d|(n/d,n%d))
        };
        let q=divide(1000u64<<32,frequency as u64).0;

        let fixed=|a:u64,b:u64|(((a as i64 as i128)*(b as i64 as i128))>>32) as u64;
        let elapsed=fixed(q,delta);
        let remaining=3_600_000u64.wrapping_sub(elapsed);
        let positive=if (remaining as i64)>0 {remaining}else{0};
        let q2=divide((positive as u32 as u64)<<32,3_600_000).0;
        let m=fixed(correction,q2).wrapping_add(elapsed).wrapping_add(b0 as u64|((b1 as u64)<<32));

        let result=s1.wrapping_add(divide(m.wrapping_sub(s1),s2).1);
        for (i,v) in saved.into_iter().enumerate(){self.put_word(s+i as u32*4,v);}
        result
    }
}

pub struct LoggerGlobals {
    pub components:u32, pub logging:u32, pub wildcard:u32, pub broker_string:u32,
    pub aliases:[[u32;2];9], pub million_suffix:u32,
}
impl Memory<'_> {

    pub fn fn_14d764(&mut self,h:&mut impl Calls,g:&LoggerGlobals,eng:[u32;2],_flags:u32,out:u32,s:u32)->u32 {
        let saved:[u32;40]=core::array::from_fn(|i|self.word(s+i as u32*4));
        let result=self.create_logger(h,g,eng,out,s);
        for (i,v) in saved.into_iter().enumerate(){self.put_word(s+i as u32*4,v);}
        result
    }
    fn logger_string_equal(&self,mut a:u32,mut b:u32)->bool {
        loop {let x=self.byte(a);if x!=self.byte(b){return false;}if x==0{return true;}a+=1;b+=1;}
    }
    fn create_logger(&mut self,h:&mut impl Calls,g:&LoggerGlobals,eng:[u32;2],out:u32,s:u32)->u32 {
        if out==0{return 0x84b02006;}
        for at in [0,4,8]{self.put_word(s+at,0);}
        if h.call(self,0x19611c,&[eng[0],eng[1],s])>>31!=0{return 0x84b02008;}
        let ctx=self.word(s);self.put_word(out,0);
        let l=h.call(self,0x1963b8,&[self.word(ctx+4),1,112]);
        if l==0{return 0x84b0200a;}

        for i in 0..112 {self.put_byte(l+i,0);}
        self.put_word(l,eng[0]);self.put_word(l+4,eng[1]);self.put_word(l+8,ctx);
        let rc=h.call(self,0x19611c,&[eng[0],eng[1],s+4]);
        let ctx2=self.word(s+4);
        let mut err=0;
        if rc>>31!=0 || self.word(ctx2+20)==0 {err=0x84b02008;}
        else {
            let broker=self.word(ctx+12);
            if h.call(self,0x17a3b8,&[broker,g.logging,g.wildcard,1,s+8])==0 && self.word(s+8)!=0 {
                let it=self.word(s+8);
                for at in [12,16,20]{self.put_word(s+at,0);}
                while h.call(self,0x17a814,&[it,s+12,s+16,0])>>31==0 {
                    let mut name=self.word(s+12);let mut value=self.word(s+16);
                    if name==0 || value==0 || self.logger_string_equal(name,g.broker_string){continue;}
                    self.put_byte(s+32,0);
                    for (index,pair) in g.aliases.iter().enumerate(){
                        if self.logger_string_equal(name,pair[0]) {
                            name=pair[1];
                            if index==3 {
                                let mut n=0;loop{let c=self.byte(value+n);self.put_byte(s+32+n,c);if c==0{break;}n+=1;}
                                let mut i=0;loop{let c=self.byte(g.million_suffix+i);self.put_byte(s+32+n+i,c);if c==0{break;}i+=1;}
                                value=s+32;
                            }
                            break;
                        }
                    }
                    let params=self.word(ctx2+20);
                    if h.call(self,0x175a1c,&[params,name,s+20])>>31==0 {
                        h.call(self,0x1757a8,&[params,self.word(s+20)]);
                    } else if h.call(self,0x1759a0,&[params,name,value])>>31!=0 {
                        err=0x84b02000;break;
                    }
                }
                h.call(self,0x17a2ec,&[it]);
            }
        }
        if err!=0 {self.fn_14d6dc(h,l,s+24);return err;}
        for i in 0..5 {
            let name=self.word(g.components+i*4);
            if name!=0 {h.call(self,0x177e0c,&[self.word(ctx+12),name,1,1,l+24+i*20,l+20+i*20]);}
        }
        self.put_word(out,l);0
    }
}

impl Memory<'_> {

    pub fn boot_heap_allocate(&mut self, a: [u32; 6]) -> u32 {
        assert_eq!(self.base, 0);
        assert!(a[0] >= 88);
        let mut saved = [0u8; 88];
        saved.copy_from_slice(&self.bytes[..88]);
        self.bytes.copy_within(a[0] as usize..a[0] as usize + 88, 0);
        let mut heap = crate::heap::EngineHeap::from_arena(self.bytes);
        heap.set_pool(a[3] as u16);
        heap.set_flags(a[4] as u16);
        heap.set_fill(a[5] as u8);
        let result = heap.block_allocate(a[1], a[2] as i32);
        self.bytes.copy_within(0..88, a[0] as usize);
        self.bytes[..88].copy_from_slice(&saved);
        result
    }

    pub fn boot_heap_release(&mut self, sub: u32, pointer: u32) {
        assert_eq!(self.base, 0);
        assert!(sub >= 112);
        let object = (sub - 24) as usize;
        let mut saved = [0u8; 88];
        saved.copy_from_slice(&self.bytes[..88]);
        self.bytes.copy_within(object..object + 88, 0);
        crate::heap::EngineHeap::from_arena(self.bytes).block_release(pointer);
        self.bytes.copy_within(0..88, object);
        self.bytes[..88].copy_from_slice(&saved);
    }

    pub fn boot_archive_prefix(&mut self, h: &mut impl Calls, g: &Globals,
        engine: u32, mode: u8, delta: u32) {
        self.boot_request_prefix(h, g, engine, mode, delta);
        let loader = self.word(engine + 0x61bc + delta);
        let manager = self.word(g.manager);
        self.mount_components(loader, manager);
        self.fn_134440(h, g, loader, mode as u32);
        self.mount_dictionary(loader, manager);
        self.fn_1343b8(h, g, loader, mode as u32);
        self.mount_rules(loader, manager);
        h.call(self, 0x19126c, &[self.word(engine + 0x124 + delta),
            self.word(engine + 0x128 + delta), self.word(engine + 0x138 + delta),
            self.word(engine + 0x154 + delta), engine + 0x11c + delta, engine]);
    }
}

impl Memory<'_> {

    pub fn fn_1961d0(&mut self, h: &mut impl Calls, a: [u32; 5]) -> u32 {
        let [heap, resource, stream, flags, out] = a;
        if out == 0 { return 0x81602006; }
        let object = h.call(self, 0x1963b8, &[heap, 32, 1]);
        if object == 0 {
            self.put_word(out, 0);
            self.put_word(out + 4, 0);
            return 0x8160200a;
        }
        self.put_word(out, object);
        self.put_word(out + 4, 0x1e61);
        self.put_half(object, 0x1e61);
        self.put_word(object + 4, heap);
        self.put_word(object + 8, resource);
        self.put_word(object + 12, stream);
        self.put_word(object + 16, flags);
        0
    }
}

impl Memory<'_> {

    pub fn fn_39f5e8(&mut self, h: &mut impl Calls, a: [u32; 3]) -> u32 {
        self.fn_163f20(h, [a[0], a[1], a[2], 0])
    }

    pub fn fn_4a7758(&mut self, h: &mut impl Calls, a: [u32; 3]) -> u32 {
        let n = a[1].wrapping_mul(a[2]);
        let method = self.word(self.word(a[0]) + 8);
        let result = h.call(self, method, &[a[0], n, 4, 4]);
        if result != 0 {
            for i in 0..n { self.put_byte(result.wrapping_add(i), 0); }
        }
        result
    }
}

pub struct WorkerGlobals {
    pub category: u32,
    pub begin: u32,
    pub broker: u32,
    pub log_type: u32,
    pub wait_factor: u32,
}

impl Memory<'_> {

    pub fn boot_worker_context_prefix(&mut self, h: &mut impl Calls,
        g: &WorkerGlobals, a: [u32; 6], scratch: u32) -> u32 {
        let saved = core::array::from_fn::<_, 4, _>(|i| self.word(scratch + i as u32 * 4));
        let rc = self.worker_context(h, g, a, scratch);
        for (i, value) in saved.into_iter().enumerate() {
            self.put_word(scratch + i as u32 * 4, value);
        }
        rc
    }

    fn worker_context(&mut self, h: &mut impl Calls,
        g: &WorkerGlobals, a: [u32; 6], s: u32) -> u32 {
        let [owner, tag, user, argument, _out, flags] = a;
        if tag != 0x68434c53 || self.word(owner) != 0x73434c53 || owner == 0 {
            return 0x80000008;
        }
        self.put_word(s, 0);
        self.put_word(s + 4, 0);
        h.call(self, 0x1738e8, &[self.word(self.word(owner + 176) + 16),
            g.category, 4, 0, g.begin]);
        let state = h.call(self, self.word(owner + 8), &[user, 1, 0x1ac]);
        if state == 0 { return 0x8000000a; }
        self.put_word(state + 20, user);
        self.put_word(state + 24, self.word(owner + 156));
        self.put_word(state + 28, self.word(owner + 160));
        self.put_word(state + 32, argument);
        self.put_word(owner + 220, self.word(owner + 220).wrapping_add(1));
        let rc = h.call(self, 0x196318, &[owner + 4, user, s]);
        assert!(rc < 0x80000000, "worker prefix requires successful task open");
        let task = self.word(s);
        self.put_word(state + 4, owner);
        self.put_word(state + 0x184, flags);
        self.put_word(state + 0x188, 0);
        self.put_word(state + 0x18c, 0);
        self.put_word(state, 0x734f424a);
        self.put_word(state + 0x1a8, 0);
        self.put_byte(state + 52, 12);
        self.put_byte(state + 53, 1);
        let rc = h.call(self, 0x177b7c, &[g.broker, self.word(owner + 184), s + 4]);
        assert!(rc < 0x80000000, "worker prefix requires successful stream open");
        let resource = self.word(self.word(owner + 176) + 8);
        let rc = h.call(self, 0x176e40, &[resource, task, state + 36]);
        assert!(rc < 0x80000000, "worker prefix requires successful mutex open");
        let rc = self.fn_1961d0(h, [task, resource, self.word(s + 4), 0, state + 8]);
        assert!(rc < 0x80000000, "worker prefix requires successful engine open");
        let rc = h.call(self, 0x19611c, &[self.word(state + 8),
            self.word(state + 12), state + 16]);
        assert!(rc < 0x80000000, "worker prefix requires successful context acquire");
        let ctx = self.word(state + 16);
        self.put_word(ctx + 28, self.word(self.word(owner + 176) + 28));
        self.put_byte(state + 40, 1);
        if self.word(owner + 180) != 0 {
            self.fn_14dc94(h, self.word(owner + 180), ctx + 16);
            if self.word(state + 24) != 0 || self.word(state + 28) != 0 {
                h.call(self, 0x14de40, &[self.word(ctx + 16), state + 24, g.log_type]);
            }
        }
        let rc = h.call(self, 0x174ca8, &[0, self.word(state + 8),
            self.word(state + 12), ctx + 20]);
        assert!(rc < 0x80000000, "worker prefix requires successful parameters open");
        let rc = h.call(self, 0x17ae04, &[0, self.word(state + 8),
            self.word(state + 12), ctx + 24]);
        assert!(rc < 0x80000000, "worker prefix requires successful linguistic open");
        h.call(self, 0x1758ac, &[self.word(ctx + 20), g.wait_factor, 1])
    }
}

impl Memory<'_> {

    pub fn fn_174ca8(&mut self, h: &mut impl Calls, a: [u32; 4], desc: u32) -> u32 {
        let [_flags, ctx, _tag, out] = a;
        if out == 0 { return 0x8b702007; }
        let heap = self.word(ctx + 4);
        let object = h.call(self, 0x1963b8, &[heap, 1, 20]);
        self.put_word(out, object);
        if object == 0 { return 0x8b70200a; }
        self.put_word(object, self.word(ctx + 4));
        let mut rc = h.call(self, 0x176e40,
            &[self.word(ctx + 8), self.word(ctx + 4), self.word(out) + 4]);
        if rc < 0x80000000 {
            let map = core::array::from_fn::<_, 8, _>(|i| match i {
                0 => self.word(ctx + 4), 1 => 0, 7 => self.word(out) + 8,
                _ => self.word(desc + (i as u32 - 2) * 4),
            });
            rc = h.call(self, 0x1964e0, &map);
            if rc < 0x80000000 {
                let list = |m: &Memory<'_>, d: u32, index: u32, target: u32| [
                    m.word(ctx + 4), m.word(d), m.word(d + 4), m.word(d + 8),
                    index, target,
                ];
                let args = list(self, desc + 20, 1, self.word(out) + 12);
                if h.call(self, 0x196a64, &args) == 0 { rc = 0x8b70200a; }
                else {
                    let args = list(self, desc + 32, 2, self.word(out) + 16);
                    if h.call(self, 0x196a64, &args) == 0 { rc = 0x8b70200a; }
                }
            }
        }
        if rc >= 0x80000000 && self.word(out) != 0 {
            for (offset, function) in [(16, 0x196c7c), (12, 0x196c7c),
                (8, 0x19665c), (4, 0x176edc)] {
                let p = self.word(self.word(out) + offset);
                if p != 0 { h.call(self, function, &[p]); }
            }
            h.call(self, 0x196428, &[self.word(ctx + 4), self.word(out)]);
            self.put_word(out, 0);
        }
        rc
    }
}

impl Memory<'_> {

    pub fn boot_vsnprintf(&mut self, dst: u32, capacity: u32, format: u32,
        arguments: u32) -> u32 {
        crate::boot_fmt::fn_10012c(self, dst, capacity, format,
            &mut crate::boot_fmt::VaList { address: arguments },
            &mut crate::boot_fmt::CLocale)
    }

    pub fn boot_sprintf(&mut self, dst: u32, format: u32, arguments: &[u32]) -> u32 {
        crate::boot_fmt::fn_100b50(self, dst, format,
            &mut RegisterVarargs { words: arguments, cursor: 0 },
            &mut crate::boot_fmt::CLocale)
    }
}

struct RegisterVarargs<'a> {
    words: &'a [u32],
    cursor: usize,
}
impl crate::boot_fmt::Arguments for RegisterVarargs<'_> {
    fn word(&mut self, _: &Memory<'_>) -> u32 {
        let value = self.words[self.cursor];
        self.cursor += 1;
        value
    }
    fn wide(&mut self, memory: &Memory<'_>) -> u64 {
        self.cursor = (self.cursor + 1) & !1;
        let low = self.word(memory) as u64;
        low | ((self.word(memory) as u64) << 32)
    }
    fn pointer(&mut self, memory: &Memory<'_>, wide: bool) -> u32 {
        let result = self.words[self.cursor];
        if wide { self.wide(memory); } else { self.word(memory); }
        result
    }
}

impl Memory<'_> {

    pub fn fn_17ad98(&mut self, h: &mut impl Calls, src: u32, heap: u32, out: u32) -> u32 {
        if src==0 || out==0 {return 0x8b602007;}
        self.put_word(out,0);
        let object=h.call(self,0x196394,&[heap,32]);
        if object==0 {return 0x8b60200a;}
        h.call(self,0x195ae8,&[object,src,32]);
        self.put_word(out,object);
        0
    }

    pub fn fn_174a04(&mut self, h: &mut impl Calls, src: u32, heap: u32, out: u32) -> u32 {
        if src == 0 || out == 0 { return 0; }
        self.put_word(out, 0);
        let object = h.call(self, 0x1963b8, &[heap, 1, 12]);
        if object == 0 { return 0x8b70200a; }
        let rc = h.call(self, 0x176054, &[object, self.word(src), self.word(src + 4), heap]);
        if rc < 0x80000000 {
            self.put_word(out, object);
            return 0;
        }
        let callbacks = self.word(object + 8);
        if callbacks != 0 {
            h.call(self, 0x196c7c, &[callbacks]);
            self.put_word(object + 8, 0);
        }
        let data = self.word(object);
        if data != 0 {
            h.call(self, 0x196428, &[heap, data]);
            self.put_word(object, 0);
        }
        self.put_word(object + 4, 0);
        h.call(self, 0x196428, &[heap, object]);
        rc
    }

    pub fn fn_4a779c(&mut self, h: &mut impl Calls, object: u32, size: u32) -> u32 {
        let target = self.word(self.word(object) + 8);
        h.call(self, target, &[object, size, 4])
    }

}

struct BootHeapResult {
    object: u32,
    size: u32,
    result: u32,
}
impl crate::frontend::HeapVtable for BootHeapResult {
    fn alloc_at(&mut self, object: u32, size: u32) -> u32 {
        assert_eq!((object, size), (self.object, self.size));
        self.result
    }
    fn realloc_at(&mut self, _: u32, _: u32, _: u32) -> u32 { panic!("unexpected reallocate") }
    fn release_at(&mut self, _: u32, _: u32) { panic!("unexpected release") }
}

impl Memory<'_> {

    pub fn boot_frontend_context_object(&mut self, a: [u32;8],
        methods: &mut impl crate::frontend::MethodTable) -> u32 {
        crate::boot_part_b2::guest_1964e0(self, a, methods)
    }

    pub fn boot_parameter_call(&mut self, h: &mut impl Calls, f: u32, a: &[u32],
        scratch: u32) -> Option<u32> {
        assert!(self.base == 0 || matches!(f,0x17b0f8|0x17b058),
            "frontend offsets require a base-zero boot view");
        let saved = core::array::from_fn::<_,128,_>(|i| self.byte(scratch + i as u32));
        let result = self.parameter_call_inner(h, f, a, scratch);
        for (i,b) in saved.into_iter().enumerate() { self.put_byte(scratch+i as u32,b); }
        result
    }

    fn parameter_call_inner(&mut self, h: &mut impl Calls, f: u32, a: &[u32], s: u32) -> Option<u32> {
        use crate::frontend as fe;
        let rc = match f {
            0x17b058 => {
                if a.iter().any(|v|*v==0) {0x8b602007} else {
                    let lock=h.call(self,0x176f2c,&[self.word(a[0]+12)]);
                    if lock>=0x80000000 {return Some(lock);}
                    self.put_word(s,0);
                    let find=h.call(self,0x1966ac,&[self.word(a[0]+16),a[1],s]);
                    let rc=if find>=0x80000000 {(find&0x1fff)|0x8b602000} else {
                        let slot=self.word(s);
                        self.put_word(a[2],slot);
                        let at=self.offset(slot+28);
                        let refs=u16::from_le_bytes(self.bytes[at..at+2].try_into().unwrap());
                        self.bytes[at..at+2].copy_from_slice(&refs.wrapping_add(1).to_le_bytes());
                        find
                    };
                    let unlock=h.call(self,0x176f5c,&[self.word(a[0]+12)]);
                    if unlock>=0x80000000 && rc<0x80000000 {unlock}else{rc}
                }
            }
            0x17b0f8 => {
                if a.iter().any(|v|*v==0) {0x8b602007} else {
                    let owner=self.word(a[0]+12);
                    let lock=h.call(self,0x176f2c,&[owner]);
                    if lock>=0x80000000 {return Some(lock);}
                    self.put_word(s,0);
                    let map=self.word(a[0]+16);
                    let find=h.call(self,0x1966ac,&[map,a[1],s]);
                    let rc=if find<0x80000000 {
                        let slot=self.word(s);
                        h.call(self,0x195ae8,&[slot,a[2],20]);
                        find
                    } else {
                        for i in 0..32 {self.put_byte(s+16+i,0);}
                        h.call(self,0x195ae8,&[s+16,a[2],20]);
                        h.call(self,0x19668c,&[map,a[1],s+16])
                    };
                    let unlock=h.call(self,0x176f5c,&[owner]);
                    if unlock>=0x80000000 && rc<0x80000000 {unlock}else{rc}
                }
            }
            0x19690c => {
                if a.iter().any(|v|*v==0) {0x89002007} else {
                    let n=fe::strlen_bytes(&self.bytes[self.offset(a[0])..])+1;
                    let allocated=h.call(self,0x196394,&[a[1],n]);
                    let object=self.word(a[1]+4);
                    fe::clone_str(self.bytes,Some(a[0]),Some(a[1]),Some(a[2]),
                        &mut BootHeapResult {object,size:n,result:allocated})
                }
            }
            0x196980 => {
                match (a[0],a[1]) {
                    (0,0)=>0, (0,_)=>u32::MAX, (_,0)=>1,
                    (x,y)=>fe::compare_str(&self.bytes[x as usize..],&self.bytes[y as usize..]) as u32,
                }
            }
            0x1758ac | 0x1757b8 => {
                let at = self.offset(s);
                let rendered = if f == 0x1758ac {
                    fe::render_unsigned(a[2], &mut self.bytes[at..at+16], 10)
                } else { fe::render_signed(a[2] as i32, &mut self.bytes[at..at+16], 10) };
                if rendered.is_err() { self.put_byte(s,0); }
                let n = fe::strlen_bytes(&self.bytes[at..]) + 1;
                h.call(self, 0x175170, &[a[0],a[1],s,n,0])
            }
            0x1759a0 => {

                if a[2] == 0 { return None; }
                let at = self.offset(a[2]);
                let n = fe::strlen_bytes(&self.bytes[at..]) + 1;
                h.call(self,0x175170,&[a[0],a[1],a[2],n,0])
            }
            0x176f2c | 0x176f5c => {
                if a[0]==0 {0x81402007} else {
                    let inner=self.word(self.word(a[0])+8);
                    let target=self.word(inner+if f==0x176f2c {8}else{12});
                    let value=if target==0 {0}else{h.call(self,target,&[self.word(a[0]+8)])};
                    if f==0x176f2c {
                        fe::acquire_lock(self.bytes,Some(a[0]),|_|value)
                    }else{fe::release_lock(self.bytes,Some(a[0]),|_|value)}
                }
            }
            0x196394 => {
                if a[0]==0 || a[1]==0 {0} else {
                    let target=self.word(self.word(a[0]));
                    h.call(self,target,&[self.word(a[0]+4),a[1]])
                }
            }
            0x196428 => {
                if a[0]!=0 && a[1]!=0 {
                    h.call(self,self.word(self.word(a[0])+12),&[self.word(a[0]+4),a[1]]);
                }
                0
            }
            0x1966ac => {
                if a[0]==0 || a[1]==0 || a[2]==0 {0x89002007} else {
                    let node=h.call(self,0x198760,&[a[0]+24,a[1]]);
                    if node==0 {0x89002014} else {self.put_word(a[2],self.word(node+20));0}
                }
            }
            0x198760 => {
                let tree=a[0];let mut node=self.word(tree);
                let mut found=0;
                while node!=tree {
                    let cmp=h.call(self,self.word(self.word(tree+36)+8),&[a[1],self.word(node+16)]) as i32;
                    if cmp==0 {found=node;break;}
                    node=self.word(node+if cmp<0 {0}else{4});
                }
                found
            }
            0x19668c => {
                if a[0]==0 || a[1]==0 || a[2]==0 {0x89002007}
                else {h.call(self,0x198e34,&[a[0]+24,a[1],a[2],0])}
            }
            0x198e34 => {
                let tree=a[0];let heap=self.word(tree+32);let vt=self.word(tree+36);
                let node=h.call(self,0x196394,&[heap,24]);
                if node==0 {0x8900200a} else {
                    self.put_word(s,0);
                    let r=if a[2]==0 {0} else {h.call(self,self.word(vt+12),&[a[2],heap,s])};
                    let value=self.word(s);
                    if r>=0x80000000 {h.call(self,0x196428,&[heap,node]);r} else {
                        for off in [0,4,8,16] {self.put_word(node+off,0);}
                        self.put_byte(node+12,0);self.put_word(node+20,value);
                        let r=h.call(self,0x198868,&[tree,node,a[1]]);
                        if r<0x80000000 {0} else {
                            let key=self.word(node+16);
                            if key!=0 {h.call(self,self.word(vt+4),&[key,heap]);}
                            if self.word(node+20)!=0 {h.call(self,self.word(vt+16),&[value,heap]);}
                            h.call(self,0x196428,&[heap,node]);r
                        }
                    }
                }
            }
            0x198868 => self.boot_tree_insert(h,a[0],a[1],a[2],s),
            0x176054 => {
                let [slot,src,n,heap]:[u32;4]=a.try_into().unwrap();
                if slot==0 {0x8b702007} else if n!=0 && self.word(slot+4)==0 {
                    let allocated=h.call(self,0x196394,&[heap,n]);
                    let object=if heap==0 {0} else {self.word(heap+4)};
                    fe::store_sized(self.bytes,Some(slot),src,n,heap,
                        &mut BootHeapResult {object,size:n,result:allocated})
                } else {

                    if (n!=0 && self.word(slot+4)<n) ||
                        (n==0 && self.word(slot)!=0 && self.word(slot+4)!=0) {return None;}
                    fe::store_sized(self.bytes,Some(slot),src,n,heap,
                        &mut BootHeapResult {object:0,size:0,result:0})
                }
            }
            0x175170 => self.boot_set_parameter(h,a.try_into().unwrap(),s),
            0x176104 => {

                struct NoCallback;
                impl fe::ElementCallback for NoCallback {
                    fn call_element(&mut self,_:u32,_:u32,_:u32)->u32 {panic!("nonempty boot callback list")}
                }
                if a.iter().all(|v|*v!=0) {
                    for list in [self.word(a[2]+8),self.word(a[0]+12)] {
                        if list!=0 && self.word(list+20)!=0 {return None;}
                    }
                }
                fe::run_element_callbacks(self.bytes,(a[0]!=0).then_some(a[0]),a[1],
                    (a[2]!=0).then_some(a[2]),a[3],&mut NoCallback)
            }
            _ => return None,
        };
        Some(rc)
    }

    fn boot_tree_insert(&mut self,h:&mut impl Calls,tree:u32,node:u32,key:u32,s:u32)->u32 {

        let mut path=[(0u32,0i32);64];let mut n=0;
        let mut x=self.word(tree);let vt=self.word(tree+36);
        while x!=tree {
            assert!(n<path.len(),"invalid boot tree depth");
            let k=self.word(x+16);
            let cmp=h.call(self,self.word(vt+8),&[key,k]) as i32;
            path[n]=(k,cmp);n+=1;
            if self.byte(tree+40)==0 && cmp==0 {return 0x89002002;}
            x=self.word(x+if cmp<0 {0}else{4});
        }
        self.put_word(s,self.word(node+16));
        let rc=h.call(self,self.word(vt),&[key,self.word(tree+32),s]);
        let created=self.word(s);
        struct Results<'a> {path:&'a[(u32,i32)],at:usize,key:u32,heap:u32,node:u32,rc:u32,created:u32}
        impl fe_tree::TreeOps for Results<'_> {
            fn tree_cmp(&mut self,key:u32,node_key:u32)->i32 {
                assert_eq!(key,self.key);let (expected,result)=self.path[self.at];self.at+=1;
                assert_eq!(node_key,expected);result
            }
            fn tree_init(&mut self,key:u32,heap:u32,node:u32)->(u32,Option<u32>) {
                assert_eq!((key,heap,node),(self.key,self.heap,self.node));(self.rc,Some(self.created))
            }
            fn tree_alloc(&mut self,_:u32,_:u32)->(u32,u32){panic!("unexpected tree allocation")}
            fn tree_free_node(&mut self,_:u32,_:u32){panic!("unexpected tree free")}
            fn tree_free_handle(&mut self,_:u32,_:u32){panic!("unexpected handle free")}
        }
        use crate::frontend as fe_tree;
        let heap=self.word(tree+32);
        fe_tree::tree_insert(self.bytes,tree,node,key,&mut Results {
            path:&path[..n],at:0,key,heap,node,rc,created,
        })
    }

    fn boot_set_parameter(&mut self,h:&mut impl Calls,a:[u32;5],s:u32)->u32 {
        let [params,key,data,size,_update]=a;
        if params==0 || key==0 || data==0 {return 0x8b702007;}
        let lock=self.word(params+4);
        let mut result=h.call(self,0x176f2c,&[lock]);
        if result>=0x80000000 {return result;}
        self.put_word(s,0);
        let lookup=h.call(self,0x1966ac,&[self.word(params+8),key,s]);
        let value=self.word(s);
        let incoming=if lookup<0x80000000 {value} else {
            self.put_word(s+16,data);self.put_word(s+20,size);self.put_word(s+24,0);s+16
        };
        let old_size=if lookup<0x80000000 && value!=0 {self.word(value+4)}else{0};
        let old_data=if old_size!=0 {self.word(value)}else{0};
        let changed=lookup>=0x80000000 || old_size!=size || {
            let aa=self.offset(old_data);let bb=self.offset(data);
            crate::frontend::compare_bytes(&self.bytes[aa..aa+size as usize],
                &self.bytes[bb..bb+size as usize],size)!=0
        };
        if changed {

            let local=if incoming==0 {0}else{self.word(incoming+8)};
            for list in [local,self.word(params+12)] {
                assert!(list==0 || self.word(list+20)==0,"unbound nonempty before-change callback list");
            }
            let attrs=self.word(params+16);
            assert!(attrs==0 || self.word(attrs+20)==0,"unbound nonempty parameter attributes");
            result=if lookup<0x80000000 {
                h.call(self,0x176054,&[value,data,size,self.word(params)])
            }else{
                h.call(self,0x19668c,&[self.word(params+8),key,incoming])
            };
            if result>=0x80000000 && lookup>=0x80000000 {result=(result&0x1fff)|0x8b702000;}
            if result<0x80000000 {
                result=h.call(self,0x176104,&[params,key,incoming,data]);
                if result>=0x80000000 && lookup>=0x80000000 {result=(result&0x1fff)|0x8b702000;}
            }
        }
        let unlock=h.call(self,0x176f5c,&[lock]);
        if unlock>=0x80000000 && result<0x80000000 {unlock}else{result}
    }
}

struct BridgePlatform<'a, H> { host: &'a mut H, arbiter: u32 }
impl<H: crate::boot_part_b::Host> crate::bridge::Platform for BridgePlatform<'_, H> {
    fn thread_id(&mut self) -> u32 { self.host.thread_id() }
    fn arbiter_address(&self) -> u32 { self.arbiter }
    fn arbitrate(&mut self, memory: &mut Memory<'_>, args: &[u32]) -> u32 {
        self.host.call(memory, 0x172d18, args)
    }
}
impl Memory<'_> {

    pub fn boot_shared_call(&mut self, host: &mut impl crate::boot_part_b::Host,
        f: u32, a: &[u32], arbiter: u32, workspace: &mut [u8])
        -> Option<Result<u32, crate::engine::Error>> {
        if !matches!(f, 0x184f20|0x184f84|0x196394|0x1963b8|0x196428|0x1964e0|
            0x176e40|0x19665c|0x163f20|0x39f5e8|0x4a7758) { return None; }
        let result = (|| {
            let mut regions = [crate::engine::Region {base:self.base, bytes:&mut *self.bytes}];
            let mut memory = crate::engine::Memory::new(&mut regions)?;
            let mut platform = BridgePlatform {host,arbiter};
            let mut flat = crate::bridge::Flat::new(&mut memory,workspace,&mut platform,0)?;
            let rc = match f {
                0x196394 => flat.fn_196394(a[0],a[1]),
                0x1963b8 => flat.fn_1963b8(a[0],a[1],a[2]),
                0x196428 => {flat.fn_196428(a[0],a[1]);0},
                0x1964e0 => flat.fn_1964e0(a.try_into().unwrap()),
                0x176e40 => flat.fn_176e40(a.try_into().unwrap()),
                0x19665c => flat.fn_19665c(a[0]),
                _ => flat.allocator_method(f,a),
            };
            flat.finish()?;
            Ok(rc)
        })();
        Some(result)
    }
}

impl Memory<'_> {

    pub fn boot_worker_defaults_prefix(&mut self, h: &mut impl Calls, state: u32,
        names: [u32;4], property_key: u32, tail: [u32;3], scratch: u32) -> u32 {
        let ctx=self.word(state+16);let params=self.word(ctx+20);
        let mut rc=h.call(self,0x1758ac,&[params,names[0],250]);
        if rc>=0x80000000 {return rc;}
        rc=h.call(self,0x1759a0,&[params,names[1],names[2]]);
        if rc>=0x80000000 {return rc;}
        rc=h.call(self,0x1757b8,&[params,names[3],0]);
        if rc>=0x80000000 {return rc;}
        let saved=core::array::from_fn::<_,80,_>(|i|self.byte(scratch+i as u32));
        for i in 0..20 {self.put_byte(scratch+i,0);}
        self.put_byte(scratch,1);
        self.put_word(scratch+4,self.word(self.word(state+4)+212));
        rc=h.call(self,0x17b0f8,&[self.word(ctx+24),property_key,scratch]);
        if rc<0x80000000 {
            self.put_word(scratch+4,self.word(self.word(state+4)+216));
            rc=h.call(self,0x17b0f8,&[self.word(ctx+24),tail[0],scratch]);
        }
        if rc<0x80000000 {
            for i in 0..20 {self.put_byte(scratch+32+i,0);}
            self.put_word(scratch+36,tail[1]);
            self.put_word(scratch+48,tail[2]);
            let owner=self.word(state+4);
            let setup=self.word(self.word(owner+188)+16);
            rc=h.call(self,setup,&[self.word(owner+192),self.word(owner+196),
                self.word(state+8),self.word(state+12),scratch+32,state,0,state+44]);
        }
        for (i,b) in saved.into_iter().enumerate(){self.put_byte(scratch+i as u32,b);}
        rc
    }
}

impl Memory<'_> {

    pub fn boot_copy_forward(&mut self, dst: u32, src: u32, n: u32) -> u32 {
        let d=self.offset(dst);let s=self.offset(src);let n=n as usize;
        if d+n<=s {
            let (left,right)=self.bytes.split_at_mut(s);
            crate::frontend::copy_bytes(&mut left[d..d+n],&right[..n],n as u32);
        } else if s+n<=d {
            let (left,right)=self.bytes.split_at_mut(d);
            crate::frontend::copy_bytes(&mut right[..n],&left[s..s+n],n as u32);
        } else {
            for i in 0..n {self.bytes[d+i]=self.bytes[s+i];}
        }
        dst
    }
}

impl Memory<'_> {

    pub fn boot_round10_call(&mut self, function: u32, a: &[u32]) -> Option<u32> {
        use crate::{frontend as fe, fe_text as ft, normalization as n};
        if self.base != 0 { return None; }
        Some(match function {
            0x3023b0 => self.rvct_memclr(a[0],a[1]),
            0x1757a8 => fe::check_handle(a[0]!=0),
            0x1769d0 => ft::strcmp_ci_1769d0(self.bytes,a[0],a[1]) as u32,
            0x176c58 => n::udiv_itoa(self.bytes,a[0],a[1],a[2]),
            0x195b38 => fe::move_bytes(self.bytes,a[0],a[1],a[2]),
            0x195c90 => fe::append_str(self.bytes,a[0],a[1]),
            0x176a48 => ft::fold_ncasecmp(self.bytes,a[0],a[1],a[2]),
            0x176ac8 => n::itoa(self.bytes,a[0],a[1],a[2]),
            0x176f94 => fe::validate_handle(a[0]!=0,a[1],a[2],a[3]),
            0x176fc0 => fe::match_halves(a[0],a[1],a[2],a[3]),
            0x17c8c0 => ft::fold_17c8c0(a[0]),
            0x195d30 => fe::find_byte(self.bytes,a[0],a[1]),
            0x195e38 => fe::copy_str_n(self.bytes,a[0],a[1],a[2]),
            0x1960a8 => fe::find_substr(self.bytes,a[0],a[1]),
            0x2ffb64 => crate::render_part_d::write_byte(self.bytes,a[0],a[1]),
            0x2ffba0 => crate::render_part_d::write_wide_bounded(self.bytes,a[0],a[1]),
            0x195aac => ft::memcmp_195aac(self.bytes,a[0],a[1],a[2]) as u32,
            0x195df8 => fe::compare_str_n(&self.bytes[a[0] as usize..],&self.bytes[a[1] as usize..],a[2] as i32) as u32,
            0x19600c => n::u8_pbrk(self.bytes,a[0],a[1]),
            0x196764 => crate::selection::arena_leaf::word_at_12(&crate::selection::arena::Arena::new(0,self.bytes),a[0]),
            0x1970a0 => n::idx5(self.bytes,a[0]),
            0x198728 => { n::node_init(self.bytes,a[0],a[1],a[2],a[3]); 0 },
            0x198a80 => n::tree_next(self.bytes,a[0],a[1]),
            0x198f74 => n::list_last(self.bytes,a[0]),
            0x198fa8 => { n::flag_set(self.bytes,a[0]); 0 },
            0x1a9948 => n::compare_packed(a[0],a[1]),
            0x1b20a4 => n::quad_copy(self.bytes,a[0],a[1],a[2]),
            0x198ae0 => n::rb_erase(self.bytes,a[0],a[1]),
            0x195cf8 => n::strrchr_at(self.bytes,a[0],a[1]),
            0x1a8684 => n::range_0_2(a[0]),
            0x1b3580 => n::dbl_deref_ne(self.bytes,a[0],a[1]),
            0x1ba3e0 => n::adler32(self.bytes,a[0],a[1],a[2]),
            0x1b3c00 => n::code_1(a[0]),
            0x1baa14 => n::table_entry(self.bytes,a[0],a[1]),
            0x1970ac => n::span_at(self.bytes,a[0],a[1],a[2]),
            0x198358 => crate::boot_part_b::fn_198358(self,a[0]),
            0x198414 => crate::boot_part_b::fn_198414(self,a[0]),
            _ => return None,
        })
    }
}

struct BootPureLock<'a> { bytes: &'a [u8], owner: Option<u32> }
impl crate::frontend::StateLock for BootPureLock<'_> {
    fn acquire(&mut self)->u32 {
        crate::frontend::acquire_lock(self.bytes,self.owner,|_|unreachable!())
    }
    fn release(&mut self)->u32 {
        crate::frontend::release_lock(self.bytes,self.owner,|_|unreachable!())
    }
}
impl Memory<'_> {

    pub fn boot_lookup_pure(&mut self,f:u32,a:&[u32],comparator:u32)->Option<u32> {
        use crate::frontend as fe;
        if self.base!=0 || !matches!(f,0x1756d0|0x175a1c) { return None; }
        let owner=if a[0]==0 {0}else{self.word(a[0]+4)};
        if owner!=0 {
            let methods=self.word(self.word(owner)+8);
            if self.word(methods+8)!=0 || self.word(methods+12)!=0 {return None;}
        }
        if a[0]!=0 {
            let map=self.word(a[0]+8);
            if map!=0 {
                let got=self.word(self.word(map+60)+8);
                let got=crate::rom_region::to_us(crate::rom_region::Rom::from_entry(got),got);
                if got!=comparator {return None;}
            }
        }
        let out=a[2];
        let size_out=if f==0x1756d0 {a[3]}else{0};
        if a[0]!=0 && a[1]!=0 && out!=0 {
            self.put_word(out,0);
            if size_out!=0 {self.put_word(size_out,0);}
        }
        let mut value=if out==0 {0}else{self.word(out)};
        let mut size=if size_out==0 {0}else{self.word(size_out)};
        let rc={
            let mem=&*self.bytes;
            let mut lock=BootPureLock {bytes:mem,owner:(owner!=0).then_some(owner)};
            let cmp=|x:u32,y:u32|match(x,y) {
                (0,0)=>0,(0,_)=>-1,(_,0)=>1,
                _=>fe::compare_str(&mem[x as usize..],&mem[y as usize..]),
            };
            fe::lookup_param(mem,(a[0]!=0).then_some(a[0]),a[1],
                if out==0 {None}else{Some(&mut value)},
                if size_out==0 {None}else{Some(&mut size)},&mut lock,cmp)
        };
        if out!=0 {self.put_word(out,value);}
        if size_out!=0 {self.put_word(size_out,size);}
        Some(rc)
    }

    pub fn boot_log_level_pure(&self,p:u32,finish:u32,mode:u32)->Option<u32> {
        use crate::frontend as fe;
        if self.base!=0 {return None;}
        let mut calls=[(0,0);6];let mut count=0;
        if p!=0 {
            for off in [8,16,24,32,40,52] {
                let vt=self.word(p+off+4);
                if vt!=0 {
                    let f=self.word(vt+40);
                    if f!=0 {
                        if f!=finish && f!=mode {return None;}
                        calls[count]=(self.word(p+off),f);count+=1;
                    }
                }
            }
        }
        let mut at=0;
        Some(fe::max_slot_level(self.bytes,(p!=0).then_some(p),|obj| {
            let (expected,target)=calls[at];at+=1;assert_eq!(obj,expected);
            if target==finish {fe::finish_request()} else {
                let state=if obj==0 {None} else {
                    let inner=self.word(obj);
                    Some((self.byte(inner+4),self.byte(inner+0x10c)))
                };
                fe::streaming_mode(state)
            }
        }))
    }
}

impl Memory<'_> {

    pub fn boot_get_uint_pure(&mut self,a:[u32;3],comparator:u32,scratch:u32)->Option<u32> {
        if self.base!=0 || a[2]==0 {return None;}
        let saved=[self.word(scratch),self.word(scratch+4)];
        self.put_word(scratch,0);self.put_word(scratch+4,0);
        let rc=self.boot_lookup_pure(0x1756d0,&[a[0],a[1],scratch,scratch+4],comparator);
        let pointer=self.word(scratch);let size=self.word(scratch+4);
        self.put_word(scratch,saved[0]);self.put_word(scratch+4,saved[1]);
        let rc=rc?;
        let value=if rc<0x80000000 && size>1 {
            crate::frontend::parse_uint(self.bytes,(pointer!=0).then_some(pointer)) as u32
        }else{0};
        self.put_word(a[2],value);
        Some(rc)
    }
}

impl Memory<'_> {

    pub fn boot_round10_stream_call(
        &mut self,h:&mut impl crate::boot_part_e::ArchiveHost,f:u32,a:&[u32],
        e:&crate::boot_part_e::ArchiveEnvironment,
    )->Option<u32> {
        let mut host=crate::boot_part_e::ArchiveCalls {h,e,wrapper:0};
        Some(match f {
            0x1b4104=>self.create_stream_wrapper(&mut host,a[0],a[1],a[2]),
            0x1b3368=>self.create_substream(&mut host,a[..8].try_into().unwrap(),a[8]),
            0x1a8cf8=>self.initialize_stream(&mut host,a[0],a[1],a[2],a[3],a[4],a[5] as u8),
            0x1a86a0=>self.validate_stream(&mut host,a[0],a[1],a[2],a[3]),
            0x1aaa80=>self.open_stream(&mut host,a[..5].try_into().unwrap(),a[5]),
            _=>return None,
        })
    }
}
