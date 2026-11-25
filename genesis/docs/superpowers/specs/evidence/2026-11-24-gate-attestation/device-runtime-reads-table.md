---
title: "What five gates use of device-runtime, against the read they declare"
id: gate-attestation-device-runtime-reads
status: draft
class: process-meta
date: 2026-11-25
context-tier: disclosed
steward: human:matthew
author: "Claude (session d6cfb7e1, Opus 5.5, ethosengine device), for integrator shem; cells gathered by a read-only search agent at dev 6365a4922, the deciding ones re-checked by hand"
serves: [push-gate-pays-once]
tags: [gate, closure-key, read-set, device-runtime, narrowing]
graduation-trigger: >-
  Station Attest-narrow-cargo-reads of the verdict-as-attestation design lands or is ruled out,
  and this table becomes its evidence or is retired with it.
cites: []
---

# What five gates use of device-runtime, against the read they declare

The verdict-as-attestation design marks one gap on station Attest-narrow-cargo-reads: what
`elohim-storage`, `steward-node`, `elohim-ark`, `cargo-bridge` and `jenkins-bridge` actually read of
`elohim/eprfs/device-runtime`. Their keys moved on that crate between roll 2a v3 and the batched fold
(`v3-to-fold.tsv` rows 6, 18, 28, 42 and 61, `MOVED reads(paths:elohim/eprfs/device-runtime)`), so a
two-line change re-ran all five. Read at dev `6365a4922`; nothing here is built or decided, and any
narrowing is integrity-class, a steward ruling.

## Where the whole-crate read comes from

None of the five declares device-runtime at project level, and none has a `// gate-reads:` line for
it. Each one's step `sources` glob covers the crate, for example `elohim/eprfs/**` in the bridges'
manifests. The input audit then scans every `.rs` file under those sources
(`gate-input-audit.mjs` ~:459-467). That includes device-runtime's own integration tests, and one of
them yields the whole-directory read: `tests/manifest_store.rs:103` reads
`Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../sdk/schemas/v1/views/{name}.schema.json"))`.
The real target is the view schemas under `elohim/sdk/schemas/v1/views/`. The audit does not resolve
the `format!` join, so it records the bare crate directory, `elohim/eprfs/device-runtime`, and in the
paths scope a read is keyed by its object id (`gate-evidence.mjs` ~:679-684): any edit anywhere in the
crate moves all five keys. Two other device-runtime tests add narrower reads (`tests/boundary.rs`
:4, :20-23; `tests/session_boundary.rs:4`).

None of the five gates runs those tests: a dependent's `cargo test` does not execute a dependency's
integration tests.

## Facts that hold for every gate

- device-runtime has no `[features]`, no `build.rs` and no `macro_rules!`; its only `cfg` gates are
  test-only.
- Its module graph is coupled: `lib.rs` declares every module, `bin_store` uses `manifest_store` and
  `layout`, which use `bin_store` in turn, and `envelope`, `commit_attribution`, `follow_record`,
  `launch`, `lifecycle` and `session_stasis` use `session`.
- One compile-time read leaves the crate, `include_str!` of
  `genesis/orchestrator/manifests/runtime/workspace-device.manifest.json` (`manifest_store.rs`
  ~:15-17), and all five gates already list it.
- The gate that compiles and tests device-runtime itself is `eprfs` (`_gate-eprfs` in the root
  `justfile`: lint `--all-targets`, `cargo build -p elohim-device-runtime --bin bin-launch`, the ark
  build under `elohim/Cargo.lock`, `cargo test --workspace`). Its sources include `elohim/eprfs/**`,
  so it is selected on any device-runtime change, and it runs the whole workspace
  (`GATE_NARROW=on` is refused).

## Per gate

| Gate | Path to device-runtime | What it uses | Narrower read that would be sound for its tests | What would make that unsound |
|---|---|---|---|---|
| elohim-storage | `elohim/elohim-storage/Cargo.toml:10`, no features | `flock::Flock` (chain write gate, sled reclaim); `manifest_store::{boot_runtime_config, for_runtime_config, observe, discover_runtime_config}`; `workspace_base`, `ManifestStore`, `Projections` in test code | `lib.rs`, `flock.rs`, `manifest_store.rs`, `bin_store/*` without its test files, `layout.rs`, `session.rs`, device-runtime's `Cargo.toml` and `elohim/eprfs/Cargo.toml` | an edit to a module it does not call can still break its build (row A); its lockfile resolves rustix 1.1.3 and chrono 0.4.42 where eprfs resolves 1.1.4 and 0.4.45 (row B); epr-rea's `sidecar` feature is off here and on in eprfs, and storage sets `--cfg getrandom_backend="custom"` (row C) |
| steward-node | `steward/node/Cargo.toml:36` → elohim-storage (`p2p`) → device-runtime | nothing directly; it compiles storage's library and so storage's sites above, without storage's binary | the same set as storage | as storage; its lockfile resolves rustix 1.1.3 and chrono 0.4.44 |
| elohim-ark | `ark/supervisor/Cargo.toml:15`; `ark/cli/Cargo.toml:16` through the supervisor; and a dev-dependency at `ark/cli/Cargo.toml:40` | `bin_store::{Refusal, Registry::discover, Store::from_environment}` (supervisor native.rs); `manifest_store::observe_local` (supervisor.rs); in cli tests `bin_store` ingest and store types and `manifest_store::{workspace_base, registry}` | the same set as storage | rows A and C; its lockfile resolves rustix 1.1.5 and chrono 0.4.42. eprfs's ark build step covers its non-test code under the same lockfile; the cli's test-only uses are not covered |
| cargo-bridge | `bridges/cargo/Cargo.toml:28` → epr-cli (`default-features = false`) → `epr-cli/Cargo.toml:45` | through epr-cli, every public module: the bridge's flow code reaches `session_envelope`, which reaches `session_stasis` and `session_attribution` | all of device-runtime's `src/**` except `bin/bin-launch.rs`, `fork_test.rs` and the test files, plus the two `Cargo.toml` files: in effect only `tests/**` and `seam-registry.yaml` drop out | row A; it can also reach a run-time read of `.epr-meta/elohim/policies/dialogue-stasis-policy.json` (session_stasis policy.rs ~:18), which a narrowed read must keep. Its lockfile matches eprfs exactly |
| jenkins-bridge | `bridges/jenkins/Cargo.toml:29` → epr-cli (`default-features = false`) | as cargo-bridge, through epr-cli's flow and memory modules | the same as cargo-bridge | as cargo-bridge; its lockfile matches eprfs exactly |

## Why a narrowing is sound only under conditions

- **A. The whole crate compiles as a dependency.** A type error in a module a gate does not call still
  breaks that gate's build. A narrowed read is sound only if `eprfs` runs on the same push and its red
  refuses the push.
- **B. Different lockfiles.** eprfs builds against `elohim/eprfs/Cargo.lock`. Storage, steward and
  ark resolve device-runtime's dependencies to other versions, so an edit that compiles against one
  version can pass eprfs and fail them. Only the two bridges match eprfs exactly.
- **C. Features and flags unify differently.** epr-rea's `sidecar` feature is off in the storage and
  steward builds and on in eprfs, ark and the bridges; storage adds a custom getrandom backend cfg.
- **D. Generics compile in the caller.** `ManifestStore::workspace(impl Into<PathBuf>)`,
  `Store::new(impl AsRef<Path>)` and serde's derived code for device-runtime types are instantiated in
  the calling crate, so the caller's own build is the only full check of them.
- **E. Reads outside the crate stay in.** device-runtime's dependencies (ark-core, epr, epr-rea,
  eprfs-core, elohim-settings), `elohim/eprfs/Cargo.toml`, the `include_str!` manifest and, for the
  bridges, the dialogue-stasis policy file.

## What this suggests

For all five gates the read that does not belong is `tests/**` of device-runtime: a dependency's
integration tests, which they do not run, and one of them recorded as the whole crate because the
audit did not resolve its path join. Two changes follow, each smaller than narrowing five gates by
hand, and each integrity-class:

1. The input audit leaves out a dependency crate's `tests/**` (and its `#[cfg(test)]` code) when it
   builds a dependent gate's read set; the crate's own gate keeps them.
2. The audit resolves `CARGO_MANIFEST_DIR`-relative `join(format!(…))` paths to their literal prefix
   (here `elohim/sdk/schemas/v1/views/`) instead of the crate directory, or marks them unresolved.

With (1) alone, an edit to a device-runtime test would stop moving the five keys. Under conditions
A-C, a `src/**` edit should still move them; narrowing below `src/**` is the riskier step, and only
the bridges, whose lockfile matches eprfs, are candidates for it.
