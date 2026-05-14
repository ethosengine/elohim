# Recovery Protocol Phase 2 — M1-Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md`

**Goal:** Reverse the three deletable M1 entry types (`RecoverySeedCommitment`, `HeldRecoveryShare`, `MyRecoveryAuthorization`), evolve `KeyRotation` to carry a `RecoveryAuthority` enum, and modernize the legacy `RecoveryRequest` struct — bringing the codebase in line with the revised Phase 2 spec that uses existing protocol primitives (`KeyStewardship`, `HumanityWitness`, `IdentityChallenge`) instead of parallel reinventions.

**Architecture:** The M1 work committed last night (`5e997cea..31564f04`) added four new entry types, three of which duplicate existing primitives and must be removed. `KeyRotation` survives as the one genuine missing primitive but evolves from Ed25519-signature-over-seed to a graduated-authority enum with five variants. `RecoveryRequest` (currently a stubby Jan 2026 scaffold) gets its fields replaced wholesale. Validator logic for variant implementations is stub-rejected in M1-cleanup and lands in M2.

**Tech Stack:** Holochain HDK/HDI 0.7/0.6 (Rust, wasm32), Diesel migrations (SQLite), `ts-rs` codegen, JSON Schema wire contracts, pnpm workspaces.

**Scope boundary:** M1-cleanup only. Validator variant implementations (IntimateQuorum, CryptographicQuorum happy paths), floor-rises freeze check, libp2p coordination, defender specialist, and UI all belong to later milestones. M1-cleanup's single purpose is to leave the codebase in a state that compiles cleanly, passes existing tests, and has the data model shape the revised spec declares.

## Source-of-Truth Declarations (plan-wide)

Every data entity introduced or modified in this plan carries an explicit source-of-truth declaration per the `p2p-design-gate` skill:

| Entity | Source of truth | Projection / Local? |
|---|---|---|
| `KeyRotation` (DHT entry) | Holochain DHT (imagodei DNA) | Projected to `key_rotations` SQLite table with `dht_anchor_hash` |
| `RecoveryRequest` (DHT entry, modernized) | Holochain DHT (imagodei DNA) | Projected to `recovery_requests` SQLite table with `dht_anchor_hash` |
| `key_rotations` (SQLite) | DHT (projection) | read-optimized cache of the DHT KeyRotation entry |
| `recovery_requests` (SQLite) | DHT (projection) | read-optimized cache of the DHT RecoveryRequest entry |
| JSON wire schemas (`recovery-request`, `key-rotation`) | Authoritative wire-contract definition | Validated against Rust views and generated TypeScript |
| Deleted entities (`RecoverySeedCommitment`, `HeldRecoveryShare`, `MyRecoveryAuthorization`, `recovery_seed_commitments` table, related views/schemas) | *(formerly DHT; now deleted — superseded by protocol's existing `KeyStewardship` primitive per revised spec §4)* | *(no projection — removed)* |

All SQL migration tables (see Task 8), Rust view structs (Task 9), and JSON schemas (Task 10) in this plan carry corresponding inline source-of-truth comments/descriptions per the detailed task instructions.

---

## File Structure

### Modified
- `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs` — delete 3 entry types + validators, add `RecoveryAuthority` + `NetworkWitnessPurpose` + `RecoveryAuthorityKind` enums, evolve `KeyRotation` struct + validator, delete `RecoveryQuorumRequest` (its role merges into modernized `RecoveryRequest` in `lib.rs`)
- `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs` — remove deleted entry types from `EntryTypes` enum, remove validation dispatch arms, remove deleted link types from `LinkTypes` enum, REPLACE legacy `RecoveryRequest` struct with modernized version, adjust its `validate_recovery_request` function
- `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` — delete `commit_recovery_seed` + `create_recovery_quorum_request` coordinator functions, delete their input/output types, modernize `create_recovery_request` (legacy) to match new struct, update `commit_key_rotation` to use `RecoveryAuthority`, remove `SeedCommitmentCreated`/`SeedCommitmentSuperseded`/`RecoveryQuorumRequestCreated` signal variants
- `elohim/elohim-storage/src/schema.rs` — remove `recovery_seed_commitments` table, modify `key_rotations` and `recovery_quorum_requests` (the latter renamed to `recovery_requests`)
- `elohim/elohim-storage/src/views.rs` — delete `RecoverySeedCommitmentView`, update `KeyRotationView`, rename `RecoveryQuorumRequestView` → `RecoveryRequestView`
- `elohim/sdk/schemas/scripts/codegen-ts.mjs` — remove `recovery-seed-commitment` entry, rename `recovery-quorum-request` → `recovery-request`
- `elohim/elohim-storage/tests/schema_contract_recovery_v2.rs` — update three tests for new shapes

### Created
- `elohim/elohim-storage/migrations/2026-04-22-000000_recovery_phase_2_cleanup/up.sql`
- `elohim/elohim-storage/migrations/2026-04-22-000000_recovery_phase_2_cleanup/down.sql`
- `elohim/sdk/schemas/v1/views/recovery-request.schema.json` (renamed from recovery-quorum-request.schema.json with updated shape)

### Deleted
- `elohim/sdk/schemas/v1/views/recovery-seed-commitment.schema.json`
- `elohim/sdk/schemas/v1/views/recovery-request.schema.json` (renamed, so old file removed)
- Generated TypeScript files for deleted views (across three consumer directories): `RecoverySeedCommitmentView.ts`, `RecoveryQuorumRequestView.ts` (regenerated as `RecoveryRequestView.ts`)

---

## Task 1: Delete `RecoverySeedCommitment` from integrity zome

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs`

- [ ] **Step 1: Remove struct and validator from recovery_v2.rs**

Open `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs`. Find the `RecoverySeedCommitment` section (starts around line 37 per the audit). Delete:

- The `// ===== RecoverySeedCommitment =====` banner comment
- The `#[hdk_entry_helper]` struct `pub struct RecoverySeedCommitment { ... }`
- The `pub fn validate_recovery_seed_commitment(...) -> ExternResult<ValidateCallbackResult>` function

Also delete the module docstring mention (near the top of the file):
```rust
//! - RecoverySeedCommitment: on-DHT public half + thresholds, no holder list
```

- [ ] **Step 2: Remove from EntryTypes enum in lib.rs**

Open `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs`. Find the `EntryTypes` enum. Delete the line:
```rust
    RecoverySeedCommitment(RecoverySeedCommitment),
```

- [ ] **Step 3: Remove from validation dispatch in lib.rs**

In the same file, find the `validate` function's `OpEntry::CreateEntry` match. Delete:
```rust
                EntryTypes::RecoverySeedCommitment(commitment) =>
                    validate_recovery_seed_commitment(&commitment, &action),
```

Also find the `OpEntry::UpdateEntry` match and delete:
```rust
                EntryTypes::RecoverySeedCommitment(_) =>
                    Ok(ValidateCallbackResult::Invalid(
                        "RecoverySeedCommitment is immutable; use SeedCommitmentSupersededBy link to supersede"
                            .to_string(),
                    )),
```

- [ ] **Step 4: Type-check**

```bash
cd /projects/elohim && just dna-imagodei
```

Expected: compiles clean. If unused-import warnings appear, note them but don't fix here — cumulative cleanup at end.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/{recovery_v2.rs,lib.rs}
git commit -m "feat(imagodei): remove RecoverySeedCommitment — superseded by KeyStewardship"
```

---

## Task 2: Delete `HeldRecoveryShare` and `MyRecoveryAuthorization` from integrity zome

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs`

- [ ] **Step 1: Remove HeldRecoveryShare from recovery_v2.rs**

Delete the `HeldRecoveryShare` struct, its `validate_held_recovery_share` function, its banner comment `// ===== HeldRecoveryShare =====`, and the docstring line `//! - HeldRecoveryShare: private source-chain entry on holder devices`.

- [ ] **Step 2: Remove MyRecoveryAuthorization from recovery_v2.rs**

Delete the `MyRecoveryAuthorization` struct, its `validate_my_recovery_authorization` function, its banner comment, and the docstring line `//! - MyRecoveryAuthorization: optional private audit log on holder devices`.

- [ ] **Step 3: Remove from EntryTypes enum**

In `lib.rs`, delete:
```rust
    HeldRecoveryShare(HeldRecoveryShare),
    MyRecoveryAuthorization(MyRecoveryAuthorization),
```

- [ ] **Step 4: Remove from validation dispatch**

In `lib.rs`, delete both entries' CreateEntry and UpdateEntry match arms (similar pattern to Task 1).

- [ ] **Step 5: Type-check**

```bash
cd /projects/elohim && just dna-imagodei
```

Expected: compiles clean.

- [ ] **Step 6: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/{recovery_v2.rs,lib.rs}
git commit -m "feat(imagodei): remove HeldRecoveryShare + MyRecoveryAuthorization

Shares are holder-local private source-chain entries with no DHT presence
needed. Holder private audit is covered by self-authored HumanityWitness."
```

---

## Task 3: Remove seed-commitment link types from integrity zome

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs`

- [ ] **Step 1: Remove link variants**

Find the `LinkTypes` enum. Delete:
```rust
    HumanToCurrentSeedCommitment,
    SeedCommitmentSupersededBy,
    SeedCommitmentToRequest,
```

These were added in M1 Task 8. They reference deleted entry types and are dead.

- [ ] **Step 2: Type-check**

```bash
cd /projects/elohim && just dna-imagodei
```

Expected: compiles clean. The coordinator zome's `commit_recovery_seed` function still references these link types, so integrity will build but coordinator won't — that's fine, Task 5 deletes those references.

- [ ] **Step 3: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
git commit -m "feat(imagodei): remove seed-commitment link types"
```

---

## Task 4: Add `RecoveryAuthority` enum + `NetworkWitnessPurpose` + `RecoveryAuthorityKind`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs`

- [ ] **Step 1: Replace RecoveryMode with new enum ecosystem**

The existing `RecoveryMode` enum in `recovery_v2.rs` (added in M1 Task 2) has variants `Normal` and `Stewarded { grant_hash }`. Delete it and replace with the new authority enums per spec §5.1 and §5.3.

Find this block near the top of `recovery_v2.rs`:
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecoveryMode {
    Normal,
    Stewarded { grant_hash: ActionHash },
}
```

Replace it with:

```rust
/// Purpose of a NetworkWitness authority — either restore access or retire the account.
/// Dissolution variant is reserved for cradle-to-grave care (deferred to constitutional-governance design).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum NetworkWitnessPurpose {
    /// Rescue: restore access to the human's active identity.
    Rescue,
    /// Dissolution: retire the account (deceased, irrecoverable).
    /// new_agent_pubkey is a memorial-marker null agent.
    /// Phase 2: stub-rejected in validator; shape reserved for constitutional-governance design.
    Dissolution,
}

/// Evidence supporting a KeyRotation. Five variants; any one sufficient for authorization.
/// Phase 2 implements IntimateQuorum + CryptographicQuorum; other variants stub-reject.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecoveryAuthority {
    /// Layer 1: Intimate-circle quorum via HumanityWitness entries from emergency contacts.
    /// Phase 2: IMPLEMENTED (structural shape; variant-specific validation lands in M2).
    IntimateQuorum {
        witness_hashes: Vec<ActionHash>,
    },
    /// Layer 2: Extended community via IdentityChallenge resolution.
    /// Phase 2: STUB-REJECTED (Phase 2b).
    CommunityConsensus {
        challenge_hash: ActionHash,
    },
    /// Layer 3: Governance act via qahal/stewardship resolution.
    /// Phase 2: STUB-REJECTED (cross-DNA qahal/mishpat work pending).
    GovernanceAct {
        grant_hash: ActionHash,
        resolution_hash: ActionHash,
    },
    /// Layer 4: Global elohim witness — prevents absolute lockout.
    /// Phase 2: STUB-REJECTED (pending elohim constitutional-governance design).
    NetworkWitness {
        witness_entries: Vec<ActionHash>,
        consensus_threshold_met_at: Timestamp,
        purpose: NetworkWitnessPurpose,
    },
    /// Layer 5 (orthogonal): Cryptographic M-of-N threshold via KeyStewardship.
    /// Provisioned only when elohim judges the human vulnerable enough.
    /// Phase 2: IMPLEMENTED (structural shape; variant-specific validation lands in M2).
    CryptographicQuorum {
        stewardship_hash: ActionHash,
        quorum_signature: Vec<u8>,
    },
}

/// Claimant's declared intent for which authority path a RecoveryRequest will pursue.
/// The actual KeyRotation authority can differ (escalation is allowed).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecoveryAuthorityKind {
    IntimateQuorum,
    CommunityConsensus,
    GovernanceAct { grant_hash: ActionHash },
    NetworkWitness { purpose: NetworkWitnessPurpose },
    CryptographicQuorum { stewardship_hash: ActionHash },
}
```

The existing `ConfidenceTier` enum stays as-is (used by elohim assessments, orthogonal).

- [ ] **Step 2: Type-check**

```bash
cd /projects/elohim && just dna-imagodei
```

Expected: compiles clean (new enums are used nowhere yet; KeyRotation struct update in Task 5 wires them in).

- [ ] **Step 3: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs
git commit -m "feat(imagodei): add RecoveryAuthority enum ecosystem for graduated recovery"
```

---

## Task 5: Evolve `KeyRotation` struct + validator to use `RecoveryAuthority`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs`

- [ ] **Step 1: Replace KeyRotation struct**

Find the `KeyRotation` struct in `recovery_v2.rs`. Replace the entire struct definition with:

```rust
/// The authoritative claim that a human's agent key has rotated.
/// Evidence is carried in the `authority` field as one of five graduated variants.
/// Phase 2 validator accepts the structural shape; variant-specific validation
/// (Ed25519 sig verification, HumanityWitness quorum counting) lands in M2.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct KeyRotation {
    pub human_agent_pubkey: AgentPubKey,
    pub new_agent_pubkey: AgentPubKey,
    pub superseded_agent_pubkey: AgentPubKey,
    pub recovery_request_hash: ActionHash,
    pub authority: RecoveryAuthority,
    pub rotated_at: Timestamp,
}
```

The old struct had `seed_commitment_hash` + `quorum_signature` fields — both are removed. The `authority` field carries that evidence (as `CryptographicQuorum { stewardship_hash, quorum_signature }` when it applies).

- [ ] **Step 2: Replace validator with structural-only version**

Find `pub fn validate_key_rotation(...)`. Replace its entire body with:

```rust
pub fn validate_key_rotation(
    rotation: &KeyRotation,
) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: new_agent_pubkey must differ from superseded_agent_pubkey
    if rotation.new_agent_pubkey == rotation.superseded_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation new_agent_pubkey must differ from superseded_agent_pubkey".to_string(),
        ));
    }

    // Rule 2: Resolve the referenced RecoveryRequest and verify matching fields.
    // Rule 2 requires RecoveryRequest to be the modernized struct (Task 6).
    // Until Task 6 lands, we verify structural integrity only.
    let request_record = must_get_valid_record(rotation.recovery_request_hash.clone())?;
    let request_entry: RecoveryRequest = request_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(
            "KeyRotation references non-RecoveryRequest entry: {e:?}"
        ))))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "KeyRotation recovery_request_hash entry missing".to_string()
        )))?;

    if request_entry.human_agent_pubkey != rotation.human_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation human_agent_pubkey must match RecoveryRequest".to_string(),
        ));
    }
    if request_entry.new_agent_pubkey != rotation.new_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation new_agent_pubkey must match RecoveryRequest".to_string(),
        ));
    }

    // Rule 3: Phase 2 stub-rejects all variant-specific validation.
    // M2 milestone implements IntimateQuorum + CryptographicQuorum happy paths
    // and wires the floor-check against active IdentityFreeze entries.
    match &rotation.authority {
        RecoveryAuthority::IntimateQuorum { .. } => Ok(ValidateCallbackResult::Invalid(
            "KeyRotation::IntimateQuorum: variant validation pending in M2".to_string(),
        )),
        RecoveryAuthority::CommunityConsensus { .. } => Ok(ValidateCallbackResult::Invalid(
            "KeyRotation::CommunityConsensus: Phase 2b — IdentityChallenge resolution flow not yet implemented".to_string(),
        )),
        RecoveryAuthority::GovernanceAct { .. } => Ok(ValidateCallbackResult::Invalid(
            "KeyRotation::GovernanceAct: Phase 2b — cross-DNA qahal/mishpat resolution not yet implemented".to_string(),
        )),
        RecoveryAuthority::NetworkWitness { .. } => Ok(ValidateCallbackResult::Invalid(
            "KeyRotation::NetworkWitness: reserved for elohim constitutional-governance design".to_string(),
        )),
        RecoveryAuthority::CryptographicQuorum { .. } => Ok(ValidateCallbackResult::Invalid(
            "KeyRotation::CryptographicQuorum: variant validation pending in M2".to_string(),
        )),
    }
}
```

Remove the `verify_quorum_signature` helper function (the whole fn) — its logic moves into M2's IntimateQuorum/CryptographicQuorum implementation.

Also remove the `use ed25519_dalek::{...}` import inside `verify_quorum_signature` if the helper is the only thing using it. If ed25519-dalek is imported at module scope in `recovery_v2.rs` for no other reason, keep the import — M2 will need it.

- [ ] **Step 3: Type-check**

```bash
cd /projects/elohim && just dna-imagodei
```

Expected: compiles clean. If unused-import warnings emerge from ed25519-dalek, suppress with `#[allow(unused_imports)]` at module scope (M2 will use them).

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs
git commit -m "feat(imagodei): evolve KeyRotation to RecoveryAuthority enum (M2 implements variants)"
```

---

## Task 6: Modernize legacy `RecoveryRequest` struct in integrity lib.rs

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs`

- [ ] **Step 1: Delete RecoveryQuorumRequest from recovery_v2.rs**

In `recovery_v2.rs`, find and delete:
- The `// ===== RecoveryQuorumRequest =====` banner
- The `#[hdk_entry_helper] pub struct RecoveryQuorumRequest { ... }` struct
- The `pub fn validate_recovery_quorum_request(...)` function
- The docstring line `//! - RecoveryQuorumRequest: claimant's request, authored by hosting doorway`

This entry type's role is merging into the modernized `RecoveryRequest` in lib.rs (next steps).

- [ ] **Step 2: Replace legacy RecoveryRequest struct in lib.rs**

Open `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs`. Find the `RecoveryRequest` struct (around line 528 per the earlier audit — it has `id: String`, `human_id: String`, `doorway_id: String`, `recovery_method: String`, `elohim_score`, etc.).

Replace the entire struct definition AND its preceding doc comment block with:

```rust
/// RecoveryRequest — claimant's request to rotate their agent key.
///
/// Authored by the hosting doorway on behalf of the claimant's new device (which
/// has no working cell yet). Authority for the eventual rotation is declared as
/// `proposed_authority` (intent) but can be escalated to a higher layer by the
/// time the KeyRotation is committed. Evidence for the rotation lives in the
/// KeyRotation entry's `authority: RecoveryAuthority` field.
///
/// Supersedes the Jan 2026 stubby struct (string-IDs, elohim_score fields).
/// The revised struct uses AgentPubKey for identity and delegates authority
/// evidence to the graduated-authority primitives introduced in recovery_v2.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RecoveryRequest {
    /// The human whose identity is being recovered.
    pub human_agent_pubkey: AgentPubKey,
    /// Proposed new agent pubkey (generated by the claimant's new device session).
    pub new_agent_pubkey: AgentPubKey,
    /// The federated doorway hosting this recovery session.
    pub hosting_doorway_pubkey: AgentPubKey,
    /// Claimant's declared intent for which authority path the eventual rotation will use.
    /// The KeyRotation can use a different (escalated) authority; this is intent only.
    pub proposed_authority: RecoveryAuthorityKind,
    /// Random 16-byte nonce disambiguating concurrent or retried attempts.
    pub request_nonce: Vec<u8>,
    pub created_at: Timestamp,
}
```

Note: `RecoveryAuthorityKind` is imported from `recovery_v2` via the existing `pub use recovery_v2::*;` re-export. If the re-export isn't pulling the new enums, add an explicit `use crate::recovery_v2::RecoveryAuthorityKind;` at the top of lib.rs.

- [ ] **Step 3: Replace validate_recovery_request**

Find `fn validate_recovery_request(request: &RecoveryRequest) -> ExternResult<ValidateCallbackResult>` (around line 1438 per earlier audit). Replace its entire body with:

```rust
fn validate_recovery_request(request: &RecoveryRequest) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: request_nonce must be 16 bytes
    if request.request_nonce.len() != 16 {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryRequest request_nonce must be exactly 16 bytes".to_string(),
        ));
    }
    // Rule 2: new_agent_pubkey must differ from human_agent_pubkey
    if request.human_agent_pubkey == request.new_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryRequest new_agent_pubkey must differ from human_agent_pubkey".to_string(),
        ));
    }
    // Rule 3: if proposed_authority is a stubbed variant, that's still OK at request time —
    // the request is claimant's intent; the actual authorization may escalate. The validator
    // only rejects stubbed variants at KeyRotation time (Task 5).
    //
    // Rule 4 (deferred to M3): author of the request should be the hosting_doorway's agent
    // pubkey. We cannot enforce this here without a cross-field check against the signing
    // action; M3 coordinator function ensures this and the validator trusts it.
    Ok(ValidateCallbackResult::Valid)
}
```

- [ ] **Step 4: Update EntryTypes and validation dispatch for RecoveryQuorumRequest deletion**

Remove `RecoveryQuorumRequest(RecoveryQuorumRequest),` from the `EntryTypes` enum in `lib.rs`.

In the `validate` function's `CreateEntry` match, remove:
```rust
                EntryTypes::RecoveryQuorumRequest(request) =>
                    validate_recovery_quorum_request(&request),
```

And its `UpdateEntry` arm.

- [ ] **Step 5: Update link types for naming consistency**

In the `LinkTypes` enum in lib.rs, rename `HumanToRecoveryQuorumRequest` to be reused for the modernized `RecoveryRequest`. Since the existing `HumanToRecoveryRequest` link type already exists (from the Jan 2026 legacy coordinator code), KEEP `HumanToRecoveryRequest` and DELETE `HumanToRecoveryQuorumRequest`:

```rust
    // Delete this line:
    HumanToRecoveryQuorumRequest,
```

The existing `HumanToRecoveryRequest` now anchors the modernized struct. The legacy `IdToRecoveryRequest`, `RecoveryRequestByStatus`, `PendingRecoveryVote`, `RecoveryVoteToRequest`, `HumanToRecoveryHint`, `RecoveryHintByType` links in the enum are legacy — leave them in place for now (the legacy coordinator uses them; a later cleanup can decide if the legacy flow fully retires).

- [ ] **Step 6: Type-check**

```bash
cd /projects/elohim && just dna-imagodei
```

Expected: ERROR — the coordinator zome still references the old `RecoveryRequest` shape and the deleted `RecoveryQuorumRequest` type. This is expected; Task 7 fixes the coordinator. Note the errors for Task 7.

- [ ] **Step 7: Commit (no-build-allowed exception)**

Because the coordinator zome is now broken (deliberate; fixed in Task 7), this commit doesn't pass `just dna-imagodei`. That's OK — commit it as a checkpoint, Task 7 restores build.

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/{recovery_v2.rs,lib.rs}
git commit -m "feat(imagodei): modernize RecoveryRequest struct, delete RecoveryQuorumRequest

WIP: coordinator zome temporarily broken. Task 7 restores build."
```

---

## Task 7: Update coordinator zome — delete dead functions, modernize RecoveryRequest coordinator, update KeyRotation coordinator

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`

- [ ] **Step 1: Delete commit_recovery_seed and its types**

Find (around line 2327 per earlier audit):
- `pub struct CommitRecoverySeedInput { ... }`
- `pub struct RecoverySeedCommitmentOutput { ... }`
- `#[hdk_extern] pub fn commit_recovery_seed(...) -> ExternResult<RecoverySeedCommitmentOutput> { ... }`

Delete all three.

- [ ] **Step 2: Delete create_recovery_quorum_request and its types**

Find:
- `pub struct CreateRecoveryQuorumRequestInput { ... }`
- `pub struct RecoveryQuorumRequestOutput { ... }`
- `#[hdk_extern] pub fn create_recovery_quorum_request(...) -> ExternResult<RecoveryQuorumRequestOutput> { ... }`

Delete all three.

- [ ] **Step 3: Delete dead signal variants**

Find the `RecoveryV2Signal` enum. Delete the variants:
```rust
    SeedCommitmentCreated { ... },
    SeedCommitmentSuperseded { ... },
    RecoveryQuorumRequestCreated { ... },
```

Keep `KeyRotationCommitted { ... }` (still needed). Add a new variant for modernized RecoveryRequest:

```rust
    RecoveryRequestCreated {
        action_hash: ActionHash,
        request: RecoveryRequest,
    },
```

- [ ] **Step 4: Modernize the existing create_recovery_request legacy coordinator**

Find the legacy `pub fn create_recovery_request` function (from the Jan 2026 code; around line 1600+ per earlier audit). It currently uses the old `RecoveryRequest` struct with string IDs, `elohim_questions_json`, etc.

Replace it with:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateRecoveryRequestInput {
    pub human_agent_pubkey: AgentPubKey,
    pub new_agent_pubkey: AgentPubKey,
    pub hosting_doorway_pubkey: AgentPubKey,
    pub proposed_authority: RecoveryAuthorityKind,
    pub request_nonce: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecoveryRequestOutput {
    pub action_hash: ActionHash,
    pub request: RecoveryRequest,
}

#[hdk_extern]
pub fn create_recovery_request(input: CreateRecoveryRequestInput) -> ExternResult<RecoveryRequestOutput> {
    let now = sys_time()?;

    let request = RecoveryRequest {
        human_agent_pubkey: input.human_agent_pubkey.clone(),
        new_agent_pubkey: input.new_agent_pubkey,
        hosting_doorway_pubkey: input.hosting_doorway_pubkey,
        proposed_authority: input.proposed_authority,
        request_nonce: input.request_nonce,
        created_at: now,
    };

    let action_hash = create_entry(&EntryTypes::RecoveryRequest(request.clone()))?;

    // Link Anchor(human_pubkey) → request using the existing HumanToRecoveryRequest link type.
    // This uses the existing StringAnchor pattern.
    let anchor = StringAnchor {
        anchor_type: "recovery_request".to_string(),
        anchor_text: input.human_agent_pubkey.to_string(),
    };
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(
        anchor_hash,
        action_hash.clone(),
        LinkTypes::HumanToRecoveryRequest,
        (),
    )?;

    emit_signal(RecoveryV2Signal::RecoveryRequestCreated {
        action_hash: action_hash.clone(),
        request: request.clone(),
    })?;

    Ok(RecoveryRequestOutput {
        action_hash,
        request,
    })
}
```

This modernizes the existing `create_recovery_request` coordinator. If the legacy function was named differently or was wrapped in other logic, delete whatever the legacy wrapping was and replace with this clean version.

- [ ] **Step 5: Update commit_key_rotation coordinator for new KeyRotation shape**

Find `pub fn commit_key_rotation`. Replace its input struct and body:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommitKeyRotationInput {
    pub human_agent_pubkey: AgentPubKey,
    pub new_agent_pubkey: AgentPubKey,
    pub superseded_agent_pubkey: AgentPubKey,
    pub recovery_request_hash: ActionHash,
    pub authority: RecoveryAuthority,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeyRotationOutput {
    pub action_hash: ActionHash,
    pub rotation: KeyRotation,
}

#[hdk_extern]
pub fn commit_key_rotation(input: CommitKeyRotationInput) -> ExternResult<KeyRotationOutput> {
    let now = sys_time()?;

    let rotation = KeyRotation {
        human_agent_pubkey: input.human_agent_pubkey.clone(),
        new_agent_pubkey: input.new_agent_pubkey.clone(),
        superseded_agent_pubkey: input.superseded_agent_pubkey,
        recovery_request_hash: input.recovery_request_hash,
        authority: input.authority,
        rotated_at: now,
    };

    // NOTE: Validation runs in-zome and currently stub-rejects all variants.
    // M2 adds variant-specific validation. Until then, this coordinator will
    // return Err on any attempted rotation — intentional during M1-cleanup.
    let action_hash = create_entry(&EntryTypes::KeyRotation(rotation.clone()))?;

    let current_agent_anchor = StringAnchor {
        anchor_type: "current_agent".to_string(),
        anchor_text: input.human_agent_pubkey.to_string(),
    };
    let current_agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(current_agent_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(current_agent_anchor))?;
    create_link(
        current_agent_anchor_hash,
        action_hash.clone(),
        LinkTypes::HumanToCurrentAgent,
        (),
    )?;

    let agent_rotation_anchor = StringAnchor {
        anchor_type: "agent_rotation".to_string(),
        anchor_text: input.new_agent_pubkey.to_string(),
    };
    let agent_rotation_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_rotation_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(agent_rotation_anchor))?;
    create_link(
        agent_rotation_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToKeyRotation,
        (),
    )?;

    emit_signal(RecoveryV2Signal::KeyRotationCommitted {
        action_hash: action_hash.clone(),
        rotation: rotation.clone(),
    })?;

    Ok(KeyRotationOutput {
        action_hash,
        rotation,
    })
}
```

If the original M1 `commit_key_rotation` created a separate `HumanToCurrentSeedCommitment` link or referenced `SeedCommitmentToRequest`, remove those — those link types were deleted in Task 3.

- [ ] **Step 6: Delete any remaining legacy coordinator wrapping**

The Jan 2026 legacy `create_recovery_request` had output types like `RecoveryRequestOutput` with fields for `elohim_questions`, `confidence_score`, etc. If any leftover wrapper functions (e.g., `score_recovery_request`, `apply_elohim_verification`) exist that depended on the old struct's fields, delete them. They're stubs that no frontend uses.

Grep for any leftover:
```bash
grep -n "elohim_score\|elohim_questions\|confidence_score\|recovery_method" /projects/elohim/elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
```

If references remain, either update to use the new struct (if the feature is still meaningful) or delete (if it was stubby). Prefer delete.

- [ ] **Step 7: Type-check and build full DNA**

```bash
cd /projects/elohim && just dna-imagodei
```

Expected: builds clean. Restores the broken state from Task 6.

- [ ] **Step 8: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "feat(imagodei): modernize coordinator zome for revised recovery data model

Deletes commit_recovery_seed and create_recovery_quorum_request. Modernizes
create_recovery_request to use the new RecoveryRequest struct. Updates
commit_key_rotation to use the RecoveryAuthority enum. Coordinator now
builds with the integrity zome changes from Task 6."
```

---

## Task 8: Storage migration — drop dead tables, restructure surviving ones

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-22-000000_recovery_phase_2_cleanup/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-22-000000_recovery_phase_2_cleanup/down.sql`
- Modify: `elohim/elohim-storage/src/schema.rs`

- [ ] **Step 1: Write up.sql**

Create `elohim/elohim-storage/migrations/2026-04-22-000000_recovery_phase_2_cleanup/up.sql`:

```sql
-- Recovery Protocol Phase 2 — M1-Cleanup
-- Source of truth: DHT (imagodei DNA).
-- Drops deleted-entry-type projections and restructures surviving ones for the revised shape.

-- Drop the dead seed-commitment projection entirely.
DROP INDEX IF EXISTS idx_recovery_seed_commitments_active;
DROP INDEX IF EXISTS idx_recovery_seed_commitments_human;
DROP TABLE IF EXISTS recovery_seed_commitments;

-- Restructure recovery_quorum_requests → recovery_requests.
-- Source of truth: DHT (imagodei RecoveryRequest entry).
-- Projection populated from RecoveryV2Signal::RecoveryRequestCreated.
DROP INDEX IF EXISTS idx_recovery_quorum_requests_commitment;
DROP INDEX IF EXISTS idx_recovery_quorum_requests_human;
DROP TABLE IF EXISTS recovery_quorum_requests;

CREATE TABLE recovery_requests (
    dht_anchor_hash          TEXT PRIMARY KEY NOT NULL,
    human_agent_pubkey       TEXT NOT NULL,
    new_agent_pubkey         TEXT NOT NULL,
    hosting_doorway_pubkey   TEXT NOT NULL,
    proposed_authority_kind  TEXT NOT NULL,   -- "intimate_quorum" | "community_consensus" | "governance_act" | "network_witness" | "cryptographic_quorum"
    proposed_authority_json  TEXT NOT NULL,   -- JSON blob for variant-specific fields (grant_hash, purpose, stewardship_hash)
    request_nonce            BLOB NOT NULL,
    created_at               TEXT NOT NULL
);
CREATE INDEX idx_recovery_requests_human ON recovery_requests(human_agent_pubkey);
CREATE INDEX idx_recovery_requests_kind ON recovery_requests(proposed_authority_kind);

-- Restructure key_rotations for the RecoveryAuthority enum.
-- Source of truth: DHT (imagodei KeyRotation entry).
-- Projection populated from RecoveryV2Signal::KeyRotationCommitted.
-- The dht_anchor_hash is authoritative; this projection is a fast-lookup cache.
DROP INDEX IF EXISTS idx_key_rotations_new_agent;
DROP INDEX IF EXISTS idx_key_rotations_human;
DROP TABLE IF EXISTS key_rotations;

CREATE TABLE key_rotations (
    dht_anchor_hash          TEXT PRIMARY KEY NOT NULL,
    human_agent_pubkey       TEXT NOT NULL,
    new_agent_pubkey         TEXT NOT NULL,
    superseded_agent_pubkey  TEXT NOT NULL,
    recovery_request_hash    TEXT NOT NULL,
    authority_kind           TEXT NOT NULL,   -- variant discriminator
    authority_json           TEXT NOT NULL,   -- JSON blob for variant fields (witness_hashes, challenge_hash, etc.)
    rotated_at               TEXT NOT NULL
);
CREATE INDEX idx_key_rotations_human ON key_rotations(human_agent_pubkey);
CREATE INDEX idx_key_rotations_new_agent ON key_rotations(new_agent_pubkey);
CREATE INDEX idx_key_rotations_authority_kind ON key_rotations(authority_kind);
```

- [ ] **Step 2: Write down.sql**

Create `down.sql`:

```sql
-- Down migration: reverse M1-cleanup by restoring the prior M1 shape.
-- Not expected to run in practice (dev-only clean slate) but provided for completeness.

DROP INDEX IF EXISTS idx_key_rotations_authority_kind;
DROP INDEX IF EXISTS idx_key_rotations_new_agent;
DROP INDEX IF EXISTS idx_key_rotations_human;
DROP TABLE IF EXISTS key_rotations;

DROP INDEX IF EXISTS idx_recovery_requests_kind;
DROP INDEX IF EXISTS idx_recovery_requests_human;
DROP TABLE IF EXISTS recovery_requests;

-- Restore the M1 seed-commitment table (for completeness; content is lost).
CREATE TABLE recovery_seed_commitments (
    dht_anchor_hash      TEXT PRIMARY KEY NOT NULL,
    human_agent_pubkey   TEXT NOT NULL,
    seed_public_half     BLOB NOT NULL,
    threshold_n          INTEGER NOT NULL,
    total_m              INTEGER NOT NULL,
    commitment_nonce     BLOB NOT NULL,
    superseded_by        TEXT,
    created_at           TEXT NOT NULL
);
CREATE INDEX idx_recovery_seed_commitments_human ON recovery_seed_commitments(human_agent_pubkey);
CREATE INDEX idx_recovery_seed_commitments_active ON recovery_seed_commitments(human_agent_pubkey) WHERE superseded_by IS NULL;

-- Restore the M1 recovery_quorum_requests and key_rotations (prior shapes).
CREATE TABLE recovery_quorum_requests (
    dht_anchor_hash        TEXT PRIMARY KEY NOT NULL,
    human_agent_pubkey     TEXT NOT NULL,
    seed_commitment_hash   TEXT NOT NULL,
    new_agent_pubkey       TEXT NOT NULL,
    hosting_doorway_pubkey TEXT NOT NULL,
    recovery_mode          TEXT NOT NULL,
    stewarded_grant_hash   TEXT,
    request_nonce          BLOB NOT NULL,
    created_at             TEXT NOT NULL
);
CREATE INDEX idx_recovery_quorum_requests_human ON recovery_quorum_requests(human_agent_pubkey);
CREATE INDEX idx_recovery_quorum_requests_commitment ON recovery_quorum_requests(seed_commitment_hash);

CREATE TABLE key_rotations (
    dht_anchor_hash          TEXT PRIMARY KEY NOT NULL,
    human_agent_pubkey       TEXT NOT NULL,
    new_agent_pubkey         TEXT NOT NULL,
    superseded_agent_pubkey  TEXT NOT NULL,
    seed_commitment_hash     TEXT NOT NULL,
    recovery_request_hash    TEXT NOT NULL,
    quorum_signature         BLOB NOT NULL,
    rotated_at               TEXT NOT NULL
);
CREATE INDEX idx_key_rotations_human ON key_rotations(human_agent_pubkey);
CREATE INDEX idx_key_rotations_new_agent ON key_rotations(new_agent_pubkey);
```

- [ ] **Step 3: Update schema.rs**

The M1 work hand-patched `src/schema.rs` with `table!` declarations for `recovery_seed_commitments`, `recovery_quorum_requests`, `key_rotations`. Update manually:

- Delete the `table! { recovery_seed_commitments { ... } }` block
- Replace the `recovery_quorum_requests` block with:

```rust
diesel::table! {
    recovery_requests (dht_anchor_hash) {
        dht_anchor_hash -> Text,
        human_agent_pubkey -> Text,
        new_agent_pubkey -> Text,
        hosting_doorway_pubkey -> Text,
        proposed_authority_kind -> Text,
        proposed_authority_json -> Text,
        request_nonce -> Binary,
        created_at -> Text,
    }
}
```

- Replace the `key_rotations` block with:

```rust
diesel::table! {
    key_rotations (dht_anchor_hash) {
        dht_anchor_hash -> Text,
        human_agent_pubkey -> Text,
        new_agent_pubkey -> Text,
        superseded_agent_pubkey -> Text,
        recovery_request_hash -> Text,
        authority_kind -> Text,
        authority_json -> Text,
        rotated_at -> Text,
    }
}
```

Match the exact style/indentation of surrounding `table!` blocks in `schema.rs`.

- [ ] **Step 4: Build storage to verify**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -20
```

Expected: builds clean. If there are references to the old table names elsewhere in code (queries, projection handlers), note them — they must be updated in Task 9.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-04-22-000000_recovery_phase_2_cleanup \
        elohim/elohim-storage/src/schema.rs
git commit -m "feat(storage): migration for recovery phase 2 m1-cleanup

Drops recovery_seed_commitments. Restructures recovery_quorum_requests
into recovery_requests and key_rotations for the new RecoveryAuthority
enum shape."
```

---

## Task 9: Update views.rs for new shapes

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1: Delete RecoverySeedCommitmentView**

Find the `RecoverySeedCommitmentView` struct and its doc comment. Delete both.

- [ ] **Step 2: Rename RecoveryQuorumRequestView → RecoveryRequestView with new fields**

Find `RecoveryQuorumRequestView`. Replace its entire definition (doc comment + struct) with:

```rust
/// Source of truth: DHT (imagodei RecoveryRequest entry).
/// RecoveryRequestView — projection of the modernized RecoveryRequest DHT entry.
/// Replaces the M1 RecoveryQuorumRequestView. See recovery phase 2 revised spec §5.3.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecoveryRequestView {
    pub dht_anchor_hash: String,
    pub human_agent_pubkey: String,
    pub new_agent_pubkey: String,
    pub hosting_doorway_pubkey: String,
    /// Discriminator for proposed_authority — "intimateQuorum" | "communityConsensus" | "governanceAct" | "networkWitness" | "cryptographicQuorum".
    pub proposed_authority_kind: String,
    /// JSON-encoded variant-specific fields (grant_hash, purpose, stewardship_hash, etc.).
    /// Empty string `""` for variants with no extra fields.
    pub proposed_authority_json: String,
    pub request_nonce: Vec<u8>,
    pub created_at: String,
}
```

- [ ] **Step 3: Update KeyRotationView**

Find `KeyRotationView`. Replace its entire definition with:

```rust
/// Source of truth: DHT (imagodei KeyRotation entry).
/// KeyRotationView — projection of the modernized KeyRotation DHT entry with RecoveryAuthority enum.
/// Replaces the M1 KeyRotationView (which had seed_commitment_hash + quorum_signature fields).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KeyRotationView {
    pub dht_anchor_hash: String,
    pub human_agent_pubkey: String,
    pub new_agent_pubkey: String,
    pub superseded_agent_pubkey: String,
    pub recovery_request_hash: String,
    /// Discriminator for authority — "intimateQuorum" | "communityConsensus" | "governanceAct" | "networkWitness" | "cryptographicQuorum".
    pub authority_kind: String,
    /// JSON-encoded variant-specific fields (witness_hashes, challenge_hash, etc.).
    pub authority_json: String,
    pub rotated_at: String,
}
```

- [ ] **Step 4: Regenerate TS bindings**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -15
```

Expected: test passes. Three new `.ts` files in `elohim/sdk/storage-client-ts/src/generated/`:
- `RecoveryRequestView.ts` (new)
- `KeyRotationView.ts` (updated)
- Also verify the old `RecoverySeedCommitmentView.ts` is no longer generated — it may need manual deletion if `cargo test export_bindings` doesn't clean up.

Manual cleanup if needed:
```bash
rm -f /projects/elohim/elohim/sdk/storage-client-ts/src/generated/RecoverySeedCommitmentView.ts
rm -f /projects/elohim/elohim/sdk/storage-client-ts/src/generated/RecoveryQuorumRequestView.ts
```

Verify the generated files look right:
```bash
cat /projects/elohim/elohim/sdk/storage-client-ts/src/generated/RecoveryRequestView.ts
cat /projects/elohim/elohim/sdk/storage-client-ts/src/generated/KeyRotationView.ts
```

- [ ] **Step 5: Check for broken references**

```bash
grep -rn "RecoverySeedCommitmentView\|RecoveryQuorumRequestView" /projects/elohim/elohim/ 2>/dev/null | grep -v "target/\|node_modules/\|\.git/" | head -20
```

Any remaining references (outside docs/ and specs/) are broken code — fix them. Likely places:
- Projection handler code (if any exists yet)
- Re-exports in lib.rs

For each, update to new names or delete if dead.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/views.rs \
        elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): update views.rs for revised recovery phase 2 shapes

Deletes RecoverySeedCommitmentView. Renames RecoveryQuorumRequestView to
RecoveryRequestView with proposed_authority_kind + proposed_authority_json
fields. Updates KeyRotationView to carry authority_kind + authority_json
instead of seed_commitment_hash + quorum_signature."
```

---

## Task 10: Update JSON schemas + codegen pipeline

**Files:**
- Delete: `elohim/sdk/schemas/v1/views/recovery-seed-commitment.schema.json`
- Delete: `elohim/sdk/schemas/v1/views/recovery-request.schema.json`
- Create: `elohim/sdk/schemas/v1/views/recovery-request.schema.json`
- Modify: `elohim/sdk/schemas/v1/views/key-rotation.schema.json`
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs`

- [ ] **Step 1: Delete dead schemas**

```bash
rm /projects/elohim/elohim/sdk/schemas/v1/views/recovery-seed-commitment.schema.json
rm /projects/elohim/elohim/sdk/schemas/v1/views/recovery-request.schema.json
```

- [ ] **Step 2: Create recovery-request.schema.json**

Create `elohim/sdk/schemas/v1/views/recovery-request.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "epr:schema:view:recovery-request",
  "title": "RecoveryRequestView",
  "description": "Projection of imagodei RecoveryRequest DHT entry (modernized). Source of truth: DHT.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "dhtAnchorHash",
    "humanAgentPubkey",
    "newAgentPubkey",
    "hostingDoorwayPubkey",
    "proposedAuthorityKind",
    "proposedAuthorityJson",
    "requestNonce",
    "createdAt"
  ],
  "properties": {
    "dhtAnchorHash": { "type": "string" },
    "humanAgentPubkey": { "type": "string" },
    "newAgentPubkey": { "type": "string" },
    "hostingDoorwayPubkey": { "type": "string" },
    "proposedAuthorityKind": {
      "type": "string",
      "enum": ["intimateQuorum", "communityConsensus", "governanceAct", "networkWitness", "cryptographicQuorum"]
    },
    "proposedAuthorityJson": { "type": "string" },
    "requestNonce": {
      "type": "array",
      "items": { "type": "integer", "minimum": 0, "maximum": 255 },
      "minItems": 16,
      "maxItems": 16
    },
    "createdAt": { "type": "string" }
  }
}
```

- [ ] **Step 3: Update key-rotation.schema.json**

Replace the contents of `elohim/sdk/schemas/v1/views/key-rotation.schema.json` with:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "epr:schema:view:key-rotation",
  "title": "KeyRotationView",
  "description": "Projection of imagodei KeyRotation DHT entry with RecoveryAuthority enum. Source of truth: DHT. The authoritative claim that a human's agent key has rotated. Authority evidence carried as variant_kind + JSON-encoded variant fields.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "dhtAnchorHash",
    "humanAgentPubkey",
    "newAgentPubkey",
    "supersededAgentPubkey",
    "recoveryRequestHash",
    "authorityKind",
    "authorityJson",
    "rotatedAt"
  ],
  "properties": {
    "dhtAnchorHash": { "type": "string" },
    "humanAgentPubkey": { "type": "string" },
    "newAgentPubkey": { "type": "string" },
    "supersededAgentPubkey": { "type": "string" },
    "recoveryRequestHash": { "type": "string" },
    "authorityKind": {
      "type": "string",
      "enum": ["intimateQuorum", "communityConsensus", "governanceAct", "networkWitness", "cryptographicQuorum"]
    },
    "authorityJson": { "type": "string" },
    "rotatedAt": { "type": "string" }
  }
}
```

- [ ] **Step 4: Update codegen-ts.mjs**

Open `elohim/sdk/schemas/scripts/codegen-ts.mjs`. Find `INTERFACE_FILES`. Remove:
- `'recovery-seed-commitment'`
- `'recovery-quorum-request'`

Add (in alphabetical place matching existing style):
- `'recovery-request'`

(The `'key-rotation'` entry stays.)

- [ ] **Step 5: Run schema pipeline**

```bash
cd /projects/elohim && pnpm run schema:test && pnpm run schema:codegen:ts && pnpm run schema:validate
```

Expected: all three pass. If any fail, read the error and fix (usually a typo in the JSON or a mismatched $id). `schema:validate` may flag existing seed data that references deleted schemas — if so, check the seed data and update or note for follow-up.

- [ ] **Step 6: Commit**

```bash
git add -A elohim/sdk/schemas/
git commit -m "feat(schema): update JSON wire schemas for revised recovery phase 2

Deletes recovery-seed-commitment and recovery-quorum-request schemas.
Adds recovery-request schema (modernized). Updates key-rotation schema
to carry authorityKind + authorityJson for the RecoveryAuthority enum."
```

---

## Task 11: Update schema contract tests

**Files:**
- Modify: `elohim/elohim-storage/tests/schema_contract_recovery_v2.rs`

- [ ] **Step 1: Replace test file with revised versions**

Open `elohim/elohim-storage/tests/schema_contract_recovery_v2.rs`. Replace the `recovery_seed_commitment_view_matches_schema` test (delete entirely) and update the other two:

Replace the file's test functions with:

```rust
#[test]
fn recovery_request_view_matches_schema() {
    let view = RecoveryRequestView {
        dht_anchor_hash: "req001".to_string(),
        human_agent_pubkey: "uhCAk_human".to_string(),
        new_agent_pubkey: "uhCAk_new".to_string(),
        hosting_doorway_pubkey: "uhCAk_doorway".to_string(),
        proposed_authority_kind: "intimateQuorum".to_string(),
        proposed_authority_json: "{}".to_string(),
        request_nonce: vec![0u8; 16],
        created_at: "2026-04-22T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&view).expect("serializes");
    let schema = load_schema("recovery-request");
    validate_against_schema(&schema, &json, "RecoveryRequestView");
}

#[test]
fn key_rotation_view_matches_schema() {
    let view = KeyRotationView {
        dht_anchor_hash: "rot001".to_string(),
        human_agent_pubkey: "uhCAk_human".to_string(),
        new_agent_pubkey: "uhCAk_new".to_string(),
        superseded_agent_pubkey: "uhCAk_old".to_string(),
        recovery_request_hash: "req001".to_string(),
        authority_kind: "intimateQuorum".to_string(),
        authority_json: "{\"witnessHashes\":[]}".to_string(),
        rotated_at: "2026-04-22T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&view).expect("serializes");
    let schema = load_schema("key-rotation");
    validate_against_schema(&schema, &json, "KeyRotationView");
}
```

If the test file also contains `load_schema` and `validate_against_schema` helpers (copied from the existing pattern in M1 Task 15), keep them — they're still correct. If they reference jsonschema 0.17 APIs that don't work in 0.28, check the existing `schema_contract.rs` for the current working pattern.

Also update the imports at the top of the file:

```rust
use elohim_storage::views::{KeyRotationView, RecoveryRequestView};
```

Remove `RecoverySeedCommitmentView` and `RecoveryQuorumRequestView` from the imports.

- [ ] **Step 2: Run the test**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract_recovery_v2 2>&1 | tail -15
```

Expected: two tests pass. If failures: inspect the schema mismatch, adjust either the test data (most likely) or the schema/view (if a real bug was introduced).

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/tests/schema_contract_recovery_v2.rs
git commit -m "test(storage): update schema contract tests for revised recovery shapes"
```

---

## Task 12: Full-stack verification + push

**Files:** none modified; verification + push only.

- [ ] **Step 1: DNA build**

```bash
cd /projects/elohim && just dna-imagodei 2>&1 | tail -10
```

Expected: WASM builds clean.

- [ ] **Step 2: Storage build + test**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -10 && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins 2>&1 | tail -10
```

Expected: build clean; unit tests pass.

- [ ] **Step 3: Full schema pipeline**

```bash
cd /projects/elohim && pnpm run schema:test && pnpm run schema:codegen:ts && pnpm run schema:validate && pnpm run schema:check-dna
```

Expected: all four pass.

- [ ] **Step 4: Check for dead references one last time**

```bash
cd /projects/elohim && grep -rn "RecoverySeedCommitment\|HeldRecoveryShare\|MyRecoveryAuthorization\|RecoveryQuorumRequest" . 2>/dev/null | \
  grep -v "target/\|node_modules/\|\.git/\|genesis/docs/\|\.claude/" | head -30
```

Expected: only hits are in the spec (historical reference) and comments explaining the cleanup. Any live code hits are bugs that must be fixed.

- [ ] **Step 5: Review git log and push**

```bash
cd /projects/elohim && git log --oneline origin/dev..HEAD
```

Expected: ~11 commits (Tasks 1–11). Review messages for clarity.

```bash
cd /projects/elohim && git push origin dev 2>&1 | tail -40
```

Husky pre-push hooks will run elohim-library tests, elohim-storage cargo fmt/clippy, elohim-app tests. If any fail:
1. Read the failure carefully.
2. Fix the underlying issue (formatting, linting, broken test).
3. Stage the fix, commit with a `fix(...)` message.
4. Re-push.

Do NOT bypass with `HUSKY=0`.

- [ ] **Step 6: Verify remote**

```bash
git log --oneline origin/dev -15
```

Expected: local and remote HEAD match. M1-cleanup is on `origin/dev`.

---

## Definition of Done (M1-cleanup)

- [ ] `RecoverySeedCommitment`, `HeldRecoveryShare`, `MyRecoveryAuthorization` entry types removed from imagodei DNA
- [ ] Corresponding link types (`HumanToCurrentSeedCommitment`, `SeedCommitmentSupersededBy`, `SeedCommitmentToRequest`) removed
- [ ] Coordinator zome cleaned up: `commit_recovery_seed`, `create_recovery_quorum_request` deleted; signal variants removed
- [ ] `RecoveryAuthority`, `RecoveryAuthorityKind`, `NetworkWitnessPurpose` enums added
- [ ] `KeyRotation` struct evolved to use `RecoveryAuthority`; validator stub-rejects all variants with clear Phase-2b/M2 messages
- [ ] Legacy `RecoveryRequest` struct replaced with modernized shape; coordinator `create_recovery_request` updated
- [ ] `RecoveryQuorumRequest` fully deleted (its role merged into modernized `RecoveryRequest`)
- [ ] Storage migration `2026-04-22-000000_recovery_phase_2_cleanup` drops old tables and creates revised ones
- [ ] Rust views updated: `RecoverySeedCommitmentView` deleted, `RecoveryQuorumRequestView` renamed/restructured to `RecoveryRequestView`, `KeyRotationView` updated
- [ ] JSON schemas pruned and updated; codegen pipeline passes
- [ ] Schema contract tests pass against new shapes
- [ ] DNA, storage, TS SDK all build clean
- [ ] All changes pushed to `origin/dev` with husky guards passing

## What M1-cleanup does NOT do (deferred to M2+)

- Implement `IntimateQuorum` + `CryptographicQuorum` variant validators (happy paths). Stub-reject until M2.
- Add `frozen_at_layer` field to `IdentityFreeze` struct. M2.
- Wire the floor-rises-after-attack validator check. M2.
- libp2p coordination, defender specialist, UI work. M3+.

---

## Self-Review

**Spec coverage:** Each requirement of §9 "M1-cleanup" in the revised spec is covered by a task above. Specifically:
- "Delete RecoverySeedCommitment, HeldRecoveryShare, MyRecoveryAuthorization" → Tasks 1, 2
- "Delete link types" → Task 3
- "Delete storage projections for seed commitments" → Task 8
- "Delete views for seed commitments" → Task 9
- "Delete JSON schemas + TS codegen artifacts" → Task 10
- "Contract tests reference deleted types removed" → Task 11
- "Coordinator functions deleted / modernized" → Task 7
- "Signal variants for deleted types removed" → Task 7
- "Evolve KeyRotation to use RecoveryAuthority enum" → Tasks 4, 5
- "Evolve storage projection + views + schemas + contract tests accordingly" → Tasks 8, 9, 10, 11
- "Modernize legacy RecoveryRequest struct in place" → Task 6
- "Full-stack verification + push" → Task 12

**Placeholder scan:** No "TBD", "TODO", "implement later", or vague instructions. Each task has complete code, exact commands, and clear acceptance criteria. One instance of "note them" in Task 9 Step 5 is fine — it's instruction to the engineer to observe and fix *real* broken references, not a deferral of specified work.

**Type consistency:**
- `RecoveryAuthority` variant names are consistent across: recovery_v2.rs definitions (Task 4), KeyRotation struct (Task 5), coordinator input type (Task 7), SQL migration `authority_kind` values (Task 8), Rust view field names (Task 9), JSON schema enum (Task 10), test fixtures (Task 11).
- `RecoveryAuthorityKind` variant names (without inner fields) consistent across recovery_v2.rs (Task 4), RecoveryRequest struct (Task 6), coordinator input (Task 7), SQL migration `proposed_authority_kind` (Task 8), Rust view (Task 9), JSON schema enum (Task 10).
- Field names (snake_case Rust ↔ camelCase JSON/TS) aligned via `#[serde(rename_all = "camelCase")]` everywhere views export to JSON/TS.

**Scope:** 12 tasks covering a cohesive cleanup. Each task commits, each task is 2–5 actions, each task has clear pre/post state. Sized for ~3 subagent dispatches of 3–4 tasks each.

---

Plan complete and saved to `genesis/docs/superpowers/plans/2026-04-22-recovery-protocol-phase-2-m1-cleanup.md`.
