use std::mem::size_of;

use crate::Move;

use super::{MATE_SCORE, MAX_PLY};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Entry {
    pub key: u64,
    pub score: i32,
    pub best_move: Option<Move>,
    pub depth: u8,
    pub bound: Bound,
}

pub(super) struct TranspositionTable {
    entries: Box<[Option<Entry>]>,
}

impl TranspositionTable {
    pub fn new(bytes: usize) -> Self {
        let capacity = (bytes / size_of::<Option<Entry>>()).max(1);
        Self {
            entries: vec![None; capacity].into_boxed_slice(),
        }
    }

    pub fn probe(&self, key: u64) -> Option<Entry> {
        self.entries[self.index(key)].filter(|entry| entry.key == key)
    }

    pub fn store(&mut self, entry: Entry) {
        let index = self.index(entry.key);
        let replace = self.entries[index].is_none_or(|old| {
            if entry.key == old.key {
                entry.depth > old.depth
                    || (entry.depth == old.depth && (entry.bound == Bound::Exact || old.bound != Bound::Exact))
            } else {
                entry.depth >= old.depth
            }
        });
        if replace {
            self.entries[index] = Some(entry);
        }
    }

    fn index(&self, key: u64) -> usize {
        key as usize % self.entries.len()
    }
}

pub(super) fn score_to_tt(score: i32, ply: usize) -> i32 {
    if score >= MATE_SCORE - MAX_PLY as i32 {
        score + ply as i32
    } else if score <= -MATE_SCORE + MAX_PLY as i32 {
        score - ply as i32
    } else {
        score
    }
}

pub(super) fn score_from_tt(score: i32, ply: usize) -> i32 {
    if score >= MATE_SCORE - MAX_PLY as i32 {
        score - ply as i32
    } else if score <= -MATE_SCORE + MAX_PLY as i32 {
        score + ply as i32
    } else {
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: u64, depth: u8, score: i32, bound: Bound) -> Entry {
        Entry {
            key,
            score,
            best_move: None,
            depth,
            bound,
        }
    }

    #[test]
    fn probes_only_matching_full_keys() {
        let mut table = TranspositionTable::new(size_of::<Option<Entry>>());
        table.store(entry(1, 4, 25, Bound::Exact));
        assert_eq!(table.probe(1).map(|stored| stored.score), Some(25));

        table.store(entry(2, 5, 50, Bound::Lower));
        assert!(table.probe(1).is_none());
        assert_eq!(table.probe(2).map(|stored| stored.score), Some(50));
    }

    #[test]
    fn allocation_respects_the_requested_byte_budget() {
        let entry_size = size_of::<Option<Entry>>();
        let table = TranspositionTable::new(entry_size * 4);

        assert_eq!(table.entries.len(), 4);
    }

    #[test]
    fn shallower_collision_does_not_replace_deeper_entry() {
        let mut table = TranspositionTable::new(size_of::<Option<Entry>>());
        table.store(entry(1, 6, 60, Bound::Lower));
        table.store(entry(2, 5, 50, Bound::Exact));

        assert_eq!(table.probe(1).map(|stored| stored.depth), Some(6));
        assert!(table.probe(2).is_none());
    }

    #[test]
    fn exact_entry_wins_at_equal_depth_but_not_at_shallower_depth() {
        let mut table = TranspositionTable::new(size_of::<Option<Entry>>());
        table.store(entry(1, 6, 60, Bound::Lower));
        table.store(entry(1, 5, 50, Bound::Exact));
        assert_eq!(table.probe(1).map(|stored| stored.score), Some(60));

        table.store(entry(1, 6, 61, Bound::Exact));
        assert_eq!(table.probe(1).map(|stored| stored.score), Some(61));
        assert_eq!(table.probe(1).map(|stored| stored.bound), Some(Bound::Exact));
    }

    #[test]
    fn mate_scores_round_trip_at_different_plies() {
        let winning = MATE_SCORE - 7;
        let losing = -MATE_SCORE + 9;

        assert_eq!(score_from_tt(score_to_tt(winning, 7), 7), winning);
        assert_eq!(score_from_tt(score_to_tt(losing, 9), 9), losing);
        assert_eq!(score_from_tt(score_to_tt(winning, 7), 3), MATE_SCORE - 3);
        assert_eq!(score_from_tt(score_to_tt(losing, 9), 4), -MATE_SCORE + 4);
    }
}
