use crate::{
    dim_lo_process,
    types::{
        ProcessArgs, ProcessItem, ProcessMod, ProcessResults, ProcessSetupContext, ProcessStatMod,
        NUM_ITEM_BUCKETS,
    },
};

#[no_mangle]
pub fn lo_init(num_items: usize) -> *mut ProcessSetupContext {
    let items = Vec::<ProcessItem>::with_capacity(num_items);
    let mods = Vec::<ProcessMod>::with_capacity(15);
    let auto_mods = Vec::<ProcessStatMod>::with_capacity(13);
    let ctx = Box::new(ProcessSetupContext {
        args: ProcessArgs::default(),
        items: items.into_raw_parts(),
        mods: mods.into_raw_parts(),
        auto_mods: auto_mods.into_raw_parts(),
    });
    Box::into_raw(ctx)
}

#[no_mangle]
pub fn lo_items_ptr(ctx: *mut ProcessSetupContext) -> *mut ProcessItem {
    unsafe { (*ctx).items.0 }
}

#[no_mangle]
pub fn lo_mods_ptr(ctx: *mut ProcessSetupContext) -> *mut ProcessMod {
    unsafe { (*ctx).mods.0 }
}

#[no_mangle]
pub fn lo_auto_mods_ptr(ctx: *mut ProcessSetupContext) -> *mut ProcessStatMod {
    unsafe { (*ctx).auto_mods.0 }
}

#[no_mangle]
pub fn lo_run(ctx: *mut ProcessSetupContext) -> *mut ProcessResults {
    let ctx = unsafe { &*ctx };

    let mut lists: [&[ProcessItem]; NUM_ITEM_BUCKETS] = [&[]; NUM_ITEM_BUCKETS];
    let mut running_offset = 0;
    for (list, len) in lists.iter_mut().zip(ctx.args.num_items) {
        *list = unsafe {
            core::slice::from_raw_parts(ctx.items.0.offset(running_offset), len as usize)
        };
        running_offset += len as isize;
    }

    let general_mods = unsafe { &*(ctx.mods.0 as *const [ProcessMod; 5]) };
    let combat_mods = unsafe { &*(ctx.mods.0.offset(5) as *const [ProcessMod; 5]) };
    let activity_mods = unsafe { &*(ctx.mods.0.offset(10) as *const [ProcessMod; 5]) };
    let auto_mods = unsafe { &*(ctx.auto_mods.0 as *const [ProcessStatMod; 13]) };

    let (stats, results, min_seen, max_seen) = dim_lo_process(
        lists,
        general_mods,
        combat_mods,
        activity_mods,
        ctx.args.base_stats,
        auto_mods,
        ctx.args.auto_mods != 0,
        ctx.args.lower_bounds,
        ctx.args.upper_bounds,
        ctx.args.any_exotic != 0,
    );

    let parts = results.into_raw_parts();

    let ret = Box::new(ProcessResults {
        ptr: parts.0,
        len: parts.1,
        cap: parts.2,
        stats,
        min_seen,
        max_seen,
    });

    Box::into_raw(ret)
}

#[no_mangle]
pub fn lo_free(ctx: *mut ProcessSetupContext, res: *mut ProcessResults) {
    let ctx = unsafe { Box::from_raw(ctx) };
    let res = unsafe { Box::from_raw(res) };
    let _items = unsafe { Vec::from_raw_parts(ctx.items.0, ctx.items.1, ctx.items.2) };
    let _mods = unsafe { Vec::from_raw_parts(ctx.mods.0, ctx.mods.1, ctx.mods.2) };
    let _auto_mods =
        unsafe { Vec::from_raw_parts(ctx.auto_mods.0, ctx.auto_mods.1, ctx.auto_mods.2) };
    let _sets = unsafe { Vec::from_raw_parts(res.ptr, res.len, res.cap) };
}
