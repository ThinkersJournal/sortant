# Changelog

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
