use dim_lo_core::types::{
    ProcessArgs, ProcessArmorSet, ProcessItem, ProcessMinMaxStats, ProcessMod, ProcessStatMod,
    ProcessStats, NUM_ITEM_BUCKETS,
};

#[repr(C)]
pub struct ProcessResults {
    pub ptr: *mut ProcessArmorSet,
    pub len: usize,
    pub cap: usize,
    pub stats: ProcessStats,
    pub min_max: ProcessMinMaxStats,
}

#[repr(C)]
pub struct ProcessSetupContext {
    pub args: ProcessArgs,
    pub num_items: [u16; NUM_ITEM_BUCKETS],
    pub num_auto_mods: usize,
    pub items: (*mut ProcessItem, usize, usize),
    pub mods: (*mut ProcessMod, usize, usize),
    pub auto_mods: (*mut ProcessStatMod, usize, usize),
}
