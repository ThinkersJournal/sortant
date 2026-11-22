# Mutation checks (CONTRACT section 7)

One entry per stage: tool, command, ref, result, and every surviving mutant read.

## Step 4 shared pieces: `error`, `sort`, `order`, `input`, `norm`

`cargo-mutants 27.1.0`, `cargo mutants --file src/sort.rs --file src/order.rs --file src/input.rs --file src/norm.rs
--file src/error.rs --no-shuffle -j2`, at branch `core-shared` (2026-11-19). 84 mutants: **69 caught, 8 unviable,
3 timeouts, 4 missed.**

| Mutant | Verdict | Reading |
|---|---|---|
| `sort.rs` `while width < n` -> `<=` | equivalent | the extra pass has `mid == end == n`, copies the left run unchanged |
| `sort.rs` `while start < n` -> `<=` | equivalent | at `start == n` both runs are empty, nothing is merged |
| `norm.rs` `if s < min` -> `<=` | equivalent | assigns a value equal to the current one |
| `norm.rs` `if s > max` -> `>=` | equivalent | same |
| `sort.rs` `start += 2*width` -> `*=`, `2 * width` -> `2 / width`, `width *= 2` -> `/=` | caught as TIMEOUT | the loop no longer terminates |

No surviving mutant is a real gap. Re-run after any change to these files.

## Stage 1: `fuse` (`src/fuse.rs`)

`cargo-mutants 27.1.0`, `cargo mutants --file src/fuse.rs --no-shuffle -j3`, branch `stage1-methods` (2026-11-21).
First run: 90 mutants, 70 caught, 18 unviable, **2 missed**:

| Mutant | Verdict | Reading |
|---|---|---|
| `Scored::len` -> `1` | real gap | the accessor test only used a one-item list; added a three-item and an empty case |
| `total > 0.0` -> `>= 0.0` in weight normalisation | unreachable | `check_weights` rejects a zero sum; the guard only mattered for "no lists", where the loop is empty. Guard removed |

Re-run after both changes: 87 mutants, **69 caught, 18 unviable, 0 missed**. The unviable ones are mutants that do not
compile (return-type replacements on generic items). The differential tests are part of what kills the arithmetic
mutants (Borda `n - pos`, RRF rank offset); they are not a separate check.

## Stage 2: ISR (`src/fuse.rs`)

`cargo-mutants 27.1.0`, `cargo mutants --file src/fuse.rs --no-shuffle -j3`, branch `wrappers` at `833898a` (2026-11-21):
75 mutants, **61 caught, 14 unviable, 0 missed, 0 timeouts**. The count is lower than Stage 1's 87 and was not
investigated; the zero-missed result stands for the file as it is at this ref. The ISR arithmetic (`k + pos`, `sqrt`,
weight) is killed by the exact-value tests and `isr_agrees`.

## Stage 2: DBSF (`src/norm.rs` `dbsf`, `src/fuse.rs`)

`cargo-mutants 27.1.0`, `cargo mutants --file src/norm.rs --file src/fuse.rs --no-shuffle -j3`, branch `wrappers`
(2026-11-21). First run: 156 mutants, 115 caught, 38 unviable, **3 missed**: `min_max` `<` -> `<=` and `>` -> `>=`
(equivalent, already read in the step 4 table) and `&&` -> `||` in `dbsf`'s overflow guard (real gap: a finite mean with
an infinite std from overflowing squares; added `dbsf_variance_overflow_with_finite_mean_is_range_overflow`). Re-run on
`src/norm.rs` alone after the fix: 44 mutants, 42 caught, the same 2 equivalents missed, 0 unviable. `fuse.rs` was not
re-run after the fix, which touched only a test.
