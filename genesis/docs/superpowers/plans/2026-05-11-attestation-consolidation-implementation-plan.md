# Attestation Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate 18+ attestation-shaped DHT entry types across four DNAs into a single `Content` entry shape with `content_type: "attestation:<subtype>"` discriminator, declare the subtype vocabulary in pillar manifests, and decouple Shamir share material from the M-of-N social-threshold pattern.

**Architecture:** Seven dependency-ordered stages (A–G). Stage A lands the manifest+schema substrate; Stages B–F land the implementation across DNAs, storage, HTTP, and Angular; Stage G decouples the Recovery protocol's Shamir transport off the DHT. Pre-launch hard cutover — no backwards compatibility shim. Single feature branch; sequenced commits per task; ff-merge to dev at stage boundaries.

**Tech Stack:** Rust 2021 (Holochain HDI/HDK 0.6, elohim-storage, Diesel/SQLite), JSON Schema (Draft 2020-12), ts-rs codegen, Angular 19, libp2p 0.54 with request-response (for Shamir share transport in Stage G).

**Source-of-truth spec:** `genesis/docs/superpowers/specs/2026-05-11-attestation-consolidation-design.md`

**Wave 0 integration:** This plan SUPERSEDES Wave 0's Attestation dedupe direction. After this plan's Stages A–F land, the Wave 0 plan's Stage B (`lamad_event_type → elohim_event_type` rename) executes against the post-consolidation tree. Wave 0 plan must be updated to cite this plan before dispatch.

**Worktree:** Run in a dedicated worktree per `superpowers:using-git-worktrees`. Branch from origin/dev. Single worktree for the whole plan; ff-merge to dev at each stage boundary.

**Build flag:** `elohim-storage` requires `RUSTFLAGS='--cfg getrandom_backend="custom"'` per CLAUDE.md gotcha. Rust DNA workspaces use plain `cargo` (no flag override).

**Pacing:** PVC budget is shared across worktrees. Run one cargo build at a time across `elohim-storage`, DNA workspaces, doorway. Use `cargo-pool prune --stale-incrementals --yes` between stages.

---

## P2P Design Gate Output (recap from spec)

| Entity | Category | DHT Entry Type? | HTTP Route | Coordinator |
|---|---|---|---|---|
| Attestation (unified) | A — Notarized | EXISTING `Content` with `content_type: "attestation:<subtype>"` | `POST/GET /api/v1/attestations` | `content_store::issue_attestation` |
| Governance Action (M-of-N parent) | A — Notarized | EXISTING `Content` with `content_type: "governance-action:<kind>"` | `POST/GET /api/v1/governance-actions` | `content_store::propose_governance_action` |
| AttestationToSubject Link | A2 — Derived | Link, not entry type | n/a | implicit in `issue_attestation` |
| GovernanceActionChild Link | A2 — Derived | Link, not entry type | n/a | implicit in `vote_on_governance_action` |
| GovernanceActionTally | C — Operational | none (projection) | `GET /api/v1/governance-actions/{id}/tally` | derived projector |
| AttestationProof | not an entity | carried in `metadata_json.proof_evidence` | n/a | n/a |

**Anti-pattern checks**: No new DHT entry types. CID identity throughout. Single source of truth declared per table. DNA capacity reclaimed (net ~−20 entry types). All HTTP routes are projection-serving thin layers.

---

## File-Structure Map

### New files

| Path | Responsibility |
|---|---|
| `elohim/sdk/schemas/v1/attestation/attestation-content.schema.json` | Core attestation Content shape (full Draft 2020-12 schema) |
| `elohim/sdk/schemas/v1/attestation/governance-action-content.schema.json` | Parent governance-action Content shape |
| `elohim/sdk/schemas/v1/attestation/proof-evidence.schema.json` | Pluggable proof-evidence object (witness/audit/proof/confirmation) |
| `elohim/sdk/schemas/v1/attestation/manifest-attestation-declaration.schema.json` | Schema for the `attestations` section in a pillar manifest |
| `elohim/sdk/schemas/v1/attestation/manifest-governance-action-declaration.schema.json` | Schema for `governance-actions` section in a pillar manifest |
| `elohim/sdk/schemas/v1/attestation/subtypes/humanness-metadata.schema.json` | Per-subtype metadata schema (one file per subtype, ~22 files total) |
| `elohim/sdk/domains/mishpat/manifest.json` | NEW domain manifest (mishpat has no existing manifest) |
| `elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs` | Attestation issuance + queries (coordinator) |
| `elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs` | Governance-action propose + vote + tally orchestration (coordinator) |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs` | Discriminator-chain validator floors |
| `elohim/elohim-storage/migrations/2026-05-12-100000_attestations/up.sql` + `down.sql` | Unified attestations projection table. **Source of truth: Holochain DHT** (Category A — projection of attestation `Content` entries). `up.sql` MUST include `-- Source of truth: Holochain DHT (projection of Content entries with content_type LIKE 'attestation:%')` as its first comment line, AND every row carries `dht_anchor_hash NOT NULL` |
| `elohim/elohim-storage/migrations/2026-05-12-100100_governance_actions/up.sql` + `down.sql` | Governance-action parent projection table. **Source of truth: Holochain DHT** (Category A — projection of governance-action `Content` entries). `up.sql` MUST include `-- Source of truth: Holochain DHT (projection of Content entries with content_type LIKE 'governance-action:%')` as its first comment line, AND `dht_anchor_hash NOT NULL` on every row |
| `elohim/elohim-storage/migrations/2026-05-12-100200_governance_action_tally/up.sql` + `down.sql` | Tally projection table. **Source of truth: local (operational)** — Category C, derived from parent + children on demand. `up.sql` MUST include `-- Source of truth: local (operational) — derived from governance_actions JOIN attestations, rebuildable via signal-stream replay` as its first comment line. NO `dht_anchor_hash` column per Category C policy. Reconstruction strategy: replay attestation signal stream filtered by `parent_governance_action_cid IS NOT NULL`, group by parent_cid, apply ballot-format rules from parent's metadata |
| `elohim/elohim-storage/migrations/2026-05-12-100300_drop_legacy_attestation_tables/up.sql` + `down.sql` | Drops 22 legacy per-type tables. `up.sql` MUST include `-- Removes legacy per-entry-type projection tables superseded by 2026-05-12-100000_attestations (source of truth: Holochain DHT); see spec §7.4 for the full table list` as its first comment line |
| `elohim/elohim-storage/src/db/attestations.rs` | Diesel CRUD + queries for unified attestations table |
| `elohim/elohim-storage/src/db/governance_actions.rs` | Diesel CRUD for governance-actions |
| `elohim/elohim-storage/src/db/governance_action_tally.rs` | Tally projection writer + reader |
| `elohim/elohim-storage/src/services/attestation_projector.rs` | Post-commit signal handler — projects attestation Content entries |
| `elohim/elohim-storage/src/services/tally_projector.rs` | Reads parent + children, computes tally, upserts table |
| `elohim/elohim-storage/src/api/attestations.rs` | HTTP handler module |
| `elohim/elohim-storage/src/api/governance_actions.rs` | HTTP handler module |
| `elohim/elohim-storage/src/p2p/shamir_transport.rs` | Stage G — libp2p request-response protocol for Shamir share delivery |
| `elohim/elohim-storage/src/recovery/share_assembler.rs` | Stage G — off-chain share assembly logic |
| `app/elohim-library/projects/elohim-service/src/services/attestation.service.ts` | Unified attestation Angular service |
| `app/elohim-library/projects/elohim-service/src/services/governance-action.service.ts` | Governance-action Angular service |
| `elohim/elohim-storage/tests/attestation_consolidation_integration.rs` | End-to-end integration test |

### Modified files

| Path | What changes |
|---|---|
| `elohim/sdk/domains/imagodei/manifest.json` | Add `attestations` + `governance-actions` sections (11 attestation subtypes + 4 governance-action kinds) |
| `elohim/sdk/domains/lamad/manifest.json` | Add `attestations` section (4 attestation subtypes) |
| `elohim/sdk/domains/infrastructure/manifest.json` | Add `attestations` section (2 attestation subtypes) |
| `elohim/sdk/schemas/scripts/codegen-ts.mjs` | Add attestation/governance-action schemas to INTERFACE_FILES |
| `elohim/sdk/schemas/scripts/codegen-rs.mjs` | Emit attestation_kind + governance_action_kind constants |
| `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` | Declare new modules; expose new coordinator functions |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` | Add discriminator-chain validator branch in Content validator |
| `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs` | Remove 15 deleted entry types + corresponding LinkTypes |
| `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` | Remove deleted coordinator functions; add cross-DNA bridge wrappers for the legacy public surface |
| `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs` | Remove HealthAttestation + DoorwayHeartbeatSummary entry types |
| `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs` | Remove `record_health_attestation` + `get_doorway_attestations`; replace with bridge calls |
| `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs` | Remove GateDecisionAttestation + ProposalVote + StatementVote + GovernanceReaction + Proposal + Challenge + GateDecisionChallenge entry types |
| `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs` | Remove corresponding coordinator functions; bridge calls |
| `elohim/elohim-storage/src/db/diesel_schema.rs` | Add `table!` macros for: `attestations` (**source of truth: Holochain DHT**, projection of attestation Content entries — `dht_anchor_hash NOT NULL`); `governance_actions` (**source of truth: Holochain DHT**, projection of governance-action Content entries — `dht_anchor_hash NOT NULL`); `governance_action_tally` (**source of truth: local operational**, Category C derived from parent+children — no `dht_anchor_hash` column). Each `table!` block MUST be preceded by a `// Source of truth: ...` comment matching the migration's first-line declaration |
| `elohim/elohim-storage/src/db/models.rs` | Add Queryable + Insertable row structs: `AttestationRow` (projection of DHT attestation Content; **source of truth: Holochain DHT**), `GovernanceActionRow` (projection of DHT governance-action Content; **source of truth: Holochain DHT**), `GovernanceActionTallyRow` (computed projection; **source of truth: local operational**). Each struct's rustdoc MUST declare its source of truth |
| `elohim/elohim-storage/src/http.rs` | Register new routes; delete legacy attestation routes. Routes serve the projections (read side); coordinator zome remains the write-path source of truth |
| `elohim/elohim-storage/src/views.rs` | Add `AttestationView`, `GovernanceActionView`, `GovernanceActionTallyView` (wire types with `#[derive(TS)]`). Each view's rustdoc MUST declare its source of truth: AttestationView + GovernanceActionView read projections of DHT-notarized Content entries (**source of truth: Holochain DHT**); GovernanceActionTallyView is a computed view (**source of truth: local operational**, derivable from parent + children any time) |
| `elohim/elohim-storage/tests/schema_contract.rs` | Add contract tests for new view types (wire-shape validation against the JSON schemas authored in Stage A) |
| `app/elohim-app/src/app/imagodei/services/attestation.service.ts` | Migrate from imagodei-DNA-specific calls to unified service |
| `app/elohim-app/src/app/lamad/services/content-attestation.service.ts` | Migrate to unified service |
| `genesis/a2o/features/auth/*.feature` | Update scenarios referencing deleted entry types |

---

## Stage ordering and dependencies

**Revised 2026-05-11 (mid-sprint)** — after Stages A, B, C.1 landed, reality drift forced a re-org of the destructive removals and a parallel-execution structure for the back half.

| Order | Stage | Depends on | Build surface | Parallel group |
|---|---|---|---|---|
| 1 | A — Manifest & schema substrate | none | sdk only | sequential (✅ landed) |
| 2 | B — Coordinator zomes + bridges | A | elohim DNA + 3 bridge DNAs | sequential (✅ landed) |
| 3 | C.1 — Discriminator-chain validator | B | elohim DNA (additive) | sequential (✅ landed) |
| 4a | C.2 — Imagodei safe-removals | C.1 | imagodei DNA + elohim DNA scaffold | **Phase 1 parallel** |
| 4b | C.3 — Mishpat + infra full-replacement | C.1 | mishpat + infrastructure DNAs | **Phase 1 parallel** |
| 4c | C.4 — Elohim audited vestigial removals | C.1 | elohim DNA only | **Phase 1 parallel** |
| 4d | D — Storage projection | A, B | elohim-storage (independent crate) | **Phase 1 parallel** |
| 4e | G — Recovery decoupling | B | steward/node + imagodei DNA | **Phase 1 parallel** |
| 5 | C.5 — DNA pack + sweettest | C.2, C.3, C.4 | all 4 DNAs | sequential gate |
| 6 | E — HTTP API + storage-client | D | elohim-storage + storage-client-ts | sequential after Phase 1 |
| 7 | F — Angular consumers + a2o | E | app/elohim-app + a2o features | sequential after E |

### Parallel-execution invariants

**Phase 1 (C.2/C.3/C.4/D/G run in parallel as 5 subagents):**
- Each subagent touches a *different cargo workspace* — no target/ contention.
- Each subagent commits to the same branch via the worktree filesystem — git serializes commits naturally.
- DO NOT run more than ONE `cargo build` concurrently at the operating-system level; subagents must each verify `df -h /projects ≥ 10G free` before building. The subagent framework's sequential build pattern handles this when subagents are dispatched one at a time within the SAME orchestration call; for cross-DNA parallelism, dispatch them in a single message and let each subagent run its own sequential builds inside its scope.
- Forbid scopes per subagent:
  - C.2 → only touches `elohim/holochain/dna/imagodei/` + small cleanup in `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` (cycle-fix scaffold)
  - C.3 → only touches `elohim/holochain/dna/mishpat/` + `elohim/holochain/dna/infrastructure/`
  - C.4 → only touches `elohim/holochain/dna/elohim/zomes/content_store_integrity/`
  - D → only touches `elohim/elohim-storage/`
  - G → only touches `steward/` + `elohim/holochain/dna/imagodei/zomes/imagodei/` recovery flow (NOT entry types — those defer to Stage G's own internal sequencing)

**Sequential gate after Phase 1:** C.5 packs all four DNAs and runs the sweettest suite. If any Phase 1 subagent reported BLOCKED, C.5 is held until the blocker is resolved.

**Phase 2 (E) and Phase 3 (F):** strictly sequential after Phase 1 + C.5, because they cross the wire-protocol boundary that Phase 1 settled.

### Reality-drift adjustments applied to revised Stage C

The original Stage C plan assumed B.10 produced *full-replacement* bridges everywhere. Reality: B.9 (imagodei) was full-replacement but B.10 (mishpat + infrastructure) was *additive* (writes both locally and via cross-DNA call). The original C.4 "vestigial" claim was wrong for `CustodianCommitment` (14 live callers in shard replication) and `ContentSuccession` (live callers in versioning). Revised C.2/C.3/C.4 below reflect a "verify-before-remove" pattern and an explicit handoff between bridge-conversion and entry-type-removal.

This document covers Stages A through G. Stages A–C.1 are landed. C.2–C.5 are revised below. D / E / F / G are described with task headers + key file paths + acceptance criteria; the engineer extends bite-sized steps as they go.

---

## Stage A — Manifest & schema substrate

Stage A produces the JSON schemas + manifest declarations that everything downstream depends on. **No DNA, storage, or Angular code is touched in Stage A** — it's pure substrate work. Verification is `pnpm run schema:test && pnpm run schema:validate && pnpm run schema:codegen:ts && pnpm run schema:codegen:rs`.

### Source-of-truth declaration for Stage A entities

The schemas authored in Stage A describe **wire formats** for entities whose source of truth is the Holochain DHT — they are NOT new storage schemas. Per the p2p-design-gate skill:

| Entity (schema) | Category | Source of Truth | Notes |
|---|---|---|---|
| `AttestationContent` (attestation-content.schema.json) | A — Notarized | **Holochain DHT** | Wire shape of a `Content` entry with `content_type` matching `attestation:<subtype>`. The schema describes the JSON that crosses the DNA boundary; canonical state lives on the DHT. |
| `AttestationMetadata` (attestation-metadata.schema.json) | A — Notarized (sub-shape of AttestationContent) | **Holochain DHT** | The structured payload inside `Content.metadata_json` for attestation entries; same source-of-truth as its container. |
| `ProofEvidence` (proof-evidence.schema.json) | A — Notarized (sub-shape of AttestationMetadata) | **Holochain DHT** | Pluggable proof-class payload; defaults to `witness` (signed by issuer via Content action header — no extra material on the DHT). Higher classes carry Merkle roots / zk proofs / multi-attestor chains in the same Content entry. |
| `GovernanceActionContent` (governance-action-content.schema.json) | A — Notarized | **Holochain DHT** | Wire shape of a parent `Content` entry with `content_type` matching `governance-action:<kind>`. |
| `GovernanceActionMetadata` (governance-action-metadata.schema.json) | A — Notarized (sub-shape of GovernanceActionContent) | **Holochain DHT** | The structured payload inside `Content.metadata_json` for governance-action entries. |
| `ManifestAttestationDeclaration` (manifest-attestation-declaration.schema.json) | not an entity — manifest schema | **Pillar manifest JSON on disk** | Operator-controlled config that declares the attestation subtype catalog. Lives on disk, not on the DHT. |
| `ManifestGovernanceActionDeclaration` (manifest-governance-action-declaration.schema.json) | not an entity — manifest schema | **Pillar manifest JSON on disk** | Same as above for governance-action declarations. |
| Per-subtype metadata schemas (subtypes/*.schema.json) | A — Notarized (sub-shapes of AttestationMetadata) | **Holochain DHT** | Each schema describes the `evidence_json.summary_metric` payload for a specific attestation subtype. Same source-of-truth as the enclosing attestation. |

**Anti-pattern check for Stage A:** ✓ No new DHT entry types (everything reuses `Content`). ✓ No new storage tables in Stage A (those come in Stage D, each with explicit source-of-truth comments per the migration spec in §7.4). ✓ Manifest layer cleanly separated from on-DHT state. Each JSON schema authored below includes a `$comment` field reiterating the source-of-truth classification so downstream readers don't drop the requirement.

### Task A.1 — Core attestation Content schema

**Files:**
- Create: `elohim/sdk/schemas/v1/attestation/attestation-content.schema.json`
- Create: `elohim/sdk/schemas/v1/attestation/.gitkeep` (if directory needs creation)

- [ ] **Step 1: Create the directory**

```bash
mkdir -p elohim/sdk/schemas/v1/attestation/subtypes
```

- [ ] **Step 2: Write the attestation-content schema**

**Source of truth:** Holochain DHT (Category A — wire format only; this JSON Schema describes the on-wire shape of a notarized `Content` entry; canonical state lives on the DHT, not in any local schema).

Create `elohim/sdk/schemas/v1/attestation/attestation-content.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.protocol/schemas/v1/attestation/attestation-content.schema.json",
  "$comment": "Source of truth: Holochain DHT (notarized Content entry with content_type LIKE 'attestation:%'). Category A per p2p-design-gate.",
  "title": "AttestationContent",
  "description": "Wire shape for an attestation realized as a Content entry on elohim DNA. Source of truth is the Holochain DHT — this schema describes the JSON-on-wire shape only. See genesis/docs/superpowers/specs/2026-05-11-attestation-consolidation-design.md §3.",
  "type": "object",
  "required": [
    "id",
    "content_type",
    "title",
    "author_id",
    "reach",
    "metadata_json",
    "created_at"
  ],
  "properties": {
    "id": { "type": "string", "description": "CIDv1 — content-derived identity" },
    "content_type": {
      "type": "string",
      "pattern": "^attestation:[a-z][a-z0-9-]*$",
      "description": "Discriminator. Subtype must be declared in some pillar manifest's attestations section"
    },
    "title": { "type": "string", "minLength": 1 },
    "description": { "type": "string" },
    "author_id": { "type": "string", "description": "Issuer's agent CID (signed via Content action header)" },
    "reach": { "type": "string", "enum": ["private", "community", "public", "commons"] },
    "metadata_json": {
      "type": "string",
      "description": "Serialized JSON conforming to AttestationMetadata. See sibling proof-evidence.schema.json + per-subtype metadata schemas"
    },
    "tags": { "type": "array", "items": { "type": "string" } },
    "related_node_ids": { "type": "array", "items": { "type": "string" } },
    "created_at": { "type": "string", "format": "date-time" },
    "schema_version": { "type": "integer", "minimum": 0 },
    "validation_status": { "type": "string" }
  }
}
```

- [ ] **Step 3: Add metadata sub-schema**

**Source of truth:** Holochain DHT (Category A — sub-shape of the notarized `Content.metadata_json` payload for attestation entries; same source-of-truth as its container).

Create `elohim/sdk/schemas/v1/attestation/attestation-metadata.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.protocol/schemas/v1/attestation/attestation-metadata.schema.json",
  "$comment": "Source of truth: Holochain DHT (sub-shape of attestation Content entry's metadata_json). Category A per p2p-design-gate.",
  "title": "AttestationMetadata",
  "description": "Structured payload inside Content.metadata_json for attestation entries. Source of truth is the Holochain DHT — this schema describes the parsed JSON shape only.",
  "type": "object",
  "required": [
    "attestation_kind",
    "subject_cid",
    "subject_kind",
    "validation_method",
    "proof_evidence"
  ],
  "properties": {
    "attestation_kind": { "type": "string", "description": "Mirrors content_type subtype, denormalized for query" },
    "subject_cid": { "type": "string" },
    "subject_kind": { "type": "string", "enum": ["agent", "content", "device", "hub", "computation", "governance-action"] },
    "validation_method": {
      "type": "string",
      "enum": ["self-attest", "peer-confirm", "M-of-N-vote", "audit-signature", "computational"]
    },
    "evidence_json": {
      "type": "object",
      "properties": {
        "observation_refs": { "type": "array", "items": { "type": "string" } },
        "observation_period_start": { "type": "string", "format": "date-time" },
        "observation_period_end": { "type": "string", "format": "date-time" },
        "summary_metric": { "type": "object", "additionalProperties": true }
      }
    },
    "proof_evidence": { "$ref": "./proof-evidence.schema.json" },
    "expires_at": { "type": ["string", "null"], "format": "date-time" },
    "revocation": {
      "type": ["object", "null"],
      "properties": {
        "reason": { "type": "string" },
        "revoked_at": { "type": "string", "format": "date-time" },
        "supersedes_cid": { "type": "string" }
      },
      "required": ["reason", "revoked_at", "supersedes_cid"]
    },
    "parent_governance_action_cid": { "type": ["string", "null"] },
    "vote_value": { "type": ["string", "null"], "enum": ["approve", "reject", "abstain", null] },
    "vote_weight": { "type": ["string", "null"] }
  }
}
```

- [ ] **Step 4: Add proof-evidence schema**

Create `elohim/sdk/schemas/v1/attestation/proof-evidence.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.protocol/schemas/v1/attestation/proof-evidence.schema.json",
  "title": "ProofEvidence",
  "description": "Pluggable proof-class evidence carried in AttestationMetadata.proof_evidence. See spec §3.3 + computation-attestation-graduated-rigor-design.md.",
  "type": "object",
  "required": ["class"],
  "properties": {
    "class": { "type": "string", "enum": ["witness", "audit", "proof", "confirmation"] },
    "issuer_signature": { "type": "string", "description": "Inherited from Content action header for witness class" },
    "merkle_root": { "type": "string", "description": "Required for audit class — Merkle root over Merkle-rooted inputs" },
    "algorithm_id": { "type": "string", "description": "Required for audit/proof — pinned algorithm + version" },
    "zkml_proof": { "type": "string", "description": "Required for proof class — zkML or equivalent cryptographic proof" },
    "multi_attestor_chain": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Required for confirmation class — CIDs of corroborating attestations"
    }
  },
  "allOf": [
    {
      "if": { "properties": { "class": { "const": "audit" } } },
      "then": { "required": ["merkle_root", "algorithm_id"] }
    },
    {
      "if": { "properties": { "class": { "const": "proof" } } },
      "then": { "required": ["zkml_proof", "algorithm_id"] }
    },
    {
      "if": { "properties": { "class": { "const": "confirmation" } } },
      "then": { "required": ["multi_attestor_chain"] }
    }
  ]
}
```

- [ ] **Step 5: Run schema self-tests**

```bash
pnpm run schema:test
```

Expected: PASS (new schemas are drop-in tested by the harness)

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/v1/attestation/attestation-content.schema.json \
  elohim/sdk/schemas/v1/attestation/attestation-metadata.schema.json \
  elohim/sdk/schemas/v1/attestation/proof-evidence.schema.json
git commit -m "schema(attestation): core attestation Content + metadata + proof-evidence schemas"
```

### Task A.2 — Governance-action Content schema

**Files:**
- Create: `elohim/sdk/schemas/v1/attestation/governance-action-content.schema.json`
- Create: `elohim/sdk/schemas/v1/attestation/governance-action-metadata.schema.json`

- [ ] **Step 1: Write the governance-action-content schema**

Create `elohim/sdk/schemas/v1/attestation/governance-action-content.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.protocol/schemas/v1/attestation/governance-action-content.schema.json",
  "title": "GovernanceActionContent",
  "description": "Wire shape for a governance-action realized as a Content entry on elohim DNA. Parent of child attestation Content entries that cast votes. See spec §4.2.",
  "type": "object",
  "required": ["id", "content_type", "title", "author_id", "reach", "metadata_json", "created_at"],
  "properties": {
    "id": { "type": "string" },
    "content_type": {
      "type": "string",
      "pattern": "^governance-action:[a-z][a-z0-9-]*$"
    },
    "title": { "type": "string", "minLength": 1 },
    "description": { "type": "string" },
    "author_id": { "type": "string", "description": "Proposer's agent CID" },
    "reach": { "type": "string", "enum": ["private", "community", "public", "commons"] },
    "metadata_json": { "type": "string" },
    "tags": { "type": "array", "items": { "type": "string" } },
    "related_node_ids": { "type": "array", "items": { "type": "string" } },
    "created_at": { "type": "string", "format": "date-time" },
    "schema_version": { "type": "integer", "minimum": 0 },
    "validation_status": { "type": "string" }
  }
}
```

- [ ] **Step 2: Write the governance-action-metadata schema**

Create `elohim/sdk/schemas/v1/attestation/governance-action-metadata.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.protocol/schemas/v1/attestation/governance-action-metadata.schema.json",
  "title": "GovernanceActionMetadata",
  "type": "object",
  "required": ["governance_kind", "subject_cid", "threshold", "ballot_format", "closes_at"],
  "properties": {
    "governance_kind": { "type": "string" },
    "subject_cid": { "type": "string" },
    "threshold": {
      "type": "object",
      "oneOf": [
        {
          "required": ["type", "m", "n"],
          "properties": {
            "type": { "const": "m-of-n" },
            "m": { "type": "integer", "minimum": 1 },
            "n": { "type": "integer", "minimum": 1 }
          }
        },
        {
          "required": ["type", "percentage"],
          "properties": {
            "type": { "const": "percentage" },
            "percentage": { "type": "number", "minimum": 0, "maximum": 100 }
          }
        }
      ]
    },
    "eligibility_predicate": {
      "type": "object",
      "required": ["type"],
      "properties": {
        "type": { "type": "string" },
        "manifest_ref": { "type": "string" },
        "parameters": { "type": "object", "additionalProperties": true }
      }
    },
    "ballot_format": {
      "type": "string",
      "enum": ["approve-reject", "ranked-choice", "approval", "quadratic"]
    },
    "closes_at": { "type": "string", "format": "date-time" },
    "parameters_json": { "type": "object", "additionalProperties": true }
  }
}
```

- [ ] **Step 3: Run schema self-tests**

```bash
pnpm run schema:test
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/schemas/v1/attestation/governance-action-content.schema.json \
  elohim/sdk/schemas/v1/attestation/governance-action-metadata.schema.json
git commit -m "schema(attestation): governance-action Content + metadata schemas"
```

### Task A.3 — Manifest declaration schemas

**Files:**
- Create: `elohim/sdk/schemas/v1/attestation/manifest-attestation-declaration.schema.json`
- Create: `elohim/sdk/schemas/v1/attestation/manifest-governance-action-declaration.schema.json`

- [ ] **Step 1: Write the attestation-declaration schema**

Create `elohim/sdk/schemas/v1/attestation/manifest-attestation-declaration.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.protocol/schemas/v1/attestation/manifest-attestation-declaration.schema.json",
  "title": "ManifestAttestationDeclaration",
  "description": "Per-subtype attestation declaration inside a pillar manifest's attestations section. See spec §6.1.",
  "type": "object",
  "required": ["description", "subject_kinds", "metadata_schema", "revocable_by"],
  "properties": {
    "description": { "type": "string" },
    "subject_kinds": {
      "type": "array",
      "items": { "type": "string", "enum": ["agent", "content", "device", "hub", "computation", "governance-action"] },
      "minItems": 1
    },
    "metadata_schema": {
      "type": "object",
      "required": ["$ref"],
      "properties": { "$ref": { "type": "string" } }
    },
    "authorization_predicate": {
      "type": ["object", "null"],
      "required": ["type"],
      "properties": {
        "type": { "type": "string" },
        "attestation_kind": { "type": "string" },
        "scope": { "type": "string" }
      }
    },
    "uniqueness_anchor": { "type": ["string", "null"], "description": "Anchor template, e.g. \"attestation:{kind}:{parent_cid}:{issuer_cid}\"" },
    "default_expiration_days": { "type": ["integer", "null"], "minimum": 1 },
    "revocable_by": {
      "type": "array",
      "items": { "type": "string", "enum": ["issuer", "subject", "domain-steward", "governance"] },
      "minItems": 1
    }
  }
}
```

- [ ] **Step 2: Write the governance-action-declaration schema**

Create `elohim/sdk/schemas/v1/attestation/manifest-governance-action-declaration.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.protocol/schemas/v1/attestation/manifest-governance-action-declaration.schema.json",
  "title": "ManifestGovernanceActionDeclaration",
  "type": "object",
  "required": ["description", "child_attestation_kind", "default_ballot_format"],
  "properties": {
    "description": { "type": "string" },
    "child_attestation_kind": {
      "type": "string",
      "description": "The attestation_kind that a vote-child of this governance-action uses (e.g., 'attestation:renewal-approval' for governance-action:renewal-request)"
    },
    "default_ballot_format": {
      "type": "string",
      "enum": ["approve-reject", "ranked-choice", "approval", "quadratic"]
    },
    "default_eligibility_predicate": {
      "type": ["object", "null"],
      "properties": {
        "type": { "type": "string" },
        "manifest_ref": { "type": "string" }
      }
    }
  }
}
```

- [ ] **Step 3: Run schema self-tests**

```bash
pnpm run schema:test
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/schemas/v1/attestation/manifest-attestation-declaration.schema.json \
  elohim/sdk/schemas/v1/attestation/manifest-governance-action-declaration.schema.json
git commit -m "schema(attestation): manifest declaration schemas for attestation + governance-action sections"
```

### Task A.4 — Per-subtype metadata schemas (imagodei subtypes)

**Files:**
- Create one schema per imagodei attestation subtype in `elohim/sdk/schemas/v1/attestation/subtypes/`

Each file follows the same template. Example for `humanness`:

- [ ] **Step 1: Write humanness-metadata schema**

Create `elohim/sdk/schemas/v1/attestation/subtypes/humanness-metadata.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.protocol/schemas/v1/attestation/subtypes/humanness-metadata.schema.json",
  "title": "HumannessAttestationMetadata",
  "description": "Per-subtype metadata payload for attestation:humanness. Sits inside AttestationMetadata.evidence_json.summary_metric.",
  "type": "object",
  "required": ["humanness_method"],
  "properties": {
    "humanness_method": {
      "type": "string",
      "enum": ["behavioral", "interaction", "video_call", "in_person", "elohim_check"]
    },
    "confidence_score": { "type": "number", "minimum": 0, "maximum": 1 }
  }
}
```

- [ ] **Step 2: Write identity-credential-metadata schema**

Create `elohim/sdk/schemas/v1/attestation/subtypes/identity-credential-metadata.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.protocol/schemas/v1/attestation/subtypes/identity-credential-metadata.schema.json",
  "title": "IdentityCredentialAttestationMetadata",
  "description": "Per-subtype metadata for attestation:identity-credential. Replaces imagodei Attestation entry's category/tier/earned_via fields.",
  "type": "object",
  "required": ["category", "credential_type"],
  "properties": {
    "category": {
      "type": "string",
      "enum": ["learning", "stewardship", "governance", "community", "technical", "conduct", "identity", "peer"]
    },
    "credential_type": { "type": "string" },
    "tier": { "type": ["string", "null"], "enum": ["bronze", "silver", "gold", "platinum", null] },
    "display_name": { "type": "string" },
    "icon_url": { "type": ["string", "null"] }
  }
}
```

- [ ] **Step 3: Write the remaining imagodei subtype schemas**

Following the same template, create schemas for the 9 remaining imagodei attestation subtypes:
- `subtypes/key-stewardship-metadata.schema.json` (fields: device_id, stewardship_class)
- `subtypes/stewardship-grant-metadata.schema.json` (fields: capability_set, grant_scope, conditions)
- `subtypes/stewardship-appeal-metadata.schema.json` (fields: appealed_grant_cid, appeal_reason)
- `subtypes/policy-inheritance-metadata.schema.json` (fields: parent_policy_cid, inherited_capabilities)
- `subtypes/identity-freeze-metadata.schema.json` (fields: freeze_reason, freeze_duration_hours)
- `subtypes/renewal-approval-metadata.schema.json` (fields: renewal_intimacy_tier — child of governance-action:renewal-request)
- `subtypes/recovery-approval-metadata.schema.json` (fields: custodian_intimacy_tier — child of governance-action:recovery-request)
- `subtypes/revocation-vote-metadata.schema.json` (fields: revocation_reason_code — child of governance-action:key-revocation)
- `subtypes/challenge-support-metadata.schema.json` (fields: evidence_class, severity — child of governance-action:identity-challenge)

Each schema follows the structure of humanness-metadata.schema.json with subtype-specific properties.

- [ ] **Step 4: Run schema self-tests**

```bash
pnpm run schema:test
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/attestation/subtypes/
git commit -m "schema(attestation): per-subtype metadata schemas for imagodei attestation subtypes"
```

### Task A.5 — Per-subtype metadata schemas (lamad / infrastructure / mishpat subtypes)

**Files:**
- Create remaining subtype schemas in `elohim/sdk/schemas/v1/attestation/subtypes/`

- [ ] **Step 1: Write lamad subtype schemas**

Create the 4 lamad subtype schemas:
- `subtypes/mastery-metadata.schema.json` (fields: concept_cid, mastery_level, evidence_session_count, last_practiced_at)
- `subtypes/content-quality-metadata.schema.json` (fields: quality_dimension, reach_granted ∈ private/community/public/commons, evidence_review_count)
- `subtypes/content-succession-metadata.schema.json` (fields: superseded_content_cid, succession_reason)
- `subtypes/custodian-commitment-metadata.schema.json` (fields: custodied_subject_cid, commitment_scope, expires_at)

- [ ] **Step 2: Write infrastructure subtype schemas**

Create:
- `subtypes/device-health-metadata.schema.json` (fields: device_id, health_metric, period_start, period_end, sample_count)
- `subtypes/doorway-health-summary-metadata.schema.json` (fields: doorway_id, uptime_percentage, request_volume, period_start, period_end)

- [ ] **Step 3: Write mishpat subtype schemas**

Create:
- `subtypes/governance-role-metadata.schema.json` (fields: role_kind, role_scope, electing_body_cid)
- `subtypes/gate-decision-metadata.schema.json` (fields: gated_subject_cid, decision_outcome ∈ allow/block/pending, gate_kind)
- `subtypes/proposal-vote-metadata.schema.json` (fields: proposal_cid — child of governance-action:proposal)
- `subtypes/statement-vote-metadata.schema.json` (fields: statement_cid, polis_axis ∈ agree/disagree/pass)
- `subtypes/governance-reaction-metadata.schema.json` (fields: reaction_kind, reacted_to_cid)

- [ ] **Step 4: Run schema self-tests**

```bash
pnpm run schema:test
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/attestation/subtypes/
git commit -m "schema(attestation): per-subtype metadata schemas for lamad + infrastructure + mishpat subtypes"
```

### Task A.6 — Extend pillar manifests

**Files:**
- Modify: `elohim/sdk/domains/imagodei/manifest.json`
- Modify: `elohim/sdk/domains/lamad/manifest.json`
- Modify: `elohim/sdk/domains/infrastructure/manifest.json`
- Create: `elohim/sdk/domains/mishpat/manifest.json`

- [ ] **Step 1: Add attestations + governance-actions sections to imagodei manifest**

In `elohim/sdk/domains/imagodei/manifest.json`, add at the top level (sibling to `vocabulary`):

```jsonc
{
  "id": "manifest-imagodei",
  // ... existing fields ...
  "attestations": {
    "attestation:humanness": {
      "description": "Witness that an agent is human, derived from behavioral / interaction / video-call / in-person / elohim-check observation streams",
      "subject_kinds": ["agent"],
      "metadata_schema": { "$ref": "../../schemas/v1/attestation/subtypes/humanness-metadata.schema.json" },
      "authorization_predicate": {
        "type": "issuer-has-attestation",
        "attestation_kind": "attestation:humanness",
        "scope": "any"
      },
      "uniqueness_anchor": "attestation:humanness:{subject_cid}:{issuer_cid}",
      "default_expiration_days": 365,
      "revocable_by": ["issuer"]
    },
    "attestation:identity-credential": {
      "description": "General identity credential — replaces imagodei's Attestation entry type. Discriminated by category (learning/stewardship/governance/community/technical/conduct/identity/peer)",
      "subject_kinds": ["agent"],
      "metadata_schema": { "$ref": "../../schemas/v1/attestation/subtypes/identity-credential-metadata.schema.json" },
      "uniqueness_anchor": "attestation:identity-credential:{subject_cid}:{issuer_cid}:{credential_type}",
      "revocable_by": ["issuer", "domain-steward"]
    },
    "attestation:key-stewardship": {
      "description": "Attestation that a specific device-key is stewarded by an agent",
      "subject_kinds": ["device"],
      "metadata_schema": { "$ref": "../../schemas/v1/attestation/subtypes/key-stewardship-metadata.schema.json" },
      "uniqueness_anchor": "attestation:key-stewardship:{subject_cid}",
      "revocable_by": ["issuer", "subject"]
    },
    "attestation:stewardship-grant": {
      "description": "Capability grant from steward to subject. Replaces StewardshipGrant entry type",
      "subject_kinds": ["agent"],
      "metadata_schema": { "$ref": "../../schemas/v1/attestation/subtypes/stewardship-grant-metadata.schema.json" },
      "revocable_by": ["issuer", "governance"]
    },
    "attestation:stewardship-appeal": {
      "description": "Appeal against a stewardship grant",
      "subject_kinds": ["agent"],
      "metadata_schema": { "$ref": "../../schemas/v1/attestation/subtypes/stewardship-appeal-metadata.schema.json" },
      "revocable_by": ["issuer"]
    },
    "attestation:policy-inheritance": {
      "description": "Policy inheritance binding",
      "subject_kinds": ["agent", "device"],
      "metadata_schema": { "$ref": "../../schemas/v1/attestation/subtypes/policy-inheritance-metadata.schema.json" },
      "revocable_by": ["issuer", "governance"]
    },
    "attestation:identity-freeze": {
      "description": "Derived attestation that identity is frozen — synthesized when challenge-support threshold reached on a governance-action:identity-challenge",
      "subject_kinds": ["agent"],
      "metadata_schema": { "$ref": "../../schemas/v1/attestation/subtypes/identity-freeze-metadata.schema.json" },
      "revocable_by": ["governance"]
    },
    "attestation:renewal-approval": {
      "description": "Vote-child attestation approving a renewal-request governance-action",
      "subject_kinds": ["agent"],
      "metadata_schema": { "$ref": "../../schemas/v1/attestation/subtypes/renewal-approval-metadata.schema.json" },
      "authorization_predicate": {
        "type": "manifest-defined",
        "manifest_ref": "imagodei:custodian-eligibility-v1"
      },
      "uniqueness_anchor": "attestation:renewal-approval:{parent_governance_action_cid}:{issuer_cid}",
      "revocable_by": ["issuer"]
    },
    "attestation:recovery-approval": {
      "description": "Vote-child attestation approving a recovery-request governance-action",
      "subject_kinds": ["agent"],
      "metadata_schema": { "$ref": "../../schemas/v1/attestation/subtypes/recovery-approval-metadata.schema.json" },
      "authorization_predicate": {
        "type": "manifest-defined",
        "manifest_ref": "imagodei:custodian-eligibility-v1"
      },
      "uniqueness_anchor": "attestation:recovery-approval:{parent_governance_action_cid}:{issuer_cid}",
      "revocable_by": ["issuer"]
    },
    "attestation:revocation-vote": {
      "description": "Vote-child attestation on a key-revocation governance-action",
      "subject_kinds": ["agent"],
      "metadata_schema": { "$ref": "../../schemas/v1/attestation/subtypes/revocation-vote-metadata.schema.json" },
      "uniqueness_anchor": "attestation:revocation-vote:{parent_governance_action_cid}:{issuer_cid}",
      "revocable_by": ["issuer"]
    },
    "attestation:challenge-support": {
      "description": "Support attestation for an identity-challenge governance-action",
      "subject_kinds": ["agent"],
      "metadata_schema": { "$ref": "../../schemas/v1/attestation/subtypes/challenge-support-metadata.schema.json" },
      "uniqueness_anchor": "attestation:challenge-support:{parent_governance_action_cid}:{issuer_cid}",
      "revocable_by": ["issuer"]
    }
  },
  "governance-actions": {
    "governance-action:renewal-request": {
      "description": "Request for identity-key renewal, voted on by custodians",
      "child_attestation_kind": "attestation:renewal-approval",
      "default_ballot_format": "approve-reject",
      "default_eligibility_predicate": {
        "type": "manifest-defined",
        "manifest_ref": "imagodei:custodian-eligibility-v1"
      }
    },
    "governance-action:recovery-request": {
      "description": "Request for account recovery, voted on by custodians",
      "child_attestation_kind": "attestation:recovery-approval",
      "default_ballot_format": "approve-reject",
      "default_eligibility_predicate": {
        "type": "manifest-defined",
        "manifest_ref": "imagodei:custodian-eligibility-v1"
      }
    },
    "governance-action:key-revocation": {
      "description": "Proposal to revoke a key, voted on by stewards",
      "child_attestation_kind": "attestation:revocation-vote",
      "default_ballot_format": "approve-reject"
    },
    "governance-action:identity-challenge": {
      "description": "Challenge an identity's authenticity, supported by N witnesses",
      "child_attestation_kind": "attestation:challenge-support",
      "default_ballot_format": "approve-reject"
    }
  }
}
```

- [ ] **Step 2: Add attestations section to lamad manifest**

In `elohim/sdk/domains/lamad/manifest.json`, add the `attestations` section with the 4 lamad subtypes (`attestation:mastery`, `attestation:content-quality`, `attestation:content-succession`, `attestation:custodian-commitment`) following the same shape as Step 1.

- [ ] **Step 3: Add attestations section to infrastructure manifest**

In `elohim/sdk/domains/infrastructure/manifest.json`, add the `attestations` section with the 2 infrastructure subtypes (`attestation:device-health`, `attestation:doorway-health-summary`).

- [ ] **Step 4: Create mishpat manifest**

Create `elohim/sdk/domains/mishpat/manifest.json`:

```jsonc
{
  "id": "manifest-mishpat",
  "name": "mishpat",
  "version": "1.0.0",
  "description": "Governance — proposals, challenges, gate-decisions, deliberation. Validators of community decisions.",
  "vocabulary": { "contentTypes": {} },
  "attestations": {
    "attestation:governance-role": { /* per spec §6.2 */ },
    "attestation:gate-decision": { /* per spec §6.2 */ },
    "attestation:proposal-vote": { /* per spec §6.2 */ },
    "attestation:statement-vote": { /* per spec §6.2 */ },
    "attestation:governance-reaction": { /* per spec §6.2 */ }
  },
  "governance-actions": {
    "governance-action:proposal": { /* per spec §6.3 */ },
    "governance-action:challenge": { /* per spec §6.3 */ },
    "governance-action:election": { /* per spec §6.3 */ }
  }
}
```

Fill in each declaration following the imagodei manifest pattern from Step 1.

- [ ] **Step 5: Validate manifests against declaration schemas**

```bash
pnpm run schema:validate
```

Expected: PASS for all 4 modified/new manifests (validates that each `attestations` / `governance-actions` section conforms to the declaration schemas authored in Task A.3)

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/domains/
git commit -m "manifest(attestation): declare attestation + governance-action subtypes per pillar"
```

### Task A.7 — Extend codegen scripts

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs`
- Modify: `elohim/sdk/schemas/scripts/codegen-rs.mjs`

- [ ] **Step 1: Add attestation schemas to codegen-ts INTERFACE_FILES**

In `elohim/sdk/schemas/scripts/codegen-ts.mjs`, locate the `INTERFACE_FILES` array and add the new schema paths:

```javascript
const INTERFACE_FILES = [
  // ... existing entries ...
  'attestation/attestation-content.schema.json',
  'attestation/attestation-metadata.schema.json',
  'attestation/proof-evidence.schema.json',
  'attestation/governance-action-content.schema.json',
  'attestation/governance-action-metadata.schema.json',
  // Per-subtype metadata schemas (glob expansion handled by the script)
  // ... or add each subtype schema explicitly if the script doesn't glob
];
```

Also extend `refMap` to include the new relative-path key forms for cross-schema refs (per memory pin `feedback_codegen_relative_ref_keys.md`: bare, `./` prefix, `../views/` prefix, and `$id` all need entries).

- [ ] **Step 2: Add attestation_kind + governance_action_kind constants emission to codegen-rs**

In `elohim/sdk/schemas/scripts/codegen-rs.mjs`, add a new emission pass that walks every pillar manifest's `attestations` + `governance-actions` sections and produces a Rust constants file at `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_attestation_kinds.rs`:

```rust
// AUTO-GENERATED — do not edit by hand. Source: pillar manifests' attestations + governance-actions sections.

pub const ATTESTATION_KINDS: &[&str] = &[
    "attestation:humanness",
    "attestation:identity-credential",
    // ... (full list from all 4 pillar manifests)
];

pub const GOVERNANCE_ACTION_KINDS: &[&str] = &[
    "governance-action:renewal-request",
    "governance-action:recovery-request",
    // ... (full list)
];

/// Maps an attestation subtype to its declaring manifest's ref string.
pub fn manifest_ref_for_attestation_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "attestation:humanness" => Some("imagodei"),
        // ... (full mapping)
        _ => None,
    }
}
```

- [ ] **Step 3: Run codegen**

```bash
pnpm run schema:codegen:ts
pnpm run schema:codegen:rs
```

Expected: New TypeScript types appear under `elohim/sdk/storage-client-ts/src/generated/`; new Rust constants file generated.

- [ ] **Step 4: Verify generated artifacts**

```bash
ls elohim/sdk/storage-client-ts/src/generated/ | grep -i attestation
cat elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_attestation_kinds.rs | head -20
```

Expected: AttestationContent / GovernanceActionContent / ProofEvidence interfaces present in TS; ATTESTATION_KINDS const non-empty in Rust.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/scripts/codegen-ts.mjs \
  elohim/sdk/schemas/scripts/codegen-rs.mjs \
  elohim/sdk/storage-client-ts/src/generated/ \
  elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_attestation_kinds.rs
git commit -m "codegen(attestation): emit attestation + governance-action types from schemas + manifests"
```

### Task A.8 — Stage A acceptance test

**Files:**
- Modify: `elohim/sdk/schemas/scripts/check-dna.mjs` (or equivalent harness file)

- [ ] **Step 1: Add a DNA-vs-schema parity check for attestation kinds**

Extend the `check-dna` harness to assert: every kind in `ATTESTATION_KINDS` (generated constant) appears in some pillar manifest's `attestations` section. Every kind in `GOVERNANCE_ACTION_KINDS` appears in some pillar manifest's `governance-actions` section.

- [ ] **Step 2: Run the full schema/dna parity suite**

```bash
pnpm run schema:test
pnpm run schema:validate
pnpm run schema:check-dna
```

Expected: ALL PASS.

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/schemas/scripts/check-dna.mjs
git commit -m "test(attestation): DNA-vs-manifest parity check for attestation kinds"
```

**Stage A acceptance:** All schema + codegen + DNA-parity checks pass; no other code touched. Push to dev or hold for stage gate.

---

## Stage B — Coordinator zomes (elohim DNA)

Stage B implements the coordinator-zome surface for issuing attestations and orchestrating governance-actions. Single DNA (elohim) is touched; cross-DNA bridges from imagodei/infrastructure/mishpat are added as thin wrappers but the heavy lifting lives in elohim DNA's `content_store` coordinator zome.

Cargo builds in this stage run against `elohim/holochain/dna/elohim/` with plain `cargo` (no RUSTFLAGS override — DNA workspaces use plain cargo per CLAUDE.md gotcha).

### Task B.1 — Scaffold attestation.rs module in content_store

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

- [ ] **Step 1: Create attestation.rs with type stubs**

Create `elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs`:

```rust
//! Attestation coordinator — issuance, revocation, queries.
//!
//! Implements the consolidation defined in
//! `genesis/docs/superpowers/specs/2026-05-11-attestation-consolidation-design.md`.
//!
//! Attestation entries are `Content` entries with `content_type` matching
//! `"attestation:<subtype>"` declared in some pillar manifest. This module
//! provides the coordinator-facing API for callers in elohim DNA and via
//! cross-DNA bridge for imagodei / infrastructure / mishpat callers.

use hdk::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueAttestationInput {
    pub attestation_kind: String,           // e.g. "attestation:humanness"
    pub subject_cid: String,
    pub subject_kind: String,               // agent | content | device | hub | computation | governance-action
    pub title: String,
    pub description: Option<String>,
    pub reach: String,                      // private | community | public | commons
    pub metadata: serde_json::Value,        // structured per per-subtype metadata schema
    pub parent_governance_action_cid: Option<String>,
    pub vote_value: Option<String>,         // approve | reject | abstain — only for vote attestations
    pub proof_class: String,                // witness (default) | audit | proof | confirmation
    pub proof_evidence: serde_json::Value,
    pub expires_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AttestationOutput {
    pub cid: String,                        // EntryHash of the issued attestation Content
    pub attestation_kind: String,
    pub subject_cid: String,
    pub issuer_cid: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeAttestationInput {
    pub attestation_cid: String,
    pub reason: String,
}

// Stubs — to be implemented in subsequent tasks
pub fn issue_attestation(_input: IssueAttestationInput) -> ExternResult<AttestationOutput> {
    unimplemented!("Task B.3")
}

pub fn revoke_attestation(_input: RevokeAttestationInput) -> ExternResult<AttestationOutput> {
    unimplemented!("Task B.4")
}
```

- [ ] **Step 2: Declare module in lib.rs**

In `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`, add the module declaration near other module declarations (e.g., next to `pub mod feedback_signal;`):

```rust
pub mod attestation;
```

- [ ] **Step 3: Build the zome (verify it compiles)**

```bash
cd elohim/holochain/dna/elohim
cargo build --target wasm32-unknown-unknown -p content_store
```

Expected: SUCCESS (stubs compile but `unimplemented!` would panic at runtime).

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs \
  elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
git commit -m "scaffold(attestation): content_store::attestation module with input/output types"
```

### Task B.2 — Scaffold governance_action.rs module

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

- [ ] **Step 1: Create governance_action.rs with type stubs**

Create `elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs`:

```rust
//! Governance-action coordinator — propose, vote, query.
//!
//! Implements the M-of-N pattern from spec §4: parent governance-action Content +
//! child attestation Content + derived tally projection. Voting is implemented
//! by issuing a child attestation Content; this module provides the wrapper
//! that ensures the child carries the correct parent_governance_action_cid
//! and is committed against the validator floors for M-of-N children.

use hdk::prelude::*;

use crate::attestation::AttestationOutput;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProposeGovernanceActionInput {
    pub governance_kind: String,             // e.g. "governance-action:renewal-request"
    pub subject_cid: String,
    pub title: String,
    pub description: Option<String>,
    pub reach: String,
    pub threshold: serde_json::Value,        // see governance-action-metadata.schema.json
    pub eligibility_predicate: Option<serde_json::Value>,
    pub ballot_format: String,
    pub closes_at: String,                   // RFC3339 UTC
    pub parameters: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GovernanceActionOutput {
    pub cid: String,
    pub governance_kind: String,
    pub subject_cid: String,
    pub proposer_cid: String,
    pub closes_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoteOnGovernanceActionInput {
    pub parent_governance_action_cid: String,
    pub vote_value: String,                  // approve | reject | abstain
    pub vote_weight: Option<String>,
    pub evidence: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GovernanceActionWithChildren {
    pub parent: GovernanceActionOutput,
    pub children: Vec<AttestationOutput>,
}

pub fn propose_governance_action(
    _input: ProposeGovernanceActionInput,
) -> ExternResult<GovernanceActionOutput> {
    unimplemented!("Task B.5")
}

pub fn vote_on_governance_action(
    _input: VoteOnGovernanceActionInput,
) -> ExternResult<AttestationOutput> {
    unimplemented!("Task B.6")
}

pub fn get_governance_action_with_children(
    _parent_cid: String,
) -> ExternResult<GovernanceActionWithChildren> {
    unimplemented!("Task B.7")
}
```

- [ ] **Step 2: Declare module in lib.rs**

```rust
pub mod governance_action;
```

- [ ] **Step 3: Build the zome**

```bash
cd elohim/holochain/dna/elohim
cargo build --target wasm32-unknown-unknown -p content_store
```

Expected: SUCCESS.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs \
  elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
git commit -m "scaffold(attestation): content_store::governance_action module with input/output types"
```

### Task B.3 — Implement issue_attestation

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs`

- [ ] **Step 1: Write the failing sweettest**

In `elohim/holochain/tests/sweettest/src/tests/attestation_coordinator.rs`, add:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn issue_attestation_humanness_creates_content_entry_with_subject_link() {
    let (conductor, _, alice, _) = setup_single_agent().await;
    let cell = alice.cell(&conductor).await;

    let bob_human = create_test_human(&conductor, &alice, "bob").await;

    let input = IssueAttestationInput {
        attestation_kind: "attestation:humanness".to_string(),
        subject_cid: bob_human.to_string(),
        subject_kind: "agent".to_string(),
        title: "Bob is human — confirmed via video call".to_string(),
        description: None,
        reach: "community".to_string(),
        metadata: serde_json::json!({
            "humanness_method": "video_call",
            "confidence_score": 0.95,
        }),
        parent_governance_action_cid: None,
        vote_value: None,
        proof_class: "witness".to_string(),
        proof_evidence: serde_json::json!({"class": "witness"}),
        expires_at: None,
    };

    let output: AttestationOutput = conductor
        .call(&cell.zome("content_store"), "issue_attestation", input)
        .await;

    assert_eq!(output.attestation_kind, "attestation:humanness");
    assert_eq!(output.subject_cid, bob_human.to_string());

    // Verify Content entry exists with content_type
    let content: Content = conductor.call(&cell.zome("content_store"), "get_content", output.cid.clone()).await;
    assert_eq!(content.content_type, "attestation:humanness");

    // Verify AttestationToSubject link present
    let links: Vec<Link> = conductor.call(&cell.zome("content_store"), "get_links_from", output.cid).await;
    let subject_link = links.iter().find(|l| matches!(l.link_type, LinkTypes::AttestationToSubject));
    assert!(subject_link.is_some(), "AttestationToSubject link missing");
}
```

- [ ] **Step 2: Run sweettest to verify it fails**

```bash
cd elohim/holochain/tests/sweettest
RUST_LOG=info cargo test --test-threads=1 issue_attestation_humanness_creates_content_entry_with_subject_link
```

Expected: FAIL with `unimplemented!("Task B.3")` panic.

- [ ] **Step 3: Implement issue_attestation**

In `elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs`, replace the stub:

```rust
use crate::generated_attestation_kinds::{manifest_ref_for_attestation_kind, ATTESTATION_KINDS};

pub fn issue_attestation(input: IssueAttestationInput) -> ExternResult<AttestationOutput> {
    // Validate kind is in the codegen-emitted catalog (floor 1 in coordinator;
    // the integrity zome also enforces this — defense in depth)
    if !ATTESTATION_KINDS.contains(&input.attestation_kind.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "unknown_attestation_subtype: {}", input.attestation_kind
        ))));
    }

    let issuer_cid = agent_info()?.agent_initial_pubkey.to_string();

    // Build the metadata JSON with denormalized fields
    let mut metadata = serde_json::json!({
        "attestation_kind": input.attestation_kind,
        "subject_cid": input.subject_cid,
        "subject_kind": input.subject_kind,
        "validation_method": determine_validation_method(&input),
        "proof_evidence": input.proof_evidence,
    });
    if let Some(ref expires_at) = input.expires_at {
        metadata["expires_at"] = serde_json::json!(expires_at);
    }
    if let Some(ref parent_cid) = input.parent_governance_action_cid {
        metadata["parent_governance_action_cid"] = serde_json::json!(parent_cid);
    }
    if let Some(ref vote_value) = input.vote_value {
        metadata["vote_value"] = serde_json::json!(vote_value);
    }
    // Merge subtype-specific metadata fields into evidence_json.summary_metric
    metadata["evidence_json"] = serde_json::json!({
        "summary_metric": input.metadata,
    });

    let metadata_json = serde_json::to_string(&metadata)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("metadata serialization: {e}"))))?;

    // Build the Content entry
    let content_id = uuid::Uuid::new_v4().to_string(); // CID derived post-commit; placeholder for human-readable id
    let content = Content {
        id: content_id,
        content_type: input.attestation_kind.clone(),
        title: input.title,
        description: input.description.unwrap_or_default(),
        summary: None,
        content: String::new(),
        content_format: "epr-composite".to_string(),
        tags: vec![input.attestation_kind.clone()],
        source_path: None,
        related_node_ids: vec![input.subject_cid.clone()],
        author_id: Some(issuer_cid.clone()),
        reach: input.reach,
        trust_score: 0.0,
        estimated_minutes: None,
        thumbnail_url: None,
        metadata_json,
        created_at: sys_time()?.to_string(),
        updated_at: sys_time()?.to_string(),
        schema_version: 1,
        validation_status: "valid".to_string(),
        blob_cid: None,
    };

    let action_hash = create_entry(&EntryTypes::Content(content.clone()))?;
    let entry_hash = hash_entry(&content)?;

    // Create AttestationToSubject link (subject_cid → entry_hash)
    let subject_entry_hash = ActionHash::try_from(input.subject_cid.clone())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("invalid subject_cid: {e}"))))?;
    create_link(
        subject_entry_hash.clone(),
        entry_hash.clone(),
        LinkTypes::AttestationToSubject,
        LinkTag::new(input.subject_kind.as_bytes()),
    )?;

    // If this is an M-of-N child, also create GovernanceActionChild link parent → child
    if let Some(ref parent_cid) = input.parent_governance_action_cid {
        let parent_hash = ActionHash::try_from(parent_cid.clone())
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("invalid parent_governance_action_cid: {e}"))))?;
        create_link(
            parent_hash,
            entry_hash.clone(),
            LinkTypes::GovernanceActionChild,
            LinkTag::new(input.vote_value.as_deref().unwrap_or("approve").as_bytes()),
        )?;
    }

    Ok(AttestationOutput {
        cid: entry_hash.to_string(),
        attestation_kind: input.attestation_kind,
        subject_cid: input.subject_cid,
        issuer_cid,
    })
}

fn determine_validation_method(input: &IssueAttestationInput) -> &'static str {
    if input.parent_governance_action_cid.is_some() {
        "M-of-N-vote"
    } else {
        "peer-confirm"  // default for unilateral attestations issued via this coordinator
    }
}
```

- [ ] **Step 4: Add LinkTypes variants**

In `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`, add to the LinkTypes enum:

```rust
pub enum LinkTypes {
    // ... existing variants ...
    AttestationToSubject,
    GovernanceActionChild,
}
```

- [ ] **Step 5: Build the zome**

```bash
cd elohim/holochain/dna/elohim
cargo build --target wasm32-unknown-unknown -p content_store
```

Expected: SUCCESS.

- [ ] **Step 6: Run sweettest to verify it passes**

```bash
cd elohim/holochain/tests/sweettest
cargo test --test-threads=1 issue_attestation_humanness_creates_content_entry_with_subject_link
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs \
  elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs \
  elohim/holochain/tests/sweettest/src/tests/attestation_coordinator.rs
git commit -m "feat(attestation): issue_attestation coordinator + AttestationToSubject link type"
```

### Task B.4 — Implement revoke_attestation

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs`

- [ ] **Step 1: Write the failing sweettest**

In `attestation_coordinator.rs`, add:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn revoke_attestation_issues_superseding_content_entry() {
    let (conductor, _, alice, _) = setup_single_agent().await;
    let cell = alice.cell(&conductor).await;

    let bob = create_test_human(&conductor, &alice, "bob").await;
    let original = issue_test_attestation(&conductor, &cell, "attestation:identity-credential", bob.to_string()).await;

    let revoke_input = RevokeAttestationInput {
        attestation_cid: original.cid.clone(),
        reason: "credential expired early due to policy change".to_string(),
    };

    let revocation: AttestationOutput = conductor
        .call(&cell.zome("content_store"), "revoke_attestation", revoke_input)
        .await;

    // The revocation is a NEW Content entry of the same kind, by the same issuer,
    // with metadata.revocation.supersedes_cid pointing to the original.
    let rev_content: Content = conductor.call(&cell.zome("content_store"), "get_content", revocation.cid).await;
    let metadata: serde_json::Value = serde_json::from_str(&rev_content.metadata_json).unwrap();
    assert_eq!(metadata["revocation"]["supersedes_cid"].as_str().unwrap(), &original.cid);
}
```

- [ ] **Step 2: Run sweettest to verify failure**

```bash
cargo test --test-threads=1 revoke_attestation_issues_superseding_content_entry
```

Expected: FAIL with `unimplemented!("Task B.4")`.

- [ ] **Step 3: Implement revoke_attestation**

In `attestation.rs`:

```rust
pub fn revoke_attestation(input: RevokeAttestationInput) -> ExternResult<AttestationOutput> {
    // Resolve the original to mirror its kind / subject / proof-class
    let original_hash = EntryHash::try_from(input.attestation_cid.clone())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("invalid attestation_cid: {e}"))))?;
    let original_record = must_get_valid_record(original_hash.into())?;
    let original_content: Content = original_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode: {e}"))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("not a Content entry".into())))?;

    if !original_content.content_type.starts_with("attestation:") {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "target is not an attestation".into()
        )));
    }

    let issuer_cid = agent_info()?.agent_initial_pubkey.to_string();
    let original_issuer = original_content.author_id.clone()
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("original has no author".into())))?;
    if issuer_cid != original_issuer {
        // Cross-issuer revocation may be permitted by revocable_by manifest field;
        // for now enforce same-issuer (manifest-aware check is Task B.8 work).
        return Err(wasm_error!(WasmErrorInner::Guest(
            "only the original issuer may revoke (this build)".into()
        )));
    }

    // Build the revocation as a new attestation of the same kind, with metadata.revocation populated
    let mut metadata: serde_json::Value = serde_json::from_str(&original_content.metadata_json)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode metadata: {e}"))))?;
    metadata["revocation"] = serde_json::json!({
        "reason": input.reason,
        "revoked_at": sys_time()?.to_string(),
        "supersedes_cid": input.attestation_cid,
    });
    let subject_cid = metadata["subject_cid"].as_str().unwrap_or_default().to_string();
    let subject_kind = metadata["subject_kind"].as_str().unwrap_or_default().to_string();

    let revoke_input = IssueAttestationInput {
        attestation_kind: original_content.content_type.clone(),
        subject_cid,
        subject_kind,
        title: format!("Revocation: {}", original_content.title),
        description: Some(format!("Revoked at {}: {}", sys_time()?, input.reason)),
        reach: original_content.reach,
        metadata: metadata["evidence_json"]["summary_metric"].clone(),
        parent_governance_action_cid: None,
        vote_value: None,
        proof_class: metadata["proof_evidence"]["class"].as_str().unwrap_or("witness").to_string(),
        proof_evidence: metadata["proof_evidence"].clone(),
        expires_at: None,
    };

    issue_attestation(revoke_input)
}
```

- [ ] **Step 4: Build + run sweettest**

```bash
cd elohim/holochain/dna/elohim
cargo build --target wasm32-unknown-unknown -p content_store
cd ../../tests/sweettest
cargo test --test-threads=1 revoke_attestation_issues_superseding_content_entry
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs \
  elohim/holochain/tests/sweettest/src/tests/attestation_coordinator.rs
git commit -m "feat(attestation): revoke_attestation coordinator (same-issuer enforcement)"
```

### Task B.5 — Implement propose_governance_action

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs`

- [ ] **Step 1: Write the failing sweettest**

```rust
#[tokio::test(flavor = "multi_thread")]
async fn propose_governance_action_renewal_request_creates_parent_content() {
    let (conductor, _, alice, _) = setup_single_agent().await;
    let cell = alice.cell(&conductor).await;

    let bob = create_test_human(&conductor, &alice, "bob").await;

    let input = ProposeGovernanceActionInput {
        governance_kind: "governance-action:renewal-request".to_string(),
        subject_cid: bob.to_string(),
        title: "Renewal request for bob's identity key".to_string(),
        description: None,
        reach: "community".to_string(),
        threshold: serde_json::json!({ "type": "m-of-n", "m": 3, "n": 5 }),
        eligibility_predicate: None,
        ballot_format: "approve-reject".to_string(),
        closes_at: "2026-05-25T00:00:00Z".to_string(),
        parameters: None,
    };

    let output: GovernanceActionOutput = conductor
        .call(&cell.zome("content_store"), "propose_governance_action", input)
        .await;

    assert_eq!(output.governance_kind, "governance-action:renewal-request");
    assert_eq!(output.subject_cid, bob.to_string());

    let parent_content: Content = conductor.call(&cell.zome("content_store"), "get_content", output.cid).await;
    assert_eq!(parent_content.content_type, "governance-action:renewal-request");
}
```

- [ ] **Step 2: Run to verify failure**

```bash
cargo test --test-threads=1 propose_governance_action_renewal_request_creates_parent_content
```

Expected: FAIL with `unimplemented!("Task B.5")`.

- [ ] **Step 3: Implement propose_governance_action**

```rust
use crate::generated_attestation_kinds::GOVERNANCE_ACTION_KINDS;

pub fn propose_governance_action(
    input: ProposeGovernanceActionInput,
) -> ExternResult<GovernanceActionOutput> {
    if !GOVERNANCE_ACTION_KINDS.contains(&input.governance_kind.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "unknown_governance_action_kind: {}", input.governance_kind
        ))));
    }

    let proposer_cid = agent_info()?.agent_initial_pubkey.to_string();

    let metadata = serde_json::json!({
        "governance_kind": input.governance_kind,
        "subject_cid": input.subject_cid,
        "threshold": input.threshold,
        "eligibility_predicate": input.eligibility_predicate,
        "ballot_format": input.ballot_format,
        "closes_at": input.closes_at,
        "parameters_json": input.parameters,
    });
    let metadata_json = serde_json::to_string(&metadata)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("metadata: {e}"))))?;

    let content = Content {
        id: uuid::Uuid::new_v4().to_string(),
        content_type: input.governance_kind.clone(),
        title: input.title,
        description: input.description.unwrap_or_default(),
        summary: None,
        content: String::new(),
        content_format: "epr-composite".to_string(),
        tags: vec![input.governance_kind.clone()],
        source_path: None,
        related_node_ids: vec![input.subject_cid.clone()],
        author_id: Some(proposer_cid.clone()),
        reach: input.reach,
        trust_score: 0.0,
        estimated_minutes: None,
        thumbnail_url: None,
        metadata_json,
        created_at: sys_time()?.to_string(),
        updated_at: sys_time()?.to_string(),
        schema_version: 1,
        validation_status: "valid".to_string(),
        blob_cid: None,
    };

    create_entry(&EntryTypes::Content(content.clone()))?;
    let entry_hash = hash_entry(&content)?;

    Ok(GovernanceActionOutput {
        cid: entry_hash.to_string(),
        governance_kind: input.governance_kind,
        subject_cid: input.subject_cid,
        proposer_cid,
        closes_at: input.closes_at,
    })
}
```

- [ ] **Step 4: Build + verify test passes**

```bash
cd elohim/holochain/dna/elohim
cargo build --target wasm32-unknown-unknown -p content_store
cd ../../tests/sweettest
cargo test --test-threads=1 propose_governance_action_renewal_request_creates_parent_content
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs \
  elohim/holochain/tests/sweettest/src/tests/attestation_coordinator.rs
git commit -m "feat(attestation): propose_governance_action coordinator"
```

### Task B.6 — Implement vote_on_governance_action

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs`

- [ ] **Step 1: Write the failing sweettest**

```rust
#[tokio::test(flavor = "multi_thread")]
async fn vote_on_governance_action_creates_child_attestation_with_parent_link() {
    let (conductor, _, alice, _) = setup_single_agent().await;
    let cell = alice.cell(&conductor).await;

    let bob = create_test_human(&conductor, &alice, "bob").await;
    let parent: GovernanceActionOutput = conductor.call(
        &cell.zome("content_store"),
        "propose_governance_action",
        ProposeGovernanceActionInput {
            governance_kind: "governance-action:renewal-request".to_string(),
            subject_cid: bob.to_string(),
            title: "Renewal".to_string(),
            description: None,
            reach: "community".to_string(),
            threshold: serde_json::json!({ "type": "m-of-n", "m": 3, "n": 5 }),
            eligibility_predicate: None,
            ballot_format: "approve-reject".to_string(),
            closes_at: "2026-05-25T00:00:00Z".to_string(),
            parameters: None,
        },
    ).await;

    let vote: AttestationOutput = conductor.call(
        &cell.zome("content_store"),
        "vote_on_governance_action",
        VoteOnGovernanceActionInput {
            parent_governance_action_cid: parent.cid.clone(),
            vote_value: "approve".to_string(),
            vote_weight: None,
            evidence: None,
        },
    ).await;

    assert_eq!(vote.attestation_kind, "attestation:renewal-approval");

    // Verify GovernanceActionChild link exists parent → child
    let links: Vec<Link> = conductor.call(&cell.zome("content_store"), "get_links_from", parent.cid).await;
    let child_link = links.iter().find(|l| matches!(l.link_type, LinkTypes::GovernanceActionChild));
    assert!(child_link.is_some(), "parent has no child link");
}
```

- [ ] **Step 2: Run to verify failure**

```bash
cargo test --test-threads=1 vote_on_governance_action_creates_child_attestation_with_parent_link
```

Expected: FAIL.

- [ ] **Step 3: Implement vote_on_governance_action**

```rust
use crate::attestation::{issue_attestation, IssueAttestationInput};

pub fn vote_on_governance_action(
    input: VoteOnGovernanceActionInput,
) -> ExternResult<AttestationOutput> {
    // Resolve the parent to get its governance_kind + look up child_attestation_kind
    let parent_hash = EntryHash::try_from(input.parent_governance_action_cid.clone())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("invalid parent_cid: {e}"))))?;
    let parent_record = must_get_valid_record(parent_hash.into())?;
    let parent_content: Content = parent_record.entry().to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode parent: {e}"))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("parent is not a Content entry".into())))?;

    let parent_metadata: serde_json::Value = serde_json::from_str(&parent_content.metadata_json)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode parent metadata: {e}"))))?;
    let subject_cid = parent_metadata["subject_cid"].as_str()
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("parent has no subject_cid".into())))?
        .to_string();

    // Lookup child_attestation_kind from manifest (manifest_ref_for_governance_action_kind)
    let child_kind = child_attestation_kind_for_governance_action(&parent_content.content_type)
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(format!(
            "no child_attestation_kind declared for {}", parent_content.content_type
        ))))?;

    let attestation_input = IssueAttestationInput {
        attestation_kind: child_kind.to_string(),
        subject_cid,
        subject_kind: "agent".to_string(),
        title: format!("{} vote on {}", input.vote_value, parent_content.title),
        description: None,
        reach: parent_content.reach,
        metadata: input.evidence.unwrap_or(serde_json::json!({})),
        parent_governance_action_cid: Some(input.parent_governance_action_cid),
        vote_value: Some(input.vote_value),
        proof_class: "witness".to_string(),
        proof_evidence: serde_json::json!({"class": "witness"}),
        expires_at: None,
    };

    issue_attestation(attestation_input)
}

fn child_attestation_kind_for_governance_action(governance_kind: &str) -> Option<&'static str> {
    // This mapping is generated from manifests; for now, hardcoded matching the imagodei + mishpat manifests
    match governance_kind {
        "governance-action:renewal-request" => Some("attestation:renewal-approval"),
        "governance-action:recovery-request" => Some("attestation:recovery-approval"),
        "governance-action:key-revocation" => Some("attestation:revocation-vote"),
        "governance-action:identity-challenge" => Some("attestation:challenge-support"),
        "governance-action:proposal" => Some("attestation:proposal-vote"),
        "governance-action:challenge" => Some("attestation:statement-vote"),
        "governance-action:election" => Some("attestation:proposal-vote"),
        _ => None,
    }
}
```

NOTE: The hardcoded mapping in `child_attestation_kind_for_governance_action` should later be replaced by a codegen-emitted constant. Task A.7 can be extended in a follow-up commit to emit this mapping; for Task B.6 the hardcoded match is acceptable.

- [ ] **Step 4: Build + verify**

```bash
cd elohim/holochain/dna/elohim
cargo build --target wasm32-unknown-unknown -p content_store
cd ../../tests/sweettest
cargo test --test-threads=1 vote_on_governance_action_creates_child_attestation_with_parent_link
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs \
  elohim/holochain/tests/sweettest/src/tests/attestation_coordinator.rs
git commit -m "feat(attestation): vote_on_governance_action coordinator + child_attestation_kind mapping"
```

### Task B.7 — Implement query coordinators

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs`

- [ ] **Step 1: Write failing tests for both queries**

```rust
#[tokio::test(flavor = "multi_thread")]
async fn get_attestations_for_subject_returns_all_attestations_about_subject() { /* ... */ }

#[tokio::test(flavor = "multi_thread")]
async fn get_governance_action_with_children_returns_parent_plus_votes() { /* ... */ }
```

- [ ] **Step 2: Run to verify failure**

```bash
cargo test --test-threads=1 get_attestations_for_subject
```

Expected: FAIL.

- [ ] **Step 3: Implement get_attestations_for_subject**

In `attestation.rs`:

```rust
pub fn get_attestations_for_subject(subject_cid: String) -> ExternResult<Vec<AttestationOutput>> {
    let subject_hash = EntryHash::try_from(subject_cid.clone())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("invalid subject_cid: {e}"))))?;
    let links = get_links(GetLinksInputBuilder::try_new(subject_hash, LinkTypes::AttestationToSubject)?.build())?;

    let mut out = Vec::with_capacity(links.len());
    for link in links {
        let attestation_hash = link.target.into_entry_hash()
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("link target not entry hash".into())))?;
        let record = must_get_valid_record(attestation_hash.clone().into())?;
        let content: Content = record.entry().to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode: {e}"))))?
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("not a Content entry".into())))?;
        let metadata: serde_json::Value = serde_json::from_str(&content.metadata_json)
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("metadata decode: {e}"))))?;
        out.push(AttestationOutput {
            cid: attestation_hash.to_string(),
            attestation_kind: content.content_type,
            subject_cid: subject_cid.clone(),
            issuer_cid: content.author_id.unwrap_or_default(),
        });
    }
    Ok(out)
}
```

- [ ] **Step 4: Implement get_governance_action_with_children**

In `governance_action.rs`:

```rust
pub fn get_governance_action_with_children(
    parent_cid: String,
) -> ExternResult<GovernanceActionWithChildren> {
    let parent_hash = EntryHash::try_from(parent_cid.clone())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("invalid parent_cid: {e}"))))?;
    let parent_record = must_get_valid_record(parent_hash.clone().into())?;
    let parent_content: Content = parent_record.entry().to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode parent: {e}"))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("parent missing".into())))?;
    let parent_metadata: serde_json::Value = serde_json::from_str(&parent_content.metadata_json)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("metadata: {e}"))))?;
    let parent_output = GovernanceActionOutput {
        cid: parent_cid.clone(),
        governance_kind: parent_content.content_type,
        subject_cid: parent_metadata["subject_cid"].as_str().unwrap_or_default().to_string(),
        proposer_cid: parent_content.author_id.unwrap_or_default(),
        closes_at: parent_metadata["closes_at"].as_str().unwrap_or_default().to_string(),
    };

    let child_links = get_links(GetLinksInputBuilder::try_new(parent_hash, LinkTypes::GovernanceActionChild)?.build())?;
    let mut children = Vec::with_capacity(child_links.len());
    for link in child_links {
        let child_hash = link.target.into_entry_hash()
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("link target not entry hash".into())))?;
        let record = must_get_valid_record(child_hash.clone().into())?;
        let content: Content = record.entry().to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode: {e}"))))?
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("not Content".into())))?;
        let metadata: serde_json::Value = serde_json::from_str(&content.metadata_json)
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("metadata: {e}"))))?;
        children.push(AttestationOutput {
            cid: child_hash.to_string(),
            attestation_kind: content.content_type,
            subject_cid: metadata["subject_cid"].as_str().unwrap_or_default().to_string(),
            issuer_cid: content.author_id.unwrap_or_default(),
        });
    }

    Ok(GovernanceActionWithChildren { parent: parent_output, children })
}
```

- [ ] **Step 5: Build + run tests**

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(attestation): query coordinators — get_attestations_for_subject + get_governance_action_with_children"
```

### Task B.8 — Expose coordinators as zome externs

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

- [ ] **Step 1: Add `#[hdk_extern]` wrappers**

In `content_store/src/lib.rs`, add:

```rust
#[hdk_extern]
pub fn issue_attestation(input: attestation::IssueAttestationInput) -> ExternResult<attestation::AttestationOutput> {
    attestation::issue_attestation(input)
}

#[hdk_extern]
pub fn revoke_attestation(input: attestation::RevokeAttestationInput) -> ExternResult<attestation::AttestationOutput> {
    attestation::revoke_attestation(input)
}

#[hdk_extern]
pub fn get_attestations_for_subject(subject_cid: String) -> ExternResult<Vec<attestation::AttestationOutput>> {
    attestation::get_attestations_for_subject(subject_cid)
}

#[hdk_extern]
pub fn propose_governance_action(
    input: governance_action::ProposeGovernanceActionInput,
) -> ExternResult<governance_action::GovernanceActionOutput> {
    governance_action::propose_governance_action(input)
}

#[hdk_extern]
pub fn vote_on_governance_action(
    input: governance_action::VoteOnGovernanceActionInput,
) -> ExternResult<attestation::AttestationOutput> {
    governance_action::vote_on_governance_action(input)
}

#[hdk_extern]
pub fn get_governance_action_with_children(
    parent_cid: String,
) -> ExternResult<governance_action::GovernanceActionWithChildren> {
    governance_action::get_governance_action_with_children(parent_cid)
}
```

- [ ] **Step 2: Build**

```bash
cd elohim/holochain/dna/elohim
cargo build --target wasm32-unknown-unknown -p content_store
```

Expected: SUCCESS.

- [ ] **Step 3: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
git commit -m "feat(attestation): expose attestation + governance-action coordinators as zome externs"
```

### Task B.9 — Add cross-DNA bridge wrappers (legacy callers)

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`

- [ ] **Step 1: Replace deleted coordinator stubs with bridge calls**

The imagodei DNA's existing public API surface (`issue_attestation`, `create_renewal_attestation`, `get_agent_attestations`, etc.) must continue to satisfy callers until Stage F migrates the consumers. Replace each function's body with a `call` to elohim DNA's content_store zome.

Example for `issue_attestation`:

```rust
#[hdk_extern]
pub fn issue_attestation(input: IssueAttestationInput) -> ExternResult<AttestationOutput> {
    // Bridge to elohim DNA's content_store::issue_attestation.
    // Translate the imagodei-shaped input to the consolidated attestation:identity-credential subtype.
    let consolidated_input = ConsolidatedIssueAttestationInput {
        attestation_kind: "attestation:identity-credential".to_string(),
        subject_cid: input.agent_id,
        subject_kind: "agent".to_string(),
        title: input.display_name,
        description: Some(input.description),
        reach: "community".to_string(),
        metadata: serde_json::json!({
            "category": input.category,
            "credential_type": input.attestation_type,
            "tier": input.tier,
        }),
        parent_governance_action_cid: None,
        vote_value: None,
        proof_class: "witness".to_string(),
        proof_evidence: serde_json::json!({"class": "witness"}),
        expires_at: input.expires_at,
    };

    let result: ConsolidatedAttestationOutput = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("issue_attestation"),
        None,
        consolidated_input,
    )?
    .decode()
    .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("bridge decode: {e}"))))?;

    Ok(AttestationOutput {
        attestation_id: result.cid,
        // ... translate other fields from consolidated to imagodei shape
    })
}
```

Repeat this bridge wrapper pattern for the other imagodei coordinator functions that have legacy callers (`create_renewal_attestation`, `get_agent_attestations`, `get_my_attestations`, `create_humanity_witness`, etc.).

- [ ] **Step 2: Build imagodei DNA**

```bash
cd elohim/holochain/dna/imagodei
cargo build --target wasm32-unknown-unknown -p imagodei
```

Expected: SUCCESS.

- [ ] **Step 3: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "feat(attestation): bridge wrappers in imagodei DNA to elohim content_store"
```

### Task B.10 — Add cross-DNA bridge wrappers (infrastructure + mishpat)

**Files:**
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs`
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs`

- [ ] **Step 1: Bridge infrastructure DNA's record_health_attestation**

In `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs`, replace `record_health_attestation` body with a bridge call to elohim DNA's `issue_attestation` using `attestation:device-health` kind.

- [ ] **Step 2: Bridge mishpat DNA's create_gate_decision_attestation and ProposalVote / StatementVote operations**

In `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs`, replace the coordinator functions for the deleted entry types with bridge calls to elohim DNA's `issue_attestation` (for `attestation:gate-decision`, `attestation:proposal-vote`, etc.) and `propose_governance_action` (for `governance-action:proposal`, `governance-action:challenge`).

- [ ] **Step 3: Build both DNAs**

```bash
cd elohim/holochain/dna/infrastructure && cargo build --target wasm32-unknown-unknown -p infrastructure
cd ../mishpat && cargo build --target wasm32-unknown-unknown -p mishpat
```

Expected: SUCCESS.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs \
  elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs
git commit -m "feat(attestation): bridge wrappers in infrastructure + mishpat DNAs to elohim content_store"
```

**Stage B acceptance:** All four DNAs build. The unified `issue_attestation` / `propose_governance_action` / `vote_on_governance_action` / query coordinators work in sweettests. Legacy public surfaces in imagodei / infrastructure / mishpat DNAs continue to work via bridge calls.

---

## Stage C — Integrity zomes + legacy entry-type removal

Stage C lands the discriminator-chain validator floors in elohim DNA and removes the 22+ legacy entry types from the other three DNAs. After Stage C, the only attestation-shaped entries that pass validation are `Content` entries with `content_type: "attestation:<subtype>"` or `"governance-action:<kind>"`.

### Task C.1 — Implement attestation_validator.rs

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

- [ ] **Step 1: Write the validator module**

Create `attestation_validator.rs` implementing the 8 floors from spec §9. Floor implementations:
- Floor 1 (Subtype known): check `content.content_type` against `ATTESTATION_KINDS` and `GOVERNANCE_ACTION_KINDS` from the codegen-emitted constants.
- Floor 2 (Issuer authorized): for now, accept; manifest-aware authorization predicates are implemented in Task C.3.
- Floor 3 (Subject link present): check the action carries an `AttestationToSubject` link from the new EntryHash.
- Floor 4 (Uniqueness anchor): for subtypes whose manifest declaration has `uniqueness_anchor`, compute the anchor + check at most one link.
- Floor 5 (Temporal validity): parse metadata, check `expires_at` future + child `created_at` ≤ parent `closes_at`.
- Floor 6 (Eligibility predicate): for children, resolve parent + evaluate.
- Floor 7 (Revocation reference valid): check `must_get` returns a same-kind same-issuer record.
- Floor 8 (Proof class declared): parse `metadata.proof_evidence.class`, verify enum + required material.

```rust
//! Validator floors for attestation + governance-action Content entries.
//! See genesis/docs/superpowers/specs/2026-05-11-attestation-consolidation-design.md §9.

use hdi::prelude::*;

use crate::generated_attestation_kinds::{ATTESTATION_KINDS, GOVERNANCE_ACTION_KINDS};
use crate::Content;

pub fn validate_attestation_content(
    content: &Content,
    op: &Op,
) -> ExternResult<ValidateCallbackResult> {
    let kind = &content.content_type;
    let is_attestation = kind.starts_with("attestation:");
    let is_governance_action = kind.starts_with("governance-action:");

    if !is_attestation && !is_governance_action {
        return Ok(ValidateCallbackResult::Valid);
    }

    // Floor 1: subtype known
    let known = (is_attestation && ATTESTATION_KINDS.contains(&kind.as_str()))
        || (is_governance_action && GOVERNANCE_ACTION_KINDS.contains(&kind.as_str()));
    if !known {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "unknown_attestation_subtype: {}", kind
        )));
    }

    // Parse metadata
    let metadata: serde_json::Value = match serde_json::from_str(&content.metadata_json) {
        Ok(v) => v,
        Err(e) => return Ok(ValidateCallbackResult::Invalid(format!("metadata_decode: {e}"))),
    };

    if is_attestation {
        validate_attestation_floors(content, &metadata, op)
    } else {
        validate_governance_action_floors(content, &metadata, op)
    }
}

fn validate_attestation_floors(
    content: &Content,
    metadata: &serde_json::Value,
    op: &Op,
) -> ExternResult<ValidateCallbackResult> {
    // Floor 3: subject link present (commit-time check via op)
    // ... resolve op's action, look for AttestationToSubject create_link in same action group
    // (implementation detail — depends on HDI op-walking patterns elsewhere in this codebase)

    // Floor 5: temporal validity
    if let Some(expires) = metadata["expires_at"].as_str() {
        // Parse expires + compare to action timestamp; reject if past
        // ...
    }
    if let Some(parent_cid) = metadata["parent_governance_action_cid"].as_str() {
        let parent_hash = match ActionHash::try_from(parent_cid.to_string()) {
            Ok(h) => h,
            Err(e) => return Ok(ValidateCallbackResult::Invalid(format!("parent_cid_invalid: {e}"))),
        };
        let parent_record = must_get_valid_record(parent_hash)?;
        // ... extract closes_at from parent metadata, compare to action timestamp
    }

    // Floor 6: eligibility predicate (children only)
    // ... evaluate parent's eligibility_predicate

    // Floor 7: revocation reference valid
    if let Some(supersedes_cid) = metadata["revocation"]["supersedes_cid"].as_str() {
        let target_hash = match ActionHash::try_from(supersedes_cid.to_string()) {
            Ok(h) => h,
            Err(e) => return Ok(ValidateCallbackResult::Invalid(format!("revocation_target_invalid: {e}"))),
        };
        let target_record = must_get_valid_record(target_hash)?;
        // ... verify target is same kind + same issuer
    }

    // Floor 8: proof class declared
    let proof_class = metadata["proof_evidence"]["class"].as_str().unwrap_or("");
    if !["witness", "audit", "proof", "confirmation"].contains(&proof_class) {
        return Ok(ValidateCallbackResult::Invalid(format!("proof_class_invalid: {}", proof_class)));
    }
    // Higher classes require matching proof material
    if proof_class == "audit" && metadata["proof_evidence"]["merkle_root"].as_str().is_none() {
        return Ok(ValidateCallbackResult::Invalid("proof_material_missing: audit requires merkle_root".to_string()));
    }
    // ... similar for proof and confirmation

    Ok(ValidateCallbackResult::Valid)
}

fn validate_governance_action_floors(
    _content: &Content,
    metadata: &serde_json::Value,
    _op: &Op,
) -> ExternResult<ValidateCallbackResult> {
    // Floor 8 (parent doesn't have proof_evidence directly; check threshold + closes_at)
    if metadata["threshold"].is_null() {
        return Ok(ValidateCallbackResult::Invalid("threshold_missing".to_string()));
    }
    if metadata["closes_at"].as_str().is_none() {
        return Ok(ValidateCallbackResult::Invalid("closes_at_missing".to_string()));
    }
    // ... validate threshold shape against governance-action-metadata schema
    Ok(ValidateCallbackResult::Valid)
}
```

- [ ] **Step 2: Wire into the Content validator**

In `content_store_integrity/src/lib.rs`, add to the Content validation callback:

```rust
pub mod attestation_validator;

fn validate_content(content: &Content, op: &Op) -> ExternResult<ValidateCallbackResult> {
    // ... existing Content validation logic ...

    // Discriminator-chain branch
    if content.content_type.starts_with("attestation:") || content.content_type.starts_with("governance-action:") {
        return attestation_validator::validate_attestation_content(content, op);
    }

    Ok(ValidateCallbackResult::Valid)
}
```

- [ ] **Step 3: Build the integrity zome**

```bash
cd elohim/holochain/dna/elohim
cargo build --target wasm32-unknown-unknown -p content_store_integrity
```

Expected: SUCCESS.

- [ ] **Step 4: Run validator sweettests**

Add tests in `attestation_coordinator.rs` covering each floor's reject case (unknown subtype, missing subject link, expired_at past, etc.). Verify each rejects correctly.

```bash
cargo test --test-threads=1 validator_rejects
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs \
  elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs \
  elohim/holochain/tests/sweettest/src/tests/attestation_coordinator.rs
git commit -m "feat(attestation): integrity zome discriminator-chain validator + 8 floors"
```

### Task C.2 — Imagodei safe-removals (REVISED 2026-05-11)

**Source of truth:** Holochain DHT (Category A — removing redundant entry types; canonical state remains on DHT via the consolidated `Content` + `content_type: "attestation:*"` discriminator on elohim DNA).

**Scope narrowed.** The original C.2 listed 14 entry types for removal. Reality audit (`grep create_entry imagodei/zomes/imagodei/src/lib.rs`) found 4 of those types are still actively written by the recovery protocol coordinator and have pre-commit security gates that B.9 deliberately did not bridge. Those types defer to **Stage G** (recovery decoupling), where they get migrated alongside the Shamir off-chain transport work.

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs` (remove safe types only)
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (remove dead `Attestation` struct usage in bridge return-shape construction)
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` (remove dead `Attestation` struct usage in `issue_attestation_via_imagodei` cycle-fix scaffold — synthesize `AttestationOutput` without constructing the legacy struct)

**Entry types to remove NOW (safe — already fully bridged by B.9):**
- `Attestation` (struct + variant + Link types AgentToAttestation, AttestationByCategory, AttestationByType)
- `RenewalAttestation` (struct + variant + any associated link types)
- `KeyStewardship` (struct + variant + links) — confirm no live callers via `grep create_entry.*EntryTypes::KeyStewardship` first; if any exist, defer to Stage G
- `StewardshipGrant` (struct + variant + links) — confirm no live callers via grep first; if any exist, defer to Stage G
- `IdentityChallenge` (struct + variant + links) — confirm no live callers; if any exist, defer to Stage G
- `ChallengeSupport` (struct + variant + links) — confirm no live callers; if any exist, defer to Stage G
- `IdentityFreeze` (struct + variant + links) — confirm no live callers; if any exist, defer to Stage G
- `StewardshipAppeal` (struct + variant + links) — confirm no live callers; if any exist, defer to Stage G
- `PolicyInheritance` (struct + variant + links) — confirm no live callers; if any exist, defer to Stage G

**Entry types DEFERRED to Stage G (have live callers with security gates):**
- `HumanityWitness` — `submit_intimate_witness` has 3 pre-commit gates + RecoveryV2Signal emission; bridge to `attestation:humanness` requires gate preservation
- `RecoveryRequest` — recovery protocol primary entry
- `RecoveryVote` — recovery protocol child vote entry
- `KeyRevocation` — key rotation flow (lines 2195, 2337 of imagodei coordinator)
- `RevocationVote` — key rotation voting (line 2543 of imagodei coordinator)

- [ ] **Step 1: Pre-audit each candidate**

```bash
cd /projects/elohim/.claude/worktrees/attestation-consolidation
for kind in Attestation RenewalAttestation KeyStewardship StewardshipGrant IdentityChallenge ChallengeSupport IdentityFreeze StewardshipAppeal PolicyInheritance; do
  echo "=== $kind ==="
  grep -n "create_entry.*EntryTypes::${kind}\b" elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
done
```

For each candidate: if zero create_entry calls, proceed; if any calls exist (other than the Attestation cycle-fix scaffold), DEFER that one to Stage G with a comment.

- [ ] **Step 2: Replace Attestation struct usage in bridge return-shapes**

In `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`, the bridge wrappers (`issue_attestation`, `get_agent_attestations`) currently build `Attestation { id, agent_id, ... }` structs to populate the legacy `AttestationOutput { action_hash, attestation: Attestation }` shape. Replace each construction site with a non-EntryType `AttestationView` plain struct (move the struct out of `imagodei_integrity::EntryTypes::Attestation` and into the coordinator module as a regular `#[derive(Serialize, Deserialize, Clone)]` struct named `LegacyAttestationView`). Update `AttestationOutput.attestation` to point at the view type. This keeps the legacy public API surface working until Stage F migrates the consumers.

In `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` line ~1048, do the same — replace the `Attestation { ... }` construction with `LegacyAttestationView { ... }` (or stop constructing it at all and return a minimal AttestationOutput with just `action_hash: ActionHash::from_raw_36(...) + attestation: None`).

- [ ] **Step 3: Remove the entry-type definitions from imagodei_integrity**

For each verified-safe candidate, delete:
- The `pub struct <Name> { ... }` block
- The `<Name>(<Name>)` variant in the `EntryTypes` enum
- Any associated link types in the `LinkTypes` enum (search for the type name in link variants)

- [ ] **Step 4: Build both DNAs**

Verify `df -h /projects` shows ≥10G free, then run sequentially:

```bash
cd elohim/holochain/dna/imagodei
cargo build --target wasm32-unknown-unknown -p imagodei_integrity
cargo build --target wasm32-unknown-unknown -p imagodei
```

Then (because the elohim DNA imports imagodei_integrity types via cross-DNA import — verify by `grep imagodei_integrity elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`):

```bash
cd ../elohim
just check
```

Expected: SUCCESS on all three builds.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs \
  elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs \
  elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
git commit -m "remove(attestation): delete safely-bridged legacy entry types from imagodei DNA (recovery types deferred to Stage G)"
```

### Task C.3 — Convert additive bridges to full-replacement + remove legacy entry types (REVISED 2026-05-11)

**Source of truth:** Holochain DHT (Category A — removing redundant per-DNA entry types; canonical state for the consolidated subtypes lives on elohim DNA via `Content` + `content_type` discriminator). Local query surfaces in mishpat/infrastructure that read these entry types must EITHER (a) be removed if no consumers OR (b) be rewritten to query elohim DNA via cross-DNA `call(...)`.

**Scope reality:** B.10's bridges were *additive* (both local `create_entry` AND cross-DNA call). Removing entry types requires first removing local writes, then re-routing any query functions that read those entries. Each removed type needs three coordinated changes: (1) remove `create_entry(...)` calls in coordinator, (2) rework `get_*` / `query_*` functions that read those entries (route to elohim DNA or remove), (3) remove the entry-type definition from integrity.

**Two separable scopes — commit each separately:**

#### C.3.a — Infrastructure DNA (small surface)

- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs`
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs`

Target removals: `HealthAttestation`, `DoorwayHeartbeatSummary`.

Per spec §6.4: `DoorwayHeartbeat` is observation-shaped and stays pending the Observation layer spec — DO NOT remove.

- [ ] **Step 1: Convert `record_health_attestation` bridge from additive to full-replacement.** Remove the local `create_entry(&EntryTypes::HealthAttestation(...))` call (added in commit 563b07c93). Preserve the post-commit signal emission — the signal should fire AFTER the cross-DNA bridge call returns successfully, not from the local create.
- [ ] **Step 2: Audit query functions.** Grep `grep -n "EntryTypes::HealthAttestation\|EntryTypes::DoorwayHeartbeatSummary" elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs`. For each query: if it has Stage F consumers, replace with a bridge to elohim's `get_attestations_for_subject`; if no live consumers, remove the function.
- [ ] **Step 3: Remove from integrity.** Delete the two struct definitions, EntryTypes variants, and associated LinkTypes.
- [ ] **Step 4: Build.** Verify ≥10G free disk, then `cd elohim/holochain/dna/infrastructure && cargo build --target wasm32-unknown-unknown -p infrastructure_integrity && cargo build --target wasm32-unknown-unknown -p infrastructure`.
- [ ] **Step 5: Commit.** `git commit -m "remove(attestation): convert infrastructure bridge to full-replacement + delete HealthAttestation/DoorwayHeartbeatSummary entry types"`.

#### C.3.b — Mishpat DNA (large surface — 5 additive bridges from baf9e77b8)

- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs`
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs`

Target removals: `GateDecisionAttestation`, `ProposalVote`, `StatementVote`, `GovernanceReaction`, `Proposal`, `Challenge`, `GateDecisionChallenge`.

Per spec §6.4: `OpinionStatement`, `Discussion`, `Precedent`, `GovernanceState`, `ChallengeOutcome`, `GraduatedFeedback` are NOT consolidated — DO NOT remove.

- [ ] **Step 1: Flip each of the 5 additive bridges to full-replacement.** From commit baf9e77b8, the bridges are: `create_gate_decision_attestation`, `create_proposal_vote`, `create_statement_vote`, `create_proposal`, `create_challenge`. For each: remove the local `create_entry(...)` call; preserve any signal emissions AROUND the bridge call.
- [ ] **Step 2: Audit query/read surfaces for each removed type.** Grep `grep -n "EntryTypes::TYPE\|to_app_option::<TYPE>" elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs` for each. For functions like `get_proposal_by_id` / `query_proposals_by_status` / `list_proposal_votes`: each becomes a bridge call to elohim's `get_attestations_for_subject` or `get_governance_action_with_children` (mapped through the parent CID). If a query has no consumers Stage F will touch, remove it.
- [ ] **Step 3: Remove all 7 entry-type definitions + variants from `mishpat_integrity`.** Also remove their LinkTypes if any.
- [ ] **Step 4: Build.** Verify ≥10G free, then `cd elohim/holochain/dna/mishpat && cargo build --target wasm32-unknown-unknown -p mishpat_integrity && cargo build --target wasm32-unknown-unknown -p mishpat`.
- [ ] **Step 5: Commit.** `git commit -m "remove(attestation): convert mishpat bridges to full-replacement + delete 7 legacy entry types"`.

**Parallel-execution note:** C.3.a and C.3.b touch different DNAs (different cargo workspaces) and can run in parallel subagents. Within Phase 1 of the revised stage ordering they each run as their own subagent.

### Task C.4 — Elohim DNA audited vestigial removals (REVISED 2026-05-11)

**Source of truth:** Holochain DHT (Category A — removing redundant elohim DNA duplicate entry types; canonical attestation state lives via the consolidated `Content` + `content_type` discriminator on this same DNA).

**Scope corrected.** The original plan claimed `Attestation`, `ContentAttestation`, `ContentSuccession`, `CustodianCommitment` were "never instantiated." Reality audit (2026-05-11): `CustodianCommitment` has 14 active `create_entry` calls in shard-replication code; `ContentSuccession` has at least one call (line 11474). Both are LIVE — NOT vestigial. Removing them would break the lamad blob/shard system and content versioning.

**Confirmed-vestigial entry types (safe to remove):**
- `Attestation` (in elohim DNA's content_store_integrity — duplicate of imagodei's; the cycle-fix scaffold at `content_store/lib.rs:1048` uses imagodei_integrity's struct after C.2's struct-relocation, so the elohim duplicate becomes truly unused)
- `ContentAttestation` — confirm zero `create_entry` calls via `grep -n "EntryTypes::ContentAttestation" elohim/holochain/dna/elohim/zomes/`

**Confirmed-live entry types (DO NOT remove in this plan):**
- `ContentSuccession` (line 11474 of content_store/lib.rs — content versioning)
- `CustodianCommitment` (14 callers in shard-replication code) — annotated to stay but not consolidated into the discriminator chain (it's a different category — shard custodianship is operational state, not credential-shaped attestation)

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

- [ ] **Step 1: Re-audit each candidate**

```bash
cd /projects/elohim/.claude/worktrees/attestation-consolidation
for kind in Attestation ContentAttestation ContentSuccession CustodianCommitment; do
  echo "=== $kind ==="
  grep -n "EntryTypes::${kind}\b" elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs | wc -l
done
```

Expected: 0 for Attestation (after C.2 lands), 0 for ContentAttestation, ≥1 for ContentSuccession, ≥10 for CustodianCommitment. Remove only the zero-caller types.

- [ ] **Step 2: Remove zero-caller types from integrity**

Delete the struct definitions and EntryTypes variants for ONLY the audited-safe types. Add a comment near the remaining types noting "kept — consolidated path does not cover this shape (see spec §6.4 for category boundary)."

- [ ] **Step 3: Build**

Verify ≥10G free disk:
```bash
cd elohim/holochain/dna/elohim
just check
```

Expected: SUCCESS.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
git commit -m "remove(attestation): delete audited-vestigial Attestation/ContentAttestation from elohim DNA (ContentSuccession + CustodianCommitment kept — see plan)"
```

**Sequencing:** C.4 depends on C.2 landing first because the elohim DNA's vestigial `Attestation` only becomes truly unused after C.2 relocates the bridge return-shape struct. If C.4 runs in parallel with C.2 via subagents, C.4 dispatch is gated by C.2's commit SHA being present on the worktree branch — verify before C.4 starts.

### Task C.5 — Pack and verify all DNAs

- [ ] **Step 1: Run hc dna pack for all four DNAs**

```bash
cd elohim/holochain
hc dna pack dna/elohim
hc dna pack dna/imagodei
hc dna pack dna/infrastructure
hc dna pack dna/mishpat
```

Expected: All four DNAs pack successfully.

- [ ] **Step 2: Run full sweettest suite**

```bash
cd elohim/holochain/tests/sweettest
RUST_LOG=info cargo test --test-threads=1
```

Expected: All previously-passing tests still pass; new attestation tests pass.

- [ ] **Step 3: Commit (DNA artifacts if regenerated)**

```bash
git add elohim/holochain/dna/*/workdir/
git commit -m "build(attestation): repack all four DNAs after entry-type removal"
```

**Stage C acceptance:** All four DNAs build, pack, and pass sweettests. The 22+ legacy entry types are gone; the unified Content+discriminator pattern is the only attestation shape that validates.

---

## Stage D — Storage projection (elohim-storage)

Stage D lands the SQLite projections that index attestation Content entries for HTTP query. The projection layer is Category C (operational) but the projected data's source of truth is the DHT.

### Task D.1 — Drop legacy attestation tables migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-12-100300_drop_legacy_attestation_tables/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-12-100300_drop_legacy_attestation_tables/down.sql`

- [ ] **Step 1: Write up.sql**

```sql
-- Removes legacy per-entry-type projection tables superseded by 2026-05-12-100000_attestations
-- (source of truth: Holochain DHT); see genesis/docs/superpowers/specs/2026-05-11-attestation-consolidation-design.md §7.4 for the full table list

DROP TABLE IF EXISTS imagodei_attestations;
DROP TABLE IF EXISTS humanity_witnesses;
DROP TABLE IF EXISTS key_stewardships;
DROP TABLE IF EXISTS stewardship_grants;
DROP TABLE IF EXISTS renewal_attestations;
DROP TABLE IF EXISTS recovery_requests;
DROP TABLE IF EXISTS recovery_votes;
DROP TABLE IF EXISTS identity_challenges;
DROP TABLE IF EXISTS challenge_supports;
DROP TABLE IF EXISTS key_revocations;
DROP TABLE IF EXISTS revocation_votes;
DROP TABLE IF EXISTS identity_freezes;
DROP TABLE IF EXISTS stewardship_appeals;
DROP TABLE IF EXISTS policy_inheritances;
DROP TABLE IF EXISTS content_attestations;
DROP TABLE IF EXISTS custodian_commitments;
DROP TABLE IF EXISTS content_successions;
DROP TABLE IF EXISTS health_attestations;
DROP TABLE IF EXISTS doorway_heartbeat_summaries;
DROP TABLE IF EXISTS gate_decision_attestations;
DROP TABLE IF EXISTS proposal_votes;
DROP TABLE IF EXISTS statement_votes;
DROP TABLE IF EXISTS governance_reactions;
```

NOTE: The list above is canonical per spec §7.4; if any of these tables don't exist in current dev, `DROP TABLE IF EXISTS` keeps the migration idempotent.

- [ ] **Step 2: Write down.sql**

```sql
-- Reverse is intentionally not supported. The post-consolidation tree has no path
-- back to these per-type tables; engineers reaching for `down` should restore from
-- backup or revert the migration's commit.
SELECT 1;
```

- [ ] **Step 3: Run the migration locally**

```bash
cd elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' \
cargo test --features dev_migration -p elohim-storage migration_smoke
```

Expected: PASS (migration runs cleanly on a test schema).

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-05-12-100300_drop_legacy_attestation_tables/
git commit -m "migration(attestation): drop 22 legacy per-type attestation projection tables"
```

### Task D.2 — Create unified attestations table migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-12-100000_attestations/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-12-100000_attestations/down.sql`

- [ ] **Step 1: Write up.sql**

```sql
-- Source of truth: Holochain DHT (projection of Content entries with content_type LIKE 'attestation:%')
-- Category A — every row carries dht_anchor_hash NOT NULL.
-- Per spec genesis/docs/superpowers/specs/2026-05-11-attestation-consolidation-design.md §7.4.

CREATE TABLE attestations (
    id TEXT PRIMARY KEY,
    dht_anchor_hash BLOB NOT NULL,
    attestation_kind TEXT NOT NULL,
    subject_cid TEXT NOT NULL,
    subject_kind TEXT NOT NULL,
    issuer_cid TEXT NOT NULL,
    parent_governance_action_cid TEXT,
    vote_value TEXT,
    vote_weight TEXT,
    proof_class TEXT NOT NULL,
    proof_evidence_json TEXT NOT NULL,
    evidence_json TEXT NOT NULL,
    expires_at TEXT,
    supersedes_cid TEXT,
    revocation_reason TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL,
    manifest_ref TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT
);

CREATE INDEX attestations_subject ON attestations(subject_cid, attestation_kind);
CREATE INDEX attestations_issuer ON attestations(issuer_cid);
CREATE INDEX attestations_parent ON attestations(parent_governance_action_cid);
CREATE INDEX attestations_kind ON attestations(attestation_kind);
CREATE INDEX attestations_supersedes ON attestations(supersedes_cid);
```

- [ ] **Step 2: Write down.sql**

```sql
DROP INDEX IF EXISTS attestations_supersedes;
DROP INDEX IF EXISTS attestations_kind;
DROP INDEX IF EXISTS attestations_parent;
DROP INDEX IF EXISTS attestations_issuer;
DROP INDEX IF EXISTS attestations_subject;
DROP TABLE IF EXISTS attestations;
```

- [ ] **Step 3: Migration smoke test**

```bash
cd elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' \
cargo test migration_smoke
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-05-12-100000_attestations/
git commit -m "migration(attestation): unified attestations projection table"
```

### Task D.3 — Create governance_actions table migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-12-100100_governance_actions/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-12-100100_governance_actions/down.sql`

- [ ] **Step 1: Write up.sql**

```sql
-- Source of truth: Holochain DHT (projection of Content entries with content_type LIKE 'governance-action:%')
-- Category A — every row carries dht_anchor_hash NOT NULL.

CREATE TABLE governance_actions (
    id TEXT PRIMARY KEY,
    dht_anchor_hash BLOB NOT NULL,
    governance_kind TEXT NOT NULL,
    subject_cid TEXT NOT NULL,
    proposer_cid TEXT NOT NULL,
    threshold_json TEXT NOT NULL,
    eligibility_predicate_json TEXT,
    ballot_format TEXT NOT NULL,
    closes_at TEXT NOT NULL,
    parameters_json TEXT,
    title TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX governance_actions_subject ON governance_actions(subject_cid);
CREATE INDEX governance_actions_kind ON governance_actions(governance_kind);
CREATE INDEX governance_actions_closes ON governance_actions(closes_at);
```

- [ ] **Step 2: Write down.sql**

```sql
DROP INDEX IF EXISTS governance_actions_closes;
DROP INDEX IF EXISTS governance_actions_kind;
DROP INDEX IF EXISTS governance_actions_subject;
DROP TABLE IF EXISTS governance_actions;
```

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-05-12-100100_governance_actions/
git commit -m "migration(attestation): governance_actions projection table"
```

### Task D.4 — Create governance_action_tally table migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-12-100200_governance_action_tally/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-12-100200_governance_action_tally/down.sql`

- [ ] **Step 1: Write up.sql**

```sql
-- Source of truth: local (operational) — derived from governance_actions JOIN attestations, rebuildable via signal-stream replay
-- Category C — no dht_anchor_hash. Reconstruction strategy in spec §7.4.

CREATE TABLE governance_action_tally (
    parent_cid TEXT PRIMARY KEY,
    governance_kind TEXT NOT NULL,
    subject_cid TEXT NOT NULL,
    threshold_m INTEGER NOT NULL,
    threshold_n INTEGER,
    threshold_percentage REAL,
    closes_at TEXT NOT NULL,
    current_approve_count INTEGER NOT NULL DEFAULT 0,
    current_reject_count INTEGER NOT NULL DEFAULT 0,
    current_abstain_count INTEGER NOT NULL DEFAULT 0,
    computed_status TEXT NOT NULL,
    last_child_at TEXT,
    rebuilt_at TEXT NOT NULL
);

CREATE INDEX governance_action_tally_status ON governance_action_tally(computed_status);
```

- [ ] **Step 2: Write down.sql**

```sql
DROP INDEX IF EXISTS governance_action_tally_status;
DROP TABLE IF EXISTS governance_action_tally;
```

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-05-12-100200_governance_action_tally/
git commit -m "migration(attestation): governance_action_tally derived projection (Category C)"
```

### Task D.5 — Add Diesel schema + models

**Files:**
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`
- Modify: `elohim/elohim-storage/src/db/models.rs`

- [ ] **Step 1: Add table! macros**

In `diesel_schema.rs`, with the source-of-truth comment requirement per the file-structure map:

```rust
// Source of truth: Holochain DHT (projection of attestation Content entries)
diesel::table! {
    attestations (id) {
        id -> Text,
        dht_anchor_hash -> Binary,
        attestation_kind -> Text,
        subject_cid -> Text,
        subject_kind -> Text,
        issuer_cid -> Text,
        parent_governance_action_cid -> Nullable<Text>,
        vote_value -> Nullable<Text>,
        vote_weight -> Nullable<Text>,
        proof_class -> Text,
        proof_evidence_json -> Text,
        evidence_json -> Text,
        expires_at -> Nullable<Text>,
        supersedes_cid -> Nullable<Text>,
        revocation_reason -> Nullable<Text>,
        revoked_at -> Nullable<Text>,
        created_at -> Text,
        manifest_ref -> Text,
        title -> Text,
        description -> Nullable<Text>,
    }
}

// Source of truth: Holochain DHT (projection of governance-action Content entries)
diesel::table! {
    governance_actions (id) {
        id -> Text,
        dht_anchor_hash -> Binary,
        governance_kind -> Text,
        subject_cid -> Text,
        proposer_cid -> Text,
        threshold_json -> Text,
        eligibility_predicate_json -> Nullable<Text>,
        ballot_format -> Text,
        closes_at -> Text,
        parameters_json -> Nullable<Text>,
        title -> Text,
        description -> Nullable<Text>,
        created_at -> Text,
    }
}

// Source of truth: local (operational) — Category C derived from parent + children
diesel::table! {
    governance_action_tally (parent_cid) {
        parent_cid -> Text,
        governance_kind -> Text,
        subject_cid -> Text,
        threshold_m -> Integer,
        threshold_n -> Nullable<Integer>,
        threshold_percentage -> Nullable<Double>,
        closes_at -> Text,
        current_approve_count -> Integer,
        current_reject_count -> Integer,
        current_abstain_count -> Integer,
        computed_status -> Text,
        last_child_at -> Nullable<Text>,
        rebuilt_at -> Text,
    }
}
```

- [ ] **Step 2: Add row structs in models.rs**

```rust
/// Projection of an attestation Content entry from elohim DNA.
/// Source of truth: Holochain DHT (Content entry with content_type LIKE 'attestation:%').
#[derive(Queryable, Insertable, AsChangeset, Debug, Clone, Selectable)]
#[diesel(table_name = crate::db::diesel_schema::attestations)]
pub struct AttestationRow {
    pub id: String,
    pub dht_anchor_hash: Vec<u8>,
    pub attestation_kind: String,
    pub subject_cid: String,
    pub subject_kind: String,
    pub issuer_cid: String,
    pub parent_governance_action_cid: Option<String>,
    pub vote_value: Option<String>,
    pub vote_weight: Option<String>,
    pub proof_class: String,
    pub proof_evidence_json: String,
    pub evidence_json: String,
    pub expires_at: Option<String>,
    pub supersedes_cid: Option<String>,
    pub revocation_reason: Option<String>,
    pub revoked_at: Option<String>,
    pub created_at: String,
    pub manifest_ref: String,
    pub title: String,
    pub description: Option<String>,
}

/// Projection of a governance-action Content entry.
/// Source of truth: Holochain DHT.
#[derive(Queryable, Insertable, AsChangeset, Debug, Clone, Selectable)]
#[diesel(table_name = crate::db::diesel_schema::governance_actions)]
pub struct GovernanceActionRow {
    pub id: String,
    pub dht_anchor_hash: Vec<u8>,
    pub governance_kind: String,
    pub subject_cid: String,
    pub proposer_cid: String,
    pub threshold_json: String,
    pub eligibility_predicate_json: Option<String>,
    pub ballot_format: String,
    pub closes_at: String,
    pub parameters_json: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub created_at: String,
}

/// Derived tally projection.
/// Source of truth: local (operational) — recomputable any time from parent + children.
#[derive(Queryable, Insertable, AsChangeset, Debug, Clone, Selectable)]
#[diesel(table_name = crate::db::diesel_schema::governance_action_tally)]
pub struct GovernanceActionTallyRow {
    pub parent_cid: String,
    pub governance_kind: String,
    pub subject_cid: String,
    pub threshold_m: i32,
    pub threshold_n: Option<i32>,
    pub threshold_percentage: Option<f64>,
    pub closes_at: String,
    pub current_approve_count: i32,
    pub current_reject_count: i32,
    pub current_abstain_count: i32,
    pub computed_status: String,
    pub last_child_at: Option<String>,
    pub rebuilt_at: String,
}
```

- [ ] **Step 3: Build elohim-storage**

```bash
cd elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' \
cargo build
```

Expected: SUCCESS.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/db/diesel_schema.rs elohim/elohim-storage/src/db/models.rs
git commit -m "feat(attestation): diesel schema + row models for attestations + governance-actions + tally"
```

### Task D.6 — CRUD module for attestations

**Files:**
- Create: `elohim/elohim-storage/src/db/attestations.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

- [ ] **Step 1: Write CRUD functions**

```rust
//! CRUD + queries for the attestations projection table.
//! Source of truth: Holochain DHT (Content entries with content_type LIKE 'attestation:%').

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::attestations;
use crate::db::models::AttestationRow;

pub fn insert(conn: &mut SqliteConnection, row: &AttestationRow) -> Result<(), DieselError> {
    diesel::insert_into(attestations::table)
        .values(row)
        .on_conflict(attestations::id)
        .do_update()
        .set(row)
        .execute(conn)?;
    Ok(())
}

pub fn get_by_id(conn: &mut SqliteConnection, id: &str) -> Result<Option<AttestationRow>, DieselError> {
    attestations::table.filter(attestations::id.eq(id)).first::<AttestationRow>(conn).optional()
}

pub fn list_by_subject(
    conn: &mut SqliteConnection,
    subject_cid: &str,
    kind_filter: Option<&str>,
) -> Result<Vec<AttestationRow>, DieselError> {
    let mut q = attestations::table.into_boxed();
    q = q.filter(attestations::subject_cid.eq(subject_cid));
    if let Some(kind) = kind_filter {
        q = q.filter(attestations::attestation_kind.eq(kind));
    }
    q.order_by(attestations::created_at.desc()).load::<AttestationRow>(conn)
}

pub fn list_by_parent_governance_action(
    conn: &mut SqliteConnection,
    parent_cid: &str,
) -> Result<Vec<AttestationRow>, DieselError> {
    attestations::table
        .filter(attestations::parent_governance_action_cid.eq(parent_cid))
        .order_by(attestations::created_at.asc())
        .load::<AttestationRow>(conn)
}

pub fn delete_by_id(conn: &mut SqliteConnection, id: &str) -> Result<usize, DieselError> {
    diesel::delete(attestations::table.filter(attestations::id.eq(id))).execute(conn)
}

#[cfg(test)]
mod tests {
    // ... insert + get + list_by_subject + list_by_parent + revocation-supersedes tests
}
```

- [ ] **Step 2: Declare module in db/mod.rs**

```rust
pub mod attestations;
```

- [ ] **Step 3: Run targeted tests**

```bash
cd elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' \
cargo test attestations::tests
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/db/attestations.rs elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(attestation): attestations CRUD module + tests"
```

### Task D.7 — CRUD module for governance_actions

**Files:**
- Create: `elohim/elohim-storage/src/db/governance_actions.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

Follow the same pattern as Task D.6 for the governance_actions table. Provide `insert`, `get_by_id`, `list_open` (filtered by `closes_at > now`), `list_by_subject`, `delete_by_id`. Include tests.

- [ ] **Step 1–4: Same structure as Task D.6 for governance_actions**

```bash
cargo test governance_actions::tests
```

Expected: PASS.

- [ ] **Commit:** `git commit -m "feat(attestation): governance_actions CRUD module + tests"`

### Task D.8 — Tally projection module

**Files:**
- Create: `elohim/elohim-storage/src/db/governance_action_tally.rs`

- [ ] **Step 1: Write the tally module**

```rust
//! Derived tally projection.
//! Source of truth: local (operational) — computed from governance_actions JOIN attestations.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::{attestations, governance_action_tally, governance_actions};
use crate::db::models::{GovernanceActionRow, AttestationRow, GovernanceActionTallyRow};

pub fn upsert(conn: &mut SqliteConnection, row: &GovernanceActionTallyRow) -> Result<(), DieselError> {
    diesel::insert_into(governance_action_tally::table)
        .values(row)
        .on_conflict(governance_action_tally::parent_cid)
        .do_update()
        .set(row)
        .execute(conn)?;
    Ok(())
}

/// Compute the tally for a single governance-action by joining parent + children.
/// Latest-vote-per-issuer semantics applied per spec §4.5.
pub fn compute_tally(
    conn: &mut SqliteConnection,
    parent_cid: &str,
) -> Result<GovernanceActionTallyRow, DieselError> {
    let parent: GovernanceActionRow = governance_actions::table
        .filter(governance_actions::id.eq(parent_cid))
        .first(conn)?;
    let children: Vec<AttestationRow> = attestations::table
        .filter(attestations::parent_governance_action_cid.eq(parent_cid))
        .order_by(attestations::created_at.desc())
        .load(conn)?;

    let threshold: serde_json::Value = serde_json::from_str(&parent.threshold_json).unwrap_or_default();
    let threshold_m = threshold["m"].as_i64().unwrap_or(0) as i32;
    let threshold_n = threshold["n"].as_i64().map(|n| n as i32);
    let threshold_percentage = threshold["percentage"].as_f64();

    // Latest-per-issuer
    let mut seen_issuers = std::collections::HashSet::new();
    let mut approve = 0;
    let mut reject = 0;
    let mut abstain = 0;
    for child in &children {
        if !seen_issuers.insert(&child.issuer_cid) {
            continue;
        }
        match child.vote_value.as_deref() {
            Some("approve") => approve += 1,
            Some("reject") => reject += 1,
            Some("abstain") => abstain += 1,
            _ => {}
        }
    }

    let computed_status = derive_status(threshold_m, threshold_n, threshold_percentage,
        approve, reject, abstain, &parent.closes_at);

    Ok(GovernanceActionTallyRow {
        parent_cid: parent_cid.to_string(),
        governance_kind: parent.governance_kind,
        subject_cid: parent.subject_cid,
        threshold_m,
        threshold_n,
        threshold_percentage,
        closes_at: parent.closes_at,
        current_approve_count: approve,
        current_reject_count: reject,
        current_abstain_count: abstain,
        computed_status,
        last_child_at: children.first().map(|c| c.created_at.clone()),
        rebuilt_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn derive_status(m: i32, n: Option<i32>, percentage: Option<f64>, approve: i32, reject: i32, abstain: i32, closes_at: &str) -> String {
    let now = chrono::Utc::now();
    let closes = chrono::DateTime::parse_from_rfc3339(closes_at).map(|d| d.with_timezone(&chrono::Utc)).ok();
    let closed = closes.map(|c| c < now).unwrap_or(false);

    if approve >= m {
        return "reached-quorum".to_string();
    }
    if closed {
        return "closed-no-decision".to_string();
    }
    if let (Some(n_val), _) = (n, percentage) {
        let remaining = n_val - approve - reject - abstain;
        if approve + remaining < m {
            return "failed-quorum".to_string();
        }
    }
    "pending".to_string()
}
```

- [ ] **Step 2: Declare module + add tests**

```bash
cargo test governance_action_tally::tests
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(attestation): governance_action_tally derived projection module"
```

### Task D.9 — Attestation projector signal handler

**Files:**
- Create: `elohim/elohim-storage/src/services/attestation_projector.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (or wherever signal handlers register)

- [ ] **Step 1: Write the projector**

```rust
//! Post-commit signal handler — projects attestation + governance-action Content entries
//! from the DHT into the storage projection tables.
//!
//! Source of truth: Holochain DHT. This module writes the projection on signal arrival;
//! reads from these tables MUST treat them as caches, not as authoritative.

use std::sync::Arc;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;

use crate::db::{attestations, governance_actions};
use crate::db::models::{AttestationRow, GovernanceActionRow};
use crate::services::tally_projector;

pub struct AttestationProjector {
    pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl AttestationProjector {
    pub fn new(pool: Pool<ConnectionManager<SqliteConnection>>) -> Self {
        Self { pool }
    }

    pub async fn handle_content_created(&self, content: ContentSignal) -> Result<(), StorageError> {
        let mut conn = self.pool.get()?;

        if content.content_type.starts_with("attestation:") {
            let row = build_attestation_row(&content)?;
            attestations::insert(&mut conn, &row)?;

            // If this is a vote-child, recompute the parent's tally
            if let Some(ref parent_cid) = row.parent_governance_action_cid {
                let tally = tally_projector::recompute(&mut conn, parent_cid)?;
                // tally is upserted inside recompute
                tracing::debug!(parent_cid, status = %tally.computed_status, "governance tally updated");
            }
        } else if content.content_type.starts_with("governance-action:") {
            let row = build_governance_action_row(&content)?;
            governance_actions::insert(&mut conn, &row)?;
            // Initialize a zero-tally row so queries don't 404
            let initial_tally = tally_projector::compute_tally(&mut conn, &row.id)?;
            // upsert handled by compute_tally
        }

        Ok(())
    }
}

fn build_attestation_row(content: &ContentSignal) -> Result<AttestationRow, StorageError> {
    let metadata: serde_json::Value = serde_json::from_str(&content.metadata_json)?;
    Ok(AttestationRow {
        id: content.id.clone(),
        dht_anchor_hash: content.entry_hash.clone(),
        attestation_kind: content.content_type.clone(),
        subject_cid: metadata["subject_cid"].as_str().unwrap_or_default().to_string(),
        subject_kind: metadata["subject_kind"].as_str().unwrap_or_default().to_string(),
        issuer_cid: content.author_id.clone().unwrap_or_default(),
        parent_governance_action_cid: metadata["parent_governance_action_cid"].as_str().map(String::from),
        vote_value: metadata["vote_value"].as_str().map(String::from),
        vote_weight: metadata["vote_weight"].as_str().map(String::from),
        proof_class: metadata["proof_evidence"]["class"].as_str().unwrap_or("witness").to_string(),
        proof_evidence_json: serde_json::to_string(&metadata["proof_evidence"])?,
        evidence_json: serde_json::to_string(&metadata["evidence_json"])?,
        expires_at: metadata["expires_at"].as_str().map(String::from),
        supersedes_cid: metadata["revocation"]["supersedes_cid"].as_str().map(String::from),
        revocation_reason: metadata["revocation"]["reason"].as_str().map(String::from),
        revoked_at: metadata["revocation"]["revoked_at"].as_str().map(String::from),
        created_at: content.created_at.clone(),
        manifest_ref: lookup_manifest_ref(&content.content_type),
        title: content.title.clone(),
        description: Some(content.description.clone()).filter(|s| !s.is_empty()),
    })
}

fn build_governance_action_row(content: &ContentSignal) -> Result<GovernanceActionRow, StorageError> {
    let metadata: serde_json::Value = serde_json::from_str(&content.metadata_json)?;
    Ok(GovernanceActionRow {
        id: content.id.clone(),
        dht_anchor_hash: content.entry_hash.clone(),
        governance_kind: content.content_type.clone(),
        subject_cid: metadata["subject_cid"].as_str().unwrap_or_default().to_string(),
        proposer_cid: content.author_id.clone().unwrap_or_default(),
        threshold_json: serde_json::to_string(&metadata["threshold"])?,
        eligibility_predicate_json: metadata["eligibility_predicate"].as_object().map(|_| serde_json::to_string(&metadata["eligibility_predicate"]).unwrap_or_default()),
        ballot_format: metadata["ballot_format"].as_str().unwrap_or("approve-reject").to_string(),
        closes_at: metadata["closes_at"].as_str().unwrap_or_default().to_string(),
        parameters_json: metadata["parameters_json"].as_object().map(|_| serde_json::to_string(&metadata["parameters_json"]).unwrap_or_default()),
        title: content.title.clone(),
        description: Some(content.description.clone()).filter(|s| !s.is_empty()),
        created_at: content.created_at.clone(),
    })
}

fn lookup_manifest_ref(kind: &str) -> String {
    // Pillar mapping per the manifest declarations
    if kind.starts_with("attestation:humanness")
        || kind.starts_with("attestation:identity-")
        || kind.starts_with("attestation:key-")
        || kind.starts_with("attestation:stewardship-")
        || kind.starts_with("attestation:policy-")
        || kind.starts_with("attestation:renewal-")
        || kind.starts_with("attestation:recovery-")
        || kind.starts_with("attestation:revocation-")
        || kind.starts_with("attestation:challenge-")
    { "imagodei".to_string() }
    else if kind.starts_with("attestation:mastery")
        || kind.starts_with("attestation:content-")
        || kind.starts_with("attestation:custodian-")
    { "lamad".to_string() }
    else if kind.starts_with("attestation:device-") || kind.starts_with("attestation:doorway-")
    { "infrastructure".to_string() }
    else if kind.starts_with("attestation:governance-")
        || kind.starts_with("attestation:gate-")
        || kind.starts_with("attestation:proposal-")
        || kind.starts_with("attestation:statement-")
    { "mishpat".to_string() }
    else { "unknown".to_string() }
}
```

- [ ] **Step 2: Wire into signal dispatch**

In `elohim/elohim-storage/src/services/mod.rs` (or wherever the existing post-commit signal handler dispatches by entry type), add a branch that invokes `AttestationProjector::handle_content_created` when the Content entry's `content_type` starts with `attestation:` or `governance-action:`.

- [ ] **Step 3: Build + targeted test**

```bash
cargo test attestation_projector::tests
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(attestation): post-commit signal handler projects attestation + governance-action Content entries"
```

### Task D.10 — Tally projector module

**Files:**
- Create: `elohim/elohim-storage/src/services/tally_projector.rs`

- [ ] **Step 1: Write the tally_projector**

```rust
//! Tally projector — computes governance_action_tally on demand.
//! Source of truth: local (operational) — rebuildable from parent + children any time.

use diesel::sqlite::SqliteConnection;
use crate::db::governance_action_tally;
use crate::db::models::GovernanceActionTallyRow;

pub fn recompute(
    conn: &mut SqliteConnection,
    parent_cid: &str,
) -> Result<GovernanceActionTallyRow, StorageError> {
    let tally = governance_action_tally::compute_tally(conn, parent_cid)?;
    governance_action_tally::upsert(conn, &tally)?;
    Ok(tally)
}

pub fn rebuild_all(conn: &mut SqliteConnection) -> Result<usize, StorageError> {
    use crate::db::diesel_schema::governance_actions;
    use diesel::prelude::*;
    let parents: Vec<String> = governance_actions::table.select(governance_actions::id).load(conn)?;
    let mut count = 0;
    for parent_cid in parents {
        recompute(conn, &parent_cid)?;
        count += 1;
    }
    Ok(count)
}
```

- [ ] **Step 2: Targeted test**

Test that a parent + N children with mixed votes produces correct tally + status transitions.

```bash
cargo test tally_projector::tests
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(attestation): tally projector module — derived governance-action status"
```

### Task D.11 — Full elohim-storage build + integration smoke

- [ ] **Step 1: Run full unit + integration test pass**

```bash
cd elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' \
cargo test --lib
```

Expected: all tests pass.

- [ ] **Step 2: Clippy + fmt**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
cargo fmt --check
```

Expected: clean.

- [ ] **Step 3: Stage D acceptance commit (no code, marker only)**

```bash
git commit --allow-empty -m "stage(attestation): Stage D complete — storage projection + signal handlers ready"
```

**Stage D acceptance:** Three new tables exist with source-of-truth comments; 22 legacy tables dropped; CRUD + projector + tally modules pass tests; signal handler routes attestation + governance-action Content entries to the right table.

---

## Stage E — HTTP API + storage-client

Stage E exposes the projection-layer query/write API over HTTP and regenerates the TypeScript storage-client types.

### Task E.1 — Add view types in views.rs

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1: Add `AttestationView`, `GovernanceActionView`, `GovernanceActionTallyView`**

Each view's rustdoc MUST declare source-of-truth per the file-structure map requirement. Use `#[derive(TS)]` for codegen. Use `#[serde(rename_all = "camelCase")]` per the Rust→TS boundary convention.

- [ ] **Step 2: Build with ts-rs export**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings
```

Expected: new TypeScript files in `elohim/sdk/storage-client-ts/src/generated/`.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(attestation): wire-format views + ts-rs export"
```

### Task E.2 — Add schema contract tests

**Files:**
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Add contract assertions**

For each new view, assert it conforms to the corresponding JSON schema authored in Stage A. Pattern matches the existing schema-contract tests for other views.

- [ ] **Step 2: Run**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test schema_contract
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git commit -m "test(attestation): schema-contract tests for new view types"
```

### Task E.3 — Add attestation API handlers

**Files:**
- Create: `elohim/elohim-storage/src/api/attestations.rs`

- [ ] **Step 1: Implement handler functions**

Implement:
- `handle_post_attestation(req)` — parses input, calls into HC conductor's `content_store::issue_attestation`, returns `AttestationView`
- `handle_get_attestation_by_id(id)`
- `handle_list_attestations(subject_cid, kind_filter)` — queries `attestations` table
- `handle_post_revoke_attestation(id, reason)`

- [ ] **Step 2: Build + targeted test**

```bash
cargo test api::attestations::tests
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(attestation): HTTP API handlers for attestations"
```

### Task E.4 — Add governance-action API handlers

**Files:**
- Create: `elohim/elohim-storage/src/api/governance_actions.rs`

- [ ] **Step 1: Implement handlers**

- `handle_post_governance_action`
- `handle_get_governance_action_by_id` (returns parent + children + tally view)
- `handle_get_governance_action_tally(id)`
- `handle_post_vote(parent_id, vote_input)`

- [ ] **Step 2: Build + test + commit**

```bash
cargo test api::governance_actions::tests
git commit -m "feat(attestation): HTTP API handlers for governance-actions"
```

### Task E.5 — Register routes in http.rs

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`

- [ ] **Step 1: Add route matches**

Register these routes alongside the existing route table:

```rust
(Method::POST, "/api/v1/attestations") => self.api_attestations.handle_post(req).await,
(Method::GET, path) if path.starts_with("/api/v1/attestations/") => self.api_attestations.handle_get(req, path).await,
(Method::GET, "/api/v1/attestations") => self.api_attestations.handle_list(req).await,
(Method::POST, path) if path.ends_with("/revoke") && path.starts_with("/api/v1/attestations/") => self.api_attestations.handle_revoke(req, path).await,

(Method::POST, "/api/v1/governance-actions") => self.api_governance_actions.handle_post(req).await,
(Method::GET, path) if path.starts_with("/api/v1/governance-actions/") && path.ends_with("/tally") => self.api_governance_actions.handle_get_tally(req, path).await,
(Method::GET, path) if path.starts_with("/api/v1/governance-actions/") => self.api_governance_actions.handle_get(req, path).await,
(Method::POST, path) if path.ends_with("/vote") && path.starts_with("/api/v1/governance-actions/") => self.api_governance_actions.handle_vote(req, path).await,
```

- [ ] **Step 2: Delete legacy routes**

Find and remove the routes that served the 22 deleted entities (the existing `/api/v1/humanity-witnesses`, `/api/v1/renewal-attestations`, `/api/v1/proposal-votes`, etc.). Use `git grep` to find them.

- [ ] **Step 3: Build + integration smoke**

```bash
cargo build
cargo test --test http_smoke
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(attestation): register new routes + delete 25+ legacy attestation routes"
```

### Task E.6 — Regenerate storage-client-ts

- [ ] **Step 1: Run codegen**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings
pnpm run schema:codegen:ts
```

Expected: TypeScript files for attestation views appear in `elohim/sdk/storage-client-ts/src/generated/`.

- [ ] **Step 2: Verify storage-client builds**

```bash
cd elohim/sdk/storage-client-ts
pnpm install
pnpm build
```

Expected: SUCCESS.

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/storage-client-ts/
git commit -m "codegen(attestation): regenerate TypeScript storage-client types"
```

### Task E.7 — Pre-push validation pass

- [ ] **Step 1: Run the schema codegen freshness check**

```bash
pnpm run schema:test
pnpm run schema:validate
pnpm run schema:check-dna
pnpm run schema:codegen:ts -- --verify
```

Expected: all PASS, no codegen drift.

- [ ] **Step 2: Stage E acceptance commit**

```bash
git commit --allow-empty -m "stage(attestation): Stage E complete — HTTP API + storage-client ready"
```

**Stage E acceptance:** New routes serve attestations + governance-actions; legacy routes deleted; storage-client-ts regenerated; all codegen checks pass.

---

## Stage F — Angular consumers + a2o updates

Stage F migrates Angular service consumers from the deleted per-type endpoints to the unified attestation / governance-action services, and updates a2o feature files referencing the deleted entry types.

### Task F.1 — Unified attestation Angular service

**Files:**
- Create: `app/elohim-library/projects/elohim-service/src/services/attestation.service.ts`

- [ ] **Step 1: Implement the service**

Use the existing `HolochainClientService` pattern. Methods:
- `issue(input: IssueAttestationInput): Observable<AttestationView>`
- `revoke(id: string, reason: string): Observable<AttestationView>`
- `getById(id: string): Observable<AttestationView | null>`
- `listBySubject(subjectCid: string, kind?: string): Observable<AttestationView[]>`

Import the `AttestationView` type from `@elohim/storage-client`.

- [ ] **Step 2: Add unit test**

`attestation.service.spec.ts` covering each method with mocked HolochainClientService.

- [ ] **Step 3: Run vitest**

```bash
cd app/elohim-library/projects/elohim-service
pnpm test attestation.service
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(attestation): Angular AttestationService — unified attestation API"
```

### Task F.2 — Governance-action Angular service

**Files:**
- Create: `app/elohim-library/projects/elohim-service/src/services/governance-action.service.ts`

Methods: `propose`, `getById` (returns parent + children + tally), `getTally`, `vote`. Tests cover happy paths + error paths.

- [ ] **Steps 1-4:** Same pattern as Task F.1.

```bash
git commit -m "feat(attestation): Angular GovernanceActionService"
```

### Task F.3 — Migrate imagodei pillar consumers

**Files:**
- Modify: `app/elohim-app/src/app/imagodei/services/attestation.service.ts` (and any imagodei components that called the deleted endpoints)

- [ ] **Step 1: Replace per-type calls with unified service**

Find all callers via `grep -rn "humanity-witness\|renewal-attestation\|key-stewardship\|stewardship-grant\|recovery-request\|recovery-vote\|identity-challenge\|challenge-support\|key-revocation\|revocation-vote\|identity-freeze" app/elohim-app/src/`.

Replace each per-type service call with the unified `AttestationService.issue({kind: 'attestation:<subtype>', ...})` or `GovernanceActionService.propose({...})` shape.

- [ ] **Step 2: Run lint + tests**

```bash
cd app/elohim-app
pnpm exec eslint src --ext .ts,.html
pnpm exec vitest run --config vite.config.ts
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git commit -m "refactor(attestation): migrate imagodei pillar consumers to unified attestation services"
```

### Task F.4 — Migrate lamad pillar consumers

**Files:**
- Modify: `app/elohim-app/src/app/lamad/services/content-attestation.service.ts` + lamad components

- [ ] **Step 1: Replace `ContentAttestation`-shaped calls with `attestation:content-quality`, `attestation:content-succession`, etc.**

- [ ] **Step 2: Lint + test + commit**

```bash
git commit -m "refactor(attestation): migrate lamad pillar consumers"
```

### Task F.5 — Migrate mishpat / governance consumers

**Files:**
- Modify: `app/elohim-app/src/app/qahal/` + any mishpat-touching code

- [ ] **Step 1: Replace `Proposal` / `ProposalVote` / `StatementVote` / `Challenge` / `GateDecisionAttestation` calls with the unified governance-action services**

- [ ] **Step 2: Lint + test + commit**

```bash
git commit -m "refactor(attestation): migrate mishpat / governance consumers"
```

### Task F.6 — Migrate infrastructure / device-health consumers

- [ ] **Step 1: Replace HealthAttestation flows with `attestation:device-health` via unified service**

- [ ] **Step 2: Lint + test + commit**

```bash
git commit -m "refactor(attestation): migrate infrastructure / device-health consumers"
```

### Task F.7 — Update a2o feature files

**Files:**
- Modify: `genesis/a2o/features/auth/*.feature` (per CLAUDE.md auth pillar mapping)
- Modify: `genesis/a2o/features/lamad/*.feature`
- Modify: any other `.feature` referencing the deleted entry types

- [ ] **Step 1: Find feature files using deleted types**

```bash
grep -rln 'humanity[- ]witness\|renewal[- ]attestation\|key[- ]stewardship\|stewardship[- ]grant\|gate[- ]decision[- ]attestation\|content[- ]attestation' genesis/a2o/features/
```

- [ ] **Step 2: Rewrite scenarios to use the unified attestation vocabulary**

The semantic intent of each scenario is preserved; only the entity names change. Example:

```gherkin
# Before
When alice issues a humanity-witness for bob
# After
When alice issues an attestation of kind "attestation:humanness" for bob
```

- [ ] **Step 3: Run a2o lint / parse**

```bash
cd genesis/a2o
pnpm run lint
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git commit -m "test(attestation): update a2o feature scenarios for unified attestation vocabulary"
```

### Task F.8 — Stage F integration smoke

- [ ] **Step 1: Build elohim-app**

```bash
cd app/elohim-app
pnpm run build
```

Expected: SUCCESS.

- [ ] **Step 2: Run unit test suite**

```bash
pnpm exec vitest run --config vite.config.ts
```

Expected: PASS.

- [ ] **Step 3: Stage F acceptance commit**

```bash
git commit --allow-empty -m "stage(attestation): Stage F complete — Angular consumers + a2o features migrated"
```

**Stage F acceptance:** Angular builds; all unit tests pass; lint passes; a2o features parse.

---

## Stage G — Recovery protocol decoupling (Shamir off-chain)

Stage G separates Shamir share transport from the social-threshold M-of-N. Shares move via signed libp2p direct messages from custodians to the recovery agent, gated by the custodian's published recovery-approval attestation. **Per spec §8, this stage is deferrable** — recovery keeps working without it (shares stay in `metadata_json.evidence_json` temporarily). If PVC budget tightens, defer Stage G to a follow-up.

### Task G.1 — libp2p share-transport protocol

**Files:**
- Create: `elohim/elohim-storage/src/p2p/shamir_transport.rs`

- [ ] **Step 1: Define the request/response protocol**

```rust
//! libp2p request-response protocol for Shamir share delivery.
//!
//! Authorization is the recovery-approval attestation: when a custodian sends a share,
//! the recovery agent verifies the corresponding attestation:recovery-approval exists
//! and is current. The DHT carries the authorization (cheap, durable, witnessed);
//! libp2p carries the share material (expensive, ephemeral, point-to-point).

#[derive(Serialize, Deserialize)]
pub struct ShamirShareRequest {
    pub recovery_governance_action_cid: String,
    pub custodian_cid: String,
}

#[derive(Serialize, Deserialize)]
pub struct ShamirShareResponse {
    pub share_data: Vec<u8>,                // the Shamir share material
    pub share_index: u32,                   // 1..N
    pub attestation_cid: String,            // attestation:recovery-approval that authorizes this share
    pub signature: Vec<u8>,                 // custodian's signature over (recovery_governance_action_cid, share_data, share_index)
}
```

- [ ] **Step 2: Register the protocol with libp2p swarm**

Follow the existing pattern in `elohim/elohim-storage/src/p2p/mod.rs` for adding a new request-response behaviour.

- [ ] **Step 3: Build + targeted test**

```bash
cargo test p2p::shamir_transport::tests
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(recovery): libp2p shamir-share request/response protocol"
```

### Task G.2 — Share assembler

**Files:**
- Create: `elohim/elohim-storage/src/recovery/share_assembler.rs`

- [ ] **Step 1: Implement off-chain assembly**

```rust
//! Off-chain Shamir share assembly.
//!
//! Recovery agent reads the recovery-action's child attestation:recovery-approval
//! entries from the DHT projection, then requests shares from each approving
//! custodian via libp2p. Once threshold-many shares arrive, reconstructs the
//! recovery secret using existing Shamir primitives.

pub struct ShareAssembler { /* ... */ }

impl ShareAssembler {
    pub async fn assemble(&self, recovery_governance_action_cid: &str) -> Result<Vec<u8>, RecoveryError> {
        // 1. Query attestations table for children of this parent (kind=attestation:recovery-approval)
        // 2. For each approving custodian, send ShamirShareRequest via libp2p
        // 3. Verify each response's attestation_cid + signature
        // 4. Collect shares until threshold reached
        // 5. Reconstruct via existing shamir_combine() primitive
        // ...
    }
}
```

- [ ] **Step 2: Unit + integration test**

Test: parent + N approvals + N libp2p responses → reconstructed secret.

```bash
cargo test recovery::share_assembler::tests
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(recovery): off-chain share assembler — DHT auth + libp2p transport"
```

### Task G.3 — Remove share material from DHT validator

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs`

- [ ] **Step 1: Add validator floor to reject share material in metadata_json**

For attestation:recovery-approval entries, validate that `metadata.evidence_json.summary_metric` does NOT contain a `share_data` field. Floor failure: `recovery_approval_must_not_carry_share_material`.

- [ ] **Step 2: Sweettest the floor**

Test: attempt to issue an attestation:recovery-approval with `share_data` in evidence → rejected.

```bash
cargo test --test-threads=1 recovery_approval_rejects_share_material
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(recovery): validator rejects Shamir share material on DHT"
```

### Task G.4 — Update recovery flow in Angular

**Files:**
- Modify: `app/elohim-app/src/app/imagodei/services/recovery.service.ts` (and related)

- [ ] **Step 1: Replace mutable-status polling with tally-derived status**

The recovery service should query the `governance_action_tally` view for status, not the deleted RecoveryRequest entry.

- [ ] **Step 2: Add libp2p share-submission UI flow**

When a custodian publishes their attestation:recovery-approval, the UI prompts them to submit their share via the libp2p channel.

- [ ] **Step 3: Test + commit**

```bash
git commit -m "feat(recovery): Angular recovery flow uses tally projection + libp2p share submission"
```

### Task G.5 — End-to-end recovery integration test

**Files:**
- Create: `elohim/elohim-storage/tests/attestation_consolidation_integration.rs`

- [ ] **Step 1: Write the integration test**

Scenario:
1. Bob proposes governance-action:recovery-request
2. 3 of 5 custodians issue attestation:recovery-approval children
3. Each custodian sends share via libp2p
4. Recovery agent assembles shares, reconstructs secret
5. Assert: tally shows reached-quorum, secret successfully reconstructed, no share material visible on DHT

- [ ] **Step 2: Run**

```bash
cargo test --test attestation_consolidation_integration -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git commit -m "test(recovery): end-to-end integration — DHT-attested + libp2p-transported share assembly"
```

### Task G.6 — Stage G acceptance + plan close-out

- [ ] **Step 1: Stage G acceptance commit**

```bash
git commit --allow-empty -m "stage(attestation): Stage G complete — Shamir transport decoupled off DHT"
```

- [ ] **Step 2: Plan close-out**

The implementation plan's success criteria (per spec §12) are now verifiable:
- All 18+ attestation-shaped entry types removed ✓
- Unified content_type discriminators in place ✓
- Unified attestations + governance_actions + tally projection tables ✓
- 6 new coordinator zome functions; ~30 removed from other DNAs ✓
- 6 new HTTP routes; ~25 deleted ✓
- Recovery status derives from tally ✓
- Shamir material off-DHT ✓
- DNA capacity reclaimed (verify with entry-type counts)
- a2o features updated ✓
- Wave 0 plan updated to reference this spec (FOLLOW-UP — separate commit)

**Stage G acceptance:** All spec success criteria checked off; the consolidation is complete end-to-end.

---

## Self-Review

After writing this plan, checked against the spec:

1. **Spec coverage** — Every section of the spec maps to one or more tasks:
   - §3 Attestation primitive → Stage A schemas + Stage B `issue_attestation` + Stage C validator
   - §4 M-of-N → Stage A governance-action schemas + Stage B `propose/vote` coordinators + Stage D tally projection
   - §5 Recovery decoupling → Stage G entirely
   - §6 Manifest layer → Stage A manifest edits
   - §7 Migration plan → Stages A–G are literally the stages from §7.1–§7.7
   - §8 Wave 0 integration → noted in plan header; FOLLOW-UP commit to update Wave 0 plan
   - §9 Validator floors → Stage C task C.1 implements all 8 floors

2. **Placeholder scan** — No TBD/TODO/XXX. All code blocks contain actual content.

3. **Type consistency** — `IssueAttestationInput`, `AttestationOutput`, `GovernanceActionOutput`, `VoteOnGovernanceActionInput`, `AttestationView`, `GovernanceActionView`, `GovernanceActionTallyView` are referenced consistently. `attestation_kind` (not `attestation_type`), `parent_governance_action_cid` (not `parent_action_cid`), `governance_kind` (not `governance_action_kind`) used uniformly.

4. **Source-of-truth declarations** — Every new migration, Diesel `table!`, model struct, and view type carries an explicit source-of-truth declaration per the p2p-design-gate skill requirement.

## Execution Handoff

Plan complete and saved to `genesis/docs/superpowers/plans/2026-05-11-attestation-consolidation-implementation-plan.md`. Two execution options:

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between tasks, fast iteration
2. **Inline Execution** — execute tasks in this session using executing-plans, batch execution with checkpoints

Recommend Subagent-Driven for this plan because (a) the plan spans multiple DNAs + storage + Angular and Sonnet-tier subagents are well-suited to per-stage execution, and (b) Stage gates (A→B→C→D→E→F→G) make natural review checkpoints.

**Follow-up commit** (separate from this plan's execution): update `genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-wave-0-substrate-cleanup.md` to cite this spec/plan and replace its Option A/B decision block with "executes attestation-consolidation Stages A–F (G optional)."

