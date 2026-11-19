// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
// Derived from rankops 0.3.0 (Copyright (c) 2025 Arc, MIT OR Apache-2.0), src/lib.rs `mod tests` lines 4098-5537.
// Modified: moved to its own file; the MMR, cosine, NDCG, precision, MAP, hit-rate and metric-dispatch tests (and their
// `make_qrels` helper) dropped with the code they test; rustfmt. See PROVENANCE.md.
//! Unit tests carried over from rankops 0.3.0 for the fusion core. They pin rankops' behaviour, defects included;
//! each is replaced or tightened as its method is ported under `docs/CONTRACT.md`.
use super::*;

fn ranked<'a>(ids: &[&'a str]) -> Vec<(&'a str, f32)> {
    ids.iter()
        .enumerate()
        .map(|(i, &id)| (id, 1.0 - i as f32 * 0.1))
        .collect()
}

#[test]
fn rrf_basic() {
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d2", "d3", "d4"]);
    let f = rrf(&a, &b);

    assert_eq!(f.iter().position(|(id, _)| *id == "d2").unwrap(), 0);
}

#[test]
fn rrf_with_top_k() {
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d2", "d3", "d4"]);
    let f = rrf_with_config(&a, &b, RrfConfig::default().with_top_k(2));

    assert_eq!(f.len(), 2);
}

#[test]
fn rrf_score_formula() {
    let a = vec![("d1", 1.0)];
    let b: Vec<(&str, f32)> = vec![];
    let f = rrf_with_config(&a, &b, RrfConfig::new(60));

    // Cormack et al. 2009: the top document has rank 1, so 1/(60+1).
    let expected = 1.0 / 61.0;
    assert!((f[0].1 - expected).abs() < 1e-6);
}

/// Every RRF entry point uses 1-based ranks, so the head of a single
/// list scores 1/(k+1) and the second item 1/(k+2).
#[test]
fn rrf_variants_use_one_based_rank() {
    let a = [("d1", 0.0), ("d2", 0.0)];
    let lists = [&a[..]];
    let expect = |f: &[(&str, f32)]| {
        assert!((f[0].1 - 1.0 / 61.0).abs() < 1e-7, "head {}", f[0].1);
        assert!((f[1].1 - 1.0 / 62.0).abs() < 1e-7, "second {}", f[1].1);
    };
    expect(&rrf_multi(&lists, RrfConfig::new(60)));
    expect(&rrf_weighted(&lists, &[1.0], RrfConfig::new(60)).unwrap());
    let explained = rrf_explain(&lists, &[RetrieverId::new("a")], RrfConfig::new(60));
    assert!((explained[0].score - 1.0 / 61.0).abs() < 1e-7);
    assert!((explained[0].explanation.sources[0].contribution - 1.0 / 61.0).abs() < 1e-7);
}

/// Tie order must not depend on std's unspecified `DefaultHasher`.
/// FNV-1a 64 over the bytes `Hash for str` feeds ("d1" then 0xff).
#[test]
fn tie_hash_is_fnv1a_and_pinned() {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in [b'd', b'1', 0xff] {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    assert_eq!(stable_hash(&"d1"), h);
}

/// Verify RRF score formula: score(d) = Σ 1/(k + rank) for all lists containing d
#[test]
fn rrf_exact_score_computation() {
    // d1 at rank 1 in list A, rank 3 in list B (1-based, Cormack et al. 2009)
    // With k=60: score = 1/(60+1) + 1/(60+3) = 1/61 + 1/63
    let a = vec![("d1", 0.9), ("d2", 0.8), ("d3", 0.7)];
    let b = vec![("d4", 0.9), ("d5", 0.8), ("d1", 0.7)];

    let f = rrf_with_config(&a, &b, RrfConfig::new(60));

    // Find d1's score
    let d1_score = f.iter().find(|(id, _)| *id == "d1").unwrap().1;
    let expected = 1.0 / 61.0 + 1.0 / 63.0; // rank 1 in A + rank 3 in B

    assert!(
        (d1_score - expected).abs() < 1e-6,
        "d1 score {} != expected {}",
        d1_score,
        expected
    );
}

/// Verify ISR score formula: score(d) = Σ 1/sqrt(k + rank)
#[test]
fn isr_exact_score_computation() {
    // d1 at rank 0 in list A, rank 2 in list B
    // With k=1: score = 1/sqrt(1+0) + 1/sqrt(1+2) = 1 + 1/sqrt(3)
    let a = vec![("d1", 0.9), ("d2", 0.8), ("d3", 0.7)];
    let b = vec![("d4", 0.9), ("d5", 0.8), ("d1", 0.7)];

    let f = isr_with_config(&a, &b, RrfConfig::new(1));

    let d1_score = f.iter().find(|(id, _)| *id == "d1").unwrap().1;
    let expected = 1.0 / 1.0_f32.sqrt() + 1.0 / 3.0_f32.sqrt();

    assert!(
        (d1_score - expected).abs() < 1e-6,
        "d1 score {} != expected {}",
        d1_score,
        expected
    );
}

/// Verify Borda score formula: score(d) = Σ (N - rank) where N = list length
#[test]
fn borda_exact_score_computation() {
    // List A: 3 items, d1 at rank 0 -> score = 3-0 = 3
    // List B: 4 items, d1 at rank 2 -> score = 4-2 = 2
    // Total d1 score = 3 + 2 = 5
    let a = vec![("d1", 0.9), ("d2", 0.8), ("d3", 0.7)];
    let b = vec![("d4", 0.9), ("d5", 0.8), ("d1", 0.7), ("d6", 0.6)];

    let f = borda(&a, &b);

    let d1_score = f.iter().find(|(id, _)| *id == "d1").unwrap().1;
    let expected = 3.0 + 2.0; // (3-0) + (4-2)

    assert!(
        (d1_score - expected).abs() < 1e-6,
        "d1 score {} != expected {}",
        d1_score,
        expected
    );
}

#[test]
fn rrf_weighted_applies_weights() {
    // d1 appears in list_a (rank 0), d2 appears in list_b (rank 0)
    let list_a = [("d1", 0.0)];
    let list_b = [("d2", 0.0)];

    // Weight list_b 3x more than list_a
    let weights = [0.25, 0.75];
    let f = rrf_weighted(&[&list_a[..], &list_b[..]], &weights, RrfConfig::new(60)).unwrap();

    // d2 should rank higher because its list has 3x the weight
    assert_eq!(f[0].0, "d2", "weighted RRF should favor higher-weight list");

    // Verify score formula: w / (k + rank)
    // d1: 0.25 / 61 = 0.00410
    // d2: 0.75 / 61 = 0.0123
    let d1_score = f.iter().find(|(id, _)| *id == "d1").unwrap().1;
    let d2_score = f.iter().find(|(id, _)| *id == "d2").unwrap().1;
    assert!(
        d2_score > d1_score * 2.0,
        "d2 should score ~3x higher than d1"
    );
}

#[test]
fn rrf_weighted_zero_weights_error() {
    let list_a = [("d1", 0.0)];
    let list_b = [("d2", 0.0)];
    let weights = [0.0, 0.0];

    let result = rrf_weighted(&[&list_a[..], &list_b[..]], &weights, RrfConfig::default());
    assert!(matches!(result, Err(FusionError::ZeroWeights)));
}

// ─────────────────────────────────────────────────────────────────────────
// ISR Tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn isr_basic() {
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d2", "d3", "d4"]);
    let f = isr(&a, &b);

    // d2 appears in both lists, should rank at the top
    assert_eq!(f.iter().position(|(id, _)| *id == "d2").unwrap(), 0);
}

#[test]
fn isr_score_formula() {
    // Single item in one list: score = 1/sqrt(k + 0) = 1/sqrt(k)
    let a = vec![("d1", 1.0)];
    let b: Vec<(&str, f32)> = vec![];
    let f = isr_with_config(&a, &b, RrfConfig::new(1));

    let expected = 1.0 / 1.0_f32.sqrt(); // 1/sqrt(1) = 1.0
    assert!((f[0].1 - expected).abs() < 1e-6);
}

#[test]
fn isr_gentler_decay_than_rrf() {
    // ISR should have a gentler decay than RRF
    // At rank 0 and rank 3 (with k=1):
    // RRF: 1/1 vs 1/4 = ratio of 4
    // ISR: 1/sqrt(1) vs 1/sqrt(4) = 1 vs 0.5 = ratio of 2
    let a = vec![("d1", 1.0), ("d2", 0.9), ("d3", 0.8), ("d4", 0.7)];
    let b: Vec<(&str, f32)> = vec![];

    let rrf_result = rrf_with_config(&a, &b, RrfConfig::new(1));
    let isr_result = isr_with_config(&a, &b, RrfConfig::new(1));

    // Calculate ratio of first to last score
    let rrf_ratio = rrf_result[0].1 / rrf_result[3].1;
    let isr_ratio = isr_result[0].1 / isr_result[3].1;

    // ISR should have smaller ratio (gentler decay)
    assert!(
        isr_ratio < rrf_ratio,
        "ISR should have gentler decay: ISR ratio={}, RRF ratio={}",
        isr_ratio,
        rrf_ratio
    );
}

#[test]
fn isr_multi_works() {
    let a = ranked(&["d1", "d2"]);
    let b = ranked(&["d2", "d3"]);
    let c = ranked(&["d3", "d4"]);
    let f = isr_multi(&[&a, &b, &c], RrfConfig::new(1));

    // All items should be present
    assert_eq!(f.len(), 4);
    // d2 and d3 appear in 2 lists each, d1 and d4 in 1
    // d2 at rank 1,0 => 1/sqrt(2) + 1/sqrt(1)
    // d3 at rank 1,0 => 1/sqrt(2) + 1/sqrt(1)
    // They should be top
    let top_2: Vec<_> = f.iter().take(2).map(|(id, _)| *id).collect();
    assert!(top_2.contains(&"d2") && top_2.contains(&"d3"));
}

#[test]
fn isr_with_top_k() {
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d2", "d3", "d4"]);
    let f = isr_with_config(&a, &b, RrfConfig::new(1).with_top_k(2));

    assert_eq!(f.len(), 2);
}

#[test]
fn isr_empty_lists() {
    let empty: Vec<(&str, f32)> = vec![];
    let non_empty = ranked(&["d1"]);

    assert_eq!(isr(&empty, &non_empty).len(), 1);
    assert_eq!(isr(&non_empty, &empty).len(), 1);
    assert_eq!(isr(&empty, &empty).len(), 0);
}

#[test]
fn fusion_method_isr() {
    let a = ranked(&["d1", "d2"]);
    let b = ranked(&["d2", "d3"]);

    let f = FusionMethod::isr().fuse(&a, &b);
    assert_eq!(f[0].0, "d2");

    // With custom k
    let f = FusionMethod::isr_with_k(10).fuse(&a, &b);
    assert_eq!(f[0].0, "d2");
}

#[test]
fn fusion_method_isr_multi() {
    let a = ranked(&["d1", "d2"]);
    let b = ranked(&["d2", "d3"]);
    let c = ranked(&["d3", "d4"]);
    let lists = [&a[..], &b[..], &c[..]];

    let f = FusionMethod::isr().fuse_multi(&lists);
    assert!(!f.is_empty());
}

#[test]
fn combmnz_rewards_overlap() {
    let a = ranked(&["d1", "d2"]);
    let b = ranked(&["d2", "d3"]);
    let f = combmnz(&a, &b);

    assert_eq!(f[0].0, "d2");
}

#[test]
fn combsum_basic() {
    let a = vec![("d1", 0.5), ("d2", 1.0)];
    let b = vec![("d2", 1.0), ("d3", 0.5)];
    let f = combsum(&a, &b);

    assert_eq!(f[0].0, "d2");
}

#[test]
fn weighted_skewed() {
    let a = vec![("d1", 1.0)];
    let b = vec![("d2", 1.0)];

    let f = weighted(
        &a,
        &b,
        WeightedConfig::default()
            .with_weights(0.9, 0.1)
            .with_normalize(false),
    );
    assert_eq!(f[0].0, "d1");

    let f = weighted(
        &a,
        &b,
        WeightedConfig::default()
            .with_weights(0.1, 0.9)
            .with_normalize(false),
    );
    assert_eq!(f[0].0, "d2");
}

#[test]
fn borda_symmetric() {
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d3", "d2", "d1"]);
    let f = borda(&a, &b);

    let scores: Vec<f32> = f.iter().map(|(_, s)| *s).collect();
    assert!((scores[0] - scores[1]).abs() < 0.01);
    assert!((scores[1] - scores[2]).abs() < 0.01);
}

#[test]
fn rrf_multi_works() {
    let lists: Vec<Vec<(&str, f32)>> = vec![
        ranked(&["d1", "d2"]),
        ranked(&["d2", "d3"]),
        ranked(&["d1", "d3"]),
    ];
    let f = rrf_multi(&lists, RrfConfig::default());

    assert_eq!(f.len(), 3);
}

#[test]
fn borda_multi_works() {
    let lists: Vec<Vec<(&str, f32)>> = vec![
        ranked(&["d1", "d2"]),
        ranked(&["d2", "d3"]),
        ranked(&["d1", "d3"]),
    ];
    let f = borda_multi(&lists, FusionConfig::default());
    assert_eq!(f.len(), 3);
    assert_eq!(f[0].0, "d1");
}

#[test]
fn combsum_multi_works() {
    let lists: Vec<Vec<(&str, f32)>> = vec![
        vec![("d1", 1.0), ("d2", 0.5)],
        vec![("d2", 1.0), ("d3", 0.5)],
        vec![("d1", 1.0), ("d3", 0.5)],
    ];
    let f = combsum_multi(&lists, FusionConfig::default());
    assert_eq!(f.len(), 3);
}

#[test]
fn combmnz_multi_works() {
    let lists: Vec<Vec<(&str, f32)>> = vec![
        vec![("d1", 1.0)],
        vec![("d1", 1.0), ("d2", 0.5)],
        vec![("d1", 1.0), ("d2", 0.5)],
    ];
    let f = combmnz_multi(&lists, FusionConfig::default());
    assert_eq!(f[0].0, "d1");
}

#[test]
fn weighted_multi_works() {
    let a = vec![("d1", 1.0)];
    let b = vec![("d2", 1.0)];
    let c = vec![("d3", 1.0)];

    let f = weighted_multi(&[(&a, 1.0), (&b, 1.0), (&c, 1.0)], false, None).unwrap();
    assert_eq!(f.len(), 3);

    let f = weighted_multi(&[(&a, 10.0), (&b, 1.0), (&c, 1.0)], false, None).unwrap();
    assert_eq!(f[0].0, "d1");
}

#[test]
fn weighted_multi_zero_weights() {
    let a = vec![("d1", 1.0)];
    let result = weighted_multi(&[(&a, 0.0)], false, None);
    assert!(matches!(result, Err(FusionError::ZeroWeights)));
}

#[test]
fn empty_inputs() {
    let empty: Vec<(&str, f32)> = vec![];
    let non_empty = ranked(&["d1"]);

    assert_eq!(rrf(&empty, &non_empty).len(), 1);
    assert_eq!(rrf(&non_empty, &empty).len(), 1);
}

#[test]
fn both_empty() {
    let empty: Vec<(&str, f32)> = vec![];
    assert_eq!(rrf(&empty, &empty).len(), 0);
    assert_eq!(combsum(&empty, &empty).len(), 0);
    assert_eq!(borda(&empty, &empty).len(), 0);
}

#[test]
fn duplicate_ids_in_same_list() {
    let a = vec![("d1", 1.0), ("d1", 0.5)];
    let b: Vec<(&str, f32)> = vec![];
    let f = rrf_with_config(&a, &b, RrfConfig::new(60));

    assert_eq!(f.len(), 1);
    let expected = 1.0 / 61.0 + 1.0 / 62.0;
    assert!((f[0].1 - expected).abs() < 1e-6);
}

#[test]
fn builder_pattern() {
    let config = RrfConfig::default().with_k(30).with_top_k(5);
    assert_eq!(config.k, 30);
    assert_eq!(config.top_k, Some(5));

    let config = WeightedConfig::default()
        .with_weights(0.8, 0.2)
        .with_normalize(false)
        .with_top_k(10);
    assert_eq!(config.weight_a, 0.8);
    assert!(!config.normalize);
    assert_eq!(config.top_k, Some(10));
}

// ─────────────────────────────────────────────────────────────────────────
// Edge Case Tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn nan_scores_handled() {
    let a = vec![("d1", f32::NAN), ("d2", 0.5)];
    let b = vec![("d2", 0.9), ("d3", 0.1)];

    // Should not panic and should return non-empty results
    let r = rrf(&a, &b);
    assert!(!r.is_empty());
    assert!(r.iter().all(|(_, s)| s.is_finite()));

    let r = combsum(&a, &b);
    assert!(!r.is_empty());

    let r = combmnz(&a, &b);
    assert!(!r.is_empty());

    let r = borda(&a, &b);
    assert!(!r.is_empty());
}

#[test]
fn inf_scores_handled() {
    let a = vec![("d1", f32::INFINITY), ("d2", 0.5)];
    let b = vec![("d2", f32::NEG_INFINITY), ("d3", 0.1)];

    // Should not panic and should return non-empty results
    let r = rrf(&a, &b);
    assert!(!r.is_empty());
    assert!(r.iter().all(|(_, s)| s.is_finite()));

    let r = combsum(&a, &b);
    assert!(!r.is_empty());
}

#[test]
fn zero_scores() {
    let a = vec![("d1", 0.0), ("d2", 0.0)];
    let b = vec![("d2", 0.0), ("d3", 0.0)];

    let f = combsum(&a, &b);
    assert_eq!(f.len(), 3);
}

#[test]
fn negative_scores() {
    let a = vec![("d1", -1.0), ("d2", -0.5)];
    let b = vec![("d2", -0.9), ("d3", -0.1)];

    let f = combsum(&a, &b);
    assert_eq!(f.len(), 3);
    // Should normalize properly
}

#[test]
fn large_k_value() {
    let a = ranked(&["d1", "d2"]);
    let b = ranked(&["d2", "d3"]);

    // k = u32::MAX should not overflow
    let f = rrf_with_config(&a, &b, RrfConfig::new(u32::MAX));
    assert!(!f.is_empty());
}

#[test]
#[should_panic(expected = "k must be >= 1")]
fn k_zero_panics() {
    let _ = RrfConfig::new(0);
}

#[test]
#[should_panic(expected = "k must be >= 1")]
fn k_zero_with_k_panics() {
    let _ = RrfConfig::default().with_k(0);
}

#[test]
fn all_nan_scores() {
    let a = vec![("d1", f32::NAN), ("d2", f32::NAN)];
    let b = vec![("d3", f32::NAN), ("d4", f32::NAN)];

    // Should not panic, but results may contain NaN
    let f = rrf(&a, &b);
    assert_eq!(f.len(), 4);
    // NaN values are valid RRF scores (1/(k+rank) is always finite)
    // But if all scores are NaN, the RRF calculation still works
    // Actually, RRF ignores scores, so NaN scores don't matter
    // All documents get RRF scores based on ranks, which are finite
    for (_, score) in &f {
        assert!(
            score.is_finite(),
            "RRF scores should be finite (based on ranks, not input scores)"
        );
    }
}

#[test]
fn empty_lists_multi() {
    let empty: Vec<Vec<(&str, f32)>> = vec![];
    assert_eq!(rrf_multi(&empty, RrfConfig::default()).len(), 0);
    assert_eq!(combsum_multi(&empty, FusionConfig::default()).len(), 0);
    assert_eq!(combmnz_multi(&empty, FusionConfig::default()).len(), 0);
    assert_eq!(borda_multi(&empty, FusionConfig::default()).len(), 0);
    assert_eq!(dbsf_multi(&empty, FusionConfig::default()).len(), 0);
    assert_eq!(isr_multi(&empty, RrfConfig::default()).len(), 0);
}

#[test]
fn rrf_weighted_list_weight_mismatch() {
    let a = [("d1", 1.0)];
    let b = [("d2", 1.0)];
    let weights = [0.5, 0.5, 0.0]; // 3 weights for 2 lists

    let result = rrf_weighted(&[&a[..], &b[..]], &weights, RrfConfig::default());
    assert!(matches!(result, Err(FusionError::InvalidConfig(_))));
}

#[test]
fn rrf_weighted_list_weight_mismatch_short() {
    let a = [("d1", 1.0)];
    let b = [("d2", 1.0)];
    let weights = [0.5]; // 1 weight for 2 lists

    let result = rrf_weighted(&[&a[..], &b[..]], &weights, RrfConfig::default());
    assert!(matches!(result, Err(FusionError::InvalidConfig(_))));
}

#[test]
fn duplicate_ids_commutative() {
    // Test that duplicate handling is commutative
    let a = vec![("d1", 1.0), ("d1", 0.5), ("d2", 0.3)];
    let b = vec![("d2", 0.9), ("d3", 0.7)];

    let ab = rrf(&a, &b);
    let ba = rrf(&b, &a);

    // Should have same document IDs (order may differ due to ties)
    let ab_ids: Vec<&str> = ab.iter().map(|(id, _)| *id).collect();
    let ba_ids: Vec<&str> = ba.iter().map(|(id, _)| *id).collect();
    assert_eq!(ab_ids.len(), ba_ids.len());
    // All IDs should appear in both
    for id in &ab_ids {
        assert!(ba_ids.contains(id));
    }
}

#[test]
fn dbsf_zero_variance() {
    // All scores equal in one list
    let a = vec![("d1", 1.0), ("d2", 1.0), ("d3", 1.0)];
    let b = vec![("d1", 0.9), ("d2", 0.5), ("d3", 0.1)];

    // Should not panic, list a contributes the midpoint 0.5 for all
    let f = dbsf(&a, &b);
    assert_eq!(f.len(), 3);
    // d1 should win (0.5 + highest normalized score from b)
    assert_eq!(f[0].0, "d1");
}

#[test]
fn single_item_lists() {
    let a = vec![("d1", 1.0)];
    let b = vec![("d1", 1.0)];

    let f = rrf(&a, &b);
    assert_eq!(f.len(), 1);

    let f = combsum(&a, &b);
    assert_eq!(f.len(), 1);

    let f = borda(&a, &b);
    assert_eq!(f.len(), 1);
}

#[test]
fn disjoint_lists() {
    let a = vec![("d1", 1.0), ("d2", 0.9)];
    let b = vec![("d3", 1.0), ("d4", 0.9)];

    let f = rrf(&a, &b);
    assert_eq!(f.len(), 4);

    let f = combmnz(&a, &b);
    assert_eq!(f.len(), 4);
    // No overlap bonus
}

#[test]
fn identical_lists() {
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d1", "d2", "d3"]);

    let f = rrf(&a, &b);
    // Order should be preserved
    assert_eq!(f[0].0, "d1");
    assert_eq!(f[1].0, "d2");
    assert_eq!(f[2].0, "d3");
}

#[test]
fn reversed_lists() {
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d3", "d2", "d1"]);

    let f = rrf(&a, &b);
    // All items appear in both lists, so all have same total RRF score
    // d2 at rank 2 in both gets: 2 * 1/(60+2) = 2/62
    // d1 at ranks 1,3 gets: 1/61 + 1/63
    // d3 at ranks 3,1 gets: 1/63 + 1/61
    // d1 and d3 tie, d2 is slightly lower (rank 2+2 vs 1+3)
    // Just check we get all 3
    assert_eq!(f.len(), 3);
}

#[test]
fn top_k_larger_than_result() {
    let a = ranked(&["d1"]);
    let b = ranked(&["d2"]);

    let f = rrf_with_config(&a, &b, RrfConfig::default().with_top_k(100));
    assert_eq!(f.len(), 2);
}

#[test]
fn top_k_zero() {
    let a = ranked(&["d1", "d2"]);
    let b = ranked(&["d2", "d3"]);

    let f = rrf_with_config(&a, &b, RrfConfig::default().with_top_k(0));
    assert_eq!(f.len(), 0);
}

// ─────────────────────────────────────────────────────────────────────────
// FusionMethod Tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn fusion_method_rrf() {
    let a = ranked(&["d1", "d2"]);
    let b = ranked(&["d2", "d3"]);

    let f = FusionMethod::rrf().fuse(&a, &b);
    assert_eq!(f[0].0, "d2"); // Appears in both
}

#[test]
fn fusion_method_combsum() {
    // Use scores where d2 clearly wins after normalization
    // a: d1=1.0 (norm: 1.0), d2=0.5 (norm: 0.0)
    // b: d2=1.0 (norm: 1.0), d3=0.5 (norm: 0.0)
    // Final: d1=1.0, d2=1.0, d3=0.0 - still a tie!
    // Use 3 elements to break the tie:
    let a = vec![("d1", 1.0_f32), ("d2", 0.6), ("d4", 0.2)];
    let b = vec![("d2", 1.0_f32), ("d3", 0.5)];
    // a norms: d1=(1.0-0.2)/0.8=1.0, d2=(0.6-0.2)/0.8=0.5, d4=0.0
    // b norms: d2=(1.0-0.5)/0.5=1.0, d3=0.0
    // Final: d1=1.0, d2=0.5+1.0=1.5, d3=0.0, d4=0.0

    let f = FusionMethod::CombSum.fuse(&a, &b);
    // d2 appears in both lists with high scores, should win
    assert_eq!(f[0].0, "d2");
}

#[test]
fn fusion_method_combmnz() {
    let a = ranked(&["d1", "d2"]);
    let b = ranked(&["d2", "d3"]);

    let f = FusionMethod::CombMnz.fuse(&a, &b);
    assert_eq!(f[0].0, "d2"); // Overlap bonus
}

#[test]
fn fusion_method_borda() {
    let a = ranked(&["d1", "d2"]);
    let b = ranked(&["d2", "d3"]);

    let f = FusionMethod::Borda.fuse(&a, &b);
    assert_eq!(f[0].0, "d2");
}

#[test]
fn fusion_method_weighted() {
    let a = vec![("d1", 1.0f32)];
    let b = vec![("d2", 1.0f32)];

    // Heavy weight on first list
    let f = FusionMethod::weighted(0.9, 0.1).fuse(&a, &b);
    assert_eq!(f[0].0, "d1");

    // Heavy weight on second list
    let f = FusionMethod::weighted(0.1, 0.9).fuse(&a, &b);
    assert_eq!(f[0].0, "d2");
}

#[test]
fn fusion_method_multi() {
    let lists: Vec<Vec<(&str, f32)>> = vec![
        ranked(&["d1", "d2"]),
        ranked(&["d2", "d3"]),
        ranked(&["d1", "d3"]),
    ];

    let f = FusionMethod::rrf().fuse_multi(&lists);
    assert_eq!(f.len(), 3);
    // d1 and d2 both appear in 2 lists, should be top 2
}

#[test]
fn fusion_method_default_is_rrf() {
    let method = FusionMethod::default();
    assert!(matches!(method, FusionMethod::Rrf { k: 60 }));
}

#[test]
fn rbc_weights_follow_bailey_2017() {
    // Bailey et al. 2017: weight at 1-indexed rank r is (1-phi) * phi^(r-1).
    let list = vec![("a", 5.0), ("b", 4.0), ("c", 3.0), ("d", 2.0), ("e", 1.0)];
    let fused = rbc_multi(&[list], 0.8);
    let score = |id: &str| fused.iter().find(|(i, _)| *i == id).unwrap().1;
    let expected = [0.2_f32, 0.16, 0.128, 0.1024, 0.08192];
    for (id, want) in ["a", "b", "c", "d", "e"].iter().zip(expected) {
        assert!(
            (score(id) - want).abs() < 1e-5,
            "{id}: {} vs {want}",
            score(id)
        );
    }
}

#[test]
fn rbc_higher_persistence_is_less_top_heavy() {
    // Expected depth is 1/(1-phi): higher persistence spreads weight deeper.
    let list = vec![("a", 3.0), ("b", 2.0), ("c", 1.0)];
    let ratio = |p: f32| {
        let f = rbc_multi(std::slice::from_ref(&list), p);
        let s = |id: &str| f.iter().find(|(i, _)| *i == id).unwrap().1;
        s("c") / s("a")
    };
    assert!(ratio(0.9) > ratio(0.5));
}

#[test]
fn normalize_quantile_tolerates_nan_input() {
    // 64 entries incl. NaN: an inconsistent comparator can panic in sort_by.
    let results: Vec<(usize, f32)> = (0..64)
        .map(|i| {
            (
                i,
                if i % 5 == 0 {
                    f32::NAN
                } else {
                    ((i * 37) % 64) as f32
                },
            )
        })
        .collect();
    let out = normalize_scores(&results, Normalization::Quantile);
    assert_eq!(out.len(), 64);
    // Finite scores keep their relative order.
    let q = |i: usize| out[i].1;
    assert!((q(1) < q(2)) == (results[1].1 < results[2].1));
}

// ── Normalization Tests ────────────────────────────────────────────────

#[test]
fn quantile_normalization() {
    let results = vec![
        ("a", 10.0),
        ("b", 20.0),
        ("c", 30.0),
        ("d", 40.0),
        ("e", 50.0),
    ];
    let normed = normalize_scores(&results, Normalization::Quantile);

    // Sorted ascending: a(10)=0.0, b(20)=0.25, c(30)=0.5, d(40)=0.75, e(50)=1.0
    assert!((normed[0].1 - 0.0).abs() < 1e-6, "a should be 0.0");
    assert!((normed[1].1 - 0.25).abs() < 1e-6, "b should be 0.25");
    assert!((normed[2].1 - 0.5).abs() < 1e-6, "c should be 0.5");
    assert!((normed[4].1 - 1.0).abs() < 1e-6, "e should be 1.0");
}

#[test]
fn quantile_normalization_single() {
    let results = vec![("a", 42.0)];
    let normed = normalize_scores(&results, Normalization::Quantile);
    assert!((normed[0].1 - 0.5).abs() < 1e-6, "single item gets 0.5");
}

#[test]
fn sigmoid_normalization() {
    let results = vec![("a", -10.0), ("b", 0.0), ("c", 10.0)];
    let normed = normalize_scores(&results, Normalization::Sigmoid);

    // sigmoid(-10) ~ 0.0000454, sigmoid(0) = 0.5, sigmoid(10) ~ 0.99995
    assert!(normed[0].1 < 0.01, "sigmoid(-10) should be near 0");
    assert!((normed[1].1 - 0.5).abs() < 1e-6, "sigmoid(0) should be 0.5");
    assert!(normed[2].1 > 0.99, "sigmoid(10) should be near 1");
}

#[test]
fn sigmoid_preserves_order() {
    let results = vec![("a", 1.0), ("b", 3.0), ("c", 2.0)];
    let normed = normalize_scores(&results, Normalization::Sigmoid);

    // b(3.0) > c(2.0) > a(1.0) should hold after sigmoid
    assert!(normed[1].1 > normed[2].1);
    assert!(normed[2].1 > normed[0].1);
}

#[test]
fn quantile_handles_non_gaussian() {
    // Scores with extreme outlier -- quantile should be robust
    let results = vec![
        ("a", 0.1),
        ("b", 0.2),
        ("c", 0.3),
        ("d", 100.0), // extreme outlier
    ];
    let normed = normalize_scores(&results, Normalization::Quantile);

    // Quantile normalization: ranks are 0/3, 1/3, 2/3, 3/3
    assert!((normed[0].1 - 0.0).abs() < 1e-6);
    assert!((normed[1].1 - 1.0 / 3.0).abs() < 1e-6);
    assert!((normed[2].1 - 2.0 / 3.0).abs() < 1e-6);
    assert!((normed[3].1 - 1.0).abs() < 1e-6);
}

// ── Copeland & Median Rank Tests ───────────────────────────────────────

#[test]
fn copeland_basic() {
    // Three lists, d2 is preferred by majority in all pairwise comparisons
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d2", "d1", "d3"]);
    let c = ranked(&["d2", "d3", "d1"]);

    let f = copeland_multi(&[&a, &b, &c], FusionConfig::default());
    // d2 ranks first in 2/3 lists vs d1, and first in all vs d3
    assert_eq!(f[0].0, "d2", "d2 should be Copeland winner");
}

#[test]
fn copeland_net_wins() {
    // d1 at rank 0 in both lists. d2 at rank 1 in both. d3 at rank 2 in both.
    let a = vec![("d1", 0.9), ("d2", 0.8), ("d3", 0.7)];
    let b = vec![("d1", 0.9), ("d2", 0.8), ("d3", 0.7)];

    let f = copeland(&a, &b);
    // d1 beats both d2 and d3 in both lists: net = +2
    // d2 loses to d1, beats d3: net = 0
    // d3 loses to both: net = -2
    assert_eq!(f[0].0, "d1");
    assert!((f[0].1 - 2.0).abs() < 1e-6, "d1 net wins should be 2");
    assert_eq!(f[2].0, "d3");
    assert!((f[2].1 - (-2.0)).abs() < 1e-6, "d3 net wins should be -2");
}

#[test]
fn copeland_vs_condorcet_more_discriminative() {
    // Copeland distinguishes between "close loser" and "total loser"
    // Condorcet only counts wins, so both losers get the same score
    let a = vec![("d1", 0.9), ("d2", 0.8), ("d3", 0.7)];
    let b = vec![("d1", 0.9), ("d3", 0.8), ("d2", 0.7)];

    let cope = copeland(&a, &b);
    let cond = condorcet(&a, &b);

    // Condorcet: d1=2 wins, d2=0 wins (ties with d3), d3=0 wins
    // Copeland: d1=+2, d2=0 (1 win, 1 loss), d3=0 (1 win, 1 loss)
    // Both give d1 first place
    assert_eq!(cope[0].0, "d1");
    assert_eq!(cond[0].0, "d1");
}

#[test]
fn copeland_commutative() {
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d3", "d1", "d2"]);

    let f1 = copeland(&a, &b);
    let f2 = copeland(&b, &a);

    // Same results regardless of input order
    assert_eq!(f1.len(), f2.len());
    for (r1, r2) in f1.iter().zip(f2.iter()) {
        assert_eq!(r1.0, r2.0);
        assert!((r1.1 - r2.1).abs() < 1e-6);
    }
}

#[test]
fn median_rank_basic() {
    // d1: rank 0 in both lists -> median 0 -> score 1/(1+0) = 1.0
    // d2: rank 1 in list a, rank 0 in list b -> median 0.5 -> score 1/1.5
    // d3: rank 2 in list a, absent in list b -> ranks [2, 3] -> median 2.5
    let a = vec![("d1", 0.9), ("d2", 0.8), ("d3", 0.7)];
    let b = vec![("d1", 0.9), ("d2", 0.8)];

    let f = median_rank(&a, &b);
    assert_eq!(f[0].0, "d1", "d1 should rank first (median rank 0)");
    assert!((f[0].1 - 1.0).abs() < 1e-6);
}

#[test]
fn median_rank_outlier_robust() {
    // d1: ranks [0, 0, 100] -> median 0 (outlier ignored)
    // d2: ranks [1, 1, 1] -> median 1
    // With 3 lists, d1 should still rank above d2 despite one terrible rank
    let a = vec![("d1", 0.9), ("d2", 0.8)];
    let b = vec![("d1", 0.9), ("d2", 0.8)];
    // In list c, d1 is at rank 5 (far down), d2 is at rank 0
    let c: Vec<(&str, f32)> = vec![
        ("x1", 0.9),
        ("x2", 0.8),
        ("x3", 0.7),
        ("x4", 0.6),
        ("x5", 0.5),
        ("d1", 0.4),
        ("d2", 0.3),
    ];

    let f = median_rank_multi(&[&a, &b, &c], FusionConfig::default());

    let d1_pos = f.iter().position(|(id, _)| *id == "d1").unwrap();
    let d2_pos = f.iter().position(|(id, _)| *id == "d2").unwrap();
    // d1 median rank = 0 (ranks: [0, 0, 5] -> sorted [0, 0, 5] -> median 0)
    // d2 median rank = 1 (ranks: [1, 1, 6] -> sorted [1, 1, 6] -> median 1)
    assert!(
        d1_pos < d2_pos,
        "d1 should rank above d2 (outlier-robust median)"
    );
}

#[test]
fn median_rank_commutative() {
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d3", "d1", "d2"]);

    let f1 = median_rank(&a, &b);
    let f2 = median_rank(&b, &a);

    assert_eq!(f1.len(), f2.len());
    for (r1, r2) in f1.iter().zip(f2.iter()) {
        assert_eq!(r1.0, r2.0);
        assert!((r1.1 - r2.1).abs() < 1e-6);
    }
}

#[test]
fn fusion_method_copeland_dispatch() {
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d2", "d1", "d3"]);

    let direct = copeland(&a, &b);
    let via_enum = FusionMethod::Copeland.fuse(&a, &b);

    // Compare as score maps (tie-breaking order may differ)
    let direct_map: HashMap<_, _> = direct.into_iter().collect();
    let enum_map: HashMap<_, _> = via_enum.into_iter().collect();
    assert_eq!(direct_map.len(), enum_map.len());
    for (id, score) in &direct_map {
        let other = enum_map.get(id).expect("same keys");
        assert!((score - other).abs() < 1e-6);
    }
}

#[test]
fn fusion_method_median_rank_dispatch() {
    let a = ranked(&["d1", "d2", "d3"]);
    let b = ranked(&["d3", "d1", "d2"]);

    let direct = median_rank(&a, &b);
    let via_enum = FusionMethod::MedianRank.fuse(&a, &b);

    let direct_map: HashMap<_, _> = direct.into_iter().collect();
    let enum_map: HashMap<_, _> = via_enum.into_iter().collect();
    assert_eq!(direct_map.len(), enum_map.len());
    for (id, score) in &direct_map {
        let other = enum_map.get(id).expect("same keys");
        assert!((score - other).abs() < 1e-6);
    }
}

#[test]
fn fusion_ties_have_a_stable_order() {
    let a = vec![("a", 1.0), ("b", 1.0)];
    let b = vec![("c", 1.0), ("d", 1.0)];
    let expected = rrf(&a, &b);
    for _ in 0..32 {
        assert_eq!(rrf(&a, &b), expected);
    }
}
