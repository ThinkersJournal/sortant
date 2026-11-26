# Provenance

SortAnt starts from a fork of `rankops` 0.3.0. This file records exactly what we took, from where, and how we checked it.
Credit and licence notices: `NOTICE`, `licenses/rankops-LICENSE-MIT`.

## Origin

| Item | Value |
|---|---|
| Crate | `rankops` 0.3.0, <https://github.com/arclabs561/rankops> |
| Licence | MIT OR Apache-2.0 |
| Copyright line | `Copyright (c) 2025 Arc` (upstream `LICENSE-MIT:3`) |
| Tarball sha256 | `d68480df682831038007caed5f2efe3d1a738fe51bcd89c536d5f0f683fb5089` |
| crates.io checksum for 0.3.0 | the same 64 hex digits (crates.io API) |
| Git commit (`.cargo_vcs_info.json`) | `2181e75369fb2d73869afb50bea3c8c137a41553` |

## Verification done (2026-11-01, this repo's lane)

- `sha256sum` of a copy of the tarball printed the digest above, equal to the crates.io checksum for 0.3.0 (read from
  the crates.io API).
- The tarball was extracted afresh and `diff -r` against a second extracted reading copy printed no differences, so
  that reading copy is the tarball's content.
- `licenses/rankops-LICENSE-MIT` is a byte copy of the tarball's `LICENSE-MIT` (checked by `sha256sum` of both).
- Upstream `LICENSE-APACHE` is the unfilled Apache-2.0 template. Ours is the canonical Apache-2.0 text, byte-identical to the portfolio's canonical copy (sha256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`, 11358 bytes; replaced 2026-11-20, PM finding: our earlier copy had a non-canonical section 6 and 9). Copyright holders are named in `NOTICE` and the file headers, not in the licence text.
- Not verified: that the git commit contains the tarball's files. Tarball and commit are linked only by
  `.cargo_vcs_info.json`, which we have not cross-checked against the forge.

## Derived-file table

Copied in the strip PR (step 3). Every file below carries the header "Derived from rankops 0.3.0 ... See PROVENANCE.md";
the upstream MIT text and copyright line are in `licenses/` and `NOTICE`. All still carry rankops' behaviour (f32 API,
defects included) until each method is ported under `docs/CONTRACT.md` (step 4).

| SortAnt file | Origin in rankops 0.3.0 | Change made |
|---|---|---|
| `src/lib.rs` | `src/lib.rs` lines 1-3346 (fusion, normalisers, configs, explain) | crate renamed; rerank/metrics/pipeline/adapt/diagnostics/trec/dp_topk modules, metrics prelude, serde derives and an unverified OpenSearch benchmark claim dropped; `forbid(unsafe_code)`; rustfmt; two clippy `collapsible_if` rewrites **Removed in the old-API removal change (old f32 API retired); the carried copy is in git history at `37189d0`.** |
| `src/fusion_order.rs` | `src/fusion_order.rs` | crate renamed; rustfmt; clippy `collapsible_if` where flagged **Removed in the old-API removal change (old f32 API retired); the carried copy is in git history at `37189d0`.** |
| `src/validate.rs` | `src/validate.rs` | crate renamed; rustfmt; clippy `collapsible_if` where flagged **Removed in the old-API removal change (old f32 API retired); the carried copy is in git history at `37189d0`.** |
| `src/proptests.rs` | `src/proptests.rs` | crate renamed; rustfmt; clippy `collapsible_if` where flagged **Removed in the old-API removal change (old f32 API retired); the carried copy is in git history at `37189d0`.** |
| `src/tests.rs` | `src/lib.rs` `mod tests`, lines 4098-5537 | moved to its own file; 76 fusion tests kept; the 25 MMR, cosine, NDCG, precision, MAP, hit-rate, metric-dispatch tests and the `make_qrels` helper dropped with the code they test; rustfmt **Removed in the old-API removal change (old f32 API retired); the carried copy is in git history at `37189d0`.** |
| `src/pairwise.rs` | `src/lib.rs` `condorcet_multi`, `copeland_multi`, `median_rank_multi` (lines 3100-3340 in the carried copy) | reimplemented, not copied: same pairwise rules and median-rank penalty (`longest list + 1`, 0-indexed), but f64, first-appearance order instead of `HashSet` order, per-list weights (D13), no `i32`/`f32` counters; checked against rankops by `tests/differential.rs` |
| `tests/dbsf_order.rs` | `tests/bughunt_dbsf_knapsack.rs` (the `dbsf` test and `pos` helper) | crate path renamed; the `dp_knapsack` tests dropped **Removed in the old-API removal change (old f32 API retired); the carried copy is in git history at `37189d0`.** |

### Files written new for SortAnt that follow rankops' rules

These are not copies: they were written under `docs/CONTRACT.md` and carry the SortAnt copyright header. Where they
reproduce a rankops formula (a published or textbook definition that rankops also implements), the row says which, and
`tests/differential.rs` compares the result with rankops 0.3.0 itself.

| SortAnt file | Follows rankops 0.3.0 for | Not rankops |
|---|---|---|
| `src/fuse.rs` | the per-method formulas of RRF, Borda, ISR (`1/sqrt(k+pos)`), RBC, CombSUM/MNZ/MAX/MIN/MED/ANZ, weighted, DBSF and Standardized (as called in `src/lib.rs`) | the single `fuse` entry, `Options`, input types, accumulation, ordering, veto/force, CombANZ meaning, validation |
| `src/norm.rs` | the min-max, DBSF and z-score definitions | f64, exact-equality constant rule (A3), overflow error, clip validation |
| `src/pairwise.rs` | see its row above | |
| `src/input.rs`, `src/order.rs`, `src/sort.rs`, `src/error.rs` | nothing copied; written for the step 4 contract | recorded by the step 4 author, not re-verified in the Stage 2 PR |

Not copied from rankops' `tests/`: `adaptive_mmr.rs`, `rerank_backend_contract.rs`, `trec_parity.rs`, `fixtures/`
(all out of scope).

Everything else in rankops (`rerank/*`, `adapt`, `diagnostics`, `pipeline`, `metrics`, `trec`, `dp_topk`, MMR,
optimisation) is out of scope and will not be copied.

## Rules for adding a row

1. Copy from the verified tarball content, not from memory or another project.
2. Keep the upstream licence and copyright; add a header: modified from rankops 0.3.0 `<path>`, with a one-line
   statement of the change.
3. Record the row here in the same PR.
