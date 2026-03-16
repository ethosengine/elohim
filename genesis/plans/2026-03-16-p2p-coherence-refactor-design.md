# P2P Coherence Refactor — Storage Provenance & Entity Reclassification

**Date:** 2026-03-16
**Status:** Approved
**Problem:** 48 SQLite tables in elohim-storage but only 4 have `dht_anchor_hash`. The DHT entry types exist for most notarized entities — the provenance link from storage back to the notary is missing. This undermines the protocol's core promise: verifiable, P2P-native data. The storage layer has drifted into a standalone relational database instead of functioning as a projection of DHT truth.

**Approach:** Full P2P-native rewrite of the storage layer. Every entity reclassified using the 5-category system (A/A2/B/B2/C). Every notarized entity gets `dht_anchor_hash`. Slugs become aliases. Storage becomes purely a projection service. No new DHT entry types needed for most entities — the gap is the provenance link, not the entry type.

**Constraint:** Holochain DHT is a notary, not a database. Lamad DNA is at 83/~100 entry types. Do not add entry types without confirming headroom. Entries must be <1KB (proofs only). DHT chokes at ~3000 entries. Each sprint may discover entry types that can be collapsed, freeing headroom.

## Sprint Structure

4 sprints, one pillar each, vertical slice (migration → model → view → TypeScript → Angular). Each sprint leaves its pillar fully coherent. Sprints execute sequentially. Per the iterative sprint planning preference: write all 4 plan outlines upfront, execute one at a time, revisit and update the next plan after each sprint.

## Sprint 1: Shefa (Economics) Coherence

**Goal:** Complete provenance wiring for the economics pillar. 4 tables already anchored — finish the remaining 5.

### Entity Classifications

| Table | Classification | Entry Type | Anchor Strategy |
|---|---|---|---|
| economic_events | A (Notarized) ✅ | EconomicEvent | Already has dht_anchor_hash |
| rea_commitments | A (Notarized) ✅ | Commitment | Already has dht_anchor_hash |
| agreements | A (Notarized) ✅ | Agreement | Already has dht_anchor_hash |
| stewarded_nodes | A (Notarized) ✅ | StewardedResource | Already has dht_anchor_hash |
| stewardship_allocations | A2 (Derived) | Link on Agreement | Anchor = parent Agreement's ActionHash |
| steward_credentials | A (Notarized) | Attestation (imagodei) | Anchor = Attestation ActionHash, type=credential |
| access_grants | A (Notarized) | Attestation (imagodei) | Anchor = Attestation ActionHash, type=access |
| premium_gates | A (Notarized) | Link on Content | Anchor = parent Content's ActionHash |
| steward_affinity | C (Operational) | N/A | No anchor — derived from curation acts, reconstructable |

### DNA Cleanup Opportunity
steward_credentials and access_grants both map to imagodei Attestation with different type discriminators. If confirmed, no new entry types needed — just different attestation_type values on the existing type.

### Deliverables
- Migrations: `dht_anchor_hash` on 4 tables, source-of-truth comments on all 9 shefa tables
- Reclassify steward_affinity as C (operational) with documented reconstruction strategy
- Model/View/TypeScript regen for changed structs
- Audit the 4 already-anchored tables for correct end-to-end provenance wiring
- Prove the full vertical migration pattern for subsequent sprints

---

## Sprint 2: Lamad (Content) Coherence

**Goal:** Wire provenance for the highest-traffic data path. Reclassify content_mastery as B2 (agent-scoped + attestation).

### Entity Classifications

| Table | Classification | Entry Type | Anchor Strategy |
|---|---|---|---|
| content | A (Notarized) | Content (lamad DNA) | Anchor = Content ActionHash |
| paths | A (Notarized) | LearningPath (lamad DNA) | Anchor = LearningPath ActionHash |
| chapters | A2 (Derived) | PathChapter (linked from LearningPath) | Anchor = parent LearningPath ActionHash |
| steps | A2 (Derived) | PathStep (linked from PathChapter) | Anchor = parent chain (LearningPath → Chapter → Step) |
| relationships | A (Notarized) | Relationship (lamad DNA) | Anchor = Relationship ActionHash |
| content_mastery | B2 (Agent-Scoped + Attestation) | ContentMastery (imagodei) | Raw progress: private. Threshold crossing: Attestation anchor |
| content_attestations | A (Notarized) | Attestation (imagodei DNA) | Anchor = Attestation ActionHash |
| knowledge_maps | B (Agent-Scoped) or C (Operational) | TBD during sprint | Decide: personal sensemaking (B) or governance-weighted (A) |

### Key Design Decision: content_mastery as B2
Raw mastery progress (every quiz attempt, every time-on-content event) stays on the agent's private source chain. When mastery crosses a threshold that gates governance participation or stewardship eligibility, the system issues a public Attestation (imagodei DNA). Storage projection tracks both: private mastery for the learner's UI, and dht_anchor_hash of the attestation for verifiable gating.

### DNA Cleanup Opportunity
PathChapter and PathStep exist as standalone entry types in lamad DNA. If they can collapse into Link metadata on LearningPath (the path defines its structure via typed links, each link carries chapter/step metadata in the tag), that frees 2 entry types from Lamad's 83 budget. Audit during sprint.

### Deliverables
- Migrations: `dht_anchor_hash` on 6 tables, source-of-truth comments on all 8
- Reclassify content_mastery as B2, knowledge_maps as B or C
- Audit PathChapter/PathStep collapsibility (DNA cleanup)
- Model/View/TypeScript regen
- Angular content services updated to pass through dhtAnchorHash

---

## Sprint 3: Imagodei (Identity) Coherence

**Goal:** Anchor the trust root. After this sprint, every author, relationship, and credential is cryptographically verifiable.

### Entity Classifications

| Table | Classification | Entry Type | Anchor Strategy |
|---|---|---|---|
| humans | A (Notarized) | Human (imagodei DNA) | Anchor = Human ActionHash |
| human_relationships | A (Notarized) | HumanRelationship (imagodei DNA) | Anchor = HumanRelationship ActionHash |
| contributor_presences | A (Notarized) | ContributorPresence (imagodei DNA) | Anchor = ContributorPresence ActionHash |
| path_attestations | A (Notarized) | Attestation (imagodei DNA) | Anchor = Attestation ActionHash |

### Trust Chain Completion
After this sprint: content (Sprint 2) → authored by verified human (Sprint 3) → stewardship claims (Sprint 1) trace to verified relationships (Sprint 3) → credentials (Sprint 1) anchored to verified attestations (Sprint 3). The full provenance chain is complete.

### DNA Cleanup Opportunity
Imagodei DNA (28/~100) has headroom. But audit whether:
- ContentMastery, ContributorPresence, and Attestation have meaningfully different validation rules, or if they can collapse into a generalized Attestation with type discriminator. Potential to free 2 entry types.
- The 14 identity immune system entries (RecoveryRequest, RecoveryVote, HumanityWitness, etc.) are correctly DHT-only with no missing storage projections. Confirm this is intentional.

### Deliverables
- Migrations: `dht_anchor_hash NOT NULL` on all 4 tables, source-of-truth comments
- Backfill strategy for existing rows (query conductor for ActionHashes matching each entity id)
- Model/View/TypeScript regen
- Angular IdentityService/PresenceService updated for provenance
- Audit imagodei DNA for collapsible entry types

---

## Sprint 4: Qahal (Governance) Coherence

**Goal:** Anchor the governance pillar. Establish the B2 (ballot secrecy + verifiable tally) pattern at scale.

### Entity Classifications

| Table | Classification | Entry Type | Anchor Strategy |
|---|---|---|---|
| proposals | A (Notarized) | GovernanceSignal family (lamad DNA) | Anchor = proposal ActionHash |
| proposal_options | A2 (Derived) | Link on proposal | Anchor = parent proposal ActionHash |
| votes | B2 (Agent-Scoped + Attestation) | Private ballot + Attestation tally | Raw vote: private. Tally: Attestation anchor |
| ranked_votes | B2 (Agent-Scoped + Attestation) | Private ballot + Attestation tally | Same B2 pattern as votes |
| governance_signals | B2 (Agent-Scoped + Attestation) | Private reaction + aggregate | Signal: private. Aggregate: Attestation anchor |
| governance_states | A2 (Derived) | Link metadata on proposal | Anchor = parent proposal ActionHash |
| challenges | A (Notarized) | Audit — may be Attestation-shaped | Anchor = challenge ActionHash or Attestation |
| appeals | A (Notarized) | Audit — may derive from challenge | Anchor = appeal ActionHash or Attestation |
| statements | A (Notarized) | Audit — Polis sensemaking | Anchor = statement ActionHash |
| statement_votes | B2 (Agent-Scoped + Attestation) | Private stance + cluster aggregate | Same B2 pattern |
| precedents | A (Notarized) | Audit — governance memory | Anchor = precedent ActionHash |

### Key Design Decision: Ballot Secrecy Levels
- **Secret ballot** (formal governance, levels 3-7): Raw vote is B (private, never revealed). Tally attestation is A (notarized aggregate, no individual votes exposed).
- **Open signal** (casual feedback, levels 0-2): Raw signal is B2 (private but attestation reveals the stance). Aggregate is A2 (derived from collection of attestations).

Discriminator: `governance_signals.signal_level` or governance mechanism type, not separate tables.

### DNA Cleanup Opportunity — Major
Governance entries currently live in the lamad DNA (at 83/~100). Two paths:

**Option 1: Governance-as-Attestation collapse.** If proposals, challenges, appeals, and precedents can all be modeled as Attestations with different type discriminators, they collapse 4-5 entry types into the existing Attestation shape. This frees major headroom in lamad.

**Option 2: Governance DNA separation.** If governance needs its own validation rules (e.g., quorum enforcement, constitutional limits), it may warrant its own DNA. This frees lamad headroom AND gives governance isolated validation. Cost: cross-DNA bridge for "vote on this content" flows.

Decision made during sprint based on validation rule analysis.

### Also Reclassify
- discussions → C (Operational) — reconstructable from thread references
- comments → C (Operational) — reconstructable or agent-scoped

### Deliverables
- Migrations: `dht_anchor_hash` on notarized tables, source-of-truth comments on all 11+
- Reclassify votes/signals as B2 with attestation pattern
- Design ballot secrecy levels
- Model/View/TypeScript regen
- Angular governance components updated for provenance
- DNA audit: governance-as-Attestation feasibility vs governance DNA separation
- Classify remaining tables (discussions, comments)

---

## Cross-Sprint Concerns

### Backfill Strategy
Existing rows have no `dht_anchor_hash`. Three approaches per entity:
1. **Query conductor** — for entities that were created through zome calls, the ActionHash exists on the DHT. Query by entity id, retrieve ActionHash, backfill.
2. **Re-create on DHT** — for entities that bypassed the conductor (imported directly to storage), create the DHT entry now and capture the ActionHash.
3. **Mark as pre-coherence** — for entities where neither approach works, add a sentinel value (e.g., `pre-coherence`) and let them be gradually replaced as the system runs.

### Migration Safety
- Add `dht_anchor_hash` as NULLABLE first
- Run backfill
- Alter to NOT NULL once populated (or accept nullable for pre-coherence rows)
- Each migration is a separate file, reversible via down.sql

### TypeScript Boundary
Every View struct that gains `dht_anchor_hash` triggers a TypeScript type regen (`cargo test export_bindings`). Angular adapters pass the field through — they never transform it. The field name in TypeScript is `dhtAnchorHash` (camelCase via serde).

### Testing
Each sprint includes:
- Rust unit tests: verify View conversion includes anchor hash
- API integration tests: verify routes return anchor hash
- Angular service tests: verify provenance field flows to components

### Success Criteria
After all 4 sprints:
- Every notarized entity in storage has `dht_anchor_hash` linking to its DHT proof
- Every table has a source-of-truth comment in its migration
- TypeScript types expose `dhtAnchorHash` for notarized entities
- Angular components can display provenance (even if UI is minimal initially)
- DNA entry type count has decreased or held steady (cleanup, not growth)
- The p2p-schema-audit hook produces zero warnings on all modified files
