// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
//! Methods that compare items across the whole set of lists instead of adding per-list contributions: Condorcet,
//! Copeland and median rank. Each returns `(id, score)` pairs in first-appearance order (lists in the order given, items
//! in list order), so no hash order reaches the output (contract D14). Lists must already be validated (no duplicate id
//! inside one list) and weights validated (finite, non-negative, not all zero).
//!
//! Derived from rankops 0.3.0 `condorcet_multi`, `copeland_multi` and `median_rank_multi` (see PROVENANCE.md); the
//! unweighted results are the same, and per-list weights (contract D13) are honoured where rankops has none.

use std::collections::HashMap;
use std::hash::Hash;

/// Every distinct id in first-appearance order, and for each the 0-indexed position in each list (`None` if absent).
fn rank_matrix<I: Clone + Eq + Hash>(lists: &[&[I]]) -> (Vec<I>, Vec<Vec<Option<usize>>>) {
    let mut index: HashMap<I, usize> = HashMap::new();
    let mut ids: Vec<I> = Vec::new();
    let mut ranks: Vec<Vec<Option<usize>>> = Vec::new();
    for (l, list) in lists.iter().enumerate() {
        for (pos, id) in list.iter().enumerate() {
            let i = *index.entry(id.clone()).or_insert_with(|| {
                ids.push(id.clone());
                ranks.push(vec![None; lists.len()]);
                ids.len() - 1
            });
            ranks[i][l] = Some(pos);
        }
    }
    (ids, ranks)
}

/// Total weight of the lists in which `a` is preferred to `b`, and of those preferring `b` to `a`. A list prefers the
/// item with the better (smaller) position; an item that is present beats one that is absent; a list containing neither
/// has no preference.
fn preference_weights(a: &[Option<usize>], b: &[Option<usize>], weights: &[f64]) -> (f64, f64) {
    let (mut for_a, mut for_b) = (0.0, 0.0);
    for ((ra, rb), &w) in a.iter().zip(b).zip(weights) {
        match (ra, rb) {
            (Some(x), Some(y)) if x < y => for_a += w,
            (Some(x), Some(y)) if y < x => for_b += w,
            (Some(_), None) => for_a += w,
            (None, Some(_)) => for_b += w,
            _ => {}
        }
    }
    (for_a, for_b)
}

/// Condorcet: the score is the number of other items the item beats, where it beats another when the lists preferring it
/// carry more than half of the total weight (with unit weights: a strict majority of all lists, absent lists counting
/// against it). This is the simplified method, not Kemeny optimal.
pub fn condorcet<I: Clone + Eq + Hash>(lists: &[&[I]], weights: &[f64]) -> Vec<(I, f64)> {
    let (ids, ranks) = rank_matrix(lists);
    let total: f64 = weights.iter().sum();
    ids.into_iter()
        .enumerate()
        .map(|(i, id)| {
            let wins = (0..ranks.len())
                .filter(|&j| {
                    j != i && {
                        let (for_i, _) = preference_weights(&ranks[i], &ranks[j], weights);
                        for_i * 2.0 > total
                    }
                })
                .count();
            (id, wins as f64)
        })
        .collect()
}

/// Copeland: the score is pairwise wins minus pairwise losses, where the lists carrying more weight decide each pair and
/// a pair with equal weight on both sides is a tie (neither a win nor a loss).
pub fn copeland<I: Clone + Eq + Hash>(lists: &[&[I]], weights: &[f64]) -> Vec<(I, f64)> {
    let (ids, ranks) = rank_matrix(lists);
    ids.into_iter()
        .enumerate()
        .map(|(i, id)| {
            let mut net = 0.0;
            for j in (0..ranks.len()).filter(|&j| j != i) {
                let (for_i, for_j) = preference_weights(&ranks[i], &ranks[j], weights);
                if for_i > for_j {
                    net += 1.0;
                } else if for_j > for_i {
                    net -= 1.0;
                }
            }
            (id, net)
        })
        .collect()
}

/// Median rank: the weighted median of an item's 0-indexed positions, where a list not containing it counts as the
/// penalty position `longest list length + 1`. The score is `1 / (1 + median)`, so a lower median ranks higher.
///
/// The weighted median is the first position (in ascending order) at which the cumulative weight reaches half the total;
/// if it reaches exactly half, it is the midpoint of that position and the next one. With unit weights this is the usual
/// median (the middle value, or the mean of the two middle values for an even count). Exact-half detection is exact for
/// weights that are exactly representable sums; other weights can land on either side of half by rounding.
pub fn median_rank<I: Clone + Eq + Hash>(lists: &[&[I]], weights: &[f64]) -> Vec<(I, f64)> {
    let (ids, ranks) = rank_matrix(lists);
    let penalty = lists.iter().map(|l| l.len()).max().unwrap_or(0) + 1;
    let total: f64 = weights.iter().sum();
    ids.into_iter()
        .zip(ranks)
        .map(|(id, r)| {
            let mut pts: Vec<(usize, f64)> = r
                .iter()
                .zip(weights)
                .map(|(p, &w)| (p.unwrap_or(penalty), w))
                .collect();
            pts.sort_by_key(|p| p.0);
            let mut cum = 0.0;
            let mut median = 0.0;
            for (k, &(pos, w)) in pts.iter().enumerate() {
                cum += w;
                if cum * 2.0 >= total {
                    median = match pts.get(k + 1) {
                        Some(next) if cum * 2.0 == total => (pos as f64 + next.0 as f64) / 2.0,
                        _ => pos as f64,
                    };
                    break;
                }
            }
            (id, 1.0 / (1.0 + median))
        })
        .collect()
}
