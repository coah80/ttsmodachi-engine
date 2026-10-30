//! Live part-a call tree: fn_22e378, the audiofetch component open.
//!
//! The port lives here rather than in frontend_part_c because that file's
//! PromptServices trait hands its methods no memory. An adapter that holds the
//! live arena cannot be borrowed at the same time as the `&mut [u8]` that
//! open_audio_fetch receives, so that port cannot drive the boot's arena.
//! The trait signatures there belong to another lane and stay untouched.
//!
//! Every callee below is the crate's real Rust port. Every address and every
//! byte belongs to the caller.
use crate::boot::Memory;

/// The live services fn_22e378 needs. The methods take the arena, which is
/// the crate's host convention.
pub trait AudioFetchHooksM {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32;
    fn parser_m(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32;
}

/// The regional data fn_22e378 reads. Every value is the caller's.
pub struct AudioFetchConfig {
    /// fn_19611c's null pair and its default context.
    pub pair: [u8; 8],
    pub default: u32,
    /// The HwCfg22 callback words: the request map's five, then the
    /// speechbase map's five.
    pub callbacks: [[u32; 5]; 2],
    /// The "INET" resource name.
    pub inet: u32,
    /// The log category and the four message formats, in the order the ARM
    /// uses them: entering the open, entering the close, exiting the close,
    /// exiting the open.
    pub category: u32,
    pub formats: [u32; 4],
    /// Caller-owned scratch. The environment word, the INET result word and
    /// the 32 bytes fn_17b1c4 needs follow it.
    pub scratch: u32,
}

/// The heap and resource ports forward to the live dispatcher.
struct Heap<'a, H>(&'a mut H);
impl<H: AudioFetchHooksM> crate::boot_part_e::Host for Heap<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
}

/// The registry ports forward to the live dispatcher.
struct Parser<'a, H>(&'a mut H);
impl<H: AudioFetchHooksM> crate::render_part_c::ParserMethodsM for Parser<'_, H> {
    fn method_m(&mut self, method: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.parser_m(m, method, args)
    }
}

/// fn_1738e8 through the live dispatcher. The dispatcher owns the logger.
fn log(m: &mut Memory<'_>, h: &mut impl AudioFetchHooksM, logger: u32, cfg: &AudioFetchConfig, format: u32) {
    h.call(m, 0x1738e8, &[logger, cfg.category, 4, 0, format, 0]);
}

/// fn_173dc0 through the live dispatcher. The ARM passes a null format here.
fn error(m: &mut Memory<'_>, h: &mut impl AudioFetchHooksM, logger: u32, cfg: &AudioFetchConfig, code: u32) {
    h.call(m, 0x173dc0, &[logger, cfg.category, code, 0]);
}

/// fn_22e378. The audiofetch component open.
pub fn fn_22e378(
    m: &mut Memory<'_>,
    component: u32,
    tag: u32,
    session: [u32; 2],
    out: u32,
    cfg: &AudioFetchConfig,
    h: &mut impl AudioFetchHooksM,
) -> u32 {
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    let env_at = cfg.scratch;
    let saved_env = m.word(env_at);
    let mut rc = crate::boot_port_1::fn_19611c(m, session, env_at, cfg.pair, cfg.default);
    if rc >= 0x80000000 {
        m.put_word(env_at, saved_env);
        return rc;
    }
    let env = m.word(env_at);
    rc = crate::boot_port_1::fn_176f94(m, component, tag, 0x2bd, 32);
    if rc >= 0x80000000 {
        m.put_word(env_at, saved_env);
        return 0x87402008;
    }
    let logger = m.word(env + 16);
    log(m, h, logger, cfg, cfg.formats[0]);
    let heap = m.word(env + 4);
    let object = crate::boot_part_e::calloc(m, heap, 1, 44, &mut Heap(h));
    if object == 0 {
        error(m, h, logger, cfg, 100);
        m.put_word(env_at, saved_env);
        log(m, h, logger, cfg, cfg.formats[3]);
        return 0x8740200a;
    }
    m.put_word(object, session[0]);
    m.put_word(object + 4, session[1]);
    m.put_word(object + 8, env);
    m.put_word(object + 12, component);
    m.put_word(object + 16, 1);
    m.bytes[object as usize + 28] = 1;
    if m.word(object + 32) == 0 {
        let res_at = cfg.scratch + 4;
        let saved_res = m.word(res_at);
        let found = crate::render_part_c::fn_17b058_m(m, m.word(env + 24), cfg.inet, res_at, &mut Parser(h));
        let res = m.word(res_at);
        if found < 0x80000000 && res != 0 {
            let interface = m.word(res + 4);
            if interface != 0 && m.bytes[res as usize] == 1 {
                m.put_word(object + 32, interface);
                m.put_word(object + 36, m.word(res + 8));
                m.put_word(object + 40, m.word(res + 12));
            } else {
                error(m, h, logger, cfg, 129);
                release_inet(m, h, m.word(env + 24), cfg);
            }
            m.bytes[object as usize + 28] = 0;
        }
        m.put_word(res_at, saved_res);
    }
    rc = crate::boot_port_2::fn_1964e0(m, heap, 0, cfg.callbacks[0], object + 20, &mut Heap(h));
    if rc < 0x80000000 {
        rc = crate::boot_port_2::fn_1964e0(m, heap, 0, cfg.callbacks[1], object + 24, &mut Heap(h));
        if rc < 0x80000000 {
            m.put_word(out, object);
            m.put_word(out + 4, 0x2be);
            m.put_word(env_at, saved_env);
            log(m, h, logger, cfg, cfg.formats[3]);
            return rc;
        }
    }
    error(m, h, logger, cfg, 130);
    m.put_word(out, object);
    m.put_word(out + 4, 0x2be);
    let close = crate::boot_port_1::fn_176f94(m, object, tag, tag, 44);
    if close < 0x80000000 {
        log(m, h, logger, cfg, cfg.formats[1]);
        if m.word(object + 24) != 0 {
            h.call(m, 0x19665c, &[m.word(object + 24)]);
        }
        if m.word(object + 20) != 0 {
            h.call(m, 0x19665c, &[m.word(object + 20)]);
        }
        if m.word(object + 32) != 0 {
            release_inet(m, h, m.word(env + 24), cfg);
        }
        log(m, h, logger, cfg, cfg.formats[2]);
        crate::boot_part_e::release(m, heap, object, &mut Heap(h));
    }
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    m.put_word(env_at, saved_env);
    log(m, h, logger, cfg, cfg.formats[3]);
    rc
}

/// fn_17b1c4. The "INET" release.
fn release_inet(m: &mut Memory<'_>, h: &mut impl AudioFetchHooksM, registry: u32, cfg: &AudioFetchConfig) {
    crate::boot_port_2c::fn_17b1c4(m, registry, cfg.inet, cfg.scratch + 8, &mut Parser(h));
}

/// The regional data the domain manager open reads. Every value is the
/// caller's.
pub struct DomainConfig {
    /// fn_19611c's null pair and its default context.
    pub pair: [u8; 8],
    pub default: u32,
    /// The Cfg table: the first list's three callback words at +0xb4, the
    /// map's five at +0xc0 and the second list's three at +0xd4.
    pub cfg: u32,
    /// The resource names by the domain label: LINGDB, SYNTHSTREAM, two
    /// unused labels, AUDIOFETCHER and INET.
    pub names: [u32; 6],
    /// The log category.
    pub category: u32,
    /// Caller-owned scratch: the environment word, the block out word and
    /// the resource out word.
    pub scratch: u32,
}

/// The reentrant hooks the list teardown needs.
struct Reentrant<'a, H>(&'a mut H);
impl<H: AudioFetchHooksM> crate::reentrant2::HooksM for Reentrant<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.call(m, f, a)
    }
}

/// fn_22e378's log and error sinks, with the domain manager's category.
fn domain_log(m: &mut Memory<'_>, h: &mut impl AudioFetchHooksM, logger: u32, cfg: &DomainConfig) {
    h.call(m, 0x1738e8, &[logger, cfg.category, 4, 0, 0, 0]);
}

fn domain_error(m: &mut Memory<'_>, h: &mut impl AudioFetchHooksM, logger: u32, cfg: &DomainConfig, code: u32) {
    h.call(m, 0x173dc0, &[logger, cfg.category, code, 0]);
}

/// fn_1fd9f8. The domain manager open.
pub fn fn_1fd9f8(
    m: &mut Memory<'_>,
    _component: u32,
    _tag: u32,
    mgr: [u32; 2],
    out: u32,
    cfg: &DomainConfig,
    h: &mut impl AudioFetchHooksM,
) -> u32 {
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    let env_at = cfg.scratch;
    let saved_env = m.word(env_at);
    let mut status = crate::boot_port_1::fn_19611c(m, mgr, env_at, cfg.pair, cfg.default);
    if status >= 0x80000000 {
        m.put_word(env_at, saved_env);
        return status;
    }
    let env = m.word(env_at);
    let heap = m.word(env + 4);
    let logger = m.word(env + 16);
    let registry = m.word(env + 24);
    let n = crate::boot_part_e::calloc(m, heap, 1, 72, &mut Heap(h));
    if n == 0 {
        domain_error(m, h, logger, cfg, 0xb798);
        m.put_word(env_at, saved_env);
        return 0x88e0200a;
    }
    m.put_word(out + 4, 0x11d);
    m.put_word(out, n);
    m.put_word(n, mgr[0]);
    m.put_word(n + 4, mgr[1]);
    m.put_word(n + 8, env);
    m.bytes[n as usize + 48] = 1;
    m.bytes[n as usize + 68] = 0;
    let block_at = cfg.scratch + 4;
    let mut failed = false;
    let list1 = cfg.cfg + 0xb4;
    let rc = h.call(m, 0x196a64, &[heap, m.word(list1), m.word(list1 + 4), m.word(list1 + 8), 1, block_at]);
    m.put_word(n + 44, m.word(block_at));
    if rc == 0 {
        domain_error(m, h, logger, cfg, 0xb798);
        status = 0x88e0200a;
        failed = true;
    }
    if !failed {
        let map = cfg.cfg + 0xc0;
        let callbacks = [
            m.word(map),
            m.word(map + 4),
            m.word(map + 8),
            m.word(map + 12),
            m.word(map + 16),
        ];
        status = crate::boot_port_2::fn_1964e0(m, heap, 0, callbacks, block_at, &mut Heap(h));
        m.put_word(n + 60, m.word(block_at));
        if status >= 0x80000000 {
            domain_error(m, h, logger, cfg, 0xb798);
            failed = true;
        }
    }
    if !failed {
        let list2 = cfg.cfg + 0xd4;
        let rc = h.call(m, 0x196a64, &[heap, m.word(list2), m.word(list2 + 4), m.word(list2 + 8), 1, block_at]);
        m.put_word(n + 64, m.word(block_at));
        if rc == 0 {
            domain_error(m, h, logger, cfg, 0xb798);
            status = 0x88e0200a;
            failed = true;
        }
    }
    if !failed {
        for (label, offset, required) in [(1usize, 12u32, true), (0, 16, true), (5, 20, false), (4, 32, false)] {
            let res_at = cfg.scratch + 8;
            let found = crate::render_part_c::fn_17b058_m(m, registry, cfg.names[label], res_at, &mut Parser(h));
            let res = m.word(res_at);
            if required {
                status = found;
                if found >= 0x80000000 {
                    failed = true;
                    break;
                }
            }
            if found < 0x80000000 {
                m.put_word(n + offset, m.word(res + 4));
                if !required {
                    m.put_word(n + offset + 4, m.word(res + 8));
                    m.put_word(n + offset + 8, m.word(res + 12));
                }
            }
        }
    }
    if failed {
        close_domain(m, h, n, cfg);
        m.put_word(out, 0);
        m.put_word(out + 4, 0);
    }
    m.put_word(env_at, saved_env);
    status
}

/// fn_1fd854. The domain manager close.
fn close_domain(m: &mut Memory<'_>, h: &mut impl AudioFetchHooksM, p: u32, cfg: &DomainConfig) -> u32 {
    let status = crate::boot_port_1::fn_176f94(m, p, 0x11d, 0x11d, 0x48);
    if status >= 0x80000000 {
        return status;
    }
    let env_at = cfg.scratch + 12;
    let saved = m.word(env_at);
    let rc = crate::boot_port_1::fn_19611c(m, [m.word(p), m.word(p + 4)], env_at, cfg.pair, cfg.default);
    if rc >= 0x80000000 {
        m.put_word(env_at, saved);
        return rc;
    }
    let env = m.word(env_at);
    let registry = m.word(env + 24);
    for (at, label) in [(32u32, 4usize), (20, 5), (16, 0), (12, 1)] {
        if m.word(p + at) != 0 {
            crate::boot_port_2c::fn_17b1c4(m, registry, cfg.names[label], cfg.scratch + 16, &mut Parser(h));
        }
    }
    if m.word(p + 64) != 0 {
        crate::reentrant2::destroy_array_m(m, m.word(p + 64), &mut Reentrant(h));
    }
    if m.word(p + 60) != 0 {
        h.call(m, 0x19665c, &[m.word(p + 60)]);
    }
    if m.word(p + 44) != 0 {
        crate::reentrant2::destroy_array_m(m, m.word(p + 44), &mut Reentrant(h));
    }
    crate::boot_part_e::release(m, m.word(env + 4), p, &mut Heap(h));
    m.put_word(env_at, saved);
    status
}


// ---------------------------------------------------------------------------
// fn_1ec070: the fe_dctlkp component open, live.
//
// The boot-port-2b port owns the fe_dctlkp chain. Its two seams are the
// engine services (the context, the registry, the parameters) and the
// dictionary service (fn_26b5f0, fn_26b7ec, fn_26bb6c). This section binds
// both to the live boot: the host runs the crate's real ports over the
// caller's arena, and the dictionary runs the real broker, the real file
// read and the real trie walk.
//
// The port carries the US address of the FE_DCTLKP log category. The live
// host relocates it to the region's own address, the way the region relocates
// every other literal.

/// The US address of the FE_DCTLKP log category the port carries.
pub const DCTLKP_CATEGORY_US: u32 = 0x8f8f44;

/// The regional strings the fe_dctlkp open reads. Every value is a guest
/// address in the region's own image.
#[derive(Clone, Copy)]
pub struct DctlkpStrings {
    /// "LINGDB": the linguistic registry the open takes.
    pub lingdb: u32,
    /// "sysdct": the system dictionary.
    pub sysdct: u32,
    /// "lid": the language dictionary.
    pub lid: u32,
    /// "punc": the entry the first lookup asks for.
    pub punc: u32,
    /// "fecfg": the section the first lookup asks for.
    pub fecfg: u32,
    /// "langcode": the language parameter.
    pub langcode: u32,
    /// "fecfg": the configuration directory parameter.
    pub fecfg_dir: u32,
    /// "fedataprefix": the data prefix parameter.
    pub fedataprefix: u32,
    /// "fedatapackaging": the packaging parameter.
    pub fedatapackaging: u32,
    /// "clc": the packaging name the path compares against.
    pub clc: u32,
    /// "clc/": the packaging prefix the path appends.
    pub clc_slash: u32,
    /// "/": the path separator.
    pub slash: u32,
    /// ":": the separator the packaging path uses.
    pub colon: u32,
    /// "sysdct/": the system dictionary prefix.
    pub sysdct_slash: u32,
}

/// The regional data the fe_dctlkp open needs. Every value is the caller's.
pub struct DctlkpConfig {
    /// fn_19611c's null pair and its default context.
    pub pair: [u8; 8],
    pub default: u32,
    /// The FE_DCTLKP log category, in the region's own image.
    pub category: u32,
    /// The regional strings.
    pub strings: DctlkpStrings,
    /// The broker's regional constants.
    pub broker: crate::boot_port_2c::BrokerGlobals,
    /// The broker's allocator addresses, in the region's own image.
    pub allocators: [u32; 4],
    /// The caller's scratch. The port's 720 byte frame comes first.
    pub scratch: u32,
    /// The unmap environment the file release uses.
    pub unmap: crate::boot_part_e::UnmapEnvironment,
}

/// The offset of the adapter's name buffer, past the port's frame.
const NAME_BUFFER: u32 = 736;
/// The offset of the lookup's half buffer.
const HALF_BUFFER: u32 = 992;
/// The offset of the broker open's scratch.
const OPEN_SCRATCH: u32 = 1000;
/// The offset of the broker query's scratch.
const QUERY_SCRATCH: u32 = 1024;
/// The offset of the guest words the services write their results into.
const OUT_WORDS: u32 = 1040;
/// The offset of the guest word fn_17a1a4 writes the window address into.
const WINDOW_WORD: u32 = 1044;
/// The size the caller has to provide.
pub const DCTLKP_SCRATCH_SIZE: u32 = 1088;

impl DctlkpConfig {
    /// The config strings the port's path builder reads.
    pub fn config_strings(&self) -> crate::boot_port_2::ConfigStrings {
        crate::boot_port_2::ConfigStrings {
            langcode: self.strings.langcode,
            voice: 0,
            fecfg: self.strings.fecfg_dir,
            prefix: self.strings.fedataprefix,
            packaging: self.strings.fedatapackaging,
            clc: self.strings.clc,
            clc_slash: self.strings.clc_slash,
            slash: self.strings.slash,
            colon: self.strings.colon,
            sysdct_slash: self.strings.sysdct_slash,
        }
    }
}

/// The live fe_dctlkp open. The host is the caller's dispatcher.
///
/// The dispatcher and the trie tables live behind cells, so the engine
/// services and the dictionary service can hold the same open at once, the
/// way the port's two seams do.
pub struct Dctlkp<'a, H> {
    pub host: core::cell::RefCell<&'a mut H>,
    pub cfg: &'a DctlkpConfig,
    pub workspace: core::cell::RefCell<crate::component_live_c::DictionaryWorkspace>,
}

impl<'a, H: crate::boot::Calls> Dctlkp<'a, H> {
    /// The live open. The caller's dispatcher and the regional data.
    pub fn new(
        host: &'a mut H,
        cfg: &'a DctlkpConfig,
        workspace: crate::component_live_c::DictionaryWorkspace,
    ) -> Self {
        Dctlkp {
            host: core::cell::RefCell::new(host),
            cfg,
            workspace: core::cell::RefCell::new(workspace),
        }
    }

    /// fn_1ec070. The fe_dctlkp component open.
    pub fn open(
        &self,
        m: &mut Memory<'_>,
        a0: u32,
        a1: u32,
        h0: u32,
        h1: u32,
        slot: u32,
    ) -> u32 {
        let scratch = crate::boot_port_2b::Scratch {
            base: self.cfg.scratch,
        };
        // The port reads the dictionary names by buffer. The ROM's fn_1ec070
        // frame carries these literals (src/fn_1ea4e0_merged.c:378 "fecfg",
        // "domdef"; :877 "fecfg", "punc"; :896 "fecfg", "canonmapdomain";
        // :904/:907 "normal"); the live caller owns the scratch, so the
        // binding writes them the way it already wrote sysdct/lid.
        for (at, name) in [
            (scratch.sysdct(), &b"sysdct\0"[..]),
            (scratch.lid(), &b"lid\0"[..]),
            (scratch.fecfg(), &b"fecfg\0"[..]),
            (scratch.domdef(), &b"domdef\0"[..]),
            (scratch.punc(), &b"punc\0"[..]),
            (scratch.canonmapdomain(), &b"canonmapdomain\0"[..]),
            (scratch.normal(), &b"normal\0"[..]),
        ] {
            let o = m.offset(at);
            m.bytes[o..o + name.len()].copy_from_slice(name);
        }
        let k = self.cfg.config_strings();
        let mut host = Host(self);
        let mut dictionary = Dictionary(self);
        crate::boot_port_2b::fn_1ec070(
            m,
            a0,
            a1,
            h0,
            h1,
            slot,
            &scratch,
            &k,
            &mut host,
            &mut dictionary,
        )
    }

    /// The dispatcher, with the port's US log category relocated.
    fn dispatch(&self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        let mut guard = self.host.borrow_mut();
        let host = &mut **guard;
        if a.len() >= 2 && a[1] == DCTLKP_CATEGORY_US {
            let mut args = [0u32; 8];
            let n = a.len().min(8);
            args[..n].copy_from_slice(&a[..n]);
            args[1] = self.cfg.category;
            return host.call(m, f, &args[..n]);
        }
        host.call(m, f, a)
    }

    /// A guest copy of a name in the caller's scratch.
    fn name_at(&self, m: &mut Memory<'_>, name: &[u8]) -> u32 {
        let at = self.cfg.scratch + NAME_BUFFER;
        let o = m.offset(at);
        let n = name.len().min(255);
        m.bytes[o..o + n].copy_from_slice(&name[..n]);
        m.bytes[o + n] = 0;
        at
    }

    /// The broker's open, fn_1795f0 with the region's flags.
    ///
    /// Round 25: public so the voice harness can broker-open depes
    /// resource files through the same proven path (additive visibility
    /// only; no behaviour change).
    pub fn broker_open(&self, m: &mut Memory<'_>, broker: u32, name: u32, flags: u32, out: u32) -> u32 {
        let mut globals = self.cfg.broker.data_open(self.cfg.scratch + OPEN_SCRATCH);
        globals.allocators = self.cfg.allocators;
        let mut host = BrokerHost(self);
        crate::boot_part_b::fn_1795f0(m, broker, name, flags, out, &globals, &mut host)
    }

    /// fn_17a1a4. Read a window of a mapped file.
    fn read(
        &self,
        m: &mut Memory<'_>,
        hdata: u32,
        handle: u32,
        offset: u32,
        size: u32,
        out: u32,
    ) -> u32 {
        let mut host = BrokerHost(self);
        crate::boot_port_1::fn_17a1a4(
            m,
            hdata,
            handle,
            offset,
            size,
            out,
            self.cfg.scratch + OUT_WORDS + 8,
            &self.cfg.unmap,
            &mut host,
        )
    }

    /// fn_179208. Release a mapped window.
    fn release(&self, m: &mut Memory<'_>, hdata: u32, handle: u32, buf: u32) -> u32 {
        let mut host = BrokerHost(self);
        crate::boot_part_e::fn_179208(m, hdata, handle, buf, &self.cfg.unmap, &mut host)
    }

    /// fn_179f3c. Close a mapped file.
    fn close(&self, m: &mut Memory<'_>, resources: u32, handle: u32) -> u32 {
        self.dispatch(m, 0x179f3c, &[resources, handle])
    }
}

/// The engine services the fe_dctlkp chain needs.
struct Host<'a, 'b, H>(&'a Dctlkp<'b, H>);

impl<H: crate::boot::Calls> crate::boot_part_e::Host for Host<'_, '_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.dispatch(m, f, a)
    }
}

impl<H: crate::boot::Calls> crate::boot_port_2b::Host for Host<'_, '_, H> {
    /// fn_19611c. The engine context of a handle pair.
    fn context(&mut self, m: &mut Memory<'_>, h0: u32, h1: u32, out: &mut u32) -> u32 {
        let cfg = self.0.cfg;
        // The context service writes the context pointer into a guest word;
        // the port reads the pointer itself.
        let slot = self.0.cfg.scratch + OUT_WORDS;
        let rc = crate::boot_port_1::fn_19611c(m, [h0, h1], slot, cfg.pair, cfg.default);
        *out = m.word(slot);
        rc
    }

    /// fn_179f30. The broker's open of a named resource.
    fn resource_open(
        &mut self,
        m: &mut Memory<'_>,
        resources: u32,
        name: &[u8],
        out: &mut u32,
    ) -> u32 {
        let at = self.0.name_at(m, name);
        // The broker writes the handle into a guest word; the port reads
        // the handle itself.
        let slot = self.0.cfg.scratch + OUT_WORDS;
        let rc = self.0.broker_open(m, resources, at, 0, slot);
        *out = m.word(slot);
        rc
    }

    /// fn_17b058. The registry's open of a named resource.
    fn open_named(
        &mut self,
        m: &mut Memory<'_>,
        resources: u32,
        name: &[u8],
        out: &mut u32,
    ) -> u32 {
        let at = self.0.name_at(m, name);
        // The registry writes the slot address into a guest word; the port
        // reads the address itself.
        let slot = self.0.cfg.scratch + OUT_WORDS;
        let mut host = ParserHost(self.0);
        let rc = crate::render_part_c::fn_17b058_m(m, resources, at, slot, &mut host);
        *out = m.word(slot);
        rc
    }

    /// fn_17b1c4. The registry's release of a named resource.
    fn resource_close(&mut self, m: &mut Memory<'_>, resources: u32, name: &[u8]) {
        let at = self.0.name_at(m, name);
        let scratch = self.0.cfg.scratch + OPEN_SCRATCH;
        let mut host = ParserHost(self.0);
        crate::boot_port_2c::fn_17b1c4(m, resources, at, scratch, &mut host);
    }

    /// fn_175a1c. A named parameter.
    fn parameter(&mut self, m: &mut Memory<'_>, params: u32, name: &[u8], out: &mut u32) -> u32 {
        let at = self.0.name_at(m, name);
        // The parameter service writes the value pointer into a guest word.
        let slot = self.0.cfg.scratch + OUT_WORDS;
        let rc = self.0.dispatch(m, 0x175a1c, &[params, at, slot]);
        *out = m.word(slot);
        rc
    }

    /// fn_175944. A named integer setting.
    fn setting(&mut self, m: &mut Memory<'_>, settings: u32, name: &[u8]) -> u32 {
        let at = self.0.name_at(m, name);
        let slot = self.0.cfg.scratch + OUT_WORDS;
        let status = self.0.dispatch(m, 0x175944, &[settings, at, slot]);
        if status >= 0x80000000 {
            status
        } else {
            m.word(slot)
        }
    }

    /// fn_175944. A named integer setting, with its value.
    fn setting_value(
        &mut self,
        m: &mut Memory<'_>,
        settings: u32,
        name: &[u8],
        out: &mut u32,
    ) -> u32 {
        let at = self.0.name_at(m, name);
        let slot = self.0.cfg.scratch + OUT_WORDS;
        let rc = self.0.dispatch(m, 0x175944, &[settings, at, slot]);
        *out = m.word(slot);
        rc
    }
}

/// The registry's parser methods reach the live dispatcher.
struct ParserHost<'a, 'b, H>(&'a Dctlkp<'b, H>);

impl<H: crate::boot::Calls> crate::render_part_c::ParserMethodsM for ParserHost<'_, '_, H> {
    fn method_m(&mut self, method: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.dispatch(m, method, args)
    }
}

/// The broker's host services.
struct BrokerHost<'a, 'b, H>(&'a Dctlkp<'b, H>);

impl<H: crate::boot::Calls> crate::boot_part_e::Host for BrokerHost<'_, '_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.dispatch(m, f, a)
    }
}

impl<H: crate::boot::Calls> crate::boot_part_b::Host for BrokerHost<'_, '_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.dispatch(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0
    }
}

impl<H: crate::boot::Calls> crate::boot_part_b::OutputHost for BrokerHost<'_, '_, H> {
    fn output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        let out = self.0.cfg.scratch + OUT_WORDS + 4;
        let mut args = [0u32; 8];
        let n = a.len().min(7);
        args[..n].copy_from_slice(&a[..n]);
        args[n] = out;
        let rc = self.0.dispatch(m, f, &args[..n + 1]);
        (rc, m.word(out))
    }
}

impl<H: crate::boot::Calls> crate::boot_part_b::PropertyHost for BrokerHost<'_, '_, H> {
    fn property(&mut self, m: &mut Memory<'_>, dictionary: u32, key: &[u8]) -> (u32, u32) {
        let at = self.0.name_at(m, key);
        // The map find writes the value into a guest word; the port reads
        // the value's slot, so the slot's address is the result.
        let out = self.0.cfg.scratch + OUT_WORDS;
        let rc = self.0.dispatch(m, 0x1966ac, &[dictionary, at, out]);
        (rc, out)
    }
    fn insert(
        &mut self,
        m: &mut Memory<'_>,
        dictionary: u32,
        key: u32,
        entry: [u32; 3],
    ) -> u32 {
        let at = self.0.cfg.scratch + OUT_WORDS;
        for (i, v) in entry.iter().enumerate() {
            m.put_word(at + i as u32 * 4, *v);
        }
        self.0.dispatch(m, 0x19668c, &[dictionary, key, at])
    }
}

impl<H: crate::boot::Calls> crate::boot_part_b::QueryHost for BrokerHost<'_, '_, H> {
    fn query_output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        let out = self.0.cfg.scratch + OUT_WORDS;
        if f == 0x1772b0 {
            let mut query = BrokerQuery(self.0);
            let scratch = self.0.cfg.scratch + QUERY_SCRATCH;
            let rc = crate::boot_port_2c::fn_1772b0(m, a[0], a[1], out, scratch, &mut query);
            return (rc, m.word(out));
        }
        let mut args = [0u32; 8];
        let n = a.len().min(7);
        args[..n].copy_from_slice(&a[..n]);
        args[n] = out;
        let rc = self.0.dispatch(m, f, &args[..n + 1]);
        (rc, m.word(out))
    }
    fn insert_record(
        &mut self,
        m: &mut Memory<'_>,
        db: u32,
        key: u32,
        record: [u32; 4],
    ) -> u32 {
        let at = self.0.cfg.scratch + OUT_WORDS;
        for (i, v) in record.iter().enumerate() {
            m.put_word(at + i as u32 * 4, *v);
        }
        self.0.dispatch(m, 0x19668c, &[db, key, at])
    }
    fn open_pair(&mut self, m: &mut Memory<'_>, f: u32, owner: [u32; 2], out: u32) -> u32 {
        self.0.dispatch(m, f, &[owner[0], owner[1], out])
    }
}

impl<H: crate::boot::Calls> crate::boot_part_b::DataHost for BrokerHost<'_, '_, H> {
    fn data_output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32], out: u32) -> u32 {
        let mut args = [0u32; 8];
        let n = a.len().min(7);
        args[..n].copy_from_slice(&a[..n]);
        args[n] = out;
        self.0.dispatch(m, f, &args[..n + 1])
    }
}

/// The broker database's query services.
struct BrokerQuery<'a, 'b, H>(&'a Dctlkp<'b, H>);

impl<H: crate::boot::Calls> crate::boot_port_2c::BrokerQuery for BrokerQuery<'_, '_, H> {
    fn db_lock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32 {
        self.0.dispatch(m, 0x176f2c, &[lock])
    }
    fn db_unlock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32 {
        self.0.dispatch(m, 0x176f5c, &[lock])
    }
    fn db_open(
        &mut self,
        m: &mut Memory<'_>,
        database: u32,
        name: u32,
        mode: u32,
        out: u32,
    ) -> u32 {
        self.0.dispatch(m, 0x196794, &[database, name, mode, out])
    }
    fn db_next(&mut self, m: &mut Memory<'_>, iterator: u32, key: u32, value: u32) -> u32 {
        self.0.dispatch(m, 0x196860, &[iterator, key, value])
    }
    fn db_close(&mut self, m: &mut Memory<'_>, iterator: u32) -> u32 {
        self.0.dispatch(m, 0x196830, &[iterator])
    }
    fn db_compare(&mut self, m: &Memory<'_>, a: u32, b: u32) -> i32 {
        crate::fe_text::strcmp_ci_1769d0(m.bytes, a, b) as i32
    }
}

/// The dictionary service: fn_26b5f0, fn_26b7ec and fn_26bb6c.
///
/// Round 31: public so the live voice harness can drive fn_1ea4e0's
/// dictionary loads through the real broker files (the 1ea4e0 lookup
/// path needs it; the image-backed test double cannot serve real files).
/// The wrapped open is borrowed per call; the harness must not hold the
/// dictionary across calls that reborrow its host.
pub struct Dictionary<'a, 'b, H>(pub &'a Dctlkp<'b, H>);

impl<H: crate::boot::Calls> crate::boot_port_2b::Dictionaries for Dictionary<'_, '_, H> {
    /// fn_26b5f0. Open a dictionary resource and report its payload size.
    fn open(&mut self, m: &mut Memory<'_>, st: u32, name: u32, size: &mut u32) -> u32 {
        let owner = m.word(st + crate::boot_port_2b::state::H0);
        // The broker writes the handle into a guest word; the port reads the
        // handle itself.
        let handle_at = self.0.cfg.scratch + OUT_WORDS;
        let rc = self.0.broker_open(m, owner, name, 1, handle_at);
        let handle = m.word(handle_at);
        if rc != 0 {
            m.put_word(st + crate::boot_port_2b::state::H1, 0);
            return rc;
        }
        // fn_17a1a4 writes the window address into the caller's word; the
        // window is the header the ROM reads.
        let window_at = self.0.cfg.scratch + WINDOW_WORD;
        let rc = self.0.read(m, owner, handle, 0, 40, window_at);
        if rc >= 0x80000000 {
            self.0.close(m, owner, handle);
            m.put_word(st + crate::boot_port_2b::state::H1, 0);
            return rc;
        }
        let buf = m.word(window_at);
        let tag = m.word(buf);
        let mut vendor = [0u8; 8];
        for (i, slot) in vendor.iter_mut().enumerate() {
            *slot = m.byte(buf + 4 + i as u32);
        }
        let mut kind = [0u8; 7];
        for (i, slot) in kind.iter_mut().enumerate() {
            *slot = m.byte(buf + 12 + i as u32);
        }
        if tag != 0xfeff || &vendor != b"SCANSOFT" || &kind != b"SYS_DCT" {
            return 0x89802019;
        }
        let length = m.word(buf + 20);
        self.0.release(m, owner, handle, buf);
        *size = length.wrapping_sub(40);
        m.put_word(st + crate::boot_port_2b::state::H1, handle);
        0
    }

    /// fn_26b7ec. Load the section tables for `key` into the state block.
    fn load(&mut self, m: &mut Memory<'_>, st: u32, key: u32, size: u32, resource: u32) -> u32 {
        let owner = m.word(st + crate::boot_port_2b::state::H0);
        let handle = m.word(st + crate::boot_port_2b::state::H1);
        let tables = m.word(st + crate::boot_port_2b::state::TABLES);
        let window_at = self.0.cfg.scratch + WINDOW_WORD;
        let rc = self.0.read(m, owner, handle, 0x28, size, window_at);
        if rc >= 0x80000000 {
            self.0.close(m, owner, handle);
            m.put_word(st + crate::boot_port_2b::state::H1, 0);
            return rc;
        }
        let buf = m.word(window_at);
        // The section index: a name, its length, then the offset word.
        let mut pos = 0u32;
        while pos < size {
            let name = buf + pos;
            let equal =
                crate::fe_text::strcmp_ci_1769d0(m.bytes, name - m.base, key - m.base) == 0;
            if equal {
                break;
            }
            let mut next =
                crate::fe_text::strlen_195d90(m.bytes, name - m.base) as u32 + pos + 1;
            while next & 3 != 0 {
                next += 1;
            }
            pos = next + 4;
        }
        if pos >= size {
            self.0.release(m, owner, handle, buf);
            return 0x89802014;
        }
        let name = buf + pos;
        let mut next = crate::fe_text::strlen_195d90(m.bytes, name - m.base) as u32 + pos + 1;
        while next & 3 != 0 {
            next += 1;
        }
        let offset = m.word(buf + next);
        self.0.release(m, owner, handle, buf);
        let rc = self.0.read(m, owner, handle, offset, 4, window_at);
        if rc >= 0x80000000 {
            self.0.close(m, owner, handle);
            m.put_word(st + crate::boot_port_2b::state::H1, 0);
            return rc;
        }
        let head = m.word(window_at);
        let length = m.word(head);
        self.0.release(m, owner, handle, head);
        let rc = self.0.read(m, owner, resource, offset, length, tables);
        if rc >= 0x80000000 {
            return rc;
        }
        // The nineteen offsets the section loader writes.
        let section = m.word(tables);
        let first = m.byte(section + 4);
        let last = m.byte(section + 5);
        let first6 = m.byte(section + 6);
        let last6 = m.byte(section + 7);
        let count1a = m.half(section + 0x1a);
        let count1c = m.half(section + 0x1c);
        let count1e = m.half(section + 0x1e);
        let count20 = m.half(section + 0x20);
        let count22 = m.half(section + 0x22);
        let count24 = m.half(section + 0x24);
        let count26 = m.half(section + 0x26);
        let count28 = m.half(section + 0x28);
        let count2a = m.half(section + 0x2a);
        let count2e = m.half(section + 0x2e);
        let size3c = m.word(section + 0x3c);
        let size40 = m.word(section + 0x40);
        let end = length.wrapping_add(offset);
        let mut cursor = section.wrapping_add(0x48);
        let mut offsets = [0u32; 19];
        offsets[0] = section;
        offsets[1] = cursor;
        cursor = cursor.wrapping_add(
            (u32::from(last).wrapping_sub(u32::from(first)).wrapping_add(4)) & !3,
        );
        offsets[2] = cursor;
        cursor = cursor.wrapping_add(
            (u32::from(last6).wrapping_sub(u32::from(first6)).wrapping_mul(2).wrapping_add(5)) & !3,
        );
        offsets[3] = cursor;
        cursor = cursor.wrapping_add((u32::from(count2e).wrapping_add(3)) & !3);
        offsets[4] = cursor;
        cursor = cursor.wrapping_add(u32::from(count2e) * 4);
        offsets[5] = cursor;
        cursor = cursor.wrapping_add(u32::from(count2e) * 4);
        offsets[6] = cursor;
        cursor = cursor.wrapping_add(u32::from(count1a) * 4 + 4);
        offsets[7] = cursor;
        cursor = cursor.wrapping_add(u32::from(count1c) * 4 + 4);
        offsets[8] = cursor;
        cursor = cursor.wrapping_add(u32::from(count1e) * 4 + 4);
        offsets[9] = cursor;
        cursor = cursor.wrapping_add(u32::from(count20) * 4 + 4);
        offsets[10] = cursor;
        cursor = cursor.wrapping_add(u32::from(count22) * 4 + 4);
        offsets[11] = cursor;
        cursor = cursor.wrapping_add((u32::from(count24) * 2 + 3) & !3);
        offsets[12] = cursor;
        cursor = cursor.wrapping_add((u32::from(count26) + 3) & !3);
        offsets[13] = cursor;
        cursor = cursor.wrapping_add((u32::from(count28) + 3) & !3);
        offsets[14] = cursor;
        cursor = cursor.wrapping_add((u32::from(count2a) * 2 + 3) & !3);
        offsets[15] = cursor;
        offsets[16] = end;
        let mut tail = end.wrapping_add((size3c + 3) & !3);
        offsets[17] = tail;
        tail = tail.wrapping_add((size40 + 3) & !3);
        offsets[18] = tail;
        for (index, value) in offsets.iter().enumerate() {
            m.put_word(tables + index as u32 * 4, *value);
        }
        0
    }

    /// fn_26bb6c. The byte pair search.
    fn lookup(
        &mut self,
        m: &mut Memory<'_>,
        st: u32,
        text: u32,
        start: &mut i16,
        end: &mut i16,
        fold: i32,
        length: &mut i16,
    ) -> i32 {
        let half = self.0.cfg.scratch + HALF_BUFFER;
        m.put_half(half, *start as u16);
        m.put_half(half + 2, *end as u16);
        m.put_half(half + 4, *length as u16);
        let mut workspace = self.0.workspace.borrow_mut();
        let mut host = DictHost(self.0);
        let result = crate::component_live_c::dictionary_lookup_entry_m(
            m,
            [st, text, half, half + 2, fold as u32, half + 4],
            &mut workspace,
            &mut host,
        );
        *length = m.half(half + 4) as i16;
        match result {
            Ok(found) => found as i32,
            Err(_) => 0,
        }
    }
}

/// The trie walk's services reach the live dispatcher.
struct DictHost<'a, 'b, H>(&'a Dctlkp<'b, H>);

impl<H: crate::boot::Calls> crate::component_live_c::Host for DictHost<'_, '_, H> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.dispatch(m, entry, args)
    }
}

// ---------------------------------------------------------------------------
// fn_19869c: the map's tree destroy.
//
// The broker's database insert creates a broker string entry, and the
// registry releases it when the map is destroyed. The boot-port-2 and
// boot-port-5 lanes each hold a private copy of this walk; the live boot
// needs its own, so the port lives here beside the fe_dctlkp binding that
// reaches it.

/// `fn_198fb4`. The map tree's post-order walk.
///
/// The broker's maps carry no key or value free callbacks: the ARM's own
/// destroy frees the nodes only, and the tree's key and value belong to the
/// broker.
fn fn_198fb4(m: &mut Memory<'_>, h: &mut impl crate::boot_part_e::Host, t: u32, n: u32) {
    if n == t {
        return;
    }
    fn_198fb4(m, h, t, m.word(n));
    fn_198fb4(m, h, t, m.word(n + 4));
    let heap = m.word(t + 0x20);
    crate::boot_part_e::release(m, heap, n, h);
}

/// `fn_19869c`. The map's tree destroy.
pub fn fn_19869c(m: &mut Memory<'_>, h: &mut impl crate::boot_part_e::Host, t: u32) {
    fn_198fb4(m, h, t, m.word(t));
    m.put_word(t + 0x18, 0);
    m.put_word(t, t);
    m.put_word(t + 4, t);
}

/// `fn_177044`. The resource pair's release.
///
/// The map's value release reaches this: the pair holds the two handles
/// the broker opened, and each handle's map goes first.
pub fn fn_177044(
    m: &mut Memory<'_>,
    h: &mut impl crate::boot_part_e::Host,
    resources: u32,
    heap: u32,
) {
    if resources == 0 {
        return;
    }
    let first = m.word(resources + 4);
    let second = m.word(resources + 8);
    if first != 0 {
        fn_19665c(m, h, first);
    }
    if second != 0 {
        fn_19665c(m, h, second);
    }
    crate::boot_part_e::release(m, heap, resources, h);
}

/// `fn_19665c`. The map's close: the tree first, then the map itself.
pub fn fn_19665c(m: &mut Memory<'_>, h: &mut impl crate::boot_part_e::Host, p: u32) -> u32 {
    if p == 0 {
        return 0x89002007;
    }
    fn_19869c(m, h, p + 0x18);
    crate::boot_part_e::release(m, m.word(p + 0x14), p, h);
    0
}

/// `fn_177498`. The archive cursor read.
///
/// The archive's platform read is the ROM's cursor read: the bytes come
/// from the cursor's own window, the position advances, and the caller gets
/// the number of bytes it may take.
pub fn fn_177498(m: &mut Memory<'_>, cursor: u32, out: &mut [u8]) -> u32 {
    let data = m.word(cursor);
    let length = m.word(cursor + 4);
    let position = m.word(cursor + 8);
    let mut bytes = out.len() as u32;
    let remaining = length.wrapping_sub(position);
    if remaining < bytes {
        bytes = remaining;
    }
    let count = bytes as usize;
    if count > 0 {
        let src = m.offset(data.wrapping_add(position));
        out[..count].copy_from_slice(&m.bytes[src..src + count]);
    }
    m.put_word(cursor + 8, position.wrapping_add(bytes));
    bytes
}

/// fn_1b2380. Close the archive stream's callback, then release the stream.
///
/// The ROM's stream object keeps its callback at +44; the callback's own
/// method table carries the close function and the adjustment that turns
/// the callback pointer back into the object the close function expects.
pub fn fn_1b2380(m: &mut Memory<'_>, p: u32, h: &mut impl crate::boot::Calls) -> u32 {
    let callback = m.word(p + 44);
    if callback != 0 {
        let methods = m.word(callback);
        let close = m.word(methods);
        let adjustment = m.word(methods + 4);
        let error = h.call(m, close, &[callback.wrapping_sub(adjustment)]);
        if error != 0 {
            return error;
        }
    }
    crate::boot_port_1::fn_1b44c8(m)
}

/// The live close of a broker data handle.
///
/// The crate's port (`leftovers_1::fn_179f3c`) frees the handle through the
/// detached `frontend::HeapVtable`, which carries no arena, so the live
/// cannot serve that release.  This port follows `src/fn_179f3c.c` over the
/// caller's arena: the begin log, the reference count, the unlink of the
/// last reference, the resource close, the release of the handle and the
/// end log.  Every heap operation is dispatched to the caller's dispatcher,
/// so the release happens over the same arena the ROM released from.
///
/// The ARM's own close in the boot (`rust/scripts/boot_part_a_r11_r19_trace.py`)
/// takes the reference-count path: the count stays above zero, so the
/// unlink is not taken and the only heap operation is the release of the
/// handle, after the resource close and before the end log.
pub trait LiveCloseHost:
    crate::boot::Calls + crate::log_live::LogMethodsM + crate::boot_part_b::FormatHost
{
}
impl<T: crate::boot::Calls + crate::log_live::LogMethodsM + crate::boot_part_b::FormatHost>
    LiveCloseHost for T
{
}

/// The close's own calls: the heap work goes to the live dispatcher.
struct CloseCalls<'a, H>(&'a mut H);

impl<H: LiveCloseHost> crate::boot_part_b::Host for CloseCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        crate::boot::Calls::call(self.0, m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}

impl<H: LiveCloseHost> crate::boot_part_d::Host for CloseCalls<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        crate::boot::Calls::call(self.0, m, f, a)
    }
    fn params_call_m(
        &mut self,
        _: u32,
        _: &mut [u32; 4],
        _: u32,
        _: &mut u32,
        _: u32,
        _: &mut Memory<'_>,
    ) -> u32 {
        panic!("the close does not open archive")
    }
}

/// The close's log line: the arguments go into the caller's scratch, the
/// logger writes them, and the scratch is restored.
fn close_log(
    m: &mut Memory<'_>,
    h: &mut impl LiveCloseHost,
    ctx: u32,
    k: &crate::leftovers_1::CloseRuntime,
    format: u32,
    args: &[u32],
) {
    let mut saved = [0u8; 64];
    saved.copy_from_slice(&m.bytes[k.scratch as usize..k.scratch as usize + 64]);
    for (i, &v) in args.iter().enumerate() {
        m.put_word(k.scratch + i as u32 * 4, v);
    }
    crate::log_live::fn_1738e8(m, m.word(ctx), k.category, 4, 0, format, k.scratch, h);
    m.bytes[k.scratch as usize..k.scratch as usize + 64].copy_from_slice(&saved);
}

/// The close's name lookup, over the broker's own tree.
fn close_lookup(
    m: &mut Memory<'_>,
    ctx: u32,
    name: u32,
    k: &crate::leftovers_1::CloseRuntime,
    h: &mut impl LiveCloseHost,
) -> u32 {
    let mut found = 0;
    let io = k.scratch + 64;
    crate::boot_live::lookup_keyed_value_m(
        m.bytes,
        Some(ctx),
        Some(name),
        &mut found,
        &k.fold,
        io,
        h,
    );
    found
}

/// fn_179f3c. The broker's data close, over the caller's arena.
pub fn fn_179f3c_m(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    k: &crate::leftovers_1::CloseRuntime,
    h: &mut impl LiveCloseHost,
) -> u32 {
    if ctx == 0 || handle == 0 {
        return 0x81506007;
    }
    let box_ = m.word(m.word(ctx + 4) + 8);
    let heap = m.word(box_ + 4);
    let mut rc = 0u32;
    close_log(m, h, ctx, k, k.begin, &[handle]);
    let data = m.word(handle + 4);
    if data != 0 {
        let refs =
            u16::from_le_bytes([m.byte(data + 60), m.byte(data + 61)]).wrapping_sub(1);
        m.put_half(data + 60, refs);
        if refs == 0 {
            let mut found = close_lookup(m, ctx, m.word(data + 64), k, h);
            if found == 0 {
                let name = m.word(data + 64);
                let length = crate::frontend::strlen_bytes(&m.bytes[name as usize..]);
                let copy = crate::boot::Calls::call(h, m, 0x196394, &[heap, length.wrapping_add(1)]);
                if copy == 0 {
                    return 0x8150600a;
                }
                let mut i = 0u32;
                while i < length {
                    let c = m.byte(name + i);
                    let c = if c.is_ascii_alphanumeric() {
                        c
                    } else if c == b'/' || c == b'_' {
                        b'_'
                    } else {
                        b'-'
                    };
                    m.put_byte(copy + i, c);
                    i += 1;
                }
                m.put_byte(copy + i, 0);
                if crate::frontend::compare_str(
                    &m.bytes[name as usize..],
                    &m.bytes[copy as usize..],
                ) != 0
                {
                    found = close_lookup(m, ctx, copy, k, h);
                }
                crate::boot::Calls::call(h, m, 0x196428, &[heap, copy]);
            }
            if found != 0 {
                m.put_word(found + 8, 0);
            }
            let data = m.word(handle + 4);
            if m.word(data + 56) != 0 {
                let resource = m.word(data + 56);
                crate::boot_part_d::fn_1730fc(m, resource, &mut CloseCalls(h));
                m.put_word(m.word(handle + 4) + 56, 0);
            }
            crate::boot_part_b::fn_17ca94(m, m.word(handle + 4), &mut CloseCalls(h));
            m.put_word(handle + 4, 0);
        }
    } else if m.word(handle) != 0 {
        let resource = m.word(handle);
        rc = crate::boot_part_d::fn_1730fc(m, resource, &mut CloseCalls(h));
        m.put_word(handle, 0);
    }
    if rc >= 0x80000000 {
        let mut saved = [0u8; 64];
        saved.copy_from_slice(&m.bytes[k.scratch as usize..k.scratch as usize + 64]);
        for (i, v) in [k.handle_key, handle, k.status_key, rc].into_iter().enumerate() {
            m.put_word(k.scratch + i as u32 * 4, v);
        }
        crate::render_part_a::error_log(
            m,
            h,
            m.word(ctx),
            k.category,
            0x118,
            k.error,
            k.scratch,
            k.scratch + 32,
        );
        m.bytes[k.scratch as usize..k.scratch as usize + 64].copy_from_slice(&saved);
    }
    crate::boot::Calls::call(h, m, 0x196428, &[heap, handle]);
    close_log(m, h, ctx, k, k.end, &[rc]);
    rc
}

/// fn_16b554. The archive find, live over the archive's own pages.
///
/// The crate's `speech_part_1::fn_16b554` is a script model of the walk: it
/// restores its scratch and it takes the case-classification table from its
/// caller. The ROM's fn_16b554 does neither: the name copy lands in the
/// archive list's own name field for good, and the folding the name compare
/// uses comes from the thread's own locale table (fn_10062c reads it through
/// the thread pointer), so a caller cannot hand the walk a different table.
///
/// This port follows the ROM instruction for instruction: the name copy, the
/// archive list walk, the group folder, the directory scan, the file lookup
/// and the size out-parameter. Every tree access reads the archive's own
/// pages in the arena through the crate's real ports.
struct ResolvePath {
    path: u32,
    locale: u32,
}

impl crate::boot_part_b::Host for ResolvePath {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        assert_eq!(f, 0x19de08);
        crate::boot_part_b2::fn_19de08(m, a[0], self.path, self.locale)
    }
    fn thread_id(&mut self) -> u32 {
        unreachable!()
    }
}

/// fn_19ddd0. Change the archive's current directory.
fn archive_chdir(m: &mut Memory<'_>, archive: u32, path: u32, locale: u32) -> u32 {
    crate::boot_part_b::fn_19ddd0(m, archive, &mut ResolvePath { path, locale })
}

/// fn_16b554. The archive find, over the archive's own pages.
///
/// `locale` is the thread's case-classification table, the one the ROM's
/// fn_10062c reads; `env.dot` and `env.parent` are the region's own "."
/// and ".." strings. `scratch` needs 80 bytes plus 32 per directory level.
pub fn fn_16b554_m(
    m: &mut Memory<'_>,
    list: u32,
    group: u32,
    name: u32,
    size: u32,
    scratch: u32,
    env: &crate::speech_part_1::ArchiveEnvironment,
) -> u32 {
    let locale = env.locale;
    // The name goes into the list's own name field, as u16, at most 63
    // characters, each byte sign extended.
    let mut len = 0u32;
    while len < 63 {
        let c = m.byte(name + len) as i8;
        if c == 0 {
            break;
        }
        m.put_half(list + 40 + len * 2, c as i16 as u16);
        len += 1;
    }
    m.put_half(list + 40 + len * 2, 0);
    let mut link = m.word(list + 8);
    while link != list + 8 {
        let archive = link + 8;
        let path = link + 36;
        if crate::boot_part_b2::fn_19de08(m, archive, path, locale) != u32::MAX
            && archive_chdir(m, archive, path, locale) != 0
        {
            let mut index = u32::MAX;
            if group != 0 {
                for i in 0..4u32 {
                    m.put_half(scratch + i * 2, ((group >> (24 - i * 8)) & 255) as u16);
                }
                m.put_half(scratch + 8, 0);
                if crate::boot_part_b2::fn_19de08(m, archive, scratch, locale) != u32::MAX
                    && archive_chdir(m, archive, scratch, locale) != 0
                {
                    index = crate::boot_part_b2::fn_19de08(m, archive, list + 40, locale);
                    archive_chdir(m, archive, env.parent, locale);
                }
            } else {
                crate::boot_part_b2::fn_19dcd8(m, archive, env.dot, scratch + 32, locale);
                while crate::boot_part_b::fn_19dd30(m, scratch + 32, scratch + 48) != 0 {
                    let entry_name = m.word(scratch + 56);
                    if m.byte(scratch + 60) != 0 {
                        archive_chdir(m, archive, entry_name, locale);
                        index = crate::boot_part_b2::fn_189040(
                            m,
                            archive,
                            list + 40,
                            env.dot,
                            env.parent,
                            locale,
                            scratch + 80,
                        );
                        archive_chdir(m, archive, env.parent, locale);
                        if index != u32::MAX {
                            break;
                        }
                    } else if m.wide_case_compare(locale, list + 40, entry_name) == 0 {
                        index = m.word(scratch + 52);
                        break;
                    }
                }
            }
            archive_chdir(m, archive, env.parent, locale);
            if index != u32::MAX {
                crate::boot_part_b::fn_188f9c(m, archive, index as i32, scratch + 16);
                let result = crate::boot_part_b::fn_189030(m, scratch + 16);
                if size != 0 {
                    m.put_word(size, m.word(scratch + 24));
                }
                if result != 0 {
                    return result;
                }
            }
        }
        link = m.word(link);
    }
    0
}

/// Caller-owned inputs of fn_243d50: the two log formats and one arg word.
/// The bytes are the ROM's; the addresses are caller scratch.
pub struct ChunkReleaseEnv {
    pub begin: u32,
    pub end: u32,
    pub args: u32,
}

/// fn_243d50. Release a detached chunk: close the owner pair, release the
/// record through the engine heap. Follows src/fn_243ad4.c: the Begin log,
/// fn_244324, fn_196428 (which runs even when the close failed), the End
/// log. The owner close runs through the live reentrant port.
pub fn fn_243d50_m(
    m: &mut Memory<'_>,
    record: u32,
    e: &ChunkReleaseEnv,
    h: &mut (impl crate::boot::Calls
          + crate::boot_part_e::Host
          + crate::log_live::LogMethodsM),
) -> u32 {
    if record == 0 || m.word(record + 16) == 0 {
        return 0x8a202007;
    }
    let env = m.word(record);
    let log = m.word(env + 16);
    let heap = m.word(env + 4);
    m.put_word(e.args, record);
    crate::log_live::fn_1738e8(m, log, 0x914878, 4, 0, e.begin, e.args, h);
    let rc = crate::reentrant2::close_owner_pair_m(m, record, h);
    crate::boot_part_e::release(m, heap, record, h);
    m.put_word(e.args, rc);
    crate::log_live::fn_1738e8(m, log, 0x914878, 4, 0, e.end, e.args, h);
    rc
}



/// Caller-owned inputs of fn_22af2c (the RETTT component open).
/// `pair`/`default` are the region's null pair and default context;
/// `g80`..`gfa0` are the region's D_8fa74c row; the remaining fields
/// are guest NUL-terminated strings plus disjoint caller scratch (at
/// least 64 bytes at `scratch`: the resolve cell, the 17b058 link
/// triple, the 175a1c value cell and spares).
pub struct RetttEnv {
    pub pair: [u8; 8],
    pub default: u32,
    pub g80: u32,
    pub g84: u32,
    pub g88: u32,
    pub g90: u32,
    pub g94: u32,
    pub g9c: u32,
    pub gfa0: u32,
    pub synth: u32,
    pub inet: u32,
    pub markup: u32,
    pub pipe: u32,
    pub reten: u32,
    pub zero: u32,
    pub scratch: u32,
}

/// Host surface of fn_22af2c_m: live dispatch plus the arena heap and
/// the log sink. Every seam calls the same certified port or live
/// entry the surrounding voice boot already runs.
pub trait RetttHost: crate::boot::Calls + crate::boot_part_b::Host + crate::boot_part_e::Host {
    /// Fail-path-only callee with no live route in this boot (174f9c,
    /// 17b1c4, 2ae784): name it the way the harness names unbound calls.
    /// The ARM voice boot never takes the fail path (rc is always 0 at
    /// 0x22af2c), so reaching this names the next binding to write.
    fn fail_unbound(&mut self, f: u32, args: &[u32]) -> u32;
}

struct RetttParser<'a, H>(&'a mut H);
impl<H: crate::boot_part_b::Host> crate::render_part_c::ParserMethodsM for RetttParser<'_, H> {
    fn method_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        crate::boot_part_b::Host::call(self.0, m, f, a)
    }
}

struct RetttCall<'a, H>(&'a mut H);
impl<H: crate::boot::Calls> crate::component_live_b::Host for RetttCall<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        crate::boot::Calls::call(self.0, m, f, a)
    }
}


struct RetttHooks<'a, H>(&'a mut H);
impl<H: RetttHost> crate::reentrant2::HooksM for RetttHooks<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut crate::boot::Memory<'_>) -> u32 {
        crate::boot::Calls::call(self.0, m, f, a)
    }
}

/// fn_22af2c. Open the RETTT component: resolve the handle, allocate the
/// 0x38-byte Xcoder, open SYNTHSTREAM (plus INET when present), build the
/// pattern engine and the linguistic list, publish the ruleset parameters
/// and register the owner. Follows src/fn_22ada0_merged.c branch for
/// branch; every callee is the certified port or live entry the voice
/// boot already runs. The fail path tears down through the same pieces;
/// the three fail-only callees with no live route in this boot (174f9c,
/// 17b1c4, 2ae784) name themselves instead of scripting a result.
#[allow(clippy::too_many_arguments)]
pub fn fn_22af2c_m<H: RetttHost>(
    m: &mut Memory<'_>,
    _a0: u32,
    _a1: u32,
    ha: u32,
    hb: u32,
    out: u32,
    e: &RetttEnv,
    h: &mut H,
) -> u32 {
    if out == 0 {
        return 0x88602007;
    }
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    // 176fd4(&e); *out = e.
    crate::boot_part_d::fn_176fd4(
        m,
        out,
        [u32::from_le_bytes(e.pair[..4].try_into().unwrap()), u32::from_le_bytes(e.pair[4..].try_into().unwrap())],
    );
    // 19611c(ha, hb, &o).
    let o_cell = e.scratch;
    let rc = crate::boot_port_1::fn_19611c(m, [ha, hb], o_cell, e.pair, e.default);
    if rc >= 0x80000000 {
        return rc;
    }
    let o = m.word(o_cell);
    // n = 1963b8(o->f04, 1, 0x38).
    let heap = m.word(o + 4);
    let n = crate::boot_part_e::calloc(m, heap, 1, 0x38, h);
    if n == 0 {
        return 0x8860200a;
    }
    m.put_word(n + 0x04, hb);
    m.put_word(n, ha);
    // lk = 17b058(o->f18, "SYNTHSTREAM").
    let res = m.word(o + 24);
    let lk = e.scratch + 4;
    let rc = crate::render_part_c::fn_17b058_m(m, res, e.synth, lk, &mut RetttParser(h));
    if rc >= 0x80000000 {
        return fail_22af2c(m, n, o, e, h, rc);
    }
    m.put_word(n + 0x20, m.word(lk));
    // 17b058(o->f18, "INET") — optional.
    let rc = crate::render_part_c::fn_17b058_m(m, res, e.inet, lk, &mut RetttParser(h));
    if rc < 0x80000000 {
        m.put_word(n + 0x08, m.word(lk));
        m.put_word(n + 0x0c, m.word(lk + 4));
        m.put_word(n + 0x10, m.word(lk + 8));
    }
    // 2ae690(ha, hb, &n->f14): 16-byte pattern engine object.
    let rc = pattern_create_22af2c(m, ha, hb, o, e, h);
    if rc >= 0x80000000 {
        return fail_22af2c(m, n, o, e, h, rc);
    }
    m.put_word(n + 0x14, rc);
    // 196a64(o->f04, g80, g84, g88, 1, &n->f18): 32-byte linguistic block.
    // The ROM rejects exactly one-sided nonzero (c, d); otherwise it
    // allocates, zeroes, and publishes the address.
    if (e.g84 != 0) != (e.g88 != 0) {
        return fail_22af2c(m, n, o, e, h, 0x88602007);
    }
    let lb = crate::boot_part_e::calloc(m, heap, 1, 32, h);
    if lb == 0 {
        return fail_22af2c(m, n, o, e, h, 0x88602007);
    }
    for i in 0..8u32 {
        m.put_word(lb + i * 4, 0);
    }
    m.put_word(lb + 0x04, e.g80);
    m.put_word(lb + 0x08, e.g84);
    m.put_word(lb + 0x0c, e.g88);
    m.put_half(lb + 0x18, 1);
    m.put_word(lb + 0x1c, heap);
    m.put_word(n + 0x18, lb);
    // 175a1c(o->f14, "rulesetsfrommarkup", &s1).
    let params = m.word(o + 20);
    let sval = e.scratch + 20;
    m.put_word(sval, 0);
    let rc = crate::boot::Calls::call(h, m, 0x175a1c, &[params, e.markup, sval]);
    if rc >= 0x80000000 {
        m.put_word(n + 0x1c, 0);
        let rc = crate::boot::Calls::call(h, m, 0x1759a0, &[params, e.markup, e.zero]);
        if rc >= 0x80000000 {
            return fail_22af2c(m, n, o, e, h, rc);
        }
    } else {
        let s = m.word(sval);
        let t = if s == 0 { &[0u8][..] } else { &m.bytes[m.offset(s)..] };
        let z = t.iter().position(|&b| b == 0).unwrap_or(t.len());
        let w = &t[..z.min(256)];
        let is_true = w.eq_ignore_ascii_case(b"true") || w == b"1";
        m.put_word(n + 0x1c, u32::from(is_true));
    }
    // 174e74(o->f14, "rulesetsfrommarkup", n, g90, g94).
    let rc = crate::boot::Calls::call(h, m, 0x174e74, &[params, e.markup, n, e.g90, e.g94]);
    if rc >= 0x80000000 {
        return fail_22af2c(m, n, o, e, h, rc);
    }
    m.put_word(n + 0x34, 0);
    // clcpppipelinemode? (default 0; set when the param is a non-empty string)
    m.put_word(sval, 0);
    let rc = crate::boot::Calls::call(h, m, 0x175a1c, &[params, e.pipe, sval]);
    if rc < 0x80000000 && m.word(sval) != 0 && m.bytes[m.offset(m.word(sval))] != 0 {
        m.put_word(n + 0x34, 1);
    }
    // 1757b8(o->f14, "enablerett", 1).
    let rc = crate::boot::Calls::call(h, m, 0x1757b8, &[params, e.reten, 1]);
    if rc >= 0x80000000 {
        return fail_22af2c(m, n, o, e, h, rc);
    }
    m.put_word(out, n);
    m.put_word(out + 4, 0x10d);
    0
}

/// fn_2ae690 for the 22af2c pattern engine: resolve, allocate the 16-byte
/// object, open the PCRE handle into f8/fc and initialize it. Returns the
/// object address on success, or an error code with bit 31 set.
fn pattern_create_22af2c<H: RetttHost>(m: &mut Memory<'_>, ha: u32, hb: u32, _o: u32, e: &RetttEnv, h: &mut H) -> u32 {
    // 19611c(ha, hb, &o2): same owner the caller resolved.
    let o2 = e.scratch + 24;
    let rc = crate::boot_port_1::fn_19611c(m, [ha, hb], o2, e.pair, e.default);
    if rc >= 0x80000000 {
        return rc;
    }
    let oo = m.word(o2);
    // p = 196394(o->heap, 16).
    let heap = m.word(oo + 4);
    let p = crate::boot::Calls::call(h, m, 0x196394, &[heap, 16]);
    if p == 0 {
        return 0x8a50200a;
    }
    for i in 0..4u32 {
        m.put_word(p + i * 4, 0);
    }
    m.put_word(p, ha);
    m.put_word(p + 4, hb);
    // 23fe84(h0, h1, &p->f8): resolve, allocate the 24-byte head.
    let rc = crate::boot_port_1::fn_19611c(m, [ha, hb], o2, e.pair, e.default);
    if rc >= 0x80000000 {
        crate::boot::Calls::call(h, m, 0x196428, &[heap, p]);
        return rc;
    }
    let po = m.word(o2);
    let ph = m.word(po + 4);
    let head = crate::boot_part_e::calloc(m, ph, 1, 24, h);
    if head == 0 {
        crate::boot::Calls::call(h, m, 0x196428, &[heap, p]);
        return 0x8a50200a;
    }
    m.put_word(head, po);
    m.put_word(p + 8, head);
    m.put_word(p + 12, 0xdd57);
    // 2401a0(p->f8, p->fc, 10, 10).
    let rc = crate::component_live_b::pcre_init_m(m, m.word(p + 8), m.word(p + 12), 10, 10, &mut RetttCall(h));
    if rc >= 0x80000000 {
        crate::boot::Calls::call(h, m, 0x196428, &[heap, p]);
        return rc;
    }
    p
}

/// Fail path of fn_22af2c: tear down the partial Xcoder the way fn_22ae34
/// does. The three fail-only callees with no live route in this boot
/// (174f9c, 17b1c4, 2ae784) name themselves instead of scripting a result;
/// the ARM voice boot never takes this path (rc is always 0 at 0x22af2c).
fn fail_22af2c<H: RetttHost>(m: &mut Memory<'_>, n: u32, o: u32, e: &RetttEnv, h: &mut H, rc: u32) -> u32 {
    let mut ocell = [0u32; 1];
    let rc0 = crate::boot_port_1::fn_19611c(m, [m.word(n), m.word(n + 4)], e.scratch + 28, e.pair, e.default);
    if rc0 < 0x80000000 {
        let oo = m.word(e.scratch + 28);
        let _ = (oo, ocell);
        h.fail_unbound(0x174f9c, &[m.word(o + 20), e.markup, n, e.g9c, e.gfa0]);
        if m.word(n + 8) != 0 {
            h.fail_unbound(0x17b1c4, &[m.word(o + 24), e.inet]);
        }
        if m.word(n + 0x20) != 0 {
            h.fail_unbound(0x17b1c4, &[m.word(o + 24), e.synth]);
        }
        if m.word(n + 0x14) != 0 {
            h.fail_unbound(0x2ae784, &[m.word(n + 0x14)]);
        }
        if m.word(n + 0x18) != 0 {
            let mut hooks = RetttHooks(h);
            crate::reentrant2::destroy_array_m(m, m.word(n + 0x18), &mut hooks);
        }
        crate::boot::Calls::call(h, m, 0x196428, &[m.word(oo + 4), n]);
    }
    let _ = (o, ocell);
    rc
}
