# Recovery Protocol Phase 2 M3 — Coordinator, Mesh Substrate, Storage Projection

**Status:** Design approved, ready for implementation planning
**Date:** 2026-04-24
**Supersedes:** the seven enumerated gaps in `genesis/docs/plans/2026-04-22-m3-session-kickoff-prompt.md`
**Predecessor:** `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` (§9 M3 row, as amended by commit `14192752`)

## 1. Purpose

M3 is the milestone where recovery rotations can actually happen end-to-end. It delivers the **plumbing**: DNA coordinator correctness, libp2p mesh invitation substrate, and storage projection for witness accumulation. The test corpus exercises cross-cell flows and cross-doorway topology.

M3 explicitly does **not** design:
- The account/login layer (deferred — see §11; shape captured in memory `project_peer_native_account_canonical_surface`).
- The hosted-conductor bootstrap ceremony for browser claimants (M5).
- Peer-native recovery initiation UX (M5+).
- Elohim defender / specialist evaluation (M5).

The design principle guiding M3: **ship plumbing that forecloses nothing.** Any cell that can invoke coordinator functions — tauri-local, doorway-hosted, sweettest harness — drives the flow identically. Any elohim-storage pod on the mesh publishes and subscribes identically. Future account-layer design (whether graduation-based OAuth-pattern or otherwise) consumes this substrate unchanged.

## 2. Resiliency story — the framing

Every design decision below traces back to preserving these protocol properties:

- **DHT is the notary.** Integrity and authoritative state live on the DHT.
- **libp2p mesh is data-ops.** Content distribution, invitations, gossip, sharded replication. Canonically hosted by elohim-storage pods, one per deployment.
- **Doorway is web2 projection.** Routes, proxies, CDN, convenience facilitation. **Not a P2P participant**; a doorway-steward's P2P participation lives in their colocated elohim-storage pod.
- **Humans register with multiple doorways.** No "the human's doorway"; a human's identity is reachable through any registered doorway.
- **Data replicates across doorway-stewards' elohim-storage pods.** Losing access to one doorway does not lose the human's history; peer-sharded backups form the view.
- **Account layer is deferred.** Plumbing stays initiator-agnostic.

See memories: `project_three_layer_truth_model`, `project_multi_doorway_human_registration`, `project_peer_native_account_canonical_surface`, `project_dht_vs_libp2p_scoping`.

## 3. Architecture

Three tiers, signal-driven between them, entirely doorway-agnostic:

```
┌─────────────────────────────────────────────────────────────┐
│ imagodei DNA (Holochain) — the notary                       │
│   integrity: RecoveryRequest, KeyRotation, HumanityWitness, │
│              IdentityFreeze, new link types                 │
│   coordinator: create_recovery_request (bridging),          │
│                commit_key_rotation (freeze-gate),           │
│                submit_intimate_witness (new)                │
│                → emits RecoveryV2Signal (rich payloads)     │
└──────────────┬───────────────────────────┬──────────────────┘
               │ cell signals               │ cell signals
               ▼                            ▼
     ┌────────────────────────────┐  ┌──────────────────────┐
     │ elohim-storage (libp2p)    │  │ elohim-storage       │
     │  reuse EPR-2C swarm:       │  │  (diesel projection) │
     │  + gossipsub behaviour     │  │  recovery_requests,  │
     │  + recovery.invitation     │  │  recovery_witnesses  │
     │  + CBOR codec              │  │  schema-first views  │
     │  + signal→publish bridge   │  │  camelCase wire      │
     │  + subscribe/log stub      │  │                      │
     └────────────────────────────┘  └──────────────────────┘
                   ▲
                   │ (doorway is absent from M3 changes;
                   │  existing app_ws proxy carries cell
                   │  calls through unchanged)
                   │
            ┌──────┴──────────┐
            │ doorway         │ web2 projection only — no libp2p,
            │ (no M3 changes) │ no recovery-layer code this phase
            └─────────────────┘
```

**Trust boundary:** the DNA coordinator is authoritative. Storage is projection (cached view, rebuildable from DHT). libp2p gossip is discovery (lost messages are recoverable; the DHT has the truth). No layer below DHT can override a coordinator rejection.

**Multi-doorway reality:** the claimant walks into any of their registered doorways; the invitation gossip-fans to every elohim-storage pod in the mesh (including the pods colocated with contacts' other doorways); witnesses can respond from whichever cell is alive. The ceremony is location-independent because the DHT and mesh are.

**Stage posture:** Stage 1 structural enforcement (see §8). Witnesses are humans tapping confirm; elohim-specialist evaluation lands in M5.

## 4. DNA changes

### 4.1 New link types

Register in imagodei integrity zome `LinkTypes`:

- `RecoveryRequestToHumanityWitness` — `ActionHash(RecoveryRequest) → ActionHash(HumanityWitness)`. Powers IntimateQuorum.
- `RecoveryRequestToKeyStewardship` — `ActionHash(RecoveryRequest) → ActionHash(KeyStewardship)`. Reserved for CryptographicQuorum; no M3 coordinator creates it, but the integrity type lands now so M4/M5 can use it without re-migrating.

Link-validation stubs return `Ok(())` (structural) pending Stage-2 elohim-attested validation.

### 4.2 `create_recovery_request` — bridging population

**Input:** unchanged from M2 (`human_agent_pubkey: AgentPubKey`).

**Coordinator logic:**
1. Resolve `human_id` via `Agent` entry lookup on `input.human_agent_pubkey`. Bail if no Agent entry exists.
2. Count the human's active `HumanRelationship` entries with `emergency_access_enabled = true`. Call that M.
3. Compute `required_witness_count = max(2, ceil(M / 2) + 1)`.
4. Commit `RecoveryRequest { human_id: Some(resolved), required_witness_count: computed, ... }`.
5. Create `HumanToRecoveryRequest` link from `StringAnchor("recovery_request", human_id) → RecoveryRequest`. **Anchor is human_id-keyed. No pubkey anchor.** See §12 decision log.
6. Emit `RecoveryV2Signal::RecoveryRequestCreated { request_hash, human_id, human_agent_pubkey, required_witness_count, created_at }` (rich payload).

### 4.3 `commit_key_rotation` — freeze-floor gate

**Input:** unchanged from M2.

**Coordinator logic (new gate, before `create_entry(KeyRotation)`):**
1. **Exemption check:** if `rotation.authority == CryptographicQuorum`, skip the freeze-floor gate entirely (cryptographic layer is orthogonal per spec §1.1).
2. Traverse freezes via `HumanToFreeze` anchor (human_id-keyed, consistent with §4.2), filtered to `is_active = true`.
3. Call `check_freeze_floor_rules(&authority, &human_id, &active_freezes)` — the M2 pure helper.
4. Bail with descriptive error on blocker.
5. Proceed to existing M2 commit logic.

The gate goes live structurally even before defenders author freezes in volume. M5 populates the freeze surface; the gate pre-exists.

### 4.4 `submit_intimate_witness` — new coordinator function

**Signature:**
```rust
pub fn submit_intimate_witness(input: SubmitIntimateWitnessInput) -> ExternResult<Record>;

pub struct SubmitIntimateWitnessInput {
    pub recovery_request_hash: ActionHash,
    pub note: Option<String>,
}
```

**Caller:** the emergency-contact authorizer's cell (their device, their agent key, their elohim's judgment — once elohim-specialists land in M5). Not doorway-proxied. Consistent with spec §6.2 step 8 and memory `project_socially_derived_security`.

**Coordinator logic (pre-commit gates):**
1. Fetch `RecoveryRequest` via `must_get_entry(recovery_request_hash)`. Extract `human_id`.
2. **Emergency-contact membership check:** traverse the target human's active `HumanRelationship` entries, confirm `agent_info().agent_pubkey` (the authorizer) is on one with `emergency_access_enabled = true`. Bail if not.
3. **Dedupe check:** traverse `RecoveryRequestToHumanityWitness` links outgoing from the request; confirm no existing witness is authored by `agent_info().agent_pubkey`. Bail if duplicate.
4. Commit `HumanityWitness { human_id, witness_agent_id: agent_info().agent_pubkey.to_string(), note, revoked_at: None, ... }`.
5. Create `RecoveryRequestToHumanityWitness` link.
6. Emit `RecoveryV2Signal::IntimateWitnessSubmitted { request_hash, witness_hash, witness_agent_id, human_id, note, submitted_at }` (rich payload).

The emergency-contact membership check is the key Stage-1 structural guardrail against social-engineering pile-on. See §8.

### 4.5 Signal enum amendments

Extend `RecoveryV2Signal`:

```rust
pub enum RecoveryV2Signal {
    RecoveryRequestCreated {
        request_hash: ActionHash,
        human_id: String,
        human_agent_pubkey: AgentPubKey,
        required_witness_count: u32,
        created_at: Timestamp,
    },
    IntimateWitnessSubmitted {
        request_hash: ActionHash,
        witness_hash: ActionHash,
        witness_agent_id: AgentPubKey,
        human_id: String,
        note: Option<String>,
        submitted_at: Timestamp,
    },
    KeyRotationCommitted { /* existing M2 fields, unchanged */ },
}
```

Rich (denormalized) payloads so projections and gossip bridges can act from the signal alone without secondary DHT reads. Pattern alignment with EPR-2C.

**Cross-cell delivery:** Holochain's `emit_signal` fires to local subscribers only. For cross-cell delivery to the colocated elohim-storage pod, the existing signal-plumbing M1/M2 established is reused. If `remote_signal` fan-out is needed for notifying multi-doorway watchers, that pattern is adopted (subagent verifies precedent).

## 5. Mesh substrate — elohim-storage gossipsub

All libp2p work lands in **elohim-storage**, reusing the EPR-2C swarm. No new crate, no new swarm, no doorway libp2p.

### 5.1 Additions

- **New gossipsub behaviour** added to the existing EPR-2C swarm. libp2p 0.54, consistent transport and identity config.
- **Wire contract file** — `elohim/elohim-storage/src/libp2p/wire/recovery_invitation.rs` (or mirror of EPR-2C's directory):
  ```rust
  #[derive(Serialize, Deserialize)]
  pub struct RecoveryInvitation {
      pub request_hash: ActionHash,
      pub human_id: String,
      pub created_at: Timestamp,
  }
  ```
  CBOR-serialized via ciborium (same codec convention as EPR-2C).
- **Topic:** single broadcast topic `recovery.invitation`. All elohim-storage pods subscribe on startup. Subscribers filter client-side in M5 when the elohim specialist is the consumer.
- **Signal bridge:** elohim-storage subscribes to `RecoveryV2Signal::RecoveryRequestCreated`. On receipt: construct `RecoveryInvitation`, CBOR-encode, publish to topic.
- **Subscribe stub:** on topic message, decode and log (`target = "elohim_storage::recovery", message = "received invitation human_id=... request_hash=..."`). No projection, no routing. M5 plugs the elohim specialist in here.

### 5.2 Out of scope for M5 plumbing

- Per-request topic sharding (`recovery.invitation.{request_hash}` or `recovery.invitation.{human_id}`) — single broadcast topic is appropriate for M3 substrate.
- Hashcash / proof-of-work gating on publishes — M5.
- Signed payloads — M5.
- Routing to the right elohim specialist — M5.

## 6. Storage projection

Schema-first, per memory `feedback_schema_first_ioc`.

### 6.1 JSON schemas

Write first, in `elohim/sdk/schemas/v1/views/`:

- `recovery-request.schema.json`:
  ```
  {
    "requestHash": "string (ActionHash base64)",
    "humanId": "string",
    "humanAgentPubkey": "string (AgentPubKey base64)",
    "requiredWitnessCount": "integer >= 2",
    "createdAt": "string (ISO8601)"
  }
  ```
- `recovery-witness.schema.json`:
  ```
  {
    "witnessHash": "string (ActionHash base64)",
    "requestHash": "string (ActionHash base64)",
    "witnessAgentId": "string (AgentPubKey base64)",
    "humanId": "string",
    "note": "string | null",
    "submittedAt": "string (ISO8601)"
  }
  ```

### 6.2 Rust views

Hand-written in `elohim/elohim-storage/src/views/recovery.rs` with `#[serde(rename_all = "camelCase")]`. Schema-contract tests in `elohim/elohim-storage/tests/schema_contract.rs` enforce shape parity per repo convention.

### 6.3 Diesel migrations + projection handlers

Two new tables (`recovery_requests`, `recovery_witnesses`); migrations in `elohim/elohim-storage/migrations/`. Projection handlers:

- `RecoveryRequestCreated` handler → INSERT into `recovery_requests` (idempotent on `request_hash`).
- `IntimateWitnessSubmitted` handler → INSERT into `recovery_witnesses` (idempotent on `witness_hash`).

No denormalized counter column. Frontend queries `count(*)` from `recovery_witnesses WHERE request_hash = ?`. Witnesses per request are single-digit; indexed request_hash makes count trivial.

### 6.4 TS codegen

`cargo test export_bindings` regenerates TypeScript types in `elohim/sdk/storage-client-ts/src/generated/`.

## 7. Data flow — happy path

1. Claimant opens any of their registered doorways, initiates recovery. Existing doorway HTTP proxy carries the request through to a cell on a conductor (hosted-cell bootstrap is M5; for M3 testing this is either a sweettest harness or a direct app_ws call).
2. Cell invokes `create_recovery_request { human_agent_pubkey: <lost-pubkey> }`.
3. Coordinator resolves `human_id`, computes `required_witness_count = 3`, commits, links via human_id anchor, emits `RecoveryRequestCreated` (rich).
4. Colocated elohim-storage receives signal: CBOR-publishes `RecoveryInvitation` to `recovery.invitation` topic; simultaneously projection handler INSERTs `recovery_requests` row.
5. Every other elohim-storage pod on the mesh receives the gossipsub message — including pods at other doorways where the human's emergency contacts are registered. M3: logged. M5: routed to elohim specialists.
6. Emergency contact B's UI (served via whatever doorway B is using today) surfaces the pending invitation. B taps confirm. B's UI invokes `submit_intimate_witness { recovery_request_hash, note: Some("Sarah called me, recognized the dog's name") }`.
7. B's coordinator cell runs pre-commit gates: (a) fetches request, extracts `human_id`; (b) confirms B is in A's emergency contacts; (c) confirms B hasn't already witnessed; (d) commits `HumanityWitness`; (e) creates `RecoveryRequestToHumanityWitness` link; (f) emits `IntimateWitnessSubmitted` (rich).
8. Colocated elohim-storage projects the witness row. Claimant's UI queries `recovery_witnesses`, renders "1 of 3: Sarah said yes."
9. C and D repeat. Storage view: "3 of 3."
10. Claimant invokes `commit_key_rotation { authority: IntimateQuorum { witness_hashes: [B, C, D] } }`.
11. Coordinator: CryptographicQuorum exemption check fails (this isn't crypto), so freeze-floor gate runs. No active freezes → passes. Existing M2 commit logic commits the rotation entry. Emits `KeyRotationCommitted` (M2 payload, unchanged).
12. Storage projects rotation (existing M2 projection). Frontend: "recovery complete."

**Failure modes rehearsed by tests:**
- Active `IdentityFreeze { frozen_at_layer: Some("intimate") }` → step 11 rejects. `CryptographicQuorum` path would succeed.
- Non-contact tries to submit witness at step 6 → pre-commit gate rejects.
- Same contact tries to submit twice at step 6 → pre-commit dedupe rejects.
- Witness count below threshold at step 10 → M2 validator rejects (already working).

## 8. Stage-1 security acceptance

M3 ships **standard account-security-level social recovery** absent institutional backing. This is honest framing, not a defect.

### 8.1 Explicitly delivered structurally

- **Anchor durability across rotation.** human_id-keyed anchors survive key rotations by design.
- **Emergency-contact membership gate** on `submit_intimate_witness`.
- **Dedupe** on witness submission per authorizer per request.
- **Freeze-floor gate** on `commit_key_rotation` (IntimateQuorum and CommunityConsensus/GovernanceAct variants; CryptographicQuorum exempt).
- **Human's other devices can observe the request** via the human_id anchor (M5 is when the defender specialist acts on it).

### 8.2 Explicitly not delivered (deferred)

- Coordinator-collusion defense — a malicious doorway-steward authoring witness entries (their own agent can't pass the emergency-contact check; but a coordinated cross-doorway attack is not structurally prevented).
- Under-duress detection — an authorizer coerced into tapping confirm.
- Social-engineering defense beyond membership — impersonation of legitimate contacts who are tricked into attesting.
- Timing-window quiet period before rotation executes.
- Hashcash / rate limiting on invitations.

### 8.3 Mitigation trajectory

Per memory `project_bootstrap_to_elohim_security_gradient`:

- Stage 1 (M3, shipped): coordinator enforces structural rules. Witnesses are humans. Attacks above this floor are known and accepted.
- Stage 2 (M5): elohim-defender specialists evaluate invitations against behavioral baselines, authorize or counter-attest at machine speed. Per memory `project_elohim_as_counsel`.
- Stage 3 (M6+): validators reference elohim attestations as first-class integrity evidence.

M3 does not promise better-than-Stage-1. The protocol promises Stage 2+ eventually; M3 lays the plumbing the wisdom slots into.

## 9. Testing

### 9.1 Sweettest — `elohim/holochain/tests/sweettest/`

1. **Happy-path intimate quorum.** Human A + 3 contacts (B, C, D) with `emergency_access_enabled = true`. Request: `human_id` populated, `required_witness_count == 3`. Rotation with [B, C] rejects; [B, C, D] succeeds.
2. **Freeze-floor blocks intimate, allows cryptographic.** Active `IdentityFreeze { frozen_at_layer: Some("intimate") }` causes `IntimateQuorum` rotation to fail. `CryptographicQuorum` rotation with valid stewardship passes.
3. **Anchor durability across rotation.** After successful rotation, a fresh cell (new agent key) queries `Anchor("recovery_request", human_id)` and finds the original request. Regression guard for the human_id anchor decision.
4. **Non-contact witness rejected.** Agent not in A's emergency-contact set invokes `submit_intimate_witness`. Coordinator pre-commit gate rejects. Regression guard for the membership check.

### 9.2 a2o — `genesis/a2o/features/auth/recovery/`

- `intimate-quorum-happy-path.feature` — tagged `@stage1-structural`. Runs in shem cross-node topology with the claimant and at least one emergency contact on **different doorway-stewards** (validates cross-doorway invitation fan-out).
- `freeze-floor-blocks-intimate-rotation.feature` — tagged `@stage1-structural`.

**Execution path:** through doorway's generic conductor app_ws proxy (existing). No new REST routes in M3; the polished `RecoveryCoordinatorService` REST surface lands in M5.

### 9.3 Out of M3 test corpus (deferred)

Red-team scenarios, under-duress scenarios, coordinator-collusion scenarios, UI-driven Cypress, per-request gossipsub topic sharding tests — all M5+.

## 10. Execution

### 10.1 Branch

`feature/recovery-m3-coordinator` from current `dev` HEAD. Picks up M2 merge + EPR-2C-adjacent CI fixes. Does not touch the stale `wave1-manifest-hygiene-and-sweettest` branch.

### 10.2 Subagent dispatch — batched parallel

**Wave 1 (parallel, isolated worktrees, zero file overlap):**

- **rust-architect #1 — DNA coordinator.** Scope: `elohim/holochain/dna/imagodei/` (integrity + coordinator). Deliverables: new link types, `create_recovery_request` bridging population, `commit_key_rotation` freeze-floor gate, `submit_intimate_witness` new function with membership + dedupe gates, signal enum amendments.
- **rust-architect #2 — elohim-storage.** Scope: `elohim/elohim-storage/` + `elohim/sdk/schemas/v1/views/recovery-*.schema.json`. Deliverables: gossipsub behaviour + wire contract + signal bridge + subscribe stub on the existing EPR-2C swarm; schemas + Rust views + migrations + projection handlers for the two tables; TS codegen regen; schema-contract tests.

Merge Wave 1 back to the feature branch when both land and compile together.

**Wave 2 (single dispatch):**

- **rust-architect #3 — tests.** Scope: `elohim/holochain/tests/sweettest/` + `genesis/a2o/features/auth/recovery/`. Deliverables: 4 sweettest scenarios, 2 a2o features (shem cross-doorway topology, app_ws execution path).

### 10.3 Explicit scope guardrails (all dispatches)

Per memory `feedback_subagent_scope_guardrails`, dispatch prompts must include:

- "Do not `git revert` or `git reset` any pre-existing commit. If you encounter a scope conflict, BLOCK and report — do not silently clean up."
- "Files outside your listed scope are forbidden to modify."
- Post-dispatch: orchestrator scans the SHA range for any out-of-scope edits before approving merge.

### 10.4 Commit discipline

Commit after each logical unit. No bypassing pre-push hooks (`HUSKY=0` is forbidden). No `cargo test --all-features --release` in development — standard `cargo test --lib --bins` per repo convention.

## 11. Out of scope (carried from kickoff, extended here)

- **Account/login layer design.** Pre-graduation (hosted-doorway) vs. post-graduation (peer-native OAuth-pattern) flows — see memory `project_peer_native_account_canonical_surface`. Revisit when stewardship graduation is a live feature.
- **Peer-native recovery initiation UX.** Tauri-steward / Moss / Holochain Launcher integration patterns for a fresh-device claimant driving the flow directly. DNA + mesh plumbing already supports this; UX work is M5+.
- **Hosted-cell bootstrap.** Provisioning a temporary hosted conductor on demand for a browser claimant mid-ceremony. M5.
- **Browser session handoff.** Moving an active recovery session between devices. M5.
- **`CommunityConsensus`, `GovernanceAct`, `NetworkWitness` variant implementations.** Phase 2b+.
- **Elohim defender authoring freezes in volume.** M5 specialist.
- **Holder-side elohim evaluation of recovery invitations.** M5 specialist.
- **Hashcash / proof-of-work gating** on gossipsub publishes. M5.
- **Rate limiting / abuse detection.** M5.
- **Fast-path revocation.** M4.
- **Frontend `RecoveryCoordinatorService` real REST endpoints.** M5.
- **Anti-lockout audit scenario suite.** M6.

## 12. Decision log (traceability)

Each decision below links a design choice to its brainstorm rationale and guiding memory.

| # | Decision | Rationale | Memory |
|---|----------|-----------|--------|
| 1 | Branch from current `dev` HEAD | Includes M2 + CI fixes; EPR-2C is on parallel branch | — |
| 2 | `HumanToRecoveryRequest` anchor = human_id-keyed only | Validator operates on human_id; sibling links agree; durable across rotations | `project_graduated_recovery_authority` |
| 3 | `submit_intimate_witness` input = `{ request_hash, note: Option<String> }` with dedupe + membership check | Minimal-viable signal at Stage 1; leaves room for Stage-2 elohim evaluation | `project_bootstrap_to_elohim_security_gradient`, `project_elohim_as_counsel` |
| 4 | Storage: `recovery_requests` + `recovery_witnesses` tables | UI renders per-witness, not just count; defender-ready | `feedback_schema_first_ioc` |
| 5 | Signals: rich (denormalized) payloads | No secondary DHT reads in projection path; operational state on libp2p/signals | `project_dht_vs_libp2p_scoping` |
| 6 | libp2p substrate in elohim-storage, NOT doorway | Doorway is web2 projection; P2P participation is the colocated storage pod | `project_three_layer_truth_model`, `project_doorway_manifest_driven_routes` |
| 7 | Test corpus: 4 sweettest + 2 a2o (app_ws-driven, shem cross-doorway topology) | Sweettest proves DNA correctness; a2o proves cross-doorway mesh flow | `project_shem_is_p2p_live_canvas`, `project_multi_doorway_human_registration` |
| 8 | Account layer explicitly deferred | Plumbing is initiator-agnostic; do not foreclose peer-native or hosted paths | `project_peer_native_account_canonical_surface` |

---

**Next step:** implementation plan at `genesis/docs/superpowers/plans/2026-04-24-recovery-protocol-phase-2-m3-coordinator-and-storage.md` via `writing-plans` skill.
