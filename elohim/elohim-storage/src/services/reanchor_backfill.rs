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
use crate::generated_enums::{ALL_CONTENT_TYPES, CORE_REACH_LEVELS};
use crate::services::provide_loop_status::ProvideLoopState;
use crate::services::ContentService;
use crate::StorageError;

/// True when `content_type` is re-authorable through the conductor's
/// `create_content` — i.e. it would pass `Content::validate()`
/// (`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/healing.rs`),
/// which checks the FULL extensible vocabulary (`ALL_CONTENT_TYPES`, not the
/// DNA-notarized-core-only subset — content_type has an extensible tier,
/// unlike reach) plus the `attestation:`/`governance-action:` discriminator
/// prefixes that are enumerated separately (`generated_attestation_kinds`),
/// never in the content-type list itself.
pub(crate) fn is_canonical_content_type(content_type: &str) -> bool {
    content_type.starts_with("attestation:")
        || content_type.starts_with("governance-action:")
        || ALL_CONTENT_TYPES.contains(&content_type)
}

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
    /// Rows found ALREADY committed to the DHT (`create_content` refused the
    /// duplicate id); their existing committed anchor was recovered and stamped
    /// this sweep. Idempotent success — counted apart from `reanchored` for
    /// observability, but NEVER as `failed`. Its existence is the fix for the
    /// per-boot re-thrash that saturated adam's Cache-DB read pool: without it,
    /// these rows took the `failed` branch, never stamped, and were re-selected
    /// on every single boot forever.
    pub already_anchored: usize,
    /// Rows that errored re-authoring (non-fatal, retried next boot).
    pub failed: usize,
    /// Rows skipped because their stored reach is non-canonical — not
    /// re-authorable (the DNA rejects it), so re-attempting would fail
    /// every sweep and saturate the conductor. Fix via the seed data.
    pub skipped: usize,
    /// NULL-anchor rows remaining AFTER the sweep (for `/p2p/status` pending).
    pub remaining: usize,
}

/// True when a re-author error means the content is ALREADY committed to the
/// conductor's DHT view. `create_content` refuses a duplicate id with a Guest
/// error (`content_store/src/lib.rs:2361`:
/// *"Content with id '…' already exists. Use update_content to modify existing
/// entries."*) that `HcClient::call_zome` surfaces as
/// `StorageError::Conductor("Zome call failed: … already exists. Use
/// update_content …")`.
///
/// This is the idempotent-SUCCESS signal for the reanchor sweep: "already
/// anchored" IS the goal state. Matching it lets the sweep recover the existing
/// anchor and stamp the row (leaving the NULL-anchor candidate set permanently)
/// instead of re-thrashing it every boot. Both substrings are required so this
/// stays specific to the `create_content` duplicate error and never matches the
/// stale-anchor class (*"no Content entry found"*) or an unrelated
/// "already exists" from another zome function. Mirrors the substring-match
/// discipline of the sibling stale-anchor heal in `update_via_conductor`.
pub(crate) fn is_already_anchored_error(err: &StorageError) -> bool {
    let msg = err.to_string();
    msg.contains("already exists") && msg.contains("update_content")
}

/// Disposition of a single NULL-anchor row within a sweep. Pure + total so the
/// sweep's accounting is unit-testable without a live conductor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowOutcome {
    /// Re-authored via `create_content` — the projection stamped a fresh anchor.
    Reanchored,
    /// Already committed to the DHT (create refused the duplicate); the existing
    /// committed anchor was recovered and stamped. Idempotent success — the fix
    /// for the perpetual-retry saturation.
    AlreadyAnchored,
    /// Stored reach outside the DNA vocabulary — never re-authorable; skipped
    /// permanently (a seed-data fix, not a retry).
    SkippedNonCanonicalReach,
    /// Stored content_type outside the vocabulary `Content::validate()`
    /// accepts (`ALL_CONTENT_TYPES` + the `attestation:`/`governance-action:`
    /// prefixes) — never re-authorable; skipped permanently (a seed-data fix,
    /// not a retry). Symmetric with `SkippedNonCanonicalReach`.
    SkippedNonCanonicalContentType,
    /// Genuine, retryable failure — healed on a future boot's sweep.
    Failed,
}

impl ReanchorReport {
    /// Fold a single row's outcome into the report counters.
    fn record(&mut self, outcome: RowOutcome) {
        match outcome {
            RowOutcome::Reanchored => self.reanchored += 1,
            RowOutcome::AlreadyAnchored => self.already_anchored += 1,
            RowOutcome::SkippedNonCanonicalReach => self.skipped += 1,
            RowOutcome::SkippedNonCanonicalContentType => self.skipped += 1,
            RowOutcome::Failed => self.failed += 1,
        }
    }
}

/// Decide a row's outcome from the re-author result and — only for the
/// already-exists case — the anchor-recovery result. Pure + total.
///
/// The idempotency contract lives here: an "already exists" re-author error is
/// a FAILURE only when the existing anchor could not be recovered (`recovery`
/// is `None`, `Some(Ok(false))`, or `Some(Err(_))`). When recovery stamps the
/// row (`Some(Ok(true))`), the outcome is [`RowOutcome::AlreadyAnchored`],
/// never [`RowOutcome::Failed`] — so the row leaves the NULL-anchor candidate
/// set and is never re-selected. A genuine error (not "already exists") stays
/// `Failed` and is retried next boot; we never fabricate an anchor.
pub(crate) fn decide_outcome(
    reauthor: &Result<(), StorageError>,
    recovery: Option<&Result<bool, StorageError>>,
) -> RowOutcome {
    match reauthor {
        Ok(()) => RowOutcome::Reanchored,
        Err(e) if is_already_anchored_error(e) => match recovery {
            Some(Ok(true)) => RowOutcome::AlreadyAnchored,
            _ => RowOutcome::Failed,
        },
        Err(_) => RowOutcome::Failed,
    }
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

    let candidates: Vec<(String, String, String)> = {
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

    for (id, reach, content_type) in &candidates {
        // Guard: a stored reach outside the DNA-notarized vocabulary can
        // never be re-authored (the content_store zome rejects it), so it
        // would fail every sweep forever and saturate the conductor. Skip
        // it loudly — one bad row must not wedge the heal loop. The fix is a
        // seed-data correction, caught by check-reach-drift.mjs.
        if !CORE_REACH_LEVELS.contains(&reach.as_str()) {
            report.record(RowOutcome::SkippedNonCanonicalReach);
            tracing::warn!(
                content_id = %id,
                reach = %reach,
                "reanchor_backfill: skipping row with non-canonical reach (not re-authorable)"
            );
            continue;
        }
        // Symmetric guard: a stored content_type outside the vocabulary
        // Content::validate() accepts (ALL_CONTENT_TYPES plus the
        // attestation:/governance-action: discriminator prefixes) can never
        // be re-authored either — create_content_unchecked's
        // prepare_content_for_storage rejects it every sweep. Skip it
        // loudly; the fix is a seed-data correction (e.g. the a2o
        // resilience seed steps' now-fixed 'album' → 'narrative' typo).
        if !is_canonical_content_type(content_type) {
            report.record(RowOutcome::SkippedNonCanonicalContentType);
            tracing::warn!(
                content_id = %id,
                content_type = %content_type,
                "reanchor_backfill: skipping row with non-canonical content_type (not re-authorable)"
            );
            continue;
        }
        // Empty patch → for a NULL-anchor row, update_via_conductor takes the
        // bootstrap branch: re-publishes the full entry from the existing SQL
        // row via create_content and projects dht_anchor_hash.
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
        let reauthor = content_service
            .update_via_conductor(hc, id, empty_patch)
            .await
            .map(|_| ());

        // IDEMPOTENCY FIX (adam alpha Cache-DB saturation, 2026-07-05): when the
        // re-author is refused because the entry is ALREADY committed to the
        // conductor's DHT view, "already anchored" IS the goal state. Recover the
        // committed ActionHash (get_content_by_id) and stamp it so this row
        // leaves the NULL-anchor candidate set forever. Previously this took the
        // `failed` branch and never stamped, so thousands of leaked e2e-<uuid>
        // rows were re-selected and re-thrashed on EVERY boot. Only pay the extra
        // conductor read for the already-exists class — genuine errors skip it
        // and retry next boot.
        let recovery = if matches!(&reauthor, Err(e) if is_already_anchored_error(e)) {
            Some(content_service.project_existing_anchor(hc, id).await)
        } else {
            None
        };

        let outcome = decide_outcome(&reauthor, recovery.as_ref());
        if outcome == RowOutcome::Failed {
            tracing::warn!(
                content_id = %id,
                reauthor_error = ?reauthor.as_ref().err().map(|e| e.to_string()),
                recovery_error = ?recovery
                    .as_ref()
                    .and_then(|r| r.as_ref().err())
                    .map(|e| e.to_string()),
                "reanchor_backfill: row not healed this sweep (non-fatal, retried next boot)"
            );
        }
        report.record(outcome);

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

    // `completed` = rows brought to the anchored state this sweep. Both fresh
    // re-authors AND already-anchored recoveries are healed rows, so both count
    // toward progress; `remaining` (recounted above) already excludes them.
    state
        .publish_reanchor_sweep(
            report.reanchored + report.already_anchored,
            report.failed,
            report.remaining,
        )
        .await;

    tracing::info!(
        reanchored = report.reanchored,
        already_anchored = report.already_anchored,
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
    fn core_reach_excludes_local_includes_community() {
        // Locks the guard contract to the canonical DNA vocabulary: 'local'
        // is NOT a content-visibility reach (it wedged reanchor_backfill),
        // and the remap target 'community' IS.
        assert!(!CORE_REACH_LEVELS.contains(&"local"));
        assert!(CORE_REACH_LEVELS.contains(&"community"));
    }

    #[test]
    fn content_type_guard_excludes_album_includes_narrative() {
        // Locks the guard contract to the vocabulary Content::validate()
        // actually accepts: 'album' is NOT a valid content_type (the a2o
        // resilience seed steps' bug that poisoned reanchor candidates with
        // an unre-authorable row), and the fix target 'narrative' IS —
        // it's in the extensible ALL_CONTENT_TYPES tier, not DNA-notarized
        // core, which is why this guard must check ALL_CONTENT_TYPES and
        // not the narrower CORE_CONTENT_TYPES.
        assert!(!is_canonical_content_type("album"));
        assert!(is_canonical_content_type("narrative"));
    }

    #[test]
    fn content_type_guard_admits_attestation_and_governance_action_prefixes() {
        // attestation:*/governance-action:* content_type values are enumerated
        // in generated_attestation_kinds, never in the content-type list
        // itself — the guard must not treat every such row as non-canonical.
        assert!(is_canonical_content_type("attestation:humanness"));
        assert!(is_canonical_content_type(
            "governance-action:recovery-request"
        ));
        // A near-miss prefix (no trailing colon) must NOT be admitted by the
        // prefix check alone — it still has to be a real vocabulary member.
        assert!(!is_canonical_content_type("attestation"));
    }

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
        assert_eq!(report.already_anchored, 0);
        assert_eq!(report.remaining, 0);
    }

    #[test]
    fn already_exists_error_is_classified_as_anchored_not_failed() {
        // The zome's create_content refuses a duplicate id with this exact Guest
        // error (content_store/src/lib.rs:2361); HcClient::call_zome wraps it as
        // StorageError::Conductor("Zome call failed: … already exists. Use
        // update_content …"). The reanchor sweep MUST read this as the
        // idempotent already-anchored state (recover + stamp), NOT as a
        // retryable failure — otherwise the SAME rows are re-selected and
        // re-thrashed every boot, saturating the conductor's Cache-DB read pool
        // (the adam alpha incident, 2026-07-05).
        let already = StorageError::Conductor(
            "Zome call failed: WasmError { error: Guest(\"Content with id \
             'e2e-2f1c' already exists. Use update_content to modify existing \
             entries.\") }"
                .to_string(),
        );
        assert!(
            is_already_anchored_error(&already),
            "'already exists … Use update_content' must be the idempotent \
             already-anchored case, not a failure"
        );

        // Genuine, retryable failures must NOT be swallowed as anchored.
        let stale = StorageError::Conductor(
            "Zome call failed: Guest(\"no Content entry found for id e2e-xyz\")".to_string(),
        );
        assert!(
            !is_already_anchored_error(&stale),
            "the stale-anchor / missing-entry class must stay a retryable failure"
        );

        let pool = StorageError::Internal("Pool error: connection timed out".to_string());
        assert!(!is_already_anchored_error(&pool));
    }

    #[test]
    fn already_exists_outcome_stamps_and_is_not_counted_failed() {
        // The bug: on 'already exists' the old code did `report.failed += 1` and
        // NEVER stamped the row, so it was re-selected forever. The fix routes it
        // to AlreadyAnchored *iff* the existing anchor was recovered (Ok(true)).
        let already: Result<(), StorageError> = Err(StorageError::Conductor(
            "Zome call failed: Guest(\"Content with id 'x' already exists. \
             Use update_content to modify existing entries.\")"
                .to_string(),
        ));

        // Recovered + stamped → AlreadyAnchored (NOT Failed). This is the row
        // leaving the NULL-anchor candidate set — the end of the thrash.
        assert_eq!(
            decide_outcome(&already, Some(&Ok(true))),
            RowOutcome::AlreadyAnchored
        );
        // 'already exists' but the entry could not be read back / recovery errored
        // → keep it a retryable failure; never fabricate an anchor.
        assert_eq!(
            decide_outcome(&already, Some(&Ok(false))),
            RowOutcome::Failed
        );
        assert_eq!(
            decide_outcome(
                &already,
                Some(&Err(StorageError::Conductor("read plane down".into())))
            ),
            RowOutcome::Failed
        );

        // A fresh re-author success is Reanchored; a genuine error is Failed.
        let ok: Result<(), StorageError> = Ok(());
        assert_eq!(decide_outcome(&ok, None), RowOutcome::Reanchored);
        let genuine: Result<(), StorageError> =
            Err(StorageError::Conductor("no Content entry found".into()));
        assert_eq!(decide_outcome(&genuine, None), RowOutcome::Failed);

        // Accounting: AlreadyAnchored increments its own counter and leaves
        // `failed` at zero — the load-bearing property of the fix. Both skip
        // classes (reach and content_type) fold into the same `skipped`
        // counter — symmetric non-re-authorable-vocabulary dispositions.
        let mut report = ReanchorReport::default();
        report.record(RowOutcome::AlreadyAnchored);
        report.record(RowOutcome::Reanchored);
        report.record(RowOutcome::SkippedNonCanonicalReach);
        report.record(RowOutcome::SkippedNonCanonicalContentType);
        assert_eq!(report.already_anchored, 1);
        assert_eq!(report.reanchored, 1);
        assert_eq!(report.skipped, 2);
        assert_eq!(
            report.failed, 0,
            "an already-anchored row must NEVER be counted as failed"
        );
    }
}
