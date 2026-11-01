

use super::arena::Arena;

pub fn is_leaf(f: u32) -> bool {
    matches!(
        f,
        0x195ae8
            | 0x195bec
            | 0x195c90
            | 0x195cd0
            | 0x195d30
            | 0x195d58
            | 0x195d90
            | 0x195df8
            | 0x195e38
            | 0x195aac
            | 0x195cf8
            | 0x195eb4
            | 0x19600c
            | 0x1765ac
            | 0x176624
            | 0x176644
            | 0x176fc0
            | 0x17497c
            | 0x1757a8
            | 0x19861c
            | 0x19862c
            | 0x196764
            | 0x1970a0
            | 0x1970ac
            | 0x197168
            | 0x198f74
            | 0x198728
            | 0x198a80
            | 0x198fa8
            | 0x1b3c00
            | 0x1baa14
            | 0x1b20a4
            | 0x1b3580
            | 0x1a8684
            | 0x1a9948
            | 0x17c7b8
            | 0x17c7d0
            | 0x17c800
            | 0x17c8c0
            | 0x17c8dc
            | 0x2ebc88
            | 0x304a48
    )
}

pub fn run(arena: &mut Arena<'_>, f: u32, args: &[u32]) -> u32 {
    match f {
        0x195ae8 => copy_bytes(arena, args[0], args[1], args[2]),
        0x195bec => fill_bytes(arena, args[0], args[1], args[2]),
        0x195c90 => append_string(arena, args[0], args[1]),
        0x195cd0 => copy_string(arena, args[0], args[1]),
        0x195d30 => find_byte(arena, args[0], args[1]),
        0x195d58 => compare_strings(arena, args[0], args[1]),
        0x195d90 => string_length(arena, args[0]),
        0x195df8 => compare_bounded(arena, args[0], args[1], args[2]),
        0x195e38 => copy_string_bounded(arena, args[0], args[1], args[2]),
        0x195aac => compare_bytes(arena, args[0], args[1], args[2]),
        0x195cf8 => find_last_byte(arena, args[0], args[1]),
        0x195eb4 => span_bytes(arena, args[0], args[1]),
        0x19600c => find_any_byte(arena, args[0], args[1]),
        0x1765ac => compare_wide_strings(arena, args[0], args[1]),
        0x176624 => wide_string_length(arena, args[0]),
        0x176644 => copy_wide_string_bounded(arena, args[0], args[1], args[2]),
        0x176fc0 => u32::from(compare_words(args[0], args[1], args[2], args[3])),
        0x17497c => byte_at_40(arena, args[0]),
        0x1757a8 => require_null(args[0], 0x8b70_2007),
        0x19861c => require_null(args[0], 0x8840_2007),
        0x19862c => require_null(args[0], 0x8840_2007),
        0x196764 => word_at_12(arena, args[0]),
        0x1970a0 => word_at_5(arena, args[0]),
        0x1970ac => chunk_at(arena, args[0], args[1], args[2]),
        0x197168 => chunk_last(arena, args[0], args[1]),
        0x198f74 => list_last(arena, args[0]),
        0x198728 => init_node(arena, args[0], args[1], args[2], args[3]),
        0x198a80 => list_neighbour(arena, args[0], args[1]),
        0x198fa8 => {
            arena.set_u8(args[0].wrapping_add(0x28), 1);
            args[0]
        }
        0x1b3c00 => 1,
        0x1baa14 => word_array_at(arena, args[0], args[1]),
        0x1b20a4 => copy_quad(arena, args[0], args[1], args[2]),
        0x1b3580 => u32::from(compare_indirect(arena, args[0], args[1])),
        0x1a8684 => small_count(args[0]),
        0x1a9948 => u32::from(compare_high_bytes(args[0], args[1])),
        0x17c7b8 => class_bits(arena, args[1], args[0], 3),
        0x17c7d0 => class_bits(arena, args[1], args[0], 4),
        0x17c800 => class_bits(arena, args[1], args[0], 8),
        0x17c8c0 => add_class_bit(arena, args[1], args[0], 1, 0x20),
        0x17c8dc => subtract_class_bit(arena, args[1], args[0], 2, 0x20),
        0x2ebc88 => {
            let index = args[1] as i32;
            if index < 0 || arena.u32(args[0].wrapping_add(8)) as i32 <= index {
                0
            } else {
                arena.u32(args[0].wrapping_add(4))
                    .wrapping_add(arena.u32(arena.u32(args[0])))
                    .wrapping_add((index as u32).wrapping_shl(5))
            }
        }
        0x304a48 => {
            let a = args[0] as i32;
            let b = args[1] as i32;
            let column = (a % 200).wrapping_mul(20);
            if (b / 20) & 1 == 0 {
                column.wrapping_add(b % 20) as u32
            } else {
                column.wrapping_sub(b % 20).wrapping_add(19) as u32
            }
        }
        _ => panic!("unknown leaf {f:x}"),
    }
}

pub fn copy_bytes(arena: &mut Arena<'_>, destination: u32, source: u32, count: u32) -> u32 {
    if count != 0 {
        arena.copy(destination, source, count);
    }
    destination
}

pub fn fill_bytes(arena: &mut Arena<'_>, destination: u32, value: u32, count: u32) -> u32 {
    let byte = value as u8;
    let mut i = 0;
    while i < count {
        arena.set_u8(destination.wrapping_add(i), byte);
        i = i.wrapping_add(1);
    }
    destination
}

pub fn append_string(arena: &mut Arena<'_>, destination: u32, source: u32) -> u32 {
    let mut d = destination;
    while arena.u8(d) != 0 {
        d = d.wrapping_add(1);
    }
    let mut s = source;
    loop {
        let byte = arena.u8(s);
        arena.set_u8(d, byte);
        if byte == 0 {
            break;
        }
        s = s.wrapping_add(1);
        d = d.wrapping_add(1);
    }
    destination
}

pub fn copy_string(arena: &mut Arena<'_>, destination: u32, source: u32) -> u32 {
    let mut d = destination;
    let mut s = source;
    loop {
        let byte = arena.u8(s);
        arena.set_u8(d, byte);
        if byte == 0 {
            return destination;
        }
        d = d.wrapping_add(1);
        s = s.wrapping_add(1);
    }
}

pub fn find_byte(arena: &mut Arena<'_>, string: u32, value: u32) -> u32 {
    let wanted = value as u8;
    let mut s = string;
    loop {
        let byte = arena.u8(s);
        if byte == 0 || byte == wanted {
            break;
        }
        s = s.wrapping_add(1);
    }
    if arena.u8(s) == wanted {
        s
    } else {
        0
    }
}

pub fn compare_strings(arena: &mut Arena<'_>, left: u32, right: u32) -> u32 {
    let mut a = left;
    let mut b = right;
    let mut difference;
    loop {
        difference = arena.u8(a) as i32 - arena.u8(b) as i32;
        if difference != 0 || arena.u8(b) == 0 {
            break;
        }
        a = a.wrapping_add(1);
        b = b.wrapping_add(1);
    }
    (difference.signum() as u32) & 0xffff_ffff
}

pub fn string_length(arena: &Arena<'_>, string: u32) -> u32 {
    let mut length = 0;
    while arena.u8(string.wrapping_add(length)) != 0 {
        length = length.wrapping_add(1);
    }
    length
}

pub fn compare_bounded(arena: &Arena<'_>, left: u32, right: u32, count: u32) -> u32 {
    if count == 0 {
        return 0;
    }
    let mut remaining = count;
    let mut a = left;
    let mut b = right;
    loop {
        remaining = remaining.wrapping_sub(1);
        if remaining == 0 || arena.u8(a) == 0 || arena.u8(a) != arena.u8(b) {
            break;
        }
        a = a.wrapping_add(1);
        b = b.wrapping_add(1);
    }
    (arena.u8(a) as u32).wrapping_sub(arena.u8(b) as u32)
}

pub fn copy_string_bounded(
    arena: &mut Arena<'_>,
    destination: u32,
    source: u32,
    count: u32,
) -> u32 {
    let mut d = destination;
    let mut s = source;
    let mut remaining = count;
    loop {
        if remaining == 0 {
            return destination;
        }
        let byte = arena.u8(s);
        s = s.wrapping_add(1);
        arena.set_u8(d, byte);
        d = d.wrapping_add(1);
        if byte == 0 {
            break;
        }
        remaining = remaining.wrapping_sub(1);
    }
    let mut padding = remaining.wrapping_sub(1);
    while padding != 0 {
        arena.set_u8(d, 0);
        d = d.wrapping_add(1);
        padding = padding.wrapping_sub(1);
    }
    destination
}

pub fn compare_bytes(arena: &Arena<'_>, left: u32, right: u32, count: u32) -> u32 {
    if count == 0 {
        return 0;
    }
    let mut remaining = count;
    let mut a = left;
    let mut b = right;
    loop {
        remaining = remaining.wrapping_sub(1);
        if remaining == 0 || arena.u8(a) != arena.u8(b) {
            break;
        }
        a = a.wrapping_add(1);
        b = b.wrapping_add(1);
    }
    (arena.u8(a) as u32).wrapping_sub(arena.u8(b) as u32)
}

pub fn find_last_byte(arena: &Arena<'_>, string: u32, value: u32) -> u32 {
    let wanted = value as u8;
    let mut end = string;
    while arena.u8(end) != 0 {
        end = end.wrapping_add(1);
    }
    let mut p = end;
    loop {
        if arena.u8(p) == wanted {
            return p;
        }
        if p == string {
            return 0;
        }
        p = p.wrapping_sub(1);
    }
}

pub fn span_bytes(arena: &Arena<'_>, string: u32, accept: u32) -> u32 {
    let mut table = [0u8; 32];
    let mut a = accept;
    loop {
        let byte = arena.u8(a);
        if byte == 0 {
            break;
        }
        table[(byte >> 3) as usize] |= 1 << (byte & 7);
        a = a.wrapping_add(1);
    }
    let mut s = string;
    let mut length = 0;
    loop {
        let byte = arena.u8(s);
        if byte == 0 || table[(byte >> 3) as usize] & (1 << (byte & 7)) == 0 {
            return length;
        }
        length = length.wrapping_add(1);
        s = s.wrapping_add(1);
    }
}

pub fn find_any_byte(arena: &Arena<'_>, string: u32, set: u32) -> u32 {
    let mut table = [0u8; 32];
    let mut a = set;
    loop {
        let byte = arena.u8(a);
        if byte == 0 {
            break;
        }
        table[(byte >> 3) as usize] |= 1 << (byte & 7);
        a = a.wrapping_add(1);
    }
    let mut s = string;
    loop {
        let byte = arena.u8(s);
        if byte == 0 {
            return 0;
        }
        if table[(byte >> 3) as usize] & (1 << (byte & 7)) != 0 {
            return s;
        }
        s = s.wrapping_add(1);
    }
}

pub fn compare_wide_strings(arena: &Arena<'_>, left: u32, right: u32) -> u32 {
    let mut a = left;
    let mut b = right;
    let mut current;
    loop {
        current = arena.u16(a);
        if current == 0 {
            break;
        }
        if current != arena.u16(b) {
            break;
        }
        a = a.wrapping_add(2);
        b = b.wrapping_add(2);
    }
    (current as u32).wrapping_sub(arena.u16(b) as u32)
}

pub fn wide_string_length(arena: &Arena<'_>, string: u32) -> u32 {
    let mut length: u32 = 0;
    while arena.u16(string.wrapping_add(length.wrapping_mul(2))) != 0 {
        length = length.wrapping_add(1);
    }
    length
}

pub fn copy_wide_string_bounded(
    arena: &mut Arena<'_>,
    destination: u32,
    source: u32,
    count: u32,
) -> u32 {
    let mut d = destination;
    let mut s = source;
    let mut remaining = count;
    let mut current = 0u16;
    loop {
        if remaining == 0 {
            return destination;
        }
        current = arena.u16(s);
        s = s.wrapping_add(2);
        if current != 0 {
            remaining = remaining.wrapping_sub(1);
        }
        arena.set_u16(d, current);
        d = d.wrapping_add(2);
        if current == 0 {
            break;
        }
    }
    let mut padding = remaining.wrapping_sub(1);
    while padding != 0 {
        arena.set_u16(d, current);
        d = d.wrapping_add(2);
        padding = padding.wrapping_sub(1);
    }
    destination
}

pub fn compare_words(low: u32, high: u32, other_low: u32, other_high: u32) -> bool {
    high == other_high && low == other_low
}

pub fn byte_at_40(arena: &Arena<'_>, object: u32) -> u32 {
    if object == 0 {
        0
    } else {
        arena.u8(object.wrapping_add(0x40)) as u32
    }
}

pub fn require_null(pointer: u32, error: u32) -> u32 {
    if pointer != 0 {
        0
    } else {
        error
    }
}

pub fn word_at_12(arena: &Arena<'_>, object: u32) -> u32 {
    if object == 0 {
        0
    } else {
        arena.u32(object.wrapping_add(0x30))
    }
}

pub fn word_at_5(arena: &Arena<'_>, object: u32) -> u32 {
    if object == 0 {
        0
    } else {
        arena.u32(object.wrapping_add(0x14))
    }
}

pub fn chunk_at(arena: &mut Arena<'_>, chunks: u32, index: u32, out: u32) -> u32 {
    if chunks == 0 || out == 0 || index >= arena.u32(chunks.wrapping_add(0x14)) {
        return 0;
    }
    let base = arena.u32(chunks);
    let stride = arena.u16(chunks.wrapping_add(4)) as u32;
    arena.set_u32(out, base.wrapping_add(index.wrapping_mul(stride)));
    1
}

pub fn chunk_last(arena: &mut Arena<'_>, chunks: u32, out: u32) -> u32 {
    if chunks == 0 || out == 0 {
        return 0;
    }
    let count = arena.u32(chunks.wrapping_add(0x14));
    if count == 0 {
        return 0;
    }
    let base = arena.u32(chunks);
    let stride = arena.u16(chunks.wrapping_add(4)) as u32;
    arena.set_u32(
        out,
        base.wrapping_add(count.wrapping_sub(1).wrapping_mul(stride)),
    );
    1
}

pub fn list_last(arena: &Arena<'_>, head: u32) -> u32 {
    let mut p = arena.u32(head);
    if p != head {
        let mut q = arena.u32(p);
        if q != head {
            loop {
                p = q;
                q = arena.u32(q);
                if q == head {
                    break;
                }
            }
        }
    }
    if p == head {
        0
    } else {
        p
    }
}

pub fn init_node(arena: &mut Arena<'_>, node: u32, first: u32, second: u32, third: u32) -> u32 {
    arena.set_u32(node.wrapping_add(0x18), 0);
    arena.set_u8(node.wrapping_add(0x0c), 1);
    arena.set_u8(node.wrapping_add(0x28), 0);
    arena.set_u32(node.wrapping_add(0x24), second);
    arena.set_u32(node, node);
    arena.set_u32(node.wrapping_add(0x1c), first);
    arena.set_u32(node.wrapping_add(0x20), third);
    arena.set_u32(node.wrapping_add(4), node);
    arena.set_u32(node.wrapping_add(8), node);
    node
}

pub fn list_neighbour(arena: &Arena<'_>, sentinel: u32, node: u32) -> u32 {
    if arena.u32(node.wrapping_add(4)) != sentinel {
        let mut previous = arena.u32(node.wrapping_add(4));
        let mut next = arena.u32(previous);
        while next != sentinel {
            previous = next;
            next = arena.u32(next);
        }
        return previous;
    }
    let mut current = arena.u32(node.wrapping_add(8));
    let mut last = node;
    while current != sentinel && arena.u32(current.wrapping_add(4)) == last {
        last = current;
        current = arena.u32(current.wrapping_add(8));
    }
    if current == sentinel {
        0
    } else {
        current
    }
}

pub fn word_array_at(arena: &Arena<'_>, table: u32, index: u32) -> u32 {
    let base = arena.u32(table.wrapping_add(4));
    arena.u32(base.wrapping_add(index.wrapping_mul(4)))
}

pub fn copy_quad(arena: &mut Arena<'_>, destination: u32, source: u32, extra: u32) -> u32 {
    let mut i = 0;
    while i < 4 {
        let word = arena.u32(source.wrapping_add(i * 4));
        arena.set_u32(destination.wrapping_add(i * 4), word);
        i += 1;
    }
    arena.set_u32(destination.wrapping_add(0x10), extra);
    0
}

pub fn compare_indirect(arena: &Arena<'_>, left: u32, right: u32) -> bool {
    let a = arena.u32(arena.u32(left));
    let b = arena.u32(arena.u32(right));
    a != b
}

pub fn small_count(value: u32) -> u32 {
    let value = value as i32;
    if value < 0 {
        return 0;
    }
    if value <= 2 {
        1
    } else {
        0
    }
}

pub fn compare_high_bytes(left: u32, right: u32) -> bool {
    let left_high = left >> 16;
    let right_high = right >> 16;
    let left_third = left.wrapping_shl(16) >> 24;
    let right_third = right.wrapping_shl(16) >> 24;
    left_high == right_high && left_third <= right_third
}

pub fn class_bits(arena: &Arena<'_>, table: u32, character: u32, mask: u32) -> u32 {
    let entry = arena.u16(table.wrapping_add((character & 0xff) * 2)) as u32;
    entry & mask
}

pub fn add_class_bit(arena: &Arena<'_>, table: u32, character: u32, bit: u32, delta: u32) -> u32 {
    let entry = arena.u16(table.wrapping_add((character & 0xff) * 2)) as u32;
    if entry & bit != 0 {
        character.wrapping_add(delta)
    } else {
        character
    }
}

pub fn subtract_class_bit(
    arena: &Arena<'_>,
    table: u32,
    character: u32,
    bit: u32,
    delta: u32,
) -> u32 {
    let entry = arena.u16(table.wrapping_add((character & 0xff) * 2)) as u32;
    if entry & bit != 0 {
        character.wrapping_sub(delta)
    } else {
        character
    }
}
