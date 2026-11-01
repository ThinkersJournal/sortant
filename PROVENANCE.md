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
| crates.io checksum for 0.3.0 | the same 64 hex digits (read by the PM from the crates.io API) |
| Git commit (`.cargo_vcs_info.json`) | `2181e75369fb2d73869afb50bea3c8c137a41553` |

## Verification done (2026-11-01, this repo's lane)

- `sha256sum` of the tarball held at the PM's `.lane-state/upstream/rankops/rankops-0.3.0.crate` printed the digest above.
- The tarball was extracted afresh and `diff -r` against the PM's extracted reading copy printed no differences, so the
  reading copy is the tarball's content.
- `licenses/rankops-LICENSE-MIT` is a byte copy of the tarball's `LICENSE-MIT` (checked by `sha256sum` of both).
- Upstream `LICENSE-APACHE` differs from ours: it is the unfilled template. We keep our own filled `LICENSE-APACHE`.
- Not verified: the crates.io checksum itself (taken from the PM's reading); that the git commit contains the tarball's
  files (tarball and commit are linked only by `.cargo_vcs_info.json`, which we have not cross-checked against the forge).

## Derived-file table

**Nothing is copied yet.** Each row is added in the PR that copies the file, with the origin path, what changed, and
the header the file carries. Planned origins, from the PM's inventory (a reading, to be verified when copied):

| SortAnt file | Origin in rankops 0.3.0 | Status |
|---|---|---|
| fusion methods, normalisers, configs | `src/lib.rs` lines 1-3346 (split by section) | not copied |
| tie-break by comparator | `src/fusion_order.rs` | not copied |
| output validators | `src/validate.rs` | not copied |
| property tests | `src/proptests.rs` | not copied |
| fusion unit tests | `src/lib.rs` `mod tests` (fusion share) | not copied |

Everything else in rankops (`rerank/*`, `adapt`, `diagnostics`, `pipeline`, `metrics`, `trec`, `dp_topk`, MMR,
optimisation) is out of scope and will not be copied.

## Rules for adding a row

1. Copy from the verified tarball content, not from memory or another project.
2. Keep the upstream licence and copyright; add a header: modified from rankops 0.3.0 `<path>`, with a one-line
   statement of the change.
3. Record the row here in the same PR.
