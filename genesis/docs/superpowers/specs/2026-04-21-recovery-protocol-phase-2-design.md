# Recovery Protocol Phase 2 — Socially Derived Identity Recovery

**Status:** SUPERSEDED (2026-04-22)
**Superseded by:** `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md`
**Superseded because:** A retrospective after the M1 data-model work surfaced that this spec (a) reinvented existing protocol primitives (`RecoverySeedCommitment` duplicated `KeyStewardship`, etc.), (b) embedded a crypto-first framing incompatible with the Elohim Protocol's graduated-community-authority vision, and (c) lacked an explicit anti-lockout commitment. The revised spec is the current authority. This file is kept for historical reference and to show the journey — see §13 of the revised spec for the retrospective analysis.

**Date:** 2026-04-21
**Owner:** Matthew Dowell
**Supersedes for Phase 2 scope:** `doorway/doorway-service/RECOVERY-PROTOCOL.md` (Jan 2026), `doorway/doorway-service/RECOVERY-SPRINT-PLAN.md` (Jan 2026)
**Companion specs:** `genesis/docs/superpowers/specs/2026-04-19-gate-challenge-and-indemnification-design.md`

---

## 1. Vision and Purpose

### 1.1 The bar

Grandma must be able to trust that her family photos, her identity, and her perspective on the network are safe. Not "safe if she memorizes a 24-word phrase." Not "safe if she keeps her recovery email secure." Not "safe if a corporation doesn't have a bad day." **Safe because the people she trusts can help her recover when she needs it, at the same convenience any trillion-dollar tech company provides — via social trust instead of corporate custody.**

### 1.2 The protocol commitment

Identity and key recovery in the Elohim Protocol are **socially derived**. No single party — not any doorway, not any peer, not the protocol itself — can recover a human's identity alone. Recovery requires a quorum of the human's emergency contacts, each participating through their own elohim-agent, each acting as an accountable witness to the claimant's identity.

This is the protocol's answer to the fundamental failure mode of custodial systems: if any single point (corporation, seed phrase, device) can grant access, that point can also fail, be compromised, or be weaponized. Social quorum spreads the authority across people who actually know you.

### 1.3 Elohims as accountable guardians

The elohim-agents of participating peers are not passive authorizers. They are **accountable guardians** — carrying forward the wisest judgment of the network's best-self at machine speed, intervening in harmful dynamics faster than human networks can coordinate. They slow duress, surface coercion patterns, flag concurrent-attack signals, and require out-of-band confirmation when discernment demands it. This is what makes social recovery trustworthy: the humans make decisions, but an impartial witness at each step keeps the decisions honest.

### 1.4 Phase scope

Phase 2 of the Recovery Protocol delivers the **Recovery Request Flow** — the DHT primitives, libp2p coordination, doorway session management, and hosted-conductor bootstrap required for a human to regain network access from a new device through social quorum. Phase 1 (shard tracking in node-registry DNA) is complete and provides the foundation for Phase 3's content reconstruction.

Phase 2 produces a running hosted cell on the recovering doorway, bound to a new agent key, recognized by the DHT as the recovering human's current authorized agent. Matthew can see his network again. Graduation to his own replacement device, content shard reassembly, and related concerns are out of scope.

---

## 2. Design Principles

1. **Socially derived, not custodially derived.** The authority to recover is held by the claimant's people, distributed cryptographically, not centralized at any operator.

2. **Seed-derived, not key-preserving.** What is shared among contacts is a *recovery seed*, distinct from the everyday agent key. Recovery authorizes a fresh agent key via `KeyRotation`; the lost device's key is revoked, not preserved. Stolen devices do not mean permanent compromise.

3. **Blind doorway.** The doorway coordinates recovery but never holds shares at rest and never holds the reassembled seed beyond the milliseconds needed to commit `KeyRotation`. Compromise of a doorway cannot produce impersonation.

4. **Hidden share-holders.** The DHT records no list of who holds shares. An adversary reading the DHT learns a human is recoverable, never who would vote.

5. **DHT narrow to integrity.** Only integrity-critical facts (seed commitment, recovery request, key rotation) hit the DHT. Authorizations, challenge content, and session state flow through libp2p and doorway-local storage.

6. **Elohim as accountable guardian.** Every recovery touchpoint has an elohim-agent intervention point. Elohims evaluate coercion patterns, concurrency attacks, anomaly signals, and surface them with graduated responses (surface → pause → require out-of-band → alert network). Design against weaponization, not just external attack.

7. **Ambient over interruptive.** The user never sees seed phrases, shares, key material, or cryptographic terms. Setup happens automatically when emergency contacts reach threshold; status is surfaced in a profile panel, never pushed. Recovery UX feels like "sign in on a new device with help from your people."

8. **Graduated capability for stewarded humans.** Children, seniors with cognitive decline, IDD wards, and humans in custodial contexts (probation) recover through their stewards under elohim oversight that enforces "just" use of custodial authority — with appeal paths for ward voice.

---

## 3. Scope

### 3.1 In scope (Phase 2)

- Three new DHT entry types in imagodei DNA: `RecoverySeedCommitment`, `RecoveryRequest`, `KeyRotation`
- Leverage of existing imagodei entry types: `KeyRevocation`, `RevocationVote`, `StewardshipGrant`, `DevicePolicy`, `StewardshipAppeal`, `HumanRelationship`
- Private source-chain entry type: `HeldRecoveryShare`, optional `MyRecoveryAuthorization`
- Coordinator zome functions for commit, request, authorize, rotate, revoke flows
- libp2p protocols: `/elohim/recovery-provisioning/1.0.0` (share distribution) and `/elohim/recovery/1.0.0` (authorization + share release)
- Doorway-side `recovery_sessions` Diesel/SQLite table, in-memory seed reassembly cache, libp2p gossipsub recovery invitation broadcasting
- Shamir Secret Sharing via `vsss-rs` crate (verifiable, Ed25519-compatible)
- Doorway conductor worker pool wiring: install hosted imagodei cell with new agent key post-rotation
- Frontend: wire existing `RecoveryCoordinatorService` to real endpoints; new holder-side authorization notification UI
- elohim-agent discernment rule 4 handler: produce authorization assessment for holder
- Stewarded recovery: `RecoveryRequest.recovery_mode` discriminator, validation branch for steward-authority path (baseline primitives, UX polish in Phase 2b)
- Fast-path revocation: three paths (self-revoke, emergency-contact vote, atomic-with-rotation)
- Multi-doorway federation semantics (doorway-agnostic libp2p, cross-doorway invitation propagation)
- Elohim guardian intervention points at each stage of the flow
- Red-team deliverable covering 14 enumerated threat scenarios
- a2o scenarios covering happy paths, edge cases, and red-team scenarios
- Shem canvas acceptance: cross-node topology recovery demo (household cluster + shem)

### 3.2 Out of Phase 2

- Content-encryption key recovery and content shard reassembly (Phase 3)
- Cross-DNA signal handler to re-key `CustodianAssignment` references from old to new agent (Phase 3 consumer)
- Hosted-cell migration between doorways (Phase 3+)
- Doorway continuity after a doorway operator's hardware loss (operational concern, separate from identity)
- Verifiable threshold signing replacing plaintext seed reassembly (Phase 5+; requires upstream crypto work)
- Duress canary / silent-freeze for initiating recovery under coercion (Phase 5+)
- Mix-net / onion-routed libp2p traffic for social graph privacy (Phase 5+)
- Recovery drills, network recovery health dashboard (Phase 5, per the original sprint plan)
- Stewarded recovery UX polish + full a2o coverage (Phase 2b, follow-up)

### 3.3 Relationship to existing design documents

The Jan 2026 `doorway/doorway-service/RECOVERY-PROTOCOL.md` and `RECOVERY-SPRINT-PLAN.md` established the four-layer recovery model and defined the original phase boundaries. Phase 1 (shard tracking) was built against that design and is complete. This spec **supersedes** those documents for Phase 2 scope, refactoring to reflect architectural evolution:

- **DHT-narrow architecture** (authorizations move off DHT; only seed commitment, request, rotation remain)
- **Elohim-agent sense-and-respond** (discernment rule 4 mediates authorization decisions rather than app-only UI)
- **Manifest-driven doorway routes** (no per-domain proxy code; elohim-storage exposes recovery routes via manifest where possible)
- **Household-first resilience** (household stewardship and multi-doorway federation are first-class)
- **Seed-derived recovery** (Shamir-split seed, fresh agent key, revocable — replacing attestation-chain model)
- **Stewarded recovery path** (integrates with StewardshipGrant for children/seniors/IDD/custody cases)
- **Hidden share-holders** (privacy hardening over original public-holder design)

The Jan 2026 documents remain valid as historical context and for Phases 3-5, which still operate on their model.

---

## 4. Architecture Overview

### 4.1 Two flows

```
SETUP (calm moment, Matthew present, has working device)
  Matthew's device
    └─> generate recovery_seed (random 256 bits)
    └─> Shamir-split: (seed_private_half, N, M) → [share_1 ... share_M]
        via vsss-rs (verifiable, Ed25519-compatible)
    └─> commit RecoverySeedCommitment on DHT (public half only, no holder list)
    └─> for each emergency contact:
          libp2p /elohim/recovery-provisioning/1.0.0 → holder's device
          holder's device stores HeldRecoveryShare (private source-chain entry)
          holder's elohim subscribes to gossipsub topic:
            recovery.invitation.{commitment_H}
    └─> seed zeroized; Matthew's device retains nothing beyond the DHT commitment


RECOVERY (crisis moment, Matthew on new device, old device gone)
  new_device (browser) → doorway D
    └─> doorway commits RecoveryRequest on DHT
    └─> doorway publishes libp2p gossipsub:
          topic: recovery.invitation.{commitment_H}
          payload: { request_hash, session_pubkey, encrypted claimant_answers,
                     hosting_doorway_url }
          ──> subscribed holders receive
              ├─> elohim evaluates (challenge plausibility + anomaly +
              │    relationship signals) → confidence_tier
              ├─> human contact confirms (single prompt)
              └─> if authorize: holder's device sends
                   libp2p /elohim/recovery/1.0.0:
                     { authorization_payload, encrypted_share }
    └─> doorway collects N distinct shares in-memory
         reassembles seed_private via vsss-rs
         signs (new_agent_pubkey || request_hash) with seed_private
         commits KeyRotation on DHT
         commits KeyRevocation for superseded agent (defense-in-depth)
         zeroizes seed material
    └─> doorway's conductor worker pool installs imagodei cell
        with new_agent_pubkey
    └─> new_device receives hosted-cell session (SSE + JWT handoff)
    └─> Matthew sees his network
```

### 4.2 Layer map

| Layer | What lives here | What does NOT live here |
|---|---|---|
| **DHT (imagodei DNA)** | `RecoverySeedCommitment` (no holder list), `RecoveryRequest`, `KeyRotation`; existing `KeyRevocation`, `RevocationVote` | Shares, authorizations, session state, reassembled seed, holder identities |
| **Private source chains (holder-side)** | `HeldRecoveryShare` (mandatory), `MyRecoveryAuthorization` (optional) | Matthew's own recovery seed (held only at setup time, zeroized after split) |
| **libp2p (gossipsub + direct)** | Recovery invitation broadcasts; authorization + share delivery messages | Persisted state (all messages are transient) |
| **Doorway (Diesel/SQLite)** | `recovery_sessions` table (operational), schema alongside existing `oauth_session`, `api_key`, `host`, `user` | Shares, seed material, holder identity records |
| **Doorway (in-memory bounded cache)** | Reassembled seed during the `reassembling → rotating` transition — modeled on `custodial_keys/cache.rs`, zeroized on completion, session abort, or TTL expiry | Persistent key material; anything that should survive process restart |
| **Doorway (MongoDB)** | *Not used for recovery.* Reserved for DHT projection cache. | — |
| **Doorway (conductor worker pool)** | Hosted imagodei cell installation for recovered user, reusing existing pool | — |
| **Frontend (Angular)** | `RecoveryCoordinatorService` (existing, updated to real endpoints); new holder-side authorization notification component | Any key material (hosted-cell keys stay server-side until graduation) |

### 4.3 Reuse of existing infrastructure

| Component | Source | Role in recovery |
|---|---|---|
| `KeyRevocation` entry type | imagodei (Phase 2 Network-Attested Identity) | Revoke superseded agent keys (defense-in-depth + fast-path) |
| `RevocationVote` entry type | imagodei | Emergency-contact votes for fast-path revocation |
| `StewardshipGrant`, `DevicePolicy`, `PolicyInheritance`, `StewardshipAppeal` | imagodei | Stewarded recovery path (children, seniors, IDD, custody) |
| `HumanRelationship.emergency_access_enabled` | imagodei | Identify emergency contacts |
| Conductor worker pool | `doorway/doorway-service/src/worker/{conductor,pool,processor,zome_call}.rs` | Host recovered user's cell |
| `custodial_keys/cache.rs` pattern | doorway-service | In-memory bounded cache for ephemeral seed material |
| Diesel session table pattern (`oauth_session`) | doorway-service | `recovery_sessions` schema template |
| libp2p protocol stack | elohim-storage | Transport for share distribution and authorization |
| MongoDB projection cache | doorway-service | DHT entry projections (seed commitment, key rotation lookups) |
| SSE streaming pattern | doorway-service | Progress updates to claimant browser during session |
| elohim-agent discernment rule 4 | elohim-agent (Apr 18, 2026 commit `f9e68b4c`) | Authorization assessment for holders |
| `RecoveryCoordinatorService` | `app/elohim-app/src/app/imagodei/services/recovery-coordinator.service.ts` (98.7% coverage) | Wire to real endpoints |

---

## 5. Data Model

### 5.1 DHT entry types (imagodei DNA, 3 new — 28 → 31)

#### `RecoverySeedCommitment`

```rust
#[hdk_entry_helper]
pub struct RecoverySeedCommitment {
    /// The human this commitment protects
    pub human_agent_pubkey: AgentPubKey,
    /// Public half of the Ed25519 keypair derived from the recovery seed.
    /// The private half is Shamir-split across emergency contacts, never on DHT.
    pub seed_public_half: [u8; 32],
    /// Quorum threshold — minimum shares needed to reassemble
    pub threshold_n: u8,
    /// Total shares distributed (informational; share-holders are NOT listed)
    pub total_m: u8,
    /// Random nonce to distinguish reshares
    pub commitment_nonce: [u8; 16],
    pub created_at: Timestamp,
}
```

**Validation rules:**
- `threshold_n >= 2` and `threshold_n <= total_m`
- `total_m >= 2` and `total_m <= 16`
- `human_agent_pubkey == author_agent_pubkey` (only the Human can commit their own seed)
- `seed_public_half` is a well-formed Ed25519 public key

**Links:**
- `Anchor("recovery_seed_commitment:{human_pubkey}") → RecoverySeedCommitment`
- `SeedCommitmentSupersededBy: old_commitment → new_commitment` (reshare chain)

**Privacy invariant:** No field reveals holder identities or the social graph. An attacker reading the DHT learns only that a human is recoverable and the aggregate (N, M) structure.

#### `RecoveryRequest`

```rust
#[hdk_entry_helper]
pub struct RecoveryRequest {
    /// Human whose identity is being recovered
    pub human_agent_pubkey: AgentPubKey,
    /// The seed commitment this request targets (must not be superseded)
    pub seed_commitment_hash: EntryHash,
    /// Proposed new agent pubkey
    pub new_agent_pubkey: AgentPubKey,
    /// Doorway hosting this session
    pub hosting_doorway_pubkey: AgentPubKey,
    /// Mode discriminator — Phase 2 supports Normal; Stewarded added in Phase 2b
    pub recovery_mode: RecoveryMode,
    /// Nonce for multiple attempts
    pub request_nonce: [u8; 16],
    pub created_at: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RecoveryMode {
    Normal,
    Stewarded { grant_hash: EntryHash },
}
```

**Validation rules:**
- `seed_commitment_hash` must resolve to a non-superseded `RecoverySeedCommitment`
- `human_agent_pubkey` must match the commitment's
- Author is the hosting doorway's agent (doorway submits on behalf of claimant's new device; no authorization implied by authorship — authority comes from `KeyRotation`'s quorum signature)
- `new_agent_pubkey` must not already be the current agent of any other Human
- If `recovery_mode == Stewarded`, `grant_hash` must resolve to an active `StewardshipGrant` for `human_agent_pubkey`

**Links:**
- `Anchor("recovery_request:{human_pubkey}") → RecoveryRequest`
- `SeedCommitmentToRequest: seed_commitment → request`

#### `KeyRotation`

```rust
#[hdk_entry_helper]
pub struct KeyRotation {
    /// Human whose agent key is rotating
    pub human_agent_pubkey: AgentPubKey,
    /// The new authorized agent
    pub new_agent_pubkey: AgentPubKey,
    /// The superseded agent key (lost device)
    pub superseded_agent_pubkey: AgentPubKey,
    /// The seed commitment whose quorum authorizes this rotation
    pub seed_commitment_hash: EntryHash,
    /// The request this rotation fulfills
    pub recovery_request_hash: EntryHash,
    /// Ed25519 signature:
    ///   seed_private_half.sign(new_agent_pubkey || recovery_request_hash)
    pub quorum_signature: [u8; 64],
    pub rotated_at: Timestamp,
}
```

**Validation rules (the hardest-working validator in the spec):**

1. `recovery_request_hash` must resolve to a `RecoveryRequest` entry.
2. The request's `new_agent_pubkey` matches this rotation's `new_agent_pubkey`.
3. The request's `seed_commitment_hash` matches this rotation's `seed_commitment_hash`.
4. `seed_commitment_hash` must resolve to a non-superseded `RecoverySeedCommitment`.
5. `human_agent_pubkey` matches across request, commitment, and rotation.
6. **For `recovery_mode == Normal`:** `quorum_signature` verifies as `seed_public_half.verify(new_agent_pubkey || recovery_request_hash, quorum_signature)`. This is the cryptographic proof of quorum — reassembling the seed requires ≥ N shares, which requires ≥ N authorizations; the signature is what they collectively produce.
7. **For `recovery_mode == Stewarded`:** `quorum_signature` verifies against the steward authority defined in the referenced `StewardshipGrant`. Threshold, steward set, and ward-consent semantics are carried in the grant and its `PolicyInheritance` chain. (Full stewarded-path validation deferred to Phase 2b; Phase 2 validates the Normal branch end-to-end and stub-rejects Stewarded until Phase 2b lands.)
8. `superseded_agent_pubkey` must be the Human's current agent (as resolved by the latest preceding `KeyRotation` or initial `Agent` entry).
9. Rate limit: at most one non-superseded `KeyRotation` per `RecoveryRequest`.

**Links:**
- `Anchor("current_agent:{human_pubkey}") → KeyRotation` (latest = current)
- `KeyRotationSupersededBy: old_rotation → new_rotation` (audit chain)
- `AgentToKeyRotation: new_agent → rotation` (reverse lookup)

### 5.2 Private source-chain entries (Category B)

#### `HeldRecoveryShare` (holder-side, mandatory)

```rust
#[hdk_entry_helper]
pub struct HeldRecoveryShare {
    /// Which seed commitment this share belongs to
    pub seed_commitment_hash: EntryHash,
    /// Share ciphertext, encrypted under holder's own agent key for at-rest protection
    pub encrypted_share: Vec<u8>,
    /// Friendly label for holder's UI: "Share for Matthew"
    pub relationship_label: String,
    /// The human being protected
    pub protected_human_pubkey: AgentPubKey,
    pub received_at: Timestamp,
}
```

Private to the holder. Never on DHT. At-rest encryption under the holder's agent key means a stolen holder device must also leak the agent key before the share is usable.

#### `MyRecoveryAuthorization` (holder-side, optional)

```rust
#[hdk_entry_helper]
pub struct MyRecoveryAuthorization {
    pub recovery_request_hash: EntryHash,
    pub protected_human_pubkey: AgentPubKey,
    pub authorized_at: Timestamp,
    pub elohim_confidence_tier: ConfidenceTier,
    /// Brief reasoning the elohim captured for the holder's own records
    pub reasoning_summary: String,
}

pub enum ConfidenceTier {
    None,
    Light,
    Deep,
    Constitutional,
}
```

Optional. Holders can disable private-audit storage if preferred. Confidence tier taxonomy mirrors the existing gate tiers per the gate-challenge spec.

### 5.3 Operational state (doorway-side, Category C)

#### `recovery_sessions` (Diesel/SQLite)

```sql
-- Source of truth: local (operational).
-- Reconstructible from DHT recovery_requests if lost.
CREATE TABLE recovery_sessions (
    id                       TEXT PRIMARY KEY,          -- UUID
    recovery_request_hash    TEXT NOT NULL,             -- DHT EntryHash as hex
    human_agent_pubkey       TEXT NOT NULL,
    new_agent_pubkey         TEXT NOT NULL,
    hosting_doorway_pubkey   TEXT NOT NULL,
    recovery_mode            TEXT NOT NULL,             -- 'normal' | 'stewarded'
    state                    TEXT NOT NULL,             -- 'pending' | 'collecting' |
                                                        -- 'reassembling' | 'rotating' |
                                                        -- 'hosting' | 'complete' |
                                                        -- 'failed' | 'aborted'
    shares_received_count    INTEGER NOT NULL DEFAULT 0,
    threshold_n              INTEGER NOT NULL,
    total_m                  INTEGER NOT NULL,
    hosted_cell_id           TEXT,                       -- populated when cell installed
    expires_at               TIMESTAMP NOT NULL,         -- TTL (default 24h)
    created_at               TIMESTAMP NOT NULL,
    updated_at               TIMESTAMP NOT NULL
);
CREATE INDEX idx_recovery_sessions_request ON recovery_sessions(recovery_request_hash);
CREATE INDEX idx_recovery_sessions_state ON recovery_sessions(state);
CREATE INDEX idx_recovery_sessions_expires ON recovery_sessions(expires_at);
```

#### Ephemeral seed material (in-memory)

Modeled on `doorway/doorway-service/src/custodial_keys/cache.rs`:

```rust
pub struct RecoverySeedCache {
    sessions: DashMap<SessionId, SeedAssemblyState>,
    max_entries: usize,  // bounded
}

pub struct SeedAssemblyState {
    shares: HashMap<AgentPubKey, VssShare>,  // distinct holders only
    session_x25519_private: SessionPrivateKey,  // for share decryption
    expires_at: Instant,
    // Zeroize derive ensures Drop wipes memory
}

impl Drop for SeedAssemblyState {
    // Zeroize shares and session key on drop
}
```

Zeroized on: successful `KeyRotation` commit, session `failed`/`aborted`, TTL expiry, or process shutdown.

### 5.4 Storage projections (elohim-storage)

Following existing conventions (`views.rs` → `#[derive(TS)]` → TypeScript codegen):

| Table | Source of truth | `dht_anchor_hash` | Purpose |
|---|---|---|---|
| `recovery_seed_commitments` | DHT | yes | Query "does Matthew have a recovery commitment?" for UI readiness indicators |
| `recovery_requests` | DHT | yes | Query "active request for this human?" (Phase 5 alerting, and validator lookups) |
| `key_rotations` | DHT | yes | Resolve "what is Matthew's current agent key?" across the protocol |

**Authorizations are NOT projected** — they never hit the DHT.

### 5.5 Link taxonomy

| Link | Source | Target | Purpose |
|---|---|---|---|
| `HumanToCurrentSeedCommitment` | Anchor(human_pubkey) | `RecoverySeedCommitment` | Find current commitment |
| `SeedCommitmentSupersededBy` | old `RecoverySeedCommitment` | new `RecoverySeedCommitment` | Reshare chain |
| `SeedCommitmentToRequest` | `RecoverySeedCommitment` | `RecoveryRequest` | Find pending requests |
| `HumanToCurrentAgent` | Anchor(human_pubkey) | `KeyRotation` | "Who is the current agent?" (protocol-wide dependency) |
| `KeyRotationSupersededBy` | old `KeyRotation` | new `KeyRotation` | Key history |
| `AgentToKeyRotation` | new_agent | `KeyRotation` | Reverse lookup: "this agent was authorized by this rotation" |

---

## 6. Setup Flow (Provisioning)

### 6.1 User-facing framing (grandma standard)

Setup is **ambient** — it happens automatically when Matthew's emergency-access relationships reach threshold, not via a dedicated ritual. His profile's Recovery Protection panel shows a passive status:

> **Recovery protection: active**
> 5 people are helping protect your account.
> *Tap to review or change.*

A detail view lists contacts with their status (share delivered, pending, unreachable). Advanced controls (threshold adjustment, explicit reshare) are available but not required.

The first time Matthew flips `emergency_access_enabled` on his Nth `HumanRelationship` where N = 2 (minimum viable), the elohim-agent triggers provisioning automatically with sensible defaults (threshold = majority). No setup page required. No notifications. The status panel lights up after completion.

### 6.2 Technical sequence

```
Matthew's device                                Each chosen contact's device
────────────────                                ─────────────────────────────

[1] Emergency-access relationships reach
    threshold (default M=2) OR Matthew
    explicitly adjusts via profile panel
[2] Matthew's elohim-agent confirms sanity:
    - contacts reachable within last 30 days
    - not all in same household (diversity
      warning, not block)
    - N, M chosen per defaults or explicit

[3] Generate recovery_seed (rand::rngs::OsRng,
    256 bits)
[4] Derive Ed25519 keypair from seed;
    seed_public_half → DHT,
    seed_private_half → split

[5] Shamir-split via vsss-rs:
    (seed_private_half, N, M) → [share_1..share_M]
    Each share carries VSS verification commitments

[6] Commit RecoverySeedCommitment on DHT:
      { human_agent_pubkey,
        seed_public_half,
        threshold_n, total_m,
        commitment_nonce,
        created_at }
    → entry_hash commitment_H

[7] For each contact c_i in [1..M]:
      Encrypt share_i under c_i's agent pubkey
      (X25519-ECDH → ChaCha20-Poly1305 sealed-box)
      Send via libp2p direct:
      /elohim/recovery-provisioning/1.0.0
      → { commitment_hash: commitment_H,
          encrypted_share, relationship_label,
          protected_human_pubkey }
                                                ──────────►  [8] Receive share delivery
                                                             [9] Decrypt with own agent key
                                                             [10] Holder's elohim asks the
                                                                  human ONCE:
                                                                  "Matthew is setting up
                                                                   account recovery.
                                                                   Accept responsibility?"
                                                             [11] On accept:
                                                                  - Commit HeldRecoveryShare
                                                                    (private source-chain)
                                                                    with at-rest encryption
                                                                  - Subscribe to gossipsub
                                                                    topic:
                                                                    recovery.invitation.{H}
                                                             [12] Send libp2p ACK

                                                ◄──────────
[13] Track ACKs; retry via libp2p
     store-and-forward for unreachable contacts

[14] After all ACKs (or 72h timeout per contact):
     - zeroize seed_private_half, all shares,
       ephemeral Ed25519 private key
     - Update profile panel status ambiently
```

### 6.3 Security details

- **Seed generation on Matthew's device only** — never touches a server, never on DHT, zeroized after split.
- **Per-holder transport encryption** — sealed-box construction using holder's agent pubkey. Compromised libp2p transport does not expose shares.
- **Per-holder at-rest encryption** — holder's device re-encrypts the share under the holder's own agent key before committing to source chain.
- **Refusal handling** — if a contact refuses at step [10], they're excluded from the accepted count. If accepted count drops below M, Matthew's elohim surfaces ambient prompt: "not enough contacts accepted; we'll add more when you enable emergency access on additional relationships."
- **Store-and-forward for offline holders** — libp2p message queuing (existing protocol stack). Contacts offline >72h are flagged in the status panel as "unreachable — offer protection on their next sign-in."

### 6.4 Reshare (elohim-stewarded)

Matthew's elohim-agent watches for emergency-contact drift:
- `HumanRelationship.emergency_access_enabled` flipped or entry deleted
- New relationship labeled family/intimate with emergency access enabled
- A contact's agent key rotates via their own recovery (old shares inaccessible)

On drift, elohim surfaces calmly in the profile panel (**not** a push notification): *"Your emergency contacts changed. Update recovery protection?"*

On user confirm, the flow is identical to initial setup: fresh seed, fresh commitment (linked as superseding the prior one), fresh shares distributed, holders prompted once to accept new share and discard old. Old `RecoverySeedCommitment` remains DHT-resident but is cryptographically inert because the `KeyRotation` validator (rule 4) rejects references to superseded commitments.

### 6.5 Edge cases

| Case | Handling |
|---|---|
| Fewer than 2 emergency contacts | Setup doesn't trigger; profile panel shows "add at least 2 emergency contacts to enable protection" |
| Threshold = M (all required) | Allowed; status panel warns about brittleness |
| All contacts in same household | Setup proceeds; elohim flags diversity risk per household-is-resilience-unit memory |
| Contact refuses | Excluded; setup proceeds if accepted ≥ M; otherwise setup remains pending |
| Doorway federation mid-setup | No impact — setup is peer-to-peer via libp2p, not doorway-mediated |
| Matthew already has a commitment | Routes to reshare flow, not fresh setup |

---

## 7. Recovery Flow (Crisis Moment)

### 7.1 User-facing framing

**Matthew on a new device (any browser, any federated doorway):**
> Welcome back. Enter your handle or email to sign in, or **recover access** if you've lost your device.

One button. No technical vocabulary.

**Holder's perspective:**
> **Matthew needs your help recovering access.** *(his photo, last-seen date, relationship label)*
>
> *Your elohim reviewed the request and thinks this is genuine.* ◯ confidence high
>
> *If this wasn't you expecting this, say no — don't feel bad; protection works by caution.*
>
> `[ Yes, help Matthew ]`   `[ No, this feels wrong ]`

One prompt. Silent if ignored. Holder can change their mind up to quorum close. Confidence indicator surfaces the elohim's assessment so the holder can pause and verify out-of-band ("let me call Matthew before I confirm") if it's below a comfortable threshold.

### 7.2 Happy path sequence

```
Matthew (new browser)          Doorway D                                   Holders (each with HeldRecoveryShare)
───────────────────           ─────────                                    ────────────────────────────────────

[1] Visit doorway URL
[2] Enter handle/email
[3] Optional: answer a few
    "something only you'd know"
    challenge questions
    (elohim-generated from
     public-but-personal signals)

     ──handle+answers────►    [4] Resolve handle → human_agent_pubkey
                                  via elohim-storage projection
                              [5] Rate-limit check
                                  (per-IP, per-handle, recent attempts)
                              [6] Browser computes hashcash-style
                                  proof-of-work (~1s CPU)
                              [7] Generate ephemeral X25519 keypair
                                  for this session (session_pubkey)
                              [8] Generate new_agent_pubkey for claimant
                              [9] Commit RecoveryRequest on DHT:
                                  { human, new_agent,
                                    seed_commitment_hash,
                                    hosting_doorway, recovery_mode: Normal,
                                    nonce, timestamp }
                             [10] Create recovery_sessions row
                                  (state: collecting, TTL 24h)
                             [11] Publish libp2p gossipsub:
                                  topic: recovery.invitation.{commitment_H}
                                  payload: { request_hash,
                                             session_pubkey,
                                             encrypted_claimant_answers,
                                             hosting_doorway_url }
                                                                   ────────►  [12] Subscribed holders receive
                                                                              [13] Holder's elohim:
                                                                                   - validates request on DHT
                                                                                   - cross-refs with local
                                                                                     HeldRecoveryShare
                                                                                   - decrypts claimant_answers
                                                                                   - evaluates signals:
                                                                                     • answer plausibility
                                                                                     • anomaly detection
                                                                                     • relationship recency
                                                                                     • behavioral trust
                                                                                     • concurrent-request check
                                                                                   → RecoveryAuthorizationAssessment
                                                                                     with confidence_tier and
                                                                                     reasoning
                                                                              [14] App renders ONE prompt
                                                                                   with elohim's assessment
                                                                                   pre-surfaced
                                                                              [15] Holder decides:
                                                                                   ─ yes  → proceed
                                                                                   ─ no   → libp2p DENY (optional)
                                                                                   ─ abstain → silence
                                                                              [16] On yes:
                                                                                   - decrypt HeldRecoveryShare
                                                                                     (holder's agent key)
                                                                                   - re-encrypt share under
                                                                                     session_pubkey (ECIES)
                                                                                   - sign authorization_payload:
                                                                                     { request_hash,
                                                                                       confidence_tier,
                                                                                       timestamp }
                                                                                   - libp2p direct:
                                                                                     /elohim/recovery/1.0.0
                                                                                     → { authorization_payload,
                                                                                         encrypted_share }
                                                                              [17] Optional: commit
                                                                                   MyRecoveryAuthorization
                                                                                   to own source chain

                             ◄───────────────
                             [18] Verify each arriving bundle:
                                  - authorization signature valid under
                                    holder's agent pubkey
                                  - share decrypts under session private key
                                    (proves transport)
                                  - vsss-rs verification commitment check
                                    (rejects malicious shares)
                             [19] Update recovery_sessions.
                                  shares_received_count (deduplicated
                                  by holder pubkey)
                             [20] SSE to claimant browser:
                                  "3 of 5 people helping..." (ambient)

                             [21] When shares_received_count ≥ N:
                                  - Reassemble seed_private via vsss-rs
                                  - Derive Ed25519 keypair; verify
                                    public half matches commitment
                                  - Sign(new_agent_pubkey || request_hash)
                                    → quorum_signature
                                  - Commit KeyRotation on DHT (Normal branch)
                                  - Commit KeyRevocation for superseded
                                    agent (defense-in-depth)
                                  - Zeroize seed, shares, session keys
                                  - recovery_sessions.state → rotating → hosting

[22] Browser receives SSE:
     "Access restored. Loading      [23] Conductor worker pool installs
      your space..."                      imagodei cell:
                                          - agent_key: new_agent_pubkey
                                          - membrane_proof: KeyRotation
                                            entry (DNA validator accepts
                                            via rule 6 signature check)
                                     [24] Hand session handle to browser
                                          (existing hosted-user JWT pattern)
                                     [25] recovery_sessions.state = complete

[26] Matthew sees his network        [27] Hosted session TTL 24h
     governance, stewardship,             (existing hosted-user behavior)
     content, everything
```

### 7.3 Fast-path revocation

Revocation is protective; recovery is constructive. Protection triggers faster and from more sources.

**Three paths to `KeyRevocation`:**

**Path A — Self-revoke from another device.** Matthew still has any working agent (another laptop, tablet, phone). That cell commits `KeyRevocation` for the compromised agent directly. Signed by an authorized agent of the same Human; no quorum needed. Instant. The common case — "my phone was stolen but I have my laptop."

**Path B — Emergency-contact revocation votes (independent of recovery).** Emergency contacts don't need a full `RecoveryRequest` to pause a compromised agent. Any holder's elohim can commit `RevocationVote` on DHT attesting "I have reason to believe Matthew's agent X is compromised" — reasons include out-of-band confirmation (phone call from Matthew), elohim anomaly detection (prolonged inactivity + behavioral flags), or observation of a concurrent `RecoveryRequest`. When votes from distinct holders reach the **revocation threshold** (same N as recovery by default, configurable lower), the DNA validator accepts a `KeyRevocation` entry referencing them.

A `RevocationVote` from an emergency-contact agent is valid if the voter has a provable `HumanRelationship` with the target Human that carries `emergency_access_enabled = true`. The validator checks the relationship entry on DHT. (A stronger scheme using VSS commitment zero-knowledge proofs is possible but over-engineering for Phase 2.)

**Path C — Atomic with `KeyRotation`.** When quorum for `KeyRotation` is reached at doorway, `KeyRevocation` for the superseded agent is committed in the same operation. No gap between "new key authorized" and "old key revoked."

### 7.4 Stewarded recovery path (baseline primitives)

When a Human has an active `StewardshipGrant`, recovery routes through the steward-authority path:

- `RecoveryRequest.recovery_mode = Stewarded { grant_hash }` where `grant_hash` references the active grant.
- The steward set defined in the grant acts as the authorizing quorum (rather than the Shamir share-holders).
- Threshold and ward-consent semantics are carried by the grant's policy fields and inherited policies (`PolicyInheritance`).
- For Phase 2, Normal path is the primary end-to-end flow; Stewarded path has the data-model discriminator and validation rule 7 scaffold, with `stub_reject` behavior until Phase 2b lands the full steward-quorum-signing mechanism.

**Elohim intervention in stewarded recovery** is first-class (per Section 9.1 below) and addresses the adversarial stewardship patterns (steward-weaponized takeover, steward-blocked ward recovery, malicious grant installation) with appeal flows through the existing `StewardshipAppeal` entry type.

### 7.5 Edge cases and failure modes

| Case | Behavior |
|---|---|
| Quorum not reached within TTL | Session fails, in-memory material zeroized, Matthew notified: "not enough people were available; try again or reach out directly." `RecoveryRequest` persists (attempt audit) without `KeyRotation` |
| Matthew's old device still functional | Old device continues operating until `KeyRotation` lands on DHT. Post-commit, the old cell observes via post-commit signal that its agent has been superseded and stops being authoritative. Matthew can wipe or repurpose the device |
| Concurrent recoveries (different doorways) | Each request is independent on DHT. First to reach quorum commits `KeyRotation`; the second's attempted `KeyRotation` fails validation (commitment now superseded). Holders' elohims detect concurrency and raise caution. No deadlock |
| Holder is offline | Remaining holders can still reach quorum if ≥ N respond. Offline holder's silence = abstention |
| Matthew-as-doorway-operator recovery | Personal identity recovery is independent of doorway service. Matthew recovers at a peer doorway; his doorway service continuity is an operational Phase 3+ concern, not an identity concern |
| Malicious share returned by compromised holder | `vsss-rs` verification commitment check at step [18] rejects corrupted shares; holder excluded from count; Matthew's UI notes "one share couldn't be verified; asked the others" |
| Replay of old authorization | Authorization payload signs `(request_hash, confidence_tier, timestamp)`; `request_hash` is nonce-bound. Replayed authorizations don't apply to new requests |

---

## 8. Federation, elohim-agent Integration, Security Model

### 8.1 Multi-doorway federation semantics

| Concern | Behavior |
|---|---|
| Recovery initiation | Any doorway Matthew is federated with can host the session |
| Session binding | `RecoveryRequest.hosting_doorway_pubkey` binds session to one doorway; not cross-doorway-synced |
| Invitation reach | libp2p gossipsub is P2P; invitations cross doorway boundaries naturally |
| Concurrent sessions (different doorways) | Both exist on DHT; first to quorum wins; the other fails validation when it tries to commit `KeyRotation`; holders' elohims flag concurrency |
| Doorway compromise | Start over at a different doorway; DHT is source of truth for request + rotation; no state sync needed |
| Cross-doorway hosted-cell migration | **Not in Phase 2.** Hosted cell lives at the completing doorway. Migration = new `KeyRotation` (supported by the primitive) |

Nothing structural changes about doorway federation for recovery — the existing peer/federation model covers it. The recovery flow inherits multi-doorway resilience automatically.

### 8.2 elohim-agent integration (discernment rule 4)

Each peer's elohim-agent is the trust oracle for holder-side authorization. Contract between peer-app and peer-elohim:

**Request — app to elohim on invitation receipt:**
```rust
pub struct RecoveryAuthorizationRequest {
    pub request_hash: EntryHash,
    pub protected_human_pubkey: AgentPubKey,
    pub commitment_hash: EntryHash,
    pub claimant_answers: Option<Vec<ClaimantAnswer>>,
    pub session_pubkey: [u8; 32],
    pub hosting_doorway_pubkey: AgentPubKey,
    pub received_at: Timestamp,
}
```

**Response — elohim to app:**
```rust
pub struct RecoveryAuthorizationAssessment {
    pub confidence_tier: ConfidenceTier,
    pub reasoning: Vec<SignalSummary>,
    pub recommended_action: Recommendation,
    pub factors_considered: Vec<String>,
    pub concurrent_requests_detected: bool,
}

pub enum Recommendation {
    Authorize,
    Decline,
    Abstain,
    NeedsHumanJudgment,
    RequireOutOfBand,  // suggests holder contacts Matthew directly before confirming
}
```

Inputs available to the holder's elohim locally:
- Holder's `HumanRelationship` with the claimant
- Historical interaction patterns (behavioral trust module)
- Claimant-provided challenge answers
- Anomaly signals (unusual geography for request origin, frequency of recovery attempts, timing relative to known events)
- Concurrent `RecoveryRequest` entries on DHT for the same Human (red flag)
- Discernment rule 4 hook already present in elohim-agent (April 18 commit `f9e68b4c`)

The elohim produces an assessment; the human sees it and decides. The elohim does not decide for the human. This lines up with the elohim-agent sense-and-respond architecture: discernment lives in elohim-agent (Rust), TypeScript is sense-and-respond only.

**Fallback when elohim is unavailable** (no sidecar, hosted mode, pre-graduation): the app shows the raw invitation and lets the holder decide without assessment. Flow still works; less-informed judgment. `Phase::ElohimActive` observation determines which path fires.

### 8.3 Security model — threat matrix

| Threat | Mitigation |
|---|---|
| Handle-spoofing (attacker claims to be Matthew) | Handle → pubkey lookup is convenience; cryptographic gate is the seed signature on `KeyRotation`. Attacker cannot produce this without actual shares |
| Stolen laptop + attacker attempts impersonation | Old agent key cannot authorize `KeyRotation` (no access to seed private half). Fast-path revocation (Path A or B) cuts off the stolen device within minutes |
| Doorway compromise at session host | Doorway is blind; shares only in memory during reassembly; zeroized immediately. Compromised doorway can disrupt but not impersonate |
| Multi-doorway concurrent compromise | Each session independent; no key material shared between doorways |
| Emergency-contact compromise (one holder) | Single holder insufficient for quorum (N ≥ 2). Share at rest is encrypted under holder's agent key |
| Emergency-contact coercion (partial) | elohim-agent makes independent assessments per holder; concurrent-request detection surfaces coordinated attacks; threshold N provides quorum floor |
| Colluding quorum ≥ N | **Fundamental social-recovery limitation.** Mitigated by careful contact selection + elohim surfacing diversity/cluster warnings + retrospective rollback via supersession chain if detected post-hoc |
| Malicious-dealer attack at setup | `vsss-rs` verifiable shares with commitments; corrupt shares detected at recovery time and excluded |
| Replay of old authorizations | Authorization payload signs `(request_hash, confidence_tier, ts)`; request_hash is nonce-bound |
| Social-graph inference (DHT observation) | Share-holder list absent from DHT; only aggregate (N, M) visible; `KeyRotation` carries no list of signers |
| DoS via spam recovery requests | Rate limits (per-IP, per-handle, per-doorway), proof-of-work on initiation (~1s), holder-side dampening of repeats |
| Key material leak at session end | In-memory cache with `Drop`-time zeroization; 24h session TTL caps exposure window |
| DHT content-address forgery | Standard Holochain integrity (hash collision required) |
| `KeyRotation` forgery | Validator rule 6 signature verification is the cryptographic gate. Forgery requires seed compromise or hash break |
| Cross-DNA re-keying lag (`CustodianAssignment`) | Flagged as Phase 3 signal handler; `KeyRotation` commit triggers elohim-storage post-commit re-map old→new agent in projection |
| Matthew-as-doorway-operator identity-doorway coupling confusion | Spec explicitly separates identity recovery (Phase 2) from doorway service continuity (Phase 3+). No shared failure mode |

### 8.4 What this design does NOT protect against

- **Malicious quorum** — if ≥ N of Matthew's emergency contacts collude, they perform a recovery against his will. Mitigations: careful selection + elohim diversity warnings + post-hoc supersession. Cannot be prevented architecturally without breaking the social-recovery premise.
- **Physical coercion of Matthew** to initiate recovery under duress to attacker-chosen new agent key. Phase 5 candidate: duress canary / silent-freeze.
- **Global passive adversary** observing all libp2p traffic. Share-holder identity inferrable from libp2p direct-message patterns during active recovery. Phase 5+: mix-net / onion routing.
- **Side-channel attacks on doorway memory** during reassembly. Standard memory hygiene helps; full defense requires hardware enclaves.

---

## 9. Elohim Guardianship, Red-Team, Testing, Rollout

### 9.1 Elohim intervention points (the guardian layer)

Elohim acts **adversarially against misuse** at every stage, not only as a passive authorization oracle. The following intervention points are Phase 2 deliverables:

| Stage | Misuse pattern | Elohim intervention |
|---|---|---|
| **Setup (Matthew)** | Contact set lacks diversity (same household, same geography) or shows duress signals (sudden drastic change without relationship history) | Ambient diversity warning in profile panel; pause commit if duress pattern detected (surface, don't block) |
| **Recovery initiation** | Attempt during Matthew's known-vulnerable window (travel, illness, legal proceedings) | Holders' elohims raise confidence threshold; insert suggestion to verify out-of-band |
| **Holder authorization** | Holder under coercion or distraction | Elohim recommends `RequireOutOfBand`: *"Call Matthew directly before you confirm. We'll wait."* |
| **Recovery in progress** | Colluding subset attempting takeover | Participating holder's elohim detects unusual quorum formation (contacts who don't normally interact coordinating tightly); raises `IdentityAnomaly` entry visible to non-participating contacts and Matthew's remaining devices, who can issue `RevocationVote` |
| **Key rotation pre-commit** | New agent pubkey flagged as malicious (federation-shared blocklist, repeated misuse) | Hold rotation pending out-of-band review |
| **Post-recovery** | Unusual first-24h activity (mass content deletions, governance reversals, contact purges) | Matthew's new elohim watches + escalates; rollback via supersession chain supported |
| **Stewarded recovery: steward takeover** | Steward attempts unilateral recovery without ward consent | Require ward affirming signal per declared `DevicePolicy`; absence triggers 72h appeal window for non-steward emergency contacts to challenge |
| **Stewarded recovery: steward-blocked** | Steward refuses legitimate ward recovery | Ward initiates `StewardshipAppeal`; elohim + qahal/mishpat dispute flow evaluates; network override possible for egregious cases |
| **Stewarded recovery: steward-weaponized revocation** | Steward uses fast-path revocation to cut off ward unjustly | Steward-alone revocation (without quorum or ward consent) subject to appeal window before becoming authoritative |

Each intervention is a pattern-match + graduated response: **surface → pause → require out-of-band → block → alert network**. Phase 2 ships at least surface/pause levels; block/alert-network tuning continues in Phase 5.

These intervention points attach to the same elohim-agent rule 4 handler already scaffolded; each is an additional signal evaluator within the rule, not a separate subsystem.

### 9.2 Red-team deliverable (part of the spec)

Red-teaming is a required Phase 2 gate, not a follow-up. Before rollout items below ship, a red-team pass must:

- Enumerate specific attack scenarios with actors, timing, and required resources
- Produce a2o scenarios for the successful defense of each
- Produce a2o scenarios for graceful failure where attacks succeed despite defenses
- Confirm security model invariants hold under each scenario
- Flag residual risks needing Phase 5+ follow-up

**Minimum scenarios the red-team pass must cover:**

1. Stolen laptop with extracted agent key, no quorum cooperation
2. Stolen laptop + phishing to obtain recovery challenge answers
3. Compromised single doorway
4. Compromised single emergency contact
5. Colluding minority of emergency contacts (< N)
6. Colluding majority of emergency contacts (≥ N) — must fail safely, not catastrophically
7. Matthew under duress initiating recovery against his will
8. Concurrent recovery races from different doorways
9. Holder-side elohim compromised or tampered
10. Denial-of-service spam at multiple layers
11. Social-graph inference attack observing DHT + libp2p
12. Replay attack against old authorizations
13. Malicious-dealer setup corruption
14. Cross-DNA inconsistency during `KeyRotation` (stale `CustodianAssignment` references)
15. Steward-weaponized stewarded recovery (Phase 2b)
16. Steward-blocked stewarded recovery (Phase 2b)

Each scenario produces a gherkin `.feature` in `genesis/a2o/features/auth/recovery/red-team/`, tagged `@red-team @recovery`, run in a dedicated suite.

### 9.3 Testing strategy

**Unit (Rust):**
- `imagodei_integrity` validation rules, especially `KeyRotation` rule 6 with a full matrix of valid/invalid signatures, superseded commitments, mismatched pubkeys
- `imagodei_coordinator` happy-path + error-path tests for each function
- `vsss-rs` integration: correct threshold reassembly, verification commitment enforcement, malicious-share detection
- Doorway `recovery_sessions` lifecycle: state transitions, TTL expiry, `Drop`-time zeroization assertions
- libp2p codec roundtrips: `/elohim/recovery-provisioning/1.0.0`, `/elohim/recovery/1.0.0`
- elohim-agent discernment rule 4: assessment outputs for a signal-input matrix

**Integration (Rust, multi-node):**
- End-to-end happy path: provisioning → recovery → `KeyRotation` → hosted cell accessible
- End-to-end fast-path revocation via each of paths A, B, C
- Multi-doorway recovery: initiate at D_1, holders at D_2 and D_3, complete
- Concurrent-request race: two doorways initiate simultaneously; first wins, second fails

**Frontend (Vitest):**
- `RecoveryCoordinatorService` extended to real endpoints; preserve 98.7% coverage baseline
- New holder-side authorization notification component: elohim assessment render, single-prompt flow, abstain-by-silence
- Hosted-cell session landing component: lands user in their network post-recovery

**A2O scenarios (Gherkin, `genesis/a2o/features/auth/recovery/`):**
```
recovery-provisioning.feature
recovery-reshare.feature
recovery-request-flow.feature
recovery-fast-path-self-revoke.feature
recovery-fast-path-contact-revoke.feature
recovery-fast-path-atomic-revoke.feature
recovery-hosted-cell-landing.feature
recovery-multi-doorway.feature
recovery-concurrent-sessions.feature
recovery-elohim-intervention.feature
red-team/*.feature  (14+ scenarios)
```

**Shem canvas acceptance:**

Per the corrected household-plus-shem topology:

- **Household cluster:** Matthew, Jessica, Timothy on separate nodes. Matthew operates a doorway (D_matthew).
- **Shem:** Pete, Susan, Adam, Eve, Nancy, Gertrude, Maria, ... as real peers with their own conductors. Shem's doorway (D_shem).

Phase 2 acceptance scenarios on the live cross-node topology:

**Scenario 1 — Cross-doorway happy path:** Matthew loses his laptop (and D_matthew goes down). He recovers via D_shem. Jessica and Timothy (household nodes) + Pete + one other shem persona authorize. Matthew lands in a hosted cell on D_shem. Dashboards on both doorways light up correctly.

**Scenario 2 — Stewarded recovery (Phase 2b):** Timothy loses his tablet. Jessica (his steward) initiates recovery on his behalf from her household node. `RecoveryRequest.recovery_mode = Stewarded`. Age-appropriate ward affirmation carried via `DevicePolicy`. Timothy lands in his hosted cell.

**Scenario 3 — Concurrent recovery race (red-team):** Attacker initiates `RecoveryRequest` for Matthew at D_shem simultaneously with Matthew's legitimate request at D_matthew. Holders' elohims detect concurrency. Legitimate request reaches quorum first. Attacker's `KeyRotation` attempt fails validation (commitment superseded).

Phase 2 is called done only when all three scenarios pass on the live topology with real peers exchanging real traffic.

### 9.4 Rollout milestones

| Milestone | Deliverable | Acceptance |
|---|---|---|
| **M1** — Data model | 3 new imagodei entry types with validation, coordinator functions, storage projections | Unit tests pass; `schema:validate` clean; TS codegen runs |
| **M2** — Provisioning | Setup flow end-to-end (entry + libp2p distribution + holder storage + elohim-stewarded reshare) | `recovery-provisioning.feature` + `recovery-reshare.feature` pass on shem |
| **M3** — Recovery + authorization | libp2p gossipsub invitation + elohim rule-4 integration + share release + reassembly + `KeyRotation` | `recovery-request-flow.feature` + `recovery-multi-doorway.feature` pass |
| **M4** — Fast-path revocation | All three revocation paths | Three fast-path `.feature` files pass |
| **M5** — Hosted cell + frontend integration | Conductor worker pool wiring + frontend coordinator service updates + holder-side UI | `recovery-hosted-cell-landing.feature` passes; Matthew sees his network on shem |
| **M6** — Guardian + red-team | Elohim intervention points + red-team scenarios | `recovery-elohim-intervention.feature` + 14+ red-team scenarios pass |

**Phase 2b** (follow-up, not gating Phase 2 ship):
- Stewarded recovery UX polish
- Full stewarded-path a2o coverage
- Steward-specific red-team scenarios (15, 16)

### 9.5 Dependencies and prerequisites to start M1

- `vsss-rs` crate vetted and added to `imagodei_coordinator` + doorway dependencies
- `KeyRotation` validation rule 6 reviewed with a cryptographer (Ed25519 signature verification details, domain separation)
- Doorway `/elohim/recovery/*` and `/elohim/recovery-provisioning/*` libp2p protocols registered in the `libp2p-protocols` skill's wire-format spec
- Red-team participants identified (preferably external to the immediate design team)
- Shem personas confirmed ready to run the acceptance canvas (verify household cluster has Matthew/Jessica/Timothy deployed correctly)
- Phase 1 shard tracking confirmed stable on shem (prerequisite for later phases; not blocking Phase 2)

---

## 10. Revision History

| Date | Change | Author |
|---|---|---|
| 2026-04-21 | Initial spec (supersedes Jan 2026 RECOVERY-PROTOCOL.md for Phase 2 scope) | Matthew Dowell |
