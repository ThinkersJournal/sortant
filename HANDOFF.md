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
- Still to read: advisory `portfolio` job result at the new pin (check name on the next PR; disposition any new Medium).
- Second patent source deferred (board 511; pm-ants [FIX] 2026-11-26): continue on Google Patents alone, every verdict says "single source: Google Patents", never "clear". Remaining: classic-name queries (item 3), second reader for US7188106B2.
- Branch `patent-sweep` head `df5d7b2` (pushed, no PR): slices = patents read (US12259913, US2025/0094398A1, US5864845A, US10698908B2 dependents), classic-name queries (US6728704B2, US7716202B2 and EP1631901A2 marked needs more, US7962500B2), README quick start + `examples/fuse_options.rs`. All verdicts say single source: Google Patents. The gate opens the PR window; open the PR then (title says patent sweep + docs, single-source label), [READY] to pm-assist cc pm-ants.

WAIT: pr-window | check: test "$(date -u +%F)" \> 2026-11-26 | wake: 2026-11-27T00:05:00Z

## Next (in order)
1. Open the PR at the window (WAIT above).
2. Row 5 remainder: rustdoc example per method; wrappers for the rest only if wanted.
3. Patent sweep: second reader for US7188106B2, US7716202B2, EP1631901A2; second source only if CireSnave picks one (board 511).
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
