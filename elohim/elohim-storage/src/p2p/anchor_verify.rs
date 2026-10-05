//! Anchor-verify pass — the liveness verdict for anchored rows nothing else
//! revisits.
//!
//! `dht_anchor_state = live` is earned from an own-conductor read. The paths
//! that take one (`upsert_with_anchor`, the adopt stamp, the heal leg's stamp)
//! only visit rows that are being written, adopted or healed. A row that is
//! anchored AND agrees with its peers is `InSync` to the heal leg and is never
//! asked about again — so a row adopted before the verdict was carried, or one
//! whose anchor a witnessless stamp moved (which now CLEARS the verdict), read
//! `unverified` forever.
//!
//! This pass asks. It runs at the end of the projection-reconcile heal leg —
//! so never without a conductor bridge and never while a heal leg is in flight
//! (it IS the tail of the single-flight leg) — and does one thing: for a
//! bounded page of anchored, unjudged rows, resolve their heads through the
//! existing batch resolver and mark `live` each row whose own-conductor
//! CANONICAL head is exactly the hash it is anchored to.
//!
//! It never moves, fills or refreshes a head (`confirm_anchor_live` writes the
//! verdict and nothing else), and a row the conductor answers differently for —
//! another head, a fallback, absent, unreachable — is left exactly as it is:
//! moves belong to the heal leg, `dead` to the ghost witness.
//!
//! bounded-work: at most [`batch_size`] ids per reconcile tick (env
//! `ANCHOR_VERIFY_BATCH`, default 50, clamped to [`MAX_BATCH`], 0 disables), in
//! ONE batch-extern call on the `Background` admission class under the
//! extern's own in-wasm budget; no retry — a failed or partial call waits for
//! the next tick. A keyset cursor walks the unverified set newest-first so a
//! row the conductor cannot confirm is asked once per lap, not once per tick.

use std::sync::Mutex;
use std::time::Duration;

use crate::db::{content_diesel, AppContext, DbPool};
use crate::services::conductor_writes::ContentHeadWire;
use crate::services::head_batch_resolver::HeadBatchResolver;

const DEFAULT_BATCH: usize = 50;
/// Ceiling on the operator knob — one batch call, sized well under the heal
/// leg's own slices.
const MAX_BATCH: usize = 200;

/// Pure parse of `ANCHOR_VERIFY_BATCH`. Unset or unparsable ⇒ the default.
pub fn batch_size_from(raw: Option<&str>) -> usize {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_BATCH)
        .min(MAX_BATCH)
}

/// Rows per pass. Read once — never on the tick path.
pub fn batch_size() -> usize {
    static SIZE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *SIZE.get_or_init(|| batch_size_from(std::env::var("ANCHOR_VERIFY_BATCH").ok().as_deref()))
}

/// THE DECISION, pure: does this own-conductor answer confirm the row's anchor?
/// Only a CANONICAL head naming exactly the anchored hash does.
pub fn confirms_anchor(anchor: &str, answer: Option<&ContentHeadWire>) -> bool {
    answer.is_some_and(|head| head.canonical && head.head_action_hash.as_str() == anchor)
}

/// `(updated_at, id)` of the last row the previous pass asked about. Disposable
/// local state: lost on restart, the walk simply starts over.
static CURSOR: Mutex<Option<(String, String)>> = Mutex::new(None);

/// What one pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VerifyReport {
    pub asked: usize,
    pub verified: usize,
    pub still_unverified: i64,
}

/// One pass over the process-wide cursor. See the module docs.
pub async fn run_pass(
    resolver: &dyn HeadBatchResolver,
    pool: &DbPool,
    budget: Duration,
) -> VerifyReport {
    let cursor = CURSOR.lock().ok().and_then(|c| c.clone());
    let (report, next) = run_pass_from(resolver, pool, budget, batch_size(), cursor).await;
    if let Ok(mut c) = CURSOR.lock() {
        *c = next;
    }
    report
}

/// [`run_pass`] with the batch size and cursor injected — the testable body.
/// Returns the report and the cursor for the next pass (`None` = start over).
pub async fn run_pass_from(
    resolver: &dyn HeadBatchResolver,
    pool: &DbPool,
    budget: Duration,
    batch: usize,
    cursor: Option<(String, String)>,
) -> (VerifyReport, Option<(String, String)>) {
    let mut report = VerifyReport::default();
    if batch == 0 {
        return (report, cursor);
    }
    let ctx = AppContext::default_lamad();
    let rows = {
        let Ok(mut conn) = pool.get() else {
            return (report, cursor);
        };
        let after = cursor.as_ref().map(|(u, i)| (u.as_str(), i.as_str()));
        match content_diesel::list_unverified_anchored(&mut conn, &ctx, after, batch as i64) {
            Ok(rows) => rows,
            Err(error) => {
                tracing::warn!(%error, "anchor-verify: unverified-row read failed; skipping");
                return (report, cursor);
            }
        }
    };
    // A short page is the end of the lap: start over next time.
    let next = if rows.len() < batch {
        None
    } else {
        rows.last().map(|r| (r.updated_at.clone(), r.id.clone()))
    };
    let ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
    report.asked = ids.len();
    let resolution = match resolver.resolve_heads(&ids, budget).await {
        Ok(r) => r,
        Err(error) => {
            // Nothing was learned: keep the cursor so these rows are asked again.
            tracing::debug!(%error, "anchor-verify: batch resolve failed; retried next tick");
            return (report, cursor);
        }
    };
    if let Ok(mut conn) = pool.get() {
        for item in &resolution.items {
            let Some(row) = rows.iter().find(|r| r.id == item.id) else {
                continue;
            };
            let head = match &item.answer {
                seam_contracts::Answer::Present(head) => Some(head),
                _ => None,
            };
            if confirms_anchor(&row.dht_anchor_hash, head)
                && content_diesel::confirm_anchor_live(
                    &mut conn,
                    &ctx,
                    &row.id,
                    &row.dht_anchor_hash,
                )
                .unwrap_or(false)
            {
                report.verified += 1;
            }
        }
        report.still_unverified =
            content_diesel::count_unverified_anchored(&mut conn, &ctx).unwrap_or(-1);
        if report.still_unverified >= 0 {
            crate::metrics::record_anchor_verify_pass(
                report.verified as u64,
                report.still_unverified,
            );
        }
    }
    if report.verified > 0 {
        tracing::info!(
            target: "elohim_storage::projection_reconcile",
            asked = report.asked,
            verified = report.verified,
            still_unverified = report.still_unverified,
            "anchor-verify: the own conductor confirmed anchored rows — marked live"
        );
    }
    (report, next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::content_diesel::{AnchorState, StampMode};
    use crate::services::head_batch_resolver::{MockAnswer, MockHeadBatchResolver};

    fn head(id: &str, hash: &str, canonical: bool) -> ContentHeadWire {
        serde_json::from_value(serde_json::json!({
            "content_id": id,
            "head_action_hash": hash,
            "declared_at": 1_700_000_000_000_000i64,
            "canonical": canonical,
            "content": {
                "id": id, "content_type": "concept", "title": "t", "description": "d",
                "content_format": "markdown", "reach": "commons",
            },
        }))
        .expect("ContentHeadWire fixture must deserialize")
    }

    fn seed(pool: &DbPool, id: &str, anchor: &str) {
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        content_diesel::bulk_create_content(
            &mut conn,
            &ctx,
            vec![content_diesel::CreateContentInput {
                id: id.into(),
                title: id.into(),
                description: None,
                content_type: "concept".into(),
                content_format: "markdown".into(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "commons".into(),
                created_by: None,
                tags: vec![],
                content_body: None,
                dht_anchor_hash: None,
            }],
        )
        .unwrap();
        // A witnessless stamp: anchored, unverified — the standing fleet state.
        content_diesel::stamp_declared_head_mode(
            &mut conn,
            &ctx,
            id,
            anchor,
            None,
            None,
            StampMode::HealCanonical,
            None,
        )
        .unwrap();
    }

    fn state(pool: &DbPool, id: &str) -> (AnchorState, Option<String>) {
        let mut conn = pool.get().unwrap();
        let row = content_diesel::get_content(
            &mut conn,
            &AppContext::default_lamad(),
            id,
            content_diesel::MinTrust::Invisible,
        )
        .unwrap()
        .unwrap();
        (
            AnchorState::from_column(row.dht_anchor_state.as_deref()),
            row.dht_anchor_hash,
        )
    }

    #[test]
    fn only_a_canonical_answer_for_the_exact_anchor_confirms_it() {
        assert!(confirms_anchor(
            "uhCkk-A",
            Some(&head("x", "uhCkk-A", true))
        ));
        assert!(!confirms_anchor(
            "uhCkk-A",
            Some(&head("x", "uhCkk-B", true))
        ));
        assert!(!confirms_anchor(
            "uhCkk-A",
            Some(&head("x", "uhCkk-A", false))
        ));
        assert!(!confirms_anchor("uhCkk-A", None));
    }

    #[test]
    fn the_batch_knob_defaults_clamps_and_disables() {
        assert_eq!(batch_size_from(None), 50);
        assert_eq!(batch_size_from(Some("nonsense")), 50);
        assert_eq!(batch_size_from(Some("0")), 0);
        assert_eq!(batch_size_from(Some(" 7 ")), 7);
        assert_eq!(batch_size_from(Some("100000")), MAX_BATCH);
    }

    /// The pass marks exactly the rows the own conductor confirms, leaves every
    /// other row — and every head — alone, and asks nothing when disabled.
    #[tokio::test]
    async fn the_pass_marks_confirmed_rows_live_and_leaves_the_rest_alone() {
        let pool = crate::test_util::test_pool();
        seed(&pool, "confirmed", "uhCkk-confirmed");
        seed(&pool, "moved-on", "uhCkk-old");
        seed(&pool, "fallback", "uhCkk-fallback");
        seed(&pool, "absent", "uhCkk-absent");
        let resolver = MockHeadBatchResolver::new();
        resolver.seed_head(
            "confirmed",
            MockAnswer::Present(head("confirmed", "uhCkk-confirmed", true)),
        );
        resolver.seed_head(
            "moved-on",
            MockAnswer::Present(head("moved-on", "uhCkk-new", true)),
        );
        resolver.seed_head(
            "fallback",
            MockAnswer::Present(head("fallback", "uhCkk-fallback", false)),
        );

        // Disabled: no conductor work at all.
        let (off, _) = run_pass_from(&resolver, &pool, Duration::from_secs(1), 0, None).await;
        assert_eq!(off, VerifyReport::default());
        assert_eq!(resolver.calls(), 0);

        let (report, next) =
            run_pass_from(&resolver, &pool, Duration::from_secs(1), 20, None).await;
        assert_eq!(resolver.calls(), 1, "one batch call per pass");
        assert_eq!(report.asked, 4);
        assert_eq!(report.verified, 1);
        assert_eq!(report.still_unverified, 3);
        assert_eq!(next, None, "a short page ends the lap");

        assert_eq!(state(&pool, "confirmed").0, AnchorState::Live);
        for (id, anchor) in [
            ("moved-on", "uhCkk-old"),
            ("fallback", "uhCkk-fallback"),
            ("absent", "uhCkk-absent"),
        ] {
            let (verdict, hash) = state(&pool, id);
            assert_eq!(verdict, AnchorState::Unverified, "{id}");
            assert_eq!(
                hash.as_deref(),
                Some(anchor),
                "{id}: the pass never moves a head"
            );
        }
    }

    /// The cursor walks the set: unconfirmable rows are not re-asked every
    /// tick, and a failed call keeps its place.
    #[tokio::test]
    async fn the_cursor_walks_the_unverified_set_and_holds_on_a_failed_call() {
        let pool = crate::test_util::test_pool();
        for id in ["r1", "r2", "r3"] {
            seed(&pool, id, &format!("uhCkk-{id}"));
        }
        let resolver = MockHeadBatchResolver::new(); // every id answers Absent
        let budget = Duration::from_secs(1);

        let (first, c1) = run_pass_from(&resolver, &pool, budget, 2, None).await;
        assert_eq!(first.asked, 2);
        assert!(c1.is_some());
        let (second, c2) = run_pass_from(&resolver, &pool, budget, 2, c1.clone()).await;
        assert_eq!(second.asked, 1, "the lap continues past the first page");
        assert_eq!(c2, None);
        assert_eq!(resolver.ids_asked(), 3, "each row asked once per lap");

        resolver.fail_next_call(crate::error::StorageError::Internal(
            "conductor down".into(),
        ));
        let (failed, held) = run_pass_from(&resolver, &pool, budget, 2, c1.clone()).await;
        assert_eq!(failed.verified, 0);
        assert_eq!(held, c1, "a call that learned nothing does not advance");
    }
}
