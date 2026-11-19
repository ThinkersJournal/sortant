# sortant

Rank fusion: combine several ranked lists into one decision. Work in progress; not published (`publish = false`).

**What exists today.** The fusion core of [`rankops`](https://github.com/arclabs561/rankops) 0.3.0 (MIT OR Apache-2.0,
Copyright (c) 2025 Arc), stripped of everything that is not fusion and carried over unrepaired: RRF, ISR, weighted,
Borda, CombSUM/MNZ/MAX/MIN/MED/ANZ, DBSF, standardised, additive multi-task, RBC, Condorcet, Copeland and median-rank
fusion, the normalisers, the explain functions and the output validators, with rankops' unit, property and doc tests.
It still has rankops' f32 API and its known defects. Each method is rewritten against `docs/CONTRACT.md` (f64, typed
errors, a total tie-break order, zero dependencies) before any release; see `ROADMAP.md`.

**Origin and credit.** See `NOTICE` and `PROVENANCE.md` for the pinned upstream, the per-file derivation table and
what was changed.

Licensed under either of the Apache License, Version 2.0 (`LICENSE-APACHE`) or the MIT licence (`LICENSE-MIT`), at your
option.
