---
id: "backlog-deprecation-iroh-stack-proc-macro-hack-transitive"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Transitive proc-macro-hack (0.5.20+deprecated) inherited via the frozen iroh-blobs 0.94 pin"
slug: "deprecation-iroh-stack-proc-macro-hack-transitive"
written: "2026-06-15"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["5402d986cb56", "edf8f0ff36f5"]
relatedNodeIds: []
tags: [deprecation, rust, cargo, proc-macro-hack, genawaiter, iroh, iroh-blobs, transitive, p2p-iroh]
cites:
  - https://github.com/dtolnay/proc-macro-hack
  - https://crates.io/crates/genawaiter
  - elohim/elohim-storage/Cargo.toml
  - elohim/elohim-storage/Cargo.lock
  - genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md
---

## What is deprecated

```
Downloaded proc-macro-hack v0.5.20+deprecated (registry `elohim-mirror`)
```

`proc-macro-hack` is a build-time-only procedural-macro shim that worked around
Rust's pre-1.45 limitation preventing `#[proc_macro]` invocation in expression
position. That restriction was lifted in **Rust 1.45**, making the crate
unnecessary; upstream (`github.com/dtolnay/proc-macro-hack`) was **archived
read-only on 2022-12-27**. The maintainer publishes the terminal release with a
`+deprecated` build-metadata suffix as the in-band end-of-life signal — surfaced
here by a native cargo `Downloaded …` line while resolving the `p2p-iroh`
feature tree of `elohim-storage`.

No security exposure: it executes only at compile time inside the macro
expansion of its single dependent (`genawaiter`), produces no runtime code path,
and is not a CVE target. The concern is supply-chain hygiene (an archived,
unmaintained crate in the build graph), not a vulnerability.

## Usage inventory

**Zero direct usage.** Neither `proc-macro-hack` nor its sole dependent
`genawaiter` appears in any `Cargo.toml` in the workspace — both are purely
transitive, resolved only in `elohim/elohim-storage/Cargo.lock`. The chain
(verified by reverse-walking `Cargo.lock`):

```
elohim-storage  (feature: p2p-iroh)
└─ iroh-blobs v0.94.0            (Cargo.toml:240, pinned "=0.94")
   ├─ genawaiter v0.99.1         (Cargo.lock:4460)
   │  └─ proc-macro-hack v0.5.20+deprecated   (Cargo.lock:2939)
   └─ bao-tree                   (Cargo.lock:4454)
      └─ genawaiter v0.99.1      (Cargo.lock:556)
         └─ proc-macro-hack v0.5.20+deprecated (Cargo.lock:2955, via genawaiter-proc-macro)
```

- `proc-macro-hack` Cargo.lock declaration: `elohim/elohim-storage/Cargo.lock:7272`
  (`0.5.20+deprecated`, no dependents other than the genawaiter family).
- Its only consumers are `genawaiter v0.99.1` and its sub-crate
  `genawaiter-proc-macro v0.99.1` (Cargo.lock:2939, 2955).
- `genawaiter`'s only consumers are `iroh-blobs v0.94.0` and `bao-tree`
  (a transitive of `iroh-blobs`) — i.e. the **entire subtree exists solely
  because of the `iroh-blobs = "=0.94"` pin**, reachable only when the
  `p2p-iroh` feature is enabled.
- iroh stack resolved: `iroh v0.92.0`, `iroh-blobs v0.94.0`, `iroh-gossip v0.92.0`
  (all `=`-pinned in `elohim/elohim-storage/Cargo.toml:239-241`).

## Migration path

There is no first-party migration: `elohim-storage` does not use the deprecated
feature; the fix must come from upstream dropping it. Two upstream-owned routes,
both out of reach for a bounded background fix:

1. **`genawaiter` drops `proc-macro-hack`.** `genawaiter 0.99.1` (May 2021) is
   the latest published release and still uses the shim; there is no newer
   version to bump to. A fix would require either upstream republishing
   `genawaiter` against native `#[proc_macro]`, or `iroh-blobs`/`bao-tree`
   migrating off `genawaiter` entirely. We do not control either.
2. **Bump the iroh stack.** `iroh-blobs 0.95+` would likely carry a refreshed
   transitive tree — but it moves to a **pre-release crypto path
   (curve25519-dalek 5.0.0-pre.1)**, which is exactly why the pins are frozen at
   `=0.94`/`=0.92` (stable ed25519-dalek 2.2 + curve25519-dalek 4.1). Bumping is
   **explicitly out of scope** (see `Cargo.toml:230-239` and the parallel-stack
   plan) until the crypto path stabilizes.

A `[patch.crates-io]` or pinned `cargo update` of `genawaiter`/`proc-macro-hack`
is not viable — no compatible non-deprecated version exists, so there is nothing
to patch *to*.

## Current decision

**Blocked — inherited transitive, no first-party action.** `elohim-storage` has
zero direct or indirect *source* dependency on `proc-macro-hack`; it is dragged
in solely by the frozen `iroh-blobs = "=0.94"` pin via `genawaiter`. The pin is
deliberately held for ed25519-dalek 2.2 / curve25519-dalek 4.1 crypto
compatibility (iroh-blobs 0.95+ requires a pre-release crypto path), and bumping
it is out of scope. There is no published non-deprecated `genawaiter`, so no
`[patch]`/`update` target exists.

The deprecation is **build-time-only with no runtime or security impact** — it is
supply-chain hygiene noise that we inherit and will shed automatically the next
time the iroh stack can be safely bumped (when iroh-blobs' crypto path
stabilizes off the curve25519-dalek 5.0 pre-release and we lift the `=0.94`
pin). That bump is owned by the iroh-parallel-stack plan, not by this concern.

Ledger status set to `blocked`; the sentinel will cite this decision
deterministically and stop re-dispatching. The stasis sweep re-checks on the
next iroh pin movement.

## Verification

N/A — not fixed (terminal-blocked on an upstream pin we are intentionally
holding). The trajectory that *clears* this: when the iroh-parallel-stack plan
lifts the `iroh-blobs = "=0.94"` pin, regenerate `elohim-storage/Cargo.lock` and
confirm `proc-macro-hack` is gone from the resolved tree
(`grep proc-macro-hack elohim/elohim-storage/Cargo.lock` returns nothing); then
delete fingerprint `5402d986cb56` from the ledger and delete this entry. If the
bump lands but the line persists, the concern reopens against the new resolved
tree.
