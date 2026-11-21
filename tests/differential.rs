// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
//! Differential tests against rankops 0.3.0 (crates.io dev-dependency), CONTRACT section 7.
//!
//! Where rankops is correct, SortAnt agrees (scores within f32 precision). Every place it differs is a named decision
//! from `docs/CONTRACT.md`, with a test that asserts SortAnt's side next to rankops' behaviour.

use proptest::prelude::*;
use sortant::fuse::{Options, RankMethod, Ranked, ScoreMethod, Scored, fuse};
use std::collections::HashMap;

fn dedup<T: Clone>(v: &[(u8, T)]) -> Vec<(u8, T)> {
    let mut seen = std::collections::HashSet::new();
    v.iter()
        .filter(|(id, _)| seen.insert(*id))
        .cloned()
        .collect()
}

fn close(a: f64, b: f32) -> bool {
    (a - f64::from(b)).abs() <= 1e-4 * a.abs().max(1.0)
}

fn agree(ours: &[(u8, f64)], theirs: &[(u8, f32)]) -> Result<(), TestCaseError> {
    prop_assert_eq!(ours.len(), theirs.len());
    let t: HashMap<u8, f32> = theirs.iter().copied().collect();
    for (id, s) in ours {
        let other = t.get(id);
        prop_assert!(
            other.is_some_and(|o| close(*s, *o)),
            "id {} ours {} theirs {:?}",
            id,
            s,
            other
        );
    }
    // Ranking order agrees wherever our adjacent scores are clearly apart (ties are decided differently, D5).
    let pos: HashMap<u8, usize> = theirs.iter().enumerate().map(|(i, x)| (x.0, i)).collect();
    for w in ours.windows(2) {
        if w[0].1 - w[1].1 > 1e-3 {
            prop_assert!(pos[&w[0].0] < pos[&w[1].0]);
        }
    }
    Ok(())
}

fn ranked_lists() -> impl Strategy<Value = Vec<Vec<u8>>> {
    prop::collection::vec(
        prop::collection::vec((0u8..15, Just(())), 0..10)
            .prop_map(|v| dedup(&v).into_iter().map(|x| x.0).collect()),
        1..5,
    )
}

fn scored_lists() -> impl Strategy<Value = Vec<Vec<(u8, f64)>>> {
    // Quarter-unit scores are exact in f32; lists with a single distinct value are excluded (decision D16).
    prop::collection::vec(
        prop::collection::vec(
            (0u8..15, (-40i32..40).prop_map(|x| f64::from(x) / 4.0)),
            2..10,
        )
        .prop_map(|v| dedup(&v))
        .prop_filter("not constant", |v| v.iter().any(|x| x.1 != v[0].1)),
        1..5,
    )
}

fn as_rank(g: &[Vec<u8>]) -> Vec<Vec<(u8, f32)>> {
    g.iter()
        .map(|l| l.iter().map(|&i| (i, 0.0)).collect())
        .collect()
}

fn as_f32(g: &[Vec<(u8, f64)>]) -> Vec<Vec<(u8, f32)>> {
    g.iter()
        .map(|l| l.iter().map(|&(i, s)| (i, s as f32)).collect())
        .collect()
}

proptest! {
    #[test]
    fn rrf_agrees(g in ranked_lists()) {
        let lists: Vec<_> = g.iter().cloned().map(Ranked::new).collect();
        let ours = fuse(&lists, &RankMethod::Rrf { k: 60.0 }, &Options::new()).unwrap();
        let theirs = rankops::rrf_multi(&as_rank(&g), rankops::RrfConfig::new(60));
        agree(&ours.items, &theirs)?;
    }

    #[test]
    fn isr_agrees(g in ranked_lists()) {
        let lists: Vec<_> = g.iter().cloned().map(Ranked::new).collect();
        let ours = fuse(&lists, &RankMethod::Isr { k: 1.0 }, &Options::new()).unwrap();
        let theirs = rankops::isr_multi(&as_rank(&g), rankops::RrfConfig::new(1));
        agree(&ours.items, &theirs)?;
    }

    #[test]
    fn borda_agrees(g in ranked_lists()) {
        let lists: Vec<_> = g.iter().cloned().map(Ranked::new).collect();
        let ours = fuse(&lists, &RankMethod::Borda, &Options::new()).unwrap();
        let theirs = rankops::borda_multi(&as_rank(&g), rankops::FusionConfig::default());
        agree(&ours.items, &theirs)?;
    }

    #[test]
    fn combsum_agrees(g in scored_lists()) {
        let lists: Vec<_> = g.iter().cloned().map(Scored::new).collect();
        let ours = fuse(&lists, &ScoreMethod::CombSum, &Options::new()).unwrap();
        let theirs = rankops::combsum_multi(&as_f32(&g), rankops::FusionConfig::default());
        agree(&ours.items, &theirs)?;
    }

    #[test]
    fn combmnz_agrees(g in scored_lists()) {
        let lists: Vec<_> = g.iter().cloned().map(Scored::new).collect();
        let ours = fuse(&lists, &ScoreMethod::CombMnz, &Options::new()).unwrap();
        let theirs = rankops::combmnz_multi(&as_f32(&g), rankops::FusionConfig::default());
        agree(&ours.items, &theirs)?;
    }

    #[test]
    fn weighted_agrees(g in scored_lists(), w in prop::collection::vec(1u8..9, 5)) {
        let lists: Vec<_> = g.iter().cloned().map(Scored::new).collect();
        let weights: Vec<f64> = w[..g.len()].iter().map(|&x| f64::from(x)).collect();
        let ours = fuse(&lists, &ScoreMethod::Weighted, &Options::new().weights(weights.clone())).unwrap();
        let paired: Vec<(Vec<(u8, f32)>, f32)> =
            as_f32(&g).into_iter().zip(weights.iter().map(|&x| x as f32)).collect();
        let theirs = rankops::weighted_multi(&paired, true, None).unwrap();
        agree(&ours.items, &theirs)?;
    }
}

// ---- named differences (each is a decision in docs/CONTRACT.md) --------------------------------------------------

#[test]
fn d16_constant_list_is_neutral_not_raw() {
    let list = vec![("a", 7.0f64), ("b", 7.0)];
    let ours = fuse(&[Scored::new(list)], &ScoreMethod::CombSum, &Options::new()).unwrap();
    assert_eq!(ours.items, vec![("a", 0.5), ("b", 0.5)]);
    let theirs = rankops::combsum_multi(
        &[vec![("a", 7.0f32), ("b", 7.0)]],
        rankops::FusionConfig::default(),
    );
    assert_eq!(
        theirs,
        vec![("a", 7.0), ("b", 7.0)],
        "rankops passes the raw score through for a constant list"
    );
}

#[test]
fn d10_duplicate_id_is_an_error_not_a_sum() {
    let ours = fuse(
        &[Ranked::new(vec!["a", "a"])],
        &RankMethod::Borda,
        &Options::new(),
    );
    assert_eq!(
        ours,
        Err(sortant::error::FuseError::DuplicateId { list: 0, index: 1 })
    );
    let theirs = rankops::borda_multi(
        &[vec![("a", 0.0f32), ("a", 0.0)]],
        rankops::FusionConfig::default(),
    );
    assert_eq!(
        theirs,
        vec![("a", 3.0)],
        "rankops adds a point per occurrence"
    );
}

#[test]
fn d10_combmnz_duplicate_no_longer_inflates_the_multiplier() {
    let ours = fuse(
        &[Scored::new(vec![("a", 1.0), ("a", 2.0), ("b", 0.0)])],
        &ScoreMethod::CombMnz,
        &Options::new(),
    );
    assert!(ours.is_err());
    let theirs = rankops::combmnz_multi(
        &[vec![("a", 1.0f32), ("a", 2.0), ("b", 0.0)]],
        rankops::FusionConfig::default(),
    );
    assert_eq!(theirs[0].0, "a");
    assert!(
        theirs[0].1 > 1.0,
        "rankops counts the two occurrences as two lists"
    );
}

#[test]
fn d9_nan_is_an_error_not_a_silent_poison() {
    let ours = fuse(
        &[Scored::new(vec![("a", f64::NAN), ("b", 1.0)])],
        &ScoreMethod::CombSum,
        &Options::new(),
    );
    assert_eq!(
        ours,
        Err(sortant::error::FuseError::NonFiniteScore { list: 0, index: 0 })
    );
}
