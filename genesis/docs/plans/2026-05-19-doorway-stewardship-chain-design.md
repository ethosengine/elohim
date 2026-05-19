# Doorway Stewardship Chain — Design

**Status:** Substrate landed (Phase 2 L3 + amend-in-place). Wiring in progress (task #16).
**Date:** 2026-05-19
**Predecessor:** `genesis/docs/plans/2026-05-19-topology-resilience-qahal-synthesis.md` §1.2 flagged doorway dashboard auth as a delivery gap; the gap turned out to be the tip of a substrate question about hardware stewardship succession.
**P2P-design-gate:** Run inline (see §4). Substrate decision: no new DHT entry types; reuses `Commitment` (REA) and `Attestation` (imagodei).

---

## 0. The grandpa scenario, in one paragraph

Grandpa stewards a doorway. He owns the hardware, he runs the daily operations, hosted users register through his box. Grandpa dies. How does the network find out, who's authorized to take over, what happens to hosted users, what happens to the operators (his kids) who were running specific capabilities under him? The naive answer — "there's an admin role, transfer it to the next person" — fails on every axis: it pre-assumes a central authority, it loses the social complexity of real-world succession (executor vs. household vs. recovery committee), it conflates legal custody with operational stewardship, and it provides no mechanism for the network to verify the succession happened legitimately. This design replaces "admin role" with a three-tier attestation chain whose transitions are notarized social facts witnessed by a quorum grandpa named while alive.

## 1. Three-tier authority chain

```
Hardware Custodian-of-record
    │  Attestation, kind=hardware-custody
    │  "I am the legal/social person of record for this physical device"
    │  Succession quorum: M-of-N (reused Recovery Phase 2 witnesses)
    │  Reach: commons (current state + transitions); quorum members public,
    │         seed material at intimate reach in recovery_seed_commitments
    ▼
Steward-of-record
    │  Attestation, kind=steward-of-record
    │  "I am the accountable operational steward for this doorway"
    │  Serves under custody (references custodyAttestationHash)
    │  Succession quorum: independent of custody (often overlaps)
    │  Reach: commons
    ▼
Active Operators
    │  Commitment, action=operate-doorway (REA, already landed Phase 2 L3)
    │  "I commit specific capabilities for this doorway"
    │  Serves under custody + steward (references both hashes)
    │  Capability scope per operator-classification.schema.json
    │  Reach: per-binding (operator-private / stewards-only / public)
    ▼
JWT OperatorSnapshot
    Cached at /auth/login; embedded snapshot of the active commitment plus
    custody + steward attestation hashes. Auth fast path checks the snapshot;
    snapshot expiry forces a refresh, which queries the projection for the
    current chain.
```

**Independent revocation.** Each tier is its own attestation/commitment. Custody transfer orphans everything below it (forces new steward + new operator commitments). Steward transfer orphans only operator commitments. Operator commitment can be cancelled without disturbing custody or stewardship.

**No new DHT entry types.** All three tiers reuse existing entry types — `Attestation` (imagodei) for the top two, `Commitment` (REA, content_store) for the operators. New `kind` discriminators on `Attestation` and a new `action` value on `Commitment`.

## 2. Reach model

| Surface | Reach | Why |
|---|---|---|
| Current hardware custodian-of-record | commons | Hosted users + federated peers need to verify their trust assumption |
| Current steward-of-record | commons | Same — the social contract is between the user and the steward |
| Current operator bindings (active capabilities) | per-binding (operator-private default; configurable) | Routine operational delegation — visible to fellow operators by default |
| Transition events (`hardware-custody-transfer`, `steward-of-record-transfer`) | commons | Whole network needs to see authority change so trust can be re-evaluated |
| Succession quorum membership (the witnesses) | commons | Already public via `recovery_witnesses`; quorum membership is the network's accountability anchor |
| Seed material backing the quorum (Shamir shares, presigned authorizations) | intimate | Already private via `recovery_seed_commitments` — exposing this is an attack vector |

The privacy model is **public-default for the social facts** (who's currently in charge, what authority changed when) and **private-default for the secret machinery that enables succession** (the cryptographic material witnesses hold). This matches the existing protocol pattern from Recovery Phase 2 — we're reusing it, not inventing new privacy semantics.

## 3. The grandpa scenario, walked

### 3.1 Pre-mortem state (on the DHT)

```
Attestation #A1 — kind=hardware-custody
  doorwayId: grandpa-hub
  custodianAgent: grandpa_key
  successionQuorum: { thresholdM: 2, witnessAgents: [uncle_jim, aunt_clara, neighbor_pete] }
  validFrom: 2024-03-15
  predecessor: null  ← genesis

Attestation #S1 — kind=steward-of-record
  doorwayId: grandpa-hub
  stewardAgent: grandpa_key  ← same as custodian (collapsed roles)
  custodyAttestationHash: #A1
  successionQuorum: { thresholdM: 2, witnessAgents: [household_collective_key, kid_1_key, kid_2_key] }
  validFrom: 2024-03-15

Commitment #C1 — action=operate-doorway
  provider: grandpa_key
  scope: doorway:grandpa-hub
  capabilities: ["*"]
  succession_role: primary
  custodyAttestationHash: #A1
  stewardAttestationHash: #S1

Commitment #C2 — action=operate-doorway
  provider: kid_1_key
  scope: doorway:grandpa-hub
  capabilities: ["hosted_users.lifecycle", "cache.read"]
  succession_role: deputy
  custodyAttestationHash: #A1
  stewardAttestationHash: #S1
```

### 3.2 Grandpa dies. Real-world events drive the substrate.

1. **Family convenes the succession quorum** named in #A1. They convene off-protocol; the protocol does not detect death.

2. **Quorum issues `Attestation #A2` (kind=hardware-custody-transfer)** referencing #A1 as predecessor. Each of {uncle_jim, aunt_clara, neighbor_pete} who agrees signs `witnessSignatures` — they need 2 of 3 per #A1's thresholdM. `transferReason: "death"`. `incomingCustodian: household_collective_key` (the family decided collective custody).

3. **DHT validators check #A2:**
   - Predecessor #A1 is currently the latest valid custody attestation? ✓
   - Signature count ≥ #A1.successionQuorum.thresholdM? ✓
   - Each signature is from a member of #A1.successionQuorum.witnessAgents? ✓
   - transferReason allowed without outgoingCustodian signature (death/incapacitation/recovery-event)? ✓

4. **The chain orphans.** Any cached snapshot referencing #C1 or #C2 (under custodyAttestationHash=#A1) now fails the auth resolver's chain check. The next API call from grandpa's kids returns `CapabilityError::CustodyTransferred { previous_hash: #A1, current_hash: #A2 }`, which the doorway translates to 401 "re-authenticate" with the supersession context attached.

5. **New custodian decides disposition.**
   - **Continue:** household_collective_key (signing as multisig per its own setup) issues a new `Attestation #S2 (kind=steward-of-record)` referencing #A2 as custody, possibly with the same or different stewardAgent (the household might pick kid_1 as new steward, or keep collective stewardship). Then issues fresh operate-doorway Commitments #C3, #C4 under #A2 + #S2 with the capability scopes for the new operators.
   - **Decommission:** New custodian issues `Attestation #A3 (kind=doorway-decommissioned)` — auth resolver hard-fails every operator commitment, DoorwayRegistration discovery layer marks the doorway offline, hosted users get migration paths via the existing graduation flow.
   - **Migrate to new hardware:** New custodian issues `Attestation #A3 (kind=hardware-custody)` for a new doorway-id with a migrated-from link to grandpa-hub; old doorway gets decommissioned in tandem.

6. **Hosted users see the change.** Because the transitions are commons-reach, the network's projection of "who currently stewards grandpa-hub?" updates. Hosted users registered there see the new steward in their account dashboard; their elohim agent can advise them whether they want to stay or migrate to a different doorway based on their trust assumptions about the new steward.

7. **Federated peers re-evaluate.** Doorway federation peers query the current custody/steward of grandpa-hub; their own elohim agents decide whether the federation relationship continues unchanged, narrows in scope, or is severed.

No central registry was involved. No admin role was transferred. The succession is a series of cryptographically-witnessed social facts.

## 4. P2P-design-gate restatement

Compressed reference; full classification per `.claude/skills/p2p-design-gate/SKILL.md`.

| Entity | Cat | Address | SoT | Coordinator | Projection | Anti-pattern check |
|---|---|---|---|---|---|---|
| HardwareCustodyAttestation | A (existing entry type) | content-derived (Attestation ActionHash) | DHT (imagodei `Attestation`, kind=`hardware-custody`) | imagodei::create_attestation | `attestations` table + dedicated index on `(kind, scope)` | ✅ — no new entry type; just a new `kind` discriminator |
| StewardOfRecordAttestation | A (existing entry type) | content-derived | DHT (imagodei `Attestation`, kind=`steward-of-record`) | imagodei::create_attestation | same table, same index | ✅ |
| CustodyTransferAttestation | A (existing entry type) | content-derived; references predecessor | DHT (kind=`hardware-custody-transfer`) | imagodei::create_attestation | predecessor link materializes "current_custody_per_doorway" view | ✅ |
| StewardTransferAttestation | A (existing entry type) | content-derived | DHT (kind=`steward-of-record-transfer`) | imagodei::create_attestation | predecessor link materializes "current_steward_per_doorway" view | ✅ |
| OperateDoorwayCommitment | A (existing entry type) | agent-scoped composite (provider, scope) | DHT (`Commitment`, action=`operate-doorway`) | content_store::create_commitment | `rea_commitments` table + composite index | ✅ — already landed Phase 2 L3 substrate |
| OperatorCapabilityScope | A2 (derived) | n/a (link metadata) | Commitment's resource_classified_as | n/a | denormalized into rea_commitments columns | ✅ |
| RecoveryQuorumRequest (reused, extended) | A (existing) | content-derived | DHT (imagodei `RecoveryQuorumRequest`) | imagodei::create_recovery_quorum_request | `recovery_quorum_requests` table — add `request_kind` column | ✅ — reuses existing primitive |
| OperatorSnapshot (in JWT) | C (operational) | random/JWT-bound | doorway-local cache | n/a | not persisted (JWT-embedded) | ✅ |

**Capacity:** Mishpat 11/100, Lamad ~73/100, Imagodei 28/~100. Zero new entry types. Zero impact on DNA budgets.

## 5. Reusing Recovery Phase 2 primitives

Recovery Phase 2 (migration `2026-04-21-000000_recovery_phase_2`) introduced:
- `recovery_seed_commitments` — agent's recovery setup, private/intimate reach
- `recovery_quorum_requests` — when a recovery is triggered, the M-of-N request kicks off here
- `recovery_witnesses` — the people designated as witnesses (commons reach)
- `key_rotations` — the outcome of a successful key recovery

**Reuse map for stewardship transitions:**

| Recovery primitive | Custody/steward transition use |
|---|---|
| `recovery_seed_commitments` | Same shape — quorum members' seed shares for ALL succession kinds (key recovery, hardware custody, stewardship). Existing intimate reach is correct. |
| `recovery_quorum_requests` | Add a `request_kind` column with values: `key-recovery` (existing default), `hardware-custody-transfer`, `steward-of-record-transfer`, `doorway-decommission`. Same M-of-N quorum machine; different request kind drives validator logic. |
| `recovery_witnesses` | Same — the witness designation is shared across all succession kinds. A witness in your custody quorum is the same kind of entity as a witness in your key-recovery quorum (often the same people). |
| `key_rotations` | Specific to key recovery; not reused. CustodyTransferAttestation and StewardTransferAttestation are the equivalent outcome artifacts for their respective transitions, stored as new Attestation kinds. |

**Migration:** `2026-05-XX-000000_recovery_quorum_request_kind` — adds `request_kind TEXT NOT NULL DEFAULT 'key-recovery'` to `recovery_quorum_requests`. Old rows backfill to `'key-recovery'`.

## 6. Hardware identity binding (deferred)

This design uses `DoorwayRegistration.id` as the bound hardware identity. That's pragmatic for landing the chain — the DoorwayRegistration is already a notarized fact, it's already the addressing handle.

**Limitation:** DoorwayRegistration.id is a software-chosen identifier. Two doorways could share an id (collision); a doorway could change its id (rebinding); hardware could move between DoorwayRegistrations (re-imaging). Custody bound to DoorwayRegistration.id will sometimes need to migrate when the underlying hardware is unchanged but the software-level id rotates.

**Deferred to backlog (task #18):** Extend the compute reporting surface to capture a hardware fingerprint (TPM serial when available; first-boot UUID stored in TPM/secure-enclave/encrypted disk otherwise). When this lands, custody attestations can bind to the hardware fingerprint instead of (or in addition to) the DoorwayRegistration.id, decoupling legal custody from the software identity rotation cycle.

## 7. What's landed (substrate)

Phase 2 L3 base substrate (committed 2026-05-19):
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — `REA_ACTIONS` extended with `"operate-doorway"`
- `elohim/elohim-storage/migrations/2026-05-19-000000_doorway_operator_action_indexes/` — composite index for fast lookups
- `elohim/elohim-storage/src/db/rea_commitments.rs` — `OPERATE_DOORWAY_ACTION`, `doorway_scope()`, `list_active_doorway_operators()`, `find_active_operator_binding()`
- `elohim/elohim-views/src/shefa.rs` — `DoorwayOperatorBindingView`
- `elohim/elohim-storage/src/views_convert/shefa.rs` — `project_doorway_operator_binding()`
- `elohim/sdk/schemas/v1/objects/operator-classification.schema.json` — schemaVersion 1+2
- `elohim/sdk/schemas/v1/views/doorway-operator-binding-view.schema.json`

L3 amend-in-place (committed 2026-05-19, this design):
- `elohim/sdk/schemas/v1/attestation/subtypes/hardware-custody-metadata.schema.json`
- `elohim/sdk/schemas/v1/attestation/subtypes/steward-of-record-metadata.schema.json`
- `elohim/sdk/schemas/v1/attestation/subtypes/custody-transfer-metadata.schema.json`
- `elohim/sdk/schemas/v1/attestation/subtypes/steward-transfer-metadata.schema.json`
- `operator-classification.schema.json` bumped to support `custodyAttestationHash` + `stewardAttestationHash`
- `doorway-operator-binding-view.schema.json` extended with the two optional hash fields
- Rust types updated (`DoorwayOperatorBindingView`, `OperatorSnapshot`)
- `CapabilityError::CustodyTransferred`, `CapabilityError::StewardTransferred` added
- Auth helper landed: `doorway/doorway-service/src/auth/operator.rs` with `resolve_operator_capability` + `legacy_admin_fallback`

## 8. What's deferred (task #16 — wiring)

These don't change the design above; they're the integration work.

1. **Imagodei DNA — attestation kinds.** Add `hardware-custody`, `steward-of-record`, `hardware-custody-transfer`, `steward-of-record-transfer`, `doorway-decommissioned` to the imagodei Attestation `kind` enum.

2. **Recovery Phase 2 extension migration.** Add `request_kind` column to `recovery_quorum_requests`.

3. **Storage helpers.**
   - `db::attestations::current_hardware_custody(doorway_id) -> Option<Attestation>` — returns the latest non-superseded
   - `db::attestations::current_steward_of_record(doorway_id) -> Option<Attestation>` — same
   - HTTP routes exposing these for doorway-service consumption (per the registry-driven manifest pattern)

4. **`verify_custody_chain`.** New function in `doorway/doorway-service/src/auth/operator.rs`. Queries elohim-storage for the current custody + steward attestation hashes for the snapshot's doorway, compares to the snapshot's embedded hashes, returns `Ok` if both match or `CustodyTransferred` / `StewardTransferred` otherwise. Called inside `resolve_operator_capability` after the capability-membership check.

5. **`/auth/login` projection lookup.** After agent-key challenge succeeds, query `find_active_operator_binding(agent, doorway_id)` to populate `OperatorSnapshot`. If the agent has no active operator binding, the JWT carries `doorway_operator: None` (legacy users continue to work; the new auth helper returns `NotOperator` for them).

6. **Bootstrap path.** On first doorway boot:
   - If no active `HardwareCustodyAttestation` exists for this doorway's id → auto-issue one signed by `STEWARD_AGENT_KEY`, with the steward as primary custodian, succession quorum = `STEWARD_AGENT_KEY` alone (1-of-1 trivial quorum, must be expanded by the steward in normal operation).
   - Then issue `StewardOfRecordAttestation` referencing the custody attestation.
   - Then issue `OperateDoorwayCommitment` with `capabilities=["*"]`, `succession_role=primary`, references both attestations.
   - From the next boot onward, all three are queried from the DHT; bootstrap is no-op.

7. **Per-route wiring.** Replace bare `match` arms for `/admin/dashboard/topology`, `/admin/federation/peers`, `/admin/hosted-users/*`, `/admin/cache/*` with capability-checked dispatch. Start with `legacy_admin_fallback` during the transition window; cut over to strict `resolve_operator_capability` once the bootstrap path has been live for at least one snapshot TTL across the deployment.

## 9. Failure modes

| Scenario | Outcome |
|---|---|
| JWT carries snapshot with stale custody hash; current custody is fresh | `CapabilityError::CustodyTransferred` → 401 force re-auth |
| JWT carries snapshot with fresh hashes but capability missing | `CapabilityError::CapabilityMissing` → 403 |
| Snapshot age > TTL | `CapabilityError::SnapshotExpired` → 401 refresh path |
| Legacy JWT (no operator snapshot) on a route that requires capability | `CapabilityError::NotOperator` → 401; transition routes use `legacy_admin_fallback` to accept these temporarily |
| New doorway has no bootstrap attestations yet | First-boot path auto-creates the chain; subsequent boots no-op |
| Custody quorum disagrees / threshold not met | The CustodyTransferAttestation never lands on the DHT (validator rejects insufficient signatures); the predecessor remains current |
| Two competing transfer attestations land simultaneously (split) | Validators MUST enforce: at most one non-superseded successor per predecessor. First-write-wins per the standard Holochain conflict resolution; the loser's witnesses can re-issue under the winner |
| Hardware physically destroyed (no transfer attestation possible) | Hosted users see the doorway as offline (heartbeat absent), migrate via graduation; the orphaned chain on the DHT is a forensic artifact, not a live authority |

## 10. Non-goals

- **No automatic detection of human death.** The protocol requires the succession quorum to act. If the quorum is unreachable or unwilling, the doorway's authority chain freezes — hosted users have a graceful migration path; the doorway is just no longer governed.
- **No central authority arbitration.** If the quorum is split, the protocol does not resolve disputes for them. Dispute resolution is a social process; the protocol just notarizes the outcome.
- **No retroactive transition invalidation.** A CustodyTransferAttestation once notarized is permanent; if it was issued in bad faith, the remedy is for the social network to refuse to recognize the new custodian (downrate, refuse federation, alert hosted users via elohim agents) — not for the protocol to "undo" the transition.

## 11. Open questions for follow-up sprints

- **Quorum overlap rules.** Can the custody quorum and the steward quorum be identical? Probably yes by default, but is there a security argument for forced disjointness?
- **Recovery-event-driven transfer.** When Recovery Phase 2 produces a `key_rotations` event for a custodian's key, should that automatically trigger a `hardware-custody-transfer` to the new key? Or does the new key just sign a fresh attestation manually?
- **Multi-doorway federation custody.** When a single agent custodies multiple doorways, is each chain independent? (Yes by current design; could be unified into a "doorway portfolio" later.)
- **Capability vocabulary governance.** Who decides what capability strings are valid? Currently the operator-classification.schema.json enum is hardcoded. Future: derive from manifest declarations of admin endpoints.

---

**End of design.**
