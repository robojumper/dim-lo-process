#![feature(vec_into_raw_parts)]
#![no_std]

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use dim_lo_core::{
    dim_lo_process,
    types::{
        ProcessArgs, ProcessArmorSet, ProcessItem, ProcessMinMaxStats, ProcessMod, ProcessStatMod,
        ProcessStats, ProcessTierBounds, NUM_ITEM_BUCKETS, NUM_STATS,
    },
};
use types::{ProcessResults, ProcessSetupContext};

mod types;

/// Initialize memory for a context holding the configuration of the algorithm,
/// `num_items` ProcessItems, and `num_auto_mods` auto stat mods.
#[no_mangle]
fn lo_init(num_items: usize, num_auto_mods: usize) -> *mut ProcessSetupContext {
    let items = Vec::<ProcessItem>::with_capacity(num_items);
    let mods = Vec::<ProcessMod>::with_capacity(15);
    let auto_mods = Vec::<ProcessStatMod>::with_capacity(num_auto_mods);
    let ctx = Box::new(ProcessSetupContext {
        args: ProcessArgs::default(),
        num_items: Default::default(),
        num_auto_mods,
        items: items.into_raw_parts(),
        mods: mods.into_raw_parts(),
        auto_mods: auto_mods.into_raw_parts(),
    });
    Box::into_raw(ctx)
}

/// Sets some LO settings. `ctx` must have been allocated via `lo_init`.
#[no_mangle]
fn lo_setup_settings(ctx: *mut ProcessSetupContext, any_exotic: usize, allowed_auto_mods: usize) {
    let ctx = unsafe { &mut *ctx };
    ctx.args.auto_mods = allowed_auto_mods as u8;
    ctx.args.any_exotic = any_exotic != 0;
}

/// Gets a pointer to the buffer holding the 6 base stats.
/// `ctx` must have been allocated via `lo_init`.
#[no_mangle]
fn lo_setup_base_stats_ptr(ctx: *mut ProcessSetupContext) -> *mut [u16; NUM_STATS] {
    unsafe { &mut (*ctx).args.base_stats.0 }
}

/// Gets a pointer to the buffer holding the number of items per bucket.
/// `ctx` must have been allocated via `lo_init`.
#[no_mangle]
fn lo_setup_num_items_per_bucket_ptr(
    ctx: *mut ProcessSetupContext,
) -> *mut [u16; NUM_ITEM_BUCKETS] {
    unsafe { &mut (*ctx).num_items }
}

/// Gets a pointer to the buffer holding stat minimums and maximums.
/// `ctx` must have been allocated via `lo_init`.
#[no_mangle]
fn lo_setup_bounds_ptr(ctx: *mut ProcessSetupContext) -> *mut ProcessTierBounds {
    unsafe { &mut (*ctx).args.bounds }
}

/// Gets a pointer to the buffer allocated for `num_items` ProcessItems
/// in `lo_init`. `ctx` must have been allocated via `lo_init`.
#[no_mangle]
fn lo_setup_items_ptr(ctx: *mut ProcessSetupContext) -> *mut ProcessItem {
    unsafe { (*ctx).items.0 }
}

/// Gets a pointer to the buffer allocated for 15 ProcessMods.
/// This buffer must be filled with 5 general mods, then 5 combat mods, then
/// 5 activity mods. Not-filled slots must be zeroed entirely.
/// `ctx` must have been allocated via `lo_init`.
#[no_mangle]
fn lo_setup_mods_ptr(ctx: *mut ProcessSetupContext) -> *mut ProcessMod {
    unsafe { (*ctx).mods.0 }
}

/// Gets a pointer to the buffer allocated for `num_auto_mods` ProcessStatMods.
/// `ctx` must have been allocated via `lo_init`.
#[no_mangle]
fn lo_setup_auto_mods_ptr(ctx: *mut ProcessSetupContext) -> *mut ProcessStatMod {
    unsafe { (*ctx).auto_mods.0 }
}

#[no_mangle]
fn lo_run(ctx: *mut ProcessSetupContext) -> *mut ProcessResults {
    let ctx = unsafe { &*ctx };

    let mut lists: [&[ProcessItem]; NUM_ITEM_BUCKETS] = [&[]; NUM_ITEM_BUCKETS];
    let mut running_offset = 0;
    for (list, len) in lists.iter_mut().zip(ctx.num_items) {
        *list = unsafe {
            core::slice::from_raw_parts(ctx.items.0.offset(running_offset), len as usize)
        };
        running_offset += len as isize;
    }

    let general_mods = unsafe { &*(ctx.mods.0 as *const [ProcessMod; 5]) };
    let combat_mods = unsafe { &*(ctx.mods.0.offset(5) as *const [ProcessMod; 5]) };
    let activity_mods = unsafe { &*(ctx.mods.0.offset(10) as *const [ProcessMod; 5]) };
    let auto_mods = unsafe { core::slice::from_raw_parts(ctx.auto_mods.0, ctx.num_auto_mods) };

    let (stats, results, min_max) = dim_lo_process(
        lists,
        general_mods,
        combat_mods,
        activity_mods,
        auto_mods,
        &ctx.args,
    );

    let parts = results.into_raw_parts();

    let ret = Box::new(ProcessResults {
        ptr: parts.0,
        len: parts.1,
        cap: parts.2,
        stats,
        min_max,
    });

    Box::into_raw(ret)
}

/// Gets how many sets were generated.
/// `ctx` must be the result of `lo_run`.
#[no_mangle]
fn lo_result_num_sets(ctx: *mut ProcessResults) -> usize {
    unsafe { (*ctx).len }
}

/// Gets a pointer to the generated sets buffer.
/// `ctx` must be the result of `lo_run`.
#[no_mangle]
fn lo_result_sets_ptr(ctx: *mut ProcessResults) -> *mut ProcessArmorSet {
    unsafe { (*ctx).ptr }
}

/// Gets a pointer to the auxiliary information about generated sets.
/// `ctx` must be the result of `lo_run`.
#[no_mangle]
fn lo_result_info_ptr(ctx: *mut ProcessResults) -> *mut ProcessStats {
    unsafe { &mut (*ctx).stats }
}

/// Gets a pointer to the buffer containing min/max observed stats.
/// `ctx` must be the result of `lo_run`.
#[no_mangle]
fn lo_result_minmax_ptr(ctx: *mut ProcessResults) -> *mut ProcessMinMaxStats {
    unsafe { &mut (*ctx).min_max }
}

/// Free all memory allocated as part of the algorithm setup and runtime.
/// Passing null pointers is allowed, e.g. when you decide to not call `lo_run`
/// and instead just free the setup data.
#[no_mangle]
fn lo_free(ctx: *mut ProcessSetupContext, res: *mut ProcessResults) {
    // Restore the types used to allocate, this will deallocate upon dropping.
    if !ctx.is_null() {
        let ctx = unsafe { Box::from_raw(ctx) };
        let _items = unsafe { Vec::from_raw_parts(ctx.items.0, ctx.items.1, ctx.items.2) };
        let _mods = unsafe { Vec::from_raw_parts(ctx.mods.0, ctx.mods.1, ctx.mods.2) };
        let _auto_mods =
            unsafe { Vec::from_raw_parts(ctx.auto_mods.0, ctx.auto_mods.1, ctx.auto_mods.2) };
    }

    if !res.is_null() {
        let res = unsafe { Box::from_raw(res) };
        let _sets = unsafe { Vec::from_raw_parts(res.ptr, res.len, res.cap) };
    }
}
