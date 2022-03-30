use crate::types::{ProcessArmorSet, NUM_STATS};

struct StatMix {
    tiers: [u8; NUM_STATS],
    sets: Vec<ProcessArmorSet>,
}

struct TierSet {
    tier: u8,
    // Boxing the contents is really important because it avoids tons
    // of memmoves which are really slow in WASM. StatMix is 24 bytes,
    // a Box is 4.
    #[allow(clippy::vec_box)]
    mixes: Vec<Box<StatMix>>,
}

pub struct SetTracker {
    tiers: Vec<TierSet>,
    total_sets: usize,
    capacity: usize,
}

impl SetTracker {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            total_sets: 0,
            tiers: vec![],
        }
    }

    pub fn could_insert(&self, tier: u8) -> bool {
        let worst_tier = self.tiers.last().map_or(0, |t| t.tier);
        tier >= worst_tier || self.total_sets < self.capacity
    }

    pub fn insert(&mut self, tiers: [u8; NUM_STATS], set: ProcessArmorSet) {
        match self.tiers.binary_search_by(|p| set.total_tier.cmp(&p.tier)) {
            Ok(idx) => self.tiers[idx].insert(tiers, set),
            Err(idx) => {
                self.tiers.insert(
                    idx,
                    TierSet {
                        tier: set.total_tier,
                        mixes: vec![Box::new(StatMix {
                            tiers,
                            sets: vec![set],
                        })],
                    },
                );
            }
        }

        self.total_sets += 1;

        self.trim_worst();
    }

    fn trim_worst(&mut self) {
        if self.total_sets <= self.capacity {
            return;
        }

        let worst_tier_set = self.tiers.last_mut().unwrap();
        let worst_mix = worst_tier_set.mixes.last_mut().unwrap();
        worst_mix.sets.pop();
        if worst_mix.sets.is_empty() {
            worst_tier_set.mixes.pop();
        }
        if worst_tier_set.mixes.is_empty() {
            self.tiers.pop();
        }
        self.total_sets -= 1;
    }

    pub fn sets_by_best(self) -> impl Iterator<Item = ProcessArmorSet> {
        self.tiers
            .into_iter()
            .flat_map(|sets| sets.mixes.into_iter().flat_map(|m| m.sets))
    }
}

impl TierSet {
    fn insert(&mut self, tiers: [u8; NUM_STATS], set: ProcessArmorSet) {
        match self.mixes.binary_search_by(|s| tiers.cmp(&s.tiers)) {
            Ok(idx) => self.mixes[idx].insert(set),
            Err(idx) => self.mixes.insert(
                idx,
                Box::new(StatMix {
                    tiers,
                    sets: vec![set],
                }),
            ),
        }
    }
}

impl StatMix {
    fn insert(&mut self, set: ProcessArmorSet) {
        match self.sets.iter().position(|s| set.power > s.power) {
            Some(idx) => self.sets.insert(idx, set),
            None => self.sets.push(set),
        }
    }
}
