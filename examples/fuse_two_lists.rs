// Copyright (c) 2026 Eric Evans. SPDX-License-Identifier: MIT OR Apache-2.0
//! Fuse two ranked lists in three lines.

use sortant::fuse::{Ranked, rrf};

fn main() {
    let keyword = Ranked::new(vec!["doc1", "doc2", "doc3"]);
    let dense = Ranked::new(vec!["doc2", "doc4", "doc1"]);
    let fused = rrf(&keyword, &dense).unwrap();
    for (id, score) in fused.items {
        println!("{id}: {score:.5}");
    }
}
