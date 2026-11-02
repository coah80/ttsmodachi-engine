use crate::regional_addresses::{address, context};
use crate::regional_ports as ports;
use tts_core::boot::{Calls, Memory};
use tts_core::prosody_rules::{self, arena::Arena, RuleContext, RuleItem, RuleScanHost};
use tts_core::regex::{self, FrameBlockState, FrameStore, MatchFrame, PcreHost, PcreTables};

struct Allocator(u32);
impl PcreHost for Allocator {
    fn validate(&mut self, p: u32, key: u32, tag: u32, size: u32) -> u32 {
        tts_core::frontend::validate_handle(p != 0, key, tag, size)
    }
    fn resolve_owner(&mut self, _: u32, _: i32) -> (u32, u32) {
        (0, 1)
    }
    fn heap_of(&mut self, _: u32) -> u32 {
        1
    }
    fn alloc(&mut self, _: u32, count: u32, size: u32) -> Option<u32> {
        self.0 += (count * size + 15) & !15;
        Some(self.0)
    }
    fn free(&mut self, _: u32, _: u32) {}
}

fn compile(
    pattern: &[u8],
    opts: u32,
    tables: &PcreTables<'_>,
    errpos: &mut i32,
) -> Option<Vec<u8>> {
    let mut bytes = vec![0; 65536];
    let mut host = Allocator(0x100);
    let compiled = regex::compile(
        &mut host,
        1,
        0xdd57,
        1,
        pattern,
        opts as i32,
        tables,
        &mut bytes,
        Some(errpos),
    )
    .ok()??;
    bytes.truncate(compiled.size);
    Some(bytes)
}

fn matches(program: &[u8], subject: &[u8], tables: &PcreTables<'_>) -> bool {
    let mut frames = vec![MatchFrame::default(); 1000];
    let mut tags = vec![0; 1000];
    let mut blocks = vec![FrameBlockState::default(); 100];
    let mut store = FrameStore {
        frames: &mut frames,
        tags: &mut tags,
        blocks: &mut blocks,
        frames_per_block: 10,
        frame_size: 284,
        max_blocks: 100,
        block_count: 0,
        current_block: 0,
        highest_block: 0,
        max_frames: 0,
        fresh: MatchFrame::default(),
        heap: 1,
        blocks_id: None,
    };
    let mut host = Allocator(0x100);
    if regex::pcre_init(&mut host, 1, 0xdd57, &mut store, 100, 10) != 0 {
        return false;
    }
    let len = subject
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(subject.len());
    let mut input = vec![0; len + 512];
    input[256..256 + len].copy_from_slice(&subject[..len]);
    let mut captures = [0; 30];
    regex::pcre_exec_simple(
        &mut host,
        1,
        0xdd57,
        &mut store,
        program,
        None,
        &input,
        256,
        len,
        0,
        0,
        Some(&mut captures),
        30,
        tables,
    ) >= 0
}

struct Scanner<'a> {
    context: RuleContext,
    tables: PcreTables<'a>,
    checks: [Vec<u8>; 3],
    pending: Option<Vec<u8>>,
    rules: Vec<(Vec<u8>, Vec<u8>)>,
}
impl RuleScanHost for Scanner<'_> {
    fn resolve(&mut self, _: u32, _: u32) -> (u32, Option<RuleContext>) {
        (0, Some(self.context))
    }
    fn check_stream(&mut self, _: [u32; 2], tag: &[u8], buf: &[u8]) -> u32 {
        let index = if tag == prosody_rules::RULE_CHECK_A {
            0
        } else {
            1
        };
        if matches(&self.checks[index], buf, &self.tables) {
            0
        } else {
            0x8a502004
        }
    }
    fn compile(
        &mut self,
        _: &mut Arena<'_>,
        _: [u32; 2],
        re: &[u8],
        opts: u32,
        errpos: &mut i32,
    ) -> u32 {
        self.pending = compile(re, opts, &self.tables, errpos);
        u32::from(self.pending.is_some())
    }
    fn insert_rule(&mut self, _: u32, replacement: &[u8]) -> bool {
        let Some(program) = self.pending.take() else {
            return false;
        };
        let n = replacement
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(replacement.len());
        self.rules.push((program, replacement[..n].to_vec()));
        true
    }
    fn count_rules(&mut self, _: u32) -> u32 {
        self.rules.len() as u32
    }
    fn alloc(&mut self, arena: &mut Arena<'_>, _: u32, size: u32) -> u32 {
        arena.alloc(size)
    }
    fn free(&mut self, _: u32, _: u32) {}
    fn check_value(&mut self, _: [u32; 2], value: &[u8]) -> bool {
        matches(&self.checks[2], value, &self.tables)
    }
    fn logging(&mut self) -> bool {
        false
    }
    fn log(&mut self, _: u32) {}
    fn log_error(&mut self, code: u32) {
        let _ = code;
    }
    fn error(&mut self, _: u32, _: u32) {}
}

fn put_bytes(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    heap: u32,
    bytes: &[u8],
    terminated: bool,
) -> u32 {
    let p = ports::fn_196394(m, h, heap, (bytes.len() + usize::from(terminated)) as u32);
    if p != 0 {
        m.bytes[p as usize..p as usize + bytes.len()].copy_from_slice(bytes);
        if terminated {
            wb(m, p + bytes.len() as u32, 0);
        }
    }
    p
}

pub fn load(m: &mut Memory<'_>, h: &mut impl Calls, a: &[u32]) -> u32 {
    if a.len() < 9 || a[2] == 0 || a[4] == 0 || a[5] == 0 || a[8] == 0 {
        return 0x88602007;
    }
    let rc = tts_core::frontend::validate_handle(a[0] != 0, a[1], 0x10d, 56);
    if rc >= 0x80000000 {
        return rc;
    }
    let scratch = ports::fn_129778(m, h, 64);
    if scratch == 0 {
        return 0x8860200a;
    }
    let rc = ports::fn_19611c(m, rd(m, a[0]), rd(m, a[0] + 4), scratch);
    if rc >= 0x80000000 {
        return 0x88602007;
    }
    let ctx = rd(m, scratch);
    let heap = rd(m, ctx + 4);
    let image = m.bytes[..0xc00000].to_vec();
    let rom = context().0;
    let at = |us: u32| tts_core::rom_region::addr(rom, us) as usize;
    let block = |us: u32, n: usize| &image[at(us)..at(us) + n];
    let escapes = core::array::from_fn(|i| {
        i16::from_le_bytes(block(0x9140e4, 150)[i * 2..i * 2 + 2].try_into().unwrap())
    });
    let limits = core::array::from_fn(|i| {
        i32::from_le_bytes(block(0x9147ac, 24)[i * 4..i * 4 + 4].try_into().unwrap())
    });
    let first = core::array::from_fn(|i| {
        i32::from_le_bytes(block(0x9147c4, 24)[i * 4..i * 4 + 4].try_into().unwrap())
    });
    let mask = core::array::from_fn(|i| {
        u32::from_le_bytes(block(0x9147dc, 24)[i * 4..i * 4 + 4].try_into().unwrap())
    });
    let tables = PcreTables {
        op_lengths: block(0x914092, 256).try_into().unwrap(),
        ctypes: block(0x91426c, 256).try_into().unwrap(),
        default_tables: block(0x91436c, 1088).try_into().unwrap(),
        escapes: &escapes,
        utf8_limits: &limits,
        utf8_first: &first,
        utf8_cont: block(0x9147f4, 64).try_into().unwrap(),
        utf8_mask: &mask,
        rep_min: block(0x914044, 6).try_into().unwrap(),
        rep_max: block(0x91404a, 6).try_into().unwrap(),
        posix_lengths: &[],
        posix_names: &[],
        posix_maps: &[],
        ctypes_overspill: block(0x9147ac, 4096),
    };
    let checks = [0x9178b8, 0x9178f0, 0x917924].map(|us| {
        let p = address(us);
        let n = rd(m, p + 4) as usize;
        m.bytes[p as usize..p as usize + n].to_vec()
    });
    let mut scanner = Scanner {
        context: RuleContext {
            heap,
            log: rd(m, ctx + 16),
            params: rd(m, ctx + 20),
            res: rd(m, ctx + 24),
        },
        tables,
        checks,
        pending: None,
        rules: Vec::new(),
    };
    let data = m.bytes[a[4] as usize..(a[4] + a[5]) as usize].to_vec();
    let mut local = vec![0; 65536];
    let mut arena = Arena::new(&mut local);
    let mut item = RuleItem {
        handle: [rd(m, a[0]), rd(m, a[0] + 4)],
        engine: rd(m, a[0] + 20),
        active: 1,
        ..RuleItem::default()
    };
    let name = if a[3] != 0 && m.bytes[a[3] as usize] != 0 {
        let p = a[3] as usize;
        let n = m.bytes[p..].iter().position(|b| *b == 0).unwrap();
        m.bytes[p..p + n].to_vec()
    } else {
        b"in-memory RETT".to_vec()
    };
    item.name = arena.alloc(name.len() as u32 + 1);
    arena.put(item.name, &name);
    let mut pos = 0;
    let mut rem = data.len() as u32;
    let mut line = [0; 1024];
    let rc = prosody_rules::scan_header(
        &mut arena,
        &mut item,
        [1, 0xdd57],
        &mut pos,
        &mut rem,
        &mut line,
        &data,
        &mut scanner,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let rc = prosody_rules::scan_rules(
        &mut arena,
        &mut item,
        [1, 0xdd57],
        &mut pos,
        &mut rem,
        &mut line,
        &data,
        &mut scanner,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let language = arena.str(item.language).to_vec();
    let kind = if item.kind == 0 {
        None
    } else {
        Some(arena.str(item.kind).to_vec())
    };
    let spec = address(0x917a58);
    if ports::fn_196a64(
        m,
        h,
        heap,
        rd(m, spec),
        rd(m, spec + 4),
        rd(m, spec + 8),
        8,
        scratch,
    ) == 0
    {
        return 0x8860200a;
    }
    item.rules = rd(m, scratch);
    for (program, replacement) in scanner.rules {
        let code = put_bytes(m, h, heap, &program, false);
        let text = put_bytes(m, h, heap, &replacement, true);
        if code == 0 || text == 0 || ports::fn_196cfc(m, h, item.rules, scratch) == 0 {
            return 0x8860200a;
        }
        let slot = rd(m, scratch);
        wr(m, slot, code);
        wr(m, slot + 4, text);
    }
    item.name = put_bytes(m, h, heap, &name, true);
    item.language = put_bytes(m, h, heap, &language, true);
    item.kind = kind.map_or(0, |k| put_bytes(m, h, heap, &k, true));
    let rc = ports::route_call(m, h, 0x23fe84, &[item.handle[0], item.handle[1], scratch]).unwrap();
    if rc >= 0x80000000 {
        return rc;
    }
    item.stream = [rd(m, scratch), rd(m, scratch + 4)];
    struct Host<'a, H>(&'a mut H);
    impl<H: Calls> tts_core::component_live_b::Host for Host<'_, H> {
        fn rom(&self) -> tts_core::rom_region::Rom {
            context().0
        }
        fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
            ports::route_call(m, self.0, f, a).unwrap_or_else(|| self.0.call(m, f, a))
        }
    }
    let rc = tts_core::component_live_b::pcre_init_m(
        m,
        item.stream[0],
        item.stream[1],
        10,
        10,
        &mut Host(h),
    );
    if rc >= 0x80000000 {
        return rc;
    }
    for (i, w) in item.words().into_iter().enumerate() {
        wr(m, scratch + i as u32 * 4, w);
    }
    if ports::fn_196db0(m, h, rd(m, a[0] + 24), scratch) != 1 {
        return 0x8860200a;
    }
    wr(m, a[8], item.stream[0]);
    wr(m, a[8] + 4, 0x12c);
    0
}

fn rd(m: &Memory<'_>, p: u32) -> u32 {
    u32::from_le_bytes(m.bytes[p as usize..p as usize + 4].try_into().unwrap())
}
fn wr(m: &mut Memory<'_>, p: u32, v: u32) {
    m.bytes[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn wb(m: &mut Memory<'_>, p: u32, v: u8) {
    m.bytes[p as usize] = v;
}
