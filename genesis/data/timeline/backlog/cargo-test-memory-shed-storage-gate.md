---
id: "backlog-cargo-test-memory-shed-storage-gate"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The elohim-storage gate's cargo build phase peaked at 15.4 GB and was shed by the workspace RAM guard six times — capped to 5.5 GB by a manifest-declared CARGO_BUILD_JOBS, the new run.cargo.env key"
slug: "cargo-test-memory-shed-storage-gate"
written: "2026-09-08"
author: "sprint 2026-09-08 T8"
status: "resolved"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [gate, cargo, ram-guard, orchestrator, build-manifest, velocity]
---

**Concern.** `just gate elohim-storage` — the gate the `dataplane-convergence` habit's checks
run — was killed by the workspace RAM guard six times on 2026-09-05/06 (`cargo test` at
7.6–14.5 GB, `cargo clippy --features p2p p2p-iroh` at 12.6–13.7 GB; `/projects/.claude-config/ram-guard/events.jsonl`).
Every shed presents as `terminated … by signal 15` and costs a whole gate cycle. The
crate's justfile already carried a heuristic brake (`cargo-jobs-flag` emits `-j1` when the
guard reports under ~14 GB of headroom), but it fires only *after* the machine is already
loaded — so a gate started on a quiet machine still climbed to the shed line and died there.

**Measured 2026-09-08.** All runs on the `sprint` cargo-pool slot
(`/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev` — `detect_family()`
reads the *branch*, so a worktree on `sprint/*` never touches the `dev` slot), one at a time
under the `cargo` berth, `RUSTFLAGS=--cfg getrandom_backend="custom"`. Peak is the summed RSS
of the whole cargo+rustc process group sampled at 2 s, not `/usr/bin/time -v`'s
largest-single-process figure.

| # | phase | knob | wall s | peak tree GB | largest single rustc GB |
|---|---|---|---|---|---|
| warm | `cargo test --no-run`, feature-set switch | `CARGO_BUILD_JOBS=4` | 462.4 | 15.69 | 8.47 |
| a | `cargo test --no-run` after `touch src/lib.rs` | `CARGO_BUILD_JOBS=4` | 430.2 | **11.62** | 6.13 |
| b | `cargo test` (run phase only) | `RUST_TEST_THREADS=4` | 303.5 | **1.76** | 1.70 |
| c | `cargo test --no-run` after `touch src/lib.rs` | `CARGO_BUILD_JOBS=2` | 255.1 | **9.61** | 5.27 |
| d | `cargo test` (run phase only) | `RUST_TEST_THREADS=2` | 338.1 | **1.71** | 1.70 |
| c2 | `cargo test --no-run` after `touch src/lib.rs` | `CARGO_BUILD_JOBS=1` | 491.5 | **5.28** | 5.27 |

**Which phase peaks, and which knob moves it.** The peak is entirely the build/codegen
phase; the test-run phase is 1.7 GB and does not move with `RUST_TEST_THREADS` (1.76 → 1.71 GB
for 4 → 2 threads, while costing +11 % wall on that phase). `CARGO_BUILD_JOBS` is the only knob
that moves the peak, and it moves it linearly in concurrent rustc: 15.4 GB at cargo's default
(24 on this host) → 11.6 at 4 → 9.6 at 2 → 5.3 at 1. The floor is 5.27 GB, the single rustc
that compiles the storage lib itself — so `jobs` alone reaches the target and
`-C link-arg=-Wl,--no-keep-memory` was never needed and was not added.

**Full-gate runs** (`migration-hygiene` + `fmt-check` + `clippy` + `test`, clippy artifacts
warm, `touch src/lib.rs` before each so every run does the same relink):

| run | `CARGO_BUILD_JOBS` | wall s | peak tree GB | RAM-guard shed lines |
|---|---|---|---|---|
| baseline (uncapped) | cargo default | 813.9 | **15.42** | 0 |
| v1 | 1 (manifest) | 892.6 | **5.49** | 0 |
| v2 | 1 (manifest) | 948.9 | **5.42** | 0 |
| v3 | 1 (manifest) | 893.0 | **5.29** | 0 |

Peak **15.42 GB → 5.40 GB mean (−65 %)** for **+12.0 % wall-clock** (813.9 → 911.5 s mean;
median 893.0 s, +9.7 %), inside the 15 % budget. v2 alone ran +16.6 %, which is run-to-run
variance on a shared workspace, not a cost of the cap — v1 and v3 are the same configuration
6 % apart from each other, so a single-sample baseline is worth about ±8 %. The RAM-guard
shed ledger gained zero lines across all six measured runs plus the three verification gates
(76 lines before and after). The uncapped baseline survived only because the workspace sat at
8.2 GB committed that hour: 8.2 + 15.4 = 23.6 GB against a 24.8 GB shed line. With the
household mesh up (~9–13 GB committed) the same gate crosses it — which is exactly the six
recorded sheds.

**Cure (landed).** `run.cargo.env`, a new optional `string→string` map in
`genesis/orchestrator/manifest.schema.json` under `run.cargo`. `gate-runner.mjs` serializes it
into a single `GATE_CARGO_ENV` variable in the child environment — the positional contract to
`run-local-gate.sh` stays at exactly four cargo args, which every caller depends on — and
`run-local-gate.sh` parses it, exports each pair, and consumes the carrier before `just`.
`elohim/holochain/build-manifest.json` `gate.elohim-storage` declares
`"env": {"CARGO_BUILD_JOBS": "1"}`. `RUST_TEST_THREADS` is deliberately **not** declared: the
measurement shows it buys no memory and costs wall-clock. Both `just gate` and the pre-push
hook route through `gate-runner.mjs`, so both are capped. The justfile's `cargo-jobs-flag`
heuristic stays as an independent emergency brake for direct `cd elohim/elohim-storage && just
gate` invocations, which do not pass through the manifest.

**Known limit.** The cap is declared per gate project, so it applies to the manifest-routed
gate only. A developer running cargo by hand in the crate gets the old heuristic, not the cap.
If the class recurs on another crate, declare `run.cargo.env` on that project rather than
widening a global.

**Done when:** three consecutive `just gate elohim-storage` runs complete with peak tree RSS
under 6 GB and zero new lines in the RAM-guard shed ledger — met on 2026-09-08 (v1–v3 above).

**Amendment 2026-09-08 (later same day).** `pre-push`'s `rakia-validate` refused
`elohim/holochain/build-manifest.json` — `/gate/projects/elohim-storage/run/cargo must NOT have
additional properties` — once the SOURCE schema in the pinned `elohim/rakia` submodule (not the
`genesis/orchestrator/manifest.schema.json` mirror this backlog item widened) was enforced
against the committed manifest. That schema is operator-owned and could not be widened tonight,
so the cap's declaration home moved: `genesis/agentic/pool-policy.json`'s new
`cargo_env_overrides.elohim-storage` carries `{"CARGO_BUILD_JOBS": "1"}` (already the home of
cargo parallelism policy — `default_jobs`, `max_concurrent_heavy`). `gate-runner.mjs`'s
`gateChildEnv` now merges manifest `run.cargo.env` (if a project ever declares one) ∪
pool-policy `cargo_env_overrides[project]`, manifest winning on a key conflict — so the exact
mechanism this item built (`GATE_CARGO_ENV`, the printed `cargo env:` line) is unchanged; only
where the cap is *declared* moved. `elohim/holochain/build-manifest.json`'s `env` key was
removed to satisfy rakia-validate. The `run.cargo.env` schema key in
`genesis/orchestrator/manifest.schema.json` (~:161-166) and the runner plumbing described above
were deliberately left in place — the manifest key returns as the cap's home the moment the
operator widens `elohim/rakia`'s `GateProject.run.cargo` to accept `env` (see
`genesis/data/timeline/backlog/rakia-executor-untracked-in-submodule-pin.md` for that ask).
Verified: `pnpm run rakia:schema:validate` passes (15/15 manifests); `node genesis/orchestrator/gate-runner.mjs --target elohim-storage --print` prints
`"resolvedCargoEnv":{"CARGO_BUILD_JOBS":"1"}` with no `env` key present in the manifest itself.
