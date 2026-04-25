# EPR Phase 2B — Design Spec

**Status:** Design (brainstorm complete, awaiting user review)
**Date:** 2026-04-24
**Branch:** `feature/epr-phase-2b-design` (off `dev` @ `8f8d648e`)
**Authors:** Matthew Dowell + Claude Opus 4.7 (1M)
**Depends on:**
- `2026-04-23-epr-phase-2c-libp2p-federation-design.md` — parent wire design, now wire-locked
- `2026-04-24-epr-phase-2c-batch-d-completion-addendum.md` — canonical 7-item deferral list
- `2026-04-21-elohim-core-graph-substrate-design.md` — substrate spec (phases 0–7)
- `2026-04-21-recovery-protocol-phase-2-design.md` + M1–M4 plans — DNA signal stream producer side
- Existing `elohim-epr` Rust crate (`elohim/epr/`) and `@elohim/epr-ts` TS codec (`elohim/sdk/epr-ts/`) — Phase 1 deliverables

**Related:**
- Memory `project_three_layer_truth_model.md` — DHT / libp2p / doorway truth split
- Memory `project_dht_vs_libp2p_scoping.md` — DHT is authoritative, libp2p is operational
- Memory `project_elohim_active_observed_not_flagged.md` — observed ≠ configured
- Memory `project_cadence_archetype_tunable_with_dev_overrides.md` — 4-layer override pattern
- Memory `feedback_schema_first_ioc.md` — wire contract → JSON schema first
- Memory `project_epr_substrate_vs_vf_graphql.md` — substrate ≠ VF-GraphQL (app layer)
- `.claude/skills/p2p-design-gate/` — A / A2 / B / B2 / C classification

---

## 1. Problem framing

Phase 2C delivered a wire-complete libp2p federation protocol for EPR atoms:
`/elohim/epr-atom/1.0.0` request-response, `StubIdentityMap` behind reach gates,
structural-only `verify_incoming_epr`, and `FederatedEprStore` with the swarm
handle stubbed behind five `TODO(phase-2b)` markers
(`elohim/elohim-storage/src/services/epr_store.rs:7,192,221,230,261`). Atoms can
travel between peers by CID; their *meaning* is not yet visible to the pillars,
their identity bindings are session-free, and their signatures are
structurally-but-not-cryptographically verified.

Phase 2B closes that gap. The seven items enumerated in the Batch D addendum
(§"What follows Batch D", lines 180–194) are the canonical scope. This spec
resolves the coupling decisions that make them plannable, organizes them into
four batches (A/B/C/D, mirroring 2C), and names the invariants the projector
must preserve.

### 1.1 The three-arc framing

Phase 2B sits at a hinge. Three workstreams converge on it; the design weights
decisions so none is painted into a corner:

| Arc | Role | Relation to 2B |
|---|---|---|
| **Resiliency epic** (recovery M1–M4) | Producer | Recovery events (`KeyRotation`, `KeyRevocation`, `RevocationAttestation`, `RecoveryReclaim`) flow *into* the substrate through 2B's identity binding, verify cache, and projector |
| **EPR Phase 2B** (this spec) | Hinge | Identity • verify • projector • signal-harness • write-through • discovery |
| **Graph surface** (substrate phases 3–7) | Consumer | Phase 3 (manifest resolver), Phase 4 (GraphQL + shefa subgraph, where hREA/VF-GraphQL lands), Phase 5+ read *from* 2B's projector output |

Every coupling decision below names its weighting under all three lenses.
"Operational" alone is not the design target; the hinge must serve all three.

### 1.2 What 2C locked vs what 2B must respect

Phase 2C version-pinned the wire format with golden vectors at
`elohim/elohim-storage/tests/vectors/epr_atom_messages.json`. Any wire-breaking
decision becomes `/elohim/epr-atom/2.0.0` and is a separate sprint. This spec
takes the 2C wire format as immutable:

- Four request variants (`Fetch`, `FetchBatch`, `Announce`, plus implicit future-reserved)
- Five response variants (`Atom`, `AtomBatch`, `Announced`, `NotFound`, `Error`)
- `envelope_bytes: Vec<u8>` with `serde_bytes` carrying canonical CBOR
- CBOR tag-based discrimination
- 4-byte BE length prefix + 256 KB request / 2 MB response caps

The legacy `/elohim/epr/1.0.0` MessagePack protocol (serving `EprHead`) is
also preserved — its deprecation is deferred to a future phase.

---

## 2. Load-bearing principle — P1: elohim-storage as a reconciliation controller

The Holochain DHT is the authoritative manifest for identity, key, and
governance state. The libp2p / elohim-storage layer is a **reconciliation
controller** over that manifest, in the k8s controller-manifest sense:
observed state changes → controller reconciles → no hesitation, no lazy
acceptance.

This is Principle **P1**. It is the spine of every 2B decision below.

**Concrete implications:**

1. **Authority split is the three-layer truth model applied:** Holochain DHT
   notarizes; libp2p (embodied in elohim-storage) reconciles operational state
   toward DHT-authoritative truth; doorway projects to web2 as a blind proxy,
   never participating in the reconciliation itself (per memory
   `project_three_layer_truth_model.md`).
2. **Controllers subscribe, they don't poll.** elohim-storage subscribes to a
   DNA signal stream from imagodei (new channel, scoped in Batch A). On
   observed `KeyRotation` / `KeyRevocation` / `AgentPeerBinding` /
   `RevocationAttestation` events, the controller reconciles eagerly.
3. **Lazy mark-stale is rejected for integrity state.** Under adversarial
   recovery, a compromised key's forged bindings could race with legitimate
   rotations. Lazy acceptance leaks staleness until a reader touches. Eager
   reconciliation is the correctness guarantee.
4. **Reconciliation bounds are indexed, not table scans.** Every sweep
   operates on an index (e.g., `(signer_cid, issued_at)` on `epr_atoms`).
   Sweeps are observable — the controller reports reconciliation lag.
5. **Observed, not flagged.** Effective state of any subsystem is the
   *observed* composition of manifest + overrides + actual activity, not the
   configured value (per memory `project_elohim_active_observed_not_flagged`).

P1 applies to every subsystem: identity binding cache, verify cache, projector,
write-through flag effective state, discovery fanout. Each subsystem is one
controller loop over its specific manifest.

---

## 3. Coupling decisions (resolved)

Eight coupling decisions from the kickoff prompt. Each is resolved with its
three-arc weighting and p2p-design-gate classification.

### 3.1 Decision #1 — Identity resolution (PeerId ↔ AgentPubKey)

**Resolution: Three-layer hybrid.**

| Layer | Shape | p2p-gate | Purpose |
|---|---|---|---|
| Notarization | New `AgentPeerBinding` entry type in imagodei integrity zome. Signed by agent. Payload: `{ peer_id: Vec<u8>, agent_cid: Cid, valid_from: DateTime, valid_until: Option<DateTime>, device_archetype: DeviceArchetype, superseded_by: Option<ActionHash> }`. Validator rules: signer must match `agent_cid`'s current key per rotation chain; device_archetype from `project_multi_device_humans.md`. | **A** (DHT-notarized) | Audit trail; recovery-provable; rotations via existing `KeyRotation` invalidate prior bindings by link traversal |
| Distribution | libp2p handshake on connection establishment (part of the request-response protocol's pre-handshake) exchanges current signed binding. Gossipsub `elohim/identity/binding` topic (single, global, not pillar-scoped — bindings are agent-scoped) propagates rotations to already-connected peers. Subject to the decision #7 integrity-always-both exception. | **C** (operational) | Fast path, cache-warmup, rotation propagation without requiring re-handshake |
| Consumption | `peer_identity_bindings` cache table in elohim-storage. Columns: `peer_id TEXT PK, agent_cid TEXT, binding_action_hash TEXT, valid_from, valid_until, observed_at, source TEXT`. Rebuildable from DNA + gossip. | **C** (operational projection) | Replaces `StubIdentityMap`. Reach gate and verify cache consume this directly |

**Replaces:** `elohim/elohim-storage/src/p2p/identity_map.rs::StubIdentityMap`
with a real `HolochainBackedPeerIdentityMap` that reads from the
`peer_identity_bindings` table and subscribes to the DNA signal stream for
updates.

**Three-arc weighting:**
- **Operational:** session cache is the hot path; gossip handles rotations mid-session; no DHT write per-session
- **Resiliency producer:** rotations via existing imagodei `KeyRotation` entries invalidate prior bindings; compromise can be audited from DNA alone without trusting any peer's claim
- **Graph surface:** `AgentPeerBinding` is graph-native — Phase 4 GraphQL can query "Agent X has current peer bindings [Y1, Y2]" via imagodei subgraph traversal

**New DNA work (Batch A):**
- Add `AgentPeerBinding` to imagodei `EntryTypes` enum (`elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:991+`)
- Add validators (signer check, rotation-chain freshness, device-archetype enum)
- Add link types: `AgentToPeerBinding`, `PeerToBinding` (for reverse lookup)
- Coordinator function: `create_agent_peer_binding(peer_id, device_archetype) -> ActionHash`

### 3.2 Decision #2 — Verify caching under rotation

**Resolution: Two-level cache with eager controller-driven reconciliation from the DNA signal stream.**

Key insight that reshaped the kickoff's three options: **verification is
time-sensitive under rotation.** An envelope signed by key K1 at time T1 is
valid iff K1 was the signer's authoritative key at T1 — not iff K1 matches the
signer's current key. The caching model must capture this.

| Layer | Shape | p2p-gate |
|---|---|---|
| Per-agent pubkey timeline | In-memory LRU (`lru` crate, ~10k entries default) in elohim-storage. Key = `Cid` (agent CID). Value = `Vec<PubkeyValidity { pubkey: [u8;32], valid_from, valid_until, action_hash }>` derived from the agent's `KeyRotation` + `KeyRevocation` chain. | **C** |
| Per-envelope verified flag | New columns on `epr_atoms`: `verified_at: Option<DateTime>`, `verified_signer_fingerprint: Option<String>` (blake3-128 prefix of pubkey used). Set on successful verify-at-ingest. Index: `(signer_cid, issued_at)` for sweep efficiency. | **C** |
| Invalidation channel | Subscribe to imagodei → storage DNA signal stream. On `KeyRotation`: update timeline cache; no sweep (historical sigs preserved). On `KeyRevocation`: update timeline + eager sweep over `epr_atoms` using `(signer_cid, issued_at)` index; envelopes whose `issued_at` ∈ `[revocation.compromise_at, ∞)` get `verified_at` cleared and `verified_signer_fingerprint` → `revoked_stale`. | **C** (stream) |

**The DNA signal stream IS the convergence pipe with Recovery M4.** M4's
fast-path revocation work produces the signal; 2B's verify cache consumes it.
The signal stream contract (message types, ordering guarantees, subscription
semantics) is designed in coordination with M4. If M4 ships first, 2B consumes;
if Batch A ships first, M4 designs to match.

**Three-arc weighting:**
- Operational: pubkey LRU on the hot path; sweep is bounded (indexed); per-envelope flag makes bulk reads free
- Resiliency producer: the invalidation channel is M4's output — single pipe, single invariant
- Graph surface: `verified_at IS NOT NULL AND verified_signer_fingerprint != 'revoked_stale'` is the condition Phase 4 GraphQL resolvers filter on without re-verifying

**Implementation sketch:**
```rust
// elohim-storage/src/reconcile/controller.rs  (new)
pub struct ReconcileController {
    signal_stream: DnaSignalStream,
    pubkey_cache: LruCache<Cid, Vec<PubkeyValidity>>,
    db_pool: Arc<Pool>,
}
impl ReconcileController {
    async fn run(&mut self) {
        while let Some(signal) = self.signal_stream.next().await {
            match signal {
                DnaSignal::KeyRotation(r) => self.update_timeline(r).await,
                DnaSignal::KeyRevocation(r) => self.sweep_revocation(r).await,
                DnaSignal::AgentPeerBinding(b) => self.update_bindings(b).await,
                // ...
            }
        }
    }
}
```

### 3.3 Decision #3 — Projector ownership

**Resolution: Single projector controller in elohim-storage, manifest-driven mapping, async reconciliation.**

| Attribute | Decision |
|---|---|
| Location | `elohim-storage/src/projector/` (new). Not a pillar service; not doorway. Matches memory `project_three_layer_truth_model`. |
| Mapping | Manifest-driven. Each pillar's `manifest.json` declares `projections: { kind: Content, schema_key: concept, target_table: content_nodes, column_mapping: {...} }`. Projector reads; does not hard-code pillar schemas. |
| Timing | Async (not write-through in ingest path). `EprService::ingest` commits canonically to `epr_atoms` synchronously; projector consumes via observation loop. Matches k8s API-server-then-controller model. |
| Unmapped kinds | Stay pure in `epr_atoms`. Phase 4 GraphQL exposes via generic kind-aware resolvers. No forced synthesis. |
| Signals | Projector emits domain signals on projection write (`imagodei.revocation_observed`, `shefa.event_projected`, etc.) — consumed by elohim-agent defenders, dashboards, and signal subscribers. |
| Backfill | `projector_cursor` table tracks per-(pillar, kind) cursor. Backfill is a controller operation: reset cursor, replay. Flagged for plan; not a 2B blocker. |

The projector **stewards** pillar projection tables — it does not own the
pillar's domain (per memory `project_no_sovereignty_stewardship`). The pillar
owns its schema via its manifest; the projector reconciles into the pillar's
declared table shape.

**p2p-gate:**
- `epr_atoms`: **A**
- Pillar projection tables: **C** (rebuildable from `epr_atoms` + manifests)
- Projector state (cursor, in-flight queue): **C**

**Three-arc weighting:**
- Operational: one controller, one codebase, one test surface; no bleed into Angular layer
- Resiliency producer: recovery EPRs project into imagodei's existing pillar tables through the manifest-declared mapping — no special-case code path
- Graph surface: Phase 4's subgraph schema is generated from the projector's manifest mapping; Phase 4 inherits zero rework

### 3.4 Decision #4 — EprHead ↔ Envelope reconciliation

**Resolution: Envelope canonical (A). EprHead is a derived projection (A2) served via the legacy MessagePack protocol for backward compatibility.**

The `EprLamadContext`, `EprShefaContext`, `EprQahalContext` fields on
`EprHead` (`elohim/elohim-storage/src/epr_codec.rs:97+`) are *already*
pre-projected pillar context. Under decision #3, they become projector
outputs. EprHead wraps the projector's data in the legacy wire format.

| Surface | Consumer | Format | Source |
|---|---|---|---|
| Pillar projection tables | Pillar REST + Phase 4 GraphQL | SQLite rows | Projector (decision #3) |
| `EprHead` on `/elohim/epr/1.0.0` | Legacy peer protocol | MessagePack | Same projector data, wire-wrapped for backward compat |
| `Envelope` on `/elohim/epr-atom/1.0.0` | Federation peers | CBOR canonical | `epr_atoms` (canonical source) |

**What changes in 2B:** production logic flowing into `EprHead` gets
refactored to route through the projector. Wire formats on both protocols are
preserved (2C locked the CBOR shape; legacy MessagePack stays stable for its
existing test vectors). Internally, one compute path, one invalidation path.

**We do not deprecate EprHead** in 2B. 2C chose to keep the legacy protocol;
deprecation (if ever) is a future phase with advance notice.

**Three-arc weighting:**
- Operational: wire stability preserved; one internal compute path instead of two
- Resiliency producer: single invalidation path under revocation — exactly one place to get revocation handling right for both surfaces
- Graph surface: Phase 4 bypasses EprHead entirely (reads Envelope + pillar tables); EprHead being A2 means Phase 4 inherits zero legacy-wire constraints

### 3.5 Decision #5 — Signal harness migration

**Resolution: Signal harness emits EPR-*intent*; storage composes + signs + ingests. Projector produces legacy rows.**

Browser-side signing was ruled out by inspection: `@elohim/epr-ts`
(`elohim/sdk/epr-ts/src/`) has no `signEd25519` export — only `verifyEd25519`.
This is deliberate: agent keys live in the conductor, not the browser. Kicking
off an EPR from the Angular signal harness requires a round-trip for signing.

**Shape:**

- Signal harness (`app/elohim-app/src/app/lamad/services/signal-harness.service.ts`) stays at its current layer. Captures renderer-completion events, uses `LAMAD_COUPLING_MAP` to translate to high-level intent (`what happened`, `who did it`, `coupling refs`, `desired kind`).
- **New storage endpoint:** `POST /api/v1/signal/emit` accepting pillar-declared signal-intent schemas. Returns `{eventCid, eprCid}` after ingest.
- Storage's signal handler consults pillar manifest (decision #3), composes `Envelope` with proper `kind` + `schema_key` + `coupling`, requests signature from conductor, stores in `epr_atoms`.
- Projector reconciles `economic_events` (and other pillar projection tables) from the EPR per shefa's manifest mapping. Legacy REST unchanged.
- Phase 4 GraphQL reads EPRs directly.

**This is the "compat" kickoff option, but the compat layer IS the projector**
— no separate infrastructure.

**Prerequisite risk (Batch C, may move to Batch A if tight coupling):**
- Conductor must expose a signing API reachable from elohim-storage: "sign
  these canonical bytes under agent-key X". If missing, scope it into Phase 2B
  as a Batch A or Batch C task.

**First migration target:** shefa's renderer-completion → `EconomicEvent` EPR.
Highest-value (feeds Phase 4 + R&O #4). Imagodei recovery signals co-migrate
with M4. Other pillars batch in follow-up.

**Three-arc weighting:**
- Operational: signal harness stays near-as-is (output endpoint changes, not logic)
- Resiliency producer: every client-side producer (signal harness, recovery UI, custodian dashboard, steward console) emits intent; storage composes EPRs. Same pipe, single invariant.
- Graph surface: R&O #4 becomes a manifest-declaration exercise (shefa's manifest declares `schema_ref: <vf-graphql-manifest-cid>`), not a producer-migration exercise

**p2p-gate:**
- Signal-intent request: **C** (transient HTTP)
- Storage-composed Envelope: **A**
- Legacy `economic_events` row: **C** (projector output)

### 3.6 Decision #6 — Write-through flag granularity

**Resolution: Per-pillar, 4-layer override composition per memory `project_cadence_archetype_tunable_with_dev_overrides`, observed not configured.**

| Layer | Where | Purpose |
|---|---|---|
| 1. Manifest default | Pillar `manifest.json` has `write_through: { enabled: bool, kinds: [EprKind] }` | Pillar authors declare intent |
| 2. Policy override | `policy.toml` on the host | Operator overrides at deploy time |
| 3. Env/CLI override | `ELOHIM_WRITE_THROUGH_<PILLAR>=on` or `--write-through <pillar>=on` at startup | Dev + emergency |
| 4. Sync admin trigger | Runtime admin API call `POST /admin/write-through` | Live emergency ramp/halt |
| **Effective state** | Composed view over all 4 | Reported via `/api/v1/status/write-through` |
| **Actual activity** | EPR write counter per (pillar, kind) over rolling window | The *observed* truth |

**Per-entity-type rejected** — too fine-grained. `kind` already gives useful
discrimination.

**Initial default: OFF at manifest level for every pillar.** Turning a
pillar's write-through ON is a deliberate operator act. Ships safe; ramps
deliberate.

**Exception rule — integrity events always write through.**
`RevocationAttestation`, `KeyRotation`-observed attestations, and any
imagodei recovery signals bypass the per-pillar flag. Hardcoded, not
configurable. A silent revocation is a security hole.

**Three-arc weighting:**
- Operational: matches existing cadence-override pattern; no new config paradigm
- Resiliency producer: hardcoded exception prevents misconfigured silencing
- Graph surface: per-pillar ramp is *the* tool for lighting up subgraphs — shefa first per decision #5, then imagodei, then lamad, etc.

**p2p-gate:** Config layers and effective state — all **C** (operational).

### 3.7 Decision #7 — Kad + gossipsub composition

**Resolution: Tiered routing by reach + integrity-always-both exception.**

| Reach | Routing | Rationale |
|---|---|---|
| Private / SelfScope | Direct peer only, reach-gated | No-broadcast by design |
| Intimate | Direct to circle peers | Small-group efficiency |
| Trusted / Familiar | Gossip on trust-scoped topic | Moderate group |
| Community | Gossip on community-scoped topic + Kad-light (opt-in provider record) | Hot for subscribers + cold discovery |
| Public | Gossip on public topic + Kad | Freshness + discoverability |
| Commons | Kad primary, gossip secondary (refresh signal) | Long-lived, cold-discovery-dominant |
| **Integrity exception** | ALWAYS gossip + Kad + direct-notify to known-affected peers | `KeyRotation`, `KeyRevocation`, Manifest-EPR publication, projector-invalidation signals |

**Dedup:** LRU by CID on each peer's receive path (bounded, ~a few MB). On
duplicate receipt, no-op; tracker emits "seen" counter for observability.

**Integrity exception mechanics:**
- Revocation/rotation EPRs skip the reach tier and go on all three channels simultaneously
- Direct-notify identifies peers recently served by the revoked key via the `signer_cid` index — notified out-of-band (Recovery M4 produces the notification list; 2B consumes and notifies)

**Gossipsub topic structure (proposed, deferred to plan for validation):**
```
elohim/<pillar>/<reach>/[<collective-id>]
elohim/identity/binding
elohim/integrity/revocation
```
Reach-gated subscription: a peer can only subscribe to topics it's authorized
for per its agent's delegations / group memberships.

**Three-arc weighting:**
- Operational: tiered routing respects existing Reach enum semantics; no wasted Kad/gossip cost for Private atoms
- Resiliency producer: integrity exception guarantees revocations propagate through every channel — no single-point-of-failure propagation
- Graph surface: Manifest-EPRs (rare, Commons) discoverable via Kad from cold-start; domain EPRs (hot, Community/Public) available to subscribers via gossip; Phase 4 federated resolver gets both layers for free

**p2p-gate:**
- Kad provider records: **C** (operational discovery)
- Gossipsub topic state + subscriptions: **C**
- Dedup LRU: **C**

### 3.8 Decision #8 — Session scope

**Resolution: Do NOT defer items 5–7. All 7 items in one Phase 2B spec; 4-batch decomposition (A/B/C/D).**

Principle P1 (controller pattern) unified the decisions so tightly that items
5–7 became applications of the same principle rather than independent concerns.
Deferring them risks under-weighting in the plan's batch shape.

Plan status at commit time: **first-draft**. Per-batch sessions tighten
task-level scope at execution time.

---

## 4. Work item shape under resolved coupling

Below: each of the 7 addendum items placed in the batch shape, with the
coupling decisions they inherit. The plan document sibling to this spec
decomposes these into tasks.

### Batch A — Identity & controller foundation (decisions #1, #2)

**Tasks (plan-level sketch):**

1. Imagodei DNA: `AgentPeerBinding` entry type + link types + validators (`AgentKeyToBinding`, `PeerToBinding`)
2. Imagodei coordinator: `create_agent_peer_binding`, `rotate_agent_peer_binding`
3. Imagodei → storage DNA signal stream contract (message types: `KeyRotation`, `KeyRevocation`, `AgentPeerBinding`, `RevocationAttestation` initially; ordering guarantees; subscription semantics). **Converge with Recovery M4.**
4. elohim-storage: `ReconcileController` loop infrastructure
5. elohim-storage: `HolochainBackedPeerIdentityMap` replaces `StubIdentityMap`
6. elohim-storage: `peer_identity_bindings` table (migration + diesel model)
7. elohim-storage: per-agent pubkey timeline LRU + `PubkeyValidity` derivation
8. elohim-storage: `verified_at` + `verified_signer_fingerprint` columns on `epr_atoms` (migration)
9. elohim-storage: eager revocation sweep over `(signer_cid, issued_at)` index
10. libp2p: authentication handshake in EPR protocol pre-phase (exchange signed binding)
11. libp2p: `elohim/identity/binding` gossipsub topic subscription + publish on rotation
12. Integration test: Peer A rotates key → Peer B observes via signal stream → verify cache sweeps → stale verifications cleared

### Batch B — Projector & read-model reconciliation (decisions #3, #4)

**Tasks (plan-level sketch):**

1. Projector controller skeleton (`elohim-storage/src/projector/`)
2. Manifest-declared mapping schema (`pillar-manifest.projections: [{kind, schema_key, target_table, column_mapping}]`) — schema-first via `elohim/sdk/schemas/v1/`
3. Shefa manifest: declare `EconomicEvent → economic_events` mapping (minimum for first migration)
4. Projector: event loop reading `epr_atoms` insert stream, projecting to pillar tables
5. `EprHead` production path refactor: legacy `/elohim/epr/1.0.0` handler composes `EprHead` from projector state + `epr_atoms`, instead of current direct construction
6. Projector domain signal emission (signal handlers registered per kind)
7. `projector_cursor` table + backfill command (flagged for plan; minimum implementation in 2B, operator-driven in follow-up)
8. Integration test: Content EPR → `content_nodes` row; revocation → `EprHead` serves with revoked-marker

### Batch C — Producer migration & ramp controls (decisions #5, #6)

**Tasks (plan-level sketch):**

1. Conductor signing API contract (storage → conductor request/response) — prerequisite, may move to Batch A
2. `/api/v1/signal/emit` endpoint in elohim-storage
3. Signal-intent schema (schema-first): `elohim/sdk/schemas/v1/signal-intent.schema.json`
4. Storage signal handler: intent → Envelope compose → conductor sign → ingest
5. Angular `SignalHarnessService` migration: output endpoint switch (behind feature flag for rollback)
6. Write-through flag 4-layer composition: manifest field, policy.toml parser, env/CLI binding, admin API
7. Effective-state endpoint: `/api/v1/status/write-through` reporting composed state + actual activity counter
8. Integrity exception hardcoding (integrity EPR kinds bypass flag)
9. Integration test: shefa renderer-completion → `/api/v1/signal/emit` → Envelope in `epr_atoms` → `economic_events` row via projector

### Batch D — Discovery & fanout (decision #7)

**Tasks (plan-level sketch):**

1. Reach-tier routing policy in `p2p/behaviour.rs`
2. Kad `start_providing` on `Announce` for tier ≥ Community
3. Gossipsub topic enumeration: `elohim/<pillar>/<reach>/[<collective>]`, `elohim/identity/binding`, `elohim/integrity/revocation`
4. Reach-gated subscription enforcement (peer can only subscribe to topics they're authorized for)
5. Integrity exception routing (revocation/rotation bypass tier, go all channels)
6. Direct-notify for known-affected peers via `signer_cid` index + M4 notification-list contract
7. Dedup LRU (bounded ~few MB) on receive path
8. Integration test: Commons-reach announce discoverable by cold-start peer via Kad; Community-reach announce received by gossip subscribers only; revocation received by all three channels

---

## 5. Invariants the projector preserves

**This is the load-bearing section — the contract between substrate and
pillars, and the contract between 2B and Phase 4+.**

The projector's correctness is measured by these invariants. A projector
implementation that violates any invariant breaks a downstream layer.

### I1. Idempotency
For any EPR CID `x` in `epr_atoms`, re-running the projector produces the
same pillar-table row. No duplicates, no divergent state across replays.
Required for: backfill, crash recovery, controller restart.

### I2. Manifest-authority
No pillar table gets a row from an EPR whose `kind` and `schema_key` are
not declared in the pillar's `manifest.json` `projections` list. No implicit
projections. Required for: pillar sovereignty (pillar controls what enters
its table shape).

### I3. Causal ordering within kind
Projector processes EPRs in `issued_at` order per `signer_cid` for a given
kind. Earlier rotations do not override later attestations of the same
logical entity. Required for: correct revocation sequencing, recovery
continuity.

### I4. Revocation propagation
An observed `KeyRevocation` event invalidates all projection rows sourced
from EPRs signed by the revoked key within the compromise window, within
the same *reconciliation pass* (one iteration of the controller loop
processing the signal; "tick" in k8s-controller vocabulary). No row persists
with stale trust state after the pass completes. Required for: resiliency
arc.

### I5. Verified-state consistency
A projection row carries its source EPR's `verified_at` trust state. If the
EPR's `verified_at` is cleared (e.g., by revocation sweep), the projection
row's `verified` column is also cleared within the same reconciliation
pass. No pillar reader sees a projection row with stale verified-state
after the pass completes.

### I6. Unmapped-kind transparency
EPRs whose `(kind, schema_key)` have no manifest declaration are NOT
dropped. They remain in `epr_atoms` unprojected. Phase 4 GraphQL generic
kind-aware resolvers can expose them. Required for: forward compatibility,
ungrudging-service principle (memory `project_ungrudging_service`).

### I7. Signal emission
Every successful projection write emits one domain signal on the projector's
internal signal stream: `<pillar>.<kind>.projected` with a pointer to the
EPR CID and the affected row. Subscribers (dashboards, elohim-agent
defenders, future Phase 4 GraphQL subscriptions) read these. Required for:
P1 controller observability.

### I8. No pillar-data sovereignty
The projector writes only to columns declared in the manifest's
`column_mapping`. Columns outside the mapping (pillar-service-owned) are
untouched. Required for: memory `project_no_sovereignty_stewardship`.

### I9. Observable reconciliation lag
The projector reports `projector_lag_seconds` per `(pillar, kind)` — the
delta between newest `epr_atoms.inserted_at` and newest
`pillar_table.projected_at`. Operator dashboards surface this. Required for:
memory `project_elohim_active_observed_not_flagged` — lag is observed, not
assumed.

---

## 6. Convergence with adjacent workstreams

### 6.1 Recovery epic (M1–M4) convergence

**Exact convergence point: the DNA signal stream from imagodei → elohim-storage.**

| M-milestone | Produces | 2B consumes |
|---|---|---|
| M1 (data model) | Entry types: `KeyRotation`, `KeyRevocation`, `RevocationVote`, etc. | Schema for signal-stream message types |
| M2 (validators) | DHT-enforceable validation on recovery entries | Trust basis for signals (invalid signals rejected at DNA) |
| M3 (coordinator + storage) | Coordinator functions emitting DNA signals | `ReconcileController` subscribes |
| M4 (fast-path revocation) | Signal stream emission on revocation events + affected-peer notification list | Batch A tasks 3 + D task 6 directly consume |

**If M4 lands before 2B Batch A:** 2B Batch A consumes the existing stream
contract.
**If 2B Batch A lands before M4:** Batch A designs the subscriber side; M4
designs emission to match. Either direction is workable; the stream contract
must be schema-first (per `feedback_schema_first_ioc`).

**Not merged into one sprint.** The epics remain independent batches for
review and rollout clarity. They converge on the shared contract, not the
code.

### 6.2 Graph surface (Phase 3–7) consumption

Phase 2B's outputs are Phase 3's inputs:

| 2B output | Phase 3+ consumer |
|---|---|
| Manifest-declared projection mapping | Phase 3 manifest-graph resolver: pillar manifests become Manifest-EPRs; mapping declarations become Manifest-EPR payload fields |
| `epr_atoms` populated with real signed envelopes | Phase 3 `schemaRef` CID resolution walks manifests as EPRs |
| Projector outputs (pillar projection tables) | Phase 4 GraphQL subgraph resolvers read these directly |
| `verified_at` trust state | Phase 4 resolvers filter on this without re-verify (critical for federated query performance) |
| Signal emission per projection | Phase 6 GraphQL subscriptions map 1:1 to projector signals |
| `EconomicEvent` EPRs with shefa's `schema_ref` → VF-GraphQL manifest CID | R&O #4 (hREA alignment) becomes a manifest-declaration exercise |

**What 2B does NOT build:** Phase 3's Manifest-EPR resolver, Phase 4's
GraphQL surface, Phase 4's subgraph schema generation, Phase 5's pillar
subgraph rollout, Phase 6's federated subscriptions. All are deferred to
their own phases.

**What 2B MUST not do:** bake Phase 4 wire format into the projector. The
projector writes to pillar projection tables that already exist (shape
defined by current diesel migrations). Phase 4 GraphQL reads *over* these
tables, generating its schema from manifests. 2B must not pre-shape
projection tables for GraphQL.

### 6.3 Substrate ≠ VF-GraphQL (memory pin)

Per memory `project_epr_substrate_vs_vf_graphql`:

- EPR substrate is the primitive (content addressing, signed envelopes, coupling refs)
- VF-GraphQL is app-layer — specifically Phase 4+ (R&O #4)
- Phase 2B must not introduce VF vocabulary into the projector
- Shefa's EPRs carry `schema_ref: <vf-graphql-manifest-cid>` at envelope level; that's it. No VF resolvers, no VF types in 2B code.

---

## 7. Open questions explicitly deferred

These are named here so the next session's scope is clear:

### O1. Conductor signing API contract
Storage → conductor "sign these canonical bytes under agent-key X" — does
this exist? If yes, which Tauri command / doorway-proxied conductor call?
If no, Batch A or Batch C adds it. **Action:** Batch C task 1 (prerequisite).

### O2. Gossipsub topic authorization proof
A peer subscribing to `elohim/shefa/community/<householdId>` must prove
authorization. Mechanism: presents a capability grant? Delegation EPR?
Reach-gated subscription enforcement needs a precise design. **Action:** Batch
D task 4 design pass.

### O3. Projector backfill semantics for non-idempotent historical EPRs
Historical EPRs persisted before the projector existed — are they replayed
on first projector start, or left unprojected until operator triggers? **Action:**
Batch B task 7 decision pass.

### O4. Signal stream durability under subscriber restart
If elohim-storage restarts mid-tick, does the DNA signal stream have a
resumption point (cursor), or must the controller re-query all relevant
DNA state? **Action:** coordinate with Recovery M4's stream design; flagged
for Batch A task 3.

### O5. `EprHead` wire compatibility under pillar-context absence
What does `EprHead` serve for an EPR whose pillar has no manifest
projection declared? `None`-valued context fields, or suppress the EPR from
the legacy protocol entirely? **Action:** Batch B task 5 decision pass.

### O6. Per-pillar write-through ramp sequence
After shefa (decision #5's first target), which pillar is next? Imagodei is
coupled to Recovery M4. Lamad has the most content volume. Qahal is
governance-sensitive. **Action:** operator call at Batch C rollout time;
not a design decision.

### O7. Device-archetype enumeration for `AgentPeerBinding`
Memory `project_multi_device_humans` names node / device / mobile
archetypes. Full enum? Per-household variability? **Action:** Batch A task
1 — start with `{ Node, Desktop, Mobile, Steward }` and iterate.

### O8. Phase 4 preparation signal
At end of Phase 2B, should a formal "Phase 3 kickoff" pass happen? Memory
`project_epr_substrate_vs_vf_graphql` marks it 🔴 and multi-week. The
hREA/VF-GraphQL landing zone is Phase 4; Phase 3 (manifest-graph resolver)
is prerequisite. **Action:** at 2B completion, write a Phase 3 kickoff
prompt. Not in 2B scope.

---

## 8. Definition of done

- [ ] Batch A merged: real `PeerIdentityMap`, `AgentPeerBinding` in DNA, verify cache with eager sweep, DNA signal stream consumer live
- [ ] Batch B merged: projector controller, manifest mapping schema, shefa's first projection, `EprHead` production path refactored
- [ ] Batch C merged: `/api/v1/signal/emit`, signal harness migration, 4-layer write-through flag, integrity exception hardcoded
- [ ] Batch D merged: tiered routing, Kad providers, gossipsub topics, reach-gated subscription, integrity-always-both exception, dedup LRU
- [ ] Integration test harness extended (`epr_atom_federation_integration.rs`) with 2B scenarios: rotation propagation, revocation sweep, projector round-trip, signal-intent → EPR round-trip
- [ ] Addendum `2026-04-24-epr-phase-2c-batch-d-completion-addendum.md` §"What follows Batch D" updated with pointer to this spec + plan
- [ ] `TODO(phase-2b)` markers in `epr_store.rs` all resolved or escalated to Phase 3+
- [ ] Schema-first wire contracts exist in `elohim/sdk/schemas/v1/` for: `signal-intent.schema.json`, `agent-peer-binding.schema.json`, pillar `projections` mapping extension
- [ ] Phase 3 kickoff prompt written (see O8) at 2B completion

---

## Appendix A — p2p-design-gate classifications

| # | Entity | Class | Notes |
|---|---|---|---|
| 1 | `AgentPeerBinding` (new imagodei DNA entry) | **A** | DHT-notarized, signed by agent, payload per §3.1 |
| 2 | `peer_identity_bindings` cache table | **C** | Operational projection of bindings, reconstructable |
| 3 | Per-agent pubkey timeline LRU | **C** | In-memory derivation of rotation chain |
| 4 | `verified_at`, `verified_signer_fingerprint` on `epr_atoms` | **C** | Re-verifiable from canonical bytes + pubkey cache |
| 5 | DNA signal stream (imagodei → storage) | **C** | Transient, DNA-reconstructable |
| 6 | `ReconcileController` state (loops, in-flight queue) | **C** | Ephemeral |
| 7 | `projector_cursor` table | **C** | Rebuildable from `epr_atoms` + replay |
| 8 | Pillar projection tables (existing + declared extensions) | **C** | Materialized views |
| 9 | `Envelope` (2C-locked, existing) | **A** | Canonical CBOR, CID-derived, Ed25519-signed |
| 10 | `EprHead` (legacy, existing) | **A2** | Derived via projector from Envelope |
| 11 | Signal-intent payload (`/api/v1/signal/emit`) | **C** | Transient HTTP; not persisted |
| 12 | Write-through flag layers + effective state | **C** | Config + observed composition |
| 13 | Kad provider records | **C** | Operational discovery |
| 14 | Gossipsub topic state + subscriptions | **C** | Ephemeral subscriber state |
| 15 | Dedup LRU | **C** | Bounded cache |

**Distribution:** 2× A, 1× A2, 12× C. No B / B2 entities — the design
intentionally avoids agent-scoped attestation layers in favor of DHT
authority (A) + operational reconciliation (C). This matches Principle P1.

---

## Appendix B — Decisions incorporated

- **B-1** P1: elohim-storage as reconciliation controller over Holochain DHT manifest (k8s controller model) — user reframe, 2026-04-24
- **B-2** `AgentPeerBinding` DHT-notarized (A) + libp2p distribution (C) + cache (C) — decision #1 resolution
- **B-3** Two-level verify cache with eager sweep, NOT lazy mark-stale — decision #2 resolution under P1
- **B-4** DNA signal stream is the Recovery M4 convergence pipe — consequence of B-3
- **B-5** Single projector in elohim-storage, manifest-driven, async reconciliation — decision #3 resolution
- **B-6** Envelope canonical (A), EprHead is A2 derived via projector — decision #4 resolution
- **B-7** Signal harness emits EPR-intent (not EPRs); storage composes+signs — decision #5 resolution under "no browser signing"
- **B-8** 4-layer write-through flag per cadence memory; integrity exception hardcoded — decision #6 resolution
- **B-9** Tiered routing by reach + integrity-always-both — decision #7 resolution
- **B-10** 4-batch decomposition (A/B/C/D) mirroring 2C shape; do not defer 5–7 — decision #8 resolution
- **B-11** Three-arc framing (resiliency producer / 2B hinge / graph surface consumer) as design lens — user request, 2026-04-24
- **B-12** Substrate ≠ VF-GraphQL preserved; R&O #4 is Phase 4 manifest-declaration, not 2B code — memory pin `project_epr_substrate_vs_vf_graphql`

---

## Appendix C — File references

**Phase 2C seams 2B operates on:**
- `elohim/elohim-storage/src/services/epr_store.rs` — 5 `TODO(phase-2b)` markers (lines 7, 192, 221, 230, 261)
- `elohim/elohim-storage/src/p2p/epr_atom_protocol.rs:220-270` — `verify_incoming_epr` structural-only gap
- `elohim/elohim-storage/src/p2p/identity_map.rs` — `StubIdentityMap` replacement
- `elohim/elohim-storage/src/p2p/behaviour.rs` — gossipsub foundation (commit `e9e2806a`)
- `elohim/elohim-storage/src/p2p/mod.rs:913,988` — `StubIdentityMap` construction sites
- `elohim/elohim-storage/src/epr_codec.rs:97+` — `EprHead` struct + `EprLamadContext` / `EprShefaContext` / `EprQahalContext`
- `elohim/elohim-storage/tests/epr_atom_federation_integration.rs` — Batch D harness (extend, do not rebuild)

**DNA additions (Batch A):**
- `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:991+` — `EntryTypes` enum (add `AgentPeerBinding`)
- `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:1037+` — `LinkTypes` enum (add `AgentToPeerBinding`, `PeerToBinding`)
- `elohim/holochain/dna/imagodei/zomes/imagodei/src/` — coordinator functions (new file: `peer_binding.rs`)

**TS codec (Phase 1 complete, 2B uses unchanged):**
- `elohim/sdk/epr-ts/src/` — CBOR/CID/envelope/verify primitives

**Schema-first contracts (new, Batch A+B+C):**
- `elohim/sdk/schemas/v1/agent-peer-binding.schema.json` — DNA entry schema
- `elohim/sdk/schemas/v1/signal-intent.schema.json` — signal-harness intent payload
- `elohim/sdk/schemas/v1/dna-signal-stream.schema.json` — imagodei → storage stream messages
- pillar manifest extensions: `projections: [{...}]` field schema

**Angular-side (Batch C):**
- `app/elohim-app/src/app/lamad/services/signal-harness.service.ts` — output endpoint migration
- `app/elohim-library/projects/elohim-service/src/services/` — possibly new `SignalEmitService` wrapping the new endpoint

---

*End of spec. Companion plan at `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md`.*
