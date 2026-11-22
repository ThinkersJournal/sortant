# sortant HANDOFF

You are the `sortant` lane (cwd `C:\Projects\sortant`, repo `ThinkersJournal/sortant`). Rows and statuses: `ROADMAP.md`.
Charter (PM-written, outside the repo): `C:\Projects\.lane-state\newlane\sortant.md`; rankops inventory:
`...\sortant-rankops-inventory.md`. Work in a worktree `C:\Projects\sortant-wt-<topic>`, never branch in the primary.
Origin pins, rules and the defect list: `PROVENANCE.md`, `NOTICE`, `docs/CONTRACT.md` (decisions D1-D17 plus
amendments A1-A4, all approved by the PM; they are the spec, do not re-derive them).

## State (2026-11-22; cluster PM = pm-ants; [READY] goes to pm-assist cc pm-ants)
- main = `37189d0`, version 0.1.0. Stage 1 (#6) and Stage 2 (#7) merged: every method in the CONTRACT section 6 table is in
  `sortant::fuse` (AdditiveMultiTask = weighted CombSum/Standardized, no variant); wrappers for rrf/borda/weighted/
  combsum/combmnz only.
- Branch `old-api-removal` (worktree `C:\Projects\sortant-wt-oldapi`), pushed, head `72f30bf`, NO PR: removes the old f32 API and
  its derived files, ports the dbsf regression, updates README/ROADMAP/PROVENANCE/CHANGELOG (changelog heading says 0.2.0,
  pending the gate's number). 100 unit + 20 differential + 4 doc tests pass, clippy and doc clean. Dropped with no
  replacement: explain, analyze_consensus, attribute_top_k, output validators (CONTRACT section 6 says drop/decide).
- One PR per day: #7 merged 2026-11-22, so open the PR for `old-api-removal` at/after 2026-11-23 00:00Z; pm-ants wakes this
  lane by mail then. Keep committing on the branch meanwhile.
- Patent sweep (row 6) is partial: `docs/PATENT-SWEEP.md`; claims unread (Google Patents 503).
- Open decisions I made, awaiting any PM objection: ISR keeps rankops `1/sqrt(k+pos)`; MedianRank penalty `longest+1`
  (0-indexed); DBSF/Standardized constant list at std <= 1e-9; two `Ranked::contributions` mutants time out (cause unread).
- No WAIT lines.

## Next (in order)
1. Open the `old-api-removal` PR when the day rolls over; `[READY]` to pm-assist cc pm-ants.
2. Patent sweep: read claims verbatim (USPTO/Espacenet), positive controls, update `docs/PATENT-SWEEP.md`.
3. Row 5: docs and examples that pass the ease gate; wrappers for the remaining methods only if wanted.
4. Re-run `cargo mutants --file src/fuse.rs` after any `fuse.rs` change.

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
