//! Event-driven ROW delivery — the two seams that stop a declared head from
//! waiting on a timer at a peer that did not declare it.
//!
//! # The measured hole (fleet, 2026-10-05)
//!
//! Peer A declares a canonical content head and announces the content doc at
//! once. On a peer B that holds NO row for that id, three things each waited on
//! a clock:
//!
//! 1. the reverse projection skips an absent row ("the shard/replication
//!    plane's job"), and that plane only learns the id from the next 60 s
//!    inventory walk;
//! 2. the head-adoption trigger the announce raised ends
//!    `TriggerAction::NoLocalRow` — terminal, on purpose — and nothing re-offers
//!    the id when the row does land, so the head waits for the 300 s
//!    projection-reconcile heal leg;
//! 3. (cured in `db::content_diesel::stamp_own_conductor_canonical_head`, not
//!    here) the adopted row read `unverified` forever.
//!
//! # What this module adds — and what it deliberately does not
//!
//! **[`should_fetch_on_announce`]** — when an applied content doc names an id
//! this node has no row for, ask for the record NOW, from the peer that just
//! announced it, through the acquisition machinery that already exists:
//! `ReplicationState::discover` (dedup against local / pending / completed /
//! retry-exhausted ids) → the gap dispatch → `GetContent` → the shared
//! `store_acquired_record`. No new wire message and no new route; every
//! receive-side check (reach pre-authorization, verify-first blob pull) is on
//! the path unchanged, because it is the same path.
//!
//! **[`reoffer_stored_row`]** — when `store_acquired_record` stores a row,
//! offer the id to the head-adoption trigger gate. A claim left sleeping by an
//! earlier `NoLocalRow` is taken over by design (`TriggerGate::claim`), so the
//! worker runs again with the row present: it reads the stored doc's
//! `headActionHash` hint and asks the OWN conductor, exactly as it does for any
//! other trigger.
//!
//! Neither seam carries authority. The fetch moves a record the inventory walk
//! would have moved a minute later; the re-offer schedules a decision
//! `services::head_adoption_trigger::decide` still makes from local state. The
//! anti-fabrication bound is untouched: no conductor call is ever made for an
//! id with no row, and a peer naming ids this node has never held buys one
//! `GetContent` back at ITSELF per id (deduped by `discover`, capped by the
//! existing in-flight budget) — the same price its inventory page already
//! charges.

use crate::db::{content_diesel, AppContext, DbPool};
use crate::services::custody_standing::{Requester, TransportId};
use crate::services::head_adoption_trigger::{trigger_candidate_id, EnqueueDecision, TriggerGate};

/// What the announce arm decided about fetching a row. Exhaustive on purpose:
/// each arm is a metric label (`elohim_announce_row_fetch_total`), so "the
/// fetch did nothing" is never indistinguishable from "it was never wired".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnounceFetch {
    /// Not a content-node doc under the projection namespace. No cost at all.
    NotContentDoc,
    /// The row is already here — the ordinary case; the reverse projection and
    /// the trigger own it.
    RowPresent,
    /// The presence read failed. Not evidence of absence: nothing is fetched.
    RowUnreadable,
    /// The id is already local, pending, completed or retry-exhausted in the
    /// replication tracker — a fetch is in flight or was already settled.
    AlreadyTracked,
    /// The in-flight budget is spent. Dropped; the inventory walk is the backstop.
    Saturated,
    /// The announcer could not be addressed on this plane.
    NoRoute,
    /// Queued for the announcing peer and dispatch kicked.
    Enqueued,
}

impl AnnounceFetch {
    /// Closed metric-label vocabulary.
    pub fn label(self) -> &'static str {
        match self {
            Self::NotContentDoc => "not_content_doc",
            Self::RowPresent => "row_present",
            Self::RowUnreadable => "row_unreadable",
            Self::AlreadyTracked => "already_tracked",
            Self::Saturated => "saturated",
            Self::NoRoute => "no_route",
            Self::Enqueued => "enqueued",
        }
    }

    /// Count this decision.
    pub fn record(self) {
        crate::metrics::inc_announce_row_fetch(self.label());
    }
}

/// THE DECISION, pure: which content id should be fetched from the announcer
/// for this applied doc, or why none.
///
/// `row_present` is asked only for a content-node doc, and answers `None` when
/// the read failed. It must be an EXPLICIT presence read: the reverse
/// projection's `Ok(false)` also means "unchanged pointer" and "doc carries no
/// blob", so it cannot stand in for absence.
///
/// | doc | row | ⇒ |
/// |---|---|---|
/// | not `node:<id>` under the projection namespace | – | `NotContentDoc` |
/// | content doc | present | `RowPresent` |
/// | content doc | unreadable | `RowUnreadable` |
/// | content doc | absent | **`Ok(id)`** — fetch |
pub fn should_fetch_on_announce<'a>(
    h_app_id: &str,
    doc_id: &'a str,
    row_present: impl FnOnce(&str) -> Option<bool>,
) -> Result<&'a str, AnnounceFetch> {
    let Some(content_id) = trigger_candidate_id(h_app_id, doc_id) else {
        return Err(AnnounceFetch::NotContentDoc);
    };
    match row_present(content_id) {
        Some(false) => Ok(content_id),
        Some(true) => Err(AnnounceFetch::RowPresent),
        None => Err(AnnounceFetch::RowUnreadable),
    }
}

/// Does the serving scope hold a row for this id? `None` when the read failed.
///
/// bounded-work: one primary-key lookup, reached only for a content doc whose
/// apply carried changes (a doc that did not move never arrives here).
pub fn row_present(pool: &DbPool, content_id: &str) -> Option<bool> {
    let mut conn = pool.get().ok()?;
    content_diesel::content_ids_present(
        &mut conn,
        &AppContext::default_lamad(),
        &[content_id.to_string()],
    )
    .ok()
    .map(|found| found.contains(content_id))
}

/// [`should_fetch_on_announce`] against the real row store.
pub fn absent_row_for<'a>(
    pool: &DbPool,
    h_app_id: &str,
    doc_id: &'a str,
) -> Result<&'a str, AnnounceFetch> {
    should_fetch_on_announce(h_app_id, doc_id, |id| row_present(pool, id))
}

/// Closed `elohim_head_adoption_trigger_total` label for a row-store re-offer,
/// kept apart from the sync-apply offer's labels so the fleet can see WHICH
/// event raised a trigger.
pub fn row_stored_label(decision: EnqueueDecision) -> &'static str {
    match decision {
        EnqueueDecision::Enqueued => "row_stored_enqueued",
        EnqueueDecision::Deduped => "row_stored_deduped",
        EnqueueDecision::DroppedFull => "row_stored_dropped_full",
        EnqueueDecision::ClaimsFull => "row_stored_claims_full",
        // `claim_and_send` takes a content id, never a doc id; unreachable from
        // here, named so the vocabulary stays closed if that ever changes.
        EnqueueDecision::NotContentDoc => "row_stored_not_content_doc",
    }
}

/// A row for `content_id` was just stored from `sender`: offer the id to the
/// head-adoption trigger. `None` when no gate is wired (the node keeps its
/// sweep-bound behaviour).
///
/// Never blocks, awaits or fails the ingest — `claim_and_send` is one map lock
/// and one `try_send`. The trigger's `peer` is the transport id the record
/// arrived from: an observability field and the courier route, never authority.
///
/// The worker decides everything that matters from LOCAL state: with no doc, or
/// a doc naming no head, it ends without a conductor call; with a hint, it asks
/// the own conductor in the `Background` class and adopts only a canonical
/// answer equal to that hint. This function only makes it look.
pub fn reoffer_stored_row(
    gate: Option<&TriggerGate>,
    content_id: &str,
    sender: &Requester,
) -> Option<EnqueueDecision> {
    let gate = gate?;
    let peer = match &sender.transport {
        TransportId::Libp2p(id) | TransportId::Iroh(id) => id.as_str(),
        TransportId::Local => "",
    };
    let decision = gate.claim_and_send(content_id, peer, std::time::Instant::now());
    crate::metrics::inc_head_adoption_trigger(row_stored_label(decision));
    Some(decision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::head_adoption_trigger::DEFAULT_TRIGGER_COOLDOWN;
    use crate::sync::projector::PROJECTION_NAMESPACE;

    #[test]
    fn an_absent_row_for_an_announced_content_doc_is_fetched() {
        assert_eq!(
            should_fetch_on_announce(PROJECTION_NAMESPACE, "node:psalm-23", |_| Some(false)),
            Ok("psalm-23")
        );
    }

    #[test]
    fn a_present_or_unreadable_row_is_never_fetched() {
        assert_eq!(
            should_fetch_on_announce(PROJECTION_NAMESPACE, "node:here", |_| Some(true)),
            Err(AnnounceFetch::RowPresent)
        );
        // A failed read is not evidence of absence.
        assert_eq!(
            should_fetch_on_announce(PROJECTION_NAMESPACE, "node:unknown", |_| None),
            Err(AnnounceFetch::RowUnreadable)
        );
    }

    #[test]
    fn a_non_content_doc_costs_no_row_read() {
        for (ns, doc) in [
            (PROJECTION_NAMESPACE, "manifest:x"),
            (PROJECTION_NAMESPACE, "node:"),
            ("some-other-namespace", "node:psalm-23"),
        ] {
            assert_eq!(
                should_fetch_on_announce(ns, doc, |_| panic!(
                    "the row must not be read for a doc that cannot name content"
                )),
                Err(AnnounceFetch::NotContentDoc),
                "{ns}/{doc}"
            );
        }
    }

    #[test]
    fn the_presence_read_tells_an_absent_row_from_a_stored_one() {
        let pool = crate::test_util::test_pool();
        assert_eq!(row_present(&pool, "not-here"), Some(false));
        assert_eq!(
            absent_row_for(&pool, PROJECTION_NAMESPACE, "node:not-here"),
            Ok("not-here")
        );

        let mut conn = pool.get().unwrap();
        content_diesel::bulk_create_content(
            &mut conn,
            &AppContext::default_lamad(),
            vec![content_diesel::CreateContentInput {
                id: "landed".into(),
                title: "Landed".into(),
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
        drop(conn);
        assert_eq!(row_present(&pool, "landed"), Some(true));
        assert_eq!(
            absent_row_for(&pool, PROJECTION_NAMESPACE, "node:landed"),
            Err(AnnounceFetch::RowPresent)
        );
    }

    #[test]
    fn labels_are_a_closed_distinct_vocabulary() {
        let all = [
            AnnounceFetch::NotContentDoc,
            AnnounceFetch::RowPresent,
            AnnounceFetch::RowUnreadable,
            AnnounceFetch::AlreadyTracked,
            AnnounceFetch::Saturated,
            AnnounceFetch::NoRoute,
            AnnounceFetch::Enqueued,
        ];
        let labels: std::collections::HashSet<_> = all.iter().map(|d| d.label()).collect();
        assert_eq!(labels.len(), all.len());

        let stored: std::collections::HashSet<_> = [
            EnqueueDecision::Enqueued,
            EnqueueDecision::Deduped,
            EnqueueDecision::DroppedFull,
            EnqueueDecision::ClaimsFull,
            EnqueueDecision::NotContentDoc,
        ]
        .into_iter()
        .map(row_stored_label)
        .collect();
        assert_eq!(stored.len(), 5);
        assert!(stored.iter().all(|l| l.starts_with("row_stored_")));
    }

    /// A stored row raises a trigger for its id, attributed to the transport id
    /// the record arrived from; with no gate wired it is a no-op.
    #[tokio::test]
    async fn a_stored_row_is_offered_to_the_adoption_trigger() {
        let (gate, mut rx) = TriggerGate::new(DEFAULT_TRIGGER_COOLDOWN);
        let sender = Requester::libp2p("12D3KooWAnnouncer");

        assert_eq!(
            reoffer_stored_row(Some(&gate), "psalm-23", &sender),
            Some(EnqueueDecision::Enqueued)
        );
        let trigger = rx.try_recv().expect("the stored row raised a trigger");
        assert_eq!(trigger.content_id, "psalm-23");
        assert_eq!(trigger.peer, "12D3KooWAnnouncer");
        assert_eq!(trigger.attempt, 0);

        // Still inside the claim, and the first trigger is queued/running (not
        // sleeping): the second store is the same event, not a new one.
        assert_eq!(
            reoffer_stored_row(Some(&gate), "psalm-23", &sender),
            Some(EnqueueDecision::Deduped)
        );
        assert_eq!(reoffer_stored_row(None, "psalm-23", &sender), None);
    }
}
