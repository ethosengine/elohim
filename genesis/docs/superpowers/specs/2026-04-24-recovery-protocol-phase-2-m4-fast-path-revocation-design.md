# Recovery Protocol Phase 2 M4 — Fast-Path Revocation

**Status:** Design approved, ready for implementation planning
**Date:** 2026-04-24
**Resolves:** the seven enumerated gaps in `genesis/docs/plans/2026-04-24-recovery-m4-fast-path-revocation-kickoff-prompt.md`
**Predecessor:** `genesis/docs/superpowers/specs/2026-04-24-recovery-protocol-phase-2-m3-coordinator-and-storage-design.md`

## 1. Purpose

M4 is the "kill a compromised key quickly" milestone. It delivers the minimum DHT action that invalidates a pubkey's future authority, via two end-to-end paths plus a coordinator-level stub for the M5 specialist path:

1. **Self-revocation** — a human with a still-valid agent key voluntarily revokes a different (compromised) key they control. Single-cell authority; no quorum; no witnesses.
2. **Emergency-contact quorum revocation** — when the human's key is captured and they cannot sign themselves, enough emergency contacts can commit a `KeyRevocation` + `RevocationVote` entries that, when threshold is reached, flip the revocation to effective.
3. **Specialist attestation stub** — `trigger_type = "challenge"` is structurally accepted by the integrity validator but coordinator-rejected with `NotYetImplemented`. Mirrors M3's stub pattern for Community/Governance/Network layers.

Plus:

4. **Rotation-vs-revocation ordering gate** — a pending or effective `KeyRevocation` on the rotating agent's current key blocks `commit_key_rotation`.
5. **Storage projection** — `key_revocations` + `revocation_votes` tables, schema-first, rebuildable via signal replay.
6. **Signal variants** — three per-event additions to `RecoveryV2Signal`.
7. **Mesh substrate** — dedicated `recovery.revocation` gossipsub topic with a MessagePack wire contract.

M4 explicitly does **not** design:
- Specialist elohim attack detection (M5 — elohim defender pattern).
- Account/login layer graduation (M5 + its own brainstorm).
- Hosted-cell bootstrap / browser session handoff (M5).
- Hashcash / rate limiting (M5).
- CommunityConsensus / GovernanceAct / NetworkWitness variant impls (Phase 2b+).
- Fast-path revocation UX in elohim-app (M5).
- Anti-lockout audit suite (M6).

## 2. Design principles

### 2.1 Principle P1 — storage as reconciliation controller over the DHT manifest

M4 is the first concrete test of a load-bearing architecture principle emerging in parallel with the EPR Phase 2B brainstorm:

> **The Holochain DHT is the authoritative manifest. `elohim-storage` is a reconciliation controller over that manifest. Observed state changes → controller reconciles eagerly, with no lazy-mark-stale and no check-on-read. This is the k8s controller pattern: manifest = desired state, controller = reconciliation loop.**

Implications M4 must honour:
- **Eager invalidation**, not lazy. When `KeyRevocationEffective` is observed, storage immediately sweeps dependent cached state tied to the revoked key (peer/session caches today; `epr_atoms.signer_cid` rows in Phase 2B+).
- **Signal payloads rich enough to reconcile without DHT re-fetch.** Matches M3's richness convention.
- **Outbound reconciliation signals.** After storage completes its projection + sweep, it emits `imagodei.revocation_observed` for downstream controllers (M5 elohim defender, Phase 2B projector, Phase 4 GraphQL subgraph).
- **Projection tables are disposable.** `key_revocations` and `revocation_votes` are read-optimized caches rebuildable from DHT via signal replay. Migration comments declare `-- Source of truth: DHT` per the M3 convention.

### 2.2 Principle P2 — coordinator gate vs. controller reconciliation

Two orthogonal axes:
- **Coordinator gates** enforce the manifest synchronously at write time. Example: revocation-floor gate in `commit_key_rotation`.
- **Controller reconciliation** projects the manifest asynchronously at read time via signal replay. Example: `key_revocations` table upsert on `KeyRevocationRequested`.

Both are valid and both ship in M4. Gates prevent inconsistent writes. The controller reconciles observed state.

### 2.3 Principle P3 — dual-subject entities earn dual-anchor lookups

Revocation has two primary subjects: the key being killed, and the human whose authority is affected. The existing link infrastructure supports both. M4 declares `RevokedKeyToRevocation` and `HumanToKeyRevocation` as **co-first-class primaries** with documented query roles, rather than collapsing to a single primary for false clarity.

### 2.4 Substrate-coupling note (three-arc convergence)

M4 belongs to the **resiliency-producer arc** that converges into EPR Phase 2B alongside the graph-surface-consumer arc. Design decisions below are weighted for clean re-ingestion by Phase 2B's projector without forcing M4 to couple against undecided Phase 2B shapes:

- M4 entries, signals, and projections are designed for re-projection as EPR atoms with coupling refs when Phase 2B's projector lands.
- Coupling refs (`revocation ↔ rotation ↔ recovery_request ↔ witnesses ↔ humanity_witness`) are carried explicitly in signal payloads for Phase 2B's graph-coupling ingestion.
- Eager cache-invalidation sweep on `KeyRevocationEffective` is the hook point Phase 2B will extend to sweep `epr_atoms WHERE signer_cid = revoked_key`.
- `imagodei.revocation_observed` outbound signal is the seam Phase 4 GraphQL subscriptions will consume.

## 3. Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ imagodei DNA (Holochain) — the notary                       │
│   integrity: KeyRevocation, RevocationVote (existing types) │
│              link types: {Id,Human,RevokedKey,              │
│              Pending,Effective}-to-KeyRevocation,           │
│              {Id,Revocation,Steward}-to-RevocationVote      │
│              (all existing)                                 │
│   integrity: validate_key_revocation — trigger-type-aware   │
│              (voluntary: required_votes==1;                 │
│               steward_vote/challenge: >=2)                  │
│   coordinator: create_self_revocation,                      │
│                create_revocation_request (emergency path),  │
│                submit_revocation_vote,                      │
│                commit_key_rotation (+ revocation-floor gate)│
│                → emits RecoveryV2Signal (rich payloads)     │
└──────────────┬───────────────────────────┬──────────────────┘
               │ cell signals               │ cell signals
               ▼                            ▼
     ┌────────────────────────────┐  ┌──────────────────────┐
     │ elohim-storage (libp2p)    │  │ elohim-storage       │
     │  existing EPR-2C swarm:    │  │  (reconciliation     │
     │  + recovery.revocation     │  │   controller)        │
     │    gossipsub topic         │  │  key_revocations,    │
     │  + MessagePack wire        │  │  revocation_votes    │
     │  + signal→publish bridge   │  │  eager sweep on      │
     │  + subscribe/log stub      │  │  KeyRevocationEffective│
     │                            │  │  + imagodei.revocation_observed │
     │                            │  │    outbound signal   │
     └────────────────────────────┘  └──────────────────────┘
```

**Trust boundary:** DNA coordinator is authoritative. Storage is projection. Mesh is notification fan-out. DHT wins all disagreements.

## 4. DNA changes

### 4.1 Entry types and link types — no new types

M4 reuses existing integrity declarations:
- Entries: `KeyRevocation`, `RevocationVote` (already in `EntryTypes`).
- Link types: `IdToKeyRevocation`, `HumanToKeyRevocation`, `RevokedKeyToRevocation`, `PendingRevocations`, `EffectiveRevocations`, `IdToRevocationVote`, `RevocationToVote`, `StewardToRevocationVote`.

This preserves DNA headroom and keeps imagodei at 28/~100 entry types.

### 4.2 Validator softening

`validate_key_revocation` currently hardcodes `required_votes < 2 → invalid`. M4 replaces this with a trigger-type-aware rule:

```rust
match revocation.trigger_type.as_str() {
    "voluntary" => {
        if revocation.required_votes != 1 {
            return invalid("voluntary revocation must have required_votes == 1");
        }
    }
    "steward_vote" | "challenge" => {
        if revocation.required_votes < 2 {
            return invalid("quorum revocation must have required_votes >= 2");
        }
    }
    other => return invalid(format!("unknown trigger_type: {other}")),
}
```

The validator remains deterministic (single-entry inspection only; no `get_links`). All cross-entity enforcement lives in coordinator gates.

**Legacy `votes_json` field**: remains on `KeyRevocation` struct for source-chain compatibility with legacy entries. M4 coordinators write empty string and never read it; votes are canonical via separate `RevocationVote` entries linked through `RevocationToVote`.

### 4.3 Coordinator functions

#### 4.3.1 `create_self_revocation`

```
fn create_self_revocation(
    revoked_key: AgentPubKey,
    reason: String,       // must be one of REVOCATION_REASONS
) -> ExternResult<KeyRevocationRecord>
```

- Pre-commit gates:
  - Caller's human_id is resolvable via `resolve_human_id_for_agent` (M3 helper).
  - Caller controls `revoked_key` — verified via existing Agent→Human link or equivalent (reuse M3's agent-key resolution path).
  - `reason` ∈ REVOCATION_REASONS.
- Build `KeyRevocation` with:
  - `trigger_type = "voluntary"`, `initiated_by = caller_human_id`
  - `required_votes = 1`, `current_votes = 1`
  - `threshold_reached = true`, `effective_at = Some(now)`
  - `votes_json = String::new()` (legacy field, unused)
- Commit entry.
- Create links: `IdToKeyRevocation`, `HumanToKeyRevocation`, `RevokedKeyToRevocation`, `EffectiveRevocations` (skip `PendingRevocations` — voluntary is effective on creation).
- `emit_signal`: `KeyRevocationRequested { …full entry… }` **AND** `KeyRevocationEffective { revocation_id, revoked_key, human_id, effective_at, triggering_vote_id: None }`. Both emitted from the same coordinator call.

#### 4.3.2 `create_revocation_request` (emergency-contact quorum path)

```
fn create_revocation_request(
    target_human_id: String,
    revoked_key: AgentPubKey,
    reason: String,
) -> ExternResult<KeyRevocationRecord>
```

- Pre-commit gates:
  - Caller is an active emergency contact for `target_human_id` via `is_active_emergency_contact` (M3 helper).
  - `revoked_key` belongs to `target_human_id` (agent→human resolution).
  - `reason` ∈ REVOCATION_REASONS.
- Compute quorum threshold:
  ```
  let m = count_active_emergency_contacts(target_human_id)?;
  let required = compute_required_witness_count(m); // max(2, ceil(M/2)+1)
  // TODO(M4-post): revisit whether revocation quorum should diverge from
  // recovery quorum. For now, parity keeps the two paths coherent.
  ```
- Build `KeyRevocation` with:
  - `trigger_type = "steward_vote"`, `initiated_by = caller_human_id`
  - `required_votes = required`, `current_votes = 0`
  - `threshold_reached = false`, `effective_at = None`
- Commit entry.
- Create links: `IdToKeyRevocation`, `HumanToKeyRevocation`, `RevokedKeyToRevocation`, `PendingRevocations`.
- `emit_signal`: `KeyRevocationRequested { …full entry… }`.

#### 4.3.3 `submit_revocation_vote`

```
fn submit_revocation_vote(
    revocation_id: String,
    approved: bool,
    attestation: String,
) -> ExternResult<RevocationVoteRecord>
```

- Pre-commit gates:
  - Revocation exists and `trigger_type == "steward_vote"`. (Voluntary revocations don't accept votes.)
  - Caller is an active emergency contact for the revocation's `human_id`.
  - No existing `RevocationVote` from this steward on this revocation: query `StewardToRevocationVote` anchor for caller_steward_id, filter to this revocation_id.
  - `attestation` is non-empty.
- Build `RevocationVote` entry; commit.
- Create links: `IdToRevocationVote`, `RevocationToVote`, `StewardToRevocationVote`.
- Recompute threshold:
  - `get_links(RevocationToVote anchor for revocation_id)` → vector of links to `RevocationVote` entries.
  - For each link, `must_get_entry(link.target.into())` → `RevocationVote`; count those with `approved == true`. Count is authoritative, not `current_votes` on the in-memory `KeyRevocation`.
  - If count < `required_votes`: emit `RevocationVoteSubmitted { …vote fields…, current_votes = count, required_votes, threshold_now_reached: false }`.
  - If count >= `required_votes` and revocation is still pending:
    - `update_entry` on `KeyRevocation`: bump `current_votes`, flip `threshold_reached = true`, set `effective_at = now`, bump `updated_at`.
    - Remove `PendingRevocations` link; add `EffectiveRevocations` link.
    - Emit **both** `RevocationVoteSubmitted { …, threshold_now_reached: true }` **AND** `KeyRevocationEffective { revocation_id, revoked_key, human_id, effective_at: now, triggering_vote_id: Some(vote_id) }`.
- **Rejection votes** (`approved = false`): counted in the `RevocationVote` count for audit purposes, but only `approved == true` counts toward the threshold. Rejections never advance the pending→effective transition.

#### 4.3.4 Specialist path (M5 seam)

A coordinator function `create_specialist_revocation` (name TBD in implementation) is **not** added in M4. If a caller constructs a `KeyRevocation` with `trigger_type = "challenge"` via a future path, the integrity validator accepts it structurally (the enum value is valid), but no M4 coordinator emits such entries. This is the deliberate M5 seam — M5 will add the elohim-defender-initiated coordinator.

### 4.4 Revocation-floor gate on `commit_key_rotation`

Extend M3's existing `commit_key_rotation` coordinator function. After the freeze-floor gate, before commit:

```rust
let pending = get_links(
    Anchor::new(&human_id, LinkTypes::PendingRevocations),
    LinkTypes::PendingRevocations, None)?;
let effective = get_links(
    Anchor::new(&human_id, LinkTypes::EffectiveRevocations),
    LinkTypes::EffectiveRevocations, None)?;

for link in pending.iter().chain(effective.iter()) {
    let revocation: KeyRevocation = must_get_entry(link.target.into())?.try_into()?;
    if revocation.revoked_key == rotation.rotating_from {
        return Err(wasm_error!("commit_key_rotation blocked: key {} has a {} revocation ({}). Resolve or await the revocation before rotating.",
            rotation.rotating_from,
            if revocation.threshold_reached { "effective" } else { "pending" },
            revocation.id));
    }
}
```

**No authority-layer exemption.** Asymmetric with M3's freeze-floor (which exempts `CryptographicQuorum`), because revocation is structural: a revoked key must not produce valid rotations under any authority claim. Documented as conscious asymmetry.

## 5. Signal layer

Extend `RecoveryV2Signal` enum (serde tag `"type"`):

```rust
pub enum RecoveryV2Signal {
    // existing variants: RecoveryRequestCreated, IntimateWitnessSubmitted, KeyRotationCommitted
    KeyRevocationRequested {
        id: String,
        human_id: String,
        revoked_key: String,
        reason: String,
        trigger_type: String,
        initiated_by: String,
        required_votes: u32,
        current_votes: u32,
        threshold_reached: bool,
        effective_at: Option<String>,
        created_at: String,
    },
    RevocationVoteSubmitted {
        id: String,
        revocation_id: String,
        steward_id: String,
        approved: bool,
        attestation: String,
        voted_at: String,
        current_votes: u32,
        required_votes: u32,
        threshold_now_reached: bool,
    },
    KeyRevocationEffective {
        revocation_id: String,
        revoked_key: String,
        human_id: String,
        effective_at: String,
        triggering_vote_id: Option<String>,
    },
}
```

Payloads are deliberately rich — storage's controller reconciles without re-fetching.

## 6. Storage layer (reconciliation controller)

### 6.1 Schema-first

Two new view schemas under `elohim/sdk/schemas/v1/views/`:
- `key-revocation-view.schema.json`
- `revocation-vote-view.schema.json`

Both follow the M3 conventions: `camelCase` fields, `dhtAnchorHash`, all timestamps as ISO-8601 strings.

Matching Rust views in `elohim-storage/src/views.rs` with `#[serde(rename_all = "camelCase")]` and `#[derive(TS)]`.

Schema-contract tests extend `elohim-storage/tests/schema_contract.rs`.

Added to `INTERFACE_FILES` in `elohim/sdk/schemas/scripts/codegen-ts.mjs`.

### 6.2 Migrations

Two migrations in `elohim-storage/migrations/`:

```sql
-- key_revocations migration header
-- Source of truth: DHT (imagodei KeyRevocation entries)
-- Projection: read-optimized; rebuildable via signal replay on RecoveryV2Signal::KeyRevocationRequested/Effective
CREATE TABLE key_revocations (
    dht_anchor_hash TEXT PRIMARY KEY NOT NULL,
    id TEXT NOT NULL UNIQUE,
    human_id TEXT NOT NULL,
    revoked_key TEXT NOT NULL,
    reason TEXT NOT NULL,
    trigger_type TEXT NOT NULL,
    initiated_by TEXT NOT NULL,
    required_votes INTEGER NOT NULL,
    current_votes INTEGER NOT NULL,
    threshold_reached INTEGER NOT NULL,  -- 0/1
    effective_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_key_revocations_human ON key_revocations(human_id);
CREATE INDEX idx_key_revocations_revoked_key ON key_revocations(revoked_key);
CREATE INDEX idx_key_revocations_pending ON key_revocations(threshold_reached) WHERE threshold_reached = 0;
```

```sql
-- revocation_votes migration header
-- Source of truth: DHT (imagodei RevocationVote entries)
-- Projection: rebuildable via signal replay on RecoveryV2Signal::RevocationVoteSubmitted
CREATE TABLE revocation_votes (
    dht_anchor_hash TEXT PRIMARY KEY NOT NULL,
    id TEXT NOT NULL UNIQUE,
    revocation_dht_anchor_hash TEXT NOT NULL,
    revocation_id TEXT NOT NULL,
    steward_id TEXT NOT NULL,
    approved INTEGER NOT NULL,  -- 0/1
    attestation TEXT NOT NULL,
    voted_at TEXT NOT NULL,
    UNIQUE(revocation_id, steward_id)  -- enforce no-double-vote at the projection layer too
);
CREATE INDEX idx_revocation_votes_revocation ON revocation_votes(revocation_id);
CREATE INDEX idx_revocation_votes_steward ON revocation_votes(steward_id);
```

### 6.3 Projection handler

Extend `handle_recovery_v2_signal` dispatcher in `elohim-storage/src/recovery_v2/projection.rs` (or equivalent):

- `KeyRevocationRequested` → upsert `key_revocations` row; emit `imagodei.revocation_observed { status: "pending" | "effective-on-create", … }`.
- `RevocationVoteSubmitted` → insert `revocation_votes` row; update denormalized `current_votes` on `key_revocations` row.
- `KeyRevocationEffective` → update `key_revocations` row (flip `threshold_reached`, set `effective_at`) **AND** perform eager cache-invalidation sweep:
  - Identify dependent cached state tied to `revoked_key`. In M4's current surface, this is:
    - `peer_identity_bindings` rows (if any exist; discovered in implementation) whose `pubkey == revoked_key`.
    - Any session/verify cache entries keyed by `revoked_key`.
  - Sweep is bounded (indexed by `revoked_key`), not a table scan.
  - **Phase 2B seam:** when `epr_atoms` table lands, this sweep extends to `UPDATE epr_atoms SET verified_at = NULL WHERE signer_cid = revoked_key AND issued_at >= …`. M4 leaves a commented hook for that extension.
  - Emit `imagodei.revocation_observed { status: "effective", revoked_key, human_id, effective_at, … }` after the sweep completes.

### 6.4 Publish-intent extractor

In `elohim-storage/src/recovery_v2/mesh.rs` (or equivalent — verified in implementation), add:

```rust
pub fn recovery_revocation_from_signal(signal: &RecoveryV2Signal) -> Option<RecoveryRevocationMessage> {
    match signal {
        RecoveryV2Signal::KeyRevocationRequested { … } => Some(RecoveryRevocationMessage {
            revocation_id, human_id, revoked_key, trigger_type, reason,
            status: if threshold_reached { "effective" } else { "pending" }.into(),
            sender_peer_id: local_peer_id(),
            sent_at: iso_now(),
        }),
        RecoveryV2Signal::KeyRevocationEffective { … } => Some(RecoveryRevocationMessage {
            …,
            status: "effective".into(),
            …,
        }),
        _ => None,
    }
}
```

## 7. Mesh layer (libp2p substrate)

### 7.1 Dedicated topic

New gossipsub topic: `recovery.revocation`.

Rationale:
- Subscriber sets differ from `recovery.invitation`. Revocation subscribers are emergency contacts + specialist-elohim watchers + security dashboards; invitation subscribers are the human's intimate recovery circle. Overlap is partial.
- Topic-level filter is cheaper than payload-discriminator filter at scale.
- Keeps `recovery.invitation` scoped to its M3 semantics; avoids overloading a topic with divergent subscriber expectations.

### 7.2 Wire contract

MessagePack (NOT CBOR — per `feedback_swarm_composition_fresh_tree_build`):

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryRevocationMessage {
    pub revocation_id: String,
    pub human_id: String,
    pub revoked_key: String,
    pub trigger_type: String,
    pub reason: String,
    pub status: String,       // "pending" | "effective"
    pub sender_peer_id: String,
    pub sent_at: String,
}
```

Subscribe/log stub only for M4; active consumer logic lands in M5 elohim defender + M5 UI.

## 8. Decision log (M3 §12-style)

| # | Decision | Rationale | Alternatives considered |
|---|----------|-----------|--------------------------|
| 1 | Emergency-contact revocation threshold = `max(2, ceil(M/2)+1)` (reuse M3 helper). TODO annotation to revisit. | Parity with M3 prevents attacker gaming (picking the weaker path). Defender symmetry. Code reuse. | Lower threshold (faster kill) rejected — creates asymmetric attack surface. Higher (stricter) rejected — revocation is not more destructive than rotation-recovery. |
| 2 | Fast-path definition: "fast" = latency not authority; skips KeyStewardship provisioning; no auto-freeze; no auto-key-promotion. | Scope clarity. Avoids creeping into M5 territory. | Auto-freeze on revocation rejected — separate concerns. Auto-promotion rejected — policy, not protocol. |
| 3 | Rotation-floor gate blocks on pending OR effective revocation of the `rotating_from` key; no layer exemption; coordinator-level (HDI-compatible). | Prevents compromised-key escape via rotation. Asymmetric with freeze-floor (revocation is structural, freezes are soft); asymmetry is intentional and documented. | "Effective only" rejected — pending window is the attack window. "from + to" deferred to a follow-up if needed; "to" is operationally nonsensical today. |
| 4 | Dual-anchor primacy: both `RevokedKeyToRevocation` (hot gate query) and `HumanToKeyRevocation` (user listing) are first-class primaries. Four links per revocation. | Revocation has two primary subjects (key, human). Single-primary gives false clarity. Links are cheap; revocations are rare. Graduated-concern flexibility for future authority layers. | human_id-only (B) rejected — would force O(n-keys) filtering on every rotation. revoked_key-only (C) rejected — user-facing listings suffer. |
| 5 | Three per-event signal variants: `KeyRevocationRequested`, `RevocationVoteSubmitted`, `KeyRevocationEffective`. Self-revocation emits Requested + Effective atomically. | Parity with M3 convention. Rich payloads → P1-compatible controller reconciliation. New authority layers extend via payload fields, not variant explosion. | Single rich variant rejected — loses subscription granularity. Strict-in-order variant rejected — no semantic win over A. |
| 6 | Dedicated `recovery.revocation` gossipsub topic (not reuse of `recovery.invitation`). MessagePack wire. | Subscriber sets differ. Topic-level filter is cheaper than payload-level filter. Keeps invitation topic scoped to M3 semantics. | Reuse invitation (A) rejected — conflates subscriber vocabularies. Broad `imagodei.events` (C) premature — Phase 2B will land that consolidation cleanly. |
| 7 | Two subagents in sequence: (1) DNA+Storage combined, (2) Tests. | DNA + storage are schema-coupled (schema-first: schema → view → signal → coordinator emits → storage projects). Splitting risks drift. Tests validate end-to-end after plumbing stabilizes. | Three-way split rejected — over-orchestration. Single combined rejected — test pass can't start until DNA+Storage merge. |

## 9. Tests

### 9.1 sweettest scenarios (`elohim_sweettest` crate)

- `self_revocation_happy_path` — human revokes a device key they control; `KeyRevocation` lands effective; signals emitted in order.
- `emergency_contact_quorum_met` — 3 emergency contacts vote approved, threshold reached, status flips to effective, `KeyRevocationEffective` signal emitted.
- `emergency_contact_quorum_not_met` — 2 approvals + 1 rejection on a 3-required threshold; status remains pending; no `KeyRevocationEffective`.
- `rotation_blocked_by_pending_revocation` — pending KeyRevocation on current key → `commit_key_rotation` rejects with descriptive error.
- `rotation_blocked_by_effective_revocation` — effective KeyRevocation on current key → `commit_key_rotation` rejects.
- `rotation_unaffected_by_revocation_of_other_key` — revocation on a different key the human controls does not block rotation.
- `duplicate_vote_rejected` — second `submit_revocation_vote` from the same steward on the same revocation rejects.
- `non_emergency_contact_cannot_initiate` — caller without emergency-contact relationship to target cannot call `create_revocation_request`.
- `specialist_path_coordinator_rejected` — any attempt at M5-seam path rejects with NotYetImplemented (if a coordinator path is wired; else enforced structurally).

### 9.2 a2o feature files (tagged `@recovery-m4`)

- `genesis/a2o/features/auth/revocation-self.feature` — "A human revokes a compromised device key." Acceptance:
  - Given Matthew has two devices with keys K1 and K2
  - When Matthew's phone (K2) is stolen and he revokes K2 from his laptop (K1)
  - Then K2 is recorded as effectively revoked on the DHT
  - And any future actions signed by K2 are rejected
  - And K1 remains valid

- `genesis/a2o/features/auth/revocation-emergency-quorum.feature` — "Emergency contacts kill a captured key." Acceptance:
  - Given Matthew's only key K1 is captured by an attacker
  - And Matthew has 4 active emergency contacts
  - When 3 emergency contacts submit approved revocation votes on K1
  - Then K1 is recorded as effectively revoked
  - And the attacker's future actions signed by K1 are rejected
  - And Matthew can initiate full recovery (M3 path) afterwards

### 9.3 Schema contract tests

- `key_revocation_view_matches_schema` — Rust view serialization matches JSON schema.
- `revocation_vote_view_matches_schema` — same.
- `signal_variants_serde_tag_preserved` — all three new variants use `serde(tag = "type")` and round-trip through JSON identical to the DNA emission.

## 10. Subagent dispatch plan

Two dispatches in sequence.

### 10.1 Subagent 1 — DNA + Storage (combined)

**Scope**:
- imagodei integrity: validator softening.
- imagodei coordinator: three new functions + revocation-floor gate on `commit_key_rotation` + helpers.
- `RecoveryV2Signal` three new variants (DNA side + elohim-storage mirror).
- Storage: two new schemas, two new migrations, two new views, schema-contract tests, projection handler extensions, publish-intent extractor, eager-sweep stub + `imagodei.revocation_observed` outbound signal, `recovery.revocation` topic registration.

**Explicit guardrails in dispatch prompt**:
- Forbid `git revert` / `git reset` on pre-existing commits. Any scope conflict → BLOCKED report, not a silent cleanup.
- Forbid modifying files outside: `elohim/holochain/dna/imagodei/**`, `elohim/elohim-storage/src/**`, `elohim/elohim-storage/migrations/**`, `elohim/elohim-storage/tests/schema_contract.rs`, `elohim/sdk/schemas/v1/views/key-revocation-view.schema.json`, `elohim/sdk/schemas/v1/views/revocation-vote-view.schema.json`, `elohim/sdk/schemas/scripts/codegen-ts.mjs` (INTERFACE_FILES addition only).
- Mandatory: fresh-tree `cargo build --release` on `elohim-storage` with `RUSTFLAGS='--cfg getrandom_backend="custom"'` before any commit touching swarm/topic composition.
- Mandatory: `cd elohim/holochain/dna/imagodei && just check && just pack` before commit.
- Report BLOCKED if any gate fails.

**Success criteria**:
- `just check && just pack` clean in imagodei DNA.
- `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release` clean in elohim-storage.
- Schema contract tests pass.
- Codegen `pnpm run schema:codegen:ts` regenerates TypeScript without drift.

### 10.2 Subagent 2 — Tests

**Scope**:
- sweettest scenarios (§9.1).
- a2o feature files (§9.2).
- Any integration glue needed in the sweettest harness.

**Explicit guardrails**:
- Forbid `git revert` / `git reset` on pre-existing commits.
- Forbid modifying files outside: `elohim/holochain/tests/sweettest/**`, `genesis/a2o/features/auth/revocation-*.feature`.
- Do not modify DNA or storage source files; if a test surfaces a bug, open a BLOCKED report with repro notes — the orchestrator decides whether to loop subagent 1 or accept the test as failing pending next sprint.

**Success criteria**:
- `cd elohim/holochain/tests/sweettest && CARGO_TARGET_DIR=target/native-tests cargo test` clean in the nix shell (CI-driven; local Eclipse Che cannot run this per `feedback_shift_measure_jenkins`).
- a2o features parse without `cucumber --dry-run` errors.

### 10.3 Post-dispatch checks (orchestrator)

After each subagent:
- `git log --oneline <pre-dispatch-SHA>..HEAD` — verify no out-of-scope commits.
- `git diff --stat <pre-dispatch-SHA>..HEAD` — verify file list stays in scope.
- Run husky pre-push locally before the actual push: `HUSKY=0 git push` is **forbidden** per kickoff constraint. If pre-push fails, fix forward with a new commit — never bypass.

## 11. Out-of-scope (M5+ handoff)

- **Elohim defender / specialist revocation coordinator** — M5. `trigger_type == "challenge"` path stays coordinator-stubbed.
- **UI for fast-path revocation** — M5. Angular `imagodei` pillar adds the revocation surface.
- **Account-layer supersession** — M5 + its own brainstorm. M4 stays initiator-agnostic: any cell (tauri, doorway, sweettest) drives identical.
- **Rate limiting / hashcash on revocation requests** — M5. Prevents spam-revocation attacks from compromised emergency contacts.
- **Anti-lockout audit suite** — M6.
- **Phase 2B projector integration** — when `epr_atoms` table lands, the M4 eager-sweep extends; no M4 rework required.

## 12. Memories informing this design

- `project_graduated_recovery_authority` — five-layer authority stack; M4 ships Stage 1.
- `project_elohim_as_counsel` — informs the M5 specialist seam.
- `project_socially_derived_security` — why the emergency-contact path exists.
- `project_bootstrap_to_elohim_security_gradient` — M4 = Stage 1 structural; M5 = Stage 2 elohim.
- `project_hdi_no_get_links_in_validators` — revocation-floor gate lives in coordinator.
- `feedback_schema_first_ioc` — JSON schemas precede Rust/TS.
- `feedback_subagent_scope_guardrails` — explicit forbidding language in dispatch prompts.
- `feedback_swarm_composition_fresh_tree_build` — fresh elohim-storage cargo build before commit; MessagePack not CBOR.
- `feedback_shift_measure_jenkins` — sweettest is CI-measured; Eclipse Che cannot run it locally.
- `project_peer_native_account_canonical_surface` — M4 stays initiator-agnostic.
- `project_three_layer_truth_model` — DHT notary, libp2p data-ops, doorway web2.
