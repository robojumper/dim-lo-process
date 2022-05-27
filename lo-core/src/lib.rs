#![feature(array_methods)]
#![feature(array_windows)]
#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;

use alloc::vec::Vec;
use set_tracker::SetTracker;
use stat_mod_set::SomeMods;
use types::{
    EnergyType, ProcessArgs, ProcessArmorSet, ProcessItem, ProcessMinMaxStats, ProcessMod,
    ProcessStatMod, ProcessStats, Stats, NUM_ITEM_BUCKETS, NUM_STATS,
};

mod set_tracker;
mod stat_mod_set;
pub mod types;

#[cfg(test)]
mod tests;

struct ModAssignmentInvariants<'a> {
    combat_mod_perms: Vec<[&'a ProcessMod; NUM_ITEM_BUCKETS]>,
    activity_mod_perms: Vec<[&'a ProcessMod; NUM_ITEM_BUCKETS]>,
    combat_mod_cost: u8,
    activity_mod_cost: u8,
    mod_set: BTreeMap<Stats, SomeMods<'a>>,
    lower: &'a [u8; NUM_STATS],
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn dim_lo_process(
    items: [&[ProcessItem]; NUM_ITEM_BUCKETS],
    general_mods: &[ProcessMod; NUM_ITEM_BUCKETS],
    combat_mods: &[ProcessMod; NUM_ITEM_BUCKETS],
    activity_mods: &[ProcessMod; NUM_ITEM_BUCKETS],
    optional_stat_mods: &[ProcessStatMod],
    args: &ProcessArgs,
) -> (ProcessStats, Vec<ProcessArmorSet>, ProcessMinMaxStats) {
    let mut info = ProcessStats::default();
    let mut max = [0u16; 6];
    let mut min = [100u16; 6];

    let empty_stat_mod = ProcessStatMod {
        inner_mod: ProcessMod {
            hash: None,
            mod_tag: None,
            energy_type: EnergyType::Any,
            energy_val: 0,
        },
        stats: Stats([0; 6]),
    };

    let mod_assignment_invars = {
        let activity_mod_perms = generate_permutations_of(activity_mods);
        let combat_mod_perms = generate_permutations_of(combat_mods);
        let activity_mod_cost = activity_mods.iter().map(|m| m.energy_val).sum();
        let combat_mod_cost = combat_mods.iter().map(|m| m.energy_val).sum();

        let num_stat_mods = general_mods.iter().filter(|m| m.hash.is_some()).count();

        // Generate all choices of stat mods along with the stats they provide,
        // with our pre-picked stat mods included.
        let mod_set = stat_mod_set::generate_mods_options(
            &general_mods[0..num_stat_mods],
            optional_stat_mods,
            &empty_stat_mod,
            args.auto_mods,
        );

        ModAssignmentInvariants {
            activity_mod_cost,
            activity_mod_perms,
            combat_mod_cost,
            combat_mod_perms,
            lower: &args.bounds.lower_bounds,
            mod_set,
        }
    };

    let mut set_tracker = SetTracker::new(10_000);

    for helm in items[0] {
        for gaunt in items[1] {
            if gaunt.exotic && helm.exotic {
                info.skipped_double_exotic +=
                    (items[2].len() * items[3].len() * items[4].len()) as u32;
                continue;
            }

            for chest in items[2] {
                if chest.exotic && (gaunt.exotic || helm.exotic) {
                    info.skipped_double_exotic += (items[3].len() * items[4].len()) as u32;
                    continue;
                }

                for leg in items[3] {
                    if leg.exotic && (chest.exotic || gaunt.exotic || helm.exotic) {
                        info.skipped_double_exotic += (items[4].len()) as u32;
                        continue;
                    }

                    if args.any_exotic
                        && !helm.exotic
                        && !gaunt.exotic
                        && !chest.exotic
                        && !leg.exotic
                    {
                        info.skipped_no_exotic += 1;
                        continue;
                    }

                    'classItemLoop: for class_item in items[4] {
                        let set = [helm, gaunt, chest, leg, class_item];
                        let stats = set
                            .iter()
                            .fold(args.base_stats, |stats, item| stats + item.stats);

                        // First, check what effective stats we end up with and whether we actually want this in the
                        // sets tracker.
                        let mut sorting_tiers =
                            stats.0.map(|s| s / 10).map(|s| s.clamp(0, 10) as u8);
                        let mut sorting_total_tier = 0;

                        for i in 0..NUM_STATS {
                            max[i] = core::cmp::max(max[i], stats.0[i].clamp(0, 100));
                            min[i] = core::cmp::min(min[i], stats.0[i].clamp(0, 100));
                            // If a stat has a maximum, we still show sets that have a higher tier,
                            // but we stop caring about the surplus. A user may specify that they
                            // want 5 mobility at most because Dragon's Shadow gives 5 bonus mobility
                            // after dodging, but hiding a really good T6 mobility set just because of
                            // that is wrong, we should just treat it as if it had T5 mobility.
                            if args.bounds.upper_bounds[i] < sorting_tiers[i] {
                                sorting_tiers[i] = args.bounds.upper_bounds[i];
                            }
                            sorting_total_tier += sorting_tiers[i];
                        }

                        if !set_tracker.could_insert(sorting_total_tier) {
                            info.skipped_low_tier += 1;
                            continue 'classItemLoop;
                        }

                        let result = can_take_mods_auto(set, &stats, &mod_assignment_invars);

                        match result {
                            StatModPickResults::Ok(pick) => {
                                info.num_valid_sets += 1;

                                set_tracker.insert(
                                    sorting_tiers,
                                    ProcessArmorSet {
                                        stats: pick.resulting_stats,
                                        items: set.map(|i| i.id),
                                        total_tier: sorting_total_tier,
                                        power: set.map(|i| i.power).iter().sum::<u16>() / 5,
                                        extra_stat_mods: pick.pick.map(|m| m.hash),
                                    },
                                );
                            }
                            StatModPickResults::AutoModsDidntFit
                            | StatModPickResults::ModsDidntFit => {
                                info.skipped_mods_unfit += 1;
                                continue;
                            }
                            StatModPickResults::LowStats => {
                                info.skipped_stat_range += 1;
                                continue;
                            }
                        }
                    }
                }
            }
        }
    }

    let sets = Vec::from_iter(set_tracker.sets_by_best().take(200));
    let min_max = ProcessMinMaxStats { min, max };
    (info, sets, min_max)
}

#[inline]
fn energies_match(item_energy: EnergyType, mod_energy: EnergyType) -> bool {
    item_energy == EnergyType::Any || mod_energy == EnergyType::Any || item_energy == mod_energy
}

#[inline]
fn energy_spec(md: &ProcessMod) -> (u8, EnergyType) {
    match md.hash {
        Some(_) => (md.energy_val, md.energy_type),
        None => (0, EnergyType::Any),
    }
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy)]
struct StatModPick<'a> {
    pick: &'a [&'a ProcessMod; NUM_ITEM_BUCKETS],
    resulting_stats: Stats,
}

enum StatModPickResults<'a> {
    Ok(StatModPick<'a>),
    ModsDidntFit,
    LowStats,
    AutoModsDidntFit,
}

#[inline(never)]
fn can_take_mods_auto<'a>(
    items: [&ProcessItem; NUM_ITEM_BUCKETS],
    base_stats: &Stats,
    invars: &'a ModAssignmentInvariants<'a>,
) -> StatModPickResults<'a> {
    let [any_items, specific_items @ ..] = get_energy_counts(&items);
    let [_, specific_combat @ ..] = get_energy_counts(&invars.combat_mod_perms[0]);
    let [_, specific_activity @ ..] = get_energy_counts(&invars.activity_mod_perms[0]);

    // Early exit if not enough pieces with element
    for ty in 0..4 {
        let matching_items = any_items + specific_items[ty];
        if matching_items < specific_combat[ty] || matching_items < specific_activity[ty] {
            return StatModPickResults::ModsDidntFit;
        }
    }

    // Early exit if not enough items with fitting slots
    if !invars.activity_mod_perms[0].is_empty() {
        let mut item_tags = items.map(|i| i.mod_tags);
        for act_mod in invars.activity_mod_perms[0]
            .iter()
            .filter_map(|p| p.mod_tag)
        {
            let fitting_item = item_tags.iter().position(|i| i & act_mod.get() != 0);
            match fitting_item {
                Some(idx) => item_tags[idx] &= !act_mod.get(),
                None => return StatModPickResults::ModsDidntFit,
            }
        }
    }

    // Check out which stats are missing to get to the lower bounds.
    // This always creates non-negative multiples of 5, which are
    // exactly the stats the auto stat mods map is keyed by.
    let mut contribution = Stats([0u16; NUM_STATS]);
    for i in 0..NUM_STATS {
        contribution.0[i] = (invars.lower[i] as u16 * 10).saturating_sub(base_stats.0[i]);
        contribution.0[i] += (5 - contribution.0[i] % 5) % 5;
    }

    // Retrieve the stat mod picks that could help us get to the minimum stats we need.
    // NB this includes our locked general mods
    let orig_options = match invars.mod_set.get(&contribution) {
        Some(mods) => mods.mods.as_slice(),
        None => return StatModPickResults::LowStats,
    };

    // (Unlikely, maybe not even worth including here)
    // Early exit if we don't have enough remaining energy for
    // any pick of mods, no matter bucket independent mod positions.
    let mut total_remaining_energy = items
        .iter()
        .map(|i| i.energy_cap - i.energy_val)
        .sum::<u8>() as i8;
    total_remaining_energy -= (invars.activity_mod_cost + invars.combat_mod_cost) as i8;
    if !orig_options
        .iter()
        .any(|o| (o.sum_cost as i8) <= total_remaining_energy)
    {
        return StatModPickResults::AutoModsDidntFit;
    }

    let mut assigned_mods_at_least_once = false;

    'activityModLoop: for activity_perm in &invars.activity_mod_perms {
        'activityItemLoop: for (i, &item) in items.iter().enumerate() {
            let activity_mod = activity_perm[i];
            if activity_mod.hash.is_none() {
                continue 'activityItemLoop;
            }

            match activity_mod.mod_tag {
                Some(tag) if item.mod_tags & tag.get() == 0 => continue 'activityModLoop,
                _ => {}
            }

            let activity_energy_valid = item.energy_val + activity_mod.energy_val
                <= item.energy_cap
                && energies_match(item.energy_type, activity_mod.energy_type);
            if !activity_energy_valid {
                continue 'activityModLoop;
            }
        }

        'combatModLoop: for combat_perm in &invars.combat_mod_perms {
            'combatItemLoop: for (i, &item) in items.iter().enumerate() {
                let combat_mod = combat_perm[i];
                if combat_mod.hash.is_none() {
                    continue 'combatItemLoop;
                }

                match combat_mod.mod_tag {
                    Some(tag) if item.mod_tags & tag.get() == 0 => continue 'combatModLoop,
                    _ => {}
                }

                let (activity_val, activity_type) = energy_spec(activity_perm[i]);

                let combat_energy_valid = item.energy_val + combat_mod.energy_val + activity_val
                    <= item.energy_cap
                    && energies_match(item.energy_type, combat_mod.energy_type)
                    && energies_match(combat_mod.energy_type, activity_type);
                if !combat_energy_valid {
                    continue 'combatModLoop;
                }
            }

            assigned_mods_at_least_once = true;

            // Activity and combat mods fit wrt tag, element, energy. Calculate the leftover energy per item
            let mut leftover_energy = [0; NUM_ITEM_BUCKETS];
            for i in 0..NUM_ITEM_BUCKETS {
                let (activity_energy_val, _) = energy_spec(activity_perm[i]);
                let (combat_energy_val, _) = energy_spec(combat_perm[i]);
                leftover_energy[i] = items[i].energy_cap
                    - items[i].energy_val
                    - activity_energy_val
                    - combat_energy_val;
            }

            leftover_energy.sort_by_key(|&x| core::cmp::Reverse(x));

            let pick = orig_options.iter().find(|&res| {
                res.sum_cost as i8 <= total_remaining_energy && fits(&leftover_energy, &res.costs)
            });

            if let Some(pick) = pick {
                let stats = *base_stats + contribution;
                return StatModPickResults::Ok(StatModPick {
                    pick: &pick.mods,
                    resulting_stats: stats,
                });
            }
        }
    }

    if assigned_mods_at_least_once {
        StatModPickResults::AutoModsDidntFit
    } else {
        StatModPickResults::ModsDidntFit
    }
}

fn fits(rem: &[u8; NUM_ITEM_BUCKETS], assign: &[u8; NUM_ITEM_BUCKETS]) -> bool {
    rem[0] >= assign[0]
        && rem[1] >= assign[1]
        && rem[2] >= assign[2]
        && rem[3] >= assign[3]
        && rem[4] >= assign[4]
}

trait Energy {
    fn energy(&self) -> EnergyType;
}
impl Energy for ProcessItem {
    fn energy(&self) -> EnergyType {
        self.energy_type
    }
}
impl Energy for ProcessMod {
    fn energy(&self) -> EnergyType {
        self.energy_type
    }
}

fn get_energy_counts<T: Energy>(items: &[&T; NUM_ITEM_BUCKETS]) -> [u8; NUM_ITEM_BUCKETS] {
    let mut energies = [0; NUM_ITEM_BUCKETS];
    for &item in items {
        energies[item.energy() as u8 as usize] += 1;
    }
    energies
}

const fn fac(n: usize) -> usize {
    match n {
        0 => 1,
        1 => 1,
        n => n * fac(n - 1),
    }
}

#[inline(never)]
fn generate_permutations_of<const N: usize>(items: &[ProcessMod; N]) -> Vec<[&ProcessMod; N]> {
    let mut cursors = [0; N];
    let mut items = items.each_ref();
    let mut retn = Vec::with_capacity(fac(N));
    retn.push(items);

    let mut i = 0;

    while i < 5 {
        if cursors[i] < i {
            if i % 2 == 0 {
                items.swap(0, i);
            } else {
                items.swap(cursors[i], i);
            }
            retn.push(items);
            cursors[i] += 1;
            i = 0;
        } else {
            cursors[i] = 0;
            i += 1;
        }
    }

    retn.sort_by_key(|p| p.map(|m| (m.energy_type, m.energy_val, m.mod_tag)));
    retn.dedup_by_key(|p| p.map(|m| (m.energy_type, m.energy_val, m.mod_tag)));
    retn
}
