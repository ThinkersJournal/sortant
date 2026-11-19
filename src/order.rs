// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
//! The total order of fusion output (contract D5, amendment A2).

use crate::sort::stable_sort_by;
use std::cmp::Ordering;

/// Rewrites `-0.0` as `+0.0` (amendment A2); every other value, including NaN, is returned unchanged.
///
/// `f64::total_cmp` puts `-0.0` below `+0.0`, so without this two equal scores would not tie.
pub fn canonical_score(x: f64) -> f64 {
    if x == 0.0 { 0.0 } else { x }
}

/// Orders scores best (largest) first with `f64::total_cmp`. A total order; callers pass finite, canonical scores.
pub fn score_descending(a: f64, b: f64) -> Ordering {
    b.total_cmp(&a)
}

/// Sorts `(id, score)` pairs by score descending, then by `tie` (the designated tie-break), then by input order.
///
/// Scores must already be finite (contract D9) and canonical (`-0.0` rewritten, see [`canonical_score`]).
/// The id's `Hash` and `Ord` are never consulted (contract D3). `tie` may be any comparator, even an inconsistent one:
/// the sort is stable and never panics (see [`stable_sort_by`]); pass `|_, _| Ordering::Equal` for "no tie-break".
pub fn sort_scored<I, F>(items: Vec<(I, f64)>, mut tie: F) -> Vec<(I, f64)>
where
    F: FnMut(&I, &I) -> Ordering,
{
    stable_sort_by(items, |a, b| {
        score_descending(a.1, b.1).then_with(|| tie(&a.0, &b.0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_score_rewrites_only_negative_zero() {
        assert_eq!(canonical_score(-0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(canonical_score(0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(canonical_score(1.5).to_bits(), 1.5f64.to_bits());
        assert_eq!(canonical_score(-1.5).to_bits(), (-1.5f64).to_bits());
        assert_eq!(
            canonical_score(f64::MIN_POSITIVE / 2.0).to_bits(),
            (f64::MIN_POSITIVE / 2.0).to_bits()
        );
        assert!(canonical_score(f64::NAN).is_nan());
        assert_eq!(canonical_score(f64::NEG_INFINITY), f64::NEG_INFINITY);
    }

    #[test]
    fn score_descending_is_total_and_largest_first() {
        assert_eq!(score_descending(2.0, 1.0), Ordering::Less);
        assert_eq!(score_descending(1.0, 2.0), Ordering::Greater);
        assert_eq!(score_descending(1.0, 1.0), Ordering::Equal);
        assert_eq!(score_descending(-1.0, -2.0), Ordering::Less);
    }

    #[test]
    fn order_is_score_then_tie_break_then_input_order() {
        let items = vec![("a", 1.0), ("b", 2.0), ("c", 1.0), ("d", 1.0), ("e", 2.0)];
        // No tie-break: equal scores keep input order.
        let none = sort_scored(items.clone(), |_, _| Ordering::Equal);
        assert_eq!(
            none.iter().map(|x| x.0).collect::<Vec<_>>(),
            ["b", "e", "a", "c", "d"]
        );
        // Tie-break prefers later letters; "a","c","d" (and "b","e") are reordered only within equal scores.
        let rank = |s: &&str| match *s {
            "d" => 0,
            "c" => 1,
            _ => 2,
        };
        let tied = sort_scored(items, |x, y| rank(x).cmp(&rank(y)));
        assert_eq!(
            tied.iter().map(|x| x.0).collect::<Vec<_>>(),
            ["b", "e", "d", "c", "a"]
        );
    }

    #[test]
    fn id_hash_and_ord_never_decide() {
        // Ids with no Ord at all: sorting needs only the comparator we pass.
        #[derive(Clone, PartialEq, Eq, Hash, Debug)]
        struct Id(u8);
        let out = sort_scored(vec![(Id(9), 1.0), (Id(1), 1.0)], |_, _| Ordering::Equal);
        assert_eq!(out, vec![(Id(9), 1.0), (Id(1), 1.0)]);
    }

    #[test]
    fn negative_zero_ties_after_canonicalising() {
        let items = vec![("a", canonical_score(-0.0)), ("b", canonical_score(0.0))];
        let out = sort_scored(items, |x, y| y.cmp(x)); // tie-break prefers "b"
        assert_eq!(out[0].0, "b");
        assert_eq!(out[0].1.to_bits(), 0.0f64.to_bits());
        assert_eq!(out[1].1.to_bits(), 0.0f64.to_bits());
        // Without canonicalising, total_cmp would NOT tie them: the reason A2 exists.
        assert_eq!(score_descending(0.0, -0.0), Ordering::Less);
    }
}
