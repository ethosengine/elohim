---
title: "The workspace as a stewarded device of matthew — native content sync from a peer runtime"
id: workspace-stewarded-device-peer
tier: spec
status: Draft
created: 2026-08-30
maintainers: Matthew Dowell + Claude Fable 5
class: substrate
context-tier: disclosed
sovereignty-frame: descriptive
steward: rust-architect
graduation-trigger: stewarded-device-sync.feature stations 3-4 green on the household mesh and confirmed on alpha (a device-authored declared head served by both doorways with no doorway seed and no Jenkins pass), OR superseded-by-implementation
domain: D2 identity (controllers, device binding) x dataplane sync planes (inventory gossip · acquisition pull · declared head)
habits: [dataplane-convergence]
topic: [stewarded-device, workspace-peer, sovereign-peer, binds-identity, delegates-compute, declared-head, native-sync, ratchet, quiesce, measurement]
cites:
  - genesis/a2o/features/deployment/sovereign-peer-join.feature
  - genesis/a2o/features/auth/stewarded-device-sync.feature
  - genesis/a2o/features/dataplane/resiliency-saga/06-heads-converge.feature
  - "identity-head-key-lineage | the binds-identity controller-set primitive this device binding is the first consumer of — W joins matthew controllers, attribution resolves through the chain-root | sha256:95950b918c8803bc | path: genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md"
  - "stewardship-over-sovereignty | the canon that makes the binding a Commitment (§6 Rule 2 bounded_by) and forbids a self-sovereign apex — W is a device, never a sole controller | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md"
  - genesis/data/timeline/backlog/sovereign-peer-network-read-no-authorities.md
  - genesis/data/timeline/backlog/agent-peer-binding-signing.md
  - genesis/data/timeline/backlog/agent-peer-binding-cross-signed-proof.md
  - genesis/data/timeline/backlog/stewarded-device-sync-feature-authoring.md
  - genesis/data/stories/james-son--as-stewardee--stewarded-device-sync.md
---

# The workspace as a stewarded device of matthew

## Why this exists

Tonight's manifesto update did not reach `elohim.host` because a generated CID twin
drifted and `elohim-genesis #1522` stopped at *Validate Constants*. The fix was one
regenerated file — but the shape of the failure is the point: **a content edit reaches
the public surface only through a Jenkins pass and a doorway seed.** The plane that is
supposed to be authoritative (the p2p dataplane) never got the write; the doorways, which
are only its projection, were where the edit was aimed.

This spec turns that around for the next batch. The workspace — this dev container —
joins the alpha cluster as a peer runtime that is **one of matthew's devices**, so that a
write authored here propagates because the peers recognise it, not because a gateway
ingests it. It is the delivery ratchet run *from inside* the plane:

    localdev (isolated conductor)  →  hybrid mesh (workspace conductor joined to alpha)
                                   →  native sync (a content update is a peer act)

Each rung yields quiesce and propagation numbers measured from a participant's seat
rather than from a CI gate outside the network. The last rung removes the pipeline from
the content path entirely for the surfaces the doorways already expose.

## What already exists (read from the tree, 2026-08-30, HEAD `a17316afc`)

- **Joining is built and green.** `just dev conductor alpha` (`app/elohim-app/scripts/hc-start.sh`
  `join-alpha`, fork iroh pair, bootstrap + signal via doorway-alpha, deployed bundle
  installed for DNA-hash parity). `sovereign-peer-join.feature` scenario 1 is GREEN on the
  T3 rung. The conductor **always mints a fresh agent key** (`hc sandbox generate
  --in-process-lair`; `happ_manager.rs:292-297` `generate_agent_pub_key`) — there is no
  key-import path, and the feature forbids copying a fleet member's key. The workspace
  agent is therefore a NEW `AgentPubKey` **W**, never matthew's key.
- **Reading and being pulled are the RED gaps, already named.** Every live alpha
  agent-info advertises `storageArc: null`, so a joiner's reads find no authority and a
  workspace-authored entry has nobody to be fetched by
  (`backlog/sovereign-peer-network-read-no-authorities.md`). The fleet's storage learns
  ids only from other storages, not from the network
  (`backlog-p1-dht-authored-content-not-projected`). These are the two "(RED — the gap)"
  scenarios in `sovereign-peer-join.feature` and the leg adam was measured failing on
  tonight (42/42 `ContentNotFound` on both transports).
- **Identity: W can be a peer, not yet matthew's peer.** imagodei binds one
  `AgentPubKey` to a `Human` (`AgentKeyToHuman`, self-only, one-to-one, created by the
  caller for itself in `create_human`). No coordinator binds a second key to an existing
  Human. `AgentPeerBinding` (existing entry, `agent_peer_binding.rs:87-97`) binds a
  transport id to an `agent_cid` with a Stage-1 sentinel signature; it is projected to
  `peer_identity_bindings`, the canonical join table (`identity_namespace.rs`).
- **The primitive for "a second key acts for this identity" is specced, not shipped.**
  `binds-identity` is a `Mishpat::Commitment` action discriminator
  (`commitments.rs:618-657` validates `chain_root, head_key, controllers[],
  controller_policy`); the coordinator `bind_identity` is the identity-head spec's open
  graduation trigger. The device binding this spec needs **is that primitive's first
  consumer** — W joins matthew's `controllers` set.
- **Authority is a Commitment, checkable today.** `operation_authorization.rs`
  `authorize_operation(performer, capability)` finds the active `delegates-compute` grant
  and runs the shared 7-check bounds validator (fetch · revocation · window · scope ·
  reach ceiling · rate · key rotation) — fail-closed. Canon requires every witnessable act
  to name its bounding Commitment (`stewardship-over-sovereignty.md` §6 Rule 2).
- **What refuses an unknown key today — and what doesn't.** `classify_reach_authorization`
  (`p2p/reach_authorization.rs:71-101`) refuses `Intimate|Trusted|Familiar|Community`
  writes from a signer with no `peer_identity_bindings` row; `Public|Commons` writes are
  **never gated on signer identity** at Stage 1, and the known-agent check fails OPEN on a
  DB error. So stations 1–2 can run with a throwaway W at commons reach; station 3 is what
  makes the write *matthew's*; station 4 is what makes it *bounded*.

## P2P Design Gate: workspace as stewarded device

### Entity: workspace agent W
- **Classification**: none new — an `AgentPubKey` minted by the workspace conductor's lair. Not an entry.
- **Content Address Strategy**: Holochain agent hash (`uhCAk…`); never a CID, never copied from matthew.
- **Anti-Pattern Check**: identity-ontology guard — W is a *device*, the human's agency is exercised through it; there is no "self-sovereign" tier here and none is introduced. Cross-namespace: W's iroh `NodeId` / libp2p `PeerId` are resolved to `agent_cid` through `peer_identity_bindings`, never string-compared.

### Entity: device binding (W is a controller of matthew's identity head)
- **Classification**: Notarized (A) — the protocol would be lying if "this key acts for matthew" changed silently.
- **Justification**: it is a thing in its own right (a witnessed declaration, revocable, with lineage), not an attribute of the content W later writes.
- **Head-Plane Cost Budget**: 1 item per device per human; ~5 at seed, <20 at 1 yr for the household — trivial; no bundling needed.
- **Network Stakes**: all four stages; the binding itself is **floor-protected** (`Constitutional` — a delegation), never cheapened at Simulacra.
- **Content Address Strategy**: Content-Derived — `cid == entry_hash` of the Commitment; the identity's durable identifier is the **chain-root cid**, not W and not matthew's current key (identity-head spec §2.1).
- **Source of Truth**: Holochain DHT.
- **Integrity Zome + DNA-hash class**: `mishpat_integrity` — **DNA-hash-NEUTRAL**: reuses `Mishpat::Commitment` with the existing `binds-identity` action; the payload's `controllers` gains W and `controller_policy` stays `Steward-set | RecoveryAuthority M-of-N` (community-backstopped by construction). No new entry type, no new link type.
- **Coordinator Zome**: `mishpat::bind_identity(BindIdentityInput) -> EntryHash` (the identity-head spec's unshipped fn — this batch ships it, first consumer). Revocation reuses `revokes-commitment` (`commitments.rs:343`).
- **Projections**: SQLite `mishpat_commitments` (dht_anchor_hash: yes — the existing projection); Automerge sync: no (governance commitments are not content docs).
- **HTTP Route**: none authored — the workspace calls its own conductor over the admin/app websocket. The existing read surface (`did:elohim` head assembly) shows the controller set.
- **Anti-Pattern Check**: caught and corrected — first draft reached for a new imagodei `DeviceBinding` entry + `Human→AgentPubKey` link; rejected because the concept already has a consolidating home (`binds-identity`) and minting a second type on another DNA inverts a paid-for consolidation.

### Entity: transport binding for W (`AgentPeerBinding`)
- **Classification**: Notarized (A) — existing entry type, reused as-is.
- **Head-Plane Cost Budget**: 1–2 items per device (iroh + libp2p ids); negligible.
- **Network Stakes**: all stages; floor-protected (`Constitutional`).
- **Content Address Strategy**: Agent-Scoped Composite `(W, peer_id, device_archetype)`; supersession via `superseded_by`.
- **Source of Truth**: Holochain DHT → `peer_identity_bindings` projection (dht_anchor_hash: yes).
- **Integrity Zome + DNA-hash class**: `imagodei_integrity` — DNA-hash-NEUTRAL (no type change). The signature moves from the Stage-1 sentinel to W's real Ed25519 signature over the binding — the open `agent-peer-binding-signing` backlog, which this batch takes because station 3 is its first real consumer.
- **Coordinator Zome**: `imagodei::create_agent_peer_binding -> EntryHash` (exists; `device_archetype: Steward`).
- **Projections**: SQLite `peer_identity_bindings` (existing, via `ReconcileController::on_agent_peer_binding`); Automerge: no.
- **HTTP Route**: none new.
- **Anti-Pattern Check**: honesty clause applies — the binding stays self-asserted until the cross-signed proof lands (`agent-peer-binding-cross-signed-proof`); **no economic attribution joins through it in this batch.** Attribution of W's writes to matthew resolves through the identity chain-root (identity-head spec §4.2), not through this transport binding.

### Entity: declare-head authority for the corpus (`delegates-compute` grant)
- **Classification**: Notarized (A) — existing action; the grant is adam (genesis corpus steward) → matthew, capability `declare-head`, scope `epr:elohim-protocol/*`, bounded window + rate.
- **Head-Plane Cost Budget**: 1 item per steward per corpus; negligible.
- **Network Stakes**: all stages; floor-protected. Stage-priceable: none.
- **Content Address Strategy**: Content-Derived (`cid == entry_hash`); W's write carries `bounded_by: <this cid>`.
- **Source of Truth**: Holochain DHT → `mishpat_commitments`.
- **Integrity Zome + DNA-hash class**: `mishpat_integrity` — DNA-hash-NEUTRAL (existing action, existing validator).
- **Coordinator Zome**: `mishpat::create_commitment -> EntryHash` (exists); verdict by `authorize_operation(performer = matthew's chain-root, capability = "declare-head")` — the performer resolves W → chain-root through the controller set, which is the one new predicate step.
- **Projections**: existing.
- **HTTP Route**: none new. The declare path is a peer act on the workspace's own storage; the doorways learn the head through the plane.
- **Anti-Pattern Check**: the fail-open Stage-1 commons write is named, not relied upon: station 4 asserts that a device write **without** a bounding commitment is refused at community reach and *flagged* at commons reach (C14 witnessed residual), so the batch measures the gap rather than papering it.

### Entity: workspace peer's `peer_transport_manifest` row
- **Classification**: Ephemeral (C) — existing operational projection (`p2p_iroh/peer_map.rs`), rebuilt from `peer_identity_bindings` + live endpoints. No change.

### Design Constraints Discovered
- **Order is forced:** transport binding (W ↔ NodeId) must exist before the identity binding is useful to any peer's reach gate, and both must exist before a station-4 write can be attributed. Stations 1–2 need neither (commons reach is ungated) — which is exactly why they run first and measure the pull leg in isolation.
- **The pull-leg RED gates the whole ladder.** Until a fleet peer can fetch a workspace-authored id (arc advertisement + network-authored id discovery), no station past 2 can be green on alpha; they can be green on the household mesh, which is where they are proven first.
- **Never copy matthew's key.** W is a device; matthew's agency reaches the plane *through* W by the controller set. A copied key would make every device indistinguishable and every revocation impossible.
- **Never seed through the doorway for these stations.** A doorway seed passing where the peer act fails is the false-green this spec exists to remove.

### Back-fill detector
1. `bind_identity` returns the Commitment `EntryHash`; there is no route, so nothing accepts a different hash.
2. `mishpat_integrity` hosts the entry type; nothing here moves the DNA hash (coordinator hot-swap via `ALLOW_COORDINATOR_UPDATE`).
3. <30 head-plane items at 1 year across all three entities; quiesce cost unmeasurable against the ~3.5k content heads.

## The ladder (four stations, each with its own counter)

| # | Station | Assertion | Counter / probe | Where proven |
|---|---|---|---|---|
| 1 | **Joined** | workspace conductor + storage on alpha's DHT and iroh swarm as W | `sovereign-peer-join` scenario 1 (green); `elohim_inventory_pages_total` on a fleet peer shows pages from W's node | alpha (T3 rung, today) |
| 2 | **Pulled** | a fleet peer fetches a workspace-authored id | `sovereign-peer-join` RED scenarios; `elohim_acquisition_outcomes_total{outcome="fetched"}` on adam for W's id; `/p2p/status pull.fetched` | household mesh first, then alpha |
| 3 | **Recognised** | W is a controller of matthew's identity head and its transport ids are bound; a fleet peer's `signer_is_known_agent(W)` is true | `stewarded-device-sync` scenarios 1–3; `peer_identity_bindings` row for W on ≥1 fleet peer within 3 min | household mesh, then alpha |
| 4 | **Native sync** | a device write under adam's grant moves the declared head; both doorways serve it; no Jenkins, no seed | `stewarded-device-sync` 4–6 reusing saga-06's served-head-matches-declared-head step on `alpha-A` and `elohim.host` | household mesh, then alpha |

Stations 1–2 are `sovereign-peer-join.feature` unchanged (compose, don't fork). Stations 3–4
are `genesis/a2o/features/auth/stewarded-device-sync.feature`, which also closes the canonical
story's dangling `feature:` reference — matthew's workspace is the adult instance of the same
handshake james's second device makes as stewardee.

## What the ratchet measures (from inside the plane)

Each station adds a row the latency scoreboard can carry
(`.claude/scripts/latency-scoreboard.py`, already one of the habit's checks):

- **t_join** — start of `just dev conductor alpha` → W listed live by doorway-alpha's conductor diagnostics (today's bound: 10 min, the fleet's announcement interval).
- **t_advert** — W's storage publishes an inventory page → first fleet peer applies it (`inventory_pages_total{applied}` with W's node id).
- **t_pull** — declared on W → first fleet peer holds the bytes (`acquisition_outcomes_total{fetched}`), and the fan-out curve to all 7.
- **t_head** — device declare → served head equals declared head on `alpha-A` and on `elohim.host` (the saga-06 predicate, timed).
- **quiesce Δ** — per-write cost on the projection-reconcile leg (`projectionReconcile.pending/healedTotal` before/after), i.e. what one native content update costs the fleet versus the ~20 min churn + hours of catch-up a redeploy costs today.

These numbers are the batch's deliverable as much as the green scenarios are: the first
end-to-end measurement of convergence from a participant's seat.

## Concern canon (Step 4) — for `bind_identity` and the W-aware reach gate

| Class | State | Note |
|---|---|---|
| C0 plane location | answered | identity binding on the DHT (mishpat), reach verdict in storage, projection in SQLite — nothing in k8s |
| C1 anti-self-election | answered | W cannot bind itself as controller; the policy names matthew's steward-set / recovery quorum as authority |
| C2 monotonic authority | answered | bindings supersede, never mutate; revocation is a new commitment |
| C3 liveness | partial | `bind_identity` runs on the workspace's own conductor; no fleet dependency — but station-3 recognition on alpha depends on the pull leg (RED) |
| C4 honest absence | answered | `signer_is_known_agent` unknown (DB error) must become `Refused{reason}` at community reach — this batch flips the Stage-1 fail-open to fail-closed and pins it |
| C5 evidence-not-authority | answered | the binding is evidence; the grant is authority; a write needs both |
| C6a bounded work | answered | one conductor call per binding; no sweep |
| C6b idempotent effect | answered | re-binding the same W is a no-op supersession |
| C7 advertise/serve symmetry | partial | a controller set the DID head advertises but the reach gate can't resolve is the gap station 3 measures |
| C8 observability-per-decision | answered | every refusal carries `reason` + commitment cid; a `signer_binding_lookup_total{outcome}` counter is added |
| C9 identity-lineage continuity | answered | attribution resolves through the chain-root, surviving W's rotation |
| C10 contract-evolution honesty | answered | `binds-identity` payload is versioned by the commitment's own cid |
| C11 externally-imposed backpressure | n-a | no external queue |
| C12 consent/authorization | answered | the grant is the consent; `bounded_by` is mandatory at station 4 |
| C13 graduated authority | answered | `controller_policy` carries the M-of-N; a ward's device binds via its guardian's steward-set, same shape |
| C14 witnessed residual | partial | commons-reach writes without `bounded_by` are flagged, not refused, until the fleet carries station 4 |

Registration: rows for `bind_identity` (verdict-fn) and the W-aware `classify_reach_authorization`
land in the crates' `seam-registry.yaml` when the code does, with `contractTests` naming the pins.

## Out of scope, named

- Economic attribution through `AgentPeerBinding` (blocked on the cross-signed proof).
- DNA-reinstall key migration (identity-head spec §4.4).
- Replacing `just seed apply` for **fixture** content — fixtures stay pipeline-seeded; this
  spec is for authored corpus updates on already-exposed surfaces.
- Any `kubectl`; any doorway-side route. The doorway learns the head through the plane or
  the station is not green.
