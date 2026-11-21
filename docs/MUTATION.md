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
