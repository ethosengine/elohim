---
id: "backlog-crates-seam-contracts-ci-pipeline-coverage-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "crates/seam-contracts has a local pre-push gate but no dedicated CI pipeline backstop (test/clippy/fmt/wasm32 coverage is push-time-only, bypassable)"
slug: "crates-seam-contracts-ci-pipeline-coverage-gap"
written: "2026-08-02"
author: "fix-integration (seam-concern architecture sprint, Wave A)"
status: "backlog"
priority: "medium"
tags: [crates, seam-contracts, ci-coverage-gap, pre-push-gate, orchestrator, build-manifest, concern-canon]
cites:
  - .husky/pre-push.bash
  - crates/seam-contracts/Cargo.toml
  - elohim/holochain/build-manifest.json
  - steward/device/build-manifest.json
  - genesis/data/timeline/backlog/elohim-sdk-sync-full-features-broken.md
---

# crates/seam-contracts CI-execution coverage is partial

## The gap

`crates/seam-contracts` (`elohim-seam-contracts`, added 2026-08-02) is the executable
concern-canon contract surface for the protocol — `Answer<T>` (honest absence),
`ReasonLabel` (observability-per-decision), and the `Arbitrated`/`Quiescent` property
harnesses. No `build-manifest.json` anywhere in the repo declares a dedicated pipeline
step for it. Verified (2026-08-02): `find . -iname build-manifest.json` under `crates/`
returns nothing, and `grep -rn crates genesis/orchestrator/graph-walker.mjs` finds no
`crates/` handling at all in the manifest-driven dependency walker.

`crates/**` (a broad glob) IS listed as a source input in two existing manifests:

- `elohim/holochain/build-manifest.json:64` — the `cargo-build-storage` step of the
  **elohim-edge** pipeline (since `elohim-storage` path-depends on `elohim-sdk`, which
  path-depends on `elohim-seam-contracts`).
- `steward/device/build-manifest.json:21` — the **steward** pipeline, same
  transitive-dependency reasoning.

Both are **incidental, transitive, default-features-only compiles** — the crate gets
built as a dependency of a Docker image, nothing more. Neither runs `cargo test -p
elohim-seam-contracts --all-features`, `cargo clippy -p elohim-seam-contracts
--all-features -- -D warnings`, `cargo fmt --check` scoped to the crate, or the
`--target wasm32-unknown-unknown --no-default-features` build that proves the crate's
own documented "leaf crate: zero first-party dependencies, std-only by default" claim
actually holds under a no_std-shaped target.

## What Wave A (this sprint, F4) added — and its limit

A **local pre-push gate** (`.husky/pre-push.bash`, a standalone block modeled on the
existing `elohim/eprfs` gate) now runs the full contract — `cargo test --all-features
&& cargo clippy --all-features -- -D warnings && cargo fmt --check && cargo build
--target wasm32-unknown-unknown --no-default-features` — from `crates/seam-contracts`,
whenever a push range touches `crates/seam-contracts/**`. Verified green against the
current tree (2026-08-02): all four steps pass (`cargo test`: 43 passed, 0 failed;
clippy, fmt, and the wasm32 build all exit 0).

This closes the **local dev-loop** gap but not the CI one:

- It is bypassable — `HUSKY=0 git push` or `git push --no-verify` skips it entirely, and
  a push from any machine without the toolchain (or without the hook installed) never
  runs it.
- It has **no CI backstop.** Contrast with `sweettest-check`, whose CLAUDE.md-documented
  tiering explicitly names "CI's DNA pipeline (`--run-ignored all`) remains the backstop
  either way" when the local gate is deferred — `crates/seam-contracts` has no equivalent
  downstream catch. A `--no-verify` push (or an agent/CI actor that never runs the husky
  hook) can land a broken concern-canon contract with nothing downstream noticing until a
  human happens to run the crate's own tests.

Given this crate is specifically the executable contract OTHER crates are meant to
inherit from (per its own doc comment and `crates/elohim-sdk/Cargo.toml`'s
`elohim-seam-contracts` re-export), a silent break here is higher-stakes than an
ordinary leaf crate — the whole point of the contract is that consumers trust it without
re-deriving it.

## Fix sketch (NOT attempted here — explicitly out of scope for Wave A: "do NOT invent
a new build-manifest.json — that changes orchestrator dispatch")

- Add a dedicated `crates/seam-contracts/build-manifest.json` (or a `crates/` -level
  manifest covering the whole family — `doorway-client`, `elohim-sdk`,
  `elohim-storage-client`, `seam-contracts`) wired into an existing pipeline (or a new
  lightweight one) so the pre-push gate's exact contract also runs in CI as a backstop.
- Cheaper alternative: fold an explicit `cargo test -p elohim-seam-contracts
  --all-features` (+ clippy/fmt/wasm32) sub-step into the existing edge pipeline's
  `cargo-build-storage` step — the crate is std-only with zero first-party dependencies,
  so the added cost is sub-second.
- Either path is an **orchestrator-dispatch change** and needs its own review — this
  entry documents the gap; it does not resolve it.

Status: open, unowned.

## Addendum (2026-08-02, Wave B verification): schema_contract has the same shape of gap

The Wave B behavior-neutrality review confirmed a sibling instance: the P1.4 wire-honesty
guarantees are asserted by `elohim/elohim-storage/tests/schema_contract.rs`, but that
integration target runs only in the local pre-push `just gate` (`elohim/elohim-storage/
justfile` -> `cargo test`) — the edge pipeline's storage quality gate (`elohim/
elohim-storage/Dockerfile` check stage) runs `cargo test --lib` and `schema_contract`
appears in no Jenkinsfile, Dockerfile, or CI script repo-wide. Pre-existing (the file
pre-dates this sprint), but any fix here should cover both: the seam-contracts gate AND
the storage integration-test tier (`schema_contract`, `liveness_contract` filter) need a
CI backstop, not pre-push-only enforcement.
