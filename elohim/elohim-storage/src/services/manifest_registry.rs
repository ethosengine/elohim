//! ManifestRegistry — replaces pillar_for_kind_provisional.
//!
//! Reads the `manifests` projection table to map EprKind → pillar via
//! pillar-projection manifest entries. Falls back to lowercase kind name
//! when no manifest is registered (bootstrap path).
//!
//! Phase 3 = registry reads from local projection (Category C).
//! Phase 3.5 = registry consults FeedbackSignal-derived standing for
//! cache priority and refresh schedule.
//!
//! Phase 3.5 Light-Up-Graph: adds standing-policy accessors (debit_weights,
//! reach_threshold, unknown_treatment, new_voice_baseline) that read from
//! the full standing-policy manifest payload stored in `standing_policy_payload`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use diesel::SqliteConnection;
use elohim_epr::EprKind;

use crate::db::manifests::{fetch_manifests_by_kind, ManifestRow};
use crate::error::StorageError;
use crate::services::standing::Standing;

/// How an unknown-standing author is treated at non-floor reach.
///
/// Corresponds to the `unknownTreatment.default` field in the standing-policy
/// manifest. Defaults to `Conservative` when no manifest is registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownTreatment {
    /// Unknown authors are held as Pending at non-floor reach (safe default).
    #[default]
    Conservative,
    /// Unknown authors are evaluated as if they have the newVoiceBaseline score.
    NewVoiceBaseline,
    /// Unknown authors are evaluated as if they have Neutral standing.
    Neutral,
}

/// Registry caching pillar-projection manifests for fast pillar lookup.
pub struct ManifestRegistry {
    /// kind canonical name (lowercase) -> pillar; populated by `load_from_db`.
    cache: Arc<RwLock<HashMap<String, String>>>,
    /// Full JSON payload of the active standing-policy manifest, if one has
    /// been loaded. Populated by `load_from_db` when it finds a row with
    /// `manifest_kind == "standing-policy"`. Uses `Mutex` for interior
    /// mutability so `load_from_db` can take `&self` (consistent with the
    /// existing `RwLock` cache pattern).
    standing_policy_payload: Mutex<Option<serde_json::Value>>,
}

impl ManifestRegistry {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            standing_policy_payload: Mutex::new(None),
        }
    }

    /// Parse a JSON payload string (as stored in the manifests table) into a
    /// registry instance. Used in tests; production loads via `load_from_db`.
    pub fn from_payload_json(json: &str) -> Result<Self, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let r = ManifestRegistry::new();
        *r.standing_policy_payload
            .lock()
            .expect("standing_policy_payload lock poisoned") = Some(value);
        Ok(r)
    }

    /// Load (or refresh) the registry from the manifests projection table.
    /// Reads pillar-projection manifests and extracts kind→pillar mappings
    /// from each row's payload_json. Also picks up the standing-policy manifest
    /// payload when a row with `manifest_kind == "standing-policy"` is found.
    /// Returns the count of pillar-projection mappings loaded.
    pub fn load_from_db(&self, conn: &mut SqliteConnection) -> Result<usize, StorageError> {
        let rows = fetch_manifests_by_kind(conn, "pillar-projection")?;
        let mut new_cache = HashMap::new();
        for row in &rows {
            extract_kind_pillar_pairs(row, &mut new_cache);
        }
        let count = new_cache.len();
        {
            let mut cache = self
                .cache
                .write()
                .expect("manifest registry cache write lock poisoned");
            *cache = new_cache;
        }

        // Load standing-policy payload if present.
        let policy_rows = fetch_manifests_by_kind(conn, "standing-policy").unwrap_or_default();
        if let Some(row) = policy_rows.first() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&row.payload_json) {
                *self
                    .standing_policy_payload
                    .lock()
                    .expect("standing_policy_payload lock poisoned") = Some(v);
            }
        }

        Ok(count)
    }

    // -------------------------------------------------------------------------
    // Standing-policy accessors (Phase 3.5 Light-Up-Graph)
    // -------------------------------------------------------------------------

    /// Returns flat `(signal_kind, impact) → weight` map from the standing-policy
    /// manifest's `debitWeights` block.
    /// Returns `None` if no standing-policy manifest is registered.
    pub fn debit_weights(&self) -> Option<HashMap<(String, String), i32>> {
        let guard = self
            .standing_policy_payload
            .lock()
            .expect("standing_policy_payload lock poisoned");
        let payload = guard.as_ref()?;
        let dw = payload.get("debitWeights")?.as_object()?;
        let mut out = HashMap::new();
        for (kind, impacts) in dw {
            if let Some(obj) = impacts.as_object() {
                for (impact, weight) in obj {
                    if let Some(w) = weight.as_i64() {
                        out.insert((kind.clone(), impact.clone()), w as i32);
                    }
                }
            }
        }
        Some(out)
    }

    /// Returns the manifest-declared threshold string for the given reach
    /// (e.g. `"any"`, `"neutral"`, `"high"`). Returns `None` when the reach
    /// key is absent from the manifest map, or when no manifest is registered.
    pub fn reach_threshold(&self, reach: &str) -> Option<String> {
        let guard = self
            .standing_policy_payload
            .lock()
            .expect("standing_policy_payload lock poisoned");
        let payload = guard.as_ref()?;
        let thresholds = payload.get("reachThresholds")?.as_object()?;
        thresholds.get(reach)?.as_str().map(|s| s.to_string())
    }

    /// Returns the manifest-declared `unknownTreatment.default` policy.
    /// Defaults to `Conservative` when no manifest is registered or the field
    /// is absent/unrecognised.
    pub fn unknown_treatment(&self) -> UnknownTreatment {
        let guard = self
            .standing_policy_payload
            .lock()
            .expect("standing_policy_payload lock poisoned");
        let Some(payload) = guard.as_ref() else {
            return UnknownTreatment::Conservative;
        };
        match payload
            .get("unknownTreatment")
            .and_then(|v| v.get("default"))
            .and_then(|v| v.as_str())
        {
            Some("newVoiceBaseline") => UnknownTreatment::NewVoiceBaseline,
            Some("neutral") => UnknownTreatment::Neutral,
            _ => UnknownTreatment::Conservative,
        }
    }

    /// Returns true if the agent appears in any quarantine list registered in
    /// the manifests. Phase 3.5 stub: always returns false; full quarantine
    /// list is a future sprint.
    pub fn is_quarantined(&self, _agent: &[u8]) -> bool {
        false
    }

    /// Returns the manifest-declared baseline lift for vulnerable-class agents.
    /// Phase 3.5 stub: always returns None; classification fetch is a future sprint.
    pub fn vulnerable_class_lift(
        &self,
        _agent: &[u8],
    ) -> Option<crate::services::standing::StandingScore> {
        None
    }

    /// Returns the bootstrap manifest's `newVoiceBaseline.score`, or `None`
    /// if not set.
    pub fn new_voice_baseline(&self) -> Option<crate::services::standing::StandingScore> {
        use crate::services::standing::StandingScore;
        let guard = self
            .standing_policy_payload
            .lock()
            .expect("standing_policy_payload lock poisoned");
        let payload = guard.as_ref()?;
        let score_str = payload.get("newVoiceBaseline")?.get("score")?.as_str()?;
        match score_str {
            "floor" => Some(StandingScore::Floor),
            "low" => Some(StandingScore::Low),
            "neutral" => Some(StandingScore::Neutral),
            "high" => Some(StandingScore::High),
            "trusted" => Some(StandingScore::Trusted),
            _ => None,
        }
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

// NOTE: ManifestRegistry is intentionally not Clone — it holds an Arc<RwLock<..>>
// for the pillar cache and a Mutex<Option<..>> for the policy payload. The registry
// is constructed once at startup and shared via Arc<ManifestRegistry>.

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
        EprKind::AttentionTending => "attentiontending",
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

    // -------------------------------------------------------------------------
    // T8 — Phase 3.5 Light-Up-Graph: standing-policy accessor tests
    // -------------------------------------------------------------------------

    #[test]
    fn debit_weights_extracts_vouch_block_from_manifest() {
        let json = r#"{
            "manifestKind": "standing-policy",
            "revision": 1,
            "floor": { "classes": [] },
            "newVoiceBaseline": { "score": "floor", "vulnerableClassLift": "low" },
            "debitWeights": {
                "squelch":    { "advisory": 0, "debit-soft": 1,  "debit-firm": 3 },
                "correction": { "advisory": 0, "debit-soft": 10, "debit-firm": 20 },
                "retraction": { "advisory": 0, "debit-soft": -5, "debit-firm": -10 },
                "quarantine": { "advisory": 0, "debit-soft": 12, "debit-firm": 30 },
                "vouch":      { "advisory": 0, "debit-soft": -3, "debit-firm": -8 }
            }
        }"#;
        let registry = ManifestRegistry::from_payload_json(json).expect("parse");
        let weights = registry.debit_weights().expect("weights present");
        assert_eq!(
            weights.get(&("vouch".into(), "debit-soft".into())),
            Some(&-3)
        );
        assert_eq!(
            weights.get(&("correction".into(), "debit-firm".into())),
            Some(&20)
        );
    }

    #[test]
    fn debit_weights_returns_none_for_empty_registry() {
        let registry = ManifestRegistry::default();
        assert!(registry.debit_weights().is_none());
    }

    #[test]
    fn reach_threshold_returns_correct_value() {
        let json = r#"{ "manifestKind": "standing-policy", "revision": 1,
            "floor": { "classes": [] },
            "newVoiceBaseline": { "score": "floor", "vulnerableClassLift": "low" },
            "debitWeights": {
                "squelch": {"advisory":0,"debit-soft":1,"debit-firm":3},
                "correction": {"advisory":0,"debit-soft":10,"debit-firm":20},
                "retraction": {"advisory":0,"debit-soft":-5,"debit-firm":-10},
                "quarantine": {"advisory":0,"debit-soft":12,"debit-firm":30},
                "vouch": {"advisory":0,"debit-soft":-3,"debit-firm":-8}
            },
            "reachThresholds": { "public": "high", "household": "any" }
        }"#;
        let r = ManifestRegistry::from_payload_json(json).expect("parse");
        assert_eq!(r.reach_threshold("public"), Some("high".to_string()));
        assert_eq!(r.reach_threshold("household"), Some("any".to_string()));
        assert_eq!(r.reach_threshold("not-in-map"), None);
    }

    #[test]
    fn unknown_treatment_defaults_when_missing() {
        let r = ManifestRegistry::default();
        assert_eq!(r.unknown_treatment(), UnknownTreatment::Conservative);
    }
}
