//! LIVE EARNED — `GET /db/content/{id}/head?election=live` (F26).
//!
//! `ContentHeadView.earned` normally reads the projection's `canonical_earned`
//! column. That column is written only when a stamp CARRIES an election
//! (`content_diesel::stamp_declared_head_mode` with `Some(canonical_ordering)`),
//! so a peer that adopted a steward's head through an unordered channel keeps it
//! at NULL/0 forever — observed on the household: jessica and james answered
//! `earned: false` while matthew answered `true` for the same declared head, and
//! a seeder asking the wrong peer overwrote a stewarded atom.
//!
//! The live read asks THIS peer's own conductor for its local canonical election
//! (`content_store::resolve_canonical_election`) and answers from it.
//!
//! ## Bounded before the call
//!
//! `HcClient::call_zome` is uncancellable (`src/.epr-meta`
//! `conductor-call-is-uncancellable`). The work is bounded on the wasm side, not
//! by the caller's timeout: `resolve_canonical_election` runs
//! `select_election(id, GetStrategy::Local)` — one id's LOCAL canonical-head
//! links, never a network `get_links`, never a Content record decode. The
//! caller's deadline bounds only how long the HTTP read waits for it; on expiry
//! the read answers from the column and says so (`earnedSource: "cached"`).
//!
//! And it adds no conductor call on the common path: an unbound `/head` read
//! already asks this very election for its staging candidate, so the live read
//! reuses that ONE answer for both (see `HttpServer::handle_content_head`). Only
//! a slug bound to a release channel — whose candidate comes from the CHANNEL's
//! election — pays one extra ask, for the slug's own election.
//!
//! ## The answer
//!
//! `earned` = the live winner is EARNED and IS the row's declared head — OR the
//! projection already recorded an earned tier for this row. The second clause is
//! C2 monotonic authority, not a fudge: a conductor's LOCAL link view can lack
//! the earned declaration (restart, un-integrated link) and then elects the
//! staging declaration for the same head or none at all; it can never un-earn
//! one. A live answer may therefore RAISE `earned`, never lower it — the same
//! verdict `canonical_move_verdict` gives the heal plane (tier first).
//!
//! ## The heal
//!
//! When the live answer proves an earned tier the column does not record, the
//! same request records it (`content_diesel::heal_election_columns`): the
//! election columns only, compare-and-set on the head the read saw — never the
//! head, the anchor, the body or `updated_at`. Heal fills; it never moves.

use std::future::Future;

use elohim_views::EarnedSource;

use crate::db::content_diesel::{self, CanonicalOrdering, ElectionColumnHeal};
use crate::db::AppContext;
use crate::error::StorageError;
use crate::services::conductor_writes::CanonicalElectionWire;

/// How long a `/head` read waits for its conductor's local election — the same
/// 2 s the staging-candidate read has always had, because on the unbound path it
/// IS the same ask.
pub const LIVE_ELECTION_BUDGET: std::time::Duration = std::time::Duration::from_secs(2);

/// Why the conductor did not answer. Countable labels (C8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectionUnavailable {
    /// No lamad conductor client on this peer.
    NoClient,
    /// The ask returned an error.
    Error,
    /// The read's deadline elapsed first.
    Deadline,
}

impl ElectionUnavailable {
    pub fn label(self) -> &'static str {
        match self {
            ElectionUnavailable::NoClient => "no_client",
            ElectionUnavailable::Error => "error",
            ElectionUnavailable::Deadline => "deadline",
        }
    }
}

/// This peer's conductor's answer to "what did you elect for this id?".
#[derive(Debug, Clone)]
pub enum LocalElection {
    /// The conductor answered. `None` = no canonical-head declaration in its
    /// LOCAL view — an answer, but never authoritative absence.
    Answered(Option<CanonicalElectionWire>),
    /// The conductor did not answer inside the read's budget.
    Unavailable(ElectionUnavailable),
}

/// Put ONE election ask, waiting no longer than `deadline`.
///
/// Generic over the ask so the budget/timeout arms are provable without a
/// conductor (the `resolve_exact_candidate_blob_with` pattern).
pub async fn ask_local_election_with<F>(ask: F, deadline: tokio::time::Instant) -> LocalElection
where
    F: Future<Output = Result<Option<CanonicalElectionWire>, StorageError>>,
{
    match tokio::time::timeout_at(deadline, ask).await {
        Ok(Ok(election)) => LocalElection::Answered(election),
        Ok(Err(_)) => LocalElection::Unavailable(ElectionUnavailable::Error),
        Err(_) => LocalElection::Unavailable(ElectionUnavailable::Deadline),
    }
}

/// The live verdict for one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveEarned {
    pub earned: bool,
    pub source: EarnedSource,
    /// `Some` when the live answer proves an earned tier the column does not
    /// record — the election to record via [`heal_behind_column`].
    pub heal: Option<CanonicalOrdering>,
}

/// Pure and total: the live `earned` for a row whose declared head is
/// `projected_head` and whose column reads `column_earned`.
pub fn live_earned_verdict(
    projected_head: &str,
    column_earned: bool,
    live: &LocalElection,
) -> LiveEarned {
    match live {
        LocalElection::Unavailable(_) => LiveEarned {
            earned: column_earned,
            source: EarnedSource::Cached,
            heal: None,
        },
        LocalElection::Answered(election) => {
            let proves_earned = election.as_ref().filter(|election| {
                election.canonical_earned && election.winner_target.to_string() == projected_head
            });
            LiveEarned {
                earned: column_earned || proves_earned.is_some(),
                source: EarnedSource::Live,
                heal: proves_earned
                    .filter(|_| !column_earned)
                    .map(CanonicalElectionWire::ordering),
            }
        }
    }
}

/// Record a live-proven earned election on the row that is behind it. `None`
/// when the verdict carries nothing to heal.
pub fn heal_behind_column(
    conn: &mut diesel::SqliteConnection,
    ctx: &AppContext,
    id: &str,
    projected_head: &str,
    verdict: &LiveEarned,
) -> Result<Option<ElectionColumnHeal>, StorageError> {
    let Some(ordering) = verdict.heal else {
        return Ok(None);
    };
    content_diesel::heal_election_columns(conn, ctx, id, projected_head, ordering).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::content_diesel::{create_content, CreateContentInput};
    use crate::db::diesel_schema::content;
    use diesel::prelude::*;

    const HEAD: &str = "uhCkkc4f18wPJTfxqSdzRUdXJjLD5DRRn7YbF_yHh0xyRENnbCK-A";
    const OTHER: &str = "uhCkkp1ebd48niDLCWjV2aOoa2AWCtLSxjcjDFQRwZ9oMtjHS2q0w";

    fn election(winner: &str, earned: bool) -> CanonicalElectionWire {
        serde_json::from_value(serde_json::json!({
            "winner_target": winner,
            "canonical_declared_at": 42,
            "canonical_earned": earned,
        }))
        .expect("canonical election wire")
    }

    fn answered(winner: &str, earned: bool) -> LocalElection {
        LocalElection::Answered(Some(election(winner, earned)))
    }

    /// A row declared at `HEAD` whose column records `earned` (None = no
    /// election recorded at all — the unordered-adoption shape F26 names).
    fn row(conn: &mut SqliteConnection, ctx: &AppContext, id: &str, earned: Option<i32>) {
        create_content(
            conn,
            ctx,
            CreateContentInput {
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
                dht_anchor_hash: Some(HEAD.into()),
            },
        )
        .unwrap();
        diesel::update(content::table.filter(content::id.eq(id)))
            .set((
                content::declared_head_action_hash.eq(Some(HEAD)),
                content::canonical_declared_at.eq(earned.map(|_| 7_i64)),
                content::canonical_earned.eq(earned),
            ))
            .execute(conn)
            .unwrap();
    }

    fn columns(
        conn: &mut SqliteConnection,
        id: &str,
    ) -> (Option<String>, Option<i64>, Option<i32>, String) {
        content::table
            .filter(content::id.eq(id))
            .select((
                content::declared_head_action_hash,
                content::canonical_declared_at,
                content::canonical_earned,
                content::updated_at,
            ))
            .first(conn)
            .unwrap()
    }

    #[test]
    fn live_earned_winner_on_the_declared_head_is_earned_and_heals_a_behind_column() {
        let verdict = live_earned_verdict(HEAD, false, &answered(HEAD, true));
        assert!(verdict.earned);
        assert_eq!(verdict.source, EarnedSource::Live);
        assert_eq!(
            verdict.heal,
            Some(election(HEAD, true).ordering()),
            "the column is behind the live answer — carry the election to record"
        );
    }

    #[test]
    fn live_answer_is_false_unless_the_earned_winner_is_this_head() {
        for live in [
            answered(HEAD, false),
            answered(OTHER, true),
            LocalElection::Answered(None),
        ] {
            let verdict = live_earned_verdict(HEAD, false, &live);
            assert!(!verdict.earned, "{live:?}");
            assert_eq!(verdict.source, EarnedSource::Live);
            assert_eq!(verdict.heal, None, "{live:?}");
        }
    }

    #[test]
    fn a_local_link_view_never_lowers_a_recorded_earned_tier() {
        for live in [
            answered(HEAD, false),
            answered(OTHER, true),
            LocalElection::Answered(None),
            answered(HEAD, true),
        ] {
            let verdict = live_earned_verdict(HEAD, true, &live);
            assert!(verdict.earned, "{live:?}");
            assert_eq!(verdict.source, EarnedSource::Live);
            assert_eq!(
                verdict.heal, None,
                "column already records earned: {live:?}"
            );
        }
    }

    #[test]
    fn unavailable_conductor_falls_back_to_the_column_and_says_so() {
        for (column, reason) in [
            (false, ElectionUnavailable::NoClient),
            (true, ElectionUnavailable::Error),
            (false, ElectionUnavailable::Deadline),
        ] {
            let verdict = live_earned_verdict(HEAD, column, &LocalElection::Unavailable(reason));
            assert_eq!(verdict.earned, column);
            assert_eq!(verdict.source, EarnedSource::Cached);
            assert_eq!(verdict.heal, None);
        }
    }

    #[tokio::test]
    async fn the_ask_is_bounded_by_the_read_deadline() {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(20);
        let slow = async {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            Ok(Some(election(HEAD, true)))
        };
        let started = std::time::Instant::now();
        let live = ask_local_election_with(slow, deadline).await;
        assert!(matches!(
            live,
            LocalElection::Unavailable(ElectionUnavailable::Deadline)
        ));
        assert!(started.elapsed() < std::time::Duration::from_secs(2));

        let deadline = tokio::time::Instant::now() + LIVE_ELECTION_BUDGET;
        let failing = async { Err(StorageError::Internal("conductor gone".into())) };
        assert!(matches!(
            ask_local_election_with(failing, deadline).await,
            LocalElection::Unavailable(ElectionUnavailable::Error)
        ));

        let answering = async { Ok(Some(election(HEAD, true))) };
        let live = ask_local_election_with(answering, deadline).await;
        assert!(live_earned_verdict(HEAD, false, &live).earned);
    }

    #[test]
    fn the_heal_records_the_election_on_a_behind_row_and_nothing_else() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::new("lamad");
        row(&mut conn, &ctx, "behind", None);
        let before = columns(&mut conn, "behind");

        let verdict = live_earned_verdict(HEAD, false, &answered(HEAD, true));
        let healed = heal_behind_column(&mut conn, &ctx, "behind", HEAD, &verdict).unwrap();
        assert_eq!(healed, Some(ElectionColumnHeal::Healed));

        let after = columns(&mut conn, "behind");
        assert_eq!(after.0.as_deref(), Some(HEAD), "the head never moves");
        assert_eq!(after.1, Some(42));
        assert_eq!(after.2, Some(1));
        assert_eq!(after.3, before.3, "updated_at is not a heal's to touch");

        // The next plain read now tells the truth from the column alone.
        let cwt = content_diesel::get_content_with_tags(
            &mut conn,
            &ctx,
            "behind",
            content_diesel::MinTrust::Invisible,
        )
        .unwrap()
        .unwrap();
        assert!(
            crate::views::content_head_view_from_content(&cwt.content)
                .unwrap()
                .earned
        );

        // Idempotent: a second live read finds nothing behind.
        let again = live_earned_verdict(HEAD, true, &answered(HEAD, true));
        assert_eq!(
            heal_behind_column(&mut conn, &ctx, "behind", HEAD, &again).unwrap(),
            None
        );
    }

    #[test]
    fn the_heal_upgrades_a_recorded_staging_tier_but_never_lowers_an_earned_one() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::new("lamad");

        row(&mut conn, &ctx, "staged", Some(0));
        assert_eq!(
            content_diesel::heal_election_columns(
                &mut conn,
                &ctx,
                "staged",
                HEAD,
                election(HEAD, true).ordering()
            )
            .unwrap(),
            ElectionColumnHeal::Healed
        );
        assert_eq!(columns(&mut conn, "staged").2, Some(1));

        row(&mut conn, &ctx, "earned", Some(1));
        assert_eq!(
            content_diesel::heal_election_columns(
                &mut conn,
                &ctx,
                "earned",
                HEAD,
                election(HEAD, false).ordering()
            )
            .unwrap(),
            ElectionColumnHeal::Unchanged
        );
        assert_eq!(columns(&mut conn, "earned").2, Some(1));
    }

    #[test]
    fn the_heal_is_compare_and_set_on_the_head_the_read_saw() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::new("lamad");
        row(&mut conn, &ctx, "moved", None);

        assert_eq!(
            content_diesel::heal_election_columns(
                &mut conn,
                &ctx,
                "moved",
                OTHER,
                election(OTHER, true).ordering()
            )
            .unwrap(),
            ElectionColumnHeal::HeadMismatch
        );
        let after = columns(&mut conn, "moved");
        assert_eq!(after.0.as_deref(), Some(HEAD));
        assert_eq!(
            after.2, None,
            "nothing written for a head the row does not carry"
        );

        assert_eq!(
            content_diesel::heal_election_columns(
                &mut conn,
                &ctx,
                "absent",
                HEAD,
                election(HEAD, true).ordering()
            )
            .unwrap(),
            ElectionColumnHeal::NoRow
        );
    }
}
