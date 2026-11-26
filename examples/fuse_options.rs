// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
//! Fuse three lists with weights, a veto and a cut-off, then fuse scored lists with a normalised method.

use sortant::fuse::{Options, RankMethod, Ranked, ScoreMethod, Scored, fuse};

fn main() {
    // Three ranked lists (best first). The first one is trusted twice as much.
    let lists = [
        Ranked::new(vec!["a", "b", "c", "d"]),
        Ranked::new(vec!["b", "a", "e"]),
        Ranked::new(vec!["c", "b", "a"]),
    ];
    let options = Options::new()
        .weights(vec![2.0, 1.0, 1.0])
        .veto(["d"]) // never return "d"
        .top_k(3);
    let fused = fuse(&lists, &RankMethod::Rrf { k: 60.0 }, &options).unwrap();
    for (id, score) in &fused.items {
        println!("{id}: {score:.5}");
    }

    // Scored lists on different scales: DBSF normalises each list before adding.
    let bm25 = Scored::new(vec![("a", 12.0), ("b", 9.5), ("c", 3.0)]);
    let cosine = Scored::new(vec![("b", 0.91), ("c", 0.88), ("a", 0.40)]);
    let fused = fuse(&[bm25, cosine], &ScoreMethod::Dbsf, &Options::new()).unwrap();
    for (id, score) in &fused.items {
        println!("{id}: {score:.5}");
    }
}
