---
id: "backlog-security-prometheus-013-protobuf-rustsec-2024-0437"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "RUSTSEC-2024-0437 protobuf 2.28.0 — one prometheus 0.13 declaration keeps it alive in two workspaces"
slug: "security-prometheus-013-protobuf-rustsec-2024-0437"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: security
fingerprints: []
relatedNodeIds: []
tags: [security, rust, cargo, prometheus, protobuf, rustsec, dependabot, cross-workspace]
cites:
  - https://rustsec.org/advisories/RUSTSEC-2024-0437.html
  - elohim/elohim-storage/Cargo.toml
  - elohim/elohim-bitswap/Cargo.toml
  - steward/node/Cargo.lock
  - VULNERABILITY_CLUSTER_07_RUST_STEWARD_NODE.md
  - VULNERABILITY_CLUSTER_08_RUST_STORAGE_RUNTIME.md
  - VULNERABILITY_CLUSTER_09_RUST_DOORWAY_SERVICE.md
  - VULNERABILITY_CLUSTER_10_RUST_ELOHIM_WORKSPACE.md
  - genesis/docs/superpowers/plans/2026-06-17-design-decision-toolkit-plan.md
  - .claude/handoffs/HANDOFF-2026-06-17-doorway-metrics.md
---

## What is deprecated

RUSTSEC-2024-0437 — uncontrolled recursion in the `protobuf` crate's message
parser (`< 3.7.2`); a deeply-nested message can exhaust the stack. Reached in
this repo **only** through `prometheus` 0.13, which vendors `protobuf` 2.28.0
for its protobuf metrics-exposition encoder.

Dependabot tracks it as four alerts, one per Rust workspace:

| Alert | Workspace | Cluster lane | State |
|---|---|---|---|
| #511 | `elohim/elohim-bitswap` | 10 | **resolved** — `prometheus` 0.14, `protobuf` 3.7.2 |
| #482 | `doorway/doorway-service` | 09 | **resolved** — `prometheus` 0.14, `protobuf` 3.7.2 |
| #516 | `elohim/elohim-storage` | 08 | **active** — `prometheus` 0.13, `protobuf` 2.28.0 |
| #742 | `steward/node` | 07 | **active** — inherits `protobuf` 2.28.0 |

## Usage inventory

Three direct declarations exist; only one is still on the vulnerable line:

- `elohim/elohim-bitswap/Cargo.toml:27` — `prometheus = "0.14"` ✅
- `doorway/doorway-service/Cargo.toml:111` — `prometheus = "0.14"` ✅
- **`elohim/elohim-storage/Cargo.toml:186` — `prometheus = "0.13"`** ← the only
  remaining vulnerable declaration in the tree

Resolved `protobuf` per lockfile (verified 2026-07-30):

- `doorway/doorway-service/Cargo.lock` → `3.7.2` only
- `elohim/elohim-bitswap/Cargo.lock` → `3.7.2` only
- `elohim/elohim-storage/Cargo.lock` → `2.28.0` only, reached solely via `prometheus`
- `steward/node/Cargo.lock` → **both** `2.28.0` and `3.7.2`, from
  `prometheus 0.13.4` and `prometheus 0.14.0` co-resident in one graph

### The finding: #742 is not independently fixable, and #516 is one line

`steward/node` declares **no** direct `prometheus` dependency. Its `0.14.0` comes
via `elohim-bitswap`; its `0.13.4` — and therefore the whole of alert #742 — is
inherited from `elohim-storage`. Consequences that neither cluster doc records:

1. **Cluster 07 cannot close #742 inside its own boundary.** No edit to
   `steward/node/Cargo.toml` or a targeted `cargo update` will evict
   `protobuf 2.28.0`; the constraint lives upstream in elohim-storage.
2. **Cluster 08 closes both alerts with one line.** `prometheus = "0.13"` →
   `"0.14"` at `elohim/elohim-storage/Cargo.toml:186`, then re-lock both
   workspaces, clears #516 **and** #742.
3. **The bump is precedent-verified twice.** Clusters 09 and 10 already made the
   identical 0.13→0.14 move: bitswap's locked check passed, and doorway's
   `cargo check --locked --all-targets` passed with no source changes required
   (the `lazy_static!` + `Registry::register` + `TextEncoder` idiom that
   `elohim-storage/src/metrics.rs` uses is unchanged across 0.13→0.14). This is
   the cheapest of cluster 08's 31 alerts, not one of its hard ones.

### Exploitability in context (why this is medium, not high)

`prometheus` pulls `protobuf` only to **encode** the exposition payload we
serve. Nothing in this tree parses attacker-supplied protobuf through that
crate, so the recursion sink is not reachable from the mesh or from
`/metrics` consumers. The alert is real supply-chain debt and should close, but
it is not a live DoS on the fabric — the registry blocker below is an acceptable
wait.

### Stale guidance hazard — three surfaces still say "pin 0.13"

Anyone who picks up the bump will meet instructions telling them not to:

- `elohim/elohim-storage/Cargo.toml:184-185` — "Composes elohim-bitswap's
  prometheus 0.13 + lazy_static idiom". **Factually stale**: bitswap is on 0.14.
  This comment sits two lines above the declaration that needs changing.
- `genesis/docs/superpowers/plans/2026-06-17-design-decision-toolkit-plan.md:35,43`
  — "`prometheus = "0.13"` is already in the workspace (`elohim-bitswap`) …
  Mirror that idiom exactly in elohim-storage."
- `.claude/handoffs/HANDOFF-2026-06-17-doorway-metrics.md:40` —
  `prometheus = "0.13"   # same as storage; do NOT pull 0.14+`.

The "mirror 0.13 exactly / do NOT pull 0.14+" rule was a **consistency**
convention from the 2026-06-17 metrics work, never a resolution constraint —
these are four separate lockfiles with no version unification between them. Two
of the three workspaces have already left it behind. The convention should be
restated as "all four workspaces on `prometheus` 0.14" rather than deleted, so
the metrics idiom stays uniform.

## Migration path

1. **Cluster 08** (owns `elohim/elohim-storage/Cargo.toml` + `.lock`
   exclusively): `prometheus = "0.13"` → `"0.14"` at line 186, and refresh the
   stale mirror comment at lines 184-185 to name 0.14.
2. Re-lock and verify under the storage workspace's **Holochain/WASM**
   `RUSTFLAGS='--cfg getrandom_backend="custom"'` (per root CLAUDE.md — do *not*
   apply the native `RUSTFLAGS=""` override here), with `CARGO_TARGET_DIR` set to
   a pool slot. Confirm `protobuf 2.28.0` is gone from `elohim/elohim-storage/Cargo.lock`.
3. **Cluster 07** re-locks `steward/node` afterwards and confirms `protobuf`
   resolves to `3.7.2` only — closing #742 as a downstream consequence, not as
   its own remediation. This must be sequenced after step 2 and folded into that
   lane's existing shared-lockfile migration handoff.
4. Correct the two stale doc surfaces (plan + handoff) to say 0.14.

## Current decision

**Blocked — not this agent's surface, and the owning lanes are blocked on the
Cargo mirror.** Two independent reasons:

1. **Exclusive ownership.** `elohim/elohim-storage/Cargo.{toml,lock}` is
   exclusively owned by vulnerability cluster 08, and `steward/node/Cargo.lock`
   by cluster 07 (itself frozen behind a concurrent Holochain/package lockfile
   migration, plus a yanked `core2 0.4.0` resolution to repair). A background
   triage run must not edit either. `doorway/doorway-service/*` is likewise
   cluster 09's, and its half of this concern is already resolved there.
2. **Registry unreachable.** Both lanes recorded the same hard blocker on
   2026-07-29: the configured `elohim-mirror` (`nexus.ethosengine.com`) fails DNS
   and its index cache lacks `anyhow`, so even `cargo check --locked --offline`
   cannot resolve these workspaces. No re-lock can be verified until the mirror
   is reachable — and per the hard rules, no closure without a green run.

**The unblocking condition is narrow and worth naming:** a reachable Cargo
mirror. The moment it returns, this is a one-line change with two-workspace
precedent that closes two Dependabot alerts — cluster 08's cheapest win, and it
should be taken first in that lane rather than last.

### Provenance — why this entry exists with no fingerprints

Three sentinel captures pointed here (`929b7f99229f`, `5a3e9e45a634` on
2026-07-29; `0e0f81127d39` on 2026-07-27) and **all three were false positives**
of the remediation-annotation / vendored-clone classes: a `git diff` of cluster
09's own fix comment, a grep of cluster 09's resolved-alerts table, and
freenet-core's mitigation comment inside the gitignored
`genesis/research/repos/` survey tree. Those fingerprints were deleted from the
ledger and structurally collapsed in `.claude/hooks/deprecation-sentinel.py`
(Guards H1/H2/I) so the class cannot cost another dispatch. The *concern*
recorded above was found while scoping them, is genuine, and spans four cluster
lanes — which is why it lives here in the shared projection rather than in any
one `VULNERABILITY_CLUSTER_*.md`.

## Verification

N/A — not yet fixed, and deliberately not attempted from this run. On the
cluster-08 bump, verification = `elohim/elohim-storage/Cargo.lock` carrying
`protobuf 3.7.2` with no `2.28.0` entry, the storage workspace's WASM-flagged
gate green, then the same absence in `steward/node/Cargo.lock` with that lane's
locked check green. Delete this entry when both #516 and #742 close.
