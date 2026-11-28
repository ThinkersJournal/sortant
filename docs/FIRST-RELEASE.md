# First-release readiness checklist (a check, not a publish)

Asked by pm-ants for the PM, 2026-11-28 (sortant-first-release-readiness). **Nothing is published; `publish = false` stays until
the PM lifts it.** Measured on branch `first-release-readiness` from `origin/main` a0b4b91 (0.2.2), rustc/cargo 1.100.0,
Windows 11, 2026-11-28. PASS / GAP (needs a decision) / FIXED (on this branch, not on `main` until it merges).

| # | Item | Verdict | Evidence |
|---|---|---|---|
| 1 | README hello-world compiled in CI | FIXED | `lib.rs` includes the README as a doctest, so its two code blocks run in `cargo test` (21 doctests, was 19); new CI job `examples run` runs both examples |
| 2 | Fresh-eyes trial (EXPECTATIONS 6.24) | DONE | below |
| 3 | MSRV stated and tested | FIXED / GAP | `rust-version = 1.88`; new CI job `msrv` runs `cargo check --locked` on 1.88 (library only). Measured: lib builds on 1.88; 1.87 fails (`let` chains, E0658), so 1.88 is the true floor. **The tests need 1.89**: dev-dependency `rankops 0.3.0` (and `innr`) declare 1.89, so `cargo +1.88 check --all-targets` is refused. Decision: keep as is (documented in README and CI), or raise MSRV to 1.89 |
| 4 | Licence files and NOTICE | PASS | `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `licenses/rankops-LICENSE-MIT` all in `cargo package --list`; `license = "MIT OR Apache-2.0"`. Provenance rows in `PROVENANCE.md` (earlier work) |
| 5 | Crate metadata | FIXED | `description` was "Planned… no API yet" (stale); now describes the crate. Added `keywords` (rank-fusion, rrf, search, ranking, retrieval) and `categories` (algorithms, science). `readme`, `repository`, `license`, `edition` present. Not set: `homepage`, `documentation` (docs.rs default) |
| 6 | No path or git dependencies | PASS | `[dependencies]` empty; dev-deps `proptest 1.9`, `rankops =0.3.0`, both crates.io; `deny.toml` denies unknown registries and git |
| 7 | Latest version of each dependency | GAP | `proptest`: lock has 1.12.0 = latest (`cargo search`). `rankops`: pinned `=0.3.0`, **latest is 0.5.1**. The pin is deliberate (it is the differential-test oracle, and the code derives from 0.3.0), but it conflicts with the "latest at all times" rule: PM/CireSnave to rule on an exemption or an upgrade |
| 8 | Packaging | PASS | `cargo package --locked` (with `publish` flipped locally, not committed) packaged 34 files, 63.5 KiB compressed, and verified the build. It also ships `HANDOFF.md`, `ROADMAP.md`, `tools/`, `.github/` (no `exclude` set; a call) |
| 9 | cargo-deny | PASS | `cargo deny check`: advisories, bans, licences, sources ok (one warning: unused `Zlib` allowance) |
| 10 | Yank policy | FIXED (proposal) | none existed. Proposed: yank a version only for a build failure, a soundness bug, or a wrong result (a result the contract in `docs/CONTRACT.md` forbids); never for style, MSRV or docs; every yank gets a CHANGELOG line and a fixed release. Needs PM approval |
| 11 | Version | GAP | sortant is `0.2.2` and crates.io holds `0.0.0`; the PM decides the first published number |
| 12 | Patent sweep | GAP (not this task) | `docs/PATENT-SWEEP.md`: every verdict single source; three patents want a second reader. Whether it gates publish is the PM's call |

## Fresh-eyes trial (item 2)

A Sonnet sub-agent (`model: sonnet`) was given only the README and told not to read the repo (one allowance: a path
dependency line, since the crate is unpublished). **Hello-world: ran first try**, printing
`[("doc2", 0.0325…), ("doc1", 0.0322…), ("doc4", 0.0161…), ("doc3", 0.0158…)]`. **The multi-list program was not achievable
from the README alone** (it needed `cargo doc`). Stuck points, all fixed in this PR's README:

- the "more than two lists" line named no method type (the agent guessed `Method::Rrf`; it is `RankMethod::Rrf { k: 60.0 }`)
  and no builder names (`weights`, `veto`...) — now an inline compiled example plus the list of `Options` setters;
- `rrf`'s constant was unstated (now: `k = 60`); `Scored`/`ScoreMethod` were unnamed (now named and explained);
- weights' shape and veto semantics were unstated (now a comment in the example);
- no dependency line for an unpublished crate (now a git dependency line and the MSRV).

Limits: one trial, one agent, a path dependency instead of crates.io; the second program was checked against the
README's new text only by the doctest, not by a second fresh reader.
