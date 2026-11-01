

use crate::boot::Memory;
use crate::engine_components::InternalCalls;
use crate::engine_components2::ComponentPlatform;
use crate::engine::{Error, Memory as EngineMemory, Region};

pub fn flat_regions<'a>(
    data: &'a mut [u8],
) -> [Region<'a>; 1] {
    [Region { base: 0, bytes: data }]
}

pub struct Platform;
impl ComponentPlatform for Platform {
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
    fn arbiter_address(&self) -> u32 {
        0xa7bb7c
    }
    fn arbitrate(&mut self, _: &mut Memory<'_>, _: &[u32]) -> u32 {
        0
    }
}
impl crate::engine::HeapPlatform for Platform {
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
    fn arbiter_address(&self) -> u32 {
        0xa7bb7c
    }
    fn arbitrate(&mut self, _: &mut Memory<'_>, _: &[u32]) -> u32 {
        0
    }
}

pub struct C2Calls<'a, P> {
    pub inner: crate::engine_components2::BridgeCalls<'a, P>,
    pub first_unbound: Option<u32>,
    pub calls: u32,
    pub seen: [u32; 256],
    pub seen_len: usize,

    pub call_bound: u32,
    pub recent: [u32; 32],
    pub recent_at: usize,
}

impl<'a, P: ComponentPlatform> C2Calls<'a, P> {
    pub fn new(workspace: &'a mut [u8], platform: &'a mut P) -> Self {
        C2Calls {
            inner: crate::engine_components2::BridgeCalls { workspace, platform, rom: crate::rom_region::Rom::Us },
            first_unbound: None,
            calls: 0,
            seen: [0; 256],
            seen_len: 0,
            call_bound: 0,
            recent: [0; 32],
            recent_at: 0,
        }
    }

    pub fn note(&mut self, entry: u32) {
        let mut i = 0;
        while i < self.seen_len {
            if self.seen[i] == entry {
                return;
            }
            i += 1;
        }
        if self.seen_len < self.seen.len() {
            self.seen[self.seen_len] = entry;
            self.seen_len += 1;
        }
    }
}

impl<P: ComponentPlatform> InternalCalls for C2Calls<'_, P> {
    fn rom(&self) -> crate::rom_region::Rom {
        self.inner.rom
    }
    fn call(&mut self, m: &mut EngineMemory<'_,'_>, entry: u32, a: &[u32]) -> Result<u32, Error> {

        let entry = crate::rom_region::to_us(self.inner.rom, entry);
        self.calls += 1;
        self.note(entry);
        self.recent[self.recent_at] = entry;
        self.recent_at = (self.recent_at + 1) % self.recent.len();
        if self.call_bound != 0 && self.calls > self.call_bound {
            panic!(
                "1d3314 call bound {} exceeded; last32={:x?}",
                self.call_bound, &self.recent
            );
        }
        if entry == 0x1795f0 {

            return Ok(self.open_data(m, a));
        }
        if entry == 0x1ecf68 {

            return self.lid_query(m, a);
        }
        if entry == 0x26b718 {
            return Ok(self.lid_lookup(m, a));
        }
        if entry == 0x179f3c {
            return Ok(self.data_close(m, a));
        }
        match self.inner.call(m, entry, a) {
            Ok(rc) => Ok(rc),
            Err(e) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(entry);
                }
                Err(e)
            }
        }
    }
}

impl<P: ComponentPlatform> C2Calls<'_, P> {

    fn open_data(&mut self, m: &mut EngineMemory<'_,'_>, a: &[u32]) -> u32 {
        let Some(mut view) = boot_view(m) else {
            return 0x8150_6007;
        };

        let rom = self.inner.rom;
        let globals = crate::engine_components2::broker_for(rom).data_open(DATA_SCRATCH);
        let mut files = crate::engine_components2::live_fs::FilePlatform {
            env: crate::speech_part_1::ArchiveEnvironment {
                dot: crate::rom_region::addr(rom, 0x8cd208),
                parent: crate::rom_region::addr(rom, 0x8cd20c),
                locale: 0,
            },
            scratch: FILE_SCRATCH,
            rb: crate::engine_components2::rb_literal(rom),
        };
        let mut host = OpenHost { query: QUERY_SCRATCH, pending_release: None };
        crate::engine_components2::fn_1795f0_live(
            &mut view,
            a[0],
            a[1],
            a[2],
            a[3],
            &globals,
            &mut files,
            &mut host,
        )
    }
}

impl<P: ComponentPlatform> C2Calls<'_, P> {

    fn lid_lookup(&mut self, m: &mut EngineMemory<'_,'_>, a: &[u32]) -> u32 {
        let Some(mut view) = boot_view(m) else {
            return 0x8150_6007;
        };
        let mut inner = OpenHost { query: QUERY_SCRATCH, pending_release: None };
        let mut host = LidHost { inner: &mut inner };
        crate::speech_part_2::fn_26b718(
            &mut view,
            a[0],
            a[1],
            a[2],
            a.get(4).copied().unwrap_or(0),
            LID_SCRATCH,
            &mut host,
        )
    }

    fn data_close(&mut self, m: &mut EngineMemory<'_,'_>, a: &[u32]) -> u32 {
        let Some(mut view) = boot_view(m) else {
            return 0x8150_6007;
        };
        let mut inner = OpenHost { query: QUERY_SCRATCH, pending_release: None };
        let rc = {
            let mut host = CloseHost { inner: &mut inner };
            crate::leftovers_1::fn_179f3c(&mut view, a[0], a[1], &close_runtime(), &mut host)
        };
        if let Some(object) = inner.pending_release.take() {
            let heap = view.word(view.word(view.word(a[0] + 4) + 8) + 4);
            crate::boot_part_e::release(&mut view, heap, object, &mut inner);
        }
        rc
    }
}

impl<P: ComponentPlatform> C2Calls<'_, P> {

    fn lid_query(&mut self, m: &mut EngineMemory<'_,'_>, a: &[u32]) -> Result<u32, Error> {
        let p = a[0];
        if crate::normalization::quad_check(p, a[1], 0xf37f, 0x2a4) >= 0x8000_0000 {
            return Ok(0x8980_2008);
        }
        m.write(a[4], &[0])?;
        let env = m.word(p)?;
        let logger = m.word(env + 16)?;
        if a[2] == 0 {
            self.call(m, 0x173dc0, &[logger, crate::rom_region::addr(self.inner.rom, 0x8f8f44), 0xa029, 0])?;
            return Ok(0x8980_2007);
        }
        let s = *a.get(5).ok_or(Error::UnsupportedStage { entry: 0x1ecf68 })?;
        literal(m, s + 16, b"Entering fe_dctlkp_QueryLidDictExists : %s?\0")?;
        self.call(m, 0x1738e8, &[logger, crate::rom_region::addr(self.inner.rom, 0x8f8f44), 4, 0, s + 16, a[2]])?;
        let broker = m.word(env + 12)?;
        let rc = if a[3] != 0 {
            literal(m, s + 8, b"lid\0")?;
            self.call(m, 0x1795f0, &[broker, s + 8, 1, s])?
        } else {
            self.call(m, 0x179f30, &[broker, p + 0x9e, s])?
        };
        if rc >= 0x8000_0000 {
            return Ok(if a[3] != 0 { 0 } else { rc });
        }
        let handle = m.word(s)?;
        let slot = if a[3] != 0 { 0x48 } else { 8 };
        let key = if a[3] != 0 { 0x44 } else { 4 };
        let rc = self.call(
            m,
            0x26b718,
            &[
                p + slot,
                a[2],
                m.word(p + key)?,
                handle,
                a[4],
            ],
        )?;
        if rc >= 0x8000_0000 {
            return Ok(rc);
        }
        self.call(m, 0x179f3c, &[broker, handle])?;
        Ok(rc)
    }
}

fn literal(m: &mut EngineMemory<'_,'_>, at: u32, bytes: &[u8]) -> Result<(), Error> {
    m.write(at, bytes)?;
    m.write(at + bytes.len() as u32, &[0])?;
    Ok(())
}

fn boot_view<'x>(m: &'x mut EngineMemory<'_,'_>) -> Option<crate::boot::Memory<'x>> {
    if m.regions.len() != 1 {
        return None;
    }
    let r = &mut m.regions[0];
    if r.base != 0 {
        return None;
    }
    Some(crate::boot::Memory { base: 0, bytes: &mut *r.bytes })
}

const DATA_SCRATCH: u32 = 0x0e0f_e800;

pub(crate) const QUERY_SCRATCH: u32 = 0x0e0f_e820;

const FILE_SCRATCH: u32 = 0x0e0f_e000;

const LID_SCRATCH: u32 = 0x0e0f_e840;

const UNMAP_SCRATCH: u32 = 0x0e0f_e900;

const CLOSE_SCRATCH: u32 = 0x0e0f_e940;

fn unmap_environment() -> crate::boot_part_e::UnmapEnvironment {
    crate::boot_part_e::UnmapEnvironment {
        category: 0x8d7668,
        begin_format: 0x179310,
        end_format: 0x17935c,
        error_format: 0x17934c,
        labels: [0x179344, 0x17933c, 0x1791d8],
        scratch: UNMAP_SCRATCH,
    }
}

pub(crate) fn close_runtime() -> crate::leftovers_1::CloseRuntime {
    crate::leftovers_1::CloseRuntime {
        category: 0x8d7668,
        begin: 0x17a14c,
        end: 0x17a18c,
        error: 0x17a180,
        handle_key: hash_text(b"hData"),
        status_key: hash_text(b"lhError"),
        scratch: CLOSE_SCRATCH,
        fold: ascii_fold_table(),
    }
}

fn hash_text(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(2166136261u32, |h, &b| (h ^ b as u32).wrapping_mul(16777619))
}

fn ascii_fold_table() -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < 26 {
        table[b'A' as usize + i] = 1;
        i += 1;
    }
    table
}

pub fn ascii_fold_classes() -> [u16; 256] {
    ascii_fold_table()
}

pub struct LidHost<'a> {
    pub(crate) inner: &'a mut OpenHost,
}

impl crate::speech_part_2::Host for LidHost<'_> {
    fn call(&mut self, memory: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.inner.dispatch(memory, method, args)
    }
}

impl crate::log_live::LogMethodsM for LidHost<'_> {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        self.inner
            .dispatch(m, method, &[object, category, level, arg, format, varargs])
    }
}

impl crate::boot_part_b::Host for LidHost<'_> {
    fn real_log_args(&mut self)->bool {crate::boot_part_b::Host::real_log_args(self.inner)}
    fn format_address_m(&mut self,m:&Memory<'_>,format:&[u8])->Option<u32> {crate::boot_part_b::Host::format_address_m(self.inner,m,format)}
    fn log_variadic(&mut self,m:&mut Memory<'_>,entry:u32,a:[u32;4],format:&[u8],varargs:&[u32])->bool {crate::boot_part_b::Host::log_variadic(self.inner,m,entry,a,format,varargs)}
    fn call(&mut self, memory: &mut Memory<'_>, function: u32, args: &[u32]) -> u32 {
        self.inner.dispatch(memory, function, args)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}

impl crate::boot_part_b::FormatHost for LidHost<'_> {}

impl crate::reentrant::HooksM for LidHost<'_> {
    fn call_m(&mut self, address: u32, args: &[u32], mem: &mut Memory<'_>) -> u32 {
        self.inner.dispatch(mem, address, args)
    }
}

pub struct CloseHost<'a> {
    pub(crate) inner: &'a mut OpenHost,
}

impl crate::frontend::HeapVtable for CloseHost<'_> {
    fn alloc_at(&mut self, _obj: u32, _size: u32) -> u32 {
        0
    }
    fn realloc_at(&mut self, _obj: u32, _old: u32, _size: u32) -> u32 {
        0
    }
    fn release_at(&mut self, _obj: u32, object: u32) {
        self.inner.pending_release = Some(object);
    }
}

impl crate::frontend::MethodTable for CloseHost<'_> {
    fn call_at(&mut self, _obj: u32, _a: u32, _b: u32) -> u32 {
        0
    }
}

impl crate::frontend::StateLock for CloseHost<'_> {
    fn acquire(&mut self) -> u32 {
        0
    }
    fn release(&mut self) -> u32 {
        0
    }
}

impl crate::boot_part_b::FormatHost for CloseHost<'_> {}

impl crate::boot_part_b::Host for CloseHost<'_> {
    fn call(&mut self, memory: &mut Memory<'_>, function: u32, args: &[u32]) -> u32 {
        self.inner.dispatch(memory, function, args)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}

impl crate::log_live::LogMethodsM for CloseHost<'_> {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        self.inner
            .dispatch(m, method, &[object, category, level, arg, format, varargs])
    }
}

impl crate::leftovers_1::CloseHost for CloseHost<'_> {
    fn select_close_lock(&mut self, _lock: u32) {}

    fn compare_tree_keys(&mut self, _a: u32, _b: u32) -> i32 {
        1
    }
    fn indirect(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.inner.dispatch(m, method, args)
    }
}

struct DataHost<'a>(&'a mut OpenHost);

impl crate::boot_part_d::Host for DataHost<'_> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.dispatch(m, f, a)
    }
    fn params_call_m(
        &mut self,
        _q: u32,
        _params: &mut [u32; 4],
        _p: u32,
        _out: &mut u32,
        _flags: u32,
        _m: &mut Memory<'_>,
    ) -> u32 {
        0x8060_2011
    }
}

pub fn fn_17497c(m: &Memory<'_>, log: u32) -> u8 {
    if log == 0 {
        return 0;
    }
    m.byte(log + 0x40)
}

pub fn fn_17649c(m: &mut Memory<'_>, dst: u32, mut src: u32, mut n: u32) -> u32 {
    let mut at = dst;
    loop {
        if n == 0 {
            return at;
        }
        let c = (m.half(src) & 0xff) as u8;
        src += 2;
        m.put_byte(at, c);
        at += 1;
        if c != 0 {
            n -= 1;
        } else {
            break;
        }
    }
    n -= 1;
    while n != 0 {
        m.put_byte(at, 0);
        at += 1;
        n -= 1;
    }
    at
}

pub fn fn_176514(m: &Memory<'_>, s: u32, c: u32) -> u32 {
    let mut at = s;
    while m.half(at) != 0 && u32::from(m.half(at)) != c {
        at += 2;
    }
    if u32::from(m.half(at)) != c {
        return 0;
    }
    at
}

pub fn fn_1765ac(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    let mut x = a;
    let mut y = b;
    loop {
        let c = m.half(x);
        if c == 0 {
            break;
        }
        if c != m.half(y) {
            break;
        }
        x += 2;
        y += 2;
    }
    i32::from(m.half(x)) - i32::from(m.half(y))
}

pub fn fn_176624(m: &Memory<'_>, s: u32) -> i32 {
    let mut at = s;
    let mut n = 0i32;
    while m.half(at) != 0 {
        at += 2;
        n += 1;
    }
    n
}

pub fn fn_1d3314_m(
    m: &mut EngineMemory<'_,'_>,
    p: u32,
    key: u32,
    active: u32,
    s: u32,
    h: &mut impl crate::engine_components::InternalCalls,
) -> Result<u32, Error> {
    match crate::engine_components2::process_1d3314(m, p, key, active, s, h) {
        Ok(rc) => Ok(rc),
        Err(Error::UnsupportedStage { entry: 0x1d3314 }) => {

            let state = m.word(s + 44)?;
            if state != 0 {
                let mut live = [0u8; 1];
                m.read(state, &mut live)?;
                if live[0] != 0 {
                    return Err(Error::UnsupportedStage { entry: 0x1d3314 });
                }
            }
            publish_tail(m, p, s, h)
        }
        Err(e) => Err(e),
    }
}

fn consume_text(
    m: &mut EngineMemory<'_,'_>,
    stream: u32,
    count: u32,
    src: u32,
    h: &mut impl crate::engine_components::InternalCalls,
) -> Result<u32, Error> {
    let state = m.word(stream + 4)?;
    let heap = m.word(m.word(state + 4)? + 4)?;
    h.call(m, 0x14e440, &[state, count])?;
    let capacity = m.word(m.word(state)? + 16)?;
    let need = count.max(capacity);
    let buffer = h.call(m, 0x196394, &[heap, need])?;
    if buffer == 0 {
        return Ok(0x88f0_200a);
    }
    {
        let Some(view) = boot_view(m) else {
            return Ok(0x8150_6007);
        };
        crate::fe_text::memcpy_195ae8(view.bytes, buffer, src, count);
    }
    m.put_word(state + 44, m.word(state + 44)?.wrapping_add(count))?;
    m.put_word(state + 60, buffer)?;
    m.put_word(state + 64, need)?;
    Ok(0)
}

fn publish_tail(
    m: &mut EngineMemory<'_,'_>,
    p: u32,
    s: u32,
    h: &mut impl crate::engine_components::InternalCalls,
) -> Result<u32, Error> {
    if m.word(s + 0x00)? != 0 {
        let rc = consume_text(
            m,
            m.word(p + 0x30)?,
            m.word(s + 0x00)?,
            m.word(s + 0x0c)?,
            h,
        )?;
        if rc >= 0x8000_0000 {
            return Ok(rc);
        }
    }
    let events = m.word(s + 0x1c)?;
    let rc = h.call(
        m,
        0x150c0c,
        &[m.word(p + 0x38)?, m.word(p + 0x3c)?, events.wrapping_mul(24)],
    )?;
    if rc >= 0x8000_0000 {
        return Ok(rc);
    }
    if m.word(s + 0x10)? != 0 {
        let rc = h.call(
            m,
            0x15017c,
            &[m.word(p + 0x20)?, m.word(p + 0x24)?, m.word(s + 0x10)?],
        )?;
        if rc >= 0x8000_0000 {
            return Ok(rc);
        }
    }
    let kept = m.word(s + 0x20)?;
    if kept != 0 {
        let rc = h.call(m, 0x1cf868, &[p, 1, 0, 0])?;
        if rc >= 0x8000_0000 {
            return Ok(rc);
        }
        let rc = h.call(
            m,
            0x15017c,
            &[m.word(p + 0x28)?, m.word(p + 0x2c)?, kept.wrapping_mul(24)],
        )?;
        if rc >= 0x8000_0000 {
            return Ok(rc);
        }
    }
    Ok(0)
}

pub struct OpenHost {

    pub query: u32,

    pub pending_release: Option<u32>,
}

impl OpenHost {

    pub(crate) fn dispatch(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {

        let f = crate::rom_region::to_us(crate::rom_region::Rom::from_entry(f), f);
        match f {
            0xffff0001 => panic!("unbound logger dispatch {f:#x}"),
            0x196394 => crate::boot_part_e::alloc(m, a[0], a[1], self),
            0x1963b8 => crate::boot_part_e::calloc(m, a[0], a[1], a[2], self),
            0x196428 => {
                crate::boot_part_e::release(m, a[0], a[1], self);
                0
            }
            0x39f5e8 => self.locked_allocate(m, a[0], a[1], a[2], 0),
            0x39f5b8 => {
                let inner = m.word(a[0] + 4);
                self.lock(m, inner + 88);
                self.dispatch(m, 0x19b78c, &[inner, a[1]]);
                self.unlock(m, inner + 88);
                0
            }
            0x19b78c => {
                m.boot_heap_release(a[0] + 24, a[1]);
                let count = m.word(a[0] + 84).wrapping_sub(1);
                m.put_word(a[0] + 84, count);
                0
            }
            0x184f20 => {
                let args: [u32; 6] = core::array::from_fn(|i| a.get(i).copied().unwrap_or(0));
                m.boot_heap_allocate(args)
            }
            0x184f84 => {
                m.boot_heap_release(a[0], a[1]);
                0
            }
            0x163f20 => self.locked_allocate(m, a[0], a[1], a[2], a[3]),
            0x4a7758 => {
                let n = a[1].wrapping_mul(a[2]);
                let target = m.word(m.word(a[0]) + 8);
                let p = self.dispatch(m, target, &[a[0], n, 4, 4]);
                if p != 0 {
                    let at = m.offset(p);
                    m.bytes[at..at + n as usize].fill(0);
                }
                p
            }
            0x4a779c => {
                let target = m.word(m.word(a[0]) + 8);
                self.dispatch(m, target, &[a[0], a[1], 4])
            }
            0x4a774c => {
                let target = m.word(m.word(a[0]) + 12);
                self.dispatch(m, target, &[a[0], a[1]])
            }
            0x183edc => {crate::boot_part_b2::fn_183edc(m,a[0],m.word(0xa7bb7c),self);0}
            0x172d18 => {if a.get(2)==Some(&0) {0}else{panic!("unbound arbiter wait")}}
            0x165d00 => {
                let v=m.word(a[0]).wrapping_neg();m.put_word(a[0],v);
                if v as i32>1 {self.dispatch(m,0x172d18,&[m.word(0xa7bb7c),a[0],0,1,0,0]);}0
            }
            0x1738e8 => {
                let varargs = a.get(5).copied().unwrap_or(0);
                crate::log_live::fn_1738e8(m, a[0], a[1], a[2], a[3], a[4], varargs, self)
            }
            0x173dc0 => crate::cold_boot::fn_173dc0(m,self,a[0],a[1],a[2],a[3],a.get(4).copied().unwrap_or(0),0x0e0f7ba0),
            0x18f3fc => {let target=m.word(a[0]);self.dispatch(m,target,&[m.word(a[0]+8),a[2],a[3],a[4],a[5]]);0}
            0x4a7adc => a[0],

            0x196794 => {
                crate::component_live_a2::alloc_handle_for_key_m(m, a[0], a[1], a[2], a[3], self)
            }
            0x196860 => self.step(m, a[0], a[1], a[2]),
            0x196830 => crate::component_live_a::release_owner_m(m, a[0], self),
            0x176f2c => crate::boot_port_1::fn_176f2c(m,a[0],self),
            0x176f5c => crate::boot_port_1::fn_176f5c(m,a[0],self),

            0x17a1a4 => crate::leftovers_1::fn_17a1a4(
                m,
                a[0],
                a[1],
                a[2],
                a[3],
                a[4],
                UNMAP_SCRATCH,
                &unmap_environment(),
                self,
            ),
            0x179208 => crate::boot_part_e::fn_179208(
                m,
                a[0],
                a[1],
                a[2],
                &unmap_environment(),
                self,
            ),
            0x1731c4 => crate::boot_part_d::fn_1731c4(m, a[0], a[1], a[2], a[3], &mut DataHost(self)),
            0x1730fc => crate::boot_part_d::fn_1730fc(m, a[0], &mut DataHost(self)),
            0x17ca94 => crate::boot_part_b::fn_17ca94(m, a[0], self),

            0x1769d0 => crate::fe_text::strcmp_ci_1769d0(m.bytes, a[0], a[1]) as u32,
            0x195d90 => crate::fe_text::strlen_195d90(m.bytes, a[0]) as u32,
            0x195d58 => crate::normalization::strcmp_at(m.bytes, a[0], a[1]) as u32,
            0x17c8c0 => crate::fe_text::fold_17c8c0(a[0]),

            0x4a794c => crate::speech_part_3::fn_4a794c(m, a[0], a[1], a[2], a[3]),
            0x4a78f0 => crate::speech_part_3::fn_4a78f0(m, a[0], a[1], a[2], a[3]),
            0x4a7aac => crate::speech_part_3::fn_4a7aac(m, a[0], self),

            0x1770f8 | 0x1771a8 | 0x177154 => crate::speech_part_1::fn_1770f8(m, a[0], a[1]),
            _ => 0x8000_2001,
        }
    }

    fn step(&mut self, m: &mut Memory<'_>, iterator: u32, key: u32, value: u32) -> u32 {
        let mut k = 0u32;
        let mut v = 0u32;
        let r = crate::frontend::step_iter(
            m.bytes,
            (iterator != 0).then_some(iterator),
            (key != 0).then_some(&mut k),
            (value != 0).then_some(&mut v),
        );
        if key != 0 {
            m.put_word(key, k);
        }
        if value != 0 {
            m.put_word(value, v);
        }
        r
    }

    fn locked_allocate(
        &mut self,
        m: &mut Memory<'_>,
        object: u32,
        size: u32,
        align: u32,
        pool: u32,
    ) -> u32 {
        let inner = m.word(object + 4);
        self.lock(m, inner + 88);
        let r = m.boot_heap_allocate([inner, size, align, pool, 0, 0]);
        self.unlock(m, inner + 88);
        r
    }

    fn lock(&mut self, m: &mut Memory<'_>, address: u32) {
        crate::boot_part_b::fn_1840fc(m, address, self);
    }

    fn unlock(&mut self, m: &mut Memory<'_>, address: u32) {
        crate::boot_part_b::fn_18423c(m, address, self);
    }
}

impl crate::boot_part_b::Host for OpenHost {
    fn real_log_args(&mut self)->bool {true}
    fn format_address_m(&mut self,m:&Memory<'_>,format:&[u8])->Option<u32> {
        let format=format.strip_suffix(&[0]).unwrap_or(format);
        let end=m.bytes.len().min(0xa70000);
        m.bytes[0x100000..end].windows(format.len()+1).position(|w| &w[..format.len()]==format && w[format.len()]==0).map(|i|i as u32+0x100000)
    }
    fn log_variadic(&mut self,m:&mut Memory<'_>,entry:u32,a:[u32;4],format:&[u8],varargs:&[u32])->bool {
        crate::log_live::variadic_m(m,entry,a,format,varargs,0x0e0f7b00,|m,f,a|self.dispatch(m,f,a))
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}
impl crate::boot::Calls for OpenHost {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
    fn call_wide(&mut self, _: &mut Memory<'_>, entry: u32, _: &[u32]) -> u64 {
        panic!("unbound wide call {entry:#x}")
    }
}
impl crate::boot_part_f::Host for OpenHost {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.dispatch(m,f,a)}
    fn compare(&mut self,m:&mut Memory<'_>,_:u32,key:&mut crate::boot_part_f::Key,value:&mut u32)->u32 {crate::boot_part_f::compare_parameter(m,*key,*value)}
}
impl crate::boot_part_e::Host for OpenHost {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, arguments: &[u32]) -> u32 {
        self.dispatch(m, method, arguments)
    }
}
impl crate::speech_part_3::Host for OpenHost {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, arguments: &[u32]) -> u32 {
        self.dispatch(m, method, arguments)
    }
}
impl crate::component_live_a2::Calls for OpenHost {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.dispatch(m, entry, args)
    }
}
impl crate::log_live::LogMethodsM for OpenHost {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        self.dispatch(m, method, &[object, category, level, arg, format, varargs])
    }
}
impl crate::boot_part_b::OutputHost for OpenHost {
    fn output(&mut self, _m: &mut Memory<'_>, f: u32, _a: &[u32]) -> (u32, u32) {
        panic!("unbound output call {f:#x}")
    }
}
impl crate::boot_part_b::PropertyHost for OpenHost {
    fn property(&mut self, _m: &mut Memory<'_>, _p: u32, _k: &[u8]) -> (u32, u32) {
        panic!("unbound broker property lookup")
    }
    fn insert(&mut self, _m: &mut Memory<'_>, _p: u32, _k: u32, _e: [u32; 3]) -> u32 {
        panic!("unbound broker property insert")
    }
}
impl crate::boot_part_b::QueryHost for OpenHost {
    fn query_output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        if f != 0x1772b0 {
            panic!("unbound broker query {f:#x}");
        }
        let scratch = self.query;
        let out = scratch + 12;
        let mut query = BrokerQuery { host: self };
        let rc = crate::boot_port_2c::fn_1772b0(m, a[0], a[1], out, scratch, &mut query);
        (rc, m.word(out))
    }
    fn insert_record(&mut self, _m: &mut Memory<'_>, _db: u32, _k: u32, _r: [u32; 4]) -> u32 {
        0x8150_6007
    }
    fn open_pair(&mut self, _m: &mut Memory<'_>, _f: u32, _o: [u32; 2], _out: u32) -> u32 {
        0x8150_6007
    }
}
impl crate::boot_part_b::DataHost for OpenHost {
    fn data_output(&mut self, _m: &mut Memory<'_>, _f: u32, _a: &[u32], _out: u32) -> u32 {
        0x8060_2001
    }
}

struct BrokerQuery<'a> {
    host: &'a mut OpenHost,
}

impl crate::boot_port_2c::BrokerQuery for BrokerQuery<'_> {
    fn db_lock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32 {
        self.host.dispatch(m,0x176f2c,&[lock])
    }
    fn db_unlock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32 {
        self.host.dispatch(m,0x176f5c,&[lock])
    }
    fn db_open(&mut self, m: &mut Memory<'_>, database: u32, name: u32, mode: u32, out: u32) -> u32 {
        crate::component_live_a2::alloc_handle_for_key_m(
            m,
            database,
            name,
            mode,
            out,
            &mut *self.host,
        )
    }
    fn db_next(&mut self, m: &mut Memory<'_>, iterator: u32, key: u32, value: u32) -> u32 {
        self.host.step(m, iterator, key, value)
    }
    fn db_close(&mut self, m: &mut Memory<'_>, iterator: u32) -> u32 {
        crate::component_live_a::release_owner_m(m, iterator, &mut *self.host)
    }
    fn db_compare(&mut self, m: &Memory<'_>, a: u32, b: u32) -> i32 {
        crate::fe_text::strcmp_ci_1769d0(m.bytes, a, b) as i32
    }
}

pub const D_9154B0: u32 = 0x9154b0;

pub const D_9154C8: u32 = 0x9154c8;

pub const D_201498: u32 = 0x201498;
pub const D_2014B8: u32 = 0x2014b8;
pub const D_2014D8: u32 = 0x2014d8;

fn rb(m: &Memory<'_>, at: u32) -> u8 {
    let off = at.wrapping_sub(m.base) as usize;
    m.bytes.get(off).copied().unwrap_or(0)
}

fn rh(m: &Memory<'_>, at: u32) -> u16 {
    let off = at.wrapping_sub(m.base) as usize;
    match m.bytes.get(off..off + 2) {
        Some(b) => u16::from_le_bytes([b[0], b[1]]),
        None => 0,
    }
}

fn rw(m: &Memory<'_>, at: u32) -> u32 {
    let off = at.wrapping_sub(m.base) as usize;
    match m.bytes.get(off..off + 4) {
        Some(b) => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        None => 0,
    }
}

fn wb(m: &mut Memory<'_>, at: u32, value: u8) {
    let off = at.wrapping_sub(m.base) as usize;
    if let Some(b) = m.bytes.get_mut(off) {
        *b = value;
    }
}

fn wh(m: &mut Memory<'_>, at: u32, value: u16) {
    let off = at.wrapping_sub(m.base) as usize;
    if let Some(b) = m.bytes.get_mut(off..off + 2) {
        b.copy_from_slice(&value.to_le_bytes());
    }
}

fn ww(m: &mut Memory<'_>, at: u32, value: u32) {
    let off = at.wrapping_sub(m.base) as usize;
    if let Some(b) = m.bytes.get_mut(off..off + 4) {
        b.copy_from_slice(&value.to_le_bytes());
    }
}

fn compare_narrow(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    let mut x = a;
    let mut y = b;
    loop {
        let cx = rb(m, x);
        let cy = rb(m, y);
        if cx != cy {
            return if cx < cy { -1 } else { 1 };
        }
        if cy == 0 {
            return 0;
        }
        x = x.wrapping_add(1);
        y = y.wrapping_add(1);
    }
}

pub fn fn_283348_m(c: i32) -> u32 {
    let d: i32 = if (0x41..=0x5a).contains(&c)
        || (0xc0..=0xd6).contains(&c)
        || (0xd8..=0xde).contains(&c)
    {
        0x20
    } else if ((0x100..=0x136).contains(&c) && (c & 1) == 0)
        || ((0x139..=0x147).contains(&c) && (c & 1) != 0)
        || ((0x14a..=0x178).contains(&c) && (c & 1) == 0)
        || ((0x179..=0x17d).contains(&c) && (c & 1) != 0)
    {
        if c == 0x130 {
            -199
        } else if c == 0x178 {
            -121
        } else {
            1
        }
    } else if c == 0x386
        || (0x388..=0x38a).contains(&c)
        || c == 0x38c
        || (0x38e..=0x38f).contains(&c)
        || (0x391..=0x3a1).contains(&c)
        || (0x3a3..=0x3ab).contains(&c)
    {
        match c {
            0x386 => 0x26,
            0x388..=0x38a => 0x25,
            0x38c => 0x40,
            0x38e..=0x38f => 0x3f,
            _ => 0x20,
        }
    } else {
        let t = c.wrapping_sub(0x400) as u32;
        if t <= 0x2f
            || ((0x460..=0x480).contains(&c) && (c & 1) == 0)
            || ((0x48c..=0x4be).contains(&c) && (c & 1) == 0)
            || c == 0x4c1
            || c == 0x4c3
            || c == 0x4c7
            || c == 0x4cb
            || ((0x4d0..=0x4f4).contains(&c) && (c & 1) == 0)
            || c == 0x4f8
        {
            if t <= 0xf {
                0x50
            } else if (0x410..=0x42f).contains(&c) {
                0x20
            } else {
                1
            }
        } else {
            0
        }
    };
    (c.wrapping_add(d) as u16) as u32
}

pub fn fn_2831a4_m(c: u32) -> u32 {
    u32::from(c == 0x22)
}

pub fn fn_2832d8_m(c: u32) -> u32 {
    let punctuation = (0x21..=0x2f).contains(&c)
        || (0x3a..=0x40).contains(&c)
        || (0x5b..=0x60).contains(&c)
        || (0x2000..=0x206f).contains(&c)
        || (0x3000..=0x303f).contains(&c)
        || (0xfe30..=0xfe4f).contains(&c)
        || (0xfe50..=0xfe6f).contains(&c)
        || (0xff00..=0xffef).contains(&c);
    u32::from(!punctuation)
}

pub fn fn_283188_m(m: &Memory<'_>, c: u32, rom: crate::rom_region::Rom) -> u32 {
    fn_283188_at(m, c, crate::rom_region::addr(rom, D_9154C8))
}

pub fn fn_283188_at(m: &Memory<'_>, c: u32, table: u32) -> u32 {
    u32::from(fn_176514(m, table, c) != 0)
}

pub fn fn_1763e4_m(m: &mut Memory<'_>, dst: u32, src: u32) -> u32 {
    let mut d = dst;
    let mut s = src;
    loop {
        let v = rb(m, s) as u16;
        wh(m, d, v);
        if v == 0 {
            break;
        }
        d = d.wrapping_add(2);
        s = s.wrapping_add(1);
    }
    dst
}

pub fn fn_2826f8_m(m: &mut Memory<'_>, p: u32, v: u16, a: u32, b: u32) -> u32 {
    if p == 0 || a == 0 || b == 0 {
        return 0x8a80_2007;
    }
    wh(m, a, 0);
    wh(m, b, v);
    0
}

pub fn fn_282718_m(m: &mut Memory<'_>, s: u32, n: u32, start: u32, len: u32) -> u32 {
    if s == 0 || start == 0 || len == 0 {
        return 0x8a80_2007;
    }
    let old_len = rh(m, len);
    if old_len == 0 {
        return 0x8a80_2014;
    }
    let old_start = rh(m, start);
    let mut i: u32 = 0;
    while rh(m, s.wrapping_add(i.wrapping_mul(2))) != 0
        && fn_2832d8_m(u32::from(rh(m, s.wrapping_add(i.wrapping_mul(2))))) == 0
    {
        i = (i + 1) & 0xffff;
    }
    wh(m, start, i as u16);
    wh(m, len, (n.wrapping_sub(i)) as u16);
    if i < n {
        let mut tail = n;
        while tail != 0
            && u32::from(rh(m, start)) <= tail - 1
            && (rh(m, s.wrapping_add(tail.wrapping_sub(1).wrapping_mul(2))) == 0x2e
                || fn_2832d8_m(u32::from(rh(
                    m,
                    s.wrapping_add(tail.wrapping_sub(1).wrapping_mul(2)),
                ))) == 0)
        {
            tail = tail.wrapping_sub(1);
        }
        if rh(m, s.wrapping_add(tail.wrapping_mul(2))) == 0x2e {
            tail = tail.wrapping_add(1);
        }
        wh(m, len, tail.wrapping_sub(u32::from(rh(m, start))) as u16);
        if rh(m, start) != old_start || rh(m, len) != old_len {
            return 0;
        }
    }
    0x8a80_2014
}

pub fn fn_282810_m(m: &mut Memory<'_>, s: u32, len: u32, pos: u32, count: u32, rom: crate::rom_region::Rom) -> u32 {
    if s == 0 || pos == 0 || count == 0 {
        return 0x8a80_2007;
    }
    let old_count = rh(m, count);
    if old_count == 0 {
        return 0x8a80_2014;
    }
    let old_pos = rh(m, pos);
    let mut open_quote = false;
    let mut close_quote = false;
    let mut i: u32 = 0;
    loop {
        let c = rh(m, s.wrapping_add(i.wrapping_mul(2)));
        if c == 0 || fn_283188_m(m, u32::from(c), rom) == 0 {
            break;
        }
        i = (i + 1) & 0xffff;
    }
    if rh(m, s.wrapping_add(i.wrapping_mul(2))) != 0 {
        if fn_2831a4_m(u32::from(rh(m, s.wrapping_add(i.wrapping_mul(2))))) != 0 {
            open_quote = true;
            i = (i + 1) & 0xffff;
        }
        loop {
            let c = rh(m, s.wrapping_add(i.wrapping_mul(2)));
            if c == 0 || fn_283188_m(m, u32::from(c), rom) == 0 {
                break;
            }
            i = (i + 1) & 0xffff;
        }
    }
    wh(m, pos, i as u16);
    wh(m, count, (len.wrapping_sub(i)) as u16);
    if i >= len {
        return 0x8a80_2014;
    }
    let mut end = len;
    while end != 0
        && u32::from(rh(m, pos)) <= end - 1
        && fn_283188_m(m, u32::from(rh(m, s.wrapping_add(end.wrapping_sub(1).wrapping_mul(2)))), rom) != 0
    {
        end = end.wrapping_sub(1);
    }
    if end != 0
        && u32::from(rh(m, pos)) <= end - 1
        && fn_2831a4_m(u32::from(rh(m, s.wrapping_add(end.wrapping_sub(1).wrapping_mul(2))))) != 0
    {
        close_quote = true;
        end = end.wrapping_sub(1);
    }
    while end != 0
        && u32::from(rh(m, pos)) <= end - 1
        && fn_283188_m(m, u32::from(rh(m, s.wrapping_add(end.wrapping_sub(1).wrapping_mul(2)))), rom) != 0
    {
        end = end.wrapping_sub(1);
    }
    wh(m, count, end.wrapping_sub(u32::from(rh(m, pos))) as u16);
    if rh(m, pos) == old_pos && rh(m, count) == old_count && !(open_quote && close_quote) {
        return 0x8a80_2014;
    }
    0
}

pub fn fn_2829a0_m(m: &mut Memory<'_>, buf: u32, unused: u32, start: u32, len: u32) -> u32 {
    if buf == 0 || start == 0 || len == 0 {
        return 0x8a80_2007;
    }
    let n = rh(m, len);
    if n != 0 {
        let s = rh(m, start);
        let mut e = (u32::from(s).wrapping_add(u32::from(n)) & 0xffff) as u32;
        while e != 0 && e.wrapping_sub(1) >= u32::from(s) && rh(m, buf.wrapping_add(e.wrapping_sub(1).wrapping_mul(2))) == 0x2e {
            e = e.wrapping_sub(1);
        }
        wh(m, len, e.wrapping_sub(u32::from(s)) as u16);
        if rh(m, start) != s || (e.wrapping_sub(u32::from(s)) as u16) != n {
            return 0;
        }
    }
    0x8a80_2014
}

pub fn fn_282a28_m(m: &mut Memory<'_>, buf: u32, unused: u32, start: u32, len: u32) -> u32 {
    if buf == 0 || start == 0 || len == 0 {
        return 0x8a80_2007;
    }
    let orig = rh(m, len);
    if orig != 0 {
        let mut end = (u32::from(rh(m, start)).wrapping_add(u32::from(orig)) & 0xffff) as u32;
        if rh(m, buf.wrapping_add(end.wrapping_mul(2))) == 0x2e {
            end = (end.wrapping_add(1) & 0xffff) as u32;
        }
        wh(m, len, end.wrapping_sub(u32::from(rh(m, start))) as u16);
        let mut changed = false;
        let mut i = u32::from(rh(m, start));
        let stop = u32::from(rh(m, start)).wrapping_add(u32::from(rh(m, len)));
        while i < stop {
            let at = buf.wrapping_add(i.wrapping_mul(2));
            let old = rh(m, at);
            let lower = fn_283348_m(i32::from(old)) as u16;
            if old != lower {
                wh(m, at, lower);
                changed = true;
            }
            i = (i + 1) & 0xffff;
        }
        if rh(m, len) != orig || changed {
            return 0;
        }
    }
    0x8a80_2014
}

pub fn fn_1748a0_m(
    m: &mut Memory<'_>,
    o: u32,
    a: u32,
    b: u32,
    c: u32,
    n: u32,
    f: u32,
    host: &mut C2DictHost,
) -> u32 {
    if o == 0 {
        return 0;
    }
    let v = rb(m, o.wrapping_add(0x40)) as u32;
    if v == 0 {
        return 0;
    }
    if n == 0 {
        return 0x84b0_2006;
    }
    let mut r = 0u32;
    let mut i = 0u32;
    loop {
        let e = o.wrapping_add(i.wrapping_mul(8));
        let vt = rw(m, e.wrapping_add(8).wrapping_add(4));
        if vt != 0 {
            let g = rw(m, vt.wrapping_add(0x34));
            if g != 0 {
                let ctx = rw(m, e.wrapping_add(8));
                let t = host.dispatch(m, g, &[ctx, a, b, c, n, f]);
                if t >= 0x8000_0000 {
                    r = t;
                }
            }
        }
        i = i.wrapping_add(1) & 0xff;
        if i >= 5 {
            break;
        }
    }
    let vt = rw(m, o.wrapping_add(0x38));
    if vt != 0 {
        let g = rw(m, vt.wrapping_add(0x34));
        if g != 0 {
            let ctx = rw(m, o.wrapping_add(0x34));
            let t = host.dispatch(m, g, &[ctx, a, b, c, n, f]);
            if t >= 0x8000_0000 {
                r = t;
            }
        }
    }
    r
}

pub fn fn_201eb0_m(m: &Memory<'_>, p: u32, magic: u32, rom: crate::rom_region::Rom) -> u32 {
    if crate::frontend::validate_handle(p != 0, magic, 0x2f0, 0x30) >= 0x8000_0000 {
        return 0;
    }
    let mut it: u32 = 0;
    loop {
        let entry = if it != 0 {
            let next = rw(m, it.wrapping_add(8));
            it = next;
            if next != 0 {
                rw(m, next.wrapping_add(4))
            } else {
                0
            }
        } else {
            let head = rw(m, p.wrapping_add(28));
            it = head;
            if head != 0 {
                rw(m, head.wrapping_add(4))
            } else {
                0
            }
        };
        if entry == 0 {
            break;
        }
        let kind = rw(m, entry.wrapping_add(8));
        let matched = compare_narrow(m, kind, crate::rom_region::addr(rom, D_201498)) == 0
            || compare_narrow(m, kind, crate::rom_region::addr(rom, D_2014B8)) == 0
            || compare_narrow(m, kind, crate::rom_region::addr(rom, D_2014D8)) == 0;
        if matched && rb(m, entry.wrapping_add(16)) == 1 {
            return 1;
        }
    }
    u32::from(rw(m, p.wrapping_add(40)) != 0)
}

pub const L_NUMBEROUTPUTTRANSCRIPTIONS: u32 = 0x282f30;
pub const L_LANGCODE: u32 = 0x282f4c;
pub const L_IN: u32 = 0x282f5c;
pub const L_BEG: u32 = 0x282f60;
pub const L_FMT_BEG: u32 = 0x282f64;
pub const L_OUT: u32 = 0x282f70;
pub const L_RETC: u32 = 0x282f74;
pub const L_END: u32 = 0x282f7c;
pub const L_FMT_END: u32 = 0x282f80;

pub const D_TRANSFORM_TABLE: u32 = 0x282f2c;

pub const L282324_NUMBEROUTPUTTRANSCRIPTIONS: u32 = 0x28265c;
pub const L282324_BAD_HANDLE: u32 = 0x282678;
pub const L282324_NOT_FOUND: u32 = 0x28267c;

pub const FN282AF0_FRAME: u32 = 0xb8;

pub trait C2DepesHost {

    fn call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32;

    fn rom(&self) -> crate::rom_region::Rom {
        crate::rom_region::Rom::Us
    }
}

const LOOKUP_CALLBACK_SLOT: u32 = 56;

const DICTIONARY_LOOKUP_SLOT: u32 = 0;
const DICTIONARY_RELEASE_SLOT: u32 = 4;
const DICTIONARY_LANGUAGE_SLOT: u32 = 8;
const DICTIONARY_ADJUSTMENT_SLOT: u32 = 24;

#[allow(clippy::too_many_arguments)]

#[allow(clippy::too_many_arguments)]
pub fn fn_282af0_m(
    m: &mut Memory<'_>,
    frame: u32,
    ctx: u32,
    dictionary: u32,
    callbacks: u32,
    handle: u32,
    handle_user: u32,
    all_transforms: u32,
    input: u32,
    content_type: u32,
    output: u32,
    capacity: u32,
    flags: u32,
    h: &mut impl C2DepesHost,
) -> u32 {
    if input == 0 {
        return 0x8a80_2007;
    }
    if dictionary == 0 && (callbacks == 0 || handle == 0) {
        return 0x8a80_2014;
    }
    let settings = rw(m, ctx + 20);
    let logger = rw(m, ctx + 16);

    wh(m, frame + 0x30, 0);
    wh(m, frame + 0x14, 0);
    ww(m, frame + 0x44, 0);
    ww(m, frame + 0x60, 0);
    ww(m, frame + 0x64, 0);
    ww(m, frame + 0x68, 0);
    ww(m, frame + 0x6c, 0);
    ww(m, frame + 0x24, 0);
    let mut limit = 1u32;
    let status = h.call(
        m,
        0x175944,
        &[settings, crate::rom_region::addr(h.rom(), L_NUMBEROUTPUTTRANSCRIPTIONS), frame + 0x6c],
    );
    if status < 0x8000_0000 {
        limit = rw(m, frame + 0x6c) & 0xffff;
    }
    let status = h.call(m, 0x1756d0, &[settings, crate::rom_region::addr(h.rom(), L_LANGCODE), frame + 0x44, 0]);
    if status >= 0x8000_0000 {
        return status;
    }
    let language = rw(m, frame + 0x44);
    let mut word = 0u32;
    if callbacks == 0 {

        let interface = rw(m, dictionary + 0x28);
        let vtable = rw(m, interface);
        let object = interface.wrapping_sub(rw(m, vtable + DICTIONARY_ADJUSTMENT_SLOT));
        let status = h.call(m, rw(m, vtable + DICTIONARY_LANGUAGE_SLOT), &[object, frame + 0x60]);
        let dictionary_language = rw(m, frame + 0x60);
        if status != 0 || dictionary_language == 0 {
            return 0x8a80_2014;
        }
        if fn_1769d0_m(m, dictionary_language, language) != 0 {
            return 0x8a80_2014;
        }
        word = h.call(m, 0x196394, &[rw(m, ctx + 4), fn_17bff4_m(m, input)]);
        if word == 0 {
            return 0x8a80_200a;
        }
        ww(m, frame + 0x24, word);
    }
    let input_length = fn_176624(m, input) as u32;
    ww(m, frame + 0x70, input_length);
    let initial_capacity = rh(m, capacity);
    let table = rw(m, crate::rom_region::addr(h.rom(), D_TRANSFORM_TABLE));
    let transform_count = if all_transforms != 0 { 6u32 } else { 1u32 };
    let mut transform = 0u32;
    let mut result = 0u32;
    while transform < transform_count {
        let entry = rw(m, table + transform * 4);

        result = transform_m(m, crate::rom_region::to_us_guest(h.rom(), entry), input, input_length, frame, h.rom());
        if result & 0x1fff == 0x14 {
            transform += 1;
            continue;
        }
        if result >= 0x8000_0000 {
            return result;
        }
        let start = u32::from(rh(m, frame + 0x30));
        let length = u32::from(rh(m, frame + 0x14));
        let text = input.wrapping_add(start.wrapping_mul(2));
        let saved = rh(m, text.wrapping_add(length.wrapping_mul(2)));
        wh(m, text.wrapping_add(length.wrapping_mul(2)), 0);
        if callbacks != 0 {
            h.call(
                m,
                0x17486c,
                &[logger, 0x6b, crate::rom_region::addr(h.rom(), L_FMT_BEG), crate::rom_region::addr(h.rom(), L_BEG), 0, crate::rom_region::addr(h.rom(), L_IN), text],
            );
            let lookup = rw(m, callbacks + LOOKUP_CALLBACK_SLOT);
            result = h.call(
                m,
                lookup,
                &[
                    handle,
                    handle_user,
                    text,
                    capacity,
                    output,
                    content_type,
                    flags,
                ],
            );
            h.call(
                m,
                0x17486c,
                &[logger, 0x6b, crate::rom_region::addr(h.rom(), L_FMT_END), crate::rom_region::addr(h.rom(), L_END), 0, crate::rom_region::addr(h.rom(), L_RETC), result, crate::rom_region::addr(h.rom(), L_OUT), output],
            );
            wh(m, text.wrapping_add(length.wrapping_mul(2)), saved);
        } else {
            result = fn_17bf70_m(m, text, word, h.rom());
            let mut records = 0u32;
            let mut count = 0u32;
            if result < 0x8000_0000 {
                let interface = rw(m, dictionary + 0x28);
                let vtable = rw(m, interface);
                let object =
                    interface.wrapping_sub(rw(m, vtable + DICTIONARY_ADJUSTMENT_SLOT));
                result = h.call(
                    m,
                    rw(m, vtable + DICTIONARY_LOOKUP_SLOT),
                    &[object, word, frame + 0x68, frame + 0x64],
                );
                records = rw(m, frame + 0x68);
                count = rw(m, frame + 0x64);
            }
            wh(m, text.wrapping_add(length.wrapping_mul(2)), saved);
            if result == 0 && count != 0 && records != 0 {
                let kind = rb(m, records);
                ww(m, content_type, u32::from(kind));
                match kind {
                    1 => {
                        let mut output_length = 0u32;
                        wh(m, output, 0);
                        let total = if limit < count { limit } else { count };
                        let mut i = 0u32;
                        let mut failure = 0u32;
                        while i < total {
                            let record = records + i * 12;
                            let record_text = rw(m, record + 4);
                            let mut record_length = rw(m, record + 8);
                            let mut offset = 0u32;
                            while offset < record_length {
                                if rb(m, record_text.wrapping_add(offset)) == 0 {
                                    record_length = offset;
                                    break;
                                }
                                offset += 1;
                            }
                            if record_length + 1 > u32::from(rh(m, capacity)) {
                                failure = 0x8a80_2009;
                                break;
                            }
                            if i != 0 {
                                fn_176314_m(m, output.wrapping_add(output_length * 2), 0x12);
                                output_length += 1;
                            }
                            fn_176314_m(m, output.wrapping_add(output_length * 2), record_text);
                            output_length += record_length;
                            i += 1;
                        }
                        if failure != 0 {
                            result = failure;
                        } else {
                            wh(m, output.wrapping_add(output_length * 2), 0);
                            wh(m, capacity, output_length as u16);
                        }
                    }
                    4 => {
                        let record_text = rw(m, records + 4);
                        let record_length = fn_17bfc0_m(m, record_text);
                        if record_length > u32::from(initial_capacity) {
                            result = 0x8a80_2009;
                        } else {
                            result = fn_17bd74_m(m, record_text, output);
                            if result < 0x8000_0000 {
                                wh(m, capacity, (record_length - 1) as u16);
                            }
                        }
                    }
                    _ => result = 0x8a80_2014,
                }
            } else if result < 0x8000_0000 {
                result = 0x8a80_2014;
            }
            if result < 0x8000_0000 && records != 0 {
                let interface = rw(m, dictionary + 0x28);
                let vtable = rw(m, interface);
                let object = interface.wrapping_sub(rw(m, vtable + DICTIONARY_ADJUSTMENT_SLOT));
                let release = h.call(
                    m,
                    rw(m, vtable + DICTIONARY_RELEASE_SLOT),
                    &[object, records],
                );
                if release != 0 {
                    return (release + 0x1ea0) | 0x8a80_2000;
                }
            }
        }
        if result & 0x1fff != 0x14 && result & 0x1fff != 0x404 {
            if result >= 0x8000_0000 {
                if word != 0 {
                    h.call(m, 0x196428, &[rw(m, ctx + 4), word]);
                }
                return result;
            }
            if result == 0 {
                break;
            }
        }
        transform += 1;
    }
    if word != 0 {
        h.call(m, 0x196428, &[rw(m, ctx + 4), word]);
    }
    if transform >= transform_count {
        return 0x8a80_2014;
    }
    let start = u32::from(rh(m, frame + 0x30));
    let length = u32::from(rh(m, frame + 0x14));
    if rw(m, content_type) == 4 {
        result = restore_text_m(
            m,
            output,
            capacity,
            input,
            input_length,
            u32::from(initial_capacity),
            start,
            length,
        );
    }
    result
}

fn fn_1769d0_m(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    compare_narrow(m, a, b)
}

pub(crate) fn fn_17bf70_m(m: &mut Memory<'_>, text: u32, word: u32, rom: crate::rom_region::Rom) -> u32 {
    const OUT_CELL: u32 = 0x0e0fdd80;
    let len = fn_176624(m, text).wrapping_add(1) as u32;
    crate::normalization::transcode_u16_with_lead(
        m.bytes, text, len, 0, word, 0xffff_ffff, OUT_CELL, 0, crate::rom_region::addr(rom,0x8dc1e0)+12,
    );
    m.word(OUT_CELL)
}

pub fn fn_17bff4_m(m: &Memory<'_>, s: u32) -> u32 {
    let mut at = s;
    let mut n = 1u32;
    loop {
        let c = u32::from(rh(m, at));
        at += 2;
        if c == 0 {
            break;
        }
        n += if c < 0x80 {
            1
        } else if c < 0x800 {
            2
        } else if c < 0x10000 {
            3
        } else if c < 0x110000 {
            4
        } else {
            5
        };
    }
    n
}

pub(crate) fn fn_176314_m(m: &mut Memory<'_>, dst: u32, src: u32) {
    let mut d = dst;
    let mut s = src;
    loop {
        let v = rb(m, s) as u16;
        wh(m, d, v);
        if v == 0 {
            break;
        }
        d += 2;
        s += 1;
    }
}

pub(crate) fn fn_17bfc0_m(m: &Memory<'_>, s: u32) -> u32 {
    let mut n = 1u32;
    let mut p = s;
    let mut c = rb(m, p);
    p = p.wrapping_add(1);
    while c != 0 {
        n = n.wrapping_add(1);
        p = p.wrapping_add(u32::from(rb(m, 0x8dc1f3u32.wrapping_add(u32::from(c)))));
        c = rb(m, p);
        p = p.wrapping_add(1);
    }
    n
}

pub(crate) fn fn_17bd74_m(m: &mut Memory<'_>, src: u32, dst: u32) -> u32 {
    let extra: [u8; 256] = core::array::from_fn(|i| m.byte(0x8dc1f3 + i as u32));
    let mask: [u8; 16] = core::array::from_fn(|i| m.byte(0x8dc1e0 + i as u32));
    let base: [u8; 1024] = core::array::from_fn(|i| m.byte(0x8dc2f4 + i as u32));
    let t = crate::normalization::DecoderTables {
        extra: &extra,
        mask: &mask,
        mask_base: 6,
        base: &base,
        base_base: 0,
    };
    let len = crate::fe_text::strlen_195d90(m.bytes, src).max(0) as u32;
    let (_status, _offset, count) = crate::normalization::decode_run(
        m.bytes,
        src,
        len.wrapping_add(1),
        dst,
        0xffff_ffff,
        0,
        &t,
    );
    count
}

#[allow(clippy::too_many_arguments)]
fn restore_text_m(
    m: &mut Memory<'_>,
    output: u32,
    capacity: u32,
    input: u32,
    input_length: u32,
    initial_capacity: u32,
    prefix: u32,
    replaced: u32,
) -> u32 {
    let cap = u32::from(rh(m, capacity));
    if (cap + input_length - replaced) as i32 > initial_capacity as i32 {
        return 0x8a80_2009;
    }
    if prefix != 0 {
        move_bytes_m(m, output + prefix * 2, output, cap * 2);
        move_bytes_m(m, output, input, prefix * 2);
    }
    let tail = input_length - replaced - prefix;
    if tail > 0 {
        move_bytes_m(
            m,
            output + (prefix + cap) * 2,
            input + (prefix + replaced) * 2,
            tail * 2,
        );
    }
    wh(m, capacity, (cap + input_length - replaced) as u16);
    0
}

fn move_bytes_m(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) {
    let mut i = 0u32;
    while i < n {
        let v = rb(m, src + i);
        wb(m, dst + i, v);
        i += 1;
    }
}

fn transform_m(
    m: &mut Memory<'_>,
    entry: u32,
    input: u32,
    input_length: u32,
    frame: u32,
    rom: crate::rom_region::Rom,
) -> u32 {
    let start = frame + 0x30;
    let length = frame + 0x14;
    match entry {
        0x2826f8 => fn_2826f8_m(m, input, input_length as u16, start, length),
        0x282810 => fn_282810_m(m, input, input_length, start, length, rom),
        0x282718 => fn_282718_m(m, input, input_length, start, length),
        0x2829a0 => fn_2829a0_m(m, input, input_length, start, length),
        0x282a28 => fn_282a28_m(m, input, input_length, start, length),
        _ => 0x8a80_2007,
    }
}

pub const FN282324_FRAME: u32 = 0x5c;

#[allow(clippy::too_many_arguments)]
pub fn fn_282324_m(
    m: &mut Memory<'_>,
    frame: u32,
    owner: u32,
    dictionary: u32,
    language: u32,
    word: u32,
    output: u32,
    capacity: u32,
    content_type: u32,
    h: &mut impl C2DepesHost,
) -> u32 {
    let mut limit = 1u32;
    ww(m, frame + 0x10, 0);
    let context = rw(m, owner + 8);
    let settings = rw(m, context + 0x14);
    let status = h.call(
        m,
        0x175944,
        &[settings, L282324_NUMBEROUTPUTTRANSCRIPTIONS, frame + 0x10],
    );
    if status < 0x8000_0000 {
        limit = rw(m, frame + 0x10) & 0xffff;
    }
    if owner == 0
        || rw(m, owner + 0x24) == 0
        || rw(m, dictionary + 8) == 0
        || word == 0
        || rb(m, word) == 0
        || capacity == 0
    {
        return 0x8500_2007;
    }
    let interface = rw(m, dictionary + 0x28);
    let vtable = rw(m, interface);
    let object = interface.wrapping_sub(rw(m, vtable + DICTIONARY_ADJUSTMENT_SLOT));
    ww(m, frame + 0x18, 0);
    let status = h.call(m, rw(m, vtable + DICTIONARY_LANGUAGE_SLOT), &[object, frame + 0x18]);

    let language_object = rw(m, frame + 0x18);
    if status != 0 || language_object == 0 {
        return 0x8500_2014;
    }
    let dictionary_language = rw(m, language_object);
    if dictionary_language == 0 {
        return 0x8500_2014;
    }

    for i in 0..3u32 {
        let folded = crate::fe_text::fold_17c8c0(u32::from(rb(m, dictionary_language + i)));
        wb(m, frame + 0x1c + i, folded as u8);
    }
    wb(m, frame + 0x1f, 0);
    if rw(m, language) != 0
        && crate::frontend::find_substr(m.bytes, rw(m, language), frame + 0x1c) == 0
    {
        return 0x8500_2014;
    }
    ww(m, language, dictionary_language);
    ww(m, frame + 0x14, 0);
    ww(m, frame + 0x0c, 0);
    let mut result = h.call(
        m,
        rw(m, vtable + DICTIONARY_LOOKUP_SLOT),
        &[object, word, frame + 0x14, frame + 0x0c],
    );
    let records = rw(m, frame + 0x14);
    let count = rw(m, frame + 0x0c);
    if result == 0 && count != 0 && records != 0 {
        let kind = rb(m, records);
        ww(m, content_type, u32::from(kind));
        match kind {
            1 => {
                wh(m, output, 0);
                let mut output_length = 0u32;
                let total = if limit < count { limit } else { count };
                let mut i = 0u32;
                while i < total {
                    let record = records + i * 12;
                    if rb(m, record) == 1 {
                        let record_text = rw(m, record + 4);
                        let mut record_length = rw(m, record + 8);
                        let mut offset = 0u32;
                        while offset < record_length {
                            if rb(m, record_text.wrapping_add(offset)) == 0 {
                                record_length = offset;
                                break;
                            }
                            offset += 1;
                        }
                        if record_length + 1 > u32::from(rh(m, capacity)) {
                            result = 0x8500_2009;
                            break;
                        }
                        if i != 0 {
                            fn_176314_m(m, output.wrapping_add(output_length * 2), 0x12);
                            output_length += 1;
                        }
                        fn_176314_m(m, output.wrapping_add(output_length * 2), record_text);
                        output_length += record_length;
                    }
                    i += 1;
                }
                if result == 0 {
                    wh(m, output.wrapping_add(output_length * 2), 0);
                    wh(m, capacity, output_length as u16);
                }
            }
            4 => {
                let record_text = rw(m, records + 4);
                let record_length = fn_17bfc0_m(m, record_text);
                if record_length > u32::from(rh(m, capacity)) {
                    result = 0x8500_2009;
                } else {
                    result = fn_17bd74_m(m, record_text, output);
                    if result < 0x8000_0000 {
                        wh(m, capacity, (record_length - 1) as u16);
                    }
                }
            }
            _ => result = 0x8500_2014,
        }
    } else if result < 0x8000_0000 {
        result = 0x8500_2014;
    }
    if records != 0 {
        let release = h.call(
            m,
            rw(m, vtable + DICTIONARY_RELEASE_SLOT),
            &[object, records],
        );
        if release != 0 {
            return (release + 0x1ea0) | 0x8500_2000;
        }
    }
    result
}

struct C2HeapHost<'a, H: C2DepesHost>(&'a mut H);

impl<H: C2DepesHost> crate::boot_part_f::Host for C2HeapHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.0.call(m, method, args)
    }
    fn compare(
        &mut self,
        m: &mut Memory<'_>,
        _method: u32,
        key: &mut crate::boot_part_f::Key,
        value: &mut u32,
    ) -> u32 {
        crate::boot_part_f::compare_parameter(m, key.clone(), *value)
    }
}

pub fn fn_3015a4_m(
    m: &mut Memory<'_>,
    ctx: u32,
    crit: u32,
    list: u32,
    out_buf: u32,
    out_count: u32,
    h: &mut impl C2DepesHost,
) -> u32 {
    let mut host = C2HeapHost(h);
    crate::speech_part_3::fn_3015a4(m, ctx, crit, list, out_buf, out_count, &mut host)
}

pub fn fn_15140c_m(
    m: &mut Memory<'_>,
    stream: u32,
    key: u32,
    out_remaining: u32,
    out_finished: u32,
    h: &mut C2DictHost,
) -> u32 {
    let rc = h.c2_dict_call(m, 0x176f94, &[stream, key, 0x47de, 12]);
    if rc >= 0x8000_0000 {
        return rc;
    }
    if out_remaining == 0 {
        return 0x88f0_2007;
    }
    if rb(m, stream) != 0 {
        ww(m, out_remaining, 0);
        return 0x88f0_2007;
    }
    let state = rw(m, stream + 4);
    ww(
        m,
        out_remaining,
        rw(m, state + 44).wrapping_sub(rw(m, stream + 8)),
    );
    if out_finished != 0 {
        wb(m, out_finished, rb(m, state + 68));
    }
    0
}

pub fn fn_14fe9c_m(
    m: &mut Memory<'_>,
    object: u32,
    tag: u32,
    out_buffer: u32,
    out_available: u32,
    h: &mut C2DictHost,
) -> u32 {
    let rc = h.c2_dict_call(m, 0x176f94, &[object, tag, 0x47de, 12]);
    if rc >= 0x8000_0000 {
        return rc;
    }
    if out_buffer == 0 || out_available == 0 {
        if out_buffer != 0 {
            ww(m, out_buffer, 0);
        }
        if out_available != 0 {
            ww(m, out_available, 0);
        }
        return 0x88f0_2007;
    }
    if rb(m, object) != 0 {
        ww(m, out_buffer, 0);
        ww(m, out_available, 0);
        return 0x88f0_2007;
    }
    let state = rw(m, object + 4);
    let position = rw(m, object + 8);
    let base = rw(m, state + 56);
    if position < base {
        ww(m, out_buffer, 0);
        ww(m, out_available, 0);
        return 0x88f0_2004;
    }
    let available = rw(m, state + 44).wrapping_sub(position);
    ww(m, out_available, available);
    ww(
        m,
        out_buffer,
        position.wrapping_sub(base).wrapping_add(rw(m, state + 60)),
    );
    if available != 0 && rw(m, out_buffer) == 0 {
        return 0x88f0_200a;
    }
    0
}

pub fn fn_21958c_m(
    m: &mut Memory<'_>,
    a: u32,
    b: u32,
    out: u32,
    which: u32,
    h: &mut C2DictHost,
) -> u32 {
    let rc = h.c2_dict_call(m, 0x176f94, &[a, b, 0xf37e, 64]);
    if rc >= 0x8000_0000 {
        return 0x8970_2008;
    }
    if which == 1 {
        ww(m, out, rw(m, a + 56));
    } else {
        ww(m, out, rw(m, a + 60));
    }
    rc
}

pub const FE_DEPES_KEY: u32 = 0xf38a;

pub const STREAM_KEY: u32 = 0x47de;

pub const STREAM_REMAINING: u32 = 0x15140c;

pub const STREAM_GETTEXT: u32 = 0x14fe9c;

pub const STREAM_WRITE: u32 = 0x1503b0;

pub const STREAM_CLOSE: u32 = 0x14f918;

pub fn fn_2031b0_m(
    m: &mut Memory<'_>,
    frame: u32,
    p: u32,
    b: u32,
    done: u32,
    h: &mut impl C2DepesHost,
) -> u32 {
    let rc = h.call(m, 0x176f94, &[p, b, FE_DEPES_KEY, 0xb8]);
    if rc >= 0x8000_0000 {
        return 0x8a80_2008;
    }
    wb(m, done, 1);
    let ctx = rw(m, p + 8);
    let log = rw(m, ctx + 0x10);
    let reg = rw(m, ctx + 0x14);
    let heap = rw(m, ctx + 4);
    let in0 = rw(m, p + 0x1c);
    let in0_key = rw(m, p + 0x20);
    let in1 = rw(m, p + 0x24);
    let in1_key = rw(m, p + 0x28);

    let out0 = rw(m, p + 0x2c);
    let out0_key = rw(m, p + 0x30);
    let out1 = rw(m, p + 0x34);
    let out1_key = rw(m, p + 0x38);
    let mut available_text = 0u32;
    let mut available_markers = 0u32;
    let mut finished = 0u32;
    let rc = h.call(
        m,
        STREAM_REMAINING,
        &[in0, in0_key, frame + 0, frame + 4],
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    available_text = rw(m, frame + 0);
    finished = rw(m, frame + 4);
    let rc = h.call(
        m,
        STREAM_REMAINING,
        &[in1, in1_key, frame + 8, frame + 4],
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    available_markers = rw(m, frame + 8);
    h.call(m, 0x17486c, &[log, 103, crate::rom_region::addr(h.rom(), 0x202fb8), crate::rom_region::addr(h.rom(), 0x202fbc), 0]);
    if available_text == 0 {

        let mut tail = rc;
        if finished == 0 && available_markers == 0 {
            let mut setting = 0u32;
            let status = h.call(m, 0x175a1c, &[reg, crate::rom_region::addr(h.rom(), 0x202f80), frame + 0x10]);
            if status >= 0x8000_0000
                || rw(m, frame + 0x10) == 0
                || rb(m, rw(m, frame + 0x10)) == 0
            {
                wb(m, done, 0);
            }
        } else {
            if available_markers != 0 {
                let out1 = rw(m, p + 0x34);
                let out1_key = rw(m, p + 0x38);
                let rc = h.call(
                    m,
                    STREAM_GETTEXT,
                    &[in1, in1_key, frame + 0x18, frame + 0x1c],
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                let marker_bytes = rw(m, frame + 0x1c);
                let rc = h.call(
                    m,
                    STREAM_WRITE,
                    &[out1, out1_key, rw(m, frame + 0x18), marker_bytes],
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                if h.call(m, 0x17497c, &[log]) != 0 {
                    h.call(
                        m,
                        0x1748a0,
                        &[
                            log,
                            104,
                            crate::rom_region::addr(h.rom(), 0x202f78),
                            crate::rom_region::addr(h.rom(), 0x202f7c),
                            rw(m, frame + 0x18),
                            marker_bytes,
                        ],
                    );
                }
                let rc = h.call(m, 0x15017c, &[in1, in1_key, marker_bytes]);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                tail = rc;
            }
            h.call(m, 0x1748a0, &[log, 104, crate::rom_region::addr(h.rom(), 0x202f78), crate::rom_region::addr(h.rom(), 0x202f84), 0, 0]);
            if finished != 0 {
                for i in 0..2u32 {
                    if rw(m, p + 0x2c + i * 8) != 0 {

                        tail = h.call(
                            m,
                            0x14f918,
                            &[rw(m, p + 0x2c + i * 8), rw(m, p + 0x30 + i * 8)],
                        );
                        ww(m, p + 0x2c + i * 8, 0);
                        ww(m, p + 0x30 + i * 8, 0);
                    }
                }
            }
        }
        h.call(m, 0x17486c, &[log, 103, crate::rom_region::addr(h.rom(), 0x202f8c), crate::rom_region::addr(h.rom(), 0x202f90), 0]);
        return tail;
    }
    let rc = h.call(
        m,
        STREAM_GETTEXT,
        &[in0, in0_key, frame + 0x18, frame + 0x1c],
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    let text = rw(m, frame + 0x18);
    let text_bytes = rw(m, frame + 0x1c);
    let text_count = (text_bytes >> 1) as u16;
    h.call(
        m,
        0x1748a0,
        &[log, 104, crate::rom_region::addr(h.rom(), 0x202f78), crate::rom_region::addr(h.rom(), 0x202f84), text, text_bytes],
    );

    let text_count = u32::from(text_count);
    let rom = h.rom();
    let category = crate::rom_region::addr(rom, 0x8f9878);
    let mut lookup = 0u32;
    let mut narrow = 0u32;
    let mut replacement = 0u32;
    let mut markers = 0u32;
    let mut ordered = 0u32;
    let mut owned = 0u32;
    let mut multiword_ends = 0u32;
    let mut marker_count = 0u32;
    let mut extra_count = 0u32;
    let rc = 'body: {
        let rc = h.call(m, STREAM_GETTEXT, &[in1, in1_key, frame + 0x20, frame + 0x24]);
        if rc >= 0x8000_0000 {
            break 'body rc;
        }
        let marker_input = rw(m, frame + 0x20);
        let marker_bytes = rw(m, frame + 0x24);
        marker_count = (marker_bytes / 24) & 0xffff;
        let markers_type = crate::rom_region::addr(rom, 0x2035e4);
        if h.call(m, 0x17497c, &[log]) != 0 {
            h.call(
                m,
                0x1748a0,
                &[log, 104, crate::rom_region::addr(rom, 0x2035c0), markers_type, marker_input, marker_count * 24],
            );
        }
        let stream = rw(m, p + 0x18);
        let write = rw(m, stream.wrapping_add(0x34));
        let consume = rw(m, stream.wrapping_add(0x30));
        let text_type = crate::rom_region::addr(rom, 0x2035c4);

        let has_dictionary = rw(m, p + 0x44) != 0
            || (rw(m, p + 0x58) != 0 && fn_201eb0_m(m, rw(m, p + 0x5c), rw(m, p + 0x60), rom) != 0);
        if !has_dictionary {

            let passthrough = crate::rom_region::addr(rom, 0x205ca4);
            let rc = h.call(m, write, &[out0, out0_key, text, text_bytes]);
            if rc >= 0x8000_0000 {
                break 'body rc;
            }
            h.call(m, 0x1748a0, &[log, 105, passthrough, text_type, text, text_bytes]);
            let rc = h.call(m, consume, &[in0, in0_key, text_bytes]);
            if rc >= 0x8000_0000 {
                break 'body rc;
            }
            let rc = h.call(m, write, &[out1, out1_key, marker_input, marker_bytes]);
            if rc >= 0x8000_0000 {
                break 'body rc;
            }
            if h.call(m, 0x17497c, &[log]) != 0 {
                h.call(m, 0x1748a0, &[log, 105, passthrough, markers_type, marker_input, marker_count * 24]);
            }
            break 'body h.call(m, consume, &[in1, in1_key, marker_bytes]);
        }

        fn_1763e4_m(m, frame + 0x40, crate::rom_region::addr(rom, 0x203960));
        fn_1763e4_m(m, frame + 0x50, crate::rom_region::addr(rom, 0x203968));
        fn_1763e4_m(m, frame + 0x60, crate::rom_region::addr(rom, 0x203970));
        fn_1763e4_m(m, frame + 0x70, crate::rom_region::addr(rom, 0x203978));
        fn_1763e4_m(m, frame + 0x80, crate::rom_region::addr(rom, 0x20397c));
        let spell = frame + 0x40;
        let normal = frame + 0x50;
        let lid = frame + 0x60;
        let dummy = frame + 0x70;
        let punctuation = frame + 0x80;
        let language_text = frame + 0x38;
        let f100 = u32::from(rh(m, p + 0x64));
        let f102 = u32::from(rh(m, p + 0x66));
        let f104 = u32::from(rh(m, p + 0x68));
        lookup = h.call(m, 0x1963b8, &[heap, f102 + 1, 2]);
        if lookup == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        let max_length = text_count.max(f100);
        narrow = h.call(m, 0x1963b8, &[heap, 1, f104.wrapping_mul(max_length + 1).wrapping_add(1)]);
        if narrow == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        replacement = h.call(m, 0x1963b8, &[heap, (f100 + 1).wrapping_mul(f104).wrapping_add(1), 2]);
        if replacement == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        markers = h.call(m, 0x196394, &[heap, marker_bytes]);
        if markers == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        h.call(m, 0x195ae8, &[markers, marker_input, marker_bytes]);
        h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x203984)]);
        for i in 0..marker_count {
            c2_log_marker(m, h, log, category, markers + i * 24);
        }

        for i in 0..marker_count {
            let marker = markers + i * 24;
            if rh(m, marker) == 1 {
                ww(m, marker + 0x10, 0);
            }
        }

        let depes = rw(m, p + 0x14);
        let depes_handle = rw(m, p + 0x0c);
        let depes_key = rw(m, p + 0x10);
        let mut rc = h.call(m, rw(m, depes.wrapping_add(0x3c)), &[depes_handle, depes_key, frame + 0x28, 0]);
        if rc >= 0x8000_0000 {
            break 'body rc;
        }
        ordered = h.call(m, 0x1963b8, &[heap, marker_count, 4]);
        if ordered == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        owned = h.call(m, 0x1963b8, &[heap, marker_count, 4]);
        if owned == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        multiword_ends = h.call(m, 0x1963b8, &[heap, text_bytes.wrapping_add(1), 2]);
        if multiword_ends == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }

        let base_position = rw(m, markers + 0xc);
        let mut state = C2MarkerState { mode: normal, language: normal, in_tag: 0, pronunciation: 15 };
        let mut marker_index = 0u32;
        let mut position_delta = 0i32;
        while marker_index < marker_count && rw(m, markers + marker_index * 24 + 0xc) == base_position {
            let marker = markers + marker_index * 24;
            state.track(m, marker);
            ww(m, marker + 0xc, rw(m, marker + 0xc).wrapping_add(position_delta as u32));
            ww(m, ordered + marker_index * 4, marker);
            wb(m, owned + marker_index, 0);
            marker_index += 1;
        }

        let mut start = 0xffffu32;
        for i in 0..marker_count {
            let marker = markers + i * 24;
            let kind = rh(m, marker);
            let offset = (rw(m, marker + 0xc).wrapping_sub(base_position) >> 1) & 0xffff;
            if kind == 0x20 {
                let value = rw(m, marker + 0x14);
                if value == 1 {
                    if start != 0xffff {
                        h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x8f9928), start, offset]);
                    }
                    start = offset;
                } else if value == 0 {
                    if start == 0xffff {
                        h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x8f99a0), offset]);
                    } else {
                        let mut end = offset;
                        if end == text_count && end != 0 && c2_delimiter(m, text + (end - 1) * 2, rom) {
                            end -= 1;
                        }
                        while end != text_count && end != 0 && c2_delimiter(m, text + end * 2, rom) {
                            end -= 1;
                        }
                        if end != text_count && !c2_delimiter(m, text + end * 2, rom) {
                            end = (end + 1) & 0xffff;
                        }
                        wh(m, multiword_ends + start * 2, end as u16);
                        start = 0xffff;
                    }
                }
            } else if (kind == 0x15 || kind == 7 || kind == 0x22) && start != 0xffff {
                h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x8f99fc), start]);
                start = 0xffff;
            }
        }
        if start != 0xffff {
            h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x8f9a50), start]);
        }

        let sink = C2Sink { write, out: out0, key: out0_key, log, content: text_type };
        let word_label = crate::rom_region::addr(rom, 0x204148);
        let output_label = crate::rom_region::addr(rom, 0x205818);
        let moved_format = crate::rom_region::addr(rom, 0x8f9b3c);
        let mut word_end = 0u32;
        let mut word_start;
        let mut retry = 0u32;
        let mut multiword_start = 0u32;
        let mut multiword_end = 0u32;
        let mut length;
        let mut capacity;
        let mut output_type;
        let mut output_language = 0u32;
        let mut dummy_count;
        while word_end < text_count {
            let run_start = word_end;
            while word_end < text_count && c2_delimiter(m, text + word_end * 2, rom) {
                word_end += 1;
            }
            let mut is_multiword = 0u32;
            if retry == 0 {
                let mut i = run_start;
                while i <= word_end {
                    let end = u32::from(rh(m, multiword_ends + i * 2));
                    if end != 0 {
                        multiword_start = i;
                        is_multiword = 1;
                        multiword_end = end;
                        break;
                    }
                    i += 1;
                }
                if run_start == word_end {
                    let end = u32::from(rh(m, multiword_ends + word_end * 2));
                    if end != 0 {
                        multiword_end = end;
                        is_multiword = 1;
                        multiword_start = word_end;
                    }
                }
            }

            if word_end > run_start {
                if is_multiword != 0 && multiword_end > word_end && word_end > multiword_start {
                    word_end = multiword_start;
                }
                if word_end as i32 - run_start as i32 > 0 {
                    rc = c2_write(m, h, &sink, word_label, text + run_start * 2, (word_end - run_start) * 2);
                    if rc >= 0x8000_0000 {
                        break 'body rc;
                    }
                }
            }
            if is_multiword == 1 {
                let span = multiword_end.wrapping_sub(multiword_start) as i32;
                length = span as u32 & 0xffff;
                h.call(
                    m,
                    0x1738e8,
                    &[log, category, 5, 0, crate::rom_region::addr(rom, 0x20414c), multiword_start, multiword_end],
                );
                if f102 < length {
                    h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x204160), length, f102]);
                    word_end = (word_end + length) & 0xffff;
                    if span > 0 {
                        rc = c2_write(m, h, &sink, word_label, text + multiword_start * 2, length * 2);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    }
                    continue;
                }
                word_start = multiword_start;
                word_end = multiword_end;
                crate::fe_text::strncpy16_176644(m.bytes, lookup, text + multiword_start * 2, length);
                wh(m, lookup + length * 2, 0);
            } else {
                word_start = word_end;
                length = 0;
                crate::fe_text::memset_195bec(m.bytes, lookup, 0, f102 * 2);
                while word_end < text_count
                    && !c2_delimiter(m, text + word_end * 2, rom)
                    && f102 as i32 - 1 > length as i32
                {
                    wh(m, lookup + length * 2, rh(m, text + word_end * 2));
                    length = (length + 1) & 0xffff;
                    word_end = (word_end + 1) & 0xffff;
                }
                if word_end < text_count && !c2_delimiter(m, text + word_end * 2, rom) {

                    while word_end < text_count && !c2_delimiter(m, text + word_end * 2, rom) {
                        word_end += 1;
                    }
                    if word_end > word_start {
                        rc = c2_write(m, h, &sink, word_label, text + word_start * 2, (word_end - word_start) * 2);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    }
                    continue;
                }
                if length == 0 {
                    continue;
                }
            }

            output_type = 1;
            while marker_index < marker_count {
                let marker = markers + marker_index * 24;
                let position = rw(m, marker + 0xc);
                if word_start * 2 < (position.wrapping_sub(base_position) & 0xffff) {
                    break;
                }
                ww(m, marker + 0xc, position.wrapping_add(position_delta as u32));
                let slot = extra_count + marker_index;
                ww(m, ordered + slot * 4, marker);
                wb(m, owned + slot, 0);
                state.track(m, marker);
                marker_index = (marker_index + 1) & 0xffff;
            }
            capacity = f100;
            if fn_1765ac(m, state.mode, spell) != 0 && state.in_tag == 0 {
                fn_17649c(m, narrow, text + word_start * 2, length);
                wb(m, narrow + length, 0);
                h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x204604), narrow]);
                crate::fe_text::memset_195bec(m.bytes, replacement, 0, f100 * 2);
                let language_length = fn_176624(m, state.language);
                output_language = if language_length == 3
                    || (fn_176624(m, state.language) == 7
                        && crate::fe_text::strstr16_176538(m.bytes, state.language, lid) != 0)
                {
                    let target = rw(m, p + 0x6c);
                    crate::fe_text::narrow_176460(m.bytes, target, state.language);
                    rw(m, p + 0x6c)
                } else {
                    0
                };
                h.call(
                    m,
                    0x17486c,
                    &[
                        log,
                        106,
                        crate::rom_region::addr(rom, 0x204644),
                        crate::rom_region::addr(rom, 0x204640),
                        0,
                        crate::rom_region::addr(rom, 0x20463c),
                        lookup,
                        crate::rom_region::addr(rom, 0x204634),
                        crate::rom_region::addr(rom, if is_multiword != 0 { 0x20462c } else { 0x204630 }),
                        crate::rom_region::addr(rom, 0x204624),
                        output_language,
                        crate::rom_region::addr(rom, 0x20461c),
                        word_start,
                        crate::rom_region::addr(rom, 0x204614),
                        word_start + length,
                    ],
                );

                ww(m, frame + 0x210, output_type);
                wh(m, frame + 0x214, capacity as u16);
                ww(m, frame + 0x218, output_language);
                rc = if h.call(m, 0x176f94, &[p, b, FE_DEPES_KEY, 0xb8]) >= 0x8000_0000 {
                    0x8a80_2008
                } else {
                    fn_282af0_m(
                        m,
                        frame + 0x280,
                        ctx,
                        rw(m, p + 0x44),
                        rw(m, p + 0x58),
                        rw(m, p + 0x5c),
                        rw(m, p + 0x60),
                        1,
                        lookup,
                        frame + 0x210,
                        replacement,
                        frame + 0x214,
                        frame + 0x218,
                        h,
                    )
                };
                output_type = rw(m, frame + 0x210);
                capacity = u32::from(rh(m, frame + 0x214));
                output_language = rw(m, frame + 0x218);
                if rc < 0x8000_0000 {
                    h.call(
                        m,
                        0x17486c,
                        &[
                            log,
                            106,
                            crate::rom_region::addr(rom, 0x204678),
                            crate::rom_region::addr(rom, 0x204674),
                            0,
                            crate::rom_region::addr(rom, 0x20466c),
                            rc,
                            crate::rom_region::addr(rom, 0x204634),
                            output_type,
                            crate::rom_region::addr(rom, 0x204668),
                            replacement,
                            crate::rom_region::addr(rom, 0x204624),
                            output_language,
                        ],
                    );
                } else {
                    h.call(
                        m,
                        0x17486c,
                        &[
                            log,
                            106,
                            crate::rom_region::addr(rom, 0x204694),
                            crate::rom_region::addr(rom, 0x204674),
                            0,
                            crate::rom_region::addr(rom, 0x20466c),
                            rc,
                        ],
                    );
                }
            }
            dummy_count = 0;
            if rc < 0x8000_0000 && capacity != 0 && capacity < f100 {
                crate::fe_text::narrow_176460(m.bytes, narrow, replacement);
                h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x2046a0), narrow]);
                if output_language != 0 {

                    fn_1763e4_m(m, language_text, output_language);
                    let count = marker_count + extra_count;
                    let grown = h.call(m, 0x1963e0, &[heap, ordered, (count + 1) * 4]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    ordered = grown;
                    let grown = h.call(m, 0x1963e0, &[heap, owned, (count + 1) * 4]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    owned = grown;
                    wb(m, owned + count, 0);
                    let marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                    if marker == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    let saved = h.call(m, 0x1963b8, &[heap, fn_176624(m, language_text) as u32 + 1, 2]);
                    if saved == 0 {
                        let rc = c2_out_of_memory(m, h, log, category);
                        h.call(m, 0x196428, &[heap, marker]);
                        break 'body rc;
                    }
                    crate::fe_text::strcpy16_176214(m.bytes, saved, language_text);
                    let position = base_position.wrapping_add(word_start * 2).wrapping_add(position_delta as u32);
                    c2_fill_marker(m, marker, 0x24, position, saved);
                    let slot = extra_count + marker_index;
                    extra_count = (extra_count + 1) & 0xffff;
                    ww(m, ordered + slot * 4, marker);
                    wb(m, owned + slot, 1);
                }
                if output_type == 1 {

                    let count = marker_count + extra_count;
                    let grown = h.call(m, 0x1963e0, &[heap, ordered, count * 4 + 8]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    ordered = grown;
                    let grown = h.call(m, 0x1963e0, &[heap, owned, count * 4 + 8]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    owned = grown;
                    wb(m, owned + count, 0);
                    wb(m, owned + count + 1, 0);
                    let phonetic_marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                    if phonetic_marker == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    let phonetic = h.call(m, 0x1963b8, &[heap, capacity + 1, 2]);
                    if phonetic == 0 {
                        let rc = c2_out_of_memory(m, h, log, category);
                        h.call(m, 0x196428, &[heap, phonetic_marker]);
                        break 'body rc;
                    }
                    crate::fe_text::strcpy16_176214(m.bytes, phonetic, replacement);
                    if is_multiword == 1 {

                        let mut orthographic = 0u32;
                        let mut j = 0u32;
                        while length > j && c2_delimiter(m, lookup + j * 2, rom) {
                            j = (j + 1) & 0xffff;
                        }
                        while j < length && rh(m, lookup + j * 2) != 0 {
                            if c2_delimiter(m, lookup + j * 2, rom) {
                                orthographic = (orthographic + 1) & 0xffff;
                                while length > j && c2_delimiter(m, lookup + j * 2, rom) {
                                    j = (j + 1) & 0xffff;
                                }
                            }
                            j = (j + 1) & 0xffff;
                        }
                        let mut phonetic_breaks = 0u32;
                        let mut i = 1u32;
                        while capacity as i32 - 1 > i as i32 {
                            let c = rh(m, phonetic + i * 2);
                            if c == u16::from(b'_') || c == u16::from(b'#') {
                                phonetic_breaks = (phonetic_breaks + 1) & 0xffff;
                            }
                            if c == 0x12 {
                                break;
                            }
                            i = (i + 1) & 0xffff;
                        }
                        if orthographic < phonetic_breaks {
                            dummy_count = (phonetic_breaks - orthographic) & 0xffff;
                            h.call(
                                m,
                                0x1738e8,
                                &[
                                    log,
                                    category,
                                    5,
                                    0,
                                    crate::rom_region::addr(rom, 0x8f9a94),
                                    dummy_count,
                                    orthographic + 1,
                                    phonetic_breaks + 1,
                                ],
                            );
                        } else if orthographic > phonetic_breaks {

                            let mut remaining = (orthographic - phonetic_breaks) & 0xffff;
                            h.call(
                                m,
                                0x1738e8,
                                &[
                                    log,
                                    category,
                                    5,
                                    0,
                                    crate::rom_region::addr(rom, 0x8f9aec),
                                    orthographic,
                                    phonetic_breaks,
                                ],
                            );
                            fn_17649c(m, narrow, text + word_start * 2, length);
                            wb(m, narrow + length, 0);
                            let mut k = length.wrapping_sub(1) & 0xffff;
                            while k != 0 && rb(m, narrow + k) == b' ' {
                                k -= 1;
                            }
                            while k != 0 {
                                k -= 1;
                                if k == 0 || rb(m, narrow + k) == b' ' {
                                    break;
                                }
                            }
                            while remaining != 0 && k != 0 {
                                while k != 0 && rb(m, narrow + k) == b' ' {
                                    wb(m, narrow + k, b'-');
                                    k -= 1;
                                }
                                remaining = remaining.wrapping_sub(1) & 0xffff;
                                while k != 0 {
                                    k -= 1;
                                    if k == 0 || rb(m, narrow + k) == b' ' {
                                        break;
                                    }
                                }
                            }
                            h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x204e54), narrow]);
                            for i in 0..length {
                                let c = rb(m, narrow + i);
                                wh(m, text + (word_start + i) * 2, u16::from(c));
                            }
                            let moved = base_position
                                .wrapping_add((word_start.wrapping_add(position_delta as u32) & 0xffff) * 2);
                            for i in 0..marker_count {
                                let marker = markers + i * 24;
                                let position = rw(m, marker + 0xc);
                                let offset = position.wrapping_sub(rw(m, markers + 0xc));
                                if word_start * 2 < offset && offset < (word_start + length) * 2 {
                                    h.call(m, 0x1738e8, &[log, category, 5, 0, moved_format, i, position, moved]);
                                    ww(m, marker + 0xc, moved);
                                }
                            }
                        }
                    } else {

                        let mut i = 1u32;
                        while capacity as i32 - 1 > i as i32 {
                            let c = rh(m, phonetic + i * 2);
                            if (c == u16::from(b'_') || c == u16::from(b'#')) && rh(m, phonetic + (i + 1) * 2) != 0x12 {
                                dummy_count = (dummy_count + 1) & 0xffff;
                            }
                            if c == 0x12 {
                                break;
                            }
                            i = (i + 1) & 0xffff;
                        }
                    }
                    for i in 0..length {
                        let at = text + (word_start + i) * 2;
                        if rh(m, at) == u16::from(b'_') {
                            wh(m, at, u16::from(b'-'));
                        }
                    }
                    let position = base_position.wrapping_add(word_start * 2).wrapping_add(position_delta as u32);
                    c2_fill_marker(m, phonetic_marker, 0x22, position, phonetic);
                    let slot = extra_count + marker_index;
                    ww(m, ordered + slot * 4, phonetic_marker);
                    wb(m, owned + slot, 1);

                    let end_position = base_position
                        .wrapping_add(word_end * 2)
                        .wrapping_add(position_delta as u32)
                        .wrapping_add(dummy_count * 4);
                    let mut language_changed = false;
                    while marker_index < marker_count {
                        let marker = markers + marker_index * 24;
                        let position = rw(m, marker + 0xc);
                        if (position.wrapping_sub(base_position) & 0xffff) >= word_end * 2 {
                            break;
                        }
                        if rh(m, marker) == 0x24 {
                            language_changed = true;
                            state.language = rw(m, marker + 0x14);
                            ww(m, marker + 0xc, end_position);
                        } else if language_changed {
                            ww(m, marker + 0xc, end_position);
                        } else {
                            ww(m, marker + 0xc, position.wrapping_add(position_delta as u32));
                        }
                        let slot = marker_index + extra_count + 1;
                        ww(m, ordered + slot * 4, marker);
                        wb(m, owned + slot, 0);
                        state.track_inside(m, marker);
                        marker_index = (marker_index + 1) & 0xffff;
                    }
                    if word_end < text_count {

                        let mode_marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                        if mode_marker == 0 {
                            let rc = c2_out_of_memory(m, h, log, category);
                            extra_count = (extra_count + 1) & 0xffff;
                            break 'body rc;
                        }
                        let saved = h.call(m, 0x196394, &[heap, (fn_176624(m, state.mode) as u32 + 1) * 2]);
                        if saved == 0 {
                            let rc = c2_out_of_memory(m, h, log, category);
                            h.call(m, 0x196428, &[heap, mode_marker]);
                            extra_count = (extra_count + 1) & 0xffff;
                            break 'body rc;
                        }
                        crate::fe_text::strcpy16_176214(m.bytes, saved, state.mode);
                        let slot = marker_index + extra_count + 1;
                        extra_count = (extra_count + 1) & 0xffff;
                        c2_fill_marker(m, mode_marker, 0x15, end_position, saved);
                        ww(m, ordered + slot * 4, mode_marker);
                        wb(m, owned + slot, 1);
                    }
                    position_delta = c2_sxth(position_delta.wrapping_add(c2_sxth((dummy_count << 2) as i32)));
                    extra_count = (extra_count + 1) & 0xffff;
                } else if output_type == 4
                    && capacity >= 7
                    && crate::fe_text::memcmp_195aac(m.bytes, crate::rom_region::addr(rom, 0x8f9858), replacement, 6) == 0
                    && crate::fe_text::memcmp_195aac(
                        m.bytes,
                        crate::rom_region::addr(rom, 0x8f9858),
                        (replacement + capacity * 2).wrapping_sub(6),
                        6,
                    ) == 0
                {

                    crate::fe_text::memmove_195b38(
                        m.bytes,
                        replacement,
                        replacement + 6,
                        (capacity * 2).wrapping_sub(12),
                    );
                    wh(m, (replacement + capacity * 2).wrapping_sub(12), 0);
                    capacity = capacity.wrapping_sub(6) & 0xffff;
                    crate::fe_text::narrow_176460(m.bytes, narrow, replacement);
                    wb(m, narrow + length, 0);
                    h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x205300), narrow]);
                    let count = marker_count + extra_count;

                    ordered = h.call(m, 0x1963e0, &[heap, ordered, count * 4 + 8]);
                    if ordered == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    owned = h.call(m, 0x1963e0, &[heap, owned, count * 4 + 8]);
                    if owned == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    wb(m, owned + count, 0);
                    wb(m, owned + count + 1, 0);
                    let start_marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                    if start_marker == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    let word_position = base_position.wrapping_add(word_start * 2);
                    let index = marker_index + extra_count;
                    c2_fill_marker(m, start_marker, 7, word_position.wrapping_add(position_delta as u32), 0x3c);
                    ww(m, ordered + index * 4, start_marker);
                    wb(m, owned + index, 1);
                    let end_marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                    if end_marker == 0 {
                        extra_count = (extra_count + 1) & 0xffff;
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    let position = word_position.wrapping_add(capacity * 2).wrapping_add(position_delta as u32);
                    c2_fill_marker(m, end_marker, 7, position, state.pronunciation);
                    extra_count = (extra_count + 2) & 0xffff;
                    ww(m, ordered + (index + 1) * 4, end_marker);
                    wb(m, owned + index + 1, 1);
                }
                if output_language != 0 {

                    fn_1763e4_m(m, language_text, output_language);
                    let count = marker_count + extra_count;
                    let grown = h.call(m, 0x1963e0, &[heap, ordered, (count + 1) * 4]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    ordered = grown;
                    let grown = h.call(m, 0x1963e0, &[heap, owned, (count + 1) * 4]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    owned = grown;
                    wb(m, owned + count, 0);
                    let marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                    if marker == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    let saved = h.call(m, 0x1963b8, &[heap, fn_176624(m, state.language) as u32 + 1, 2]);
                    if saved == 0 {
                        let rc = c2_out_of_memory(m, h, log, category);
                        h.call(m, 0x196428, &[heap, marker]);
                        break 'body rc;
                    }
                    crate::fe_text::strcpy16_176214(m.bytes, saved, state.language);
                    let position = match output_type {
                        1 => base_position.wrapping_add(word_end * 2).wrapping_add(position_delta as u32),
                        4 => base_position
                            .wrapping_add(word_start * 2)
                            .wrapping_add(capacity * 2)
                            .wrapping_add(position_delta as u32),
                        _ => 0,
                    };
                    c2_fill_marker(m, marker, 0x24, position, saved);
                    let slot = extra_count + marker_index;
                    extra_count = (extra_count + 1) & 0xffff;
                    ww(m, ordered + slot * 4, marker);
                    wb(m, owned + slot, 1);
                }
            }

            if is_multiword != 0 && rc >= 0x8000_0000 {
                retry = 1;
            } else {
                let original_length = word_end as i32 - word_start as i32;
                if original_length <= 0
                    || (output_type != 1 && capacity != 0 && capacity < f100 && rc < 0x8000_0000)
                {

                    rc = c2_write(m, h, &sink, output_label, replacement, capacity * 2);
                    if rc >= 0x8000_0000 {
                        break 'body rc;
                    }
                    let original_words = c2_count_words(m, lookup, length, rom);
                    let replacement_words = c2_count_words(m, replacement, capacity, rom);
                    if is_multiword != 0 || replacement_words != original_words {
                        h.call(
                            m,
                            0x1738e8,
                            &[
                                log,
                                category,
                                5,
                                0,
                                crate::rom_region::addr(rom, 0x20581c),
                                original_words,
                                replacement_words,
                            ],
                        );
                        let moved = base_position.wrapping_add(multiword_start * 2);
                        for i in 0..marker_count {
                            let marker = markers + i * 24;
                            let position = rw(m, marker + 0xc);
                            let offset = position.wrapping_sub(rw(m, markers + 0xc));
                            if multiword_start * 2 < offset && offset <= multiword_end * 2 {
                                h.call(m, 0x1738e8, &[log, category, 5, 0, moved_format, i, position, moved]);
                                ww(m, marker + 0xc, moved);
                            }
                        }
                    }
                    let shift = (capacity * 2).wrapping_sub(length * 2) as i32;
                    position_delta = c2_sxth(position_delta.wrapping_add(c2_sxth(shift)));
                } else {
                    let last = text.wrapping_add(word_end.wrapping_sub(1).wrapping_mul(2));
                    let trailing = |m: &Memory<'_>| fn_176514(m, punctuation, u32::from(rh(m, last))) != 0;
                    if !trailing(m) {
                        rc = c2_write(m, h, &sink, output_label, text + word_start * 2, original_length as u32 * 2);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    } else if original_length - 1 > 0 {
                        let bytes = (original_length - 1) as u32 * 2;
                        rc = c2_write(m, h, &sink, output_label, text + word_start * 2, bytes);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    }
                    for _ in 0..dummy_count {
                        rc = c2_write(m, h, &sink, output_label, dummy, 4);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    }
                    if trailing(m) {
                        rc = c2_write(m, h, &sink, output_label, last, 2);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    }
                }
                retry = 0;
            }
            if is_multiword == 1 {
                if retry == 0 {

                    word_end = multiword_end;
                    while marker_index < marker_count {
                        let marker = markers + marker_index * 24;
                        let position = rw(m, marker + 0xc);
                        if (position.wrapping_sub(base_position) & 0xffff) > multiword_end * 2 {
                            break;
                        }
                        let index = extra_count + marker_index;
                        let at = ordered.wrapping_add(index.wrapping_mul(4));
                        let before = at.wrapping_sub(4);
                        let previous = rw(m, before);
                        if rw(m, previous.wrapping_add(0xc)) > position {
                            ww(m, at, previous);
                            let previous_owned = rb(m, owned.wrapping_add(index).wrapping_sub(1));
                            wb(m, owned.wrapping_add(index), previous_owned);
                            ww(m, before, marker);
                            wb(m, owned.wrapping_add(index).wrapping_sub(1), 0);
                        } else {
                            ww(m, at, marker);
                            wb(m, owned.wrapping_add(index), 0);
                        }
                        marker_index = (marker_index + 1) & 0xffff;
                    }
                } else {
                    word_end = multiword_start;
                }
            }
        }

        while marker_index < marker_count {
            let marker = markers + marker_index * 24;
            ww(m, marker + 0xc, rw(m, marker + 0xc).wrapping_add(position_delta as u32));
            ww(m, ordered + (extra_count + marker_index) * 4, marker);
            marker_index = (marker_index + 1) & 0xffff;
        }
        ww(m, markers + 0x10, rw(m, markers + 0x10).wrapping_add(position_delta as u32));
        h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x205c84)]);
        let count = marker_count + extra_count;
        for i in 0..count {
            rc = h.call(m, write, &[out1, out1_key, rw(m, ordered + i * 4), 24]);
            if rc >= 0x8000_0000 {
                break 'body rc;
            }

            if h.call(m, 0x173fd8, &[log]) >= 5 {
                c2_log_marker(m, h, log, category, rw(m, ordered + i * 4));
            }
        }
        if h.call(m, 0x17497c, &[log]) != 0 {
            h.call(
                m,
                0x1748a0,
                &[log, 105, crate::rom_region::addr(rom, 0x205c9c), markers_type, ordered, count * 4],
            );
        }
        rc = h.call(m, consume, &[in0, in0_key, text_bytes]);
        if rc >= 0x8000_0000 {
            break 'body rc;
        }
        rc = h.call(m, consume, &[in1, in1_key, marker_bytes]);
        if rc >= 0x8000_0000 {
            break 'body rc;
        }
        let stream_position = rw(m, frame + 0x28).wrapping_add(position_delta as u32);
        ww(m, frame + 0x28, stream_position);
        h.call(m, rw(m, depes.wrapping_add(0x38)), &[depes_handle, depes_key, stream_position, 0])
    };

    h.call(
        m,
        0x17486c,
        &[
            log,
            103,
            crate::rom_region::addr(rom, 0x205e4c),
            crate::rom_region::addr(rom, 0x205e48),
            0,
            crate::rom_region::addr(rom, 0x205cac),
            rc,
        ],
    );
    if ordered != 0 {
        if owned != 0 {
            let count = marker_count + extra_count;
            let mut j = 1u32;
            while j < count {
                if rb(m, owned + j) != 0 {
                    let marker = rw(m, ordered + j * 4);
                    if rh(m, marker) != 7 {
                        h.call(m, 0x196428, &[heap, rw(m, marker + 0x14)]);
                    }
                    h.call(m, 0x196428, &[heap, rw(m, ordered + j * 4)]);
                }
                j = (j + 1) & 0xffff;
            }
        }
        h.call(m, 0x196428, &[heap, ordered]);
    }
    for buffer in [owned, multiword_ends, lookup, replacement, narrow, markers] {
        if buffer != 0 {
            h.call(m, 0x196428, &[heap, buffer]);
        }
    }
    rc
}

struct C2MarkerState {
    mode: u32,
    language: u32,
    in_tag: u32,
    pronunciation: u32,
}

impl C2MarkerState {

    fn track(&mut self, m: &Memory<'_>, marker: u32) {
        let kind = rh(m, marker);
        let value = rw(m, marker + 0x14);
        if self.in_tag == 0 && kind == 0x15 {
            self.mode = value;
        } else if self.in_tag == 0 && kind == 0x24 {
            self.language = value;
        } else {
            if ((kind == 7 && value == 0x2b) || kind == 0x22) && self.in_tag == 0 {
                self.in_tag = 1;
            } else if (kind == 7 || kind == 0x15) && self.in_tag != 0 {
                self.in_tag = 0;
            }
            if kind == 7 {
                self.pronunciation = value;
            }
        }
    }

    fn track_inside(&mut self, m: &Memory<'_>, marker: u32) {
        let kind = rh(m, marker);
        let value = rw(m, marker + 0x14);
        if self.in_tag == 0 && kind == 0x15 {
            self.mode = value;
        } else if kind == 7 {
            if value == 0x2b {
                self.in_tag = u32::from(self.in_tag == 0);
            } else {
                self.in_tag = 0;
            }
            self.pronunciation = value;
        }
    }
}

struct C2Sink {
    write: u32,
    out: u32,
    key: u32,
    log: u32,
    content: u32,
}

fn c2_write(m: &mut Memory<'_>, h: &mut impl C2DepesHost, sink: &C2Sink, label: u32, at: u32, bytes: u32) -> u32 {
    let rc = h.call(m, sink.write, &[sink.out, sink.key, at, bytes]);
    if rc < 0x8000_0000 {
        h.call(m, 0x1748a0, &[sink.log, 105, label, sink.content, at, bytes]);
    }
    rc
}

fn c2_out_of_memory(m: &mut Memory<'_>, h: &mut impl C2DepesHost, log: u32, category: u32) -> u32 {
    h.call(m, 0x173dc0, &[log, category, 40000, 0]);
    0x8a80_200a
}

fn c2_delimiter(m: &Memory<'_>, at: u32, rom: crate::rom_region::Rom) -> bool {
    fn_283188_m(m, u32::from(rh(m, at)), rom) != 0
}

fn c2_count_words(m: &Memory<'_>, s: u32, n: u32, rom: crate::rom_region::Rom) -> u32 {
    let mut words = 1u32;
    let mut i = 0u32;
    while i < n {
        if c2_delimiter(m, s + i * 2, rom) {
            words = (words + 1) & 0xffff;
            while n > i && c2_delimiter(m, s + i * 2, rom) {
                i = (i + 1) & 0xffff;
            }
        }
        i = (i + 1) & 0xffff;
    }
    words
}

fn c2_fill_marker(m: &mut Memory<'_>, marker: u32, kind: u16, position: u32, data: u32) {
    wh(m, marker, kind);
    ww(m, marker + 4, position);
    ww(m, marker + 8, 0);
    ww(m, marker + 0xc, position);
    ww(m, marker + 0x10, 0);
    ww(m, marker + 0x14, data);
}

fn c2_log_marker(m: &mut Memory<'_>, h: &mut impl C2DepesHost, log: u32, category: u32, marker: u32) {
    let rom = h.rom();
    let kind = rh(m, marker);
    let fields = [rw(m, marker + 4), rw(m, marker + 8), rw(m, marker + 0xc), rw(m, marker + 0x10)];
    if kind == 0x4000 {
        let format = crate::rom_region::addr(rom, 0x2039bc);
        h.call(m, 0x1738e8, &[log, category, 5, 0, format, fields[0], fields[1], fields[2], fields[3]]);
    } else {
        let format = crate::rom_region::addr(rom, 0x203990);
        h.call(
            m,
            0x1738e8,
            &[log, category, 5, 0, format, u32::from(kind), fields[0], fields[1], fields[2], fields[3]],
        );
    }
}

fn c2_sxth(value: i32) -> i32 {
    i32::from(value as u16 as i16)
}

pub const C2_LANGCODE_KEY: u32 = 0x15df6c;

pub const C2_MLSET_KEY: u32 = 0x180970;

pub const C2_NUMBEROUTPUTTRANSCRIPTIONS: u32 = 0x28265c;

pub const C2_DICT_SCRATCH: u32 = 0x1a2000;
pub const C2_2B3608_SCRATCH: u32 = 0x1a3000;

pub struct C2DictTables {
    pub types: [[u8; 48]; 3],
    pub type_lens: [usize; 3],
    pub disabled: [u8; 32],
    pub disabled_len: usize,
    pub classes: [u16; 256],
}

fn copy_nul(dst: &mut [u8], bytes: &[u8], addr: u32) -> usize {
    let mut n = 0usize;
    let mut at = addr as usize;
    while n + 1 < dst.len() && at < bytes.len() && bytes[at] != 0 {
        dst[n] = bytes[at];
        n += 1;
        at += 1;
    }
    if n < dst.len() {
        dst[n] = 0;
        n += 1;
    }
    n
}

pub fn c2_dict_tables(m: &Memory<'_>, tables: &mut C2DictTables, rom: crate::rom_region::Rom) {
    use crate::rom_region::addr;
    for (i, pool) in [0x201498u32, 0x2014b8, 0x2014d8].iter().enumerate() {
        tables.type_lens[i] = copy_nul(&mut tables.types[i], m.bytes, addr(rom, *pool));
    }
    tables.disabled_len = copy_nul(&mut tables.disabled, m.bytes, addr(rom, 0x201b74));
}

pub struct C2DictHost {
    pub first_unbound: Option<u32>,
    pub unbound_count: u32,
    pub error_log_fired: u32,
    pub lang_cb_fired: u32,
    pub trace: [(u32, u32, u32); 512],
    pub trace_len: usize,

    pub rom: crate::rom_region::Rom,
}

impl C2DictHost {
    pub fn new() -> Self {
        C2DictHost {
            first_unbound: None,
            unbound_count: 0,
            error_log_fired: 0,
            lang_cb_fired: 0,
            trace: [(0, 0, 0); 512],
            trace_len: 0,
            rom: crate::rom_region::Rom::Us,
        }
    }
    fn record(&mut self, method: u32, arg0: u32, result: u32) {
        if self.trace_len < self.trace.len() {
            self.trace[self.trace_len] = (method, arg0, result);
            self.trace_len += 1;
        }
    }

    pub fn c2_dict_call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.dispatch(m, method, args)
    }
    fn unbound(&mut self, method: u32) -> u32 {
        if self.first_unbound.is_none() {
            self.first_unbound = Some(method);
        }
        self.unbound_count += 1;
        0x8000_2001
    }
    fn dispatch(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {

        let method = crate::rom_region::to_us(self.rom, method);
        let arg0 = args.first().copied().unwrap_or(0);
        let result = self.dispatch_inner(m, method, args);
        self.record(method, arg0, result);
        result
    }
    fn dispatch_inner(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        match method {

            0x17bff4 => {
                let mut at = args[0];
                let mut n = 1u32;
                loop {
                    let c = u32::from(rh(m, at));
                    at += 2;
                    if c == 0 {
                        break;
                    }
                    n += if c < 0x80 {
                        1
                    } else if c < 0x800 {
                        2
                    } else if c < 0x10000 {
                        3
                    } else if c < 0x110000 {
                        4
                    } else {
                        5
                    };
                }
                n
            }

            0x176f94 => crate::boot_port_1::fn_176f94(m, args[0], args[1], args[2], args[3]),

            0x196394 => crate::boot_part_e::alloc(m, args[0], args[1], self),
            0x1963b8 => crate::boot_part_e::calloc(m, args[0], args[1], args[2], self),
            0x196428 => {
                crate::boot_part_e::release(m, args[0], args[1], self);
                0
            }

            0x4a779c => {
                let target = rw(m, rw(m, args[0]).wrapping_add(8));
                self.dispatch(m, target, &[args[0], args[1], 4])
            }

            0x4a7758 => {
                let n = args[1].wrapping_mul(args[2]);
                let target = rw(m, rw(m, args[0]).wrapping_add(8));
                let p = self.dispatch(m, target, &[args[0], n, 4, 4]);
                if p != 0 {
                    let at = (p as usize).wrapping_sub(m.base as usize);
                    let end = at.wrapping_add(n as usize).min(m.bytes.len());
                    if at < m.bytes.len() {
                        m.bytes[at..end].fill(0);
                    }
                }
                p
            }

            0x4a774c => {
                let target = rw(m, rw(m, args[0]).wrapping_add(12));
                self.dispatch(m, target, &[args[0], args[1]])
            }

            0x39f5e8 => {
                let inner = rw(m, args[0].wrapping_add(4));
                self.lock(m, inner.wrapping_add(88));
                let r = m.boot_heap_allocate([inner, args[1], args[2], 0, 0, 0]);
                self.unlock(m, inner.wrapping_add(88));
                r
            }

            0x39f5b8 => {
                let inner = rw(m, args[0].wrapping_add(4));
                self.lock(m, inner.wrapping_add(88));
                self.dispatch(m, 0x19b78c, &[inner, args[1]]);
                self.unlock(m, inner.wrapping_add(88));
                0
            }

            0x184f20 => {
                let a: [u32; 6] = core::array::from_fn(|i| args.get(i).copied().unwrap_or(0));
                m.boot_heap_allocate(a)
            }
            0x184f84 => {
                m.boot_heap_release(args[0], args[1]);
                0
            }

            0x19b78c => {
                m.boot_heap_release(args[0].wrapping_add(24), args[1]);
                let count = rw(m, args[0].wrapping_add(84)).wrapping_sub(1);
                ww(m, args[0].wrapping_add(84), count);
                0
            }

            0x183edc => 0x1ff82000,

            0x165d00 => 0,
            0x172d18 => 0,
            0x175a1c => {
                let lock_word = if args[0] == 0 { 0 } else { rw(m, args[0] + 4) };
                let mut lock = C2ParamLock { host: &mut *self, word: lock_word };
                crate::speech_part_2::fetch_parameter_m(
                    m,
                    args[0],
                    args[1],
                    args[2],
                    &mut lock,
                    c2_compare,
                )
            }
            0x175944 => {
                let lock_word = if args[0] == 0 { 0 } else { rw(m, args[0] + 4) };
                let mut lock = C2ParamLock { host: &mut *self, word: lock_word };
                crate::reentrant2::get_param_uint_m(
                    m.bytes,
                    if args[0] == 0 { None } else { Some(args[0]) },
                    args[1],
                    args[2],
                    &mut lock,
                    c2_compare,
                )
            }

            0x1756d0 => {
                if args[0] != 0 && args[1] != 0 && args[2] != 0 {
                    ww(m, args[2], 0);
                    if args[3] != 0 {
                        ww(m, args[3], 0);
                    }
                }
                let p = if args[0] == 0 { 0 } else { rw(m, args[0] + 4) };
                let mut value = if args[2] == 0 { 0 } else { rw(m, args[2]) };
                let mut size = if args[3] == 0 { 0 } else { rw(m, args[3]) };
                let mut lock = C2Lock176f { host: &mut *self, p };
                let rc = crate::reentrant2::lookup_param_m(
                    m.bytes,
                    if args[0] == 0 { None } else { Some(args[0]) },
                    args[1],
                    (args[2] != 0).then_some(&mut value),
                    (args[3] != 0).then_some(&mut size),
                    &mut lock,
                    c2_compare,
                );
                if args[2] != 0 {
                    ww(m, args[2], value);
                }
                if args[3] != 0 {
                    ww(m, args[3], size);
                }
                rc
            }

            0x281aa4 => {
                let current = rw(m, args[1]);
                let next = if current == 0 {
                    rw(m, args[0] + 4)
                } else {
                    rw(m, current + 8)
                };
                ww(m, args[1], next);
                if next == 0 {
                    0
                } else {
                    rw(m, next + 4)
                }
            }
            0x17bf70 => fn_17bf70_m(m, args[0], args[1], self.rom),

            0x17486c => 0,

            0x2b331c => {
                m.put_word(args[1], m.word(args[0].wrapping_add(0x44)));
                0
            }
            0x2b3608 => crate::speech_part_3::fn_2b3608(
                m,
                args[0],
                args[1],
                args[2],
                args[3],
                C2_2B3608_SCRATCH,
                self,
            ),

            0x2b4004 => 0,
            0x17c8c0 => crate::fe_text::fold_17c8c0(args[0]),

            0x1ce56c => {
                if args[0] == 0 {
                    0
                } else {
                    let inner = rw(m, args[0]);
                    if rb(m, inner + 4) != 0 {
                        u32::from(rb(m, inner + 0x10c))
                    } else {
                        1
                    }
                }
            }

            0x18f488 => 0,
            0x1960a8 => crate::frontend::find_substr(m.bytes, args[0], args[1]),
            0x176314 => {
                fn_176314_m(m, args[0], args[1]);
                0
            }
            0x17bfc0 => fn_17bfc0_m(m, args[0]),
            0x17bd74 => fn_17bd74_m(m, args[0], args[1]),
            0x173dc0 => {
                self.error_log_fired += 1;
                crate::render_part_a::error_log(
                    m, self, args[0], args[1], args[2], args[3], args[4], args[5],
                )
            }
            0x1738e8 => crate::log_live::fn_1738e8(
                m,
                args[0],
                args[1],
                args[2],
                args[3],
                args[4],
                args.get(5).copied().unwrap_or(0),
                self,
            ),
            _ => self.unbound(method),
        }
    }
}

impl C2DictHost {

    fn lock(&mut self, m: &mut Memory<'_>, address: u32) {
        crate::boot_part_b::fn_1840fc(m, address, self);
    }

    fn unlock(&mut self, m: &mut Memory<'_>, address: u32) {
        crate::boot_part_b::fn_18423c(m, address, self);
    }
}

struct C2Wake<'a> {
    host: &'a mut C2DictHost,
}

impl crate::boot::Calls for C2Wake<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.host.dispatch(m, f, a)
    }
    fn call_wide(&mut self, _: &mut Memory<'_>, entry: u32, _: &[u32]) -> u64 {
        panic!("unbound wide call {entry:#x}")
    }
}

struct C2Lock176f<'a> {
    host: &'a mut C2DictHost,
    p: u32,
}

impl crate::reentrant2::StateLockM for C2Lock176f<'_> {
    fn acquire_m(&mut self, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        crate::boot_port_1::fn_176f2c(&mut m, self.p, self.host)
    }
    fn release_m(&mut self, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        crate::boot_port_1::fn_176f5c(&mut m, self.p, self.host)
    }
}

struct C2ParamLock<'a> {
    host: &'a mut C2DictHost,
    word: u32,
}

impl crate::reentrant2::StateLockM for C2ParamLock<'_> {
    fn acquire_m(&mut self, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        crate::speech_part_2::parameter_lock_m(&mut m, self.word, false, self.host)
    }
    fn release_m(&mut self, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        crate::speech_part_2::parameter_lock_m(&mut m, self.word, true, self.host)
    }
}

fn c2_compare(a: u32, b: u32, bytes: &mut [u8]) -> i32 {
    let mut i = 0usize;
    loop {
        let x = bytes.get(a as usize + i).copied().unwrap_or(0);
        let y = bytes.get(b as usize + i).copied().unwrap_or(0);
        if x != y {
            return if x < y { -1 } else { 1 };
        }
        if x == 0 {
            return 0;
        }
        i += 1;
    }
}

impl crate::speech_part_2::Host for C2DictHost {
    fn call(&mut self, memory: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.dispatch(memory, method, args)
    }
}

impl crate::log_live::LogMethodsM for C2DictHost {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        mem: &mut Memory<'_>,
    ) -> u32 {
        self.dispatch(mem, method, &[object, category, level, arg, format, varargs])
    }
}

impl crate::boot_part_b::FormatHost for C2DictHost {}

impl crate::boot_part_b::Host for C2DictHost {
    fn call(&mut self, memory: &mut Memory<'_>, function: u32, args: &[u32]) -> u32 {
        self.dispatch(memory, function, args)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}

impl crate::boot_part_e::Host for C2DictHost {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, arguments: &[u32]) -> u32 {
        self.dispatch(m, method, arguments)
    }
}

impl crate::reentrant::HooksM for C2DictHost {
    fn call_m(&mut self, address: u32, args: &[u32], mem: &mut Memory<'_>) -> u32 {
        if address == 0 {
            self.lang_cb_fired += 1;
            return 0x8000_2001;
        }
        self.dispatch(mem, address, args)
    }
    fn literal_m(&self, literal: crate::reentrant::LiteralM) -> u32 {
        if literal == crate::reentrant::LiteralM::NumberOutputTranscriptions {
            C2_NUMBEROUTPUTTRANSCRIPTIONS
        } else {
            u32::MAX
        }
    }
}

impl crate::boot_part_f::Host for C2DictHost {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.dispatch(m, method, args)
    }
    fn compare(
        &mut self,
        m: &mut Memory<'_>,
        _method: u32,
        key: &mut crate::boot_part_f::Key,
        value: &mut u32,
    ) -> u32 {
        crate::boot_part_f::compare_parameter(m, key.clone(), *value)
    }
}

impl crate::speech_part_5::Host for C2DictHost {
    fn scratch(&self) -> u32 {
        C2_2B3608_SCRATCH
    }
    fn format(&self, callsite: u32) -> u32 {
        callsite
    }
    fn dictionary_entries(
        &mut self,
        _m: &mut Memory<'_>,
        _dictionary: u32,
        entries: &mut u32,
        count: &mut u32,
    ) -> u32 {
        *entries = 0;
        *count = 0;
        0
    }
    fn record(
        &mut self,
        _m: &mut Memory<'_>,
        _record: u32,
        _arg: u32,
        _buffer: &mut crate::speech_part_5::RecordBuffer,
        found: &mut u8,
    ) -> u32 {
        *found = 0;
        0
    }
    fn entries(
        &mut self,
        _m: &mut Memory<'_>,
        _a: u32,
        _b: u32,
        _c: u32,
        array: &mut u32,
        count: &mut u32,
    ) -> u32 {
        *array = 0;
        *count = 0;
        0
    }
}

impl crate::speech_part_3::SpeechHost for C2DictHost {}

#[allow(clippy::too_many_arguments)]
pub fn fn_202348_m(
    m: &mut Memory<'_>,
    p: u32,
    magic: u32,
    text: u32,
    len: u32,
    out: u32,
    arg5: u32,
    lang: u32,
    scratch: u32,
    tables: &C2DictTables,
    h: &mut C2DictHost,
) -> u32 {
    let t = crate::speech_part_2::DictionaryLiterals {
        types: [
            &tables.types[0][..tables.type_lens[0]],
            &tables.types[1][..tables.type_lens[1]],
            &tables.types[2][..tables.type_lens[2]],
        ],
        disabled_mode: &tables.disabled[..tables.disabled_len],
        langcode: crate::rom_region::addr(h.rom, C2_LANGCODE_KEY),
        mlset: crate::rom_region::addr(h.rom, C2_MLSET_KEY),
        category: crate::rom_region::addr(h.rom, 0x8f9734),
        invalid_format: crate::rom_region::addr(h.rom, 0x155ec0),
        name: crate::rom_region::addr(h.rom, 0x14db07),
        empty: crate::rom_region::addr(h.rom, 0x8f972c),
        type_label: crate::rom_region::addr(h.rom, 0x201500),
        trace_formats: [crate::rom_region::addr(h.rom, 0x202778), crate::rom_region::addr(h.rom, 0x2027a0), crate::rom_region::addr(h.rom, 0x2027a5), crate::rom_region::addr(h.rom, 0x1feedc)],
        labels: [crate::rom_region::addr(h.rom, 0x1fee78), crate::rom_region::addr(h.rom, 0x14daf4), crate::rom_region::addr(h.rom, 0x1cd7e8), crate::rom_region::addr(h.rom, 0x1feed8), crate::rom_region::addr(h.rom, 0x155ea0), crate::rom_region::addr(h.rom, 0x1feed0), crate::rom_region::addr(h.rom, 0x14da60)],
    };
    crate::speech_part_2::fn_202348(
        m, p, magic, text, len, out, arg5, lang, &tables.classes, &t, scratch, h,
    )
}
