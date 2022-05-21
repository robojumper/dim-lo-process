use dim_lo_core::types::{
    ProcessArgs, ProcessArmorSet, ProcessItem, ProcessMod, ProcessStatMod, ProcessStats, NUM_STATS,
};

#[repr(C)]
pub struct ProcessResults {
    pub ptr: *mut ProcessArmorSet,
    pub len: usize,
    pub cap: usize,
    pub stats: ProcessStats,
    pub min_seen: [u16; NUM_STATS],
    pub max_seen: [u16; NUM_STATS],
}

#[repr(C)]
pub struct ProcessSetupContext {
    pub args: ProcessArgs,
    pub items: (*mut ProcessItem, usize, usize),
    pub mods: (*mut ProcessMod, usize, usize),
    pub auto_mods: (*mut ProcessStatMod, usize, usize),
}
