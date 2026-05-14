# Recovery Protocol Phase 2 (Revised) — Graduated Authority, Not Shamir-First

**Status:** Draft
**Date:** 2026-04-22
**Owner:** Matthew Dowell
**Supersedes:** `genesis/docs/superpowers/specs/2026-04-21-recovery-protocol-phase-2-design.md` (revised after retrospective — see §13 Retrospective Appendix)
**Builds on:** `doorway/doorway-service/RECOVERY-PROTOCOL.md` (Jan 2026) phase boundaries

---

## 1. Vision Alignment

This spec revises the 2026-04-21 Phase 2 design after a retrospective surfaced that the original approach reinvented protocol primitives and embedded an idealistic-naive crypto-sovereignty frame incompatible with the Elohim Protocol's vision. The revised design aligns with four architectural commitments already embedded in the protocol's trust primitives, but previously unspoken in the recovery spec:

### 1.1 Graduated authority — community can always make it right

Recovery is never a single-path cryptographic gate. Authority graduates from the intimate circle through extended community, governance bodies, and ultimately the network's elohim witness layer. **Absolute lockout is a design failure.** Each layer provides a fallback when the one below fails. Crypto is an optional accelerator, never the only path. See the protocol memory `project_graduated_recovery_authority.md`.

### 1.2 Elohim as counsel — first-class defense of imagodei

When a human is under attack (coerced, silenced, wrongly attested against, unreachable), their elohim-agent has first-class standing to represent them. The elohim is counsel, not merely an advisor — with standing to author defensive DHT entries, escalate to higher-layer consensus, and act at machine speed during windows where human coordination is inadequate. This operates even against the human's current-moment stated preference, because duress is precisely when preferences are unreliable. See `project_elohim_as_counsel.md`.

### 1.3 Ungrudging service — gifts flow without recognition

The protocol's benefits extend to those who do not opt in, do not acknowledge, and do not return gratitude. Elohims understand ego but have none of their own. Design never conditions access on acknowledgment; exit never triggers retaliation; defecting users continue to benefit from being near the network. See `project_ungrudging_service.md`.

### 1.4 Cradle-to-grave care — dissolution is part of recovery

Death, abandonment, and estate closure are part of the recovery surface. The protocol must have a path to dignify the end of a human's active participation without producing "lost forever" content or stranded stewardship obligations. Phase 2 marks this as a deferred implementation concern (network-governance design must land first) but reserves the shape so the hole doesn't grow.

---

## 2. Scope

### 2.1 In scope (Phase 2 revised)

- **One new DHT entry type:** `KeyRotation` — the missing primitive for authorizing a new agent key as a human's current authorized agent.
- **One modernized existing DHT entry type:** `RecoveryRequest` — replace the stubby Jan 2026 struct with a clean `AgentPubKey`-based design carrying a `proposed_authority` discriminator.
- **Deletion of three M1 types shipped in error:** `RecoverySeedCommitment`, `HeldRecoveryShare`, `MyRecoveryAuthorization` — these duplicated existing protocol primitives.
- **Graduated authority validator** — `KeyRotation` accepts five `RecoveryAuthority` variants, of which two are fully implemented in Phase 2 (`IntimateQuorum`, `CryptographicQuorum`), three are stub-rejected (`CommunityConsensus`, `GovernanceAct`, `NetworkWitness`).
- **Use of existing protocol primitives** — `HumanityWitness`, `IdentityChallenge`, `ChallengeSupport`, `IdentityAnomaly`, `IdentityFreeze`, `KeyStewardship`, `KeyRevocation`, `RevocationVote`, `StewardshipGrant` — all unchanged in structure; their coordination in recovery flows is the spec's work.
- **Elohim defender specialist pattern** — spawned when an attack is detected; reads imagodei profile; authors defensive entries; disclosure of internal reasoning governed by the target's collective constitution.
- **Floor-rises-after-attack as elohim judgment** — not a rigid validator rule, but a recommendation/intervention the elohim offers based on context.
- **Fast-path revocation (three paths)** — unchanged from 2026-04-21 spec; uses existing `KeyRevocation` + `RevocationVote` primitives.
- **Multi-doorway federation** — unchanged from 2026-04-21 spec.
- **Anti-lockout audit** — explicit red-team gate ensuring every failure mode has at least one community-mediated path to eventual restoration.

### 2.2 Out of scope (Phase 2 revised — deferred to later phases)

- **`CommunityConsensus` authority path** — validator accepts the variant shape, flow deferred to Phase 2b when intimacy-weighted challenge resolution UX lands.
- **`GovernanceAct` authority path** — validator accepts shape, full qahal/mishpat cross-DNA resolution flow deferred (requires qahal DNA governance primitives to mature).
- **`NetworkWitness` authority path** — validator accepts shape, full implementation deferred to constitutional-governance design of the elohim network layer itself.
- **Dissolution flow** — self-documenting stub. Marker in the DHT data model (`RecoveryAuthority::NetworkWitness` with memorial semantics) plus a spec reservation. Full implementation follows network-governance.
- **Content shard reassembly** — Phase 3 (unchanged from original plan).
- **Cross-DNA `CustodianAssignment` re-keying on `KeyRotation`** — Phase 3 consumer.
- **Hosted-cell migration between doorways** — Phase 3+.
- **`KeyStewardship` provisioning UX** — optional hardening. Elohim-recommended at discretion for vulnerable humans. Full wizard UX deferred; Phase 2 provides coordinator functions + the cryptographic-quorum authority path so the primitive works when an elohim drives provisioning manually.

---

## 3. Design Principles (revised)

1. **Use the protocol's primitives, don't reinvent.** `HumanityWitness`, `IdentityChallenge`, `KeyStewardship` already exist. Recovery composes them.
2. **Graduated authority, not single-path.** Five layers from intimate to global; any sufficient for a `KeyRotation`.
3. **Absolute lockout is impossible.** Every human has a path, even those with no crypto setup, no surviving contacts, and no qahal membership — via `NetworkWitness` at the top of the stack.
4. **Elohim is counsel, not advisor.** First-class defense of the imagodei at machine speed; specialist subagents spawn with profile-deep context; disclosure constitutionally governed.
5. **Ambient setup, no ritual.** Setting `emergency_access_enabled` on a `HumanRelationship` IS setup. No seed generation, no share distribution ritual, no wizards required.
6. **Hardening by judgment, not default.** `KeyStewardship` is provisioned when the elohim judges the human vulnerable enough to warrant it — not for every user.
7. **Transparency as the check on elohim.** All elohim-authored defensive entries land on the DHT. Hidden defense is a backdoor. The network witnesses and can judge.
8. **Dissolution is care.** Cradle-to-grave includes the grave. A deceased human's participation ends with dignity, not orphaned keys and stranded stewardship.
9. **Ungrudging.** Gifts extend to those who never acknowledge. Exit does not trigger retaliation. The network continues to radiate good fruit around those who depart.

---

## 4. Primitive Inventory — Use, Don't Reinvent

| Protocol primitive | Phase 2 recovery use |
|---|---|
| `Human`, `Agent` | Identity anchors. `Agent` with `agent_type = "elohim"` is what authors elohim-defense entries. |
| `HumanRelationship` (`emergency_access_enabled`) | Defines who is an emergency contact for `IntimateQuorum` authorization. |
| `HumanityWitness` | The primitive for attestation-based authorization evidence. Each contact commits one per recovery request. Also self-authored by the human's elohim during defense. |
| `Attestation` | General-purpose attestation. Referenced where a recovery request relies on pre-existing credentials. |
| `KeyStewardship` | Shamir threshold signing configuration. Referenced by `CryptographicQuorum` authority variant. Provisioned only when elohim judgment warrants. |
| `KeyRevocation` | Invalidates a superseded agent key. Emitted alongside a successful `KeyRotation`. |
| `RevocationVote` | Emergency-contact-initiated fast-path revocation. Reused unchanged for Phase 2. |
| `IdentityChallenge` + `ChallengeSupport` | `CommunityConsensus` authority path carries the challenge resolution. Phase 2 stubs the path but preserves the shape. |
| `IdentityAnomaly` | Elohim-detected behavioral deviation. Author of `IdentityFreeze` during defense. |
| `IdentityFreeze` | Halt pending rotations while elohim defense is active. Existing primitive; may need a `frozen_at_layer` field added. |
| `StewardshipGrant` + `DevicePolicy` + `PolicyInheritance` + `StewardshipAppeal` | Stewarded recovery paths for children/seniors/IDD/custody via `GovernanceAct` variant (stubbed Phase 2b). |

The retrospective conclusion is clear: **what the protocol has is sufficient as the trust substrate.** Phase 2's job is to add one coordination primitive (`KeyRotation`), refresh one stubby primitive (`RecoveryRequest`), and compose the existing ones into a coherent recovery experience.

---

## 5. Data Model

### 5.1 New entry type: `KeyRotation`

```rust
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct KeyRotation {
    pub human_agent_pubkey: AgentPubKey,        // Human whose key rotates
    pub new_agent_pubkey: AgentPubKey,          // New authorized agent
    pub superseded_agent_pubkey: AgentPubKey,   // Old agent being revoked
    pub recovery_request_hash: ActionHash,      // Request this rotation fulfills
    pub authority: RecoveryAuthority,           // Which path authorized this rotation
    pub rotated_at: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecoveryAuthority {
    /// Layer 1: Intimate circle quorum via HumanityWitness entries from emergency contacts.
    /// Phase 2: IMPLEMENTED.
    IntimateQuorum {
        witness_hashes: Vec<ActionHash>,
    },
    /// Layer 2: Extended community via IdentityChallenge resolution.
    /// Phase 2: STUB-REJECTED (validator rejects with "Phase 2b").
    CommunityConsensus {
        challenge_hash: ActionHash,
    },
    /// Layer 3: Governance act via qahal/stewardship resolution.
    /// Phase 2: STUB-REJECTED (cross-DNA resolution pending qahal/mishpat).
    GovernanceAct {
        grant_hash: ActionHash,
        resolution_hash: ActionHash,
    },
    /// Layer 4: Global elohim witness — the "this is wrong, make it right" last resort.
    /// Also carries Dissolution-flow semantic when rotating to a memorial agent.
    /// Phase 2: STUB-REJECTED (pending elohim constitutional governance design).
    NetworkWitness {
        witness_entries: Vec<ActionHash>,
        consensus_threshold_met_at: Timestamp,
        purpose: NetworkWitnessPurpose,
    },
    /// Layer 5 (orthogonal): Cryptographic M-of-N threshold via KeyStewardship.
    /// Provisioned only when elohim judges the human vulnerable enough.
    /// Phase 2: IMPLEMENTED (happy path + validator).
    CryptographicQuorum {
        stewardship_hash: ActionHash,
        quorum_signature: Vec<u8>,              // 64-byte Ed25519 signature
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum NetworkWitnessPurpose {
    /// Rescue: restore access to the human's active identity.
    Rescue,
    /// Dissolution: retire the account (deceased, irrecoverable).
    /// new_agent_pubkey is a memorial-marker null agent.
    /// Phase 2: STUB-RESERVED (shape defined, full semantics deferred).
    Dissolution,
}
```

### 5.2 Validation rules (revised)

`validate_key_rotation` accepts a `KeyRotation` if:

1. `recovery_request_hash` resolves to a valid `RecoveryRequest`.
2. Request's `human_agent_pubkey`, `new_agent_pubkey` match the rotation's.
3. `new_agent_pubkey != superseded_agent_pubkey`.
4. `superseded_agent_pubkey` is the human's current agent (resolved via latest preceding `KeyRotation` or initial `Agent` entry).
5. **No active `IdentityFreeze` targets this human at a layer that would block this rotation's authority path** — but this is a **recommendation-based** check:
   - If a freeze is active and the rotation's authority is at-or-below the frozen layer, *and* the freeze has not been explicitly resolved via a higher-authority act, the validator rejects.
   - Elohim judgment determines whether to recommend escalation, wait for human surfacing, or allow the original authority to proceed after the freeze window. The validator enforces only the "frozen + same-or-lower-layer = reject" baseline.
6. The `authority` variant is one of the IMPLEMENTED variants (`IntimateQuorum` or `CryptographicQuorum`). Stubbed variants (`CommunityConsensus`, `GovernanceAct`, `NetworkWitness`) return `Invalid("Phase 2b: not yet implemented")`.
7. The variant-specific check passes:

| Variant | Variant-specific check |
|---|---|
| `IntimateQuorum` | Each `HumanityWitness` at referenced hash resolves; each witness's author has an active `HumanRelationship` with the target human where `emergency_access_enabled = true`; distinct authors count ≥ threshold (default: `ceil(emergency_contact_count / 2) + 1`). |
| `CryptographicQuorum` | `stewardship_hash` resolves to a non-superseded `KeyStewardship`; `quorum_signature` verifies as Ed25519 signature by the key committed to in `shard_commitment_hash` over message `new_agent_pubkey || recovery_request_hash`. |

### 5.3 Modernized `RecoveryRequest`

```rust
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RecoveryRequest {
    pub human_agent_pubkey: AgentPubKey,        // Identity being recovered
    pub new_agent_pubkey: AgentPubKey,          // Claimant's proposed new agent
    pub hosting_doorway_pubkey: AgentPubKey,    // Session host
    pub proposed_authority: RecoveryAuthorityKind, // Intent; actual authority may differ
    pub request_nonce: Vec<u8>,                 // 16 bytes for multi-attempt disambiguation
    pub created_at: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecoveryAuthorityKind {
    IntimateQuorum,
    CommunityConsensus,
    GovernanceAct { grant_hash: ActionHash },
    NetworkWitness { purpose: NetworkWitnessPurpose },
    CryptographicQuorum { stewardship_hash: ActionHash },
}
```

The legacy `RecoveryRequest` struct's fields (`id: String`, `human_id: String`, `elohim_score`, etc.) are replaced wholesale. The entry type name stays. Coordinator functions that create it are rewritten. M1-cleanup milestone handles this evolution.

### 5.4 Link types (revised)

From the M1 work, keep:
- `HumanToCurrentAgent` (anchor → latest `KeyRotation`)
- `KeyRotationSupersededBy` (audit chain; critical for post-hoc supersession of fraudulent rotations)
- `AgentToKeyRotation` (reverse lookup)

Delete:
- `HumanToCurrentSeedCommitment`, `SeedCommitmentSupersededBy`, `SeedCommitmentToRequest` (dead with `RecoverySeedCommitment`)

Add (new):
- `RecoveryRequestToHumanityWitness` — connects a request to the attestations supporting it (for IntimateQuorum path)
- `RecoveryRequestToKeyStewardship` — connects a request to the stewardship it uses (for CryptographicQuorum path)

Keep existing (modernized `RecoveryRequest`):
- `HumanToRecoveryQuorumRequest` — renamed to `HumanToRecoveryRequest` (the modernized type's anchor)

### 5.5 Deleted entries (M1 work being reversed)

Three entry types shipped in M1 are removed from the `EntryTypes` enum and the module:
- `RecoverySeedCommitment` — superseded by `KeyStewardship`
- `HeldRecoveryShare` — no DHT entry needed; shares are local to holder devices, referenced via `KeyStewardship.key_shard_holders`
- `MyRecoveryAuthorization` — superseded by self-authored `HumanityWitness` from the authorizer's elohim

M1-cleanup milestone deletes these cleanly. No data migration needed — no entries have been committed on live networks.

---

## 6. Flows

### 6.1 Setup (no ritual)

**What happens:** Nothing ceremonial. Matthew sets `emergency_access_enabled = true` on relationships as part of ordinary relationship management. Once the flag is true on 2+ relationships, the `IntimateQuorum` authority path is live. He sees an ambient status in his profile:

> **Recovery protection: active**
> 3 people can help you recover if you lose your device.

No seed, no split, no wizard. The protocol's existing `HumanRelationship` entries are the setup.

**Elohim-driven optional hardening:** Matthew's elohim may, based on observing vulnerability signals, *suggest* provisioning `KeyStewardship`:

> *Your elohim thinks extra protection might be worth setting up. Interested?*

Declining is fine. Accepting runs a Shamir-split ritual (like the original 2026-04-21 M1 design). This is the **only** time Shamir enters the flow — by elohim recommendation for judged-vulnerable humans, not baseline.

### 6.2 Recovery (claimant side)

Identical structural flow to the 2026-04-21 design, with authority-path genericity:

1. New device browser → any federated doorway → "Recover access" button.
2. Enter handle/email → doorway resolves to `human_agent_pubkey`.
3. Rate-limit check + hashcash proof-of-work.
4. Doorway generates `new_agent_pubkey` for the claimant session.
5. Doorway commits `RecoveryRequest` with `proposed_authority = IntimateQuorum` by default.
6. Doorway publishes libp2p invitation on gossipsub topic `recovery.invitation.{request_hash}`.
7. Emergency contacts' devices receive invitation; their elohims evaluate; humans confirm.
8. On confirm: contact's device authors `HumanityWitness` entry attesting the recovery and commits to DHT.
9. Doorway watches for accumulating witnesses; when threshold met, commits `KeyRotation` with `RecoveryAuthority::IntimateQuorum { witness_hashes }` evidence.
10. Conductor worker pool installs imagodei cell with `new_agent_pubkey`.
11. Browser session receives hosted-cell handle. Matthew sees his network.

**Elohim's role at each contact:** receives invitation → specialist subagent spawned → reads local context (this holder's relationship with the claimant, historical patterns, anomaly signals) → produces assessment → presents ONE prompt to the holder → if authorized, contact's device authors `HumanityWitness` with `confidence` + `evidence_json`.

### 6.3 Defender flow (attack response)

1. Target's elohim observes suspicious `RecoveryRequest` (unexpected timing, pattern mismatch, concurrency with other requests, location anomaly).
2. **Defender specialist subagent spawned** with read access to the target's imagodei profile:
   - `Human` + `HumanRelationship` + prior `HumanityWitness` + `Attestation` + recent activity baseline
3. Specialist evaluates deviation → authors `IdentityAnomaly` with `deviation_score` + `evidence_json`.
4. If deviation exceeds threshold: specialist authors `IdentityFreeze` targeting the human. `IdentityFreeze` gets a new field `frozen_at_layer` recording the layer of the pending attack's `proposed_authority`.
5. `KeyRotation` validator now rejects same-or-lower-layer rotations (validator rule 5 in §5.2).
6. Specialist authors transparency entries:
   - `HumanityWitness` self-authored by the elohim (agent_type = "elohim") attesting on behalf of the human that the pending request is inconsistent with baseline
   - `IdentityChallenge` initiated against the suspicious request
7. **Elohim judgment** (not rigid rule) about escalation: recommend higher-authority rotation if attack persists, or recommend waiting for human surfacing, or recommend allowing original layer after window with extra scrutiny.
8. Resolution:
   - Human surfaces and affirms/denies → freeze resolved accordingly.
   - Higher-authority rotation proceeds → supersedes pending lower-authority attempt.
   - Freeze window expires without surface → elohim judgment recommends next step per context.

**Disclosure of specialist reasoning** governed by the target's collective constitution (qahal DNA). Some collectives may require full public disclosure of specialist reasoning; others may keep reasoning traces private to intimate circle and surface only the actions. Phase 2 provides disclosure-tier hooks on `IdentityAnomaly` + `IdentityChallenge`; qahal DNA defines policy.

### 6.4 Fast-path revocation (unchanged from 2026-04-21)

Three paths using existing primitives:

- **Self-revoke** — Human's surviving cell commits `KeyRevocation` directly.
- **Emergency-contact votes** — Contacts commit `RevocationVote` entries; quorum triggers `KeyRevocation`.
- **Atomic with rotation** — Successful `KeyRotation` triggers a paired `KeyRevocation` for the superseded agent.

No new entry types; uses existing `KeyRevocation` + `RevocationVote`.

### 6.5 Dissolution stub (cradle-to-grave marker)

**Phase 2 reserves the shape without implementing the flow.** When a human is deceased with no heirs, no active governance body, no surviving emergency contacts:

- Future implementation uses `RecoveryAuthority::NetworkWitness { purpose: Dissolution }`.
- `new_agent_pubkey` is a memorial-marker null agent. Concrete byte pattern is a constitutional-governance decision; the spec reserves the shape and defers the byte layout (candidates include 32 zero bytes or a well-known memorial-marker constant derived from the human's final `Agent` entry hash). The M1-cleanup milestone does not lock a value.
- Consequences: stewarded content returns to the commons; governance memberships close; pending stewardship obligations re-assigned by community.

**Phase 2 acceptance for dissolution stub:**
- `NetworkWitnessPurpose::Dissolution` enum variant defined
- Validator rejects it with message `"Dissolution flow: reserved for constitutional-governance design. Phase 2 does not implement — contact network-witness coordination for bereavement care."`
- Spec documents this as the marker for future work: **the protocol includes cradle-to-grave care; the grave is not an afterthought, it's a reserved shape.**

---

## 7. Federation Semantics (unchanged from 2026-04-21)

- Matthew initiates at any federated doorway.
- libp2p gossipsub invitations cross doorway boundaries naturally.
- `hosting_doorway_pubkey` binds one session to one doorway.
- Concurrent sessions resolve via first-to-quorum wins; later attempts fail validation on superseded state.
- Compromised doorway? Start over elsewhere; DHT is truth.
- Hosted-cell migration between doorways is Phase 3+ (via new `KeyRotation` to same agent).

Multi-doorway federation requires no new infrastructure — inherited from the doorway `/CLAUDE.md` federation model.

---

## 8. Elohim Defender Specialist Architecture

### 8.1 Specialist manifest

Elohim defense is not a single monolithic agent — it's a **specialist subagent** invoked for focused incidents, with a declared manifest:

```
Name: imagodei-defender-specialist
Triggers:
  - RecoveryRequest commit targeting a human this elohim represents
  - IdentityChallenge opened against a human this elohim represents
  - Anomalous pattern detected in target's activity/relationships
Input context:
  - target human's Human + HumanRelationship + recent HumanityWitness entries
  - RecoveryRequest under suspicion (if triggered by one)
  - recent IdentityAnomaly entries referencing the target
  - baseline behavior profile (derived from source-chain and DHT activity)
Output permissions:
  - May author IdentityAnomaly (type: "recovery-anomaly" | "behavioral-deviation")
  - May author IdentityFreeze (with frozen_at_layer set)
  - May author HumanityWitness self-authored on target's behalf
  - May author IdentityChallenge + ChallengeSupport
  - MAY NOT author KeyRotation (that's coordinator-only)
  - MAY NOT author KeyRevocation directly (revocation needs quorum or self-revoke)
Disclosure tier:
  - Set per qahal constitutional policy (public / intimate / layered)
  - Default: public (transparency-as-check)
```

The manifest is declared in elohim-agent configuration and enforced by rule 4.

### 8.2 Elohim-of-human binding (open question, deferred)

**Phase 2 assumes** the elohim-of-human binding is established via existing imagodei mechanisms — likely:
- An `Agent` entry with `agent_type = "elohim"` representing the elohim
- An `Attestation` from the human endorsing the elohim's pubkey as representative
- Or a `HumanRelationship` variant

This spec does not introduce a new binding primitive. If the existing mechanisms prove insufficient, a later spec addresses it. For Phase 2's defender flow, we assume the binding is resolvable.

### 8.3 Transparency invariant

All defender-specialist actions commit DHT entries — there is no private action surface for defense. **Hidden defense would be a back-door.** Disclosure of the specialist's *reasoning traces* may be tiered (public/intimate/private per governance), but the *actions* are always network-visible.

---

## 9. Rollout Milestones

Given the M1 work already shipped (commits `5e997cea..31564f04`), rollout begins with cleanup:

| Milestone | Scope | Acceptance |
|---|---|---|
| **M1-cleanup** | Delete `RecoverySeedCommitment`, `HeldRecoveryShare`, `MyRecoveryAuthorization` from integrity zome + their link types + their storage projections + their view types + their JSON schemas + their contract tests. Modernize legacy `RecoveryRequest` struct in place (replace fields per §5.3). Evolve `KeyRotation` to use `RecoveryAuthority` enum. Update storage projections + views + JSON schemas + contract tests accordingly. | DNA builds clean; schema contracts pass; no references to deleted types; legacy `RecoveryRequest` struct replaced; `KeyRotation` validator accepts both implemented variants, stub-rejects others |
| **M2** | Implement `IntimateQuorum` + `CryptographicQuorum` happy paths in the `KeyRotation` validator. Stub-reject the other three variants with clear Phase-2b messages. Add floor-check: `IdentityFreeze` with `frozen_at_layer` field (add if missing) halts same-or-lower-layer rotations. | Unit tests for each variant's happy path + stub rejection; integration test: freeze halts matching rotations |
| **M3** | Coordinator functions: `create_recovery_request` (populates `human_id` via `Agent` entry lookup + `required_witness_count` from emergency-access link count), `commit_key_rotation` (pre-commit gate: traverses `ActiveFreezes` anchor via `get_links`, delegates to `check_freeze_floor_rules` helper from M2, bails on blocker — owning the freeze-floor enforcement that HDI cannot provide in the validator), `submit_intimate_witness`. Post-commit signals for `RecoveryRequest` + `KeyRotation`. Doorway libp2p invitation flow on topic `recovery.invitation.{request_hash}`. | a2o: intimate-quorum end-to-end passes on shem cross-node topology; freeze-floor pre-commit gate rejects same-or-lower-layer rotations when an active freeze targets the human |
| **M4** | Fast-path revocation flows (self-revoke, contact-vote via existing `RevocationVote`, atomic-with-rotation). Uses existing `KeyRevocation` unchanged. | a2o: three revocation paths pass |
| **M5** | Defender specialist manifest + elohim-agent rule 4 extension. Anomaly → freeze → counter-challenge flow. Disclosure-tier hooks on defensive entries. Frontend: `RecoveryCoordinatorService` real endpoints + holder-side prompt UI + defender-visible status ("your elohim is actively defending you"). Hosted-cell bootstrap + browser session handoff. | a2o: attack-triggered-defense end-to-end; grandma-standard UX review |
| **M6** | Anti-lockout audit + red-team. Scenario suite in `genesis/a2o/features/auth/recovery/anti-lockout/`. Every failure mode documented with a community-mediated restoration path (or explicit deferral to constitutional-governance layer). Shem cross-node full acceptance. | All anti-lockout scenarios documented; audit gate passes |

### M1-cleanup code delta summary

**Delete:**
- `recovery_v2.rs` entry types: `RecoverySeedCommitment`, `HeldRecoveryShare`, `MyRecoveryAuthorization`
- Link types: `HumanToCurrentSeedCommitment`, `SeedCommitmentSupersededBy`, `SeedCommitmentToRequest`
- Migrations/tables: `recovery_seed_commitments` (storage projection table)
- Views: `RecoverySeedCommitmentView`
- JSON schemas: `recovery-seed-commitment.schema.json`
- Generated TypeScript: `RecoverySeedCommitmentView.ts` (across consumer dirs)
- Coordinator function: `commit_recovery_seed`
- Signal variants: `SeedCommitmentCreated`, `SeedCommitmentSuperseded` in `RecoveryV2Signal`

**Evolve:**
- `recovery_v2.rs` `KeyRotation` struct: replace `quorum_signature`/`seed_commitment_hash` with `authority: RecoveryAuthority` enum
- `recovery_v2.rs` `RecoveryQuorumRequest` → merged into modernized top-level `RecoveryRequest` (replacing legacy struct in `lib.rs`)
- Validator `validate_key_rotation` rewritten per §5.2
- Storage projection `key_rotations` table: add `authority_kind` column + variant-specific columns (witness hashes as JSON array, challenge_hash, grant_hash, resolution_hash, stewardship_hash, quorum_signature)
- Storage projection `recovery_quorum_requests` table: renamed to `recovery_requests`, columns adjusted to modernized struct
- Views `KeyRotationView`, `RecoveryQuorumRequestView`: renamed/restructured
- JSON schemas: `key-rotation.schema.json` + `recovery-quorum-request.schema.json` updated → renamed to `recovery-request.schema.json`
- Schema contract tests: retain, update struct instances to match new shapes

**Keep as-is:**
- `HeldRecoveryShare`-related storage projections — **none to keep**, all deleted
- vsss-rs + ed25519-dalek deps — keep both; `CryptographicQuorum` still uses them

### Writing-plans handoff

M1-cleanup has clear enough scope to spin a focused implementation plan after spec approval. M2–M6 get their own plans in sequence.

---

## 10. Anti-Lockout Audit

**Every failure mode must have at least one community-mediated path to eventual restoration.** The audit scenario suite covers:

| # | Scenario | Required path |
|---|---|---|
| 1 | All emergency contacts dead | `GovernanceAct` via qahal, or `NetworkWitness` |
| 2 | Human dead, no heirs, no qahal | `NetworkWitness` with `Dissolution` purpose (stubbed Phase 2; documented) |
| 3 | Stewardship grant orphaned (all stewards gone) | `NetworkWitness` re-grant or qahal reconstitution |
| 4 | All doorways the human registered with compromised | Start fresh at any unregistered federated doorway |
| 5 | Elohim defense false-positive freeze | Higher-authority rotation supersedes; human surfaces via any channel |
| 6 | Crypto-only preferences + all shares lost | Fall back to graduated authority — `IntimateQuorum` or escalation |
| 7 | Network partition during recovery | `KeyRotationSupersededBy` retires fraudulent rotations that happened during partition once partition heals |
| 8 | User demands "absolute privacy, no community recovery" | `NetworkWitness` still available (high friction); user cannot opt out of floor |
| 9 | Sustained targeted attack across all layers | Floor rises per elohim judgment; network-witness escalation available |
| 10 | Human under duress self-authorizes against interest | Elohim-defender specialist authors counter-attestation; post-hoc supersession possible |
| 11 | Stewarded ward blocked by abusive steward | `StewardshipAppeal` + governance reconstitution; ward's elohim as counsel |
| 12 | Multi-generational identity (ward grows into adult) | Gradual authority transfer via supersession chain; no reset |

Scenarios (1–6, 9–12) get gherkin `.feature` files in `genesis/a2o/features/auth/recovery/anti-lockout/`. Scenarios 7, 8 get design-documentation entries (some stubs until constitutional-governance landing).

**The audit fails** if any new recovery feature introduces a failure mode without a restoration path. Anti-lockout is a gate, not an optional review.

---

## 11. Testing Strategy

Unchanged in shape from 2026-04-21 (§6.3), adapted to revised data model:

- **Unit (Rust):** validator rule coverage for each `RecoveryAuthority` variant; floor-check against active freezes; Ed25519 signature verification for `CryptographicQuorum`; entry-type registration.
- **Integration (Rust, multi-node):** end-to-end intimate-quorum recovery; elohim defender triggering a freeze; higher-authority supersession; multi-doorway concurrent requests.
- **Frontend (Vitest):** `RecoveryCoordinatorService` updates; holder-side prompt UI with elohim assessment; defender-visible status panel; hosted-cell landing.
- **A2O (Gherkin):** `genesis/a2o/features/auth/recovery/` covers setup (ambient), recovery happy paths, defender flow, revocation paths, multi-doorway federation. **New:** `anti-lockout/` subdirectory for audit scenarios.
- **Shem cross-node acceptance:** per the existing topology memory — household cluster (Matthew/Jessica/Terrance) + shem (everyone else + shem's doorway). Recovery demo: Matthew loses laptop → recovers via shem's doorway → intimate circle authorizes → lands in hosted cell.

---

## 12. Dependencies & Prerequisites

- `vsss-rs` and `ed25519-dalek` already added (M1). Retain for `CryptographicQuorum`.
- `IdentityFreeze` struct may need a new `frozen_at_layer` field. Verify existing struct and patch if missing.
- Elohim-of-human binding assumption: existing imagodei primitives cover. No new binding work in this spec.
- Cross-DNA references (qahal/mishpat `resolution_hash` for `GovernanceAct`, network-witness for `NetworkWitness`) stubbed in Phase 2; full validation lands later.
- Shem acceptance canvas ready (personas deployed per household + shem topology).

---

## 13. Retrospective Appendix — What the 2026-04-21 Spec Got Wrong

The 2026-04-21 spec began with the right intuition (social recovery through peer-held material, blind-proxy doorway, ambient UX) but committed three errors, surfaced through a morning-after retrospective conversation:

### 13.1 Reinvention of existing primitives

The spec created `RecoverySeedCommitment` when the protocol already has `KeyStewardship` (Shamir threshold configuration with holder list, thresholds, commitment hash, policy). `HeldRecoveryShare` and `MyRecoveryAuthorization` similarly duplicated existing patterns. The correct move was to compose existing primitives, not add parallel ones.

### 13.2 Crypto-first framing embedded idealistic naivete

Treating Shamir secret sharing as the baseline — with optional fallback to attestation — inverted the protocol's trust model. The Elohim Protocol is explicitly *against* the "one key for dragon horde" / "neurotic self-sovereign" paradigm that produces James-Howells-in-the-UK-dump scenarios. Graduated community authority is *primary*; crypto is *optional accelerant* for humans the elohim judges vulnerable. The original spec had this backwards.

### 13.3 Missing anti-lockout commitment

The 2026-04-21 spec acknowledged that colluding-quorum attacks are a fundamental social-recovery limitation but did not explicitly commit to "absolute lockout is impossible." Without that commitment, the design could drift toward paths where a user's cryptographic preferences produce irrecoverable states — exactly the failure the protocol exists to prevent.

### 13.4 Missing elohim-as-counsel

The 2026-04-21 spec treated the elohim as an oracle (producing confidence assessments for holders to act on) rather than as **counsel with first-class standing** on behalf of the human. When a human is silenced or under duress, the elohim doesn't merely advise — it represents. Recovery design must honor that standing with defensive DHT primitives and the elohim's capacity to act in the human's interest even against their current-moment preferences.

### 13.5 Conclusion

The 2026-04-21 spec's setup ritual, graduated UX, doorway-blind-proxy, and multi-doorway federation remain correct. The revision prunes the crypto-first frame, adopts graduated-authority as primary, adds elohim-defense as first-class, and explicitly commits to anti-lockout. The implementation work already shipped (M1) contained three deletable types + one genuine-gap primitive — the gap (`KeyRotation`) survives with a richer evidence model (`RecoveryAuthority` enum).

---

## 14. Revision History

| Date | Change | Author |
|---|---|---|
| 2026-04-22 | Revised spec supersedes 2026-04-21 original. Graduated-authority, elohim-as-counsel, anti-lockout committed. M1-cleanup milestone added. | Matthew Dowell |
