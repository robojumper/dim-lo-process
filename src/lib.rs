#![feature(array_methods)]
#![feature(array_windows)]
#![feature(vec_into_raw_parts)]

extern crate alloc;

use std::collections::BTreeMap;

use alloc::vec::Vec;
use set_tracker::SetTracker;
use stat_mod_set::StatProvider;
use types::{
    DestinyEnergyType, ProcessArmorSet, ProcessItem, ProcessMod, ProcessStatMod, ProcessStats,
    Stats, NO_TIER, NUM_ITEM_BUCKETS, NUM_STATS,
};

mod ffi;
mod set_tracker;
mod stat_mod_set;
pub mod types;

#[cfg(test)]
mod tests;

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn dim_lo_process(
    items: [&[ProcessItem]; NUM_ITEM_BUCKETS],
    general_mods: &[ProcessMod; 5],
    combat_mods: &[ProcessMod; 5],
    activity_mods: &[ProcessMod; 5],
    base_stats: Stats,
    optional_stat_mods: &[ProcessStatMod],
    auto_add_stat_mods: bool,
    lower_bounds: [u8; NUM_STATS],
    upper_bounds: [u8; NUM_STATS],
    any_exotic: bool,
) -> (
    ProcessStats,
    Vec<ProcessArmorSet>,
    [u16; NUM_STATS],
    [u16; NUM_STATS],
) {
    let mut info = ProcessStats::default();
    let mut max = [0u16; 6];
    let mut min = [100u16; 6];

    // let general_mod_perms = generate_permutations_of(general_mods);
    let activity_mod_perms = generate_permutations_of(activity_mods);
    let combat_mod_perms = generate_permutations_of(combat_mods);

    let num_stat_mods = general_mods.iter().filter(|m| m.hash.is_some()).count();

    let has_mods = num_stat_mods
        + activity_mods.iter().filter(|m| m.hash.is_some()).count()
        + combat_mods.iter().filter(|m| m.hash.is_some()).count()
        > 0;

    let mod_set = if auto_add_stat_mods && num_stat_mods < 5 {
        Some(stat_mod_set::generate_mods_options(
            &general_mods[0..num_stat_mods],
            optional_stat_mods,
        ))
    } else {
        None
    };

    let mut set_tracker = SetTracker::new(10_000);

    for helm in items[0] {
        for gaunt in items[1] {
            if gaunt.exotic != 0 && helm.exotic != 0 {
                info.skipped_double_exotic +=
                    (items[2].len() * items[3].len() * items[4].len()) as u32;
                continue;
            }

            for chest in items[2] {
                if chest.exotic != 0 && (gaunt.exotic != 0 || helm.exotic != 0) {
                    info.skipped_double_exotic += (items[3].len() * items[4].len()) as u32;
                    continue;
                }

                for leg in items[3] {
                    if leg.exotic != 0
                        && (chest.exotic != 0 || gaunt.exotic != 0 || helm.exotic != 0)
                    {
                        info.skipped_double_exotic += (items[4].len()) as u32;
                        continue;
                    }

                    if any_exotic
                        && helm.exotic == 0
                        && gaunt.exotic == 0
                        && chest.exotic == 0
                        && leg.exotic == 0
                    {
                        info.skipped_no_exotic += 1;
                        continue;
                    }

                    'classItemLoop: for class_item in items[4] {
                        let set = [helm, gaunt, chest, leg, class_item];
                        let stats = set
                            .iter()
                            .fold(base_stats, |stats, item| stats + item.stats);

                        if let Some(mod_set) = mod_set.as_ref() {
                            let result = can_take_mods_auto(
                                general_mods,
                                &combat_mod_perms,
                                &activity_mod_perms,
                                set,
                                &stats,
                                mod_set,
                                &lower_bounds,
                                &upper_bounds,
                            );

                            match result {
                                Some(pick) => {
                                    info.num_valid_sets += 1;

                                    set_tracker.insert(
                                        pick.tiers,
                                        ProcessArmorSet {
                                            stats: pick.resulting_stats,
                                            items: set.map(|i| i.id),
                                            total_tier: pick.resulting_total_tier,
                                            power: set.map(|i| i.power).iter().sum::<u16>() / 5,
                                            extra_stat_mods: pick.pick.map(|m| m.hash),
                                        },
                                    );
                                }
                                None => {
                                    info.skipped_mods_unfit += 1;
                                    continue;
                                }
                            }
                        } else {
                            let mut total_tier = 0;
                            let mut tiers = stats.0.map(|s| s / 10).map(|s| s.clamp(0, 10) as u8);
                            for i in 0..NUM_STATS {
                                if lower_bounds[i] != NO_TIER {
                                    if lower_bounds[i] > tiers[i] || upper_bounds[i] < tiers[i] {
                                        info.skipped_stat_range += 1;
                                        continue 'classItemLoop;
                                    }
                                    max[i] = core::cmp::max(max[i], stats.0[i].clamp(0, 100));
                                    min[i] = core::cmp::min(min[i], stats.0[i].clamp(0, 100));
                                    total_tier += tiers[i];
                                } else {
                                    tiers[i] = 0;
                                }
                            }

                            if !set_tracker.could_insert(total_tier) {
                                info.skipped_low_tier += 1;
                            }

                            if has_mods
                                && !can_take_mods(
                                    general_mods,
                                    &combat_mod_perms,
                                    &activity_mod_perms,
                                    set,
                                )
                            {
                                info.skipped_mods_unfit += 1;
                                continue;
                            }

                            info.num_valid_sets += 1;

                            set_tracker.insert(
                                tiers,
                                ProcessArmorSet {
                                    stats,
                                    items: set.map(|i| i.id),
                                    total_tier,
                                    power: set.map(|i| i.power).iter().sum::<u16>() / 5,
                                    extra_stat_mods: [None; 5],
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    let sets = Vec::from_iter(set_tracker.sets_by_best().take(200));
    (info, sets, min, max)
}

#[inline]
fn energies_match(item_energy: DestinyEnergyType, mod_energy: DestinyEnergyType) -> bool {
    item_energy == DestinyEnergyType::Any
        || mod_energy == DestinyEnergyType::Any
        || item_energy == mod_energy
}

#[inline]
fn energy_spec(md: &ProcessMod) -> (u8, DestinyEnergyType) {
    match md.hash {
        Some(_) => (md.energy_val, md.energy_type),
        None => (0, DestinyEnergyType::Any),
    }
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy)]
struct StatModPickResults<'a> {
    pick: &'a [&'a ProcessMod; 5],
    tiers: [u8; NUM_STATS],
    resulting_total_tier: u8,
    resulting_stats: Stats,
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn can_take_mods_auto<'a>(
    general_mods: &[ProcessMod; 5],
    combat_mod_perms: &[[&ProcessMod; 5]],
    activity_mod_perms: &[[&ProcessMod; 5]],
    items: [&ProcessItem; 5],
    base_stats: &Stats,
    mod_set: &'a BTreeMap<Stats, StatProvider<'a>>,
    lower: &[u8; NUM_STATS],
    upper: &[u8; NUM_STATS],
) -> Option<StatModPickResults<'a>> {
    let [any_items, specific_items @ ..] = get_energy_counts(&items);
    let [_, specific_combat @ ..] = get_energy_counts(&combat_mod_perms[0]);
    let [_, specific_activity @ ..] = get_energy_counts(&activity_mod_perms[0]);

    // Early exit if not enough pieces with element
    for ty in 0..4 {
        let matching_items = any_items + specific_items[ty];
        if matching_items < specific_combat[ty] || matching_items < specific_activity[ty] {
            return None;
        }
    }

    // Early exit if not enough items with fitting slots
    if !activity_mod_perms[0].is_empty() {
        let mut item_tags = items.map(|i| i.mod_tags);
        for act_mod in activity_mod_perms[0].iter().filter_map(|p| p.mod_tag) {
            let fitting_item = item_tags.iter().position(|i| i & act_mod.get() != 0);
            match fitting_item {
                Some(idx) => item_tags[idx] &= !act_mod.get(),
                None => return None,
            }
        }
    }

    // Sort general mod costs descending
    let mut general_mod_costs = general_mods.each_ref().map(|m| m.energy_val);
    general_mod_costs.sort_by_key(|&x| core::cmp::Reverse(x));

    let mut contribution = Stats([0u16; NUM_STATS]);
    for i in 0..NUM_STATS {
        if lower[i] != NO_TIER {
            contribution.0[i] = (lower[i] as u16 * 10).saturating_sub(base_stats.0[i]);
            contribution.0[i] += (5 - contribution.0[i] % 5) % 5;
        }
    }

    // get all stat mod options we could fit in here
    let orig_options = match mod_set.get(&contribution) {
        Some(mods) => mods.mods.as_slice(),
        None => return None,
    };

    let mut leftover_energies = vec![];

    'activityModLoop: for activity_perm in activity_mod_perms {
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

        'combatModLoop: for combat_perm in combat_mod_perms {
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
            leftover_energies.push(leftover_energy);

            if let &[leftover_energy, ..] = &leftover_energies[..] {
                // Ask our mod picks set for the stat mods we can fit in here
                let pick = orig_options
                    .iter()
                    .find(|&res| fits(&leftover_energy, &res.costs));
        
                if let Some(pick) = pick {
                    let stats = *base_stats + contribution;
                    let mut total_tier = 0;
                    let mut tiers = stats.0.map(|s| s / 10).map(|s| s.clamp(0, 10) as u8);
                    for i in 0..NUM_STATS {
                        if lower[i] != NO_TIER {
                            total_tier += tiers[i];
                        } else {
                            tiers[i] = 0;
                        }
                    }
                    return Some(StatModPickResults {
                        pick: &pick.mods,
                        tiers,
                        resulting_total_tier: total_tier,
                        resulting_stats: stats,
                    });
                }
            }
        }
    }

    None
}

fn fits(rem: &[u8; 5], assign: &[u8; 5]) -> bool {
    rem[0] >= assign[0]
        && rem[1] >= assign[1]
        && rem[2] >= assign[2]
        && rem[3] >= assign[3]
        && rem[4] >= assign[4]
}

#[inline(never)]
fn can_take_mods(
    general_mod_perms: &[ProcessMod; 5],
    combat_mod_perms: &[[&ProcessMod; 5]],
    activity_mod_perms: &[[&ProcessMod; 5]],
    items: [&ProcessItem; 5],
) -> bool {
    let [any_items, specific_items @ ..] = get_energy_counts(&items);
    let [_, specific_combat @ ..] = get_energy_counts(&combat_mod_perms[0]);
    let [_, specific_activity @ ..] = get_energy_counts(&activity_mod_perms[0]);

    // Sort general mod costs descending
    let mut general_mod_costs = general_mod_perms.each_ref().map(|m| m.energy_val);
    general_mod_costs.sort_by_key(|&x| core::cmp::Reverse(x));

    let mut allowed = false;

    // Early exit if not enough pieces with element
    for ty in 0..4 {
        let matching_items = any_items + specific_items[ty];
        if matching_items < specific_combat[ty] || matching_items < specific_activity[ty] {
            return false;
        }
    }

    // Early exit if not enough items with fitting slots
    if !activity_mod_perms[0].is_empty() {
        let mut item_tags = items.map(|i| i.mod_tags);
        for act_mod in activity_mod_perms[0].iter().filter_map(|p| p.mod_tag) {
            let fitting_item = item_tags.iter().position(|i| i & act_mod.get() != 0);
            match fitting_item {
                Some(idx) => item_tags[idx] &= !act_mod.get(),
                None => return false,
            }
        }
    }

    'activityModLoop: for activity_perm in activity_mod_perms {
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

        'combatModLoop: for combat_perm in combat_mod_perms {
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

            allowed = fits(&leftover_energy, &general_mod_costs) || allowed;

            if !fits(&leftover_energy, &general_mod_costs) {
                continue 'combatModLoop;
            }

            return true;
        }
    }

    allowed
}

trait Energy {
    fn energy(&self) -> DestinyEnergyType;
}
impl Energy for ProcessItem {
    fn energy(&self) -> DestinyEnergyType {
        self.energy_type
    }
}
impl Energy for ProcessMod {
    fn energy(&self) -> DestinyEnergyType {
        self.energy_type
    }
}

fn get_energy_counts<T: Energy>(items: &[&T; 5]) -> [u8; 5] {
    let mut energies = [0; 5];
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
