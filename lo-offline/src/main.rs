#![feature(array_methods)]

use std::{env, fs::File, io, num::NonZeroU32};

use serde::Deserialize;
use serde_repr::Deserialize_repr;

use dim_lo_core::{
    dim_lo_process,
    types::{
        EnergyType, ProcessArgs, ProcessItem, ProcessMod, ProcessStatMod, ProcessStats,
        ProcessTierBounds, Stats, NUM_ITEM_BUCKETS, NUM_STATS,
    },
};

#[repr(u8)]
#[derive(Clone, Copy, Deserialize_repr)]
enum DimEnergyType {
    Any = 0,
    Arc = 1,
    Solar = 2,
    Void = 3,
    Stasis = 4,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DimItemEnergy {
    r#type: DimEnergyType,
    capacity: u8,
    val: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DimModEnergy {
    r#type: DimEnergyType,
    val: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DimAutoStatMod {
    hash: u32,
    energy: DimModEnergy,
    investment_stats: [u16; NUM_STATS],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DimMod {
    hash: u32,
    energy: DimModEnergy,
    tag: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DimStatFilter {
    min: u8,
    max: u8,
    ignored: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DimItem {
    is_exotic: bool,
    power: u16,
    id: String,
    name: String,
    stats: [u16; NUM_STATS],
    energy: DimItemEnergy,
    compatible_mod_seasons: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DimLockedMods {
    general_mods: Vec<DimMod>,
    combat_mods: Vec<DimMod>,
    activity_mods: Vec<DimMod>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DimExport {
    filtered_items: [Vec<DimItem>; NUM_ITEM_BUCKETS],
    mod_stat_totals: [u16; NUM_STATS],
    auto_stat_mods: Vec<DimAutoStatMod>,
    locked_mods: DimLockedMods,
    stat_filters: [DimStatFilter; NUM_STATS],
    any_exotic: bool,
}

fn map_energy(e: DimEnergyType) -> EnergyType {
    match e {
        DimEnergyType::Any => EnergyType::Any,
        DimEnergyType::Arc => EnergyType::Arc,
        DimEnergyType::Solar => EnergyType::Solar,
        DimEnergyType::Void => EnergyType::Void,
        DimEnergyType::Stasis => EnergyType::Stasis,
    }
}

fn map_stat_mod(m: &DimAutoStatMod) -> ProcessStatMod {
    ProcessStatMod {
        inner_mod: ProcessMod {
            hash: NonZeroU32::new(m.hash),
            mod_tag: None,
            energy_type: map_energy(m.energy.r#type),
            energy_val: m.energy.val,
        },
        stats: Stats(m.investment_stats),
    }
}

fn map_mod(m: &DimMod, get_tag: &mut dyn FnMut(&str) -> NonZeroU32) -> ProcessMod {
    ProcessMod {
        hash: NonZeroU32::new(m.hash),
        mod_tag: m.tag.as_ref().map(|t| get_tag(t)),
        energy_type: map_energy(m.energy.r#type),
        energy_val: m.energy.val,
    }
}

const EMPTY_MOD: ProcessMod = ProcessMod {
    hash: None,
    mod_tag: None,
    energy_type: EnergyType::Any,
    energy_val: 0,
};

fn main() -> Result<(), io::Error> {
    let path = env::args()
        .nth(1)
        .expect("provide path to exported json on command line");

    let dim_export: DimExport = serde_json::from_reader(io::BufReader::new(File::open(path)?))?;

    let auto_mods = dim_export
        .auto_stat_mods
        .iter()
        .map(map_stat_mod)
        .collect::<Vec<_>>();

    let mut tag_list: Vec<String> = vec![];
    let mut get_tag = |tag: &str| {
        let idx = match tag_list.iter().position(|x| x == tag) {
            Some(idx) => idx,
            None => {
                let idx = tag_list.len();
                tag_list.push(tag.to_owned());
                idx
            }
        };
        NonZeroU32::new(1 << idx).unwrap()
    };

    let mut map_mods = |mods: &[DimMod]| -> [ProcessMod; NUM_ITEM_BUCKETS] {
        let mut mods = mods
            .iter()
            .map(|m| map_mod(m, &mut get_tag))
            .collect::<Vec<_>>();
        while mods.len() < 5 {
            mods.push(EMPTY_MOD);
        }
        match mods.try_into() {
            Ok(x) => x,
            Err(_) => unreachable!(),
        }
    };
    let general_mods = map_mods(&dim_export.locked_mods.general_mods);
    let combat_mods = map_mods(&dim_export.locked_mods.combat_mods);
    let activity_mods = map_mods(&dim_export.locked_mods.activity_mods);

    let mut lower = [0; NUM_STATS];
    let mut upper = [0; NUM_STATS];
    for (idx, filter) in dim_export.stat_filters.iter().enumerate() {
        if filter.ignored {
            lower[idx] = 0;
            lower[idx] = 10;
        } else {
            lower[idx] = filter.min;
            upper[idx] = filter.max;
        }
    }

    let mut item_backrefs = vec![];
    let mut track_item = |it: &DimItem| {
        let len = item_backrefs.len();
        item_backrefs.push((it.name.clone(), it.id.clone()));
        len as u16
    };

    let items = dim_export.filtered_items.each_ref().map(|l| {
        l.iter()
            .map(|item| {
                let idx = track_item(item);
                ProcessItem {
                    id: idx,
                    power: item.power,
                    energy_type: map_energy(item.energy.r#type),
                    energy_val: item.energy.val,
                    energy_cap: item.energy.capacity,
                    exotic: item.is_exotic,
                    mod_tags: item
                        .compatible_mod_seasons
                        .iter()
                        .fold(0, |acc, season| acc | get_tag(season).get()),
                    stats: Stats(item.stats),
                }
            })
            .collect::<Vec<_>>()
    });
    let sliced = items.each_ref().map(|x| &**x);

    let args = ProcessArgs {
        base_stats: Stats(dim_export.mod_stat_totals),
        bounds: ProcessTierBounds {
            lower_bounds: lower,
            upper_bounds: upper,
        },
        any_exotic: dim_export.any_exotic,
        auto_mods: 5,
    };

    let (info, results, min_max) = dim_lo_process(
        sliced,
        &general_mods,
        &combat_mods,
        &activity_mods,
        &auto_mods,
        &args,
    );

    let ProcessStats {
        num_valid_sets,
        skipped_low_tier,
        skipped_stat_range,
        skipped_mods_unfit,
        skipped_double_exotic,
        skipped_no_exotic,
    } = info;

    println!(
        r#"Completed LO Run.
Num Valid Sets: {num_valid_sets}
Skipped Low Tier: {skipped_low_tier}
Skipped Stat Range: {skipped_stat_range}
Skipped Mods Didn't Fit: {skipped_mods_unfit}
Skipped Double Exotic: {skipped_double_exotic}
Skipped No Exotic: {skipped_no_exotic}
"#
    );

    println!("MinMax: {:?} - {:?}", min_max.min, min_max.max);
    println!("Num Results: {}", results.len());

    Ok(())
}
