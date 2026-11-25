---
title: "Per-crate key closure gaps, 2026-11-24"
id: gate-attestation-key-closure-gaps
status: draft
class: process-meta
date: 2026-11-24
context-tier: disclosed
steward: human:matthew
author: "Claude (session d6cfb7e1, Opus 5.5, ethosengine device), from the integrator's independent review of the crate-shadow lane; each counterexample checked against the code at the base named below"
serves: [push-gate-pays-once]
tags: [gate, crate-key, closure, evidence, checklist]
graduation-trigger: >-
  Every row is closed by a key change with a test that reproduces its counterexample, or the
  steward rules a row out of scope; then this checklist is retired into the station that consumed it.
cites: []
---

# Per-crate key closure gaps

The crate-shadow lane (branch `cd/crate-shadow`, base dev `9f272d343`) writes one shadow key per
Cargo workspace member (`genesis/orchestrator/crate-keys.mjs`, context from `crateKeyContext` in
`genesis/orchestrator/gate-runner.mjs`). It decides nothing today. Before any per-crate key may
stand in for a recipe run, each gap below must be closed. A gap is a change that alters what a
crate's tests compile or read while leaving that crate's key unchanged, so a reused green would
stand for code that did not run.

The key composes: step name, toolchain (`rustc -vV`), the gate's cargo environment and
`RUSTFLAGS`, the member's git tree, the workspace `Cargo.lock` digest, one digest of reads outside
all members, the member's own resolved features, and its workspace dependencies' keys.

| # | Gap | Counterexample (what moves, what stays) |
|---|---|---|
| 1 | External dependency feature unification | Commit `a2bdce376` (fold-pipeline-v2) adds `tokio` `test-util` to `eprfs-peer`'s dev-dependencies. Under `cargo test --workspace`, Cargo unifies features, so every member that depends on `tokio` now compiles it with `test-util`. `Cargo.lock` does not record features, so only `eprfs-peer`'s key moves; `elohim-epr-cli`'s key stays while its `tokio` changed. Closed in the shadow key by digesting resolved non-member feature closures, test `external feature unification moves every member reaching the package`. |
| 2 | Path dependencies outside the members (vendored Quinn) | `elohim/eprfs/Cargo.toml` patches `iroh-quinn-proto` to `../../vendor/iroh-quinn-proto-0.13.0`. A path dependency has no checksum in `Cargo.lock`, and the vendored tree is no member's tree. Editing its source moves no key, although every member that reaches it compiles new code. Closed in the shadow key by adding reached non-member path trees, test `a non-member path dependency tree moves every member reaching it`. |
| 3 | Workspace-root and tool configuration | The workspace root `elohim/eprfs/Cargo.toml` is in no member's tree. It holds `resolver = "2"`, `[patch.crates-io]` and `[profile.dev.package."*"]`. Changing a profile setting such as `opt-level` or `debug-assertions`, or the resolver, keeps every key. The same holds for `[workspace.lints]`, `.cargo/config.toml` (rustflags, target settings) and `clippy.toml` or `rustfmt.toml` above the members, none of which exist in eprfs today but each of which a later commit could add. Closed in the shadow key by digesting workspace and ancestor configuration object IDs, test `workspace configuration digest moves every step key`. |
| 4 | Recipe command identity | The key hashes the step NAME (`eprfs:test`), not its command. Editing `_gate-eprfs` in the root `justfile`, for example widening `--skip live_corpus_` or adding `-- --ignored`, changes which tests run but keeps every crate key. The project-level key covers the justfile; the crate key does not. Closed in the shadow key by digesting the coarse recipe command files, test `recipe command digest moves every step key`. |
| 5 | Unresolved reads are discarded | `readSet` returns `paths` and `unresolved` (`gate-input-audit.mjs` ~:473). `crateKeyContext` reads only `paths`. A member test whose read the audit cannot name (a computed path) adds nothing to the key. At project level the same unresolved read keys the whole commit tree. |
| 6 | Reads into another member add no edge | `crateKeyContext` drops every read that falls inside any member's directory, on the grounds that member trees are keyed already. If crate A reads a fixture under crate B's directory (for example `include_str!` of a file under B's tests) and A does not depend on B in Cargo, a change to that fixture moves B's key and leaves A's key unchanged. |
| 7 | The clean guard is narrower than the inputs | The guard writes no crate rows unless HEAD is the gated commit and `git status --porcelain --untracked-files=all` is empty for the workspace directory and the member directories. (a) A read outside those directories, which the key takes from the committed object, can be dirty in the checkout the tests ran on. (b) Ignored files (such as a generated file read by `build.rs`) are not listed by `git status`. (c) A submodule checked out at a commit other than its gitlink shows only when the submodule path lies inside the guarded paths. |

## How each row becomes a check

Each row closes with a key change and a test that reproduces its counterexample:

1. A test that adds a dependency feature in one member and expects the key to move for every member
   that shares the dependency, for example by keying on `cargo metadata`'s resolved features of
   external packages too.
2. A test that edits a file under a non-member path dependency and expects the key to move for every
   member that reaches it.
3. A test that edits the root manifest's `[profile]`, `[patch]` or `resolver`, or adds a
   `.cargo/config.toml`, and expects every key to move.
4. A test that edits the recipe's command for one step and expects that step's keys to move.
5. A test with one unresolved read in a member that expects either no crate key for that member or
   a key over the whole tree, the project-level fallback.
6. A test with a cross-member read and no Cargo edge that expects the reader's key to move.
7. Guard tests for a dirty outside read, an ignored input and a moved submodule, each expecting no
   crate rows.

Until all seven rows close, the per-crate key stays shadow-only and decides nothing.
