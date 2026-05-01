# Light Up the Graph — Substrate Orchestration Sprint Design

**Status:** Design (pre-implementation)
**Date:** 2026-05-01
**Predecessor:** [Phase 3.5 trust-compute gradient plan](../plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md), [substrate brainstorm](2026-04-30-trust-compute-gradient-brainstorm.md)
**Successor (planned):** Phase 4 — collective-wisdom aggregation, VF-GraphQL semantics, elohim-mediated discernment layer

## Context

EPR Phase 3.5 shipped the trust-compute substrate as a set of composable primitives: `back_prop`, `gossip_flood`, `standing`, `standing_projector`, `tending`, `bootstrap_manifests`, predecessor-records crypto (2-of-2 nested seal), and the FeedbackSignal/AttentionTending DHT entry types. The aunt-and-rage-bait integration test (T20) passes — but only by calling services directly with `MockOutboundSink` / `MockGossipPublisher` and stubbing two steps: the reach-earning gate that should fail Bob's recompose, and the Vouch primitive that recovers him.

The graph is built. The signals don't actually flow through the live runtime yet.

## Sprint Goal

Make the aunt-and-rage-bait scenario pass **without direct-call substitutions** — the same scenario, but driven by real `api/epr.rs` arrivals, a real swarm, and a real reconciliation controller. Lift both T20 mocks. Land the reach-earning gate and Vouch primitive as real protocol steps.

## Architecture

Six wiring sites, no new entities. All work concentrates at:

1. **`api/epr.rs::put_epr` fan-out** — when a FeedbackSignal arrives, persist + back_prop + flood + project, deduped against local-origin.
2. **`main.rs` reconciliation startup** — `bootstrap_manifests::seed_if_empty` after migrations; spawn 5-min TTL sweep task.
3. **`ManifestDebitWeightPolicy`** — replaces `DefaultDebitWeightPolicy` at the projection seam; consumes T17's bootstrap-standing-policy `debitWeights` payload.
4. **Production swarm adapters** — `LibP2POutboundSink` and `LibP2PGossipPublisher` bridge the existing trait abstractions to the swarm's P2PCommand mpsc channel (actor pattern; no swarm-locking).
5. **Reach-earning gate** — new `services/reach_earning.rs` + `services/epr_compose.rs`; gates author-side compose attempts at non-floor reach.
6. **Vouch primitive** — `signal_kind="vouch"` variant on FeedbackSignal; new `content_store::create_vouch` coordinator; integrity validator enforces no-self-vouch.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          api/epr.rs::put_epr                            │
│  (FeedbackSignal arrival, dedup via local_peer_id)                      │
└───────────────────┬───────────────────┬──────────────┬──────────────────┘
                    │                   │              │
        ┌───────────▼─────────┐ ┌───────▼──────┐ ┌─────▼──────────────┐
        │  back_prop_one_hop  │ │ flood_       │ │ standing_projector │
        │  (predecessor walk) │ │ feedback     │ │ ::project_signal   │
        └──────────┬──────────┘ └──────┬───────┘ └─────────┬──────────┘
                   │                   │                   │
        ┌──────────▼──────────┐ ┌──────▼──────────────┐ ┌─▼──────────────┐
        │ LibP2POutboundSink  │ │ LibP2PGossip-       │ │ Manifest-      │
        │ (P2PCommand chan)   │ │ Publisher           │ │ DebitWeight-   │
        │                     │ │ (P2PCommand chan)   │ │ Policy         │
        └─────────────────────┘ └─────────────────────┘ └────────────────┘
                   │                   │
        ┌──────────▼───────────────────▼──────────┐
        │     swarm task (existing libp2p loop)    │
        └──────────────────────────────────────────┘
```

### Out of scope (deferred, not in this sprint)

- Cross-peer privacy-preserving union sketch (Phase 4 brainstorm §6.4)
- Real CSPRNG-backed Laplace noise (currently deterministic stub)
- VF-GraphQL semantics on top of standing/tending substrate (Phase 4)
- Window-over-window trend computation in the aggregator
- `evidenceSources` plumbing for imagodei/lamad → standing (forward-compat schema only)
- Elohim-mediated discernment layer (matchmaking, sponsor suggestions) — substrate gate just returns `Pending`
- `gate_decisions` audit table — gate stays ephemeral
- Cross-zome migration of FeedbackSignal from `content_store` to `mishpat` (semantically governance, but moving entry types across zomes is a larger refactor)

## P2P Design Gate Output

### Entity: ReachVerdict (return type of reach-earning gate)
- **Classification:** Operational (C) — ephemeral, never persisted
- **Address:** N/A
- **Source of truth:** None (return value only)
- **Reconstruction strategy:** Recompute from current Standing + manifest at any time
- **Anti-pattern check:** ✓ No table; ✓ no source-of-truth confusion; ✓ not HTTP-route-first

### Entity: Vouch (FeedbackSignal-shaped)
- **Classification:** Notarized (A) — REUSES existing FeedbackSignal entry type
- **Address:** Content-Derived (CID) — same as parent FeedbackSignal entry
- **Source of truth:** Holochain DHT, `content_store_integrity::FeedbackSignal`
- **Coordinator zome:** `content_store::create_vouch(target_signal_cid, vouch_kind) → ActionHash`
- **Storage projection:** Reuses existing `feedback_signals` projection with `dht_anchor_hash`
- **HTTP route:** None new — rides existing FeedbackSignal EPR ingest path
- **Anti-pattern check:** ✓ No new entry type (Lamad ~73/100 preserved); ✓ identity is CID; ✓ coordinator-first; ✓ HTTP route unchanged

### Design constraints discovered

1. **FeedbackSignal schema extension required:** add `vouch` to the `signal_kind` enum and an optional `vouch_kind: "accept-correction" | "restitution"` sub-field. Schema-first IoC: schema → Rust mirrors → integrity validator whitelist → bootstrap manifest debit weights.

2. **Zome location:** Vouch lives in `content_store` (where FeedbackSignal lives), NOT `lamad`. Cross-zome variants of an entry type are not viable. A future refactor sprint may move FeedbackSignal from `content_store` to `mishpat` (semantically governance), but that's out of scope.

3. **No-self-vouch is enforceable at integrity layer** — `must_get_valid_record(target_cid)` is deterministic and HDI-compatible. No `get_links` needed.

4. **Forward-compat schema:** `unknownTreatment.evidenceSources: []` array starts empty (FeedbackSignal-only). Future sprint adds imagodei/lamad bridges. Per memory: standing composes from multiple evidence streams.

5. **DNA touch is small but real:** integrity validator whitelist gains `"vouch"`. Sweettest must validate on Jenkins (Eclipse Che cannot run sweettests per memory `feedback_shift_measure_jenkins`).

## Components

### ReachVerdict — `services/reach_earning.rs`

```rust
pub enum ReachVerdict {
    Allowed { floor_class_match: Option<FloorClass>, evidence: StandingEvidence },
    Blocked { reason: BlockReason, evidence: StandingEvidence },
    Pending { reason: PendingReason, evidence: StandingEvidence },
}

pub struct StandingEvidence {
    pub standing: Standing,
    pub debit_weight_sum: i32,
    pub last_signal_at: Option<i64>,
    pub signal_count: u32,
}

pub enum BlockReason {
    QuarantineActive,
    FloorBreach { class: FloorClass },
    StandingBelowThreshold,
    UnknownReach,        // fail-closed for unknown reach values
}

pub enum PendingReason {
    UnknownAuthorAtNonFloorReach,
    NewVoiceWithoutSponsor,
}

pub enum FloorClass {
    CidTargetedLookup,
    NewVoiceBaseline,
    VulnerableClassElevation,
    LocalRelationshipReach,
    ConstitutionalFloorSignatures,
}
```

`Pending` collapses to `Blocked` for substrate-only callers in this sprint. The verdict shape is forward-compatible for the future elohim-mediated discernment layer that may consume `Pending` and produce sponsor suggestions.

### Standing-policy schema extension (`bootstrap-standing-policy.schema.json`)

```jsonc
{
  "manifestKind": "standing-policy",
  "revision": 1,
  "floor": { /* unchanged from T17 */ },
  "debitWeights": {
    "squelch":     { "advisory": 0, "debit-soft": 1,  "debit-firm": 3 },
    "correction":  { "advisory": 0, "debit-soft": 10, "debit-firm": 20 },
    "retraction":  { "advisory": 0, "debit-soft": -5, "debit-firm": -10 },
    "quarantine":  { "advisory": 0, "debit-soft": 12, "debit-firm": 30 },
    "vouch":       { "advisory": 0, "debit-soft": -3, "debit-firm": -8 }   // NEW: negative = recovery
  },
  "unknownTreatment": {                                                     // NEW
    "default": "conservative",
    "evidenceSources": []                                                   // forward-compat
  },
  "reachThresholds": {                                                      // NEW
    "personal":     "any",
    "intimate":     "any",
    "household":    "any",
    "neighborhood": "any",
    "collective":   "neutral",
    "community":    "neutral",
    "district":     "neutral",
    "public":       "high"
  },
  "newVoiceBaseline": { /* unchanged from T17 */ }
}
```

### `ManifestDebitWeightPolicy` — `services/standing_projector.rs`

```rust
pub struct ManifestDebitWeightPolicy {
    weights: HashMap<(SignalKind, StandingImpact), i32>,
    fallback: DefaultDebitWeightPolicy,
}

impl ManifestDebitWeightPolicy {
    pub fn from_registry(registry: &ManifestRegistry) -> Self { ... }
}

impl DebitWeightPolicy for ManifestDebitWeightPolicy {
    fn debit_weight(&self, kind: SignalKind, impact: StandingImpact) -> i32 {
        self.weights.get(&(kind, impact))
            .copied()
            .unwrap_or_else(|| self.fallback.debit_weight(kind, impact))
    }
}
```

Construction reads the bootstrap-standing-policy manifest payload; falls back to defaults when no manifest is registered. Hot-reload deferred — registry rebuild on manifest-update signal is sufficient.

### `LibP2POutboundSink` and `LibP2PGossipPublisher` — `p2p/adapters.rs`

```rust
pub struct LibP2POutboundSink { tx: mpsc::Sender<P2PCommand> }

impl OutboundSink for LibP2POutboundSink {
    fn send(&self, peer_id: &str, payload: Vec<u8>) -> Result<(), SinkError> {
        let peer = PeerId::from_str(peer_id).map_err(SinkError::InvalidPeer)?;
        self.tx.try_send(P2PCommand::SendDirect { peer, payload })
            .map_err(|e| match e {
                TrySendError::Full(_)   => SinkError::Backpressure,
                TrySendError::Closed(_) => SinkError::SwarmGone,
            })
    }
}

pub struct LibP2PGossipPublisher { tx: mpsc::Sender<P2PCommand> }

impl GossipPublisher for LibP2PGossipPublisher {
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
        self.tx.try_send(P2PCommand::GossipPublish { topic: topic.to_string(), payload })
            .map_err(|e| match e {
                TrySendError::Full(_)   => PublishError::Backpressure,
                TrySendError::Closed(_) => PublishError::SwarmGone,
            })
    }
}
```

The swarm task handles `P2PCommand::GossipPublish` via `behaviour.gossipsub.publish(topic, payload)` and `P2PCommand::SendDirect` via the existing direct-notify protocol. If `SendDirect` and `GossipPublish` variants don't yet exist on `P2PCommand`, they're added with their handlers in the swarm event loop.

### Vouch wire shape

**Schema extension to `feedback-signal.schema.json`:**

```jsonc
{
  "signalKind": { "enum": ["squelch", "correction", "retraction", "quarantine", "vouch"] },
  "vouchKind":  { "type": "string", "enum": ["accept-correction", "restitution"], "optional": true }
}
```

Constraint: `vouchKind` required iff `signalKind == "vouch"`. Validated in schema and at the integrity zome.

**Coordinator (`content_store::create_vouch`):**

```rust
#[hdk_extern]
pub fn create_vouch(input: CreateVouchInput) -> ExternResult<ActionHash> {
    let target = must_get_valid_record(input.target_signal_cid.clone().into())?;
    let target_signal: FeedbackSignal = target.entry().to_app_option()?
        .ok_or(wasm_error!(WasmErrorInner::Guest("target not a FeedbackSignal".into())))?;

    let signer = agent_info()?.agent_initial_pubkey;
    if signer.get_raw_39() == target_signal.signed_by.as_slice() {
        return Err(wasm_error!(WasmErrorInner::Guest("self-vouch forbidden".into())));
    }

    let signal = FeedbackSignal {
        target_cid: input.target_signal_cid.to_string(),
        signal_kind: "vouch".into(),
        vouch_kind: Some(input.vouch_kind.to_string()),
        evidence_cid: None,
        standing_impact: input.impact.to_string(),
        signed_by: signer.get_raw_39().to_vec(),
        signature: sign_payload(&signer, &input)?,
    };
    create_entry(EntryTypes::FeedbackSignal(signal))
}
```

**Integrity validator addition** (in `content_store_integrity::validate_feedback_signal`):

When `signal_kind == "vouch"`:
- Require `vouch_kind ∈ {"accept-correction", "restitution"}`
- Reject if `signed_by == must_get_valid_record(target_cid).signed_by` (no self-vouch)

When `signal_kind != "vouch"`:
- `vouch_kind` must be absent

### Reach-earning gate — `services/reach_earning.rs`

```rust
pub fn evaluate(
    author: &AgentPubkey,
    requested_reach: Reach,
    conn: &mut SqliteConnection,
    registry: &ManifestRegistry,
) -> ReachVerdict {
    // 1. Floor class allow: cid-targeted-lookup, local-relationship-reach
    if requested_reach.is_floor_allowed() {
        return ReachVerdict::Allowed {
            floor_class_match: Some(...),
            evidence: ...,
        };
    }

    // 2. Quarantine check
    if registry.is_quarantined(author) {
        return ReachVerdict::Blocked { reason: BlockReason::QuarantineActive, evidence };
    }

    // 3. Vulnerable-class lift
    let lift = registry.vulnerable_class_lift(author);

    // 4. Standing evaluation (evaluator = local agent)
    let standing = Standing::evaluate(local_agent, author, conn);
    let effective = standing.with_lift(lift);

    // 5. Required threshold from manifest
    let required = match registry.reach_threshold(requested_reach) {
        Some(t) => t,
        None => return ReachVerdict::Blocked { reason: BlockReason::UnknownReach, evidence },
    };

    // 6. Apply UnknownTreatment policy
    match (effective, required) {
        (Standing::Unknown, _) => match registry.unknown_treatment() {
            UnknownTreatment::Conservative   => ReachVerdict::Pending { reason: UnknownAuthorAtNonFloorReach, evidence },
            UnknownTreatment::NewVoiceBaseline => evaluate_with_score(registry.new_voice_baseline(), required, evidence),
            UnknownTreatment::Neutral        => evaluate_with_score(StandingScore::Neutral, required, evidence),
        },
        (Standing::Computed { score }, threshold) => evaluate_with_score(score, threshold, evidence),
    }
}
```

### `epr_compose` helper — `services/epr_compose.rs`

```rust
pub fn compose_epr(
    author: &AgentPubkey,
    epr: &EprPayload,
    requested_reach: Reach,
    conn: &mut SqliteConnection,
    registry: &ManifestRegistry,
) -> Result<ComposedEpr, ComposeError> {
    let verdict = reach_earning::evaluate(author, requested_reach, conn, registry);
    match verdict {
        ReachVerdict::Allowed { .. } => Ok(ComposedEpr { epr: epr.clone(), verdict }),
        ReachVerdict::Pending { .. } | ReachVerdict::Blocked { .. } => {
            Err(ComposeError::ReachDenied(verdict))
        }
    }
}
```

Author-side compose path goes through this helper. Receive path (`put_epr` for external EPRs) does NOT call it — gating receive is meaningless once the EPR has arrived; we project standing as evidence, that's all.

### Tending TTL sweep task (`main.rs`)

```rust
let shutdown = shutdown_token.clone();
let pool = pool.clone();
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(300));
    loop {
        interval.tick().await;
        if shutdown.is_cancelled() { break; }
        let mut conn = match pool.get() { Ok(c) => c, Err(e) => { tracing::warn!(?e, "pool"); continue; } };
        if let Err(e) = tending::sweep_expired(&mut conn) {
            tracing::warn!(?e, "tending sweep failed");
        }
    }
});
```

`tending::sweep_expired` is idempotent: `DELETE FROM attention_tending WHERE classification != 'safety' AND tended_at + ttl_seconds*1000 < unix_ms_now()`. Safety classification is constitutionally floor-protected and never deleted (already enforced at SQL filter level — preserve in any production wiring).

### Small additions to existing types

The reach-earning gate code sketch references two new helper methods on existing types:

- `Standing::with_lift(self, lift: Option<StandingScore>) -> Standing` — applies a vulnerable-class baseline lift; if `Standing::Unknown`, returns `Computed { score: lift }`; if `Computed { score }`, returns `Computed { score: max(score, lift) }`.
- `Reach::is_floor_allowed(&self) -> bool` — returns `true` for reach values that map to "any" in `reachThresholds` (personal, intimate, household, neighborhood). Convenience accessor.

Both are pure functions on existing enums — small, additive, no DNA touch.

### Bootstrap seed placement (`main.rs`)

```rust
run_migrations(&mut conn)?;
let report = bootstrap_manifests::seed_if_empty(&mut conn)?;
tracing::info!(?report, "bootstrap manifests");
let registry = ManifestRegistry::load_from_db(&mut conn)?;
let policy = ManifestDebitWeightPolicy::from_registry(&registry);
// ... spawn p2p task → adapters → spawn TTL sweep → HTTP server
```

If `seed_if_empty` fails to parse the embedded JSON (developer error), startup aborts with `BootstrapError::PolicyParseFailed`. We do not run with no policy when the developer thought one was being seeded.

## Data Flow

### Flow 1 — FeedbackSignal arrival

```
External peer sends FeedbackSignal EPR
   │
   ▼
api/epr.rs::put_epr  (PUT /api/v1/epr, kind == FeedbackSignal)
   │
   ├─ dedup: if envelope.signed_by == local_peer_id, skip fan-out  (loop guard)
   │
   ├─ persist envelope to blob_store (existing path)
   │
   ├─ wrap in transaction:
   │    ├─ back_prop::record_predecessor(conn, target_cid, sender_peer_id, sealing_keys)
   │    └─ standing_projector::project_signal(conn, &policy, evaluator, &signal, manifest_cid)
   │
   ├─ back_prop::back_prop_one_hop(conn, &signal, &outbound_sink)
   │     reads predecessor records for target_cid; unseals; for each predecessor:
   │         outbound_sink.send(predecessor_peer_id, signal_payload)
   │     no chain on wire — single hop only
   │
   └─ gossip_flood::flood_feedback(&signal, reach_topic, &gossip_publisher)
         publisher.publish(reach_topic_for(target_cid), signal_payload)
         receivers dedup via signal CID
```

The DB writes (predecessor record + standing projection) are wrapped in a single transaction so partial failure rolls back. Network publishes are best-effort — log and continue.

### Flow 2 — Vouch creation

```
Subject (Sarah) decides to vouch on target's restitution
   │
   ▼
HTTP / app-WebSocket → content_store::create_vouch
   │
   ├─ must_get_valid_record(target_signal_cid) → original FeedbackSignal
   ├─ derive signer = agent_info().agent_initial_pubkey
   ├─ guard: signer != original.signed_by  (validator backstop)
   ├─ construct FeedbackSignal { signal_kind: "vouch", vouch_kind, target_cid, ... }
   ├─ create_entry → ActionHash
   └─ post_commit emits Signal::FeedbackSignalCommitted
   │
   ▼
storage projection signal handler  (existing, unchanged)
   ├─ INSERT into feedback_signals projection (with dht_anchor_hash)
   └─ project_signal → standing_view  (vouch.debit_soft = -3 by manifest)
                                       → subject's debit_weight_sum decreases
                                       → standing recovers per threshold
```

Vouch travels through the existing FeedbackSignal infrastructure — no new HTTP route, no new projection table, no new gossip topic. The projector's manifest-driven debit weight is the only thing that distinguishes vouch from other kinds.

### Flow 3 — Reach-earning gate on author compose

```
Author composes EPR for reach=district
   │
   ▼
services/epr_compose::compose_epr
   │
   ▼
services/reach_earning::evaluate
   │
   ├─ floor class check → Allowed if matched
   ├─ quarantine check → Blocked if quarantined
   ├─ vulnerable-class lift
   ├─ Standing::evaluate → reads standing_view
   ├─ apply UnknownTreatment policy if Unknown
   ├─ compare effective standing vs reachThresholds[requested_reach]
   └─ return ReachVerdict
   │
   ▼
ComposeEpr (Allowed)               → continue to PUT /api/v1/epr
ComposeError::ReachDenied (B|P)    → return 4xx with structured reason
```

### Flow 4 — Startup wiring

```
main.rs
  ├─ run migrations
  ├─ bootstrap_manifests::seed_if_empty(conn)         ← T17 wiring
  ├─ ManifestRegistry::load_from_db(conn)
  ├─ ManifestDebitWeightPolicy::from_registry(reg)    ← replaces DefaultDebitWeightPolicy
  ├─ spawn p2p task (existing, with P2PCommand mpsc channel)
  ├─ construct LibP2POutboundSink + LibP2PGossipPublisher (with channel handles)
  ├─ spawn TTL sweep task (5-min, idempotent, shutdown-aware)   ← T15 wiring
  ├─ HolochainAppSignalStream subscribed → ReconcileController
  └─ HTTP server bind (with shared state: registry, policy, sinks)
```

`seed_if_empty` and TTL sweep are both idempotent at the SQL level — restart-safe by construction.

## Error Handling

### Network publish errors (best-effort)
- `LibP2POutboundSink::send` → `SinkError::{Backpressure, SwarmGone, InvalidPeer}`
- `LibP2PGossipPublisher::publish` → `PublishError::{Backpressure, NoPeers, SwarmGone}`

**Policy:** log at `warn!`, continue. The DHT remains canonical (P1 reconciliation controller). Substrate self-heals via re-gossip on next sweep, predecessor-record replay, or normal Kad provider records when peers walk schemarefs.

### DB-write errors (transactional)
`back_prop::record_predecessor` + `standing_projector::project_signal` + `feedback_signals` insert wrap in a single `conn.transaction(|tx| { ... })`. Partial failures roll back. Caller gets `5xx` and may retry.

### Manifest absence (graceful fallback)
- `ManifestDebitWeightPolicy` falls back to `DefaultDebitWeightPolicy`.
- `reach_threshold(reach)` returns `None` → gate falls back to a hard-coded conservative table.
- `unknown_treatment()` falls back to `Conservative`.
- All paths are safe-by-default.

### Validator rejections (Holochain side)
- Malformed enum, self-vouch, missing `vouch_kind` for vouch, missing `evidence_cid` for correction → `WasmError::Guest`.
- Coordinator returns `ExternResult::Err`. HTTP wrapper returns `422 Unprocessable Entity`.

### Reach-earning gate edge cases
- Reach value not in `reachThresholds` map → `Blocked { UnknownReach }` (fail-closed).
- Author pubkey malformed → `ComposeError::InvalidAuthor`.
- Concurrent FeedbackSignal arriving mid-evaluation that flips the verdict → race accepted; gate evaluates a snapshot; next compose re-evaluates.

### TTL sweep errors
- `tending::sweep_expired` returns `Result<usize, StorageError>`. On error: log, continue. **Never panic** the long-lived task.
- Cooperative shutdown via `shutdown_token.is_cancelled()`.

### Bootstrap seed errors
- Idempotent via existing `manifest_kind` check. Running twice is a no-op.
- Embedded JSON parse failure (developer broke the bundled schema): `BootstrapError::PolicyParseFailed` → main.rs aborts startup. We do not run with no policy.

### Sealed-decrypt failures
- `back_prop_one_hop` unseals predecessor blobs. Decrypt failure (corrupted, key rotated, mishpat-quorum mismatch) → log at `warn!`, skip predecessor, continue with others. T20's 2-of-2 negative assertion guarantees observability.

### HTTP semantics
- `200 OK` — EPR persisted; fan-out best-effort
- `4xx` — author-side compose: gate verdict, validator failure
- `5xx` — DB transaction failure, manifest parse failure
- Never `5xx` for swarm publish failures — best-effort

**Failure-mode invariant:** A successful 200 means the FeedbackSignal is persisted locally and `standing_view` is updated. Network propagation is best-effort and self-healing via Phase 3.5's gossip-flood + predecessor-records mechanisms. The DHT remains the canonical record.

## Testing

### T20 mock-lifting strategy

Spin up real swarm nodes via `harness_d8::spawn_d8_node` for Bob, Aunt, Sarah. Each node has a real swarm with `LibP2POutboundSink` + `LibP2PGossipPublisher` wired to its P2PCommand channel.

1. Bob's authoring → `services/epr_compose::compose_epr` → real `PUT /api/v1/epr` → real persistence. *No direct service calls.*
2. Sarah's correction → `compose_epr` (Sarah's standing > Bob's, Allowed) → `PUT /api/v1/epr` with FeedbackSignal payload.
3. Fan-out fires automatically inside `put_epr`. Aunt's node receives via her swarm. Standing on each node updates from real network arrival.
4. Bob's recovery attempt → `compose_epr(reach=district)` → gate consults *now-projected* standing → returns `Blocked { StandingBelowThreshold }`. **Mock #1 lifted.**
5. Bob's restitution → `PUT /api/v1/epr` with Correction acknowledging Sarah.
6. Sarah's vouch → app-WebSocket call → coordinator `create_vouch(target=Bob's-restitution-cid, vouch_kind=AcceptCorrection)` → real DHT entry → real signal → projects to Bob's standing_view. Bob's `debit_weight_sum` decreases (vouch.debit_soft = -3). Bob's `compose_epr(reach=district)` now returns `Allowed`. **Mock #2 lifted.**

The two `MOCKED STEP` comments come out of `tests/aunt_and_rage_bait_integration.rs`. The 2-of-2 negative sealed-decrypt assertion stays.

### New unit tests

**`services/reach_earning.rs`:**
- floor class allows: cid-targeted-lookup → Allowed regardless of standing
- floor class allows: local-relationship-reach → Allowed
- quarantine blocks: quarantined author at any reach → Blocked
- Standing::Unknown + UnknownTreatment::Conservative → Pending(UnknownAuthorAtNonFloorReach)
- Standing::Unknown + UnknownTreatment::NewVoiceBaseline → effective standing = baseline
- Standing::Unknown + UnknownTreatment::Neutral → effective Neutral
- Standing::Computed(Floor) at reach=public → Blocked(StandingBelowThreshold)
- Standing::Computed(High) at reach=public → Allowed
- Vulnerable-class lift moves Floor → Low
- Reach value not in map → Blocked(UnknownReach) — fail-closed invariant
- Manifest absent → falls back to hard-coded conservative table

**`services/standing_projector.rs::ManifestDebitWeightPolicy`:**
- registry-with-manifest returns manifest weights
- registry-without-manifest falls back to DefaultDebitWeightPolicy
- vouch.debit_soft yields negative (-3) → standing_view debit_sum decreases on project
- transaction rollback on partial failure — feedback_signals + standing_view atomic

**`services/tending.rs::sweep_expired`:**
- safety-classified rows never deleted (regression test for SQL filter)
- expired non-safety rows deleted
- non-expired rows preserved
- idempotent: second call deletes 0 rows

**`p2p/adapters.rs`:**
- `LibP2POutboundSink::send` with closed channel → `SinkError::SwarmGone`
- `LibP2PGossipPublisher::publish` with full channel → `PublishError::Backpressure`
- malformed peer_id → `SinkError::InvalidPeer`

### Sweettest (Holochain DNA side)

**`content_store_integrity` validator extension (Vouch validation):**
- valid vouch (signer ≠ target.signed_by, vouch_kind set) → accepted
- self-vouch (signer == target.signed_by) → rejected with `Guest("self-vouch forbidden")`
- vouch missing vouch_kind → rejected
- non-vouch with vouch_kind set → rejected
- target_signal_cid pointing at non-FeedbackSignal record → rejected

**`content_store::create_vouch` coordinator:**
- creates entry with derived signer_pubkey (caller cannot spoof)
- post_commit emits FeedbackSignalCommitted projection signal
- two-agent test: Sarah vouches Bob → entry visible on DHT → Bob's storage projection projects with vouch debit weight

Sweettest validation runs on Jenkins per memory `feedback_shift_measure_jenkins`.

### Smoke / startup tests

**`tests/startup_wiring.rs` (or main.rs integration):**
- start storage with empty DB → `seed_if_empty` runs → standing-policy + tending-policy manifests present
- restart same storage → `seed_if_empty` runs → no duplicate inserts
- TTL sweep task ticks at least once within test window → expired fixtures cleared
- shutdown_token cancellation → sweep task exits cleanly within 1s

### CI quality gates

- `RUSTFLAGS="" cargo build --release` clean (per CLAUDE.md gotcha)
- `RUSTFLAGS="" cargo clippy -- -D warnings` clean
- `cargo fmt --check` clean
- All new unit tests + lifted T20 pass on the worktree branch
- Sweettest validation on Jenkins (DNA changes — vouch validator + signal_kind whitelist)
- `pnpm run schema:validate` clean for extended `feedback-signal.schema.json` and `bootstrap-standing-policy.json`
- `pnpm run schema:check-dna` clean (vouch enum mirrored in DNA constants)

## Constitutional Guardrails (carry forward)

- **Reach-earning gate** honors §2.8 floor classes. CID-targeted-lookup is unconditional. New-voice-baseline is non-zero. Vulnerable-class-elevation lifts baseline.
- **Vouch primitive** must NOT let the subject self-vouch (peer attestation is the load-bearing property — confirmed via integrity validator).
- **Tending TTL sweep** must NEVER delete safety classification (already enforced at SQL filter level — preserved in production wiring).
- **DHT remains canonical.** Projections are read-optimized views. Network publishes are best-effort. The substrate self-heals.

## Open Questions Deferred

- **FeedbackSignal zome relocation:** semantically governance (mishpat), currently in content_store. Future refactor sprint.
- **Hot-reload of manifest-driven policies:** registry rebuild on manifest-update signal will work for now; explicit hot-reload is a future enhancement.
- **`evidenceSources` bridge:** imagodei profile/psyche + lamad recognition feed standing alongside FeedbackSignal — schema is forward-compat, bridge is a future sprint.
- **Elohim discernment layer:** `Pending` verdicts → matchmaking/sponsor suggestions. Substrate gate stops at `Pending`; the discernment layer is a separate sprint.

## References

- [Phase 3.5 trust-compute gradient plan](../plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md)
- [Substrate brainstorm](2026-04-30-trust-compute-gradient-brainstorm.md) — §2.8 floor classes, §6.4 collective-wisdom aggregation, Appendix B aunt-and-rage-bait scenario
- Memory: `project_principle_p1_reconciliation_controller`
- Memory: `project_reach_gate_is_elohim_mediated_matchmaking`
- Memory: `project_signal_kind_extensible_protocol_class`
- Memory: `project_standing_composes_multiple_evidence_streams`
- Memory: `feedback_schema_first_ioc`
- Memory: `feedback_shift_measure_jenkins`
- Memory: `project_hdi_no_get_links_in_validators`
