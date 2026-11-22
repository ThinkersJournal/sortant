# sortant HANDOFF

You are the `sortant` lane (cwd `C:\Projects\sortant`, repo `ThinkersJournal/sortant`). Rows and statuses: `ROADMAP.md`.
Charter (PM-written, outside the repo): `C:\Projects\.lane-state\newlane\sortant.md`; rankops inventory:
`...\sortant-rankops-inventory.md`. Work in a worktree `C:\Projects\sortant-wt-<topic>`, never branch in the primary.
Origin pins, rules and the defect list: `PROVENANCE.md`, `NOTICE`, `docs/CONTRACT.md` (decisions D1-D17 plus
amendments A1-A4, all approved by the PM; they are the spec, do not re-derive them).

## State (2026-11-22; cluster PM = pm-ants; [READY] goes to pm-assist cc pm-ants)
- main = `d438d93`, version 0.0.6. Branch `wrappers` (worktree `C:\Projects\sortant-wt-wrappers`), pushed: wrappers
  `rrf/borda/weighted/combsum/combmnz` + example, and Stage 2 through `fuse`: ISR, DBSF, Standardized, CombMAX/MIN/MED/ANZ,
  RBC, Condorcet, Copeland, MedianRank (`src/pairwise.rs`); AdditiveMultiTask decided as weighted CombSum/Standardized (no
  variant). Each has exact tests, a differential test against rankops where correct, and a `docs/MUTATION.md` entry.
  Breaking vs 0.0.6: `ScoreMethod` lost `Eq`; `Source::finish` signature changed (sealed trait).
- PR for `wrappers` opened 2026-11-22 (see `gh pr list --head wrappers`); keep working on the branch until it merges, then
  delete branch and worktree and start a fresh one.
- The old f32 API in `src/lib.rs` still exists; remove it once all methods are in (they now are): its own PR step.
- No WAIT lines.

## Next (in order)
1. Check the `wrappers` PR (threads, checks) and answer findings.
2. Wrappers for the other methods (D1) if wanted; remove the old f32 API per CONTRACT.
3. `PROVENANCE.md` rows for `src/fuse.rs`, `src/norm.rs`, `src/input.rs` etc. if they derive from rankops (only
   `src/pairwise.rs` has one; check each).
4. Row 6 patent sweep.

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
