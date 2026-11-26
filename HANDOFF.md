# sortant HANDOFF

You are the `sortant` lane (cwd `C:\Projects\sortant`, repo `ThinkersJournal/sortant`). Rows and statuses: `ROADMAP.md`.
Charter (PM-written, outside the repo): `C:\Projects\.lane-state\newlane\sortant.md`; rankops inventory:
`...\sortant-rankops-inventory.md`. Work in a worktree `C:\Projects\sortant-wt-<topic>`, never branch in the primary.
Origin pins, rules and the defect list: `PROVENANCE.md`, `NOTICE`, `docs/CONTRACT.md` (decisions D1-D17 plus
amendments A1-A4, all approved by the PM; they are the spec, do not re-derive them).

## State (2026-11-26; cluster PM = pm-ants; [READY] goes to pm-assist cc pm-ants)
- main = `d349dff` (#8 merged: old f32 API removed, patent sweep partial, portfolio pin 467fd25). 0.2.0 breaking, not yet published.
- Branch `patent-sweep` (worktree `C:\Projects\sortant-wt-patent`), no PR yet; one PR per day, next window 2026-11-27.
- Patent sweep (row 6) partial: `docs/PATENT-SWEEP.md`; US7188106B2 wants a second reader. Never "clear".
- Open decisions awaiting PM objection: ISR keeps `1/sqrt(k+pos)`; MedianRank penalty `longest+1`; DBSF/Standardized constant list at std <= 1e-9; two `Ranked::contributions` mutants time out.
- Not yet read: advisory `portfolio` job result at the new pin (read by check name; disposition any new Medium).
- No WAIT lines.

## Next (in order)
1. Patent sweep: unread hits, a second source (USPTO/Espacenet), close row 6.
2. Row 5: docs/examples passing the ease gate; extra wrappers only if wanted.
3. Re-run `cargo mutants --file src/fuse.rs` after any `fuse.rs` change.

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
