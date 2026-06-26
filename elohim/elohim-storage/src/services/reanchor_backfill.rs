//! One-shot startup pass: re-anchor content rows the cold-conductor seed left
//! provenance-only.
//!
//! ## The gap it closes (the dark resilience card, root cause #2)
//!
//! The genesis seeder stamps reach on every content row by PATCHing
//! `{p2pPublishedAt, reach}`. The substrate-correct PATCH path routes any patch
//! touching a DNA-notarized field (`reach`/`blob_hash`) through
//! `ContentService::update_via_conductor` so the re-authored entry reaches the
//! DHT. But when the in-pod conductor's cells are still `CellDisabled` during the
//! seed/boot window, those conductor calls fail; the seeder's circuit latches
//! "provenance-only" and the rows land with `dht_anchor_hash IS NULL` — never
//! DHT-authored, reach never notarized. With no notarized reach there are no
//! `content:<reach>` provide rows, and the resilience snapshot reads zeros.
//!
//! This backfill is the elohim-storage-side recovery: once the lamad HcClient
//! bridge connects (cells enabled — which happens AFTER boot on a slow conductor,
//! per main.rs:616), it walks the NULL-anchor rows and re-authors each through
//! the conductor's `create_content`. The `ContentCommitted` projection stamps
//! `dht_anchor_hash` and notarizes reach — so a cold-conductor seed self-heals
//! on the next boot instead of latching dark forever.
//!
//! It reuses `ContentService::update_via_conductor` with an empty patch: for a
//! NULL-anchor row that method takes the bootstrap branch (`call_create_content`
//! from the existing SQL row) and projects the anchor — the canonical re-anchor
//! path, no duplicated conductor-call code.
//!
//! Category C (operational): re-authoring is idempotent. A row that gained an
//! anchor on a prior sweep is no longer a candidate. Bounded (batched, paced —
//! each re-author is a conductor round-trip) and tolerant: a failed re-author is
//! logged and retried on a future boot's sweep, never fatal.

use std::sync::Arc;
use std::time::Duration;

use crate::db::DbPool;
use crate::services::provide_loop_status::ProvideLoopState;
use crate::services::ContentService;
use crate::StorageError;

/// Tuning for a single re-anchor sweep.
#[derive(Debug, Clone)]
pub struct ReanchorConfig {
    /// Maximum NULL-anchor rows to re-author in one sweep (bounds conductor
    /// load on a large cold seed; the remainder heals on the next boot).
    pub max_per_sweep: i64,
    /// Pause between re-authors (each is a conductor round-trip; pace so the
    /// backfill never starves live HTTP/seed traffic on the same conductor).
    pub item_delay: Duration,
}

impl Default for ReanchorConfig {
    fn default() -> Self {
        Self {
            max_per_sweep: 2000,
            item_delay: Duration::from_millis(25),
        }
    }
}

/// Outcome counters for a sweep (returned for logging/tests).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReanchorReport {
    /// NULL-anchor candidates selected this sweep.
    pub candidates: usize,
    /// Rows successfully re-authored (anchor now stamped).
    pub reanchored: usize,
    /// Rows that errored re-authoring (non-fatal, retried next boot).
    pub failed: usize,
    /// NULL-anchor rows remaining AFTER the sweep (for `/p2p/status` pending).
    pub remaining: usize,
}

/// Run one re-anchor sweep over the NULL-anchor content rows.
///
/// Returns the sweep report and publishes it to `state` for `/p2p/status`.
/// Soft-fail throughout: a single row's failure is logged and counted, never
/// propagated — the goal is to heal as many rows as the conductor will accept.
pub async fn run_once(
    pool: &DbPool,
    content_service: &ContentService,
    hc: &Arc<crate::hc_client::HcClient>,
    state: &ProvideLoopState,
    cfg: &ReanchorConfig,
) -> Result<ReanchorReport, StorageError> {
    let app_ctx = crate::db::AppContext::default_lamad();

    let candidates: Vec<String> = {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("reanchor: db conn: {e}")))?;
        crate::db::content_diesel::list_unanchored_content_ids(
            &mut conn,
            &app_ctx,
            cfg.max_per_sweep,
        )?
    };

    let mut report = ReanchorReport {
        candidates: candidates.len(),
        ..Default::default()
    };

    if candidates.is_empty() {
        // Nothing to heal — caught up. Publish so the card shows green.
        state.publish_reanchor_sweep(0, 0, 0).await;
        return Ok(report);
    }

    tracing::info!(
        candidates = report.candidates,
        "reanchor_backfill: re-authoring NULL-anchor content via conductor (cold-seed recovery)"
    );

    for id in &candidates {
        // Empty patch → for a NULL-anchor row, update_via_conductor takes the
        // bootstrap branch: re-publishes the full entry from the existing SQL
        // row via create_content and projects dht_anchor_hash. Idempotent.
        let empty_patch = crate::views::UpdateContentInputView {
            title: None,
            description: None,
            content_body: None,
            content_format: None,
            metadata: None,
            tags: None,
            reach: None,
            blob_hash: None,
            server_blob_hash: None,
            p2p_published_at: None,
        };
        match content_service
            .update_via_conductor(hc, id, empty_patch)
            .await
        {
            Ok(_) => report.reanchored += 1,
            Err(e) => {
                report.failed += 1;
                tracing::warn!(
                    content_id = %id,
                    error = %e,
                    "reanchor_backfill: re-author failed (non-fatal, retried next boot)"
                );
            }
        }
        if !cfg.item_delay.is_zero() {
            tokio::time::sleep(cfg.item_delay).await;
        }
    }

    // Recount remaining NULL-anchor rows so pending/caughtUp are honest even
    // when a sweep was capped at max_per_sweep or some re-authors failed.
    report.remaining = {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("reanchor: recount conn: {e}")))?;
        crate::db::content_diesel::count_unanchored_content(&mut conn, &app_ctx)? as usize
    };

    state
        .publish_reanchor_sweep(report.reanchored, report.failed, report.remaining)
        .await;

    tracing::info!(
        reanchored = report.reanchored,
        failed = report.failed,
        remaining = report.remaining,
        "reanchor_backfill: sweep complete"
    );

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_bounded_and_paced() {
        let cfg = ReanchorConfig::default();
        assert!(cfg.max_per_sweep > 0, "must bound conductor load per sweep");
        assert!(
            !cfg.item_delay.is_zero(),
            "must pace re-authors so live traffic is not starved"
        );
    }

    #[test]
    fn empty_candidate_report_is_caught_up_shaped() {
        // The report shape an all-anchored DB produces (no candidates).
        let report = ReanchorReport::default();
        assert_eq!(report.candidates, 0);
        assert_eq!(report.reanchored, 0);
        assert_eq!(report.remaining, 0);
    }
}
