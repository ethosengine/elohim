//! Custody-scoped read gate for `private`-reach rows on the SHARD REPLICATION
//! plane (libp2p `/elohim/shard/1.0.0` and the iroh `/elohim/shard/2.0.0` ALPN).
//!
//! ## The hole this closes
//!
//! `ShardService` answered every peer identically. `handle_list_content` and
//! `handle_get_content` both read at [`crate::db::content_diesel::MinTrust::Invisible`]
//! with the comment "peers must see all local rows so replication can cover
//! pre-drain content", and `handle_get` served blob bytes with no DB read and no
//! reach question at all. `reach_filter` on `ListContent` is a WHERE narrowing
//! chosen by the REQUESTER — a filter, never a gate.
//!
//! A death witness (`content_type: "issue-report"`, `reach: "private"`, written
//! by [`crate::services::spool_ingest`]) therefore left its ward's peer toward
//! any peer that asked. Over HTTP the same row already refuses an anonymous
//! caller (403 + `requiredReach` on `/db/content/{id}`; [`crate::blob_reach`] on
//! `/blob/{hash}`). The replication plane was the asymmetric half.
//!
//! ## The rule, and why it is stated this way
//!
//! > A `private` row (and the bytes only it references) is servable to a peer
//! > iff that peer RESOLVED to an agent, and that agent is the ward, or holds a
//! > live `custody-spool` commitment naming the ward, or holds a live
//! > `custody-blob` commitment naming this digest.
//!
//! This is the **two faces** model the crate already states
//! ([`crate::p2p::reach_authorization`] module docs): the serving peer is the
//! steward of what its agent authored and enforces the reach the author
//! declared. Standing is never taken from the requester's claim — it derives
//! from a notarized `Commitment` row whose provider is the requester and whose
//! authorship IS the counter-signature (C5).
//!
//! ## Scope — declared, deliberately narrow (station 3b, M9)
//!
//! ONLY `reach == "private"` changes behaviour. `public` / `commons` and every
//! other tier (`intimate` / `trusted` / `familiar` / `community` / …) serve on
//! this plane exactly as they did before this module existed. Widening to those
//! tiers is a later station and must be a deliberate edit here, not a silent
//! consequence — [`every_other_reach_serves_unchanged`] pins that.
//!
//! Note the asymmetry with [`crate::blob_reach`]: THAT module fails an
//! unrecognized tier CLOSED (an unknown word could be more restricted than
//! `public`). THIS module fails an unrecognized tier OPEN, because its whole
//! contract is "one named tier and nothing else" — an unknown word here is a
//! tier this station did not take responsibility for, and refusing it would be
//! a silent scope widening on the replication plane.
//!
//! ## Honest absence (C4)
//!
//! A withheld row is REFUSED, never reported missing. `GetContent` and blob
//! `Get` answer `ShardResponse::Error("reach-withheld: <reason>")` and never
//! `ContentNotFound` / `NotFound`. `ListContent` omits the row from the page and
//! COUNTS the omission (`storage_private_withheld_total{site,reason}`) — an
//! omission a caller cannot see is still an omission the operator can.

use std::fmt;

/// The reach tier this gate takes responsibility for. Exactly one.
pub const PRIVATE_REACH: &str = "private";

/// Is this row's reach the tier this gate governs?
///
/// Exact canonical equality is load-bearing in this station: every other value,
/// including `"Private"`, stays outside this deliberately narrow change.
pub fn is_private(reach: &str) -> bool {
    reach == PRIVATE_REACH
}

/// Why a `private` row was withheld from a peer.
///
/// A closed vocabulary: it is the `reason` label on
/// `storage_private_withheld_total` and the suffix of the typed
/// `reach-withheld: <reason>` refusal, so a new variant is a metric-cardinality
/// decision and a wire-visible string change at the same time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WithholdReason {
    /// The transport identity resolved to no agent — no binding in
    /// `peer_identity_bindings` (libp2p) or `peer_transport_manifest` (iroh).
    /// Fail-closed: an unresolved peer can never be shown to hold standing.
    UnresolvedRequester,
    /// The requester resolved to an agent, and that agent is neither the ward
    /// nor a custodian of the ward or of this digest.
    NoStanding,
    /// This peer cannot say WHOSE private row this is, so it cannot say who
    /// would have standing over it. Distinct from `NoStanding` on purpose: the
    /// refusal is about the holder's own knowledge, not the requester.
    WardUnresolved,
    /// The serving peer could not read the authority needed to decide: the
    /// content pool/resolver was missing, checkout failed, or a reference/row
    /// query failed. Blob bytes fail closed on every such condition.
    AuthorityUnavailable,
}

impl WithholdReason {
    /// Stable metric-label / wire rendering. Kebab-case, closed set.
    pub fn label(&self) -> &'static str {
        match self {
            Self::UnresolvedRequester => "unresolved-requester",
            Self::NoStanding => "no-standing",
            Self::WardUnresolved => "ward-unresolved",
            Self::AuthorityUnavailable => "authority-unavailable",
        }
    }

    /// Every variant — used to pre-touch metric series so each reads as a
    /// measured zero from boot rather than an absent series.
    pub const ALL: [WithholdReason; 4] = [
        WithholdReason::UnresolvedRequester,
        WithholdReason::NoStanding,
        WithholdReason::WardUnresolved,
        WithholdReason::AuthorityUnavailable,
    ];
}

/// Why a permitted `private` row may leave this peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServeReason {
    /// This station does not govern the row's reach.
    NonPrivate,
    /// The resolved requester is the ward.
    Ward,
    /// The requester holds a live `custody-spool` for the ward.
    SpoolCustody,
    /// The requester holds a live `custody-blob` for the exact digest.
    BlobCustody,
}

impl ServeReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::NonPrivate => "non-private",
            Self::Ward => "ward",
            Self::SpoolCustody => "spool-custody",
            Self::BlobCustody => "blob-custody",
        }
    }
}

impl fmt::Display for WithholdReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// What the serving peer must do with this row for this requester.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivateServeVerdict {
    /// Serve, retaining the evidence class for decision observability.
    Serve(ServeReason),
    /// Refuse. `ListContent` omits + counts; `GetContent` / `Get` answer a typed
    /// `reach-withheld: <reason>` error — NEVER a not-found (C4).
    Withhold(WithholdReason),
}

impl PrivateServeVerdict {
    pub fn is_serve(&self) -> bool {
        matches!(self, Self::Serve(_))
    }

    /// The withhold reason, if this is a refusal.
    pub fn reason(&self) -> Option<WithholdReason> {
        match self {
            Self::Serve(_) => None,
            Self::Withhold(r) => Some(*r),
        }
    }

    pub fn serve_reason(&self) -> Option<ServeReason> {
        match self {
            Self::Serve(reason) => Some(*reason),
            Self::Withhold(_) => None,
        }
    }
}

/// The facts one (row, requester) pair presents to [`private_serve_verdict`].
///
/// The caller resolves every field; this struct exists so the DECISION is pure
/// and the RESOLUTION (identity maps, `rea_commitments`, an own-conductor read)
/// is somebody else's bounded problem — see
/// [`crate::services::custody_standing`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateServeFacts {
    /// The row's declared audience, verbatim from the `content` projection.
    pub reach: String,
    /// The requesting transport identity resolved to SOME agent.
    pub requester_resolved: bool,
    /// The resolved requester IS the ward this row belongs to.
    pub requester_is_ward: bool,
    /// The resolved requester provides a LIVE `custody-spool` naming the ward.
    pub custody_for_ward: bool,
    /// The resolved requester provides a LIVE `custody-blob` naming this
    /// digest. Independent of the ward: a per-blob pledge stands on its own.
    pub custody_for_digest: bool,
    /// This peer could name the ward at all.
    pub ward_resolved: bool,
    /// The authority needed for the final standing answer was readable. False
    /// only when a required conductor fallback had no client, errored, or timed
    /// out; that is never equivalent to an absent commitment.
    pub authority_available: bool,
}

impl PrivateServeFacts {
    /// Facts for a row this gate does not govern — every standing field false
    /// because none of them was asked. The verdict is `Serve` from `reach`
    /// alone, so a caller building this pays no resolution cost.
    pub fn not_gated(reach: impl Into<String>) -> Self {
        Self {
            reach: reach.into(),
            requester_resolved: false,
            requester_is_ward: false,
            custody_for_ward: false,
            custody_for_digest: false,
            ward_resolved: false,
            authority_available: true,
        }
    }

    /// Facts for a `private` row whose requester resolved to no agent. The
    /// fail-closed floor: nothing else needs resolving to answer.
    pub fn unresolved_requester(reach: impl Into<String>) -> Self {
        Self::not_gated(reach)
    }
}

/// The custody-scoped serve decision for one row and one requester.
///
/// PURE — no DB, no clock, no network, no conductor. Every input is a bool the
/// caller has already resolved, exactly as [`crate::blob_reach::blob_serve_verdict`]
/// takes a resolved `identity_resolved` rather than a header.
///
/// Order is load-bearing and stated once here:
///
/// 1. Not `private` → `Serve`. No other tier's behaviour moves in this station.
/// 2. Requester unresolved → `Withhold(UnresolvedRequester)`. Fail-closed, and
///    checked before ward resolution so a peer that cannot even be named is
///    refused for the reason that is actually true of it.
/// 3. Requester is the ward → `Serve`. The ward's own copy comes home.
/// 4. Requester holds spool custody for the ward, or blob custody for this
///    digest → `Serve`. Either pledge alone is standing.
/// 5. Required authority unavailable → `Withhold(AuthorityUnavailable)`; a
///    transient bridge failure is not evidence that standing is absent.
/// 6. Ward unresolved → `Withhold(WardUnresolved)`, chosen over `NoStanding`
///    because "I cannot say whose this is" is a different operator action from
///    "you are a stranger to its ward".
/// 7. Otherwise → `Withhold(NoStanding)`.
pub fn private_serve_verdict(facts: &PrivateServeFacts) -> PrivateServeVerdict {
    if !is_private(&facts.reach) {
        return PrivateServeVerdict::Serve(ServeReason::NonPrivate);
    }
    if !facts.requester_resolved {
        return PrivateServeVerdict::Withhold(WithholdReason::UnresolvedRequester);
    }
    if facts.requester_is_ward {
        return PrivateServeVerdict::Serve(ServeReason::Ward);
    }
    if facts.custody_for_ward {
        return PrivateServeVerdict::Serve(ServeReason::SpoolCustody);
    }
    // A live custody-blob commitment names the exact bytes and therefore
    // stands even when this holder cannot independently resolve the ward.
    if facts.custody_for_digest {
        return PrivateServeVerdict::Serve(ServeReason::BlobCustody);
    }
    if !facts.authority_available {
        return PrivateServeVerdict::Withhold(WithholdReason::AuthorityUnavailable);
    }
    if !facts.ward_resolved {
        return PrivateServeVerdict::Withhold(WithholdReason::WardUnresolved);
    }
    PrivateServeVerdict::Withhold(WithholdReason::NoStanding)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A resolved stranger to a `private` row with a known ward.
    fn stranger() -> PrivateServeFacts {
        PrivateServeFacts {
            reach: PRIVATE_REACH.to_string(),
            requester_resolved: true,
            requester_is_ward: false,
            custody_for_ward: false,
            custody_for_digest: false,
            ward_resolved: true,
            authority_available: true,
        }
    }

    #[test]
    fn public_and_commons_serve_to_anyone() {
        for reach in ["public", "commons"] {
            assert_eq!(
                private_serve_verdict(&PrivateServeFacts::not_gated(reach)),
                PrivateServeVerdict::Serve(ServeReason::NonPrivate),
                "{reach} must serve unchanged on the replication plane"
            );
        }
    }

    #[test]
    fn private_with_an_unresolved_requester_is_withheld() {
        let facts = PrivateServeFacts::unresolved_requester(PRIVATE_REACH);
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Withhold(WithholdReason::UnresolvedRequester)
        );
    }

    #[test]
    fn private_serves_to_the_ward() {
        let facts = PrivateServeFacts {
            requester_is_ward: true,
            ..stranger()
        };
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Serve(ServeReason::Ward)
        );
    }

    #[test]
    fn private_serves_to_a_spool_custodian_of_the_ward() {
        let facts = PrivateServeFacts {
            custody_for_ward: true,
            ..stranger()
        };
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Serve(ServeReason::SpoolCustody)
        );
    }

    /// A per-blob pledge stands alone — it does not need the ward to be named.
    /// This is the path a custodian-to-custodian onward fetch takes.
    #[test]
    fn private_serves_to_a_blob_custodian_even_with_an_unresolved_ward() {
        let facts = PrivateServeFacts {
            custody_for_digest: true,
            ward_resolved: false,
            ..stranger()
        };
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Serve(ServeReason::BlobCustody)
        );
    }

    #[test]
    fn private_withholds_from_a_resolved_stranger() {
        assert_eq!(
            private_serve_verdict(&stranger()),
            PrivateServeVerdict::Withhold(WithholdReason::NoStanding)
        );
    }

    #[test]
    fn unavailable_authority_is_not_laundered_into_absent_standing() {
        let facts = PrivateServeFacts {
            authority_available: false,
            ..stranger()
        };
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Withhold(WithholdReason::AuthorityUnavailable)
        );
    }

    #[test]
    fn private_with_an_unresolvable_ward_says_so() {
        let facts = PrivateServeFacts {
            ward_resolved: false,
            ..stranger()
        };
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Withhold(WithholdReason::WardUnresolved)
        );
    }

    /// The DECLARED scope of station 3b. Every tier other than `private`
    /// replicates exactly as it did before this module existed — including
    /// tiers the HTTP path gates hard. Widening the gate must fail HERE first,
    /// so it can only ever be a deliberate edit.
    #[test]
    fn every_other_reach_serves_unchanged() {
        for reach in [
            "public",
            "commons",
            "community",
            "familiar",
            "trusted",
            "intimate",
            "self-scope",
            "restricted",
            "a-tier-nobody-has-invented-yet",
        ] {
            let facts = PrivateServeFacts {
                reach: reach.to_string(),
                ..stranger()
            };
            assert_eq!(
                private_serve_verdict(&facts),
                PrivateServeVerdict::Serve(ServeReason::NonPrivate),
                "station 3b governs `private` and nothing else; {reach} moved"
            );
        }
    }

    /// This station gates only the canonical wire value; capitalization does
    /// not silently widen its scope.
    #[test]
    fn only_canonical_private_is_gated() {
        assert!(is_private("private"));
        assert!(!is_private("Private"));
        assert!(!is_private("  PRIVATE "));
        assert!(!is_private("privately-held"));
        let facts = PrivateServeFacts {
            reach: "Private".to_string(),
            requester_resolved: false,
            ..stranger()
        };
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Serve(ServeReason::NonPrivate)
        );
    }

    /// The refusal string the two byte/record sites render is built from this
    /// label, so it is pinned rather than re-spelled at each call site.
    #[test]
    fn withhold_reason_labels_are_stable() {
        assert_eq!(
            WithholdReason::UnresolvedRequester.label(),
            "unresolved-requester"
        );
        assert_eq!(WithholdReason::NoStanding.label(), "no-standing");
        assert_eq!(WithholdReason::WardUnresolved.label(), "ward-unresolved");
        assert_eq!(
            WithholdReason::AuthorityUnavailable.label(),
            "authority-unavailable"
        );
        assert_eq!(
            format!("reach-withheld: {}", WithholdReason::NoStanding),
            "reach-withheld: no-standing"
        );
        assert_eq!(
            WithholdReason::ALL.len(),
            4,
            "ALL must cover the closed vocabulary"
        );
    }
}
