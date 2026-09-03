//! Receiver-side pre-authorization for a `private`-reach record ARRIVING over
//! the shard replication plane (station 3b, M9, Task 2 phase B).
//!
//! ## The hole this closes
//!
//! Task 1 ([`crate::private_reach`] + [`crate::services::custody_standing`])
//! closed the SERVE side: a peer asking this node for a `private` row is
//! refused unless it is the ward or a standing custodian. Before this module,
//! `store_acquired_record` (`p2p/mod.rs`) asked no such question on the way
//! IN — any record a peer handed over during acquisition (a `ListContent` /
//! `GetContent` pull, or a proactive `Content` push response) was inserted
//! unconditionally. A peer with no custody standing at all could still end up
//! holding a local COPY of a stranger's `private` row, simply by pulling it —
//! the serve gate protects who can ASK, not what a peer chooses to KEEP.
//!
//! ## The rule
//!
//! > This peer keeps a `private` record iff the SENDER resolves to an agent,
//! > and this peer holds a live `custody-spool` naming that agent as ward, or
//! > a live `custody-blob` naming the record's digest.
//!
//! `ward = the sender's resolved agent` — DELIBERATELY not the wire record's
//! own `created_by` claim. The record travelled here FROM the ward (or a
//! peer already custodying it on the ward's behalf); trusting a payload field
//! an adversarial peer controls would let a spoofed `created_by` manufacture
//! standing. The sender's TRANSPORT identity — resolved through the same
//! `peer_identity_bindings` / `peer_transport_manifest` projections
//! [`crate::services::custody_standing::CustodyStanding`] already uses on the
//! serve side — is the only fact this decision trusts.
//!
//! This is the identical predicate [`crate::private_reach::private_serve_verdict`]
//! decides, asked from the opposite chair: "would THIS peer be served this
//! row, if it asked itself?" It is built from the SAME resolver
//! (`CustodyStanding::resolve_agent` + `CustodyStanding::custody_of`) rather
//! than a parallel one — no second cache, no second bounded conductor
//! fallback, no second interpretation of "live."
//!
//! ## "Who am I?" — resolved here, NOT from `AcquisitionIngestCtx::self_cid`
//!
//! `self_cid` is the TRANSPORT identity join key (`node_transport.rs`:
//! `Libp2pTransport` wraps the libp2p `PeerId`, `IrohTransport` wraps the iroh
//! `NodeId` — `main.rs`'s derivation logs literally read "self_cid is the
//! libp2p peer id" / "self_cid is the iroh NodeId"). `rea_commitments.provider`
//! / `.receiver` — what `CustodyStanding::custody_of` filters on — are always
//! `agent_cid`-shaped (`uhCAk…`), per Task 1's own fixtures and this crate's
//! documented identity-coherence rule ("never join or match raw identity
//! strings across namespaces"). Passing `self_cid` in here would silently
//! empty the join on every real peer — this function instead resolves "who am
//! I" through [`CustodyStanding::resolve_agent`] with [`Requester::local`],
//! the SAME self-agent seam `facts_for`'s `Requester::Local` arm already uses
//! and Task 1 already tests.
//!
//! Non-`private` records are unaffected — `is_private` short-circuits before
//! any resolution, exactly as it does on the serve side, so `public` /
//! `commons` / every other tier keeps its pre-station cost and behaviour.

use crate::p2p::shard_protocol::ContentRecord;
use crate::private_reach::{is_private, WithholdReason};
use crate::services::custody_standing::{digest_key, CustodyStanding, Requester};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use tracing::info;

/// What this peer should do with a record it was just handed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivateReceiveVerdict {
    /// Keep it — insert as today. The verdict for every non-`private` record;
    /// this station changes behaviour for `private` alone.
    Keep,
    /// Do not insert. Carries the SAME closed reason vocabulary
    /// [`WithholdReason`] uses on the serve side, so one metric label set and
    /// one operator mental model cover both directions of this gate.
    Skip(WithholdReason),
}

impl PrivateReceiveVerdict {
    pub fn is_keep(&self) -> bool {
        matches!(self, Self::Keep)
    }

    /// The skip reason, if this is a refusal.
    pub fn reason(&self) -> Option<WithholdReason> {
        match self {
            Self::Keep => None,
            Self::Skip(r) => Some(*r),
        }
    }
}

/// Decide whether this peer may keep a `private` record it received from
/// `sender`. Transport-neutral — the libp2p `ShardResponse::Content` arm and
/// the iroh pull leg both call this exact function before inserting.
///
/// `standing` mirrors [`crate::shard_service::ShardService`]'s own field:
/// `None` fails closed for a `private` record (authority cannot be read),
/// exactly as a missing resolver fails closed on the serve side.
pub async fn preauthorize_private_record(
    standing: Option<&dyn CustodyStanding>,
    sender: &Requester,
    record: &ContentRecord,
) -> PrivateReceiveVerdict {
    if !is_private(&record.reach) {
        return PrivateReceiveVerdict::Keep;
    }
    let Some(standing) = standing else {
        return PrivateReceiveVerdict::Skip(WithholdReason::AuthorityUnavailable);
    };

    // The sender's transport identity is the ward for this decision — never
    // the wire record's own `created_by` claim (see module docs).
    let Some(sender_agent) = standing.resolve_agent(sender).await else {
        log_skip_once(
            sender,
            None,
            WithholdReason::UnresolvedRequester,
            &record.id,
        );
        return PrivateReceiveVerdict::Skip(WithholdReason::UnresolvedRequester);
    };

    // "Who am I?" — through the SAME resolver `Requester::Local` already
    // means in `facts_for` (`ProjectionCustodyStanding::self_agent`), never
    // `AcquisitionIngestCtx::self_cid` (see module docs: that is a transport
    // identity, not an agent_cid).
    let Some(this_peer_agent) = standing.resolve_agent(&Requester::local()).await else {
        log_skip_once(
            sender,
            Some(&sender_agent),
            WithholdReason::AuthorityUnavailable,
            &record.id,
        );
        return PrivateReceiveVerdict::Skip(WithholdReason::AuthorityUnavailable);
    };

    // This peer's own copy, handed back by a caching peer — always keep.
    // Mirrors `ServeReason::Ward` on the serve side.
    if this_peer_agent == sender_agent {
        return PrivateReceiveVerdict::Keep;
    }

    let digest = record
        .blob_hash
        .as_deref()
        .or(record.blob_cid.as_deref())
        .and_then(digest_key);
    let (custody_for_ward, custody_for_digest) = standing
        .custody_of(&this_peer_agent, &sender_agent, digest.as_deref())
        .await;

    if custody_for_ward || custody_for_digest {
        PrivateReceiveVerdict::Keep
    } else {
        log_skip_once(
            sender,
            Some(&sender_agent),
            WithholdReason::NoStanding,
            &record.id,
        );
        PrivateReceiveVerdict::Skip(WithholdReason::NoStanding)
    }
}

/// Log-once-per-(sender, ward) dedup for the skip line — a low-standing
/// sender's whole backlog would otherwise log one line per row. Process-
/// lifetime and unbounded is an acceptable simplification here: the key space
/// is one entry per (transport identity, resolved agent) pair this node ever
/// hears from, bounded by how many peers/humans exist, not by message
/// volume. The counter (`storage_private_preauth_skipped_total`) records
/// EVERY occurrence regardless of this dedup — the log is for a human, the
/// metric is for an operator dashboard.
static SKIP_LOGGED: OnceLock<Mutex<HashSet<(String, String)>>> = OnceLock::new();

fn log_skip_once(sender: &Requester, ward: Option<&str>, reason: WithholdReason, record_id: &str) {
    let set = SKIP_LOGGED.get_or_init(|| Mutex::new(HashSet::new()));
    let key = (sender.label(), ward.unwrap_or("").to_string());
    let mut guard = match set.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if guard.insert(key) {
        info!(
            target: "elohim_storage::reach",
            sender = %sender.label(),
            ward = ward.unwrap_or("unresolved"),
            reason = reason.label(),
            record = %record_id,
            "reach-preauth-skipped: a private record handed to this peer was not kept (no custody standing)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::custody_standing::FakeCustodyStanding;

    const THIS_PEER: &str = "uhCAkThisPeerAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const WARD: &str = "uhCAkWardAgentKeyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const STRANGER: &str = "uhCAkStrangerKeyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const DIGEST: &str = "26d7ced97ee329025135f0ad4791b3e24d526b200b9147943450cb9141480406";

    fn record(reach: &str) -> ContentRecord {
        ContentRecord {
            id: "bafkreiacquired".to_string(),
            title: "t".to_string(),
            description: None,
            content_type: "issue-report".to_string(),
            content_format: "json".to_string(),
            blob_hash: Some(DIGEST.to_string()),
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: reach.to_string(),
            // Deliberately spoofed — the fixture proves this field is never
            // trusted for ward attribution on receive.
            created_by: Some("spoofed-not-the-real-sender".to_string()),
            tags: vec![],
            content_body: None,
        }
    }

    /// A `FakeCustodyStanding` with `Requester::local()` already bound to
    /// `this_peer` — the in-memory double's analog of a resolvable own agent.
    fn fake_with_local(this_peer: &str) -> FakeCustodyStanding {
        let fake = FakeCustodyStanding::new();
        fake.bind(&Requester::local(), this_peer);
        fake
    }

    #[tokio::test]
    async fn a_non_private_record_keeps_without_any_resolution() {
        // No sender binding at all — proves the short-circuit never resolves.
        let fake = FakeCustodyStanding::new();
        let verdict = preauthorize_private_record(
            Some(&fake),
            &Requester::libp2p("unbound"),
            &record("public"),
        )
        .await;
        assert_eq!(verdict, PrivateReceiveVerdict::Keep);
    }

    #[tokio::test]
    async fn no_resolver_fails_closed_for_a_private_record() {
        let verdict = preauthorize_private_record(
            None,
            &Requester::libp2p("12D3KooWAny"),
            &record("private"),
        )
        .await;
        assert_eq!(
            verdict,
            PrivateReceiveVerdict::Skip(WithholdReason::AuthorityUnavailable)
        );
    }

    #[tokio::test]
    async fn an_unresolved_sender_is_skipped() {
        let fake = fake_with_local(THIS_PEER);
        let sender = Requester::libp2p("12D3KooWUnbound");
        let verdict = preauthorize_private_record(Some(&fake), &sender, &record("private")).await;
        assert_eq!(
            verdict,
            PrivateReceiveVerdict::Skip(WithholdReason::UnresolvedRequester)
        );
    }

    #[tokio::test]
    async fn a_resolved_sender_with_no_standing_is_skipped() {
        let fake = fake_with_local(THIS_PEER);
        let sender = Requester::libp2p("12D3KooWStranger");
        fake.bind(&sender, STRANGER);
        let verdict = preauthorize_private_record(Some(&fake), &sender, &record("private")).await;
        assert_eq!(
            verdict,
            PrivateReceiveVerdict::Skip(WithholdReason::NoStanding)
        );
    }

    #[tokio::test]
    async fn a_sender_this_peer_holds_spool_custody_for_is_kept() {
        let fake = fake_with_local(THIS_PEER);
        let sender = Requester::libp2p("12D3KooWWard");
        fake.bind(&sender, WARD);
        fake.spool_custody(THIS_PEER, WARD);
        let verdict = preauthorize_private_record(Some(&fake), &sender, &record("private")).await;
        assert_eq!(verdict, PrivateReceiveVerdict::Keep);
    }

    #[tokio::test]
    async fn a_digest_this_peer_holds_blob_custody_for_is_kept_even_without_spool_custody() {
        let fake = fake_with_local(THIS_PEER);
        let sender = Requester::libp2p("12D3KooWWard");
        fake.bind(&sender, WARD);
        fake.blob_custody(THIS_PEER, DIGEST);
        let verdict = preauthorize_private_record(Some(&fake), &sender, &record("private")).await;
        assert_eq!(verdict, PrivateReceiveVerdict::Keep);
    }

    #[tokio::test]
    async fn the_senders_own_row_ricocheted_back_is_always_kept() {
        // The sender IS this peer's own agent (e.g. a caching peer handed the
        // row back) — no custody commitment needed.
        let fake = fake_with_local(THIS_PEER);
        let sender = Requester::libp2p("12D3KooWSelf");
        fake.bind(&sender, THIS_PEER);
        let verdict = preauthorize_private_record(Some(&fake), &sender, &record("private")).await;
        assert_eq!(verdict, PrivateReceiveVerdict::Keep);
    }

    #[tokio::test]
    async fn this_peers_own_agent_unresolved_fails_closed() {
        // No `Requester::local()` binding registered — this peer cannot even
        // name itself, so the decision must fail closed rather than silently
        // treat an empty string as an agent_cid.
        let fake = FakeCustodyStanding::new();
        let sender = Requester::libp2p("12D3KooWWard");
        fake.bind(&sender, WARD);
        fake.spool_custody("anyone", WARD);
        let verdict = preauthorize_private_record(Some(&fake), &sender, &record("private")).await;
        assert_eq!(
            verdict,
            PrivateReceiveVerdict::Skip(WithholdReason::AuthorityUnavailable)
        );
    }

    /// The wire record's own `created_by` is a spoofable claim — this fixture
    /// pins that it is never consulted for ward attribution: standing is
    /// granted purely from the sender's resolved identity, even though the
    /// record's `created_by` names neither the ward nor this peer.
    #[tokio::test]
    async fn created_by_on_the_wire_is_never_trusted_for_ward_attribution() {
        let fake = fake_with_local(THIS_PEER);
        let sender = Requester::libp2p("12D3KooWWard");
        fake.bind(&sender, WARD);
        fake.spool_custody(THIS_PEER, WARD);
        let rec = record("private");
        assert_ne!(rec.created_by.as_deref(), Some(WARD));
        let verdict = preauthorize_private_record(Some(&fake), &sender, &rec).await;
        assert_eq!(verdict, PrivateReceiveVerdict::Keep);
    }

    #[tokio::test]
    async fn a_public_record_is_kept_regardless_of_sender_binding() {
        let fake = FakeCustodyStanding::new();
        let sender = Requester::libp2p("12D3KooWStranger");
        let verdict = preauthorize_private_record(Some(&fake), &sender, &record("public")).await;
        assert_eq!(verdict, PrivateReceiveVerdict::Keep);
    }
}
