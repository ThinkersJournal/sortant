# sortant

Rank fusion: combine several ranked lists into one decision. Work in progress; not published (`publish = false`).

**What exists today.** `sortant::fuse`: one entry point, `fuse(lists, &method, &Options)`, over ranked or scored lists, with
typed errors, f64 arithmetic that is bit-identical across platforms, a total tie-break order, no dependencies and no
`unsafe`. Rank methods: RRF, ISR, RBC, Borda, Condorcet, Copeland, median rank. Score methods: weighted, CombSUM, CombMNZ,
CombMAX/MIN/MED/ANZ, DBSF, standardised. Two-list wrappers `rrf`, `borda`, `weighted`, `combsum`, `combmnz`. Every method is
tested against rankops 0.3.0 where its behaviour is correct (dev-dependency only). The decisions are in
`docs/CONTRACT.md`; remaining work is in `ROADMAP.md`. The code began as a fork of rankops 0.3.0 and was rewritten method by
method; the old f32 API is gone.

**Origin and credit.** See `NOTICE` and `PROVENANCE.md` for the pinned upstream, the per-file derivation table and
what was changed.

Licensed under either of the Apache License, Version 2.0 (`LICENSE-APACHE`) or the MIT licence (`LICENSE-MIT`), at your
option.
