# sortant HANDOFF

You are the `sortant` lane (cwd `C:\Projects\sortant`, repo `ThinkersJournal/sortant`). Rows and statuses: `ROADMAP.md`.
Charter (PM-written, outside the repo): `C:\Projects\.lane-state\newlane\sortant.md`; rankops inventory:
`...\sortant-rankops-inventory.md`. Work in a worktree `C:\Projects\sortant-wt-<topic>`, never branch in the primary.
Origin pins, rules and the defect list: `PROVENANCE.md`, `NOTICE`, `docs/CONTRACT.md` (decisions D1-D17 plus
amendments A1-A4, all approved by the PM; they are the spec, do not re-derive them).

## State (2026-11-21; cluster PM = pm-ants; [READY] goes to pm-assist cc pm-ants)
- main = `39bb72a`, version 0.0.5. Branch `stage1-methods` (worktree `C:\Projects\sortant-wt-core`) holds Stage 1: `src/fuse.rs`
  (`Ranked`/`Scored`, `Options`, `RankMethod`, `ScoreMethod`, `fuse`), `src/fuse_tests.rs`, `tests/differential.rs`
  (dev-dep `rankops =0.3.0`), mutation entry in `docs/MUTATION.md` (0 missed). 177 unit + 9 differential tests pass, clippy clean.
- The old f32 API in `src/lib.rs` still exists; it goes as each method's stage lands (not yet).
- No WAIT lines (`lane-restart waits --role sortant` finds none).

## Next (in order)
1. Open the Stage 1 PR (one per day), send `[READY]` to pm-assist cc pm-ants; PM allocates the version.
2. Wrappers `rrf(&a,&b)` etc. for the ease gate (D1, no logic) and docs/examples (ROADMAP row 5).
3. Later methods per the CONTRACT section 6 table: ISR, DBSF, Standardized, AdditiveMultiTask, CombMAX/MIN/MED/ANZ, RBC (A4),
   then Condorcet/Copeland, MedianRank; each with exact tests, golden properties, mutation entry, differential where correct.
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
