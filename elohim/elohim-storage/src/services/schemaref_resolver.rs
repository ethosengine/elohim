//! schemaRef resolver — recursively walks schemaRef CID chains from EPR
//! atoms to their Manifest-EPR schemas.
//!
//! Phase 3 P3.3:
//! - cycle detection (HashSet of visited CIDs)
//! - depth limit modulated by Standing (placeholder; full depth at Unknown)
//! - floor: protocol-load-bearing schemaRef (kind == Manifest) always full depth
//!
//! Phase 3.5 will fold in cold-fetch via swarm when a manifest is missing
//! locally; today the resolver reads only from the local `manifests`
//! projection table.

use std::collections::HashSet;

use diesel::SqliteConnection;
use elohim_epr::EprKind;

use crate::db::manifests::fetch_manifest_by_cid;
use crate::error::StorageError;
use crate::services::floor_protections::is_protocol_load_bearing_schemaref;
use crate::services::standing::Standing;

#[derive(Debug, thiserror::Error)]
pub enum SchemaRefError {
    #[error("schema cycle detected at cid {0}")]
    Cycle(String),
    #[error("depth limit ({limit}) exceeded at cid {at}")]
    DepthExceeded { limit: usize, at: String },
    #[error("missing manifest at cid {0}")]
    Missing(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

#[derive(Debug, Clone)]
pub struct SchemaRefWalk {
    pub start: String,
    pub depth: usize,
    pub manifest_cids: Vec<String>,
}

/// Walk the schemaRef chain starting at `start_cid`. Returns the chain of
/// manifest CIDs encountered (terminal manifest first to last in walk order).
///
/// Floor protection: when `kind == EprKind::Manifest`, the depth limit is
/// effectively unlimited (usize::MAX); manifest-on-manifest references must
/// always resolve at full depth regardless of standing gradient.
pub fn walk_schemaref(
    conn: &mut SqliteConnection,
    start_cid: &str,
    kind: EprKind,
    standing: Standing,
) -> Result<SchemaRefWalk, SchemaRefError> {
    let limit = if is_protocol_load_bearing_schemaref(kind) {
        usize::MAX
    } else {
        standing.schemaref_depth_limit()
    };

    let mut visited: HashSet<String> = HashSet::new();
    let mut chain: Vec<String> = Vec::new();
    let mut current = start_cid.to_string();
    let mut depth: usize = 0;

    loop {
        if visited.contains(&current) {
            return Err(SchemaRefError::Cycle(current));
        }
        visited.insert(current.clone());

        let manifest = fetch_manifest_by_cid(conn, &current)?
            .ok_or_else(|| SchemaRefError::Missing(current.clone()))?;
        chain.push(current.clone());

        let Some(next) = manifest.schema_ref else {
            return Ok(SchemaRefWalk {
                start: start_cid.to_string(),
                depth,
                manifest_cids: chain,
            });
        };

        if depth + 1 > limit {
            return Err(SchemaRefError::DepthExceeded { limit, at: next });
        }
        depth += 1;
        current = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manifests::{insert_manifest, ManifestRow};
    use crate::test_util::test_pool;

    fn manifest_with_schema_ref(cid: &str, schema_ref: Option<&str>) -> ManifestRow {
        ManifestRow {
            cid: cid.to_string(),
            manifest_kind: "pillar-projection".to_string(),
            pillar: Some("lamad".to_string()),
            payload_json: "{}".to_string(),
            schema_ref: schema_ref.map(String::from),
            signer_pubkey: vec![0u8; 32],
            created_at: "2026-04-30T00:00:00Z".to_string(),
            verified_at: Some("2026-04-30T00:00:00Z".to_string()),
            revision: 1,
        }
    }

    #[test]
    fn single_manifest_no_schemaref_returns_chain_of_one() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_manifest(&mut conn, &manifest_with_schema_ref("a", None)).unwrap();
        let walk = walk_schemaref(&mut conn, "a", EprKind::Content, Standing::Unknown).unwrap();
        assert_eq!(walk.manifest_cids, vec!["a".to_string()]);
        assert_eq!(walk.depth, 0);
    }

    #[test]
    fn chain_walks_to_terminal_manifest() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_manifest(&mut conn, &manifest_with_schema_ref("a", Some("b"))).unwrap();
        insert_manifest(&mut conn, &manifest_with_schema_ref("b", Some("c"))).unwrap();
        insert_manifest(&mut conn, &manifest_with_schema_ref("c", None)).unwrap();
        let walk = walk_schemaref(&mut conn, "a", EprKind::Content, Standing::Unknown).unwrap();
        assert_eq!(
            walk.manifest_cids,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        assert_eq!(walk.depth, 2);
    }

    #[test]
    fn cycle_is_detected() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_manifest(&mut conn, &manifest_with_schema_ref("a", Some("b"))).unwrap();
        insert_manifest(&mut conn, &manifest_with_schema_ref("b", Some("a"))).unwrap();
        let result = walk_schemaref(&mut conn, "a", EprKind::Content, Standing::Unknown);
        match result {
            Err(SchemaRefError::Cycle(c)) => assert_eq!(c, "a"),
            other => panic!("expected SchemaRefError::Cycle, got {other:?}"),
        }
    }

    #[test]
    fn missing_manifest_errors() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let result = walk_schemaref(&mut conn, "nope", EprKind::Content, Standing::Unknown);
        assert!(matches!(result, Err(SchemaRefError::Missing(_))));
    }

    #[test]
    fn floor_protection_full_depth_for_manifest_kind() {
        // Even at lowest standing, EprKind::Manifest walks at full depth.
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // Build a chain of 6 manifests (well beyond Floor's depth limit of 3).
        for (cid, next) in [
            ("a", Some("b")),
            ("b", Some("c")),
            ("c", Some("d")),
            ("d", Some("e")),
            ("e", Some("f")),
            ("f", None),
        ] {
            insert_manifest(&mut conn, &manifest_with_schema_ref(cid, next)).unwrap();
        }
        let low_standing = Standing::Computed {
            score: crate::services::standing::StandingScore::Floor,
        };
        // Manifest is protocol-load-bearing — full depth bypasses the gradient.
        let walk = walk_schemaref(&mut conn, "a", EprKind::Manifest, low_standing).unwrap();
        assert_eq!(walk.manifest_cids.len(), 6);
    }

    #[test]
    fn low_standing_clips_non_manifest_walk() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        for (cid, next) in [
            ("a", Some("b")),
            ("b", Some("c")),
            ("c", Some("d")),
            ("d", Some("e")),
            ("e", None),
        ] {
            insert_manifest(&mut conn, &manifest_with_schema_ref(cid, next)).unwrap();
        }
        let low_standing = Standing::Computed {
            score: crate::services::standing::StandingScore::Floor,
        };
        // Floor standing limits depth to 3 for non-Manifest kinds; the walk
        // exceeds that and returns DepthExceeded with limit == 3.
        let result = walk_schemaref(&mut conn, "a", EprKind::Content, low_standing);
        match result {
            Err(SchemaRefError::DepthExceeded { limit, .. }) => assert_eq!(limit, 3),
            other => panic!("expected DepthExceeded, got {other:?}"),
        }
    }
}
