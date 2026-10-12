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

/// How long a row the own conductor DEFINITELY did not confirm is left alone
/// before it is asked about again.
///
/// The keyset cursor above was meant to make an unconfirmable row cost one
/// question a lap, but a set smaller than the batch is one page, so the lap is
/// one tick: 18 such rows on a household peer were re-asked on every reconcile
/// sweep, forever, and answered the same. The first window is the heal leg's
/// own missing-content back-off; it doubles per consecutive non-confirming
/// answer to the miss ledger's FIRST dormancy rung and stops there (production
/// 10 min → 1 h; the household's scaled values 1 min → 6 min). The ceiling is
/// deliberately the low one: a row becomes confirmable when gossip brings its
/// head to this conductor, which no local event announces, so the wait is the
/// longest a `live` verdict can lag the fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnconfirmedLadder {
    pub base: Duration,
    pub cap: Duration,
}

impl UnconfirmedLadder {
    pub fn from_config() -> Self {
        let base = crate::config::heal_missing_backoff_window();
        Self {
            base,
            cap: crate::config::miss_dormancy_base_window().max(base),
        }
    }

    /// The window after the `strikes`-th consecutive non-confirming answer.
    pub fn window(self, strikes: u32) -> Duration {
        let shift = strikes.saturating_sub(1).min(16);
        self.base.saturating_mul(1u32 << shift).min(self.cap)
    }
}

/// Most rows remembered at once; see [`UnconfirmedAnchors::note_unconfirmed`].
const MAX_REMEMBERED: usize = 4096;

#[derive(Debug, Clone)]
struct Unconfirmed {
    anchor: String,
    strikes: u32,
    asked_at: std::time::Instant,
}

/// Rows whose anchor the own conductor has definitely not confirmed, each
/// against the anchor it was asked about. Skip-only: it never writes a verdict
/// and never moves a head. A row whose anchor changed is a different question
/// and is asked at once. Disposable, like the cursor: a restart asks again.
#[derive(Debug, Default)]
pub struct UnconfirmedAnchors {
    rows: Mutex<std::collections::HashMap<String, Unconfirmed>>,
}

impl UnconfirmedAnchors {
    /// Is `id` resting on a non-confirming answer for exactly `anchor`?
    pub fn is_resting(
        &self,
        id: &str,
        anchor: &str,
        ladder: UnconfirmedLadder,
        now: std::time::Instant,
    ) -> bool {
        self.rows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .is_some_and(|u| {
                u.anchor == anchor
                    && now.saturating_duration_since(u.asked_at) < ladder.window(u.strikes)
            })
    }

    /// The conductor answered for `id` and the answer did not confirm `anchor`.
    pub fn note_unconfirmed(&self, id: &str, anchor: &str, now: std::time::Instant) {
        let mut rows = self.rows.lock().unwrap_or_else(|e| e.into_inner());
        // A hard ceiling for a set too large to ever be one page. Forgetting
        // is always safe here: it costs one more question, never a verdict.
        if rows.len() >= MAX_REMEMBERED && !rows.contains_key(id) {
            rows.clear();
        }
        let strikes = match rows.get(id) {
            Some(u) if u.anchor == anchor => u.strikes.saturating_add(1),
            _ => 1,
        };
        rows.insert(
            id.to_string(),
            Unconfirmed {
                anchor: anchor.to_string(),
                strikes,
                asked_at: now,
            },
        );
    }

    /// Keep only the rows in `live`. Called when one page WAS the whole
    /// unverified set, so anything remembered and not on it has left the set
    /// (deleted, or judged by another path) and would otherwise be held for the
    /// life of the process.
    pub fn retain_only(&self, live: &std::collections::HashSet<&str>) {
        self.rows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|id, _| live.contains(id.as_str()));
    }

    /// `id` was confirmed, or is no longer in the unverified set.
    pub fn forget(&self, id: &str) {
        self.rows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
    }

    pub fn len(&self) -> usize {
        self.rows.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn unconfirmed_anchors() -> &'static UnconfirmedAnchors {
    static MEMORY: std::sync::OnceLock<UnconfirmedAnchors> = std::sync::OnceLock::new();
    MEMORY.get_or_init(UnconfirmedAnchors::default)
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
    /// Rows on the page not asked about: resting on a remembered answer.
    pub resting: usize,
}

/// One pass over the process-wide cursor. See the module docs.
pub async fn run_pass(
    resolver: &dyn HeadBatchResolver,
    pool: &DbPool,
    budget: Duration,
) -> VerifyReport {
    let cursor = CURSOR.lock().ok().and_then(|c| c.clone());
    let (report, next) = run_pass_with(
        resolver,
        pool,
        budget,
        batch_size(),
        cursor,
        unconfirmed_anchors(),
        UnconfirmedLadder::from_config(),
        std::time::Instant::now(),
    )
    .await;
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
    // No memory: every row on the page is asked, as the cursor tests expect.
    run_pass_with(
        resolver,
        pool,
        budget,
        batch,
        cursor,
        &UnconfirmedAnchors::default(),
        UnconfirmedLadder {
            base: Duration::ZERO,
            cap: Duration::ZERO,
        },
        std::time::Instant::now(),
    )
    .await
}

/// The pass itself, with its memory, ladder and clock injected.
#[allow(clippy::too_many_arguments)]
pub async fn run_pass_with(
    resolver: &dyn HeadBatchResolver,
    pool: &DbPool,
    budget: Duration,
    batch: usize,
    cursor: Option<(String, String)>,
    memory: &UnconfirmedAnchors,
    ladder: UnconfirmedLadder,
    now: std::time::Instant,
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
    if cursor.is_none() && next.is_none() {
        // This page is the whole unverified set.
        memory.retain_only(&rows.iter().map(|r| r.id.as_str()).collect());
    }
    // Rows resting on a definite non-confirming answer for the anchor they
    // still carry are not asked again until their window lapses.
    let ids: Vec<String> = rows
        .iter()
        .filter(|r| !memory.is_resting(&r.id, &r.dht_anchor_hash, ladder, now))
        .map(|r| r.id.clone())
        .collect();
    report.asked = ids.len();
    report.resting = rows.len() - ids.len();
    if ids.is_empty() {
        // Nothing to learn this pass: no conductor call. The page was still
        // walked, so the cursor advances exactly as if it had been asked.
        return (report, next);
    }
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
            if confirms_anchor(&row.dht_anchor_hash, head) {
                memory.forget(&row.id);
                if content_diesel::confirm_anchor_live(
                    &mut conn,
                    &ctx,
                    &row.id,
                    &row.dht_anchor_hash,
                )
                .unwrap_or(false)
                {
                    report.verified += 1;
                }
            } else if !matches!(item.answer, seam_contracts::Answer::Unreachable) {
                // A DEFINITE answer that does not confirm: another head, a
                // non-canonical one, or an observed absence. `Unreachable` is
                // no answer at all and is asked again next pass.
                memory.note_unconfirmed(&row.id, &row.dht_anchor_hash, now);
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

    #[test]
    fn the_ladder_doubles_from_its_base_to_its_cap() {
        let ladder = UnconfirmedLadder {
            base: Duration::from_secs(600),
            cap: Duration::from_secs(3600),
        };
        let secs: Vec<u64> = (1..=6).map(|n| ladder.window(n).as_secs()).collect();
        assert_eq!(secs, vec![600, 1200, 2400, 3600, 3600, 3600]);
        assert_eq!(ladder.window(u32::MAX), Duration::from_secs(3600));
    }

    /// A set smaller than the batch is one page, so without memory every
    /// unconfirmable row was asked on every pass. With it: asked once, then
    /// left alone until the window lapses, and asked at once if its anchor
    /// changes or the conductor never actually answered.
    #[tokio::test]
    async fn an_unconfirmable_row_rests_until_its_window_lapses_or_its_anchor_moves() {
        let pool = crate::test_util::test_pool();
        seed(&pool, "absent", "uhCkk-absent");
        seed(&pool, "moved-on", "uhCkk-old");
        seed(&pool, "unreached", "uhCkk-unreached");
        let resolver = MockHeadBatchResolver::new();
        resolver.seed_head(
            "moved-on",
            MockAnswer::Present(head("moved-on", "uhCkk-new", true)),
        );
        resolver.seed_head("unreached", MockAnswer::Unattempted);
        let memory = UnconfirmedAnchors::default();
        let ladder = UnconfirmedLadder {
            base: Duration::from_secs(600),
            cap: Duration::from_secs(3600),
        };
        let budget = Duration::from_secs(1);
        let t0 = std::time::Instant::now();
        let pass = |at: std::time::Instant| {
            run_pass_with(&resolver, &pool, budget, 20, None, &memory, ladder, at)
        };

        let (first, _) = pass(t0).await;
        assert_eq!((first.asked, first.resting), (3, 0));
        assert_eq!(
            memory.len(),
            2,
            "two definite answers; the unreached id is not one"
        );

        // Next sweep: only the row the conductor never answered for is asked.
        let (second, _) = pass(t0 + Duration::from_secs(30)).await;
        assert_eq!((second.asked, second.resting), (1, 2));

        // The first window lapses: both are asked again, and strike twice.
        let (third, _) = pass(t0 + Duration::from_secs(600)).await;
        assert_eq!((third.asked, third.resting), (3, 0));
        // 600 s after THAT ask they are still inside the doubled window.
        let (fourth, _) = pass(t0 + Duration::from_secs(1200)).await;
        assert_eq!((fourth.asked, fourth.resting), (1, 2));

        // The memory is about ONE anchor. The same row carrying a different
        // anchor is a different question and is not resting; and once the
        // conductor's answer does confirm, the row is marked and forgotten.
        let at = t0 + Duration::from_secs(1230);
        assert!(memory.is_resting("moved-on", "uhCkk-old", ladder, at));
        assert!(!memory.is_resting("moved-on", "uhCkk-new", ladder, at));
        resolver.seed_head(
            "absent",
            MockAnswer::Present(head("absent", "uhCkk-absent", true)),
        );
        // Still resting on the old answer until its window lapses...
        let (fifth, _) = pass(at).await;
        assert_eq!(fifth.verified, 0);
        // ...then asked, confirmed, marked live and forgotten.
        let (sixth, _) = pass(t0 + Duration::from_secs(600 + 1200)).await;
        assert_eq!(sixth.verified, 1);
        assert_eq!(state(&pool, "absent").0, AnchorState::Live);
        assert!(!memory.is_resting("absent", "uhCkk-absent", ladder, at));
    }

    /// A remembered row that leaves the unverified set by ANOTHER path (deleted,
    /// or marked by the adopt stamp) must not be held for the life of the
    /// process: when a page is the whole set, anything not on it is dropped.
    #[tokio::test]
    async fn a_row_that_left_the_set_by_another_path_is_forgotten() {
        let pool = crate::test_util::test_pool();
        seed(&pool, "stays", "uhCkk-stays");
        seed(&pool, "leaves", "uhCkk-leaves");
        let resolver = MockHeadBatchResolver::new(); // Absent for both
        let memory = UnconfirmedAnchors::default();
        let ladder = UnconfirmedLadder {
            base: Duration::from_secs(600),
            cap: Duration::from_secs(3600),
        };
        let budget = Duration::from_secs(1);
        let t0 = std::time::Instant::now();
        run_pass_with(&resolver, &pool, budget, 20, None, &memory, ladder, t0).await;
        assert_eq!(memory.len(), 2);
        {
            let mut conn = pool.get().unwrap();
            assert!(content_diesel::confirm_anchor_live(
                &mut conn,
                &AppContext::default_lamad(),
                "leaves",
                "uhCkk-leaves",
            )
            .unwrap());
        }
        run_pass_with(&resolver, &pool, budget, 20, None, &memory, ladder, t0).await;
        assert_eq!(memory.len(), 1);
        assert!(memory.is_resting("stays", "uhCkk-stays", ladder, t0));
    }

    #[test]
    fn the_memory_is_bounded() {
        let memory = UnconfirmedAnchors::default();
        let now = std::time::Instant::now();
        for n in 0..(MAX_REMEMBERED + 10) {
            memory.note_unconfirmed(&format!("row-{n}"), "uhCkk", now);
        }
        assert!(memory.len() <= MAX_REMEMBERED);
    }

    /// When every row on the page is resting there is nothing to learn, and the
    /// pass makes no conductor call at all.
    #[tokio::test]
    async fn a_page_of_resting_rows_costs_no_conductor_call() {
        let pool = crate::test_util::test_pool();
        for id in ["r1", "r2"] {
            seed(&pool, id, &format!("uhCkk-{id}"));
        }
        let resolver = MockHeadBatchResolver::new(); // every id answers Absent
        let memory = UnconfirmedAnchors::default();
        let ladder = UnconfirmedLadder {
            base: Duration::from_secs(600),
            cap: Duration::from_secs(3600),
        };
        let budget = Duration::from_secs(1);
        let t0 = std::time::Instant::now();
        run_pass_with(&resolver, &pool, budget, 20, None, &memory, ladder, t0).await;
        assert_eq!(resolver.calls(), 1);
        for sweep in 1..=19u64 {
            let at = t0 + Duration::from_secs(30 * sweep);
            let (report, _) =
                run_pass_with(&resolver, &pool, budget, 20, None, &memory, ladder, at).await;
            assert_eq!((report.asked, report.resting), (0, 2));
        }
        assert_eq!(
            resolver.calls(),
            1,
            "nineteen sweeps inside the window, no call"
        );
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
