use core::{num::NonZeroU32, ops::Add};

pub const NUM_STATS: usize = 6;
pub const NUM_ITEM_BUCKETS: usize = 5;

#[repr(transparent)]
#[derive(Clone, Copy, Default)]
pub struct Stats(pub [u16; NUM_STATS]);

pub const NO_TIER: u8 = u8::MAX;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DestinyEnergyType {
    Any = 0,
    Arc = 1,
    Solar = 2,
    Void = 3,
    Stasis = 4,
}

#[repr(C)]
pub struct ProcessItem {
    /// The id to map the generated set back to real items
    pub id: u16,
    pub power: u16,
    pub energy_type: DestinyEnergyType,
    pub energy_val: u8,
    pub energy_cap: u8,
    pub exotic: u8,
    /// A bit mask of mod tags this item can slot. Currently imposes a limit of 32 slot tags.
    pub mod_tags: u32,
    pub stats: Stats,
}

#[repr(C)]
pub struct ProcessMod {
    pub hash: Option<NonZeroU32>,
    pub mod_tag: Option<NonZeroU32>,
    pub energy_type: DestinyEnergyType,
    pub energy_val: u8,
}

#[repr(C)]
pub struct ProcessArmorSet {
    pub stats: Stats,
    pub items: [u16; NUM_ITEM_BUCKETS],
    pub power: u16,
    pub total_tier: u8,
}

#[repr(C)]
#[derive(Default)]
pub struct ProcessArgs {
    pub base_stats: Stats,
    pub num_items: [u16; NUM_ITEM_BUCKETS],
    pub lower_bounds: [u8; NUM_STATS],
    pub upper_bounds: [u8; NUM_STATS],
    pub any_exotic: u8,
}

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
#[derive(Default)]
pub struct ProcessStats {
    pub num_valid_sets: u32,
    pub skipped_low_tier: u32,
    pub skipped_stat_range: u32,
    pub skipped_mods_unfit: u32,
    pub skipped_double_exotic: u32,
    pub skipped_no_exotic: u32,
}

#[repr(C)]
pub struct ProcessSetupContext {
    pub args: ProcessArgs,
    pub items: (*mut ProcessItem, usize, usize),
    pub mods: (*mut ProcessMod, usize, usize),
}

macro_rules! assert_size_align {
    ($ty:ident, $size:literal, $align:literal) => {
        #[cfg(target_pointer_width = "32")]
        const _: () = {
            if core::mem::size_of::<$ty>() != $size {
                panic!()
            }
            if core::mem::align_of::<$ty>() != $align {
                panic!()
            }
        };
    };
}

// FFI guarantees...
assert_size_align!(ProcessItem, 24, 4);
assert_size_align!(ProcessMod, 12, 4);
assert_size_align!(ProcessArmorSet, 26, 2);
assert_size_align!(ProcessArgs, 36, 2);
assert_size_align!(ProcessResults, 60, 4);
assert_size_align!(ProcessStats, 24, 4);

impl Add for Stats {
    type Output = Stats;

    #[inline]
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0[0] += rhs.0[0];
        self.0[1] += rhs.0[1];
        self.0[2] += rhs.0[2];
        self.0[3] += rhs.0[3];
        self.0[4] += rhs.0[4];
        self.0[5] += rhs.0[5];
        self
    }
}
