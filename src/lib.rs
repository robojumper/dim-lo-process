#![feature(array_methods)]
#![feature(array_windows)]
#![feature(vec_into_raw_parts)]

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use set_tracker::SetTracker;
use types::{
    DestinyEnergyType, ProcessArmorSet, ProcessItem, ProcessMod, ProcessStats, Stats, NO_TIER,
    NUM_ITEM_BUCKETS, NUM_STATS,
};

mod ffi;
mod set_tracker;
mod types;

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn dim_lo_process(
    items: [&[ProcessItem]; NUM_ITEM_BUCKETS],
    general_mods: &[ProcessMod; 5],
    combat_mods: &[ProcessMod; 5],
    activity_mods: &[ProcessMod; 5],
    base_stats: Stats,
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

    let general_mod_perms = generate_permutations_of(general_mods);
    let activity_mod_perms = generate_permutations_of(activity_mods);
    let combat_mod_perms = generate_permutations_of(combat_mods);

    let has_mods = general_mods.iter().filter(|m| m.hash.is_some()).count()
        + activity_mods.iter().filter(|m| m.hash.is_some()).count()
        + combat_mods.iter().filter(|m| m.hash.is_some()).count()
        > 0;

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
                        let mut tiers = stats.0.map(|s| s / 10).map(|s| s.clamp(0, 10) as u8);

                        let mut total_tier = 0;
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
                                &general_mod_perms,
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
                            },
                        );
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

#[inline(never)]
fn can_take_mods(
    general_mod_perms: &[[&ProcessMod; 5]],
    combat_mod_perms: &[[&ProcessMod; 5]],
    activity_mod_perms: &[[&ProcessMod; 5]],
    items: [&ProcessItem; 5],
) -> bool {
    let [any_items, specific_items @ ..] = get_energy_counts(&items);
    let [_, specific_general @ ..] = get_energy_counts(&general_mod_perms[0]);
    let [_, specific_combat @ ..] = get_energy_counts(&combat_mod_perms[0]);
    let [_, specific_activity @ ..] = get_energy_counts(&activity_mod_perms[0]);

    // Early exit if not enough pieces with element
    for ty in 0..4 {
        let matching_items = any_items + specific_items[ty];
        if matching_items < specific_general[ty]
            || matching_items < specific_combat[ty]
            || matching_items < specific_activity[ty]
        {
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

            'generalModLoop: for general_perm in general_mod_perms {
                'generalItemLoop: for (i, &item) in items.iter().enumerate() {
                    let general_mod = general_perm[i];
                    if general_mod.hash.is_none() {
                        continue 'generalItemLoop;
                    }

                    let (activity_val, activity_type) = energy_spec(activity_perm[i]);
                    let (combat_val, combat_type) = energy_spec(combat_perm[i]);

                    match general_mod.mod_tag {
                        Some(tag) if item.mod_tags & tag.get() == 0 => continue 'generalModLoop,
                        _ => {}
                    }

                    let general_energy_valid =
                        item.energy_val + general_mod.energy_val + combat_val + activity_val
                            <= item.energy_cap
                            && energies_match(item.energy_type, general_mod.energy_type)
                            && energies_match(general_mod.energy_type, activity_type)
                            && energies_match(general_mod.energy_type, combat_type);
                    if !general_energy_valid {
                        continue 'generalModLoop;
                    }
                }

                return true;
            }
        }
    }

    false
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
fn generate_permutations_of<const N: usize>(items: &[ProcessMod; N]) -> Box<[[&ProcessMod; N]]> {
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
    retn.into_boxed_slice()
}
