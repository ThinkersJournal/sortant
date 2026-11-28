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

`rrf` is reciprocal rank fusion with `k = 60`. For more than two lists, weights, a veto, a cut-off or other methods, use
`fuse(&lists, &method, &options)` (`examples/fuse_options.rs`, run with `cargo run --example fuse_options`):

```rust
use sortant::fuse::{Options, RankMethod, Ranked, fuse};

let lists = [
    Ranked::new(vec!["a", "b", "c"]),
    Ranked::new(vec!["b", "a", "d"]),
    Ranked::new(vec!["c", "a", "e"]),
];
// One weight per list, in list order (a wrong count is an error); a vetoed id is dropped from the output.
let options = Options::new().weights(vec![2.0, 1.0, 1.0]).veto(["b"]);
let fused = fuse(&lists, &RankMethod::Rrf { k: 60.0 }, &options).unwrap();
```

Rank methods (`RankMethod`) take `Ranked` lists; score methods (`ScoreMethod`) take `Scored` lists of `(id, f64)` and normalise
different scales. `Options` setters: `weights`, `veto`, `force`, `top_k`, `tie_break_ranker`, `tie_break_by`. Ids need only be
`Clone + Eq + Hash`; bad input is a typed `FuseError`, never a panic. Full API: `cargo doc --open`.

**Dependency.** Not yet on crates.io (the name is reserved by a `0.0.0` placeholder): until the first release, depend on it
with `sortant = { git = "https://github.com/ThinkersJournal/sortant" }`. Minimum Rust version 1.88 (library; its tests need 1.89).

**Origin and credit.** See `NOTICE` and `PROVENANCE.md` for the pinned upstream, the per-file derivation table and
what was changed.

Licensed under either of the Apache License, Version 2.0 (`LICENSE-APACHE`) or the MIT licence (`LICENSE-MIT`), at your
option.
