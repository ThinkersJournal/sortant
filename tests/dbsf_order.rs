// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
// Derived from rankops 0.3.0 (Copyright (c) 2025 Arc, MIT OR Apache-2.0), tests/bughunt_dbsf_knapsack.rs (the `dbsf` test
// and `pos` helper only; the `dp_knapsack` tests dropped with that module). Modified: crate path renamed. See PROVENANCE.md.
//! Regression for `dbsf` ordering.

use sortant::dbsf;

fn pos(v: &[(&str, f32)], id: &str) -> usize {
    v.iter().position(|(d, _)| *d == id).unwrap()
}

/// The `dbsf` rustdoc example states "d2 and d3 appear in both lists, so they
/// rank highest". DBSF (Mazzeschi 2023; Qdrant) maps each list with
/// (s - (mu - 3 sigma)) / (6 sigma) and sums, so an absent document gets 0 and a
/// present one ~[0, 1]: expected order d2, d3, d1, d4. Summing raw clipped
/// z-scores makes "absent" equal to "at the list mean", so d3 (middle of B,
/// bottom of A) falls below d4 (only in B, at its bottom).
#[test]
fn dbsf_doc_example_ranks_shared_docs_first() {
    let bm25 = vec![("d1", 15.0), ("d2", 12.0), ("d3", 8.0)];
    let dense = vec![("d2", 0.9), ("d3", 0.7), ("d4", 0.5)];
    let fused = dbsf(&bm25, &dense);
    let order: Vec<&str> = fused.iter().map(|(d, _)| *d).collect();
    assert!(
        pos(&fused, "d3") < pos(&fused, "d4") && pos(&fused, "d3") < pos(&fused, "d1"),
        "dbsf order {order:?} (scores {fused:?}); DBSF reference order [d2, d3, d1, d4]"
    );
}
