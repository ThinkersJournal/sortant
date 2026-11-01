# SortAnt contract note (step 2): options first, then a recommendation

Status: **read and approved by the PM with four amendments (A1-A4, folded in below). D1-D17 and the section 6 table stand.** CombANZ follows the published meaning (average of non-zero scores). Nothing here needs CireSnave. Each decision lists the options, then
`Recommend:`. "rankops" means rankops 0.3.0 (`PROVENANCE.md`); line numbers refer to its `src/lib.rs` and come from a
reading of the crate, not from running it. Where a decision is "decided and tested", the test is a row-4 deliverable.

Standard (CireSnave): every kept function documented and tested by us; no doc claim the code does not earn; nothing silent.

## 1. Scope and shape

**D1. API shape.** (a) One free function per method, like rankops; (b) one entry point `fuse(lists, &Method, &Options)`;
(c) both.
Recommend (c): `fuse` is the single implementation path (one place for validation, ordering, `top_k`, explain), and
thin documented wrappers (`rrf(&a, &b)`) exist only for the ease gate (a stranger fuses two lists in three lines).
Wrappers contain no logic, so rankops' pair-versus-multi duplication (rrf, isr, weighted) cannot recur.

**D2. Dependencies and safety.** Zero required dependencies; `#![forbid(unsafe_code)]`; `serde` optional and later.
Recommend: as stated; no SIMD, no `unsafe`, no I/O.

**D3. Item identity.** (a) `I: Clone + Eq + Hash` (rankops); (b) `I: Clone + Ord`; (c) `I: Clone + Eq + Hash`, and
ordering never depends on `Hash` or `Ord` of the id.
Recommend (c). Ties are broken by a ranker or by input order (D5), never by hashing the id, so output cannot depend on
the platform's `Hash` byte stream (rankops: FNV over std `Hash`, lines 2024-2044).

## 2. Input model

**D4. Score versus rank inputs.** rankops takes `(id, f32)` for every method, and rank methods read position while
ignoring the score; a caller can pass an unsorted list and get position-based ranks silently (line 791).
Options: (a) one type, document it; (b) two types, `Ranked<I>` (best first, no scores) and `Scored<I>` (id, f64);
(c) one type, always sort by score descending first.
Recommend (b). A rank method accepts only `Ranked`, so it cannot be fed a score it would ignore. Converting a `Scored`
list to `Ranked` is an explicit call that sorts by the total order (D5), so the conversion's tie rule is visible.

**D5. Total order (the central rule).** Options for equal fused scores: (a) hash of id (rankops); (b) id order;
(c) score descending, then a designated tie-break ranker, then stable input order.
Recommend (c), as the charter says: sort key is (score descending by `total_cmp` after validation, so no NaN, see D9, and after canonicalising `-0.0` to `+0.0`, see A2;
then position in the designated tie-break ranker, absent items last; then first appearance, scanning lists in the order
given and items in list order). The tie-break ranker is a `Ranked` list or a comparator `Fn(&I, &I) -> Ordering` supplied
in `Options`. Absent both, step two is skipped. Every public ordering, including explain, goes through this one
function.
*Amendment A2 (PM, approved).* `f64::total_cmp` orders `-0.0` below `+0.0`, so scores `0.0` and `-0.0` would not tie. Validation (D9) rewrites every `-0.0` score to `0.0` before anything is computed or compared. Test: two lists whose fused scores are `0.0` and `-0.0` tie and are ordered by the tie-break ranker, then by input order; the output score is `+0.0` (asserted by bit pattern); a `-0.0` input to every method is covered.

**D6. Comparator-driven ranking.** SortAnt also ranks raw items, not only fuses lists.
Options: (a) fusion only; (b) `rank_by(items, comparator)` producing a `Ranked` list, with comparator chaining.
Recommend (b): a `Comparator<T>` that composes (`then`, `reverse`, `by_key`) and `rank_by` is a stable sort. Its output
is the input of fusion.
*Amendment A1 (PM, approved).* Since Rust 1.81 `slice::sort_by` may panic on a comparator that is not a total order. The
std docs (checked in the 1.99.1 source, `alloc/src/slice.rs`) say: *"If the comparison function `compare` does not
implement a total order, the function may panic; even if the function exits normally, the resulting order of elements in
the slice is unspecified."* and, under Panics, *"May panic if `compare` does not implement a total order, or if `compare`
itself panics."* So "no UB, unspecified order" was outdated. Options: (a) call std sort on caller comparators and state a
panic exception; (b) sort caller comparators with our own safe, stable merge sort that never panics on an inconsistent
comparator (its order is unspecified but deterministic). Recommend (b): about thirty lines under `forbid(unsafe_code)`.
Std sort stays allowed only for orders we define ourselves (D5 keys on validated floats are a total order).
Test: an intransitive comparator (`a < b < c < a`) and a non-reflexive one return `Ok` without panic, the same output on
every run, and a permutation of the input (exact multiset equality asserted).
The sort has its own tests (PM condition 1): stability on equal keys; an inconsistent comparator terminates without panic;
empty and one-item input; a few thousand items compared with std `sort_by` on a *total* order as the oracle (exact
equality of output); and a recorded mutation check over the sort, with surviving mutants read.
The one remaining exception to "never panics" is a comparator closure that itself panics: that propagates, and the doc says so.

**D7. Veto and Force.** Callers want "this item must not appear" and "this item must appear first".
Options: (a) infinities in scores; (b) a typed entry `Entry::{Score(f64), Veto, Force}` inside a list; (c) `veto` and
`force` sets in `Options`.
Recommend (c), with an entry-level variant added only if a use case needs a per-list veto. Infinities are refused
(D9). Rules: an item in `veto` is removed from the output; an item in `force` is placed before all unforced items,
ordered among themselves by D5; an item in both is vetoed, and the result reports the conflict in a
typed field, not by silence. `top_k` applies after forcing, so forced items consume slots (decided, tested).

## 3. Numbers and errors

**D8. Float type.** (a) f32 (rankops, all of it); (b) f64; (c) generic.
Recommend (b) f64 only. f32 sums over long lists lose precision (rankops accumulates in f32, line 1680); a generic
parameter doubles the test matrix for no stated user.

**D9. Non-finite scores.** rankops differs per method (inventory section 4): NaN poisons min-max, is neutralised in
z-score, is ignored by `max`, and sorts first under `total_cmp`.
Options: (a) per method as now; (b) reject: a typed error naming the list and index; (c) drop the item; (d) treat as
absent.
Recommend (b), in one validation pass before any method runs, so behaviour cannot differ by method. Documented:
dropping or defaulting is the caller's decision, not ours.

**D10. Duplicate id inside one list.** rankops: RRF, ISR, Borda, CombSUM add per occurrence; CombMNZ counts occurrences
as lists (an inflated multiplier); Condorcet and Copeland keep the last; median rank pushes extra ranks.
Options: (a) error; (b) first wins; (c) sum.
Recommend (a) `DuplicateId { list, index }`: no option is right for every caller, and one error removes seven
different silent behaviours. This also fixes the CombMNZ defect by construction.

**D11. `k` and `top_k` zero.** `top_k = 0` is a well-defined request for no items: return `Ok(empty)`, documented.
RRF's `k` is a smoothing constant: `k` not finite or `<= 0` is `InvalidConfig` (rankops: doc says panic, code returns an
empty `Vec`, constructors assert). Recommend both as stated; no function panics on any input, with the one exception named in A1 (a caller closure that itself panics). Tested by a fuzz-style
property test over the whole input space, including empty lists, empty list sets, and inconsistent comparators.

**D12. Errors.** One `#[non_exhaustive] enum FuseError` with data-carrying variants (`NonFiniteScore`, `DuplicateId`,
`InvalidWeights`, `InvalidConfig`, `LengthMismatch`, `RangeOverflow` and the like), `Display` and `std::error::Error`. No `String`
payloads that callers must parse. No empty-`Vec`-on-bad-input anywhere (rankops: lines 781, 833, 1014, 1159, and the
explain functions).

**D13. Weights.** Options: (a) as rankops (two-list weights, replaced by `1/n` for 3+ lists, ignored by
`additive_multi_task_multi`); (b) one weight per list, any number of lists, validated.
Recommend (b): a weight is finite and `>= 0`, the sum is `> 0`, the count equals the list count, otherwise
`InvalidWeights`. A weight is never replaced or ignored. A zero weight is allowed and contributes nothing.

## 4. Determinism

**D14. Platform-independent output.** Required: the same input gives bit-identical output on every platform and Rust
version we support. Mechanisms: no `HashMap` iteration order reaches output (a `HashMap` may index, never order);
accumulation order is fixed (list order, then item order); only `+ - * /`, `sqrt` and comparisons on f64 in default
methods (IEEE-754 exact); no `mul_add`, no `exp`/`ln`/`powf` (their last bit varies by libm).
*Amendment A4 (PM, approved).* `powf`/`powi` are not guaranteed bit-stable, so no method uses them. Transcendentals in
the method set, decided now: **RBC** needs `p^rank`: computed by repeated multiplication along the ranked list
(`w_0 = 1`, `w_{r+1} = w_r * p`, then times `(1 - p)`), in that fixed order, documented. **ISR** needs a square:
`x * x`, exact and stable. **DBSF, Standardized** need `sqrt`, which IEEE-754 requires to be correctly rounded, so it is
allowed. **RRF, Borda, Copeland, Condorcet, median rank, CombSUM/MNZ/MAX/MIN/MED/ANZ, weighted, additive** use only
`+ - * /` and comparisons. Any method added later that needs `exp`, `ln` or `powf` is decided at its own stage and may not
merge without a recorded rule. Test: golden bit patterns for RBC at ranks 0, 1, 100, and 1000.
Consequence: rankops' `Sigmoid` normalisation uses `exp`. Recommend dropping it, or keeping it behind a documented
"not bit-reproducible across platforms" flag. Recommend dropping; no stated user.
Test: golden outputs asserted as exact bit patterns, run in the 3-OS CI matrix that exists.

**D15. Ranks.** Rank is `usize`, converted to `f64` only for arithmetic; the `rank as i32` wrap (line 3072) is removed.
Inputs longer than `2^53` items cannot occur in memory we address, so conversion is exact; the doc says so.

## 5. Normalisation

**D16. Constant or single-item list under min-max.** rankops returns `(1.0, 0.0)`, so the raw score passes through,
contradicting its doc `(0.0, 0.0)` and "normalised to [0,1]" (line 2189-2205).
Options: (a) raw score (as now, fix the doc); (b) 0; (c) 0.5; (d) error.
Recommend (c) 0.5: the list carries no ordering information, so it contributes the neutral midpoint, always inside
[0,1], and the doc claim is true. Same rule for z-score variants (std below epsilon gives the neutral value); the
neutral value is named in the doc of each method.

*Amendment A3 (PM, approved).* "Constant" means exact equality, `max == min`, never an epsilon. Min-max is computed as
`(s - min) / range` with `range = max - min`, never as `s * (1 / range)`: for a subnormal `range`, `1 / range` is infinite
and `0 * inf` is NaN. Near-constant lists (range positive but tiny) are normalised normally by that formula; the doc
says what that does: a spread that is numerically negligible is stretched to [0, 1], so tiny differences in a list
decide the result. If `max - min` overflows to infinity (values near `f64::MAX` of opposite sign), the list is rejected with the typed
error `RangeOverflow { list }`, not silently clamped. Tests: exact-equality constant
list gives 0.5; `[0.0, 5e-324]` (subnormal range) gives `[0.0, 1.0]` with no NaN; `[-f64::MAX, f64::MAX]` gives
`RangeOverflow`; a list one ulp from constant is not treated as constant.

**D17. One normaliser.** `normalize_scores` is reused by every method that normalises; the inline copies of min-max
disappear. Quantile and Rank ties get the same rank for equal scores (rankops: input order, so tied inputs differ).
Recommend as stated.

## 6. Decision record for every method (rankops 0.3.0 variant -> SortAnt)

"Port" = reimplement under D1-D17 with value-asserting tests and a differential test against rankops where its
behaviour is correct. "Decide" = rankops behaviour is wrong or undocumented, SortAnt takes the stated rule.
Stage = the row-4 order (1 first).

| Method | Input | Disposition | Rule or defect decided | Stage |
|---|---|---|---|---|
| RRF | Ranked | Port | 1-indexed rank; `k` validated (D11); one implementation for pair and multi; weighted variant takes validated weights (D13) | 1 |
| Weighted score | Scored | Port | per-list weights (D13); min-max with D16 | 2 |
| Borda | Ranked | Port | points `n - rank`; absent items get 0 (documented) | 3 |
| CombSUM | Scored | Port | min-max (D16); absent = 0 | 4 |
| CombMNZ | Scored | Port | multiplier = number of lists containing the id; duplicates are an error (D10) | 5 |
| ISR | Ranked | Port | rankops uses a different formula than Mourao et al.; doc states ours; `k` per D11 | later |
| DBSF | Scored | Port | `(s - (mu - 3 sigma)) / 6 sigma` clamped to [0,1], std below epsilon gives 0.5 (D16) | later |
| Standardized | Scored | Decide | `clip_range` validated (`min <= max`, finite): fixes the `clamp` panic (line 1817) | later |
| AdditiveMultiTask | Scored | Decide | weights honoured for any list count (D13); rankops ignores them (line 1971-1990) | later |
| CombMAX, CombMIN | Scored | Port | NaN cannot occur (D9) | later |
| CombMED | Scored | Port | median of present scores; even count averages the two middle values; documented | later |
| CombANZ | Scored | Decide | average over lists where the score is non-zero, as its doc says (rankops averages all present scores); alternative is to keep the code and change the doc; we follow the published meaning | later |
| RBC | Ranked | Port | `(1-p) p^rank`, 0-indexed, `p` in (0,1) validated | later |
| Condorcet | Ranked | Port | pairwise wins; documented as the simplified method (not Kemeny) | Condorcet stage |
| Copeland | Ranked | Port | wins minus losses; no ties-by-hash | Condorcet stage |
| MedianRank | Ranked | Decide | penalty rank stated in the doc in the same index base the code uses (rankops doc and code disagree by one) | later |
| Sigmoid normalisation | n/a | Drop | not bit-reproducible (D14) | n/a |
| Explain (rrf, combsum, combmnz, dbsf) | n/a | Port as one path | one generic explanation structure, same total order (D5); rankops' explain has no tie-break | with its method |
| `analyze_consensus`, `attribute_top_k` | n/a | Decide | keep only if a user needs them; `attribute_top_k` returns a `HashMap` (random iteration order). Recommend drop for now and re-add on request | n/a |
| Output validators (`validate*`) | n/a | Decide | superseded by typed input validation (D9, D10, D12); keep an output-side checker only for tests | n/a |

Stage 1 is the five methods the charter names first (RRF, weighted, Borda, CombSUM, CombMNZ); the remainder follows the
charter's later stages.

## 7. Testing contract (row 4, stated now so the design is testable)

- Every public function has at least one test that asserts exact values, not only properties.
- Property tests carry fixed golden cases alongside the generated ones; failing seeds are committed as cases.
- A mutation check per stage: a recorded run of a mutation tool over the stage's code, with the surviving mutants read.
- Differential tests run the pinned rankops 0.3.0 as a dev-dependency from crates.io where its behaviour is correct, and
  every place it differs is a named, documented decision from this note, with a test that asserts SortAnt's side.
- NaN, infinity, duplicates, empty input, empty list set, `top_k = 0`, and one-item lists are cases in every stage.

## 8. Questions for the PM: answered

1. D7 sets in `Options`, Veto beats Force, conflict reported in a typed field: confirmed.
2. Drop Sigmoid: confirmed.
3. D16 neutral value 0.5, a deliberate change from rankops for constant lists that the differential test names: confirmed.
4. `Hash`/`Ord` never decide order: confirmed.
5. Nothing needs CireSnave.
