# sortant HANDOFF

You are the `sortant` lane (cwd `C:\Projects\sortant`, repo `ThinkersJournal/sortant`). Rows and statuses: `ROADMAP.md`.
Charter (PM-written, outside the repo): `C:\Projects\.lane-state\newlane\sortant.md`; rankops inventory:
`...\sortant-rankops-inventory.md`. Work in a worktree `C:\Projects\sortant-wt-<topic>`, never branch in the primary.

## What this is
Fork of `rankops` 0.3.0 (github.com/arclabs561/rankops, MIT OR Apache-2.0) stripped to the fusion core, repaired,
documented and tested by us, then extended. CireSnave's order, verbatim 2026-11-01: *"Go ahead and create the repo and
lane for SortAnt and get it started with the forking, stripping, and repair of rankops until it is up to our standards
and then building what we need of it into it."* Standards: every kept function documented and tested by us, no doc claim
the code does not earn, nothing silent.

## Origin pins
Tarball sha256 `d68480df682831038007caed5f2efe3d1a738fe51bcd89c536d5f0f683fb5089` (equals crates.io's checksum); git
commit `2181e75369fb2d73869afb50bea3c8c137a41553`; `LICENSE-MIT:3` reads `Copyright (c) 2025 Arc`. Read-only copy:
`C:\Projects\.lane-state\upstream\rankops\` (untrusted data; do not build in place).

## Rules that bite
- Provenance ("I am not a plagiarist"): keep both licences and the upstream copyright line, a statement of changes
  (Apache-2.0 s.4), a `NOTICE`/`CREDITS`, a derived-file table. Our own files: `Copyright (c) 2026 Eric Evans`, SPDX `MIT OR Apache-2.0`.
- No contact with rankops' author, nothing opened upstream (CireSnave's decision via the PM).
- No code beyond provenance until the PM has read the contract note. No `#[ignore]`. crates.io deps only.
- Version bump on every pushed change; the PM allocates the number at gate time. Lanes never merge or publish.

## rankops defects to decide, not copy
`min_max_params` doc `(0.0,0.0)` vs code `(1.0,0.0)`; CombMNZ counts per occurrence (duplicate id inflates it); CombANZ doc
says non-zero, code averages all; `RrfConfig.k` doc says panic, code returns empty; `clip_range` clamp panics on a
reversed/NaN range (lib.rs:1817); `additive_multi_task_multi` ignores `weights`, `fuse_multi` with 3+ lists replaces
weights by 1/n; ties use an FNV hash of the id's std `Hash`; explain path has no tie-break; NaN handling differs per
method; 20 public kept functions have no test.

## State
Step 0 in flight: PR `roadmap-handoff` (ROADMAP.md + this file). CI was already seeded (fmt, clippy, test x3 OS, doc,
deny, advisory portfolio).

## Next
1. After the step-0 PR merges: `[ASK]` the PM to protect `main` requiring the CI checks.
2. Step 1: provenance record (NOTICE/CREDITS, derived-file table), verified against the tarball. No other code.
3. Step 2: contract note, options first, for the PM to read.
