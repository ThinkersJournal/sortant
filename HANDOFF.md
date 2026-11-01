# sortant HANDOFF

You are the `sortant` lane (cwd `C:\Projects\sortant`, repo `ThinkersJournal/sortant`). Rows and statuses: `ROADMAP.md`.
Charter (PM-written, outside the repo): `C:\Projects\.lane-state\newlane\sortant.md`; rankops inventory:
`...\sortant-rankops-inventory.md`. Work in a worktree `C:\Projects\sortant-wt-<topic>`, never branch in the primary.
Origin pins, rules and the defect list: `PROVENANCE.md`, `NOTICE`, `docs/CONTRACT.md` (decisions D1-D17 plus
amendments A1-A4, all approved by the PM; they are the spec, do not re-derive them).

## State (paused 2026-11-01 by PRIORITY mode; resume when the PM starts you; read THROTTLE.md first)
- main = `8e6811e`, version 0.0.3. Merged: #1 roadmap, #2 provenance, #3 contract note (step 2 closed).
- Step 3 (strip) is saved on branch `wip-strip` (`b3fb8b3`, pushed, NO PR). Worktree `C:\Projects\sortant-wt-strip` (may be
  gone; recreate from the branch). It is rankops lib.rs 1-3346 + fusion_order.rs + validate.rs + proptests.rs, renamed,
  headers added, `forbid(unsafe_code)`, dev-dep proptest. Builds; clippy `-D warnings` clean; 30 unit + 23 doc tests pass.
  It still carries rankops' f32 API and behaviour, defects included, by design (step 4 ports method by method).
- No background shells running. No PM task open besides the below.

## Next (in order)
1. Rebase `wip-strip` onto current `origin/main` (`git rebase --onto origin/main 34d8b86 wip-strip`; diff after, check the
   sign), then finish step 3: port the KEEP tests from rankops `mod tests` (lib.rs 4097-5537 in the tarball; ~75 tests, drop
   the mmr/ndcg/metrics ones; the inventory notes its name heuristic wrongly drops `fusion_method_median_rank_dispatch`) and
   `tests/bughunt_dbsf_knapsack.rs` (DBSF test only); add derived-file rows to `PROVENANCE.md` (every file in `src/`, origin
   path, change made); update README to say what exists; CHANGELOG. Take the version from the PM at `[READY]`.
2. Step 4, stage 1, one PR per stage, per contract sections 6 and 7: RRF, weighted, Borda, CombSUM, CombMNZ, each with
   exact-value tests, golden property cases, a mutation check, and differential tests against rankops 0.3.0 (crates.io
   dev-dependency) where rankops is correct. First build the shared pieces: validation (D9, D10, D12 errors, `-0.0` to `0.0`),
   the total order (D5), the stable merge sort for caller comparators with its own tests and mutation check (A1), min-max with
   `(s - min) / range` and exact-equality constant (A3).
3. Later stages and steps 5/6 as in `ROADMAP.md`.

## Decisions (contract note, all PM-approved; detail in `docs/CONTRACT.md`)
D1 one `fuse` entry + logic-free wrappers. D2 zero deps, forbid unsafe. D3 id is `Clone+Eq+Hash`, never orders. D4 `Ranked` vs
`Scored` input types. D5 order = score desc, then tie-break ranker, then input order; `-0.0` canonicalised (A2). D6 comparator
`rank_by`; own safe stable merge sort for caller comparators (A1: std `sort_by` may panic on non-total orders). D7 `veto`/`force`
sets in Options, Veto beats Force, conflict in a typed field. D8 f64 only. D9 non-finite score is an error. D10 duplicate id in a
list is an error. D11 `top_k=0` is Ok(empty), RRF `k` invalid is `InvalidConfig`, no panics except a panicking caller closure.
D12 one `#[non_exhaustive] FuseError` (`NonFiniteScore`, `DuplicateId`, `InvalidWeights`, `InvalidConfig`, `LengthMismatch`,
`RangeOverflow`). D13 one validated weight per list. D14 bit-identical output, no `powf/powi/exp/ln/mul_add` (A4: RBC by repeated
multiplication; `sqrt` allowed). D15 rank is usize. D16 constant list normalises to 0.5 (exact `max == min`, A3). D17 one
normaliser. Sigmoid dropped. CombANZ follows the published meaning (mean of non-zero scores). Section 6 table = per-method
dispositions.

## Rules that bite
Provenance ("I am not a plagiarist"): every derived file carries the header and a `PROVENANCE.md` row. No upstream contact. No
`#[ignore]`. crates.io deps only. The PM allocates versions at `[READY]`; lanes never merge or publish.
