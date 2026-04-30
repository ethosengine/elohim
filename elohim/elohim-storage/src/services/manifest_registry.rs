//! ManifestRegistry — replaces pillar_for_kind_provisional.
//!
//! Reads the `manifests` projection table to map EprKind → pillar via
//! pillar-projection manifest entries. Falls back to lowercase kind name
//! when no manifest is registered (bootstrap path).
//!
//! Phase 3 = registry reads from local projection (Category C).
//! Phase 3.5 = registry consults FeedbackSignal-derived standing for
//! cache priority and refresh schedule.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use diesel::SqliteConnection;
use elohim_epr::EprKind;

use crate::db::manifests::{fetch_manifests_by_kind, ManifestRow};
use crate::error::StorageError;
use crate::services::standing::Standing;

/// Registry caching pillar-projection manifests for fast pillar lookup.
pub struct ManifestRegistry {
    /// kind canonical name (lowercase) -> pillar; populated by `load_from_db`.
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl ManifestRegistry {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load (or refresh) the registry from the manifests projection table.
    /// Reads pillar-projection manifests and extracts kind→pillar mappings
    /// from each row's payload_json. Returns the count of mappings loaded.
    pub fn load_from_db(&self, conn: &mut SqliteConnection) -> Result<usize, StorageError> {
        let rows = fetch_manifests_by_kind(conn, "pillar-projection")?;
        let mut new_cache = HashMap::new();
        for row in &rows {
            extract_kind_pillar_pairs(row, &mut new_cache);
        }
        let count = new_cache.len();
        let mut cache = self
            .cache
            .write()
            .expect("manifest registry cache write lock poisoned");
        *cache = new_cache;
        Ok(count)
    }

    /// Fast-path author-side query: which pillar does this kind project to?
    /// Returns None if no manifest is registered for this kind. Caller falls
    /// back (bootstrap pattern: lowercased kind name as default pillar).
    ///
    /// Standing is wired through but Phase 3 does not consume it; Phase 3.5
    /// substrate (FeedbackSignal back-prop) lights up gradient-modulated
    /// registry semantics.
    pub fn pillar_for_kind(&self, kind: EprKind, _standing: Standing) -> Option<String> {
        let canonical = kind_canonical_str(kind);
        let cache = self
            .cache
            .read()
            .expect("manifest registry cache read lock poisoned");
        cache.get(canonical).cloned()
    }

    /// Whether the registry is empty (bootstrap path indicator).
    pub fn is_empty(&self) -> bool {
        self.cache
            .read()
            .expect("manifest registry cache read lock poisoned")
            .is_empty()
    }
}

impl Default for ManifestRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_kind_pillar_pairs(row: &ManifestRow, target: &mut HashMap<String, String>) {
    // pillar-projection manifest payload shape:
    // { "pillar": "lamad", "kinds": ["Content", "Mastery", …] }
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(&row.payload_json) else {
        return;
    };
    let Some(pillar) = payload.get("pillar").and_then(|v| v.as_str()) else {
        return;
    };
    let Some(kinds) = payload.get("kinds").and_then(|v| v.as_array()) else {
        return;
    };
    for k in kinds {
        if let Some(kind_str) = k.as_str() {
            target.insert(kind_str.to_lowercase(), pillar.to_string());
        }
    }
}

/// Project an EPR atom of `kind: Manifest` into the local manifests table.
/// Called by the storage-side projector when an EprKind::Manifest atom is
/// ingested via libp2p (cold-fetch or gossip).
///
/// Manifest payloads are JSON-encoded inside the EPR envelope payload bytes.
/// This helper decodes the JSON, extracts the manifestKind/pillar/schemaRef
/// fields, and inserts a row into the manifests projection table.
///
/// Field deviations from original spec:
/// - `signer_pubkey` is derived from `epr.envelope.proof.signer` (a CID) via
///   `to_bytes()` — the `Epr` struct has no `signer_pubkey` field directly.
/// - `created_at` maps to `epr.envelope.issued_at` (not `created_at`).
/// - The CID comes from `epr.envelope.cid.to_string()` (no `cid()` method).
pub fn project_manifest(
    conn: &mut diesel::SqliteConnection,
    epr: &elohim_epr::Epr,
) -> Result<(), crate::error::StorageError> {
    use crate::db::manifests::{insert_manifest, ManifestRow};

    let payload: serde_json::Value = serde_json::from_slice(&epr.payload).map_err(|e| {
        crate::error::StorageError::Database(format!("manifest payload decode: {e}"))
    })?;

    let manifest_kind = payload
        .get("manifestKind")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let pillar = payload
        .get("pillar")
        .and_then(|v| v.as_str())
        .map(String::from);
    let schema_ref = payload
        .get("schemaRef")
        .and_then(|v| v.as_str())
        .map(String::from);

    let row = ManifestRow {
        cid: epr.envelope.cid.to_string(),
        manifest_kind,
        pillar,
        payload_json: payload.to_string(),
        schema_ref,
        // Epr has no signer_pubkey field; derive raw bytes from the signer CID.
        signer_pubkey: epr.envelope.proof.signer.to_bytes(),
        created_at: epr.envelope.issued_at.to_rfc3339(),
        verified_at: Some(chrono::Utc::now().to_rfc3339()),
        revision: 1,
    };
    insert_manifest(conn, &row)?;
    Ok(())
}

/// Mirror of EprKind canonical lowercase serialization. Restated locally to
/// avoid a runtime dependency on the codec crate's specific serialization
/// helpers.
fn kind_canonical_str(kind: EprKind) -> &'static str {
    match kind {
        EprKind::Content => "content",
        EprKind::Agent => "agent",
        EprKind::Manifest => "manifest",
        EprKind::Claim => "claim",
        EprKind::Observation => "observation",
        EprKind::EconomicEvent => "economicevent",
        EprKind::Commitment => "commitment",
        EprKind::Attestation => "attestation",
        EprKind::Delegation => "delegation",
        EprKind::FeedbackSignal => "feedbacksignal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manifests::insert_manifest;
    use crate::test_util::test_pool;

    fn projection_row(cid: &str, pillar: &str, kinds: &[&str]) -> ManifestRow {
        let payload = serde_json::json!({ "pillar": pillar, "kinds": kinds }).to_string();
        ManifestRow {
            cid: cid.to_string(),
            manifest_kind: "pillar-projection".to_string(),
            pillar: Some(pillar.to_string()),
            payload_json: payload,
            schema_ref: None,
            signer_pubkey: vec![0u8; 32],
            created_at: "2026-04-30T00:00:00Z".to_string(),
            verified_at: Some("2026-04-30T00:00:00Z".to_string()),
            revision: 1,
        }
    }

    #[test]
    fn empty_registry_returns_none() {
        let registry = ManifestRegistry::new();
        let result = registry.pillar_for_kind(EprKind::Content, Standing::Unknown);
        assert!(result.is_none());
        assert!(registry.is_empty());
    }

    #[test]
    fn load_from_db_populates_cache() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_manifest(
            &mut conn,
            &projection_row("p1", "lamad", &["Content", "Observation"]),
        )
        .unwrap();
        insert_manifest(
            &mut conn,
            &projection_row("p2", "shefa", &["EconomicEvent"]),
        )
        .unwrap();
        let registry = ManifestRegistry::new();
        let loaded = registry.load_from_db(&mut conn).unwrap();
        assert_eq!(loaded, 3); // content, observation, economicevent
        assert_eq!(
            registry.pillar_for_kind(EprKind::Content, Standing::Unknown),
            Some("lamad".to_string())
        );
        assert_eq!(
            registry.pillar_for_kind(EprKind::EconomicEvent, Standing::Unknown),
            Some("shefa".to_string())
        );
        assert_eq!(
            registry.pillar_for_kind(EprKind::Manifest, Standing::Unknown),
            None
        );
        assert!(!registry.is_empty());
    }

    #[test]
    fn standing_arg_does_not_change_phase3_lookup() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_manifest(&mut conn, &projection_row("p1", "lamad", &["Content"])).unwrap();
        let registry = ManifestRegistry::new();
        registry.load_from_db(&mut conn).unwrap();
        // Phase 3: standing arg is wired but signal returns same lookup.
        // Phase 3.5 differentiates (low-standing might miss cached layer).
        let unknown = registry.pillar_for_kind(EprKind::Content, Standing::Unknown);
        let trusted = registry.pillar_for_kind(
            EprKind::Content,
            Standing::Computed {
                score: crate::services::standing::StandingScore::Trusted,
            },
        );
        assert_eq!(unknown, trusted);
    }
}
