//! Scores incoming inventory_gossip advertisements against the local peer's
//! active replicates-* commitments. Output: priority tier consumed by the
//! existing inventory subscriber to decide which advertised blobs to fetch.
//!
//! Per spec §8.2: the substrate's "commitments shape what peers cache"
//! mechanism. Without it, peers fetch indiscriminately and commitments are
//! decorative.
//!
//! ## Wave 3: active_commitments_for_provider
//!
//! `active_commitments_for_provider(conn, self_cid)` loads the local peer's
//! active `replicates-dwelling` commitments from `rea_commitments`, parses each
//! `metadata_json` as `ReplicatesDwellingPayload`, and maps into `ActiveCommitment`.
//! Mirror of `peer_capacity_service::aggregate_pledges_by_tier` (read-side inline
//! Diesel; ReconcileController owns writes).

use crate::error::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchPriority {
    High,
    #[allow(dead_code)] // reserved: commons-tier follow-up
    Medium,
    Skip,
}

#[derive(Debug, Clone)]
pub struct AdvertisedBlob {
    pub blob_cid: String,
    pub source_peer_cid: String,
    pub blob_size_bytes: Option<u64>,
    pub recipient_hub_id_hint: Option<String>,
    pub epr_kind_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActiveCommitment {
    pub commitment_cid: String,
    pub action: String, // "replicates-dwelling" etc.
    pub recipient_hub_id: String,
    pub scope_epr_kinds: Option<Vec<String>>,
    pub bytes_per_blob_max: Option<u64>,
}

/// Load the local peer's active `replicates-dwelling` commitments from `rea_commitments`.
///
/// Filters: `provider == self_cid`, `action == "replicates-dwelling"`,
/// `state NOT IN ('cancelled', 'terminated')`. Parses `metadata_json` as
/// `ReplicatesDwellingPayload`; rows with missing or unparseable metadata are skipped.
///
/// This is a read-side inline Diesel query — no writes, no ReconcileController.
/// Mirror of `peer_capacity_service::aggregate_pledges_by_tier`.
pub fn active_commitments_for_provider(
    conn: &mut diesel::SqliteConnection,
    self_cid: &str,
) -> Result<Vec<ActiveCommitment>, StorageError> {
    use crate::db::diesel_schema::rea_commitments::dsl as rc;
    use diesel::prelude::*;
    use elohim_views::replicates_dwelling::ReplicatesDwellingPayload;

    let rows: Vec<crate::db::models::ReaCommitment> = rc::rea_commitments
        .filter(rc::provider.eq(self_cid))
        .filter(rc::action.eq("replicates-dwelling"))
        .filter(rc::state.ne("cancelled"))
        .filter(rc::state.ne("terminated"))
        .load(conn)
        .map_err(|e| StorageError::Database(format!("active_commitments_for_provider: {e}")))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(meta) = row.metadata_json.as_deref() else {
            tracing::debug!(
                target: "elohim_storage::prioritizer",
                id = %row.id,
                "active_commitments_for_provider: skipping row with NULL metadata_json"
            );
            continue;
        };
        let Ok(payload) = serde_json::from_str::<ReplicatesDwellingPayload>(meta) else {
            tracing::warn!(
                target: "elohim_storage::prioritizer",
                id = %row.id,
                "active_commitments_for_provider: failed to parse metadata_json; skipping"
            );
            continue;
        };
        out.push(ActiveCommitment {
            commitment_cid: row.dht_anchor_hash.unwrap_or(row.id),
            action: row.action,
            recipient_hub_id: payload.recipient_dwelling_hub_id,
            scope_epr_kinds: payload.scope_filter.epr_kinds,
            bytes_per_blob_max: payload.scope_filter.bytes_per_blob_max,
        });
    }
    Ok(out)
}

pub fn score_advertised_blob(
    advertised: &AdvertisedBlob,
    active_commitments: &[ActiveCommitment],
) -> FetchPriority {
    for commitment in active_commitments {
        if commitment.action != "replicates-dwelling" {
            continue;
        }
        // Recipient match
        if let Some(rcpt) = &advertised.recipient_hub_id_hint {
            if rcpt != &commitment.recipient_hub_id {
                continue;
            }
        } else {
            // Without recipient hint, can't match. Skip.
            continue;
        }
        // Scope match (epr_kind)
        if let (Some(kinds), Some(kind)) = (&commitment.scope_epr_kinds, &advertised.epr_kind_hint)
        {
            if !kinds.iter().any(|k| k == kind) {
                continue;
            }
        }
        // Size match
        if let (Some(max), Some(size)) = (commitment.bytes_per_blob_max, advertised.blob_size_bytes)
        {
            if size > max {
                continue;
            }
        }
        return FetchPriority::High;
    }
    // Commons-tier eligible — deferred. Sprint 3 always returns Skip when no dwelling match.
    FetchPriority::Skip
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // active_commitments_for_provider unit tests
    // -----------------------------------------------------------------------

    fn test_pool() -> crate::db::DbPool {
        use crate::db::{run_migrations, DbPool};
        use diesel::r2d2::{ConnectionManager, Pool};
        use diesel::SqliteConnection;
        let url = format!(
            "file:prioritizer_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool: DbPool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn insert_replicates_dwelling_commitment(
        conn: &mut diesel::SqliteConnection,
        id: &str,
        provider: &str,
        recipient_hub_id: &str,
        state: &str,
        epr_kinds: Option<Vec<&str>>,
        bytes_per_blob_max: Option<u64>,
        dht_anchor_hash: Option<&str>,
    ) {
        use crate::db::diesel_schema::rea_commitments;
        use diesel::prelude::*;
        use elohim_views::replicates_dwelling::{
            ProviderRole, RatioAttestation, ReplicatesDwellingPayload, ScopeFilter,
        };

        let scope_filter = ScopeFilter {
            epr_kinds: epr_kinds.map(|v| v.into_iter().map(|s| s.to_string()).collect()),
            bytes_per_blob_max,
            requires_attestations: None,
            kinds_excluded: None,
        };
        let payload = ReplicatesDwellingPayload {
            action: "replicates-dwelling".to_string(),
            provider_dwelling_hub_id: "hub:provider".to_string(),
            recipient_dwelling_hub_id: recipient_hub_id.to_string(),
            provider_role: ProviderRole::StewardMutual,
            via_collective_hub_id: None,
            capacity_bytes: 10_000_000_000,
            scope_filter,
            valid_from: "2026-01-01T00:00:00Z".to_string(),
            valid_until: "2027-01-01T00:00:00Z".to_string(),
            grace_period_days: 7,
            rotation_ttl_days: 90,
            ratio_attestation: RatioAttestation {
                commons_pct: 0,
                dwelling_pct: 100,
                collective_pct: 0,
                free_pct: 0,
                effective_ratio_cid: "cid:ratio-test".to_string(),
            },
        };
        let metadata = serde_json::to_string(&payload).unwrap();

        diesel::insert_into(rea_commitments::table)
            .values((
                rea_commitments::id.eq(id),
                rea_commitments::h_app_id.eq("test-app"),
                rea_commitments::action.eq("replicates-dwelling"),
                rea_commitments::provider.eq(provider),
                rea_commitments::receiver.eq("hub:recipient"),
                rea_commitments::state.eq(state),
                rea_commitments::finished.eq(0),
                rea_commitments::metadata_json.eq(Some(&metadata as &str)),
                rea_commitments::dht_anchor_hash.eq(dht_anchor_hash),
                rea_commitments::created_at.eq("2026-01-01T00:00:00Z"),
            ))
            .execute(conn)
            .expect("insert commitment");
    }

    /// Wave 3 T3-5: active commitment is loaded and mapped correctly.
    #[test]
    fn active_commitments_for_provider_parses_replicates_dwelling() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        insert_replicates_dwelling_commitment(
            &mut conn,
            "comm-001",
            "agent:uhCAkProvider001",
            "collective:uhCkkHub001",
            "active",
            Some(vec!["markdown", "sophia-quiz-json"]),
            Some(50_000_000),
            Some("comm-001-dht"),
        );

        let commitments =
            active_commitments_for_provider(&mut conn, "agent:uhCAkProvider001").unwrap();

        assert_eq!(commitments.len(), 1);
        let c = &commitments[0];
        assert_eq!(c.commitment_cid, "comm-001-dht", "prefers dht_anchor_hash");
        assert_eq!(c.action, "replicates-dwelling");
        assert_eq!(c.recipient_hub_id, "collective:uhCkkHub001");
        assert_eq!(
            c.scope_epr_kinds,
            Some(vec!["markdown".to_string(), "sophia-quiz-json".to_string()])
        );
        assert_eq!(c.bytes_per_blob_max, Some(50_000_000));
    }

    /// Wave 3 T3-6: cancelled and terminated commitments are excluded.
    #[test]
    fn active_commitments_excludes_cancelled_and_terminated() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        insert_replicates_dwelling_commitment(
            &mut conn,
            "comm-cancelled",
            "agent:uhCAkProvider002",
            "hub:X",
            "cancelled",
            None,
            None,
            None,
        );
        insert_replicates_dwelling_commitment(
            &mut conn,
            "comm-terminated",
            "agent:uhCAkProvider002",
            "hub:X",
            "terminated",
            None,
            None,
            None,
        );
        insert_replicates_dwelling_commitment(
            &mut conn,
            "comm-active",
            "agent:uhCAkProvider002",
            "hub:X",
            "active",
            None,
            None,
            None,
        );

        let commitments =
            active_commitments_for_provider(&mut conn, "agent:uhCAkProvider002").unwrap();

        assert_eq!(commitments.len(), 1, "only active commitment returned");
        assert_eq!(commitments[0].commitment_cid, "comm-active");
    }

    /// Wave 3 T3-7: fallback to `id` when `dht_anchor_hash` is NULL.
    #[test]
    fn active_commitments_falls_back_to_id_when_dht_hash_null() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        insert_replicates_dwelling_commitment(
            &mut conn,
            "comm-no-dht",
            "agent:uhCAkProvider003",
            "hub:Y",
            "proposed",
            None,
            None,
            None, // dht_anchor_hash NULL
        );

        let commitments =
            active_commitments_for_provider(&mut conn, "agent:uhCAkProvider003").unwrap();

        assert_eq!(commitments.len(), 1);
        assert_eq!(
            commitments[0].commitment_cid, "comm-no-dht",
            "falls back to row.id when dht_anchor_hash is NULL"
        );
    }

    /// Wave 3 T3-8: receive-arm scoring — hint matches active commitment → HIGH.
    /// Non-matching hub → Skip. This is a pure scoring unit test (no DB).
    #[test]
    fn receive_arm_scoring_high_when_hint_matches_commitment() {
        let commitment = ActiveCommitment {
            commitment_cid: "comm:dwelling-H".into(),
            action: "replicates-dwelling".into(),
            recipient_hub_id: "collective:hubH".into(),
            scope_epr_kinds: Some(vec!["markdown".into()]),
            bytes_per_blob_max: Some(10_000_000),
        };
        let matching = AdvertisedBlob {
            blob_cid: sha256_wire_str('a'),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: Some(1_000_000),
            recipient_hub_id_hint: Some("collective:hubH".into()),
            epr_kind_hint: Some("markdown".into()),
        };
        let non_matching = AdvertisedBlob {
            blob_cid: sha256_wire_str('b'),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: Some(1_000_000),
            recipient_hub_id_hint: Some("collective:hubZ".into()),
            epr_kind_hint: Some("markdown".into()),
        };

        assert_eq!(
            score_advertised_blob(&matching, &[commitment.clone()]),
            FetchPriority::High,
            "hint matching active commitment → High"
        );
        assert_eq!(
            score_advertised_blob(&non_matching, &[commitment]),
            FetchPriority::Skip,
            "non-matching hub → Skip"
        );
    }

    fn sha256_wire_str(byte: char) -> String {
        format!(
            "sha256-{}",
            std::iter::repeat_n(byte, 64).collect::<String>()
        )
    }

    fn commitment(action: &str, recipient: &str) -> ActiveCommitment {
        ActiveCommitment {
            commitment_cid: "comm:test".into(),
            action: action.into(),
            recipient_hub_id: recipient.into(),
            scope_epr_kinds: Some(vec!["Content".into()]),
            bytes_per_blob_max: Some(1_000_000_000),
        }
    }

    fn ad(recipient: &str, kind: &str, size: u64) -> AdvertisedBlob {
        AdvertisedBlob {
            blob_cid: "bafkrei:test".into(),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: Some(size),
            recipient_hub_id_hint: Some(recipient.into()),
            epr_kind_hint: Some(kind.into()),
        }
    }

    #[test]
    fn high_when_recipient_and_scope_match() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "Content", 500_000_000);
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::High);
    }

    #[test]
    fn skip_when_no_matching_recipient() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:Z", "Content", 100);
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_blob_exceeds_size_ceiling() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "Content", 5_000_000_000); // > 1GB max
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_kind_not_in_scope() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "EconomicEvent", 100);
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_no_commitments() {
        let a = ad("hub:B", "Content", 100);
        assert_eq!(score_advertised_blob(&a, &[]), FetchPriority::Skip);
    }
}
