use alloc::{vec::Vec, collections::BTreeMap};

use crate::types::{ProcessArmorSet, NUM_STATS};

/// Tiers in stat order for purposes of sorting only!
/// We don't count tiers beyond what the user set as max (e.g. if the
/// user says max mobility 5 and we have 7, we treat this as if it had mobility 5),
/// and we also don't count auto stat mods (they're not interesting because they
/// only ever buff bad sets that need stat mods in the first place)
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct SetSortingKey {
    sorting_total_tier: u8,
    sorting_tiers: [u8; NUM_STATS],
}

pub struct SetTracker {
    tracker: BTreeMap<SetSortingKey, Vec<ProcessArmorSet>>,
    capacity: usize,
}

impl SetTracker {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            tracker: BTreeMap::new(),
        }
    }

    pub fn could_insert(&self, tier: u8) -> bool {
        match self.tracker.first_key_value() {
            Some((k, _)) => k.sorting_total_tier <= tier,
            None => true,
        }
    }

    /// Insert a set into the tracker with the given
    pub fn insert(&mut self, sorting_tiers: [u8; NUM_STATS], set: ProcessArmorSet) {
        let key = SetSortingKey {
            sorting_total_tier: set.total_tier,
            sorting_tiers,
        };

        self.tracker.entry(key).or_insert_with(Vec::new).push(set);
        self.trim_worst();
    }

    fn trim_worst(&mut self) {
        if self.tracker.len() <= self.capacity {
            return;
        }

        let mut worst_entry = self.tracker.first_entry().unwrap();
        worst_entry.get_mut().pop();
        if worst_entry.get_mut().is_empty() {
            self.tracker.pop_first();
        }
    }

    pub fn sets_by_best(self) -> impl Iterator<Item = ProcessArmorSet> {
        self.tracker.into_iter().rev().flat_map(|(_ , val)| val)
    }
}
