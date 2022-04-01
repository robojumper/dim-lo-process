use crate::types::{ProcessMod, ProcessStatMod, Stats};

use self::generate_picks::StatProvider;

/// A Set of mod picks. There are about 6000 different ways to pick stat mods
/// that result in different stats and energy usages and we need every single
/// one of them (up to 5 different ways to get some stats too!).
///
/// We should really do something smarter here but let's just check how large
/// the performance hit really is here. May want a hybrid approach.
pub struct ModPickSet<'a> {
    contents: Vec<ModPick<'a>>,
}

#[cfg_attr(test, derive(Debug))]
pub struct ModPick<'a> {
    /// The stats for sorting.
    pub stats: Stats,
    /// The costs of these mod picks and the existing stat mod picks,
    /// sorted descending to help with the greedy assignment algorithm.
    pub costs: [u8; 5],
    /// 1-5 stat mods.
    pub extra_mods: [&'a ProcessMod; 5],
}

impl<'a> ModPick<'a> {
    fn new(stats: Stats, existing_costs: &[u8], extra_mods: [&'a ProcessMod; 5]) -> Self {
        let mut costs = existing_costs
            .iter()
            .copied()
            .chain(
                extra_mods
                    .iter()
                    .filter(|m| m.hash.is_some())
                    .map(|ex| ex.energy_val),
            )
            .chain(core::iter::repeat(0))
            .take(5);
        let mut costs = [
            costs.next().unwrap(),
            costs.next().unwrap(),
            costs.next().unwrap(),
            costs.next().unwrap(),
            costs.next().unwrap(),
        ];
        costs.sort_by_key(|&x| core::cmp::Reverse(x));
        Self {
            stats,
            costs,
            extra_mods,
        }
    }
}

impl<'a> ModPickSet<'a> {
    pub fn new(existing_mods: &[ProcessMod], mod_options: &'a [ProcessStatMod]) -> Self {
        // First, generate all picks limited by existing mods
        let picks = generate_picks::generate_mods_options(mod_options, 5 - existing_mods.len());
        // Now we have a bunch of points keyed by their provided stats
        let mut all_picks = picks.into_iter().collect::<Vec<_>>();
        // Sort by stats
        all_picks.sort_by_key(|pick| pick.0);

        let existing_costs = existing_mods
            .iter()
            .map(|ex| ex.energy_val)
            .collect::<Vec<_>>();

        Self {
            contents: all_picks
                .into_iter()
                .flat_map(|(stats, StatProvider { mods })| {
                    mods.as_slice()
                        .iter()
                        .map(|m| ModPick::new(stats, &existing_costs, m.0))
                        .collect::<Vec<_>>()
                })
                .collect(),
        }
    }

    /// Extract the possible mod picks for the stat ranges. This essentially
    /// extracts a hypercube of mod picks from our stats space, but in a stupid way.
    pub fn get_options<'c, 'b: 'c>(
        &'b self,
        min: &'c Stats,
        max: &'c Stats,
    ) -> impl Iterator<Item = &'b ModPick<'a>> + 'c {
        self.contents.iter().filter(|s| {
            s.stats.0[0] >= min.0[0]
                && s.stats.0[1] >= min.0[1]
                && s.stats.0[2] >= min.0[2]
                && s.stats.0[3] >= min.0[3]
                && s.stats.0[4] >= min.0[4]
                && s.stats.0[5] >= min.0[5]
                && s.stats.0[0] <= max.0[0]
                && s.stats.0[1] <= max.0[1]
                && s.stats.0[2] <= max.0[2]
                && s.stats.0[3] <= max.0[3]
                && s.stats.0[4] <= max.0[4]
                && s.stats.0[5] <= max.0[5]
        })
    }
}

mod generate_picks {
    use alloc::collections::BTreeMap;
    use pareto_front::{Dominate, ParetoFront};

    use crate::types::{ProcessMod, ProcessStatMod, Stats};

    #[cfg_attr(test, derive(Debug))]
    pub struct ModsArray<'a>(pub [&'a ProcessMod; 5]);

    #[cfg_attr(test, derive(Debug))]
    pub struct StatProvider<'a> {
        pub mods: ParetoFront<ModsArray<'a>>,
    }

    impl Dominate for ModsArray<'_> {
        fn dominate(&self, x: &Self) -> bool {
            self.0[0].energy_val <= x.0[0].energy_val
                && self.0[1].energy_val <= x.0[1].energy_val
                && self.0[2].energy_val <= x.0[2].energy_val
                && self.0[3].energy_val <= x.0[3].energy_val
                && self.0[4].energy_val <= x.0[4].energy_val
        }
    }

    // Core observation: For any choice of stat mods involving a +10 mod that ends up overshooting by at least 5, a +5 mod suffices

    pub fn generate_mods_options(
        mods: &[ProcessStatMod],
        num_auto_mods_available: usize,
    ) -> BTreeMap<Stats, StatProvider<'_>> {
        // 13^5 = 371,293 possible assignments
        let mut map = BTreeMap::new();

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
                                > num_auto_mods_available
                            {
                                continue;
                            }

                            let stats =
                                mod0.stats + mod1.stats + mod2.stats + mod3.stats + mod4.stats;

                            mod_list.sort_by_key(|&m| core::cmp::Reverse(m.energy_val));

                            let entry = map.entry(stats).or_insert_with(|| StatProvider {
                                mods: ParetoFront::new(),
                            });

                            entry.mods.push(ModsArray(mod_list));
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

        #[test]
        fn doit() {
            /*
            let stuff = super::generate_mods_options(&SAMPLE_MODS, 1);
            let max = stuff.iter().max_by_key(|k| k.1.mods.as_slice().len());
            println!("{:#?}", stuff);
            println!("{:#?}", max);
            println!("{:#?}", stuff.len());
            println!(
                "{:#?}",
                stuff
                    .iter()
                    .map(|x| x.1.mods.as_slice().len())
                    .sum::<usize>()
            );
             */
        }
    }
}
