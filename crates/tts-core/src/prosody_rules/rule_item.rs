//! Prosody rule-interpreter instances and their engine host interface.
//! Ports fn_2ab2cc, fn_2ab40c, fn_2ab5c4, fn_22af2c, fn_22b2f4, fn_22b3bc,
//! fn_22b518, fn_2ab7b4 and the rule-set fetch path.

use super::arena::Arena;
use super::{
    arena_copy_from, destroy_rule_set, release_rule_streams, scan_header, scan_rules, scan_strlen,
    RuleScanHost, RuleSet, RuleSetHost, RuleStreamHost,
};

/// fn_19611c environment view for the rule interpreter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuleContext {
    pub heap: u32,
    pub log: u32,
    pub params: u32,
    pub res: u32,
}

/// One rule-interpreter item (`Item` in fn_2aaa20.c, 0x2c bytes).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuleItem {
    pub handle: [u32; 2],
    pub engine: u32,
    pub name: u32,
    pub alternate: u32,
    pub active: u8,
    pub in_memory: u8,
    pub language: u32,
    pub kind: u32,
    pub rules: u32,
    pub stream: [u32; 2],
}

impl RuleItem {
    /// Original field order for differential comparison.
    pub fn words(&self) -> [u32; 11] {
        [
            self.handle[0],
            self.handle[1],
            self.engine,
            self.name,
            self.alternate,
            u32::from(self.active) | (u32::from(self.in_memory) << 8),
            self.language,
            self.kind,
            self.rules,
            self.stream[0],
            self.stream[1],
        ]
    }
}

/// The 0x38-byte rule-set object shared by fn_22af2c, fn_22b2f4 and fn_22b3bc
/// (`S` in fn_22ada0_merged.c).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuleSetInstance {
    pub address: u32,
    pub f00: u32,
    pub f04: u32,
    pub f08: u32,
    pub f0c: u32,
    pub f10: u32,
    pub f14: u32,
    pub f18: u32,
    pub f1c: u8,
    pub f20: u32,
    pub f24: u32,
    pub f28: u32,
    pub f2c: u32,
    pub f30: u32,
    pub f34: u8,
}

impl RuleSetInstance {
    /// View used by `destroy_rule_set` (fn_22ae34) and `release_rule_streams`
    /// (fn_22b238).
    pub fn view(&self) -> RuleSet {
        RuleSet {
            id: self.address,
            h0: self.f00,
            h1: self.f04,
            f08: self.f08,
            f14: self.f14,
            f18: self.f18,
            f20: self.f20,
            f24: [self.f24, self.f28],
            f2c: [self.f2c, self.f30],
        }
    }

    fn adopt(&mut self, view: &RuleSet) {
        self.f24 = view.f24[0];
        self.f28 = view.f24[1];
        self.f2c = view.f2c[0];
        self.f30 = view.f2c[1];
    }
}

/// A rule-set list entry read back through fn_1970ac.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuleListEntry {
    pub in_memory: u8,
    pub active: u8,
    pub has_name: bool,
    pub has_alternate: bool,
}

pub const RULE_NAME: u32 = 1;
pub const RULE_ALTERNATE: u32 = 2;

/// Engine calls reached by the rule-interpreter ports.
pub trait RuleInterpreterHost {
    fn resolve(&mut self, h0: u32, h1: u32) -> (u32, Option<RuleContext>);
    fn new_handle(&mut self) -> [u32; 2];
    fn alloc_object(&mut self, heap: u32, count: u32, size: u32) -> u32;
    fn acquire(&mut self, res: u32, name: &[u8], out: &mut [u32; 3]) -> u32;
    fn release_resource(&mut self, res: u32, name: &[u8]);
    fn create_engine(&mut self, h0: u32, h1: u32, out: &mut u32) -> u32;
    fn create_list(&mut self, heap: u32, spec: [u32; 3], count: u32, out: &mut u32) -> u32;
    fn destroy_list(&mut self, list: u32);
    fn count_list(&mut self, list: u32) -> u32;
    fn get_item(&mut self, list: u32, index: u32, out: &mut RuleItem) -> bool;
    fn list_entry(&mut self, list: u32, index: u32, out: &mut RuleListEntry) -> bool;
    fn list_entry_text(&mut self, list: u32, index: u32, which: u32, out: &mut [u8]) -> bool;
    fn insert_list(&mut self, list: u32, entry: [u32; 2]) -> bool;
    fn remove_list(&mut self, list: u32, index: u32) -> bool;
    fn open_stream(&mut self, h0: u32, h1: u32, out: &mut [u32; 2]) -> u32;
    fn configure_stream(&mut self, h0: u32, h1: u32, a: u32, b: u32) -> u32;
    fn close_stream(&mut self, h0: u32, h1: u32) -> u32;
    fn free_stream(&mut self, h0: u32, h1: u32) -> u32;
    fn open_item_stream(
        &mut self,
        h0: u32,
        h1: u32,
        kind: u32,
        mime: &[u8],
        a: u32,
        b: u32,
        out: &mut u32,
    ) -> u32;
    fn get_param(&mut self, params: u32, key: &[u8], out: &mut [u8]) -> u32;
    fn set_param(&mut self, params: u32, key: &[u8], value: &[u8]) -> u32;
    fn subscribe(&mut self, params: u32, key: &[u8], owner: u32, a: u32, b: u32) -> u32;
    fn set_int(&mut self, params: u32, key: &[u8], value: i32) -> u32;
    fn get_int(&mut self, params: u32, key: &[u8], out: &mut i32) -> u32;
    fn logging(&mut self, log: u32) -> bool;
    fn log_line(&mut self, log: u32, kind: u32);
    fn log_error(&mut self, log: u32, code: u32);
    fn log_module(&mut self, log: u32, code: u32);
}

fn markup_enabled(text: &[u8]) -> bool {
    let text = &text[..text.iter().position(|&b| b == 0).unwrap_or(text.len())];
    !text.is_empty() && (text.eq_ignore_ascii_case(b"true") || text == b"1")
}

/// fn_2ab2cc / EU fn_2ab488 / KR fn_2ab534.
pub fn init_rule_item(
    h0: u32,
    h1: u32,
    engine: u32,
    spec: [u32; 3],
    item: &mut RuleItem,
    host: &mut impl RuleInterpreterHost,
) -> u32 {
    if engine == 0 {
        return 0x88602007;
    }
    let (status, ctx) = host.resolve(h0, h1);
    if status >= 0x80000000 {
        return 0x88602007;
    }
    let Some(ctx) = ctx else { return 0x88602007 };
    *item = RuleItem {
        handle: [h0, h1],
        engine,
        active: 1,
        ..RuleItem::default()
    };
    if host.create_list(ctx.heap, spec, 8, &mut item.rules) == 0 {
        host.log_error(ctx.log, 0x2afa);
        return 0x8860200a;
    }
    let status = host.open_stream(h0, h1, &mut item.stream);
    if status >= 0x80000000 {
        host.destroy_list(item.rules);
        item.rules = 0;
        return status;
    }
    let status = host.configure_stream(item.stream[0], item.stream[1], 10, 10);
    if status < 0x80000000 {
        return 0;
    }
    host.destroy_list(item.rules);
    item.rules = 0;
    host.free_stream(item.stream[0], item.stream[1]);
    status
}

/// fn_2ab40c / EU fn_2ab5c8 / KR fn_2ab674. Ported from the ARM body: the
/// src/fn_2aaa20.c draft differs in the second lookup handle, the rule-removal
/// loop and the return value (see RESULTS.md).
pub fn release_rule_item(
    arena: &mut Arena<'_>,
    item: &mut RuleItem,
    host: &mut impl RuleInterpreterHost,
) -> u32 {
    let (status, ctx) = host.resolve(item.handle[0], item.handle[1]);
    if status >= 0x80000000 {
        return 0x88602007;
    }
    let Some(ctx) = ctx else { return 0x88602007 };
    host.log_line(ctx.log, 3);
    let mut pending = 0x88602007u32;
    let mut removed_all = true;
    if item.name == 0 {
        pending = 0x88602011;
    } else if host.resolve(item.handle[0], item.handle[1]).0 >= 0x80000000 {
        // pending stays 0x88602007 and the buffers are left in place.
    } else {
        let mut count = (host.count_list(item.rules) & 0xffff) as u32;
        if count != 0 {
            loop {
                if !host.remove_list(item.rules, count - 1) {
                    host.log_error(ctx.log, 0x2b10);
                    removed_all = false;
                    break;
                }
                count -= 1;
                if count == 0 {
                    break;
                }
            }
        }
        if removed_all {
            pending = 0;
            for field in [&mut item.alternate, &mut item.language, &mut item.kind] {
                if *field != 0 {
                    arena.free(*field);
                    *field = 0;
                }
            }
        }
    }
    host.log_line(ctx.log, 3);
    if pending >= 0x80000000 {
        return 0;
    }
    if item.name != 0 {
        arena.free(item.name);
        item.name = 0;
    }
    pending
}

/// fn_2ab5c4 / EU fn_2ab780 / KR fn_2ab82c. The stream teardown returns the
/// failing status directly; the src/fn_2aaa20.c draft returns 0x88602007.
pub fn destroy_rule_item(
    arena: &mut Arena<'_>,
    item: &mut RuleItem,
    host: &mut impl RuleInterpreterHost,
) -> u32 {
    if host.resolve(item.handle[0], item.handle[1]).0 >= 0x80000000 {
        return 0x88602007;
    }
    if item.name != 0 {
        release_rule_item(arena, item, host);
    }
    if item.rules != 0 {
        host.destroy_list(item.rules);
        item.rules = 0;
    }
    let status = host.close_stream(item.stream[0], item.stream[1]);
    if status >= 0x80000000 {
        return status;
    }
    let status = host.free_stream(item.stream[0], item.stream[1]);
    if status >= 0x80000000 {
        return status;
    }
    *item = RuleItem::default();
    0
}

fn destroy_instance(
    instance: &RuleSetInstance,
    host: &mut (impl RuleInterpreterHost + RuleSetHost),
) -> u32 {
    let view = instance.view();
    destroy_rule_set(Some(&view), host)
}

/// fn_22af2c / EU fn_22b0e8 / KR fn_22b194.
#[allow(clippy::too_many_arguments)]
pub fn open_rule_set(
    ha: u32,
    hb: u32,
    config: [u32; 5],
    engine_spec: [u32; 3],
    out: Option<&mut [u32; 2]>,
    instance: &mut RuleSetInstance,
    host: &mut (impl RuleInterpreterHost + RuleSetHost),
) -> u32 {
    let mut rc = 0x88602007u32;
    let Some(out) = out else { return rc };
    *out = host.new_handle();
    let mut link = [0u32; 3];
    let (status, ctx) = host.resolve(ha, hb);
    if status >= 0x80000000 {
        return rc;
    }
    let Some(ctx) = ctx else { return rc };
    let object = host.alloc_object(ctx.heap, 1, 0x38);
    if object == 0 {
        return 0x8860200a;
    }
    *instance = RuleSetInstance {
        address: object,
        f00: ha,
        f04: hb,
        ..RuleSetInstance::default()
    };
    rc = host.acquire(ctx.res, b"SYNTHSTREAM", &mut link);
    if rc >= 0x80000000 {
        return fail(instance, rc, out, host);
    }
    instance.f20 = link[0];
    if host.acquire(ctx.res, b"INET", &mut link) < 0x80000000 {
        instance.f08 = link[0];
        instance.f0c = link[1];
        instance.f10 = link[2];
    }
    rc = host.create_engine(ha, hb, &mut instance.f14);
    if rc >= 0x80000000 {
        return fail(instance, rc, out, host);
    }
    if host.create_list(ctx.heap, engine_spec, 1, &mut instance.f18) == 0 {
        return fail(instance, rc, out, host);
    }
    let mut text = [0u8; 256];
    if host.get_param(instance.f14, b"rulesetsfrommarkup", &mut text) >= 0x80000000 {
        instance.f1c = 0;
        rc = host.set_param(instance.f14, b"rulesetsfrommarkup", b"0");
        if rc >= 0x80000000 {
            return fail(instance, rc, out, host);
        }
    } else {
        instance.f1c = markup_enabled(&text) as u8;
    }
    rc = host.subscribe(
        instance.f14,
        b"rulesetsfrommarkup",
        object,
        config[3],
        config[4],
    );
    if rc >= 0x80000000 {
        return fail(instance, rc, out, host);
    }
    instance.f34 = 0;
    if host.get_param(instance.f14, b"clcpppipelinemode", &mut text) < 0x80000000
        && text.first().is_some_and(|&byte| byte != 0)
    {
        instance.f34 = 1;
    }
    rc = host.set_int(instance.f14, b"enablerett", 1);
    if rc >= 0x80000000 {
        return fail(instance, rc, out, host);
    }
    let address = instance.address;
    out[0] = address;
    out[1] = 0x10d;
    0
}

fn fail(
    instance: &RuleSetInstance,
    rc: u32,
    out: &mut [u32; 2],
    host: &mut (impl RuleInterpreterHost + RuleSetHost),
) -> u32 {
    destroy_instance(instance, host);
    if rc < 0x80000000 {
        out[0] = 0;
        out[1] = 0x10d;
        0
    } else {
        rc
    }
}

/// fn_22b2f4 / EU fn_22b4b0 / KR fn_22b55c.
pub fn clear_rule_set(
    arena: &mut Arena<'_>,
    instance: &mut RuleSetInstance,
    arg: u32,
    host: &mut (impl RuleInterpreterHost + RuleStreamHost + RuleSetHost),
) -> u32 {
    let rc = host.validate(instance.address, arg, 0x10d, 0x38);
    if rc >= 0x80000000 {
        return rc;
    }
    if host.resolve(instance.f00, instance.f04).0 < 0x80000000 {
        let mut count = host.count_list(instance.f18);
        loop {
            if count == 0 {
                break;
            }
            let mut item = RuleItem::default();
            if !host.get_item(instance.f18, count - 1, &mut item) {
                break;
            }
            release_rule_item(arena, &mut item, host);
            destroy_rule_item(arena, &mut item, host);
            host.remove_list(instance.f18, count - 1);
            count -= 1;
        }
    }
    let mut view = instance.view();
    release_rule_streams(Some(&mut view), arg, host);
    instance.adopt(&view);
    destroy_instance(instance, host)
}

/// fn_22b3bc / EU fn_22b578 / KR fn_22b624.
pub fn start_rule_streams(
    instance: &mut RuleSetInstance,
    param2: u32,
    param3: u32,
    param4: u32,
    host: &mut (impl RuleInterpreterHost + RuleStreamHost + RuleSetHost),
) -> u32 {
    let mut rc = host.validate(instance.address, param2, 0x10d, 0x38);
    if rc >= 0x80000000 {
        return 0x88602008;
    }
    let (status, ctx) = host.resolve(instance.f00, instance.f04);
    if status >= 0x80000000 {
        return 0x88602007;
    }
    let Some(ctx) = ctx else { return 0x88602007 };
    rc = status;
    if instance.f34 == 0 && host.count_list(instance.f18) == 0 && instance.f1c == 0 {
        return u32::from(instance.f1c);
    }
    let mut enabled = 0i32;
    let status = host.get_int(ctx.params, b"enablerett", &mut enabled);
    if status < 0x80000000 && enabled != 1 {
        instance.f24 = 0;
        instance.f2c = 0;
        return rc;
    }
    let mut handle = 0;
    let status = host.open_item_stream(
        param3,
        param4,
        0x86,
        b"text/plain;charset=utf-16",
        0,
        0,
        &mut handle,
    );
    instance.f24 = handle;
    if status >= 0x80000000 {
        rc = status;
    } else {
        let status = host.open_item_stream(
            param3,
            param4,
            0x86,
            b"text/plain;charset=utf-16",
            0,
            1,
            &mut handle,
        );
        instance.f2c = handle;
        if status >= 0x80000000 {
            rc = status;
        }
    }
    if rc >= 0x80000000 {
        let mut view = instance.view();
        release_rule_streams(Some(&mut view), param2, host);
        instance.adopt(&view);
    }
    rc
}

/// One rule-list entry as seen by `fn_22b518`: the shared item plus the
/// restart mark at byte +0x16 (`f16`). The original stores the mark in the
/// item's padding; the two views compare identically.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProcessItem {
    pub base: RuleItem,
    pub mark: u8,
}

impl ProcessItem {
    /// The +0x1c type tag byte: nonzero means a typed rule.
    pub fn is_typed(&self) -> bool {
        self.base.kind & 0xff != 0
    }
}

/// Engine calls reached by `fn_22b518`. Text contents live in the shared
/// `Arena`; `cur`/`prev` are arena offsets (0 is null). The rule applier,
/// text codec and stream chain are scripted host calls; their inner traces
/// belong to their own tests. Only statuses, sizes and codes enter the
/// trace, never raw arena offsets. List marks persist through `update_item`.
/// Byte +0x15 is the in-memory flag, +0x16 the restart mark and +0x1c the
/// type tag byte, matching the original item view.
pub trait ProcessHost {
    fn validate(&mut self, object: u32, arg: u32) -> u32;
    fn resolve(&mut self, h0: u32, h1: u32) -> (u32, Option<RuleContext>);
    fn alloc(&mut self, arena: &mut Arena<'_>, size: u32) -> u32;
    fn free(&mut self, arena: &mut Arena<'_>, ptr: u32);
    fn stream_info(&mut self, h0: u32, h1: u32) -> (u32, u32, u8);
    fn copy_stream(&mut self, h0: u32, h1: u32) -> (u32, u32);
    fn stream_data(&mut self, h0: u32, h1: u32) -> (u32, u32, u32);
    fn advance_stream(&mut self, h0: u32, h1: u32, bytes: u32) -> u32;
    fn decode_stream(
        &mut self,
        h0: u32,
        h1: u32,
        arena: &mut Arena<'_>,
        dest: u32,
        units: u32,
    ) -> (u32, u32);
    fn get_lang(&mut self, params: u32, arena: &mut Arena<'_>) -> (u32, u32);
    fn count_list(&mut self, list: u32) -> u32;
    fn get_item(&mut self, list: u32, index: u32, out: &mut ProcessItem) -> bool;
    fn update_item(&mut self, list: u32, index: u32, item: &ProcessItem);
    fn remove_item(&mut self, list: u32, index: u32);
    fn destroy_list(&mut self, list: u32);
    fn create_list(&mut self, heap: u32, count: u32) -> (bool, u32);
    fn insert_item(&mut self, list: u32, item: &ProcessItem) -> bool;
    fn apply_ruleset(
        &mut self,
        arena: &mut Arena<'_>,
        item: &ProcessItem,
        input: u32,
        output: &mut u32,
    ) -> u32;
    fn matches_language(&mut self, item: &ProcessItem, lang: u32) -> bool;
    fn encode_text(&mut self, arena: &mut Arena<'_>, text: u32, len: usize) -> (u32, usize);
    fn reserve_stream(&mut self, h0: u32, h1: u32, len: u32) -> (u32, u32);
    fn finish_stream(&mut self, h0: u32, h1: u32, bytes: u32) -> u32;
    fn close_streams(&mut self, h0: u32, h1: u32) -> u32;
    fn logging(&mut self) -> bool;
    fn log(&mut self, code: u32);
}

/// fn_22b518 / EU fn_22b6d4 / KR fn_22b780.
///
/// Runs the untyped-private, typed and untyped rule passes over the input
/// streams, writing the final text back through the stream chain. The two
/// trailing handle arguments of the original are unused. Returns the stream
/// status and sets `*out`.
#[allow(clippy::too_many_arguments)]
pub fn process_rules(
    arena: &mut Arena<'_>,
    instance: &mut RuleSetInstance,
    arg: u32,
    out: Option<&mut u8>,
    host: &mut impl ProcessHost,
) -> u32 {
    let mut rc = host.validate(instance.address, arg);
    if rc >= 0x80000000 {
        return rc;
    }
    let (status, ctx) = host.resolve(instance.f00, instance.f04);
    if status >= 0x80000000 {
        return 0x88602007;
    }
    let Some(ctx) = ctx else { return 0x88602007 };
    let Some(out) = out else { return 0x88602007 };
    if instance.f24 == 0 || instance.f2c == 0 {
        *out = 1;
        return 0;
    }
    *out = 0;
    let (status, _, flag) = host.stream_info(instance.f24, instance.f28);
    rc = status;
    if rc >= 0x80000000 {
        return rc;
    }
    if flag == 1 && instance.f2c == 0 {
        *out = 1;
        return 0;
    }
    if flag == 0 && instance.f34 == 0 {
        *out = 0;
        return 0;
    }
    if host.count_list(instance.f18) == 0 {
        let (status, _) = host.copy_stream(instance.f2c, instance.f30);
        rc = status;
        return finish_process(arena, instance, out, rc, 0, 0, 0, host);
    }
    let (status, _, size) = host.stream_data(instance.f24, instance.f28);
    rc = status;
    if rc >= 0x80000000 {
        return rc;
    }
    if host.logging() {
        host.log(4);
        host.log(5);
    }
    // The decoded unit count is half the stream byte count.
    let units = size >> 1;
    let mut cur = host.alloc(arena, units * 4 + 1);
    if cur == 0 {
        return 0x8860200a;
    }
    let mut prev = 0u32;
    let (status, len) = host.decode_stream(instance.f24, instance.f28, arena, cur, units);
    rc = status;
    if rc >= 0x80000000 {
        return finish_process(arena, instance, out, rc, cur, prev, 0, host);
    }
    arena.put_byte(cur + len, 0);
    let status = host.advance_stream(instance.f24, instance.f28, units * 2);
    rc = status;
    if rc >= 0x80000000 {
        return finish_process(arena, instance, out, rc, cur, prev, 0, host);
    }
    let (status, lang) = host.get_lang(ctx.params, arena);
    rc = status;
    if rc >= 0x80000000 {
        return finish_process(arena, instance, out, rc, cur, prev, 0, host);
    }
    host.log(7);
    let mut n = host.count_list(instance.f18);
    while n != 0 {
        let mut item = ProcessItem::default();
        if host.get_item(instance.f18, n - 1, &mut item)
            && !item.is_typed()
            && item.base.in_memory != 0
            && host.matches_language(&item, lang)
        {
            let swap = cur;
            cur = prev;
            prev = swap;
            rc = host.apply_ruleset(arena, &item, prev, &mut cur);
            if rc >= 0x80000000 {
                break;
            }
        }
        n -= 1;
    }
    host.log(7);
    if rc >= 0x80000000 {
        return finish_process(arena, instance, out, rc, cur, prev, 0, host);
    }
    let (listed, extra) = host.create_list(ctx.heap, 1);
    if !listed {
        return finish_process(arena, instance, out, rc, cur, prev, 0, host);
    }
    // The original re-reads the list count after every item (do-while);
    // nothing in the pass mutates the list, but the call sequence is
    // observable and the stage replay checks it.
    let mut k = 0u32;
    if host.count_list(instance.f18) > k {
        loop {
            let mut item = ProcessItem::default();
            if host.get_item(instance.f18, k, &mut item)
                && item.is_typed()
                && host.matches_language(&item, lang)
                && !host.insert_item(extra, &item)
            {
                return finish_process(arena, instance, out, rc, cur, prev, extra, host);
            }
            k += 1;
            if host.count_list(instance.f18) <= k {
                break;
            }
        }
    }
    host.log(7);
    loop {
        let mut n = host.count_list(extra);
        let mut restart = false;
        while n != 0 {
            let mut item = ProcessItem::default();
            if !host.get_item(extra, n - 1, &mut item) {
                break;
            }
            if item.mark != 1 {
                let swap = cur;
                cur = prev;
                prev = swap;
                rc = host.apply_ruleset(arena, &item, prev, &mut cur);
                if rc >= 0x80000000 {
                    break;
                }
                if prev != 0 {
                    if cur == 0 || !arena_eq(arena, prev, cur) || cur == 0 {
                        item.mark = 1;
                        host.update_item(extra, n - 1, &item);
                        restart = true;
                        break;
                    }
                } else if cur != 0 {
                    item.mark = 1;
                    host.update_item(extra, n - 1, &item);
                    restart = true;
                    break;
                }
            }
            n -= 1;
        }
        if !restart {
            break;
        }
    }
    host.log(7);
    if rc >= 0x80000000 {
        return finish_process(arena, instance, out, rc, cur, prev, extra, host);
    }
    host.log(7);
    let mut n = host.count_list(instance.f18);
    while n != 0 {
        let mut item = ProcessItem::default();
        if host.get_item(instance.f18, n - 1, &mut item)
            && !item.is_typed()
            && item.base.in_memory == 0
            && host.matches_language(&item, lang)
        {
            let swap = cur;
            cur = prev;
            prev = swap;
            rc = host.apply_ruleset(arena, &item, prev, &mut cur);
            if rc >= 0x80000000 {
                break;
            }
        }
        n -= 1;
    }
    host.log(7);
    if rc >= 0x80000000 {
        return finish_process(arena, instance, out, rc, cur, prev, extra, host);
    }
    let len = if cur != 0 { arena.len(cur) } else { 0 };
    if len == 0 {
        return finish_process(arena, instance, out, rc, cur, prev, extra, host);
    }
    let (status, _) = host.reserve_stream(instance.f2c, instance.f30, len as u32 * 2);
    rc = status;
    if rc >= 0x80000000 {
        return finish_process(arena, instance, out, rc, cur, prev, extra, host);
    }
    let (status, out_len) = host.encode_text(arena, cur, len);
    rc = status;
    let len = if rc >= 0x80000000 { 0 } else { out_len };
    host.log(6);
    let status = host.finish_stream(instance.f2c, instance.f30, len as u32 * 2);
    if status >= 0x80000000 && rc < 0x80000000 {
        rc = status;
    }
    finish_process(arena, instance, out, rc, cur, prev, extra, host)
}

fn arena_eq(arena: &Arena<'_>, left: u32, right: u32) -> bool {
    arena.str(left) == arena.str(right)
}

#[allow(clippy::too_many_arguments)]
fn finish_process(
    arena: &mut Arena<'_>,
    instance: &mut RuleSetInstance,
    out: &mut u8,
    mut rc: u32,
    cur: u32,
    prev: u32,
    extra: u32,
    host: &mut impl ProcessHost,
) -> u32 {
    host.log(4);
    if extra != 0 {
        let mut n = host.count_list(extra);
        while n != 0 {
            let mut item = ProcessItem::default();
            if !host.get_item(extra, n - 1, &mut item) {
                break;
            }
            host.remove_item(extra, n - 1);
            n -= 1;
        }
        host.destroy_list(extra);
    }
    if prev != 0 {
        host.free(arena, prev);
    }
    if cur != 0 {
        host.free(arena, cur);
    }
    if instance.f34 == 0 {
        let status = host.close_streams(instance.f2c, instance.f30);
        if status >= 0x80000000 {
            rc = status;
        }
        instance.f2c = 0;
        instance.f30 = 0;
    }
    *out = 1;
    rc
}

/// Fetcher session/engine/rules/provider view (`Fetcher` in fn_22be08.c).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuleFetcher {
    pub session: [u32; 2],
    pub engine: u32,
    pub rules: u32,
    pub provider: u32,
    pub provider_handle: [u32; 2],
}

/// Engine calls reached by `fn_2ab7b4` and `fn_22be08`: environment and
/// list scans, broker/provider resolution, binary-chunk readers and the
/// text/binary compilers (composed for real through `RuleScanHost` and
/// `RuleInterpreterHost`). Data bytes are borrowed; chunk payloads are
/// opaque host tokens. Overlapping `resolve`/`alloc`/`free`/`log` calls use
/// the `RuleScanHost` spelling.
pub trait CompileHost: RuleInterpreterHost + RuleScanHost {
    fn insert_binary(&mut self, list: u32, slot: u32) -> bool;
    fn open_reader(&mut self, h0: u32, h1: u32, name: &[u8], version: u32) -> (u32, u32);
    fn read_chunk(&mut self, reader: u32, out: &mut [u8]) -> (u32, [u8; 8], u32);
    fn read_bytes_out(&mut self, reader: u32, offset: u32, out: &mut [u8]) -> (u32, u32);
    fn read_bytes_arena(
        &mut self,
        reader: u32,
        offset: u32,
        arena: &mut Arena<'_>,
        dest: u32,
    ) -> (u32, u32);
    fn close_reader(&mut self, reader: u32) -> u32;
    fn release_reader(&mut self, reader: u32) -> u32;
}

/// fn_2ab7b4 / EU fn_2ab970 / KR fn_2aba1c.
///
/// Compiles the rule text or binary `data` into the initialized `item`,
/// running the header/data scanners for text and the chunk reader for
/// binary blobs.
#[allow(clippy::too_many_arguments)]
pub fn compile_ruleset(
    arena: &mut Arena<'_>,
    item: &mut RuleItem,
    name: Option<&[u8]>,
    alternate: Option<&[u8]>,
    mime: &[u8],
    data: &[u8],
    pos: &mut u32,
    rem: &mut u32,
    buf: &mut [u8],
    host: &mut impl CompileHost,
) -> u32 {
    if data.is_empty() {
        return 0x88602007;
    }
    if item.name != 0 {
        return 0x88602011;
    }
    item.alternate = 0;
    let name = match name {
        Some(name) if !name.is_empty() && name[0] != 0 => name,
        _ => b"in-memory RETT",
    };
    let (status, ctx) = <_ as RuleScanHost>::resolve(host, item.handle[0], item.handle[1]);
    if status >= 0x80000000 {
        return 0x88602007;
    }
    let Some(ctx) = ctx else {
        return 0x88602007;
    };
    let params = ctx.params;
    let log = ctx.log;
    let heap = ctx.heap;
    item.name = <_ as RuleScanHost>::alloc(host, arena, heap, scan_strlen(name) as u32 + 1);
    if item.name == 0 {
        <_ as RuleScanHost>::log_error(host, 0x2afa);
        return 0x8860200a;
    }
    arena_copy_from(arena, item.name, name, 0);
    {
        let mut holder = [0u8; 128];
        let status = host.get_param(params, b"__NUAN_alternative_esc_seq__", &mut holder);
        let _ = holder;
        if status < 0x80000000 && arena.str(item.name) == b"__NUAN_alternative_esc_seq__" {
            item.in_memory = 1;
        }
    }
    if let Some(alt) = alternate {
        item.alternate = <_ as RuleScanHost>::alloc(host, arena, heap, scan_strlen(alt) as u32 + 1);
        if item.alternate == 0 {
            <_ as RuleScanHost>::log_error(host, 0x2afa);
            release_rule_item(arena, item, host);
            return 0x8860200a;
        }
        arena_copy_from(arena, item.alternate, alt, 0);
    }
    <_ as RuleScanHost>::log(host, 1);
    let mime = &mime[..mime.iter().position(|&b| b == 0).unwrap_or(mime.len())];
    if mime != b"application/x-vocalizer-rettt+bin"
        && mime != b"application/x-vocalizer-rettt+bin;loader=broker"
    {
        let mut result = scan_header(arena, item, item.stream, pos, rem, buf, data, host);
        if result < 0x80000000 {
            result = scan_rules(arena, item, item.stream, pos, rem, buf, data, host);
        }
        return fail_compile(arena, item, host, result);
    }
    let (status, _) = <_ as RuleScanHost>::resolve(host, item.handle[0], item.handle[1]);
    if status >= 0x80000000 {
        return fail_compile(arena, item, host, 0x88602007);
    }
    let (status, reader) = host.open_reader(item.handle[0], item.handle[1], name, 0x1b030205);
    let mut result = status;
    if result >= 0x80000000 {
        return fail_compile(arena, item, host, result);
    }
    let mut chunk = [0u8; 512];
    loop {
        let (status, tag, size) = host.read_chunk(reader, &mut chunk);
        result = status;
        if result >= 0x80000000 {
            break;
        }
        if tag_starts(&tag, b"LANG") {
            let (status, len) = host.read_bytes_out(reader, 0, &mut buf[..]);
            result = status;
            if result >= 0x80000000 {
                break;
            }
            if size == 0 {
                break;
            }
            let ok = host.check_stream(item.stream, b"ERCP4", &buf[..len as usize]);
            if ok >= 0x80000000 {
                <_ as RuleScanHost>::log_error(host, 0x2b0e);
                result = 0x88602007;
                break;
            }
            if item.language != 0 {
                <_ as RuleScanHost>::log_error(host, 0x2b0d);
                result = 0x88602007;
                break;
            }
            item.language = <_ as RuleScanHost>::alloc(host, arena, heap, len + 1);
            if item.language == 0 {
                result = 0x8860200a;
                break;
            }
            arena.put(item.language, &buf[..len as usize]);
        } else if tag_starts(&tag, b"TYPE") {
            let (status, len) = host.read_bytes_out(reader, 0, &mut buf[..]);
            result = status;
            if result >= 0x80000000 {
                break;
            }
            if size == 0 {
                break;
            }
            if item.kind != 0 {
                result = 0x88602007;
                break;
            }
            item.kind = <_ as RuleScanHost>::alloc(host, arena, heap, len + 1);
            if item.kind == 0 {
                result = 0x8860200a;
                break;
            }
            arena.put(item.kind, &buf[..len as usize]);
        } else if tag_starts(&tag, b"RULE") {
            let scan = size;
            let mut off = 0u32;
            while off < scan {
                if (off as usize) + 4 > chunk.len() {
                    break;
                }
                let tag16 =
                    u16::from_le_bytes([chunk[off as usize], chunk[off as usize + 1]]) as u32;
                off = off.wrapping_add(2);
                let slot = off.wrapping_add(2);
                off = off.wrapping_add(2).wrapping_add(tag16);
                let (status, len) = host.read_bytes_arena(reader, off, arena, 0);
                result = status;
                if result != 0x88602009 {
                    break;
                }
                let rule_buf = <_ as RuleScanHost>::alloc(host, arena, heap, len + 1);
                if rule_buf == 0 {
                    result = 0x8860200a;
                    break;
                }
                arena.put_byte(rule_buf, 0);
                let (status, len) = host.read_bytes_arena(reader, off, arena, rule_buf);
                result = status;
                if result >= 0x80000000 {
                    <_ as RuleScanHost>::free(host, heap, rule_buf);
                    break;
                }
                if !host.insert_binary(item.rules, slot) {
                    <_ as RuleScanHost>::free(host, heap, rule_buf);
                    <_ as RuleScanHost>::log_error(host, 0x2b0f);
                    result = 0x88602006;
                    break;
                }
                <_ as RuleScanHost>::free(host, heap, rule_buf);
                off = off.wrapping_add(len);
                let mut pad = tag16.wrapping_add(len) & 3;
                if pad != 0 {
                    pad = 4 - pad;
                    off = off.wrapping_add(pad);
                }
            }
        } else {
            host.error(log, 4);
        }
        let status = host.close_reader(reader);
        if status >= 0x80000000 {
            if result < 0x80000000 {
                result = status;
            }
            break;
        }
        if result >= 0x80000000 {
            break;
        }
    }
    if result == 0x88602014 {
        result = 0;
    }
    let status = host.release_reader(reader);
    if status >= 0x80000000 && result < 0x80000000 {
        result = status;
    }
    fail_compile(arena, item, host, result)
}

fn tag_starts(tag: &[u8; 8], prefix: &[u8]) -> bool {
    tag.len() >= prefix.len() && tag[..prefix.len()] == *prefix
}

fn fail_compile(
    arena: &mut Arena<'_>,
    item: &mut RuleItem,
    host: &mut impl CompileHost,
    result: u32,
) -> u32 {
    // The shared `fin:` epilogue: log the outcome, releasing the item on
    // failure like `fn_2ab40c`.
    <_ as RuleScanHost>::log(host, 1);
    if result >= 0x80000000 {
        release_rule_item(arena, item, host);
    }
    result
}

/// Fetch-level engine calls (`fn_22be08` source resolution).
pub trait FetchHost: CompileHost {
    fn validate(&mut self, object: u32, arg: u32) -> u32;
    fn check_duplicate(&mut self, list: u32, name: &[u8]) -> u32;
    fn find_broker(&mut self, format: &[u8]) -> bool;
    fn open_broker(&mut self, name: &[u8]) -> (u32, u32, u32);
    fn broker_param(&mut self, params: u32, key: &[u8], out: &mut [u8]) -> (u32, usize);
    fn map_broker(&mut self, broker: u32, handle: u32, out: &mut [u8]) -> (u32, usize);
    fn unmap_broker(&mut self, broker: u32, handle: u32);
    fn close_broker(&mut self, broker: u32, handle: u32);
    fn provider_load(
        &mut self,
        provider: u32,
        handle: [u32; 2],
        name: &[u8],
        flags: u32,
        options: u32,
        data_out: &mut [u8],
        alt_out: &mut [u8],
    ) -> (u32, u32, Option<usize>, usize);
    fn provider_unload(&mut self, provider: u32, handle: [u32; 2], state: u32);
    fn log_broker(&mut self, log: u32);
    fn insert_compiled(&mut self, list: u32, item: &RuleItem) -> bool;
}

/// fn_22be08 / EU fn_22bfc4 / KR fn_22c070.
///
/// Fetches a rule set by name or inline source, compiling it through the
/// text or binary loaders into a fresh interpreter item. `source_data` and
/// `source_alt` are caller scratch for broker/provider payloads.
#[allow(clippy::too_many_arguments)]
pub fn fetch_ruleset(
    arena: &mut Arena<'_>,
    rule: &mut RuleItem,
    fetcher: &RuleFetcher,
    object: u32,
    arg: u32,
    format: Option<&[u8]>,
    name: Option<&[u8]>,
    source: Option<&[u8]>,
    flags: u32,
    options: u32,
    output: Option<&mut [u32; 2]>,
    pos: &mut u32,
    rem: &mut u32,
    buf: &mut [u8],
    source_data: &mut [u8],
    source_alt: &mut [u8],
    host: &mut impl FetchHost,
) -> u32 {
    let Some(output) = output else {
        return 0x88602007;
    };
    let Some(format) = format else {
        return 0x88602007;
    };
    if name.is_none() {
        match source {
            Some(data) if !data.is_empty() => (),
            _ => return 0x88602007,
        }
    }
    let mut result = host.validate(object, arg);
    if result >= 0x80000000 {
        return result;
    }
    output[0] = 0;
    output[1] = 0;
    let (status, _) = <_ as RuleScanHost>::resolve(host, fetcher.session[0], fetcher.session[1]);
    if status >= 0x80000000 {
        return 0x88602007;
    }
    result = 0x88602007;
    if flags == 0 {
        if let Some(name) = name {
            if !name.is_empty() && name[0] != 0 {
                result = host.check_duplicate(fetcher.rules, name);
                if result >= 0x80000000 {
                    return result;
                }
            }
        }
    }
    // Resolve the source bytes: inline when provided, else broker/provider.
    let mut data: &[u8] = &[];
    let mut alternate: Option<&[u8]> = None;
    let mut broker_handle = 0u32;
    let mut provider_state = 0u32;
    let mut broker = 0u32;
    match source {
        Some(inline) if !inline.is_empty() => {
            data = inline;
        }
        _ => {
            if host.find_broker(format) {
                let (status, context_broker, handle) = host.open_broker(name.unwrap_or(&[]));
                result = status;
                if result >= 0x80000000 {
                    return result;
                }
                broker = context_broker;
                broker_handle = handle;
                let (status, len) = host.map_broker(broker, handle, source_data);
                result = status;
                if result >= 0x80000000 {
                    host.close_broker(broker, broker_handle);
                    return result;
                }
                data = &source_data[..len];
            } else {
                if fetcher.provider == 0 {
                    host.log_broker(0);
                    result = 0x88602001;
                    return result;
                }
                let (status, state, alt, len) = host.provider_load(
                    fetcher.provider,
                    fetcher.provider_handle,
                    name.unwrap_or(&[]),
                    flags,
                    options,
                    source_data,
                    source_alt,
                );
                result = status;
                if result >= 0x80000000 {
                    return result;
                }
                provider_state = state;
                alternate = alt.map(|len| &source_alt[..len]);
                data = &source_data[..len];
            }
        }
    }
    result = init_rule_item(
        fetcher.session[0],
        fetcher.session[1],
        fetcher.engine,
        [0, 0, 0],
        rule,
        host,
    );
    if result >= 0x80000000 {
        return cleanup_fetch(
            host,
            broker,
            broker_handle,
            data,
            provider_state,
            fetcher,
            result,
        );
    }
    *pos = 0;
    *rem = data.len() as u32;
    result = compile_ruleset(
        arena, rule, name, alternate, format, data, pos, rem, buf, host,
    );
    if result < 0x80000000 {
        if host.insert_compiled(fetcher.rules, rule) {
            output[0] = rule.stream[0];
            output[1] = 0x12c;
            return cleanup_fetch(
                host,
                broker,
                broker_handle,
                data,
                provider_state,
                fetcher,
                result,
            );
        }
        result = 0x8860200a;
    }
    destroy_rule_item(arena, rule, host);
    cleanup_fetch(
        host,
        broker,
        broker_handle,
        data,
        provider_state,
        fetcher,
        result,
    )
}

#[allow(clippy::too_many_arguments)]
fn cleanup_fetch(
    host: &mut impl FetchHost,
    broker: u32,
    broker_handle: u32,
    _data: &[u8],
    provider_state: u32,
    fetcher: &RuleFetcher,
    result: u32,
) -> u32 {
    if broker_handle != 0 {
        host.unmap_broker(broker, broker_handle);
        host.close_broker(broker, broker_handle);
    }
    if provider_state != 0 {
        host.provider_unload(fetcher.provider, fetcher.provider_handle, provider_state);
    }
    result
}
