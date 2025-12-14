# Lamad DNA Schema Versions

This document tracks schema changes across DNA versions for migration planning.

## Version: v1 (Current)

**DNA Name**: `lamad-spike`
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
