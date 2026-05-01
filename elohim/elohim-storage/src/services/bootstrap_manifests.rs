//! Bootstrap manifest seeder — Phase 3.5 P3.5.8.
//!
//! On first run, loads the bootstrap standing-policy and tending-policy
//! manifests into the local `manifests` table. Idempotent: subsequent runs
//! skip kinds already present. Communities fork-and-modify these as the
//! brainstorm §7.2 "starting point, not law" pattern.
//!
//! Bootstrap is local-only — these manifests are NOT DHT-published, NOT
//! signed, NOT gossiped. They exist to prevent a cold-start system from
//! having no policy at all. Production deployments author their own
//! standing-policy / tending-policy manifests through the normal Manifest
//! EPR flow and the bootstrap rows are then either ignored (lower
//! revision) or replaced (higher revision wins per §4 manifest revision
//! semantics).
//!
//! TODO(T19): wire seed_if_empty into main.rs startup, after migrations
//!   and before serving HTTP. Suggested call site: just after the
//!   PubkeyTimelineCache initialization in main.rs (~line 985).

use diesel::sqlite::SqliteConnection;

use crate::db::manifests::{fetch_manifests_by_kind, insert_manifest, ManifestRow};
use crate::error::StorageError;

const BOOTSTRAP_STANDING_POLICY_JSON: &str =
    include_str!("../../../sdk/schemas/v1/manifests/bootstrap-standing-policy.json");
const BOOTSTRAP_TENDING_POLICY_JSON: &str =
    include_str!("../../../sdk/schemas/v1/manifests/bootstrap-tending-policy.json");

const BOOTSTRAP_SIGNER_PUBKEY: [u8; 32] = [0u8; 32];
const BOOTSTRAP_CID_STANDING: &str = "bootstrap:standing-policy:v1";
const BOOTSTRAP_CID_TENDING: &str = "bootstrap:tending-policy:v1";

/// Errors specific to the bootstrap seeder.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    /// A database operation via the manifests repository failed.
    #[error("storage: {0}")]
    Storage(#[from] StorageError),
    /// The embedded JSON file could not be parsed (compile-time JSON is always
    /// valid; this variant exists so callers can handle it explicitly in tests).
    #[error("manifest payload parse: {0}")]
    PayloadParse(String),
}

/// Report of which bootstrap manifest kinds were seeded during this run.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SeedReport {
    /// `true` when the standing-policy bootstrap row was inserted this run.
    pub standing_policy_seeded: bool,
    /// `true` when the tending-policy bootstrap row was inserted this run.
    pub tending_policy_seeded: bool,
}

/// Seed bootstrap manifests if the corresponding kinds are absent.
///
/// Checks each manifest kind independently. If any manifests of that kind
/// already exist (peer-authored or previously bootstrapped), seeding for that
/// kind is skipped. Re-running on a populated registry is a no-op.
pub fn seed_if_empty(conn: &mut SqliteConnection) -> Result<SeedReport, BootstrapError> {
    let mut report = SeedReport::default();

    if fetch_manifests_by_kind(conn, "standing-policy")?.is_empty() {
        seed_standing_policy(conn)?;
        report.standing_policy_seeded = true;
    }
    if fetch_manifests_by_kind(conn, "tending-policy")?.is_empty() {
        seed_tending_policy(conn)?;
        report.tending_policy_seeded = true;
    }
    Ok(report)
}

fn seed_standing_policy(conn: &mut SqliteConnection) -> Result<(), BootstrapError> {
    let payload: serde_json::Value = serde_json::from_str(BOOTSTRAP_STANDING_POLICY_JSON)
        .map_err(|e| BootstrapError::PayloadParse(format!("standing-policy: {e}")))?;
    insert_manifest(
        conn,
        &ManifestRow {
            cid: BOOTSTRAP_CID_STANDING.to_string(),
            manifest_kind: "standing-policy".to_string(),
            pillar: None,
            payload_json: payload.to_string(),
            schema_ref: None,
            signer_pubkey: BOOTSTRAP_SIGNER_PUBKEY.to_vec(),
            created_at: "1970-01-01T00:00:00Z".to_string(),
            verified_at: Some(chrono::Utc::now().to_rfc3339()),
            revision: 1,
        },
    )?;
    Ok(())
}

fn seed_tending_policy(conn: &mut SqliteConnection) -> Result<(), BootstrapError> {
    let payload: serde_json::Value = serde_json::from_str(BOOTSTRAP_TENDING_POLICY_JSON)
        .map_err(|e| BootstrapError::PayloadParse(format!("tending-policy: {e}")))?;
    insert_manifest(
        conn,
        &ManifestRow {
            cid: BOOTSTRAP_CID_TENDING.to_string(),
            manifest_kind: "tending-policy".to_string(),
            pillar: None,
            payload_json: payload.to_string(),
            schema_ref: None,
            signer_pubkey: BOOTSTRAP_SIGNER_PUBKEY.to_vec(),
            created_at: "1970-01-01T00:00:00Z".to_string(),
            verified_at: Some(chrono::Utc::now().to_rfc3339()),
            revision: 1,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manifests::{fetch_manifest_by_cid, insert_manifest, ManifestRow};
    use crate::test_util::test_pool;

    // -------------------------------------------------------------------------
    // JSON structural tests (no DB required)
    // -------------------------------------------------------------------------

    #[test]
    fn bootstrap_standing_policy_json_parses_as_object() {
        let v: serde_json::Value =
            serde_json::from_str(BOOTSTRAP_STANDING_POLICY_JSON).expect("should parse");
        assert!(v.is_object(), "standing-policy JSON must be an object");
        assert_eq!(
            v["manifestKind"].as_str(),
            Some("standing-policy"),
            "manifestKind must be 'standing-policy'"
        );
    }

    #[test]
    fn bootstrap_tending_policy_json_parses_as_object() {
        let v: serde_json::Value =
            serde_json::from_str(BOOTSTRAP_TENDING_POLICY_JSON).expect("should parse");
        assert!(v.is_object(), "tending-policy JSON must be an object");
        assert_eq!(
            v["manifestKind"].as_str(),
            Some("tending-policy"),
            "manifestKind must be 'tending-policy'"
        );
    }

    #[test]
    fn bootstrap_standing_policy_has_all_five_floor_classes() {
        let v: serde_json::Value =
            serde_json::from_str(BOOTSTRAP_STANDING_POLICY_JSON).expect("should parse");
        let classes = v["floor"]["classes"]
            .as_array()
            .expect("floor.classes must be an array");
        assert_eq!(
            classes.len(),
            5,
            "standing-policy must have exactly 5 floor classes"
        );
        let names: Vec<&str> = classes
            .iter()
            .map(|c| {
                c["class"]
                    .as_str()
                    .expect("each class entry must have a 'class' string")
            })
            .collect();
        assert!(names.contains(&"local-relationship-reach"));
        assert!(names.contains(&"cid-targeted-lookup"));
        assert!(names.contains(&"constitutional-floor-signatures"));
        assert!(names.contains(&"new-voice-baseline"));
        assert!(names.contains(&"vulnerable-class-elevation"));
    }

    #[test]
    fn bootstrap_tending_policy_has_all_five_floor_classes() {
        let v: serde_json::Value =
            serde_json::from_str(BOOTSTRAP_TENDING_POLICY_JSON).expect("should parse");
        let classes = v["floor"]["classes"]
            .as_array()
            .expect("floor.classes must be an array");
        assert_eq!(
            classes.len(),
            5,
            "tending-policy must have exactly 5 floor classes"
        );
        let names: Vec<&str> = classes
            .iter()
            .map(|c| {
                c["class"]
                    .as_str()
                    .expect("each class entry must have a 'class' string")
            })
            .collect();
        assert!(names.contains(&"accountability-information"));
        assert!(names.contains(&"community-facts"));
        assert!(names.contains(&"custodial-communications"));
        assert!(names.contains(&"constitutional-updates"));
        assert!(names.contains(&"elohim-as-counsel-notifications"));
    }

    #[test]
    fn bootstrap_standing_policy_has_all_four_signal_kind_debit_weights() {
        let v: serde_json::Value =
            serde_json::from_str(BOOTSTRAP_STANDING_POLICY_JSON).expect("should parse");
        let weights = v["debitWeights"]
            .as_object()
            .expect("debitWeights must be an object");
        for kind in &["squelch", "correction", "retraction", "quarantine"] {
            let entry = weights
                .get(*kind)
                .unwrap_or_else(|| panic!("debitWeights.{kind} missing"));
            assert!(
                entry.get("advisory").is_some(),
                "debitWeights.{kind}.advisory missing"
            );
            assert!(
                entry.get("debit-soft").is_some(),
                "debitWeights.{kind}.debit-soft missing"
            );
            assert!(
                entry.get("debit-firm").is_some(),
                "debitWeights.{kind}.debit-firm missing"
            );
        }
    }

    // -------------------------------------------------------------------------
    // DB seeder tests
    // -------------------------------------------------------------------------

    #[test]
    fn seed_if_empty_seeds_both_when_table_empty() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let report = seed_if_empty(&mut conn).expect("seed_if_empty should succeed");
        assert!(
            report.standing_policy_seeded,
            "standing-policy should have been seeded"
        );
        assert!(
            report.tending_policy_seeded,
            "tending-policy should have been seeded"
        );

        // Verify rows exist
        let standing = fetch_manifests_by_kind(&mut conn, "standing-policy").unwrap();
        assert_eq!(
            standing.len(),
            1,
            "exactly one standing-policy row expected"
        );
        assert_eq!(standing[0].cid, BOOTSTRAP_CID_STANDING);

        let tending = fetch_manifests_by_kind(&mut conn, "tending-policy").unwrap();
        assert_eq!(tending.len(), 1, "exactly one tending-policy row expected");
        assert_eq!(tending[0].cid, BOOTSTRAP_CID_TENDING);
    }

    #[test]
    fn seed_if_empty_idempotent_after_first_seed() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // First seed
        seed_if_empty(&mut conn).expect("first seed should succeed");

        // Second seed — must be a no-op
        let report = seed_if_empty(&mut conn).expect("second seed should succeed");
        assert!(
            !report.standing_policy_seeded,
            "standing-policy must NOT be seeded again"
        );
        assert!(
            !report.tending_policy_seeded,
            "tending-policy must NOT be seeded again"
        );

        // Row counts unchanged
        let standing = fetch_manifests_by_kind(&mut conn, "standing-policy").unwrap();
        assert_eq!(
            standing.len(),
            1,
            "standing-policy row count must stay at 1"
        );

        let tending = fetch_manifests_by_kind(&mut conn, "tending-policy").unwrap();
        assert_eq!(tending.len(), 1, "tending-policy row count must stay at 1");
    }

    #[test]
    fn seed_if_empty_skips_only_existing_kind() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // Pre-insert a tending-policy manifest manually (simulates peer-authored, revision=99)
        insert_manifest(
            &mut conn,
            &ManifestRow {
                cid: "manual:tending-policy:rev99".to_string(),
                manifest_kind: "tending-policy".to_string(),
                pillar: None,
                payload_json: r#"{"manifestKind":"tending-policy","revision":99}"#.to_string(),
                schema_ref: None,
                signer_pubkey: vec![0u8; 32],
                created_at: "2026-04-01T00:00:00Z".to_string(),
                verified_at: None,
                revision: 99,
            },
        )
        .expect("pre-insert should succeed");

        let report = seed_if_empty(&mut conn).expect("seed_if_empty should succeed");
        assert!(
            report.standing_policy_seeded,
            "standing-policy should have been seeded"
        );
        assert!(
            !report.tending_policy_seeded,
            "tending-policy must NOT be seeded (already present)"
        );

        // Verify standing was seeded
        let standing = fetch_manifests_by_kind(&mut conn, "standing-policy").unwrap();
        assert_eq!(standing.len(), 1);
        assert_eq!(standing[0].cid, BOOTSTRAP_CID_STANDING);

        // Verify the manual tending row is preserved (not replaced)
        let tending = fetch_manifests_by_kind(&mut conn, "tending-policy").unwrap();
        assert_eq!(tending.len(), 1);
        assert_eq!(
            tending[0].cid, "manual:tending-policy:rev99",
            "manual tending-policy row must be preserved"
        );
        assert_eq!(tending[0].revision, 99, "manual revision must be preserved");

        // Also verify the bootstrap tending row was NOT inserted
        let bootstrap_tending = fetch_manifest_by_cid(&mut conn, BOOTSTRAP_CID_TENDING).unwrap();
        assert!(
            bootstrap_tending.is_none(),
            "bootstrap tending-policy CID must not exist when kind already present"
        );
    }
}
