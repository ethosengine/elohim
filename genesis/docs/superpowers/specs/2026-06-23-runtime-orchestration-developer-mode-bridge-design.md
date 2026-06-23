---
title: "Runtime Orchestration + Developer-Mode Bridge — orchestrate the pods/runtimes like a2o orchestrates activities, feature-gated to drive a simulacrum, re-exposed to human stewards for self-maintenance"
id: runtime-orchestration-developer-mode-bridge-design
status: Draft
written: "2026-06-23"
author: "operator design steer during the alpha CellWithoutGenesis incident (2026-06-22/23)"
domain: "D-identity-sovereignty / peer-runtime"
sprint: vision-deferred
cites:
  - genesis/data/timeline/backlog/peer-driven-dna-repair-update-button.md
  - genesis/data/timeline/backlog/alpha-conductor-cellwithoutgenesis-floating-happ-tag.md
  - genesis/data/timeline/backlog/agent-peer-binding-cross-signed-proof.md
  - elohim/elohim-storage/src/conductor/process_manager.rs
  - elohim/elohim-storage/src/main.rs
  - genesis/docs/architecture/stewardship-over-sovereignty.md
---

# Runtime Orchestration + Developer-Mode Bridge

## The seam (why this exists)
The alpha CellWithoutGenesis incident exposed that **maintaining a node's runtime** (repair a
genesis-less/drifted conductor, update its DNA, change its config) is today an **operator + kubectl**
act — the anti-pattern the protocol's peer-sovereign floor rejects ([[project_hub_optional_floor]],
[[feedback_k8s_is_not_the_architecture]]). The fix isn't one repair endpoint; it's a **runtime-
orchestration control plane**:

> **a2o** scenarios orchestrate *human activities WITHIN* a runtime (login, learn, govern). This plane
> orchestrates the *runtime lifecycle ITSELF* (repair, update, configure, restart) — the same
> orchestration paradigm, a different target. Where a2o drives the **app**, this drives the **pod/node**.

## Invariant: the DNA must be consistent across ALL peers (partition-safety)
Peers on different DNA hashes **cannot communicate** — a DNA hash IS the DHT/network identity (it
covers the integrity zomes + modifiers), and there is **no live cross-version bridge** (Holochain
"bridge calls" are local inter-cell calls, not cross-version network comms; DNA migration/lineage is
a separate, deliberate process). So a hash-moving DNA change is inherently **coordinated, atomic, and
fleet-wide**: roll it incrementally and the old-DNA / new-DNA peers land on different DHTs and
silently partition (CLAUDE.md: "different DNA hashes → different DHTs → P2P partition").

**Therefore the DNA MUST be consistent across all peers** — pinned + rolled atomically (incident 1a),
or baked into the deploy artifact; **never floated per-peer**. The alpha CellWithoutGenesis incident
is the proof: a floating `elohim-happ:dev-latest` let a restarting peer fetch a *newer* DNA than its
installed cell → `CellWithoutGenesis` drift, plus the partition risk of peers on mixed hashes. This
invariant holds **until either**:
1. Holochain ships native cross-version DNA bridges, OR
2. we build the **internal DNA upgrade/update/rollback path** — the runtime coordinating its own
   fleet-wide DNA migration *with lineage* (so it can upgrade/update/rollback itself safely). That
   path is precisely a plane-1 operation of this control plane (see `update`/`rollback` below) — i.e.
   **this design is the eventual mechanism that would relax "pin everything."**

The lone exception is a **coordinator-zome-only** change — it does NOT move the hash, so it hot-swaps
via `update_coordinators` with no partition and no re-genesis (CLAUDE.md). Everything that moves the
hash: pin/bake, never float.

## Three planes
### 1. Orchestratable runtime operations
Node-lifecycle/maintenance actions modeled as **declarative, orchestratable operations** on a node's
own runtime (not imperative kubectl): `repair` (clear genesis-less cell → re-genesis), `update`/`rollback`
(migrate the fleet's DNA to a new pinned bundle — a COORDINATED, atomic, all-peers-together migration
with lineage per the invariant above, NEVER a per-peer floating fetch; this is the "internal DNA
upgrade path"), `restart`, `reconfigure` (the configurable settings
— conductor config, policies, target_arc_factor, the re-seedable policy itself). Each is a P1
reconciliation-controller op the **node drives on itself**; the operator/steward *triggers* it, the node
*performs* it. The repair primitive already exists: `ConductorManager::clear_conductor_state` +
`restart` + the boot reconcile loop (`main.rs`, shipped 2026-06-22).

### 2. Developer-mode bridge (the feature gate)
These operations are **feature-gated behind a developer mode**, not raw production levers. Dev mode:
- **drives a simulacrum** — a sim/test runtime instance the operations can be developed + exercised
  against safely (the runtime-lifecycle analog of how a2o drives fixtures / FixtureHumans), so we build
  + verify the orchestration without re-keying a real node;
- is the **bridge** between dev/test orchestration and production exposure — an operation is authored +
  proven against the simulacrum in dev mode, then graduated to the steward surface (plane 3).
- OPEN: the dev-mode mechanism (a build feature / runtime flag / capability bridge), and what the
  simulacrum substrate is (an ephemeral embedded conductor? a fixture node? reuse a2o's harness?).

### 3. Steward re-exposure (graduation to self-maintenance)
The **same configurable controls** re-expose to the **human operator/steward** who maintains their own
runtime — a household repairs/updates/configures its node from the **peer menu** (imagodei), no operator,
no kubectl. This is the [[backlog-peer-driven-dna-repair-update-button]] "Update / Repair" button,
generalized: the dev-gated operations graduate into steward-facing affordances, gated by an
**agency/auth gradient** (operator on a managed cluster → human steward on a household node → mediated
agency for wards, per `stewardship-over-sovereignty.md` — NB the Identity Ontology Guard: this is
*node-stewardship*, never "sovereign reset").

## First operation: node-repair (op #1, gate-validated 2026-06-23)
P2P design gate result (`.claude/skills/p2p-design-gate`):
- **Classification: Operational (C)** — a runtime command mutating the node's own conductor state; no
  DHT entry, no persistent record, trivially re-runnable. Address: self-action (the node's own
  endpoint), single-target.
- **No coordinator/signal** (not a DHT write). **HTTP: `POST /node/repair`** on elohim-storage (the
  `ConductorManager` lives there as `Arc<Mutex>`), manifest-exposed so doorway routes it.
- **Auth: admin/steward** (operator decision 2026-06-23) — it re-keys, so the entity with stewarding
  authority over the node; the peer-menu button calls it with the steward's session.
- **Re-key gating:** runs on a **re-seedable** node (`GENESIS_SELF_HEAL_IDENTITY` policy); a
  **lineage-bearing** node must community-grounded **migrate** (notarized key-rotation /
  `AgentPeerBinding` lineage proof — blocked, [[backlog-agent-peer-binding-cross-signed-proof]]) →
  the endpoint refuses/guards rather than blind-wiping.
- **Disruptive** (drops conductor connections ~minutes) → return-fast + poll status, not blocking.
- Wiring: thread the `ConductorManager` `Arc<Mutex>` into HTTP app state.

## Open design questions (for the follow-up /plan + per-op p2p-design-gate)
1. The dev-mode mechanism + the simulacrum substrate (plane 2) — the largest unknown.
2. Whether a2o's scenario harness can *also* drive runtime-lifecycle operations (one orchestration
   substrate for activities AND runtimes), or they stay parallel.
3. The operation set + their classifications (repair = C done; update/reconfigure each need a gate pass —
   `reconfigure` of a *notarized* policy may be A/B2, not C).
4. The steward-UI affordances + the agency/auth gradient (operator → steward → mediated/ward).
5. Audit: should runtime operations emit an observation/audit log (Category C `observations`)?

## Status / sequencing
Build is a **follow-up** (operator: "capture now, build as a follow-up"). Op #1 (node-repair endpoint)
is gate-ready to build first; the dev-mode bridge + simulacrum + steward re-exposure are the arc this
doc seeds. Not the incident's immediate fix — the peer-driven boot self-repair (shipped) + the manual
recovery + the hApp pin handle the live outage.
