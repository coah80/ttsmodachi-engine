
use crate::boot::Memory;

pub fn fn_20129c(m: &mut Memory<'_>, version: u32, out: u32, descriptor: u32) -> u32 {
    if version <= 1 {
        m.put_word(out, descriptor);
        0
    } else {
        0x8bb02001
    }
}

pub fn fn_205f00(
    m: &mut Memory<'_>,
    engine: u32,
    key: u32,
    out: u32,
    present: u32,
    absent: u32,
) -> u32 {
    let result = crate::frontend::validate_handle(engine != 0, key, 0xf38a, 184);
    if result >= 0x80000000 {
        return 0x8a802008;
    }
    let value = if m.word(engine.wrapping_add(88)) != 0 {
        present
    } else {
        absent
    };
    m.put_word(out, value);
    0
}

pub fn fn_206788(m: &mut Memory<'_>, version: u32, out: u32, descriptor: u32) -> u32 {
    if version <= 1 {
        m.put_word(out, descriptor);
        0
    } else {
        0x8a802001
    }
}

pub fn fn_20714c(m: &mut Memory<'_>, version: u32, out: u32, descriptor: u32) -> u32 {
    if version <= 1 {
        m.put_word(out, descriptor);
        0
    } else {
        0x89b02001
    }
}

pub trait Host {
    fn thread_id(&mut self) -> u32 {
        panic!("platform thread id not bound")
    }
    fn call(&mut self, m: &mut Memory<'_>, function: u32, args: &[u32]) -> u32;
}
struct Calls<'a, H>(&'a mut H);
impl<H: Host> crate::boot::Calls for Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
    fn call_wide(&mut self, _: &mut Memory<'_>, _: u32, _: &[u32]) -> u64 {
        unreachable!()
    }
}
impl<H: Host> crate::boot_part_e::Host for Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
}

pub struct Environment {
    pub null_pair: [u8; 8],
    pub default_context: u32,
    pub inet_name: u32,
    pub module: u32,

    pub scratch: u32,
}
fn resolve(m: &Memory<'_>, object: u32, tag: u32, e: &Environment) -> (u32, u32) {
    let cached = [
        u32::from_le_bytes(e.null_pair[..4].try_into().unwrap()),
        u32::from_le_bytes(e.null_pair[4..].try_into().unwrap()),
    ];

    let word = if object == 0 {
        None
    } else if tag == 0x1e61 && [object, tag] != cached {
        Some(m.half(object))
    } else {
        Some(0)
    };
    let mut context = 0;
    let rc = crate::frontend::resolve_engine_context(
        object,
        word,
        tag,
        &e.null_pair,
        e.default_context,
        &mut context,
    );
    (rc, context)
}

pub fn fn_201548(
    m: &mut Memory<'_>,
    _a: u32,
    _b: u32,
    object: u32,
    tag: u32,
    out: u32,
    e: &Environment,
    h: &mut impl Host,
) -> u32 {
    let pair0 = u32::from_le_bytes(e.null_pair[..4].try_into().unwrap());
    let pair1 = u32::from_le_bytes(e.null_pair[4..].try_into().unwrap());
    if crate::frontend::match_halves(object, tag, pair0, pair1) != 0 || out == 0 {
        return 0x85002007;
    }
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    let (rc, ctx) = resolve(m, object, tag, e);
    if rc >= 0x80000000 {
        return 0x85002007;
    }
    let p = crate::boot_part_e::calloc(m, m.word(ctx + 4), 1, 0x30, &mut Calls(h));
    if p == 0 {
        h.call(m, 0x173dc0, &[m.word(ctx + 16), e.module, 0x1130, 0]);
        return 0x8500200a;
    }
    m.put_word(p, object);
    m.put_word(p + 4, tag);
    m.put_word(p + 8, ctx);

    let mut nodes = [];
    let mut list = crate::frontend_resources::ResourceList {
        nodes: &mut nodes,
        head: None,
        tail: None,
    };
    list.initialize();
    m.put_word(p + 24, m.word(ctx + 4));
    m.put_word(p + 28, list.head.map_or(0, |i| i as u32));
    m.put_word(p + 32, list.tail.map_or(0, |i| i as u32));
    let saved = m.word(e.scratch);
    m.put_word(e.scratch, 0);
    let status = m
        .boot_parameter_call(
            &mut Calls(h),
            0x17b058,
            &[m.word(ctx + 24), e.inet_name, e.scratch],
            e.scratch + 8,
        )
        .unwrap();
    let info = m.word(e.scratch);
    m.put_word(e.scratch, saved);
    if status < 0x80000000 {
        for i in 0..3 {
            m.put_word(p + 12 + i * 4, m.word(info + 4 + i * 4));
        }
    }
    m.put_word(out + 4, 0x2f0);
    m.put_word(out, p);
    rc
}

fn acquire(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    res: u32,
    name: u32,
    e: &Environment,
    slot: u32,
) -> (u32, u32) {
    m.put_word(slot, 0);
    let rc = m
        .boot_parameter_call(&mut Calls(h), 0x17b058, &[res, name, slot], e.scratch + 8)
        .unwrap();
    (rc, m.word(slot))
}
fn release_name(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    res: u32,
    name: u32,
    e: &Environment,
) -> u32 {
    if res == 0 || name == 0 {
        return 0x8b602007;
    }
    let rc = h.call(m, 0x176f2c, &[m.word(res + 12)]);
    if rc >= 0x80000000 {
        return rc;
    }
    m.put_word(e.scratch, 0);
    let find = h.call(m, 0x1966ac, &[m.word(res + 16), name, e.scratch]);
    let slot = m.word(e.scratch);
    let callback = if find < 0x80000000 {
        m.word(slot + 24)
    } else {
        0
    };
    let mut snapshot = None;
    let mut rc = crate::frontend::release_ling_entry(
        m.bytes,
        Some(res),
        Some(name),
        &crate::frontend::ReleaseScripts {
            lock: 0,
            find,
            found: if find < 0x80000000 { Some(slot) } else { None },
            remove: 0,
            unlock: 0,
        },
        if callback != 0 {
            Some(|_: u32, _: u32, bytes: &mut [u8; 32]| {
                snapshot = Some(*bytes);
            })
        } else {
            None
        },
    );
    if let Some(bytes) = snapshot {
        rc = h.call(m, 0x19672c, &[m.word(res + 16), name]);
        let at = m.offset(e.scratch + 8);
        m.bytes[at..at + 32].copy_from_slice(&bytes);
        h.call(m, callback, &[m.word(res), m.word(res + 4), e.scratch + 8]);
    }
    let unlock = h.call(m, 0x176f5c, &[m.word(res + 12)]);
    if unlock >= 0x80000000 && rc < 0x80000000 {
        rc = unlock;
    }
    rc
}

pub struct ConstructorEnvironment {
    pub common: Environment,

    pub names: [u32; 3],
    pub section: u32,
    pub keys: [u32; 2],
}
fn cleanup_names(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    res: u32,
    e: &ConstructorEnvironment,
    n: usize,
) {
    for &name in &e.names[..n] {
        release_name(m, h, res, name, &e.common);
    }
}

pub fn fn_20683c(
    m: &mut Memory<'_>,
    _a: u32,
    _b: u32,
    object: u32,
    tag: u32,
    out: u32,
    e: &ConstructorEnvironment,
    h: &mut impl Host,
) -> u32 {
    assert_eq!(m.base, 0, "frontend ports require a base-zero arena");
    let saved: [u8; 160] = core::array::from_fn(|i| m.byte(e.common.scratch + i as u32));
    let rc = open_normalizer(m, object, tag, out, e, h);
    let at = m.offset(e.common.scratch);
    m.bytes[at..at + 160].copy_from_slice(&saved);
    rc
}
fn open_normalizer(
    m: &mut Memory<'_>,
    object: u32,
    tag: u32,
    out: u32,
    e: &ConstructorEnvironment,
    h: &mut impl Host,
) -> u32 {
    if out == 0 {
        return 0x89b02007;
    }
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    let (rc, env) = resolve(m, object, tag, &e.common);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut resources = [0; 3];
    for i in 0..3 {
        let (rc, entry) = acquire(
            m,
            h,
            m.word(env + 24),
            e.names[i],
            &e.common,
            e.common.scratch,
        );
        if rc >= 0x80000000 {
            cleanup_names(m, h, m.word(env + 24), e, i);
            return rc;
        }
        resources[i] = entry;
    }
    let n = crate::boot_part_e::alloc(m, m.word(env + 4), 44, &mut Calls(h));
    if n == 0 {
        h.call(m, 0x173dc0, &[m.word(env + 16), e.common.module, 32000, 0]);
        cleanup_names(m, h, m.word(env + 24), e, 3);
        return 0x89b0200a;
    }
    m.put_word(n, env);
    m.put_word(n + 4, m.word(resources[0] + 4));
    m.put_word(n + 16, m.word(resources[1] + 4));
    m.put_word(n + 8, m.word(resources[1] + 8));
    m.put_word(n + 12, m.word(resources[1] + 12));
    m.put_word(n + 28, m.word(resources[2] + 4));
    m.put_word(n + 24, m.word(resources[2] + 12));
    m.put_word(n + 20, m.word(resources[2] + 8));
    m.put_word(n + 32, 0);
    m.put_word(n + 36, 0);
    m.put_byte(n + 40, 1);
    m.put_word(out, n);
    m.put_word(out + 4, 0xf382);
    let s = e.common.scratch + 144;
    m.put_half(s + 4, 0xffff);
    let vt = m.word(n + 16);
    let rc = h.call(
        m,
        m.word(vt + 28),
        &[
            m.word(n + 8),
            m.word(n + 12),
            e.section,
            e.keys[0],
            s,
            s + 4,
            s + 6,
        ],
    );
    if rc < 0x80000000 && m.half(s + 4) != 0 {
        let val = m.word(m.word(s));
        let sep = m.byte(s + 6) as u32;
        let end = crate::frontend::find_byte(m.bytes, val, sep);
        if end != 0 {
            m.put_byte(end, 0);
        }
        let flag = crate::pronunciation::parse_int(&m.bytes[m.offset(m.word(m.word(s)))..]);
        m.put_byte(n + 40, flag as u8);
    } else if rc >= 0x80000000 {
        let n = m.word(out);
        if crate::frontend::validate_handle(n != 0, m.word(out + 4), 0xf382, 44) < 0x80000000
            && n != 0
        {
            let ctx = m.word(n);
            cleanup_names(m, h, m.word(ctx + 24), e, 3);
            crate::boot_part_e::release(m, m.word(m.word(n) + 4), n, &mut Calls(h));
        }
        m.put_word(out, 0);
        m.put_word(out + 4, 0);
    }
    rc
}

pub fn fn_2198b4(
    m: &mut Memory<'_>,
    _a: u32,
    _b: u32,
    object: u32,
    tag: u32,
    out: u32,
    e: &ConstructorEnvironment,
    h: &mut impl Host,
) -> u32 {
    assert_eq!(m.base, 0, "frontend ports require a base-zero arena");
    let saved: [u8; 160] = core::array::from_fn(|i| m.byte(e.common.scratch + i as u32));
    let rc = open_rewriter(m, object, tag, out, e, h);
    let at = m.offset(e.common.scratch);
    m.bytes[at..at + 160].copy_from_slice(&saved);
    rc
}
fn open_rewriter(
    m: &mut Memory<'_>,
    object: u32,
    tag: u32,
    out: u32,
    e: &ConstructorEnvironment,
    h: &mut impl Host,
) -> u32 {
    if out == 0 {
        return 0x8a602007;
    }
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    let (rc, ctx) = resolve(m, object, tag, &e.common);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut resources = [0; 3];
    for i in 0..3 {
        let (rc, entry) = acquire(
            m,
            h,
            m.word(ctx + 24),
            e.names[i],
            &e.common,
            e.common.scratch,
        );
        if rc >= 0x80000000 {
            cleanup_names(m, h, m.word(ctx + 24), e, i);
            return rc;
        }
        resources[i] = entry;
    }
    let p = crate::boot_part_e::alloc(m, m.word(ctx + 4), 0x4c, &mut Calls(h));
    if p == 0 {
        h.call(m, 0x173dc0, &[m.word(ctx + 16), e.common.module, 0x9858, 0]);
        cleanup_names(m, h, m.word(ctx + 24), e, 3);
        return 0x8a60200a;
    }
    m.put_word(p, ctx);
    m.put_word(p + 4, 0);
    m.put_word(p + 8, 0);
    m.put_word(p + 28, m.word(resources[0] + 4));
    m.put_word(p + 12, m.word(resources[2] + 4));
    m.put_word(p + 4, m.word(resources[2] + 8));
    m.put_word(p + 8, m.word(resources[2] + 12));
    m.put_word(p + 24, m.word(resources[1] + 4));
    m.put_word(p + 16, m.word(resources[1] + 8));
    m.put_word(p + 20, m.word(resources[1] + 12));
    m.put_byte(p + 64, 0);
    m.put_byte(p + 65, 0);
    m.put_word(p + 68, 0);
    m.put_half(p + 72, 0);
    let s = e.common.scratch + 144;
    m.put_word(s, 0);
    m.put_half(s + 4, 0);
    m.put_byte(s + 6, 0);
    let mut rc = 0;
    for (i, key) in e.keys.iter().enumerate() {
        if i == 1 {
            m.put_half(s + 4, 0);
        }
        rc = h.call(
            m,
            m.word(m.word(p + 12) + 28),
            &[
                m.word(p + 4),
                m.word(p + 8),
                e.section,
                *key,
                s,
                s + 4,
                s + 6,
            ],
        );
        if rc >= 0x80000000 {
            destroy_spelling(m, m.word(out), m.word(out + 4), e, h);
            m.put_word(out, 0);
            m.put_word(out + 4, 0);
        }
        if m.half(s + 4) == 1 && m.word(s) != 0 && m.byte(m.word(m.word(s))) == b'1' {
            m.put_byte(p + 64 + i as u32, 1);
        }
    }
    m.put_word(out + 4, 0xf389);
    m.put_word(out, p);
    rc
}

fn destroy_spelling(
    m: &mut Memory<'_>,
    p: u32,
    key: u32,
    e: &ConstructorEnvironment,
    h: &mut impl Host,
) -> u32 {
    struct Adapter<'a, H> {
        host: &'a mut H,
        env: &'a ConstructorEnvironment,
    }
    impl<H: Host> crate::engine_components::InternalCalls for Adapter<'_, H> {
        fn call(
            &mut self,
            m: &mut crate::engine::Memory<'_, '_>,
            f: u32,
            a: &[u32],
        ) -> Result<u32, crate::engine::Error> {
            let region = &mut m.regions[0];
            let mut b = Memory {
                base: region.base,
                bytes: region.bytes,
            };
            Ok(match f {
                0x17b1c4 => release_name(&mut b, self.host, a[0], a[1], &self.env.common),
                0x196428 => {
                    crate::boot_part_e::release(&mut b, a[0], a[1], &mut Calls(self.host));
                    0
                }
                _ => self.host.call(&mut b, f, a),
            })
        }
    }
    let mut regions = [crate::engine::Region {
        base: m.base,
        bytes: m.bytes,
    }];
    let mut memory = crate::engine::Memory::new(&mut regions).unwrap();
    crate::engine_components2::destroy_spelling(
        &mut memory,
        p,
        key,
        &mut Adapter { host: h, env: e },
    )
    .unwrap()
}

pub struct DictionaryEnvironment {

    pub backend_methods: [u32; 8],

    pub trace: [u32; 6],
    pub common: Environment,
    pub empty: u32,
    pub types: [u32; 3],
    pub modes: [u32; 2],
    pub unknown_type: u32,
    pub octet_stream: u32,
    pub lang_mode: u32,
    pub already_loaded_format: u32,
    pub name_format: u32,
    pub name_type_format: u32,
    pub uri_format: u32,
    pub name_label: u32,
    pub type_label: u32,
    pub uri_label: u32,

    pub attach: crate::boot_port_5b::Attach,
}
fn strcmp(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    crate::frontend::compare_str(&m.bytes[m.offset(a)..], &m.bytes[m.offset(b)..])
}
fn contains(m: &Memory<'_>, a: u32, b: u32) -> bool {
    crate::frontend::find_substr(m.bytes, a, b) != 0
}
fn list_next(m: &Memory<'_>, list: u32, cur: &mut u32) -> u32 {
    let next = if *cur == 0 {
        m.word(list + 4)
    } else {
        m.word(*cur + 8)
    };
    let mut nodes = [crate::frontend_resources::ResourceListNode {
        key: 0,
        value: if next == 0 { 0 } else { m.word(next + 4) },
        next: None,
    }];
    let list = crate::frontend_resources::ResourceList {
        nodes: &mut nodes,
        head: if next == 0 { None } else { Some(0) },
        tail: None,
    };
    let result = list.next(&mut None);
    *cur = next;
    result
}
fn list_insert(m: &mut Memory<'_>, list: u32, value: u32, h: &mut impl Host) -> u32 {
    let n = crate::boot_part_e::calloc(m, m.word(list), 1, 12, &mut Calls(h));
    if n == 0 {
        return 0x8500200a;
    }
    let first = m.word(list + 4);
    let mut last = first;
    if first != 0 && m.word(first) != u32::MAX {
        while m.word(last + 8) != 0 {
            last = m.word(last + 8);
        }
    }
    use crate::frontend_resources::{ResourceList, ResourceListNode as Node};
    let mut nodes = [Node::default(); 3];
    if first != 0 {
        nodes[0].key = m.word(first);
        nodes[0].next = if last != first { Some(1) } else { None };
        nodes[1].key = if last == 0 { 0 } else { m.word(last) };
    }
    let mut local = ResourceList {
        nodes: &mut nodes,
        head: if first == 0 { None } else { Some(0) },
        tail: if first == 0 {
            None
        } else {
            Some(if first == last { 0 } else { 1 })
        },
    };
    let rc = local.insert(Some(2), value);
    let head = local.head;
    let tail = local.tail;
    let next = local.nodes[2].next;
    m.put_word(n, local.nodes[2].key);
    m.put_word(n + 4, local.nodes[2].value);
    m.put_word(n + 8, if next.is_some() { first } else { 0 });
    if head == Some(2) {
        m.put_word(list + 4, n);
    }
    if first != 0 && head != Some(2) {
        m.put_word(last + 8, n);
    }
    if tail == Some(2) {
        m.put_word(list + 8, n);
    }
    rc
}

pub fn fn_201748(
    m: &mut Memory<'_>,
    p: u32,
    magic: u32,
    d: u32,
    out: u32,
    e: &DictionaryEnvironment,
    h: &mut impl Host,
) -> u32 {
    assert_eq!(m.base, 0, "frontend ports require a base-zero arena");
    let saved: [u8; 320] = core::array::from_fn(|i| m.byte(e.common.scratch + i as u32));
    let rc = load_dictionary(m, p, magic, d, out, e, h);
    let at = m.offset(e.common.scratch);
    m.bytes[at..at + 320].copy_from_slice(&saved);
    rc
}
fn load_dictionary(
    m: &mut Memory<'_>,
    p: u32,
    magic: u32,
    d: u32,
    out: u32,
    e: &DictionaryEnvironment,
    h: &mut impl Host,
) -> u32 {
    if d == 0 || out == 0 {
        return 0x85002007;
    }
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    let rc = crate::frontend::validate_handle(p != 0, magic, 0x2f0, 0x30);
    if rc >= 0x80000000 {
        return rc;
    }
    let (mut rc, ctx) = resolve(m, m.word(p), m.word(p + 4), &e.common);
    if rc >= 0x80000000 {
        return rc;
    }
    let uri = m.word(d);
    if uri != 0 {
        let mut cur = 0;
        loop {
            let x = list_next(m, p + 24, &mut cur);
            if x == 0 {
                break;
            }
            if strcmp(m, uri, m.word(x + 4)) == 0 {
                h.call(
                    m,
                    0x1738e8,
                    &[
                        m.word(ctx + 16),
                        e.common.module,
                        3,
                        0,
                        e.already_loaded_format,
                        if m.word(d) != 0 { m.word(d) } else { e.empty },
                    ],
                );
                return 0x85002418;
            }
        }
    }
    let mut entry = 0;
    let mut hnd = 0;
    let mut oom = false;

    rc = (|| {
        if m.word(d) != 0 {
            entry = crate::boot_part_e::calloc(m, m.word(ctx + 4), 1, 44, &mut Calls(h));
            if entry == 0 {
                oom = true;
                return 0x8500200a;
            }
            m.put_word(entry, m.word(ctx + 4));
            let c2 = m.word(p + 8);
            let s = e.common.scratch + 144;
            m.put_word(s, 0);
            m.put_word(s + 4, 0);
            m.put_byte(entry + 17, 0);
            m.put_word(entry + 20, 0);
            m.put_word(entry + 24, 0);
            if m.word(p + 12) == 0 {
                let (status, info) = acquire(
                    m,
                    h,
                    m.word(c2 + 24),
                    e.common.inet_name,
                    &e.common,
                    e.common.scratch,
                );
                if status < 0x80000000 {
                    for i in 0..3 {
                        m.put_word(p + 12 + i * 4, m.word(info + 4 + i * 4));
                    }
                } else {
                    h.call(m, 0x173dc0, &[m.word(c2 + 16), e.common.module, 0x117a, 0]);
                    return 0x85002001;
                }
            }
            let status = h.call(
                m,
                m.word(m.word(p + 12) + 0x34),
                &[
                    m.word(p + 16),
                    m.word(p + 20),
                    m.word(d),
                    m.word(d + 20),
                    m.word(d + 24),
                    0,
                    s,
                    0,
                    s + 4,
                    entry + 20,
                    entry + 24,
                ],
            );
            hnd = m.word(s);
            if status >= 0x80000000 {
                return status;
            }
            let mut typ = m.word(d + 16);
            if typ == 0 || m.byte(typ) == 0 {
                typ = m.word(s + 4);
            }
            if typ == 0 || m.byte(typ) == 0 {
                h.call(
                    m,
                    0x173dc0,
                    &[
                        m.word(c2 + 16),
                        e.common.module,
                        0x113f,
                        e.uri_format,
                        e.uri_label,
                        m.word(d),
                    ],
                );
                return 0x85003d22;
            }
            let kind = if contains(m, typ, e.types[0]) || contains(m, typ, e.octet_stream) {
                0
            } else if contains(m, typ, e.types[1]) {
                1
            } else if contains(m, typ, e.types[2]) {
                2
            } else {
                3
            };
            m.put_word(
                entry + 8,
                if kind < 3 {
                    e.types[kind]
                } else {
                    e.unknown_type
                },
            );
            m.put_word(
                entry + 12,
                e.modes[if contains(m, typ, e.lang_mode) { 0 } else { 1 }],
            );
            rc = status;
        } else {
            if m.word(d + 8) == 0 {
                h.call(m, 0x173dc0, &[m.word(ctx + 16), e.common.module, 0x1138, 0]);
                return 0x85002007;
            }
            entry = crate::boot_part_e::calloc(m, m.word(ctx + 4), 1, 44, &mut Calls(h));
            if entry == 0 {
                oom = true;
                return 0x8500200a;
            }
            m.put_word(entry, m.word(ctx + 4));
            let typ = m.word(d + 16);
            let kind =
                if typ != 0 && (contains(m, typ, e.types[0]) || contains(m, typ, e.octet_stream)) {
                    0
                } else if typ != 0 && contains(m, typ, e.types[1]) {
                    1
                } else if typ != 0 && contains(m, typ, e.types[2]) {
                    2
                } else {
                    3
                };
            if kind == 3 {
                h.call(
                    m,
                    0x173dc0,
                    &[
                        m.word(ctx + 16),
                        e.common.module,
                        0x1133,
                        e.name_type_format,
                        e.name_label,
                        if m.word(d) != 0 { m.word(d) } else { e.empty },
                        e.type_label,
                        typ,
                    ],
                );
                return 0x85002416;
            }
            m.put_word(entry + 8, e.types[kind]);
            m.put_word(
                entry + 12,
                e.modes[if contains(m, typ, e.lang_mode) { 0 } else { 1 }],
            );
            m.put_word(
                entry + 24,
                if contains(m, typ, e.types[2]) {
                    4
                } else {
                    m.word(d + 12)
                },
            );
            m.put_byte(entry + 17, 0);
            m.put_word(entry + 20, m.word(d + 8));
        }
        let name = if m.word(d + 4) != 0 {
            m.word(d + 4)
        } else {
            e.empty
        };
        let len = crate::frontend::strlen_bytes(&m.bytes[m.offset(name)..]).wrapping_add(1);
        let copy = crate::boot_part_e::alloc(m, m.word(ctx + 4), len, &mut Calls(h));
        m.put_word(entry + 4, copy);
        if copy == 0 {
            oom = true;
            return 0x8500200a;
        }
        crate::frontend::copy_str(m.bytes, copy, name);
        let typ = m.word(entry + 8);
        if !e.types.iter().any(|&v| strcmp(m, typ, v) == 0) {
            h.call(
                m,
                0x173dc0,
                &[
                    m.word(ctx + 16),
                    e.common.module,
                    0x1133,
                    e.name_type_format,
                    e.name_label,
                    if m.word(d) != 0 { m.word(d) } else { e.empty },
                    e.type_label,
                    typ,
                ],
            );
            return 0x85002007;
        }
        if m.word(p + 36) == 0 {
            let pair = [m.word(p), m.word(p + 4)];
            let status = crate::boot_port_5::fn_281c14(
                m,
                &mut DictionaryContext {
                    host: h,
                    env: &e.common,
                },
                pair,
                p + 36,
                e.backend_methods,
                e.common.module,
                e.common.scratch + 128,
            );
            if status >= 0x80000000 {
                return status;
            }
        }
        let status = {
            let mut calls = AttachCalls {
                host: &mut *h,
                env: &e.common,
            };
            crate::boot_port_5b::fn_281f34(m, &mut calls, m.word(p + 36), entry, &e.attach)
        };
        if status >= 0x80000000 {
            return status;
        }
        m.put_byte(entry + 16, 1);
        let status = list_insert(m, p + 24, entry, h);
        if status >= 0x80000000 {
            h.call(
                m,
                0x173dc0,
                &[
                    m.word(ctx + 16),
                    e.common.module,
                    0x1142,
                    e.name_format,
                    e.name_label,
                    if m.word(entry + 4) != 0 {
                        m.word(entry + 4)
                    } else {
                        e.empty
                    },
                ],
            );
            status | 0x85002000
        } else {
            m.put_word(out + 4, 0x2f1);
            m.put_word(out, entry);
            status
        }
    })();
    if oom {
        h.call(m, 0x173dc0, &[m.word(ctx + 16), e.common.module, 0x1130, 0]);
    }
    if hnd != 0 {
        h.call(
            m,
            m.word(m.word(p + 12) + 0x38),
            &[m.word(p + 16), m.word(p + 20), hnd],
        );
    }
    if rc >= 0x80000000 && entry != 0 {
        fn_2012bc(m, p, entry, e, h);
    }
    rc
}

pub struct FrontendEnvironment {
    pub scratch: u32,
    pub detach: crate::boot_port_4b::Environment,
    pub section: u32,

    pub keys: [u32; 6],

    pub parameters: [u32; 5],

    pub strings: [u32; 9],
    pub version: u32,
    pub character_classes: [u16; 256],
    pub runtime: crate::leftovers_1::OpenRuntime,
}

fn fe_lock(m: &mut Memory<'_>, owner: u32, release: bool, h: &mut impl Host) -> u32 {
    let mut selected = None;
    let rc = if release {
        crate::frontend::release_lock(m.bytes, (owner != 0).then_some(owner), |arg| {
            selected = Some(arg);
            0
        })
    } else {
        crate::frontend::acquire_lock(m.bytes, (owner != 0).then_some(owner), |arg| {
            selected = Some(arg);
            0
        })
    };
    if let Some(arg) = selected {
        let off = if release { 12 } else { 8 };
        Host::call(h, m, m.word(m.word(m.word(owner) + 8) + off), &[arg])
    } else {
        rc
    }
}
fn fe_lookup(
    m: &mut Memory<'_>,
    f: u32,
    params: u32,
    key: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    if let Some(rc) = m.boot_lookup_pure(f, &[params, key, out, 0], 0x196980) {
        return rc;
    }
    if params == 0 || key == 0 || out == 0 {
        return 0x8b702007;
    }
    m.put_word(out, 0);
    let rc = fe_lock(m, m.word(params + 4), false, h);
    if rc >= 0x80000000 {
        return rc;
    }
    struct Locked;
    impl crate::frontend::StateLock for Locked {
        fn acquire(&mut self) -> u32 {
            0
        }
        fn release(&mut self) -> u32 {
            0
        }
    }
    let mut value = 0;
    let mut status = crate::frontend::lookup_param(
        m.bytes,
        Some(params),
        key,
        Some(&mut value),
        None,
        &mut Locked,
        |a, b| match (a, b) {
            (0, 0) => 0,
            (0, _) => -1,
            (_, 0) => 1,
            _ => crate::frontend::compare_str(&m.bytes[a as usize..], &m.bytes[b as usize..]),
        },
    );
    m.put_word(out, value);
    let rc = fe_lock(m, m.word(params + 4), true, h);
    if rc >= 0x80000000 && status < 0x80000000 {
        status = rc;
    }
    status
}
fn fe_open(
    m: &mut Memory<'_>,
    p: u32,
    path: u32,
    out: u32,
    e: &FrontendEnvironment,
    h: &mut (impl Host + crate::leftovers_1::OpenHost),
) -> u32 {
    struct Adapter<'a, H> {
        host: &'a mut H,
        runtime: &'a crate::leftovers_1::OpenRuntime,
    }
    impl<H: crate::leftovers_1::OpenHost> crate::selection::arena::Host for Adapter<'_, H> {
        fn call(
            &mut self,
            a: &mut crate::selection::arena::Arena<'_>,
            f: u32,
            args: &[u32],
        ) -> (u32, u32) {
            assert_eq!(f, 0x242fd0);
            let mut m = Memory {
                base: a.base,
                bytes: a.bytes,
            };
            (
                crate::leftovers_1::fn_242fd0(
                    &mut m,
                    args.try_into().unwrap(),
                    self.runtime,
                    self.host,
                ),
                0,
            )
        }
    }
    let pair = [m.word(p + 4), m.word(p + 8)];
    crate::selection::arena_mid::open_voice_resource(
        &mut crate::selection::arena::Arena::new(0, m.bytes),
        &mut Adapter {
            host: h,
            runtime: &e.runtime,
        },
        pair[0],
        pair[1],
        2,
        path,
        e.strings[7],
        e.version,
        out,
    )
}
fn fe_string(
    m: &mut Memory<'_>,
    p: u32,
    key: u32,
    e: &FrontendEnvironment,
    h: &mut impl Host,
) -> u32 {
    let s = e.scratch + 512;
    m.put_half(s + 4, 0xffff);
    Host::call(
        h,
        m,
        m.word(m.word(p + 24) + 28),
        &[
            m.word(p + 16),
            m.word(p + 20),
            e.section,
            key,
            s,
            s + 4,
            s + 6,
        ],
    )
}

pub fn fn_1f80d4(
    m: &mut Memory<'_>,
    p: u32,
    e: &FrontendEnvironment,
    h: &mut (impl Host + crate::leftovers_1::OpenHost),
) -> u32 {
    assert_eq!(m.base, 0);
    let saved: [u8; 768] = core::array::from_fn(|i| m.byte(e.scratch + i as u32));
    let rc = fe_load(m, p, e, h);
    for (i, b) in saved.into_iter().enumerate() {
        m.put_byte(e.scratch + i as u32, b);
    }
    rc
}
fn fe_load(
    m: &mut Memory<'_>,
    p: u32,
    e: &FrontendEnvironment,
    h: &mut (impl Host + crate::leftovers_1::OpenHost),
) -> u32 {
    let path = e.scratch;
    let buf = path + 256;
    let s = path + 512;
    let env = m.word(p);
    let params = m.word(env + 20);
    for i in 0..256 {
        m.put_byte(path + i, 0);
    }
    for i in [8, 12, 16, 20, 24, 28, 32, 36, 40] {
        m.put_word(s + i, 0);
    }
    for (i, offset, invert) in [(0, 0x178, false), (1, 0x179, true), (2, 0x17d, false)] {
        let rc = fe_string(m, p, e.keys[i], e, h);
        if rc >= 0x80000000 {
            return rc;
        }
        let mut flag = m.half(s + 4) != 0;
        if i == 2 {
            flag = flag || contains(m, e.strings[0], m.word(p + 0x2ac));
        }
        m.put_byte(p + offset, (flag ^ invert) as u8);
    }
    let a = fe_lookup(m, 0x1756d0, params, e.parameters[0], s + 8, h);
    let no_ext = a < 0x80000000
        && crate::fe_text::strlen_195d90(m.bytes, m.word(s + 8)) as u32 > 3
        && fe_lookup(m, 0x175a1c, params, e.parameters[1], s + 12, h) >= 0x80000000;
    m.put_byte(p + 0x2a8, no_ext as u8);
    let rc = fe_string(m, p, e.keys[3], e, h);
    if rc >= 0x80000000 {
        return rc;
    }
    m.put_byte(p + 0x1a4, (m.half(s + 4) != 0) as u8);
    m.put_byte(p + 0x1a5, 0);
    let rc = fe_string(m, p, e.keys[4], e, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let text = if m.half(s + 4) != 0 {
        let text = m.word(m.word(s));
        let end = crate::fe_text::strchr_195d30(m.bytes, text, m.byte(s + 6) as u32);
        if end != 0 {
            m.put_byte(end, 0);
        }
        text
    } else {
        e.strings[1]
    };
    crate::frontend::copy_str(m.bytes, p + 0x1a6, text);
    let rc = fe_string(m, p, e.keys[5], e, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let n = if m.half(s + 4) == 0 {
        0
    } else {
        let text = m.word(m.word(s));
        let end = crate::fe_text::strchr_195d30(m.bytes, text, m.byte(s + 6) as u32);
        if end != 0 {
            m.put_byte(end, 0);
        }
        crate::frontend::parse_int(&e.character_classes, m.bytes, text) as u8
    };
    m.put_byte(p + 0x2a6, n);
    for (i, key) in [e.parameters[2], e.parameters[3]].into_iter().enumerate() {
        let rc = fe_lookup(m, 0x175a1c, params, key, s + 16 + i as u32 * 4, h);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    crate::frontend::copy_str(m.bytes, path, e.strings[1]);
    if fe_lookup(m, 0x175a1c, params, e.parameters[4], s + 24, h) < 0x80000000
        && m.word(s + 24) != 0
        && strcmp(m, m.word(s + 24), e.strings[2]) == 0
    {
        for text in [
            e.strings[3],
            m.word(s + 16),
            e.strings[4],
            m.word(s + 20),
            e.strings[5],
        ] {
            crate::fe_text::strcat_195c90(m.bytes, path, text);
        }
    }
    for text in [e.strings[6], m.word(s + 16), e.strings[4], m.word(s + 20)] {
        crate::fe_text::strcat_195c90(m.bytes, path, text);
    }
    let mut i = 0u16;
    while (i as i32) < crate::fe_text::strlen_195d90(m.bytes, path) {
        m.put_byte(
            path + i as u32,
            crate::normalization::tolower_240ba0(m.byte(path + i as u32) as u32) as u8,
        );
        i = i.wrapping_add(1);
    }
    let rc = fe_open(m, p, path, s + 28, e, h);
    if rc >= 0x80000000 {
        return 0;
    }
    let reader = m.word(s + 28);
    loop {
        let rc = crate::render_part_a::open_reader_chunk(
            m,
            h,
            reader,
            s + 32,
            s + 40,
            s + 36,
            &e.runtime.reader,
        );
        if rc >= 0x80000000 {
            break;
        }
        m.put_word(s + 44, 256);
        let data = m.word(s + 36);
        let size = m.word(s + 40);
        let rc = crate::render_part_a::read_chunk_string(
            &mut crate::selection::arena::Arena::new(0, m.bytes),
            reader,
            data,
            size,
            0,
            buf,
            s + 44,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let entry = crate::boot_part_e::alloc(m, m.word(env + 4), 12, &mut Calls(h));
        if entry == 0 {
            crate::render_part_a::error_log(
                m,
                h,
                m.word(env + 16),
                e.strings[8],
                0x84d0,
                0,
                0,
                s + 200,
            );
            return 0x89b0200a;
        }
        let rc = crate::boot_port_4b::fn_243ad4(m, h, reader, entry, entry + 4, &e.detach);
        if rc >= 0x80000000 {
            return rc;
        }
        let n = m.word(s + 44).wrapping_add(3) & !3;
        let rc = Host::call(
            h,
            m,
            0x2b1a74,
            &[
                p + 0x190,
                p + 0x190,
                0,
                m.word(entry + 4).wrapping_add(n),
                0,
                entry + 8,
            ],
        );
        if rc != 0 {
            let resource = m.word(entry);
            crate::render_part_d::release_chunk(m.bytes, resource, h);
            crate::boot_part_e::release(m, m.word(env + 4), entry, &mut Calls(h));
            crate::render_part_a::error_log(
                m,
                h,
                m.word(env + 16),
                e.strings[8],
                0x84d4,
                0,
                0,
                s + 200,
            );
            return rc.wrapping_add(0x1ea0) | 0x89b02000;
        }
        let mut text = buf;
        while text != 0 {
            let end = crate::fe_text::strchr_195d30(m.bytes, text, b',' as u32);
            if end != 0 {
                m.put_byte(end, 0);
            }
            let rc = m
                .boot_parameter_call(
                    &mut Calls(h),
                    0x19668c,
                    &[m.word(p + 0x174), text, entry],
                    s + 64,
                )
                .unwrap();
            if rc >= 0x80000000 {
                let resource = m.word(entry);
                crate::render_part_d::release_chunk(m.bytes, resource, h);
                crate::boot_part_e::release(m, m.word(env + 4), entry, &mut Calls(h));
                return rc;
            }
            text = if end == 0 { 0 } else { end + 1 };
        }
        let rc = crate::render_part_a::close_reader_chunk(m, h, reader, &e.runtime.reader);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    crate::render_part_a::close_reader(m, h, reader, &e.runtime.reader)
}

pub struct FrontendConstructorEnvironment {
    pub common: Environment,
    pub frontend: FrontendEnvironment,
    pub names: [u32; 5],
    pub normal: u32,
    pub part_section: u32,
    pub part_key: u32,
    pub escape_key: u32,
    pub g2p_key: u32,
    pub morphology_keys: [u32; 8],
    pub morphology_scratch: u32,
    pub map_callbacks: [u32; 5],
    pub allocators: [u32; 4],
}

pub fn fn_1f87b8(m: &mut Memory<'_>, value: u32, _context: u32, out: u32) -> u32 {
    m.put_word(out, value);
    0
}
pub fn fn_1f87b4() {}
struct ConfigCalls<'a, H> {
    host: &'a mut H,
    scratch: u32,
}
impl<H: Host> crate::boot_port_5::ConfigHost for ConfigCalls<'_, H> {
    fn string(
        &mut self,
        m: &mut Memory<'_>,
        method: u32,
        pair: [u32; 2],
        section: u32,
        key: u32,
        value: &mut u32,
        len: &mut u16,
        sep: &mut u8,
    ) -> u32 {
        let s = self.scratch;
        let saved = [m.word(s), m.word(s + 4)];
        m.put_word(s, *value);
        m.put_half(s + 4, *len);
        m.put_byte(s + 6, *sep);
        let rc = Host::call(
            self.host,
            m,
            method,
            &[pair[0], pair[1], section, key, s, s + 4, s + 6],
        );
        *value = m.word(s);
        *len = m.half(s + 4);
        *sep = m.byte(s + 6);
        m.put_word(s, saved[0]);
        m.put_word(s + 4, saved[1]);
        rc
    }
}

pub fn fn_1f87c4(
    m: &mut Memory<'_>,
    _unused0: u32,
    _unused1: u32,
    object: u32,
    tag: u32,
    out: u32,
    e: &FrontendConstructorEnvironment,
    h: &mut (impl Host + crate::leftovers_1::OpenHost),
) -> u32 {
    let s = e.common.scratch;
    let saved: [u8; 256] = core::array::from_fn(|i| m.byte(s + i as u32));
    let rc = fe_construct(m, object, tag, out, e, h);
    for (i, b) in saved.into_iter().enumerate() {
        m.put_byte(s + i as u32, b);
    }
    rc
}
fn fe_construct(
    m: &mut Memory<'_>,
    object: u32,
    tag: u32,
    out: u32,
    e: &FrontendConstructorEnvironment,
    h: &mut (impl Host + crate::leftovers_1::OpenHost),
) -> u32 {
    if out == 0 {
        return 0x89b02007;
    }
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    let (rc, env) = resolve(m, object, tag, &e.common);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut resources = [0; 5];
    let s = e.common.scratch;
    let res = m.word(env + 24);
    for i in 0..5 {
        let (rc, value) = acquire(m, h, res, e.names[i], &e.common, s);
        if rc >= 0x80000000 {
            for name in &e.names[..i] {
                release_name(m, h, res, *name, &e.common);
            }
            return rc;
        }
        resources[i] = value;
    }
    let p = crate::boot_part_e::calloc(m, m.word(env + 4), 1, 0x4c8, &mut Calls(h));
    if p == 0 {
        crate::render_part_a::error_log(
            m,
            h,
            m.word(env + 16),
            e.frontend.strings[8],
            0x84d0,
            0,
            0,
            s + 160,
        );
        for i in [0, 1, 2, 4, 3] {
            release_name(m, h, res, e.names[i], &e.common);
        }
        return 0x89b0200a;
    }
    m.put_word(out + 4, 0xf384);
    m.put_word(out, p);
    m.put_word(p, env);
    m.put_word(p + 4, object);
    m.put_word(p + 8, tag);
    m.put_word(p + 12, m.word(resources[0] + 4));
    for (i, vt, pair) in [(1, 24, 16), (2, 36, 28), (3, 48, 40), (4, 60, 52)] {
        m.put_word(p + vt, m.word(resources[i] + 4));
        m.put_word(p + pair, m.word(resources[i] + 8));
        m.put_word(p + pair + 4, m.word(resources[i] + 12));
    }
    m.put_byte(s + 144, 0);
    m.put_byte(s + 145, 0);
    let rc = Host::call(
        h,
        m,
        m.word(m.word(p + 24) + 0x4c),
        &[m.word(p + 16), m.word(p + 20), s + 144, s + 145],
    );
    if rc >= 0x80000000 {
        for i in [0, 1, 2, 4, 3] {
            release_name(m, h, res, e.names[i], &e.common);
        }
        return rc;
    }
    m.put_byte(
        p + 0x173,
        (m.byte(s + 144) == 1 && m.byte(s + 145) == 1) as u8,
    );
    for off in [0x40, 0x44, 0x48, 0x4c, 0x54, 0x58, 0x5c, 0x60, 0x64] {
        m.put_word(p + off, 0);
    }
    m.put_half(p + 0x50, 0);
    m.put_byte(p + 0x172, 1);
    m.put_word(p + 0x174, 0);
    let mut status = fe_lookup(
        m,
        0x1756d0,
        m.word(m.word(p) + 20),
        e.frontend.parameters[2],
        p + 0x2ac,
        h,
    );
    if status < 0x80000000 {
        for off in [0x40, 0x44, 0x48, 0x4c, 0x54, 0x58, 0x5c, 0x60, 0x64] {
            let buf = crate::boot_part_e::alloc(m, m.word(m.word(p) + 4), 128, &mut Calls(h));
            m.put_word(p + off, buf);
            if buf == 0 {
                status = 0x89b0200a;
                break;
            }
        }
        if status == 0x89b0200a {
            crate::render_part_a::error_log(
                m,
                h,
                m.word(m.word(p) + 16),
                e.frontend.strings[8],
                0x84d0,
                0,
                0,
                s + 160,
            );
        }
        if status < 0x80000000 {
            crate::frontend::copy_str(m.bytes, p + 0x72, e.normal);
            status = crate::boot_port_5::fn_275204(
                m,
                &mut ConfigCalls {
                    host: h,
                    scratch: s + 144,
                },
                p,
                e.normal,
                e.morphology_keys,
                e.frontend.section,
                e.morphology_scratch,
                &e.frontend.character_classes,
            );
        }
        if status < 0x80000000 {
            m.put_half(s + 148, 0xffff);
            status = Host::call(
                h,
                m,
                m.word(m.word(p + 24) + 28),
                &[
                    m.word(p + 16),
                    m.word(p + 20),
                    e.part_section,
                    e.part_key,
                    s + 144,
                    s + 148,
                    s + 150,
                ],
            );
            if status < 0x80000000 && m.half(s + 148) != 0 {
                let text = m.word(m.word(s + 144));
                let end = crate::fe_text::strchr_195d30(m.bytes, text, m.byte(s + 150) as u32);
                if end != 0 {
                    m.put_byte(end, 0);
                }
                m.put_byte(
                    p + 0x172,
                    crate::frontend::parse_int(&e.frontend.character_classes, m.bytes, text) as u8,
                );
            }
        }
        if status < 0x80000000 {
            let map = crate::boot_part_e::calloc(m, m.word(m.word(p) + 4), 1, 68, &mut Calls(h));
            struct Allocated(u32);
            impl crate::frontend::MethodTable for Allocated {
                fn call_at(&mut self, _: u32, _: u32, _: u32) -> u32 {
                    self.0
                }
            }
            let cb = e.map_callbacks;
            status = crate::boot_part_b2::guest_1964e0(
                m,
                [
                    m.word(m.word(p) + 4),
                    0,
                    cb[0],
                    cb[1],
                    cb[2],
                    cb[3],
                    cb[4],
                    p + 0x174,
                ],
                &mut Allocated(map),
            );
        }
        if status < 0x80000000 {
            for (i, method) in e.allocators.into_iter().enumerate() {
                m.put_word(p + 0x180 + i as u32 * 4, method);
            }
            let heap = m.word(m.word(p) + 4);
            let error = crate::normalization::quad_copy(m.bytes, p + 0x190, p + 0x180, heap);
            if error != 0 {
                crate::render_part_a::error_log(
                    m,
                    h,
                    m.word(m.word(p) + 16),
                    e.frontend.strings[8],
                    0x84d4,
                    0,
                    0,
                    s + 160,
                );
                status = error.wrapping_add(0x1ea0) | 0x89b02000;
            }
        }
        if status < 0x80000000 {
            status = fn_1f80d4(m, p, &e.frontend, h);
        }
        if status < 0x80000000 {
            m.put_half(s + 148, 0xffff);
            status = Host::call(
                h,
                m,
                m.word(m.word(p + 24) + 28),
                &[
                    m.word(p + 16),
                    m.word(p + 20),
                    e.frontend.section,
                    e.escape_key,
                    s + 144,
                    s + 148,
                    s + 150,
                ],
            );
            if status < 0x80000000 {
                m.put_word(p + 0x2b0, 0);
                if m.half(s + 148) != 0 {
                    let text = m.word(m.word(s + 144));
                    let size = crate::fe_text::strlen_195d90(m.bytes, text) as u32;
                    let buf = crate::boot_part_e::alloc(
                        m,
                        m.word(m.word(p) + 4),
                        size.wrapping_add(1),
                        &mut Calls(h),
                    );
                    m.put_word(p + 0x2b0, buf);
                    if buf == 0 {
                        crate::render_part_a::error_log(
                            m,
                            h,
                            m.word(m.word(p) + 16),
                            e.frontend.strings[8],
                            0x84d0,
                            0,
                            0,
                            s + 160,
                        );
                        status = 0x89b0200a;
                    } else {
                        crate::frontend::copy_str(m.bytes, buf, text);
                        let n = crate::fe_text::strlen_195d90(m.bytes, text) as u32;
                        m.put_byte(m.word(p + 0x2b0).wrapping_add(n).wrapping_sub(1), b'|');
                    }
                }
            }
        }
        if status < 0x80000000 {
            m.put_byte(p + 0x2b4, 1);
            crate::frontend::copy_str(m.bytes, s + 152, e.g2p_key);
            for i in 33..176 {
                if m.byte(p + 0x2b4) == 0 {
                    break;
                }
                m.put_byte(s + 155, i as u8);
                m.put_half(s + 148, 0xffff);
                status = Host::call(
                    h,
                    m,
                    m.word(m.word(p + 24) + 28),
                    &[
                        m.word(p + 16),
                        m.word(p + 20),
                        e.frontend.section,
                        s + 152,
                        s + 144,
                        s + 148,
                        s + 150,
                    ],
                );
                if status >= 0x80000000 {
                    break;
                }
                if m.half(s + 148) == 0 {
                    m.put_byte(p + 0x2b4, 0);
                } else {
                    let dst = p + 0x2b5 + i * 3;
                    let text = m.word(m.word(s + 144));
                    crate::frontend::copy_str(m.bytes, dst, e.frontend.strings[1]);
                    let n = crate::fe_text::strlen_195d90(m.bytes, text) as u32;
                    crate::fe_text::strncat_195dac(m.bytes, dst, text, n.wrapping_sub(1));
                }
            }
        }
    }
    if status >= 0x80000000 {
        Host::call(h, m, 0x1f7ed0, &[m.word(out), m.word(out + 4)]);
        m.put_word(out, 0);
        m.put_word(out + 4, 0);
    }
    status
}

pub struct EngineDictionaryEnvironment<'a> {
    pub scratch: u32,

    pub trace: [u32; 6],
    pub uri_format: u32,
    pub uri_label: u32,

    pub strings: [u32; 9],
    pub types: [u32; 3],
    pub module: u32,
    pub runtime: crate::leftovers_1::OpenRuntime,

    pub dictionary: crate::boot_port_5::DictionaryEnvironment<'a>,
    pub archive: crate::boot_part_e::ArchiveEnvironment,
    pub frame: u32,

    pub tables: crate::boot_port_5b::Tables,
}

struct CdtCalls<'a, H>(&'a mut H);
impl<H: Host> crate::boot_part_f::Host for CdtCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Host::call(self.0, m, f, a)
    }
    fn compare(
        &mut self,
        m: &mut Memory<'_>,
        _f: u32,
        key: &mut crate::boot_part_f::Key,
        value: &mut u32,
    ) -> u32 {
        crate::boot_part_f::compare_parameter(m, *key, *value)
    }
}
impl<H: Host> crate::boot_port_5b::Host for CdtCalls<'_, H> {}
struct Lane5Calls<'a, H>(&'a mut H);
impl<H: Host> crate::boot_port_5::Host for Lane5Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Host::call(self.0, m, f, a)
    }
}

struct Engine5Calls<'a, H>(&'a mut H);
impl<H: Host + crate::boot_part_e::ArchiveHost> crate::boot_port_5::Host for Engine5Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Host::call(self.0, m, f, a)
    }
}
impl<H: Host + crate::boot_part_e::ArchiveHost> crate::boot_port_6::Host for Engine5Calls<'_, H> {
    fn thread_id(&mut self) -> u32 {
        panic!("unbound platform thread id")
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Host::call(self.0, m, f, a)
    }
}
impl<H: Host + crate::boot_part_e::ArchiveHost> crate::boot_part_f::Host for Engine5Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Host::call(self.0, m, f, a)
    }
    fn compare(
        &mut self,
        m: &mut Memory<'_>,
        _f: u32,
        key: &mut crate::boot_part_f::Key,
        value: &mut u32,
    ) -> u32 {
        crate::boot_part_f::compare_parameter(m, *key, *value)
    }
}
impl<H: Host + crate::boot_part_e::ArchiveHost> crate::log_live::LogMethodsM
    for Engine5Calls<'_, H>
{
    fn log_m(
        &mut self,
        _: u32,
        _: u32,
        _: u32,
        _: u32,
        _: u32,
        _: u32,
        _: u32,
        _: &mut Memory<'_>,
    ) -> u32 {
        panic!("unbound logger")
    }
}
impl<H: Host + crate::boot_part_e::ArchiveHost> crate::speech_part_5::Host
    for Engine5Calls<'_, H>
{
    fn scratch(&self) -> u32 {
        panic!("unbound speech scratch")
    }
    fn format(&self, _callsite: u32) -> u32 {
        panic!("unbound formatter")
    }
    fn dictionary_entries(
        &mut self,
        _: &mut Memory<'_>,
        _: u32,
        _: &mut u32,
        _: &mut u32,
    ) -> u32 {
        panic!("unbound dictionary entries")
    }
    fn record(
        &mut self,
        _: &mut Memory<'_>,
        _: u32,
        _: u32,
        _: &mut crate::speech_part_5::RecordBuffer,
        _: &mut u8,
    ) -> u32 {
        panic!("unbound dictionary record")
    }
    fn entries(
        &mut self,
        _: &mut Memory<'_>,
        _: u32,
        _: u32,
        _: u32,
        _: &mut u32,
        _: &mut u32,
    ) -> u32 {
        panic!("unbound dictionary entries")
    }
}
impl<H: Host + crate::boot_part_e::ArchiveHost> crate::boot_part_e::Host for Engine5Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Host::call(self.0, m, f, a)
    }
}
impl<H: Host + crate::boot_part_e::ArchiveHost> crate::boot_part_e::ArchiveHost
    for Engine5Calls<'_, H>
{
    fn service(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32], out: &mut [u32; 4]) -> u32 {
        self.0.service(m, f, a, out)
    }
    fn read(&mut self, m: &mut Memory<'_>, f: u32, ctx: u32, b: &mut [u8]) -> u32 {
        self.0.read(m, f, ctx, b)
    }
}

pub fn fn_205f40(
    m: &mut Memory<'_>,
    engine: u32,
    key: u32,
    content_type: u32,
    source: u32,
    data: u32,
    size: u32,
    options: [u32; 2],
    out: u32,
    e: &EngineDictionaryEnvironment<'_>,
    h: &mut (impl Host + crate::leftovers_1::OpenHost + crate::boot_part_e::ArchiveHost),
) -> u32 {
    let saved: [u8; 1152] = core::array::from_fn(|i| m.byte(e.scratch + i as u32));
    let rc = engine_dictionary(
        m,
        engine,
        key,
        content_type,
        source,
        data,
        size,
        options,
        out,
        e,
        h,
    );
    for (i, b) in saved.into_iter().enumerate() {
        m.put_byte(e.scratch + i as u32, b);
    }
    rc
}
fn engine_dictionary(
    m: &mut Memory<'_>,
    p: u32,
    key: u32,
    ctype: u32,
    source: u32,
    data: u32,
    size: u32,
    options: [u32; 2],
    out: u32,
    e: &EngineDictionaryEnvironment<'_>,
    h: &mut (impl Host + crate::leftovers_1::OpenHost + crate::boot_part_e::ArchiveHost),
) -> u32 {
    if crate::frontend::validate_handle(p != 0, key, 0xf38a, 184) >= 0x80000000 {
        return 0x8a802008;
    }
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    let s = e.scratch;
    let input = s;
    let buf = s + 32;
    let tmp = s + 1060;
    if ctype == 0
        || out == 0
        || (source == 0 && (data == 0 || size == 0))
        || (source != 0 && data == 0 && size != 0)
    {
        crate::render_part_a::error_log(
            m,
            h,
            m.word(m.word(p + 8) + 16),
            e.module,
            0x9c41,
            0,
            0,
            s + 1100,
        );
        return 0x8a802007;
    }
    for i in 0..28 {
        m.put_byte(input + i, 0);
    }
    for (i, v) in [
        if data != 0 && size != 0 { 0 } else { source },
        source,
        data,
        size,
        ctype,
        options[0],
        options[1],
    ]
    .into_iter()
    .enumerate()
    {
        m.put_word(input + i as u32 * 4, v);
    }
    trace_logger(
        m,
        h,
        m.word(m.word(p + 8) + 16),
        0x65,
        e.trace[0],
        [e.trace[2], 0, e.trace[3], source],
        s + 1080,
        s + 32,
    );
    m.put_word(tmp, 0);
    m.put_word(tmp + 4, e.strings[0]);
    m.put_word(tmp + 8, 0);
    let mut mapped = false;
    let mut result = 0;
    let mut cleanup = false;
    let mut finish = false;
    if size == 0 && contains(m, ctype, e.strings[7]) {
        result = crate::boot_part_b::fn_1795f0(
            m,
            m.word(m.word(p + 8) + 12),
            source,
            1,
            p + 60,
            &e.runtime.broker,
            h,
        );
        if result >= 0x80000000 {
            result = fe_lookup(
                m,
                0x175a1c,
                m.word(m.word(p + 8) + 20),
                e.strings[0],
                tmp + 8,
                h,
            );
            if result >= 0x80000000
                || m.word(tmp + 8) == 0
                || strcmp(m, m.word(tmp + 8), e.strings[3]) != 0
                || source == 0
            {
                result = 0;
                finish = true;
            } else {
                result = fe_lookup(
                    m,
                    0x175a1c,
                    m.word(m.word(p + 8) + 20),
                    e.strings[1],
                    tmp,
                    h,
                );
                if result < 0x80000000 {
                    result = fe_lookup(
                        m,
                        0x175a1c,
                        m.word(m.word(p + 8) + 20),
                        e.strings[2],
                        tmp + 4,
                        h,
                    );
                }
                if result >= 0x80000000 {
                    finish = true;
                } else {
                    m.put_byte(buf, 0);
                    for text in [
                        e.strings[4],
                        m.word(tmp),
                        e.strings[5],
                        m.word(tmp + 4),
                        e.strings[6],
                        source,
                    ] {
                        crate::fe_text::strcat_195c90(m.bytes, buf, text);
                    }
                    result = crate::boot_part_b::fn_1795f0(
                        m,
                        m.word(m.word(p + 8) + 12),
                        buf,
                        1,
                        p + 60,
                        &e.runtime.broker,
                        h,
                    );
                    if result >= 0x80000000 {
                        result = 0;
                        finish = true;
                    }
                }
            }
        }
        if !finish {
            mapped = true;
            result = crate::boot_part_b::fn_179374(
                m,
                m.word(m.word(p + 8) + 12),
                m.word(p + 60),
                0,
                input + 12,
                p + 64,
                h,
            );
            if result >= 0x80000000 {
                cleanup = true;
            } else {
                m.put_word(input, 0);
                m.put_byte(buf, 0);
                crate::fe_text::strcat_195c90(m.bytes, buf, e.types[0]);
                let mode = crate::frontend::find_substr(m.bytes, ctype, e.strings[8]);
                if mode != 0 {
                    let mut end = crate::fe_text::strchr_195d30(m.bytes, mode, b';' as u32);
                    if end == 0 {
                        end = mode + crate::fe_text::strlen_195d90(m.bytes, mode) as u32;
                    }
                    let n = crate::fe_text::strlen_195d90(m.bytes, buf) as u32;
                    m.put_byte(buf + n, b';');
                    m.put_byte(buf + n + 1, 0);
                    crate::fe_text::strncat_195dac(
                        m.bytes,
                        buf,
                        mode,
                        end.wrapping_sub(mode) as u16 as u32,
                    );
                }
                m.put_word(input + 16, buf);
                m.put_word(input + 8, m.word(p + 64));
            }
        }
    }
    if !finish && !cleanup {
        let vt = m.word(p + 88);
        if vt != 0 {
            result = Host::call(
                h,
                m,
                m.word(vt + 28),
                &[m.word(p + 92), m.word(p + 96), input, out],
            );
            if m.word(p + 60) != 0 {
                crate::boot_part_e::fn_179208(
                    m,
                    m.word(m.word(p + 8) + 12),
                    m.word(p + 60),
                    m.word(p + 64),
                    &e.runtime.unmap,
                    &mut Calls(h),
                );
                m.put_word(p + 64, 0);
                crate::leftovers_1::fn_179f3c(
                    m,
                    m.word(m.word(p + 8) + 12),
                    m.word(p + 60),
                    &e.runtime.close,
                    h,
                );
                m.put_word(p + 60, 0);
            }
            cleanup = result >= 0x80000000;
        } else if m.word(input) != 0 && m.word(input + 8) == 0 {
            m.put_word(s + 1080, e.uri_label);
            m.put_word(s + 1084, m.word(input));
            crate::render_part_a::error_log(
                m,
                h,
                m.word(m.word(p + 8) + 16),
                e.module,
                0x9c43,
                e.uri_format,
                s + 1080,
                s + 1100,
            );
            result = 0x8a802001;
            cleanup = true;
        } else {
            m.put_word(p + 168, m.word(input + 8));
            m.put_word(p + 172, m.word(p + 168));
            m.put_word(p + 176, m.word(input + 12));
            m.put_word(p + 180, 0);
            let kind = if crate::fe_text::strcmp_ci_1769d0(m.bytes, m.word(input + 16), e.types[0])
                == 0
            {
                0
            } else if crate::fe_text::strcmp_ci_1769d0(m.bytes, m.word(input + 16), e.types[1]) == 0
            {
                1
            } else {
                2
            };
            if kind < 2 {
                let err = if kind == 0 {
                    crate::boot_port_5::fn_2b4010(
                        m,
                        &mut Engine5Calls(h),
                        [p + 128, p + 128, tmp + 4, p + 148, p, p + 164, 1, tmp + 12],
                        &e.dictionary,
                        &e.archive,
                        e.frame,
                    )
                } else {
                    crate::boot_port_5b::fn_2b22b0(
                        m,
                        &mut CdtCalls(h),
                        p + 128,
                        p + 128,
                        p + 148,
                        p,
                        0,
                        tmp + 12,
                        &e.tables,
                    )
                };
                let err = if err == 0 {
                    if kind == 0 {
                        crate::boot_port_5::fn_2b3f04(
                            m,
                            &mut Lane5Calls(h),
                            m.word(tmp + 12),
                            0x18b54089,
                            p + 68,
                        )
                    } else {
                        crate::boot_port_5b::fn_2b2210(m, m.word(tmp + 12), 0x18b54089, p + 68)
                    }
                } else {
                    err
                };
                if err != 0 {
                    result = err.wrapping_add(0x1ea0) | 0x8a802000;
                    m.put_word(p + 68, 0);
                    cleanup = true;
                } else {
                    let pair = if kind == 0 { p + 80 } else { p + 72 };
                    m.put_word(pair, m.word(tmp + 12));
                    m.put_word(pair + 4, 0xf38a);
                    m.put_word(out, m.word(tmp + 12));
                    m.put_word(out + 4, 0xf38a);
                    result = 0;
                }
            } else if crate::fe_text::strcmp_ci_1769d0(m.bytes, m.word(input + 16), e.types[2]) == 0
            {
                m.put_word(p + 68, m.word(input + 8));
            }
        }
    }
    if cleanup && mapped && m.word(p + 60) != 0 {
        if m.word(p + 64) != 0 {
            crate::boot_part_e::fn_179208(
                m,
                m.word(m.word(p + 8) + 12),
                m.word(p + 60),
                m.word(p + 64),
                &e.runtime.unmap,
                &mut Calls(h),
            );
            m.put_word(p + 64, 0);
        }
        crate::leftovers_1::fn_179f3c(
            m,
            m.word(m.word(p + 8) + 12),
            m.word(p + 60),
            &e.runtime.close,
            h,
        );
        m.put_word(p + 60, 0);
    }
    trace_logger(
        m,
        h,
        m.word(m.word(p + 8) + 16),
        0x65,
        e.trace[1],
        [e.trace[4], 0, e.trace[5], result],
        s + 1080,
        s + 32,
    );
    result
}

impl<H: Host> crate::boot_part_b::Host for Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Host::call(self.0, m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        Host::thread_id(self.0)
    }
}
impl<H: Host> crate::boot_part_b::FormatHost for Calls<'_, H> {}

pub fn fn_2012bc(
    m: &mut Memory<'_>,
    p: u32,
    entry: u32,
    e: &DictionaryEnvironment,
    h: &mut impl Host,
) -> u32 {
    let saved: [u8; 320] = core::array::from_fn(|i| m.byte(e.common.scratch + i as u32));
    let rc = release_dictionary_entry(m, p, entry, e, h);
    for (i, b) in saved.into_iter().enumerate() {
        m.put_byte(e.common.scratch + i as u32, b);
    }
    rc
}
fn release_dictionary_entry(
    m: &mut Memory<'_>,
    p: u32,
    entry: u32,
    e: &DictionaryEnvironment,
    h: &mut impl Host,
) -> u32 {
    let (mut err, ctx) = resolve(m, m.word(p), m.word(p + 4), &e.common);
    if err >= 0x80000000 {
        return err;
    }
    trace_logger(
        m,
        h,
        m.word(ctx + 16),
        0x66,
        e.trace[0],
        [e.trace[2], 0, e.trace[3], m.word(entry + 4)],
        e.common.scratch,
        e.common.scratch + 160,
    );
    if m.word(entry + 32) != 0 {
        let typ = m.word(entry + 8);
        if typ == 0 || !e.types.iter().any(|&v| strcmp(m, typ, v) == 0) {
            err = 0x85002007;
            let s = e.common.scratch;
            let name = m.word(entry + 4);
            for (i, v) in [
                e.name_label,
                if name != 0 { name } else { e.empty },
                e.type_label,
                typ,
            ]
            .into_iter()
            .enumerate()
            {
                m.put_word(s + i as u32 * 4, v);
            }
            crate::render_part_a::error_log(
                m,
                &mut Calls(h),
                m.word(ctx + 16),
                e.common.module,
                if typ == 0 { 0x1135 } else { 0x1134 },
                if typ == 0 {
                    e.name_format
                } else {
                    e.name_type_format
                },
                s,
                s + 32,
            );
        } else {

            err = Host::call(h, m, 0x281d8c, &[m.word(p + 36), entry]);
            if err >= 0x80000000 {
                trace_logger(
                    m,
                    h,
                    m.word(ctx + 16),
                    0x66,
                    e.trace[1],
                    [e.trace[4], 0, e.trace[5], err],
                    e.common.scratch,
                    e.common.scratch + 160,
                );
                return err;
            }
        }
    }
    if m.word(entry + 4) != 0 {
        crate::boot_part_e::release(m, m.word(ctx + 4), m.word(entry + 4), &mut Calls(h));
    }
    if m.word(entry + 20) != 0 && m.byte(entry + 17) != 0 {
        crate::boot_part_e::release(m, m.word(ctx + 4), m.word(entry + 20), &mut Calls(h));
    }
    crate::boot_part_e::release(m, m.word(ctx + 4), entry, &mut Calls(h));
    trace_logger(
        m,
        h,
        m.word(ctx + 16),
        0x66,
        e.trace[1],
        [e.trace[4], 0, e.trace[5], err],
        e.common.scratch,
        e.common.scratch + 160,
    );
    err
}

struct DictionaryContext<'a, H> {
    host: &'a mut H,
    env: &'a Environment,
}
impl<H: Host> crate::boot_port_5::Host for DictionaryContext<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Host::call(self.host, m, f, a)
    }
}
impl<H: Host> crate::boot_port_5::ContextHost for DictionaryContext<'_, H> {
    fn context_pair(&mut self, _: &mut Memory<'_>) -> [u8; 8] {
        self.env.null_pair
    }
    fn default_context(&mut self, _: &mut Memory<'_>) -> u32 {
        self.env.default_context
    }
}

struct AttachCalls<'a, H> {
    host: &'a mut H,
    env: &'a Environment,
}
impl<H: Host> crate::boot_part_f::Host for AttachCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Host::call(self.host, m, f, a)
    }
    fn compare(
        &mut self,
        _: &mut Memory<'_>,
        _: u32,
        _: &mut crate::boot_part_f::Key,
        _: &mut u32,
    ) -> u32 {
        panic!("dictionary release does not compare keys")
    }
}
impl<H: Host> crate::boot_port_5b::Host for AttachCalls<'_, H> {
    fn context_pair(&mut self, _: &Memory<'_>) -> [u8; 8] {
        self.env.null_pair
    }
    fn default_context(&mut self, _: &Memory<'_>) -> u32 {
        self.env.default_context
    }
}

fn trace_logger(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    logger: u32,
    category: u32,
    format: u32,
    args: [u32; 4],
    va: u32,
    scratch: u32,
) -> u32 {
    let saved: [u32; 4] = core::array::from_fn(|i| m.word(va + i as u32 * 4));
    for (i, value) in args.into_iter().enumerate() {
        m.put_word(va + i as u32 * 4, value);
    }
    let result =
        crate::boot_port_1::fn_17486c(m, logger, category, format, va, scratch, &mut Calls(h));
    for (i, value) in saved.into_iter().enumerate() {
        m.put_word(va + i as u32 * 4, value);
    }
    result
}
