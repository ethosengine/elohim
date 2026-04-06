# Type Audit: Domain Wire Types vs Storage Views

**Date:** 2026-04-05
**Scope:** Compare field sets between `elohim/sdk/domains/*/types/` Rust wire types and `elohim/elohim-storage/src/views.rs` TypeScript-generating View types.

## Summary

| Domain | Status | Key Issues |
|--------|--------|-----------|
| imagodei | OK | Views add 4 projection fields (intentional) |
| lamad | CAUTION | `content` field renamed to `content_body`; metadata parsed from JSON string |
| shefa | CAUTION | Missing `updated_at` in Commitment view; JSON string→parsed Value transforms |
| qahal | CRITICAL | Views are structurally different from wire types — lose proposer details, priority, metadata |
| infrastructure | N/A | No views exist (doorway-only, not projected through storage) |
| avodah | N/A | No views exist yet |

## Critical: Qahal (Governance) Views

The qahal domain has the largest divergence. Storage views for Challenge, Proposal, and Precedent are **structurally different** from the wire types:

**Challenge loses:** `priority`, `metadata_json`, `challenger_name`, `sla_deadline`, `assigned_elohim`, `resolution_json`
**Challenge renames:** `grounds` → `grounds_primary` + `grounds_secondary`, `challenger_standing` → `standing_basis`

**Proposal loses:** `proposer_id`, `proposer_name`, `rationale`, `phase`, `amendments_json`, `voting_config_json`, `outcome_json`, `related_entity_type/id`, `metadata_json`

**Precedent loses:** `binding`, `scope_json`, `summary`, `citations`, `established_at`, `superseded_by`, `metadata_json`
**Precedent renames:** `full_reasoning` → `interpretation`

### Root cause hypothesis

The views were written BEFORE the wire types crates existed. The views represent an earlier design iteration where governance was projected with fewer fields. The wire types reflect the current DNA integrity/coordinator design. The views need updating to match.

### Recommended action

Sprint to update `views.rs` governance views to include all wire type fields. This is blocking for any governance feature that relies on storage projection.

## Intentional Differences (Not Bugs)

Views ADD these computed/projection fields that wire types correctly omit:
- `dht_anchor_hash` — DHT provenance link (storage-only)
- `h_app_id` — app identifier (storage-only)
- `agent_pub_key` — computed from zome call context
- `created_by` — computed from agent provenance
- `validation_status` — computed by storage
- `profile_photo_url` — storage-projected media

Views TRANSFORM these fields (intentional):
- `metadata_json: String` → `metadata: JsonValue` (parsed for query efficiency)
- `*_json: String` fields → parsed equivalents

These are correct — the storage layer adds queryability that the wire format doesn't need.
