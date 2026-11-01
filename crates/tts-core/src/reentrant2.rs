

use crate::boot::{Calls, Memory};

#[derive(Clone, Copy, Debug)]
pub enum LiteralM { OwnerCategory, OwnerRelease, PcreCategory, PcreInitBegin, PcreInitEnd, PcreDeinitBegin, PcreDeinitEnd }
pub trait HooksM {

    fn literal_m(&self, _literal: LiteralM) -> u32 { u32::MAX }

    fn call_m(&mut self, address: u32, args: &[u32], mem: &mut Memory<'_>) -> u32;
}
impl<T: Calls + ?Sized> HooksM for T {
    fn call_m(&mut self, address: u32, args: &[u32], mem: &mut Memory<'_>) -> u32 {
        self.call(mem, address, args)
    }
}
fn half(m: &Memory<'_>, p: u32) -> u32 {
    u16::from_le_bytes(m.bytes[m.offset(p)..m.offset(p)+2].try_into().unwrap()) as u32
}
fn copy(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) {
    let d = dst.wrapping_sub(m.base);
    let s = src.wrapping_sub(m.base);
    crate::normalization::memcpy_at(m.bytes, d, s, n);
}

pub fn clone_table_m(m: &mut Memory<'_>, tbl: u32, out: u32, host: &mut impl HooksM) -> u32 {
    if tbl == 0 || out == 0 { return 0; }
    let clone = host.call_m(0x1963b8, &[m.word(tbl+28), 1, 32], m);
    m.put_word(out, clone);
    if clone == 0 { return 0; }
    copy(m, clone, tbl, 32);
    let size = m.word(clone+16).wrapping_mul(half(m,clone+4));
    let buf = host.call_m(0x196394, &[m.word(clone+28), size], m);
    let t = m.word(out);
    m.put_word(t, buf);
    let mut result = 1;
    let mut initialized = 0u32;
    if buf == 0 {
        result = 0;
    } else {
        copy(m, buf, m.word(tbl), m.word(t+20).wrapping_mul(half(m,t+4)));
        let t = m.word(out);
        if m.word(t+8) == 0 { return result; }
        let mut i = 0u32;
        loop {
            let t = m.word(out);
            if m.word(t+20) <= i { return result; }
            let target = m.word(t).wrapping_add(i.wrapping_mul(half(m,t+4)));
            let source = m.word(tbl).wrapping_add(i.wrapping_mul(half(m,tbl+4)));
            result = host.call_m(m.word(t+8), &[source, m.word(t+28), target], m);
            if result != 1 { break; }
            initialized += 1;
            i += 1;
        }
        if result != 0 { return result; }
    }
    let t = m.word(out);
    if t == 0 { return result; }
    if m.word(t) != 0 {
        if m.word(t+12) != 0 {
            for i in 0..initialized {
                let t = m.word(out);
                let row = m.word(t).wrapping_add(i.wrapping_mul(half(m,t+4)));
                host.call_m(m.word(t+12), &[row, m.word(t+28)], m);
            }
        }
        let t = m.word(out);
        host.call_m(0x196428, &[m.word(t+28),m.word(t)], m);
    }
    let t = m.word(out);
    host.call_m(0x196428, &[m.word(t+28),t], m);
    m.put_word(out,0);
    result
}

pub fn destroy_array_m(m: &mut Memory<'_>, array: u32, host: &mut impl HooksM) -> u32 {
    if array == 0 { return 0; }
    if m.word(array) != 0 {
        if m.word(array+12) != 0 {
            let mut i = 0u32;
            while i < m.word(array+20) {
                let row = m.word(array).wrapping_add(i.wrapping_mul(half(m,array+4)));
                host.call_m(m.word(array+12), &[row, m.word(array+28)], m);
                i = i.wrapping_add(1);
            }
        }
        host.call_m(0x196428, &[m.word(array+28),m.word(array)], m);
    }
    host.call_m(0x196428, &[m.word(array+28),array], m);
    1
}

pub fn call_method_m(m: &mut Memory<'_>, heap: u32, count: u32, size: u32, host: &mut impl HooksM) -> u32 {
    if heap == 0 || count == 0 || size == 0 { return 0; }
    host.call_m(m.word(m.word(heap)+4), &[m.word(heap+4),count,size], m)
}

pub trait StateLockM {
    fn acquire_m(&mut self, mem: &mut [u8]) -> u32;
    fn release_m(&mut self, mem: &mut [u8]) -> u32;
}
impl<T: crate::frontend::StateLock + ?Sized> StateLockM for T {
    fn acquire_m(&mut self, _mem: &mut [u8]) -> u32 { self.acquire() }
    fn release_m(&mut self, _mem: &mut [u8]) -> u32 { self.release() }
}

pub fn find_node_m(
    mem: &mut [u8],
    tree: u32,
    head: u32,
    _flag: u8,
    key: u32,
    mut cmp: impl FnMut(u32, u32, &mut [u8]) -> i32,
) -> Option<u32> {
    fn word(mem: &mut [u8], at: u32) -> u32 {
        u32::from_le_bytes(mem[at as usize..at as usize + 4].try_into().unwrap())
    }
    let mut node = head;
    loop {
        if node == tree {
            return None;
        }
        let order = cmp(key, word(mem, node.wrapping_add(16)), mem);
        if order < 0 {
            node = word(mem, node);
        } else if order > 0 {
            node = word(mem, node.wrapping_add(4));
        } else {
            break;
        }
    }

    if mem[tree.wrapping_add(40) as usize] == 0 {
        return Some(node);
    }
    loop {
        let last = node;
        node = word(mem, last);
        while node != tree && cmp(key, word(mem, node.wrapping_add(16)), mem) != 0 {
            node = word(mem, node.wrapping_add(4));
        }
        if node == tree {
            return Some(last);
        }
    }
}

pub fn lookup_value_m(
    mem: &mut [u8],
    base: Option<u32>,
    key: u32,
    out: Option<&mut u32>,
    cmp: impl FnMut(u32, u32, &mut [u8]) -> i32,
) -> u32 {
    let (Some(base), Some(out)) = (base, out) else {
        return 0x89002007;
    };
    if key == 0 {
        return 0x89002007;
    }
    let tree = base.wrapping_add(24);
    let head = u32::from_le_bytes(mem[tree as usize..tree as usize + 4].try_into().unwrap());
    match find_node_m(
        mem,
        tree,
        head,
        mem[tree.wrapping_add(40) as usize],
        key,
        cmp,
    ) {
        Some(node) => {
            let at = node.wrapping_add(20) as usize;
            *out = u32::from_le_bytes(mem[at..at + 4].try_into().unwrap());
            0
        }
        None => 0x89002014,
    }
}

pub fn lookup_param_m(
    mem: &mut [u8],
    obj: Option<u32>,
    key: u32,
    p1: Option<&mut u32>,
    p2: Option<&mut u32>,
    lock: &mut impl StateLockM,
    cmp: impl FnMut(u32, u32, &mut [u8]) -> i32,
) -> u32 {
    let (Some(obj), Some(p1)) = (obj, p1) else {
        return 0x8b702007;
    };
    if key == 0 {
        return 0x8b702007;
    }
    let mut p2 = p2;
    *p1 = 0;
    if let Some(q) = p2.as_mut() {
        **q = 0;
    }
    let mut result = lock.acquire_m(mem);
    if result >= 0x80000000 {
        return result;
    }
    let mut ent = 0u32;
    let rc = lookup_value_m(
        mem,
        nz(mem_word(mem, obj + 8)),
        key,
        Some(&mut ent),
        cmp,
    );
    if rc >= 0x80000000 {
        result = (rc & 0x1fff) | 0x8b702000;
    } else {
        result = 0;
        if ent != 0 {
            let e1 = mem_word(mem, ent + 4);
            if e1 != 0 {
                *p1 = mem_word(mem, ent);
                if let Some(q) = p2.as_mut() {
                    **q = e1;
                }
            } else {
                *p1 = 0;
                if let Some(q) = p2.as_mut() {
                    **q = 0;
                }
            }
        }
    }
    let t = lock.release_m(mem);
    if t >= 0x80000000 && result < 0x80000000 {
        result = t;
    }
    result
}

pub fn get_param_uint_m(
    mem: &mut [u8],
    a: Option<u32>,
    b: u32,
    out: u32,
    lock: &mut impl StateLockM,
    cmp: impl FnMut(u32, u32, &mut [u8]) -> i32,
) -> u32 {
    let mut v0 = 0u32;
    let mut v1 = 0u32;
    let err = lookup_param_m(mem, a, b, Some(&mut v0), Some(&mut v1), lock, cmp);
    if err < 0x80000000 && v1 > 1 {
        let n = parse_uint(mem, nz(v0)) as u32;
        mem_store(mem, out, n);
    } else {
        mem_store(mem, out, 0);
    }
    err
}

pub fn get_param_int_m(
    table: &[u16; 256],
    mem: &mut [u8],
    a: Option<u32>,
    b: u32,
    out: u32,
    lock: &mut impl StateLockM,
    cmp: impl FnMut(u32, u32, &mut [u8]) -> i32,
) -> u32 {
    let mut v0 = 0u32;
    let mut v1 = 0u32;
    let err = lookup_param_m(mem, a, b, Some(&mut v0), Some(&mut v1), lock, cmp);
    if err < 0x80000000 && v1 > 1 {
        let n = parse_int(table, mem, v0) as u32;
        mem_store(mem, out, n);
    } else {
        mem_store(mem, out, 0);
    }
    err
}

pub fn fetch_param_m(
    mem: &mut [u8],
    a: Option<u32>,
    b: u32,
    c: Option<&mut u32>,
    lock: &mut impl StateLockM,
    cmp: impl FnMut(u32, u32, &mut [u8]) -> i32,
) -> u32 {
    lookup_param_m(mem, a, b, c, None, lock, cmp)
}

fn mem_word(mem: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(mem[p as usize..p as usize+4].try_into().unwrap())
}
fn mem_store(mem: &mut [u8], p: u32, v: u32) {
    mem[p as usize..p as usize+4].copy_from_slice(&v.to_le_bytes());
}
fn nz(p: u32) -> Option<u32> { if p == 0 { None } else { Some(p) } }
use crate::frontend::{parse_uint,parse_int};

pub trait BeforeChangeM {
    fn before_m(&mut self, method:u32, context:u32, key:u32, data:u32, allow:&mut u8, mem:&mut [u8])->u32;
}
impl<T: crate::frontend::BeforeChange + ?Sized> BeforeChangeM for T {
    fn before_m(&mut self,_method:u32,c:u32,k:u32,d:u32,a:&mut u8,_:&mut [u8])->u32 { self.before(c,k,d,a) }
}

pub trait ElementCallbackM {
    fn call_element_m(&mut self,method:u32,a:u32,q:u32,r:u32,mem:&mut [u8])->u32;
}
impl<T: crate::frontend::ElementCallback + ?Sized> ElementCallbackM for T {
    fn call_element_m(&mut self,_method:u32,a:u32,q:u32,r:u32,_:&mut [u8])->u32 { self.call_element(a,q,r) }
}
use crate::frontend::{HeapVtableM,TreeOpsM,heap_alloc_m,heap_release_m,heap_realloc_m,insert_guarded_m,
    ElementTable,element_at,word_at5,compare_bytes,render_unsigned,strlen_bytes};
fn mem_half(mem:&[u8],p:u32)->u16 { u16::from_le_bytes(mem[p as usize..p as usize+2].try_into().unwrap()) }

pub fn before_change_m(
    mem: &mut [u8],
    params: u32,
    v: u32,
    key: u32,
    data: u32,
    before: &mut impl BeforeChangeM,
) -> u32 {
    if params == 0 || key == 0 || data == 0 || v == 0 {
        return 0;
    }
    let mut rc = 0u32;
    let mut allow = 1u8;
    for pass in [1u32, 2] {
        let list = if pass == 1 {
            mem_word(mem, v + 8)
        } else {
            mem_word(mem, params + 12)
        };
        if list != 0 {
            let count = mem_word(mem, list + 20);
            if count != 0 {
                let mut i = 0u32;
                while rc < 0x80000000 && i < count {
                    let base = mem_word(mem, list);
                    let size = mem_half(mem, list + 4);
                    if i >= mem_word(mem, list + 20) {
                        return 0x8b702100;
                    }
                    let c = base.wrapping_add(i.wrapping_mul(size as u32));
                    if mem_word(mem, c + 4) != 0 {
                        rc = before.before_m(mem_word(mem, c+4), mem_word(mem, c), key, data, &mut allow, mem);
                        if rc < 0x80000000 && allow == 0 {
                            rc = 0x8b70200f;
                            break;
                        }
                    }
                    i = i.wrapping_add(1);
                }
            }
        }
    }
    rc
}

pub fn store_sized_m(
    mem: &mut [u8],
    slot_at: Option<u32>,
    src_off: u32,
    n: u32,
    heap: u32,
    vt: &mut impl HeapVtableM,
) -> u32 {
    let Some(slot_at) = slot_at else {
        return 0x8b702007;
    };
    if n == 0 {
        if mem_word(mem, slot_at) != 0 && mem_word(mem, slot_at + 4) != 0 {
            heap_release_m(mem, nz(heap), nz(mem_word(mem, slot_at)), vt);
        }
        mem_store(mem, slot_at + 4, 0);
        mem_store(mem, slot_at, src_off);
    } else {
        let size = mem_word(mem, slot_at + 4);
        if size == 0 {
            let fresh = heap_alloc_m(mem, nz(heap), n, vt);
            mem_store(mem, slot_at, fresh);
            if fresh == 0 {
                return 0x8b70200a;
            }
        } else if size < n {
            let grown = heap_realloc_m(mem, nz(heap), mem_word(mem, slot_at), n, vt);
            if grown == 0 {
                return 0x8b70200a;
            }
            mem_store(mem, slot_at, grown);
        }
        let data = mem_word(mem, slot_at);
        let mut i = 0u32;
        while i < n {
            let b = mem[src_off.wrapping_add(i) as usize];
            mem[data.wrapping_add(i) as usize] = b;
            i = i.wrapping_add(1);
        }
        mem_store(mem, slot_at + 4, n);
    }
    0
}

pub fn run_element_callbacks_m(
    mem: &mut [u8],
    p: Option<u32>,
    q: u32,
    o: Option<u32>,
    r: u32,
    cb: &mut impl ElementCallbackM,
) -> u32 {
    let (Some(p), Some(o)) = (p, o) else {
        return 0;
    };
    if q == 0 || r == 0 {
        return 0;
    }
    for pass in [1u32, 2] {
        let l = if pass == 1 {
            mem_word(mem, o + 8)
        } else {
            mem_word(mem, p + 12)
        };
        if l != 0 {
            let mut words = [0u32; 6];
            for (i, slot) in words.iter_mut().enumerate() {
                *slot = mem_word(mem, l.wrapping_add((i as u32).wrapping_mul(4)));
            }
            let n = word_at5(Some(&words));
            if n != 0 {
                let mut i = 0u32;
                while i < n {
                    let table = ElementTable {
                        data: &[],
                        elem_size: mem_half(mem, l + 4),
                        count: mem_word(mem, l + 20),
                    };
                    let mut h = 0u32;
                    if element_at(Some(&table), i, Some(&mut h)) == 0 {
                        return 0x8b702100;
                    }
                    let e = mem_word(mem, l).wrapping_add(h);
                    let rc = cb.call_element_m(mem_word(mem, e+8), mem_word(mem, e), q, r, mem);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    i = i.wrapping_add(1);
                }
            }
        }
    }
    0
}

pub fn update_attr_m(
    mem: &mut [u8],
    obj: u32,
    attr: u32,
    data: u32,
    size: u32,
    upd: u32,
    scratch: u32,
    heap_vt: &mut impl HeapVtableM,
    ops: &mut impl TreeOpsM,
    mut cmp: impl FnMut(u32, u32, &mut [u8]) -> i32,
) -> u32 {
    mem_store(mem, scratch, data);
    mem_store(mem, scratch + 4, size);
    mem_store(mem, scratch + 8, 0);
    let mut r = 0u32;
    let mut i = 0u32;
    loop {
        let dict = mem_word(mem, obj + 16);
        if dict == 0 {
            break;
        }
        if i >= mem_word(mem, dict + 20) {
            break;
        }
        let ent = mem_word(mem, dict).wrapping_add(i.wrapping_mul(mem_half(mem, dict + 4) as u32));
        r = insert_guarded_m(mem, nz(mem_word(mem, ent + 4)), attr, scratch, heap_vt, ops);
        if r & 0x1fff == 2 {
            if upd == 0 {
                r = 0;
            } else {
                let mut slot = 0u32;
                let ent1 = mem_word(mem, ent + 4);
                r = lookup_value_m(mem, nz(ent1), attr, Some(&mut slot), &mut cmp);
                if r >= 0x80000000 {
                    r = (r & 0x1fff) | 0x8b702000;
                } else {
                    let pi = slot;
                    let alloc = mem_word(mem, mem_word(mem, ent));
                    if pi == 0 {
                        r = 0x8b702007;
                        break;
                    }
                    if size == 0 {
                        if mem_word(mem, pi) != 0 && mem_word(mem, pi + 4) != 0 {
                            heap_release_m(mem, nz(alloc), nz(mem_word(mem, pi)), heap_vt);
                        }
                        mem_store(mem, pi, data);
                        mem_store(mem, pi + 4, 0);
                        r = 0;
                    } else if mem_word(mem, pi + 4) == 0 {
                        let fresh = heap_alloc_m(mem, nz(alloc), size, heap_vt);
                        mem_store(mem, pi, fresh);
                        if fresh == 0 {
                            r = 0x8b70200a;
                            break;
                        }
                        let mut k = 0u32;
                        while k < size {
                            let b = mem[data.wrapping_add(k) as usize];
                            mem[mem_word(mem, pi).wrapping_add(k) as usize] = b;
                            k = k.wrapping_add(1);
                        }
                        mem_store(mem, pi + 4, size);
                        r = 0;
                    } else if mem_word(mem, pi + 4) < size {
                        let m = heap_realloc_m(mem, nz(alloc), mem_word(mem, pi), size, heap_vt);
                        if m == 0 {
                            r = 0x8b70200a;
                            break;
                        }
                        mem_store(mem, pi, m);
                        let mut k = 0u32;
                        while k < size {
                            let b = mem[data.wrapping_add(k) as usize];
                            mem[mem_word(mem, pi).wrapping_add(k) as usize] = b;
                            k = k.wrapping_add(1);
                        }
                        mem_store(mem, pi + 4, size);
                        r = 0;
                    } else {
                        let mut k = 0u32;
                        while k < size {
                            let b = mem[data.wrapping_add(k) as usize];
                            mem[mem_word(mem, pi).wrapping_add(k) as usize] = b;
                            k = k.wrapping_add(1);
                        }
                        mem_store(mem, pi + 4, size);
                        r = 0;
                    }
                }
            }
        }
        i = i.wrapping_add(1);
        if r >= 0x80000000 {
            break;
        }
    }
    r
}

pub fn set_param_m(
    mem: &mut [u8],
    params_at: Option<u32>,
    key: u32,
    data: u32,
    size: u32,
    update: u32,
    scratch: u32,
    lock: &mut impl StateLockM,
    heap_vt: &mut impl HeapVtableM,
    ops: &mut impl TreeOpsM,
    mut cmp: impl FnMut(u32, u32, &mut [u8]) -> i32,
    before: impl BeforeChangeM,
    elem_cb: impl ElementCallbackM,
) -> u32 {
    const MISSING: u32 = 0x8b702007;
    let Some(params_at) = params_at else {
        return MISSING;
    };
    if key == 0 || data == 0 {
        return MISSING;
    }
    let mut result = lock.acquire_m(mem);
    if result >= 0x80000000 {
        return result;
    }
    let mut before = before;
    let mut elem_cb = elem_cb;
    let mut value = 0u32;
    let rc = lookup_value_m(mem, nz(mem_word(mem, params_at + 8)), key, Some(&mut value), &mut cmp);
    if rc < 0x80000000 {
        let (old_data, old_size) = if value == 0 {
            (0, 0)
        } else {
            let n = mem_word(mem, value + 4);
            if n == 0 {
                (0, 0)
            } else {
                (mem_word(mem, value), n)
            }
        };
        let mut need = old_size != size;
        if !need {
            let end = old_data.wrapping_add(size) as usize;
            let dend = data.wrapping_add(size) as usize;
            need = compare_bytes(
                &mem[old_data as usize..end],
                &mem[data as usize..dend],
                size,
            ) != 0;
        }
        if need {
            result = before_change_m(mem, params_at, value, key, data, &mut before);
            if result < 0x80000000 {
                let (upd_data, upd_size) = if update != 0 {
                    (data, size)
                } else {
                    (old_data, old_size)
                };
                result = update_attr_m(
                    mem,
                    params_at,
                    key,
                    upd_data,
                    upd_size,
                    update,
                    scratch.wrapping_add(12),
                    &mut *heap_vt,
                    &mut *ops,
                    &mut cmp,
                );
                if result < 0x80000000 {
                    result = store_sized_m(mem, nz(value), data, size, mem_word(mem, params_at), &mut *heap_vt);
                    if result < 0x80000000 {
                        result = run_element_callbacks_m(
                            mem,
                            Some(params_at),
                            key,
                            nz(value),
                            data,
                            &mut elem_cb,
                        );
                    }
                }
            }
        }
    } else {
        mem_store(mem, scratch, data);
        mem_store(mem, scratch + 4, size);
        mem_store(mem, scratch + 8, 0);
        result = before_change_m(mem, params_at, scratch, key, data, &mut before);
        if result < 0x80000000 {
            let (upd_data, upd_size) = if update != 0 { (data, size) } else { (0, 0) };
            result = update_attr_m(
                mem,
                params_at,
                key,
                upd_data,
                upd_size,
                update,
                scratch.wrapping_add(12),
                &mut *heap_vt,
                &mut *ops,
                &mut cmp,
            );
            if result < 0x80000000 {
                result = insert_guarded_m(mem, nz(mem_word(mem, params_at + 8)), key, scratch, &mut *heap_vt, &mut *ops);
                if result >= 0x80000000 {
                    result = (result & 0x1fff) | 0x8b702000;
                } else {
                    result = run_element_callbacks_m(
                        mem,
                        Some(params_at),
                        key,
                        Some(scratch),
                        data,
                        &mut elem_cb,
                    );
                    if result >= 0x80000000 {
                        result = (result & 0x1fff) | 0x8b702000;
                    }
                }
            }
        }
    }
    let rc = lock.release_m(mem);
    if rc >= 0x80000000 && result < 0x80000000 {
        result = rc;
    }
    result
}

pub fn set_param_uint_m(
    mem: &mut [u8],
    params_at: Option<u32>,
    key: u32,
    c: u32,
    update: u32,
    scratch: u32,
    lock: &mut impl StateLockM,
    heap_vt: &mut impl HeapVtableM,
    ops: &mut impl TreeOpsM,
    cmp: impl FnMut(u32, u32, &mut [u8]) -> i32,
    before: impl BeforeChangeM,
    elem_cb: impl ElementCallbackM,
) -> u32 {
    let at = scratch as usize;
    if render_unsigned(c, &mut mem[at..at + 16], 10).is_err() {
        mem[at] = 0;
    }
    let n = strlen_bytes(&mem[at..]).wrapping_add(1);
    set_param_m(
        mem,
        params_at,
        key,
        scratch,
        n,
        update,
        scratch.wrapping_add(16),
        lock,
        heap_vt,
        ops,
        cmp,
        before,
        elem_cb,
    )
}

pub fn close_owner_pair_m(m:&mut Memory<'_>,p:u32,h:&mut impl HooksM)->u32 {
    if m.byte(p+4)!=0 {
        for off in [8,12,16,20,24] { m.put_word(p+off,0); }
        let i=m.offset(p+4);m.bytes[i]=0;return 0;
    }
    let mut rc=0;
    if m.word(p+16)!=0 {
        let mut err=0;
        let owner=m.word(p);
        h.call_m(0x1738e8,&[m.word(owner+16),h.literal_m(LiteralM::OwnerCategory),3,0,
            h.literal_m(LiteralM::OwnerRelease),m.word(p+16),m.word(p+20),m.word(p+24)],m);
        if m.byte(p+4)==0 {
            if m.word(p+8)!=0 {err=h.call_m(0x179208,&[m.word(owner+12),m.word(p+8),m.word(p+16)],m);}
            else if m.word(p+12)!=0 {err=h.call_m(0x17331c,&[m.word(p+12),m.word(p+16)],m);}
        }
        if m.word(p+8)!=0 || m.word(p+12)!=0 {
            m.put_word(p+16,0);m.put_word(p+24,0);m.put_word(p+20,0);
        }
        m.put_word(p+16,0);rc=err;
        if rc>=0x80000000 {return err;}
    }
    if m.word(p+8)!=0 {
        rc=h.call_m(0x179f3c,&[m.word(m.word(p)+12),m.word(p+8)],m);
        m.put_word(p+8,0);
    }
    if rc>=0x80000000 || m.word(p+12)==0 {return rc;}
    rc=h.call_m(0x1730fc,&[m.word(p+12)],m);m.put_word(p+12,0);rc
}
fn pcre_log(m:&mut Memory<'_>,p:u32,which:LiteralM,status:Option<u32>,h:&mut impl HooksM) {
    let logger=m.word(m.word(p)+16);
    let args=[logger,h.literal_m(LiteralM::PcreCategory),4,0,h.literal_m(which),status.unwrap_or(0)];
    h.call_m(0x1738e8,&args[..if status.is_some(){6}else{5}],m);
}

pub fn pcre_deinit_m(m:&mut Memory<'_>,p:u32,key:u32,h:&mut impl HooksM)->u32 {
    let status=h.call_m(0x176f94,&[p,key,0xdd57,24],m);
    if status>=0x80000000 {return status;}
    pcre_log(m,p,LiteralM::PcreDeinitBegin,None,h);
    let mut i=1u32;
    while i<half(m,p+8) {
        free_pcre_entry(m,p,i,h);i=i.wrapping_add(1)&0xffff;
    }
    m.put_half(p+8,1);
    for off in [16,18,20] {m.put_half(p+off,0);}
    if m.word(p+4)!=0 {
        if m.word(m.word(p+4))!=0 {free_pcre_entry(m,p,0,h);}
        h.call_m(0x196428,&[m.word(m.word(p)+4),m.word(p+4)],m);m.put_word(p+4,0);
    }
    pcre_log(m,p,LiteralM::PcreDeinitEnd,Some(status),h);0
}
fn free_pcre_entry(m:&mut Memory<'_>,p:u32,i:u32,h:&mut impl HooksM) {
    let slot=|m:&Memory<'_>|m.word(p+4).wrapping_add(i.wrapping_mul(4));
    let entry=m.word(slot(m));
    if entry!=0 {
        h.call_m(0x196428,&[m.word(m.word(p)+4),m.word(entry)],m);
        let entry=m.word(slot(m));m.put_word(entry,0);
        h.call_m(0x196428,&[m.word(m.word(p)+4),m.word(entry+12)],m);
        let entry=m.word(slot(m));m.put_word(entry+12,0);
    }
    h.call_m(0x196428,&[m.word(m.word(p)+4),m.word(slot(m))],m);
    let slot=slot(m);m.put_word(slot,0);
}

pub fn pcre_init_m(m:&mut Memory<'_>,p:u32,key:u32,capacity:i32,width:i32,h:&mut impl HooksM)->u32 {
    let mut result=h.call_m(0x176f94,&[p,key,0xdd57,24],m);
    if result>=0x80000000 {return result;}
    let owner=m.word(p);
    pcre_log(m,p,LiteralM::PcreInitBegin,None,h);
    m.put_half(p+12,284);m.put_half(p+8,0);m.put_half(p+14,capacity as u16);
    for off in [16,18,20] {m.put_half(p+off,0);}
    m.put_half(p+10,width as u16);
    let array=h.call_m(0x1963b8,&[m.word(owner+4),1,(capacity as u32).wrapping_add(1).wrapping_mul(4)],m);
    m.put_word(p+4,array);
    let mut ok=array!=0;
    if ok {
        let allocated=h.call_m(0x1963b8,&[m.word(owner+4),1,20],m);
        m.put_word(m.word(p+4).wrapping_add(half(m,p+8)*4),allocated);
        ok=allocated!=0;
        if ok {
            let size=half(m,p+12).wrapping_mul(half(m,p+10)).wrapping_add(1);
            m.put_word(allocated+4,size);m.put_word(allocated+8,0);m.put_word(allocated,0);
            let data=h.call_m(0x1963b8,&[m.word(owner+4),1,size],m);m.put_word(allocated,data);
            ok=data!=0;
            if ok {
                let at=m.offset(allocated+16);m.bytes[at]=0;m.put_word(allocated+12,0);
                let extra=h.call_m(0x1963b8,&[m.word(owner+4),1,half(m,p+10)+1],m);
                m.put_word(allocated+12,extra);ok=extra!=0;
                if ok {let count=half(m,p+8).wrapping_add(1) as u16;m.put_half(p+8,count);m.put_half(p+18,count);}
            }
        }
    }
    if !ok {
        h.call_m(0x173dc0,&[m.word(owner+16),h.literal_m(LiteralM::PcreCategory),1000,0],m);
        result=0x8a50200a;pcre_deinit_m(m,p,key,h);
    }

    h.call_m(0x1738e8,&[m.word(owner+16),h.literal_m(LiteralM::PcreCategory),4,0,h.literal_m(LiteralM::PcreInitEnd),result],m);
    result
}
