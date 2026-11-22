# SortAnt roadmap

SortAnt is a small pure library that ranks items with pluggable comparators and fuses several weighted ranked lists
into one decision. No I/O, no ML, no `unsafe`. It starts from a fork of `rankops` 0.3.0 (MIT OR Apache-2.0), stripped
to the fusion core, repaired, documented and tested by us, then extended.

Statuses: `todo`, `doing`, `blocked`, `done`. A row is `done` only when its PR is merged on `main`.

| # | Row | Status | Notes |
|---|-----|--------|-------|
| 0 | ROADMAP.md, CI, branch protection on `main` | done | Merged in #1; protection set by the PM. |
| 1 | Provenance record: `NOTICE`/`CREDITS`, derived-file table, copy verified against the pinned tarball | done | No other code in this row. |
| 2 | Contract note: options first, then a recommendation | done | Approved by the PM with amendments A1-A4; merge of #3 closes it. |
| 3 | Strip per the inventory | done | Remove `rerank/*`, SIMD/`unsafe`, MMR, metrics, optimize, trec, dp_topk, pipeline, adapt, diagnostics, pareto, hierarchical. `#![forbid(unsafe_code)]`; zero required dependencies if possible. |
| 4 | Port in stages with value-asserting tests, property tests with golden cases, a mutation check, and differential tests against rankops 0.3.0 | done | Stage 1 (#6) and Stage 2 (#7) merged; every method in the CONTRACT section 6 table is in `fuse`; the old f32 API is removed on branch `old-api-removal`. |
| 5 | Our semantics (row 2), docs and examples that pass the ease gate | doing | A stranger fuses two lists in three lines. |
| 6 | Patent sweep | partial (see docs/PATENT-SWEEP.md; 2 of 5 hits read, 3 unread) | Ideas, queries with a positive control, claims read verbatim, "outside as read", never "clear". |

Known rankops defects that rows 2 and 4 must decide, not copy: see `HANDOFF.md`.
