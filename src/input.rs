// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
//! Input validation, done once before any method runs (contract D9, D10, A2).

use crate::error::FuseError;
use crate::order::canonical_score;
use std::collections::HashSet;
use std::hash::Hash;

/// Checks one scored list and returns it with every `-0.0` rewritten as `+0.0` (amendment A2).
///
/// `list` is the list's position among the inputs and only labels errors. Fails with
/// [`FuseError::NonFiniteScore`] for the first NaN or infinite score (contract D9), and with
/// [`FuseError::DuplicateId`] for the first id that repeats (contract D10). Scores are checked before ids, item by
/// item, so the error is the first problem in list order. An empty list is valid.
pub fn check_scored<I: Clone + Eq + Hash>(
    list: usize,
    items: &[(I, f64)],
) -> Result<Vec<(I, f64)>, FuseError> {
    let mut seen: HashSet<&I> = HashSet::with_capacity(items.len());
    let mut out = Vec::with_capacity(items.len());
    for (index, (id, score)) in items.iter().enumerate() {
        if !score.is_finite() {
            return Err(FuseError::NonFiniteScore { list, index });
        }
        if !seen.insert(id) {
            return Err(FuseError::DuplicateId { list, index });
        }
        out.push((id.clone(), canonical_score(*score)));
    }
    Ok(out)
}

/// Checks one ranked list (best first, no scores): fails with [`FuseError::DuplicateId`] for the first repeated id.
///
/// `list` labels the error. An empty list is valid.
pub fn check_ranked<I: Eq + Hash>(list: usize, items: &[I]) -> Result<(), FuseError> {
    let mut seen: HashSet<&I> = HashSet::with_capacity(items.len());
    for (index, id) in items.iter().enumerate() {
        if !seen.insert(id) {
            return Err(FuseError::DuplicateId { list, index });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scored_ok_keeps_order_and_values() {
        let got = check_scored(0, &[("a", 1.5), ("b", -2.0), ("c", 0.0)]).unwrap();
        assert_eq!(got, vec![("a", 1.5), ("b", -2.0), ("c", 0.0)]);
        assert_eq!(check_scored::<&str>(0, &[]).unwrap(), vec![]);
    }

    #[test]
    fn scored_rewrites_negative_zero_by_bit_pattern() {
        let got = check_scored(0, &[("a", -0.0), ("b", 0.0)]).unwrap();
        assert_eq!(got[0].1.to_bits(), 0.0f64.to_bits());
        assert_eq!(got[1].1.to_bits(), 0.0f64.to_bits());
    }

    #[test]
    fn scored_rejects_each_non_finite_value_with_its_location() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let e = check_scored(3, &[("a", 1.0), ("b", bad), ("c", 2.0)]).unwrap_err();
            assert_eq!(e, FuseError::NonFiniteScore { list: 3, index: 1 });
        }
    }

    #[test]
    fn scored_rejects_duplicate_id_at_the_second_occurrence() {
        let e = check_scored(2, &[("a", 1.0), ("b", 2.0), ("a", 3.0)]).unwrap_err();
        assert_eq!(e, FuseError::DuplicateId { list: 2, index: 2 });
    }

    #[test]
    fn scored_reports_the_first_problem_in_list_order() {
        let e = check_scored(0, &[("a", 1.0), ("a", 2.0), ("b", f64::NAN)]).unwrap_err();
        assert_eq!(e, FuseError::DuplicateId { list: 0, index: 1 });
        let e = check_scored(0, &[("a", f64::NAN), ("a", 2.0)]).unwrap_err();
        assert_eq!(e, FuseError::NonFiniteScore { list: 0, index: 0 });
        // A duplicate id whose second score is non-finite is reported as non-finite (scores are checked first).
        let e = check_scored(0, &[("a", 1.0), ("a", f64::NAN)]).unwrap_err();
        assert_eq!(e, FuseError::NonFiniteScore { list: 0, index: 1 });
    }

    #[test]
    fn ranked_accepts_distinct_and_empty() {
        assert_eq!(check_ranked(0, &["a", "b", "c"]), Ok(()));
        assert_eq!(check_ranked::<&str>(0, &[]), Ok(()));
        assert_eq!(check_ranked(0, &["a"]), Ok(()));
    }

    #[test]
    fn ranked_rejects_duplicate_at_the_second_occurrence() {
        assert_eq!(
            check_ranked(5, &["a", "b", "b", "a"]),
            Err(FuseError::DuplicateId { list: 5, index: 2 })
        );
    }
}
