use alloc::{collections::BTreeMap, vec::Vec};
use pareto_front::{Dominate, ParetoFront};

use crate::types::{ProcessMod, ProcessStatMod, Stats, NUM_ITEM_BUCKETS, NUM_STATS};

#[cfg_attr(test, derive(Debug))]
pub struct ModsArray<'a> {
    pub costs: [u8; NUM_ITEM_BUCKETS],
    pub mods: [&'a ProcessMod; NUM_ITEM_BUCKETS],
    pub sum_cost: u8,
    pub num_mods: u8,
}

/// While building the stat mods map, we deduplicate the picks.
/// Consider picks for 1 recovery, 1 intellect. We could use:
///
/// * [large intellect (5), small recovery (2), small recovery (2)]
/// * [large recovery (4), small intellect (2), small intellect (2)]
///
/// The second is clearly preferable (pareto dominates the first one),
/// so we just throw the first one away, it's superfluous.
///
/// Similarly:
///
/// * [large strength (3), small mobility (1), small recovery (1)]
/// * [large mobility (3), small strength (1), small strength (1)]
///
/// Just keep one of them and throw the other away, they do the same thing and have the same requirements.
#[cfg_attr(test, derive(Debug))]
struct TempMods<'a> {
    pub mods: ParetoFront<ModsArray<'a>>,
}

#[cfg_attr(test, derive(Debug))]
pub struct SomeMods<'a> {
    pub mods: Vec<ModsArray<'a>>,
}

impl Dominate for ModsArray<'_> {
    fn dominate(&self, x: &Self) -> bool {
        self.costs[0] <= x.costs[0]
            && self.costs[1] <= x.costs[1]
            && self.costs[2] <= x.costs[2]
            && self.costs[3] <= x.costs[3]
            && self.costs[4] <= x.costs[4]
    }
}

/// Okay, so LO should automatically assign stat mods to hit minimum required stats.
/// Auto mod assignment *must be* correct: If there is a way to pick stat mods and assign
/// other mods and stat mods so that they fit on the items and the minimum stats are hit,
/// LO must be able to find it. This is surprisingly tricky: Imagine we need one tier of
/// strength and one tier of recovery. We could pick the mods in different ways:
///
/// * [large recovery (4), large strength (3)]
/// * [large recovery (4), small strength (1), small strength (1)]
/// * [large strength (3), small recovery (2), small recovery (2)]
/// * [small recovery (2), small recovery (2), small strength (1), small strength (1)]
///
/// Each of these picks is pareto-optimal wrt. leftover energy. There exist armor + mod
/// picks for which exactly one of these picks fits and no other pick can fit.
/// This becomes even worse when LO has to assign bucket-independent mods (BI) like
/// combat or activity mods, because there might be a permutation of BI that doesn't
/// leave enough space for any of these picks and another permutation that does.
///
/// What saves us here is that stat mods don't have an element requirement, so we don't
/// have to test all permutations of stat mods. Instead, we just sort the costs of each pick
/// descending and compare with the leftover energy capacities, also sorced descendingly.
///
/// This function builds a map from stats -> picks of mods that generate these stats. This
/// is kind of expensive, but it only has to happen once and massively optimizes the throughput
/// of the mod assignment algorithm.
/// LO can then check what stats the set is missing and immediately find a set of picks to test,
/// and test these picks for every BI assignment it comes up with.
///
/// Finally, what's perhaps interesting about this is that this already factors in stat mods the
/// user picked themselves, but without the stats. E.g. if the user forces an intellect mod (cost 5),
/// this function will only generate picks with up to 4 extra mods, and the costs of every pick will
/// include a 5 at the front. LO thus doesn't need to iterate over stat mod permutations and gets the
/// check for free.
///
/// Corollary: If num_extra_stat_mods is 0, this map has a single entry:
///
/// `[0, 0, 0, 0, 0, 0] -> [...costs of existing stat mods, ...0]`
///
/// So this mostly doesn't affect performance of non-auto-stat-mod runs at all.
pub fn generate_mods_options<'a>(
    existing_stat_mods: &[ProcessMod],
    mods: &'a [ProcessStatMod],
    empty_mod: &'a ProcessStatMod,
    num_extra_mods: u8,
) -> BTreeMap<Stats, SomeMods<'a>> {
    let mut map = BTreeMap::new();

    let num_existing_mods = existing_stat_mods.len();

    let capacity = (num_extra_mods as usize).saturating_sub(num_existing_mods);

    let mut cost_list = [(None, 0); NUM_ITEM_BUCKETS];
    // Copy over the already existing stat costs
    for (idx, m) in existing_stat_mods.iter().enumerate() {
        cost_list[idx].1 = m.energy_val;
    }

    let mut record = |list: &[(Option<&'a ProcessStatMod>, u8); NUM_ITEM_BUCKETS],
                      num_extra_mods: usize| {
        let auto_mods = &list[num_existing_mods..(num_existing_mods + num_extra_mods)];
        let mut costs = list.map(|c| c.1);
        costs.sort_by_key(|&m| core::cmp::Reverse(m));
        let mut mods = [&empty_mod.inner_mod; NUM_ITEM_BUCKETS];
        for (idx, m) in auto_mods.iter().enumerate() {
            mods[idx] = &m.0.expect("num_extra_mods guaranteed this").inner_mod;
        }

        let stats = auto_mods
            .iter()
            .map(|m| m.0.unwrap().stats)
            .fold(Stats([0; NUM_STATS]), |acc, m| acc + m);

        let entry = map.entry(stats).or_insert_with(|| TempMods {
            mods: ParetoFront::new(),
        });

        entry.mods.push(ModsArray {
            mods,
            costs,
            sum_cost: costs.iter().sum(),
            num_mods: num_extra_mods as u8,
        });
    };

    record(&cost_list, 0);

    if capacity > 0 {
        for mod0 in mods {
            cost_list[num_existing_mods] = (Some(mod0), mod0.inner_mod.energy_val);
            record(&cost_list, 1);

            if capacity > 1 {
                for mod1 in mods {
                    cost_list[num_existing_mods + 1] = (Some(mod1), mod1.inner_mod.energy_val);
                    record(&cost_list, 2);

                    if capacity > 2 {
                        for mod2 in mods {
                            cost_list[num_existing_mods + 2] =
                                (Some(mod2), mod2.inner_mod.energy_val);
                            record(&cost_list, 3);

                            if capacity > 3 {
                                for mod3 in mods {
                                    cost_list[num_existing_mods + 3] =
                                        (Some(mod3), mod3.inner_mod.energy_val);
                                    record(&cost_list, 4);

                                    if capacity > 4 {
                                        for mod4 in mods {
                                            cost_list[num_existing_mods + 4] =
                                                (Some(mod4), mod4.inner_mod.energy_val);
                                            record(&cost_list, 5);
                                        }
                                        cost_list[num_existing_mods + 4] = (None, 0);
                                    }
                                }
                                cost_list[num_existing_mods + 3] = (None, 0);
                            }
                        }
                        cost_list[num_existing_mods + 2] = (None, 0);
                    }
                }
                cost_list[num_existing_mods + 1] = (None, 0);
            }
        }
        cost_list[num_existing_mods] = (None, 0);
    }

    // Destructure the map and create it with the picks sorted ascending by the number of mods.
    // This is purely flavor for preferring whole stat mods over halves, e.g. a single large
    // mobility mod looks better than two half tier mobility mods.
    map.into_iter()
        .map(|(key, val)| {
            let mut variants = val.mods.into_iter().collect::<Vec<_>>();
            variants.sort_by_key(|m| m.num_mods);
            (key, SomeMods { mods: variants })
        })
        .collect()
}
