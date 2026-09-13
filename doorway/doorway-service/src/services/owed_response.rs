//! The owed response — what a visitor's challenge to a standing BECOMES.
//!
//! An owed response is the difference between feedback and a suggestion box.
//! When a visitor challenges a reach ruling, the challenge is witnessed as an
//! REA `Commitment` with a NAMED party who owes an answer and a due date fixed
//! by a term that was declared before the visitor sent anything.
//!
//! ## The doorway never decides who owes
//!
//! [`OwedResponse::party`] is copied VERBATIM from the contract's own
//! `responsiveReach.party` — the collective's declaration, on the collective's
//! own `project-epr` contract, notarized before the challenge existed. When a
//! contract declares no term, NOTHING is minted and the answer is `owed: null`
//! with the reason ([`crate::services::ChallengeOutcome::NothingOwed`]). A
//! doorway that picked a party would be answering on the collective's behalf,
//! which is exactly what the story says it must not do.
//!
//! The doorway's whole role is to CARRY: it resolves the contract from its own
//! live router projection (never a client-supplied id), copies the declared
//! party, records the visitor's words verbatim, and gets out of the way. That
//! `carriedBy` is a different party from `provider` IS the record that the
//! doorway did not answer for the collective.
//!
//! ## The window is declared; the date is arithmetic
//!
//! The contract fixes `withinHours`. The due instant is that window counted
//! from when the challenge was RECEIVED — arithmetic any reader can redo from
//! the commitment's own `hasBeginning` and the contract's own term, which is
//! what makes it checkable rather than something the doorway asserted.
//! [`due_from_responsive_reach`] is that one calculation, pure and alone.
//!
//! `due` is structurally immutable on the substrate (`handle_update_state`
//! reconciles only state/finished/metadata), so a witnessed window can move
//! only by supersession — never by the doorway quietly extending it.
//!
//! ## What may NOT ride this commitment
//!
//! No refused path, no IP, no session id, no user-agent, no prior anonymous
//! browsing. A redress record is not a visitor log. What lands is: who
//! challenged, what contract, in whose words, who owes, and by when.

use chrono::{DateTime, Duration, Utc};
use elohim_views::projection::{EprProjectionView, ResponsiveReach};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Where a refused visitor is pointed so they can be heard — and the route that
/// witnesses what they say.
pub const CHALLENGE_ROUTE: &str = "/api/v1/challenge";

/// The REA action a witnessed challenge is recorded under.
///
/// A new ACTION VALUE on the existing `Commitment` entry type — never a new
/// entry type. Mirrors the `operate-doorway` / `project-epr` pattern; the DNA's
/// `REA_ACTIONS` array is declared and never validated, so this value is
/// DNA-hash-neutral and the array append rides the next DNA move. The storage
/// crate carries the same constant with a drift test.
pub const RESPOND_TO_CHALLENGE_ACTION: &str = "respond-to-challenge";

/// Longest visitor words this doorway will carry onto a commitment. A challenge
/// is a sentence a person writes, not a payload — and the note lands on a
/// notarized record, so the bound is part of the record's shape, not a
/// convenience.
pub const MAX_CHALLENGE_WORDS: usize = 4_000;

/// Route prefix that reads a commitment back.
const COMMITMENT_RECORD_ROUTE: &str = "/api/v1/commitments/";

/// The redress term a refusal advertises, and a challenge is witnessed under.
///
/// Everything here is read from the contract. Nothing is the doorway's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwedResponse {
    /// WHO owes the answer — copied verbatim from the contract's declaration.
    pub party: String,
    /// The name a person would use for them, when the contract holds one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party_label: Option<String>,
    /// How long they have, from when the challenge is received.
    pub within_hours: u32,
    /// The record that DECLARED this term — the contract itself, dereferenceable
    /// in one request. This is what makes "declared before the visitor sent it"
    /// checkable rather than asserted.
    pub declared_by: String,
    /// When that record was written. Strictly before any challenge against it.
    pub declared_at: String,
}

/// The declared redress term for a projection, or `None` when the collective
/// declared none.
///
/// `None` is a first-class answer, not a failure: it is reported as
/// `owed: null` with the reason, and nothing is minted.
#[must_use]
pub fn owed_from_contract(projection: &EprProjectionView) -> Option<OwedResponse> {
    let term = projection.responsive_reach.as_ref()?;
    if term.party.trim().is_empty() || term.within_hours == 0 {
        // A term that names nobody, or allows no time at all, is not a term.
        // Treating it as one would witness a challenge against a party the
        // contract did not really declare.
        return None;
    }
    Some(OwedResponse {
        party: term.party.trim().to_string(),
        party_label: term
            .party_label
            .as_deref()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string),
        within_hours: term.within_hours,
        declared_by: format!(
            "{COMMITMENT_RECORD_ROUTE}{}",
            urlencoding::encode(projection.commitment_id.trim())
        ),
        declared_at: projection.seeded_at.clone(),
    })
}

/// The instant the declared party's answer comes due, for a challenge received
/// at `received_at`.
///
/// PURE — the window comes from the contract, the start comes from the request,
/// and nothing else is consulted. `None` when the term allows no time at all or
/// the arithmetic would overflow: an un-computable due date is reported absent,
/// never rounded into one the doorway made up.
#[must_use]
pub fn due_from_responsive_reach(
    term: &ResponsiveReach,
    received_at: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    if term.within_hours == 0 {
        return None;
    }
    let window = Duration::try_hours(i64::from(term.within_hours))?;
    received_at.checked_add_signed(window)
}

/// The commitment id for one challenge.
///
/// Agent-scoped composite over `(challenger, action, contract, challengeId)`, so
/// ensure-not-create makes a replay mint nothing twice while two different
/// people challenging the same contract, or one person challenging twice about
/// different things, each get their own record.
///
/// The challenger is IN the id deliberately: a challenge is somebody's, and an
/// id that collapsed two people's challenges into one record would silently
/// drop the second person's.
#[must_use]
pub fn challenge_commitment_id(
    challenger_human_id: &str,
    contract_id: &str,
    challenge_id: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(challenger_human_id.trim().as_bytes());
    hasher.update(b"|");
    hasher.update(RESPOND_TO_CHALLENGE_ACTION.as_bytes());
    hasher.update(b"|");
    hasher.update(contract_id.trim().as_bytes());
    hasher.update(b"|");
    hasher.update(challenge_id.trim().as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("{RESPOND_TO_CHALLENGE_ACTION}-{}", &digest[..16])
}

/// A stable reference for a challenge the visitor did not name one for.
///
/// Derived from the words themselves, so re-sending the SAME sentence is a
/// replay (mints nothing twice) while a different sentence is a different
/// challenge. A random id would make every retry a new record, which is how a
/// redress queue fills with duplicates nobody can answer.
#[must_use]
pub fn challenge_reference_from_words(words: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(words.trim().as_bytes());
    hex::encode(hasher.finalize())[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_views::projection::{Channel, ProjectionMode};

    fn projection(term: Option<ResponsiveReach>) -> EprProjectionView {
        EprProjectionView {
            commitment_id: "project-epr-garden".into(),
            epr_id: "community-garden-club".into(),
            doorway_id: "doorway:alpha-elohim-host".into(),
            url_path: "/garden".into(),
            hostnames: vec![],
            channel: Channel::Converged,
            mode: ProjectionMode::Cached,
            reach: "private".into(),
            base_href: "/garden/".into(),
            entry_file: "index.html".into(),
            spa_fallback: true,
            redirects_from: vec![],
            redirect_templates: vec![],
            route_claims: None,
            preview_epr_ref: None,
            gate_hints: vec![],
            dead_end: false,
            steward_direct_endpoint: None,
            responsive_reach: term,
            hosting_agreement_id: None,
            seeded_at: "2026-09-01T00:00:00Z".into(),
            seeded_by: "12D3KooWAlphaHolder".into(),
        }
    }

    fn term(party: &str, hours: u32) -> ResponsiveReach {
        ResponsiveReach {
            party: party.into(),
            party_label: Some("the Dowell household".into()),
            within_hours: hours,
        }
    }

    // ── who owes ────────────────────────────────────────────────────────────

    /// The party is the CONTRACT's, and it reads back to the record that
    /// declared it.
    #[test]
    fn the_owed_party_comes_from_the_contract_and_reads_back_to_it() {
        let owed = owed_from_contract(&projection(Some(term("household-dowell", 72))))
            .expect("the contract declares a term");
        assert_eq!(owed.party, "household-dowell");
        assert_eq!(owed.party_label.as_deref(), Some("the Dowell household"));
        assert_eq!(owed.within_hours, 72);
        assert_eq!(owed.declared_by, "/api/v1/commitments/project-epr-garden");
        assert_eq!(
            owed.declared_at, "2026-09-01T00:00:00Z",
            "the declaration's own timestamp — strictly before any challenge against it"
        );
    }

    /// A collective that declared nothing owes nothing, and the doorway names
    /// no substitute. This is the whole "the doorway gets out of the way" rule.
    #[test]
    fn a_contract_with_no_term_owes_nothing_and_the_doorway_names_nobody() {
        assert!(owed_from_contract(&projection(None)).is_none());
    }

    #[test]
    fn a_term_that_names_nobody_or_allows_no_time_is_not_a_term() {
        assert!(owed_from_contract(&projection(Some(term("   ", 72)))).is_none());
        assert!(owed_from_contract(&projection(Some(term("household-dowell", 0)))).is_none());
    }

    #[test]
    fn a_blank_label_falls_back_to_the_party_rather_than_saying_nothing() {
        let mut t = term("household-dowell", 24);
        t.party_label = Some("   ".into());
        let owed = owed_from_contract(&projection(Some(t))).expect("still a term");
        assert!(owed.party_label.is_none());
        assert_eq!(owed.party, "household-dowell");
    }

    // ── the window ──────────────────────────────────────────────────────────

    #[test]
    fn the_due_instant_is_the_declared_window_from_when_it_was_received() {
        let received: DateTime<Utc> = "2026-09-13T10:00:00Z".parse().unwrap();
        let due = due_from_responsive_reach(&term("household-dowell", 72), received)
            .expect("a declared window computes");
        assert_eq!(due.to_rfc3339(), "2026-09-16T10:00:00+00:00");
        assert_eq!(
            (due - received).num_hours(),
            72,
            "the window a reader redoes from the contract is the window that was witnessed"
        );
    }

    #[test]
    fn a_window_of_no_time_yields_no_due_date_rather_than_this_instant() {
        let received: DateTime<Utc> = "2026-09-13T10:00:00Z".parse().unwrap();
        assert!(due_from_responsive_reach(&term("household-dowell", 0), received).is_none());
    }

    #[test]
    fn an_uncomputable_window_is_absent_never_rounded() {
        let far: DateTime<Utc> = DateTime::<Utc>::MAX_UTC;
        assert!(
            due_from_responsive_reach(&term("household-dowell", u32::MAX), far).is_none(),
            "an overflow must report no due date, never a wrapped one"
        );
    }

    // ── the id ──────────────────────────────────────────────────────────────

    /// Ensure-not-create: the same challenger, contract and reference mint ONE
    /// record no matter how many times it is sent.
    #[test]
    fn the_same_challenge_sent_twice_is_the_same_record() {
        let a = challenge_commitment_id("human-james", "project-epr-garden", "ref-1");
        let b = challenge_commitment_id("human-james", "project-epr-garden", "ref-1");
        assert_eq!(a, b);
        assert!(a.starts_with("respond-to-challenge-"));
        assert_eq!(a.len(), "respond-to-challenge-".len() + 16);
    }

    /// …and two PEOPLE challenging the same contract are two records. An id
    /// that collapsed them would silently drop the second person's challenge.
    #[test]
    fn two_people_challenging_the_same_contract_are_two_records() {
        assert_ne!(
            challenge_commitment_id("human-james", "project-epr-garden", "ref-1"),
            challenge_commitment_id("human-jessica", "project-epr-garden", "ref-1")
        );
    }

    #[test]
    fn one_person_challenging_two_contracts_or_two_things_is_two_records() {
        assert_ne!(
            challenge_commitment_id("human-james", "project-epr-garden", "ref-1"),
            challenge_commitment_id("human-james", "project-epr-orchard", "ref-1")
        );
        assert_ne!(
            challenge_commitment_id("human-james", "project-epr-garden", "ref-1"),
            challenge_commitment_id("human-james", "project-epr-garden", "ref-2")
        );
    }

    #[test]
    fn a_reference_derived_from_the_words_makes_a_retry_a_replay() {
        let words = "This is my neighbourhood's garden club and I should be able to read it.";
        assert_eq!(
            challenge_reference_from_words(words),
            challenge_reference_from_words(&format!("  {words}  ")),
            "whitespace around the same sentence is the same sentence"
        );
        assert_ne!(
            challenge_reference_from_words(words),
            challenge_reference_from_words("Something else entirely.")
        );
        assert_eq!(challenge_reference_from_words(words).len(), 16);
    }

    /// The action value is the wire contract. It is content-addressed into
    /// every challenge id, so changing it re-mints every standing challenge.
    #[test]
    fn the_action_value_is_the_one_the_storage_crate_carries() {
        assert_eq!(RESPOND_TO_CHALLENGE_ACTION, "respond-to-challenge");
    }

    /// The refusal's redress route and the route that witnesses a challenge are
    /// ONE constant. A refusal that pointed somewhere the challenge route is not
    /// is a suggestion box with extra steps.
    #[test]
    fn the_redress_route_a_refusal_advertises_is_the_route_that_witnesses() {
        assert_eq!(CHALLENGE_ROUTE, crate::services::WHERE_TO_BE_HEARD);
    }
}
