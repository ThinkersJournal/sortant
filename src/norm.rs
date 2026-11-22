// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
//! Score normalisation (contract D16, D17, amendment A3). Currently: min-max.

use crate::error::FuseError;

/// Min-max normalises one list of finite scores to `[0, 1]` as `(s - min) / (max - min)`.
///
/// - **Constant list** (`max == min`, exact equality, never an epsilon) and the one-item list: every score becomes
///   `0.5`, the neutral midpoint, because such a list carries no ordering information (contract D16).
/// - **Near-constant list** (range positive but tiny, even subnormal): normalised normally, so a numerically negligible
///   spread is stretched to `[0, 1]` and tiny differences decide the result. The formula is a division, never
///   `s * (1 / range)`, because `1 / range` can be infinite and `0 * inf` is NaN (amendment A3).
/// - **Empty list**: returns an empty vector.
/// - If `max - min` overflows to infinity, fails with [`FuseError::RangeOverflow`] (`list` labels it).
///
/// Scores must be finite (validate with [`check_scored`](crate::input::check_scored) first); only `+ - /` and
/// comparisons are used, so the result is bit-identical on every platform (contract D14).
pub fn min_max(list: usize, scores: &[f64]) -> Result<Vec<f64>, FuseError> {
    let Some(&first) = scores.first() else {
        return Ok(Vec::new());
    };
    let (mut min, mut max) = (first, first);
    for &s in &scores[1..] {
        if s < min {
            min = s;
        }
        if s > max {
            max = s;
        }
    }
    if max == min {
        return Ok(vec![0.5; scores.len()]);
    }
    let range = max - min;
    if !range.is_finite() {
        return Err(FuseError::RangeOverflow { list });
    }
    Ok(scores.iter().map(|&s| (s - min) / range).collect())
}

/// Below this population standard deviation a list counts as constant for [`dbsf`] (contract D16; the same absolute
/// threshold rankops uses).
pub const STD_EPSILON: f64 = 1e-9;

/// Distribution-based score fusion normalisation of one list of finite scores: `z = (s - mean) / std` with the
/// population standard deviation, `z` clamped to `[-3, 3]`, result `z / 6 + 0.5`, so `[mean - 3 std, mean + 3 std]`
/// maps to `[0, 1]`.
///
/// - **Constant list** (`std <= STD_EPSILON`, which includes one item): every score becomes `0.5` (contract D16).
/// - **Empty list**: returns an empty vector.
/// - If the sum, mean or variance is not finite (values near `f64::MAX`), fails with [`FuseError::RangeOverflow`].
///
/// Uses `+ - * /` and `sqrt` (correctly rounded by IEEE-754), accumulating in list order, so the result is
/// bit-identical on every platform (contract D14).
pub fn dbsf(list: usize, scores: &[f64]) -> Result<Vec<f64>, FuseError> {
    if scores.is_empty() {
        return Ok(Vec::new());
    }
    let n = scores.len() as f64;
    let mean = scores.iter().sum::<f64>() / n;
    let mut sq = 0.0;
    for &s in scores {
        let d = s - mean;
        sq += d * d;
    }
    let std = (sq / n).sqrt();
    if !(mean.is_finite() && std.is_finite()) {
        return Err(FuseError::RangeOverflow { list });
    }
    if std <= STD_EPSILON {
        return Ok(vec![0.5; scores.len()]);
    }
    Ok(scores
        .iter()
        .map(|&s| ((s - mean) / std).clamp(-3.0, 3.0) / 6.0 + 0.5)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_values() {
        assert_eq!(min_max(0, &[1.0, 2.0, 3.0]).unwrap(), vec![0.0, 0.5, 1.0]);
        assert_eq!(min_max(0, &[10.0, 0.0, 5.0]).unwrap(), vec![1.0, 0.0, 0.5]);
        assert_eq!(min_max(0, &[-3.0, -1.0]).unwrap(), vec![0.0, 1.0]);
        assert_eq!(min_max(0, &[0.0, 1.0, 4.0]).unwrap(), vec![0.0, 0.25, 1.0]);
    }

    #[test]
    fn empty_single_and_constant_lists_are_neutral() {
        assert_eq!(min_max(0, &[]).unwrap(), Vec::<f64>::new());
        assert_eq!(min_max(0, &[7.0]).unwrap(), vec![0.5]);
        assert_eq!(min_max(0, &[2.0, 2.0, 2.0]).unwrap(), vec![0.5; 3]);
        assert_eq!(min_max(0, &[0.0, 0.0]).unwrap(), vec![0.5; 2]);
        assert_eq!(min_max(0, &[f64::MAX, f64::MAX]).unwrap(), vec![0.5; 2]);
    }

    #[test]
    fn subnormal_range_gives_zero_and_one_without_nan() {
        let out = min_max(0, &[0.0, 5e-324]).unwrap();
        assert_eq!(out, vec![0.0, 1.0]);
        assert!(out.iter().all(|x| !x.is_nan()));
        let out = min_max(0, &[5e-324, 0.0, 1e-323]).unwrap();
        assert_eq!(out, vec![0.5, 0.0, 1.0]);
    }

    #[test]
    fn one_ulp_from_constant_is_not_constant() {
        let next = f64::from_bits(1.0f64.to_bits() + 1);
        assert_eq!(min_max(0, &[1.0, next]).unwrap(), vec![0.0, 1.0]);
    }

    #[test]
    fn overflowing_range_is_a_typed_error() {
        assert_eq!(
            min_max(4, &[-f64::MAX, f64::MAX]),
            Err(FuseError::RangeOverflow { list: 4 })
        );
        assert_eq!(
            min_max(0, &[f64::MAX, 0.0, -f64::MAX]),
            Err(FuseError::RangeOverflow { list: 0 })
        );
        // Full-width but non-overflowing range is fine.
        assert_eq!(min_max(0, &[0.0, f64::MAX]).unwrap(), vec![0.0, 1.0]);
    }

    #[test]
    fn output_is_inside_unit_interval_and_hits_both_ends() {
        let scores = [0.1, 0.7, 0.3, 1e-9, 123456.789, -5.5];
        let out = min_max(0, &scores).unwrap();
        assert!(out.iter().all(|x| (0.0..=1.0).contains(x)));
        assert!(out.contains(&0.0) && out.contains(&1.0));
        // Order-preserving.
        for i in 0..scores.len() {
            for j in 0..scores.len() {
                assert_eq!(scores[i] < scores[j], out[i] < out[j]);
            }
        }
    }
}
