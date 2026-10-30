//! Render-path component matching and pronunciation-context construction.
//!
//! Property-tree adapters use the crate's offset-addressed frontend ports: their
//! Memory must have base zero. Unmapped prefix bytes remain caller-owned.
//! Component matching also supports a nonzero guest base.
use crate::{boot::Memory, frontend, normalization, render_part_c};

/// The component's indirect match method is a platform boundary. The returned
/// list pointer is the method's stack-local output, not a captured engine result.
pub trait MatchHost {
    fn component_match(
        &mut self,
        memory: &mut Memory<'_>,
        method: u32,
        handle: [u32; 2],
    ) -> (u32, u32);
}

fn half(m: &Memory<'_>, p: u32) -> u16 {
    u16::from_le_bytes([m.byte(p), m.byte(p + 1)])
}

/// fn_2c69a0 / EU 2c6b5c / KR 2c6c08.
/// Find the first component advertising an exact or semicolon-delimited prefix.
pub fn fn_2c69a0(
    m: &mut Memory<'_>,
    context: u32,
    name: u32,
    out: u32,
    host: &mut impl MatchHost,
) -> u32 {
    if context == 0 || name == 0 || m.byte(name) == 0 {
        return 0x8c902007;
    }
    m.put_word(out, 0);
    let mut i = 0u16;
    loop {
        if half(m, context + 8) <= i {
            return 0x8c902014;
        }
        let component = m.word(m.word(context + 4) + u32::from(i) * 4);
        let method = m.word(m.word(component + 8) + 28);
        if method != 0 {
            let (rc, slot) =
                host.component_match(m, method, [m.word(component + 12), m.word(component + 16)]);
            if rc < 0x80000000 && slot != 0 {
                let mut j = 0u16;
                loop {
                    if m.word(out) != 0 {
                        break;
                    }
                    let prefix = m.word(slot + u32::from(j) * 4);
                    if prefix == 0 {
                        break;
                    }
                    let n = normalization::strlen8(m.bytes, m.offset(prefix) as u32);
                    let a = m.offset(name);
                    let b = m.offset(prefix);
                    if frontend::compare_str_n(&m.bytes[a..], &m.bytes[b..], n as i32) == 0
                        && (m.byte(name + n) == 0 || m.byte(name + n) == b';')
                    {
                        // Reload the array element, as the original does after
                        // the indirect method and before the caller-visible store.
                        let found = m.word(m.word(context + 4) + u32::from(i) * 4);
                        m.put_word(out, found);
                    }
                    j = j.wrapping_add(1);
                }
            }
        }
        i = i.wrapping_add(1);
        if m.word(out) != 0 {
            return 0;
        }
    }
}

/// Caller-owned ROM constants and cached environment pair.
pub struct Environment<'a> {
    pub engine: render_part_c::Environment,
    pub max_markers_key: u32,
    pub langcode_key: u32,
    pub parser_category: u32,
    pub nts_prefix: &'a [u8],
    pub ipa_prefix: &'a [u8],
}
/// The existing frontend ports hold property tables under StateLock. select_lock
/// chooses the lock token used by acquire/release; only its indirect platform
/// methods are host work. Pronunciation sessions execute render part d's
/// dictionary constructor and destructor ports.
pub trait ContextHost:
    render_part_c::Host + frontend::StateLock + crate::boot_part_b::FormatHost
{
    fn select_lock(&mut self, lock: u32);
}

pub(crate) fn resolve(m: &Memory<'_>, handle: [u32; 2], env: &render_part_c::Environment) -> (u32, u32) {
    let cached = [
        u32::from_le_bytes(env.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(env.cached_handle[4..].try_into().unwrap()),
    ];
    let pointed = if handle != cached && handle[0] != 0 && handle[1] == 0x1e61 {
        Some(half(m, handle[0]))
    } else {
        None
    };
    let mut context = 0;
    let rc = frontend::resolve_engine_context(
        handle[0],
        pointed,
        handle[1],
        &env.cached_handle,
        env.default_context,
        &mut context,
    );
    (rc, context)
}

fn nz(p: u32) -> Option<u32> {
    if p == 0 {
        None
    } else {
        Some(p)
    }
}

/// fn_2c2338 / EU 2c24f4 / KR 2c25a0.
/// Initialize marker capacity, preserving the lookup's status and range rules.
/// scratch is 24 caller-owned, disjoint writable bytes replacing machine-stack
/// locals. Its contents are restored. compare_keys must compare immutable key
/// strings using the same ordering as the property dictionary.
pub fn fn_2c2338(
    m: &mut Memory<'_>,
    p: u32,
    value: u32,
    scratch: u32,
    env: &Environment<'_>,
    host: &mut impl ContextHost,
    compare_keys: impl FnMut(u32, u32) -> i32,
) -> u32 {
    assert_eq!(
        m.base, 0,
        "frontend property ports require offset-addressed memory"
    );
    let (rc, context) = resolve(m, [m.word(p), m.word(p + 4)], &env.engine);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    m.put_word(p + 96, value);
    m.put_word(p + 84, 0);
    m.put_half(p + 100, 200);
    // The local C output starts at zero. The real getter overwrites it even
    // when lookup fails. Scratch is restored on return, including error paths.
    let saved = m.word(scratch);
    m.put_word(scratch, 0);
    let properties = m.word(context + 20);
    if properties != 0 {
        host.select_lock(m.word(properties + 4));
    }
    let key = env.max_markers_key;
    // The frontend getter takes a pure key comparator because its output word
    // shares the arena. Keys are immutable for the duration of the lookup.
    let mut lock = PropertyLock { host };
    let rc = frontend::get_param_uint(
        m.bytes,
        nz(properties),
        key,
        scratch,
        &mut lock,
        compare_keys,
    );
    let n = m.word(scratch);
    m.put_word(scratch, saved);
    if rc & 0x1fff == 0x14 {
        return 0;
    }
    if rc >= 0x80000000 {
        return rc;
    }
    if n.wrapping_sub(1) < 0xfffe {
        m.put_half(p + 100, n as u16);
    } else {
        // fn_173dc0's null-logger fast path, then its real Rust formatter
        // callee. No engine routine is replaced with a diagnostic script.
        let logger = m.word(context + 16);
        if logger != 0 && env.parser_category != 0 {
            let at = m.offset(scratch + 4);
            let mut saved = [0u8; 20];
            saved.copy_from_slice(&m.bytes[at..at + 20]);
            crate::boot_part_b::fn_173c70(
                m,
                logger,
                env.parser_category,
                0x71c,
                0,
                0,
                scratch + 4,
                lock.host,
            );
            m.bytes[at..at + 20].copy_from_slice(&saved);
        }
    }
    rc
}
struct PropertyLock<'a, H> {
    host: &'a mut H,
}
impl<H: ContextHost> frontend::StateLock for PropertyLock<'_, H> {
    fn acquire(&mut self) -> u32 {
        self.host.acquire()
    }
    fn release(&mut self) -> u32 {
        self.host.release()
    }
}

/// fn_2c3068 / EU 2c3224 / KR 2c32d0.
/// Construct both optional pronunciation converters and release on failure.
/// name_scratch is eight disjoint writable bytes, restored on return.
pub fn fn_2c3068(
    m: &mut Memory<'_>,
    handle: [u32; 2],
    out: u32,
    env: &Environment<'_>,
    dictionary: &crate::render_part_d::DictionaryEnvironment,
    name_scratch: u32,
    host: &mut (impl ContextHost + crate::render_part_d::DictionaryHost),
) -> u32 {
    assert_eq!(
        m.base, 0,
        "frontend property ports require offset-addressed memory"
    );
    if out == 0 {
        return 0x81102007;
    }
    let (rc, engine) = resolve(m, handle, &env.engine);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    m.put_word(out, 0);
    let heap = m.word(engine + 4);
    let p = frontend::heap_alloc(m.bytes, nz(heap), 16, host);
    if p == 0 {
        return 0x8110200a;
    }
    frontend::fill_bytes(m.bytes, p, 0, 16);
    m.put_word(p, handle[0]);
    m.put_word(p + 4, handle[1]);
    let mut code = 0x81102007;
    if engine != 0 {
        let props = m.word(engine + 20);
        if props != 0 {
            host.select_lock(m.word(props + 4));
        }
        let mut lang = 0;
        code = frontend::lookup_param(
            m.bytes,
            nz(props),
            env.langcode_key,
            Some(&mut lang),
            None,
            host,
            |a, b| normalization::strcmp_at(m.bytes, a, b),
        );
        if code < 0x80000000 {
            let mut name = [0u8; 8];
            build_name(m, &mut name, env.nts_prefix, lang);
            code = open_dictionary_name(m, &name, handle, p + 8, name_scratch, dictionary, host);
            if code < 0x80000000 || code & 0x1fff == 1 {
                build_name(m, &mut name, env.ipa_prefix, lang);
                code =
                    open_dictionary_name(m, &name, handle, p + 12, name_scratch, dictionary, host);
                if code < 0x80000000 || code & 0x1fff == 1 {
                    if code >= 0x80000000 {
                        code = 0;
                    }
                    m.put_word(out, p);
                    return code;
                }
            }
        }
    }
    // This real sibling port resolves the handle again before releasing the
    // two converters. It deliberately does not replace failure cleanup with
    // allocator-only calls.
    render_part_c::fn_2c3008(
        m,
        p,
        &env.engine,
        &mut DictionaryCleanup {
            host,
            env: dictionary,
        },
    );
    code
}

fn open_dictionary_name(
    m: &mut Memory<'_>,
    name: &[u8; 8],
    handle: [u32; 2],
    out: u32,
    scratch: u32,
    env: &crate::render_part_d::DictionaryEnvironment,
    host: &mut impl crate::render_part_d::DictionaryHost,
) -> u32 {
    let at = m.offset(scratch);
    let mut saved = [0u8; 8];
    saved.copy_from_slice(&m.bytes[at..at + 8]);
    m.bytes[at..at + 8].copy_from_slice(name);
    let rc = crate::render_part_d::dictionary_new(m.bytes, scratch, handle, out, env, host);
    m.bytes[at..at + 8].copy_from_slice(&saved);
    rc
}
struct DictionaryCleanup<'a, H> {
    host: &'a mut H,
    env: &'a crate::render_part_d::DictionaryEnvironment,
}
impl<H: ContextHost + crate::render_part_d::DictionaryHost> frontend::HeapVtable
    for DictionaryCleanup<'_, H>
{
    fn alloc_at(&mut self, o: u32, n: u32) -> u32 {
        self.host.alloc_at(o, n)
    }
    fn realloc_at(&mut self, o: u32, p: u32, n: u32) -> u32 {
        self.host.realloc_at(o, p, n)
    }
    fn release_at(&mut self, o: u32, p: u32) {
        self.host.release_at(o, p)
    }
}
impl<H: ContextHost + crate::render_part_d::DictionaryHost> frontend::MethodTable
    for DictionaryCleanup<'_, H>
{
    fn call_at(&mut self, o: u32, n: u32, size: u32) -> u32 {
        self.host.call_at(o, n, size)
    }
}
impl<H: ContextHost + crate::render_part_d::DictionaryHost> render_part_c::Host
    for DictionaryCleanup<'_, H>
{
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if f == 0x2eca2c {
            crate::render_part_d::dictionary_free(m.bytes, a[0], self.env, self.host)
        } else {
            render_part_c::Host::call(self.host, m, f, a)
        }
    }
    fn output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        self.host.output(m, f, a)
    }
}
/// Resource and reader routines not yet ported are dispatched by address.
/// Indirect callbacks use the same method. Internal string/property helpers
/// and the resource-open forwarding wrapper execute their existing Rust ports.
pub trait ResourceHost: ContextHost {
    /// Constants and workspace for the rebound resource-close port.
    fn close_runtime(&self) -> crate::leftovers_1::CloseRuntime {
        panic!("resource close requires caller-supplied CloseRuntime")
    }
    /// Constants and workspace for the rebound RIFF constructor.
    fn open_runtime(&self) -> crate::leftovers_1::OpenRuntime {
        panic!("resource open requires caller-supplied OpenRuntime")
    }
    fn broker_compare_keys(&mut self, _a:u32, _b:u32) -> i32 {
        panic!("unbound broker tree comparator")
    }
    fn resource_call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32;
    /// Region-specific reader strings and disjoint stack workspace.
    fn reader_runtime(&self) -> crate::render_part_a::ReaderRuntime {
        panic!("reader requires caller-supplied ReaderRuntime")
    }
    fn reader_context(&self) -> render_part_c::Environment {
        panic!("reader requires caller-supplied context constants")
    }
    fn reader_heap_callback(
        &mut self,
        _kind: u32,
        _heap: u32,
        _old: u32,
        _count: u32,
        _size: u32,
    ) -> u32 {
        panic!("unbound external reader block allocator")
    }
    fn reader_array_callback(&mut self, _method: u32, _args: &[u32]) {
        panic!("unbound external reader array callback")
    }
    /// Region-specific broker constants and a disjoint 24-byte stack area.
    /// Required when the real broker-open port is reached.
    fn data_open_globals(&self) -> crate::boot_part_b::DataOpenGlobals {
        panic!("broker open requires caller-supplied DataOpenGlobals")
    }
    fn data_output(&mut self, _m: &mut Memory<'_>, _f: u32, _args: &[u32]) -> (u32, u32) {
        panic!("unbound broker output service")
    }
    fn data_property(&mut self, _m: &mut Memory<'_>, _db: u32, _key: &[u8]) -> (u32, u32) {
        panic!("unbound broker property service")
    }
    fn data_insert(&mut self, _m: &mut Memory<'_>, _db: u32, _key: u32, _record: &[u32]) -> u32 {
        panic!("unbound broker insertion service")
    }
}
struct BrokerHost<'a, H>(&'a mut H);
impl<H:ResourceHost> frontend::HeapVtable for BrokerHost<'_,H>{
    fn alloc_at(&mut self,o:u32,n:u32)->u32{self.0.alloc_at(o,n)}
    fn realloc_at(&mut self,o:u32,p:u32,n:u32)->u32{self.0.realloc_at(o,p,n)}
    fn release_at(&mut self,o:u32,p:u32){self.0.release_at(o,p)}
}
impl<H:ResourceHost> frontend::MethodTable for BrokerHost<'_,H>{
    fn call_at(&mut self,o:u32,n:u32,s:u32)->u32{self.0.call_at(o,n,s)}
}
impl<H:ResourceHost> frontend::StateLock for BrokerHost<'_,H>{
    fn acquire(&mut self)->u32{self.0.acquire()}
    fn release(&mut self)->u32{self.0.release()}
}
impl<H:ResourceHost> crate::log_live::LogMethodsM for BrokerHost<'_,H>{
    fn log_m(&mut self,f:u32,o:u32,c:u32,l:u32,a:u32,fmt:u32,v:u32,m:&mut Memory<'_>)->u32{
        self.0.resource_call(m,f,&[o,c,l,a,fmt,v])
    }
}
impl<H:ResourceHost> crate::leftovers_1::CloseHost for BrokerHost<'_,H>{
    fn select_close_lock(&mut self,p:u32){self.0.select_lock(p)}
    fn compare_tree_keys(&mut self,a:u32,b:u32)->i32{self.0.broker_compare_keys(a,b)}
    fn indirect(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{self.0.resource_call(m,f,a)}
}
impl<H: ResourceHost> crate::boot_part_b::Host for BrokerHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        match f {
            0x196394 => frontend::heap_alloc(m.bytes, nz(a[0]), a[1], self.0),
            0x1963b8 => frontend::call_method(m.bytes, nz(a[0]), a[1], a[2], self.0),
            0x1963e0 => frontend::heap_realloc(m.bytes, nz(a[0]), a[1], a[2], self.0),
            0x196428 => {
                frontend::heap_release(m.bytes, nz(a[0]), nz(a[1]), self.0);
                0
            }
            _ => self.0.resource_call(m, f, a),
        }
    }
    fn thread_id(&mut self) -> u32 {
        panic!("unexpected broker thread-id request")
    }
}
impl<H: ResourceHost> crate::boot_part_b::OutputHost for BrokerHost<'_, H> {
    fn output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        self.0.data_output(m, f, a)
    }
}
impl<H: ResourceHost> crate::boot_part_b::PropertyHost for BrokerHost<'_, H> {
    fn property(&mut self, m: &mut Memory<'_>, db: u32, key: &[u8]) -> (u32, u32) {
        self.0.data_property(m, db, key)
    }
    fn insert(&mut self, m: &mut Memory<'_>, db: u32, key: u32, e: [u32; 3]) -> u32 {
        self.0.data_insert(m, db, key, &e)
    }
}
impl<H: ResourceHost> crate::boot_part_b::QueryHost for BrokerHost<'_, H> {
    fn query_output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        self.0.data_output(m, f, a)
    }
    fn insert_record(&mut self, m: &mut Memory<'_>, db: u32, key: u32, e: [u32; 4]) -> u32 {
        self.0.data_insert(m, db, key, &e)
    }
    fn open_pair(&mut self, m: &mut Memory<'_>, f: u32, owner: [u32; 2], out: u32) -> u32 {
        self.0.resource_call(m, f, &[owner[0], owner[1], out])
    }
}
impl<H: ResourceHost> crate::boot_part_b::DataHost for BrokerHost<'_, H> {
    fn data_output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32], out: u32) -> u32 {
        // Keep the guest output address, including its no-write failure cases.
        let mut args = [0u32; 16];
        assert!(a.len() < args.len());
        args[..a.len()].copy_from_slice(a);
        args[a.len()] = out;
        self.0.resource_call(m, f, &args[..a.len() + 1])
    }
}
impl<H: ResourceHost> crate::boot_part_b::FormatHost for BrokerHost<'_, H> {
    fn rvct_vsnprintf(&mut self, m: &mut Memory<'_>, dst: u32, n: u32, fmt: u32, args: u32) -> i32 {
        crate::boot_part_b::FormatHost::rvct_vsnprintf(self.0, m, dst, n, fmt, args)
    }
    fn format_encode(
        &mut self,
        unit: u16,
        out: &mut [u8; 8],
        state: &mut [u8; 8],
    ) -> Option<usize> {
        crate::boot_part_b::FormatHost::format_encode(self.0, unit, out, state)
    }
    fn format_decimal_point(&mut self) -> u8 {
        crate::boot_part_b::FormatHost::format_decimal_point(self.0)
    }
}
impl<H: ResourceHost> normalization::NormalizationHost for BrokerHost<'_, H> {
    fn calloc(&mut self, mem: &mut [u8], heap: u32, n: u32, size: u32) -> Option<u32> {
        nz(frontend::call_method(mem, nz(heap), n, size, self.0))
    }
    fn alloc(&mut self, mem: &mut [u8], heap: u32, size: u32) -> Option<u32> {
        nz(frontend::heap_alloc(mem, nz(heap), size, self.0))
    }
    fn realloc(&mut self, mem: &mut [u8], heap: u32, old: u32, size: u32) -> Option<u32> {
        nz(frontend::heap_realloc(mem, nz(heap), old, size, self.0))
    }
    fn free(&mut self, mem: &mut [u8], heap: u32, p: u32) {
        frontend::heap_release(mem, nz(heap), nz(p), self.0)
    }
    fn resolve(&mut self, mem: &mut [u8], a: u32, b: u32) -> (u32, u32) {
        let env = self.0.reader_context();
        resolve(
            &Memory {
                base: 0,
                bytes: mem,
            },
            [a, b],
            &env,
        )
    }
}
impl<H: ResourceHost> frontend::BlockHeap for BrokerHost<'_, H> {
    fn alloc_block(&mut self, heap: u32, size: u32) -> u32 {
        self.0.reader_heap_callback(0, heap, 0, 1, size)
    }
    fn realloc_block(&mut self, heap: u32, old: u32, size: u32) -> u32 {
        self.0.reader_heap_callback(1, heap, old, 1, size)
    }
    fn alloc_cells(&mut self, heap: u32, n: u32, size: u32) -> u32 {
        self.0.reader_heap_callback(2, heap, 0, n, size)
    }
}
impl<H: ResourceHost> crate::render_part_a::ReaderHost for BrokerHost<'_, H> {
    fn array_release_method(&mut self, ctx: u32, p: u32) {
        self.0.reader_array_callback(0, &[ctx, p])
    }
    fn array_destructor(&mut self, method: u32, p: u32, ctx: u32) {
        self.0.reader_array_callback(method, &[p, ctx])
    }
    fn array_initializer(
        &mut self,
        mem: &mut [u8],
        method: u32,
        source: u32,
        ctx: u32,
        dst: u32,
    ) -> u32 {
        self.0.resource_call(
            &mut Memory {
                base: 0,
                bytes: mem,
            },
            method,
            &[source, ctx, dst],
        )
    }
}
/// Immutable guest addresses for keys, path fragments and the reader format.
pub struct ResourceEnvironment {
    pub keys: [u32; 4], // langcode, fedataprefix, fedatapackaging, fecfg
    pub clc: u32,
    pub clc_path: u32,
    pub slash: u32,
    pub colon: u32,
    pub pp: u32,
    pub reader_type: u32,
    pub version: u32,
    pub log_category: u32,
    pub log_format: u32,
    pub loading_type: u32,
}
fn parameter(m: &Memory<'_>, config: u32, key: u32, host: &mut impl ResourceHost) -> (u32, u32) {
    let props = m.word(m.word(config + 8) + 20);
    if props != 0 {
        host.select_lock(m.word(props + 4));
    }
    let mut value = 0;
    let rc = frontend::fetch_param(m.bytes, nz(props), key, Some(&mut value), host, |a, b| {
        normalization::strcmp_at(m.bytes, a, b)
    });
    (rc, value)
}
/// fn_2c5ef4. scratch is 160 disjoint caller-owned bytes, restored on return.
pub fn fn_2c5ef4(
    m: &mut Memory<'_>,
    config: u32,
    filter: u32,
    mode: u32,
    scratch: u32,
    env: &ResourceEnvironment,
    host: &mut impl ResourceHost,
) -> u32 {
    assert_eq!(m.base, 0);
    if config == 0 {
        return 0x81102007;
    }
    let at = m.offset(scratch);
    let mut saved = [0u8; 160];
    saved.copy_from_slice(&m.bytes[at..at + 160]);
    let result = (|| {
        let (rc, language) = parameter(m, config, env.keys[0], host);
        if rc >= 0x80000000 {
            return rc;
        }
        let mut i = 0;
        while i < normalization::strlen8(m.bytes, language) {
            let c = crate::fe_text::fold_17c8c0(u32::from(m.byte(language + i))) as u8;
            m.bytes[(language + i) as usize] = c;
            i += 1;
        }
        let (_, prefix) = parameter(m, config, env.keys[1], host);
        let (_, packaging) = parameter(m, config, env.keys[2], host);
        frontend::fill_bytes(m.bytes, scratch, 0, 64);
        if prefix != 0 && m.byte(prefix) != 0 {
            frontend::copy_str(m.bytes, scratch, prefix);
        }
        let clc = packaging != 0 && normalization::strcmp_at(m.bytes, packaging, env.clc) == 0;
        if clc {
            let (rc, cfg) = parameter(m, config, env.keys[3], host);
            if rc >= 0x80000000 {
                return rc;
            }
            for p in [env.clc_path, language, env.slash, cfg, env.colon] {
                frontend::append_str(m.bytes, scratch, p);
            }
        }
        for p in [env.pp, language] {
            frontend::append_str(m.bytes, scratch, p);
        }
        let mut i = 0;
        while i < normalization::strlen8(m.bytes, scratch) {
            let c = crate::fe_text::fold_17c8c0(u32::from(m.byte(scratch + i))) as u8;
            m.bytes[(scratch + i) as usize] = c;
            i += 1;
        }
        let mut rc = fn_2c6434(m, config, scratch, filter, mode, scratch + 64, env, host);
        if rc >= 0x80000000 && !clc {
            let n = normalization::strlen8(m.bytes, scratch);
            m.bytes[(scratch + n - 4) as usize] = 0;
            rc = fn_2c6434(m, config, scratch, filter, mode, scratch + 64, env, host);
        }
        rc
    })();
    m.bytes[at..at + 160].copy_from_slice(&saved);
    result
}
/// fn_2c6194. Unfiltered configuration loader.
pub fn fn_2c6194(
    m: &mut Memory<'_>,
    config: u32,
    mode: u32,
    scratch: u32,
    env: &ResourceEnvironment,
    host: &mut impl ResourceHost,
) -> u32 {
    fn_2c5ef4(m, config, 0, mode, scratch, env, host)
}
/// fn_2c6434. scratch is 96 disjoint bytes replacing stack locals.
pub fn fn_2c6434(
    m: &mut Memory<'_>,
    config: u32,
    path: u32,
    filter: u32,
    mode: u32,
    scratch: u32,
    env: &ResourceEnvironment,
    host: &mut impl ResourceHost,
) -> u32 {
    assert_eq!(m.base, 0, "reader helpers use absolute-index memory");
    let at = m.offset(scratch);
    let mut saved = [0u8; 96];
    saved.copy_from_slice(&m.bytes[at..at + 96]);
    let rc = (|| {
        m.put_word(scratch, 0);
        let owner = m.word(config + 8);
        let globals = host.data_open_globals();
        let rc = crate::boot_part_b::fn_1795f0(
            m,
            m.word(owner + 12),
            path,
            1,
            scratch,
            &globals,
            &mut BrokerHost(host),
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let close_runtime = host.close_runtime();
        let rc = crate::leftovers_1::fn_179f3c(m, m.word(owner + 12), m.word(scratch), &close_runtime, &mut BrokerHost(host));
        if rc >= 0x80000000 {
            return rc;
        }
        // fn_243a94 is a pure ABI forwarding wrapper. Its existing arena
        // port calls fn_242fd0, rebound below to its certified constructor.
        // Use a collecting host to retain the wrapper's complete argument list.
        struct Collect {
            args: [u32; 9],
        }
        impl crate::selection::arena::Host for Collect {
            fn call(
                &mut self,
                _: &mut crate::selection::arena::Arena<'_>,
                f: u32,
                a: &[u32],
            ) -> (u32, u32) {
                assert_eq!(f, 0x242fd0);
                self.args.copy_from_slice(a);
                (0, 0)
            }
        }
        let mut forward = Collect { args: [0; 9] };
        let h0 = m.word(config);
        let h1 = m.word(config + 4);
        crate::selection::arena_mid::open_voice_resource(
            &mut crate::selection::arena::Arena::new(m.base, m.bytes),
            &mut forward,
            h0,
            h1,
            0,
            path,
            env.reader_type,
            env.version,
            scratch + 4,
        );
        let open_runtime = host.open_runtime();
        let rc = crate::leftovers_1::fn_242fd0(m, forward.args, &open_runtime, &mut BrokerHost(host));
        if rc >= 0x80000000 {
            return rc;
        }
        let reader = m.word(scratch + 4);
        frontend::fill_bytes(m.bytes, scratch + 8, 0, 16);
        let mut result;
        loop {
            let runtime = host.reader_runtime();
            result = crate::render_part_a::open_reader_chunk(
                m,
                &mut BrokerHost(host),
                reader,
                scratch + 8,
                scratch + 16,
                scratch + 20,
                &runtime,
            );
            if result >= 0x80000000 {
                break;
            }
            let mut i = 0u16;
            while i < half(m, config + 16) {
                let handler = m.word(config + 12) + u32::from(i) * 16;
                if normalization::strcmp_at(m.bytes, m.word(handler), scratch + 8) == 0 {
                    break;
                }
                i = i.wrapping_add(1);
            }
            if i < half(m, config + 16)
                && (filter == 0 || normalization::strcmp_at(m.bytes, filter, scratch + 8) == 0)
            {
                let handler = m.word(config + 12) + u32::from(i) * 16;
                result = host.resource_call(
                    m,
                    m.word(handler + 4),
                    &[
                        m.word(handler + 12),
                        reader,
                        m.word(scratch + 16),
                        m.word(scratch + 20),
                        mode,
                    ],
                );
                if result >= 0x80000000 {
                    m.put_word(scratch + 24, env.loading_type);
                    m.put_word(scratch + 28, scratch + 8);
                    crate::boot_part_b::fn_173c70(
                        m,
                        m.word(owner + 16),
                        env.log_category,
                        0x708,
                        env.log_format,
                        scratch + 24,
                        scratch + 32,
                        host,
                    );
                }
            }
            let runtime = host.reader_runtime();
            let rc = crate::render_part_a::close_reader_chunk(
                m,
                &mut BrokerHost(host),
                reader,
                &runtime,
            );
            if rc >= 0x80000000 && result < 0x80000000 {
                result = rc;
            }
            if result >= 0x80000000 {
                break;
            }
        }
        if result & 0x1fff == 20 {
            result = 0;
        }
        let runtime = host.reader_runtime();
        let rc = crate::render_part_a::close_reader(m, &mut BrokerHost(host), reader, &runtime);
        if rc >= 0x80000000 && result < 0x80000000 {
            result = rc;
        }
        result
    })();
    m.bytes[at..at + 96].copy_from_slice(&saved);
    rc
}
struct FetchHost<'a, H> {
    host: &'a mut H,
    env: &'a render_part_c::Environment,
}
impl<H: ContextHost> normalization::NormalizationHost for FetchHost<'_, H> {
    fn resolve(&mut self, bytes: &mut [u8], a: u32, b: u32) -> (u32, u32) {
        resolve(&Memory { base: 0, bytes }, [a, b], self.env)
    }
    fn calloc(&mut self, bytes: &mut [u8], heap: u32, n: u32, size: u32) -> Option<u32> {
        nz(frontend::call_method(bytes, nz(heap), n, size, self.host))
    }
    fn alloc(&mut self, bytes: &mut [u8], heap: u32, size: u32) -> Option<u32> {
        nz(frontend::heap_alloc(bytes, nz(heap), size, self.host))
    }
    fn realloc(&mut self, bytes: &mut [u8], heap: u32, p: u32, size: u32) -> Option<u32> {
        nz(frontend::heap_realloc(bytes, nz(heap), p, size, self.host))
    }
    fn free(&mut self, bytes: &mut [u8], heap: u32, p: u32) {
        frontend::heap_release(bytes, nz(heap), nz(p), self.host)
    }
}
/// fn_2c0e30. Apply the UTF-16 marker value as a property string.
/// scratch is a disjoint 24-byte caller-owned setter stack area, restored.
/// The lock, tree operations and callbacks are the setter's actual boundaries.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c0e30(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    marker: u32,
    name: u32,
    scratch: u32,
    env: &Environment<'_>,
    host: &mut impl ResourceHost,
    lock: &mut impl frontend::StateLock,
    ops: &mut impl frontend::TreeOps,
    before: impl frontend::BeforeChange,
    callback: impl frontend::ElementCallback,
    cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    assert_eq!(m.base, 0);
    let (rc, engine) = resolve(m, [m.word(ctx), m.word(ctx + 4)], &env.engine);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    let mut array = 0;
    let rc = normalization::fetch_entry(
        m.bytes,
        handle,
        0x3f4,
        1,
        Some(&mut array),
        &mut FetchHost {
            host,
            env: &env.engine,
        },
    );
    let text = if rc < 0x80000000 {
        m.word(array + 4)
    } else {
        0
    };
    let src = text.wrapping_add(m.word(marker + 28).wrapping_mul(2));
    let length = frontend::u16_len(m.bytes, src) as u16;
    let heap = m.word(engine + 4);
    let value = frontend::call_method(m.bytes, nz(heap), 1, u32::from(length) + 1, host);
    if value == 0 {
        return 0x8110200a;
    }
    crate::fe_text::narrow_n_17649c(m.bytes, value, src, u32::from(length));
    m.bytes[(value + u32::from(length)) as usize] = 0;
    let at = m.offset(scratch);
    let mut saved = [0u8; 24];
    saved.copy_from_slice(&m.bytes[at..at + 24]);
    let props = m.word(engine + 20);
    let result = frontend::set_param_str(
        m.bytes,
        nz(props),
        name,
        value,
        0,
        scratch,
        lock,
        host,
        ops,
        cmp,
        before,
        callback,
    );
    m.bytes[at..at + 24].copy_from_slice(&saved);
    if result < 0x80000000 {
        // fn_1738e8 has no direct Rust port. Null log is a real no-op.
        if m.word(engine + 16) != 0 {
            host.resource_call(
                m,
                0x1738e8,
                &[
                    m.word(engine + 16),
                    u32::from(half(m, marker + 10)),
                    name,
                    value,
                ],
            );
        }
    }
    frontend::heap_release(m.bytes, nz(heap), nz(value), host);
    result
}
fn c_string(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
}
fn build_name(m: &Memory<'_>, dst: &mut [u8; 8], prefix: &[u8], lang: u32) {
    dst.fill(0);
    let prefix = c_string(prefix);
    let n = normalization::strlen8(m.bytes, lang) as usize;
    assert!(
        prefix.len() + n < dst.len(),
        "pronunciation name exceeds original stack buffer"
    );
    // Project the immutable inputs and the original eight-byte stack output
    // into one local arena, then run the existing strcpy and strcat ports.
    let mut local = [0u8; 24];
    local[8..8 + prefix.len()].copy_from_slice(prefix);
    local[16..16 + n].copy_from_slice(&m.bytes[lang as usize..lang as usize + n]);
    frontend::copy_str(&mut local, 0, 8);
    frontend::append_str(&mut local, 0, 16);
    dst.copy_from_slice(&local[..8]);
}

/// Stack-independent parser inputs. The caller supplies the bytes of the
/// uninitialised marker temporary too: the ROM does not initialise its extra
/// word or structure padding before copying it to the marker arena.
pub struct ParserEnvironment<'a> {
    pub separators: &'a [u8],
    /// Region-specific boundary constants, required for modes 3 and 4.
    pub boundary: Option<&'a render_part_c::BoundaryData<'a>>,
    pub default_name: u32,
    pub parameters: [(u16, u32); 4],
    pub voice_key: u32,
    pub inserted_image: [u8; 40],
}
fn copy_bytes(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) {
    assert_eq!(m.base, 0);
    crate::fe_text::memmove_195b38(m.bytes, dst, src, n);
}
fn fetch_parser(
    m: &mut Memory<'_>,
    handle: u32,
    kind: u32,
    count: u32,
    env: &Environment<'_>,
    host: &mut impl ContextHost,
) -> (u32, u32) {
    let mut array = 0;
    let rc = normalization::fetch_entry(
        m.bytes,
        handle,
        kind,
        count,
        Some(&mut array),
        &mut FetchHost {
            host,
            env: &env.engine,
        },
    );
    (rc, array)
}
fn parameter_tag(tag: u16) -> bool {
    matches!(tag, 7 | 31 | 36 | 21 | 20)
}

/// fn_2c2564. Remove leading blanks and maintain the reference/reset markers.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c2564(
    m: &mut Memory<'_>,
    _ctx: u32,
    handle: u32,
    pos_ref: u32,
    removed: u32,
    esc_offset: u32,
    ref_reset: u32,
    reset: u32,
    env: &Environment<'_>,
    parser: &ParserEnvironment<'_>,
    host: &mut impl ContextHost,
) -> u32 {
    assert_eq!(m.base, 0);
    m.put_half(removed, 0);
    m.put_half(esc_offset, 0);
    m.put_word(ref_reset, 0);
    m.put_half(reset + 10, 0);
    let (rc, text) = fetch_parser(m, handle, 0x3fc, 1, env, host);
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, mut markers) = fetch_parser(m, handle, 0x3fd, 1, env, host);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut blanks = 0;
    let mut index = 0;
    let rc = normalization::scan_table(
        m.bytes,
        text,
        Some(&mut blanks),
        markers,
        Some(&mut index),
        0,
        parser.separators,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, p) = fetch_parser(m, handle, 0x3fd, 1, env, host);
    if rc >= 0x80000000 {
        return rc;
    }
    markers = p;
    let mut base = m.word(markers + 4);
    let base_ref = m.word(base + 20);
    let mut add_blanks = true;
    let mut i = 0u16;
    while i < half(m, markers + 8) {
        let current = base + u32::from(i) * 40;
        let tag = half(m, current + 10);
        if parameter_tag(tag) && m.bytes[current as usize] == 0 {
            let n = u32::from(blanks);
            let first = m.word(base + 20);
            let width = u32::from(half(m, text + 2));
            let limit = n.wrapping_mul(width).wrapping_add(first);
            if limit > m.word(current + 20) {
                let mut last = i;
                let mut j = i.wrapping_add(1);
                while j < half(m, markers + 8) {
                    let next = base + u32::from(j) * 40;
                    if parameter_tag(half(m, next + 10))
                        && m.bytes[next as usize] == 0
                        && half(m, next + 10) == tag
                        && limit > m.word(next + 20)
                    {
                        last = j;
                    }
                    j = j.wrapping_add(1);
                }
                let last_marker = base + u32::from(last) * 40;
                if m.word(last_marker + 20) >= first
                    && frontend::divmod_u32(m.word(last_marker + 20).wrapping_sub(first), width).0
                        < n
                {
                    index = last;
                    crate::fe_text::memcpy_195ae8(m.bytes, reset, last_marker, 40);
                    let delta = m.word(last_marker + 12).wrapping_sub(m.word(base + 12));
                    m.put_word(ref_reset, delta);
                    m.put_word(base + 16, m.word(base + 16).wrapping_sub(delta));
                    m.put_word(base + 12, m.word(last_marker + 12));
                    let width = u32::from(half(m, text + 2));
                    let value = n
                        .wrapping_sub(
                            frontend::divmod_u32(
                                m.word(last_marker + 20).wrapping_sub(m.word(base + 20)),
                                width,
                            )
                            .0,
                        )
                        .wrapping_mul(width)
                        .wrapping_add(m.word(last_marker + 16));
                    m.put_word(pos_ref, value);
                    let mut j = last.wrapping_add(1);
                    while j < half(m, markers + 8) {
                        let next = base + u32::from(j) * 40;
                        if (half(m, next + 10) == 999 || m.bytes[next as usize] == 0)
                            && n.wrapping_mul(u32::from(half(m, text + 2)))
                                .wrapping_add(base_ref)
                                >= m.word(next + 20)
                            && m.word(last_marker + 12) <= m.word(next + 12)
                        {
                            m.put_word(pos_ref, m.word(pos_ref).wrapping_add(m.word(next + 16)));
                        }
                        j = j.wrapping_add(1);
                    }
                    add_blanks = false;
                    // last is promoted to signed int in this C expression.
                    if i32::from(last) - 1 > 0 {
                        for j in 1..last {
                            let p = base + u32::from(j) * 40;
                            if half(m, p + 10) == 999 {
                                m.put_half(p + 10, 0x4000);
                            }
                        }
                    }
                } else if m.word(current + 12).wrapping_sub(m.word(base + 12))
                    == width.wrapping_mul(n)
                {
                    add_blanks = false;
                    let delta = u32::from(half(m, text + 2)).wrapping_mul(n);
                    m.put_word(base + 16, m.word(base + 16).wrapping_sub(delta));
                    m.put_word(base + 12, m.word(base + 12).wrapping_add(delta));
                    index = i;
                }
                break;
            }
        }
        i = i.wrapping_add(1);
    }
    if blanks == 0 && half(m, reset + 10) == 0 {
        return 0;
    }
    m.put_half(text + 8, half(m, text + 8).wrapping_sub(blanks));
    let data = m.word(text + 4);
    copy_bytes(
        m,
        data,
        data + u32::from(blanks) * 2,
        u32::from(half(m, text + 8)) * u32::from(half(m, text + 2)),
    );
    let (rc, p) = fetch_parser(
        m,
        handle,
        0x3fd,
        u32::from(half(m, markers + 8)) + 1,
        env,
        host,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    markers = p;
    base = m.word(markers + 4);
    let mut saved = [0u8; 40];
    if index == 1 {
        saved.copy_from_slice(&m.bytes[base as usize..base as usize + 40]);
        m.put_word(base + 16, 0);
        m.put_word(base + 24, 0);
    }
    let prev = base.wrapping_add(u32::from(index.wrapping_sub(1)) * 40);
    let delta = m
        .word(prev + 12)
        .wrapping_add(m.word(prev + 16))
        .wrapping_sub(m.word(prev + 24).wrapping_add(m.word(prev + 20)));
    let mut inserted = parser.inserted_image;
    inserted[0] = 0;
    inserted[4..8].copy_from_slice(&0u32.to_le_bytes());
    inserted[8..10].copy_from_slice(&0u16.to_le_bytes());
    inserted[10..12].copy_from_slice(&0x4000u16.to_le_bytes());
    let len = u32::from(blanks)
        .wrapping_mul(u32::from(half(m, text + 2)))
        .wrapping_add(m.word(base + 20))
        .wrapping_add(delta)
        .wrapping_sub(m.word(base + 12));
    inserted[12..16].copy_from_slice(&m.word(base + 12).to_le_bytes());
    inserted[16..20].copy_from_slice(&len.to_le_bytes());
    inserted[20..24].copy_from_slice(&m.word(base + 20).to_le_bytes());
    inserted[24..28].fill(0);
    inserted[32..36].fill(0);
    inserted[36] = 0;
    if index == 1 {
        m.bytes[base as usize..base as usize + 40].copy_from_slice(&saved);
    }
    index = 1;
    while index < half(m, markers + 8) {
        let p = base + u32::from(index) * 40;
        let ins_word = |i: usize| u32::from_le_bytes(inserted[i..i + 4].try_into().unwrap());
        if ins_word(20) != m.word(p + 20)
            || m.word(p + 24) != 0
            || ins_word(12) != m.word(p + 12)
            || ins_word(16) <= m.word(p + 16)
        {
            break;
        }
        index = index.wrapping_add(1);
    }
    let at = base + u32::from(index) * 40;
    copy_bytes(
        m,
        at + 40,
        at,
        u32::from(half(m, markers + 8).wrapping_sub(index)) * 40,
    );
    m.bytes[at as usize..at as usize + 40].copy_from_slice(&inserted);
    m.put_half(markers + 8, half(m, markers + 8).wrapping_add(1));
    let delta = u32::from(half(m, text + 2)) * u32::from(blanks);
    m.put_half(removed, delta as u16);
    if add_blanks {
        m.put_word(pos_ref, m.word(pos_ref).wrapping_add(delta));
    }
    if half(m, reset + 10) == 0 {
        for index in 1..half(m, markers + 8) {
            let p = base + u32::from(index) * 40;
            if m.word(p + 20) > m.word(base + 20)
                && m.word(p + 20) <= delta.wrapping_add(m.word(base + 20))
                && half(m, p + 10) != 0x4000
                && m.bytes[p as usize] == 0
            {
                m.put_word(pos_ref, m.word(pos_ref).wrapping_add(m.word(p + 16)));
                m.put_half(
                    esc_offset,
                    half(m, esc_offset).wrapping_add(m.word(p + 16) as u16),
                );
            }
        }
    }
    for index in 1..half(m, markers + 8) {
        let p = base + u32::from(index) * 40;
        let value = if m.word(p + 20) < delta.wrapping_add(m.word(base + 20)) {
            m.word(base + 20)
        } else {
            m.word(p + 20).wrapping_sub(delta)
        };
        m.put_word(p + 20, value);
    }
    m.put_word(base + 24, m.word(base + 24).wrapping_sub(delta));
    0
}

/// Caller-owned adapters for the property routines, not engine-call scripts.
/// Tree, lock, before-change and element callback objects are the same
/// boundaries used by frontend's real setter and removal ports.
pub struct ParserRuntime<'a, 'b, H, L, O, B, E, C> {
    pub env: &'a Environment<'b>,
    pub parser: &'a ParserEnvironment<'b>,
    pub host: &'a mut H,
    pub lock: &'a mut L,
    pub ops: &'a mut O,
    pub before: &'a mut B,
    pub callback: &'a mut E,
    pub compare: &'a mut C,
    /// 64 disjoint writable bytes replacing stack locals, restored on return.
    pub scratch: u32,
}
struct BorrowBefore<'a, B>(&'a mut B);
impl<B: frontend::BeforeChange> frontend::BeforeChange for BorrowBefore<'_, B> {
    fn before(&mut self, c: u32, k: u32, d: u32, a: &mut u8) -> u32 {
        self.0.before(c, k, d, a)
    }
}
struct BorrowElement<'a, E>(&'a mut E);
impl<E: frontend::ElementCallback> frontend::ElementCallback for BorrowElement<'_, E> {
    fn call_element(&mut self, a: u32, q: u32, r: u32) -> u32 {
        self.0.call_element(a, q, r)
    }
}
impl<H, L, O, B, E, C> ParserRuntime<'_, '_, H, L, O, B, E, C>
where
    H: ResourceHost + crate::render_part_b::ParameterHost,
    L: frontend::StateLock,
    O: frontend::TreeOps,
    B: frontend::BeforeChange,
    E: frontend::ElementCallback,
    C: FnMut(u32, u32) -> i32,
{
    fn set(&mut self, m: &mut Memory<'_>, props: u32, key: u32, value: u32, integer: bool) -> u32 {
        let s = self.scratch;
        let mut saved = [0; 64];
        saved.copy_from_slice(&m.bytes[s as usize..s as usize + 64]);
        let rc = if integer {
            frontend::set_param_uint(
                m.bytes,
                nz(props),
                key,
                value,
                0,
                s,
                self.lock,
                self.host,
                self.ops,
                &mut *self.compare,
                BorrowBefore(self.before),
                BorrowElement(self.callback),
            )
        } else {
            frontend::set_param_str(
                m.bytes,
                nz(props),
                key,
                value,
                0,
                s,
                self.lock,
                self.host,
                self.ops,
                &mut *self.compare,
                BorrowBefore(self.before),
                BorrowElement(self.callback),
            )
        };
        m.bytes[s as usize..s as usize + 64].copy_from_slice(&saved);
        rc
    }
    fn clear(&mut self, m: &mut Memory<'_>, props: u32, key: u32) -> u32 {
        let s = self.scratch;
        let mut saved = [0; 64];
        saved.copy_from_slice(&m.bytes[s as usize..s as usize + 64]);
        let rc = crate::render_part_b::fn_175a24(m, props, key, s, self.host);
        m.bytes[s as usize..s as usize + 64].copy_from_slice(&saved);
        if rc & 0x1fff == 0x14 || rc < 0x80000000 {
            0
        } else {
            rc
        }
    }
    fn voice(&mut self, m: &mut Memory<'_>, ctx: u32, handle: u32, marker: u32) -> u32 {
        fn_2c0e30(
            m,
            ctx,
            handle,
            marker,
            self.parser.voice_key,
            self.scratch,
            self.env,
            self.host,
            self.lock,
            self.ops,
            BorrowBefore(self.before),
            BorrowElement(self.callback),
            &mut *self.compare,
        )
    }
}

/// fn_2c1e2c. Apply the final parameter marker of each of the four kinds.
pub fn fn_2c1e2c<H, L, O, B, E, C>(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    rt: &mut ParserRuntime<'_, '_, H, L, O, B, E, C>,
) -> u32
where
    H: ResourceHost + crate::render_part_b::ParameterHost,
    L: frontend::StateLock,
    O: frontend::TreeOps,
    B: frontend::BeforeChange,
    E: frontend::ElementCallback,
    C: FnMut(u32, u32) -> i32,
{
    let (rc, engine) = resolve(m, [m.word(ctx), m.word(ctx + 4)], &rt.env.engine);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    let (rc, array) = fetch_parser(m, handle, 0x3f4, 1, rt.env, rt.host);
    let params = if rc < 0x80000000 {
        m.word(array + 4)
    } else {
        0
    };
    let (rc, array) = fetch_parser(m, handle, 0x3fd, 1, rt.env, rt.host);
    if rc >= 0x80000000 {
        return rc;
    }
    let props = m.word(engine + 20);
    let heap = m.word(engine + 4);
    let base = m.word(array + 4);
    let mut done = [false; 4];
    let mut result = rc;
    for k in 1..half(m, array + 8) {
        let p = base + u32::from(k) * 40;
        let tag = half(m, p + 10);
        if !matches!(tag, 7 | 31 | 36 | 21) {
            continue;
        }
        let Some(t) = rt.parser.parameters.iter().position(|e| e.0 == tag) else {
            continue;
        };
        if done[t] {
            continue;
        }
        let mut value = m.word(p + 28);
        let mut toggle = value == 0x2b;
        for j in k + 1..half(m, array + 8) {
            let next = base + u32::from(j) * 40;
            if half(m, next + 10) != tag {
                continue;
            }
            if tag == 7 {
                if m.word(next + 28) == 0x2b {
                    toggle = !toggle;
                    if !toggle {
                        value = 15;
                    }
                } else {
                    toggle = false;
                    value = m.word(next + 28);
                }
            } else {
                value = m.word(next + 28);
            }
        }
        if tag == 7 && toggle {
            value = 15;
        }
        let key = rt.parser.parameters[t].1;
        let clear = match tag {
            7 => value == 15,
            31 => value == m.word(ctx + 96),
            _ => {
                params != 0
                    && crate::fe_text::strcmp16_1765ac(
                        m.bytes,
                        params.wrapping_add(value * 2),
                        rt.parser.default_name,
                    ) == 0
            }
        };
        result = if clear {
            rt.clear(m, props, key)
        } else if matches!(tag, 7 | 31) {
            rt.set(m, props, key, value, true)
        } else {
            let src = params.wrapping_add(value.wrapping_mul(2));
            let len = frontend::u16_len(m.bytes, src) as u16;
            let buf = frontend::call_method(m.bytes, nz(heap), 1, u32::from(len) + 1, rt.host);
            if buf == 0 {
                return 0x8110200a;
            }
            crate::fe_text::narrow_n_17649c(m.bytes, buf, src, u32::from(len));
            m.bytes[(buf + u32::from(len)) as usize] = 0;
            let rc = rt.set(m, props, key, buf, false);
            frontend::heap_release(m.bytes, nz(heap), nz(buf), rt.host);
            rc
        };
        if result >= 0x80000000 {
            return result;
        }
        done[t] = true;
    }
    for (t, was_done) in done.iter().enumerate() {
        if !was_done {
            result = rt.clear(m, props, rt.parser.parameters[t].1);
            if result >= 0x80000000 {
                return result;
            }
        }
    }
    result
}
fn parser_last(m: &mut Memory<'_>, base: u32, j: u16, count: u16) -> u32 {
    let p = base + u32::from(j) * 40;
    let tag = half(m, p + 10);
    let mut value = m.word(p + 28);
    let mut toggle = tag == 7 && value == 0x2b;
    for k in j + 1..count {
        let next = base + u32::from(k) * 40;
        if half(m, next + 10) == tag {
            if tag == 7 && m.word(next + 28) == 0x2b {
                toggle = !toggle;
                if !toggle {
                    value = 15;
                }
            } else {
                toggle = false;
                value = m.word(next + 28);
            }
            m.put_half(next + 10, 999);
        }
    }
    if toggle {
        15
    } else {
        value
    }
}
fn parser_boundary(
    m: &mut Memory<'_>,
    text: u32,
    markers: u32,
    te: &mut u16,
    me: &mut u16,
) -> bool {
    if text == 0 || markers == 0 {
        return false;
    }
    let base = m.word(markers + 4);
    let stop = |m: &Memory<'_>, i: u16| {
        let p = base + u32::from(i) * 40;
        matches!(half(m, p + 10), 2 | 3) && m.word(p + 28) == 1
    };
    let mut i = 0u16;
    while i < *me && !stop(m, i) {
        i += 1;
    }
    if i >= *me {
        return false;
    }
    while i < *me && !matches!(half(m, base + u32::from(i) * 40 + 10), 26 | 29 | 40 | 8) {
        i += 1;
    }
    if i >= *me {
        return false;
    }
    while i < *me && !(stop(m, i) || matches!(half(m, base + u32::from(i) * 40 + 10), 12 | 20)) {
        i += 1;
    }
    if i >= *me {
        return false;
    }
    *te = frontend::divmod_u32(
        m.word(base + u32::from(i) * 40 + 20)
            .wrapping_sub(m.word(base + 20)) as u16 as u32,
        u32::from(half(m, text + 2)),
    )
    .0 as u16;
    *me = i;
    true
}

/// fn_2c0f74. Split a sentence, preserve parameters and trim/reset its references.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c0f74<H, L, O, B, E, C>(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    length: u32,
    position: u32,
    voice_switch: u32,
    immediate: u32,
    mode: u32,
    pos_ref: u32,
    ref_reset: u32,
    reset: u32,
    rt: &mut ParserRuntime<'_, '_, H, L, O, B, E, C>,
) -> u32
where
    H: ResourceHost + crate::render_part_b::ParameterHost,
    L: frontend::StateLock,
    O: frontend::TreeOps,
    B: frontend::BeforeChange,
    E: frontend::ElementCallback,
    C: FnMut(u32, u32) -> i32,
{
    if handle == 0 {
        return 0x81102007;
    }
    let (rc, engine) = resolve(m, [m.word(ctx), m.word(ctx + 4)], &rt.env.engine);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    m.put_word(pos_ref, 0);
    if half(m, ctx + 48) == 0 {
        return 0x81102007;
    }
    let (rc, text) = fetch_parser(m, handle, 0x3f2, 1, rt.env, rt.host);
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, array) = fetch_parser(m, handle, 0x3f4, 1, rt.env, rt.host);
    let params = if rc < 0x80000000 {
        m.word(array + 4)
    } else {
        0
    };
    let (rc, markers) = fetch_parser(m, handle, 0x3f3, 1, rt.env, rt.host);
    if rc >= 0x80000000 {
        return rc;
    }
    let base = m.word(markers + 4);
    let mut te = 0;
    let mut me = 0;
    let rc = normalization::scan_table(
        m.bytes,
        text,
        Some(&mut te),
        markers,
        Some(&mut me),
        0,
        rt.parser.separators,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    for i in 1..half(m, markers + 8) {
        let p = base + u32::from(i) * 40;
        if m.word(p + 20) > u32::from(te) * 2 + m.word(base + 20) {
            break;
        }
        if half(m, p + 10) == 20 {
            m.put_half(mode, m.word(p + 28) as u16);
        }
    }
    if !parser_boundary(m, text, markers, &mut te, &mut me) {
        let s = rt.scratch;
        let saved = m.word(s);
        m.put_half(s, te);
        m.put_half(s + 2, me);
        let rc = match half(m, mode) {
            2 | 3 | 4 => {
                if half(m, mode) == 2 {
                    render_part_c::fn_2be178(m, engine, text, markers, s, s + 2)
                } else {
                    let data = rt
                        .parser
                        .boundary
                        .expect("parser modes 3/4 require BoundaryData");
                    let properties = m.word(engine + 20);
                    if properties != 0 {
                        rt.host.select_lock(m.word(properties + 4));
                    }
                    if half(m, mode) == 3 {
                        render_part_c::fn_2be1fc(
                            m,
                            engine,
                            text,
                            markers,
                            s,
                            s + 2,
                            data,
                            rt.host,
                            &mut *rt.compare,
                        )
                    } else {
                        render_part_c::fn_2be4a0(
                            m,
                            engine,
                            text,
                            markers,
                            s,
                            s + 2,
                            data,
                            rt.host,
                            &mut *rt.compare,
                        )
                    }
                }
            }
            _ => render_part_c::fn_2be754(m, engine, text, markers, s, s + 2, rt.parser.separators),
        };
        te = half(m, s);
        me = half(m, s + 2);
        m.put_word(s, saved);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let mut nonempty = 0u16;
    let mut switch_index = 0;
    let mut i = 1u16;
    while i < me {
        let p = base + u32::from(i) * 40;
        if half(m, p + 10) != 0x4000 {
            nonempty = nonempty.wrapping_add(1);
            if half(m, p + 10) == 22 {
                m.bytes[voice_switch as usize] = 1;
                me = i;
                switch_index = i;
                if nonempty == i && m.word(p + 20) == m.word(base + 20) {
                    m.bytes[immediate as usize] = 1;
                    let rc = rt.voice(m, ctx, handle, p);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    me = i + 1;
                } else {
                    m.bytes[immediate as usize] = 0;
                }
                te = if me < half(m, markers + 8) {
                    frontend::divmod_u32(
                        m.word(p + 20).wrapping_sub(m.word(base + 20)) as u16 as u32,
                        u32::from(half(m, text + 2)),
                    )
                    .0 as u16
                } else {
                    0
                };
                break;
            }
        }
        nonempty = nonempty.wrapping_add(1);
        i = i.wrapping_add(1);
    }
    let (rc, out_text) = fetch_parser(
        m,
        handle,
        0x3fc,
        if te == 0 { 1 } else { u32::from(te) },
        rt.env,
        rt.host,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, out_markers) = fetch_parser(m, handle, 0x3fd, u32::from(me) + 1, rt.env, rt.host);
    if rc >= 0x80000000 {
        return rc;
    }
    copy_bytes(
        m,
        m.word(out_text + 4),
        m.word(text + 4),
        u32::from(te) * u32::from(half(m, text + 2)),
    );
    m.put_half(out_text + 8, te);
    m.put_half(text + 8, half(m, text + 8).wrapping_sub(te));
    copy_bytes(
        m,
        m.word(text + 4),
        m.word(text + 4) + u32::from(te) * 2,
        u32::from(half(m, text + 8)) * u32::from(half(m, text + 2)),
    );
    m.put_word(length, u32::from(half(m, out_text + 8)));
    if m.bytes[voice_switch as usize] == 1 && m.bytes[immediate as usize] == 0 {
        let mut blanks = 0;
        let mut blank_markers = 0;
        let rc = normalization::scan_table(
            m.bytes,
            out_text,
            Some(&mut blanks),
            out_markers,
            Some(&mut blank_markers),
            0,
            rt.parser.separators,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        if half(m, out_text + 8) == blanks {
            let found = (1..me)
                .any(|j| !matches!(half(m, base + u32::from(j) * 40 + 10), 2 | 7 | 31 | 36 | 21));
            if found {
                m.bytes[voice_switch as usize] = 0;
                me = i;
            } else {
                m.bytes[immediate as usize] = 1;
                let rc = rt.voice(m, ctx, handle, base + u32::from(switch_index) * 40);
                if rc >= 0x80000000 {
                    return rc;
                }
                me = i + 1;
            }
        }
    }
    copy_bytes(
        m,
        m.word(out_markers + 4),
        m.word(markers + 4),
        u32::from(me) * 40,
    );
    m.put_half(out_markers + 8, me);
    let end = u8::from(
        m.bytes[(text + 12) as usize] != 0
            && half(m, text + 8) == 0
            && m.bytes[(markers + 12) as usize] != 0
            && half(m, markers + 8) == me,
    );
    m.bytes[(out_text + 12) as usize] = end;
    m.bytes[(out_markers + 12) as usize] = end;
    let next = base + u32::from(me) * 40;
    let delta = u32::from(te) * u32::from(half(m, text + 2));
    let reset_position =
        me < half(m, markers + 8) && m.word(next + 20) <= delta.wrapping_add(m.word(base + 20));
    if reset_position {
        m.put_word(base + 12, m.word(next + 12));
        m.put_word(base + 20, m.word(next + 20));
    } else {
        m.put_word(base + 12, m.word(base + 12).wrapping_add(delta));
        m.put_word(base + 20, m.word(base + 20).wrapping_add(delta));
    }
    let mut kept = 1u16;
    for j in 1..me {
        let p = base + u32::from(j) * 40;
        let tag = half(m, p + 10);
        if tag == 2 {
            continue;
        }
        m.put_word(
            position,
            m.word(position)
                .wrapping_add(frontend::divmod_u32(m.word(p + 16), 2).0),
        );
        if matches!(tag, 7 | 31 | 36 | 21) {
            let value = parser_last(m, base, j, me);
            let keep = match tag {
                7 => value != 15,
                31 => value != m.word(ctx + 96),
                _ => {
                    params == 0
                        || crate::fe_text::strcmp16_1765ac(
                            m.bytes,
                            params.wrapping_add(value * 2),
                            rt.parser.default_name,
                        ) != 0
                }
            };
            if keep {
                let out = base + u32::from(kept) * 40;
                m.put_half(out + 10, half(m, p + 10));
                m.put_word(out + 12, m.word(base + 12));
                m.put_word(out + 16, 0);
                m.put_word(out + 20, m.word(base + 20));
                m.put_word(out + 24, 0);
                m.put_word(out + 28, value);
                m.bytes[out as usize] = 1;
                kept += 1;
            }
        }
    }
    if kept < me {
        m.put_half(markers + 8, half(m, markers + 8).wrapping_sub(me - kept));
        if half(m, markers + 8) > 1 {
            copy_bytes(
                m,
                m.word(markers + 4) + u32::from(kept) * 40,
                m.word(markers + 4) + u32::from(me) * 40,
                u32::from(half(m, markers + 8).wrapping_sub(kept)) * 40,
            );
        }
    }
    let base = m.word(out_markers + 4);
    let mut j = me;
    loop {
        j = j.wrapping_sub(1);
        if j < 2 || m.bytes[(base + u32::from(j) * 40) as usize] != 1 {
            break;
        }
    }
    let last = base + u32::from(j) * 40;
    m.put_word(
        base + 16,
        delta
            .wrapping_add(m.word(base + 20))
            .wrapping_add(m.word(last + 12).wrapping_add(m.word(last + 16)))
            .wrapping_sub(m.word(last + 24).wrapping_add(m.word(last + 20)))
            .wrapping_sub(m.word(base + 12)),
    );
    m.put_word(base + 24, delta);
    te = 0;
    for j in 0..half(m, out_markers + 8) {
        let p = base + u32::from(j) * 40;
        let tag = half(m, p + 10);
        if (tag == 3
            && (m.word(p + 28) == 0
                || m.word(p + 20).wrapping_sub(m.word(base + 20)) < m.word(base + 24)))
            || !matches!(tag, 2 | 3)
        {
            let out = base + u32::from(te) * 40;
            copy_bytes(m, out, p, 40);
            if (half(m, out + 10) == 3 && m.word(out + 28) == 0)
                || (half(m, p + 10) == 3
                    && m.word(p + 20).wrapping_sub(m.word(base + 20)) < m.word(base + 24))
            {
                m.put_half(out + 10, 999);
                m.put_word(out + 28, 0);
            }
            if half(m, out + 10) == 3 {
                m.put_half(out + 10, 2);
            }
            te = te.wrapping_add(1);
        }
        if m.word(p + 20) <= m.word(base + 20)
            && half(m, p + 10) != 0x4000
            && m.bytes[p as usize] == 0
        {
            m.put_word(pos_ref, m.word(pos_ref).wrapping_add(m.word(p + 16)));
        }
    }
    m.put_half(out_markers + 8, te);
    let s = rt.scratch;
    let saved = m.word(s);
    let rc = fn_2c2564(
        m,
        ctx,
        handle,
        pos_ref,
        s,
        s + 2,
        ref_reset,
        reset,
        rt.env,
        rt.parser,
        rt.host,
    );
    m.put_word(s, saved);
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, out_markers) = fetch_parser(m, handle, 0x3fd, 1, rt.env, rt.host);
    if rc >= 0x80000000 {
        return rc;
    }
    let base = m.word(out_markers + 4);
    let pos = m.word(base + 12);
    let len = m.word(base + 16);
    let (rc, markers) = fetch_parser(m, handle, 0x3f3, 1, rt.env, rt.host);
    if rc >= 0x80000000 {
        return rc;
    }
    let base = m.word(markers + 4);
    if !reset_position {
        m.put_word(base + 12, pos.wrapping_add(len));
        for j in 1..half(m, markers + 8) {
            let p = base + u32::from(j) * 40;
            if m.word(p + 20) == m.word(base + 20) && matches!(half(m, p + 10), 7 | 31 | 36 | 21) {
                m.put_word(p + 12, m.word(base + 12));
            }
        }
    }
    0
}

pub(crate) fn registry_acquire(
    m: &mut Memory<'_>,
    registry: u32,
    name: u32,
    scratch: u32,
    host: &mut impl crate::render_part_d::DictionaryHost,
) -> (u32, u32) {
    let (lock, find, found) =
        crate::render_part_d::dictionary_registry(m.bytes, registry, name, host);
    let saved = m.word(scratch);
    let scripts = frontend::AcquireScripts {
        lock,
        find,
        found,
        unlock: 0,
    };
    let mut rc =
        frontend::acquire_ling_entry(m.bytes, nz(registry), nz(name), Some(scratch), &scripts);
    let entry = if rc < 0x80000000 { m.word(scratch) } else { 0 };
    m.put_word(scratch, saved);
    if registry != 0 && name != 0 && lock < 0x80000000 {
        let r = crate::render_part_d::dictionary_lock(m.bytes, registry, false, host);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    (rc, entry)
}
fn registry_remove(
    m: &mut Memory<'_>,
    registry: u32,
    name: u32,
    host: &mut impl crate::render_part_d::DictionaryHost,
) -> u32 {
    let map = m.word(registry + 16);
    let mut remove = None;
    let rc = frontend::remove_tree_value(
        m.bytes,
        nz(map),
        name,
        |a, b| {
            let vt = m.word(map + 60);
            host.compare_key(m.bytes, m.word(vt + 8), a, b)
        },
        |tree, node| remove = Some((tree, node)),
    );
    if let Some((tree, node)) = remove {
        crate::render_part_d::dictionary_release_node(m.bytes, tree, node, host);
    }
    rc
}
pub fn registry_release(
    m: &mut Memory<'_>,
    registry: u32,
    name: u32,
    drop: bool,
    host: &mut impl crate::render_part_d::DictionaryHost,
) -> u32 {
    let (lock, find, found) =
        crate::render_part_d::dictionary_registry(m.bytes, registry, name, host);
    let scripts = frontend::ReleaseScripts {
        lock,
        find,
        found,
        remove: 0,
        unlock: 0,
    };
    let mut pending = None;
    let has_cb = found.is_some_and(|entry| m.word(entry + 24) != 0);
    let mut rc = if drop {
        frontend::drop_ling_entry(m.bytes, nz(registry), nz(name), &scripts)
    } else if has_cb {
        frontend::release_ling_entry(
            m.bytes,
            nz(registry),
            nz(name),
            &scripts,
            Some(|a, b, e: &mut [u8; 32]| pending = Some(([a, b], *e))),
        )
    } else {
        frontend::release_ling_entry(
            m.bytes,
            nz(registry),
            nz(name),
            &scripts,
            None::<fn(u32, u32, &mut [u8; 32])>,
        )
    };
    if let Some((owner, mut entry)) = pending {
        rc = registry_remove(m, registry, name, host);
        host.release_callback(m.bytes, owner, &mut entry);
    } else if drop
        && registry != 0
        && name != 0
        && lock < 0x80000000
        && find < 0x80000000
        && m.word(registry + 16) != 0
        && half(m, found.unwrap() + 28) == 0
    {
        rc = registry_remove(m, registry, name, host);
    }
    if registry != 0 && name != 0 && lock < 0x80000000 {
        let r = crate::render_part_d::dictionary_lock(m.bytes, registry, false, host);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    rc
}
/// fn_2c7f24. Close resource objects, then drop registrations in reverse order.
pub fn fn_2c7f24(
    m: &mut Memory<'_>,
    handle: [u32; 2],
    list: u32,
    scratch: u32,
    env: &Environment<'_>,
    host: &mut (impl ResourceHost + crate::render_part_d::DictionaryHost),
) -> u32 {
    let (mut rc, context) = resolve(m, handle, &env.engine);
    if rc >= 0x80000000 {
        return rc;
    }
    let saved = m.word(scratch);
    for pass in 1..=2 {
        let mut i = normalization::idx5(m.bytes, list);
        while i != 0 && normalization::span_at(m.bytes, list, i - 1, scratch) != 0 {
            let elem = m.word(scratch);
            let name = m.word(elem);
            let (r, res) = registry_acquire(m, m.word(context + 24), name, scratch, host);
            if r < 0x80000000 {
                registry_release(m, m.word(context + 24), name, false, host);
                let r = if pass == 1 {
                    let iface = m.word(res + 4);
                    let r = host.resource_call(
                        m,
                        m.word(iface + 20),
                        &[m.word(res + 8), m.word(res + 12)],
                    );
                    if r >= 0x80000000 {
                        r
                    } else {
                        frontend::release_broker_interface(nz(m.word(context + 12)), nz(iface))
                    }
                } else {
                    registry_release(m, m.word(context + 24), name, true, host)
                };
                if r >= 0x80000000 {
                    rc = r;
                }
            }
            i -= 1;
        }
    }
    m.put_word(scratch, saved);
    rc
}

pub struct PronunciationEnvironment<'a> {
    pub dictionary: &'a crate::render_part_d::DictionaryEnvironment,
    pub default_language: u32,
    pub language_filter: &'a [u16],
    pub stop_filter: &'a [u16],
    pub trim: &'a [u8],
    pub decoder: &'a normalization::DecoderTables<'a>,
    pub scratch: u32,
}
pub(crate) fn iterator_seek(m: &mut Memory<'_>, it: u32, index: i32) -> u32 {
    if it == 0 {
        return 0x81102007;
    }
    let arr = m.word(it + 12);
    let mut out = half(m, it + 16);
    let rc = normalization::set_index(half(m, arr + 8), index, &mut out);
    m.put_half(it + 16, out);
    rc
}
pub(crate) fn iterator_get(m: &Memory<'_>, it: u32, requested: u32) -> (u32, u32) {
    if it == 0 {
        return (0x81102007, 0);
    }
    let arr = m.word(it + 12);
    let mut out = 0;
    let rc = normalization::entry_offset(
        half(m, arr + 2),
        m.word(arr + 4) as i32,
        half(m, arr + 8),
        half(m, it + 16),
        requested,
        &mut out,
    );
    (rc, out as u32)
}
fn iterator_find(m: &mut Memory<'_>, it: u32, filter: &[u16], advance: bool) -> u32 {
    if it == 0 {
        return 0x81102007;
    }
    let arr = m.word(it + 12);
    let mut index = half(m, it + 16);
    let data = m.word(arr + 4) as usize;
    let count = half(m, arr + 8);
    let stride = half(m, arr + 2);
    let mut step = advance;
    let rc = normalization::scan_forward(&m.bytes[data..], stride, count, &mut index, &mut |p| {
        if step {
            step = false;
            return 0;
        }
        if advance {
            1
        } else {
            normalization::id_in_list(filter, u16::from_le_bytes([p[10], p[11]]))
        }
    });
    m.put_half(it + 16, index);
    rc
}
struct ConversionHost<'a, H> {
    host: &'a mut H,
    out: u32,
}
impl<H: ResourceHost> frontend::HeapVtable for ConversionHost<'_, H> {
    fn alloc_at(&mut self, a: u32, n: u32) -> u32 {
        self.host.alloc_at(a, n)
    }
    fn realloc_at(&mut self, a: u32, p: u32, n: u32) -> u32 {
        self.host.realloc_at(a, p, n)
    }
    fn release_at(&mut self, a: u32, p: u32) {
        self.host.release_at(a, p)
    }
}
impl<H: ResourceHost> frontend::MethodTable for ConversionHost<'_, H> {
    fn call_at(&mut self, a: u32, n: u32, s: u32) -> u32 {
        self.host.call_at(a, n, s)
    }
}
impl<H: ResourceHost + crate::render_part_d::DictionaryHost> render_part_c::Host
    for ConversionHost<'_, H>
{
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        render_part_c::Host::call(self.host, m, f, a)
    }
    fn output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        if f == 0x2eccb4 {
            let saved = m.word(self.out);
            let len =
                crate::render_part_d::dictionary_match(m.bytes, a[0], a[1], self.out, self.host);
            let value = m.word(self.out);
            m.put_word(self.out, saved);
            (len, value)
        } else {
            render_part_c::Host::output(self.host, m, f, a)
        }
    }
}
fn pronunciation_convert(
    m: &mut Memory<'_>,
    ctx: u32,
    mut text: u32,
    mut len: u32,
    typ: u32,
    env: &Environment<'_>,
    p: &PronunciationEnvironment<'_>,
    host: &mut (impl ResourceHost + crate::render_part_d::DictionaryHost),
) -> (u32, u32, u16) {
    if text == 0 {
        return (0x81102007, 0, 0);
    }
    let (rc, engine) = resolve(m, [m.word(ctx), m.word(ctx + 4)], &env.engine);
    if rc >= 0x80000000 || !matches!(typ, 70 | 71) {
        return (0x81102007, 0, 0);
    }
    let mut i = 0;
    while i < len && normalization::u16chr(p.trim, 0, half(m, text + i * 2)).is_some() {
        i += 1;
    }
    text += i * 2;
    len = len.wrapping_sub(i) as u16 as u32;
    i = 0;
    while i < len && normalization::u16chr(p.trim, 0, half(m, text + (len - i - 1) * 2)).is_some() {
        i += 1;
    }
    len = len.wrapping_sub(i) as u16 as u32;
    let heap = m.word(engine + 4);
    let mut copy = 0;
    let mut utf = 0;
    let mut output = 0;
    let mut output_len = 0;
    let rc = (|| {
        copy = frontend::heap_alloc(m.bytes, nz(heap), len * 2 + 2, host);
        if copy == 0 {
            return 0x8110200a;
        }
        crate::fe_text::memcpy_195ae8(m.bytes, copy, text, len * 2);
        m.put_half(copy + len * 2, 0);
        let bytes = normalization::utf8_len_u16(m.bytes, copy) as u16 as u32;
        utf = frontend::heap_alloc(m.bytes, nz(heap), bytes, host);
        if utf == 0 {
            return 0x8110200a;
        }
        let rc = normalization::transcode_u16(
            m.bytes,
            copy,
            len + 1,
            p.scratch,
            utf,
            bytes,
            p.scratch + 4,
            0,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let session = m.word(ctx + if typ == 70 { 8 } else { 12 });
        let capacity = p.scratch + 8;
        m.put_half(capacity, 0);
        let rc = render_part_c::fn_2c3308(
            m,
            session,
            utf,
            0,
            capacity,
            &mut ConversionHost {
                host,
                out: p.scratch + 12,
            },
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let converted = frontend::heap_alloc(m.bytes, nz(heap), u32::from(half(m, capacity)), host);
        if converted == 0 {
            return 0x8110200a;
        }
        let mut rc = render_part_c::fn_2c3308(
            m,
            session,
            utf,
            converted,
            capacity,
            &mut ConversionHost {
                host,
                out: p.scratch + 12,
            },
        );
        if rc < 0x80000000 {
            let bytes = (normalization::decoded_count(m.bytes, converted, p.decoder.extra) as u16
                as u32)
                * 2;
            output = frontend::heap_alloc(m.bytes, nz(heap), bytes, host);
            if output == 0 {
                rc = 0x8110200a;
            } else {
                output_len = normalization::decode_str(m.bytes, converted, output, p.decoder)
                    .wrapping_sub(1) as u16;
            }
        }
        frontend::heap_release(m.bytes, nz(heap), Some(converted), host);
        rc
    })();
    if utf != 0 {
        frontend::heap_release(m.bytes, nz(heap), Some(utf), host);
    }
    if copy != 0 {
        frontend::heap_release(m.bytes, nz(heap), Some(copy), host);
    }
    (rc, output, output_len)
}
fn pronunciation_language(
    m: &mut Memory<'_>,
    ctx: u32,
    event: u32,
    text: u32,
    env: &Environment<'_>,
    p: &PronunciationEnvironment<'_>,
    host: &mut (impl ResourceHost + crate::render_part_d::DictionaryHost),
) -> u32 {
    if ctx == 0 {
        return 0x81102007;
    }
    if event == 0 || text == 0 {
        return 0;
    }
    let at = text + m.word(event + 28) * 2;
    let language = p.scratch + 16;
    m.bytes[language as usize..language as usize + 4].fill(0);
    if crate::fe_text::strcmp16_1765ac(m.bytes, at, p.default_language) == 0 {
        let (rc, engine) = resolve(m, [m.word(ctx), m.word(ctx + 4)], &env.engine);
        if rc >= 0x80000000 {
            return 0;
        }
        if engine == 0 {
            // setLanguage deliberately suppresses lookup/setup errors.
            return 0;
        }
        let props = m.word(engine + 20);
        if props != 0 {
            host.select_lock(m.word(props + 4));
        }
        let mut value = 0;
        let rc = frontend::fetch_param(
            m.bytes,
            nz(props),
            env.langcode_key,
            Some(&mut value),
            host,
            |a, b| normalization::strcmp_at(m.bytes, a, b),
        );
        if rc >= 0x80000000 {
            return 0;
        }
        normalization::strn_copy(m.bytes, language, value, 4);
    } else {
        for i in 0..3u32 {
            if half(m, at + i * 2) == 0 {
                return 0;
            }
            m.bytes[(language + i) as usize] = half(m, at + i * 2) as u8;
        }
        if half(m, at + 6) != 0 {
            return 0;
        }
    }
    let mut result = 0;
    for (offset, prefix) in [(8, env.nts_prefix), (12, env.ipa_prefix)] {
        let mut name = [0; 8];
        build_name(m, &mut name, prefix, language);
        let name_at = p.scratch + 24;
        m.bytes[name_at as usize..name_at as usize + 8].copy_from_slice(&name);
        let session = m.word(ctx + offset);
        let rc = if session == 0 {
            let handle = [m.word(ctx), m.word(ctx + 4)];
            crate::render_part_d::dictionary_new(
                m.bytes,
                name_at,
                handle,
                ctx + offset,
                p.dictionary,
                host,
            )
        } else {
            crate::render_part_d::dictionary_rename(m.bytes, session, name_at, host)
        };
        if rc >= 0x80000000 && rc & 0x1fff != 1 {
            return rc;
        }
        result = if rc >= 0x80000000 { 0 } else { rc };
    }
    result
}
fn pronunciation_replace(
    m: &mut Memory<'_>,
    ctx: u32,
    text_it: u32,
    events: u32,
    env: &Environment<'_>,
    p: &PronunciationEnvironment<'_>,
    host: &mut (impl ResourceHost + crate::render_part_d::DictionaryHost),
) -> u32 {
    if ctx == 0 || text_it == 0 || events == 0 {
        return 0x81102007;
    }
    let (rc, first) = iterator_get(m, events, 0);
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, event) = iterator_get(m, events, 0xffff);
    if rc >= 0x80000000 {
        return rc;
    }
    let arr = m.word(events + 12);
    let mut index = 0;
    let rc = normalization::current_index(
        Some(&m.bytes[arr as usize..]),
        half(m, events + 16),
        Some(&mut index),
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut count = 0;
    let rc = normalization::inner_field(Some(&m.bytes[arr as usize..]), Some(&mut count));
    if rc >= 0x80000000 {
        return rc;
    }
    if event == 0 || half(m, event + 10) != 7 {
        return 0;
    }
    let mut typ = m.word(event + 28);
    if typ == 43 && m.word(event + 32) != 0 {
        typ = match m.bytes[(event + 36) as usize] {
            2 => 70,
            3 => 71,
            _ => return 0,
        };
    }
    if !matches!(typ, 70 | 71) {
        return 0;
    }
    iterator_find(m, events, &[], true);
    let rc = iterator_find(m, events, p.stop_filter, false);
    let next = if rc & 0x1fff == 20 {
        0
    } else {
        if rc >= 0x80000000 {
            return rc;
        }
        let (rc, next) = iterator_get(m, events, 0xffff);
        if rc >= 0x80000000 {
            return rc;
        }
        next
    };
    let (rc, text) = iterator_get(m, text_it, 0);
    if rc >= 0x80000000 {
        return rc;
    }
    let start = m.word(event + 20).wrapping_sub(m.word(first + 20)) as u16 as u32 / 2;
    let len = if next != 0 {
        m.word(next + 20).wrapping_sub(m.word(event + 20))
    } else {
        m.word(first + 24)
            .wrapping_add(m.word(first + 20))
            .wrapping_sub(m.word(event + 20))
    } as u16 as u32
        / 2;
    let (rc, output, out_len) =
        pronunciation_convert(m, ctx, text + start * 2, len, typ, env, p, host);
    if rc >= 0x80000000 {
        return rc;
    }
    let rc = normalization::delete_range(m.bytes, text_it, start as u16, len as u16);
    if rc >= 0x80000000 {
        return rc;
    }
    if out_len != 0 {
        let rc = iterator_seek(m, text_it, start as i32);
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = normalization::insert_elements(
            m.bytes,
            text_it,
            output,
            u32::from(out_len),
            &mut FetchHost {
                host,
                env: &env.engine,
            },
        );
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let (rc, engine) = resolve(m, [m.word(ctx), m.word(ctx + 4)], &env.engine);
    if output == 0 || rc >= 0x80000000 {
        return 0x81102007;
    }
    frontend::heap_release(m.bytes, nz(m.word(engine + 4)), Some(output), host);
    let delta = u32::from(out_len).wrapping_sub(len).wrapping_mul(2);
    m.put_word(first + 24, m.word(first + 24).wrapping_add(delta));
    m.put_word(event + 28, 43);
    let rc = iterator_seek(m, events, i32::from(index));
    if rc >= 0x80000000 {
        return rc;
    }
    for j in index + 1..count {
        let following = event + u32::from(j - index) * 40;
        let pos = m.word(following + 20);
        let beginning = m.word(first + 20) + start * 2;
        if beginning <= pos {
            if pos < m.word(first + 20) + (start + len) * 2 {
                m.put_word(following + 20, beginning);
                if half(m, following + 10) == 1 {
                    m.put_word(following + 24, u32::from(out_len) * 2);
                }
            } else {
                m.put_word(following + 20, pos.wrapping_add(delta));
            }
        }
    }
    rc
}
/// fn_2c3450. Apply language changes and replace pronunciation event ranges.
pub fn fn_2c3450(
    m: &mut Memory<'_>,
    ctx: u32,
    text_it: u32,
    events: u32,
    text: u32,
    env: &Environment<'_>,
    p: &PronunciationEnvironment<'_>,
    host: &mut (impl ResourceHost + crate::render_part_d::DictionaryHost),
) -> u32 {
    if ctx == 0 || text_it == 0 || events == 0 {
        return 0x81102007;
    }
    let mut saved = [0u8; 64];
    saved.copy_from_slice(&m.bytes[p.scratch as usize..p.scratch as usize + 64]);
    let rc = (|| {
        let rc = iterator_seek(m, events, 0);
        if rc >= 0x80000000 {
            return rc;
        }
        let mut rc = iterator_find(m, events, p.language_filter, false);
        loop {
            if rc & 0x1fff == 20 {
                return 0;
            }
            if rc >= 0x80000000 {
                return rc;
            }
            let (r, event) = iterator_get(m, events, 0xffff);
            if r >= 0x80000000 {
                return r;
            }
            if event == 0 {
                return r;
            }
            rc = if half(m, event + 10) == 7 {
                pronunciation_replace(m, ctx, text_it, events, env, p, host)
            } else {
                pronunciation_language(m, ctx, event, text, env, p, host)
            };
            if rc >= 0x80000000 {
                return rc;
            }
            iterator_find(m, events, &[], true);
            rc = iterator_find(m, events, p.language_filter, false);
        }
    })();
    m.bytes[p.scratch as usize..p.scratch as usize + 64].copy_from_slice(&saved);
    rc
}
/// fn_2c322c. Open the text/event iterators and run pronunciation conversion.
pub fn fn_2c322c(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    env: &Environment<'_>,
    p: &PronunciationEnvironment<'_>,
    host: &mut (impl ResourceHost + crate::render_part_d::DictionaryHost),
) -> u32 {
    if ctx == 0 || handle == 0 {
        return 0x81102007;
    }
    let handle_arg = handle;
    let mut x = 0;
    let mut y = 0;
    let handle = [m.word(ctx), m.word(ctx + 4)];
    let mut rc = normalization::open_iterator(
        m.bytes,
        handle[0],
        handle[1],
        handle_arg,
        0x3fc,
        Some(&mut x),
        &mut FetchHost {
            host,
            env: &env.engine,
        },
    );
    if rc < 0x80000000 {
        rc = normalization::open_iterator(
            m.bytes,
            handle[0],
            handle[1],
            handle_arg,
            0x3fd,
            Some(&mut y),
            &mut FetchHost {
                host,
                env: &env.engine,
            },
        );
        if rc < 0x80000000 {
            let (r, q) = fetch_parser(m, handle_arg, 0x3f4, 1, env, host);
            rc = r;
            if rc < 0x80000000 {
                rc = fn_2c3450(
                    m,
                    ctx,
                    x,
                    y,
                    if q == 0 { 0 } else { m.word(q + 4) },
                    env,
                    p,
                    host,
                );
            }
        }
    }
    if y != 0 {
        normalization::release_iterator(
            m.bytes,
            y,
            &mut FetchHost {
                host,
                env: &env.engine,
            },
        );
    }
    if x != 0 {
        normalization::release_iterator(
            m.bytes,
            x,
            &mut FetchHost {
                host,
                env: &env.engine,
            },
        );
    }
    rc
}

/// Memory-passing pronunciation forms.
///
/// `fn_2c322c`'s iterator open/release and `pronunciation_replace`'s tail
/// allocate through the arena-blind `FetchHost`, so the live render chain
/// cannot serve them. These forms take the same host for resource and
/// dictionary calls plus a [`ParserAllocatorM`] for every allocation, and
/// route the iterator and conversion heap traffic through the guest heap on
/// the caller's arena. The slice forms above are unchanged.
struct ConversionHostM<'a, H> {
    host: &'a mut H,
    out: u32,
}
impl<H: ResourceHost> frontend::HeapVtable for ConversionHostM<'_, H> {
    fn alloc_at(&mut self, _o: u32, _n: u32) -> u32 {
        panic!("pronunciation conversion reached the arena-blind allocator");
    }
    fn realloc_at(&mut self, _o: u32, _p: u32, _n: u32) -> u32 {
        panic!("pronunciation conversion reached the arena-blind reallocator");
    }
    fn release_at(&mut self, _o: u32, _p: u32) {
        panic!("pronunciation conversion reached the arena-blind releaser");
    }
}
impl<H: ResourceHost> frontend::MethodTable for ConversionHostM<'_, H> {
    fn call_at(&mut self, a: u32, n: u32, s: u32) -> u32 {
        self.host.call_at(a, n, s)
    }
}
impl<H: ResourceHost + crate::render_part_d::DictionaryHost> render_part_c::Host
    for ConversionHostM<'_, H>
{
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        render_part_c::Host::call(self.host, m, f, a)
    }
    fn output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        if f == 0x2eccb4 {
            let saved = m.word(self.out);
            let len =
                crate::render_part_d::dictionary_match(m.bytes, a[0], a[1], self.out, self.host);
            let value = m.word(self.out);
            m.put_word(self.out, saved);
            (len, value)
        } else {
            render_part_c::Host::output(self.host, m, f, a)
        }
    }
}
/// Memory-passing `pronunciation_convert`: the conversion buffers go
/// through the guest heap on the caller's arena.
fn pronunciation_convert_m(
    m: &mut Memory<'_>,
    ctx: u32,
    mut text: u32,
    mut len: u32,
    typ: u32,
    env: &Environment<'_>,
    p: &PronunciationEnvironment<'_>,
    host: &mut (impl ResourceHost + crate::render_part_d::DictionaryHost),
    allocator: &mut impl ParserAllocatorM,
) -> (u32, u32, u16) {
    if text == 0 {
        return (0x81102007, 0, 0);
    }
    let (rc, engine) = resolve(m, [m.word(ctx), m.word(ctx + 4)], &env.engine);
    if rc >= 0x80000000 || !matches!(typ, 70 | 71) {
        return (0x81102007, 0, 0);
    }
    let mut i = 0;
    while i < len && normalization::u16chr(p.trim, 0, half(m, text + i * 2)).is_some() {
        i += 1;
    }
    text += i * 2;
    len = len.wrapping_sub(i) as u16 as u32;
    i = 0;
    while i < len && normalization::u16chr(p.trim, 0, half(m, text + (len - i - 1) * 2)).is_some() {
        i += 1;
    }
    len = len.wrapping_sub(i) as u16 as u32;
    let heap = m.word(engine + 4);
    let mut copy = 0;
    let mut utf = 0;
    let mut output = 0;
    let mut output_len = 0;
    let rc = (|| {
        copy = frontend::heap_alloc_m(m.bytes, nz(heap), len * 2 + 2, allocator);
        if copy == 0 {
            return 0x8110200a;
        }
        crate::fe_text::memcpy_195ae8(m.bytes, copy, text, len * 2);
        m.put_half(copy + len * 2, 0);
        let bytes = normalization::utf8_len_u16(m.bytes, copy) as u16 as u32;
        utf = frontend::heap_alloc_m(m.bytes, nz(heap), bytes, allocator);
        if utf == 0 {
            return 0x8110200a;
        }
        let rc = normalization::transcode_u16(
            m.bytes,
            copy,
            len + 1,
            p.scratch,
            utf,
            bytes,
            p.scratch + 4,
            0,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let session = m.word(ctx + if typ == 70 { 8 } else { 12 });
        let capacity = p.scratch + 8;
        m.put_half(capacity, 0);
        let rc = render_part_c::fn_2c3308(
            m,
            session,
            utf,
            0,
            capacity,
            &mut ConversionHostM {
                host,
                out: p.scratch + 12,
            },
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let converted =
            frontend::heap_alloc_m(m.bytes, nz(heap), u32::from(half(m, capacity)), allocator);
        if converted == 0 {
            return 0x8110200a;
        }
        let mut rc = render_part_c::fn_2c3308(
            m,
            session,
            utf,
            converted,
            capacity,
            &mut ConversionHostM {
                host,
                out: p.scratch + 12,
            },
        );
        if rc < 0x80000000 {
            let bytes = (normalization::decoded_count(m.bytes, converted, p.decoder.extra) as u16
                as u32)
                * 2;
            output = frontend::heap_alloc_m(m.bytes, nz(heap), bytes, allocator);
            if output == 0 {
                rc = 0x8110200a;
            } else {
                output_len = normalization::decode_str(m.bytes, converted, output, p.decoder)
                    .wrapping_sub(1) as u16;
            }
        }
        frontend::heap_release_m(m.bytes, nz(heap), Some(converted), allocator);
        rc
    })();
    if utf != 0 {
        frontend::heap_release_m(m.bytes, nz(heap), Some(utf), allocator);
    }
    if copy != 0 {
        frontend::heap_release_m(m.bytes, nz(heap), Some(copy), allocator);
    }
    (rc, output, output_len)
}
/// Memory-passing `pronunciation_replace`: the insert grows through the
/// guest heap and the replaced output releases through it.
fn pronunciation_replace_m(
    m: &mut Memory<'_>,
    ctx: u32,
    text_it: u32,
    events: u32,
    env: &Environment<'_>,
    p: &PronunciationEnvironment<'_>,
    host: &mut (impl ResourceHost + crate::render_part_d::DictionaryHost),
    allocator: &mut impl ParserAllocatorM,
) -> u32 {
    if ctx == 0 || text_it == 0 || events == 0 {
        return 0x81102007;
    }
    let (rc, first) = iterator_get(m, events, 0);
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, event) = iterator_get(m, events, 0xffff);
    if rc >= 0x80000000 {
        return rc;
    }
    let arr = m.word(events + 12);
    let mut index = 0;
    let rc = normalization::current_index(
        Some(&m.bytes[arr as usize..]),
        half(m, events + 16),
        Some(&mut index),
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut count = 0;
    let rc = normalization::inner_field(Some(&m.bytes[arr as usize..]), Some(&mut count));
    if rc >= 0x80000000 {
        return rc;
    }
    if event == 0 || half(m, event + 10) != 7 {
        return 0;
    }
    let mut typ = m.word(event + 28);
    if typ == 43 && m.word(event + 32) != 0 {
        typ = match m.bytes[(event + 36) as usize] {
            2 => 70,
            3 => 71,
            _ => return 0,
        };
    }
    if !matches!(typ, 70 | 71) {
        return 0;
    }
    iterator_find(m, events, &[], true);
    let rc = iterator_find(m, events, p.stop_filter, false);
    let next = if rc & 0x1fff == 20 {
        0
    } else {
        if rc >= 0x80000000 {
            return rc;
        }
        let (rc, next) = iterator_get(m, events, 0xffff);
        if rc >= 0x80000000 {
            return rc;
        }
        next
    };
    let (rc, text) = iterator_get(m, text_it, 0);
    if rc >= 0x80000000 {
        return rc;
    }
    let start = m.word(event + 20).wrapping_sub(m.word(first + 20)) as u16 as u32 / 2;
    let len = if next != 0 {
        m.word(next + 20).wrapping_sub(m.word(event + 20))
    } else {
        m.word(first + 24)
            .wrapping_add(m.word(first + 20))
            .wrapping_sub(m.word(event + 20))
    } as u16 as u32
        / 2;
    let (rc, output, out_len) =
        pronunciation_convert_m(m, ctx, text + start * 2, len, typ, env, p, host, allocator);
    if rc >= 0x80000000 {
        return rc;
    }
    let rc = normalization::delete_range(m.bytes, text_it, start as u16, len as u16);
    if rc >= 0x80000000 {
        return rc;
    }
    if out_len != 0 {
        let rc = iterator_seek(m, text_it, start as i32);
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = normalization::insert_elements(
            m.bytes,
            text_it,
            output,
            u32::from(out_len),
            &mut FetchHostM {
                allocator,
                env: &env.engine,
            },
        );
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let (rc, engine) = resolve(m, [m.word(ctx), m.word(ctx + 4)], &env.engine);
    if output == 0 || rc >= 0x80000000 {
        return 0x81102007;
    }
    frontend::heap_release_m(m.bytes, nz(m.word(engine + 4)), Some(output), allocator);
    let delta = u32::from(out_len).wrapping_sub(len).wrapping_mul(2);
    m.put_word(first + 24, m.word(first + 24).wrapping_add(delta));
    m.put_word(event + 28, 43);
    let rc = iterator_seek(m, events, i32::from(index));
    if rc >= 0x80000000 {
        return rc;
    }
    for j in index + 1..count {
        let following = event + u32::from(j - index) * 40;
        let pos = m.word(following + 20);
        let beginning = m.word(first + 20) + start * 2;
        if beginning <= pos {
            if pos < m.word(first + 20) + (start + len) * 2 {
                m.put_word(following + 20, beginning);
                if half(m, following + 10) == 1 {
                    m.put_word(following + 24, u32::from(out_len) * 2);
                }
            } else {
                m.put_word(following + 20, pos.wrapping_add(delta));
            }
        }
    }
    rc
}
/// Memory-passing `fn_2c3450`: language switches run on the caller's host
/// (their dictionary calls are already memory-passing) while replacement
/// traffic goes through the guest heap.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c3450_m(
    m: &mut Memory<'_>,
    ctx: u32,
    text_it: u32,
    events: u32,
    text: u32,
    env: &Environment<'_>,
    p: &PronunciationEnvironment<'_>,
    host: &mut (impl ResourceHost + crate::render_part_d::DictionaryHost),
    allocator: &mut impl ParserAllocatorM,
) -> u32 {
    if ctx == 0 || text_it == 0 || events == 0 {
        return 0x81102007;
    }
    let mut saved = [0u8; 64];
    saved.copy_from_slice(&m.bytes[p.scratch as usize..p.scratch as usize + 64]);
    let rc = (|| {
        let rc = iterator_seek(m, events, 0);
        if rc >= 0x80000000 {
            return rc;
        }
        let mut rc = iterator_find(m, events, p.language_filter, false);
        loop {
            if rc & 0x1fff == 20 {
                return 0;
            }
            if rc >= 0x80000000 {
                return rc;
            }
            let (r, event) = iterator_get(m, events, 0xffff);
            if r >= 0x80000000 {
                return r;
            }
            if event == 0 {
                return r;
            }
            rc = if half(m, event + 10) == 7 {
                pronunciation_replace_m(m, ctx, text_it, events, env, p, host, allocator)
            } else {
                pronunciation_language(m, ctx, event, text, env, p, host)
            };
            if rc >= 0x80000000 {
                return rc;
            }
            iterator_find(m, events, &[], true);
            rc = iterator_find(m, events, p.language_filter, false);
        }
    })();
    m.bytes[p.scratch as usize..p.scratch as usize + 64].copy_from_slice(&saved);
    rc
}
/// Memory-passing `fn_2c322c`: the text/event iterators open and release
/// through the guest heap on the caller's arena.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c322c_m(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    env: &Environment<'_>,
    p: &PronunciationEnvironment<'_>,
    host: &mut (impl ResourceHost + crate::render_part_d::DictionaryHost),
    allocator: &mut impl ParserAllocatorM,
) -> u32 {
    if ctx == 0 || handle == 0 {
        return 0x81102007;
    }
    let handle_arg = handle;
    let mut x = 0;
    let mut y = 0;
    let handle = [m.word(ctx), m.word(ctx + 4)];
    let mut rc = normalization::open_iterator(
        m.bytes,
        handle[0],
        handle[1],
        handle_arg,
        0x3fc,
        Some(&mut x),
        &mut FetchHostM {
            allocator,
            env: &env.engine,
        },
    );
    if rc < 0x80000000 {
        rc = normalization::open_iterator(
            m.bytes,
            handle[0],
            handle[1],
            handle_arg,
            0x3fd,
            Some(&mut y),
            &mut FetchHostM {
                allocator,
                env: &env.engine,
            },
        );
        if rc < 0x80000000 {
            let (r, q) = fetch_parser_m(m, handle_arg, 0x3f4, 1, env, allocator);
            rc = r;
            if rc < 0x80000000 {
                rc = fn_2c3450_m(
                    m,
                    ctx,
                    x,
                    y,
                    if q == 0 { 0 } else { m.word(q + 4) },
                    env,
                    p,
                    host,
                    allocator,
                );
            }
        }
    }
    if y != 0 {
        normalization::release_iterator(
            m.bytes,
            y,
            &mut FetchHostM {
                allocator,
                env: &env.engine,
            },
        );
    }
    if x != 0 {
        normalization::release_iterator(
            m.bytes,
            x,
            &mut FetchHostM {
                allocator,
                env: &env.engine,
            },
        );
    }
    rc
}

fn cached_pair(env: &Environment<'_>) -> [u8; 8] {
    let mut out = [0; 8];
    frontend::load_pair(&mut out, &env.engine.cached_handle);
    out
}
/// fn_2c66c8. Destroy a pipeline, preserving the first failing close/release.
pub fn fn_2c66c8(
    m: &mut Memory<'_>,
    p: u32,
    env: &Environment<'_>,
    host: &mut impl ResourceHost,
) -> u32 {
    if p == 0 {
        return 0x8c902006;
    }
    let context = m.word(p);
    let heap = m.word(context + 4);
    let mut error = 0;
    let arr = m.word(p + 4);
    let trace = m.word(p + 12);
    if arr != 0 {
        if trace != 0 {
            frontend::begin_trace_hook(Some(trace));
            frontend::begin_stage_hook(Some(trace));
        }
        let count = half(m, p + 8);
        for n in (0..count).rev() {
            let component = m.word(arr + u32::from(n) * 4);
            if component == 0 {
                continue;
            }
            if half(m, component) > 1 {
                m.put_half(component, half(m, component) - 1);
                continue;
            }
            let cached = [
                u32::from_le_bytes(cached_pair(env)[..4].try_into().unwrap()),
                u32::from_le_bytes(cached_pair(env)[4..].try_into().unwrap()),
            ];
            if frontend::match_halves(
                m.word(component + 12),
                m.word(component + 16),
                cached[0],
                cached[1],
            ) == 0
            {
                if trace != 0 {
                    frontend::begin_stage_hook(Some(trace));
                }
                let iface = m.word(component + 8);
                let rc = host.resource_call(
                    m,
                    m.word(iface + 20),
                    &[m.word(component + 12), m.word(component + 16)],
                );
                if trace != 0 {
                    frontend::end_stage_hook(Some(trace));
                }
                if rc >= 0x80000000 && error < 0x80000000 {
                    error = rc;
                }
            }
            let iface = m.word(component + 8);
            if iface != 0 {
                let rc = frontend::release_broker_interface(nz(m.word(context + 12)), nz(iface));
                if error < 0x80000000 && rc >= 0x80000000 {
                    error = rc;
                }
            }
            frontend::heap_release(m.bytes, nz(heap), nz(m.word(component + 4)), host);
            frontend::heap_release(m.bytes, nz(heap), Some(component), host);
        }
        if trace != 0 {
            frontend::end_stage_hook(Some(trace));
            frontend::end_trace_hook(Some(trace), nz(m.word(context + 16)));
        }
        frontend::heap_release(m.bytes, nz(heap), Some(arr), host);
    }
    if trace != 0 {
        let mut free = None;
        frontend::release_trace_entry(m.bytes, Some(trace), m.word(p + 16), |heap, p| {
            free = Some((heap, p))
        });
        if let Some((heap, p)) = free {
            frontend::heap_release(m.bytes, nz(heap), nz(p), host);
        }
    }
    let arr2 = m.word(p + 20);
    if arr2 != 0 {
        for k in 0..u32::from(half(m, p + 8)) + 3 {
            frontend::heap_release(m.bytes, nz(heap), nz(m.word(arr2 + k * 4)), host);
        }
        frontend::heap_release(m.bytes, nz(heap), Some(arr2), host);
    }
    frontend::heap_release(m.bytes, nz(heap), Some(p), host);
    error
}
pub struct PipelineEnvironment {
    pub loader_suffix: u32,
    pub empty: u32,
    pub lang_key: u32,
    pub cfg_key: u32,
    pub clc_prefix: u32,
    pub slash: u32,
    pub colon: u32,
}
/// fn_2c6ac0. Match a pipeline component and open its typed resource.
/// scratch is a disjoint 1040-byte area replacing buf and saved handles.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c6ac0(
    m: &mut Memory<'_>,
    ctx: u32,
    flag: i32,
    name: u32,
    mut lang: u32,
    extra: [u32; 4],
    out: u32,
    scratch: u32,
    env: &Environment<'_>,
    pipeline: &PipelineEnvironment,
    host: &mut (impl ResourceHost + MatchHost),
) -> u32 {
    if ctx == 0 || name == 0 {
        return 0x8c902007;
    }
    let mut saved = [0u8; 1040];
    saved.copy_from_slice(&m.bytes[scratch as usize..scratch as usize + 1040]);
    let rc = (|| {
        let context = m.word(ctx);
        if lang == 0 {
            lang = pipeline.empty;
        } else if normalization::strstr_at(m.bytes, name, pipeline.loader_suffix).is_some() {
            let props = m.word(context + 20);
            if props != 0 {
                host.select_lock(m.word(props + 4));
            }
            let mut lc = 0;
            let mut cfg = pipeline.empty;
            let rc = frontend::fetch_param(
                m.bytes,
                nz(props),
                pipeline.lang_key,
                Some(&mut lc),
                host,
                |a, b| normalization::strcmp_at(m.bytes, a, b),
            );
            if rc >= 0x80000000 {
                return rc;
            }
            let rc = frontend::fetch_param(
                m.bytes,
                nz(props),
                pipeline.cfg_key,
                Some(&mut cfg),
                host,
                |a, b| normalization::strcmp_at(m.bytes, a, b),
            );
            if rc >= 0x80000000 {
                return rc;
            }
            m.bytes[scratch as usize] = 0;
            for part in [
                pipeline.clc_prefix,
                lc,
                pipeline.slash,
                cfg,
                pipeline.colon,
                lang,
            ] {
                frontend::append_str(m.bytes, scratch, part);
            }
            lang = scratch;
        }
        let pair = cached_pair(env);
        if out != 0 {
            m.bytes[out as usize..out as usize + 8].copy_from_slice(&pair);
        }
        let saved_handle = scratch + 1024;
        m.bytes[saved_handle as usize..saved_handle as usize + 8].copy_from_slice(&pair);
        let found = scratch + 1032;
        m.put_word(found, 0);
        let rc = fn_2c69a0(m, ctx, name, found, host);
        if rc >= 0x80000000 {
            return rc;
        }
        let component = m.word(found);
        let iface = m.word(component + 8);
        let method = m.word(iface + 32);
        if method == 0 {
            return 0x8c902007;
        }
        let rc = host.resource_call(
            m,
            method,
            &[
                m.word(component + 12),
                m.word(component + 16),
                name,
                lang,
                extra[0],
                extra[1],
                extra[2],
                extra[3],
                saved_handle,
            ],
        );
        if rc >= 0x80000000 {
            // flag only suppresses diagnostics; it never changes the status.
            let _ = flag;
            return rc;
        }
        if out != 0 {
            crate::fe_text::memcpy_195ae8(m.bytes, out, saved_handle, 8);
        }
        rc
    })();
    m.bytes[scratch as usize..scratch as usize + 1040].copy_from_slice(&saved);
    rc
}

struct BrokerQuery<'a, H> {
    host: &'a mut H,
    table: u32,
    scratch: u32,
}
impl<H: crate::boot_part_b::QueryHost> crate::boot_part_b::Host for BrokerQuery<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        assert_eq!(f, 0x177e0c);
        crate::boot_part_b::fn_177e0c(
            m,
            a[0],
            a[1],
            a[2],
            a[3],
            a[4],
            a[5],
            self.table,
            self.scratch,
            self.host,
        )
    }
    fn thread_id(&mut self) -> u32 {
        crate::boot_part_b::Host::thread_id(self.host)
    }
}
fn registry_insert(
    m: &mut Memory<'_>,
    registry: u32,
    name: u32,
    pair: u32,
    scratch: u32,
    host: &mut (impl ResourceHost + crate::render_part_d::DictionaryHost),
    ops: &mut impl frontend::TreeOps,
) -> u32 {
    let (lock, find, found) =
        crate::render_part_d::dictionary_registry(m.bytes, registry, name, host);
    let mut inserted = 0;
    if registry != 0 && name != 0 && pair != 0 && lock < 0x80000000 && find >= 0x80000000 {
        m.bytes[scratch as usize..scratch as usize + 32].fill(0);
        crate::fe_text::memcpy_195ae8(m.bytes, scratch, pair, 20);
        inserted =
            frontend::insert_guarded(m.bytes, nz(m.word(registry + 16)), name, scratch, host, ops);
    }
    let scripts = frontend::InsertScripts {
        lock,
        find,
        found,
        insert: inserted,
        unlock: 0,
    };
    let mut rc = frontend::insert_ling_entry(m.bytes, nz(registry), nz(name), nz(pair), &scripts);
    if registry != 0 && name != 0 && pair != 0 && lock < 0x80000000 {
        let r = crate::render_part_d::dictionary_lock(m.bytes, registry, false, host);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    rc
}
struct BlockResult(u32);
impl frontend::BlockHeap for BlockResult {
    fn alloc_block(&mut self, _: u32, _: u32) -> u32 {
        self.0
    }
    fn realloc_block(&mut self, _: u32, _: u32, _: u32) -> u32 {
        self.0
    }
    fn alloc_cells(&mut self, _: u32, _: u32, _: u32) -> u32 {
        panic!("unexpected calloc in append_element")
    }
}
fn append_resource(m: &mut Memory<'_>, p: u32, src: u32, host: &mut impl ResourceHost) -> u32 {
    if p == 0 {
        return 0;
    }
    let mut block = frontend::GrowBlock {
        data: m.word(p),
        elem_size: half(m, p + 4),
        callback: m.word(p + 8),
        cap: m.word(p + 16),
        count: m.word(p + 20),
        grow: half(m, p + 24),
        heap: m.word(p + 28),
    };
    // append_element borrows the arena exclusively. Run the real heap
    // dispatcher first and return its result at the allocator boundary.
    let allocation = if block.cap < block.count.wrapping_add(1) && block.grow != 0 {
        let slots = frontend::divmod_u32(block.count, u32::from(block.grow))
            .0
            .wrapping_add(1)
            .wrapping_mul(u32::from(block.grow));
        let size = u32::from(block.elem_size).wrapping_mul(slots);
        if block.data == 0 {
            frontend::heap_alloc(m.bytes, nz(block.heap), size, host)
        } else {
            frontend::heap_realloc(m.bytes, nz(block.heap), block.data, size, host)
        }
    } else {
        0
    };
    let mut bytes = [0u8; 65536];
    let width = usize::from(block.elem_size);
    bytes[..width].copy_from_slice(&m.bytes[src as usize..src as usize + width]);
    let mut pending = None;
    let callback = block.callback;
    let mut rc = frontend::append_element(
        Some(&mut block),
        m.bytes,
        src,
        &bytes[..width],
        &mut BlockResult(allocation),
        if callback != 0 {
            Some(|a, b, c| {
                pending = Some((a, b, c));
                1
            })
        } else {
            None
        },
    );
    m.put_word(p, block.data);
    m.put_word(p + 16, block.cap);
    m.put_word(p + 20, block.count);
    if let Some((a, b, c)) = pending {
        if host.resource_call(m, block.callback, &[a, b, c]) == 0 {
            m.put_word(p + 20, m.word(p + 20).wrapping_sub(1));
            rc = 0;
        }
    }
    rc
}
/// fn_2c6ddc. Reopen a registered object or query/open and register a new one.
/// scratch is 96 disjoint writable bytes, restored before return.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c6ddc(
    m: &mut Memory<'_>,
    handle: [u32; 2],
    name: u32,
    component: u32,
    list: u32,
    reopened: u32,
    scratch: u32,
    builtin_table: u32,
    env: &Environment<'_>,
    host: &mut (impl ResourceHost
              + crate::render_part_d::DictionaryHost
              + crate::boot_part_b::QueryHost),
    ops: &mut impl frontend::TreeOps,
) -> u32 {
    let mut saved = [0u8; 96];
    saved.copy_from_slice(&m.bytes[scratch as usize..scratch as usize + 96]);
    let rc = (|| {
        let (rc, context) = resolve(m, handle, &env.engine);
        if rc >= 0x80000000 {
            return rc;
        }
        let registry = m.word(context + 24);
        let (rc, entry) = registry_acquire(m, registry, name, scratch, host);
        if rc < 0x80000000 {
            let iface = m.word(entry + 4);
            let reopen = m.word(iface + 24);
            let rc = if reopen != 0 {
                if reopened != 0 {
                    m.bytes[reopened as usize] = 1;
                }
                host.resource_call(m, reopen, &[m.word(entry + 8), m.word(entry + 12)])
            } else {
                rc
            };
            registry_release(m, registry, name, false, host);
            return rc;
        }
        let args = scratch + 16;
        m.bytes[args as usize..args as usize + 20].fill(0);
        m.bytes[args as usize] = 1;
        m.bytes[scratch as usize..scratch as usize + 8].copy_from_slice(&cached_pair(env));
        let mut query = BrokerQuery {
            host: &mut *host,
            table: builtin_table,
            scratch: scratch + 64,
        };
        let rc = crate::boot_part_b::fn_178510(
            m,
            m.word(context + 12),
            component,
            1,
            scratch,
            args + 4,
            &mut query,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let iface = m.word(args + 4);
        let rc = host.resource_call(
            m,
            m.word(iface + 16),
            &[
                m.word(scratch),
                m.word(scratch + 4),
                handle[0],
                handle[1],
                args + 8,
            ],
        );
        if rc >= 0x80000000 {
            frontend::release_broker_interface(nz(m.word(context + 12)), nz(iface));
            return rc;
        }
        let rc = registry_insert(m, registry, name, args, scratch + 48, host, ops);
        if rc < 0x80000000 && (list == 0 || append_resource(m, list, name, host) != 0) {
            return rc;
        }
        let rc = if rc < 0x80000000 { 0x8c90200a } else { rc };
        host.resource_call(
            m,
            m.word(iface + 20),
            &[m.word(args + 8), m.word(args + 12)],
        );
        frontend::release_broker_interface(nz(m.word(context + 12)), nz(iface));
        rc
    })();
    m.bytes[scratch as usize..scratch as usize + 96].copy_from_slice(&saved);
    rc
}

pub trait PipelineLogger {
    /// Indirect level method only; slot is the logger's callback object.
    fn level(&mut self, slot: u32) -> u32;
}
pub struct PipelineKeys {
    pub broker_string: u32,
    pub components: u32,
    pub component: u32,
    pub parameters: u32,
    pub objects: u32,
    pub resources: u32,
    pub resource: u32,
    pub content_type: u32,
    pub total: u32,
    pub input: u32,
    pub output: u32,
    pub empty: u32,
    pub builtin_table: u32,
}
fn pipeline_lookup(m: &Memory<'_>, db: u32, key: u32) -> (u32, u32) {
    let mut out = 0;
    let rc = frontend::lookup_value(m.bytes, nz(db), key, Some(&mut out), |a, b| {
        normalization::strcmp_at(m.bytes, a, b)
    });
    (rc, out)
}
fn pipeline_iterator(m: &mut Memory<'_>, db: u32, host: &mut impl ResourceHost) -> (u32, u32) {
    let mut out = 0;
    let rc = frontend::alloc_handle_for_key(m.bytes, nz(db), 0, 0, Some(&mut out), host, |_, _| 0);
    (rc, out)
}
fn pipeline_next(m: &mut Memory<'_>, it: u32) -> (u32, u32, u32) {
    let mut key = 0;
    let mut value = 0;
    let rc = frontend::step_iter(m.bytes, nz(it), Some(&mut key), Some(&mut value));
    (rc, key, value)
}
fn pipeline_string(m: &mut Memory<'_>, heap: u32, s: u32, host: &mut impl ResourceHost) -> u32 {
    let p = frontend::heap_alloc(
        m.bytes,
        nz(heap),
        normalization::strlen8(m.bytes, s) + 1,
        host,
    );
    if p != 0 {
        frontend::copy_str(m.bytes, p, s);
    }
    p
}
struct TraceCreator<'a, 'b, H> {
    host: &'a mut H,
    env: &'a Environment<'b>,
}
impl<H: ResourceHost> crate::boot_part_b::Host for TraceCreator<'_, '_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        assert_eq!(f, 0x1963b8);
        frontend::call_method(m.bytes, nz(a[0]), a[1], a[2], self.host)
    }
    fn thread_id(&mut self) -> u32 {
        crate::boot_part_b::Host::thread_id(self.host)
    }
}
impl<H: ResourceHost> crate::boot_part_b::ContextHost for TraceCreator<'_, '_, H> {
    fn context(&mut self, m: &mut Memory<'_>, handle: [u32; 2]) -> (u32, u32) {
        resolve(m, handle, &self.env.engine)
    }
}
/// fn_2c7158. Construct a two-pass component pipeline, register objects and
/// load its optional resources. stack is 1280 disjoint caller-owned bytes.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c7158<H, L, O, B, E, C>(
    m: &mut Memory<'_>,
    handle: [u32; 2],
    resources: u32,
    spec: u32,
    resource_list: u32,
    out: u32,
    out_reopened: u32,
    stack: u32,
    keys: &PipelineKeys,
    pipeline: &PipelineEnvironment,
    rt: &mut ParserRuntime<'_, '_, H, L, O, B, E, C>,
) -> u32
where
    H: ResourceHost
        + MatchHost
        + PipelineLogger
        + crate::render_part_b::ParameterHost
        + crate::render_part_d::DictionaryHost
        + crate::boot_part_b::QueryHost,
    L: frontend::StateLock,
    O: frontend::TreeOps,
    B: frontend::BeforeChange,
    E: frontend::ElementCallback,
    C: FnMut(u32, u32) -> i32,
{
    let (rc, engine) = resolve(m, handle, &rt.env.engine);
    if rc >= 0x80000000 {
        return rc;
    }
    m.put_word(out, 0);
    m.bytes[out_reopened as usize] = 0;
    let (rc, entry) = pipeline_lookup(m, resources, keys.broker_string);
    if rc >= 0x80000000 {
        return 0x8c902007;
    }
    let name = m.word(entry);
    if name == 0 {
        return 0x8c902007;
    }
    let mut saved = [0u8; 1280];
    saved.copy_from_slice(&m.bytes[stack as usize..stack as usize + 1280]);
    let heap = m.word(engine + 4);
    let mut iter = 0;
    let mut vec = 0;
    let obj = frontend::call_method(m.bytes, nz(heap), 1, 24, rt.host);
    let rc = if obj == 0 {
        0x8c90200a
    } else {
        (|| {
            m.put_word(obj, engine);
            for pass in 1..=2 {
                let (r, list) = pipeline_lookup(m, resources, keys.components);
                if r >= 0x80000000 {
                    return r;
                }
                let (r, it) = pipeline_iterator(m, m.word(list + 8), rt.host);
                iter = it;
                if r >= 0x80000000 {
                    return r;
                }
                let mut i = 0u16;
                loop {
                    let (r, kind, list) = pipeline_next(m, iter);
                    if r >= 0x80000000 {
                        break;
                    }
                    if pass == 1 {
                        if kind == 0 || normalization::strcmp_at(m.bytes, kind, keys.component) != 0
                        {
                            return 0x8c902004;
                        }
                        let s = m.word(list);
                        if s == 0 || m.byte(s) == 0 {
                            return 0x8c902004;
                        }
                    } else {
                        let s = m.word(list);
                        let mut reused = false;
                        if spec != 0 && s != 0 && m.byte(s) != 0 {
                            for j in 0..half(m, spec + 8) {
                                let ent = m.word(m.word(spec + 4) + u32::from(j) * 4);
                                if normalization::strcmp_at(m.bytes, m.word(ent + 4), s) == 0
                                    && ent != 0
                                {
                                    m.put_word(m.word(obj + 4) + u32::from(i) * 4, ent);
                                    m.put_half(ent, half(m, ent).wrapping_add(1));
                                    reused = true;
                                    break;
                                }
                            }
                        }
                        if !reused {
                            if m.word(obj + 12) != 0 {
                                let q = pipeline_string(m, heap, s, rt.host);
                                m.put_word(m.word(obj + 20) + (u32::from(i) + 2) * 4, q);
                                frontend::begin_stage_hook(nz(m.word(obj + 12)));
                            }
                            let ent = frontend::call_method(m.bytes, nz(heap), 1, 20, rt.host);
                            if ent == 0 {
                                return 0x8c90200a;
                            }
                            m.put_word(m.word(obj + 4) + u32::from(i) * 4, ent);
                            m.put_half(ent, 1);
                            let q = pipeline_string(m, heap, s, rt.host);
                            m.put_word(ent + 4, q);
                            m.bytes[(ent + 12) as usize..(ent + 20) as usize]
                                .copy_from_slice(&cached_pair(rt.env));
                            if q == 0 {
                                return 0x8c90200a;
                            }
                            let mut query = BrokerQuery {
                                host: &mut *rt.host,
                                table: keys.builtin_table,
                                scratch: stack + 32,
                            };
                            let r = crate::boot_part_b::fn_178510(
                                m,
                                m.word(engine + 12),
                                s,
                                1,
                                vec + u32::from(i) * 8,
                                ent + 8,
                                &mut query,
                            );
                            if r >= 0x80000000 {
                                return r;
                            }
                            if m.word(obj + 12) != 0 {
                                frontend::end_stage_hook(nz(m.word(obj + 12)));
                            }
                        }
                    }
                    i = i.wrapping_add(1);
                }
                frontend::release_owner(m.bytes, nz(iter), rt.host);
                iter = 0;
                if pass == 1 {
                    let arr = frontend::call_method(m.bytes, nz(heap), u32::from(i), 4, rt.host);
                    m.put_word(obj + 4, arr);
                    if arr == 0 {
                        return 0x8c90200a;
                    }
                    vec = frontend::call_method(m.bytes, nz(heap), u32::from(i), 8, rt.host);
                    if vec == 0 {
                        return 0x8c90200a;
                    }
                    m.put_half(obj + 8, i);
                    let level =
                        frontend::max_slot_level(m.bytes, nz(m.word(engine + 16)), |slot| {
                            rt.host.level(slot)
                        });
                    if level >= 2 {
                        crate::boot_part_b::fn_19854c(
                            m,
                            handle,
                            u32::from(i) + 3,
                            obj + 12,
                            &mut TraceCreator {
                                host: rt.host,
                                env: rt.env,
                            },
                        );
                        frontend::begin_stage_hook(nz(m.word(obj + 12)));
                        if m.word(obj + 12) != 0 {
                            let names = frontend::call_method(
                                m.bytes,
                                nz(heap),
                                u32::from(i) + 3,
                                4,
                                rt.host,
                            );
                            m.put_word(obj + 20, names);
                            if names == 0 {
                                let mut free = None;
                                frontend::release_trace_entry(
                                    m.bytes,
                                    nz(m.word(obj + 12)),
                                    m.word(obj + 16),
                                    |h, p| free = Some((h, p)),
                                );
                                if let Some((h, p)) = free {
                                    frontend::heap_release(m.bytes, nz(h), nz(p), rt.host);
                                }
                                m.bytes[(obj + 12) as usize..(obj + 20) as usize]
                                    .copy_from_slice(&cached_pair(rt.env));
                            } else {
                                for (index, key) in [
                                    (0, keys.total),
                                    (1, keys.input),
                                    (u32::from(i) + 2, keys.output),
                                ] {
                                    let q = pipeline_string(m, heap, key, rt.host);
                                    m.put_word(names + index * 4, q);
                                }
                            }
                        }
                    }
                }
            }
            let (r, list) = pipeline_lookup(m, resources, keys.parameters);
            if r < 0x80000000 {
                let (r, it) = pipeline_iterator(m, m.word(list + 8), rt.host);
                iter = it;
                if r < 0x80000000 {
                    loop {
                        let (r, kind, list) = pipeline_next(m, iter);
                        if r >= 0x80000000 {
                            break;
                        }
                        if kind != 0 && m.byte(kind) != 0 {
                            let s = m.word(list);
                            rt.set(
                                m,
                                m.word(engine + 20),
                                kind,
                                if s == 0 { keys.empty } else { s },
                                false,
                            );
                        }
                    }
                    frontend::release_owner(m.bytes, nz(iter), rt.host);
                    iter = 0;
                }
            }
            let (r, list) = pipeline_lookup(m, resources, keys.objects);
            if r < 0x80000000 {
                let (r, it) = pipeline_iterator(m, m.word(list + 8), rt.host);
                iter = it;
                if r < 0x80000000 {
                    loop {
                        let (r, kind, list) = pipeline_next(m, iter);
                        if r >= 0x80000000 {
                            break;
                        }
                        let s = m.word(list);
                        if kind == 0 || s == 0 || m.byte(s) == 0 {
                            return 0x8c902004;
                        }
                        let r = fn_2c6ddc(
                            m,
                            handle,
                            kind,
                            s,
                            resource_list,
                            out_reopened,
                            stack + 64,
                            keys.builtin_table,
                            rt.env,
                            rt.host,
                            rt.ops,
                        );
                        if r >= 0x80000000 {
                            return r;
                        }
                    }
                    frontend::release_owner(m.bytes, nz(iter), rt.host);
                    iter = 0;
                }
            }
            let mut status = 0;
            for j in 0..half(m, obj + 8) {
                let ent = m.word(m.word(obj + 4) + u32::from(j) * 4);
                if ent == 0 {
                    continue;
                }
                if m.word(obj + 12) != 0 {
                    frontend::begin_stage_hook(nz(m.word(obj + 12)));
                }
                let cached = cached_pair(rt.env);
                let al = u32::from_le_bytes(cached[..4].try_into().unwrap());
                let ah = u32::from_le_bytes(cached[4..].try_into().unwrap());
                let iface = m.word(ent + 8);
                if frontend::match_halves(m.word(ent + 12), m.word(ent + 16), al, ah) != 0 {
                    status = rt.host.resource_call(
                        m,
                        m.word(iface + 16),
                        &[
                            m.word(vec + u32::from(j) * 8),
                            m.word(vec + u32::from(j) * 8 + 4),
                            handle[0],
                            handle[1],
                            ent + 12,
                        ],
                    );
                } else if m.word(iface + 24) != 0 {
                    m.bytes[out_reopened as usize] = 1;
                    status = rt.host.resource_call(
                        m,
                        m.word(iface + 24),
                        &[m.word(ent + 12), m.word(ent + 16)],
                    );
                }
                if m.word(obj + 12) != 0 {
                    frontend::end_stage_hook(nz(m.word(obj + 12)));
                }
                if status >= 0x80000000 {
                    return status;
                }
            }
            let (mut status, list) = pipeline_lookup(m, resources, keys.resources);
            if status < 0x80000000 {
                let (r, it) = pipeline_iterator(m, m.word(list + 8), rt.host);
                status = r;
                iter = it;
            }
            if status >= 0x80000000 {
                if !matches!(status & 0x1fff, 13 | 20) {
                    return status;
                }
                status = 0;
            }
            loop {
                let (r, kind, list) = pipeline_next(m, iter);
                if r >= 0x80000000 {
                    break;
                }
                if kind == 0 || normalization::strcmp_at(m.bytes, kind, keys.resource) != 0 {
                    continue;
                }
                let s = m.word(list);
                if s == 0 || m.byte(s) == 0 {
                    continue;
                }
                let (r, sub) = if m.word(list + 4) == 0 {
                    (0x8c902004, 0)
                } else {
                    pipeline_lookup(m, m.word(list + 4), keys.content_type)
                };
                if r >= 0x80000000 || sub == 0 || m.byte(sub) == 0 {
                    status = 0x8c902004;
                    continue;
                }
                let cached = cached_pair(rt.env);
                let h0 = u32::from_le_bytes(cached[..4].try_into().unwrap());
                let h1 = u32::from_le_bytes(cached[4..].try_into().unwrap());
                fn_2c6ac0(
                    m,
                    obj,
                    1,
                    sub,
                    s,
                    [0, 0, h0, h1],
                    0,
                    stack + 128,
                    rt.env,
                    pipeline,
                    rt.host,
                );
            }
            frontend::release_owner(m.bytes, nz(iter), rt.host);
            iter = 0;
            if m.word(obj + 12) != 0 && m.word(obj + 20) != 0 {
                frontend::end_stage_hook(nz(m.word(obj + 12)));
                frontend::end_trace_hook(nz(m.word(obj + 12)), nz(m.word(engine + 16)));
            }
            status
        })()
    };
    if iter != 0 {
        frontend::release_owner(m.bytes, nz(iter), rt.host);
    }
    if vec != 0 {
        frontend::heap_release(m.bytes, nz(heap), Some(vec), rt.host);
    }
    if rc < 0x80000000 {
        m.put_word(out, obj);
    } else if obj != 0 {
        fn_2c66c8(m, obj, rt.env, rt.host);
    }
    m.bytes[stack as usize..stack as usize + 1280].copy_from_slice(&saved);
    rc
}

/// Caller-owned PCRE work buffers for render part d's live fn_2ef300 port.
/// The RefCell enforces exclusive scratch use without allocating or retaining
/// an arena borrow across parser callbacks.
pub struct EscapeExecution<'a> {
    pub scratch: core::cell::RefCell<crate::render_part_d::EscapeScratch<'a>>,
    pub tables: &'a crate::regex::PcreTables<'a>,
}
pub struct EscapeEnvironment<'a> {
    /// Required when an escape command is reached; plain text needs no regex buffers.
    pub execution: Option<EscapeExecution<'a>>,
    pub parameters: [(u16, u32); 4],
    pub spaces: &'a [u8],
    pub following: [u8; 40],
    pub reset_name: u32,
    pub fallback: u32,
    pub audio: u32,
    pub marker: u32,
}
fn escape_has_string(tag: u16) -> bool {
    matches!(
        tag,
        6 | 12 | 21 | 50 | 36 | 39 | 37 | 34 | 22 | 40 | 26 | 29
    )
}
fn escape_append(
    m: &mut Memory<'_>,
    buffers: u32,
    text: u32,
    size: u16,
    env: &Environment<'_>,
    host: &mut impl ResourceHost,
) -> u32 {
    if buffers == 0 || text == 0 {
        return 0x80002007;
    }
    let (rc, mut buffer) = fetch_parser(m, buffers, 0x3f2, 1, env, host);
    if rc >= 0x80000000 {
        return rc;
    }
    let count = half(m, buffer + 8).wrapping_add(size >> 1);
    if half(m, buffer + 10) < count {
        let (rc, b) = fetch_parser(m, buffers, 0x3f2, u32::from(count), env, host);
        if rc >= 0x80000000 {
            return rc;
        }
        buffer = b;
    }
    crate::fe_text::memcpy_195ae8(
        m.bytes,
        m.word(buffer + 4) + u32::from(half(m, buffer + 8)) * 2,
        text,
        u32::from(size),
    );
    m.put_half(buffer + 8, count);
    0
}
fn escape_event(
    m: &mut Memory<'_>,
    buffers: u32,
    event: u32,
    env: &Environment<'_>,
    host: &mut impl ResourceHost,
) -> u32 {
    crate::text_input::append_event(
        m.bytes,
        buffers,
        event,
        &mut FetchHost {
            host,
            env: &env.engine,
        },
    )
}
/// fn_2c4914. Tokenize escape sequences into text and marker buffers.
/// stack is 256 disjoint bytes for the original local event records.
pub fn fn_2c4914<H, L, O, B, E, C>(
    m: &mut Memory<'_>,
    state: u32,
    buffers: u32,
    stack: u32,
    escape: &EscapeEnvironment<'_>,
    rt: &mut ParserRuntime<'_, '_, H, L, O, B, E, C>,
) -> u32
where
    H: ResourceHost + crate::render_part_b::ParameterHost,
    L: frontend::StateLock,
    O: frontend::TreeOps,
    B: frontend::BeforeChange,
    E: frontend::ElementCallback,
    C: FnMut(u32, u32) -> i32,
{
    if state == 0 || buffers == 0 {
        return 0x80002007;
    }
    let (rc, context) = resolve(m, [m.word(state), m.word(state + 4)], &rt.env.engine);
    if rc >= 0x80000000 {
        return 0x80002007;
    }
    let (rc, input) = fetch_parser(m, buffers, 1000, 1, rt.env, rt.host);
    if rc >= 0x80000000 {
        return rc;
    }
    let start = m.word(input + 4);
    if start == 0 {
        return 0x80002007;
    }
    if half(m, input + 2) != 2 {
        return 0x80002019;
    }
    let mut saved = [0u8; 256];
    saved.copy_from_slice(&m.bytes[stack as usize..stack as usize + 256]);
    let event = stack;
    let following = stack + 40;
    let temp = stack + 80;
    let reset = stack + 120;
    m.bytes[event as usize..event as usize + 40].fill(0);
    m.bytes[following as usize..following as usize + 40].copy_from_slice(&escape.following);
    m.put_half(event + 10, 0x4000);
    m.put_word(event + 12, m.word(state + 12));
    m.put_word(event + 20, m.word(state + 16));
    let heap = m.word(context + 4);
    let props = m.word(context + 20);
    let mut string_value = 0;
    let rc = (|| {
        let rc = escape_event(m, buffers, event, rt.env, rt.host);
        if rc >= 0x80000000 {
            return rc;
        }
        // Keep both C fetches even though the inQuote scan's result is dead.
        let (rc, _) = fetch_parser(m, buffers, 0x3f3, 1, rt.env, rt.host);
        if rc >= 0x80000000 {
            return rc;
        }
        let (rc, _) = fetch_parser(m, buffers, 0x3f4, 1, rt.env, rt.host);
        if rc >= 0x80000000 {
            return rc;
        }
        if m.byte(state + 21) == 1 {
            m.put_byte(state + 21, 0);
            for (tag, key) in escape.parameters {
                if matches!(tag, 7 | 31) {
                    let rc = frontend::get_param_uint(
                        m.bytes,
                        nz(props),
                        key,
                        stack + 164,
                        rt.lock,
                        &mut *rt.compare,
                    );
                    if rc & 0x1fff == 20 {
                        continue;
                    }
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    m.put_byte(event, 1);
                    m.put_half(event + 10, tag);
                    m.put_word(event + 12, m.word(state + 12));
                    m.put_word(event + 16, 0);
                    m.put_word(event + 20, m.word(state + 16));
                    m.put_word(event + 24, 0);
                    m.put_byte(event + 36, 0);
                    m.put_word(event + 32, 0);
                    m.put_word(event + 28, m.word(stack + 164));
                    let rc = escape_event(m, buffers, event, rt.env, rt.host);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                } else if matches!(tag, 21 | 36) {
                    let rc = frontend::fetch_param(
                        m.bytes,
                        nz(props),
                        key,
                        Some(&mut string_value),
                        rt.lock,
                        &mut *rt.compare,
                    );
                    if rc & 0x1fff != 20 {
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        let length = normalization::strlen8(m.bytes, string_value) as u16;
                        let wide = frontend::call_method(
                            m.bytes,
                            nz(heap),
                            1,
                            u32::from(length) * 2 + 2,
                            rt.host,
                        );
                        if wide == 0 {
                            return 0x8000200a;
                        }
                        crate::render_part_b::fn_176354(m, wide, string_value, u32::from(length));
                        m.put_half(wide + u32::from(length) * 2, 0);
                        m.put_byte(event, 1);
                        m.put_half(event + 10, tag);
                        m.put_word(event + 12, m.word(state + 12));
                        m.put_word(event + 16, 0);
                        m.put_word(event + 20, m.word(state + 16));
                        m.put_word(event + 24, 0);
                        m.put_word(event + 28, wide);
                        let rc = escape_event(m, buffers, event, rt.env, rt.host);
                        frontend::heap_release(m.bytes, nz(heap), Some(wide), rt.host);
                        if rc >= 0x80000000 {
                            return rc;
                        }
                    }
                    normalization::null_guard(string_value, 0x8b702007);
                    string_value = 0;
                }
            }
        }
        let end = start + u32::from(half(m, input + 8)) * 2;
        let mut cursor = start;
        if m.byte(state + 20) != 0 {
            let mut quoted = false;
            while cursor + 2 <= end {
                if !quoted && half(m, cursor) == 92 {
                    break;
                }
                if half(m, cursor) == 34 {
                    quoted = !quoted;
                }
                cursor += 2;
            }
            if cursor + 2 <= end {
                m.put_byte(state + 20, 0);
                cursor += 2;
            }
        }
        let mut plain = cursor;
        let mut consumed = 0u16;
        let mut limit = (u32::from(half(m, input + 8)) * 2) as u16;
        m.put_half(event + 10, 0);
        while cursor + 2 <= end && u32::from(limit) == u32::from(half(m, input + 8)) * 2 {
            while cursor + 2 <= end && half(m, cursor) != 27 {
                cursor += 2;
            }
            if cursor + 2 > end {
                break;
            }
            if cursor + 2 < end {
                if half(m, cursor + 2) == 92 {
                    let mut quoted = false;
                    let mut value = false;
                    let mut p = cursor + 4;
                    while p + 2 <= end
                        && normalization::u16chr(escape.spaces, 0, half(m, p)).is_some()
                    {
                        p += 2;
                    }
                    let name = p;
                    while p + 2 <= end
                        && !normalization::u16chr(escape.spaces, 0, half(m, p)).is_some()
                        && !matches!(half(m, p), 61 | 92)
                    {
                        p += 2;
                    }
                    let command = stack + 168;
                    m.bytes[command as usize..command as usize + 6].fill(0);
                    let len = ((p - name) / 2).min(6);
                    let mut audio = false;
                    let mut marker = false;
                    if len != 0 {
                        crate::fe_text::narrow_n_17649c(m.bytes, command, name, len);
                        audio =
                            crate::fe_text::strcmp_ci_1769d0(m.bytes, command, escape.audio) == 0;
                        if !audio {
                            marker =
                                crate::fe_text::strcmp_ci_1769d0(m.bytes, command, escape.marker)
                                    == 0;
                        }
                    }
                    while p + 2 <= end {
                        if !quoted && half(m, p) == 92 {
                            if !(audio || marker) {
                                break;
                            }
                            if (audio || marker) && value && (p + 3 >= end || half(m, p + 2) != 92)
                            {
                                break;
                            }
                        }
                        if audio && half(m, p) == 34 {
                            quoted = !quoted;
                        }
                        if !value && half(m, p) == 61 {
                            value = true;
                        }
                        if value
                            && ((quoted && audio) || marker)
                            && half(m, p) == 92
                            && p + 3 < end
                            && matches!(half(m, p + 2), 92 | 34)
                        {
                            p += 4;
                            continue;
                        }
                        p += 2;
                    }
                    if p + 2 > end && m.byte(input + 12) == 0 {
                        limit = cursor.wrapping_sub(start) as u16;
                    }
                    if limit == 0 {
                        return 0x80002009;
                    }
                } else if cursor + 4 <= end
                    && half(m, cursor + 2) == 47
                    && matches!(half(m, cursor + 4), 43 | 62 | 94)
                {
                    let mut p = cursor + 4;
                    while p + 4 <= end
                        && !(half(m, p) == 47 && matches!(half(m, p + 2), 43 | 62 | 94))
                    {
                        p += 2;
                    }
                    if p + 2 > end && m.byte(input + 12) == 0 {
                        limit = (cursor - start) as u16;
                    }
                    if limit == 0 {
                        return 0x80002009;
                    }
                } else if end - cursor < 24 && m.byte(input + 12) == 0 {
                    limit = (cursor - start) as u16;
                }
            } else if m.byte(input + 12) == 0 {
                limit = (cursor - start) as u16;
            }
            if u32::from(limit) != u32::from(half(m, input + 8)) * 2 {
                break;
            }
            let size = cursor.wrapping_sub(plain) as u16;
            if size != 0 {
                let rc = escape_append(m, buffers, plain, size, rt.env, rt.host);
                if rc >= 0x80000000 {
                    return rc;
                }
                consumed = consumed.wrapping_add(size);
            }
            m.put_word(event + 28, 0);
            // Render part d landed this callee during round 3. Execute its
            // real port rather than retaining the earlier scripted boundary.
            let parsed = {
                let execution = escape
                    .execution
                    .as_ref()
                    .expect("fn_2ef300 requires caller-owned EscapeExecution buffers");
                let mut scratch = execution.scratch.borrow_mut();
                let parser = m.word(state + 8);
                crate::render_part_d::escape_parse(
                    m.bytes,
                    parser,
                    cursor,
                    end.wrapping_sub(cursor) as u16 as u32,
                    event,
                    &mut scratch,
                    execution.tables,
                    &mut FetchHost {
                        host: rt.host,
                        env: &rt.env.engine,
                    },
                )
            };
            let mut literal = parsed != 0;
            if m.word(event + 32) != 0 && !matches!(m.word(event + 28), 43 | 70 | 71) {
                frontend::heap_release(m.bytes, nz(heap), nz(m.word(event + 32)), rt.host);
                m.put_word(event + 32, 0);
                m.put_byte(event + 36, 0);
                literal = true;
            }
            if literal {
                let rc = escape_append(m, buffers, cursor, 2, rt.env, rt.host);
                if rc >= 0x80000000 {
                    return rc;
                }
                cursor += 2;
                consumed = consumed.wrapping_add(2);
                plain = cursor;
                continue;
            }
            if escape_has_string(half(m, event + 10)) && m.word(event + 28) == 0 {
                m.put_byte(event, 0);
                m.put_word(event + 4, 0);
                m.put_half(event + 8, 0);
                m.put_half(event + 10, 0x4000);
                m.put_byte(event + 36, 0);
                m.put_word(event + 32, 0);
            } else {
                if m.word(event + 32) != 0 {
                    let mode = match m.word(event + 28) {
                        43 => 1,
                        70 => 2,
                        71 => 0,
                        _ => m.byte(event + 36),
                    };
                    m.put_byte(event + 36, mode);
                    m.put_word(event + 28, 43);
                }
                if half(m, event + 10) == 0 {
                    m.put_byte(event, 0);
                    m.put_word(event + 4, 0);
                    m.put_half(event + 8, 0);
                    m.put_half(event + 10, 999);
                    m.put_byte(event + 36, 0);
                    m.put_word(event + 32, 0);
                }
            }
            m.put_byte(event, 0);
            m.put_word(event + 12, m.word(state + 12).wrapping_add(cursor - start));
            m.put_word(
                event + 20,
                m.word(state + 16).wrapping_add(u32::from(consumed)),
            );
            let rc = escape_event(m, buffers, event, rt.env, rt.host);
            if escape_has_string(half(m, event + 10)) && m.word(event + 28) != 0 {
                frontend::heap_release(m.bytes, nz(heap), nz(m.word(event + 28)), rt.host);
            }
            m.put_word(event + 28, 0);
            if rc >= 0x80000000 {
                return rc;
            }
            if half(m, event + 10) == 4 {
                m.put_half(following + 10, 2);
                m.put_word(
                    following + 12,
                    m.word(event + 12).wrapping_add(m.word(event + 16)),
                );
                m.put_word(following + 16, 0);
                m.put_word(following + 20, m.word(event + 20));
                m.put_word(following + 24, 0);
                m.put_word(following + 28, 1);
                let rc = escape_event(m, buffers, following, rt.env, rt.host);
                if rc >= 0x80000000 {
                    return rc;
                }
            } else if half(m, event + 10) == 13 {
                let (rc, reset_context) =
                    resolve(m, [m.word(state), m.word(state + 4)], &rt.env.engine);
                if rc >= 0x80000000 {
                    return 0x80002007;
                }
                m.bytes[reset as usize..reset as usize + 40].fill(0);
                m.put_word(
                    reset + 12,
                    m.word(event + 12).wrapping_add(m.word(event + 16)),
                );
                m.put_word(reset + 20, m.word(event + 20));
                m.put_word(reset + 24, m.word(event + 24));
                let out = stack + 160;
                m.put_word(out, 0);
                let mut rc = crate::render_part_d::initial_values(
                    m.bytes,
                    m.word(state + 8),
                    out,
                    stack + 176,
                    escape.fallback,
                    &mut FetchHost {
                        host: rt.host,
                        env: &rt.env.engine,
                    },
                );
                let defaults = m.word(out);
                if rc < 0x80000000 {
                    'values: for pass in 0..2 {
                        for i in 1..40 {
                            let e = defaults + i * 40;
                            let tag = half(m, e + 10);
                            let string = escape_has_string(tag);
                            if (pass == 0 && tag != 0 && !string) || (pass == 1 && string) {
                                m.put_word(
                                    e + 12,
                                    m.word(event + 12).wrapping_add(m.word(event + 16)),
                                );
                                m.put_word(e + 24, 0);
                                m.put_word(e + 20, m.word(event + 20));
                                m.put_word(e + 24, m.word(event + 24));
                                rc = escape_event(m, buffers, e, rt.env, rt.host);
                                if rc >= 0x80000000 {
                                    break 'values;
                                }
                                if pass == 1 && m.word(e + 28) != 0 {
                                    frontend::heap_release(
                                        m.bytes,
                                        nz(m.word(reset_context + 4)),
                                        nz(m.word(e + 28)),
                                        rt.host,
                                    );
                                    m.put_word(e + 28, 0);
                                }
                            }
                        }
                    }
                    if rc < 0x80000000 {
                        m.put_half(reset + 10, 22);
                        m.put_byte(reset, 1);
                        m.put_word(reset + 28, escape.reset_name);
                        m.put_word(reset + 4, 0);
                        rc = escape_event(m, buffers, reset, rt.env, rt.host);
                    }
                }
                if defaults != 0 {
                    frontend::heap_release(
                        m.bytes,
                        nz(m.word(reset_context + 4)),
                        Some(defaults),
                        rt.host,
                    );
                }
                if rc >= 0x80000000 {
                    return rc;
                }
            }
            cursor = cursor.wrapping_add(m.word(event + 16));
            m.put_word(event + 16, 0);
            m.put_half(event + 10, 0);
            m.put_word(event + 32, 0);
            m.put_byte(event + 36, 0);
            plain = cursor;
        }
        let size = cursor.wrapping_sub(plain) as u16;
        let rc = escape_append(m, buffers, plain, size, rt.env, rt.host);
        if rc >= 0x80000000 {
            return rc;
        }
        consumed = consumed.wrapping_add(size);
        for kind in [0x3f2, 0x3f3, 0x3f4] {
            let final_flag = m.byte(input + 12);
            let (rc, b) = fetch_parser(m, buffers, kind, 1, rt.env, rt.host);
            if rc >= 0x80000000 {
                return rc;
            }
            m.put_byte(b + 12, final_flag);
        }
        if m.byte(input + 12) == 0 {
            m.put_word(state + 12, m.word(state + 12).wrapping_add(cursor - start));
            m.put_word(
                state + 16,
                m.word(state + 16).wrapping_add(u32::from(consumed)),
            );
        } else {
            m.put_word(state + 12, 0);
            m.put_word(state + 16, 0);
            m.put_byte(state + 21, 0);
        }
        let mut more = false;
        if m.byte(input + 12) != 1 {
            let (rc, b) = fetch_parser(m, buffers, 0x3f2, 1, rt.env, rt.host);
            if rc >= 0x80000000 {
                return rc;
            }
            let count = half(m, b + 8);
            if count > 3 {
                let end = m.word(b + 4) + u32::from(count) * 2;
                more = matches!(half(m, end - 4), 46 | 0xff0e | 63 | 33)
                    && normalization::u16chr(escape.spaces, 0, half(m, end - 2)).is_some();
            }
        }
        let (rc, b) = fetch_parser(m, buffers, 0x3f3, 1, rt.env, rt.host);
        if rc >= 0x80000000 {
            return rc;
        }
        let data = m.word(b + 4);
        let mut i = 1u16;
        while i < half(m, b + 8) {
            let p = data + u32::from(i) * 40;
            if half(m, p + 10) == 7 && matches!(m.word(p + 28), 16 | 60) {
                let value = m.word(p + 28);
                i += 1;
                while i < half(m, b + 8) && half(m, data + u32::from(i) * 40 + 10) != 7 {
                    i += 1;
                }
                if i < half(m, b + 8) && m.word(data + u32::from(i) * 40 + 28) == value {
                    m.put_word(data + u32::from(i) * 40 + 28, 15);
                }
            }
            i = i.wrapping_add(1);
        }
        let (rc, b) = fetch_parser(m, buffers, 0x3f3, 1, rt.env, rt.host);
        if rc >= 0x80000000 {
            return rc;
        }
        let data = m.word(b + 4);
        for i in 1..half(m, b + 8) {
            if half(m, data + u32::from(i) * 40 + 10) == 0x4000 {
                let mut j = i + 1;
                while j < half(m, b + 8) && half(m, data + u32::from(j) * 40 + 10) == 0x4000 {
                    j += 1;
                }
                if j < half(m, b + 8) && half(m, data + u32::from(j) * 40 + 10) == 0x4000 {
                    if j + 1 == half(m, b + 8) {
                        m.put_half(b + 8, half(m, b + 8) - 1);
                    } else {
                        copy_bytes(
                            m,
                            data + u32::from(i + 1) * 40,
                            data + u32::from(j + 1) * 40,
                            u32::from(half(m, b + 8) - j) * 40,
                        );
                        m.put_half(b + 8, half(m, b + 8).wrapping_sub(j));
                    }
                }
                break;
            }
        }
        let (rc, b) = fetch_parser(m, buffers, 0x3f3, 1, rt.env, rt.host);
        if rc >= 0x80000000 {
            return rc;
        }
        let data = m.word(b + 4);
        let mut quoted = false;
        let mut quote = 0;
        let mut i = 0u16;
        while i < half(m, b + 8) {
            let e = data + u32::from(i) * 40;
            if half(m, e + 10) == 7 {
                if m.word(e + 28) == 43 {
                    quoted = !quoted;
                    quote = i;
                } else {
                    quoted = false;
                }
            } else if half(m, e + 10) == 0x4000 && quoted {
                let q = data + u32::from(quote) * 40;
                let next = i + 1;
                m.put_word(e + 12, m.word(q + 12));
                m.put_word(e + 20, m.word(q + 20));
                crate::fe_text::memcpy_195ae8(m.bytes, temp, e, 40);
                if next < half(m, b + 8) {
                    copy_bytes(m, e, e + 40, u32::from(half(m, b + 8) - i - 1) * 40);
                }
                m.put_half(b + 8, half(m, b + 8) - 1);
                copy_bytes(m, q + 40, q, u32::from(half(m, b + 8) - quote) * 40);
                crate::fe_text::memcpy_195ae8(m.bytes, q, temp, 40);
                quoted = false;
                i = next;
                m.put_half(b + 8, half(m, b + 8) + 1);
                quote = 0;
            }
            i = i.wrapping_add(1);
        }
        copy_bytes(m, start, cursor, end.wrapping_sub(cursor) as u16 as u32);
        m.put_half(input + 8, (end.wrapping_sub(cursor) as u16) >> 1);
        if more {
            0x80002009
        } else {
            rc
        }
    })();
    if string_value != 0 {
        normalization::null_guard(string_value, 0x8b702007);
    }
    m.bytes[stack as usize..stack as usize + 256].copy_from_slice(&saved);
    rc
}

fn escape_event_m(
    m: &mut Memory<'_>,
    buffers: u32,
    event: u32,
    env: &Environment<'_>,
    host: &mut impl crate::normalization::NormalizationHost,
) -> u32 {
    crate::text_input::append_event(m.bytes, buffers, event, host)
}
fn escape_append_m(
    m: &mut Memory<'_>,
    buffers: u32,
    text: u32,
    size: u16,
    env: &Environment<'_>,
    allocator: &mut impl ParserAllocatorM,
) -> u32 {
    if buffers == 0 || text == 0 {
        return 0x80002007;
    }
    let (rc, mut buffer) = fetch_parser_m(m, buffers, 0x3f2, 1, env, allocator);
    if rc >= 0x80000000 {
        return rc;
    }
    let count = half(m, buffer + 8).wrapping_add(size >> 1);
    if half(m, buffer + 10) < count {
        let (rc, b) = fetch_parser_m(m, buffers, 0x3f2, u32::from(count), env, allocator);
        if rc >= 0x80000000 {
            return rc;
        }
        buffer = b;
    }
    crate::fe_text::memcpy_195ae8(
        m.bytes,
        m.word(buffer + 4) + u32::from(half(m, buffer + 8)) * 2,
        text,
        u32::from(size),
    );
    m.put_half(buffer + 8, count);
    0
}
/// Counted guest-heap alloc with the caller's arena, through a
/// render-part-c host. Mirrors `frontend::call_method` but passes the
/// arena to the method instead of dropping it.
fn call_on_heap_m(
    mem: &mut [u8],
    heap: Option<u32>,
    count: u32,
    size: u32,
    host: &mut impl crate::render_part_c::Host,
) -> u32 {
    let Some(heap) = heap else {
        return 0;
    };
    if count == 0 || size == 0 {
        return 0;
    }
    let object = crate::frontend::mem_word(mem, heap + 4);
    let method = crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, heap) + 4);
    crate::render_part_c::Host::call(
        host,
        &mut Memory { base: 0, bytes: mem },
        method,
        &[object, count, size],
    )
}
/// Memory-passing `fn_2c4914`: the escape-sequence processor with the guest
/// heap on the caller's arena. Same boundaries as [`fn_2c4914`], but every
/// allocation, counted alloc and release routes through `allocator`, the regex
/// execution buffers come from the caller, and the counted method call
/// dispatches through the host as a render-part-c host with the arena.
pub fn fn_2c4914_m<H, L, O, B, E, C>(
    m: &mut Memory<'_>,
    state: u32,
    buffers: u32,
    stack: u32,
    escape: &EscapeEnvironment<'_>,
    rt: &mut ParserRuntime<'_, '_, H, L, O, B, E, C>,
    allocator: &mut impl ParserAllocatorM,
) -> u32
where
    H: ResourceHost
        + crate::render_part_b::ParameterHost
        + crate::render_part_c::Host,
    L: frontend::StateLock,
    O: frontend::TreeOps,
    B: frontend::BeforeChange,
    E: frontend::ElementCallback,
    C: FnMut(u32, u32) -> i32,
{
    if state == 0 || buffers == 0 {
        return 0x80002007;
    }
    let (rc, context) = resolve(m, [m.word(state), m.word(state + 4)], &rt.env.engine);
    if rc >= 0x80000000 {
        return 0x80002007;
    }
    let (rc, input) = fetch_parser_m(m, buffers, 1000, 1, rt.env, allocator);
    if rc >= 0x80000000 {
        return rc;
    }
    let start = m.word(input + 4);
    if start == 0 {
        return 0x80002007;
    }
    if half(m, input + 2) != 2 {
        return 0x80002019;
    }
    let mut saved = [0u8; 256];
    saved.copy_from_slice(&m.bytes[stack as usize..stack as usize + 256]);
    let event = stack;
    let following = stack + 40;
    let temp = stack + 80;
    let reset = stack + 120;
    m.bytes[event as usize..event as usize + 40].fill(0);
    m.bytes[following as usize..following as usize + 40].copy_from_slice(&escape.following);
    m.put_half(event + 10, 0x4000);
    m.put_word(event + 12, m.word(state + 12));
    m.put_word(event + 20, m.word(state + 16));
    let heap = m.word(context + 4);
    let props = m.word(context + 20);
    let mut string_value = 0;
    let rc = (|| {
        let rc = escape_event_m(m, buffers, event, rt.env, &mut FetchHostM { allocator: &mut *allocator, env: &rt.env.engine });
        if rc >= 0x80000000 {
            return rc;
        }
        // Keep both C fetches even though the inQuote scan's result is dead.
        let (rc, _) = fetch_parser_m(m, buffers, 0x3f3, 1, rt.env, allocator);
        if rc >= 0x80000000 {
            return rc;
        }
        let (rc, _) = fetch_parser_m(m, buffers, 0x3f4, 1, rt.env, allocator);
        if rc >= 0x80000000 {
            return rc;
        }
        if m.byte(state + 21) == 1 {
            m.put_byte(state + 21, 0);
            for (tag, key) in escape.parameters {
                if matches!(tag, 7 | 31) {
                    let rc = frontend::get_param_uint(
                        m.bytes,
                        nz(props),
                        key,
                        stack + 164,
                        rt.lock,
                        &mut *rt.compare,
                    );
                    if rc & 0x1fff == 20 {
                        continue;
                    }
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    m.put_byte(event, 1);
                    m.put_half(event + 10, tag);
                    m.put_word(event + 12, m.word(state + 12));
                    m.put_word(event + 16, 0);
                    m.put_word(event + 20, m.word(state + 16));
                    m.put_word(event + 24, 0);
                    m.put_byte(event + 36, 0);
                    m.put_word(event + 32, 0);
                    m.put_word(event + 28, m.word(stack + 164));
                    let rc = escape_event_m(m, buffers, event, rt.env, &mut FetchHostM { allocator: &mut *allocator, env: &rt.env.engine });
                    if rc >= 0x80000000 {
                        return rc;
                    }
                } else if matches!(tag, 21 | 36) {
                    let rc = frontend::fetch_param(
                        m.bytes,
                        nz(props),
                        key,
                        Some(&mut string_value),
                        rt.lock,
                        &mut *rt.compare,
                    );
                    if rc & 0x1fff != 20 {
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        let length = normalization::strlen8(m.bytes, string_value) as u16;
                        let wide = call_on_heap_m(
                            m.bytes,
                            nz(heap),
                            1,
                            u32::from(length) * 2 + 2,
                            rt.host,
                        );
                        if wide == 0 {
                            return 0x8000200a;
                        }
                        crate::render_part_b::fn_176354(m, wide, string_value, u32::from(length));
                        m.put_half(wide + u32::from(length) * 2, 0);
                        m.put_byte(event, 1);
                        m.put_half(event + 10, tag);
                        m.put_word(event + 12, m.word(state + 12));
                        m.put_word(event + 16, 0);
                        m.put_word(event + 20, m.word(state + 16));
                        m.put_word(event + 24, 0);
                        m.put_word(event + 28, wide);
                        let rc = escape_event_m(m, buffers, event, rt.env, &mut FetchHostM { allocator: &mut *allocator, env: &rt.env.engine });
                        frontend::heap_release_m(m.bytes, nz(heap), Some(wide), allocator);
                        if rc >= 0x80000000 {
                            return rc;
                        }
                    }
                    normalization::null_guard(string_value, 0x8b702007);
                    string_value = 0;
                }
            }
        }
        let end = start + u32::from(half(m, input + 8)) * 2;
        let mut cursor = start;
        if m.byte(state + 20) != 0 {
            let mut quoted = false;
            while cursor + 2 <= end {
                if !quoted && half(m, cursor) == 92 {
                    break;
                }
                if half(m, cursor) == 34 {
                    quoted = !quoted;
                }
                cursor += 2;
            }
            if cursor + 2 <= end {
                m.put_byte(state + 20, 0);
                cursor += 2;
            }
        }
        let mut plain = cursor;
        let mut consumed = 0u16;
        let mut limit = (u32::from(half(m, input + 8)) * 2) as u16;
        m.put_half(event + 10, 0);
        while cursor + 2 <= end && u32::from(limit) == u32::from(half(m, input + 8)) * 2 {
            while cursor + 2 <= end && half(m, cursor) != 27 {
                cursor += 2;
            }
            if cursor + 2 > end {
                break;
            }
            if cursor + 2 < end {
                if half(m, cursor + 2) == 92 {
                    let mut quoted = false;
                    let mut value = false;
                    let mut p = cursor + 4;
                    while p + 2 <= end
                        && normalization::u16chr(escape.spaces, 0, half(m, p)).is_some()
                    {
                        p += 2;
                    }
                    let name = p;
                    while p + 2 <= end
                        && !normalization::u16chr(escape.spaces, 0, half(m, p)).is_some()
                        && !matches!(half(m, p), 61 | 92)
                    {
                        p += 2;
                    }
                    let command = stack + 168;
                    m.bytes[command as usize..command as usize + 6].fill(0);
                    let len = ((p - name) / 2).min(6);
                    let mut audio = false;
                    let mut marker = false;
                    if len != 0 {
                        crate::fe_text::narrow_n_17649c(m.bytes, command, name, len);
                        audio =
                            crate::fe_text::strcmp_ci_1769d0(m.bytes, command, escape.audio) == 0;
                        if !audio {
                            marker =
                                crate::fe_text::strcmp_ci_1769d0(m.bytes, command, escape.marker)
                                    == 0;
                        }
                    }
                    while p + 2 <= end {
                        if !quoted && half(m, p) == 92 {
                            if !(audio || marker) {
                                break;
                            }
                            if (audio || marker) && value && (p + 3 >= end || half(m, p + 2) != 92)
                            {
                                break;
                            }
                        }
                        if audio && half(m, p) == 34 {
                            quoted = !quoted;
                        }
                        if !value && half(m, p) == 61 {
                            value = true;
                        }
                        if value
                            && ((quoted && audio) || marker)
                            && half(m, p) == 92
                            && p + 3 < end
                            && matches!(half(m, p + 2), 92 | 34)
                        {
                            p += 4;
                            continue;
                        }
                        p += 2;
                    }
                    if p + 2 > end && m.byte(input + 12) == 0 {
                        limit = cursor.wrapping_sub(start) as u16;
                    }
                    if limit == 0 {
                        return 0x80002009;
                    }
                } else if cursor + 4 <= end
                    && half(m, cursor + 2) == 47
                    && matches!(half(m, cursor + 4), 43 | 62 | 94)
                {
                    let mut p = cursor + 4;
                    while p + 4 <= end
                        && !(half(m, p) == 47 && matches!(half(m, p + 2), 43 | 62 | 94))
                    {
                        p += 2;
                    }
                    if p + 2 > end && m.byte(input + 12) == 0 {
                        limit = (cursor - start) as u16;
                    }
                    if limit == 0 {
                        return 0x80002009;
                    }
                } else if end - cursor < 24 && m.byte(input + 12) == 0 {
                    limit = (cursor - start) as u16;
                }
            } else if m.byte(input + 12) == 0 {
                limit = (cursor - start) as u16;
            }
            if u32::from(limit) != u32::from(half(m, input + 8)) * 2 {
                break;
            }
            let size = cursor.wrapping_sub(plain) as u16;
            if size != 0 {
                let rc = escape_append_m(m, buffers, plain, size, rt.env, allocator);
                if rc >= 0x80000000 {
                    return rc;
                }
                consumed = consumed.wrapping_add(size);
            }
            m.put_word(event + 28, 0);
            // Render part d landed this callee during round 3. Execute its
            // real port rather than retaining the earlier scripted boundary.
            let parsed = {
                let execution = escape
                    .execution
                    .as_ref()
                    .expect("fn_2ef300 requires caller-owned EscapeExecution buffers");
                let mut scratch = execution.scratch.borrow_mut();
                let parser = m.word(state + 8);
                crate::render_part_d::escape_parse(
                    m.bytes,
                    parser,
                    cursor,
                    end.wrapping_sub(cursor) as u16 as u32,
                    event,
                    &mut scratch,
                    execution.tables,
                    &mut FetchHostM { allocator: &mut *allocator, env: &rt.env.engine },
                )
            };
            let mut literal = parsed != 0;
            if m.word(event + 32) != 0 && !matches!(m.word(event + 28), 43 | 70 | 71) {
                frontend::heap_release_m(m.bytes, nz(heap), nz(m.word(event + 32)), allocator);
                m.put_word(event + 32, 0);
                m.put_byte(event + 36, 0);
                literal = true;
            }
            if literal {
                let rc = escape_append_m(m, buffers, cursor, 2, rt.env, allocator);
                if rc >= 0x80000000 {
                    return rc;
                }
                cursor += 2;
                consumed = consumed.wrapping_add(2);
                plain = cursor;
                continue;
            }
            if escape_has_string(half(m, event + 10)) && m.word(event + 28) == 0 {
                m.put_byte(event, 0);
                m.put_word(event + 4, 0);
                m.put_half(event + 8, 0);
                m.put_half(event + 10, 0x4000);
                m.put_byte(event + 36, 0);
                m.put_word(event + 32, 0);
            } else {
                if m.word(event + 32) != 0 {
                    let mode = match m.word(event + 28) {
                        43 => 1,
                        70 => 2,
                        71 => 0,
                        _ => m.byte(event + 36),
                    };
                    m.put_byte(event + 36, mode);
                    m.put_word(event + 28, 43);
                }
                if half(m, event + 10) == 0 {
                    m.put_byte(event, 0);
                    m.put_word(event + 4, 0);
                    m.put_half(event + 8, 0);
                    m.put_half(event + 10, 999);
                    m.put_byte(event + 36, 0);
                    m.put_word(event + 32, 0);
                }
            }
            m.put_byte(event, 0);
            m.put_word(event + 12, m.word(state + 12).wrapping_add(cursor - start));
            m.put_word(
                event + 20,
                m.word(state + 16).wrapping_add(u32::from(consumed)),
            );
            let rc = escape_event_m(m, buffers, event, rt.env, &mut FetchHostM { allocator: &mut *allocator, env: &rt.env.engine });
            if escape_has_string(half(m, event + 10)) && m.word(event + 28) != 0 {
                frontend::heap_release_m(m.bytes, nz(heap), nz(m.word(event + 28)), allocator);
            }
            m.put_word(event + 28, 0);
            if rc >= 0x80000000 {
                return rc;
            }
            if half(m, event + 10) == 4 {
                m.put_half(following + 10, 2);
                m.put_word(
                    following + 12,
                    m.word(event + 12).wrapping_add(m.word(event + 16)),
                );
                m.put_word(following + 16, 0);
                m.put_word(following + 20, m.word(event + 20));
                m.put_word(following + 24, 0);
                m.put_word(following + 28, 1);
                let rc = escape_event_m(m, buffers, following, rt.env, &mut FetchHostM { allocator: &mut *allocator, env: &rt.env.engine });
                if rc >= 0x80000000 {
                    return rc;
                }
            } else if half(m, event + 10) == 13 {
                let (rc, reset_context) =
                    resolve(m, [m.word(state), m.word(state + 4)], &rt.env.engine);
                if rc >= 0x80000000 {
                    return 0x80002007;
                }
                m.bytes[reset as usize..reset as usize + 40].fill(0);
                m.put_word(
                    reset + 12,
                    m.word(event + 12).wrapping_add(m.word(event + 16)),
                );
                m.put_word(reset + 20, m.word(event + 20));
                m.put_word(reset + 24, m.word(event + 24));
                let out = stack + 160;
                m.put_word(out, 0);
                let mut rc = crate::render_part_d::initial_values(
                    m.bytes,
                    m.word(state + 8),
                    out,
                    stack + 176,
                    escape.fallback,
                    &mut FetchHostM { allocator: &mut *allocator, env: &rt.env.engine },
                );
                let defaults = m.word(out);
                if rc < 0x80000000 {
                    'values: for pass in 0..2 {
                        for i in 1..40 {
                            let e = defaults + i * 40;
                            let tag = half(m, e + 10);
                            let string = escape_has_string(tag);
                            if (pass == 0 && tag != 0 && !string) || (pass == 1 && string) {
                                m.put_word(
                                    e + 12,
                                    m.word(event + 12).wrapping_add(m.word(event + 16)),
                                );
                                m.put_word(e + 24, 0);
                                m.put_word(e + 20, m.word(event + 20));
                                m.put_word(e + 24, m.word(event + 24));
                                rc = escape_event_m(m, buffers, e, rt.env, &mut FetchHostM { allocator: &mut *allocator, env: &rt.env.engine });
                                if rc >= 0x80000000 {
                                    break 'values;
                                }
                                if pass == 1 && m.word(e + 28) != 0 {
                                    frontend::heap_release_m(
                                        m.bytes,
                                        nz(m.word(reset_context + 4)),
                                        nz(m.word(e + 28)),
                                        allocator,
                                    );
                                    m.put_word(e + 28, 0);
                                }
                            }
                        }
                    }
                    if rc < 0x80000000 {
                        m.put_half(reset + 10, 22);
                        m.put_byte(reset, 1);
                        m.put_word(reset + 28, escape.reset_name);
                        m.put_word(reset + 4, 0);
                        rc = escape_event_m(m, buffers, reset, rt.env, &mut FetchHostM { allocator: &mut *allocator, env: &rt.env.engine });
                    }
                }
                if defaults != 0 {
                    frontend::heap_release_m(
                        m.bytes,
                        nz(m.word(reset_context + 4)),
                        Some(defaults),
                        allocator,
                    );
                }
                if rc >= 0x80000000 {
                    return rc;
                }
            }
            cursor = cursor.wrapping_add(m.word(event + 16));
            m.put_word(event + 16, 0);
            m.put_half(event + 10, 0);
            m.put_word(event + 32, 0);
            m.put_byte(event + 36, 0);
            plain = cursor;
        }
        let size = cursor.wrapping_sub(plain) as u16;
        let rc = escape_append_m(m, buffers, plain, size, rt.env, allocator);
        if rc >= 0x80000000 {
            return rc;
        }
        consumed = consumed.wrapping_add(size);
        for kind in [0x3f2, 0x3f3, 0x3f4] {
            let final_flag = m.byte(input + 12);
            let (rc, b) = fetch_parser_m(m, buffers, kind, 1, rt.env, allocator);
            if rc >= 0x80000000 {
                return rc;
            }
            m.put_byte(b + 12, final_flag);
        }
        if m.byte(input + 12) == 0 {
            m.put_word(state + 12, m.word(state + 12).wrapping_add(cursor - start));
            m.put_word(
                state + 16,
                m.word(state + 16).wrapping_add(u32::from(consumed)),
            );
        } else {
            m.put_word(state + 12, 0);
            m.put_word(state + 16, 0);
            m.put_byte(state + 21, 0);
        }
        let mut more = false;
        if m.byte(input + 12) != 1 {
            let (rc, b) = fetch_parser_m(m, buffers, 0x3f2, 1, rt.env, allocator);
            if rc >= 0x80000000 {
                return rc;
            }
            let count = half(m, b + 8);
            if count > 3 {
                let end = m.word(b + 4) + u32::from(count) * 2;
                more = matches!(half(m, end - 4), 46 | 0xff0e | 63 | 33)
                    && normalization::u16chr(escape.spaces, 0, half(m, end - 2)).is_some();
            }
        }
        let (rc, b) = fetch_parser_m(m, buffers, 0x3f3, 1, rt.env, allocator);
        if rc >= 0x80000000 {
            return rc;
        }
        let data = m.word(b + 4);
        let mut i = 1u16;
        while i < half(m, b + 8) {
            let p = data + u32::from(i) * 40;
            if half(m, p + 10) == 7 && matches!(m.word(p + 28), 16 | 60) {
                let value = m.word(p + 28);
                i += 1;
                while i < half(m, b + 8) && half(m, data + u32::from(i) * 40 + 10) != 7 {
                    i += 1;
                }
                if i < half(m, b + 8) && m.word(data + u32::from(i) * 40 + 28) == value {
                    m.put_word(data + u32::from(i) * 40 + 28, 15);
                }
            }
            i = i.wrapping_add(1);
        }
        let (rc, b) = fetch_parser_m(m, buffers, 0x3f3, 1, rt.env, allocator);
        if rc >= 0x80000000 {
            return rc;
        }
        let data = m.word(b + 4);
        for i in 1..half(m, b + 8) {
            if half(m, data + u32::from(i) * 40 + 10) == 0x4000 {
                let mut j = i + 1;
                while j < half(m, b + 8) && half(m, data + u32::from(j) * 40 + 10) == 0x4000 {
                    j += 1;
                }
                if j < half(m, b + 8) && half(m, data + u32::from(j) * 40 + 10) == 0x4000 {
                    if j + 1 == half(m, b + 8) {
                        m.put_half(b + 8, half(m, b + 8) - 1);
                    } else {
                        copy_bytes(
                            m,
                            data + u32::from(i + 1) * 40,
                            data + u32::from(j + 1) * 40,
                            u32::from(half(m, b + 8) - j) * 40,
                        );
                        m.put_half(b + 8, half(m, b + 8).wrapping_sub(j));
                    }
                }
                break;
            }
        }
        let (rc, b) = fetch_parser_m(m, buffers, 0x3f3, 1, rt.env, allocator);
        if rc >= 0x80000000 {
            return rc;
        }
        let data = m.word(b + 4);
        let mut quoted = false;
        let mut quote = 0;
        let mut i = 0u16;
        while i < half(m, b + 8) {
            let e = data + u32::from(i) * 40;
            if half(m, e + 10) == 7 {
                if m.word(e + 28) == 43 {
                    quoted = !quoted;
                    quote = i;
                } else {
                    quoted = false;
                }
            } else if half(m, e + 10) == 0x4000 && quoted {
                let q = data + u32::from(quote) * 40;
                let next = i + 1;
                m.put_word(e + 12, m.word(q + 12));
                m.put_word(e + 20, m.word(q + 20));
                crate::fe_text::memcpy_195ae8(m.bytes, temp, e, 40);
                if next < half(m, b + 8) {
                    copy_bytes(m, e, e + 40, u32::from(half(m, b + 8) - i - 1) * 40);
                }
                m.put_half(b + 8, half(m, b + 8) - 1);
                copy_bytes(m, q + 40, q, u32::from(half(m, b + 8) - quote) * 40);
                crate::fe_text::memcpy_195ae8(m.bytes, q, temp, 40);
                quoted = false;
                i = next;
                m.put_half(b + 8, half(m, b + 8) + 1);
                quote = 0;
            }
            i = i.wrapping_add(1);
        }
        copy_bytes(m, start, cursor, end.wrapping_sub(cursor) as u16 as u32);
        m.put_half(input + 8, (end.wrapping_sub(cursor) as u16) >> 1);
        if more {
            0x80002009
        } else {
            rc
        }
    })();
    if string_value != 0 {
        normalization::null_guard(string_value, 0x8b702007);
    }
    m.bytes[stack as usize..stack as usize + 256].copy_from_slice(&saved);
    rc
}
pub trait SegmentCallbackHost {
    /// Retained legacy callback interface; D_2bd850 now executes its real port.
    fn previous_match(&mut self, event: &[u8], target: &[u8]) -> i32;
}
pub struct SegmentEnvironment<'a> {
    pub first: [u8; 40],
    pub split: [u8; 40],
    pub separators: &'a [u8],
    pub max_length_key: u32,
    pub number_table: &'a [u16; 256],
}
pub(crate) fn segment_scan(
    m: &Memory<'_>,
    ctx: u32,
    list: u32,
    start: &mut u16,
    end: &mut u16,
    mode: &mut u16,
) -> u32 {
    if ctx == 0 || list == 0 {
        return 0x81102007;
    }
    let events = m.word(list + 4);
    let count = half(m, list + 8);
    let mut a = u32::from(*start);
    let mut b = u32::from(*end);
    let mut found = false;
    let mut found_end = false;
    for i in 0..count {
        if found {
            break;
        }
        let ev = events + u32::from(i) * 40;
        let rel = frontend::divmod_u32(
            m.word(ev + 20).wrapping_sub(m.word(events + 20)) as u16 as u32,
            2,
        )
        .0;
        if u32::from(*end) < rel {
            return 0;
        }
        if half(m, ev + 10) != 20 {
            continue;
        }
        let val = m.word(ev + 28) as u16;
        if *mode == 1 && val != 1 {
            if u32::from(*start) < rel && rel < u32::from(*end) {
                b = rel;
                found = true;
                found_end = true;
            } else {
                *mode = val;
            }
        } else if val == 1 {
            if u32::from(*start) <= rel && rel < u32::from(*end) {
                found = true;
                found_end = false;
                *mode = val;
                a = rel;
                for j in i + 1..count {
                    let next = events + u32::from(j) * 40;
                    let rel = frontend::divmod_u32(
                        m.word(next + 20).wrapping_sub(m.word(events + 20)) as u16 as u32,
                        2,
                    )
                    .0;
                    if u32::from(*end) < rel {
                        break;
                    }
                    if half(m, next + 10) == 20 && u32::from(*start) <= rel && rel < u32::from(*end)
                    {
                        found_end = true;
                        b = rel;
                        break;
                    }
                }
            } else {
                *mode = val;
            }
        } else {
            *mode = val;
        }
    }
    if found {
        *start = a as u16;
        if found_end {
            *end = b as u16;
        }
    }
    0
}
fn segment_back(
    m: &mut Memory<'_>,
    it: u32,
    target: &[u8; 40],
    special: bool,
    _host: &mut impl SegmentCallbackHost,
) -> u32 {
    if it == 0 {
        return 0x81102007;
    }
    let arr = m.word(it + 12);
    let data = m.word(arr + 4);
    let mut index = half(m, it + 16);
    let rc = normalization::scan_backward(
        &m.bytes[data as usize..],
        half(m, arr + 2),
        half(m, arr + 8),
        &mut index,
        &mut |event| {
            if special {
                let mut pair = [0u8; 80];
                pair[..40].copy_from_slice(target);
                pair[40..].copy_from_slice(event);
                // Base 1 avoids the predicate's null-address guard.
                segment_previous(
                    &mut Memory {
                        base: 1,
                        bytes: &mut pair,
                    },
                    1,
                    41,
                ) as i32
            } else {
                crate::text_input::key_greater(Some(target), Some(event)) as i32
            }
        },
    );
    m.put_half(it + 16, index);
    rc
}
fn segment_current(m: &Memory<'_>, it: u32) -> (u32, u16) {
    if it == 0 {
        return (0x81102007, 0);
    }
    let arr = m.word(it + 12);
    let mut out = 0;
    let rc = normalization::current_index(
        Some(&m.bytes[arr as usize..]),
        half(m, it + 16),
        Some(&mut out),
    );
    (rc, out)
}
fn segment_break(
    m: &Memory<'_>,
    it: u32,
    mut index: u16,
    width: u16,
    end: u16,
    pos: &mut u16,
) -> bool {
    if it == 0 {
        return false;
    }
    let arr = m.word(it + 12);
    let mut count = 0;
    if normalization::inner_field(Some(&m.bytes[arr as usize..]), Some(&mut count)) >= 0x80000000 {
        return false;
    }
    let (rc, table) = iterator_get(m, it, 0);
    if rc >= 0x80000000 {
        return false;
    }
    while index < count
        && m.word(table + u32::from(index) * 40 + 20)
            <= u32::from(*pos) * u32::from(width) + m.word(table + 20)
    {
        index += 1;
    }
    while index < count
        && m.word(table + u32::from(index) * 40 + 20)
            < u32::from(end) * u32::from(width) + m.word(table + 20)
        && half(m, table + u32::from(index) * 40 + 10) != 40
    {
        index += 1;
    }
    if index >= count {
        return false;
    }
    let event = table + u32::from(index) * 40;
    if m.word(event + 20) >= u32::from(end) * u32::from(width) + m.word(table + 20)
        || half(m, event + 10) != 40
    {
        return false;
    }
    if width != 0 {
        *pos = frontend::divmod_u32(
            m.word(event + 20).wrapping_sub(m.word(table + 20)) as u16 as u32,
            u32::from(width),
        )
        .0 as u16;
    }
    true
}
pub(crate) fn segment_send(
    m: &mut Memory<'_>,
    ctx: u32,
    timeline: u32,
    base: u32,
    start: u32,
    end: u32,
    it: u32,
    host: &mut (impl ResourceHost + SegmentCallbackHost),
) -> u32 {
    if ctx == 0 || timeline == 0 || it == 0 {
        return 0x81102007;
    }
    let offset = start * u32::from(half(m, timeline + 2)) + base;
    let mut rc = iterator_seek(m, it, -1);
    if rc < 0x80000000 {
        let mut target = [0u8; 40];
        target[20..24].copy_from_slice(&offset.to_le_bytes());
        rc = segment_back(m, it, &target, false, host);
        if rc >= 0x80000000 && rc & 0x1fff == 20 {
            rc = iterator_seek(m, it, 0);
        }
    }
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, index) = segment_current(m, it);
    if rc >= 0x80000000 {
        return rc;
    }
    // Render part c owns fn_2be984. Keep the unported boundary explicit.
    let mut rc = host.resource_call(
        m,
        0x2be984,
        &[ctx, 0, 1, timeline, base, start, end, it, u32::from(index)],
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut rel = start as u16;
    loop {
        if segment_break(m, it, index, half(m, timeline + 2), end as u16, &mut rel) {
            rc = host.resource_call(
                m,
                0x2be984,
                &[
                    ctx,
                    1,
                    0,
                    timeline,
                    base,
                    start,
                    u32::from(rel),
                    it,
                    u32::from(index),
                ],
            );
            if rc >= 0x80000000 {
                return rc;
            }
        } else {
            return host.resource_call(
                m,
                0x2be984,
                &[ctx, 1, 0, timeline, base, start, end, it, u32::from(index)],
            );
        }
    }
}
fn segment_select(
    m: &mut Memory<'_>,
    timeline: u32,
    it: u32,
    pos: u32,
    consumed: u32,
    maxlen: u16,
    ac: &mut u32,
    scratch: u32,
    separators: &[u8],
) -> (u32, bool) {
    if timeline == 0 || it == 0 {
        return (0x81102007, true);
    }
    let mut list = 0;
    let rc = normalization::field_value(Some(&m.bytes[it as usize..]), Some(&mut list));
    if rc >= 0x80000000 {
        return (rc, true);
    }
    let events = m.word(list + 4);
    let count = half(m, list + 8);
    let i = frontend::divmod_u32(consumed, u32::from(half(m, timeline + 2))).0 as u16;
    m.put_half(scratch, i);
    normalization::skip_matched(
        m.bytes,
        m.word(timeline + 4),
        scratch,
        u32::from(half(m, timeline + 8)),
        separators,
    );
    let cnt = u32::from(half(m, timeline + 2)) * u32::from(half(m, scratch));
    let cnt = cnt as u16 as u32;
    let mut i = 0u16;
    if cnt != 0 {
        while i < count
            && m.word(events + u32::from(i) * 40 + 20) <= m.word(events + 20).wrapping_add(cnt)
        {
            i += 1;
        }
    }
    while i < count {
        let e = events + u32::from(i) * 40;
        let tag = half(m, e + 10);
        if m.word(e + 20) != m.word(events + 20)
            && ((matches!(tag, 2 | 3) && m.word(e + 28) == 1) || matches!(tag, 12 | 20))
        {
            break;
        }
        i += 1;
    }
    if i < count {
        let end = m.word(events + u32::from(i) * 40 + 20);
        if end.wrapping_sub(pos) as u16 <= maxlen {
            *ac = end;
            (0, true)
        } else {
            (0, false)
        }
    } else {
        let total = u32::from(half(m, timeline + 8)) * u32::from(half(m, timeline + 2));
        if total.wrapping_sub(cnt) <= u32::from(maxlen) {
            *ac = pos.wrapping_add(total);
            (0, true)
        } else {
            (0, false)
        }
    }
}
pub(crate) fn segment_split<H, L, O, B, E, C>(
    m: &mut Memory<'_>,
    ctx: u32,
    timeline: u32,
    base: u32,
    end: u32,
    it: u32,
    index: u16,
    stack: u32,
    seg: &SegmentEnvironment<'_>,
    rt: &mut ParserRuntime<'_, '_, H, L, O, B, E, C>,
) -> u32
where
    H: ResourceHost + SegmentCallbackHost + crate::render_part_b::ParameterHost,
    L: frontend::StateLock,
    O: frontend::TreeOps,
    B: frontend::BeforeChange,
    E: frontend::ElementCallback,
    C: FnMut(u32, u32) -> i32,
{
    if ctx == 0 || timeline == 0 || it == 0 {
        return 0x81102007;
    }
    let (rc, owner) = resolve(m, [m.word(ctx), m.word(ctx + 4)], &rt.env.engine);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    let mut maxlen = 750u16;
    let props = m.word(owner + 20);
    if props != 0 {
        let rc = frontend::get_param_int(
            seg.number_table,
            m.bytes,
            nz(props),
            seg.max_length_key,
            stack + 96,
            rt.lock,
            &mut *rt.compare,
        );
        if rc < 0x80000000 {
            let v = (m.word(stack + 96) as i32).wrapping_mul(2);
            maxlen = if v < 2 { 2 } else { v.min(10000) as u16 };
            if maxlen & 1 != 0 {
                maxlen = (maxlen + 1).min(10000);
            }
        }
    }
    let text = m.word(timeline + 4);
    let mut rc = iterator_seek(m, it, i32::from(index));
    if rc >= 0x80000000 {
        return rc;
    }
    let total = end.wrapping_mul(u32::from(half(m, timeline + 2))) as u16 as u32;
    if total == 0 {
        return rc;
    }
    let mut pos = base;
    let bound = pos.wrapping_add(total);
    let mut consumed = 0;
    let mut ac = 0;
    let mut saved_source = 0;
    let mut carry_index = 0;
    let mut carry = false;
    loop {
        let (r, fits) = segment_select(
            m,
            timeline,
            it,
            pos,
            consumed,
            maxlen,
            &mut ac,
            stack + 100,
            seg.separators,
        );
        rc = r;
        if rc >= 0x80000000 {
            return rc;
        }
        if !fits {
            let entry = stack;
            m.bytes[entry as usize..entry as usize + 40].copy_from_slice(&seg.split);
            ac = pos.wrapping_add(u32::from(maxlen));
            m.put_word(stack + 104, ac);
            crate::render_part_d::trim_end(
                m.bytes,
                m.word(ctx + 76),
                text,
                consumed,
                pos,
                stack + 104,
                stack + 108,
                seg.separators,
            );
            ac = m.word(stack + 104).min(bound);
            m.put_word(entry + 20, ac);
            m.put_word(entry + 24, 0);
            let mut target = [0u8; 40];
            target.copy_from_slice(&m.bytes[entry as usize..entry as usize + 40]);
            rc = segment_back(m, it, &target, true, rt.host);
            if rc >= 0x80000000 && rc & 0x1fff != 20 {
                return rc;
            }
            let list = m.word(it + 12);
            let events = m.word(list + 4);
            let count = half(m, list + 8);
            let mut found = false;
            let mut quote = 0;
            for i in 0..count {
                let e = events + u32::from(i) * 40;
                if m.word(e + 20) > m.word(entry + 20) {
                    break;
                }
                if half(m, e + 10) == 7 {
                    if m.word(e + 28) == 43 {
                        found = !found;
                        quote = i;
                    } else {
                        found = false;
                    }
                }
            }
            if found {
                let e = events + u32::from(quote) * 40;
                ac = m.word(e + 20);
                m.put_word(entry + 20, ac);
                rc = iterator_seek(m, it, i32::from(quote));
                if rc >= 0x80000000 {
                    return rc;
                }
                carry_index = quote;
                carry = true;
                saved_source = m.word(e + 12);
                let mut closing = None;
                for j in quote + 1..count {
                    let e = events + u32::from(j) * 40;
                    if half(m, e + 10) == 7 && m.word(e + 28) == 43 {
                        closing = Some(j);
                        break;
                    }
                }
                ac = closing
                    .map(|j| m.word(events + u32::from(j) * 40 + 20))
                    .unwrap_or(total);
            }
            rc = crate::text_input::insert_span(
                m.bytes,
                it,
                entry,
                u32::from(carry),
                &mut FetchHost {
                    host: rt.host,
                    env: &rt.env.engine,
                },
            );
            if rc >= 0x80000000 {
                return rc;
            }
            if carry {
                let list = m.word(it + 12);
                m.put_word(
                    m.word(list + 4) + u32::from(carry_index) * 40 + 12,
                    saved_source,
                );
            }
        }
        let delta = ac.wrapping_sub(pos);
        if delta.wrapping_add(consumed) >= 0xffff {
            break;
        }
        consumed = consumed.wrapping_add(if ac == pos { 1 } else { delta as u16 as u32 });
        pos = ac;
        if consumed >= total {
            break;
        }
    }
    rc
}
/// fn_2c0014. Select parser-mode windows, emit segments and add chunk splits.
/// stack is 160 disjoint caller-owned bytes, restored on return.
pub fn fn_2c0014<H, L, O, B, E, C>(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    flag3: i32,
    flag4arg: u32,
    stack: u32,
    seg: &SegmentEnvironment<'_>,
    rt: &mut ParserRuntime<'_, '_, H, L, O, B, E, C>,
) -> u32
where
    H: ResourceHost + SegmentCallbackHost + crate::render_part_b::ParameterHost,
    L: frontend::StateLock,
    O: frontend::TreeOps,
    B: frontend::BeforeChange,
    E: frontend::ElementCallback,
    C: FnMut(u32, u32) -> i32,
{
    if ctx == 0 || handle == 0 {
        return 0x81102007;
    }
    if half(m, ctx + 48) == 0 {
        return 0x81102007;
    }
    let (rc, timeline) = fetch_parser(m, handle, 0x3f2, 1, rt.env, rt.host);
    if rc >= 0x80000000 {
        return if rc & 0x1fff == 20 { 0 } else { rc };
    }
    let text_bytes = (u32::from(half(m, timeline + 8)) * u32::from(half(m, timeline + 2))) as u16;
    let pair = [m.word(ctx), m.word(ctx + 4)];
    let mut it = 0;
    let rc = normalization::open_iterator(
        m.bytes,
        pair[0],
        pair[1],
        handle,
        0x3f3,
        Some(&mut it),
        &mut FetchHost {
            host: rt.host,
            env: &rt.env.engine,
        },
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut list = 0;
    let rc = normalization::field_value(Some(&m.bytes[it as usize..]), Some(&mut list));
    if rc >= 0x80000000 {
        return rc;
    }
    let mut base = m.word(m.word(list + 4) + 20);
    let limit = base.wrapping_add(u32::from(text_bytes));
    let mut saved = [0u8; 160];
    saved.copy_from_slice(&m.bytes[stack as usize..stack as usize + 160]);
    let mut d4 = 0;
    let mut bc = 0;
    let mut index = 0;
    let mut early = false;
    let rc = (|| {
        let rc = iterator_seek(m, it, 0);
        if rc >= 0x80000000 {
            return rc;
        }
        let arr = m.word(it + 12);
        let data = m.word(arr + 4);
        let mut idx = half(m, it + 16);
        let rc = normalization::scan_forward(
            &m.bytes[data as usize..],
            half(m, arr + 2),
            half(m, arr + 8),
            &mut idx,
            &mut |event| crate::text_input::keys_equal(Some(event), Some(&seg.first)) as i32,
        );
        m.put_half(it + 16, idx);
        if rc >= 0x80000000 {
            return rc;
        }
        let (rc, first) = iterator_get(m, it, 0xffff);
        if rc >= 0x80000000 {
            return rc;
        }
        let field = m.word(first + 20);
        let pos = m.word(ctx + 84).max(base);
        m.put_word(ctx + 84, pos);
        m.put_word(ctx + 92, pos);
        d4 = frontend::divmod_u32(pos.wrapping_sub(base), u32::from(half(m, timeline + 2))).0;
        bc = d4;
        let mut rc =
            crate::text_input::timeline_window(m.bytes, timeline, it, &mut bc, &mut d4, &mut index);
        if rc >= 0x80000000 {
            return rc;
        }
        if d4 != 0 && d4 <= u32::from(half(m, timeline + 8)) {
            let mut c = d4 as u16;
            if half(m, timeline + 8) == c {
                c = c.wrapping_sub(1);
            }
            m.put_half(stack + 112, c);
            normalization::skip_matched_back(
                m.bytes,
                m.word(timeline + 4),
                stack + 112,
                seg.separators,
            );
            if half(m, stack + 112) == 0 {
                early = true;
                m.put_word(
                    ctx + 84,
                    d4.wrapping_mul(u32::from(half(m, timeline + 2)))
                        .wrapping_add(base),
                );
                return rc;
            }
        }
        if d4 <= bc {
            return rc;
        }
        let mut b4 = bc as u16;
        let mut b8 = d4 as u16;
        let mut sent = false;
        loop {
            if b4 != b8 {
                let mut mode = flag4arg as u16;
                rc = segment_scan(m, ctx, list, &mut b4, &mut b8, &mut mode);
                if rc >= 0x80000000 {
                    return rc;
                }
                if b4 != b8 && mode == 1 && flag3 != 0 {
                    sent = true;
                    rc = segment_send(
                        m,
                        ctx,
                        timeline,
                        field,
                        u32::from(b4),
                        u32::from(b8),
                        it,
                        rt.host,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    b4 = b8;
                    b8 = d4 as u16;
                    if b4 != b8 {
                        continue;
                    }
                }
            }
            b4 = bc as u16;
            b8 = d4 as u16;
            rc = crate::text_input::timeline_window(
                m.bytes, timeline, it, &mut bc, &mut d4, &mut index,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            if u32::from(b4) >= bc || u32::from(b8) >= d4 {
                bc = d4;
            }
            if d4
                .wrapping_mul(u32::from(half(m, timeline + 2)))
                .wrapping_add(base)
                > limit
            {
                bc = d4;
            }
            if d4 > bc {
                b4 = bc as u16;
                b8 = d4 as u16;
                continue;
            }
            if sent {
                rc = segment_split(m, ctx, timeline, field, d4, it, index, stack, seg, rt);
            }
            return rc;
        }
    })();
    if !early {
        let mut c = d4 as u16;
        if c != 0 {
            if half(m, timeline + 8) == c {
                c = c.wrapping_sub(1);
            }
            if c <= half(m, timeline + 8) {
                m.put_half(stack + 112, c);
                normalization::skip_matched_back(
                    m.bytes,
                    m.word(timeline + 4),
                    stack + 112,
                    seg.separators,
                );
                c = half(m, stack + 112);
            }
        }
        if u32::from(c) != d4 {
            m.put_word(
                ctx + 88,
                u32::from(c)
                    .wrapping_mul(u32::from(half(m, timeline + 2)))
                    .wrapping_add(base),
            );
        }
        if d4 > 4 {
            base = base.wrapping_sub(4);
        }
        m.put_word(
            ctx + 84,
            d4.wrapping_mul(u32::from(half(m, timeline + 2)))
                .wrapping_add(base),
        );
    }
    if it != 0 {
        normalization::release_iterator(
            m.bytes,
            it,
            &mut FetchHost {
                host: rt.host,
                env: &rt.env.engine,
            },
        );
    }
    m.bytes[stack as usize..stack as usize + 160].copy_from_slice(&saved);
    rc
}

/// Memory-passing allocator for sentence splitting. The allocator must not
/// capture the arena. All callbacks receive the caller's current bytes last.
pub trait ParserAllocatorM: frontend::HeapVtableM {
    fn calloc_m(&mut self, object: u32, count: u32, size: u32, memory: &mut [u8]) -> u32;
}
struct FetchHostM<'a, A> {
    allocator: &'a mut A,
    env: &'a render_part_c::Environment,
}
impl<A: ParserAllocatorM> normalization::NormalizationHost for FetchHostM<'_, A> {
    fn resolve(&mut self, bytes: &mut [u8], a: u32, b: u32) -> (u32, u32) {
        resolve(&Memory { base: 0, bytes }, [a, b], self.env)
    }
    fn calloc(&mut self, bytes: &mut [u8], heap: u32, n: u32, size: u32) -> Option<u32> {
        if heap == 0 || n == 0 || size == 0 {
            return None;
        }
        let object = Memory { base: 0, bytes }.word(heap + 4);
        nz(self.allocator.calloc_m(object, n, size, bytes))
    }
    fn alloc(&mut self, bytes: &mut [u8], heap: u32, size: u32) -> Option<u32> {
        nz(frontend::heap_alloc_m(
            bytes,
            nz(heap),
            size,
            self.allocator,
        ))
    }
    fn realloc(&mut self, bytes: &mut [u8], heap: u32, p: u32, size: u32) -> Option<u32> {
        nz(frontend::heap_realloc_m(
            bytes,
            nz(heap),
            p,
            size,
            self.allocator,
        ))
    }
    fn free(&mut self, bytes: &mut [u8], heap: u32, p: u32) {
        frontend::heap_release_m(bytes, nz(heap), nz(p), self.allocator)
    }
}
fn fetch_parser_m(
    m: &mut Memory<'_>,
    handle: u32,
    kind: u32,
    count: u32,
    env: &Environment<'_>,
    allocator: &mut impl ParserAllocatorM,
) -> (u32, u32) {
    assert_eq!(m.base, 0);
    let mut out = 0;
    let rc = normalization::fetch_entry(
        m.bytes,
        handle,
        kind,
        count,
        Some(&mut out),
        &mut FetchHostM {
            allocator,
            env: &env.engine,
        },
    );
    (rc, out)
}

fn store_sized_m(
    mem: &mut [u8],
    slot_at: Option<u32>,
    src_off: u32,
    n: u32,
    heap: u32,
    vt: &mut impl frontend::HeapVtableM,
) -> u32 {
    let Some(slot_at) = slot_at else {
        return 0x8b702007;
    };
    if n == 0 {
        if parser_word(mem, slot_at) != 0 && parser_word(mem, slot_at + 4) != 0 {
            frontend::heap_release_m(mem, nz(heap), nz(parser_word(mem, slot_at)), vt);
        }
        parser_store(mem, slot_at + 4, 0);
        parser_store(mem, slot_at, src_off);
    } else {
        let size = parser_word(mem, slot_at + 4);
        if size == 0 {
            let fresh = frontend::heap_alloc_m(mem, nz(heap), n, vt);
            parser_store(mem, slot_at, fresh);
            if fresh == 0 {
                return 0x8b70200a;
            }
        } else if size < n {
            let grown = frontend::heap_realloc_m(mem, nz(heap), parser_word(mem, slot_at), n, vt);
            if grown == 0 {
                return 0x8b70200a;
            }
            parser_store(mem, slot_at, grown);
        }
        let data = parser_word(mem, slot_at);
        let mut i = 0u32;
        while i < n {
            let b = mem[src_off.wrapping_add(i) as usize];
            mem[data.wrapping_add(i) as usize] = b;
            i = i.wrapping_add(1);
        }
        parser_store(mem, slot_at + 4, n);
    }
    0
}

fn update_attr_m(
    mem: &mut [u8],
    obj: u32,
    attr: u32,
    data: u32,
    size: u32,
    upd: u32,
    scratch: u32,
    heap_vt: &mut impl frontend::HeapVtableM,
    ops: &mut impl frontend::TreeOpsM,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    parser_store(mem, scratch, data);
    parser_store(mem, scratch + 4, size);
    parser_store(mem, scratch + 8, 0);
    let dict = parser_word(mem, obj + 16);
    let mut r = 0u32;
    let mut i = 0u32;
    loop {
        if dict == 0 {
            break;
        }
        if i >= parser_word(mem, dict + 20) {
            break;
        }
        let ent =
            parser_word(mem, dict).wrapping_add(i.wrapping_mul(parser_half(mem, dict + 4) as u32));
        r = frontend::insert_guarded_m(
            mem,
            nz(parser_word(mem, ent + 4)),
            attr,
            scratch,
            heap_vt,
            ops,
        );
        if r & 0x1fff == 2 {
            if upd == 0 {
                r = 0;
            } else {
                let mut slot = 0u32;
                let ent1 = parser_word(mem, ent + 4);
                r = frontend::lookup_value(mem, nz(ent1), attr, Some(&mut slot), &mut cmp);
                if r >= 0x80000000 {
                    r = (r & 0x1fff) | 0x8b702000;
                } else {
                    let pi = slot;
                    let alloc = parser_word(mem, parser_word(mem, ent));
                    if pi == 0 {
                        r = 0x8b702007;
                        break;
                    }
                    if size == 0 {
                        if parser_word(mem, pi) != 0 && parser_word(mem, pi + 4) != 0 {
                            frontend::heap_release_m(
                                mem,
                                nz(alloc),
                                nz(parser_word(mem, pi)),
                                heap_vt,
                            );
                        }
                        parser_store(mem, pi, data);
                        parser_store(mem, pi + 4, 0);
                        r = 0;
                    } else if parser_word(mem, pi + 4) == 0 {
                        let fresh = frontend::heap_alloc_m(mem, nz(alloc), size, heap_vt);
                        parser_store(mem, pi, fresh);
                        if fresh == 0 {
                            r = 0x8b70200a;
                            break;
                        }
                        let mut k = 0u32;
                        while k < size {
                            let b = mem[data.wrapping_add(k) as usize];
                            mem[parser_word(mem, pi).wrapping_add(k) as usize] = b;
                            k = k.wrapping_add(1);
                        }
                        parser_store(mem, pi + 4, size);
                        r = 0;
                    } else if parser_word(mem, pi + 4) < size {
                        let m = frontend::heap_realloc_m(
                            mem,
                            nz(alloc),
                            parser_word(mem, pi),
                            size,
                            heap_vt,
                        );
                        if m == 0 {
                            r = 0x8b70200a;
                            break;
                        }
                        parser_store(mem, pi, m);
                        let mut k = 0u32;
                        while k < size {
                            let b = mem[data.wrapping_add(k) as usize];
                            mem[parser_word(mem, pi).wrapping_add(k) as usize] = b;
                            k = k.wrapping_add(1);
                        }
                        parser_store(mem, pi + 4, size);
                        r = 0;
                    } else {
                        let mut k = 0u32;
                        while k < size {
                            let b = mem[data.wrapping_add(k) as usize];
                            mem[parser_word(mem, pi).wrapping_add(k) as usize] = b;
                            k = k.wrapping_add(1);
                        }
                        parser_store(mem, pi + 4, size);
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

fn set_param_m(
    mem: &mut [u8],
    params_at: Option<u32>,
    key: u32,
    data: u32,
    size: u32,
    update: u32,
    scratch: u32,
    lock: &mut impl frontend::StateLock,
    heap_vt: &mut impl frontend::HeapVtableM,
    ops: &mut impl frontend::TreeOpsM,
    mut cmp: impl FnMut(u32, u32) -> i32,
    before: impl frontend::BeforeChange,
    elem_cb: impl frontend::ElementCallback,
) -> u32 {
    const MISSING: u32 = 0x8b702007;
    let Some(params_at) = params_at else {
        return MISSING;
    };
    if key == 0 || data == 0 {
        return MISSING;
    }
    let heap = parser_word(mem, params_at);
    let values = parser_word(mem, params_at + 8);
    let callbacks = parser_word(mem, params_at + 12);
    let mut result = lock.acquire();
    if result >= 0x80000000 {
        return result;
    }
    let mut before = before;
    let mut elem_cb = elem_cb;
    let mut value = 0u32;
    let rc = frontend::lookup_value(mem, nz(values), key, Some(&mut value), &mut cmp);
    if rc < 0x80000000 {
        let (old_data, old_size) = if value == 0 {
            (0, 0)
        } else {
            let n = parser_word(mem, value + 4);
            if n == 0 {
                (0, 0)
            } else {
                (parser_word(mem, value), n)
            }
        };
        let mut need = old_size != size;
        if !need {
            let end = old_data.wrapping_add(size) as usize;
            let dend = data.wrapping_add(size) as usize;
            need = frontend::compare_bytes(
                &mem[old_data as usize..end],
                &mem[data as usize..dend],
                size,
            ) != 0;
        }
        if need {
            result = frontend::before_change(mem, callbacks, value, key, data, &mut before);
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
                    result = store_sized_m(mem, nz(value), data, size, heap, &mut *heap_vt);
                    if result < 0x80000000 {
                        result = frontend::run_element_callbacks(
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
        parser_store(mem, scratch, data);
        parser_store(mem, scratch + 4, size);
        parser_store(mem, scratch + 8, 0);
        result = frontend::before_change(mem, callbacks, scratch, key, data, &mut before);
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
                result = frontend::insert_guarded_m(
                    mem,
                    nz(values),
                    key,
                    scratch,
                    &mut *heap_vt,
                    &mut *ops,
                );
                if result >= 0x80000000 {
                    result = (result & 0x1fff) | 0x8b702000;
                } else {
                    result = frontend::run_element_callbacks(
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
    let rc = lock.release();
    if rc >= 0x80000000 && result < 0x80000000 {
        result = rc;
    }
    result
}

fn set_param_str_m(
    mem: &mut [u8],
    params_at: Option<u32>,
    key: u32,
    s: u32,
    empty: u32,
    scratch: u32,
    lock: &mut impl frontend::StateLock,
    heap_vt: &mut impl frontend::HeapVtableM,
    ops: &mut impl frontend::TreeOpsM,
    cmp: impl FnMut(u32, u32) -> i32,
    before: impl frontend::BeforeChange,
    elem_cb: impl frontend::ElementCallback,
) -> u32 {
    let s = if s == 0 { empty } else { s };
    let n = frontend::strlen_bytes(&mem[s as usize..]).wrapping_add(1);
    set_param_m(
        mem, params_at, key, s, n, 0, scratch, lock, heap_vt, ops, cmp, before, elem_cb,
    )
}

fn parser_word(mem: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(mem[p as usize..p as usize + 4].try_into().unwrap())
}
fn parser_store(mem: &mut [u8], p: u32, v: u32) {
    mem[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn parser_half(mem: &[u8], p: u32) -> u16 {
    u16::from_le_bytes(mem[p as usize..p as usize + 2].try_into().unwrap())
}

pub fn fn_2c0e30_m(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    marker: u32,
    name: u32,
    scratch: u32,
    env: &Environment<'_>,
    host: &mut impl ResourceHost,
    lock: &mut impl frontend::StateLock,
    ops: &mut impl frontend::TreeOpsM,
    before: impl frontend::BeforeChange,
    callback: impl frontend::ElementCallback,
    cmp: impl FnMut(u32, u32) -> i32,
    allocator: &mut impl ParserAllocatorM,
) -> u32 {
    assert_eq!(m.base, 0);
    let (rc, engine) = resolve(m, [m.word(ctx), m.word(ctx + 4)], &env.engine);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    let mut array = 0;
    let rc = normalization::fetch_entry(
        m.bytes,
        handle,
        0x3f4,
        1,
        Some(&mut array),
        &mut FetchHostM {
            allocator,
            env: &env.engine,
        },
    );
    let text = if rc < 0x80000000 {
        m.word(array + 4)
    } else {
        0
    };
    let src = text.wrapping_add(m.word(marker + 28).wrapping_mul(2));
    let length = frontend::u16_len(m.bytes, src) as u16;
    let heap = m.word(engine + 4);
    let value = if heap == 0 {
        0
    } else {
        allocator.calloc_m(m.word(heap + 4), 1, u32::from(length) + 1, m.bytes)
    };
    if value == 0 {
        return 0x8110200a;
    }
    crate::fe_text::narrow_n_17649c(m.bytes, value, src, u32::from(length));
    m.bytes[(value + u32::from(length)) as usize] = 0;
    let at = m.offset(scratch);
    let mut saved = [0u8; 24];
    saved.copy_from_slice(&m.bytes[at..at + 24]);
    let props = m.word(engine + 20);
    let result = set_param_str_m(
        m.bytes,
        nz(props),
        name,
        value,
        0,
        scratch,
        lock,
        allocator,
        ops,
        cmp,
        before,
        callback,
    );
    m.bytes[at..at + 24].copy_from_slice(&saved);
    if result < 0x80000000 {
        // fn_1738e8 has no direct Rust port. Null log is a real no-op.
        if m.word(engine + 16) != 0 {
            host.resource_call(
                m,
                0x1738e8,
                &[
                    m.word(engine + 16),
                    u32::from(half(m, marker + 10)),
                    name,
                    value,
                ],
            );
        }
    }
    frontend::heap_release_m(m.bytes, nz(heap), nz(value), allocator);
    result
}

pub fn fn_2c2564_m(
    m: &mut Memory<'_>,
    _ctx: u32,
    handle: u32,
    pos_ref: u32,
    removed: u32,
    esc_offset: u32,
    ref_reset: u32,
    reset: u32,
    env: &Environment<'_>,
    parser: &ParserEnvironment<'_>,
    allocator: &mut impl ParserAllocatorM,
) -> u32 {
    assert_eq!(m.base, 0);
    m.put_half(removed, 0);
    m.put_half(esc_offset, 0);
    m.put_word(ref_reset, 0);
    m.put_half(reset + 10, 0);
    let (rc, text) = fetch_parser_m(m, handle, 0x3fc, 1, env, allocator);
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, mut markers) = fetch_parser_m(m, handle, 0x3fd, 1, env, allocator);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut blanks = 0;
    let mut index = 0;
    let rc = normalization::scan_table(
        m.bytes,
        text,
        Some(&mut blanks),
        markers,
        Some(&mut index),
        0,
        parser.separators,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, p) = fetch_parser_m(m, handle, 0x3fd, 1, env, allocator);
    if rc >= 0x80000000 {
        return rc;
    }
    markers = p;
    let mut base = m.word(markers + 4);
    let base_ref = m.word(base + 20);
    let mut add_blanks = true;
    let mut i = 0u16;
    while i < half(m, markers + 8) {
        let current = base + u32::from(i) * 40;
        let tag = half(m, current + 10);
        if parameter_tag(tag) && m.bytes[current as usize] == 0 {
            let n = u32::from(blanks);
            let first = m.word(base + 20);
            let width = u32::from(half(m, text + 2));
            let limit = n.wrapping_mul(width).wrapping_add(first);
            if limit > m.word(current + 20) {
                let mut last = i;
                let mut j = i.wrapping_add(1);
                while j < half(m, markers + 8) {
                    let next = base + u32::from(j) * 40;
                    if parameter_tag(half(m, next + 10))
                        && m.bytes[next as usize] == 0
                        && half(m, next + 10) == tag
                        && limit > m.word(next + 20)
                    {
                        last = j;
                    }
                    j = j.wrapping_add(1);
                }
                let last_marker = base + u32::from(last) * 40;
                if m.word(last_marker + 20) >= first
                    && frontend::divmod_u32(m.word(last_marker + 20).wrapping_sub(first), width).0
                        < n
                {
                    index = last;
                    crate::fe_text::memcpy_195ae8(m.bytes, reset, last_marker, 40);
                    let delta = m.word(last_marker + 12).wrapping_sub(m.word(base + 12));
                    m.put_word(ref_reset, delta);
                    m.put_word(base + 16, m.word(base + 16).wrapping_sub(delta));
                    m.put_word(base + 12, m.word(last_marker + 12));
                    let width = u32::from(half(m, text + 2));
                    let value = n
                        .wrapping_sub(
                            frontend::divmod_u32(
                                m.word(last_marker + 20).wrapping_sub(m.word(base + 20)),
                                width,
                            )
                            .0,
                        )
                        .wrapping_mul(width)
                        .wrapping_add(m.word(last_marker + 16));
                    m.put_word(pos_ref, value);
                    let mut j = last.wrapping_add(1);
                    while j < half(m, markers + 8) {
                        let next = base + u32::from(j) * 40;
                        if (half(m, next + 10) == 999 || m.bytes[next as usize] == 0)
                            && n.wrapping_mul(u32::from(half(m, text + 2)))
                                .wrapping_add(base_ref)
                                >= m.word(next + 20)
                            && m.word(last_marker + 12) <= m.word(next + 12)
                        {
                            m.put_word(pos_ref, m.word(pos_ref).wrapping_add(m.word(next + 16)));
                        }
                        j = j.wrapping_add(1);
                    }
                    add_blanks = false;
                    // last is promoted to signed int in this C expression.
                    if i32::from(last) - 1 > 0 {
                        for j in 1..last {
                            let p = base + u32::from(j) * 40;
                            if half(m, p + 10) == 999 {
                                m.put_half(p + 10, 0x4000);
                            }
                        }
                    }
                } else if m.word(current + 12).wrapping_sub(m.word(base + 12))
                    == width.wrapping_mul(n)
                {
                    add_blanks = false;
                    let delta = u32::from(half(m, text + 2)).wrapping_mul(n);
                    m.put_word(base + 16, m.word(base + 16).wrapping_sub(delta));
                    m.put_word(base + 12, m.word(base + 12).wrapping_add(delta));
                    index = i;
                }
                break;
            }
        }
        i = i.wrapping_add(1);
    }
    if blanks == 0 && half(m, reset + 10) == 0 {
        return 0;
    }
    m.put_half(text + 8, half(m, text + 8).wrapping_sub(blanks));
    let data = m.word(text + 4);
    copy_bytes(
        m,
        data,
        data + u32::from(blanks) * 2,
        u32::from(half(m, text + 8)) * u32::from(half(m, text + 2)),
    );
    let (rc, p) = fetch_parser_m(
        m,
        handle,
        0x3fd,
        u32::from(half(m, markers + 8)) + 1,
        env,
        allocator,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    markers = p;
    base = m.word(markers + 4);
    let mut saved = [0u8; 40];
    if index == 1 {
        saved.copy_from_slice(&m.bytes[base as usize..base as usize + 40]);
        m.put_word(base + 16, 0);
        m.put_word(base + 24, 0);
    }
    let prev = base.wrapping_add(u32::from(index.wrapping_sub(1)) * 40);
    let delta = m
        .word(prev + 12)
        .wrapping_add(m.word(prev + 16))
        .wrapping_sub(m.word(prev + 24).wrapping_add(m.word(prev + 20)));
    let mut inserted = parser.inserted_image;
    inserted[0] = 0;
    inserted[4..8].copy_from_slice(&0u32.to_le_bytes());
    inserted[8..10].copy_from_slice(&0u16.to_le_bytes());
    inserted[10..12].copy_from_slice(&0x4000u16.to_le_bytes());
    let len = u32::from(blanks)
        .wrapping_mul(u32::from(half(m, text + 2)))
        .wrapping_add(m.word(base + 20))
        .wrapping_add(delta)
        .wrapping_sub(m.word(base + 12));
    inserted[12..16].copy_from_slice(&m.word(base + 12).to_le_bytes());
    inserted[16..20].copy_from_slice(&len.to_le_bytes());
    inserted[20..24].copy_from_slice(&m.word(base + 20).to_le_bytes());
    inserted[24..28].fill(0);
    inserted[32..36].fill(0);
    inserted[36] = 0;
    if index == 1 {
        m.bytes[base as usize..base as usize + 40].copy_from_slice(&saved);
    }
    index = 1;
    while index < half(m, markers + 8) {
        let p = base + u32::from(index) * 40;
        let ins_word = |i: usize| u32::from_le_bytes(inserted[i..i + 4].try_into().unwrap());
        if ins_word(20) != m.word(p + 20)
            || m.word(p + 24) != 0
            || ins_word(12) != m.word(p + 12)
            || ins_word(16) <= m.word(p + 16)
        {
            break;
        }
        index = index.wrapping_add(1);
    }
    let at = base + u32::from(index) * 40;
    copy_bytes(
        m,
        at + 40,
        at,
        u32::from(half(m, markers + 8).wrapping_sub(index)) * 40,
    );
    m.bytes[at as usize..at as usize + 40].copy_from_slice(&inserted);
    m.put_half(markers + 8, half(m, markers + 8).wrapping_add(1));
    let delta = u32::from(half(m, text + 2)) * u32::from(blanks);
    m.put_half(removed, delta as u16);
    if add_blanks {
        m.put_word(pos_ref, m.word(pos_ref).wrapping_add(delta));
    }
    if half(m, reset + 10) == 0 {
        for index in 1..half(m, markers + 8) {
            let p = base + u32::from(index) * 40;
            if m.word(p + 20) > m.word(base + 20)
                && m.word(p + 20) <= delta.wrapping_add(m.word(base + 20))
                && half(m, p + 10) != 0x4000
                && m.bytes[p as usize] == 0
            {
                m.put_word(pos_ref, m.word(pos_ref).wrapping_add(m.word(p + 16)));
                m.put_half(
                    esc_offset,
                    half(m, esc_offset).wrapping_add(m.word(p + 16) as u16),
                );
            }
        }
    }
    for index in 1..half(m, markers + 8) {
        let p = base + u32::from(index) * 40;
        let value = if m.word(p + 20) < delta.wrapping_add(m.word(base + 20)) {
            m.word(base + 20)
        } else {
            m.word(p + 20).wrapping_sub(delta)
        };
        m.put_word(p + 20, value);
    }
    m.put_word(base + 24, m.word(base + 24).wrapping_sub(delta));
    0
}

/// Reentrant fn_2c0f74. Legacy handlers remain separate from the live allocator.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c0f74_m<H, L, O, B, E, C>(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    length: u32,
    position: u32,
    voice_switch: u32,
    immediate: u32,
    mode: u32,
    pos_ref: u32,
    ref_reset: u32,
    reset: u32,
    rt: &mut ParserRuntime<'_, '_, H, L, O, B, E, C>,
    allocator: &mut impl ParserAllocatorM,
) -> u32
where
    H: ResourceHost + crate::render_part_b::ParameterHost,
    L: frontend::StateLock,
    O: frontend::TreeOpsM,
    B: frontend::BeforeChange,
    E: frontend::ElementCallback,
    C: FnMut(u32, u32) -> i32,
{
    if handle == 0 {
        return 0x81102007;
    }
    let (rc, engine) = resolve(m, [m.word(ctx), m.word(ctx + 4)], &rt.env.engine);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    m.put_word(pos_ref, 0);
    if half(m, ctx + 48) == 0 {
        return 0x81102007;
    }
    let (rc, text) = fetch_parser_m(m, handle, 0x3f2, 1, rt.env, allocator);
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, array) = fetch_parser_m(m, handle, 0x3f4, 1, rt.env, allocator);
    let params = if rc < 0x80000000 {
        m.word(array + 4)
    } else {
        0
    };
    let (rc, markers) = fetch_parser_m(m, handle, 0x3f3, 1, rt.env, allocator);
    if rc >= 0x80000000 {
        return rc;
    }
    let base = m.word(markers + 4);
    let mut te = 0;
    let mut me = 0;
    let rc = normalization::scan_table(
        m.bytes,
        text,
        Some(&mut te),
        markers,
        Some(&mut me),
        0,
        rt.parser.separators,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    for i in 1..half(m, markers + 8) {
        let p = base + u32::from(i) * 40;
        if m.word(p + 20) > u32::from(te) * 2 + m.word(base + 20) {
            break;
        }
        if half(m, p + 10) == 20 {
            m.put_half(mode, m.word(p + 28) as u16);
        }
    }
    if !parser_boundary(m, text, markers, &mut te, &mut me) {
        let s = rt.scratch;
        let saved = m.word(s);
        m.put_half(s, te);
        m.put_half(s + 2, me);
        let rc = match half(m, mode) {
            2 | 3 | 4 => {
                if half(m, mode) == 2 {
                    render_part_c::fn_2be178(m, engine, text, markers, s, s + 2)
                } else {
                    let data = rt
                        .parser
                        .boundary
                        .expect("parser modes 3/4 require BoundaryData");
                    let properties = m.word(engine + 20);
                    if properties != 0 {
                        rt.host.select_lock(m.word(properties + 4));
                    }
                    if half(m, mode) == 3 {
                        render_part_c::fn_2be1fc(
                            m,
                            engine,
                            text,
                            markers,
                            s,
                            s + 2,
                            data,
                            rt.host,
                            &mut *rt.compare,
                        )
                    } else {
                        render_part_c::fn_2be4a0(
                            m,
                            engine,
                            text,
                            markers,
                            s,
                            s + 2,
                            data,
                            rt.host,
                            &mut *rt.compare,
                        )
                    }
                }
            }
            _ => render_part_c::fn_2be754(m, engine, text, markers, s, s + 2, rt.parser.separators),
        };
        te = half(m, s);
        me = half(m, s + 2);
        m.put_word(s, saved);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let mut nonempty = 0u16;
    let mut switch_index = 0;
    let mut i = 1u16;
    while i < me {
        let p = base + u32::from(i) * 40;
        if half(m, p + 10) != 0x4000 {
            nonempty = nonempty.wrapping_add(1);
            if half(m, p + 10) == 22 {
                m.bytes[voice_switch as usize] = 1;
                me = i;
                switch_index = i;
                if nonempty == i && m.word(p + 20) == m.word(base + 20) {
                    m.bytes[immediate as usize] = 1;
                    let rc = fn_2c0e30_m(
                        m,
                        ctx,
                        handle,
                        p,
                        rt.parser.voice_key,
                        rt.scratch,
                        rt.env,
                        rt.host,
                        rt.lock,
                        rt.ops,
                        BorrowBefore(rt.before),
                        BorrowElement(rt.callback),
                        &mut *rt.compare,
                        allocator,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    me = i + 1;
                } else {
                    m.bytes[immediate as usize] = 0;
                }
                te = if me < half(m, markers + 8) {
                    frontend::divmod_u32(
                        m.word(p + 20).wrapping_sub(m.word(base + 20)) as u16 as u32,
                        u32::from(half(m, text + 2)),
                    )
                    .0 as u16
                } else {
                    0
                };
                break;
            }
        }
        nonempty = nonempty.wrapping_add(1);
        i = i.wrapping_add(1);
    }
    let (rc, out_text) = fetch_parser_m(
        m,
        handle,
        0x3fc,
        if te == 0 { 1 } else { u32::from(te) },
        rt.env,
        allocator,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, out_markers) = fetch_parser_m(m, handle, 0x3fd, u32::from(me) + 1, rt.env, allocator);
    if rc >= 0x80000000 {
        return rc;
    }
    copy_bytes(
        m,
        m.word(out_text + 4),
        m.word(text + 4),
        u32::from(te) * u32::from(half(m, text + 2)),
    );
    m.put_half(out_text + 8, te);
    m.put_half(text + 8, half(m, text + 8).wrapping_sub(te));
    copy_bytes(
        m,
        m.word(text + 4),
        m.word(text + 4) + u32::from(te) * 2,
        u32::from(half(m, text + 8)) * u32::from(half(m, text + 2)),
    );
    m.put_word(length, u32::from(half(m, out_text + 8)));
    if m.bytes[voice_switch as usize] == 1 && m.bytes[immediate as usize] == 0 {
        let mut blanks = 0;
        let mut blank_markers = 0;
        let rc = normalization::scan_table(
            m.bytes,
            out_text,
            Some(&mut blanks),
            out_markers,
            Some(&mut blank_markers),
            0,
            rt.parser.separators,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        if half(m, out_text + 8) == blanks {
            let found = (1..me)
                .any(|j| !matches!(half(m, base + u32::from(j) * 40 + 10), 2 | 7 | 31 | 36 | 21));
            if found {
                m.bytes[voice_switch as usize] = 0;
                me = i;
            } else {
                m.bytes[immediate as usize] = 1;
                let rc = fn_2c0e30_m(
                    m,
                    ctx,
                    handle,
                    base + u32::from(switch_index) * 40,
                    rt.parser.voice_key,
                    rt.scratch,
                    rt.env,
                    rt.host,
                    rt.lock,
                    rt.ops,
                    BorrowBefore(rt.before),
                    BorrowElement(rt.callback),
                    &mut *rt.compare,
                    allocator,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                me = i + 1;
            }
        }
    }
    copy_bytes(
        m,
        m.word(out_markers + 4),
        m.word(markers + 4),
        u32::from(me) * 40,
    );
    m.put_half(out_markers + 8, me);
    let end = u8::from(
        m.bytes[(text + 12) as usize] != 0
            && half(m, text + 8) == 0
            && m.bytes[(markers + 12) as usize] != 0
            && half(m, markers + 8) == me,
    );
    m.bytes[(out_text + 12) as usize] = end;
    m.bytes[(out_markers + 12) as usize] = end;
    let next = base + u32::from(me) * 40;
    let delta = u32::from(te) * u32::from(half(m, text + 2));
    let reset_position =
        me < half(m, markers + 8) && m.word(next + 20) <= delta.wrapping_add(m.word(base + 20));
    if reset_position {
        m.put_word(base + 12, m.word(next + 12));
        m.put_word(base + 20, m.word(next + 20));
    } else {
        m.put_word(base + 12, m.word(base + 12).wrapping_add(delta));
        m.put_word(base + 20, m.word(base + 20).wrapping_add(delta));
    }
    let mut kept = 1u16;
    for j in 1..me {
        let p = base + u32::from(j) * 40;
        let tag = half(m, p + 10);
        if tag == 2 {
            continue;
        }
        m.put_word(
            position,
            m.word(position)
                .wrapping_add(frontend::divmod_u32(m.word(p + 16), 2).0),
        );
        if matches!(tag, 7 | 31 | 36 | 21) {
            let value = parser_last(m, base, j, me);
            let keep = match tag {
                7 => value != 15,
                31 => value != m.word(ctx + 96),
                _ => {
                    params == 0
                        || crate::fe_text::strcmp16_1765ac(
                            m.bytes,
                            params.wrapping_add(value * 2),
                            rt.parser.default_name,
                        ) != 0
                }
            };
            if keep {
                let out = base + u32::from(kept) * 40;
                m.put_half(out + 10, half(m, p + 10));
                m.put_word(out + 12, m.word(base + 12));
                m.put_word(out + 16, 0);
                m.put_word(out + 20, m.word(base + 20));
                m.put_word(out + 24, 0);
                m.put_word(out + 28, value);
                m.bytes[out as usize] = 1;
                kept += 1;
            }
        }
    }
    if kept < me {
        m.put_half(markers + 8, half(m, markers + 8).wrapping_sub(me - kept));
        if half(m, markers + 8) > 1 {
            copy_bytes(
                m,
                m.word(markers + 4) + u32::from(kept) * 40,
                m.word(markers + 4) + u32::from(me) * 40,
                u32::from(half(m, markers + 8).wrapping_sub(kept)) * 40,
            );
        }
    }
    let base = m.word(out_markers + 4);
    let mut j = me;
    loop {
        j = j.wrapping_sub(1);
        if j < 2 || m.bytes[(base + u32::from(j) * 40) as usize] != 1 {
            break;
        }
    }
    let last = base + u32::from(j) * 40;
    m.put_word(
        base + 16,
        delta
            .wrapping_add(m.word(base + 20))
            .wrapping_add(m.word(last + 12).wrapping_add(m.word(last + 16)))
            .wrapping_sub(m.word(last + 24).wrapping_add(m.word(last + 20)))
            .wrapping_sub(m.word(base + 12)),
    );
    m.put_word(base + 24, delta);
    te = 0;
    for j in 0..half(m, out_markers + 8) {
        let p = base + u32::from(j) * 40;
        let tag = half(m, p + 10);
        if (tag == 3
            && (m.word(p + 28) == 0
                || m.word(p + 20).wrapping_sub(m.word(base + 20)) < m.word(base + 24)))
            || !matches!(tag, 2 | 3)
        {
            let out = base + u32::from(te) * 40;
            copy_bytes(m, out, p, 40);
            if (half(m, out + 10) == 3 && m.word(out + 28) == 0)
                || (half(m, p + 10) == 3
                    && m.word(p + 20).wrapping_sub(m.word(base + 20)) < m.word(base + 24))
            {
                m.put_half(out + 10, 999);
                m.put_word(out + 28, 0);
            }
            if half(m, out + 10) == 3 {
                m.put_half(out + 10, 2);
            }
            te = te.wrapping_add(1);
        }
        if m.word(p + 20) <= m.word(base + 20)
            && half(m, p + 10) != 0x4000
            && m.bytes[p as usize] == 0
        {
            m.put_word(pos_ref, m.word(pos_ref).wrapping_add(m.word(p + 16)));
        }
    }
    m.put_half(out_markers + 8, te);
    let s = rt.scratch;
    let saved = m.word(s);
    let rc = fn_2c2564_m(
        m,
        ctx,
        handle,
        pos_ref,
        s,
        s + 2,
        ref_reset,
        reset,
        rt.env,
        rt.parser,
        allocator,
    );
    m.put_word(s, saved);
    if rc >= 0x80000000 {
        return rc;
    }
    let (rc, out_markers) = fetch_parser_m(m, handle, 0x3fd, 1, rt.env, allocator);
    if rc >= 0x80000000 {
        return rc;
    }
    let base = m.word(out_markers + 4);
    let pos = m.word(base + 12);
    let len = m.word(base + 16);
    let (rc, markers) = fetch_parser_m(m, handle, 0x3f3, 1, rt.env, allocator);
    if rc >= 0x80000000 {
        return rc;
    }
    let base = m.word(markers + 4);
    if !reset_position {
        m.put_word(base + 12, pos.wrapping_add(len));
        for j in 1..half(m, markers + 8) {
            let p = base + u32::from(j) * 40;
            if m.word(p + 20) == m.word(base + 20) && matches!(half(m, p + 10), 7 | 31 | 36 | 21) {
                m.put_word(p + 12, m.word(base + 12));
            }
        }
    }
    0
}

/// D_2bd850 binding used by the segmenter's backward-scan callback.
pub fn segment_previous(m: &mut Memory<'_>, current: u32, previous: u32) -> u32 {
    render_part_c::fn_2bd850(m, current, previous)
}
