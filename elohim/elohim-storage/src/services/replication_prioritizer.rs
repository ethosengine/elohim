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

/// Fetch priority for an advertised blob. Ordering is meaningful: the enqueue
/// gate treats anything strictly above `Skip` as fetch-worthy. Declaration
/// order is ascending — `Skip < Medium < High`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FetchPriority {
    /// Below the fetch floor — never enqueued.
    Skip,
    /// Commons-tier: fetched only during an active acquisition pull (the
    /// content id is supplied as scoring context). Not fetched on passive
    /// gossip replication.
    Medium,
    /// Dwelling-tier: a `replicates-dwelling` commitment recipient/scope match.
    High,
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
    pub action: String, // "replicates-dwelling" | "replicates-commons"
    pub recipient_hub_id: String,
    pub scope_epr_kinds: Option<Vec<String>>,
    pub bytes_per_blob_max: Option<u64>,
    /// Present only for `replicates-commons`: the content head_ref this
    /// commitment covers. `None` for dwelling commitments.
    pub head_ref: Option<String>,
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
    use crate::db::diesel_schema::mishpat_commitments::dsl as mc;
    use crate::db::diesel_schema::rea_commitments::dsl as rc;
    use diesel::prelude::*;
    use elohim_views::replicates_dwelling::ReplicatesDwellingPayload;

    // -- Arm 1: replicates-dwelling (rea_commitments) — unchanged behaviour.
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
            head_ref: None,
        });
    }

    // -- Arm 2: replicates-commons (mishpat_commitments). Notarized only:
    // require dht_anchor_hash NOT NULL so un-notarized rows never drive a fetch
    // (the commons path is conductor-authored, unlike dwelling's storage-direct
    // rowid fallback). The `recipient` column holds the content head_ref.
    let commons: Vec<crate::db::models::MishpatCommitment> = mc::mishpat_commitments
        .filter(mc::provider.eq(self_cid))
        .filter(mc::action.eq("replicates-commons"))
        .filter(mc::dht_anchor_hash.is_not_null())
        .filter(mc::revoked_at.is_null())
        .load(conn)
        .map_err(|e| {
            StorageError::Database(format!("active_commitments_for_provider commons: {e}"))
        })?;

    for row in commons {
        // dht_anchor_hash is guaranteed non-null by the filter; prefer it as the cid.
        let cid = row
            .dht_anchor_hash
            .clone()
            .unwrap_or_else(|| row.cid.clone());
        out.push(ActiveCommitment {
            commitment_cid: cid,
            action: row.action,
            recipient_hub_id: row.recipient.clone(),
            scope_epr_kinds: None,
            bytes_per_blob_max: None,
            head_ref: Some(row.recipient),
        });
    }

    Ok(out)
}

/// Score an advertised blob against the local peer's active commitments.
///
/// Dwelling commitments score `High` via recipient-hub/scope/size matching
/// (unchanged). Commons commitments score `Medium` only when `content_id_ctx`
/// — the head_ref of an in-flight acquisition pull — equals the commitment's
/// `head_ref`. Passive gossip replication passes `content_id_ctx == None`, so
/// commons never fires there (no greedy whole-commons fetch).
pub fn score_advertised_blob(
    advertised: &AdvertisedBlob,
    active_commitments: &[ActiveCommitment],
    content_id_ctx: Option<&str>,
) -> FetchPriority {
    // Dwelling tier (High) — unchanged.
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

    // Commons tier (Medium) — fires only during an active acquisition pull, when
    // the head_ref under acquisition is supplied as context.
    if let Some(ctx) = content_id_ctx {
        for commitment in active_commitments {
            if commitment.action != "replicates-commons" {
                continue;
            }
            if commitment.head_ref.as_deref() == Some(ctx) {
                return FetchPriority::Medium;
            }
        }
    }

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
    ///
    /// Fixture note (storage-tier review 2026-06-04, finding #2): scope values
    /// MUST come from the replicates-dwelling `scope_filter.epr_kinds` schema
    /// enum ("Content", "Manifest", …). The original fixture used "markdown"
    /// (a lamad content_format — schema-INVALID) on both sides, which masked a
    /// real producer/consumer vocabulary mismatch.
    #[test]
    fn receive_arm_scoring_high_when_hint_matches_commitment() {
        let commitment = ActiveCommitment {
            commitment_cid: "comm:dwelling-H".into(),
            action: "replicates-dwelling".into(),
            recipient_hub_id: "collective:hubH".into(),
            scope_epr_kinds: Some(vec!["Content".into()]),
            bytes_per_blob_max: Some(10_000_000),
            head_ref: None,
        };
        let matching = AdvertisedBlob {
            blob_cid: sha256_wire_str('a'),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: Some(1_000_000),
            recipient_hub_id_hint: Some("collective:hubH".into()),
            epr_kind_hint: Some("Content".into()),
        };
        let non_matching = AdvertisedBlob {
            blob_cid: sha256_wire_str('b'),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: Some(1_000_000),
            recipient_hub_id_hint: Some("collective:hubZ".into()),
            epr_kind_hint: Some("Content".into()),
        };

        assert_eq!(
            score_advertised_blob(&matching, std::slice::from_ref(&commitment), None),
            FetchPriority::High,
            "hint matching active commitment → High"
        );
        assert_eq!(
            score_advertised_blob(&non_matching, &[commitment], None),
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
            head_ref: None,
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
        assert_eq!(score_advertised_blob(&a, &[c], None), FetchPriority::High);
    }

    #[test]
    fn skip_when_no_matching_recipient() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:Z", "Content", 100);
        assert_eq!(score_advertised_blob(&a, &[c], None), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_blob_exceeds_size_ceiling() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "Content", 5_000_000_000); // > 1GB max
        assert_eq!(score_advertised_blob(&a, &[c], None), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_kind_not_in_scope() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "EconomicEvent", 100);
        assert_eq!(score_advertised_blob(&a, &[c], None), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_no_commitments() {
        let a = ad("hub:B", "Content", 100);
        assert_eq!(score_advertised_blob(&a, &[], None), FetchPriority::Skip);
    }

    // -----------------------------------------------------------------------
    // Slice 2b T12: commons-tier scoring
    // -----------------------------------------------------------------------

    fn commons_commitment(head_ref: &str) -> ActiveCommitment {
        ActiveCommitment {
            commitment_cid: "comm:commons-1".into(),
            action: "replicates-commons".into(),
            recipient_hub_id: head_ref.into(), // recipient column == head_ref
            scope_epr_kinds: None,
            bytes_per_blob_max: None,
            head_ref: Some(head_ref.into()),
        }
    }

    #[test]
    fn fetch_priority_orders_high_above_medium_above_skip() {
        assert!(FetchPriority::High > FetchPriority::Medium);
        assert!(FetchPriority::Medium > FetchPriority::Skip);
        assert!(FetchPriority::High > FetchPriority::Skip);
        // Ordering used by the enqueue gate: only Skip is the floor.
        assert_eq!(
            [
                FetchPriority::Skip,
                FetchPriority::High,
                FetchPriority::Medium
            ]
            .iter()
            .max()
            .copied(),
            Some(FetchPriority::High)
        );
    }

    #[test]
    fn commons_scored_medium_on_content_id_match() {
        let c = commons_commitment("head:epr-XYZ");
        // A commons advertisement carries no recipient_hub_id_hint; the match is
        // purely the active-acquisition content id passed as context.
        let a = AdvertisedBlob {
            blob_cid: sha256_wire_str('c'),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: Some(2_000_000),
            recipient_hub_id_hint: None,
            epr_kind_hint: None,
        };
        assert_eq!(
            score_advertised_blob(&a, std::slice::from_ref(&c), Some("head:epr-XYZ")),
            FetchPriority::Medium,
            "replicates-commons + content_id_ctx == head_ref → Medium"
        );
    }

    #[test]
    fn commons_skipped_when_no_content_id_ctx() {
        // Passive gossip replication passes content_id_ctx = None: commons never
        // fires, so the local peer does not greedily fetch every commons blob.
        let c = commons_commitment("head:epr-XYZ");
        let a = AdvertisedBlob {
            blob_cid: sha256_wire_str('d'),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: Some(2_000_000),
            recipient_hub_id_hint: None,
            epr_kind_hint: None,
        };
        assert_eq!(
            score_advertised_blob(&a, &[c], None),
            FetchPriority::Skip,
            "no content_id_ctx (passive replication) → commons does not fire"
        );
    }

    #[test]
    fn commons_skipped_when_content_id_mismatch() {
        let c = commons_commitment("head:epr-XYZ");
        let a = AdvertisedBlob {
            blob_cid: sha256_wire_str('e'),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: Some(2_000_000),
            recipient_hub_id_hint: None,
            epr_kind_hint: None,
        };
        assert_eq!(
            score_advertised_blob(&a, &[c], Some("head:OTHER")),
            FetchPriority::Skip,
            "content_id_ctx for a different head_ref → Skip"
        );
    }

    #[test]
    fn dwelling_path_unchanged_with_content_ctx_present() {
        // A dwelling commitment still scores High via the hub-hint path even when
        // an unrelated content_id_ctx is supplied — commons ctx must not perturb it.
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "Content", 500_000_000);
        assert_eq!(
            score_advertised_blob(&a, &[c], Some("head:irrelevant")),
            FetchPriority::High,
            "dwelling High path is unaffected by content_id_ctx"
        );
    }

    fn insert_replicates_commons_commitment(
        conn: &mut diesel::SqliteConnection,
        cid: &str,
        provider: &str,
        head_ref: &str,
        dht_anchor_hash: Option<&str>,
        revoked_at: Option<&str>,
    ) {
        use crate::db::diesel_schema::mishpat_commitments;
        use diesel::prelude::*;
        diesel::insert_into(mishpat_commitments::table)
            .values((
                mishpat_commitments::cid.eq(cid),
                mishpat_commitments::action.eq("replicates-commons"),
                mishpat_commitments::scope.eq("replicates-commons"),
                mishpat_commitments::provider.eq(provider),
                mishpat_commitments::recipient.eq(head_ref), // recipient == head_ref
                mishpat_commitments::bounds_json
                    .eq(r#"{"rate_per_minute":60,"reach_ceiling":"commons"}"#),
                mishpat_commitments::valid_from.eq("2026-01-01T00:00:00Z"),
                mishpat_commitments::valid_until.eq("2027-01-01T00:00:00Z"),
                mishpat_commitments::revoked_at.eq(revoked_at),
                mishpat_commitments::state.eq("active"),
                mishpat_commitments::dht_anchor_hash.eq(dht_anchor_hash),
                mishpat_commitments::created_at.eq("2026-01-01T00:00:00Z"),
                mishpat_commitments::updated_at.eq("2026-01-01T00:00:00Z"),
            ))
            .execute(conn)
            .expect("insert replicates-commons commitment");
    }

    #[test]
    fn commons_commitment_loaded_only_when_notarized_and_not_revoked() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Notarized, live → loaded.
        insert_replicates_commons_commitment(
            &mut conn,
            "cid-commons-ok",
            "agent:uhCAkCommons",
            "head:epr-OK",
            Some("anchor-ok"),
            None,
        );
        // Un-notarized (NULL anchor) → excluded.
        insert_replicates_commons_commitment(
            &mut conn,
            "cid-commons-unnotarized",
            "agent:uhCAkCommons",
            "head:epr-UNNOTARIZED",
            None,
            None,
        );
        // Revoked → excluded.
        insert_replicates_commons_commitment(
            &mut conn,
            "cid-commons-revoked",
            "agent:uhCAkCommons",
            "head:epr-REVOKED",
            Some("anchor-rev"),
            Some("2026-02-01T00:00:00Z"),
        );

        let commitments = active_commitments_for_provider(&mut conn, "agent:uhCAkCommons").unwrap();

        assert_eq!(
            commitments.len(),
            1,
            "only the notarized, live commons row loads"
        );
        let c = &commitments[0];
        assert_eq!(c.action, "replicates-commons");
        assert_eq!(c.commitment_cid, "anchor-ok", "prefers dht_anchor_hash");
        assert_eq!(c.head_ref.as_deref(), Some("head:epr-OK"));
        assert_eq!(c.recipient_hub_id, "head:epr-OK");

        // And it scores Medium under the matching acquisition context.
        let a = AdvertisedBlob {
            blob_cid: sha256_wire_str('f'),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: Some(1_000),
            recipient_hub_id_hint: None,
            epr_kind_hint: None,
        };
        assert_eq!(
            score_advertised_blob(&a, &commitments, Some("head:epr-OK")),
            FetchPriority::Medium
        );
    }
}
