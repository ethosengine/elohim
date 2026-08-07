---
id: "backlog-deprecation-holochain-sqlite-tombstone-crate"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "holochain_sqlite retired to a compile_error! tombstone at 0.7.0+deprecated — it outranks every 0.7.0-dev prerelease, so any cargo update hard-fails three Rust workspaces"
slug: "deprecation-holochain-sqlite-tombstone-crate"
written: "2026-08-07"
author: "deprecation-triage"
status: "backlog"
priority: "high"
deprecation_status: blocked
severity: high
fingerprints: ["ff2716a33179", "db0d3a82bce6", "1ad08089f71f", "c012bf5384c5", "fc138fb125ab", "2128d54506f4", "517f54ba8954", "6319e4125d1c"]
relatedNodeIds: []
tags: [deprecation, rust, cargo, holochain, holochain_sqlite, holochain_data, semver, dependency-tombstone]
cites:
  - https://crates.io/crates/holochain_sqlite
  - https://github.com/holochain/holochain
  - elohim/elohim-storage/Cargo.toml
  - elohim/elohim-storage/Cargo.lock
  - doorway/doorway-service/Cargo.toml
  - doorway/doorway-service/Cargo.lock
  - steward/node/Cargo.lock
  - genesis/data/timeline/backlog/2026-08-04-holochain-iroh-dep-verification-pack.md
  - genesis/data/timeline/backlog/2026-08-04-conductor-fork-rebase-0-6-3.md
---

## What is deprecated

Captured by the sentinel from a `cargo update` inside `elohim/elohim-storage`:

```
Updating holochain_sqlite v0.7.0-dev.17 -> v0.7.0+deprecated
```

This is **not a soft deprecation**. Upstream published `holochain_sqlite`
`0.7.0+deprecated` (crates.io, 2026-07-10, **not yanked**, crate_size **1,518
bytes**) whose entire library body is a build-breaking tombstone:

```rust
// src/lib.rs — the complete file
compile_error!(
    "the `holochain_sqlite` crate is deprecated and no longer available; use `holochain_data` instead"
);
```

Its own CHANGELOG states the intent plainly: *"Replace the removed
implementation with a deprecated stub that emits a build error directing users
to `holochain_data`."* Description on crates.io: *"Deprecated persistence
abstractions for Holochain state via SQLite."*

### Why this is a landmine rather than a warning

`holochain_sqlite` is **transitive** in every one of our trees — nothing we own
declares it. `holochain_types` requires it (`^0.7.0-dev.19` at
`holochain_types 0.7.0-dev.23`). Under Cargo's semver rules **build metadata is
ignored in version ordering**, so `0.7.0+deprecated` compares equal to plain
`0.7.0` — which outranks *every* `0.7.0-dev.N` prerelease. The tombstone is
therefore the **maximum match** for that caret and is what a default resolve
picks.

The failure mode is maximally confusing: a developer runs a broad `cargo update`
for an unrelated reason and gets a hard build failure inside a third-party crate
they have never heard of, with no connection to their change. Real releases
(`0.7.0-dev.17` … `0.7.0-dev.24`) are ~60–62 KB each; the tombstone is 1.5 KB.
Version-number inspection alone does not reveal the difference — only the size
or the build error does.

## Usage inventory

No first-party source references `holochain_sqlite`. Exposure is purely via the
resolver, and it spans **three native Rust workspaces**:

| Workspace | Lock version now | Pulled in by | Exposed? |
|---|---|---|---|
| `elohim/elohim-storage` | `0.7.0-dev.24` | `holochain_types 0.7.0-dev.23` (`=` pinned in Cargo.toml) | **Guarded** — lock pinned + inline comment |
| `doorway/doorway-service` | `0.7.0-dev.8` | `holochain_types 0.7.0-dev.11` (caret req, unpinned) | **YES — no guard** |
| `steward/node` | `0.7.0-dev.9` | path dep on `elohim-storage` → `holochain_client 0.9.0-dev.13` → `holochain_types 0.7.0-dev.12` | **YES — no guard** |

The fork workspace `elohim/holochain-conductor` is **not** exposed: it consumes
`holochain_sqlite` as a *path* member at `^0.6.3` (`crates/holochain_sqlite`),
never from the registry. `elohim/holochain/tests/sweettest` sits on the `0.6.0`
registry line, below the 0.7 tombstone's reach.

Two unrelated `+deprecated` crates also live in these locks and are **out of
scope here** (they are ordinary archived-crate deprecations, already
canonicalized elsewhere): `serde_yaml 0.9.34+deprecated`
(`deprecation-serde-yaml-archived-crate-retire.md`) and
`proc-macro-hack 0.5.20+deprecated`
(`deprecation-iroh-stack-proc-macro-hack-transitive.md`).

## Migration path

Upstream's named successor is **`holochain_data`**. That crate belongs to the
Holochain **0.7 line**. Our running conductor is the 0.6.3-based fork at
`elohim/holochain-conductor`, and `elohim-storage`'s client pins are an
explicitly documented floor-and-ceiling tuned to that fork's wire surface
(`holochain_client =0.9.0-dev.24`, `holochain_types =0.7.0-dev.23`). Adopting
`holochain_data` means moving the whole holochain family — conductor fork
included — onto 0.7. That is a major-version campaign, not a dependency bump.

Until then the correct posture is **pin to the last real release**:

```bash
cargo update -p holochain_sqlite --precise 0.7.0-dev.24
```

`0.7.0-dev.24` (2026-07-01, 60,431 bytes) is the final functional release and
satisfies the `^0.7.0-dev.19` requirement.

## Current decision

**BLOCKED** — deliberately, and this is the terminal state for automation.

The real resolution (`holochain_sqlite` → `holochain_data`) is gated on moving
the entire Holochain family to the 0.7 line, including the `holochain-conductor`
submodule fork currently rebased to 0.6.3. That is a major-version, cross-
submodule campaign well past the scale a background agent may take
(>20 files, dependency major-version move) and it must not be attempted
opportunistically: `elohim-storage`'s client pins are load-bearing for the
storage↔conductor admin seam, where a wire-shape skew takes down every
`list_apps` / `install_app` / `enable_app` call.

The immediate emission is **already neutralized in the working tree**: the
in-flight admin-seam agent that triggered the capture re-pinned
`elohim/elohim-storage/Cargo.lock` to `holochain_sqlite 0.7.0-dev.24` and left a
TOMBSTONE GUARD comment in that crate's `Cargo.toml`. No fix was owed by this
triage run and none was applied — `elohim-storage`'s `Cargo.toml`/`Cargo.lock`
and `vendor/**` were owned by another agent for the duration.

**The residual risk this entry exists to hold**: `doorway/doorway-service` and
`steward/node` carry the identical exposure with **no guard of any kind**. Their
locks currently hold `0.7.0-dev.8` / `0.7.0-dev.9`; the next broad `cargo update`
in either tree silently lands the tombstone and produces a hard build failure
with no obvious link to the change that caused it.

### Plan sketch for the operator-initiated sprint

1. **Cheap, do first** — replicate the guard outward. Add the tombstone note plus
   a lock pin to `doorway/doorway-service` and `steward/node`. Verify per
   CLAUDE.md: `RUSTFLAGS="" cargo build --release && cargo test --lib --bins &&
   cargo clippy -- -D warnings && cargo fmt --check` for doorway-service; the
   `steward/node/justfile` `gate` target (bin-only crate — `cargo test --lib`
   exits 101) for steward/node. Not landed here: both are outside this run's
   write-set and each needs a full native gate on a workspace this run did not
   otherwise touch.
2. **Consider a durable check.** A resolver-level guard beats three copies of a
   comment. Prefer an existing reader over a new register (see CLAUDE.md on
   growing instruments with no reader) — the natural home is the existing
   pre-push gate, asserting no lock in the tree resolves a crate whose version
   carries `+deprecated` build metadata *and* whose req range still has a real
   release available.
3. **The real fix** rides the Holochain 0.7 campaign, sequenced behind the
   conductor fork's move off 0.6.3. Track alongside
   `2026-08-04-conductor-fork-rebase-0-6-3.md` and
   `2026-08-04-holochain-iroh-dep-verification-pack.md`.

## Verification

Not fixed — nothing to verify as closed. Evidence backing the analysis above:

- Tombstone body read directly from the resolved crate source at
  `/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/holochain_sqlite-0.7.0+deprecated/src/lib.rs`
  (5 lines, one `compile_error!`), against `holochain_sqlite-0.7.0-dev.17`
  (30 `.rs` files, 632 KB unpacked) in the same registry.
- crates.io version API: `0.7.0-dev.17`…`0.7.0-dev.24` are 60,431–62,083 bytes;
  `0.7.0+deprecated` is 1,518 bytes, published 2026-07-10, not yanked.
- Requirement `holochain_types 0.7.0-dev.23 → holochain_sqlite ^0.7.0-dev.19`
  read from the crates.io dependencies API.
- Resolver behavior confirmed empirically, not inferred: the sentinel captured
  the real `cargo update` performing exactly this substitution at
  2026-08-07T01:37:30Z (`ff2716a33179`), the resulting hard build failure
  (`1ad08089f71f`: `error: the holochain_sqlite crate is deprecated and no
  longer available; use holochain_data instead`), and the remediation
  (`fc138fb125ab`: `Downgrading holochain_sqlite v0.7.0+deprecated ->
  v0.7.0-dev.24`). The break and its cure are both on the record.
- Current `elohim/elohim-storage/Cargo.lock` re-verified at
  `holochain_sqlite 0.7.0-dev.24`, checksum `55f6d1677e…`, with no
  `+deprecated` holochain entry remaining.

### Note on fingerprint count

Eight ledger fingerprints canonicalize to this one concern. Only `ff2716a33179`
is an independent signal; the other seven are the sentinel re-capturing the
in-flight agent's investigation and remediation of this same tombstone, and the
guard comment that documents it — including this triage run's own reads. Same
redundant-capture-surface class as
`deprecation-sentinel-redundant-capture-surfaces.md`; they warrant no separate
dispatch.
