---
id: "backlog-task-iroh-user-agent-observed-receipt"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: make the advertised iroh user-agent OBSERVABLE — surface last-seen per-peer userAgent on /p2p/status and render the observed-version matrix, so the advertisement receipt has something to read"
slug: "task-iroh-user-agent-observed-receipt"
written: "2026-09-01"
author: "session-2026-09-01-integrator"
status: "done"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-task-iroh-plane-version-advertisement"
  - "backlog-task-fleet-version-matrix-probe"
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "habit:dataplane-convergence"
tags: [observability, iroh, mixed-version, transport-parity, receipt, delegable]
claimedBy: "codex"
---

**Claimable by any implementation agent. Additive read-only surface + probe —
the wire-level advertisement is landed (task-iroh-plane-version-advertisement,
111/111 green); this task makes what peers LEARNED readable from outside.**

## Why

The advertisement task stores each peer's last-seen `user_agent` on the
`IrohPeerEntry` in the peer book — but the peer book is process-internal.
`/p2p/status` (`http.rs` `handle_p2p_status`, iroh arm ~line 4482) exposes
pull/replication status and `transportPaths`, not peer-book entries, so no
probe, mesh receipt, or fleet read can currently answer "who do YOU see, at
what version?". The fleet-shaped receipt for the advertisement — mixed-version
peers legible without log archaeology, the upgrade-propagation north star's
observability floor — is blocked on this read surface. `version-matrix.ts`
already renders self-reported versions per peer; the observed-by-peers axis is
its missing half.

## P2P design-gate decision

- **Classification:** Ephemeral (C), T2 observability projection — the same
  classification the advertisement task carried. The read surface projects
  the in-memory peer book; rebuilt by the recurring manifest exchange. No DHT
  entry/link, persistent row, head-plane item, coordinator function, or NEW
  HTTP route (an additive field on an existing diagnostics response);
  DNA-hash-neutral.
- **Identity/address:** keyed by the already-resolved iroh `NodeId` string as
  it appears in the book; never joined to `agent_cid` by string equality.
- **Concern canon:** C4 = a peer with no observed userAgent renders explicit
  absence (`null`/dash), never an empty-string fabrication; C5 = the value is
  evidence, never authority — no capability inference; C10 = additive field
  on the JSON response only, no wire type touched. Remaining concerns n/a —
  no election, authority, ingress policy, or persistence.

## Scope

1. `elohim-storage`: expose the iroh peer book's entries on `/p2p/status`
   wherever the book is live (the iroh arm, and the libp2p/dual arm if the
   book handle is reachable there without restructuring): an additive
   `irohPeers: [{nodeId, userAgent}]` (userAgent nullable) array — read-only,
   bounded by the book's existing size. Add a peer-book accessor only if one
   is missing; no locking-model changes.
2. `genesis/a2o/scripts/version-matrix.ts`: an additive `--observed` mode
   that, per peer, also fetches `/p2p/status` and renders the observed matrix
   — rows = observer, columns = observed NodeId (abbreviated), cell =
   last-seen userAgent or dash. `DIVERGENT` marker when observers disagree
   about the same NodeId. Existing default behavior byte-preserved.
3. Mesh receipt: on a running iroh/dual mesh, run the observed mode and show
   every peer reporting every other peer's storage user-agent. Paste the
   matrix into `task-iroh-plane-version-advertisement.md`, advance that atom
   (`wip` → `done` if its own gate legs are green by then), append a one-line
   DELTA (NO status flip) to
   `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`, run
   `.claude/scripts/habits-project.py`.

## Disjointness contract

- MAY edit `src/http.rs` (the `/p2p/status` handler + focused tests only),
  add a read accessor in `src/p2p_iroh/peer_book.rs`, edit
  `genesis/a2o/scripts/version-matrix.ts`, the named backlog atoms, and
  append to the habit ledger.
- MUST NOT edit `src/p2p/transport_manifest_gossip.rs` or any signing/wire
  bytes (the v1 contract is landed and pinned), `src/p2p_iroh/announcer.rs`,
  `src/p2p/gossip_dispatch.rs`, `reconcile_peers` production logic,
  `src/p2p/view_federation.rs`, any Jenkinsfile, deployment manifests, or
  mesh scripts. The libp2p-arm `P2PStatusInfo` struct stays untouched —
  enrich via the JSON-overlay pattern already used for `transportPaths`.

## DoD + verification

- `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --manifest-path elohim/elohim-storage/Cargo.toml --features "p2p p2p-iroh" p2p_iroh; echo "EXIT=$?"` green, including a test that a populated book renders on the status JSON and an empty/absent book renders honest absence.
- `just gate elohim-storage` green, including the default-features build (the
  cross-reference trap: never reference `crate::p2p_iroh` from
  always-compiled code without the feature gate).
- Mesh: `version-matrix.ts --observed` renders the full observer×observed
  matrix with real user-agents on a 3-peer mesh.
- Fleet-shaped confirmation (the same command against alpha peers after the
  next edge roll) remains the integrator's watch — do not claim it here.

## Implementation + household receipt (2026-09-01)

`/p2p/status` now overlays the live iroh peer book as additive
`irohPeers: [{nodeId, userAgent}]`: a live empty book is `[]`, a missing book
omits the field, and an unknown advertised value is `null`. The default
version matrix is unchanged; `--observed` adds an observer-by-observed NodeId
matrix and marks a column `DIVERGENT` only when independent observers disagree.

The three-peer household mesh ran dual transport on every peer. Each observer
reported both remote peers as `elohim-storage/0.1.0`; the dash in each row is
that observer's own diagonal, and no column was divergent:

```text
OBSERVED IROH USER-AGENTS
OBSERVER  2b7e91cd1a7d…         4efd566c9125…         b4046adaf596…
--------  --------------------  --------------------  --------------------
matthew   elohim-storage/0.1.0  elohim-storage/0.1.0  —
jessica   —                     elohim-storage/0.1.0  elohim-storage/0.1.0
james     elohim-storage/0.1.0  —                     elohim-storage/0.1.0
STATUS
```

Verification: the feature-enabled `p2p_iroh` Cargo filter passed with exit 0,
including focused populated/empty/absent projection tests; `just gate
elohim-storage` passed in full, including default-feature compilation and
doctests; Prettier, direct TypeScript checking, and ESLint passed for
`version-matrix.ts`. Fleet confirmation after the next edge roll remains the
integrator's watch and is not claimed here.

## Fleet-partial receipt (2026-09-01)

Edge build `#1410` completed `UNSTABLE` after deploying storage build
`0abe0b344`, which contains the observed user-agent surface; its deploy gate
reported 6/7 storage peers Ready (adam was not Ready) and 0/2 doorways Ready.
The active alpha roster is seven peers: adam, matthew, jessica, james,
gertrude, susan, and eve. A full probe against their canonical cluster-local
service URLs exited 2 from this workspace: all seven DNS names resolved, but
six connections timed out and susan refused the connection. Those Services
are not routable from this execution context, so that run is reachability
evidence, not a version receipt.

The two public storage projections did yield a fleet-partial read. At
`2026-09-01T19:00:12Z`, matthew through `doorway-alpha.elohim.host` and adam
through `elohim.host` each reported all six remote NodeIds, and all twelve
observer→observed cells carried `elohim-storage/0.1.0+0abe0b3`. That is
twelve populated cells: the observers agreed on the five third-party NodeIds
both independently saw, and each also observed the other at the same build.
All storage Services are cluster-internal; the five peers without public
doorway projections cannot be queried as observers from here. This proves
2×6 cells, not the required seven-observer 7×6 matrix; fleet confirmation
remains open and is not claimed.
