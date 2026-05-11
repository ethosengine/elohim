---
title: Attestation Consolidation — Validated Recognition as a Content Discriminator
status: Draft (awaiting user review)
created: 2026-05-11
authors: Matthew Dowell + Opus 4.7
pillar coupling: elohim (core primitive), imagodei + lamad + mishpat + infrastructure (manifest layers)
depends on: existing `Content` entry type in elohim DNA; existing `content_type` discriminator pattern; existing `Link` primitive; existing pillar manifest registry
related:
  - genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md (precedent for contentType discriminator pattern)
  - genesis/docs/superpowers/specs/2026-05-01-computation-attestation-graduated-rigor-design.md (proof-class gradient + EPR-variant framing)
  - genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md (Standing vs Attestation disambiguation, §2.2 + §4.2)
  - genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-wave-0-substrate-cleanup.md (Wave 0; this spec supersedes Wave 0's Attestation dedupe direction)
defers:
  - Observation layer (libp2p/iroh-shaped operational data) — own spec, sibling to this one
  - Per-pillar attestation subtype metadata schemas — declared in each pillar's manifest
  - Recovery protocol full redesign — touched here for Shamir decoupling, but the broader recovery flow is its own follow-up
---

## 1. Problem

The protocol currently has **at least 18 attestation-shaped DHT entry types** scattered across four DNAs, with overlapping but inconsistent fields, no shared validator floor, and no shared projection table:

- **imagodei DNA**: `Attestation`, `HumanityWitness`, `KeyStewardship`, `StewardshipGrant`, `RenewalAttestation`, `RecoveryRequest` + `RecoveryVote`, `IdentityChallenge` + `ChallengeSupport`, `KeyRevocation` + `RevocationVote`, `IdentityFreeze`, `StewardshipAppeal`, `PolicyInheritance`
- **elohim DNA**: `Attestation` (vestigial duplicate of imagodei's), `ContentAttestation`, `CustodianCommitment`, `ContentSuccession`
- **infrastructure DNA**: `HealthAttestation`, `DoorwayHeartbeatSummary`
- **mishpat DNA**: `GateDecisionAttestation`, `ProposalVote`, `StatementVote`, `GovernanceReaction`, `GraduatedFeedback`

Three structural problems flow from this:

1. **Vocabulary drift** — Each entry type defines its own discriminator fields (`category`, `attestation_type`, `attestation_kind`, etc.) with different taxonomies. There is no canonical "what kind of attestation is this?" answer across the protocol.
2. **DNA capacity pressure** — Lamad area of elohim DNA sits at ~77/~100 entry types. Imagodei DNA sits at ~31 entry types and growing. Adding per-domain attestation entry types doesn't scale.
3. **Conflation of observation and attestation** — `HealthAttestation` is fired on every health check (high volume, observation-shaped); `Attestation` is issued on capability recognition (low volume, claim-shaped). Mixing these on the same architectural layer causes both DNA bloat AND inappropriate gossip-latency for monitoring data.

The deeper architectural problem: **attestations are being designed as bespoke entities per domain instead of as instances of a single core primitive.** This is the same anti-pattern that `signal_kind` extensibility on `FeedbackSignal` was designed to prevent — and the pattern that experience-story-EPR design recapitulated cleanly (no new entry type, just a new `content_type` on existing `Content`).

## 2. The architectural cut

Two distinct layers, today conflated, are separated:

| Layer | Purpose | Architecture | Frequency | DHT? |
|---|---|---|---|---|
| **Observation** | Raw evidence — health pings, behavioral signals, monitoring events, review responses, audit logs, opinion-statements, doorway heartbeats | libp2p/iroh data-plane with SQL-shaped query, projection-tier storage | High volume, continuous | NO — operational only |
| **Attestation** | Validated claim derived from observations — "doorway X has been healthy across period P," "Alice has mastery in concept Y," "this content is peer-reviewed by 5 stewards" | DHT-notarized `Content` entry on elohim DNA | Low volume, when threshold/policy met | YES — Category A (notarized) |

This matches the three-layer truth model (`project_three_layer_truth_model.md`): DHT=notary, libp2p=data-ops, doorway=web2 projection. The current protocol accidentally puts both observation and attestation on the DHT; this spec corrects only the attestation half. The observation layer is a sibling spec.

## 3. The Attestation primitive

### 3.1 Carrier

An attestation is a `Content` entry on elohim DNA with `content_type: "attestation:<subtype>"`. There is no new DHT entry type. The carrier reuses every existing field on `Content`:

| Field | Use in attestation |
|---|---|
| `id` | CIDv1 — content-derived identity, same as all other Content |
| `content_type` | `"attestation:<subtype>"` — declared in pillar manifests |
| `title` | Human-readable summary ("Doorway X — Q2 2026 audit pass") |
| `description` | Longer explanation of what's being attested |
| `author_id` | The **issuer** (party making the attestation) |
| `reach` | Visibility (private / community / public / commons) |
| `metadata_json` | Structured payload — see §3.3 |
| `tags` | Indexable labels for the attestation subtype |
| `related_node_ids` | Quick-graph references to subject + evidence sources |
| `created_at` | Issuance time |
| `expires_at` | (in `metadata_json`) — optional expiration |
| `schema_version`, `validation_status` | Self-healing fields, same as other Content |

### 3.2 Subject and issuer

- **Issuer** is the `author_id` on the Content entry (signed via the action header, native to Holochain).
- **Subject** is referenced via a typed Holochain Link: `AttestationToSubject` from the attestation Content's EntryHash to the subject's EntryHash. The link tag carries `subject_kind` (`agent | content | device | hub | computation | governance-action`).
- The subject is itself a Content (or referenced via existing anchor patterns, e.g., agent CID → Human Content).

Discovery patterns:
- "Find all attestations issued BY agent X" — query Content by `author_id`
- "Find all attestations ABOUT subject Y" — traverse `AttestationToSubject` links from Y
- "Find all attestations of subtype Z" — query Content by `content_type` discriminator

### 3.3 metadata_json shape

```jsonc
{
  "attestation_kind": "humanness",          // mirrors the content_type subtype (denormalized for query)
  "subject_cid": "bafyrei...",              // copy of subject for query convenience
  "subject_kind": "agent",                  // mirrors AttestationToSubject link tag
  "validation_method": "peer-confirm",      // self-attest | peer-confirm | M-of-N-vote | audit-signature | computational
  "evidence_json": {
    "observation_refs": ["libp2p://...", "bafkrei..."],   // pointers to operational evidence (Observation layer)
    "observation_period_start": "2026-04-01T00:00:00Z",
    "observation_period_end": "2026-05-01T00:00:00Z",
    "summary_metric": { /* free-form, per-subtype */ }
  },
  "proof_evidence": {
    "class": "witness",                     // witness | audit | proof | confirmation (compute-attestation gradient)
    "issuer_signature": "<inherited from Content action header>"
    // For higher classes: merkle_root, zkml_proof, multi_attestor_chain (per compute-attestation spec)
  },
  "expires_at": "2027-05-11T00:00:00Z",     // optional
  "revocation": null,                        // populated on revocation (see §3.4)
  "parent_governance_action_cid": null      // populated when this is a child vote (see §4)
}
```

**proof_evidence** defaults to `class: "witness"` (signed by issuer, no further proof). Higher classes (audit/proof/confirmation) are demand-driven escalations per the compute-attestation spec — they're optional, attached when the threat model requires them. Most attestations stay at witness.

### 3.4 Lifecycle

- **Issuance**: `content_store::issue_attestation(input)` creates a Content entry with `content_type: "attestation:<subtype>"`, validates against the subtype's manifest-declared schema, creates the `AttestationToSubject` link, emits the post-commit signal.
- **Revocation**: A NEW Content entry with `content_type: "attestation:<subtype>"` is issued where `metadata_json.revocation = { reason, revoked_at, supersedes_cid }`. The old attestation is not mutated (append-only DHT). Storage projection joins on `supersedes_cid` to surface current status.
- **Expiration**: Computed by the projection layer based on `metadata_json.expires_at`. Expired attestations remain on the DHT but are filtered out of "current capability" queries.
- **Querying**: Storage projection table `attestations` (Category C operational; rebuildable from signal stream) indexes attestation Content entries by subtype, subject, issuer, status. The HTTP route `GET /api/v1/attestations` serves this projection.

### 3.5 Validator floors (integrity zome) — summary

The Content validator handles attestation contentTypes uniformly through a discriminator-chain. Floors are summarized here; the formal enumeration with edge cases lives in §9.

1. **Subtype known** — `content_type` must match a declared subtype in some pillar manifest.
2. **Issuer authorized** — for some subtypes, the issuer must satisfy an authorization predicate declared in the manifest.
3. **Subject link present** — every attestation has exactly one `AttestationToSubject` link committed in the same action.
4. **Uniqueness anchor** (where the subtype declares one) — at most one attestation per (issuer, parent, kind) anchor.
5. **Temporal validity** — `expires_at` in the future; child `created_at` ≤ parent's `closes_at`.
6. **Eligibility predicate** (M-of-N children only) — issuer satisfies the parent's eligibility predicate.
7. **Revocation reference valid** — if `metadata_json.revocation.supersedes_cid` is set, it must resolve to a same-kind attestation by the same issuer (via `must_get`).
8. **Proof class declared** — `metadata_json.proof_evidence.class` ∈ `{witness, audit, proof, confirmation}`; higher classes require matching proof material.

Floors that DO NOT belong in the integrity zome (cross-DNA queries or operational state, run in coordinator or projection layer):
- Standing-based eligibility (cross-DHT walk)
- Quorum tally (projection layer, Category C)
- Revocation propagation (projection layer)

Validators that DO NOT belong in the integrity zome (because they involve cross-DNA queries or operational state):
- Standing-based eligibility (involves cross-DHT walk; runs in coordinator zome)
- Quorum tally (runs in the projection layer — Category C)
- Revocation propagation (runs in the projection layer)

## 4. Multi-party (M-of-N) attestations

### 4.1 Pattern

The current code has stateful M-of-N entries (RenewalAttestation with `votes_json + current_approvals + status` as mutable fields). Holochain's append-only model makes this awkward; the current implementation relies on `update_entry`, which centralizes update authority on the entry owner.

The new pattern decomposes:

- **Parent**: a Content entry with `content_type: "governance-action:<kind>"` declares the proposal — threshold, eligibility predicate, ballot format, closes_at. Immutable after publish.
- **Children**: each vote is its own attestation Content entry with `content_type: "attestation:<vote-kind>"` and `metadata_json.parent_governance_action_cid` pointing to the parent. Each child carries its own `vote_value` (approve | reject | abstain) in `metadata_json`.
- **Tally**: a derived SQLite projection (`governance_action_tally`, Category C) reads parent + children and computes `pending | reached-quorum | witnessed | failed-quorum | closed-no-decision`. Rebuildable any time from the DHT.

### 4.2 Parent governance-action shape

```jsonc
// content_type: "governance-action:renewal-request" (or :key-revocation, :election, :gate-challenge, etc.)
{
  "id": "bafyrei...",
  "title": "Renewal request for human_id 0x...",
  "metadata_json": {
    "governance_kind": "renewal-request",
    "subject_cid": "bafyrei...",              // who/what the action concerns
    "threshold": { "type": "m-of-n", "m": 3, "n": 5 },
    "eligibility_predicate": {
      "type": "manifest-defined",
      "manifest_ref": "imagodei:custodian-eligibility-v1"
    },
    "ballot_format": "approve-reject",        // or ranked-choice, approval, quadratic, etc.
    "closes_at": "2026-05-25T00:00:00Z",
    "parameters_json": { /* governance-kind-specific */ }
  }
}
```

### 4.3 Child attestation shape

```jsonc
// content_type: "attestation:renewal-approval" (or :revocation-vote, :election-vote, etc.)
{
  "id": "bafyrei...",
  "title": "Approval for renewal of human_id 0x...",
  "metadata_json": {
    "attestation_kind": "renewal-approval",
    "subject_cid": "bafyrei... (same as parent's subject)",
    "subject_kind": "agent",
    "validation_method": "M-of-N-vote",
    "parent_governance_action_cid": "bafyrei... (parent)",
    "vote_value": "approve",
    "vote_weight": null,                       // optional, for weighted voting
    "evidence_json": { /* per-kind */ },
    "proof_evidence": { "class": "witness" }
  }
}
```

### 4.4 Tally projection (Category C)

```sql
CREATE TABLE governance_action_tally (
  parent_cid TEXT PRIMARY KEY,
  governance_kind TEXT NOT NULL,
  subject_cid TEXT NOT NULL,
  threshold_m INTEGER NOT NULL,
  threshold_n INTEGER NOT NULL,                -- if applicable (some thresholds are percentage-based)
  closes_at TEXT NOT NULL,
  current_approve_count INTEGER NOT NULL,
  current_reject_count INTEGER NOT NULL,
  current_abstain_count INTEGER NOT NULL,
  computed_status TEXT NOT NULL,               -- pending | reached-quorum | witnessed | failed-quorum | closed-no-decision
  last_child_at TEXT NOT NULL,
  rebuilt_at TEXT NOT NULL                     -- for debugging projection drift
);
-- Reconstruction: replay attestation signal stream filtered by parent_governance_action_cid IS NOT NULL,
-- group by parent_cid, apply ballot-format rules from parent's metadata.
```

### 4.5 Vote modification

EPRs are append-only; vote modification is handled by issuing a NEW child attestation with the same `(issuer, parent)`. The tally projection uses **latest-by-timestamp** semantics per `(issuer, parent)`. Older child attestations remain on the DHT but are superseded in the projection.

The integrity zome ALLOWS multiple children from the same issuer for the same parent (so vote-change is possible pre-deadline); the tally enforces single-vote-per-issuer in the projection layer. This matches how revocation supersedes older attestations.

### 4.6 Vote anonymity

Out of scope. The current entry types don't support anonymous voting either (every vote is signed by the action author). When secret-ballot governance forms are needed, a sibling primitive (commit-then-reveal with cryptographic commitments) will be designed separately. Nothing in this spec precludes that addition.

## 5. Recovery protocol decoupling

The current recovery flow conflates social M-of-N (custodian approvals) with cryptographic M-of-N (Shamir share assembly). This spec separates them:

| Concern | Current shape | New shape |
|---|---|---|
| **Recovery request declaration** | `RecoveryRequest` entry (imagodei DNA) with `custodian_ids` + `threshold` + mutable `status` | Content entry with `content_type: "governance-action:recovery-request"` (elohim DNA); parameters in `metadata_json`; immutable |
| **Custodian approval** | `RecoveryVote` entry (imagodei DNA) with `approval` field, single per custodian, mutable | Content entry with `content_type: "attestation:recovery-approval"` (elohim DNA); child of the governance-action parent; immutable; latest-wins for vote-change |
| **Quorum status** | Stored in `RecoveryRequest.status`; updated via `update_entry` | Derived in `governance_action_tally` projection |
| **Shamir share material** | Currently embedded in `RecoveryVote.share_data` field (DHT-visible) | **Removed from DHT.** Shares travel via signed libp2p direct message from each approving custodian to the recovery agent, gated by the custodian's published recovery-approval attestation. Off-chain transport; on-chain authorization. |
| **Recovery completion** | Implicit when status field flips | Explicit: a final `attestation:recovery-completed` Content entry from the recovery agent, citing the parent + the N approval children + a content-hash of the reconstructed identity binding |

This decoupling means:
- Cryptographic libs (Shamir share split/combine) become a pluggable utility class, not entangled with the DHT entity model
- The social-threshold question ("did M custodians approve?") becomes a clean projection over attestation children
- Shamir is no longer mandatory — recovery flows that don't need it (e.g., low-stakes recovery via simple custodian co-signing) can use the same governance-action + attestation-children pattern without Shamir at all
- Higher-security recovery flows (Shamir-protected high-value identities) compose Shamir ON TOP of the same primitive

The full recovery protocol redesign is a follow-up spec; this spec defines only the data-shape changes needed for the consolidation.

## 6. Manifest layer

Each pillar's manifest declares the attestation subtypes it owns, their metadata schemas, and their authorization predicates. The CONTENT_TYPES generated enum extends to include all declared subtypes (via existing schema codegen pipeline).

### 6.1 Declaration shape (per pillar manifest)

```jsonc
{
  "manifestKind": "lamad",
  "attestations": {
    "attestation:mastery": {
      "description": "Recognition that a learner has demonstrated mastery of a concept",
      "subject_kinds": ["agent"],
      "metadata_schema": { "$ref": "./schemas/mastery-attestation-metadata.schema.json" },
      "authorization_predicate": {
        "type": "issuer-has-attestation",
        "attestation_kind": "attestation:steward",
        "scope": "concept-domain"
      },
      "uniqueness_anchor": "Anchor('mastery:{subject_cid}:{concept_cid}:{issuer_cid}')",
      "default_expiration_days": null,
      "revocable_by": ["issuer", "domain-steward"]
    },
    "attestation:content-quality": {
      // ... similar shape
    }
  }
}
```

### 6.2 Subtype catalog (initial set)

Migrated from existing entry types. Each row maps to a single attestation subtype declared in one pillar manifest.

| Subtype | Pillar manifest | Replaces |
|---|---|---|
| `attestation:humanness` | imagodei | `HumanityWitness` |
| `attestation:identity-credential` | imagodei | `Attestation` (imagodei + vestigial elohim) |
| `attestation:key-stewardship` | imagodei | `KeyStewardship` |
| `attestation:stewardship-grant` | imagodei | `StewardshipGrant` |
| `attestation:stewardship-appeal` | imagodei | `StewardshipAppeal` |
| `attestation:policy-inheritance` | imagodei | `PolicyInheritance` |
| `attestation:identity-freeze` | imagodei | `IdentityFreeze` (becomes derived attestation from challenge children) |
| `attestation:renewal-approval` | imagodei | `RenewalAttestation`'s votes (now children) |
| `attestation:recovery-approval` | imagodei | `RecoveryVote` |
| `attestation:revocation-vote` | imagodei | `RevocationVote` |
| `attestation:challenge-support` | imagodei | `ChallengeSupport` |
| `attestation:mastery` | lamad | (new — was conflated in `ContentMastery` + imagodei `Attestation`) |
| `attestation:content-quality` | lamad | `ContentAttestation` |
| `attestation:content-succession` | lamad | `ContentSuccession` |
| `attestation:custodian-commitment` | lamad | `CustodianCommitment` |
| `attestation:device-health` | infrastructure | `HealthAttestation` (also: heartbeat summarization moves to attestation; raw heartbeats move to observation layer) |
| `attestation:doorway-health-summary` | infrastructure | `DoorwayHeartbeatSummary` |
| `attestation:governance-role` | mishpat | (new — was implicit in challenge/proposal flows) |
| `attestation:gate-decision` | mishpat | `GateDecisionAttestation` |
| `attestation:proposal-vote` | mishpat | `ProposalVote` |
| `attestation:statement-vote` | mishpat | `StatementVote` |
| `attestation:governance-reaction` | mishpat | `GovernanceReaction` |

### 6.3 Governance-action subtype catalog

| Subtype | Pillar manifest | Replaces |
|---|---|---|
| `governance-action:renewal-request` | imagodei | `RenewalAttestation` (the request portion; votes become children) |
| `governance-action:recovery-request` | imagodei | `RecoveryRequest` |
| `governance-action:key-revocation` | imagodei | `KeyRevocation` |
| `governance-action:identity-challenge` | imagodei | `IdentityChallenge` |
| `governance-action:proposal` | mishpat | `Proposal` |
| `governance-action:challenge` | mishpat | `Challenge` + `GateDecisionChallenge` |
| `governance-action:election` | mishpat | (new — capability previously not modeled) |

### 6.4 Out of scope (not attestation-shaped)

These existing entry types are NOT attestations and remain as-is (or move to observation layer per §2):

| Entry type | DNA | Why not consolidated |
|---|---|---|
| `OpinionStatement` | mishpat | Observation-shaped (Polis input). Will move to observation layer. |
| `Discussion` | mishpat | Container/anchor; stays as-is. |
| `Precedent` | mishpat | Could become `attestation:precedent` in follow-up; out of scope here. |
| `GovernanceState` | mishpat | Derived projection; moves to operational SQLite (already projection-shaped). |
| `ChallengeOutcome` | mishpat | Derived result; becomes projection of challenge governance-action tally. |
| `GraduatedFeedback` | mishpat | Already FeedbackSignal-shaped; aligns with that primitive's extensibility, not this one. |
| `ActivityLog` | imagodei | Observation-shaped. Will move to observation layer. |
| `DoorwayHeartbeat` | infrastructure | Observation-shaped. Will move to observation layer. |

## 7. Migration plan (pre-launch hard cutover)

Pre-launch makes data migration tractable — no user data to preserve. The migration is a coordinated change across DNAs + storage + Angular + manifests.

### 7.1 Stage A — Manifest declarations

1. Author the attestation-subtype JSON schemas (per §6.2) and add them to pillar manifests.
2. Extend the schema codegen pipeline to emit the consolidated CONTENT_TYPES enum from manifest declarations.
3. Add `content_type` discriminator validation rules to the codegen-produced Rust constants.
4. Verify `pnpm run schema:test && pnpm run schema:validate` passes.

### 7.2 Stage B — Coordinator zomes

1. In elohim DNA's `content_store` coordinator zome, add:
   - `issue_attestation(input: IssueAttestationInput) -> AttestationOutput`
   - `revoke_attestation(input: RevokeAttestationInput) -> AttestationOutput`
   - `propose_governance_action(input: GovernanceActionInput) -> GovernanceActionOutput`
   - `vote_on_governance_action(input: VoteInput) -> AttestationOutput` (creates a child attestation Content)
   - `get_attestations_for_subject(subject_cid: String) -> Vec<AttestationOutput>`
   - `get_governance_action_with_children(parent_cid: String) -> GovernanceActionWithChildren`
2. Delete the imagodei DNA coordinator functions for the 11 imagodei attestation-shaped types. Replace with cross-DNA bridge calls into elohim DNA (via `CallTargetCell::OtherRole(CONTENT_STORE_ROLE)`).
3. Delete the infrastructure DNA coordinator functions for HealthAttestation. Replace with cross-DNA bridge call (volume is low because raw heartbeats move to observation layer, summaries are infrequent).
4. Delete the mishpat DNA coordinator functions for ProposalVote, StatementVote, GateDecisionAttestation, GovernanceReaction, Proposal, Challenge, GateDecisionChallenge. Replace with cross-DNA bridge calls.

### 7.3 Stage C — Integrity zomes

1. Remove the deleted entry types from each DNA's integrity zome (per §6.2 + §6.3 catalogs).
2. Remove the corresponding LinkTypes that referenced them (e.g., `AgentToAttestation`, `AttestationByCategory`, `AttestationByType` in imagodei). Replace with `AttestationToSubject` + anchor-based discovery in elohim DNA.
3. Extend elohim DNA's `content_store_integrity` Content validator with the discriminator-chain logic from §3.5.

### 7.4 Stage D — Storage projection

1. Drop the existing per-entry-type tables: `attestations` (imagodei), `humanity_witnesses`, `key_stewardships`, `stewardship_grants`, `renewal_attestations`, `recovery_requests`, `recovery_votes`, `identity_challenges`, `challenge_supports`, `key_revocations`, `revocation_votes`, `identity_freezes`, `stewardship_appeals`, `policy_inheritances`, `content_attestations`, `custodian_commitments`, `content_successions`, `health_attestations`, `doorway_heartbeat_summaries`, `gate_decision_attestations`, `proposal_votes`, `statement_votes`, `governance_reactions`.
2. Create the unified `attestations` table:
   ```sql
   CREATE TABLE attestations (
     id TEXT PRIMARY KEY,                       -- CID
     dht_anchor_hash BLOB NOT NULL,             -- Content EntryHash
     attestation_kind TEXT NOT NULL,            -- denormalized from content_type
     subject_cid TEXT NOT NULL,
     subject_kind TEXT NOT NULL,
     issuer_cid TEXT NOT NULL,                  -- denormalized from Content.author_id
     parent_governance_action_cid TEXT,         -- NULL for unilateral; set for M-of-N children
     vote_value TEXT,                           -- NULL for non-vote attestations
     vote_weight TEXT,                          -- optional
     proof_class TEXT NOT NULL,                 -- witness | audit | proof | confirmation
     proof_evidence_json TEXT NOT NULL,         -- full proof payload
     evidence_json TEXT NOT NULL,
     expires_at TEXT,
     supersedes_cid TEXT,                       -- for revocations
     created_at TEXT NOT NULL,
     manifest_ref TEXT NOT NULL                 -- which pillar manifest declares this subtype
   );
   CREATE INDEX attestations_subject ON attestations(subject_cid, attestation_kind);
   CREATE INDEX attestations_issuer ON attestations(issuer_cid);
   CREATE INDEX attestations_parent ON attestations(parent_governance_action_cid);
   ```
3. Create the `governance_actions` and `governance_action_tally` tables (per §4.4 for tally).
4. Update post-commit signal handlers to ingest attestation Content entries into the unified table.

### 7.5 Stage E — HTTP API + storage-client

1. Add HTTP routes:
   - `POST /api/v1/attestations` — calls `issue_attestation` coordinator
   - `GET /api/v1/attestations?subject={cid}&kind={subtype}` — queries projection
   - `POST /api/v1/attestations/{id}/revoke` — calls `revoke_attestation`
   - `POST /api/v1/governance-actions` — calls `propose_governance_action`
   - `GET /api/v1/governance-actions/{id}` — returns parent + tally
   - `POST /api/v1/governance-actions/{id}/vote` — calls `vote_on_governance_action`
2. Delete the per-type HTTP routes that previously served these entities (per existing route inventory in doorway-service and elohim-storage api/).
3. Regenerate TypeScript types via `cargo test export_bindings`.

### 7.6 Stage F — Angular consumers

1. Update Angular services that called the deleted per-type endpoints to call the unified ones. Migration shape: replace e.g. `AttestationService.issueAttestation(category, type, ...)` with `AttestationService.issue({ kind: 'identity-credential', subject, ... })`.
2. Update Storybook entries and component fixtures.
3. Update a2o feature files that referenced the deleted entry types.

### 7.7 Stage G — Recovery protocol decoupling

1. Implement Shamir share off-chain transport: signed libp2p direct message from custodian to recovery agent. Reference the published recovery-approval attestation as authorization.
2. Update recovery agent to assemble shares from libp2p messages keyed by the recovery-approval attestation children.
3. Update the recovery flow's UI to derive status from `governance_action_tally` instead of polling a mutable RecoveryRequest entry.

### 7.8 Stage ordering and pacing

| Order | Stage | Dependencies |
|---|---|---|
| 1 | A — Manifest declarations | none |
| 2 | B — Coordinator zomes | A |
| 3 | C — Integrity zomes | A (uses generated CONTENT_TYPES) |
| 4 | D — Storage projection | A, B (signal types) |
| 5 | E — HTTP API + storage-client | D |
| 6 | F — Angular consumers | E (TS types regenerated) |
| 7 | G — Recovery protocol decoupling | B, D, F (full stack ready) |

PVC + DNA build budget must be honored — Stages B, C, D each touch large compiled surfaces; do not parallelize cargo builds across worktrees per existing project pacing.

## 8. Wave 0 integration

This spec **supersedes** the Wave 0 plan's current Attestation dedupe direction (Option B path in `2026-05-11-tiered-quilt-wave-0-substrate-cleanup.md` §⚠️).

The Wave 0 plan's two stages become:

- **Stage A (Attestation dedupe)**: REPLACED. Wave 0 now executes Stages A–F of this spec (manifest declarations + coordinator zomes + integrity zomes + storage + HTTP + Angular). Stage G of this spec (recovery decoupling) MAY be deferred to a follow-up wave if scope grows; the recovery flow continues to work via the unified primitive even before Shamir share off-chain transport is implemented (shares can remain in the child attestation `metadata_json.evidence_json` temporarily).
- **Stage B (rename `lamad_event_type` → `elohim_event_type`)**: UNCHANGED. Still runs after the Attestation consolidation lands.

The Wave 0 plan must be updated to:
1. Cite this spec as the source-of-truth direction
2. Replace the Option A vs Option B decision block with "executes the attestation-consolidation spec"
3. Expand the file-structure map to reflect the broader cross-DNA changes
4. Update the pacing notes — broader compile surface means tighter PVC discipline

## 9. Validator floors — formal enumeration

The eight floors summarized in §3.5 are detailed here for the implementation plan. Each floor specifies its trigger, the exact check, and the failure mode.

1. **Subtype known.** Trigger: every commit of a Content entry where `content_type` starts with `"attestation:"` or `"governance-action:"`. Check: lookup `content_type` in the loaded manifest registry; reject if not declared. Failure mode: fail-closed validation error with `unknown_attestation_subtype` code.

2. **Issuer authorized.** Trigger: subtypes whose manifest declaration includes `authorization_predicate`. Check: evaluate the predicate (e.g., `issuer-has-attestation`, `issuer-is-domain-steward`) against current DHT state. Failure mode: validation error with `issuer_not_authorized`.

3. **Subject link present.** Trigger: every attestation commit. Check: same action carries exactly one `AttestationToSubject` link from the new EntryHash to a resolvable subject EntryHash. Failure mode: `subject_link_missing` or `subject_link_count_invalid`.

4. **Uniqueness anchor.** Trigger: subtypes whose manifest declaration includes `uniqueness_anchor`. Check: lookup the declared anchor; reject if more than one attestation links from it. Failure mode: `duplicate_attestation_anchor`. NOTE: vote-modification still works because `parent_governance_action_cid` is part of the anchor; the tally projection's latest-wins logic supersedes within the same anchor.

5. **Temporal validity.** Trigger: every attestation commit. Check: if `metadata_json.expires_at` is set, parse + verify it's in the future relative to action timestamp; if `metadata_json.parent_governance_action_cid` is set, resolve parent via `must_get`, parse `metadata_json.closes_at`, verify child timestamp ≤ parent closes_at. Failure mode: `expired_at_commit` or `child_after_parent_close`.

6. **Eligibility predicate (M-of-N children).** Trigger: child attestations (those with `parent_governance_action_cid` set). Check: resolve parent's `eligibility_predicate`, evaluate against issuer. Failure mode: `ineligible_voter`.

7. **Revocation reference valid.** Trigger: attestations with `metadata_json.revocation.supersedes_cid` set. Check: resolve the referenced CID via `must_get`; verify it's an attestation of the same `attestation_kind` by the same `issuer_cid`. Failure mode: `revocation_target_invalid`.

8. **Proof class declared.** Trigger: every attestation commit. Check: `metadata_json.proof_evidence.class` must be one of `witness | audit | proof | confirmation`. Higher classes additionally require their proof material to be present (`merkle_root` for audit, `zkml_proof` for proof, `multi_attestor_chain` for confirmation). Witness is the default and requires only the inherited issuer signature. Failure mode: `proof_class_invalid` or `proof_material_missing`.

## 10. Out of scope

- **Observation layer design** (libp2p/iroh-shaped operational data) — own spec, sibling to this one
- **Anonymous voting / secret ballot** — separate primitive (commit-then-reveal), not blocked by this spec
- **Per-subtype metadata schemas** — declared in each pillar manifest; this spec defines the framework, not the specific schemas
- **Full recovery protocol redesign** — only the data-shape decoupling is in scope; the broader flow is a follow-up
- **Coordinator zome splitting** (potentially splitting `content_store` into `content_store + attestation_store + governance_store`) — tactical concern, can be deferred
- **Migration to non-pre-launch (post-launch data migration)** — not applicable; we're pre-launch

## 11. Open questions

1. **`Standing` evidence stream**: today, Standing reads from `imagodei/lamad + FeedbackSignal debits`. After consolidation, attestations all live in elohim DNA. Does Standing's `services::standing::Standing::evaluate` need any reshaping, or does the consolidation make Standing's life simpler? Likely simpler (single signal stream instead of multiple), but worth verifying during implementation.
2. **`ContentMastery` overlap with `attestation:mastery`**: today `ContentMastery` is its own entry type carrying mastery state. Does it stay separate (as an agent's private progress record per Category B2) and emit `attestation:mastery` as the public proof? Likely yes — same pattern as today's "mastery is private + attestation is public" but with consolidated attestation shape.
3. **Cross-DNA bridge performance audit**: confirm that the post-consolidation cross-DNA call frequency from imagodei/mishpat/infrastructure → elohim DNA is bounded (the design assumes it's low because observations move off-DHT). If specific cases produce high cross-DNA volume, surface them as candidates for federated (per-DNA Content entry) variants.
4. **Pillar boundary violations during migration**: the existing 174 pillar boundary violations memo (`project_pillar_boundary_violations_backlog.md`) may grow during this migration because Angular services will need to read from elohim DNA via cross-pillar imports. Acceptable short-term; dedicated cleanup sprint already on backlog.

## 12. Success criteria

- [ ] All 18+ attestation-shaped entry types removed from their respective DNAs (per §6.2 + §6.3)
- [ ] One new content_type discriminator (`attestation:*`) and one new governance-action discriminator (`governance-action:*`) declared via pillar manifests
- [ ] One unified `attestations` projection table; one `governance_actions` + `governance_action_tally` projection table
- [ ] Coordinator zome surface: 6 new functions in elohim DNA `content_store`; ~30 functions removed from other DNAs (replaced by cross-DNA bridge calls)
- [ ] HTTP API surface: 6 new routes; ~25 routes deleted
- [ ] Recovery protocol's mutable status disappears — status derives from `governance_action_tally`
- [ ] Shamir share material removed from any DHT entry (off-chain transport only)
- [ ] DNA capacity: elohim ~77 → ~67 (after net additions for governance-action handlers); imagodei ~31 → ~17; infrastructure ~7 → ~5; mishpat ~15 → ~10
- [ ] All a2o feature files referencing deleted entry types updated
- [ ] Wave 0 plan updated to reference this spec

---

**Authoring note for the implementation plan**: This spec is the source-of-truth for the consolidation direction. The implementation plan (to be authored next, per the brainstorming skill's handoff to `writing-plans`) will sequence the seven stages (A–G) into concrete tasks with file paths, test points, and commit boundaries. The implementation plan inherits the Wave 0 plan's pacing constraints (single worktree, sequential cargo builds, PVC discipline) and feeds back into the Wave 0 plan's master schedule.
