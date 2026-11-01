
fn word(mem: &[u8], at: u32) -> u32 {
    u32::from_le_bytes(mem[at as usize..at as usize + 4].try_into().unwrap())
}
fn store(mem: &mut [u8], at: u32, value: u32) {
    mem[at as usize..at as usize + 4].copy_from_slice(&value.to_le_bytes());
}

pub trait SchedulerHooks {
    fn validate(&mut self, object: u32, tag: u32, magic: u32, size: u32) -> u32;

    fn resolve(&mut self, channel: bool, handle: [u32; 2]) -> (u32, u32, u32);
    fn close_channel(&mut self, context: u32, object: &mut u32) -> u32;
    fn enter(
        &mut self,
        mem: &mut [u8],
        context: u32,
        object: u32,
        name: &[u8],
        previous: &mut u8,
    ) -> u32;
    fn acquire(&mut self, mutex: u32) -> u32;
    fn release(&mut self, mutex: u32) -> u32;
    fn restore(&mut self, mem: &mut [u8], context: u32, object: u32, previous: u8);
    fn load(&mut self, mem: &mut [u8], resources: u32, request: &ResourceLoad) -> u32;
    fn unload(&mut self, mem: &mut [u8], resources: u32, resource: u32, arg: u32) -> u32;
    fn resource_method(
        &mut self,
        mem: &mut [u8],
        enabled: bool,
        handle: [u32; 2],
        resource: u32,
        arg: u32,
    ) -> u32;
}

pub fn lingdb_class_close() -> u32 {
    0
}

pub fn lingdb_unsupported() -> u32 {
    0x89202001
}

pub fn lingdb_table_value(table: &[[u32; 2]], index: u32, output: &mut u32) -> u32 {

    *output = table[(index.wrapping_mul(8) / 8) as usize][1];
    0
}

pub fn reopen_lingdb(object: u32, tag: u32, hooks: &mut impl SchedulerHooks) -> u32 {
    if hooks.validate(object, tag, 0xb2c2, 0x38) >= 0x80000000 {
        0x89202008
    } else {
        0
    }
}

pub fn close_scheduler_channel(handle: [u32; 2], hooks: &mut impl SchedulerHooks) -> u32 {
    let (status, context, mut object) = hooks.resolve(true, handle);
    if status >= 0x80000000 {
        return 0x81202008;
    }
    hooks.close_channel(context, &mut object)
}

pub fn scheduler_contains(
    mem: &[u8],
    handle: [u32; 2],
    candidate: u32,
    hooks: &mut impl SchedulerHooks,
) -> u32 {
    let (status, _, object) = hooks.resolve(false, handle);
    if status >= 0x80000000 {
        return 0;
    }
    let mut node = word(mem, object.wrapping_add(0x140));
    loop {
        if node == candidate {
            return 1;
        }
        node = word(mem, node.wrapping_add(0x80));
        if node == 0 {
            return 0;
        }
    }
}

pub struct ResourceLoad {
    pub handle: [u32; 2],
    pub name: Option<u32>,
    pub uri: Option<u32>,
    pub data: Option<u32>,
    pub size: u32,
    pub blob: [u32; 2],
    pub output: Option<u32>,
}
fn enter_resource(mem: &mut [u8], object: u32, hooks: &mut impl SchedulerHooks) -> (u32, u8) {
    let mutex = word(mem, object.wrapping_add(0x2c));
    let mut status = hooks.acquire(mutex);
    if status >= 0x80000000 {
        return (status, 0);
    }
    let at = object.wrapping_add(0x30) as usize;
    let mut old = 0;
    match mem[at] {
        1 | 3 | 4 | 5 | 6 => {
            old = mem[at];
            mem[at] = 2;
        }
        _ => status = 0x81202011,
    }
    let release = hooks.release(mutex);
    if release >= 0x80000000 && status < 0x80000000 {
        status = release;
    }
    (status, old)
}

pub fn load_scheduler_resource(
    mem: &mut [u8],
    request: &ResourceLoad,
    hooks: &mut impl SchedulerHooks,
) -> u32 {
    if (request.uri.is_none() && (request.data.is_none() || request.size == 0))
        || request.name.is_none()
        || request.output.is_none()
    {
        return 0x81202007;
    }
    let output = request.output.unwrap();
    store(mem, output, 0);
    store(mem, output.wrapping_add(4), 0);
    let (status, context, object) = hooks.resolve(false, request.handle);
    if status >= 0x80000000 {
        return status;
    }
    let (status, previous) = enter_resource(mem, object, hooks);
    if status >= 0x80000000 {
        return status;
    }
    let resources = word(mem, object.wrapping_add(0x7c));
    let status = if resources == 0 {
        0x81202011
    } else {
        hooks.load(mem, resources, request)
    };
    hooks.restore(mem, context, object, previous);
    status
}

pub fn unload_scheduler_resource(
    mem: &mut [u8],
    handle: [u32; 2],
    resource: u32,
    arg: u32,
    hooks: &mut impl SchedulerHooks,
) -> u32 {
    let (status, context, object) = hooks.resolve(false, handle);
    if status >= 0x80000000 {
        return status;
    }
    let (status, previous) = enter_resource(mem, object, hooks);
    if status >= 0x80000000 {
        return status;
    }
    let resources = word(mem, object.wrapping_add(0x7c));
    let status = if resources == 0 {
        0x81202011
    } else {
        hooks.unload(mem, resources, resource, arg)
    };
    hooks.restore(mem, context, object, previous);
    status
}
fn set_resource_enabled(
    mem: &mut [u8],
    handle: [u32; 2],
    resource: u32,
    arg: u32,
    enabled: bool,
    hooks: &mut impl SchedulerHooks,
) -> u32 {
    let (status, context, object) = hooks.resolve(false, handle);
    if status >= 0x80000000 {
        return status;
    }
    let mut previous = 0;
    let status = hooks.enter(
        mem,
        context,
        object,
        if enabled {
            b"ttseg_ResourceEnable\0"
        } else {
            b"ttseg_ResourceDisable\0"
        },
        &mut previous,
    );
    if status >= 0x80000000 {
        return status;
    }
    let list = word(mem, object.wrapping_add(0x7c));
    let mut status = 0x81202011;
    if list != 0 {
        status = 0x81202008;
        let mut i = 0u16;
        while status & 0x1fff == 8 {

            let list = word(mem, object.wrapping_add(0x7c));
            let count_at = list.wrapping_add(12) as usize;
            let count = u16::from_le_bytes(mem[count_at..count_at + 2].try_into().unwrap());
            if i >= count {
                break;
            }
            let array = word(mem, list.wrapping_add(8));
            let item = word(mem, array.wrapping_add(u32::from(i) * 4));
            let vtable = word(mem, item.wrapping_add(8));
            if word(mem, vtable.wrapping_add(if enabled { 0x28 } else { 0x2c })) != 0 {
                let handle = [
                    word(mem, item.wrapping_add(12)),
                    word(mem, item.wrapping_add(16)),
                ];
                status = hooks.resource_method(mem, enabled, handle, resource, arg);
                if status < 0x80000000 {
                    break;
                }
            }
            i = i.wrapping_add(1);
        }
    }
    hooks.restore(mem, context, object, previous);
    status
}

pub fn enable_scheduler_resource(
    mem: &mut [u8],
    handle: [u32; 2],
    resource: u32,
    arg: u32,
    hooks: &mut impl SchedulerHooks,
) -> u32 {
    set_resource_enabled(mem, handle, resource, arg, true, hooks)
}

pub fn disable_scheduler_resource(
    mem: &mut [u8],
    handle: [u32; 2],
    resource: u32,
    arg: u32,
    hooks: &mut impl SchedulerHooks,
) -> u32 {
    set_resource_enabled(mem, handle, resource, arg, false, hooks)
}

pub trait ProductFormatter {
    fn version(&mut self, mem: &mut [u8], output: u32, full: bool);
}
fn product_list(
    mem: &mut [u8],
    output: Option<u32>,
    count: Option<u32>,
    validation: u32,
    bad_handle: u32,
    bad_argument: u32,
    formatter: &mut impl ProductFormatter,
) -> u32 {
    if validation >= 0x80000000 {
        return bad_handle;
    }
    let Some(count) = count else {
        return bad_argument;
    };
    let requested = word(mem, count);
    store(mem, count, 1);
    if requested != 0 {
        if let Some(output) = output {
            formatter.version(mem, output, false);
            let at = output.wrapping_add(64) as usize;
            mem[at..at + 14].copy_from_slice(b"eng/vocalizer\0");
            formatter.version(mem, output.wrapping_add(96), true);
        }
    }
    validation
}

pub fn scheduler_products(
    mem: &mut [u8],
    handle: [u32; 2],
    output: Option<u32>,
    count: Option<u32>,
    hooks: &mut (impl SchedulerHooks + ProductFormatter),
) -> u32 {
    let (validation, _, _) = hooks.resolve(true, handle);
    product_list(
        mem, output, count, validation, 0x81202008, 0x81202007, hooks,
    )
}

pub fn query_products(
    mem: &mut [u8],
    object: u32,
    tag: u32,
    output: Option<u32>,
    count: Option<u32>,
    hooks: &mut (impl SchedulerHooks + ProductFormatter),
) -> u32 {
    let validation = hooks.validate(object, tag, 348, 32);
    product_list(
        mem, output, count, validation, 0x89502008, 0x89502007, hooks,
    )
}

pub trait SchedulerCoreHooks: SchedulerHooks {
    fn close_instance(&mut self, mem: &mut [u8], context: u32, force: u32, object: &mut u32)
        -> u32;
    fn output_step(&mut self, mem: &mut [u8], process: bool, object: u32, mode: u32) -> u32;
}

pub fn close_scheduler_instance(
    mem: &mut [u8],
    handle: [u32; 2],
    hooks: &mut impl SchedulerCoreHooks,
) -> u32 {
    let (rc, context, mut object) = hooks.resolve(false, handle);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut previous = 0;
    let rc = hooks.enter(mem, context, object, b"ttseg_ObjClose\0", &mut previous);
    if rc >= 0x80000000 {
        return rc;
    }
    hooks.close_instance(mem, context, 1, &mut object) & 0x7fffffff
}

pub fn break_scheduler(mem: &mut [u8], handle: [u32; 2], hooks: &mut impl SchedulerHooks) -> u32 {
    let (mut rc, _, object) = hooks.resolve(false, handle);
    if rc >= 0x80000000 {
        return rc;
    }
    rc = hooks.acquire(word(mem, object.wrapping_add(0x2c)));
    if rc >= 0x80000000 {
        return rc;
    }
    let state = object.wrapping_add(0x30) as usize;
    if matches!(mem[state], 3 | 4) {
        mem[state] = 6;

    } else {
        rc = 0x81202011;
    }
    let release = hooks.release(word(mem, object.wrapping_add(0x2c)));
    if release >= 0x80000000 && rc < 0x80000000 {
        rc = release;
    }
    rc
}

pub fn scheduler_output_available(
    mem: &mut [u8],
    object: u32,
    tag: u32,
    mode: u32,
    length: u32,
    markers: u32,
    hooks: &mut impl SchedulerCoreHooks,
) -> u32 {
    let mut rc = hooks.validate(object, tag, 0xcdac, 332);
    if rc >= 0x80000000 {
        return 0x81202007;
    }
    if mode == 0 && word(mem, object.wrapping_add(0xe4)) == 0 {
        rc = hooks.output_step(mem, false, object, 0);
    } else if mode == 2 && mem[object.wrapping_add(0xe2) as usize] != 2 {
        rc = hooks.output_step(mem, true, object, 1);
    }
    if rc < 0x80000000 && mode != 1 {
        if word(mem, object.wrapping_add(0x18)) != 0 {
            store(mem, length, word(mem, object.wrapping_add(0xe8)));
            store(mem, markers, word(mem, object.wrapping_add(0xec)));
        } else {
            store(mem, length, 4096);
            store(mem, markers, 0);
        }
    } else {
        store(mem, length, 0);
        store(mem, markers, 0);
    }
    rc
}

pub trait SchedulerLicenseHooks {
    fn ports(&mut self, feature: u32) -> u32;
    fn token(&mut self, feature: u32) -> u32;
    fn allocate(&mut self, feature: u32, requested: u32, available: &mut u32) -> u32;
    fn notify(&mut self, context: u32, channel: u32, kind: u32, level: u32);
    fn log_usage(
        &mut self,
        logger: u32,
        level: u32,
        used: u32,
        limit: u32,
        threshold: u32,
        description: &[u8],
    );
}

pub fn scheduler_license_usage(
    mem: &[u8],
    context: u32,
    channel: u32,
    kind: u32,
    level: u32,
    hooks: &mut impl SchedulerLicenseHooks,
) {
    if kind > 1 {
        return;
    }
    let mut text = [0u8; 20];
    text[..3].copy_from_slice(if kind == 0 { b"tts" } else { b"cpr" });
    let len = if mem[channel.wrapping_add(0x94) as usize] == 0 {
        text[3..15].copy_from_slice(b",unthrottled");
        15
    } else {
        3
    };
    let base = channel.wrapping_add(0x30).wrapping_add(kind * 24);
    let used = hooks
        .ports(if kind == 0 { 0 } else { 3 })
        .wrapping_add(word(mem, base.wrapping_add(12)));
    hooks.log_usage(
        word(mem, context.wrapping_add(0x10)),
        level,
        used,
        word(mem, base.wrapping_add(4)).wrapping_add(word(mem, base.wrapping_add(16))),
        word(mem, base.wrapping_add(8)),
        &text[..len + 1],
    );
}

pub fn configure_scheduler_license(
    mem: &mut [u8],
    context: u32,
    channel: u32,
    feature: u32,
    mode: u32,
    hooks: &mut impl SchedulerLicenseHooks,
) -> u32 {
    if mode == 2 {
        store(mem, channel.wrapping_add(0x64), 2);
        store(mem, channel.wrapping_add(0x68), 2);
        return 0;
    }
    let kind = match feature {
        0 => 0,
        3 => 1,
        4 => 3,
        _ => return 0x81202007,
    };
    let base = channel.wrapping_add(0x30).wrapping_add(kind * 24);

    let requested = if kind == 3 { 1 } else { word(mem, base) };
    if kind != 3 && requested == 0 {
        return 0;
    }
    let mut available = 0;
    let mut rc = hooks.allocate(feature, requested, &mut available);
    if kind != 3 {
        store(
            mem,
            base.wrapping_add(4),
            if rc == 0 { available } else { 0 },
        );
        if rc == 0 && word(mem, base.wrapping_add(4)) < word(mem, base) {
            rc = u32::MAX;
        }
    }
    if rc == 0 || (kind == 3 && rc == 4) {
        hooks.notify(context, channel, kind, 10);
        0
    } else if rc == 4 {
        0x81203d07
    } else {
        0x81203d06
    }
}

pub trait SchedulerStreamHooks {
    fn validate(&mut self, handle: [u32; 2], magic: u32, size: u32) -> u32;
    fn context(&mut self, handle: [u32; 2], context: &mut u32) -> u32;
    fn begin(&mut self, streams: u32, iterator: &mut u32) -> u32;
    fn next(&mut self, iterator: u32, key: &mut u32, state: &mut u32) -> u32;
    fn clear(&mut self, iterator: u32);
    fn end(&mut self, iterator: u32);
}

pub fn reset_scheduler_streams(
    mem: &[u8],
    handle: [u32; 2],
    position: u32,
    hooks: &mut impl SchedulerStreamHooks,
) -> u32 {
    let rc = hooks.validate(handle, 0x47dd, 32);
    if rc >= 0x80000000 {
        return rc;
    }
    let config = handle[0];
    let mut context = 0;
    let rc = hooks.context(
        [word(mem, config), word(mem, config.wrapping_add(4))],
        &mut context,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut iterator = 0;
    let rc = hooks.begin(word(mem, config.wrapping_add(12)), &mut iterator);
    if rc < 0x80000000 {
        let mut key = 0;
        let mut state = 0;
        while hooks.next(iterator, &mut key, &mut state) < 0x80000000 {
            let at = state.wrapping_add(0x14) as usize;
            let pos = u16::from_le_bytes(mem[at..at + 2].try_into().unwrap());
            let refs = u16::from_le_bytes(mem[at + 2..at + 4].try_into().unwrap());
            if refs == 1 && u32::from(pos) >= position {
                hooks.clear(iterator);
            }
        }
        hooks.end(iterator);
    }
    if rc & 0x1fff == 20 {
        0
    } else {
        rc
    }
}

pub trait SchedulerListHooks: SchedulerHooks {
    fn enumerate(&mut self, mem: &mut [u8], slot: u32, args: &[u32]) -> u32;
}
fn scheduler_list(
    mem: &mut [u8],
    handle: [u32; 2],
    product: u32,
    slot: u32,
    tail: &[u32],
    hooks: &mut impl SchedulerListHooks,
) -> u32 {
    let (rc, _, object) = hooks.resolve(true, handle);
    if rc >= 0x80000000 {
        return 0x81202008;
    }
    if word(mem, object.wrapping_add(0x10)) == 0 {
        return 0x81202001;
    }
    let mut args = [0; 7];
    args[0] = word(mem, object.wrapping_add(0x14));
    args[1] = word(mem, object.wrapping_add(0x18));
    args[2] = product.wrapping_add(0x40);
    args[3..3 + tail.len()].copy_from_slice(tail);
    hooks.enumerate(mem, slot, &args[..3 + tail.len()])
}

pub fn scheduler_languages(
    mem: &mut [u8],
    handle: [u32; 2],
    product: u32,
    entries: u32,
    count: u32,
    hooks: &mut impl SchedulerListHooks,
) -> u32 {
    scheduler_list(mem, handle, product, 0x20, &[entries, count], hooks)
}

pub fn scheduler_voices(
    mem: &mut [u8],
    handle: [u32; 2],
    product: u32,
    language: u32,
    selection_bits: u32,
    count: u32,
    hooks: &mut impl SchedulerListHooks,
) -> u32 {
    scheduler_list(
        mem,
        handle,
        product,
        0x24,
        &[language, selection_bits, count],
        hooks,
    )
}

pub fn scheduler_documents(
    mem: &mut [u8],
    handle: [u32; 2],
    product: u32,
    language: u32,
    selection_bits: u32,
    count: u32,
    hooks: &mut impl SchedulerListHooks,
) -> u32 {
    scheduler_list(
        mem,
        handle,
        product,
        0x28,
        &[language, selection_bits, count],
        hooks,
    )
}

pub fn scheduler_speech_databases(
    mem: &mut [u8],
    handle: [u32; 2],
    product: u32,
    language: u32,
    selection_bits: [u32; 2],
    count: u32,
    hooks: &mut impl SchedulerListHooks,
) -> u32 {
    scheduler_list(
        mem,
        handle,
        product,
        0x2c,
        &[language, selection_bits[0], selection_bits[1], count],
        hooks,
    )
}

pub trait SchedulerReadHooks: SchedulerHooks {
    fn reclaim(&mut self, mem: &mut [u8], state: u32);
}

pub fn read_scheduler_stream(
    mem: &mut [u8],
    handle: [u32; 2],
    buffer: Option<u32>,
    count: u32,
    output: u32,
    hooks: &mut impl SchedulerReadHooks,
) -> u32 {
    let rc = hooks.validate(handle[0], handle[1], 0x47de, 12);
    if rc >= 0x80000000 {
        return rc;
    }
    let stream = handle[0];
    let state = word(mem, stream.wrapping_add(4));
    let position = word(mem, stream.wrapping_add(8));
    let base = word(mem, state.wrapping_add(0x38));
    let invalid = if buffer.is_none() || count == 0 || mem[stream as usize] != 0 {
        0x88f02007
    } else if position < base {
        0x88f02004
    } else {
        0
    };
    if invalid != 0 {
        store(mem, output, 0);
        return invalid;
    }
    let bytes = count.min(word(mem, state.wrapping_add(0x2c)).wrapping_sub(position));
    store(mem, output, bytes);
    if bytes != 0 {
        let state = word(mem, stream.wrapping_add(4));
        let source = word(mem, state.wrapping_add(0x3c)).wrapping_add(position.wrapping_sub(base));
        mem.copy_within(
            source as usize..source.wrapping_add(bytes) as usize,
            buffer.unwrap() as usize,
        );
        let next = word(mem, output).wrapping_add(word(mem, stream.wrapping_add(8)));
        store(mem, stream.wrapping_add(8), next);
        let state = word(mem, stream.wrapping_add(4));
        let unused = word(mem, state.wrapping_add(0x30)).max(next);
        store(mem, state.wrapping_add(0x30), unused);
        let state = word(mem, stream.wrapping_add(4));
        if word(mem, state.wrapping_add(0x20)) == 0 {
            hooks.reclaim(mem, state);
        }
    }
    rc
}

pub fn increase_scheduler_license(
    mem: &mut [u8],
    context: u32,
    object: u32,
    locks: &mut impl SchedulerHooks,
    licensing: &mut impl SchedulerLicenseHooks,
) -> u32 {
    if mem[object.wrapping_add(0x6a) as usize] != 0 {
        return 0x81202011;
    }
    let mut kind = u32::from(mem[object.wrapping_add(0x69) as usize]);
    let channel = word(mem, object.wrapping_add(0x10));
    let mut rc = locks.acquire(word(mem, channel.wrapping_add(0x90)));
    if rc >= 0x80000000 {
        return rc;
    }
    if kind == 2 {
        let base = channel.wrapping_add(0x60);
        if word(mem, base.wrapping_add(12)) < word(mem, base.wrapping_add(4)) {
            store(
                mem,
                base.wrapping_add(12),
                word(mem, base.wrapping_add(12)).wrapping_add(1),
            );
            mem[object.wrapping_add(0x6a) as usize] = 1;
        } else {
            rc = 0x81203d06;
        }
        licensing.notify(context, channel, kind, if rc == 0x81203d06 { 7 } else { 5 });
    } else if kind <= 1 {
        if kind == 1
            && word(mem, channel.wrapping_add(0x54)) >= word(mem, channel.wrapping_add(0x4c))
            && licensing.ports(3) >= word(mem, channel.wrapping_add(0x58))
            && (word(mem, channel.wrapping_add(0x3c)) < word(mem, channel.wrapping_add(0x34))
                || licensing.ports(0) < word(mem, channel.wrapping_add(0x58)))
        {
            licensing.notify(context, channel, kind, 7);
            kind = 0;
            mem[object.wrapping_add(0x69) as usize] = 0;
        }
        let base = channel.wrapping_add(0x30).wrapping_add(kind * 24);
        let feature = if kind == 0 { 0 } else { 3 };
        if word(mem, base.wrapping_add(4)) > word(mem, base.wrapping_add(12)) {
            store(
                mem,
                base.wrapping_add(12),
                word(mem, base.wrapping_add(12)).wrapping_add(1),
            );
            mem[object.wrapping_add(0x6a) as usize] = 1;
        } else if licensing.ports(feature) < word(mem, base.wrapping_add(16)) {
            let token = licensing.token(feature);
            store(mem, object.wrapping_add(0x6c), token);
            if token != u32::MAX {
                mem[object.wrapping_add(0x6a) as usize] = 2;
            }
        }
        let level = if mem[object.wrapping_add(0x6a) as usize] == 0 {
            rc = 0x81203d06;
            7
        } else {
            if licensing
                .ports(feature)
                .wrapping_add(word(mem, base.wrapping_add(12)))
                > word(mem, base.wrapping_add(8))
            {

                licensing.ports(feature);
            }
            5
        };
        licensing.notify(context, channel, kind, level);
    }
    locks.release(word(
        mem,
        word(mem, object.wrapping_add(0x10)).wrapping_add(0x90),
    ));
    rc
}

pub trait SchedulerChannelCloseHooks {
    fn component_query(&mut self, component: u32, release: bool) -> u32;
    fn component_close(&mut self, pair: [u32; 2]) -> u32;
    fn component_unregister(&mut self, registry: u32, component: u32) -> u32;
    fn notify(&mut self, context: u32, channel: u32, kind: u32);
    fn mutex_close(&mut self, mutex: u32) -> u32;
    fn params_unlock(&mut self, params: u32) -> u32;
    fn free(&mut self, heap: u32, channel: u32);
}
fn first_error(status: &mut u32, next: u32) {
    if next >= 0x80000000 && *status < 0x80000000 {
        *status = next;
    }
}

pub fn destroy_scheduler_channel(
    mem: &mut [u8],
    context: u32,
    output: Option<u32>,
    hooks: &mut impl SchedulerChannelCloseHooks,
) -> u32 {
    let Some(output) = output else {
        return 0;
    };
    let channel = word(mem, output);
    if channel == 0 {
        return 0;
    }
    store(mem, output, 0);
    let mut rc = 0;
    if mem[channel.wrapping_add(0x1c) as usize] != 0 {
        first_error(
            &mut rc,
            hooks.component_query(word(mem, context.wrapping_add(0x18)), true),
        );
    } else if word(mem, context.wrapping_add(0x18)) != 0
        && word(mem, channel.wrapping_add(0x10)) != 0
    {
        first_error(
            &mut rc,
            hooks.component_query(word(mem, context.wrapping_add(0x18)), false),
        );
        store(mem, channel.wrapping_add(0x10), 0);
        store(mem, channel.wrapping_add(0x14), 0);
        store(mem, channel.wrapping_add(0x18), 0);
    }
    if word(mem, channel.wrapping_add(0x10)) != 0 {
        if word(mem, channel.wrapping_add(0x14)) != 0 {
            first_error(
                &mut rc,
                hooks.component_close([
                    word(mem, channel.wrapping_add(0x14)),
                    word(mem, channel.wrapping_add(0x18)),
                ]),
            );
        }
        first_error(
            &mut rc,
            hooks.component_unregister(
                word(mem, context.wrapping_add(0xc)),
                word(mem, channel.wrapping_add(0x10)),
            ),
        );
    }
    for kind in [1, 0, 2] {
        let limit = channel.wrapping_add(0x34).wrapping_add(kind * 24);
        if word(mem, limit) != 0 {
            hooks.notify(context, channel, kind);
            store(mem, limit, 0);
        }
    }
    let mutex = word(mem, channel.wrapping_add(0x90));
    if mutex != 0 {
        first_error(&mut rc, hooks.mutex_close(mutex));
    }
    if mem[channel.wrapping_add(0x1d) as usize] == 1 {
        first_error(
            &mut rc,
            hooks.params_unlock(word(mem, context.wrapping_add(0x14))),
        );
    }
    if rc < 0x80000000 {
        hooks.free(word(mem, context.wrapping_add(4)), channel);
    }
    rc
}

pub trait SchedulerQueryOpenHooks {
    fn context(&mut self, handle: [u32; 2], output: &mut u32) -> u32;
    fn allocate(&mut self, heap: u32) -> u32;
    fn interval(&mut self, params: u32, output: &mut u32) -> u32;
    fn mutex(&mut self, callbacks: u32, heap: u32, output: &mut u32) -> u32;
    fn database(&mut self, registry: u32, output: &mut u32) -> u32;
    fn component(&mut self, database: u32, output: &mut u32) -> u32;
    fn validate(&mut self, object: u32) -> u32;
    fn component_close(&mut self, component: u32) -> u32;
    fn mutex_close(&mut self, mutex: u32) -> u32;
    fn free(&mut self, heap: u32, object: u32);
}

pub fn open_scheduler_query(
    mem: &mut [u8],
    handle: [u32; 2],
    output: Option<u32>,
    hooks: &mut impl SchedulerQueryOpenHooks,
) -> u32 {
    let Some(output) = output else {
        return 0x89502006;
    };
    store(mem, output, 0);
    store(mem, output.wrapping_add(4), 0);
    let mut ctx = 0;
    let rc = hooks.context(handle, &mut ctx);
    if rc >= 0x80000000 {
        return rc;
    }
    let obj = hooks.allocate(word(mem, ctx.wrapping_add(4)));
    let mut rc = 0x8950200a;
    if obj != 0 {
        store(mem, obj, handle[0]);
        store(mem, obj.wrapping_add(4), handle[1]);
        store(mem, obj.wrapping_add(8), ctx);
        let mut interval = word(mem, obj.wrapping_add(0x14));
        hooks.interval(word(mem, ctx.wrapping_add(0x14)), &mut interval);
        store(mem, obj.wrapping_add(0x14), interval);
        let mut value = word(mem, obj.wrapping_add(0x1c));
        rc = hooks.mutex(
            word(mem, ctx.wrapping_add(8)),
            word(mem, ctx.wrapping_add(4)),
            &mut value,
        );
        store(mem, obj.wrapping_add(0x1c), value);
        if rc < 0x80000000 {
            let mut value = word(mem, obj.wrapping_add(0xc));
            rc = hooks.database(word(mem, ctx.wrapping_add(0xc)), &mut value);
            store(mem, obj.wrapping_add(0xc), value);
            if rc < 0x80000000 {
                let mut value = word(mem, obj.wrapping_add(0x10));
                rc = hooks.component(word(mem, obj.wrapping_add(0xc)), &mut value);
                store(mem, obj.wrapping_add(0x10), value);
                if rc < 0x80000000 {
                    store(mem, output, obj);
                    store(mem, output.wrapping_add(4), 0x15c);
                    return rc;
                }
            }
        }
    }
    store(mem, output, obj);
    store(mem, output.wrapping_add(4), 0x15c);
    let mut cleanup = hooks.validate(obj);
    if cleanup < 0x80000000 {
        let ctx = word(mem, obj.wrapping_add(8));
        let component = word(mem, obj.wrapping_add(0x10));
        if component != 0 {
            let next = hooks.component_close(component);
            if next >= 0x80000000 {
                cleanup = next;
            }
        }
        let mutex = word(mem, obj.wrapping_add(0x1c));
        if mutex != 0 {
            let next = hooks.mutex_close(mutex);
            store(mem, obj.wrapping_add(0x1c), 0);
            first_error(&mut cleanup, next);
        }
        hooks.free(word(mem, ctx.wrapping_add(4)), obj);
    }
    store(mem, output, 0);
    store(mem, output.wrapping_add(4), 0);
    rc
}

pub trait SchedulerMetadataHooks {
    fn validate(&mut self, object: u32, tag: u32) -> u32;
    fn component(&mut self, name: u32, database: u32, output: &mut u32) -> u32;
    fn iterator(&mut self, component: u32, output: &mut u32) -> u32;
    fn next(&mut self, iterator: u32, key: &mut u32) -> u32;
    fn tags(&mut self, database: u32, key: u32, output: &mut u32) -> u32;
    fn next_tag(&mut self, iterator: u32, tag: &mut u32, value: &mut u32) -> u32;
    fn end_tags(&mut self, iterator: u32);
    fn end(&mut self, iterator: u32);
    fn release(&mut self, component: u32);
}
fn text(mem: &[u8], pointer: u32) -> &[u8] {
    let tail = &mem[pointer as usize..];
    &tail[..tail.iter().position(|&b| b == 0).unwrap()]
}
fn metadata_list(
    mem: &mut [u8],
    object: u32,
    tag: u32,
    name: Option<u32>,
    filter: Option<u32>,
    entries: Option<u32>,
    count: Option<u32>,
    documents: bool,
    hooks: &mut impl SchedulerMetadataHooks,
) -> u32 {
    if hooks.validate(object, tag) >= 0x80000000 {
        return 0x89502008;
    }
    let (Some(name), Some(filter), Some(count)) = (name, filter, count) else {
        return 0x89502007;
    };
    let entries = if word(mem, count) == 0 { None } else { entries };
    let mut component = 0;
    let rc = hooks.component(name, word(mem, object.wrapping_add(12)), &mut component);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut iterator = 0;
    let mut rc = hooks.iterator(component, &mut iterator);
    if rc >= 0x80000000 {
        hooks.release(component);
        return rc;
    }
    let inner = word(mem, object.wrapping_add(8));
    let database = word(mem, inner.wrapping_add(12));
    let size = if documents { 184 } else { 136 };
    let mut key = 0;
    let mut total = 0u32;
    let mut tags = 0;
    while hooks.next(iterator, &mut key) < 0x80000000 {
        let mut entry = [0u8; 184];
        if documents {
            entry[..5].copy_from_slice(b"text\0");
            entry[108..136].copy_from_slice(b"Nuance Communications, Inc.\0");
        }
        if hooks.tags(database, key, &mut tags) < 0x80000000 {
            let mut tag = 0;
            let mut value = 0;
            while hooks.next_tag(tags, &mut tag, &mut value) < 0x80000000 {
                if tag == 0 {
                    continue;
                }
                let field = match text(mem, tag) {
                    b"language" => Some(32),
                    b"langversion" if documents => Some(172),
                    b"voice" if !documents => Some(0),
                    b"age" if !documents => Some(96),
                    b"gender" if !documents => Some(108),
                    b"voiceversion" if !documents => Some(124),
                    _ => None,
                };
                if let Some(at) = field {
                    let value = text(mem, value);
                    entry[at..at + value.len()].copy_from_slice(value);
                    entry[at + value.len()] = 0;
                }
            }
            hooks.end_tags(tags);
            tags = 0;
        }
        if !text(mem, filter).eq_ignore_ascii_case(text(&entry, 32)) {
            continue;
        }
        if let Some(entries) = entries {
            let mut duplicate = false;
            for index in 0..total {
                let at = entries.wrapping_add(index.wrapping_mul(size));
                if text(&entry, 0).eq_ignore_ascii_case(text(mem, at))
                    && text(&entry, 32).eq_ignore_ascii_case(text(mem, at.wrapping_add(32)))
                {
                    duplicate = true;
                    break;
                }
            }
            if duplicate {
                continue;
            }
            if word(mem, count) != 0 {
                if word(mem, count) <= total {
                    rc = 9;
                } else {
                    let at = entries.wrapping_add(total.wrapping_mul(size)) as usize;
                    mem[at..at + size as usize].copy_from_slice(&entry[..size as usize]);
                }
            }
        }
        total = total.wrapping_add(1);
    }
    store(mem, count, total);
    hooks.end(iterator);
    hooks.release(component);
    rc
}

pub fn query_scheduler_voices(
    mem: &mut [u8],
    object: u32,
    tag: u32,
    name: Option<u32>,
    filter: Option<u32>,
    entries: Option<u32>,
    count: Option<u32>,
    hooks: &mut impl SchedulerMetadataHooks,
) -> u32 {
    metadata_list(mem, object, tag, name, filter, entries, count, false, hooks)
}

pub fn query_scheduler_documents(
    mem: &mut [u8],
    object: u32,
    tag: u32,
    name: Option<u32>,
    filter: Option<u32>,
    entries: Option<u32>,
    count: Option<u32>,
    hooks: &mut impl SchedulerMetadataHooks,
) -> u32 {
    metadata_list(mem, object, tag, name, filter, entries, count, true, hooks)
}

pub trait SchedulerInstanceCloseHooks {
    fn pipeline_close(&mut self, pipeline: u32) -> u32;
    fn component_remove(&mut self, registry: u32, name: &[u8], release: bool) -> u32;
    fn stream_close(&mut self, handle: [u32; 2], stream: u32) -> u32;
    fn stream_destroy(&mut self, stream: u32);
    fn free(&mut self, heap: u32, object: u32);
    fn acquire(&mut self, mutex: u32) -> u32;
    fn release(&mut self, mutex: u32);
    fn release_token(&mut self, token: u32);
    fn notify(&mut self, context: u32, channel: u32, kind: u32);
    fn log_level(&mut self, logger: u32) -> u32;
    fn unregister_parameter(
        &mut self,
        params: u32,
        name: Option<&[u8]>,
        object: u32,
        callbacks: [u32; 2],
    );
    fn mutex_close(&mut self, mutex: u32) -> u32;
}
fn replace_error(status: &mut u32, next: u32) {
    if next >= 0x80000000 {
        *status = next;
    }
}

pub fn destroy_scheduler_instance(
    mem: &mut [u8],
    context: u32,
    force: u32,
    output: Option<u32>,
    parameter_names: &[&[u8]],
    callbacks: [u32; 2],
    hooks: &mut impl SchedulerInstanceCloseHooks,
) -> u32 {
    let Some(output) = output else {
        return 0;
    };
    let object = word(mem, output);
    if object == 0 {
        return 0;
    }
    store(mem, output, 0);
    let mut rc = 0;
    let pipeline = word(mem, object.wrapping_add(0x7c));
    if pipeline != 0 {
        replace_error(&mut rc, hooks.pipeline_close(pipeline));
        store(mem, object.wrapping_add(0x7c), 0);
    }
    for (offset, name) in [
        (0x5c, b"INET\0".as_slice()),
        (0x54, b"LINGDB\0".as_slice()),
        (0x40, b"SYNTHSTREAM\0".as_slice()),
    ] {
        if word(mem, object.wrapping_add(offset)) != 0 {
            replace_error(
                &mut rc,
                hooks.component_remove(word(mem, context.wrapping_add(0x18)), name, false),
            );
            store(mem, object.wrapping_add(offset), 0);
        }
    }
    let stream = word(mem, object.wrapping_add(0x3c));
    if stream != 0 {
        replace_error(
            &mut rc,
            hooks.stream_close(
                [word(mem, object), word(mem, object.wrapping_add(4))],
                stream,
            ),
        );
        hooks.stream_destroy(word(mem, object.wrapping_add(0x3c)));
        store(mem, object.wrapping_add(0x3c), 0);
    }
    let head = word(mem, object.wrapping_add(0x140));
    if head != 0 {
        let owner = word(mem, object.wrapping_add(0x84));
        while word(mem, head.wrapping_add(0x80)) != 0 {
            let mut previous = head;
            let mut node = word(mem, head.wrapping_add(0x80));
            while word(mem, node.wrapping_add(0x80)) != 0 {
                previous = node;
                node = word(mem, node.wrapping_add(0x80));
            }
            store(mem, previous.wrapping_add(0x80), 0);
            hooks.free(word(mem, owner.wrapping_add(4)), node);
        }
        hooks.free(
            word(mem, owner.wrapping_add(4)),
            word(mem, object.wrapping_add(0x140)),
        );
        store(mem, object.wrapping_add(0x140), 0);
    }
    if mem[object.wrapping_add(0x78) as usize] != 0 {
        replace_error(
            &mut rc,
            hooks.component_remove(
                word(mem, context.wrapping_add(0x18)),
                b"SYNTHOUTPUTSINK\0",
                true,
            ),
        );
        mem[object.wrapping_add(0x78) as usize] = 0;
    }

    if word(mem, object.wrapping_add(0x74)) != 0 {
        store(mem, object.wrapping_add(0x74), 0);
    }
    let kind = mem[object.wrapping_add(0x69) as usize];
    if mem[object.wrapping_add(0x6a) as usize] == 0 {
        rc = 0x81202000;
    } else {
        let channel = word(mem, object.wrapping_add(0x10));
        let lock = hooks.acquire(word(mem, channel.wrapping_add(0x90)));
        if lock >= 0x80000000 {
            rc = lock;
        } else {
            if kind <= 2 {
                if mem[object.wrapping_add(0x6a) as usize] == 1 {
                    let at = channel
                        .wrapping_add(0x3c)
                        .wrapping_add(u32::from(kind) * 24);
                    let count = word(mem, at);
                    if count != 0 {
                        store(mem, at, count.wrapping_sub(1));
                    }
                } else {
                    hooks.release_token(word(mem, object.wrapping_add(0x6c)));
                }
                hooks.notify(context, channel, u32::from(kind));
                mem[object.wrapping_add(0x6a) as usize] = 0;
            }
            hooks.release(word(
                mem,
                word(mem, object.wrapping_add(0x10)).wrapping_add(0x90),
            ));
        }
    }
    if mem[object.wrapping_add(0x68) as usize] == 1 {
        if hooks.log_level(word(mem, context.wrapping_add(0x10))) >= 4 {
            hooks.unregister_parameter(
                word(mem, context.wrapping_add(0x14)),
                None,
                object,
                callbacks,
            );
        } else {
            for name in parameter_names {
                hooks.unregister_parameter(
                    word(mem, context.wrapping_add(0x14)),
                    Some(name),
                    object,
                    callbacks,
                );
            }
        }
        mem[object.wrapping_add(0x68) as usize] = 0;
    }
    for (offset, name) in [
        (0x58, b"COMPQUERY\0".as_slice()),
        (0x59, b"TTSEG\0".as_slice()),
    ] {
        if mem[object.wrapping_add(offset) as usize] != 0 {
            replace_error(
                &mut rc,
                hooks.component_remove(word(mem, context.wrapping_add(0x18)), name, true),
            );
        }
    }
    let mutex = word(mem, object.wrapping_add(0x2c));
    if mutex != 0 {
        first_error(&mut rc, hooks.mutex_close(mutex));
    }
    if force != 0 || rc < 0x80000000 {
        hooks.free(word(mem, context.wrapping_add(4)), object);
    }
    rc
}

pub trait SchedulerChannelOpenHooks {
    fn context(&mut self, handle: [u32; 2], context: &mut u32) -> u32;
    fn allocate(&mut self, heap: u32) -> u32;
    fn mutex(&mut self, callbacks: u32, heap: u32, output: &mut u32) -> u32;
    fn create_params(&mut self, handle: [u32; 2], output: &mut u32) -> u32;
    fn query(&mut self, params: u32, name: &[u8], output: &mut u32) -> u32;
    fn cooperative(&mut self, params: u32) -> bool;
    fn server(&mut self, params: u32, output: &mut u32) -> u32;
    fn set_server(&mut self, params: u32) -> u32;
    fn configure(&mut self, context: u32, channel: u32, feature: u32, mode: u32) -> u32;
    fn lookup(&mut self, registry: u32, output: &mut u32) -> u32;
    fn broker(&mut self, registry: u32, pair: &mut [u32; 2], vtable: &mut u32) -> u32;
    fn open(&mut self, pair: [u32; 2], handle: [u32; 2], output: &mut [u32; 2]) -> u32;
    fn register(&mut self, registry: u32, vtable: u32, pair: [u32; 2]) -> u32;
    fn cleanup(&mut self, context: u32, object: &mut u32);
}
fn query_channel_word(
    mem: &mut [u8],
    params: u32,
    object: u32,
    at: u32,
    name: &[u8],
    hooks: &mut impl SchedulerChannelOpenHooks,
) -> u32 {
    let mut value = word(mem, object.wrapping_add(at));
    let rc = hooks.query(params, name, &mut value);
    store(mem, object.wrapping_add(at), value);
    rc
}

pub fn open_scheduler_channel(
    mem: &mut [u8],
    handle: [u32; 2],
    output: Option<u32>,
    hooks: &mut impl SchedulerChannelOpenHooks,
) -> u32 {
    let Some(output) = output else {
        return 0x81202007;
    };
    store(mem, output, 0);
    store(mem, output.wrapping_add(4), 0);
    let mut ctx = 0;
    let rc = hooks.context(handle, &mut ctx);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut object = hooks.allocate(word(mem, ctx.wrapping_add(4)));
    if object == 0 {
        return 0x8120200a;
    }
    let result = (|| {
        let mut mutex = word(mem, object.wrapping_add(0x90));
        let rc = hooks.mutex(
            word(mem, ctx.wrapping_add(8)),
            word(mem, ctx.wrapping_add(4)),
            &mut mutex,
        );
        store(mem, object.wrapping_add(0x90), mutex);
        if rc >= 0x80000000 {
            return (rc, false);
        }
        store(mem, object, handle[0]);
        store(mem, object.wrapping_add(4), handle[1]);
        let at = object.wrapping_add(0x1e) as usize;
        mem[at..at + 6].copy_from_slice(b"5.2.3\0");
        if word(mem, ctx.wrapping_add(0x14)) == 0 {
            let mut params = 0;
            let rc = hooks.create_params(handle, &mut params);
            store(mem, ctx.wrapping_add(0x14), params);
            if rc >= 0x80000000 {
                return (rc, false);
            }
            mem[object.wrapping_add(0x1d) as usize] = 1;
        }
        let params = word(mem, ctx.wrapping_add(0x14));
        if query_channel_word(mem, params, object, 0x30, b"ttslicensingports\0", hooks)
            >= 0x80000000
        {
            store(mem, object.wrapping_add(0x30), 0x7fffffff);
            store(mem, object.wrapping_add(0x38), 0x7fffffff);
        } else {
            query_channel_word(
                mem,
                params,
                object,
                0x40,
                b"ttslicensedynamicports\0",
                hooks,
            );
            if word(mem, object.wrapping_add(0x30))
                .wrapping_add(word(mem, object.wrapping_add(0x40)))
                != 0
            {
                query_channel_word(
                    mem,
                    params,
                    object,
                    0x38,
                    b"ttslicensingportsoverdraftthresh\0",
                    hooks,
                );
                query_channel_word(
                    mem,
                    params,
                    object,
                    0x44,
                    b"dynamiclicenseexpirationtime\0",
                    hooks,
                );
            }
        }
        query_channel_word(mem, params, object, 0x48, b"cprlicensingports\0", hooks);
        query_channel_word(
            mem,
            params,
            object,
            0x58,
            b"cprlicensedynamicports\0",
            hooks,
        );
        if word(mem, object.wrapping_add(0x48)).wrapping_add(word(mem, object.wrapping_add(0x58)))
            != 0
        {
            query_channel_word(
                mem,
                params,
                object,
                0x50,
                b"cprlicensingportsoverdraftthresh\0",
                hooks,
            );
            query_channel_word(
                mem,
                params,
                object,
                0x5c,
                b"dynamiclicenseexpirationtime\0",
                hooks,
            );
        }
        if hooks.cooperative(params) {
            store(mem, object.wrapping_add(0x60), 2);
            store(mem, object.wrapping_add(0x68), 2);
        }
        for base in [0x30, 0x48] {
            let sum = word(mem, object.wrapping_add(base))
                .wrapping_add(word(mem, object.wrapping_add(base + 16)));
            let threshold = word(mem, object.wrapping_add(base + 8));
            if sum < threshold || (sum != 0 && threshold == 0) {
                store(mem, object.wrapping_add(base + 8), sum);
            }
            if word(mem, object.wrapping_add(base + 16)) != 0
                && word(mem, object.wrapping_add(base + 20)) < 30
            {
                store(mem, object.wrapping_add(base + 20), 30);
            }
        }
        let mut server = 0;
        let rc = hooks.server(params, &mut server);
        if rc >= 0x80000000 || server == 0 || mem[server as usize] == 0 {
            let rc = hooks.set_server(params);
            if rc >= 0x80000000 {
                return (rc, false);
            }
        }
        let mut dummy = 0;
        hooks.server(params, &mut dummy);
        mem[object.wrapping_add(0x94) as usize] =
            u8::from(hooks.configure(ctx, object, 4, 3) >= 0x80000000);
        let mut configured = 0;
        for (feature, mode) in [(0, 0), (3, 1), (0, 2)] {
            let rc = hooks.configure(ctx, object, feature, mode);
            configured = rc;
            if rc >= 0x80000000 {
                return (rc, false);
            }
        }
        let registry = word(mem, ctx.wrapping_add(0x18));
        if registry != 0 {
            let mut query = 0;
            let rc = hooks.lookup(registry, &mut query);
            if rc < 0x80000000 {
                store(
                    mem,
                    object.wrapping_add(0x10),
                    word(mem, query.wrapping_add(4)),
                );
                store(
                    mem,
                    object.wrapping_add(0x14),
                    word(mem, query.wrapping_add(8)),
                );
                store(
                    mem,
                    object.wrapping_add(0x18),
                    word(mem, query.wrapping_add(12)),
                );
                return (configured, true);
            }
        }
        let mut pair = [
            word(mem, object.wrapping_add(8)),
            word(mem, object.wrapping_add(12)),
        ];
        let mut vtable = word(mem, object.wrapping_add(0x10));
        let rc = hooks.broker(word(mem, ctx.wrapping_add(12)), &mut pair, &mut vtable);
        store(mem, object.wrapping_add(8), pair[0]);
        store(mem, object.wrapping_add(12), pair[1]);
        store(mem, object.wrapping_add(0x10), vtable);
        if rc >= 0x80000000 {
            return (configured, true);
        }
        let mut pair_out = [
            word(mem, object.wrapping_add(0x14)),
            word(mem, object.wrapping_add(0x18)),
        ];
        let rc = hooks.open(pair, handle, &mut pair_out);
        store(mem, object.wrapping_add(0x14), pair_out[0]);
        store(mem, object.wrapping_add(0x18), pair_out[1]);
        if rc >= 0x80000000 {
            return (rc, false);
        }
        if registry != 0 {
            let rc = hooks.register(registry, word(mem, object.wrapping_add(0x10)), pair_out);
            if rc >= 0x80000000 {
                return (rc, false);
            }
            mem[object.wrapping_add(0x1c) as usize] = 1;
            return (rc, true);
        }
        (rc, true)
    })();
    if result.1 {
        store(mem, output, object);
        store(mem, output.wrapping_add(4), 0xcdad);
    } else {
        hooks.cleanup(ctx, &mut object);
    }
    result.0
}

pub trait SchedulerCooperativeHooks: SchedulerHooks {
    fn initialize(
        &mut self,
        context: u32,
        object: u32,
        source: [u32; 2],
        callback: u32,
        user: u32,
        pipe: [u32; 2],
        auxiliary: [u32; 2],
    ) -> u32;
    fn pump(&mut self, object: u32, full: &mut u8) -> u32;
    fn process(&mut self, object: u32, mode: u32) -> u32;
    fn position(&mut self, handle: [u32; 2], position: &mut u32);
    fn divide(&mut self, numerator: u32, denominator: u32) -> u32;
    fn done(&mut self, object: u32, length: u32, duration: u32) -> u32;
    fn trace_begin(&mut self, handle: [u32; 2]);
    fn trace_enter(&mut self, handle: [u32; 2]);
    fn trace_leave(&mut self, handle: [u32; 2]);
    fn trace_end(&mut self, handle: [u32; 2], logger: u32, token: u32, length: u32, duration: u32);
}

pub fn synthesize_scheduler_cooperatively(
    mem: &mut [u8],
    handle: [u32; 2],
    pipe: [u32; 2],
    auxiliary: [u32; 2],
    output: u32,
    hooks: &mut impl SchedulerCooperativeHooks,
) -> u32 {
    let (rc, context, object) = hooks.resolve(false, handle);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut lock = hooks.acquire(word(mem, object.wrapping_add(0x2c)));
    if lock >= 0x80000000 {
        return lock;
    }
    let state = mem[object.wrapping_add(0x30) as usize];
    let mut saved = 0;
    if state == 1 {
        if word(mem, object.wrapping_add(0x7c)) == 0 {
            lock = 0x81202011;
        } else {
            mem[object.wrapping_add(0x30) as usize] = 4;
            saved = 1;
        }
    } else if matches!(state, 4 | 5 | 6) {
        saved = state;
    } else {
        lock = 0x81202011;
    }
    first_error(
        &mut lock,
        hooks.release(word(mem, object.wrapping_add(0x2c))),
    );
    if lock >= 0x80000000 {
        return lock;
    }
    let mut rc = lock;
    mem[output as usize..output as usize + 2].copy_from_slice(&0x20u16.to_le_bytes());
    let mut failed = false;
    let mut empty = false;
    if saved == 1 {
        let trace = word(mem, object.wrapping_add(0x7c));
        let trace_handle = [
            word(mem, trace.wrapping_add(0x10)),
            word(mem, trace.wrapping_add(0x14)),
        ];
        if trace_handle[0] != 0 {
            hooks.trace_begin(trace_handle);
            hooks.trace_enter(trace_handle);
        }
        let source = [
            word(mem, object.wrapping_add(0x14)),
            word(mem, object.wrapping_add(0x28)),
        ];
        rc = hooks.initialize(
            context,
            object,
            source,
            word(mem, object.wrapping_add(0x1c)),
            word(mem, object.wrapping_add(0x28)),
            pipe,
            auxiliary,
        );
        failed = rc >= 0x80000000;
    } else {
        mem[object.wrapping_add(0xe2) as usize] = 1;
        let mut full = 0;
        rc = hooks.pump(object, &mut full);
        if rc >= 0x80000000 {
            failed = true;
        } else if saved == 4 {
            if full == 0 {
                rc = hooks.process(object, 2);
                failed = rc >= 0x80000000;
            }
        } else {
            empty = full == 0;
        }
    }
    if failed || empty {
        let length = word(mem, object.wrapping_add(0xf0));
        mem[output as usize..output as usize + 2].copy_from_slice(&2u16.to_le_bytes());
        let mut duration = 0;
        if word(mem, object.wrapping_add(0xc0)) != 0 {
            let mut position = 0;
            hooks.position(
                [
                    word(mem, object.wrapping_add(0xc0)),
                    word(mem, object.wrapping_add(0xc4)),
                ],
                &mut position,
            );
            duration = hooks.divide(
                position.wrapping_shl(3),
                word(mem, object.wrapping_add(0x8c)),
            );
            duration = if duration < 0x418937 {
                hooks.divide(
                    duration.wrapping_mul(1000),
                    word(mem, object.wrapping_add(0x88)),
                )
            } else {
                hooks
                    .divide(duration, word(mem, object.wrapping_add(0x88)))
                    .wrapping_mul(1000)
            };
        }
        first_error(&mut rc, hooks.done(object, length, duration));
        let trace = word(mem, object.wrapping_add(0x7c));
        let h = [
            word(mem, trace.wrapping_add(0x10)),
            word(mem, trace.wrapping_add(0x14)),
        ];
        if h[0] != 0 {
            hooks.trace_leave(h);
            hooks.trace_end(
                h,
                word(mem, context.wrapping_add(0x10)),
                word(mem, trace.wrapping_add(0x18)),
                length,
                duration,
            );
        }
        hooks.restore(mem, context, object, 1);
    }
    rc
}

pub trait SchedulerSpeakHooks: SchedulerHooks {
    fn register_inet(&mut self, components: u32, vtable: u32, pair: [u32; 2]) -> u32;
    fn unregister_inet(&mut self, components: u32) -> u32;
    fn parse_uri(
        &mut self,
        pair: [u32; 2],
        text: u32,
        aux: [u32; 2],
        strings: &mut [u32; 3],
        source: &mut [u32; 2],
    ) -> u32;

    fn uri_pipe(&mut self, pair: [u32; 2], aux: Option<[u32; 2]>, pipe: &mut [u32; 2]) -> [u32; 4];
    fn uri_base(&mut self, pipe: [u32; 2], flags: u32, base: u32, f1: u32, f2: u32) -> u32;
    fn uri_free(&mut self, pair: [u32; 2], text: u32);
    fn uri_close(&mut self, pair: [u32; 2]);
    fn stream_create(&mut self, handle: [u32; 2], out: &mut [u32; 2]) -> u32;
    fn stream_open(&mut self, pair: [u32; 2], mime: u32, out: &mut [u32; 2]) -> u32;
    fn stream_source(&mut self, pair: [u32; 2], source: [u32; 2]) -> u32;
    fn stream_release(&mut self, pair: [u32; 2], opened: bool);
    fn score_uri(
        &mut self,
        context: u32,
        object: u32,
        source: [u32; 2],
        mime: u32,
        pipe: [u32; 2],
        outputs: &mut [u32; 2],
    ) -> u32;
    fn trace_uri(
        &mut self,
        operation: u32,
        handle: [u32; 2],
        logger: u32,
        token: u32,
        outputs: [u32; 2],
    );
}

pub fn speak_scheduler(
    mem: &mut [u8],
    handle: [u32; 2],
    text: u32,
    mut mime: u32,
    aux: [u32; 2],
    hooks: &mut impl SchedulerSpeakHooks,
) -> u32 {
    let (mut rc, ctx, obj) = hooks.resolve(false, handle);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut flag = 0;
    rc = hooks.enter(mem, ctx, obj, b"ttseg_Speak\0", &mut flag);
    if rc >= 0x80000000 {
        return rc;
    }
    let trace = word(mem, obj.wrapping_add(0x7c));
    let mut registered = false;
    let mut pipe = [0; 2];
    let mut outputs = [0; 2];
    let mut trace_handle = [0; 2];
    let components = word(mem, ctx.wrapping_add(0x18));
    let vt = word(mem, obj.wrapping_add(0x5c));
    let h = [word(mem, obj), word(mem, obj.wrapping_add(4))];
    let uri = [
        word(mem, obj.wrapping_add(0x60)),
        word(mem, obj.wrapping_add(0x64)),
    ];
    if trace == 0 {
        rc = 0x81202011;
    } else {
        trace_handle = [
            word(mem, trace.wrapping_add(0x10)),
            word(mem, trace.wrapping_add(0x14)),
        ];
        if trace_handle[0] != 0 {
            hooks.trace_uri(0, trace_handle, 0, 0, outputs);
        }
        if aux[0] != 0 && vt != 0 {
            registered = hooks.register_inet(components, vt, aux) < 0x80000000;
        }
        if text == 0 || mem[text as usize] == 0 {
            let source = [
                word(mem, obj.wrapping_add(0x14)),
                word(mem, obj.wrapping_add(0x28)),
            ];
            rc = hooks.score_uri(ctx, obj, source, mime, [0; 2], &mut outputs);
        } else {
            let mut strings = [0; 3];
            let mut source = [0; 2];
            let mut a = [0; 2];
            let mut b = [0; 2];
            rc = (|| {
                if vt == 0 {
                    return 0x81202001;
                }
                let r = hooks.parse_uri(uri, text, aux, &mut strings, &mut source);
                if r >= 0x80000000 {
                    return r;
                }
                if source[1] == 0 {
                    return 0x81202800;
                }
                if mime == 0 || mem[mime as usize] == 0 {
                    if strings[2] == 0 || mem[strings[2] as usize] == 0 {
                        return 0x81203d22;
                    }
                    mime = strings[2];
                }
                let result =
                    hooks.uri_pipe(h, if aux[0] != 0 { Some(aux) } else { None }, &mut pipe);
                if result[0] < 0x80000000
                    && hooks.uri_base(
                        pipe,
                        (result[3] & 0xffff0000) | 2,
                        strings[1],
                        result[1],
                        result[2],
                    ) < 0x80000000
                {
                    let mut can_register = true;
                    if registered {
                        registered = false;
                        can_register = hooks.unregister_inet(components) < 0x80000000;
                    }
                    if can_register {
                        registered = hooks.register_inet(components, vt, pipe) < 0x80000000;
                    }
                }
                let r = hooks.stream_create(h, &mut a);
                if r >= 0x80000000 {
                    return r;
                }
                let r = hooks.stream_open(a, mime, &mut b);
                if r >= 0x80000000 {
                    return r;
                }
                hooks.stream_source(b, source)
            })();
            if b[0] != 0 {
                hooks.stream_release(b, true);
            }
            if rc < 0x80000000 {
                rc = hooks.score_uri(ctx, obj, [0; 2], mime, a, &mut outputs);
            }
            if strings[0] != 0 {
                hooks.uri_free(uri, strings[0]);
            }
            if a[0] != 0 {
                hooks.stream_release(a, false);
            }
        }
    }
    if registered {
        first_error(&mut rc, hooks.unregister_inet(components));
    }
    if pipe[0] != 0 {
        hooks.uri_close(pipe);
    }
    if trace_handle[0] != 0 {
        hooks.trace_uri(
            1,
            trace_handle,
            word(mem, ctx.wrapping_add(0x10)),
            word(mem, trace.wrapping_add(0x18)),
            outputs,
        );
    }
    hooks.restore(mem, ctx, obj, flag);
    rc
}
