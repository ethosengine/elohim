# Lamad DNA Schema Versions

This document tracks schema changes across DNA versions for migration planning.

## Version: v1 (Current)

**DNA Name**: `lamad` (within `elohim` hApp)
**Status**: Active development
**Holochain Compatibility**: 0.6.x

### Entry Types

#### Knowledge Layer (Living Data)

| Entry Type | Key Fields | Notes |
|------------|-----------|-------|
| `Content` | id, content_type, title, description, content, content_format, tags, metadata_json | Core knowledge nodes |
| `LearningPath` | id, version, title, description, purpose, difficulty, path_type, tags | Learning journey definitions |
| `PathStep` | path_id, order_index, resource_type, resource_id, title, description, mastery_requirements | Steps within paths |
| `PathChapter` | path_id, title, description, order_index, step_ids | Grouping of steps |
| `Relationship` | source_id, target_id, relationship_type, strength, created_by, metadata_json | Knowledge graph edges |

#### User State (Personal Data)

| Entry Type | Key Fields | Notes |
|------------|-----------|-------|
| `Human` | id, display_name, bio, category, profile_reach, location, affinities | Human persona |
| `Agent` | agent_pubkey, display_name, created_at | Agent identity |
| `AgentProgress` | agent_id, path_id, current_step_index, completed_step_indices, status, attestations_earned | Path progress tracking |
| `ContentMastery` | human_id, content_id, level, level_index, last_reviewed, review_count, next_review, freshness | Spaced repetition state |
| `HumanProgress` | human_id, current_missions, mastery_snapshot, created_at, updated_at | High-level progress |
| `PracticePool` | human_id, pool_type, content_ids, rotation_index, total_reviews | Practice rotation |
| `MasteryChallenge` | challenge_id, human_id, content_id, challenge_type, question, status, started_at | Active assessments |

#### Economic Layer (Frozen Data)

| Entry Type | Key Fields | Notes |
|------------|-----------|-------|
| `EconomicEvent` | event_id, action, provider, receiver, resource_conforms_to, effort_quantity, note | ValueFlows events |
| `EconomicResource` | resource_id, conforms_to, accounting_quantity, stage, contained_in, lot, note | Resource states |
| `ContributorPresence` | presence_id, human_id, display_name, tier, recognized_for, lifecycle_stage, total_recognition | Contributor reputation |
| `Attestation` | attestation_id, human_id, attestation_type, content_id, path_id, level_achieved, evidence | Achievement records |
| `Appreciation` | appreciation_id, appreciated_by, appreciation_to, content_id, message, timestamp | Recognition of value |
| `Claim` | claim_id, created_by, claimant_presence_id, resource_classification, effort_quantity, status | Entitlement claims |
| `Settlement` | settlement_id, settles_claim_id, economic_event_id, status, note | Claim settlements |

#### Intent & Commitment (Planning)

| Entry Type | Key Fields | Notes |
|------------|-----------|-------|
| `Process` | process_id, name, based_on, planned_start, planned_end, note | Planned work |
| `Intent` | intent_id, action, provider, receiver, resource_conforms_to, input_of, output_of | Future events |
| `Commitment` | commitment_id, satisfies_intent, action, provider, receiver, due, note | Agreed future events |

#### Gamification

| Entry Type | Key Fields | Notes |
|------------|-----------|-------|
| `LearnerPointBalance` | learner_id, total_points, level, level_name, streak_days, last_activity | Point tracking |
| `PointEvent` | event_id, learner_id, event_type, points, source_type, source_id, timestamp | Point history |
| `ContributorRecognition` | contributor_presence_id, total_recognition, recognition_level, badges | Contributor rewards |
| `ContributorImpact` | contributor_presence_id, content_id, total_views, completions, average_rating, impact_score | Content impact |

#### Access Control

| Entry Type | Key Fields | Notes |
|------------|-----------|-------|
| `StewardCredential` | credential_id, human_id, steward_presence_id, scopes, status, issued_at, expires_at | Steward permissions |
| `PremiumGate` | gate_id, title, gate_type, content_ids, path_ids, price, steward_presence_id | Content gating |
| `AccessGrant` | grant_id, gate_id, learner_id, granted_by, payment_event_id, is_active | Access records |
| `StewardRevenue` | revenue_id, gate_id, steward_presence_id, payment_event_id, steward_amount | Revenue tracking |

### Link Types

Key link types for querying:

- `IdToContent` - Content lookup by string ID
- `TypeToContent` - Content grouped by type
- `TagToContent` - Content by tag
- `AuthorToContent` - Content by creator
- `IdToPath` - Path lookup by ID
- `PathToStep` - Steps belonging to path
- `AgentToPathProgress` - User progress tracking
- `HumanToMastery` - Mastery records per human

### Indexes / Anchors

Global anchors for discovery:

- `all_paths/index` - All learning paths
- `content_id/{id}` - Content by ID
- `path_id/{id}` - Path by ID
- `content_type/{type}` - Content by type
- `agent_progress/{agent_id}` - Progress per agent
- `human_mastery/{human_id}` - Mastery per human

### Migration Export Functions

Available in v1 coordinator for future migration:

```rust
export_schema_version() -> String        // Returns "v1"
export_all_content() -> Vec<ContentOutput>
export_all_paths_with_steps() -> Vec<PathWithStepsExport>
export_all_mastery() -> Vec<ContentMasteryOutput>
export_all_progress() -> Vec<AgentProgressOutput>
export_for_migration() -> MigrationExport  // All data in one call
```

---

## Version: v2 — re-genesis 1 address crossing (elohim / `content_store` DNA)

**Date**: 2026-11-05
**Kind**: Breaking integrity change — new DNA hash, re-genesis (no lineage migration; peers on v1 are a different DHT)
**Network seed**: `elohim_<dna>_alpha2`
**Address profile**: every byte address is a byte CID (CIDv1 raw `0x55` + BLAKE3-256 `0x1e`, `bafkr4i…`), admitted by full decode through `elohim_address::ByteCid::parse` (`crates/elohim-address`); no prefix/length predicate.

### Changed entry types

| Entry Type | v1 | v2 | Validator (integrity) |
|------------|----|----|-----------------------|
| `Content` | `blob_cid: Option<String>` (unchecked), `content_hash: Option<String>` (sha256 of body) | `blob_cid: Option<String>`; `content_hash` **deleted** (the CID is the body's integrity proof) | `blob_cid` parses as a byte CID when `Some` |
| `BlobEntry` | `hash: String` (sha256 hex) | `cid: String` | `cid` parses as a byte CID |
| `ShardManifest` | `blob_hash, total_size, mime_type, encoding, data_shards, total_shards, shard_size, shard_hashes, reach, author_id, created_at, verified_at` | `blob_cid, byte_len: u64, mime_type, chunker: ChunkerDecl{id, min: u32, avg: u32, max: u32, level: u8}, chunks: Vec<ChunkRef{cid, len: u32}>, erasure: ErasureDecl{id, data: u8, parity: u8}, stripes: Vec<Stripe{index: u32, byte_len: u32, shards: Vec<ShardRef{cid, len: u32, index: u8}>}>, reach, author_id, created_at, verified_at` | `blob_cid` + every chunk/shard `cid` parse as byte CIDs; `chunker.id ∈ {"fastcdc-2020"}`; `erasure.id ∈ {"none", "rs-gf8-0x11d-v1"}`; `stripes` empty iff `erasure.id == "none"`; each stripe `byte_len ≤ 16 MiB`; each stripe carries exactly `data + parity` shards |
| `ShardLocation` | `shard_hash: String` | `shard_cid: String` | `shard_cid` parses as a byte CID |
| `ImportBatch` | `blob_hash: String` | `blob_cid: String` | (not validated in v2) |
| `ContentSuccession` | `content_hash: Option<String>` (sha256 of body) | `content_cid: Option<String>` | `content_cid` parses as a byte CID when `Some` |

Removed constant: `SHARD_ENCODINGS` (`none`, `chunked`, `rs-4-7`, `rs-8-12`), replaced by `CHUNKER_IDS` / `ERASURE_IDS` / `MAX_STRIPE_BYTE_LEN`.

### Link types

- `BlobToReplicas` **deleted** (never created by any coordinator). Every later `LinkTypes` discriminant shifts down by one.
- `IdToBlob`, `BlobToManifest`, `ShardToLocations` keep their names; their anchors are now keyed by CID: `StringAnchor("blob_cid", <blob_cid>)` for `BlobToManifest` / `IdToBlob` lookups and `StringAnchor("shard_cid", <cid>)` for `ShardToLocations` (v1: `"blob_hash"` / `"shard_hash"`).

### Coordinator surface (no hash effect)

- `register_shard_manifest` input mirrors the v2 `ShardManifest` minus `created_at` / `verified_at`; shape checks moved to the integrity validator.
- `resolve_blob` / `get_storage_dids_for_blob` walk custody units: stripe shards when erasure coded, otherwise the chunks. `get_storage_dids_for_blob` now reads the anchor the manifest is written under (v1 read `"blob_manifest"`, which nothing wrote).
- `ProjectionSignal::ImportBatchQueued.blob_hash → blob_cid`; `BlobMetadataOutput.hash → cid`; `*_hash` → `*_cid` on the blob/shard inputs and outputs.
- The SipHash `calculate_sha256` helper is replaced by `byte_cid_of(&[u8]) -> String` (`elohim_address::compute_bytes_cid`).
- imagodei (coordinator only, no hash effect from this change): `ContentCheckInput.content_hash → content_cid`.

---

## Migration Notes

### v1 Design Decisions

1. **metadata_json fields** - Many entry types include `metadata_json: String` for extensibility. New fields should go here first before promoting to schema.

2. **String IDs** - Most entries use string IDs (e.g., `id: String`) rather than action hashes for human readability and cross-system compatibility.

3. **Timestamps as strings** - Timestamps stored as `String` formatted from `sys_time()` for simplicity.

4. **Mastery levels** - Eight-level scale: `not_started`, `seen`, `remember`, `understand`, `apply`, `analyze`, `evaluate`, `create`.

### Known Future Changes

When planning v2, consider:

1. **Potential schema changes**:
   - Adding `version` field to Content for content versioning
   - Structured `metadata` instead of JSON string
   - Adding `archived_at` for soft deletes

2. **Potential removals**:
   - Consider whether all economic types are needed, or if hREA should be used instead

3. **Index improvements**:
   - Global content index (currently only by type)
   - Pagination support for large result sets

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| v1 | 2024-12 | Initial schema with full Lamad + Shefa types |
| v2 | 2026-11-05 | Re-genesis 1 address crossing: byte CIDs (BLAKE3) validated by full decode; `ShardManifest` recast to chunks + erasure stripes; `Content.content_hash` and `BlobToReplicas` deleted |
