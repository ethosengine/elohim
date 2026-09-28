//! Authored content edges travel INSIDE their source atom's signed entry
//! (F16, `lamad-teacher-authoring-backlog.md` "F16 design").
//!
//! An atom's authored edges are part of what its author said the atom IS, so
//! they ride the atom's own notarized `Content.metadata_json` as
//! `relationships: [{type, targetId, role?}]` — not a Holochain Link, not the
//! unused (always-public) DHT `Relationship` entry. They version, travel and
//! disappear with the head that states them.
//!
//! This module is the projection half: on every adoption / projection of a
//! conductor-verified source entry, the `relationships` table rows OF THAT
//! SOURCE are replaced by the list the entry states — insert new, update role,
//! delete edges the entry no longer lists. Rules:
//!
//! - **Only from a conductor-verified entry.** Callers obtain the list through
//!   [`edges_from_verified_metadata`] on the entry they verified; an
//!   unauthenticated sync-doc field never reaches it.
//! - **An entry with no `relationships` key states nothing** (`None` = leave the
//!   source's rows alone). A legacy entry predating F16 carries no key, and
//!   treating that as "no edges" would erase every pipeline-seeded graph on the
//!   first adoption (concern C14). An EMPTY array is a statement: all edges
//!   dropped.
//! - **Edge id** = `rel-` + first 32 hex of sha256(`source|type|target`) — the
//!   seeder's `relationshipId` (`genesis/seeder/src/content-input.ts`), the same
//!   on every peer. Role is metadata, not identity.
//! - **Edge reach** = the source atom's reach (an edge is never more open than
//!   the atom that authored it; an intimate atom's edges are intimate).
//! - **Types** are canonicalized onto the lamad manifest vocabulary
//!   (`elohim/sdk/domains/lamad/manifest/relationships.json`) with the seeder's
//!   alias table; anything else lands on `RELATES_TO`, counted, never dropped.
//! - **Cap** [`MAX_EDGES_PER_ATOM`] per atom, after a deterministic sort, so a
//!   truncation is the same on every peer (logged).
//! - Only `inference_source = 'explicit'` rows of the source are replaced;
//!   computed/path/system edges are not authored statements and are left alone.
//! - **Provenance marker.** Every row this projection writes carries
//!   [`AUTHORED_PROVENANCE_JSON`] in `provenance_chain_json`; no other writer
//!   sets it (the HTTP write routes refuse it and clear it on upsert). It is
//!   what lets the read gate serve an edge whose TARGET has no local row: only
//!   an edge a verified signed head stated vouches for that target's reach (see
//!   [`unknown_endpoint_reach`]); a POSTed edge naming an unknown id vouches for
//!   nothing.

use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;

use diesel::prelude::*;
use sha2::{Digest, Sha256};

use super::context::AppContext;
use super::diesel_schema::relationships;
use super::models::{current_timestamp, NewRelationship};
use crate::error::StorageError;

/// Per-atom ceiling on authored edges (concern C6a). Beyond it the list is
/// truncated deterministically (after the canonical sort) and a warning logged.
pub const MAX_EDGES_PER_ATOM: usize = 256;

/// The inference source every authored edge carries.
const AUTHORED: &str = "explicit";

/// The `provenance_chain_json` of a row projected from a conductor-verified
/// source head — and of no other row. Served as-is in the edge view's
/// `provenanceChain`, so a client sees the same provenance the gate reads.
pub const AUTHORED_PROVENANCE_JSON: &str = r#"{"projection":"verified-source-head"}"#;

/// Was this edge row projected from a verified signed head (this module)?
pub fn is_authored_projection(provenance_chain_json: Option<&str>) -> bool {
    provenance_chain_json == Some(AUTHORED_PROVENANCE_JSON)
}

/// The reach that judges an endpoint of `edge` which has NO local content row.
///
/// Only an edge projected from a verified source head says anything about an
/// atom this peer does not hold: its reach is the source atom's, and the head
/// that states it was verified. Any other row — a POSTed or seeded edge — is
/// an unauthenticated claim about an id nobody here can check, so the unknown
/// endpoint is judged at `""`, which no caller reads (anonymous, identified or
/// steward: `""` is not a tier, and the authorizer refuses it).
pub fn unknown_endpoint_reach(edge: &super::models::Relationship) -> &str {
    if is_authored_projection(edge.provenance_chain_json.as_deref()) {
        &edge.reach
    } else {
        ""
    }
}

/// Narrow every stored edge whose SOURCE is `source_id` to at most `reach`
/// (never widen one): the source atom's reach just moved, and an edge is never
/// more open than the atom that authored it. Returns how many rows narrowed.
///
/// Runs on the caller's connection, so a reach change and its edges move in
/// the caller's transaction. Authored rows are then set EXACTLY to the source
/// reach by [`replace_authored_edges`] when the new version states its edges.
pub fn narrow_edges_of_source(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    source_id: &str,
    reach: &str,
) -> Result<usize, StorageError> {
    let ceiling = crate::epr_service::reach_level_index(reach);
    let rows: Vec<(String, String)> = relationships::table
        .filter(relationships::h_app_id.eq(&ctx.h_app_id))
        .filter(relationships::source_id.eq(source_id))
        .select((relationships::id, relationships::reach))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("edge reach read failed: {e}")))?;
    let wider: Vec<String> = rows
        .into_iter()
        .filter(|(_, r)| crate::epr_service::reach_level_index(r) < ceiling)
        .map(|(id, _)| id)
        .collect();
    if wider.is_empty() {
        return Ok(0);
    }
    diesel::update(
        relationships::table
            .filter(relationships::h_app_id.eq(&ctx.h_app_id))
            .filter(relationships::id.eq_any(&wider)),
    )
    .set((
        relationships::reach.eq(reach),
        relationships::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("edge reach narrow failed: {e}")))
}

/// Re-stamp every AUTHORED edge of `source_id` — the rows projected from its
/// verified signed head ([`is_authored_projection`]) — to exactly `reach`, the
/// source's reach (edge reach = source reach). Returns how many rows moved.
///
/// The widening counterpart of [`narrow_edges_of_source`], for the earned
/// adoption that opens a row (`content_diesel::widen_to_adopted_earned_reach`).
/// Only authored rows follow the source up: a POSTed or seeded edge stated its
/// own reach, and a source opening is no licence to open it.
pub fn restamp_authored_edges_of_source(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    source_id: &str,
    reach: &str,
) -> Result<usize, StorageError> {
    diesel::update(
        relationships::table
            .filter(relationships::h_app_id.eq(&ctx.h_app_id))
            .filter(relationships::source_id.eq(source_id))
            .filter(relationships::inference_source.eq(AUTHORED))
            .filter(relationships::provenance_chain_json.eq(AUTHORED_PROVENANCE_JSON))
            .filter(relationships::reach.ne(reach)),
    )
    .set((
        relationships::reach.eq(reach),
        relationships::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("authored edge reach restamp failed: {e}")))
}

/// One authored edge as its source entry states it, already canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredEdge {
    /// A lamad manifest relationship id.
    pub relationship_type: String,
    pub target_id: String,
    pub role: Option<String>,
}

/// What a replace did, for the per-adoption log line (concern C8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EdgeReplaceCounts {
    pub inserted: usize,
    pub updated: usize,
    pub removed: usize,
    pub unchanged: usize,
}

/// The deterministic storage id of an authored edge — identical to the
/// seeder's `relationshipId(source, type, target)`.
pub fn relationship_id(source_id: &str, relationship_type: &str, target_id: &str) -> String {
    let digest = Sha256::digest(format!("{source_id}|{relationship_type}|{target_id}").as_bytes());
    let hex = hex::encode(digest);
    format!("rel-{}", &hex[..32])
}

fn manifest_types() -> &'static HashSet<String> {
    static TYPES: OnceLock<HashSet<String>> = OnceLock::new();
    TYPES.get_or_init(|| {
        const RAW: &str = include_str!("../../../sdk/domains/lamad/manifest/relationships.json");
        let parsed: serde_json::Map<String, serde_json::Value> = serde_json::from_str(RAW)
            .expect("elohim/sdk/domains/lamad/manifest/relationships.json must be a JSON object");
        parsed.into_iter().map(|(k, _)| k).collect()
    })
}

/// Authored-alias → manifest id; mirrors the seeder's
/// `relationship-vocabulary.ts` ALIASES (keys lower-case, `-`/space → `_`).
fn alias(key: &str) -> Option<&'static str> {
    Some(match key {
        "extends" | "extend" | "derived_from" | "derives_from" => "DEPENDS_ON",
        "prereq" | "prerequisite" | "requires" => "REQUIRES",
        "followup" | "follow_up" => "FOLLOWS",
        "parent" | "belongs_to" => "BELONGS_TO",
        "child" | "contains" => "CONTAINS",
        "relates" => "RELATES_TO",
        "references" => "REFERENCES",
        "implements" => "IMPLEMENTS",
        "validates" => "VALIDATES",
        "describes" => "DESCRIBES",
        "attached_to" => "ATTACHED_TO",
        _ => return None,
    })
}

fn normalize(raw: &str) -> String {
    raw.split(|c: char| c == '-' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// Canonicalize an authored type onto the lamad manifest vocabulary — the same
/// mapping as the seeder's `canonicalRelationshipType`: exact manifest ids pass,
/// known aliases map, anything else is `RELATES_TO`.
pub fn canonical_relationship_type(raw: &str) -> String {
    let trimmed = raw.trim();
    let upper = normalize(&trimmed.to_uppercase());
    if manifest_types().contains(&upper) {
        return upper;
    }
    if let Some(mapped) = alias(&normalize(&trimmed.to_lowercase())) {
        return mapped.to_string();
    }
    "RELATES_TO".to_string()
}

fn non_empty(v: Option<&serde_json::Value>) -> Option<String> {
    v.and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// The authored edges a conductor-verified entry's `metadata_json` states, or
/// `None` when it states none (no `relationships` array — see module doc).
///
/// Accepted item spellings match the seeder's builder: `targetId | target |
/// target_id`; `type | relationshipType | relationship_type`; `role`. Items
/// with no target and self-edges are skipped; repeats of a (type, target) pair
/// collapse onto the first. Output is sorted by (type, target) and capped at
/// [`MAX_EDGES_PER_ATOM`].
pub fn edges_from_verified_metadata(
    source_id: &str,
    metadata_json: &str,
) -> Option<Vec<AuthoredEdge>> {
    let value: serde_json::Value = serde_json::from_str(metadata_json).ok()?;
    let items = value.get("relationships")?.as_array()?;
    let mut edges: BTreeMap<(String, String), AuthoredEdge> = BTreeMap::new();
    for item in items {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let Some(target) = non_empty(obj.get("targetId"))
            .or_else(|| non_empty(obj.get("target")))
            .or_else(|| non_empty(obj.get("target_id")))
        else {
            continue;
        };
        if target == source_id {
            continue;
        }
        let raw_type = non_empty(obj.get("type"))
            .or_else(|| non_empty(obj.get("relationshipType")))
            .or_else(|| non_empty(obj.get("relationship_type")))
            .unwrap_or_else(|| "RELATES_TO".to_string());
        let relationship_type = canonical_relationship_type(&raw_type);
        edges
            .entry((relationship_type.clone(), target.clone()))
            .or_insert(AuthoredEdge {
                relationship_type,
                target_id: target,
                role: non_empty(obj.get("role")),
            });
    }
    let mut out: Vec<AuthoredEdge> = edges.into_values().collect();
    if out.len() > MAX_EDGES_PER_ATOM {
        tracing::warn!(
            source_id = %source_id,
            stated = out.len(),
            kept = MAX_EDGES_PER_ATOM,
            "authored edges over the per-atom cap; truncating deterministically (type, target order)"
        );
        out.truncate(MAX_EDGES_PER_ATOM);
    }
    Some(out)
}

fn role_metadata(role: &Option<String>) -> Option<String> {
    role.as_ref()
        .map(|r| serde_json::json!({ "role": r }).to_string())
}

/// A stored authored edge: (id, type, target, reach, metadata_json, anchor,
/// provenance).
type StoredEdge = (
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Replace the authored (`explicit`) edges of `source_id` with `edges`, at the
/// source's `reach`, stamping `dht_anchor_hash` with the source head that
/// states them. Idempotent: an unchanged list writes nothing (concern C6b).
///
/// Runs on the caller's connection — inside the same transaction as the
/// content-row projection when the caller holds one.
///
/// **Edge id on takeover.** A NEW authored edge gets the deterministic
/// [`relationship_id`]. But when the (source, type, target) slot is already
/// held — by a seeded `path` row, or a POSTed row with a caller-chosen or
/// random UUID id — the upsert takes that row over IN PLACE and KEEPS its
/// pre-existing id, so on such a peer the edge id is not the one every other
/// peer computes. Deliberately not migrated: `relationships.id` is the primary
/// key other rows point at (`inverse_relationship_id`) and callers may hold
/// it, so rewriting it here would dangle those references. The edge's identity
/// across peers is its (source, type, target) triple; the id is a local handle.
pub fn replace_authored_edges(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    source_id: &str,
    edges: &[AuthoredEdge],
    reach: &str,
    source_anchor: Option<&str>,
) -> Result<EdgeReplaceCounts, StorageError> {
    let existing: Vec<StoredEdge> = relationships::table
        .filter(relationships::h_app_id.eq(&ctx.h_app_id))
        .filter(relationships::source_id.eq(source_id))
        .filter(relationships::inference_source.eq(AUTHORED))
        .select((
            relationships::id,
            relationships::relationship_type,
            relationships::target_id,
            relationships::reach,
            relationships::metadata_json,
            relationships::dht_anchor_hash,
            relationships::provenance_chain_json,
        ))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("authored edges read failed: {e}")))?;

    let stated: HashSet<(&str, &str)> = edges
        .iter()
        .map(|e| (e.relationship_type.as_str(), e.target_id.as_str()))
        .collect();
    let mut counts = EdgeReplaceCounts::default();

    for (id, rel_type, target, _, _, _, _) in &existing {
        if !stated.contains(&(rel_type.as_str(), target.as_str())) {
            diesel::delete(relationships::table.filter(relationships::id.eq(id)))
                .execute(conn)
                .map_err(|e| StorageError::Internal(format!("authored edge delete failed: {e}")))?;
            counts.removed += 1;
        }
    }

    for edge in edges {
        let metadata = role_metadata(&edge.role);
        let prior = existing.iter().find(|(_, t, target, _, _, _, _)| {
            *t == edge.relationship_type && *target == edge.target_id
        });
        match prior {
            Some((id, _, _, row_reach, row_meta, row_anchor, row_provenance)) => {
                if row_reach == reach
                    && *row_meta == metadata
                    && row_anchor.as_deref() == source_anchor
                    && is_authored_projection(row_provenance.as_deref())
                {
                    counts.unchanged += 1;
                    continue;
                }
                diesel::update(relationships::table.filter(relationships::id.eq(id)))
                    .set((
                        relationships::reach.eq(reach),
                        relationships::metadata_json.eq(metadata.as_deref()),
                        relationships::dht_anchor_hash.eq(source_anchor),
                        relationships::provenance_chain_json.eq(AUTHORED_PROVENANCE_JSON),
                        relationships::updated_at.eq(current_timestamp()),
                    ))
                    .execute(conn)
                    .map_err(|e| {
                        StorageError::Internal(format!("authored edge update failed: {e}"))
                    })?;
                counts.updated += 1;
            }
            None => {
                let id = relationship_id(source_id, &edge.relationship_type, &edge.target_id);
                let row = NewRelationship {
                    id: &id,
                    h_app_id: &ctx.h_app_id,
                    source_id,
                    target_id: &edge.target_id,
                    relationship_type: &edge.relationship_type,
                    confidence: 1.0,
                    inference_source: AUTHORED,
                    is_bidirectional: 0,
                    inverse_relationship_id: None,
                    provenance_chain_json: Some(AUTHORED_PROVENANCE_JSON),
                    governance_layer: None,
                    reach,
                    metadata_json: metadata.as_deref(),
                };
                // A same-triple row under another inference source (e.g. a
                // seeded `path` edge) holds the unique slot: take it over as
                // the authored statement rather than failing the projection.
                // The taken-over row keeps its id (see the fn doc).
                diesel::insert_into(relationships::table)
                    .values(&row)
                    .on_conflict((
                        relationships::h_app_id,
                        relationships::source_id,
                        relationships::target_id,
                        relationships::relationship_type,
                    ))
                    .do_update()
                    .set((
                        relationships::inference_source.eq(AUTHORED),
                        relationships::reach.eq(reach),
                        relationships::metadata_json.eq(metadata.as_deref()),
                        relationships::provenance_chain_json.eq(AUTHORED_PROVENANCE_JSON),
                        relationships::updated_at.eq(current_timestamp()),
                    ))
                    .execute(conn)
                    .map_err(|e| {
                        StorageError::Internal(format!("authored edge insert failed: {e}"))
                    })?;
                diesel::update(
                    relationships::table
                        .filter(relationships::h_app_id.eq(&ctx.h_app_id))
                        .filter(relationships::source_id.eq(source_id))
                        .filter(relationships::target_id.eq(&edge.target_id))
                        .filter(relationships::relationship_type.eq(&edge.relationship_type)),
                )
                .set(relationships::dht_anchor_hash.eq(source_anchor))
                .execute(conn)
                .map_err(|e| StorageError::Internal(format!("authored edge anchor failed: {e}")))?;
                counts.inserted += 1;
            }
        }
    }

    if counts.inserted + counts.updated + counts.removed > 0 {
        tracing::info!(
            source_id = %source_id,
            inserted = counts.inserted,
            updated = counts.updated,
            removed = counts.removed,
            unchanged = counts.unchanged,
            reach = %reach,
            "authored edges projected from the verified source entry"
        );
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_id_matches_the_seeder() {
        // genesis/seeder: relationshipId('a', 'RELATES_TO', 'b') =
        // 'rel-' + sha256('a|RELATES_TO|b').slice(0, 32)
        let expected = format!(
            "rel-{}",
            &hex::encode(Sha256::digest(b"a|RELATES_TO|b"))[..32]
        );
        assert_eq!(relationship_id("a", "RELATES_TO", "b"), expected);
        assert_eq!(expected.len(), 36);
    }

    #[test]
    fn types_canonicalize_onto_the_manifest() {
        assert_eq!(canonical_relationship_type("RELATES_TO"), "RELATES_TO");
        assert_eq!(canonical_relationship_type("relates-to"), "RELATES_TO");
        assert_eq!(canonical_relationship_type("contains"), "CONTAINS");
        assert_eq!(canonical_relationship_type("extends"), "DEPENDS_ON");
        assert_eq!(canonical_relationship_type("prereq"), "REQUIRES");
        assert_eq!(canonical_relationship_type("follow up"), "FOLLOWS");
        assert_eq!(canonical_relationship_type("wobbles"), "RELATES_TO");
    }

    #[test]
    fn no_key_states_nothing_and_an_empty_list_states_none() {
        assert_eq!(edges_from_verified_metadata("s", "{}"), None);
        assert_eq!(edges_from_verified_metadata("s", "not json"), None);
        assert_eq!(
            edges_from_verified_metadata("s", r#"{"relationships":[]}"#),
            Some(vec![])
        );
    }

    #[test]
    fn items_parse_dedupe_sort_and_skip_self_edges() {
        let meta = serde_json::json!({ "relationships": [
            { "type": "RELATES_TO", "targetId": "z", "role": "callback" },
            { "relationshipType": "contains", "target": "b" },
            { "type": "RELATES_TO", "targetId": "z", "role": "other" },
            { "type": "RELATES_TO", "targetId": "s" },
            { "type": "RELATES_TO" },
        ]})
        .to_string();
        let edges = edges_from_verified_metadata("s", &meta).unwrap();
        assert_eq!(
            edges,
            vec![
                AuthoredEdge {
                    relationship_type: "CONTAINS".into(),
                    target_id: "b".into(),
                    role: None
                },
                AuthoredEdge {
                    relationship_type: "RELATES_TO".into(),
                    target_id: "z".into(),
                    role: Some("callback".into())
                },
            ]
        );
    }

    #[test]
    fn the_cap_truncates_deterministically() {
        let items: Vec<_> = (0..300)
            .rev()
            .map(|i| serde_json::json!({ "type": "RELATES_TO", "targetId": format!("t{i:03}") }))
            .collect();
        let meta = serde_json::json!({ "relationships": items }).to_string();
        let edges = edges_from_verified_metadata("s", &meta).unwrap();
        assert_eq!(edges.len(), MAX_EDGES_PER_ATOM);
        assert_eq!(edges[0].target_id, "t000");
        assert_eq!(edges[MAX_EDGES_PER_ATOM - 1].target_id, "t255");
    }

    fn rows(conn: &mut SqliteConnection) -> Vec<(String, String, String, String, Option<String>)> {
        relationships::table
            .order((relationships::relationship_type, relationships::target_id))
            .select((
                relationships::id,
                relationships::relationship_type,
                relationships::target_id,
                relationships::reach,
                relationships::metadata_json,
            ))
            .load(conn)
            .unwrap()
    }

    #[test]
    fn replace_inserts_updates_role_and_removes_dropped_edges() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        let e = |t: &str, target: &str, role: Option<&str>| AuthoredEdge {
            relationship_type: t.into(),
            target_id: target.into(),
            role: role.map(str::to_owned),
        };

        let c = replace_authored_edges(
            &mut conn,
            &ctx,
            "src",
            &[
                e("RELATES_TO", "a", Some("anchor")),
                e("CONTAINS", "b", None),
            ],
            "intimate",
            Some("uhCkk-v1"),
        )
        .unwrap();
        assert_eq!((c.inserted, c.updated, c.removed), (2, 0, 0));
        let got = rows(&mut conn);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, relationship_id("src", "CONTAINS", "b"));
        assert!(
            got.iter().all(|r| r.3 == "intimate"),
            "edge reach = source reach"
        );

        // Same list again: nothing written.
        let c = replace_authored_edges(
            &mut conn,
            &ctx,
            "src",
            &[
                e("RELATES_TO", "a", Some("anchor")),
                e("CONTAINS", "b", None),
            ],
            "intimate",
            Some("uhCkk-v1"),
        )
        .unwrap();
        assert_eq!(
            (c.inserted, c.updated, c.removed, c.unchanged),
            (0, 0, 0, 2)
        );

        // Role changes, b dropped.
        let c = replace_authored_edges(
            &mut conn,
            &ctx,
            "src",
            &[e("RELATES_TO", "a", Some("callback"))],
            "intimate",
            Some("uhCkk-v2"),
        )
        .unwrap();
        assert_eq!((c.inserted, c.updated, c.removed), (0, 1, 1));
        let got = rows(&mut conn);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].4.as_deref(), Some(r#"{"role":"callback"}"#));

        // Another source's edges are never touched.
        replace_authored_edges(
            &mut conn,
            &ctx,
            "other",
            &[e("RELATES_TO", "a", None)],
            "commons",
            None,
        )
        .unwrap();
        replace_authored_edges(&mut conn, &ctx, "src", &[], "intimate", None).unwrap();
        let got = rows(&mut conn);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, relationship_id("other", "RELATES_TO", "a"));
    }

    fn insert_plain_edge(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
        source: &str,
        target: &str,
        inference_source: &str,
        reach: &str,
    ) {
        diesel::insert_into(relationships::table)
            .values(&NewRelationship {
                id,
                h_app_id: &ctx.h_app_id,
                source_id: source,
                target_id: target,
                relationship_type: "RELATES_TO",
                confidence: 1.0,
                inference_source,
                is_bidirectional: 0,
                inverse_relationship_id: None,
                provenance_chain_json: None,
                governance_layer: None,
                reach,
                metadata_json: None,
            })
            .execute(conn)
            .unwrap();
    }

    fn provenance_of(conn: &mut SqliteConnection, id: &str) -> Option<String> {
        relationships::table
            .filter(relationships::id.eq(id))
            .select(relationships::provenance_chain_json)
            .first(conn)
            .unwrap()
    }

    /// A takeover of a slot held by another writer keeps that row's id — the
    /// deterministic id is NOT migrated onto it (see `replace_authored_edges`'
    /// doc: the id is a local handle other rows may point at) — but the row
    /// becomes the authored statement: explicit, at the source reach, marked.
    #[test]
    fn takeover_keeps_the_pre_existing_id_and_marks_the_row_authored() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        insert_plain_edge(
            &mut conn,
            &ctx,
            "uuid-seeded-1",
            "src",
            "a",
            "path",
            "commons",
        );

        let c = replace_authored_edges(
            &mut conn,
            &ctx,
            "src",
            &[AuthoredEdge {
                relationship_type: "RELATES_TO".into(),
                target_id: "a".into(),
                role: None,
            }],
            "intimate",
            Some("uhCkk-v1"),
        )
        .unwrap();
        assert_eq!(c.inserted, 1);
        let got = rows(&mut conn);
        assert_eq!(got.len(), 1);
        assert_eq!(
            got[0].0, "uuid-seeded-1",
            "the taken-over row keeps its id; it is not the deterministic one"
        );
        assert_ne!(got[0].0, relationship_id("src", "RELATES_TO", "a"));
        assert_eq!(got[0].3, "intimate");
        assert!(is_authored_projection(
            provenance_of(&mut conn, "uuid-seeded-1").as_deref()
        ));
    }

    /// An explicit row a POST wrote carries no marker; the first verified
    /// projection that states the same edge marks it (an unmarked row is never
    /// counted "unchanged").
    #[test]
    fn a_verified_projection_marks_an_unmarked_explicit_row() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        let id = relationship_id("src", "RELATES_TO", "a");
        insert_plain_edge(&mut conn, &ctx, &id, "src", "a", "explicit", "commons");
        assert!(!is_authored_projection(
            provenance_of(&mut conn, &id).as_deref()
        ));

        let c = replace_authored_edges(
            &mut conn,
            &ctx,
            "src",
            &[AuthoredEdge {
                relationship_type: "RELATES_TO".into(),
                target_id: "a".into(),
                role: None,
            }],
            "commons",
            None,
        )
        .unwrap();
        assert_eq!((c.updated, c.unchanged), (1, 0));
        assert!(is_authored_projection(
            provenance_of(&mut conn, &id).as_deref()
        ));
    }

    #[test]
    fn narrowing_a_source_never_widens_an_edge() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        insert_plain_edge(&mut conn, &ctx, "e-open", "src", "a", "explicit", "commons");
        insert_plain_edge(&mut conn, &ctx, "e-narrow", "src", "b", "explicit", "self");
        insert_plain_edge(
            &mut conn, &ctx, "e-other", "other", "a", "explicit", "commons",
        );

        let n = narrow_edges_of_source(&mut conn, &ctx, "src", "intimate").unwrap();
        assert_eq!(n, 1);
        let reach = |conn: &mut SqliteConnection, id: &str| -> String {
            relationships::table
                .filter(relationships::id.eq(id))
                .select(relationships::reach)
                .first(conn)
                .unwrap()
        };
        assert_eq!(reach(&mut conn, "e-open"), "intimate");
        assert_eq!(reach(&mut conn, "e-narrow"), "self", "never widened");
        assert_eq!(
            reach(&mut conn, "e-other"),
            "commons",
            "other sources untouched"
        );
    }
}
