---
id: "backlog-storage-client-pin-skew-fork-app-interface"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-storage holochain_client pin (=0.9.0-dev.24) skews against the fork's 0.8.3 client — fork app-interface additions unreachable from our own runtime"
slug: "storage-client-pin-skew-fork-app-interface"
written: "2026-08-08"
author: "cartographer"
status: "backlog"
priority: "high"
tags: [elohim-storage, holochain-client, dependency-pins, wire-format, conductor-fork, call-deadline]

relatedNodeIds:
  - "backlog-doorway-client-pin-realign-0-6-x"
  - "genesis/docs/superpowers/specs/2026-08-08-conductor-call-deadline-capability-spike.md"
---

# elohim-storage client pin skew vs the conductor fork's app interface

Surfaced by the T13 conductor call-deadline capability spike (2026-08-08, spec:
`genesis/docs/superpowers/specs/2026-08-08-conductor-call-deadline-capability-spike.md`,
monorepo commit `b5a92c5d3`): `elohim-storage` pins `holochain_client = "=0.9.0-dev.24"`
from crates.io while our conductor fork lineage (`elohim-0.6.3`) ships client
**0.8.3**. The pin carries its own inline warning — raising it re-opens wire-skew
risk against the fork; lowering it re-breaks the admin seam.

**Why it now matters more than a hygiene item:** any capability we add to the
app-interface protocol on the fork — starting with the Stage-1 call-deadline
patch on submodule branch `elohim-0.6.3-call-deadline` (`8ee534862`) — is
**unreachable from our own runtime** until the client families are reconciled.
Only the conductor-config leg of that patch is consumable today. The skew is the
gating dependency between the fork instrument and any fleet benefit from it.

Sibling of the doorway-side realign
(`backlog/2026-08-04-doorway-client-pin-realign-0-6-x.md`, Lane C — doorway
pins the unreleased 0.7 wire family against the 0.6.x conductor). Same disease,
different crate, different write-set (`elohim/elohim-storage/**`).

## Shape of the fix

Bounded but decision-bearing: reconcile `elohim-storage`'s client family with
the fork lineage — either lower to the 0.8.3/0.6.3 stable pairing (and repair
whatever the admin seam needed from 0.9.0-dev, the reason the pin exists), or
consume the fork's client crate directly (path/git dep on the submodule's
`crates/client`), which also makes fork app-interface additions immediately
callable. Needs the compile-driven audit of admin-seam call sites either way.
Do not guess a new pin mid-sprint — this is its own task with its own gate run
(`cargo test`, not `cargo check`, per CLAUDE.md dependency-bump rule).

shift_objective: reconcile elohim-storage's holochain_client pin with the
elohim-0.6.3 fork lineage so fork app-interface capabilities (call-deadline
Stage 1) are callable from the runtime; full cargo test gate; document the
admin-seam constraint that motivated =0.9.0-dev.24.

## Resolution (2026-08-08)

Landed as `473aa9867` (feat/angular22-node24, commit-only pending batch push):
elohim-storage's client family now comes from the conductor fork itself — git
dep on ethosengine/holochain at immutable rev `6d0814266` (client 0.8.3,
elohim-0.6.3 lineage). Option (a) (registry 0.8.3) proved IMPOSSIBLE: the sdk
domain-types crates pin `holo_hash = "=0.6.0"` as a DNA-hash guard, and cargo
unifies any registry 0.6.x-range client's holo_hash into collision with it;
a git-sourced holo_hash is a distinct package and coexists. That — not the
relay_url rename the old pin comment claimed (relay_url is in upstream 0.6.3)
— is the real constraint that motivated `=0.9.0-dev.24`; full three-constraint
analysis now lives at the pin in elohim/elohim-storage/Cargo.toml. Gates:
cargo test EXIT=0 (159 suites) · export_bindings EXIT=0 no TS drift · clippy
EXIT=0 · fmt EXIT=0. Independent review verdict SHIP (zero warnings).
Consuming the fork's call-deadline client leg is now a one-line `rev` bump —
blocked only on `elohim-0.6.3-call-deadline` (or successor) being pushed to
the fork remote.
