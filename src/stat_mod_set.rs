use alloc::collections::BTreeMap;
use pareto_front::{Dominate, ParetoFront};

use crate::types::{ProcessMod, ProcessStatMod, Stats};

#[cfg_attr(test, derive(Debug))]
pub struct ModsArray<'a> {
    pub costs: [u8; 5],
    pub mods: [&'a ProcessMod; 5],
}

#[cfg_attr(test, derive(Debug))]
pub struct StatProvider<'a> {
    pub mods: ParetoFront<ModsArray<'a>>,
}

impl Dominate for ModsArray<'_> {
    fn dominate(&self, x: &Self) -> bool {
        self.mods[0].energy_val <= x.mods[0].energy_val
            && self.mods[1].energy_val <= x.mods[1].energy_val
            && self.mods[2].energy_val <= x.mods[2].energy_val
            && self.mods[3].energy_val <= x.mods[3].energy_val
            && self.mods[4].energy_val <= x.mods[4].energy_val
    }
}

// Core observation: For any choice of stat mods involving a +10 mod that ends up overshooting by at least 5, a +5 mod suffices

pub fn generate_mods_options<'a>(
    existing_mods: &[ProcessMod],
    mods: &'a [ProcessStatMod],
) -> BTreeMap<Stats, StatProvider<'a>> {
    // 13^5 = 371,293 possible assignments
    let mut map = BTreeMap::new();
    let existing_costs = existing_mods
        .iter()
        .filter(|m| m.hash.is_some())
        .map(|m| m.energy_val)
        .collect::<Vec<_>>();

    for mod0 in mods {
        for mod1 in mods {
            for mod2 in mods {
                for mod3 in mods {
                    for mod4 in mods {
                        let mut mod_list = [
                            &mod0.inner_mod,
                            &mod1.inner_mod,
                            &mod2.inner_mod,
                            &mod3.inner_mod,
                            &mod4.inner_mod,
                        ];

                        if mod_list.iter().filter(|m| m.hash.is_some()).count()
                            > 5 - existing_mods.len()
                        {
                            continue;
                        }

                        mod_list.sort_by_key(|&m| core::cmp::Reverse(m.energy_val));

                        let mut costs = existing_costs
                            .iter()
                            .copied()
                            .chain(
                                mod_list
                                    .iter()
                                    .filter(|m| m.hash.is_some())
                                    .map(|ex| ex.energy_val),
                            )
                            .chain(core::iter::repeat(0));

                        let mut costs = [
                            costs.next().unwrap(),
                            costs.next().unwrap(),
                            costs.next().unwrap(),
                            costs.next().unwrap(),
                            costs.next().unwrap(),
                        ];
                        costs.sort_by_key(|&x| core::cmp::Reverse(x));

                        let stats = mod0.stats + mod1.stats + mod2.stats + mod3.stats + mod4.stats;

                        let entry = map.entry(stats).or_insert_with(|| StatProvider {
                            mods: ParetoFront::new(),
                        });

                        entry.mods.push(ModsArray {
                            mods: mod_list,
                            costs,
                        });
                    }
                }
            }
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use crate::tests::SAMPLE_MODS;

    /*
    #[test]
    fn doit() {
        let stuff = super::generate_mods_options(&[], &SAMPLE_MODS);
        let max = stuff.iter().max_by_key(|k| k.1.mods.as_slice().len());
        // println!("{:#?}", stuff);
        println!("{:#?}", max);
        println!("{:#?}", stuff.len());
        println!(
            "{:#?}",
            stuff
                .iter()
                .map(|x| x.1.mods.as_slice().len())
                .sum::<usize>()
        );
    }
    */
}
