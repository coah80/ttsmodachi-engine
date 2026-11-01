

use crate::boot::Memory;

impl Memory<'_> {
    fn c_word(&self, p: u32) -> u32 {
        let i = p.checked_sub(self.base).expect("address below arena") as usize;
        u32::from_le_bytes(self.bytes[i..i + 4].try_into().unwrap())
    }
    fn c_put(&mut self, p: u32, v: u32) {
        let i = p.checked_sub(self.base).expect("address below arena") as usize;
        self.bytes[i..i + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn c_byte(&self, p: u32) -> u8 {
        self.bytes[p.checked_sub(self.base).expect("address below arena") as usize]
    }
    fn c_half(&self, p: u32) -> i32 {
        let i = p.checked_sub(self.base).expect("address below arena") as usize;
        i16::from_le_bytes(self.bytes[i..i + 2].try_into().unwrap()) as i32
    }

    pub fn reset_cursor(&mut self, p: u32) {
        self.reset_cursor_in_region(p, CursorRegion::UsEu);
    }

    pub fn reset_cursor_in_region(&mut self, p: u32, region: CursorRegion) {
        let base = self.c_word(p);
        self.c_put(p.wrapping_add(12), 0);
        self.c_put(p.wrapping_add(8), base);
        self.c_put(
            p.wrapping_add(24),
            match region {
                CursorRegion::UsEu => u32::MAX,
                CursorRegion::Kr => 0,
            },
        );
        self.bytes[(p.wrapping_add(16) - self.base) as usize] = 1;
        self.c_put(p.wrapping_add(20), 0);
    }

    pub fn timestamp(&mut self, out: u32, date: [i32; 3], time: [i32; 4]) {
        let days = calendar_days(date[0], date[1], date[2]);
        let v = (days as i64)
            .wrapping_mul(86400000)
            .wrapping_add((time[0] as i64).wrapping_mul(0x36ee80))
            .wrapping_add((time[1] as i64).wrapping_mul(60000))
            .wrapping_add((time[2] as i64).wrapping_mul(1000))
            .wrapping_add(time[3] as i64) as u64;
        self.c_put(out.wrapping_add(4), (v >> 32) as u32);
        self.c_put(out, v as u32);
    }

    pub fn calendar_date(&mut self, year: u32, month: u32, day: u32, days: i32) {
        let days = days.wrapping_sub(60);
        let mut era = days / 146097;
        let mut doe = days % 146097;
        if doe < 0 {
            era -= 1;
            doe += 146097;
        }
        let cen = doe / 36524;
        let rem = doe % 36524;
        let quad = rem / 1461;
        let rem = rem % 1461;
        let yr = rem / 365;
        let doy = rem % 365;
        let mut mp = (doy * 5 + 2) / 153;
        let mut y = era * 400 + cen * 100 + quad * 4 + yr + 2000;
        let mut d = doy - (mp * 153 + 2) / 5 + 1;
        if yr == 4 || cen == 4 {
            mp = 11;
            d = 29;
            y -= 1;
        } else if mp <= 9 {
            mp += 3;
            if year != 0 {
                self.c_put(year, y as u32);
            }
            if month != 0 {
                self.c_put(month, mp as u32);
            }
            if day != 0 {
                self.c_put(day, d as u32);
            }
            return;
        }
        mp -= 9;
        y += 1;
        if year != 0 {
            self.c_put(year, y as u32);
        }
        if month != 0 {
            self.c_put(month, mp as u32);
        }
        if day != 0 {
            self.c_put(day, d as u32);
        }
    }

    pub fn vector_slot(&self, p: u32, index: i32) -> u32 {
        if index < 0 || self.c_word(p.wrapping_add(8)) as i32 <= index {
            return 0;
        }
        self.c_word(p.wrapping_add(4))
            .wrapping_add(self.c_word(self.c_word(p)))
            .wrapping_add((index as u32).wrapping_mul(32))
    }

    pub fn is_internal_symbol(&self, symbols: u32, key: i32) -> u32 {
        let p = self.vector_slot(symbols, key);
        u32::from(p != 0 && self.c_byte(p) == b'_' && self.c_byte(p.wrapping_add(1)) == b'I')
    }

    pub fn coordinate_value(&self, table: u32, a: i32, b: i32) -> u32 {
        let idx = coordinate_hash(a, b) as u32;
        let mut n = self.c_word(self.c_word(table).wrapping_add(idx.wrapping_mul(4)));
        while n != 0 {
            if self.c_half(n.wrapping_add(6)) == b && self.c_half(n.wrapping_add(4)) == a {
                return self.c_word(n);
            }
            n = self.c_word(n.wrapping_add(8));
        }
        0
    }

    pub fn rvct_memclr(&mut self, p: u32, count: u32) -> u32 {
        self.rvct_word_fill(p, count, 0)
    }

    pub fn rvct_word_fill(&mut self, mut p: u32, mut count: u32, word: u32) -> u32 {
        while count >= 4 {
            self.c_put(p, word);
            p = p.wrapping_add(4);
            count -= 4;
        }
        if count >= 2 {
            let i = (p - self.base) as usize;
            self.bytes[i..i + 2].copy_from_slice(&(word as u16).to_le_bytes());
            p = p.wrapping_add(2);
            count -= 2;
        }
        if count != 0 {
            self.bytes[(p - self.base) as usize] = word as u8;
            p = p.wrapping_add(1);
        }
        p
    }

    pub fn rvct_copy(&mut self, mut dst: u32, mut src: u32, count: u32) -> u32 {
        for _ in 0..count {
            let b = self.c_byte(src);
            self.bytes[(dst - self.base) as usize] = b;
            dst = dst.wrapping_add(1);
            src = src.wrapping_add(1);
        }
        dst
    }

    pub fn directory_search(&mut self, pp: u32, index: u32) -> i32 {
        let q = self.c_word(pp);
        let ka = self.c_word(q.wrapping_add(8)) as i32;
        let kb = self.c_word(q.wrapping_add(4)) as i32;
        let l1 = self.c_word(index.wrapping_add(16));
        let at = l1.wrapping_add((ka as u32).wrapping_mul(8));
        let mut lo = self.c_word(at) as i32;
        if lo == -1 {
            return -1;
        }
        let mut hi = self.c_word(at.wrapping_add(4)) as i32;
        let entry = |s: &Self, i: i32| -> u32 {
            if i >= 0 && (s.c_word(index.wrapping_add(12)) as i32) > i {
                s.c_word(s.c_word(index).wrapping_add((i as u32).wrapping_mul(4)))
            } else {
                0
            }
        };
        let compare = |s: &Self, m: u32| {
            (ka, kb).cmp(&(
                s.c_word(m.wrapping_add(8)) as i32,
                s.c_word(m.wrapping_add(4)) as i32,
            ))
        };
        let m = entry(self, lo);
        match compare(self, m) {
            core::cmp::Ordering::Less => return -1,
            core::cmp::Ordering::Equal => {
                self.c_put(pp, m);
                return lo;
            }
            _ => {}
        }
        {
            let m = entry(self, hi);
            match compare(self, m) {
                core::cmp::Ordering::Greater => return -1,
                core::cmp::Ordering::Equal => {
                    self.c_put(pp, m);
                    return hi;
                }
                _ => {}
            }
        }
        while hi.wrapping_sub(lo) != 1 {
            let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
            let m = entry(self, mid);
            match compare(self, m) {
                core::cmp::Ordering::Less => hi = mid,
                core::cmp::Ordering::Greater => lo = mid,
                core::cmp::Ordering::Equal => {
                    self.c_put(pp, m);
                    return mid;
                }
            }
        }
        self.c_put(pp, 0);
        -1
    }

    pub fn candidate_contains(&self, e: u32, tab: u32, symbols: u32) -> u32 {
        if tab == 0 {
            return 0;
        }
        let first = if self.c_word(tab.wrapping_add(20)) == 2 {
            0
        } else {
            1
        };
        let pair = [
            self.c_word(tab.wrapping_add(24)),
            self.c_word(tab.wrapping_add(28)),
        ];
        for i in first..2 {
            let cand = pair[i];
            if self.is_internal_symbol(symbols, self.c_word(cand) as i32) == 1 {
                if self.candidate_contains(e, cand, symbols) == 1 {
                    return 1;
                }
            } else if self.c_word(cand) == self.c_word(e)
                && self.c_word(e.wrapping_add(4)) == self.c_word(tab.wrapping_add(8 + i as u32 * 4))
            {
                return 1;
            }
        }
        0
    }

    fn c_equal(&self, mut a: u32, mut b: u32) -> bool {
        loop {
            let x = self.c_byte(a);
            if x != self.c_byte(b) {
                return false;
            }
            if x == 0 {
                return true;
            }
            a = a.wrapping_add(1);
            b = b.wrapping_add(1);
        }
    }

    pub fn candidate_attribute(&self, e: u32, tab: u32, symbols: u32) -> Option<u32> {
        if tab == 0 {
            return None;
        }
        let first = if self.c_word(tab.wrapping_add(20)) == 2 {
            0
        } else {
            1
        };
        let pair = [
            self.c_word(tab.wrapping_add(24)),
            self.c_word(tab.wrapping_add(28)),
        ];
        for i in first..2 {
            let cand = pair[i];
            if self.is_internal_symbol(symbols, self.c_word(cand) as i32) == 1 {
                if let Some(value) = self.candidate_attribute(e, cand, symbols) {
                    return Some(value);
                }
            } else if self.c_word(e) == self.c_word(cand)
                && self.c_word(e.wrapping_add(4)) == self.c_word(tab.wrapping_add(8 + i as u32 * 4))
            {
                let mut n = self.c_word(cand.wrapping_add(16));
                while n != 0 {
                    if self.c_equal(self.c_word(n), self.c_word(e.wrapping_add(8))) {
                        return Some(self.c_word(n.wrapping_add(4)));
                    }
                    n = self.c_word(n.wrapping_add(8));
                }
            }
        }
        None
    }

    fn c_operand(&self, operand: u32, context: u32, symbols: u32) -> u32 {
        let reference = self.c_word(operand.wrapping_add(4));
        if self.c_byte(operand) != 0 {
            return reference;
        }
        if let Some(v) = self.candidate_attribute(reference, context, symbols) {
            return v;
        }
        let p = self.c_word(reference.wrapping_add(8));
        if self.c_byte(p) == b'P'
            && self.c_byte(p.wrapping_add(1)) == b'O'
            && self.c_byte(p.wrapping_add(2)) == b'S'
            && self.c_byte(p.wrapping_add(3)) == 0
        {
            self.vector_slot(symbols, self.c_word(reference) as i32)
        } else {
            0
        }
    }

    pub fn compare_operands(
        &self,
        kind: i32,
        left: u32,
        right: u32,
        context: u32,
        symbols: u32,
    ) -> u32 {
        let a = self.c_operand(left, context, symbols);
        let b = self.c_operand(right, context, symbols);
        if a == 0 || b == 0 {
            return 0;
        }
        let equal = self.c_equal(a, b);
        u32::from((kind == 2 && equal) || (kind == 3 && !equal))
    }

    pub fn evaluate_predicate(&self, n: u32, context: u32, symbols: u32) -> u32 {
        if n == 0 {
            return 1;
        }
        match self.c_byte(n) {
            0 => u32::from(
                self.evaluate_predicate(self.c_word(n.wrapping_add(4)), context, symbols) == 1
                    && self.evaluate_predicate(self.c_word(n.wrapping_add(8)), context, symbols)
                        == 1,
            ),
            1 => u32::from(
                self.evaluate_predicate(self.c_word(n.wrapping_add(4)), context, symbols) == 1
                    || self.evaluate_predicate(self.c_word(n.wrapping_add(8)), context, symbols)
                        == 1,
            ),
            t @ (2 | 3) => self.compare_operands(
                t as i32,
                self.c_word(n.wrapping_add(4)),
                self.c_word(n.wrapping_add(8)),
                context,
                symbols,
            ),
            4 => self.candidate_contains(self.c_word(n.wrapping_add(4)), context, symbols),
            _ => 0,
        }
    }
}

pub fn calendar_days(year: i32, month: i32, day: i32) -> i32 {
    let mut y = year.wrapping_sub(2000);
    let mp = if month <= 2 {
        y = y.wrapping_sub(1);
        month.wrapping_add(9)
    } else {
        month.wrapping_sub(3)
    };
    let leap = if y >= 0 || y % 400 == 0 || (y % 100 != 0 && y & 3 == 0) {
        1
    } else {
        0
    };
    let c = y / 100;
    let yoe = y.wrapping_sub(c.wrapping_mul(100));
    (c.wrapping_mul(146097) / 4)
        .wrapping_add(yoe.wrapping_mul(1461) / 4)
        .wrapping_add(mp.wrapping_mul(153).wrapping_add(2) / 5)
        .wrapping_add(day.wrapping_sub(1))
        .wrapping_add(leap)
        .wrapping_add(59)
}

pub fn coordinate_hash(a: i32, b: i32) -> i32 {
    let m1 = b % 20;
    let t = (a % 200).wrapping_mul(20);
    if (b / 20) & 1 == 0 {
        t.wrapping_add(m1)
    } else {
        t.wrapping_sub(m1).wrapping_add(19)
    }
}

pub trait BootHost {
    fn svc(&mut self, number: u8, registers: [u32; 4]) -> [u32; 4];
    fn errno_address(&mut self) -> u32;
    fn allocate(&mut self, memory: &mut Memory<'_>, allocator: u32, size: u32) -> u32;
}

pub fn svc_0a(host: &mut impl BootHost, registers: [u32; 4]) -> [u32; 4] {
    host.svc(0x0a, registers)
}

pub fn svc_3c(host: &mut impl BootHost, registers: [u32; 4]) -> [u32; 4] {
    host.svc(0x3c, registers)
}

pub fn delay_result(count: i32) -> i32 {
    if count > 2 {
        -(count & 1)
    } else {
        count.wrapping_sub(2)
    }
}

pub fn return_leaf(value: u32) -> u32 {
    value
}

impl Memory<'_> {

    pub fn acquire_guard(&mut self, guard: u32) -> u32 {
        if self.c_word(guard) != 0 {
            return 0;
        }
        self.c_put(guard, 1);
        1
    }

    pub fn wide_substring(&self, mut haystack: u32, needle: u32) -> u32 {
        loop {
            let mut offset = 0u32;
            loop {
                let n = self.c_half(needle.wrapping_add(offset)) as u16;
                if n == 0 {
                    return haystack;
                }
                let h = self.c_half(haystack.wrapping_add(offset)) as u16;
                if h == 0 {
                    return 0;
                }
                if h != n {
                    break;
                }
                offset = offset.wrapping_add(2);
            }
            haystack = haystack.wrapping_add(2);
        }
    }

    pub fn locale_lower(&self, table: u32, character: u32) -> u32 {
        if self.c_byte(table.wrapping_add(character)) & 0x10 != 0 {
            character.wrapping_add(32)
        } else {
            character
        }
    }

    pub fn wide_case_compare(&self, table: u32, mut a: u32, mut b: u32) -> i32 {
        loop {
            let x = self.c_half(a) as u16 as u32;
            let y = self.c_half(b) as u16 as u32;
            let lx = if x <= 0x7f {
                self.locale_lower(table, x)
            } else {
                x
            };
            let ly = if y <= 0x7f {
                self.locale_lower(table, y)
            } else {
                y
            };
            let difference = lx.wrapping_sub(ly) as i32;
            if difference != 0 || x == 0 {
                return difference;
            }
            a = a.wrapping_add(2);
            b = b.wrapping_add(2);
        }
    }

    pub fn set_errno(&mut self, host: &mut impl BootHost, error: u32) -> u32 {
        let address = host.errno_address();
        self.c_put(address, error);
        address
    }

    pub fn insert_coordinate(
        &mut self,
        host: &mut impl BootHost,
        table: u32,
        value: u32,
        a: i32,
        b: i32,
        allocator: u32,
    ) -> u32 {
        let bucket = self
            .c_word(table)
            .wrapping_add((coordinate_hash(a, b) as u32).wrapping_mul(4));
        let mut node = self.c_word(bucket);
        let mut previous = 0;
        while node != 0 {
            if self.c_half(node.wrapping_add(6)) == b && self.c_half(node.wrapping_add(4)) == a {
                return 0x8c200001;
            }
            previous = node;
            node = self.c_word(node.wrapping_add(8));
        }
        node = host.allocate(self, allocator, 12);
        if node == 0 {
            return 0x8c20200a;
        }
        let at = (node - self.base) as usize;
        self.bytes[at + 4..at + 6].copy_from_slice(&(a as i16).to_le_bytes());
        self.c_put(node.wrapping_add(8), 0);
        let at = (node - self.base) as usize;
        self.bytes[at + 6..at + 8].copy_from_slice(&(b as i16).to_le_bytes());
        self.c_put(node, value);
        if previous == 0 {
            self.c_put(bucket, node);
        } else {
            self.c_put(previous.wrapping_add(8), node);
        }
        0
    }
}

#[derive(Clone, Copy)]
pub enum CursorRegion {
    UsEu,
    Kr,
}

pub trait BootCHost {

    fn address(&self, us: u32) -> u32 {
        us
    }
    fn invoke(&mut self, memory: &mut Memory<'_>, function: u32, arguments: &[u32]) -> [u32; 3];
}

impl Memory<'_> {
    fn c_store_byte(&mut self, p: u32, value: u8) {
        self.bytes[(p - self.base) as usize] = value;
    }
    fn c_store_half(&mut self, p: u32, value: u16) {
        let i = (p - self.base) as usize;
        self.bytes[i..i + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn morphology_lookup(&self, input: u32, table: u32) -> u32 {
        let (mut lo, mut hi) = (0u32, 0u32);
        for n in 0..8u32 {
            let c = self.c_half(input + n * 2) as u16;
            if c == 0x3a {
                if lo | hi == 0 {
                    return 0;
                }
                for i in 0..32 {
                    let e = table + i * 16;
                    if self.c_word(e) == lo && self.c_word(e + 4) == hi {
                        return self.c_word(e + 8);
                    }
                }
                return 0;
            }
            hi = hi.wrapping_shl(8) | (lo >> 24);
            let b = c as u8 as i8 as i32;
            lo = lo.wrapping_shl(8) | b as u32;
            hi |= (b >> 31) as u32;
        }
        0
    }

    pub fn set_stream_property<H: BootCHost>(&mut self, h: &mut H, args: [u32; 4]) -> u8 {
        h.invoke(
            self,
            0x1b35a0,
            &[args[0], args[1], args[2], args[3], 0, 0, 1],
        )[0] as u8
    }

    pub fn create_stream_wrapper<H: BootCHost>(
        &mut self,
        h: &mut H,
        heap: u32,
        owner: u32,
        out: u32,
    ) -> u32 {
        let a = h.invoke(self, 0x1b2140, &[heap, 12]);
        if a[1] as u8 != 0 {
            return a[1] as u8 as u32;
        }
        let p = a[0];
        let mut rc = h.invoke(self, 0x1ba3c0, &[p])[0];
        if rc == 0 {
            self.c_put(p, h.address(0x917e3c));
            self.c_put(p + 4, owner);
            let r = h.invoke(self, 0x1babcc, &[owner, owner, 10, 10]);
            rc = r[0];
            self.c_put(p + 8, r[1]);
        }
        if rc == 0 {
            self.c_put(out, p);
        } else {
            h.invoke(self, 0x1b2128, &[heap, p]);
            self.c_put(out, 0);
        }
        rc as u8 as u32
    }

    pub fn create_substream<H: BootCHost>(&mut self, h: &mut H, args: [u32; 8], out: u32) -> u32 {
        let [heap, a1, a2, a3, a4, a5, a6, a7] = args;
        let a = h.invoke(self, 0x1b2140, &[heap, 76]);
        if a[1] as u8 != 0 {
            return a[1] as u8 as u32;
        }
        let p = a[0];
        let err = h.invoke(self, 0x1b449c, &[p, 1])[0];
        if err == 0 {
            let t = h.address(0x917e04);
            self.c_put(p, t - 24);
            self.c_put(p + 12, t);
            self.c_put(p + 16, t + 28);
            self.c_put(p + 48, a2);
            self.c_put(p + 52, a3);
            self.c_store_byte(p + 20, a6 as u8);
            self.c_put(p + 24, 0);
            self.c_put(p + 28, 0x4000);
            self.c_store_half(p + 36, a7 as u16);
            self.c_put(p + 32, a4);
            self.c_put(p + 44, a5);
            self.c_put(p + 68, a1);
            self.c_store_byte(p + 72, 1);
            let count = a7 as u16 as u32;
            let size = if count == 0 {
                a5
            } else {
                let q = h.invoke(
                    self,
                    0x1994d0,
                    &[count.wrapping_add(a5).wrapping_add(3), count + 4],
                )[0];
                let size = a5.wrapping_sub(q.wrapping_mul(4));
                self.c_put(p + 40, size);
                let hash = h.invoke(self, 0x1ba3e0, &[0, 0, 0])[0];
                self.c_put(p + 64, hash);
                size
            };
            self.c_put(p + 40, size);
            self.c_put(p + 56, size);
            self.c_put(p + 60, count.min(size));
        }
        if err != 0 {
            h.invoke(self, 0x1b2128, &[heap, p]);
            self.c_put(out, 0);
        } else {
            self.c_put(p + 8, heap);
            self.c_put(out, p);
        }
        err as u8 as u32
    }

    pub fn initialize_stream<H: BootCHost>(
        &mut self,
        h: &mut H,
        p: u32,
        a: u32,
        b: u32,
        c: u32,
        d: u32,
        e: u8,
    ) -> u32 {
        let rc = h.invoke(self, 0x1b449c, &[p, 1])[0];
        if rc != 0 {
            return rc as u8 as u32;
        }
        let t = h.address(0x8faae0);
        self.c_put(p + 12, t - 8);
        self.c_put(p + 20, t + 108);
        for off in [64, 60, 40] {
            self.c_put(p + off, 0);
        }
        self.c_put(p + 16, t);
        self.c_store_byte(p + 72, 0);
        self.c_put(p + 76, 0);
        self.c_put(p, t - 32);
        self.c_store_byte(p + 81, 0);
        self.c_store_byte(p + 80, e);
        self.c_put(p + 68, a);
        let r = h.invoke(self, 0x1b2140, &[a, 16]);
        self.c_put(p + 48, r[0]);
        if r[1] as u8 != 0 {
            return r[1] as u8 as u32;
        }
        h.invoke(self, 0x195ae8, &[r[0], b, 16]);
        self.c_put(p + 52, c);
        if d == 0 {
            self.c_put(p + 56, 0);
            self.c_store_byte(p + 38, 1);
        } else {
            self.c_store_byte(p + 38, self.c_byte(d));
            let r = h.invoke(self, 0x1b2140, &[a, 1]);
            self.c_put(p + 56, r[0]);
            if r[1] as u8 != 0 {
                return r[1] as u8 as u32;
            }
            h.invoke(self, 0x195ae8, &[r[0], d, 1]);
        }
        let r = h.invoke(self, 0x1b4104, &[a, a]);
        self.c_put(p + 76, r[1]);
        r[0] as u8 as u32
    }

    pub fn open_stream<H: BootCHost>(&mut self, h: &mut H, args: [u32; 5], out: u32) -> u32 {
        let [heap, a1, a2, a3, float_bits] = args;
        let a = h.invoke(self, 0x1b2140, &[heap, 84]);
        if a[1] as u8 != 0 {
            return a[1] as u8 as u32;
        }
        let p = a[0];
        let mut r = h.invoke(self, 0x1a8cf8, &[p, a1, a2, a3, float_bits, 1])[0];
        if r == 0 {
            let result = h.invoke(self, 0x1a86a0, &[p, 0]);
            r = result[0];
            if r == 0 {

                if result[2] as u8 == 1 || result[1] as u8 == 1 {
                    r = 4;
                }
            }
        }
        if r == 0 {
            self.c_put(p + 8, heap);
            self.c_put(out, p);
        } else {
            h.invoke(self, 0x1b2128, &[heap, p]);
            self.c_put(out, 0);
        }
        r as u8 as u32
    }

    pub fn reset_playback<H: BootCHost>(&mut self, h: &mut H, p: u32) {
        if self.c_byte(p + 4) == 0 {
            return;
        }
        for offset in [0xe8, 0x14f0] {
            let q = p + offset;
            while self.c_word(q) != self.c_word(q + 4) {
                let v = self.c_word(q + 8 + self.c_word(q).wrapping_mul(20) + 16);
                h.invoke(self, 0x1bcd84, &[p, q, v]);
            }
        }
        let f = f32::from_bits(self.c_word(p + 0x58)) + (self.c_word(p + 0x68) as i32 as f32);
        self.c_put(p + 0x54, 0);

        let n = (f as i32).min(self.c_word(p + 0x40) as i32);
        for i in 0..n.max(0) as u32 {
            self.c_put(self.c_word(p + 0x3c) + i * 4, 0);
        }
        for off in [
            0x58, 0x5c, 0x60, 0x64, 0x68, 0xcc, 0xd0, 0xd4, 0xd8, 0xdc, 0xac, 0xb8,
        ] {
            self.c_put(p + off, 0);
        }
        self.c_put(p + 0x78, 1.0f32.to_bits());
        self.c_put(p + 0x2908, 0x80000000);
        self.c_put(p + 0x291c, 0x80000000);

        h.invoke(self, 0x1b033c, &[self.c_word(p + 0x44)]);
    }
}

pub fn unsigned_divmod<H: BootCHost>(
    memory: &mut Memory<'_>,
    h: &mut H,
    n: u64,
    d: u64,
) -> (u64, u64) {
    if d == 0 {
        let r = h.invoke(memory, 0x1a2ea4, &[0]);
        (r[0] as u64 | ((r[1] as u64) << 32), n)
    } else {
        (n / d, n % d)
    }
}

#[derive(Clone, Copy)]
pub struct DivisionContinuation {
    pub remainder: u32,
    pub divisor: u32,
    pub quotient: u32,
    pub signs: u32,
    pub carry: bool,
}
#[derive(Clone, Copy)]
pub enum DivisionEntry {
    Normalize8,
    Rescale8,
    Bits15,
    Bits11,
    Bits7,
    Bits4,
    Bits3,
    Bits1,
    Small,
    Signed,
    DivideByZero,
}
fn division_bit(s: &mut DivisionContinuation, shift: u32) {
    let take = s.remainder >> shift >= s.divisor;
    if take {
        s.remainder = s.remainder.wrapping_sub(s.divisor.wrapping_shl(shift));
    }
    let q = (s.quotient as u64) * 2 + u64::from(take);
    s.quotient = q as u32;
    s.carry = q > u32::MAX as u64;
}
fn division_finish(s: DivisionContinuation) -> (u32, u32) {
    let take = s.remainder >= s.divisor;
    let rem = if take {
        s.remainder.wrapping_sub(s.divisor)
    } else {
        s.remainder
    };
    (
        s.quotient.wrapping_mul(2).wrapping_add(u32::from(take)),
        rem,
    )
}

pub fn signed_divide_continue<H: BootCHost>(
    memory: &mut Memory<'_>,
    host: &mut H,
    entry: DivisionEntry,
    mut s: DivisionContinuation,
) -> (u32, u32) {
    use DivisionEntry::*;
    let mut stage = match entry {
        Normalize8 => 0,
        Rescale8 => 1,
        Bits15 => 2,
        Bits11 => 3,
        Bits7 => 4,
        Bits4 => 5,
        Bits3 => 6,
        Bits1 => 7,
        Small => 8,
        Signed => 9,
        DivideByZero => 10,
    };
    loop {
        match stage {
            0 => {
                s.divisor = s.divisor.wrapping_shl(8);
                s.quotient |= 0xff000000;
                if s.remainder >> 16 >= s.divisor {
                    s.quotient |= 0xff0000;
                    s.divisor = s.divisor.wrapping_shl(8);
                }
                if s.remainder >> 12 < s.divisor {
                    stage = 3;
                } else if s.divisor == 0 {
                    stage = 10;
                } else {
                    s.carry = false;
                    stage = 1;
                }
            }
            1 => {
                if s.carry {
                    s.divisor >>= 8;
                }
                stage = 2;
            }
            2 => {
                for shift in (12..=15).rev() {
                    division_bit(&mut s, shift);
                }
                stage = 3;
            }
            3 => {
                for shift in (8..=11).rev() {
                    division_bit(&mut s, shift);
                }
                stage = if s.carry { 1 } else { 4 };
            }
            4 => {
                for shift in (4..=7).rev() {
                    division_bit(&mut s, shift);
                }
                stage = 6;
            }
            5 => {
                division_bit(&mut s, 4);
                stage = 6;
            }
            6 => {
                division_bit(&mut s, 3);
                division_bit(&mut s, 2);
                stage = 7;
            }
            7 => {
                division_bit(&mut s, 1);
                return division_finish(s);
            }
            8 => {
                return if s.remainder >= s.divisor {
                    (1, s.remainder.wrapping_sub(s.divisor))
                } else {
                    (0, s.remainder)
                };
            }
            9 => {
                let negative_n = s.remainder >> 31 != 0;
                let negative_d = s.divisor >> 31 != 0;
                s.quotient = s.divisor & 0x80000000;
                if negative_d {
                    s.divisor = s.divisor.wrapping_neg();
                }
                s.signs = s.quotient ^ if negative_n { u32::MAX } else { 0 };
                if negative_n {
                    s.remainder = s.remainder.wrapping_neg();
                }
                if s.remainder >> 4 < s.divisor {
                    stage = 12;
                    continue;
                }
                if s.remainder >> 8 < s.divisor {
                    stage = 11;
                    continue;
                }
                s.divisor = s.divisor.wrapping_shl(6);
                s.quotient |= 0xfc000000;
                if s.remainder >> 8 < s.divisor {
                    stage = 11;
                    continue;
                }
                s.divisor = s.divisor.wrapping_shl(6);
                s.quotient |= 0x3f00000;
                if s.remainder >> 8 < s.divisor {
                    stage = 11;
                    continue;
                }
                s.divisor = s.divisor.wrapping_shl(6);
                s.quotient |= 0xfc000;
                if s.remainder >> 8 >= s.divisor {
                    s.divisor = s.divisor.wrapping_shl(6);
                    s.quotient |= 0x3f00;
                }
                if s.divisor == 0 {
                    if s.signs >> 31 != 0 {
                        s.remainder = s.remainder.wrapping_neg();
                    }
                    stage = 10;
                } else {
                    stage = 11;
                }
            }
            10 => {
                let q = host.invoke(memory, 0x1a2ea4, &[0])[0];
                return (q, s.remainder);
            }
            11 => {
                for shift in (4..=7).rev() {
                    division_bit(&mut s, shift);
                }
                stage = 12;
            }
            12 => {
                division_bit(&mut s, 3);
                division_bit(&mut s, 2);
                if s.carry {
                    s.divisor >>= 6;
                    stage = 11;
                    continue;
                }
                division_bit(&mut s, 1);
                let (mut q, mut r) = division_finish(s);
                if s.signs & 0x80000000 != 0 {
                    q = q.wrapping_neg();
                }
                if s.signs & 0x40000000 != 0 {
                    r = r.wrapping_neg();
                }
                return (q, r);
            }
            _ => unreachable!(),
        }
    }
}

impl Memory<'_> {

    pub fn unpack_clock(&mut self, out: u32, input: u32) {
        let n = (self.c_word(input) as u64 | ((self.c_word(input + 4) as u64) << 32)) as i64;
        let n = n.wrapping_add(0x395f7cfc8400);
        let q = n / 86400000;
        let m = (n % 86400000) as i32;
        let days = (q as i32).wrapping_add(0xfff4dbf9u32 as i32);
        let mut date = [0u8; 12];
        let mut local = Memory {
            base: 100,
            bytes: &mut date,
        };
        local.calendar_date(100, 104, 108, days);
        let year = local.c_word(100);
        let month = local.c_word(104);
        let day = local.c_word(108);
        let weekday = (days.wrapping_add(6) % 7 + 7) % 7;
        let scaled = |k: i64, shift: u32| {
            let hi = ((k * (m as i64)) >> 32) as i32;
            (hi >> shift) - (hi >> 31)
        };
        let hours = scaled(0x4a90be59, 20);
        let minutes = scaled(0x45e7b273, 14) % 60;
        let seconds = scaled(0x10624dd3, 6);
        let ms = m.wrapping_sub(seconds.wrapping_mul(1000));
        let o1 = (month & 255)
            | ((day & 255) << 8)
            | ((weekday as u32 & 255) << 16)
            | ((hours as u32) << 24);
        let o2 =
            (minutes as u32 & 255) | (((seconds % 60) as u32 & 255) << 8) | ((ms as u32) << 16);
        self.c_put(out + 8, o2);
        self.c_put(out, year);
        self.c_put(out + 4, o1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatDispatch {
    pub value: u32,
    pub continuation: Option<u32>,
}
pub fn float_dispatch(a: u32, b: u32, incoming_r2: u32, word: u32) -> FloatDispatch {
    let r2 = if word & 0x80000000 == 0 {
        b.wrapping_shl(1)
    } else {
        incoming_r2
    };
    let index = if word & 0x80000000 == 0 && r2 > 0xff000000 || a.wrapping_shl(1) > 0xff000000 {
        8
    } else if a.wrapping_shl(1) != 0xff000000 {
        b >> 31
    } else {
        (a >> 31) * 3 + 2 + if r2 == 0xff000000 { (b >> 31) + 1 } else { 0 }
    };
    let action = (word >> (index * 3)) & 7;
    if action < 4 {
        return FloatDispatch {
            value: a,
            continuation: Some(action),
        };
    }
    let value = if action >= 6 {
        0x7fc00000
    } else {
        let x = if action == 4 { b } else { a };
        if x.wrapping_shl(1) != 0 && x.wrapping_shl(1) < 0x1000000 {
            0
        } else {
            x
        }
    };
    FloatDispatch {
        value,
        continuation: None,
    }
}

pub trait BootC5Host {
    fn address(&self, us: u32) -> u32 {
        us
    }
    fn call(&mut self, m: &mut Memory<'_>, function: u32, args: &[u32], out: &mut [u32; 4]) -> u32;
    fn read(&mut self, m: &mut Memory<'_>, function: u32, context: u32, buf: &mut [u8]) -> u32;
}

impl Memory<'_> {

    pub fn chart_driver(&mut self, h: &mut impl BootC5Host, args: [u32; 11]) -> u32 {
        let [tab, last, start, span, cfg, ctr, out, exp, h9, i10, alloc] = args;
        let mut pos = start as i32;
        let mut acc = span as i32;
        let mut ret = 0;
        let mut o = [0; 4];
        while pos >= 0 {
            let mut end = acc.wrapping_sub(span as i32).wrapping_sub(1);
            while end >= 0 {
                let mut a = h.call(self, 0x304aa0, &[tab, pos as u32, end as u32], &mut o);
                if a != 0 {
                    let k = acc.wrapping_sub(end);
                    let i = end.wrapping_add(pos);
                    'left: loop {
                        let mut b = h.call(
                            self,
                            0x304aa0,
                            &[tab, i.wrapping_add(1) as u32, k.wrapping_sub(1) as u32],
                            &mut o,
                        );
                        if b == 0 {
                            break 'left;
                        }
                        loop {
                            let rc =
                                h.call(self, 0x2e6750, &[self.c_word(a), self.c_word(b)], &mut o);
                            let mut bounds = o[0];
                            if rc != u32::MAX {
                                let mut next = rc.wrapping_sub(1);
                                loop {
                                    let np = h.call(self, 0x196394, &[alloc, 0x38], &mut o);
                                    if np == 0 {
                                        return 0;
                                    }
                                    self.c_put(np, self.c_word(bounds));
                                    self.c_put(np + 28, b);
                                    self.c_put(np + 24, a);
                                    self.c_put(np + 4, u32::MAX);
                                    self.c_put(np + 16, 0);
                                    self.c_put(np + 20, 2);
                                    self.c_put(np + 52, 0);
                                    self.c_put(np + 8, self.c_word(bounds + 12));
                                    self.c_put(np + 12, self.c_word(bounds + 16));
                                    let index = self.c_word(bounds + 36);
                                    let mut accept = true;
                                    if index != u32::MAX {
                                        let directory = self.c_word(h.address(0x9e09ac));
                                        let entry = self
                                            .c_word(directory.wrapping_add(index.wrapping_mul(4)));
                                        accept = h.call(
                                            self,
                                            0x303868,
                                            &[self.c_word(entry), np],
                                            &mut o,
                                        ) == 1;
                                    }
                                    if accept && self.c_word(cfg + 28) == 1 {
                                        accept = h.call(self, 0x3034d4, &[self.c_word(np)], &mut o)
                                            == 1
                                            || self.c_word(np) == exp;
                                    }
                                    if accept {
                                        let v = h.call(
                                            self,
                                            0x304aa0,
                                            &[tab, pos as u32, acc as u32],
                                            &mut o,
                                        );
                                        if v == 0 {
                                            let code = h.call(
                                                self,
                                                0x304afc,
                                                &[tab, np, pos as u32, acc as u32, alloc],
                                                &mut o,
                                            );
                                            if code & 0x1fff == 10 {
                                                return 0;
                                            }
                                        } else {
                                            let mut prev = v;
                                            while self.c_word(prev + 52) != 0 {
                                                prev = self.c_word(prev + 52);
                                            }
                                            self.c_put(prev + 52, np);
                                        }
                                        if pos == 0
                                            && acc as u32 == last.wrapping_sub(1)
                                            && self.c_word(bounds) == exp
                                        {
                                            let node = h.call(
                                                self,
                                                0x2ec128,
                                                &[np, cfg, i10, alloc],
                                                &mut o,
                                            );
                                            ret = h.call(
                                                self,
                                                0x2ebda4,
                                                &[self.c_word(out), node],
                                                &mut o,
                                            );
                                            self.c_put(out, ret);
                                            self.c_put(ctr, self.c_word(ctr).wrapping_add(1));
                                        }
                                        if self.c_word(bounds + 36) != u32::MAX {
                                            let directory = self.c_word(h.address(0x9e09ac));
                                            let entry = self.c_word(directory.wrapping_add(
                                                self.c_word(bounds + 36).wrapping_mul(4),
                                            ));
                                            if h.call(
                                                self,
                                                0x303a78,
                                                &[self.c_word(entry + 4), np, alloc],
                                                &mut o,
                                            ) == 0
                                            {
                                                return 0;
                                            }
                                        }
                                    } else {
                                        h.call(self, 0x196428, &[alloc, np], &mut o);
                                    }
                                    next = h.call(self, 0x2e68a4, &[bounds, rc, next], &mut o);
                                    bounds = o[0];
                                    if next == u32::MAX {
                                        break;
                                    }
                                }
                            }
                            b = self.c_word(b + 52);
                            if b == 0 {
                                break;
                            }
                        }
                        a = self.c_word(a + 52);
                        if a == 0 {
                            break;
                        }
                    }
                }
                end = end.wrapping_sub(1);
            }
            if h.call(
                self,
                0x3076c4,
                &[
                    tab, last, ctr, pos as u32, acc as u32, cfg, out, exp, h9, i10, alloc,
                ],
                &mut o,
            ) == 0
            {
                return 0;
            }
            pos = pos.wrapping_sub(1);
            ret = 1;
            acc = acc.wrapping_add(1);
        }
        ret
    }

    pub fn validate_stream(
        &mut self,
        h: &mut impl BootC5Host,
        p: u32,
        flag: u32,
        out1: u32,
        out2: u32,
    ) -> u32 {
        let mut o = [0; 4];
        self.c_store_byte(out2, 0);
        let rc = h.call(
            self,
            0x1b36fc,
            &[
                self.c_word(p + 76),
                0x3ad83a01,
                if flag != 0 { 2 } else { 0 },
                h.address(0x1a8684),
            ],
            &mut o,
        );
        if rc != 0 {
            return rc & 255;
        }
        let mut buf = [0u8; 28];
        for pass in 0..2 {
            let size = if pass == 0 { 28 } else { 12 };
            let vtable = self.c_word(p + 48);
            if h.read(
                self,
                self.c_word(vtable),
                self.c_word(p + 52),
                &mut buf[..size],
            ) != size as u32
            {
                if h.call(
                    self,
                    self.c_word(self.c_word(p + 48) + 8),
                    &[self.c_word(p + 52)],
                    &mut o,
                ) != 0
                {
                    return 6;
                }
                if h.call(self, 0x1b3b3c, &[self.c_word(p + 76), 0x3ad83a01], &mut o) & 255 != 2 {
                    return 4;
                }
                self.c_store_byte(out2, 1);
                return 0;
            }
            let w = |at: usize| u32::from_le_bytes(buf[at..at + 4].try_into().unwrap());
            let half = |at: usize| u16::from_le_bytes(buf[at..at + 2].try_into().unwrap()) as u32;
            if pass == 0 {
                self.c_put(p + 24, w(0));
                self.c_put(p + 28, w(4));
                self.c_put(p + 32, w(8));
                self.c_store_half(p + 36, half(12) as u16);
                self.c_store_half(p + 44, half(16) as u16);
                if self.c_byte(p + 38) != 0 {
                    if w(24) == 0 {
                        self.c_store_byte(out1, 2);
                    } else {
                        let sum = w(0)
                            .wrapping_add(w(4))
                            .wrapping_add(w(8))
                            .wrapping_add(half(12))
                            .wrapping_add(half(14))
                            .wrapping_add(half(16))
                            .wrapping_add(half(18))
                            .wrapping_add(w(20));
                        if sum != w(24) {
                            self.c_store_byte(out1, 1);
                            return 0;
                        }
                    }
                }
                if half(14) != 1 {
                    return 4;
                }
            } else {
                if w(0) != 0x10181018 || w(4) != half(18) {
                    self.c_store_byte(out1, 1);
                    return 0;
                }
                if w(8) != 0x10000 {
                    return 4;
                }
            }
        }
        let amount = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        let rc = h.call(
            self,
            0x1b3368,
            &[
                self.c_word(p + 68),
                self.c_word(p + 68),
                self.c_word(p + 48),
                self.c_word(p + 52),
                self.c_word(p + 64).wrapping_add(40),
                amount,
                self.c_byte(p + 38) as u32,
                self.c_half(p + 44) as u16 as u32,
            ],
            &mut o,
        );
        let obj = o[0];
        if rc != 0 {
            return rc & 255;
        }
        let va = self.c_word(obj + 12);
        let limit = h.call(
            self,
            self.c_word(va),
            &[obj.wrapping_sub(self.c_word(va + 24)).wrapping_add(12)],
            &mut o,
        );
        let vb = self.c_word(obj + 16);
        o = [0, 0, self.c_byte(out1) as u32, self.c_byte(out2) as u32];
        let rc = h.call(
            self,
            self.c_word(vb + 8),
            &[obj.wrapping_sub(self.c_word(vb + 24)).wrapping_add(16)],
            &mut o,
        );
        self.c_store_byte(out1, o[2] as u8);
        self.c_store_byte(out2, o[3] as u8);
        if rc != 0 {
            return rc & 255;
        }
        if self.c_byte(out2) != 0 || self.c_byte(out1) == 1 {
            return h.call(self, self.c_word(self.c_word(obj) + 16), &[obj], &mut o) & 255;
        }
        let (mut src, length) = (o[0], o[1]);
        if length < 4 || length == limit {
            return 7;
        }
        let count = self.c_word(src);
        src = src.wrapping_add(4);
        self.c_put(p + 40, count);
        let bytes = count.wrapping_mul(12);
        if bytes.wrapping_add(4) > length {
            return 4;
        }
        let dst = h.call(self, 0x1b2140, &[self.c_word(p + 68), bytes], &mut o);
        self.c_put(p + 60, dst);
        if o[0] & 255 != 0 {
            return o[0] & 255;
        }
        for i in 0..count {
            for j in 0..3 {
                let offset = i.wrapping_mul(12) + j * 4;
                self.c_put(
                    self.c_word(p + 60).wrapping_add(offset),
                    self.c_word(src.wrapping_add(offset)),
                );
            }
        }
        let rc = h.call(self, self.c_word(self.c_word(obj) + 16), &[obj], &mut o);
        if rc != 0 {
            return rc & 255;
        }
        self.c_store_byte(p + 72, 0);
        if h.call(self, 0x1b3b3c, &[self.c_word(p + 76), 0x3ad83a01], &mut o) & 255 == 2 {
            return 0;
        }
        let i10 = self.c_word(p + 16);
        let rc = h.call(
            self,
            self.c_word(i10 + 16),
            &[
                p.wrapping_sub(self.c_word(i10 + 104)).wrapping_add(16),
                0x3ad73e35,
            ],
            &mut o,
        );
        if rc > 1 {
            return 4;
        }
        if rc == 0 {
            h.call(
                self,
                0x1b3fe8,
                &[self.c_word(p + 76), 0x3ad83a01, 1],
                &mut o,
            );
            return 0;
        }
        let i14 = self.c_word(p + 20);
        let this = p.wrapping_sub(self.c_word(i14 + 92)).wrapping_add(20);
        let rc = h.call(self, self.c_word(i14 + 4), &[this, 0x3ad73e35, 0], &mut o);
        if rc != 0 {
            return rc & 255;
        }
        let obj2 = o[0];
        if o[2] & 255 != 0 {
            return 4;
        }
        let vd = self.c_word(obj2);
        let rc = h.call(
            self,
            self.c_word(vd + 4),
            &[obj2.wrapping_sub(self.c_word(vd + 24))],
            &mut o,
        );
        if rc != 0 {
            return rc & 255;
        }
        if o[2] & 255 != 0 {
            return 4;
        }
        let rc = h.call(self, 0x1b3ecc, &[self.c_word(p + 76), o[0]], &mut o);
        if rc != 0 {
            return rc & 255;
        }
        if h.call(self, 0x1b3b3c, &[self.c_word(p + 76), 0x3ad83a01], &mut o) & 255 == 0 {
            return 4;
        }
        h.call(self, self.c_word(i14 + 12), &[this, obj2], &mut o) & 255
    }
}
