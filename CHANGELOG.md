# Changelog

## 0.2.0 - 2026-11-26 (old f32 API removed; **Breaking**; allocated by pm-assist)

- Removed the carried-over rankops f32 API at the crate root (`rrf`, `isr`, `combsum`, `FusionMethod`, the `*_multi` and
  `*_with_config` functions, configs, `explain`, `validate` and `prelude`), its derived test files (`src/tests.rs`,
  `src/proptests.rs`) and `src/validate.rs`, `src/fusion_order.rs`. Everything is reachable through `sortant::fuse`.
  Dropped without a replacement (contract section 6): `explain` (`rrf_explain` etc.), `analyze_consensus`,
  `attribute_top_k`, the output validators and Sigmoid normalisation; they return only on request.
- The `dbsf` regression is now `fuse_tests::dbsf_ranks_documents_in_both_lists_first`. The differential tests still compare
  every method with rankops 0.3.0 (a dev-dependency only).

## 0.1.0 (Stage 2 methods; **Breaking** for 0.0.6 callers: `ScoreMethod` lost `Eq`, sealed `Source::finish` changed; pre-1.0 second number; allocated by pm-assist 2026-11-22 per pm-ants)

- Wrappers and Stage 2 fusion methods: ISR, DBSF, Standardized, CombMAX/MIN/MED/ANZ, RBC, Condorcet, Copeland, MedianRank. Decisions to confirm (ISR formula, MedianRank penalty) are in the PR body.

## 0.0.6 (Stage 1: `fuse`; additive; allocated by pm-assist 2026-11-21)

- New `sortant::fuse` module, the single entry point (contract D1): `fuse(lists, &method, &Options) -> Result<Fused, FuseError>`.
  `Ranked<I>` lists take `RankMethod::{Rrf { k }, Borda}`, `Scored<I>` lists take `ScoreMethod::{Weighted, CombSum, CombMnz}`
  (D4), all f64, validated (D9, D10, D11, D13, D16), ordered by the one total order (D5) with `top_k`, tie-break ranker or
  comparator, `veto`/`force` and a typed conflict report (D7). The old f32 functions at the crate root are unchanged and
  will be removed as their stages land.
- Tests: exact values per method, every error, golden and generated properties, differential tests against rankops 0.3.0
  (`tests/differential.rs`, dev-dependency `rankops =0.3.0`) with each difference named; mutation check in `docs/MUTATION.md`.

## 0.0.5 (canonical Apache-2.0 text, step 4 shared pieces; additive; allocated by pm-assist 2026-11-20)

- `LICENSE-APACHE`: replaced with the canonical Apache-2.0 text (the old copy altered sections 6 and 9); PM finding via pm-ants.
- Step 4 shared pieces (not yet public API surface for any method): `error`, `sort`, `order`, `input`, `norm`; `docs/MUTATION.md`.

## 0.0.4 (strip, ROADMAP row 3; additive; allocated by pm-assist 2026-11-19)

- Strip: carry over the fusion core of rankops 0.3.0 (`src/lib.rs`, `fusion_order`, `validate`, `proptests`, 76 unit
  tests in `src/tests.rs`, the `dbsf` regression in `tests/dbsf_order.rs`), renamed, with provenance headers and
  `forbid(unsafe_code)`. Behaviour is rankops' until each method is ported (step 4). Derived-file table filled in.

## 0.0.3

- Add `docs/CONTRACT.md`, the step-2 contract note (options, recommendations, a decision record for every method), with the PM's amendments A1-A4.
- `PROVENANCE.md`: record that the crates.io checksum was checked, and stop naming a private path.

## 0.0.2

- Add the provenance record: `NOTICE`, `PROVENANCE.md` (origin pins, verification, derived-file table) and the
  upstream MIT licence text in `licenses/`. No code copied yet.

## 0.0.1

- Add `ROADMAP.md` and record the project charter essentials in `HANDOFF.md`. No code yet; the crate is still a
  placeholder (`publish = false`).
