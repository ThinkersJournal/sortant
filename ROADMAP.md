# SortAnt roadmap

SortAnt is a small pure library that ranks items with pluggable comparators and fuses several weighted ranked lists
into one decision. No I/O, no ML, no `unsafe`. It starts from a fork of `rankops` 0.3.0 (MIT OR Apache-2.0), stripped
to the fusion core, repaired, documented and tested by us, then extended.

Statuses: `todo`, `doing`, `blocked`, `done`. A row is `done` only when its PR is merged on `main`.

| # | Row | Status | Notes |
|---|-----|--------|-------|
| 0 | ROADMAP.md, CI, branch protection on `main` | done | Merged in #1; protection set by the PM. |
| 1 | Provenance record: `NOTICE`/`CREDITS`, derived-file table, copy verified against the pinned tarball | doing | No other code in this row. |
| 2 | Contract note: options first, then a recommendation | todo | Typed `Veto`/`Force`, total-order rule, comparator-driven ranking, f64, typed errors, NaN/duplicate-id/k=0 policy, a decision record for every method, deterministic output on every platform. The PM reads it before any code. |
| 3 | Strip per the inventory | todo | Remove `rerank/*`, SIMD/`unsafe`, MMR, metrics, optimize, trec, dp_topk, pipeline, adapt, diagnostics, pareto, hierarchical. `#![forbid(unsafe_code)]`; zero required dependencies if possible. |
| 4 | Port in stages with value-asserting tests, property tests with golden cases, a mutation check, and differential tests against rankops 0.3.0 | todo | Order: RRF, weighted score, Borda, CombSUM, CombMNZ; Condorcet and Copeland later. |
| 5 | Our semantics (row 2), docs and examples that pass the ease gate | todo | A stranger fuses two lists in three lines. |
| 6 | Patent sweep | todo | Ideas, queries with a positive control, claims read verbatim, "outside as read", never "clear". |

Known rankops defects that rows 2 and 4 must decide, not copy: see `HANDOFF.md`.
