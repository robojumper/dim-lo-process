use core::num::NonZeroU32;

use crate::{
    dim_lo_process,
    types::{
        EnergyType, ProcessArgs, ProcessItem, ProcessMod, ProcessStatMod, Stats, NUM_ITEM_BUCKETS,
        NUM_STATS,
    },
};

#[test]
fn check_auto_assignment() {
    let no_mods = [NO_MOD; 5];
    let base_stats = Stats([0; NUM_STATS]);
    let items: [&[_]; NUM_ITEM_BUCKETS] = [
        &[ProcessItem {
            id: 1,
            power: 1560,
            energy_type: EnergyType::Any,
            energy_val: 0,
            energy_cap: 10,
            exotic: false,
            mod_tags: 0,
            stats: Stats([11, 4, 23, 8, 24, 8]),
        }],
        &[ProcessItem {
            id: 2,
            power: 1560,
            energy_type: EnergyType::Any,
            energy_val: 0,
            energy_cap: 10,
            exotic: false,
            mod_tags: 0,
            stats: Stats([10, 4, 24, 8, 14, 18]),
        }],
        &[ProcessItem {
            id: 3,
            power: 1560,
            energy_type: EnergyType::Any,
            energy_val: 0,
            energy_cap: 10,
            exotic: true,
            mod_tags: 0,
            stats: Stats([14, 9, 18, 18, 14, 8]),
        }],
        &[ProcessItem {
            id: 4,
            power: 1560,
            energy_type: EnergyType::Any,
            energy_val: 0,
            energy_cap: 10,
            exotic: false,
            mod_tags: 0,
            stats: Stats([4, 11, 24, 18, 18, 4]),
        }],
        &[ProcessItem {
            id: 5,
            power: 1560,
            energy_type: EnergyType::Any,
            energy_val: 0,
            energy_cap: 10,
            exotic: false,
            mod_tags: 0,
            stats: Stats([2, 2, 2, 2, 2, 2]),
        }],
    ];
    let args = ProcessArgs {
        base_stats,
        bounds: crate::types::ProcessTierBounds {
            lower_bounds: [0, 0, 10, 9, 0, 0],
            upper_bounds: [10, 10, 10, 10, 10, 10],
        },
        any_exotic: true,
        auto_mods: 5,
    };
    let result = dim_lo_process(items, &no_mods, &no_mods, &no_mods, &SAMPLE_MODS, &args);

    assert!(!result.1.is_empty())
}

pub const NO_MOD: ProcessMod = ProcessMod {
    hash: None,
    mod_tag: None,
    energy_type: EnergyType::Any,
    energy_val: 0,
};

pub const SAMPLE_MODS: [ProcessStatMod; 12] = [
    // Small mob
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(1),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 1,
        },

        stats: Stats([5, 0, 0, 0, 0, 0]),
    },
    // Small res
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(2),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 1,
        },
        stats: Stats([0, 5, 0, 0, 0, 0]),
    },
    // Small rec
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(3),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 2,
        },

        stats: Stats([0, 0, 5, 0, 0, 0]),
    },
    // Small dis
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(4),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 1,
        },

        stats: Stats([0, 0, 0, 5, 0, 0]),
    },
    // Small int
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(5),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 2,
        },

        stats: Stats([0, 0, 0, 0, 5, 0]),
    },
    // Small str
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(6),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 1,
        },

        stats: Stats([0, 0, 0, 0, 0, 5]),
    },
    // Big mob
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(7),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 3,
        },

        stats: Stats([10, 0, 0, 0, 0, 0]),
    },
    // Big res
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(8),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 3,
        },

        stats: Stats([0, 10, 0, 0, 0, 0]),
    },
    // Big rec
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(9),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 4,
        },

        stats: Stats([0, 0, 10, 0, 0, 0]),
    },
    // Big dis
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(10),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 3,
        },

        stats: Stats([0, 0, 0, 10, 0, 0]),
    },
    // Big int
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(11),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 5,
        },

        stats: Stats([0, 0, 0, 0, 10, 0]),
    },
    // Big str
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(12),
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 3,
        },

        stats: Stats([0, 0, 0, 0, 0, 10]),
    },
];
