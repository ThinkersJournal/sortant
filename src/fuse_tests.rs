// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
//! Tests for `fuse`: exact values per method, every error, veto/force, ordering, and a no-panic property sweep.

use crate::error::{FuseError, WeightProblem};
use crate::fuse::{Options, RankMethod, Ranked, ScoreMethod, Scored, fuse};
use proptest::prelude::*;
use std::cmp::Ordering;

fn ids<I: Clone>(f: &crate::fuse::Fused<I>) -> Vec<I> {
    f.items.iter().map(|x| x.0.clone()).collect()
}

fn r(v: &[&'static str]) -> Ranked<&'static str> {
    Ranked::new(v.to_vec())
}

fn s(v: &[(&'static str, f64)]) -> Scored<&'static str> {
    Scored::new(v.to_vec())
}

const RRF: RankMethod = RankMethod::Rrf { k: 60.0 };

// ---- exact values -------------------------------------------------------------------------------------------------

#[test]
fn rrf_exact_values() {
    let out = fuse(&[r(&["x", "y"]), r(&["y", "z"])], &RRF, &Options::new()).unwrap();
    assert_eq!(
        out.items,
        vec![
            ("y", 1.0 / 62.0 + 1.0 / 61.0),
            ("x", 1.0 / 61.0),
            ("z", 1.0 / 62.0)
        ]
    );
    assert!(out.conflicts.is_empty());
}

#[test]
fn rrf_weighted_exact_values() {
    let o = Options::new().weights(vec![2.0, 0.5]);
    let out = fuse(&[r(&["x", "y"]), r(&["y", "z"])], &RRF, &o).unwrap();
    assert_eq!(
        out.items,
        vec![
            ("y", 2.0 / 62.0 + 0.5 / 61.0),
            ("x", 2.0 / 61.0),
            ("z", 0.5 / 62.0)
        ]
    );
}

#[test]
fn rrf_other_k() {
    let out = fuse(
        &[r(&["x", "y"])],
        &RankMethod::Rrf { k: 1.0 },
        &Options::new(),
    )
    .unwrap();
    assert_eq!(out.items, vec![("x", 0.5), ("y", 1.0 / 3.0)]);
}

#[test]
fn isr_exact_values() {
    let out = fuse(
        &[r(&["x", "y"]), r(&["y", "z"])],
        &RankMethod::Isr { k: 1.0 },
        &Options::new(),
    )
    .unwrap();
    // pos 0 -> 1/sqrt(1), pos 1 -> 1/sqrt(2)
    let h = 1.0 / 2.0_f64.sqrt();
    assert_eq!(out.items, vec![("y", 1.0 + h), ("x", 1.0), ("z", h)]);
}

#[test]
fn isr_weighted_exact_values() {
    let o = Options::new().weights(vec![4.0, 1.0]);
    let out = fuse(
        &[r(&["x"]), r(&["x", "y"])],
        &RankMethod::Isr { k: 3.0 },
        &o,
    )
    .unwrap();
    // x: 4/sqrt(3) + 1/sqrt(3); y: 1/sqrt(4)
    assert_eq!(
        out.items,
        vec![
            ("x", 4.0 / 3.0_f64.sqrt() + 1.0 / 3.0_f64.sqrt()),
            ("y", 0.5)
        ]
    );
}

#[test]
fn bad_isr_k_is_invalid_config() {
    for k in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let e = fuse(&[r(&["x"])], &RankMethod::Isr { k }, &Options::new()).unwrap_err();
        assert!(
            matches!(e, FuseError::InvalidConfig { parameter: "k", .. }),
            "{e:?}"
        );
    }
}

#[test]
fn borda_exact_values() {
    let out = fuse(
        &[r(&["x", "y", "z"]), r(&["z", "x"])],
        &RankMethod::Borda,
        &Options::new(),
    )
    .unwrap();
    assert_eq!(out.items, vec![("x", 4.0), ("z", 3.0), ("y", 2.0)]);
}

#[test]
fn borda_weighted() {
    let o = Options::new().weights(vec![1.0, 3.0]);
    let out = fuse(&[r(&["x", "y"]), r(&["y"])], &RankMethod::Borda, &o).unwrap();
    // x: 2*1; y: 1*1 + 3*1
    assert_eq!(out.items, vec![("y", 4.0), ("x", 2.0)]);
}

fn two_scored() -> [Scored<&'static str>; 2] {
    [s(&[("x", 10.0), ("y", 0.0)]), s(&[("y", 5.0), ("z", 1.0)])]
}

#[test]
fn dbsf_exact_values() {
    // list 1: mean 2, population std 1 -> z = +1, -1. list 2: both 9 -> constant -> 0.5 each.
    let out = fuse(
        &[s(&[("x", 3.0), ("y", 1.0)]), s(&[("y", 9.0), ("z", 9.0)])],
        &ScoreMethod::Dbsf,
        &Options::new(),
    )
    .unwrap();
    let hi = 1.0 / 6.0 + 0.5;
    let lo = -1.0 / 6.0 + 0.5;
    assert_eq!(out.items, vec![("y", lo + 0.5), ("x", hi), ("z", 0.5)]);
}

#[test]
fn dbsf_clamps_outliers_to_three_sigma() {
    let mut v: Vec<(String, f64)> = (0..10).map(|i| (format!("a{i}"), 0.0)).collect();
    v.push(("big".to_string(), 100.0));
    // the outlier's z is sqrt(10) > 3, so it clamps to exactly 3 / 6 + 0.5 = 1
    let out = fuse(&[Scored::new(v)], &ScoreMethod::Dbsf, &Options::new()).unwrap();
    assert_eq!(out.items[0], ("big".to_string(), 1.0));
}

#[test]
fn dbsf_overflow_is_range_overflow() {
    let e = fuse(
        &[s(&[("x", f64::MAX), ("y", f64::MAX)])],
        &ScoreMethod::Dbsf,
        &Options::new(),
    )
    .unwrap_err();
    assert_eq!(e, FuseError::RangeOverflow { list: 0 });
}

#[test]
fn dbsf_variance_overflow_with_finite_mean_is_range_overflow() {
    // mean 0 is finite, but the squared deviations overflow, so the std is infinite
    let e = fuse(
        &[s(&[("x", f64::MAX), ("y", -f64::MAX)])],
        &ScoreMethod::Dbsf,
        &Options::new(),
    )
    .unwrap_err();
    assert_eq!(e, FuseError::RangeOverflow { list: 0 });
}

const STD3: ScoreMethod = ScoreMethod::Standardized {
    clip_min: -3.0,
    clip_max: 3.0,
};

#[test]
fn standardized_exact_values() {
    // list 1: mean 2, std 1 -> z = +1, -1. list 2: constant -> 0 each.
    let out = fuse(
        &[s(&[("x", 3.0), ("y", 1.0)]), s(&[("y", 9.0), ("z", 9.0)])],
        &STD3,
        &Options::new(),
    )
    .unwrap();
    assert_eq!(out.items, vec![("x", 1.0), ("z", 0.0), ("y", -1.0)]);
}

#[test]
fn standardized_clips_to_the_given_range() {
    let m = ScoreMethod::Standardized {
        clip_min: -0.5,
        clip_max: 0.5,
    };
    let out = fuse(&[s(&[("x", 3.0), ("y", 1.0)])], &m, &Options::new()).unwrap();
    assert_eq!(out.items, vec![("x", 0.5), ("y", -0.5)]);
}

#[test]
fn bad_standardized_clip_is_invalid_config_not_a_panic() {
    let cases = [
        (f64::NAN, 1.0, "clip_min"),
        (f64::NEG_INFINITY, 1.0, "clip_min"),
        (0.0, f64::NAN, "clip_max"),
        (0.0, f64::INFINITY, "clip_max"),
        (2.0, 1.0, "clip_min"),
    ];
    for (clip_min, clip_max, want) in cases {
        let m = ScoreMethod::Standardized { clip_min, clip_max };
        let e = fuse(&[s(&[("x", 1.0), ("y", 2.0)])], &m, &Options::new()).unwrap_err();
        match e {
            FuseError::InvalidConfig { parameter, .. } => assert_eq!(parameter, want),
            other => panic!("unexpected {other:?}"),
        }
    }
}

#[test]
fn standardized_equal_clip_bounds_are_allowed() {
    let m = ScoreMethod::Standardized {
        clip_min: 1.0,
        clip_max: 1.0,
    };
    let out = fuse(&[s(&[("x", 3.0), ("y", 1.0)])], &m, &Options::new()).unwrap();
    assert_eq!(out.items, vec![("x", 1.0), ("y", 1.0)]);
}

#[test]
fn additive_multi_task_is_weighted_combsum_or_standardized_for_any_list_count() {
    // ResFlow-style weights (1, 20, 0.5) over three lists; rankops ignores weights beyond two lists.
    let lists = [
        s(&[("x", 1.0), ("y", 0.0)]),
        s(&[("x", 0.0), ("y", 1.0)]),
        s(&[("x", 5.0), ("y", 0.0)]),
    ];
    let o = Options::new().weights(vec![1.0, 20.0, 0.5]);
    let out = fuse(&lists, &ScoreMethod::CombSum, &o).unwrap();
    assert_eq!(out.items, vec![("y", 20.0), ("x", 1.5)]);
    let out = fuse(&lists, &STD3, &o).unwrap();
    // each list has mean 0.5 (or 2.5) and std 0.5 (or 2.5): z = +-1
    assert_eq!(
        out.items,
        vec![("y", 20.0 - 1.0 - 0.5), ("x", 1.0 - 20.0 + 0.5)]
    );
}

fn three_raw() -> [Scored<&'static str>; 3] {
    [
        s(&[("x", 4.0), ("y", -2.0)]),
        s(&[("x", 1.0), ("y", 0.0), ("z", 0.0)]),
        s(&[("x", 7.0), ("y", 6.0)]),
    ]
}

#[test]
fn combmax_and_combmin_use_raw_scores() {
    let max = fuse(&three_raw(), &ScoreMethod::CombMax, &Options::new()).unwrap();
    assert_eq!(max.items, vec![("x", 7.0), ("y", 6.0), ("z", 0.0)]);
    let min = fuse(&three_raw(), &ScoreMethod::CombMin, &Options::new()).unwrap();
    assert_eq!(min.items, vec![("x", 1.0), ("z", 0.0), ("y", -2.0)]);
}

#[test]
fn combmax_and_combmin_apply_list_weights() {
    let o = Options::new().weights(vec![2.0, 1.0, 1.0]);
    let max = fuse(&three_raw(), &ScoreMethod::CombMax, &o).unwrap();
    assert_eq!(max.items, vec![("x", 8.0), ("y", 6.0), ("z", 0.0)]);
    let min = fuse(&three_raw(), &ScoreMethod::CombMin, &o).unwrap();
    assert_eq!(min.items, vec![("x", 1.0), ("z", 0.0), ("y", -4.0)]);
}

#[test]
fn combmed_odd_count_ties_and_even_midpoint() {
    let out = fuse(&three_raw(), &ScoreMethod::CombMed, &Options::new()).unwrap();
    // x: median of 4,1,7 = 4; y: median of -2,0,6 = 0; z: 0 (ties keep input order)
    assert_eq!(out.items, vec![("x", 4.0), ("y", 0.0), ("z", 0.0)]);
    let out = fuse(
        &[s(&[("x", 1.0)]), s(&[("x", 3.0)])],
        &ScoreMethod::CombMed,
        &Options::new(),
    )
    .unwrap();
    assert_eq!(out.items, vec![("x", 2.0)]);
}

#[test]
fn combmed_even_midpoint_never_overflows() {
    let both = |a: f64, b: f64| {
        fuse(
            &[s(&[("x", a)]), s(&[("x", b)])],
            &ScoreMethod::CombMed,
            &Options::new(),
        )
        .unwrap()
        .items[0]
            .1
    };
    assert_eq!(both(f64::MAX, f64::MAX), f64::MAX);
    assert_eq!(both(-f64::MAX, -f64::MAX), -f64::MAX);
    assert_eq!(both(f64::MAX, -f64::MAX), 0.0);
    assert_eq!(both(-1.0, 3.0), 1.0);
    assert_eq!(both(-3.0, 0.0), -1.5);
    assert_eq!(both(0.0, 3.0), 1.5);
    assert_eq!(both(2.0, 6.0), 4.0);
}

#[test]
fn combanz_averages_only_non_zero_scores() {
    let out = fuse(&three_raw(), &ScoreMethod::CombAnz, &Options::new()).unwrap();
    // x: (4+1+7)/3 = 4; y: non-zero -2 and 6 -> 2; z: only a zero -> 0
    assert_eq!(out.items, vec![("x", 4.0), ("y", 2.0), ("z", 0.0)]);
}

#[test]
fn rbc_exact_values() {
    let m = RankMethod::Rbc { persistence: 0.5 };
    let out = fuse(&[r(&["x", "y"]), r(&["y", "z"])], &m, &Options::new()).unwrap();
    // pos 0 -> 0.5, pos 1 -> 0.25
    assert_eq!(out.items, vec![("y", 0.75), ("x", 0.5), ("z", 0.25)]);
    let o = Options::new().weights(vec![2.0, 1.0]);
    let out = fuse(&[r(&["x", "y"]), r(&["y"])], &m, &o).unwrap();
    assert_eq!(
        out.items,
        vec![("x", 1.0), ("y", 0.5 + 0.5)]
            .into_iter()
            .collect::<Vec<_>>()
    );
}

#[test]
fn rbc_golden_bit_patterns_at_ranks_0_1_100_1000() {
    // Produced by the documented recurrence in IEEE-754 double arithmetic outside Rust (Python floats):
    // w = 1; at each rank c = w * (1 - 0.8); w = w * 0.8.
    let m = RankMethod::Rbc { persistence: 0.8 };
    let list = Ranked::new((0..1001_u32).collect::<Vec<_>>());
    let out = fuse(&[list], &m, &Options::new()).unwrap();
    let score = |id: u32| out.items.iter().find(|x| x.0 == id).unwrap().1.to_bits();
    assert_eq!(score(0), 0x3fc9999999999998);
    assert_eq!(score(1), 0x3fc47ae147ae147a);
    assert_eq!(score(100), 0x3dc665bf1d3e6ab1);
    assert_eq!(score(1000), 0x2baae88381bb546b);
}

#[test]
fn bad_rbc_persistence_is_invalid_config() {
    for p in [
        0.0,
        1.0,
        -0.5,
        1.5,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        let e = fuse(
            &[r(&["x"])],
            &RankMethod::Rbc { persistence: p },
            &Options::new(),
        )
        .unwrap_err();
        assert!(
            matches!(
                e,
                FuseError::InvalidConfig {
                    parameter: "persistence",
                    ..
                }
            ),
            "{e:?}"
        );
    }
}

fn three_ranked() -> [Ranked<&'static str>; 3] {
    [r(&["x", "y", "z"]), r(&["y", "x"]), r(&["x", "z"])]
}

#[test]
fn condorcet_exact_values() {
    let out = fuse(&three_ranked(), &RankMethod::Condorcet, &Options::new()).unwrap();
    assert_eq!(out.items, vec![("x", 2.0), ("y", 1.0), ("z", 0.0)]);
}

#[test]
fn copeland_exact_values() {
    let out = fuse(&three_ranked(), &RankMethod::Copeland, &Options::new()).unwrap();
    assert_eq!(out.items, vec![("x", 2.0), ("y", 0.0), ("z", -2.0)]);
}

#[test]
fn median_rank_exact_values() {
    // penalty position = longest list (3) + 1 = 4
    let out = fuse(&three_ranked(), &RankMethod::MedianRank, &Options::new()).unwrap();
    assert_eq!(out.items, vec![("x", 1.0), ("y", 0.5), ("z", 1.0 / 3.0)]);
}

#[test]
fn median_rank_even_count_takes_the_midpoint() {
    let out = fuse(
        &[r(&["x"]), r(&["a", "b", "x"])],
        &RankMethod::MedianRank,
        &Options::new(),
    )
    .unwrap();
    // x at positions 0 and 2 -> median 1; a at 0 and the penalty 4 -> 2; b at 1 and 4 -> 2.5
    assert_eq!(
        out.items,
        vec![("x", 0.5), ("a", 1.0 / 3.0), ("b", 1.0 / 3.5)]
    );
}

#[test]
fn pairwise_methods_honour_weights() {
    fn c(
        lists: &[Ranked<&'static str>],
        m: &RankMethod,
        o: &Options<&'static str>,
    ) -> Vec<(&'static str, f64)> {
        fuse(lists, m, o).unwrap().items
    }
    let lists = [r(&["a", "b"]), r(&["b", "a"])];
    let unit = Options::new();
    let heavy = Options::new().weights(vec![3.0, 1.0]);
    assert_eq!(
        c(&lists, &RankMethod::Condorcet, &unit),
        vec![("a", 0.0), ("b", 0.0)]
    );
    assert_eq!(
        c(&lists, &RankMethod::Condorcet, &heavy),
        vec![("a", 1.0), ("b", 0.0)]
    );
    assert_eq!(
        c(&lists, &RankMethod::Copeland, &unit),
        vec![("a", 0.0), ("b", 0.0)]
    );
    assert_eq!(
        c(&lists, &RankMethod::Copeland, &heavy),
        vec![("a", 1.0), ("b", -1.0)]
    );
    // median: a is at 0 (weight 3) and 1 (weight 1) -> 0; b is at 0 (weight 1) and 1 (weight 3) -> 1
    assert_eq!(
        c(&lists, &RankMethod::MedianRank, &heavy),
        vec![("a", 1.0), ("b", 0.5)]
    );
}

#[test]
fn pairwise_methods_reject_duplicates_and_bad_weights() {
    for m in [
        RankMethod::Condorcet,
        RankMethod::Copeland,
        RankMethod::MedianRank,
    ] {
        let e = fuse(&[r(&["a", "a"])], &m, &Options::new()).unwrap_err();
        assert!(matches!(e, FuseError::DuplicateId { .. }), "{e:?}");
        let e = fuse(&[r(&["a"])], &m, &Options::new().weights(vec![1.0, 2.0])).unwrap_err();
        assert!(matches!(e, FuseError::LengthMismatch { .. }), "{e:?}");
    }
}

/// Regression carried over from rankops' `dbsf` ordering test (see PROVENANCE.md): documents in both lists must outrank
/// documents in one. DBSF maps each list by `(s - (mu - 3 sigma)) / (6 sigma)` and sums, so an absent item gets 0, and the
/// expected order is d2, d3, d1, d4. Summing raw clipped z-scores would put d3 below d4.
#[test]
fn dbsf_ranks_documents_in_both_lists_first() {
    let bm25 = s(&[("d1", 15.0), ("d2", 12.0), ("d3", 8.0)]);
    let dense = s(&[("d2", 0.9), ("d3", 0.7), ("d4", 0.5)]);
    let out = fuse(&[bm25, dense], &ScoreMethod::Dbsf, &Options::new()).unwrap();
    assert_eq!(ids(&out), vec!["d2", "d3", "d1", "d4"]);
}

#[test]
fn combsum_exact_values_and_input_order_ties() {
    let out = fuse(&two_scored(), &ScoreMethod::CombSum, &Options::new()).unwrap();
    // normalised: x=1, y=0 | y=1, z=0  => x=1, y=1, z=0; x and y tie, x appeared first
    assert_eq!(out.items, vec![("x", 1.0), ("y", 1.0), ("z", 0.0)]);
}

#[test]
fn combmnz_exact_values() {
    let out = fuse(&two_scored(), &ScoreMethod::CombMnz, &Options::new()).unwrap();
    assert_eq!(out.items, vec![("y", 2.0), ("x", 1.0), ("z", 0.0)]);
}

#[test]
fn weighted_exact_values() {
    let o = Options::new().weights(vec![3.0, 1.0]);
    let out = fuse(&two_scored(), &ScoreMethod::Weighted, &o).unwrap();
    assert_eq!(out.items, vec![("x", 0.75), ("y", 0.25), ("z", 0.0)]);
}

#[test]
fn weighted_without_weights_is_the_mean() {
    let out = fuse(&two_scored(), &ScoreMethod::Weighted, &Options::new()).unwrap();
    assert_eq!(out.items, vec![("x", 0.5), ("y", 0.5), ("z", 0.0)]);
}

#[test]
fn combsum_uses_weights_as_given() {
    let o = Options::new().weights(vec![3.0, 1.0]);
    let out = fuse(&two_scored(), &ScoreMethod::CombSum, &o).unwrap();
    assert_eq!(out.items, vec![("x", 3.0), ("y", 1.0), ("z", 0.0)]);
}

#[test]
fn constant_scored_list_contributes_one_half() {
    let out = fuse(
        &[s(&[("x", 4.0), ("y", 4.0)])],
        &ScoreMethod::CombSum,
        &Options::new(),
    )
    .unwrap();
    assert_eq!(out.items, vec![("x", 0.5), ("y", 0.5)]);
}

#[test]
fn zero_weight_contributes_nothing() {
    let o = Options::new().weights(vec![0.0, 1.0]);
    let out = fuse(&[r(&["x"]), r(&["y"])], &RRF, &o).unwrap();
    assert_eq!(out.items, vec![("y", 1.0 / 61.0), ("x", 0.0)]);
}

// ---- errors -------------------------------------------------------------------------------------------------------

#[test]
fn bad_rrf_k_is_invalid_config() {
    for k in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let e = fuse(&[r(&["x"])], &RankMethod::Rrf { k }, &Options::new()).unwrap_err();
        match e {
            FuseError::InvalidConfig { parameter, value } => {
                assert_eq!(parameter, "k");
                assert_eq!(value.to_bits(), k.to_bits());
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}

#[test]
fn non_finite_score_names_list_and_index() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let lists = [s(&[("a", 1.0)]), s(&[("b", 1.0), ("c", bad)])];
        let e = fuse(&lists, &ScoreMethod::CombSum, &Options::new()).unwrap_err();
        assert_eq!(e, FuseError::NonFiniteScore { list: 1, index: 1 });
    }
}

#[test]
fn duplicate_id_in_one_list_is_an_error_for_every_method() {
    let e = fuse(&[r(&["a"]), r(&["b", "c", "b"])], &RRF, &Options::new()).unwrap_err();
    assert_eq!(e, FuseError::DuplicateId { list: 1, index: 2 });
    let e = fuse(&[r(&["a", "a"])], &RankMethod::Borda, &Options::new()).unwrap_err();
    assert_eq!(e, FuseError::DuplicateId { list: 0, index: 1 });
    for m in [
        ScoreMethod::Weighted,
        ScoreMethod::CombSum,
        ScoreMethod::CombMnz,
    ] {
        let e = fuse(&[s(&[("a", 1.0), ("a", 2.0)])], &m, &Options::new()).unwrap_err();
        assert_eq!(e, FuseError::DuplicateId { list: 0, index: 1 });
    }
}

#[test]
fn same_id_in_different_lists_is_fine() {
    assert!(fuse(&[r(&["a"]), r(&["a"])], &RRF, &Options::new()).is_ok());
}

#[test]
fn weight_problems() {
    let lists = [r(&["a"]), r(&["b"])];
    let w = |v: Vec<f64>| fuse(&lists, &RRF, &Options::new().weights(v)).unwrap_err();
    assert_eq!(
        w(vec![1.0]),
        FuseError::LengthMismatch {
            expected: 2,
            got: 1
        }
    );
    assert_eq!(
        w(vec![1.0, 1.0, 1.0]),
        FuseError::LengthMismatch {
            expected: 2,
            got: 3
        }
    );
    assert_eq!(
        w(vec![1.0, f64::NAN]),
        FuseError::InvalidWeights(WeightProblem::NotFinite { index: 1 })
    );
    assert_eq!(
        w(vec![f64::INFINITY, 1.0]),
        FuseError::InvalidWeights(WeightProblem::NotFinite { index: 0 })
    );
    assert_eq!(
        w(vec![1.0, -0.5]),
        FuseError::InvalidWeights(WeightProblem::Negative { index: 1 })
    );
    assert_eq!(
        w(vec![0.0, 0.0]),
        FuseError::InvalidWeights(WeightProblem::ZeroSum)
    );
    assert!(matches!(
        w(vec![f64::MAX, f64::MAX]),
        FuseError::InvalidConfig { .. }
    ));
}

#[test]
fn negative_zero_weight_is_zero() {
    let o = Options::new().weights(vec![-0.0, 1.0]);
    assert!(fuse(&[r(&["a"]), r(&["b"])], &RRF, &o).is_ok());
}

#[test]
fn range_overflow_is_typed() {
    let e = fuse(
        &[s(&[("a", 1.0)]), s(&[("a", -f64::MAX), ("b", f64::MAX)])],
        &ScoreMethod::CombSum,
        &Options::new(),
    )
    .unwrap_err();
    assert_eq!(e, FuseError::RangeOverflow { list: 1 });
}

#[test]
fn validation_covers_vetoed_items() {
    let o = Options::new().veto(["a"]);
    let e = fuse(&[s(&[("a", f64::NAN)])], &ScoreMethod::CombSum, &o).unwrap_err();
    assert_eq!(e, FuseError::NonFiniteScore { list: 0, index: 0 });
}

// ---- empty cases, top_k ------------------------------------------------------------------------------------------

#[test]
fn no_lists_and_empty_lists_give_empty_ok() {
    let none: [Ranked<&str>; 0] = [];
    assert_eq!(fuse(&none, &RRF, &Options::new()).unwrap().items, vec![]);
    assert_eq!(
        fuse(&[r(&[]), r(&[])], &RRF, &Options::new())
            .unwrap()
            .items,
        vec![]
    );
    let none: [Scored<&str>; 0] = [];
    for m in [
        ScoreMethod::Weighted,
        ScoreMethod::CombSum,
        ScoreMethod::CombMnz,
    ] {
        assert_eq!(fuse(&none, &m, &Options::new()).unwrap().items, vec![]);
        assert_eq!(
            fuse(&[s(&[]), s(&[])], &m, &Options::new()).unwrap().items,
            vec![]
        );
    }
}

#[test]
fn no_lists_with_weights_is_length_mismatch_not_panic() {
    let none: [Ranked<&str>; 0] = [];
    let e = fuse(&none, &RRF, &Options::new().weights(vec![1.0])).unwrap_err();
    assert_eq!(
        e,
        FuseError::LengthMismatch {
            expected: 0,
            got: 1
        }
    );
    // no lists, zero weights: nothing to sum, ZeroSum is reported
    let e = fuse(&none, &RRF, &Options::new().weights(vec![])).unwrap_err();
    assert_eq!(e, FuseError::InvalidWeights(WeightProblem::ZeroSum));
}

#[test]
fn single_item_list() {
    let out = fuse(&[r(&["a"])], &RankMethod::Borda, &Options::new()).unwrap();
    assert_eq!(out.items, vec![("a", 1.0)]);
    let out = fuse(&[s(&[("a", 9.0)])], &ScoreMethod::CombMnz, &Options::new()).unwrap();
    assert_eq!(out.items, vec![("a", 0.5)]);
}

#[test]
fn top_k_zero_one_and_beyond() {
    let lists = [r(&["x", "y"]), r(&["y", "z"])];
    assert_eq!(
        fuse(&lists, &RRF, &Options::new().top_k(0)).unwrap().items,
        vec![]
    );
    assert_eq!(
        ids(&fuse(&lists, &RRF, &Options::new().top_k(1)).unwrap()),
        ["y"]
    );
    assert_eq!(
        ids(&fuse(&lists, &RRF, &Options::new().top_k(99)).unwrap()),
        ["y", "x", "z"]
    );
}

#[test]
fn top_k_zero_still_validates() {
    let e = fuse(&[r(&["a", "a"])], &RRF, &Options::new().top_k(0)).unwrap_err();
    assert_eq!(e, FuseError::DuplicateId { list: 0, index: 1 });
}

// ---- ordering ------------------------------------------------------------------------------------------------------

#[test]
fn ties_keep_first_appearance_order() {
    let out = fuse(&[r(&["b"]), r(&["a"])], &RRF, &Options::new()).unwrap();
    assert_eq!(ids(&out), ["b", "a"]);
}

#[test]
fn tie_break_ranker_orders_ties_and_absent_last() {
    let lists = [r(&["a"]), r(&["b"]), r(&["c"])];
    let o = Options::new().tie_break_ranker(&r(&["c", "a"]));
    assert_eq!(ids(&fuse(&lists, &RRF, &o).unwrap()), ["c", "a", "b"]);
}

#[test]
fn tie_break_never_overrides_score() {
    let lists = [r(&["a", "b"])];
    let o = Options::new().tie_break_ranker(&r(&["b", "a"]));
    assert_eq!(ids(&fuse(&lists, &RRF, &o).unwrap()), ["a", "b"]);
}

#[test]
fn tie_break_comparator() {
    let lists = [r(&["a"]), r(&["b"]), r(&["c"])];
    let o = Options::new().tie_break_by(|x: &&str, y: &&str| y.cmp(x));
    assert_eq!(ids(&fuse(&lists, &RRF, &o).unwrap()), ["c", "b", "a"]);
}

#[test]
fn inconsistent_tie_break_does_not_panic_and_is_deterministic() {
    let lists: Vec<Ranked<u32>> = (0..40).map(|i| Ranked::new(vec![i])).collect();
    // rock-paper-scissors comparator: intransitive
    let mk = || {
        Options::new().tie_break_by(|a: &u32, b: &u32| match (a % 3, b % 3) {
            (x, y) if x == y => Ordering::Equal,
            (0, 1) | (1, 2) | (2, 0) => Ordering::Less,
            _ => Ordering::Greater,
        })
    };
    let first = fuse(&lists, &RRF, &mk()).unwrap();
    let again = fuse(&lists, &RRF, &mk()).unwrap();
    assert_eq!(first, again);
    let mut got = ids(&first);
    got.sort_unstable();
    assert_eq!(got, (0..40).collect::<Vec<_>>());
}

#[test]
fn negative_zero_inputs_tie_and_output_is_positive_zero() {
    // After min-max the scores are in [0,1]; -0.0 inputs are rewritten (A2) so equal lists tie.
    let lists = [s(&[("a", -0.0), ("b", 1.0)]), s(&[("a", 1.0), ("b", 0.0)])];
    let out = fuse(
        &lists,
        &ScoreMethod::CombSum,
        &Options::new().tie_break_by(|x: &&str, y: &&str| y.cmp(x)),
    )
    .unwrap();
    assert_eq!(ids(&out), ["b", "a"]);
    for (_, score) in &out.items {
        assert_eq!(score.to_bits(), 1.0f64.to_bits());
    }
    let zero = fuse(
        &[s(&[("a", -0.0), ("b", 0.0)])],
        &ScoreMethod::CombSum,
        &Options::new(),
    )
    .unwrap();
    // constant list -> 0.5 each
    assert_eq!(zero.items, vec![("a", 0.5), ("b", 0.5)]);
    let lists = [s(&[("a", -0.0), ("b", 1.0)]), s(&[("a", 0.0), ("b", 1.0)])];
    let out = fuse(&lists, &ScoreMethod::CombMnz, &Options::new()).unwrap();
    assert_eq!(out.items, vec![("b", 4.0), ("a", 0.0)]); // (1 + 1) * 2 lists
    assert_eq!(out.items[1].1.to_bits(), 0.0f64.to_bits());
}

// ---- veto, force ---------------------------------------------------------------------------------------------------

#[test]
fn veto_removes_items() {
    let o = Options::new().veto(["y"]);
    let out = fuse(&[r(&["x", "y"]), r(&["y", "z"])], &RRF, &o).unwrap();
    assert_eq!(ids(&out), ["x", "z"]);
}

#[test]
fn force_goes_first_ordered_by_the_usual_order() {
    let o = Options::new().force(["z", "x"]);
    let out = fuse(&[r(&["x", "y"]), r(&["y", "z"])], &RRF, &o).unwrap();
    assert_eq!(ids(&out), ["x", "z", "y"]);
    assert_eq!(out.items[0].1, 1.0 / 61.0); // real fused score reported
}

#[test]
fn forced_items_consume_top_k() {
    let o = Options::new().force(["z"]).top_k(2);
    let out = fuse(&[r(&["x", "y"]), r(&["y", "z"])], &RRF, &o).unwrap();
    assert_eq!(ids(&out), ["z", "y"]);
    let o = Options::new().force(["z", "x"]).top_k(1);
    assert_eq!(
        ids(&fuse(&[r(&["x", "y"]), r(&["y", "z"])], &RRF, &o).unwrap()),
        ["x"]
    );
}

#[test]
fn force_of_unknown_id_adds_nothing() {
    let o = Options::new().force(["nope"]);
    let out = fuse(&[r(&["x"])], &RRF, &o).unwrap();
    assert_eq!(ids(&out), ["x"]);
}

#[test]
fn veto_beats_force_and_conflict_is_reported_in_first_appearance_order() {
    let o = Options::new()
        .force(["q", "z", "x", "unseen"])
        .veto(["x", "z", "unseen"]);
    let out = fuse(&[r(&["x", "y"]), r(&["y", "z", "q"])], &RRF, &o).unwrap();
    assert_eq!(ids(&out), ["q", "y"]);
    assert_eq!(out.conflicts, ["x", "z"]);
}

// ---- determinism and the type-level rules -------------------------------------------------------------------------

#[test]
fn output_does_not_depend_on_id_order_or_hash() {
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    struct Id(u8);
    let out = fuse(
        &[Ranked::new(vec![Id(9)]), Ranked::new(vec![Id(1)])],
        &RRF,
        &Options::new(),
    )
    .unwrap();
    assert_eq!(
        out.items.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
        [Id(9), Id(1)]
    );
}

#[test]
fn scored_into_ranked_sorts_by_score_then_input_order() {
    let ranked = s(&[("a", 1.0), ("b", 3.0), ("c", 1.0), ("d", 2.0)])
        .into_ranked()
        .unwrap();
    assert_eq!(ranked.as_slice(), ["b", "d", "a", "c"]);
    assert_eq!(ranked.len(), 4);
    assert!(!ranked.is_empty());
    assert_eq!(
        s(&[("a", f64::NAN)]).into_ranked().unwrap_err(),
        FuseError::NonFiniteScore { list: 0, index: 0 }
    );
    assert_eq!(
        s(&[("a", 1.0), ("a", 1.0)]).into_ranked().unwrap_err(),
        FuseError::DuplicateId { list: 0, index: 1 }
    );
}

#[test]
fn accessors() {
    let sc = s(&[("a", 1.0)]);
    assert_eq!(sc.as_slice(), [("a", 1.0)]);
    assert_eq!(sc.len(), 1);
    assert_eq!(s(&[("a", 1.0), ("b", 2.0), ("c", 3.0)]).len(), 3);
    assert_eq!(Scored::<&str>::new(vec![]).len(), 0);
    assert!(!sc.is_empty());
    assert!(Scored::<&str>::new(vec![]).is_empty());
    assert!(Ranked::<&str>::new(vec![]).is_empty());
}

// ---- golden property cases + sweep --------------------------------------------------------------------------------

#[test]
fn golden_cases_hold_the_contract_invariants() {
    // Fixed cases kept beside the generated ones: output ids are exactly the union minus veto, no duplicates.
    let golden: Vec<Vec<Vec<u8>>> = vec![
        vec![],
        vec![vec![]],
        vec![vec![1, 2, 3], vec![3, 2, 1]],
        vec![vec![1], vec![1], vec![1]],
        vec![vec![5, 4, 3, 2, 1], vec![], vec![9]],
    ];
    for g in golden {
        let lists: Vec<Ranked<u8>> = g.iter().cloned().map(Ranked::new).collect();
        for m in [RRF, RankMethod::Borda, RankMethod::Isr { k: 1.0 }] {
            let out = fuse(&lists, &m, &Options::new()).unwrap();
            let mut got = ids(&out);
            got.sort_unstable();
            let mut want: Vec<u8> = g.concat();
            want.sort_unstable();
            want.dedup();
            assert_eq!(got, want);
        }
    }
}

fn arb_ranked() -> impl Strategy<Value = Vec<Vec<u8>>> {
    prop::collection::vec(
        prop::collection::vec(0u8..12, 0..10).prop_map(|mut v| {
            let mut seen = std::collections::HashSet::new();
            v.retain(|x| seen.insert(*x));
            v
        }),
        0..5,
    )
}

proptest! {
    #[test]
    fn rank_fusion_is_total_sorted_and_a_union(g in arb_ranked(), top in prop::option::of(0usize..15)) {
        let lists: Vec<Ranked<u8>> = g.iter().cloned().map(Ranked::new).collect();
        for m in [RRF, RankMethod::Borda, RankMethod::Isr { k: 1.0 }] {
            let base = fuse(&lists, &m, &Options::new()).unwrap();
            let mut got = ids(&base);
            got.sort_unstable();
            let mut want: Vec<u8> = g.concat();
            want.sort_unstable();
            want.dedup();
            prop_assert_eq!(got, want);
            for w in base.items.windows(2) {
                prop_assert!(w[0].1 >= w[1].1);
            }
            let mut o = Options::new();
            if let Some(t) = top { o = o.top_k(t); }
            let cut = fuse(&lists, &m, &o).unwrap();
            let n = top.map_or(base.items.len(), |t| t.min(base.items.len()));
            prop_assert_eq!(&cut.items[..], &base.items[..n]);
        }
    }

    #[test]
    fn score_fusion_never_panics_and_never_emits_nan(
        raw in prop::collection::vec(prop::collection::vec((0u8..8, any::<f64>()), 0..8), 0..4)
    ) {
        let lists: Vec<Scored<u8>> = raw.into_iter().map(Scored::new).collect();
        for m in [ScoreMethod::Weighted, ScoreMethod::CombSum, ScoreMethod::CombMnz] {
            if let Ok(out) = fuse(&lists, &m, &Options::new()) {
                for (_, sc) in &out.items {
                    prop_assert!(!sc.is_nan());
                    prop_assert!(sc.to_bits() != (-0.0f64).to_bits());
                }
            }
        }
    }

    #[test]
    fn same_input_same_bits(g in arb_ranked()) {
        let lists: Vec<Ranked<u8>> = g.into_iter().map(Ranked::new).collect();
        let a = fuse(&lists, &RRF, &Options::new()).unwrap();
        let b = fuse(&lists, &RRF, &Options::new()).unwrap();
        prop_assert_eq!(a, b);
    }
}

// ---- wrappers: each must equal the matching `fuse` call exactly -----------------------------------------------------

#[test]
fn wrappers_equal_fuse() {
    use crate::fuse::{borda, combmnz, combsum, rrf, weighted};
    let (ra, rb) = (r(&["x", "y", "w"]), r(&["y", "z"]));
    let (sa, sb) = (
        s(&[("x", 3.0), ("y", 2.0), ("w", 1.0)]),
        s(&[("y", 5.0), ("z", 1.0)]),
    );
    let o = Options::new();
    assert_eq!(rrf(&ra, &rb), fuse(&[ra.clone(), rb.clone()], &RRF, &o));
    assert_eq!(
        borda(&ra, &rb),
        fuse(&[ra.clone(), rb.clone()], &RankMethod::Borda, &o)
    );
    for (got, m) in [
        (weighted(&sa, &sb), ScoreMethod::Weighted),
        (combsum(&sa, &sb), ScoreMethod::CombSum),
        (combmnz(&sa, &sb), ScoreMethod::CombMnz),
    ] {
        assert_eq!(got, fuse(&[sa.clone(), sb.clone()], &m, &o));
    }
}

#[test]
fn wrappers_report_errors() {
    use crate::fuse::{rrf, weighted};
    assert!(matches!(
        rrf(&r(&["a", "a"]), &r(&["b"])),
        Err(FuseError::DuplicateId { .. })
    ));
    assert!(matches!(
        weighted(&s(&[("a", f64::NAN)]), &s(&[("b", 1.0)])),
        Err(FuseError::NonFiniteScore { .. })
    ));
}
