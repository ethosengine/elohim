---
title: Cluster Topology — the live P2P modeling canvas
tier: architecture
status: Living document
created: 2026-06-03
authors: Matthew Dowell + Opus 4.8
pillar coupling: elohim (node/storage), imagodei (recovery), doorway (projection), shefa (devices/resilience observability)
realizes:
  - genesis/docs/content/elohim-protocol/resilience/README.md (the recovery / mutual-aid acceptance test this topology exists to exercise)
informed-by:
  - ./2026-05-02-elohim-hub-boundaries-design.md (the hub/node/storage split this topology deploys)
  - ./2026-05-23-doorway-access-tier-patterns.md (doorway stays web2-only; per-operator projection)
  - ../history/2026-04-19-d1-through-d5-node-and-household-canon.md (D1–D5 node/household/doorway/shem canon)
informs:
  - any sprint claiming "P2P resilience" — acceptance topology must be cross-node, not single-box
  - genesis/orchestrator/data/deployments.json (the per-human deployment records this doc explains)
memory_anchors:
  - project_shem_is_p2p_live_canvas
  - project_household_is_resilience_unit
  - project_doorway_peer_registration
  - project_multi_doorway_human_registration
defers:
  - per-human multi-device topology (one entry per human today; the hub holds the Shamir share)
  - chain-layer consensus mechanics for cross-doorway recovery quorum (see MAP Gap Ledger)
---

# Cluster Topology — the live P2P modeling canvas

The protocol's live test environment is a **multi-node topology, not a single-node
simulation**. a2o scenarios prove *logic correctness*; this topology proves *emergent P2P
behavior* — cross-doorway federation, real DHT gossip, real libp2p latency, real churn as
nodes go offline and come back. **Any feature claiming "P2P resilience" must run in this
topology**; a single-box simulation is not acceptance.

The roster lives in [`deployments.json`](../../../../orchestrator/data/deployments.json) — the
per-human deployment records are the source of truth for placement (`nodeTypes`) and the
archetype's validated resource floor. This doc is the *why* behind the shape; that file is the
*what*.

## Two placement classes

Placement is binary, by `nodeTypes`, and it is a storyline decision before it is a scheduling
decision:

- **Household cluster (on-prem)** — `nodeTypes: [operations, edge, performance]`. **Matthew,
  Jessica, and James** — the Dowell household — each run on a *separate* node on the on-prem
  cluster. They MUST NOT land on shem. This is the always-on leg: **the household carries the
  run when shem is down**, so the three household nodes hold a higher resource floor than their
  raw device-archetype would imply (e.g. James's chromebook-edu archetype is floored at the
  household-member profile precisely because it can't OOM-flap while shem is absent).
- **Remote pool (shem)** — `nodeTypes: [remote]`. **Every non-family persona** (Adam, Eve,
  Pete, Nancy, Gertrude, Susan, Caleb, Daniel, Emma, Terrance, Frank, …) is pinned to shem,
  with **no on-prem fallback**. When shem is down they go `Pending` — this is *intentional
  fail-loud* behavior so degradation is visible rather than masked by silent on-prem
  rescheduling. shem has the compute headroom (>100 GB RAM, ~4 TB storage) to run the full
  persona roster as real, independent peers, each with its own conductor + storage + doorway
  projection.

The two classes form a cross-node P2P environment: household peers federating through Matthew's
doorway, shem personas federating through shem's doorway, DHT gossip and libp2p traffic crossing
between them. That crossing — not everyone on one box — is what makes it a real P2P test.
**shem lighting up the dashboards is the acceptance bar.**

### shem-down peers are HELD, not failed

Because shem-pinned peers are *expected-down* when the remote pool is unavailable, a failing
remote peer is **held**, not a bug. Classify any failing peer by its `nodeTypes` *first*: a
household peer down is a real failure; a remote peer down while shem is unavailable is the
designed state. The CI/seeder consumption gate filters on the substrate signal
(`ELOHIM_REMOTE_COMPUTE_STATUS` / `cluster-state.yaml`) so the static `genesisPeer`/placement
records stay correct as shem comes and goes — `matthew-carries-genesis-alone` is the steady
state while remote compute is unavailable. If a remote-scenario *fails* instead of *skips* when
shem is down, the separation is unwired (the probe is fail-opening on a blind "unknown") — that
is the bug, not the down peer.

## Matthew as doorway operator

Matthew's node runs a **doorway** for the household's peers. His personal identity and his
doorway service share hardware but are **distinct concerns** — his personal cell is not the
doorway service. This split is load-bearing for recovery and identity design:

- When Matthew loses his device, his doorway goes down **separately** from identity recovery.
  Identity recovery routes through a *peer* doorway (e.g. shem's); doorway continuity is an
  operational concern (a node-migration story), **not** an identity concern.
- **Multi-doorway federation is tested naturally**: household peers default to Matthew's
  doorway, shem peers default to shem's. Humans register with *multiple* doorways as a
  resiliency property — cross-federation recovery and gossip exercise both. (Doorway stays
  web2-only — zero per-domain proxy files; routes come from elohim-storage's manifest. See
  [doorway access-tier patterns](./2026-05-23-doorway-access-tier-patterns.md).)

The observability surface is the dashboards (`/shefa/devices`, `/shefa/dashboard`,
`/shefa/resources`, doorway admin) on **both** Matthew's doorway and shem's doorway — both must
light up.

## Why this is the resilience proving ground

The roster is shaped to exercise the [resilience epic](../resilience/README.md), not just to
host peers:

- **Household = resilience unit.** A household with ≥2 peers survives a single-device failure on
  its own — only then can it hold a Shamir share, project content, and stay reachable through
  any single-device chaos. The Dowell household (Matthew/Jessica/James) has this; Susan+Caleb
  (Seattle) and Daniel+Emma (Tulsa) extend the property to non-Dowell households.
- **Multi-region reciprocal backup.** The reciprocal-backup chain spans three independent
  geographic regions — San Antonio TX (Dowell + Gertrude) ↔ Seattle WA (Susan + Caleb) ↔ Tulsa
  OK (Daniel + Emma). Multi-region separation means no single disaster, ISP outage, or power
  failure can take down both halves of any reciprocal pair, and the Shamir t-of-n quorum is
  reachable from any two surviving regions. Remote (shem) placement is the storyline-faithful
  way to put these peers physically distant from the Dowell on-prem hub.
- **Device-archetype diversity inside a household** (home-nuc + recycled-laptop in Tulsa) makes
  "one device shape dies, the other survives" a distinguishable chaos-test surface.

### The canonical recovery demo

Matthew loses his device (and his doorway temporarily). He recovers via **shem's** doorway;
**Jessica + James** (household, on their own nodes) plus **Pete + another shem persona**
authorize via the graduated-recovery quorum. Matthew lands in a hosted cell at shem's doorway
until he restores his own doorway operation. This single demo exercises the full
recovery / sharding / login surface across both doorways and both placement classes — which is
exactly why the topology must be cross-node.

## How to apply

- For any P2P sprint, include a **cross-node activation** deliverable: deploy personas to the
  correct placement class (household-three on-prem, everyone else on shem).
- Acceptance topology MUST include cross-node and cross-doorway flows — never a single-box
  simulation.
- When a peer fails, read its `nodeTypes` before treating it as a regression: remote-down +
  shem-unavailable is held, not failed.
- Treat the `nodeTypes` and resource-floor fields in `deployments.json` as the source of truth;
  temporary bumps must leave a `$comment` naming the archetype floor and the follow-up that
  restores it.
