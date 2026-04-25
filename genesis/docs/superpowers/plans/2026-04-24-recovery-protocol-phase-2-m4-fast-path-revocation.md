# Recovery Protocol Phase 2 M4 — Fast-Path Revocation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship fast-path key revocation — self-revocation + emergency-contact quorum revocation — with a coordinator-level rotation-floor gate, storage projection controller, and dedicated gossipsub topic, resolving all seven brainstorm gaps from the M4 kickoff.

**Architecture:** Reuse existing `KeyRevocation` and `RevocationVote` DHT entry types and their eight link types — no new DNA entry types. Soften the integrity validator to be trigger-type-aware. Add three new coordinator functions (`create_self_revocation`, `create_revocation_request`, `submit_revocation_vote`) and extend `commit_key_rotation` with a revocation-floor gate. Storage is a reconciliation controller over the DHT manifest: three new `RecoveryV2Signal` variants drive `key_revocations` and `revocation_votes` projection tables, eager invalidation of dependent caches, and outbound `imagodei.revocation_observed` signals. A dedicated `recovery.revocation` gossipsub topic carries a MessagePack wire contract for cross-peer notification.

**Tech Stack:** Rust (HDK for DNA, diesel for SQLite, libp2p 0.54 for gossipsub), JSON Schema (view contracts), ts-rs (Rust→TypeScript codegen), Cucumber/Gherkin (a2o), sweettest (Holochain end-to-end tests).

**Spec:** `genesis/docs/superpowers/specs/2026-04-24-recovery-protocol-phase-2-m4-fast-path-revocation-design.md`

**Branch:** `feature/recovery-m4-fast-path-revocation` (already cut from `dev`)

---

## File Structure

### DNA (imagodei zomes)

- `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs` — **modify** `validate_key_revocation` (~line 1804) for trigger-type-aware `required_votes` rule.
- `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` — **modify/add**:
  - New coordinator functions `create_self_revocation`, `create_revocation_request` (emergency-contact path), `submit_revocation_vote`.
  - **Extend** `commit_key_rotation` (at ~line 1788) with revocation-floor gate after the existing freeze-floor gate.
  - New helper `count_approved_revocation_votes(revocation_id) -> u32`.

### Schemas

All view schemas declare **Source of truth: DHT** (imagodei KeyRevocation + RevocationVote entries). The schemas describe read-optimized projections, not canonical records. Migrations, Rust views, and TS codegen all carry this declaration.

- `elohim/sdk/schemas/v1/views/key-revocation.schema.json` — **create**, mirrors `recovery-request.schema.json`. Source of truth: DHT.
- `elohim/sdk/schemas/v1/views/revocation-vote.schema.json` — **create**, mirrors `recovery-witness.schema.json`. Source of truth: DHT.
- `elohim/sdk/schemas/scripts/codegen-ts.mjs` — **modify** `INTERFACE_FILES` if view input types are needed (verify during implementation).

### Storage

- `elohim/elohim-storage/src/signals.rs` — **extend** `RecoveryV2Signal` enum with three new variants; **extend** `handle_recovery_v2_signal` dispatcher; **extend** `recovery_invitation_from_signal` extractor pattern with a parallel `recovery_revocation_from_signal`.
- `elohim/elohim-storage/src/views.rs` — **add** `KeyRevocationView` and `RevocationVoteView` Rust view types.
- `elohim/elohim-storage/migrations/2026-04-24-010000_key_revocations/up.sql` + `down.sql` — **create**.
- `elohim/elohim-storage/migrations/2026-04-24-020000_revocation_votes/up.sql` + `down.sql` — **create**.
- `elohim/elohim-storage/src/db/schema.rs` — **auto-regenerated** via diesel (verify after migration).
- `elohim/elohim-storage/src/db/models.rs` (or pillar-specific file — verify location) — **add** `KeyRevocationRow` + `RevocationVoteRow` diesel models and projection upsert/insert helpers.
- `elohim/elohim-storage/src/recovery_v2_mesh.rs` (or equivalent — locate during Task B.3) — **add** `RecoveryRevocationMessage` + gossipsub topic registration for `recovery.revocation`.
- `elohim/elohim-storage/tests/schema_contract.rs` — **extend** with contract tests for `KeyRevocationView` and `RevocationVoteView`.

### Tests

- `elohim/holochain/tests/sweettest/tests/recovery_m4_*.rs` — **create** sweettest scenarios (one file per major scenario or grouped).
- `genesis/a2o/features/auth/revocation-self.feature` — **create**.
- `genesis/a2o/features/auth/revocation-emergency-quorum.feature` — **create**.

---

## Phase A — Schema-first (3 tasks)

### Task A.1: Create `key-revocation.schema.json`

**Source of truth: DHT** (imagodei `KeyRevocation` entries). The view below is a read-optimized projection, not canonical.

**Files:**
- Create: `elohim/sdk/schemas/v1/views/key-revocation.schema.json` — Source of truth: DHT.

- [ ] **Step 1: Review the existing pattern**

Read `elohim/sdk/schemas/v1/views/recovery-request.schema.json` for the convention (ISO-8601 timestamps, camelCase fields, `dhtAnchorHash`).

- [ ] **Step 2: Write the schema**

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/key-revocation.schema.json",
  "title": "KeyRevocationView",
  "description": "Read-optimized projection of an imagodei KeyRevocation DHT entry. Source of truth: DHT. Rebuildable via signal replay.",
  "type": "object",
  "required": [
    "dhtAnchorHash",
    "id",
    "humanId",
    "revokedKey",
    "reason",
    "triggerType",
    "initiatedBy",
    "requiredVotes",
    "currentVotes",
    "thresholdReached",
    "createdAt",
    "updatedAt"
  ],
  "properties": {
    "dhtAnchorHash": { "type": "string", "description": "Holochain ActionHash of the KeyRevocation entry." },
    "id": { "type": "string", "description": "Coordinator-generated UUID." },
    "humanId": { "type": "string" },
    "revokedKey": { "type": "string", "description": "The AgentPubKey being revoked (base64)." },
    "reason": { "type": "string", "description": "One of REVOCATION_REASONS." },
    "triggerType": { "type": "string", "enum": ["voluntary", "steward_vote", "challenge"] },
    "initiatedBy": { "type": "string" },
    "requiredVotes": { "type": "integer", "minimum": 1 },
    "currentVotes": { "type": "integer", "minimum": 0 },
    "thresholdReached": { "type": "boolean" },
    "effectiveAt": { "type": ["string", "null"], "format": "date-time" },
    "createdAt": { "type": "string", "format": "date-time" },
    "updatedAt": { "type": "string", "format": "date-time" }
  },
  "additionalProperties": false
}
```

- [ ] **Step 3: Run schema self-tests (validates the DHT projection view)**

Source of truth: DHT (notarized `KeyRevocation` entry). The projection below tests that the view shape is consistent.

```bash
cd /projects/elohim && pnpm run schema:test
```
Expected: all assertions pass.

- [ ] **Step 4: Commit (DHT projection view only — no canonical storage)**

Source of truth remains the DHT; this commit adds only the projection schema.

```bash
git add elohim/sdk/schemas/v1/views/key-revocation.schema.json
git commit -m "schema(recovery-m4): add KeyRevocationView schema"
```

### Task A.2: Create `revocation-vote.schema.json`

**Source of truth: DHT** (imagodei `RevocationVote` entries). Projection only, not canonical.

**Files:**
- Create: `elohim/sdk/schemas/v1/views/revocation-vote.schema.json`

- [ ] **Step 1: Write the schema**

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/revocation-vote.schema.json",
  "title": "RevocationVoteView",
  "description": "Read-optimized projection of an imagodei RevocationVote DHT entry. Source of truth: DHT.",
  "type": "object",
  "required": [
    "dhtAnchorHash",
    "id",
    "revocationDhtAnchorHash",
    "revocationId",
    "stewardId",
    "approved",
    "attestation",
    "votedAt"
  ],
  "properties": {
    "dhtAnchorHash": { "type": "string" },
    "id": { "type": "string" },
    "revocationDhtAnchorHash": { "type": "string" },
    "revocationId": { "type": "string" },
    "stewardId": { "type": "string" },
    "approved": { "type": "boolean" },
    "attestation": { "type": "string" },
    "votedAt": { "type": "string", "format": "date-time" }
  },
  "additionalProperties": false
}
```

- [ ] **Step 2: Run schema self-tests (DHT projection view validation)**

Source of truth: DHT. The projection view is read-only.

```bash
cd /projects/elohim && pnpm run schema:test
```
Expected: all assertions pass.

- [ ] **Step 3: Commit (DHT projection view)**

Source of truth remains DHT; commit adds only the projection.

```bash
git add elohim/sdk/schemas/v1/views/revocation-vote.schema.json
git commit -m "schema(recovery-m4): add RevocationVoteView schema"
```

### Task A.3: Register projection schemas in TS codegen

**Source of truth: DHT.** This task wires the DHT projection view schemas into the TypeScript codegen pipeline.

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs`

- [ ] **Step 1: Inspect current pipeline (DHT projection codegen)**

```bash
cd /projects/elohim && grep -n "recovery-request\|recovery-witness\|key-rotation" elohim/sdk/schemas/scripts/codegen-ts.mjs
```

If the DHT projection view schemas are already discovered via glob (no explicit list needed), skip to Step 4. If the codegen uses an explicit list, continue.

- [ ] **Step 2: Add projection schema names**

Source of truth remains DHT. Add `"key-revocation"` and `"revocation-vote"` to the view schema list (location determined by Step 1 inspection — adjacent to the existing `recovery-request` / `recovery-witness` entries).

- [ ] **Step 3: Regenerate TypeScript (DHT projection bindings)**

```bash
cd /projects/elohim && pnpm run schema:codegen:ts
```
Expected: clean run, produces new `.ts` files for the DHT projection views under `elohim/sdk/schemas/generated-ts/views/`.

- [ ] **Step 4: Commit (DHT projection codegen wiring)**

Source of truth: DHT.

```bash
git add -A elohim/sdk/schemas/scripts/codegen-ts.mjs elohim/sdk/schemas/generated-ts/
git commit -m "schema(recovery-m4): wire new views into TS codegen"
```

---

## Phase B — DNA integrity + coordinator (5 tasks)

### Task B.1: Soften `validate_key_revocation` for trigger-type-aware required_votes

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs` (~line 1804)

- [ ] **Step 1: Read current validator**

```bash
cd /projects/elohim && sed -n '1800,1840p' elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
```

- [ ] **Step 2: Replace the `required_votes` rule**

In `validate_key_revocation`, replace:
```rust
    // Minimum required votes is 2
    if revocation.required_votes < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "required_votes must be at least 2 for security".to_string(),
        ));
    }
```

with:
```rust
    // M4: trigger-type-aware required_votes.
    // Voluntary (self-revocation) has no quorum — required_votes must be exactly 1.
    // Steward-vote (emergency-contact quorum) and challenge (M5 specialist stub)
    // require at least 2 votes for security.
    match revocation.trigger_type.as_str() {
        "voluntary" => {
            if revocation.required_votes != 1 {
                return Ok(ValidateCallbackResult::Invalid(
                    "voluntary revocation must have required_votes == 1".to_string(),
                ));
            }
        }
        "steward_vote" | "challenge" => {
            if revocation.required_votes < 2 {
                return Ok(ValidateCallbackResult::Invalid(
                    "quorum revocation must have required_votes >= 2".to_string(),
                ));
            }
        }
        other => {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "Invalid trigger_type '{}'. Must be one of: voluntary, steward_vote, challenge",
                other
            )));
        }
    }
```

- [ ] **Step 3: Verify build**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```
Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
git commit -m "dna(imagodei-integrity): trigger-type-aware required_votes for KeyRevocation"
```

### Task B.2: Add `count_approved_revocation_votes` helper

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (near other M3 helpers, ~line 1650)

- [ ] **Step 1: Locate the M3 helpers block**

```bash
cd /projects/elohim && grep -n "fn compute_required_witness_count\|fn is_active_emergency_contact" elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
```

- [ ] **Step 2: Add the helper**

Insert after `compute_required_witness_count`:

```rust
/// Count approved votes on a KeyRevocation by traversing RevocationToVote links.
///
/// Only votes with `approved == true` count toward the quorum threshold.
/// Rejections are preserved in the DHT for audit but never advance the
/// pending -> effective transition.
fn count_approved_revocation_votes(revocation_id: &str) -> ExternResult<u32> {
    let anchor_path = Path::from(format!("revocation_votes:{}", revocation_id))
        .path_entry_hash()?;
    let links = get_links(
        GetLinksInputBuilder::try_new(anchor_path, LinkTypes::RevocationToVote)?.build(),
    )?;

    let mut approved_count: u32 = 0;
    for link in links {
        let vote_hash: ActionHash = link.target.into_action_hash()
            .ok_or_else(|| wasm_error!("RevocationToVote link target was not an ActionHash"))?;
        let record = must_get_valid_record(vote_hash)?;
        let vote: RevocationVote = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!("failed to deserialize RevocationVote: {e}"))?
            .ok_or_else(|| wasm_error!("RevocationVote record missing entry data"))?;
        if vote.approved {
            approved_count += 1;
        }
    }

    Ok(approved_count)
}
```

Note: the `Path::from(format!("revocation_votes:{}", revocation_id))` pattern mirrors how the M3 code builds per-revocation anchors. If the existing code uses a different anchor construction (check the M3 witness-anchor code in the same file), conform to that.

- [ ] **Step 3: Verify build**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```
Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "dna(imagodei): add count_approved_revocation_votes helper"
```

### Task B.3: Extend `RecoveryV2Signal` with three new variants (DNA side)

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (find `RecoveryV2Signal` definition)

- [ ] **Step 1: Locate the DNA-side enum**

```bash
cd /projects/elohim && grep -n "RecoveryV2Signal" elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs | head
```

- [ ] **Step 2: Add three variants**

Extend the enum (keep the existing `#[serde(tag = "type")]` attribute — both sides must preserve it):

```rust
    #[serde(rename = "KeyRevocationRequested")]
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
    #[serde(rename = "RevocationVoteSubmitted")]
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
    #[serde(rename = "KeyRevocationEffective")]
    KeyRevocationEffective {
        revocation_id: String,
        revoked_key: String,
        human_id: String,
        effective_at: String,
        triggering_vote_id: Option<String>,
    },
```

If the existing variants don't use `#[serde(rename = "…")]` (i.e., they rely on the variant name as-is), omit the rename attributes on the new variants for consistency.

- [ ] **Step 3: Verify build**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```
Expected: clean build (enum additions only; no usage sites yet).

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "dna(imagodei): add M4 revocation signal variants to RecoveryV2Signal"
```

### Task B.4: Implement `create_self_revocation`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (insert near the other recovery functions, ~line 1713-1923)

- [ ] **Step 1: Read the M3 `create_recovery_request` precedent**

```bash
cd /projects/elohim && sed -n '1680,1760p' elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
```

This shows the pattern for: UUID generation (note: UUID is a stable external reference for signals; the **canonical DHT identity is the ActionHash** content-addressed on the DHT source chain), entry creation, link creation, signal emission.

- [ ] **Step 2: Add the input struct and coordinator function**

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateSelfRevocationInput {
    pub revoked_key: AgentPubKey,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyRevocationOutput {
    pub revocation_id: String,
    pub entry_hash: EntryHash,
    pub action_hash: ActionHash,
}

/// M4: Self-revocation. A human with a valid agent key voluntarily revokes
/// a different (compromised) key they control. Single-cell authority, no
/// quorum, no witnesses.
#[hdk_extern]
pub fn create_self_revocation(input: CreateSelfRevocationInput) -> ExternResult<KeyRevocationOutput> {
    let caller_pubkey = agent_info()?.agent_initial_pubkey;
    let human_id = resolve_human_id_for_agent(&caller_pubkey)?;

    // Ensure revoked_key is associated with this human.
    // Reuse the M3 agent-key resolution pattern: resolve_human_id_for_agent
    // on revoked_key must return the same human_id.
    let owner_human_id = resolve_human_id_for_agent(&input.revoked_key)?;
    if owner_human_id != human_id {
        return Err(wasm_error!(
            "create_self_revocation: caller does not control revoked_key (different human_id)"
        ));
    }

    if !REVOCATION_REASONS.contains(&input.reason.as_str()) {
        return Err(wasm_error!(
            "create_self_revocation: invalid reason '{}'. Must be one of {:?}",
            input.reason, REVOCATION_REASONS
        ));
    }

    let now = sys_time()?.to_string();
    // UUID is a stable external reference; canonical DHT identity is the ActionHash (content-addressed on source chain).
    let revocation_id = format!("rev-{}", now); // coordinator-generated UUID (format matches M3 convention)

    let revocation = KeyRevocation {
        id: revocation_id.clone(),
        human_id: human_id.clone(),
        revoked_key: bytes_to_b64(input.revoked_key.get_raw_39()),
        reason: input.reason.clone(),
        initiated_by: human_id.clone(),
        trigger_type: "voluntary".to_string(),
        required_votes: 1,
        current_votes: 1,
        votes_json: String::new(), // legacy field, unused by M4
        threshold_reached: true,
        effective_at: Some(now.clone()),
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    let action_hash = create_entry(&EntryTypes::KeyRevocation(revocation.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::KeyRevocation(revocation.clone()))?;

    // Dual-anchor primacy: both RevokedKeyToRevocation and HumanToKeyRevocation are first-class.
    create_link(
        anchor_hash(&format!("revocation:{}", revocation_id))?,
        entry_hash.clone(),
        LinkTypes::IdToKeyRevocation,
        LinkTag::new(revocation_id.as_bytes()),
    )?;
    create_link(
        anchor_hash(&format!("human_revocations:{}", human_id))?,
        entry_hash.clone(),
        LinkTypes::HumanToKeyRevocation,
        LinkTag::new(revocation_id.as_bytes()),
    )?;
    create_link(
        anchor_hash(&format!("revoked_key:{}", revocation.revoked_key))?,
        entry_hash.clone(),
        LinkTypes::RevokedKeyToRevocation,
        LinkTag::new(revocation_id.as_bytes()),
    )?;
    // Voluntary is effective on creation — go straight to EffectiveRevocations anchor.
    create_link(
        anchor_hash("effective_revocations")?,
        entry_hash.clone(),
        LinkTypes::EffectiveRevocations,
        LinkTag::new(revocation_id.as_bytes()),
    )?;

    // Emit both signals atomically: Requested + Effective.
    emit_signal(RecoveryV2Signal::KeyRevocationRequested {
        id: revocation.id.clone(),
        human_id: revocation.human_id.clone(),
        revoked_key: revocation.revoked_key.clone(),
        reason: revocation.reason.clone(),
        trigger_type: revocation.trigger_type.clone(),
        initiated_by: revocation.initiated_by.clone(),
        required_votes: revocation.required_votes,
        current_votes: revocation.current_votes,
        threshold_reached: revocation.threshold_reached,
        effective_at: revocation.effective_at.clone(),
        created_at: revocation.created_at.clone(),
    })?;

    emit_signal(RecoveryV2Signal::KeyRevocationEffective {
        revocation_id: revocation.id.clone(),
        revoked_key: revocation.revoked_key.clone(),
        human_id: revocation.human_id.clone(),
        effective_at: now,
        triggering_vote_id: None,
    })?;

    Ok(KeyRevocationOutput { revocation_id, entry_hash, action_hash })
}
```

Notes:
- `anchor_hash(s)` is a local helper. If an existing helper with a different name (e.g., `path_to_entry_hash` or inline `Path::from(s).path_entry_hash()`) is used in the M3 code, adopt that convention. Inspect `create_recovery_request` for the exact shape.
- `bytes_to_b64` — if not available, use the existing pubkey-to-string convention used elsewhere in the file (grep for `revoked_key` or `agent_pubkey_string`).
- `REVOCATION_REASONS` — already declared in imagodei_integrity; import or reuse.

- [ ] **Step 3: Verify build**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```
Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "dna(imagodei): add create_self_revocation coordinator"
```

### Task B.5: Implement `create_revocation_request` (emergency-contact quorum path)

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`

- [ ] **Step 1: Add input struct and function**

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateRevocationRequestInput {
    pub target_human_id: String,
    pub revoked_key: AgentPubKey,
    pub reason: String,
}

/// M4: Emergency-contact revocation request. The caller must be an active
/// emergency contact of `target_human_id`. Creates a pending KeyRevocation
/// with quorum threshold = compute_required_witness_count(active_emergency_contact_count).
#[hdk_extern]
pub fn create_revocation_request(
    input: CreateRevocationRequestInput,
) -> ExternResult<KeyRevocationOutput> {
    let caller_pubkey = agent_info()?.agent_initial_pubkey;
    let caller_human_id = resolve_human_id_for_agent(&caller_pubkey)?;

    // Gate: caller is an active emergency contact for target_human_id
    if !is_active_emergency_contact(&input.target_human_id, &caller_human_id)? {
        return Err(wasm_error!(
            "create_revocation_request: caller is not an active emergency contact for {}",
            input.target_human_id
        ));
    }

    // Gate: revoked_key belongs to target_human_id
    let owner_human_id = resolve_human_id_for_agent(&input.revoked_key)?;
    if owner_human_id != input.target_human_id {
        return Err(wasm_error!(
            "create_revocation_request: revoked_key does not belong to target_human_id"
        ));
    }

    if !REVOCATION_REASONS.contains(&input.reason.as_str()) {
        return Err(wasm_error!(
            "create_revocation_request: invalid reason '{}'. Must be one of {:?}",
            input.reason, REVOCATION_REASONS
        ));
    }

    // TODO(M4-post): revisit whether revocation quorum should diverge from
    // recovery quorum. For now, parity with M3 keeps the two paths coherent.
    let contact_count = count_active_emergency_contacts(&input.target_human_id)?;
    let required = compute_required_witness_count(contact_count);

    let now = sys_time()?.to_string();
    let revocation_id = format!("rev-{}", now);

    let revocation = KeyRevocation {
        id: revocation_id.clone(),
        human_id: input.target_human_id.clone(),
        revoked_key: bytes_to_b64(input.revoked_key.get_raw_39()),
        reason: input.reason.clone(),
        initiated_by: caller_human_id.clone(),
        trigger_type: "steward_vote".to_string(),
        required_votes: required,
        current_votes: 0,
        votes_json: String::new(),
        threshold_reached: false,
        effective_at: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    let action_hash = create_entry(&EntryTypes::KeyRevocation(revocation.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::KeyRevocation(revocation.clone()))?;

    // Dual-anchor + PendingRevocations (quorum is not yet met).
    create_link(anchor_hash(&format!("revocation:{}", revocation_id))?, entry_hash.clone(),
        LinkTypes::IdToKeyRevocation, LinkTag::new(revocation_id.as_bytes()))?;
    create_link(anchor_hash(&format!("human_revocations:{}", input.target_human_id))?, entry_hash.clone(),
        LinkTypes::HumanToKeyRevocation, LinkTag::new(revocation_id.as_bytes()))?;
    create_link(anchor_hash(&format!("revoked_key:{}", revocation.revoked_key))?, entry_hash.clone(),
        LinkTypes::RevokedKeyToRevocation, LinkTag::new(revocation_id.as_bytes()))?;
    create_link(anchor_hash("pending_revocations")?, entry_hash.clone(),
        LinkTypes::PendingRevocations, LinkTag::new(revocation_id.as_bytes()))?;

    emit_signal(RecoveryV2Signal::KeyRevocationRequested {
        id: revocation.id.clone(),
        human_id: revocation.human_id.clone(),
        revoked_key: revocation.revoked_key.clone(),
        reason: revocation.reason.clone(),
        trigger_type: revocation.trigger_type.clone(),
        initiated_by: revocation.initiated_by.clone(),
        required_votes: revocation.required_votes,
        current_votes: revocation.current_votes,
        threshold_reached: revocation.threshold_reached,
        effective_at: revocation.effective_at.clone(),
        created_at: revocation.created_at.clone(),
    })?;

    Ok(KeyRevocationOutput { revocation_id, entry_hash, action_hash })
}
```

- [ ] **Step 2: Verify build**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```
Expected: clean build.

- [ ] **Step 3: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "dna(imagodei): add create_revocation_request (emergency-contact quorum path)"
```

### Task B.6: Implement `submit_revocation_vote`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`

- [ ] **Step 1: Read M3 `submit_intimate_witness` for the pattern**

```bash
cd /projects/elohim && sed -n '1923,2030p' elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
```

- [ ] **Step 2: Add input struct and function**

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitRevocationVoteInput {
    pub revocation_id: String,
    pub approved: bool,
    pub attestation: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RevocationVoteOutput {
    pub vote_id: String,
    pub current_votes: u32,
    pub required_votes: u32,
    pub threshold_now_reached: bool,
}

/// M4: Submit an emergency-contact vote on a pending KeyRevocation.
/// On the threshold-meeting vote, the coordinator updates the KeyRevocation
/// entry (flips threshold_reached, sets effective_at), moves it from
/// PendingRevocations to EffectiveRevocations, and emits both
/// RevocationVoteSubmitted and KeyRevocationEffective signals.
#[hdk_extern]
pub fn submit_revocation_vote(
    input: SubmitRevocationVoteInput,
) -> ExternResult<RevocationVoteOutput> {
    let caller_pubkey = agent_info()?.agent_initial_pubkey;
    let caller_human_id = resolve_human_id_for_agent(&caller_pubkey)?;

    if input.attestation.trim().is_empty() {
        return Err(wasm_error!("submit_revocation_vote: attestation cannot be empty"));
    }

    // Load the KeyRevocation via IdToKeyRevocation anchor.
    let revocation_anchor = anchor_hash(&format!("revocation:{}", input.revocation_id))?;
    let revocation_links = get_links(
        GetLinksInputBuilder::try_new(revocation_anchor, LinkTypes::IdToKeyRevocation)?.build(),
    )?;
    let revocation_link = revocation_links.first()
        .ok_or_else(|| wasm_error!("submit_revocation_vote: no KeyRevocation with id {}", input.revocation_id))?;
    let revocation_hash: ActionHash = revocation_link.target.clone().into_action_hash()
        .ok_or_else(|| wasm_error!("IdToKeyRevocation target was not an ActionHash"))?;
    let revocation_record = must_get_valid_record(revocation_hash.clone())?;
    let revocation: KeyRevocation = revocation_record.entry().to_app_option()
        .map_err(|e| wasm_error!("deserialize KeyRevocation: {e}"))?
        .ok_or_else(|| wasm_error!("KeyRevocation record missing entry"))?;

    // Gate: votes only apply to the steward_vote path. Voluntary path is already effective.
    if revocation.trigger_type != "steward_vote" {
        return Err(wasm_error!(
            "submit_revocation_vote: revocation {} has trigger_type={}, votes not accepted",
            input.revocation_id, revocation.trigger_type
        ));
    }

    if revocation.threshold_reached {
        return Err(wasm_error!(
            "submit_revocation_vote: revocation {} already effective", input.revocation_id
        ));
    }

    // Gate: caller is an active emergency contact for the target human.
    if !is_active_emergency_contact(&revocation.human_id, &caller_human_id)? {
        return Err(wasm_error!(
            "submit_revocation_vote: caller is not an active emergency contact for {}",
            revocation.human_id
        ));
    }

    // Gate: no existing vote from this steward on this revocation.
    let steward_anchor = anchor_hash(&format!("steward_revocation_votes:{}", caller_human_id))?;
    let steward_vote_links = get_links(
        GetLinksInputBuilder::try_new(steward_anchor.clone(), LinkTypes::StewardToRevocationVote)?
            .build(),
    )?;
    for link in &steward_vote_links {
        let vote_hash: ActionHash = link.target.clone().into_action_hash()
            .ok_or_else(|| wasm_error!("StewardToRevocationVote target was not an ActionHash"))?;
        let rec = must_get_valid_record(vote_hash)?;
        let prior_vote: RevocationVote = rec.entry().to_app_option()
            .map_err(|e| wasm_error!("deserialize RevocationVote: {e}"))?
            .ok_or_else(|| wasm_error!("RevocationVote record missing entry"))?;
        if prior_vote.revocation_id == input.revocation_id {
            return Err(wasm_error!(
                "submit_revocation_vote: steward {} has already voted on revocation {}",
                caller_human_id, input.revocation_id
            ));
        }
    }

    let now = sys_time()?.to_string();
    let vote_id = format!("vote-{}", now);

    let vote = RevocationVote {
        id: vote_id.clone(),
        revocation_id: input.revocation_id.clone(),
        steward_id: caller_human_id.clone(),
        approved: input.approved,
        attestation: input.attestation.clone(),
        voted_at: now.clone(),
    };

    let vote_action = create_entry(&EntryTypes::RevocationVote(vote.clone()))?;
    let vote_entry = hash_entry(&EntryTypes::RevocationVote(vote.clone()))?;

    // Create the three vote-side links.
    create_link(anchor_hash(&format!("revocation_vote:{}", vote_id))?, vote_entry.clone(),
        LinkTypes::IdToRevocationVote, LinkTag::new(vote_id.as_bytes()))?;
    create_link(anchor_hash(&format!("revocation_votes:{}", input.revocation_id))?, vote_entry.clone(),
        LinkTypes::RevocationToVote, LinkTag::new(vote_id.as_bytes()))?;
    create_link(steward_anchor, vote_entry.clone(),
        LinkTypes::StewardToRevocationVote, LinkTag::new(vote_id.as_bytes()))?;

    // Recompute threshold (count approved votes from link traversal).
    let approved_count = count_approved_revocation_votes(&input.revocation_id)?;
    let threshold_now_reached = approved_count >= revocation.required_votes;

    if threshold_now_reached {
        // Update the KeyRevocation entry: flip threshold_reached, set effective_at.
        let mut updated = revocation.clone();
        updated.current_votes = approved_count;
        updated.threshold_reached = true;
        updated.effective_at = Some(now.clone());
        updated.updated_at = now.clone();
        update_entry(revocation_hash.clone(), &EntryTypes::KeyRevocation(updated.clone()))?;

        // Move from PendingRevocations to EffectiveRevocations.
        let pending_anchor = anchor_hash("pending_revocations")?;
        let pending_links = get_links(
            GetLinksInputBuilder::try_new(pending_anchor.clone(), LinkTypes::PendingRevocations)?
                .build(),
        )?;
        for link in pending_links {
            if link.tag.0 == input.revocation_id.as_bytes() {
                delete_link(link.create_link_hash)?;
            }
        }
        create_link(
            anchor_hash("effective_revocations")?,
            revocation_record.signed_action().hashed.content_address().clone().into(),
            LinkTypes::EffectiveRevocations,
            LinkTag::new(input.revocation_id.as_bytes()),
        )?;

        emit_signal(RecoveryV2Signal::RevocationVoteSubmitted {
            id: vote_id.clone(),
            revocation_id: input.revocation_id.clone(),
            steward_id: caller_human_id.clone(),
            approved: input.approved,
            attestation: input.attestation.clone(),
            voted_at: now.clone(),
            current_votes: approved_count,
            required_votes: revocation.required_votes,
            threshold_now_reached: true,
        })?;

        emit_signal(RecoveryV2Signal::KeyRevocationEffective {
            revocation_id: input.revocation_id.clone(),
            revoked_key: revocation.revoked_key.clone(),
            human_id: revocation.human_id.clone(),
            effective_at: now.clone(),
            triggering_vote_id: Some(vote_id.clone()),
        })?;
    } else {
        emit_signal(RecoveryV2Signal::RevocationVoteSubmitted {
            id: vote_id.clone(),
            revocation_id: input.revocation_id.clone(),
            steward_id: caller_human_id.clone(),
            approved: input.approved,
            attestation: input.attestation.clone(),
            voted_at: now.clone(),
            current_votes: approved_count,
            required_votes: revocation.required_votes,
            threshold_now_reached: false,
        })?;
    }

    let _ = vote_action; // silences unused warning if not returned

    Ok(RevocationVoteOutput {
        vote_id,
        current_votes: approved_count,
        required_votes: revocation.required_votes,
        threshold_now_reached,
    })
}
```

Notes:
- The `pending_links` deletion pattern uses `LinkTag` equality. If the M3 codebase uses a different idiom for link removal (e.g., storing create_link_hash at create time, or using `delete_link` with the returned hash), adopt that.
- The `link.create_link_hash` accessor may vary by HDK version — conform to how M3 deletes links.
- `content_address()` on the record — if HDK uses a different accessor, adapt.

- [ ] **Step 3: Verify build**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```
Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "dna(imagodei): add submit_revocation_vote with threshold-driven effective transition"
```

### Task B.7: Add revocation-floor gate to `commit_key_rotation`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (inside existing `commit_key_rotation`, ~line 1788)

- [ ] **Step 1: Read the existing function**

```bash
cd /projects/elohim && sed -n '1780,1860p' elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
```

Identify where the freeze-floor gate lives. Insert the revocation-floor gate immediately after it.

- [ ] **Step 2: Add the revocation-floor gate**

```rust
    // M4: revocation-floor gate.
    // If a pending or effective KeyRevocation exists for the rotating_from
    // pubkey, block the rotation. No authority-layer exemption — revocation
    // is structural (a revoked key must not produce valid rotations under
    // any claimed authority), intentionally asymmetric with the freeze-floor
    // gate which exempts CryptographicQuorum.
    let rotating_from_b64 = bytes_to_b64(input.rotating_from.get_raw_39());

    let pending_anchor = anchor_hash("pending_revocations")?;
    let pending_links = get_links(
        GetLinksInputBuilder::try_new(pending_anchor, LinkTypes::PendingRevocations)?.build(),
    )?;
    let effective_anchor = anchor_hash("effective_revocations")?;
    let effective_links = get_links(
        GetLinksInputBuilder::try_new(effective_anchor, LinkTypes::EffectiveRevocations)?.build(),
    )?;

    for (link, status) in pending_links.iter().map(|l| (l, "pending"))
        .chain(effective_links.iter().map(|l| (l, "effective"))) {
        let rev_hash: ActionHash = link.target.clone().into_action_hash()
            .ok_or_else(|| wasm_error!("Revocation anchor target was not an ActionHash"))?;
        let rev_record = must_get_valid_record(rev_hash)?;
        let rev: KeyRevocation = rev_record.entry().to_app_option()
            .map_err(|e| wasm_error!("deserialize KeyRevocation: {e}"))?
            .ok_or_else(|| wasm_error!("KeyRevocation record missing entry"))?;
        if rev.revoked_key == rotating_from_b64 {
            return Err(wasm_error!(
                "commit_key_rotation blocked: key {} has a {} revocation ({}). Resolve or await the revocation before rotating.",
                rotating_from_b64, status, rev.id
            ));
        }
    }
```

Note: `input.rotating_from` — adapt to the actual field name used on `CommitKeyRotationInput` (inspect the struct definition nearby).

- [ ] **Step 3: Verify build**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check && just pack
```
Expected: clean build AND clean pack.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "dna(imagodei): add revocation-floor gate to commit_key_rotation"
```

---

## Phase C — Storage signal mirror + views (4 tasks)

### Task C.1: Mirror the three new signal variants in storage

**Files:**
- Modify: `elohim/elohim-storage/src/signals.rs` (~line 658)

- [ ] **Step 1: Read current `RecoveryV2Signal` enum**

```bash
cd /projects/elohim && sed -n '655,705p' elohim/elohim-storage/src/signals.rs
```

- [ ] **Step 2: Add three variants matching DNA side exactly**

After the existing variants, add:

```rust
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
```

Field order and names **must** match the DNA-side enum exactly.

- [ ] **Step 3: Extend the dispatcher's exhaustive match with `unimplemented!()` stubs**

In `handle_recovery_v2_signal`, add arms that panic for now (Task F will implement them):

```rust
        RecoveryV2Signal::KeyRevocationRequested { .. } => {
            unimplemented!("Task F.1: handle KeyRevocationRequested projection")
        }
        RecoveryV2Signal::RevocationVoteSubmitted { .. } => {
            unimplemented!("Task F.2: handle RevocationVoteSubmitted projection")
        }
        RecoveryV2Signal::KeyRevocationEffective { .. } => {
            unimplemented!("Task F.3: handle KeyRevocationEffective projection + eager sweep")
        }
```

This keeps the exhaustive match compiling; the stubs are removed in Phase F.

- [ ] **Step 4: Verify build**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -40
```
Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/signals.rs
git commit -m "storage(signals): mirror M4 revocation variants on RecoveryV2Signal"
```

### Task C.2: Create `key_revocations` diesel migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-24-010000_key_revocations/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-24-010000_key_revocations/down.sql`

- [ ] **Step 1: Inspect M3 precedent**

```bash
cd /projects/elohim && cat elohim/elohim-storage/migrations/2026-04-24-000000_recovery_witnesses/up.sql
```

- [ ] **Step 2: Create up.sql**

```sql
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
CREATE INDEX idx_key_revocations_pending ON key_revocations(threshold_reached);
```

- [ ] **Step 3: Create down.sql**

```sql
DROP INDEX IF EXISTS idx_key_revocations_pending;
DROP INDEX IF EXISTS idx_key_revocations_revoked_key;
DROP INDEX IF EXISTS idx_key_revocations_human;
DROP TABLE IF EXISTS key_revocations;
```

- [ ] **Step 4: Run migration**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -20
```
Expected: migrations embedded; schema.rs regenerates on next build that hits the diesel print_schema.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-04-24-010000_key_revocations/
git commit -m "storage(migrations): create key_revocations projection table"
```

### Task C.3: Create `revocation_votes` diesel migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-24-020000_revocation_votes/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-24-020000_revocation_votes/down.sql`

- [ ] **Step 1: Create up.sql**

```sql
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
    UNIQUE (revocation_id, steward_id)
);

CREATE INDEX idx_revocation_votes_revocation ON revocation_votes(revocation_id);
CREATE INDEX idx_revocation_votes_steward ON revocation_votes(steward_id);
```

- [ ] **Step 2: Create down.sql**

```sql
DROP INDEX IF EXISTS idx_revocation_votes_steward;
DROP INDEX IF EXISTS idx_revocation_votes_revocation;
DROP TABLE IF EXISTS revocation_votes;
```

- [ ] **Step 3: Run build to embed migration**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -20
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-04-24-020000_revocation_votes/
git commit -m "storage(migrations): create revocation_votes projection table"
```

### Task C.4: Add Rust view types + schema-contract tests

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Read M3 view and test patterns**

```bash
cd /projects/elohim && grep -n "RecoveryRequestView\|RecoveryWitnessView" elohim/elohim-storage/src/views.rs | head
grep -n "recovery_request_view_matches_schema\|recovery_witness_view_matches_schema" elohim/elohim-storage/tests/schema_contract.rs | head
```

- [ ] **Step 2: Add view types to `views.rs`**

```rust
/// Projection of an imagodei KeyRevocation DHT entry.
/// Source of truth: DHT. Rebuildable via signal replay.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/views/")]
pub struct KeyRevocationView {
    pub dht_anchor_hash: String,
    pub id: String,
    pub human_id: String,
    pub revoked_key: String,
    pub reason: String,
    pub trigger_type: String,
    pub initiated_by: String,
    pub required_votes: u32,
    pub current_votes: u32,
    pub threshold_reached: bool,
    pub effective_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Projection of an imagodei RevocationVote DHT entry.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/views/")]
pub struct RevocationVoteView {
    pub dht_anchor_hash: String,
    pub id: String,
    pub revocation_dht_anchor_hash: String,
    pub revocation_id: String,
    pub steward_id: String,
    pub approved: bool,
    pub attestation: String,
    pub voted_at: String,
}
```

Adjust `ts(export_to)` path to match the existing views' path (inspect `RecoveryRequestView` annotation).

- [ ] **Step 3: Add schema-contract tests**

```rust
#[test]
fn key_revocation_view_matches_schema() {
    let sample = KeyRevocationView {
        dht_anchor_hash: "hCAkTESTHASH".into(),
        id: "rev-2026-04-24T00:00:00Z".into(),
        human_id: "human-matthew".into(),
        revoked_key: "aBCD...=".into(),
        reason: "compromised".into(),
        trigger_type: "voluntary".into(),
        initiated_by: "human-matthew".into(),
        required_votes: 1,
        current_votes: 1,
        threshold_reached: true,
        effective_at: Some("2026-04-24T00:00:00Z".into()),
        created_at: "2026-04-24T00:00:00Z".into(),
        updated_at: "2026-04-24T00:00:00Z".into(),
    };
    assert_value_matches_schema(
        serde_json::to_value(&sample).unwrap(),
        "../../sdk/schemas/v1/views/key-revocation.schema.json",
    );
}

#[test]
fn revocation_vote_view_matches_schema() {
    let sample = RevocationVoteView {
        dht_anchor_hash: "hCAkVOTEHASH".into(),
        id: "vote-2026-04-24T00:00:00Z".into(),
        revocation_dht_anchor_hash: "hCAkREVHASH".into(),
        revocation_id: "rev-2026-04-24T00:00:00Z".into(),
        steward_id: "human-jessica".into(),
        approved: true,
        attestation: "Key was captured by attacker.".into(),
        voted_at: "2026-04-24T00:01:00Z".into(),
    };
    assert_value_matches_schema(
        serde_json::to_value(&sample).unwrap(),
        "../../sdk/schemas/v1/views/revocation-vote.schema.json",
    );
}
```

Replace `assert_value_matches_schema` with the actual helper name used by existing tests in `schema_contract.rs` (inspect file for exact symbol). These tests verify the DHT projection view shape.

- [ ] **Step 4: Run schema-contract tests (DHT projection verification)**

Source of truth: DHT. Tests confirm the projection view matches the DHT entry shape.

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --release schema_contract 2>&1 | tail -30
```
Expected: both new tests pass.

- [ ] **Step 5: Regenerate TS types**

```bash
cd /projects/elohim/elohim/elohim-storage && cargo test export_bindings 2>&1 | tail
```
Expected: new TS files generated under `sdk/storage-client-ts/src/generated/views/`.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/views.rs elohim/elohim-storage/tests/schema_contract.rs elohim/sdk/storage-client-ts/src/generated/
git commit -m "storage(views): add KeyRevocationView + RevocationVoteView with schema contract tests"
```

---

## Phase D — Storage projection handlers + outbound signals (4 tasks)

### Task D.1: Add diesel models + upsert helpers for `key_revocations`

**Files:**
- Modify: `elohim/elohim-storage/src/db/models.rs` (or the appropriate pillar file — inspect where `RecoveryRequestRow` lives)

- [ ] **Step 1: Locate existing RecoveryRequestRow / RecoveryWitnessRow**

```bash
cd /projects/elohim && grep -rn "RecoveryRequestRow\|RecoveryWitnessRow\|pub struct.*recovery" elohim/elohim-storage/src/db/ | head
```

- [ ] **Step 2: Add `KeyRevocationRow` + `upsert_key_revocation`**

Following the M3 pattern, add:

```rust
#[derive(Debug, Clone, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = crate::db::schema::key_revocations)]
pub struct KeyRevocationRow {
    pub dht_anchor_hash: String,
    pub id: String,
    pub human_id: String,
    pub revoked_key: String,
    pub reason: String,
    pub trigger_type: String,
    pub initiated_by: String,
    pub required_votes: i32,
    pub current_votes: i32,
    pub threshold_reached: i32,
    pub effective_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn upsert_key_revocation(
    conn: &mut SqliteConnection,
    row: &KeyRevocationRow,
) -> Result<(), diesel::result::Error> {
    use crate::db::schema::key_revocations::dsl::*;
    diesel::insert_into(key_revocations)
        .values(row)
        .on_conflict(dht_anchor_hash)
        .do_update()
        .set(row)
        .execute(conn)?;
    Ok(())
}
```

- [ ] **Step 3: Verify build**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -20
```

- [ ] **Step 4: Commit (DHT projection row — not canonical)**

Source of truth: DHT. Row is a local projection of the `KeyRevocation` DHT entry.

```bash
git add elohim/elohim-storage/src/db/models.rs elohim/elohim-storage/src/db/schema.rs
git commit -m "storage(db): add KeyRevocationRow diesel model and upsert helper"
```

### Task D.2: Add diesel models + insert helpers for `revocation_votes`

**Files:**
- Modify: `elohim/elohim-storage/src/db/models.rs`

- [ ] **Step 1: Add `RevocationVoteRow` + insert helper**

```rust
#[derive(Debug, Clone, Queryable, Insertable)]
#[diesel(table_name = crate::db::schema::revocation_votes)]
pub struct RevocationVoteRow {
    pub dht_anchor_hash: String,
    pub id: String,
    pub revocation_dht_anchor_hash: String,
    pub revocation_id: String,
    pub steward_id: String,
    pub approved: i32,
    pub attestation: String,
    pub voted_at: String,
}

pub fn insert_revocation_vote(
    conn: &mut SqliteConnection,
    row: &RevocationVoteRow,
) -> Result<(), diesel::result::Error> {
    use crate::db::schema::revocation_votes::dsl::*;
    diesel::insert_into(revocation_votes)
        .values(row)
        .on_conflict((revocation_id, steward_id))
        .do_nothing()  // second vote from same steward is a bug upstream; idempotent on projection
        .execute(conn)?;
    Ok(())
}

pub fn set_key_revocation_effective(
    conn: &mut SqliteConnection,
    target_dht_anchor_hash: &str,
    new_effective_at: &str,
    new_updated_at: &str,
) -> Result<(), diesel::result::Error> {
    use crate::db::schema::key_revocations::dsl::*;
    diesel::update(key_revocations.filter(dht_anchor_hash.eq(target_dht_anchor_hash)))
        .set((
            threshold_reached.eq(1),
            effective_at.eq(Some(new_effective_at.to_string())),
            updated_at.eq(new_updated_at.to_string()),
        ))
        .execute(conn)?;
    Ok(())
}
```

- [ ] **Step 2: Verify build**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -20
```

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/db/models.rs
git commit -m "storage(db): add RevocationVoteRow + set_key_revocation_effective helpers"
```

### Task D.3: Implement the three projection handler arms

**Files:**
- Modify: `elohim/elohim-storage/src/signals.rs` (in `handle_recovery_v2_signal`)

- [ ] **Step 1: Replace the three `unimplemented!()` stubs from Task C.1**

```rust
        RecoveryV2Signal::KeyRevocationRequested {
            id, human_id, revoked_key, reason, trigger_type, initiated_by,
            required_votes, current_votes, threshold_reached, effective_at, created_at,
        } => {
            let dht_anchor_hash = /* obtained from signal envelope — pattern from RecoveryRequestCreated */;
            let row = KeyRevocationRow {
                dht_anchor_hash: dht_anchor_hash.clone(),
                id: id.clone(),
                human_id: human_id.clone(),
                revoked_key: revoked_key.clone(),
                reason,
                trigger_type: trigger_type.clone(),
                initiated_by,
                required_votes: required_votes as i32,
                current_votes: current_votes as i32,
                threshold_reached: if threshold_reached { 1 } else { 0 },
                effective_at: effective_at.clone(),
                created_at: created_at.clone(),
                updated_at: created_at.clone(),
            };
            upsert_key_revocation(conn, &row)?;

            // Outbound reconciliation signal.
            emit_reconciled_signal(ImagodeiReconciledEvent::RevocationObserved {
                revocation_id: id,
                revoked_key,
                human_id,
                status: if threshold_reached { "effective-on-create".into() } else { "pending".into() },
                observed_at: created_at,
            });
        }
        RecoveryV2Signal::RevocationVoteSubmitted {
            id, revocation_id, steward_id, approved, attestation, voted_at,
            current_votes, required_votes, threshold_now_reached,
        } => {
            let vote_dht_anchor_hash = /* from envelope */;
            let revocation_dht_anchor_hash = /* lookup via key_revocations.id = revocation_id */;
            let row = RevocationVoteRow {
                dht_anchor_hash: vote_dht_anchor_hash,
                id,
                revocation_dht_anchor_hash,
                revocation_id: revocation_id.clone(),
                steward_id,
                approved: if approved { 1 } else { 0 },
                attestation,
                voted_at,
            };
            insert_revocation_vote(conn, &row)?;
            // Bump denormalized current_votes on the parent key_revocations row.
            use crate::db::schema::key_revocations::dsl as kr;
            diesel::update(kr::key_revocations.filter(kr::id.eq(&revocation_id)))
                .set(kr::current_votes.eq(current_votes as i32))
                .execute(conn)?;
            let _ = required_votes;
            let _ = threshold_now_reached;
        }
        RecoveryV2Signal::KeyRevocationEffective {
            revocation_id, revoked_key, human_id, effective_at, triggering_vote_id,
        } => {
            // Lookup target row.
            use crate::db::schema::key_revocations::dsl as kr;
            let target_row: Option<KeyRevocationRow> = kr::key_revocations
                .filter(kr::id.eq(&revocation_id))
                .first::<KeyRevocationRow>(conn)
                .optional()?;
            if let Some(row) = target_row {
                set_key_revocation_effective(conn, &row.dht_anchor_hash, &effective_at, &effective_at)?;

                // Eager cache-invalidation sweep:
                // Today: peer_identity_bindings (if present) keyed by pubkey.
                // Phase 2B hook: extend to epr_atoms WHERE signer_cid = revoked_key.
                sweep_dependent_caches_on_revocation(conn, &revoked_key)?;

                emit_reconciled_signal(ImagodeiReconciledEvent::RevocationObserved {
                    revocation_id,
                    revoked_key,
                    human_id,
                    status: "effective".into(),
                    observed_at: effective_at,
                });
            }
            let _ = triggering_vote_id;
        }
```

- [ ] **Step 2: Add `sweep_dependent_caches_on_revocation` helper**

```rust
/// Sweep dependent cached state tied to a revoked key. Eager, bounded,
/// indexed (not a table scan). Extended in Phase 2B to include epr_atoms.
fn sweep_dependent_caches_on_revocation(
    conn: &mut SqliteConnection,
    revoked_key: &str,
) -> Result<(), diesel::result::Error> {
    // peer_identity_bindings — if the table exists and has a matching pubkey column, clear it.
    // Today this is a best-effort sweep. Log before/after row counts for observability.
    // TODO(Phase 2B): extend to: UPDATE epr_atoms SET verified_at = NULL WHERE signer_cid = revoked_key;
    let _ = conn;
    let _ = revoked_key;
    Ok(())
}
```

Note: if `peer_identity_bindings` doesn't exist yet, leave the helper as a stub that logs. The Phase 2B extension point is documented in the TODO.

- [ ] **Step 3: Add the outbound event enum**

If `ImagodeiReconciledEvent` doesn't exist, declare it alongside the storage signal types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ImagodeiReconciledEvent {
    RevocationObserved {
        revocation_id: String,
        revoked_key: String,
        human_id: String,
        status: String, // "pending" | "effective-on-create" | "effective"
        observed_at: String,
    },
}

fn emit_reconciled_signal(event: ImagodeiReconciledEvent) {
    // Wire to the existing outbound signal infrastructure (e.g., broadcast channel
    // or SSE stream). If a generic emit_imagodei_event() helper exists, use that.
    tracing::info!(target = "imagodei.reconciled", ?event, "reconciled event");
    let _ = event;
}
```

If a broadcast mechanism is needed, scaffold a TODO for the subscriber infrastructure; today the log line is sufficient for M4 (M5 consumers will subscribe).

- [ ] **Step 4: Verify build**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -40
```
Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/signals.rs
git commit -m "storage(signals): implement M4 revocation projection handlers with eager sweep + outbound reconciled event"
```

### Task D.4: Extend the mesh publish-intent extractor

**Files:**
- Modify: `elohim/elohim-storage/src/signals.rs` (near `recovery_invitation_from_signal`)

- [ ] **Step 1: Add `RecoveryRevocationMessage` wire struct**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
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

- [ ] **Step 2: Add `recovery_revocation_from_signal` function**

```rust
/// Extract a RecoveryRevocationMessage publish intent from a RecoveryV2Signal.
/// Returns None for signals that don't emit a mesh notification.
pub fn recovery_revocation_from_signal(
    signal: &RecoveryV2Signal,
    sender_peer_id: &str,
) -> Option<RecoveryRevocationMessage> {
    let now = chrono::Utc::now().to_rfc3339();
    match signal {
        RecoveryV2Signal::KeyRevocationRequested {
            id, human_id, revoked_key, trigger_type, reason, threshold_reached, ..
        } => Some(RecoveryRevocationMessage {
            revocation_id: id.clone(),
            human_id: human_id.clone(),
            revoked_key: revoked_key.clone(),
            trigger_type: trigger_type.clone(),
            reason: reason.clone(),
            status: if *threshold_reached { "effective".into() } else { "pending".into() },
            sender_peer_id: sender_peer_id.to_string(),
            sent_at: now,
        }),
        RecoveryV2Signal::KeyRevocationEffective {
            revocation_id, revoked_key, human_id, ..
        } => Some(RecoveryRevocationMessage {
            revocation_id: revocation_id.clone(),
            human_id: human_id.clone(),
            revoked_key: revoked_key.clone(),
            trigger_type: String::new(), // not carried on Effective signal; mesh subscribers key by revocation_id
            reason: String::new(),
            status: "effective".into(),
            sender_peer_id: sender_peer_id.to_string(),
            sent_at: now,
        }),
        _ => None,
    }
}
```

- [ ] **Step 3: Verify build**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/signals.rs
git commit -m "storage(signals): add recovery_revocation_from_signal publish-intent extractor"
```

---

## Phase E — Mesh substrate (2 tasks)

### Task E.1: Register the `recovery.revocation` gossipsub topic

**Files:**
- Modify: wherever the `recovery.invitation` topic is registered (discover via grep)

- [ ] **Step 1: Locate current topic registration**

```bash
cd /projects/elohim && grep -rn "recovery.invitation\|recovery_invitation" elohim/elohim-storage/src/ | head
```

- [ ] **Step 2: Register the new topic parallel to the existing one**

Add a topic constant:
```rust
pub const RECOVERY_REVOCATION_TOPIC: &str = "recovery.revocation";
```

In the gossipsub behaviour subscription block (near the existing `recovery.invitation` subscribe call), add:
```rust
behaviour.subscribe(&gossipsub::IdentTopic::new(RECOVERY_REVOCATION_TOPIC))
    .map_err(|e| eyre!("subscribe recovery.revocation: {e}"))?;
```

- [ ] **Step 3: Wire subscribe/log stub for inbound messages**

Pattern-match on the topic in the gossipsub message handler; log the deserialized `RecoveryRevocationMessage` without side effects for M4. Active consumer logic lands in M5.

```rust
if msg.topic == gossipsub::TopicHash::from_raw(RECOVERY_REVOCATION_TOPIC) {
    match rmp_serde::from_slice::<RecoveryRevocationMessage>(&msg.data) {
        Ok(m) => tracing::info!(target = "recovery.revocation.inbound", ?m, "received"),
        Err(e) => tracing::warn!(target = "recovery.revocation.inbound", ?e, "deserialize failed"),
    }
}
```

- [ ] **Step 4: Fresh-tree verify (MANDATORY before commit per swarm-composition memory)**

```bash
cd /projects/elohim/elohim/elohim-storage && cargo clean && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -40
```
Expected: clean build from fresh tree.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/
git commit -m "storage(mesh): register recovery.revocation gossipsub topic with subscribe/log stub"
```

### Task E.2: Wire the publish path from DNA signals into gossipsub

**Files:**
- Modify: wherever `recovery_invitation_from_signal` output is published to gossipsub (discover via grep)

- [ ] **Step 1: Locate current publish wiring**

```bash
cd /projects/elohim && grep -rn "recovery_invitation_from_signal\|publish.*recovery" elohim/elohim-storage/src/ | head
```

- [ ] **Step 2: Add parallel publish for revocation messages**

In the signal-to-mesh bridge (typically a function that receives `RecoveryV2Signal`), add a branch alongside the invitation publish:

```rust
if let Some(msg) = recovery_revocation_from_signal(&signal, &local_peer_id_str) {
    let bytes = rmp_serde::to_vec(&msg)
        .map_err(|e| eyre!("serialize RecoveryRevocationMessage: {e}"))?;
    let topic = gossipsub::IdentTopic::new(RECOVERY_REVOCATION_TOPIC);
    if let Err(e) = behaviour.publish(topic, bytes) {
        tracing::warn!(target = "recovery.revocation.outbound", ?e, "publish failed");
    }
}
```

- [ ] **Step 3: Fresh-tree verify**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/
git commit -m "storage(mesh): publish revocation messages to recovery.revocation on KeyRevocation* signals"
```

---

## Phase F — Tests (3 tasks)

### Task F.1: sweettest scenarios

**Files:**
- Create: `elohim/holochain/tests/sweettest/tests/recovery_m4_revocation.rs` (or append to an existing recovery file — follow the M3 convention)

- [ ] **Step 1: Inspect M3 sweettest scaffolding**

```bash
cd /projects/elohim && ls elohim/holochain/tests/sweettest/tests/ | grep -i recovery
```

- [ ] **Step 2: Write eight scenarios**

Use the M3 recovery sweettest as template. Scenarios (full code not shown — follow M3's DNA-install, agent-setup, call-coordinator, assert-state pattern):

```rust
#[tokio::test(flavor = "multi_thread")]
async fn self_revocation_happy_path() {
    // Setup: one human with two agent keys K1, K2.
    // Call create_self_revocation(revoked_key=K2, reason="compromised") as K1.
    // Assert: KeyRevocation entry exists with threshold_reached=true, effective_at=Some.
    // Assert: EffectiveRevocations anchor contains the revocation.
    // Assert: PendingRevocations anchor does NOT contain it.
    // Assert: two signals emitted in order — KeyRevocationRequested, KeyRevocationEffective.
}

#[tokio::test(flavor = "multi_thread")]
async fn emergency_contact_quorum_met() {
    // Setup: one human with 4 active emergency contacts.
    // Contact #1 calls create_revocation_request → threshold=compute_required_witness_count(4)=3.
    // Contacts #1, #2, #3 submit_revocation_vote(approved=true).
    // Assert: after vote #3, KeyRevocation.threshold_reached=true.
    // Assert: EffectiveRevocations anchor contains the revocation; PendingRevocations does not.
    // Assert: KeyRevocationEffective signal emitted with triggering_vote_id=Some(vote#3.id).
}

#[tokio::test(flavor = "multi_thread")]
async fn emergency_contact_quorum_not_met() {
    // Setup: one human with 4 active emergency contacts, threshold=3.
    // Contact #1 creates request; contacts #1, #2 vote approved=true; contact #3 votes approved=false.
    // Assert: threshold_reached=false.
    // Assert: revocation remains in PendingRevocations.
    // Assert: no KeyRevocationEffective signal emitted.
}

#[tokio::test(flavor = "multi_thread")]
async fn rotation_blocked_by_pending_revocation() {
    // Setup: human has current key K1; an emergency-contact revocation request is pending on K1.
    // Call commit_key_rotation(rotating_from=K1, rotating_to=K2) as the human.
    // Assert: error contains "pending revocation".
}

#[tokio::test(flavor = "multi_thread")]
async fn rotation_blocked_by_effective_revocation() {
    // Setup: human's K1 has an effective KeyRevocation.
    // Call commit_key_rotation(rotating_from=K1, rotating_to=K2).
    // Assert: error contains "effective revocation".
}

#[tokio::test(flavor = "multi_thread")]
async fn rotation_unaffected_by_revocation_of_other_key() {
    // Setup: human has K1 (current) and K2 (older, now revoked).
    // Call commit_key_rotation(rotating_from=K1, rotating_to=K3).
    // Assert: rotation succeeds.
}

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_vote_rejected() {
    // Setup: pending revocation; contact #1 votes; contact #1 votes again.
    // Assert: second call returns error containing "already voted".
}

#[tokio::test(flavor = "multi_thread")]
async fn non_emergency_contact_cannot_initiate() {
    // Setup: human A; human B (not an emergency contact of A).
    // B calls create_revocation_request(target_human_id=A, revoked_key=A's key).
    // Assert: error contains "not an active emergency contact".
}
```

- [ ] **Step 3: Verify tests run in nix shell**

Note: these cannot run in Eclipse Che — they require the nix shell with datachannel C deps. Per `feedback_shift_measure_jenkins`, CI is the measure. Locally at minimum:
```bash
cd /projects/elohim/elohim/holochain/tests/sweettest && cargo check 2>&1 | tail
```
Expected: compiles. Actual test execution deferred to Jenkins pipeline.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/tests/sweettest/tests/recovery_m4_revocation.rs
git commit -m "test(sweettest): M4 fast-path revocation scenarios (self + quorum + rotation gate)"
```

### Task F.2: a2o feature — self-revocation

**Files:**
- Create: `genesis/a2o/features/auth/revocation-self.feature`

- [ ] **Step 1: Write the feature file**

```gherkin
@recovery-m4
Feature: Self-revocation of a compromised device key
  As a human whose phone was stolen,
  I want to revoke the compromised device key from my laptop,
  so that the attacker's future actions signed by that key are rejected.

  Scenario: Matthew revokes his stolen phone's key from his laptop
    Given Matthew has two devices "laptop" and "phone"
    And the laptop holds agent key "K_laptop"
    And the phone holds agent key "K_phone"
    And Matthew's phone is stolen
    When Matthew revokes "K_phone" from the laptop with reason "compromised"
    Then a KeyRevocation entry is created with trigger_type "voluntary"
    And the revocation is marked effective immediately
    And the laptop's key "K_laptop" remains valid
    And any future action signed by "K_phone" is rejected by the network
```

- [ ] **Step 2: Commit**

```bash
git add genesis/a2o/features/auth/revocation-self.feature
git commit -m "a2o(recovery-m4): self-revocation feature"
```

### Task F.3: a2o feature — emergency-contact quorum

**Files:**
- Create: `genesis/a2o/features/auth/revocation-emergency-quorum.feature`

- [ ] **Step 1: Write the feature file**

```gherkin
@recovery-m4
Feature: Emergency contacts kill a captured key
  As a network defender, when a human's only key is captured,
  enough emergency contacts can kill that key by quorum
  so the attacker cannot continue acting as the human.

  Scenario: Four emergency contacts revoke a captured key by quorum
    Given Matthew has one agent key "K1"
    And Matthew has 4 active emergency contacts: Jessica, David, Sarah, and Timothy
    And an attacker has captured "K1"
    When Jessica initiates a revocation request for "K1" with reason "compromised"
    Then the revocation is created in pending state with required_votes 3
    When Jessica, David, and Sarah each submit an approved vote
    Then the revocation becomes effective
    And Matthew's future actions signed by "K1" are rejected
    And Matthew can subsequently initiate full recovery (M3) to obtain a new key
```

- [ ] **Step 2: Commit**

```bash
git add genesis/a2o/features/auth/revocation-emergency-quorum.feature
git commit -m "a2o(recovery-m4): emergency-contact quorum feature"
```

---

## Phase G — Final verification (2 tasks)

### Task G.1: Full-stack verify

- [ ] **Step 1: DNA**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check && just pack
```

- [ ] **Step 2: Storage (fresh tree, mandatory per swarm-composition memory)**

```bash
cd /projects/elohim/elohim/elohim-storage && cargo clean && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --release schema_contract
```

- [ ] **Step 3: Schema codegen freshness (DHT projection bindings)**

Source of truth: DHT. Regenerate TypeScript bindings for the DHT projection views and confirm no drift.

```bash
cd /projects/elohim && pnpm run schema:test && pnpm run schema:codegen:ts
git status --short  # expect: no generated drift
```

- [ ] **Step 4: Sweettest compile**

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest && cargo check
```

If any step fails, fix forward with a new commit — never bypass husky or amend.

### Task G.2: Push and open PR

- [ ] **Step 1: Push with husky**

```bash
cd /projects/elohim && git push -u origin feature/recovery-m4-fast-path-revocation
```

Husky pre-push runs fmt + clippy + tests. If it fails, fix and push again. **Do not use HUSKY=0.**

- [ ] **Step 2: Open PR via gh CLI**

```bash
gh pr create --title "Recovery M4: fast-path revocation" --body "$(cat <<'EOF'
## Summary
- Self-revocation + emergency-contact quorum revocation + rotation-floor gate
- Storage as reconciliation controller (Principle P1): eager cache sweep on KeyRevocationEffective, outbound imagodei.revocation_observed signal
- Dedicated recovery.revocation gossipsub topic (MessagePack wire)

## Test plan
(Source of truth: DHT; projection views verified below.)
- [ ] Sweettest CI green for all 8 scenarios
- [ ] Schema-contract tests pass for KeyRevocationView and RevocationVoteView (DHT projection views)
- [ ] a2o features parse cleanly
- [ ] Husky pre-push clean

## Spec
`genesis/docs/superpowers/specs/2026-04-24-recovery-protocol-phase-2-m4-fast-path-revocation-design.md`

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Subagent dispatch plan (summary)

The plan is organized so tasks A–E can be dispatched to **Subagent 1 (DNA+Storage)** as a single focused prompt with explicit scope guardrails. (Source of truth: DHT for every projection schema and view touched by this dispatch.)

- **Allowed paths** (all are DHT projections or codegen artifacts, not canonical storage): `elohim/holochain/dna/imagodei/**`, `elohim/elohim-storage/src/**`, `elohim/elohim-storage/migrations/**`, `elohim/elohim-storage/tests/schema_contract.rs`, `elohim/sdk/schemas/v1/views/key-revocation.schema.json`, `elohim/sdk/schemas/v1/views/revocation-vote.schema.json`, `elohim/sdk/schemas/scripts/codegen-ts.mjs` (INTERFACE_FILES addition only), `elohim/sdk/schemas/generated-ts/`, `elohim/sdk/storage-client-ts/src/generated/`.
- **Forbidden**: `git revert` / `git reset` on pre-existing commits; modifying files outside the allowed list; bypassing husky.
- **Mandatory before commit touching swarm/topic**: `cd elohim/elohim-storage && cargo clean && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release`.
- **BLOCKED protocol**: if any gate fails or scope conflicts arise, produce a BLOCKED report with repro notes. Do not silently clean up.

Tasks F are dispatched to **Subagent 2 (Tests)** after Subagent 1 completes and the orchestrator verifies scope:

- **Allowed paths**: `elohim/holochain/tests/sweettest/**`, `genesis/a2o/features/auth/revocation-*.feature`.
- **Forbidden**: modifying DNA or storage source files. If a test surfaces a bug, BLOCKED report — the orchestrator decides whether to loop Subagent 1 or accept the test as failing pending next sprint.

Orchestrator runs `git log --oneline <pre-dispatch-SHA>..HEAD` and `git diff --stat <pre-dispatch-SHA>..HEAD` after each dispatch.

Tasks G (final verify + push/PR) are orchestrator-driven.
