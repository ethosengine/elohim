# Recovery Protocol Phase 2 — M2: Validator Implementations

**Status:** Draft
**Date:** 2026-04-21
**Owner:** Matthew Dowell
**Parent spec:** `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md`
**Prior milestone:** `genesis/docs/superpowers/plans/2026-04-22-recovery-protocol-phase-2-m1-cleanup.md` (shipped `f4e854d8..94184854`)

---

## 1. Purpose

M1-cleanup left the `KeyRotation` validator stub-rejecting all five `RecoveryAuthority` variants — no rotations can land on DHT. M2 makes rotations actually work for the two variants Phase 2 implements, and wires the `IdentityFreeze`-based floor check that enforces graduated-authority escalation.

M2's success condition: an intimate-quorum or cryptographic-quorum rotation committed under correct conditions validates successfully; every malformed, under-threshold, or inappropriately-authorized rotation rejects with a clear reason; an active freeze halts same-or-lower-layer rotations while leaving escalation paths and cryptographic proofs unblocked.

## 2. Scope

**In scope:**
- Real validator logic for `RecoveryAuthority::IntimateQuorum` and `RecoveryAuthority::CryptographicQuorum` in `recovery_v2.rs::validate_key_rotation`.
- Stub-reject messages retained for `CommunityConsensus`, `GovernanceAct`, `NetworkWitness` (unchanged from M1-cleanup).
- New field `frozen_at_layer: Option<String>` on `IdentityFreeze` (`lib.rs`).
- New fields `human_id: Option<String>` and `required_witness_count: u32` on `RecoveryRequest` (`lib.rs`).
- Freeze-floor check wired into `validate_key_rotation`.
- Module constant `RECOVERY_AUTHORITY_LAYERS` in `recovery_v2.rs`.
- Unit tests via pure-logic extraction pattern.
- Downstream pipeline updates for the two struct changes: JSON schemas first, then storage projections, Rust views in `elohim-storage`, TS codegen, schema contract tests.

**Out of scope (deferred):**
- Coordinator functions for emergency-contact eligibility gating — M3.
- libp2p recovery invitation flow — M3.
- Elohim defender specialist authoring freezes with populated `frozen_at_layer` — M5.
- Cross-node sweettest integration scenarios — M3.
- `CommunityConsensus`, `GovernanceAct`, `NetworkWitness` implementations — Phase 2b+.
- `KeyStewardship` struct changes — unchanged; `shard_commitment_hash` gains a new interpretation but the field itself is not re-typed.

## 3. Architectural Decisions

Three decisions drive the design; each was weighed against alternatives during brainstorming.

### 3.1 Ed25519 verifying-key source: dual-purpose `shard_commitment_hash`

`KeyStewardship.shard_commitment_hash: String` is interpreted as the base64-encoded 32-byte Ed25519 `VerifyingKey`. "Commitment" is preserved because only the legitimate reassembled seed can produce signatures verifying against the published key; publishing the key *is* committing to it. This matches what the deleted `verify_quorum_signature` helper expected (it took `seed_public_half: &[u8]` directly) and avoids widening `KeyStewardship` or the `CryptographicQuorum` enum payload.

### 3.2 Stage-1 social-bootstrap security model

Per the memory `project_bootstrap_to_elohim_security_gradient.md`, M2 operates in **Stage 1** — structural/social validation only. The validator enforces what it can verify cheaply; correctness obligations that require social-graph traversal (is this witness's author truly an emergency contact?) live in the M3 coordinator and are backstopped by elohim defender escalation (M5). This is a deliberate, documented tradeoff; Stage 3 elohim-enforcement layers in as the protocol matures.

Consequence: `IntimateQuorum` validation does **not** traverse `AgentPubKey → Agent → HumanRelationship` to verify witness eligibility. The validator checks structural shape, signature integrity (via each `HumanityWitness`'s own authoring signature), author distinctness, and threshold count.

### 3.3 Coordinator-embedded bridging on `RecoveryRequest`

The identity-shape mismatch (`KeyRotation` uses `AgentPubKey`; `HumanityWitness`/`IdentityFreeze`/`HumanRelationship` use `String` ids) is resolved by adding two coordinator-populated fields to `RecoveryRequest`:

- `human_id: Option<String>` — resolved from the `human_agent_pubkey` at request time via the `Agent` entry's `holochain_agent_key` field. Enables the freeze-floor check and `HumanityWitness.human_id` consistency check.
- `required_witness_count: u32` — computed by the coordinator from the live count of emergency-access `HumanRelationship` entries using `ceil(M/2) + 1` where M is that count. Auditable on DHT; enforced by the validator as the threshold floor.

These fields are validator-visible constants carried forward from request to rotation — the coordinator commits them once at request time, and the validator references them during rotation validation. The single additional DHT round trip (the validator already fetches the `RecoveryRequest` via `must_get_valid_record` on `recovery_request_hash`) is free.

## 4. Data Model Changes

### 4.1 `recovery_v2.rs` — module constant

```rust
/// Layer names for the RecoveryAuthority variants, ordered by ascending authority.
/// `cryptographic` is orthogonal to the ordered layers — it bypasses the freeze floor.
pub const RECOVERY_AUTHORITY_LAYERS: &[&str] = &[
    "intimate",
    "community",
    "governance",
    "network",
    "cryptographic",
];

pub const LAYER_INTIMATE: &str = "intimate";
pub const LAYER_COMMUNITY: &str = "community";
pub const LAYER_GOVERNANCE: &str = "governance";
pub const LAYER_NETWORK: &str = "network";
pub const LAYER_CRYPTOGRAPHIC: &str = "cryptographic";

/// Map a RecoveryAuthority variant to its layer name.
pub fn authority_layer_name(authority: &RecoveryAuthority) -> &'static str {
    match authority {
        RecoveryAuthority::IntimateQuorum { .. }      => LAYER_INTIMATE,
        RecoveryAuthority::CommunityConsensus { .. }  => LAYER_COMMUNITY,
        RecoveryAuthority::GovernanceAct { .. }       => LAYER_GOVERNANCE,
        RecoveryAuthority::NetworkWitness { .. }      => LAYER_NETWORK,
        RecoveryAuthority::CryptographicQuorum { .. } => LAYER_CRYPTOGRAPHIC,
    }
}

/// Ordered layer rank for comparison. Cryptographic is orthogonal (not ordered).
/// Returns None for cryptographic.
pub fn authority_layer_rank(layer: &str) -> Option<u8> {
    match layer {
        "intimate"   => Some(1),
        "community"  => Some(2),
        "governance" => Some(3),
        "network"    => Some(4),
        _            => None,
    }
}
```

### 4.2 `lib.rs::IdentityFreeze` — add `frozen_at_layer`

```rust
pub struct IdentityFreeze {
    // ... existing fields unchanged ...
    pub expires_at: Option<String>,

    /// Which RecoveryAuthority layer triggered this freeze.
    /// See recovery_v2::RECOVERY_AUTHORITY_LAYERS. None on pre-M2 entries
    /// (treated as "intimate" — most restrictive — by the freeze-floor check).
    pub frozen_at_layer: Option<String>,
}
```

`validate_identity_freeze` gains: if `frozen_at_layer.is_some()`, its value must appear in `RECOVERY_AUTHORITY_LAYERS`.

### 4.3 `lib.rs::RecoveryRequest` — add `human_id` and `required_witness_count`

```rust
pub struct RecoveryRequest {
    pub human_agent_pubkey: AgentPubKey,
    pub new_agent_pubkey: AgentPubKey,
    pub hosting_doorway_pubkey: AgentPubKey,
    pub proposed_authority: RecoveryAuthorityKind,
    pub request_nonce: Vec<u8>,

    /// String human_id resolution (populated by coordinator via Agent entry lookup).
    /// Enables validator bridging to legacy string-keyed primitives. None is
    /// accepted at request commit (back-compat) but rejects any IntimateQuorum
    /// KeyRotation that would need to consistency-check against it.
    pub human_id: Option<String>,

    /// Witness-count threshold for the IntimateQuorum authority path. Coordinator
    /// computes `ceil(emergency_contact_count / 2) + 1` at request time, floored
    /// at 2. Validator enforces distinct-author count ≥ this value.
    pub required_witness_count: u32,

    pub created_at: Timestamp,
}
```

`validate_recovery_request` gains: `required_witness_count >= 2`.

### 4.4 `recovery_v2.rs::KeyRotation` — no struct change

`KeyRotation` struct and `RecoveryAuthority` enum are unchanged from M1-cleanup. Only `validate_key_rotation` and its helpers change.

## 5. Validator Implementation

### 5.1 `validate_key_rotation` (rewritten dispatch)

```rust
pub fn validate_key_rotation(
    rotation: &KeyRotation,
) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: new_agent_pubkey must differ from superseded_agent_pubkey.
    if rotation.new_agent_pubkey == rotation.superseded_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation new_agent_pubkey must differ from superseded_agent_pubkey".into(),
        ));
    }

    // Rule 2: RecoveryRequest resolves and fields match.
    let request_record = must_get_valid_record(rotation.recovery_request_hash.clone())?;
    let request: super::RecoveryRequest = request_record
        .entry().to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(
            "KeyRotation references non-RecoveryRequest entry: {e:?}"))))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "KeyRotation recovery_request_hash entry missing".into())))?;

    if request.human_agent_pubkey != rotation.human_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation human_agent_pubkey must match RecoveryRequest".into(),
        ));
    }
    if request.new_agent_pubkey != rotation.new_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation new_agent_pubkey must match RecoveryRequest".into(),
        ));
    }

    // Rule 3: Freeze-floor check (CryptographicQuorum is exempt).
    if !matches!(rotation.authority, RecoveryAuthority::CryptographicQuorum { .. }) {
        if let Some(reason) = check_freeze_floor(&rotation.authority, &request.human_id)? {
            return Ok(ValidateCallbackResult::Invalid(reason));
        }
    }

    // Rule 4: Variant-specific validation.
    match &rotation.authority {
        RecoveryAuthority::IntimateQuorum { witness_hashes } =>
            validate_intimate_quorum(&request, witness_hashes),
        RecoveryAuthority::CryptographicQuorum { stewardship_hash, quorum_signature } =>
            validate_cryptographic_quorum(rotation, stewardship_hash, quorum_signature),
        RecoveryAuthority::CommunityConsensus { .. } =>
            Ok(ValidateCallbackResult::Invalid(
                "KeyRotation::CommunityConsensus: Phase 2b — IdentityChallenge resolution flow not yet implemented".into())),
        RecoveryAuthority::GovernanceAct { .. } =>
            Ok(ValidateCallbackResult::Invalid(
                "KeyRotation::GovernanceAct: Phase 2b — cross-DNA qahal/mishpat resolution not yet implemented".into())),
        RecoveryAuthority::NetworkWitness { .. } =>
            Ok(ValidateCallbackResult::Invalid(
                "KeyRotation::NetworkWitness: reserved for elohim constitutional-governance design".into())),
    }
}
```

### 5.2 `validate_intimate_quorum`

```rust
fn validate_intimate_quorum(
    request: &super::RecoveryRequest,
    witness_hashes: &[ActionHash],
) -> ExternResult<ValidateCallbackResult> {
    // Absolute floor: no fewer than 2 witnesses, ever.
    if witness_hashes.len() < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "IntimateQuorum requires at least 2 witnesses".into(),
        ));
    }

    // Threshold floor from coordinator-computed request.required_witness_count.
    if (witness_hashes.len() as u32) < request.required_witness_count {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "IntimateQuorum requires {} witnesses; got {}",
            request.required_witness_count, witness_hashes.len(),
        )));
    }

    // Resolve all witnesses; collect (witness, author).
    let mut resolved: Vec<(super::HumanityWitness, AgentPubKey)> =
        Vec::with_capacity(witness_hashes.len());
    for h in witness_hashes {
        let rec = must_get_valid_record(h.clone())?;
        let author = rec.action().author().clone();
        let w: super::HumanityWitness = rec
            .entry().to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(
                "witness_hash does not resolve to HumanityWitness: {e:?}"))))?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "HumanityWitness entry missing".into())))?;
        resolved.push((w, author));
    }

    // Rule: all witnesses target the same human_id.
    let first_human_id = &resolved[0].0.human_id;
    if resolved.iter().any(|(w, _)| &w.human_id != first_human_id) {
        return Ok(ValidateCallbackResult::Invalid(
            "IntimateQuorum witnesses disagree on human_id target".into(),
        ));
    }

    // Rule: target human_id matches request.human_id (when provided).
    if let Some(req_id) = &request.human_id {
        if first_human_id != req_id {
            return Ok(ValidateCallbackResult::Invalid(
                "IntimateQuorum witness human_id does not match RecoveryRequest.human_id".into(),
            ));
        }
    } else {
        return Ok(ValidateCallbackResult::Invalid(
            "IntimateQuorum requires RecoveryRequest.human_id to be set by coordinator".into(),
        ));
    }

    // Rule: no witness is explicitly revoked.
    if resolved.iter().any(|(w, _)| w.revoked_at.is_some()) {
        return Ok(ValidateCallbackResult::Invalid(
            "IntimateQuorum includes a revoked HumanityWitness".into(),
        ));
    }

    // Rule: distinct authors (no double-voting by single agent across witnesses).
    let distinct_authors: std::collections::BTreeSet<&AgentPubKey> =
        resolved.iter().map(|(_, a)| a).collect();
    if distinct_authors.len() != resolved.len() {
        return Ok(ValidateCallbackResult::Invalid(
            "IntimateQuorum witnesses must have distinct authors".into(),
        ));
    }

    // Threshold against distinct authors (defense-in-depth; should equal len after the check above).
    if (distinct_authors.len() as u32) < request.required_witness_count {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "IntimateQuorum distinct authors {} below required {}",
            distinct_authors.len(), request.required_witness_count,
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}
```

### 5.3 `validate_cryptographic_quorum`

```rust
fn validate_cryptographic_quorum(
    rotation: &KeyRotation,
    stewardship_hash: &ActionHash,
    quorum_signature: &[u8],
) -> ExternResult<ValidateCallbackResult> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    // Signature length check.
    let sig_bytes: [u8; 64] = match quorum_signature.try_into() {
        Ok(b) => b,
        Err(_) => return Ok(ValidateCallbackResult::Invalid(
            "CryptographicQuorum quorum_signature must be 64 bytes".into(),
        )),
    };

    // Resolve KeyStewardship.
    let rec = must_get_valid_record(stewardship_hash.clone())?;
    let stewardship: super::KeyStewardship = rec
        .entry().to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(
            "stewardship_hash does not resolve to KeyStewardship: {e:?}"))))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "KeyStewardship entry missing".into())))?;

    // Non-superseded check.
    if stewardship.rotated_at.is_some() {
        return Ok(ValidateCallbackResult::Invalid(
            "CryptographicQuorum references a superseded KeyStewardship".into(),
        ));
    }

    // Decode verifying key from shard_commitment_hash (base64 → 32 bytes).
    let vk_bytes = match base64_decode_32(&stewardship.shard_commitment_hash) {
        Ok(b) => b,
        Err(e) => return Ok(ValidateCallbackResult::Invalid(format!(
            "CryptographicQuorum KeyStewardship.shard_commitment_hash not a base64 32-byte Ed25519 key: {e}"))),
    };
    let vk = match VerifyingKey::from_bytes(&vk_bytes) {
        Ok(k) => k,
        Err(e) => return Ok(ValidateCallbackResult::Invalid(format!(
            "CryptographicQuorum shard_commitment_hash is not a valid Ed25519 verifying key: {e}"))),
    };

    // Construct signed message: new_agent_pubkey.get_raw_39() || recovery_request_hash.get_raw_39()
    let mut message: Vec<u8> = Vec::with_capacity(39 + 39);
    message.extend_from_slice(rotation.new_agent_pubkey.get_raw_39());
    message.extend_from_slice(rotation.recovery_request_hash.get_raw_39());

    let sig = Signature::from_bytes(&sig_bytes);
    match vk.verify(&message, &sig) {
        Ok(()) => Ok(ValidateCallbackResult::Valid),
        Err(_) => Ok(ValidateCallbackResult::Invalid(
            "CryptographicQuorum signature verification failed".into(),
        )),
    }
}

/// Helper: decode a base64 string to exactly 32 bytes.
fn base64_decode_32(s: &str) -> Result<[u8; 32], String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let bytes = STANDARD.decode(s).map_err(|e| format!("base64 decode: {e}"))?;
    bytes.try_into().map_err(|v: Vec<u8>| format!("expected 32 bytes, got {}", v.len()))
}
```

If the `base64` crate is not already a dependency of the integrity zome, add it.

### 5.4 `check_freeze_floor`

**HDI architectural constraint (surfaced during M2 implementation):** the integrity-zome validation callback context does NOT expose `get_links` or `GetLinksInputBuilder`. Those are HDK-only (coordinator zome). Validators have access to deterministic primitives only: `must_get_valid_record`, `must_get_entry`, `must_get_action`, `must_get_agent_activity`, and `hash_entry`. Link state is non-deterministic; a validator that depended on link state could validate differently on different peers.

**Consequence:** the freeze-floor check cannot traverse the `ActiveFreezes` anchor inside `validate_key_rotation`. The enforcement model shifts:

1. **Pure-logic helper (`check_freeze_floor_rules`)** — fully implemented in M2, unit-tested, owns the freeze/rotation layer-comparison rules. Shared between validator and coordinator. Takes pre-resolved freezes as input.
2. **HDI wrapper (`check_freeze_floor`)** — present as a dispatch hook in `validate_key_rotation` for future tightening, but returns `Ok(None)` in Phase 2 because it has no way to enumerate freezes deterministically. A `must_get_agent_activity` lookup could surface freezes authored by a specific elohim, but the elohim-of-human binding is itself an open spec question (§8.2) deferred to later work.
3. **Coordinator pre-commit gate (M5)** — the real enforcement point. Before calling `create_entry(KeyRotation)`, the coordinator queries `get_links` on the `ActiveFreezes` anchor, feeds the results to `check_freeze_floor_rules`, and bails if a blocker exists. This matches the Stage-1 social-bootstrap model (spec §3.2) — coordinator owns correctness; validator enforces what is cheaply verifiable; elohim defender escalation (M5+) is the compensating control for adversarial coordinators.

The dispatch wiring stays in `validate_key_rotation` so M5 can replace the stub body without touching the dispatch or the test surface. Pure-logic rules are the portable piece.

**Legacy design (superseded by the above, kept for traceability):**

```rust
/// Returns Ok(None) if no blocking freeze exists, Ok(Some(reason)) if one does.
/// Caller is responsible for skipping this helper for CryptographicQuorum —
/// the exemption is handled at dispatch in validate_key_rotation (§5.1 Rule 3).
fn check_freeze_floor(
    authority: &RecoveryAuthority,
    request_human_id: &Option<String>,
) -> ExternResult<Option<String>> {
    let human_id = match request_human_id {
        Some(id) => id,
        None => return Ok(Some(
            "Freeze-floor check requires RecoveryRequest.human_id to be populated by coordinator".into(),
        )),
    };

    let rotation_layer = authority_layer_name(authority);
    let rotation_rank = authority_layer_rank(rotation_layer)
        .expect("check_freeze_floor invoked for non-ordered authority; caller must exempt cryptographic");

    // Traverse the ActiveFreezes anchor for candidate freezes targeting this human.
    // Anchor construction follows the existing zome pattern used by other freeze
    // queries (see §10 Open Question 2; resolved during plan-writing by reading
    // the current StringAnchor("active", "identity_freezes") convention).
    let anchor_hash = build_active_freezes_anchor()?;
    let links = get_links(GetLinksInputBuilder::try_new(
        anchor_hash,
        LinkTypes::ActiveFreezes,
    )?.build())?;

    for link in links {
        let target = match link.target.into_action_hash() {
            Some(h) => h,
            None => continue,
        };
        let rec = match must_get_valid_record(target) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let freeze: super::IdentityFreeze = match rec.entry().to_app_option() {
            Ok(Some(f)) => f,
            _ => continue,
        };

        if !freeze.is_active { continue; }
        if &freeze.human_id != human_id { continue; }

        // Determine freeze layer; None → default "intimate" (most restrictive).
        let frozen_layer = freeze.frozen_at_layer.as_deref().unwrap_or(LAYER_INTIMATE);
        let frozen_rank = match authority_layer_rank(frozen_layer) {
            Some(r) => r,
            None => continue, // "cryptographic" freeze doesn't participate in ordering.
        };

        // Rule: rotation's rank must exceed freeze's rank (strict >) to proceed.
        if rotation_rank <= frozen_rank {
            return Ok(Some(format!(
                "KeyRotation blocked by active IdentityFreeze at layer '{frozen_layer}'; \
                 rotation layer '{rotation_layer}' must exceed frozen layer to proceed",
            )));
        }
    }

    Ok(None)
}
```

Anchor-link lookup follows the pattern used elsewhere in the zome (e.g., `ActiveFreezes` is an `Anchor("active") → IdentityFreeze` link, already declared as a `LinkType`). Implementation follows the existing convention for whichever anchor-construction helper the zome uses.

## 6. Downstream Pipeline

Per the `feedback_schema_first_ioc.md` memory: JSON schemas **first**, then Rust, then TypeScript.

### 6.1 Schema updates

- `elohim/sdk/schemas/v1/views/identity-freeze.schema.json` — add `frozenAtLayer: string | null` with enum `["intimate", "community", "governance", "network", "cryptographic", null]`.
- `elohim/sdk/schemas/v1/views/recovery-request.schema.json` — add `humanId: string | null` + `requiredWitnessCount: integer` (minimum 2).

### 6.2 Storage projection migrations

New migration in `elohim/elohim-storage/migrations/` (next sequence number):
- `ALTER TABLE identity_freezes ADD COLUMN frozen_at_layer TEXT;`
- `ALTER TABLE recovery_requests ADD COLUMN human_id TEXT;`
- `ALTER TABLE recovery_requests ADD COLUMN required_witness_count INTEGER NOT NULL DEFAULT 2;`

### 6.3 Rust view updates

`elohim/elohim-storage/src/views.rs`:
- `IdentityFreezeView` gains `frozen_at_layer: Option<String>`.
- `RecoveryRequestView` gains `human_id: Option<String>` + `required_witness_count: u32`.
- Both with `#[serde(rename_all = "camelCase")]`.

### 6.4 Projection sync

`elohim/elohim-storage/src/imagodei/` projections update to include the new columns in insert/upsert statements for `identity_freezes` and `recovery_requests`.

### 6.5 TypeScript regeneration

Run `cargo test export_bindings` in `elohim-storage` to regenerate the TypeScript types at `elohim/sdk/storage-client-ts/src/generated/`.

### 6.6 Schema contract tests

`elohim/elohim-storage/tests/schema_contract.rs` — update fixture instances of `IdentityFreezeView` and `RecoveryRequestView` to include the new fields.

## 7. Test Plan

Unit tests added inline in `recovery_v2.rs` under `#[cfg(test)] mod tests`. Pattern: extract variant-specific logic into helpers that receive pre-resolved entries as arguments, test the pure-logic helpers exhaustively. The wrappers doing `must_get_valid_record` are thin and tested via sweettest in M3.

### 7.1 Required pure-logic helpers (testable without runtime)

```rust
fn check_intimate_quorum_rules(
    request: &RecoveryRequest,
    resolved_witnesses: &[(HumanityWitness, AgentPubKey)],
) -> ValidateCallbackResult;

fn check_cryptographic_quorum_rules(
    stewardship: &KeyStewardship,
    new_agent_pubkey_raw: &[u8; 39],
    recovery_request_hash_raw: &[u8; 39],
    quorum_signature: &[u8],
) -> ValidateCallbackResult;

fn check_freeze_floor_rules(
    authority: &RecoveryAuthority,
    human_id: &str,
    active_freezes_for_human: &[&IdentityFreeze],
) -> Option<String>;
```

These take pre-resolved data; the `validate_*` wrappers just do the HDI calls and delegate.

### 7.2 Coverage targets

**IntimateQuorum:**
- happy: N ≥ required distinct authors, all witnesses valid, consistent human_id
- error: fewer witnesses than `required_witness_count`
- error: fewer than 2 witnesses (absolute floor)
- error: duplicate author across witnesses
- error: witness with `revoked_at = Some`
- error: witnesses disagree on human_id
- error: witness human_id != request human_id
- error: request.human_id is None

**CryptographicQuorum:**
- happy: valid 64-byte sig over 78-byte message (39+39), matching verifying key
- error: signature length != 64
- error: invalid base64 in shard_commitment_hash
- error: decoded key not 32 bytes
- error: decoded bytes not a valid Ed25519 point
- error: valid-shape sig but wrong message
- error: valid sig but wrong key
- error: stewardship `rotated_at = Some`

**Freeze-floor:**
- no active freezes → pass
- active freeze at layer L, rotation at layer ≤ L → reject
- active freeze at layer L, rotation at layer > L → pass (escalation)
- `frozen_at_layer = None` → treated as `intimate` → reject intimate rotation
- `frozen_at_layer = None` → treated as `intimate` → community-rotation passes
- active freeze on this human + CryptographicQuorum rotation → helper returns None (caller exempts)
- lifted freeze (`is_active = false`) on this human → pass
- freeze on different human_id → pass

**Layer helpers:**
- `authority_layer_name` returns expected string per variant
- `authority_layer_rank` returns Some(1..=4) for ordered, None for "cryptographic"

**Struct validators:**
- `validate_identity_freeze` rejects `frozen_at_layer = Some("bogus")`
- `validate_identity_freeze` accepts `frozen_at_layer = Some("intimate")` and None
- `validate_recovery_request` rejects `required_witness_count < 2`

### 7.3 Acceptance

- `just dna-imagodei` builds clean (wasm32 target).
- `cargo test --package imagodei_integrity` passes all new unit tests.
- `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release` compiles.
- `cd elohim/elohim-storage && cargo test` passes schema contract tests.
- `pnpm run schema:validate && pnpm run schema:check-dna && pnpm run schema:codegen:ts` all pass (pre-push hook contract).

## 8. Stability Boundary

M2 leaves the validator permanently accepting two variants and structurally prepared to accept the other three when their implementations land. No data model shape changes beyond the three new fields. No coordinator interface changes (coordinator gains obligations — populating `human_id`, `required_witness_count`, eventually `frozen_at_layer` — but its API signatures don't shift; it just starts populating fields that were defaulted).

Post-M2 state: rotations can land for humans with emergency-access relationships (IntimateQuorum path) or provisioned cryptographic stewardship (CryptographicQuorum path). Three variants remain stub-rejected with clear deferral messages.

## 9. Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Coordinator forgets to populate `human_id`, rotations fail validation mysteriously | Validator error message names the missing field explicitly; coordinator integration tests in M3 catch this. |
| `base64` crate not present; adding it bloats WASM | Verify presence first; if added, it's ~10kb compressed — acceptable. |
| `get_raw_39()` format changes across HDK versions | Already used in prior helper; HDK API stability is inherited; breakage would surface as sweettest failures in M3. |
| M1 data migration: pre-M2 `IdentityFreeze` entries on dev networks lack `frozen_at_layer` | Intentionally tolerated: `None` treated as `intimate` (most restrictive); no migration needed; no production networks in play. |
| Schema-first discipline slippage (generating code before updating schemas) | Step-order enforced in the implementation plan; pre-push hook validates codegen freshness. |
| Coordinator-trust gap (Stage-1 model): malicious coordinator could stage non-eligible witnesses | Documented tradeoff; elohim defender escalation (M5) is the compensating control; each HumanityWitness is individually signed so fabrication is expensive. |

## 10. Open Questions (none that block M2)

- **Which `base64` crate?** Plan verifies; likely `base64 = "0.22"` matching protocol convention.
- **Anchor construction pattern for ActiveFreezes traversal:** plan reads existing `get_links`-on-`ActiveFreezes` usage in the zome and matches convention.

Neither blocks design approval; both are resolved during plan-writing.

## 11. Handoff to `writing-plans`

The writing-plans skill produces the implementation plan at `genesis/docs/superpowers/plans/2026-04-21-recovery-protocol-phase-2-m2-validator-impls.md`. Plan granularity mirrors M1-cleanup: task per file-or-concept, schema-first ordering, verification steps at each boundary, small commits per task. Two rust-architect subagent dispatches expected (integrity zome + storage/schema pipeline); tests integrated into each.
