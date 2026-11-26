# sortant

Rank fusion: combine several ranked lists into one decision. Work in progress; not published (`publish = false`).

**What exists today.** `sortant::fuse`: one entry point, `fuse(lists, &method, &Options)`, over ranked or scored lists, with
typed errors, f64 arithmetic that is bit-identical across platforms, a total tie-break order, no dependencies and no
`unsafe`. Rank methods: RRF, ISR, RBC, Borda, Condorcet, Copeland, median rank. Score methods: weighted, CombSUM, CombMNZ,
CombMAX/MIN/MED/ANZ, DBSF, standardised. Two-list wrappers `rrf`, `borda`, `weighted`, `combsum`, `combmnz`. Every method is
tested against rankops 0.3.0 where its behaviour is correct (dev-dependency only). The decisions are in
`docs/CONTRACT.md`; remaining work is in `ROADMAP.md`. The code began as a fork of rankops 0.3.0 and was rewritten method by
method; the old f32 API is gone.

**Quick start.** Two ranked lists, three lines (`examples/fuse_two_lists.rs`):

```rust
use sortant::fuse::{Ranked, rrf};

let keyword = Ranked::new(vec!["doc1", "doc2", "doc3"]);
let dense = Ranked::new(vec!["doc2", "doc4", "doc1"]);
let fused = rrf(&keyword, &dense).unwrap(); // fused.items: (id, score), best first
```

More than two lists, weights, a veto, a cut-off, or scored lists on different scales: `fuse(&lists, &method, &Options::new()...)`;
see `examples/fuse_options.rs` (`cargo run --example fuse_options`). Ids need only be `Clone + Eq + Hash`; bad input is a typed
`FuseError`, never a panic.

**Origin and credit.** See `NOTICE` and `PROVENANCE.md` for the pinned upstream, the per-file derivation table and
what was changed.

Licensed under either of the Apache License, Version 2.0 (`LICENSE-APACHE`) or the MIT licence (`LICENSE-MIT`), at your
option.
