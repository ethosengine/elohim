---
id: "backlog-deprecation-serde-yaml-archived-crate-retire"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire archived serde_yaml (0.9.34+deprecated) across the Rust workspaces"
slug: "deprecation-serde-yaml-archived-crate-retire"
written: "2026-06-10"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: medium
fingerprints: ["f55c7497fbcb", "e68f63b8dbf3", "ab7c8b27e94f", "afcf8ef8608a", "a6604ecc351d", "2c2c16622939", "8cc10bf8a03b", "e77871f96415"]
relatedNodeIds: []
tags: [deprecation, rust, cargo, serde_yaml, yaml, dependency-replacement]
cites:
  - https://github.com/dtolnay/serde-yaml
  - elohim/Cargo.toml
  - elohim/eae/src/config.rs
  - elohim/elohim-agent/gate-client/src/dag/content_safety_gate.rs
  - elohim/elohim-agent/gate-client/src/dag/discernment_gate.rs
  - elohim/elohim-agent/gate-client/src/dag/reach_gate.rs
  - elohim/elohim-agent/gate-client/src/dag/universal_band.rs
  - elohim/holochain/tests/manifest-hygiene/tests/manifest_hygiene.rs
  - steward/node/src/pod/decider.rs
  - elohim/constitution/Cargo.toml
---

## What is deprecated

```
Checking serde_yaml v0.9.34+deprecated
```

The crate publishes itself with a `+deprecated` build-metadata suffix on its
final version. Upstream is **archived and read-only** (dtolnay archived
`github.com/dtolnay/serde-yaml` on 2024-03-25; 0.9.34 is the terminal release).
No future fixes — including security fixes — will land. The `+deprecated` suffix
is the maintainer's in-band signal that the crate is end-of-life, surfaced here
by a native `cargo build`/`cargo check` line in the pre-push gate output.

The maintainer provides **no official migration target**. The de-facto community
successors are the maintained forks `serde_yml` and `serde_norway` (and, for
schema-shaped config, moving off YAML to `toml`/`json` entirely).

## Usage inventory

Six crates declare `serde_yaml` as a direct dependency; the API surface is small
and standard (23 call-sites across the tree).

Direct dependents (Cargo.toml):
- `elohim/Cargo.toml:40` — workspace-root dependency (`serde_yaml = "0.9"`)
- `elohim/eae/Cargo.toml:15` — `serde_yaml.workspace = true`
- `elohim/elohim-agent/gate-client/Cargo.toml:18` — `serde_yaml.workspace = true`
- `elohim/constitution/Cargo.toml:14` — `serde_yaml.workspace = true` (**DEAD: declared, never used in src/tests**)
- `elohim/holochain/tests/manifest-hygiene/Cargo.toml:15` — `serde_yaml = "0.9"`
- `steward/node/Cargo.toml:51` — `serde_yaml = "0.9"`

Source call-sites (7 files):
- `elohim/eae/src/config.rs:49,50,54,55` — `from_str`, `to_string`, `Error` (config load/dump)
- `elohim/elohim-agent/gate-client/src/dag/content_safety_gate.rs:106,124` — `from_str` (gate YAML parse)
- `elohim/elohim-agent/gate-client/src/dag/discernment_gate.rs:60,78` — `from_str`
- `elohim/elohim-agent/gate-client/src/dag/reach_gate.rs:70,87` — `from_str`
- `elohim/elohim-agent/gate-client/src/dag/universal_band.rs:84,102` — `from_str`
- `elohim/holochain/tests/manifest-hygiene/tests/manifest_hygiene.rs:39,47,50,87,105,116,299,307,330,333` — `serde_yaml::Value` (dna.yaml/happ.yaml manifest introspection)
- `steward/node/src/pod/decider.rs:36` — `from_str` (pod rules parse)

Distinct API surface (whole tree): `from_str` (×12), `Value` (×8), `Error` (×2),
`to_string` (×1). All four have drop-in equivalents in `serde_yml`/`serde_norway`.

Resolved in 6 Cargo.lock trees (all pinned `0.9.34+deprecated`):
`elohim/Cargo.lock`, `steward/node/Cargo.lock`, `doorway/doorway-service/Cargo.lock`,
`elohim/elohim-storage/Cargo.lock`, `elohim/holochain/tests/sweettest/Cargo.lock`,
`elohim/holochain/tests/manifest-hygiene/Cargo.lock`. (doorway, storage, sweettest
pick it up transitively — likely via a Holochain/test dependency, not a direct
declaration.)

## Migration path

The swap itself is mechanical given the tiny API surface; the operator decision is
*which* successor to adopt and whether to accept a new (forked) supply-chain entry.

1. **Pick a successor** (operator/maintainer call):
   - `serde_norway` — actively maintained fork, closest API parity (`from_str`,
     `to_string`, `Value`, `Error` map 1:1). Lowest-diff path.
   - `serde_yml` — also a maintained fork; has had its own maintenance-churn
     discourse — vet before adopting.
   - Drop YAML for `eae`/`constitution` config and `steward` pod-rules if those
     surfaces could equally be `toml`/`json` (removes a dependency class entirely).
2. **Replace per workspace** (each needs its own gate run with the correct
   `RUSTFLAGS` — see root CLAUDE.md):
   - `elohim` workspace root: change `serde_yaml = "0.9"` → successor; the
     `.workspace = true` consumers (`eae`, `gate-client`, `constitution`) inherit.
   - **Remove the dead `serde_yaml.workspace = true` from
     `elohim/constitution/Cargo.toml:14`** — unused, just delete the line.
   - `steward/node/Cargo.toml` + `decider.rs`: direct swap.
   - `manifest-hygiene` test crate: swap `serde_yaml::Value` →
     successor `Value` (the introspection of `Mapping`/`String` variants is the
     only structural coupling — confirm the successor exposes the same enum shape).
   - Re-point `serde_yaml::` → `serde_norway::` (or chosen) in all 7 source files.
3. **Transitive pins** (doorway/storage/sweettest): these resolve it through an
   upstream (Holochain test stack). A direct-dep swap will NOT clear the
   transitive `+deprecated` line in those three Cargo.lock trees — that subset is
   blocked on the upstream dependency dropping serde_yaml, and is out of scope for
   this concern (track separately if it keeps firing after the direct swap).
4. Regenerate each affected `Cargo.lock`; run each workspace's native gate
   (`cargo build --release && cargo test --lib --bins && cargo clippy -- -D warnings && cargo fmt --check`,
   `RUSTFLAGS=""` for native crates, `RUSTFLAGS='--cfg getrandom_backend="custom"'`
   for the storage WASM workspace).

## Current decision

**Blocked — escalate to an operator-initiated dependency sprint.** Per the
triage hard rules, a fix that *replaces a dependency entirely* and touches
**~19 files** (7 source + 6 Cargo.toml + 6 Cargo.lock, plus the workspace-root
declaration) across **6 crates in 4+ workspaces** exceeds the background-agent
bounded-fix posture. Two reasons make this an operator call, not a background
landing:

1. **Supply-chain policy.** Adopting `serde_norway`/`serde_yml` introduces a new
   forked crate into a tree that already carries an untriaged 191-vuln dependabot
   backlog (`dependabot-triage.md`). Which successor to trust — and whether to
   drop YAML from some surfaces entirely — is a maintainer decision.
2. **Cross-workspace gate cost.** Each of the 6 crates needs an independent
   native/WASM gate run under PVC pressure; this is a coordinated sprint, not a
   single verifiable closure.

The direct-dep swap is *low-risk and ready* once a successor is chosen (the API
surface is just `from_str`/`to_string`/`Value`/`Error`). The transitive subset
(doorway/storage/sweettest) stays blocked on upstream regardless. The dead
`constitution` declaration is a free line-delete to fold into the sprint.

The sentinel will suppress further dispatch on these fingerprints (ledger status:
blocked); the stasis sweep owns the re-check.

**2026-06-14 re-encounter (re-confirmed blocked).** The deprecation surfaced
again as a `Compiling serde_yaml v0.9.34+deprecated` line during the
`elohim-storage` WASM test build (fp `8cc10bf8a03b`), plus a sentinel
self-capture of the triage's own `cargo tree -i serde_yaml` scope-pass output
(fp `e77871f96415`). Both are the SAME concern — the storage workspace was
already documented above as resolving serde_yaml transitively (it has no direct
declaration; its build graph pulls it via the `gate-client`/`constitution`/`eae`
chain — `cargo tree -i` confirms `serde_yaml ← constitution ← {eae, elohim-agent
← gate-client}`). No new usage, no scope change. Folded into this entry's
fingerprint list; no new investigation warranted. Decision unchanged: blocked on
the operator-initiated dependency sprint.

## Verification

N/A — not yet fixed. On the operator sprint, verification = each affected
workspace's native gate green (`cargo test --lib --bins` + `clippy -D warnings` +
`fmt --check`) AND the `Checking serde_yaml v0.9.34+deprecated` line gone from the
direct-dep workspaces' `cargo build` output. Then delete the direct-dep
fingerprints from the ledger and this entry.
