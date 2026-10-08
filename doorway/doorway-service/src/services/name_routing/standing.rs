//! Name standing — a doorway answers for a public name it is a MEMBER of
//! (serving-edge campaign story 2.2, habit `served-under-standing`).
//!
//! # What a public name is here
//!
//! A public name (`elohim.local`, `alpha.elohim.local` on the household;
//! `elohim.host` on the fleet) is not a doorway's identity. It is a name whose
//! **membership document** lists the origins currently eligible to serve it.
//! The household's document is written by `relay-addr-beacon`'s `file` sink —
//! one beacon leg per doorway, each owning exactly its own entry, decided by
//! the same serving probe and join2/leave3 hysteresis the fleet's DNS lane
//! runs — into `<membership dir>/<public-name>.json`:
//!
//! ```json
//! { "name": "elohim.local",
//!   "members": [ { "owner": "alpha", "origin": "http://localhost:8888", "updated_at": "…" },
//!                { "owner": "apex",  "origin": "http://localhost:8889", "updated_at": "…" } ] }
//! ```
//!
//! This module reads that document (Category C: a projection of the
//! membership authority, re-read every second, never persisted, never
//! notarized) and decides, per request, what THIS doorway may do for the
//! name the client's `Host` asked for. It creates no entity; see the P2P gate
//! answer in the F3 brief.
//!
//! With no membership directory configured (`DOORWAY_MEMBERSHIP_DIR`
//! unset) the registry stays empty and every request behaves exactly as it
//! did before this module existed: no refusal, no new header.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Deserialize;
use tracing::{debug, info, warn};

use super::{RouteKey, NAME_ROUTE_HEADER};

/// The `error` token of a misdirected-request body.
pub const MISDIRECTED_ERROR: &str = "misdirected-request";

/// How often the membership directory is re-read. The household beacon's
/// probe cadence is 3 s with a two-probe join, so a one-second re-read keeps
/// this doorway's reading within a fraction of the authority's own latency.
pub const MEMBERSHIP_REFRESH: Duration = Duration::from_secs(1);

/// One entry of a membership document: who owns it and where it serves.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PublicNameMember {
    pub owner: String,
    pub origin: String,
}

/// One membership document: one public name and the origins eligible to
/// serve it, in the writer's order (the household sink keeps it sorted by
/// owner — owner order, the same stable tiebreak the relay selector ends on).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PublicNameDocument {
    pub name: String,
    pub members: Vec<PublicNameMember>,
}

/// Every membership document this doorway can currently read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PublicNameMembership {
    documents: Vec<PublicNameDocument>,
}

impl PublicNameMembership {
    /// Build from parsed documents. Names are normalised (lowercase, port
    /// stripped) so `Host: Elohim.Local:8889` finds `elohim.local`; a
    /// document with an empty name is dropped, and a second document for a
    /// name already seen is ignored (one document names one public name).
    pub fn from_documents(documents: Vec<PublicNameDocument>) -> Self {
        let mut seen = HashSet::new();
        let documents = documents
            .into_iter()
            .filter_map(|mut doc| {
                let name = RouteKey::new(Some(&doc.name), "/").host?;
                if !seen.insert(name.clone()) {
                    return None;
                }
                doc.name = name;
                Some(doc)
            })
            .collect();
        Self { documents }
    }

    /// True when no public name is known — the rule is off.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// The document for the name `host` asks for, if `host` is a public name.
    pub fn document(&self, host: Option<&str>) -> Option<&PublicNameDocument> {
        let asked = RouteKey::new(host, "/").host?;
        self.documents.iter().find(|doc| doc.name == asked)
    }

    /// The public names `me` is currently a member of, sorted.
    pub fn names_member_of(&self, me: &SelfIdentity<'_>) -> Vec<String> {
        let mut names: Vec<String> = self
            .documents
            .iter()
            .filter(|doc| doc.members.iter().any(|m| me.is(m)))
            .map(|doc| doc.name.clone())
            .collect();
        names.sort();
        names
    }
}

/// How this doorway recognises itself against the membership documents.
///
/// The document is origin-keyed (a beacon leg writes the origin it probes), so
/// a member entry is this doorway's when its origin is one of this doorway's
/// declared origins (`DOORWAY_URL` / `DOORWAY_URLS`). Only a doorway that
/// declares NO origin falls back to matching an owner equal to its doorway id —
/// with origins declared, an owner slug that happens to equal the id must not
/// let a different origin speak for this doorway.
///
/// `candidate_names` are the public names this doorway STANDS FOR
/// (`DOORWAY_PUBLIC_NAMES`, normalised): the names whose membership its beacon
/// leg competes for. A candidate that is currently withdrawn from a document is
/// still one of that name's doorways — it is not misdirected, only not the one
/// the name currently advertises.
#[derive(Debug, Clone, Copy)]
pub struct SelfIdentity<'a> {
    pub doorway_id: &'a str,
    pub origins: &'a [String],
    pub candidate_names: &'a [String],
}

impl SelfIdentity<'_> {
    pub fn is(&self, member: &PublicNameMember) -> bool {
        if self.origins.is_empty() {
            return member.owner == self.doorway_id;
        }
        self.origins
            .iter()
            .any(|origin| same_origin(origin, &member.origin))
    }

    /// True when this doorway declares it stands for `name` (already
    /// normalised by [`PublicNameMembership::from_documents`]).
    pub fn stands_for(&self, name: &str) -> bool {
        self.candidate_names
            .iter()
            .any(|candidate| candidate == name)
    }
}

/// Normalise a configured list of public names the same way the documents'
/// names are normalised (lowercase, port stripped, empties dropped).
pub fn normalize_names(raw: &[String]) -> Vec<String> {
    let mut names: Vec<String> = raw
        .iter()
        .filter_map(|name| RouteKey::new(Some(name), "/").host)
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Origin equality as the membership document means it: scheme + authority,
/// case-insensitive, trailing slash ignored.
pub fn same_origin(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.trim().trim_end_matches('/').to_ascii_lowercase();
    norm(a) == norm(b)
}

/// Whether this doorway serves the same bundle as the name's holder for the
/// request in hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadAgreement {
    /// Both sides state a served bundle for the covering mount, and it is the
    /// same one.
    Same,
    /// The heads differ — OR either side states none, so sameness cannot be
    /// shown. A member never serves a name "as the same" on a comparison it
    /// could not make.
    Differs,
}

/// Compare the bundle this doorway would serve with the one the holder
/// advertises for the same (host, path).
///
/// The served bundle is the browser bundle blob address each doorway's
/// bundle-heads reconciler last read for the mounted EPR on the contract's
/// channel (`routes::coherence::served_bundle_for`), advertised per mount as
/// `EprHeadFingerprint::served_bundle` (`servedBundle` on the wire). The
/// head itself is declared by the peer; a doorway only reports which bundle
/// it took up. Not the EPR id: two doorways mounting
/// the same EPR can still serve different versions of it while one lags.
/// Absent on either side is `Differs`.
pub fn head_agreement(own_head: Option<&str>, holder_head: Option<&str>) -> HeadAgreement {
    match (own_head, holder_head) {
        (Some(own), Some(theirs)) if own == theirs => HeadAgreement::Same,
        _ => HeadAgreement::Differs,
    }
}

/// What the request itself allows, independent of membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestShape {
    /// A read this doorway may forward one hop (GET, not a service path, no
    /// inbound federation hop). Anything else is answered here.
    pub relayable: bool,
    /// A path a refusal may answer. The doorway's own operational surfaces
    /// (health, `.well-known`, federation, admin, websocket upgrades — the
    /// admission / declared-shed allow-list) always answer, whatever `Host`
    /// the caller used, so a deployment whose own origin IS a public name can
    /// never refuse its own health check.
    pub refusable: bool,
}

/// What this doorway does for a request whose `Host` is a public name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameDecision {
    /// Serve it here, under this doorway's own contract.
    Serve,
    /// Hand the request to the name's holder, one hop. If the holder does not
    /// answer, the caller serves its own bytes (the name staying up outranks
    /// its freshness).
    Relay(PublicNameMember),
    /// 421 Misdirected Request: this doorway does not stand for the name.
    Refuse {
        name: String,
        /// The names this doorway IS currently a member of.
        member_of: Vec<String>,
        /// The members of the asked name — where the client can go instead.
        members: Vec<PublicNameMember>,
    },
}

/// **The routing decision.** PURE: the membership, this doorway's identity,
/// what the request allows and the head comparison are all handed in.
///
/// Returns `None` when `host` names no public name this doorway knows of —
/// the rule does not apply and the request is dispatched as it always was
/// (a doorway's own origin, `localhost`, an unrelated hostname).
///
/// For a known public name, whose HOLDER is the document's first member (the
/// writer's owner order, the same stable tiebreak the relay selector ends on):
///
/// 1. **No members at all → `Serve`.** An empty document says nobody is
///    currently advertised, not that everybody is misdirected; refusing would
///    turn a membership outage into a total one.
/// 2. **The holder → `Serve`.** Its answer is the name's reference answer.
/// 3. **Another member → `Serve`** when it serves the SAME bundle as the
///    holder advertises for the covering mount; **`Relay(holder)`** when the
///    heads differ or either is unknown, so a visitor never receives a version
///    a lagging member only believes is current.
/// 4. **A candidate that is not currently a member** (withdrawn by the serving
///    probe: restarting, a flaked probe, a restarted beacon leg) →
///    **`Relay(holder)`**. It is one of the name's doorways, momentarily not
///    advertised — never misdirected.
/// 5. **Neither member nor candidate → `Refuse`** (421), unless the path is an
///    operational surface (`shape.refusable == false`), which is served.
///
/// A `Relay` for a request that cannot be relayed (`shape.relayable ==
/// false`) is answered here instead: `Serve`. `head_vs_holder` is consulted
/// only in case 3 and only for a relayable request, at most once.
pub fn served_under_standing(
    host: Option<&str>,
    membership: &PublicNameMembership,
    me: &SelfIdentity<'_>,
    shape: RequestShape,
    head_vs_holder: impl FnOnce(&PublicNameMember) -> HeadAgreement,
) -> Option<NameDecision> {
    let doc = membership.document(host)?;
    let Some(holder) = doc.members.first() else {
        return Some(NameDecision::Serve);
    };
    let relay_or_serve = |holder: &PublicNameMember| {
        if shape.relayable {
            NameDecision::Relay(holder.clone())
        } else {
            NameDecision::Serve
        }
    };
    if doc.members.iter().any(|member| me.is(member)) {
        if me.is(holder) || !shape.relayable {
            return Some(NameDecision::Serve);
        }
        return Some(match head_vs_holder(holder) {
            HeadAgreement::Same => NameDecision::Serve,
            HeadAgreement::Differs => relay_or_serve(holder),
        });
    }
    if me.stands_for(&doc.name) {
        return Some(relay_or_serve(holder));
    }
    if !shape.refusable {
        return Some(NameDecision::Serve);
    }
    Some(NameDecision::Refuse {
        name: doc.name.clone(),
        member_of: membership.names_member_of(me),
        members: doc.members.clone(),
    })
}

/// The `x-elohim-served-by` value for a response to a public-name request:
/// the id of the doorway whose bytes these are. A response this doorway
/// relayed carries `x-elohim-name-route: relay:<holder id>` and is the
/// holder's; every other response is this doorway's own.
///
/// Under a public name the header names WHO served, not WHERE to go next:
/// the name is the address a client should keep asking, so there is no
/// origin to stick to. (A request that names a doorway's own origin keeps
/// the relay tier's origin-valued header, unchanged.)
pub fn served_by_value(name_route: Option<&str>, self_doorway_id: &str) -> String {
    name_route
        .and_then(|route| route.strip_prefix("relay:"))
        .map(str::trim)
        .filter(|holder| !holder.is_empty())
        .unwrap_or(self_doorway_id)
        .to_string()
}

/// Read [`served_by_value`]'s input from a response's headers.
pub fn served_by_for_response<B>(response: &Response<B>, self_doorway_id: &str) -> String {
    served_by_value(
        response
            .headers()
            .get(NAME_ROUTE_HEADER)
            .and_then(|value| value.to_str().ok()),
        self_doorway_id,
    )
}

/// The 421 a non-member answers. JSON, `no-store`: membership moves with the
/// serving probe, so a refusal must not outlive the reading that produced it.
pub fn misdirected_response(
    name: &str,
    doorway_id: &str,
    member_of: &[String],
    members: &[PublicNameMember],
) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": MISDIRECTED_ERROR,
        "name": name,
        "doorway": doorway_id,
        "message": format!(
            "This doorway does not stand for \"{name}\"; ask one of the name's members."
        ),
        "memberOf": member_of,
        "members": members
            .iter()
            .map(|m| serde_json::json!({ "owner": m.owner, "origin": m.origin }))
            .collect::<Vec<_>>(),
    });
    Response::builder()
        .status(StatusCode::MISDIRECTED_REQUEST)
        .header("content-type", "application/json")
        .header("cache-control", "no-store")
        .body(Full::new(Bytes::from(body.to_string())))
        .expect("static misdirected response is infallible")
}

// =============================================================================
// The registry — the doorway's current reading of the membership directory.
// =============================================================================

/// This doorway's latest reading of the membership directory. Swapped whole
/// on every refresh; a request clones the `Arc` and never holds the lock.
#[derive(Debug, Default)]
pub struct PublicNameRegistry {
    current: RwLock<Arc<PublicNameMembership>>,
}

impl PublicNameRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Arc<PublicNameMembership> {
        Arc::clone(&self.current.read().unwrap_or_else(PoisonError::into_inner))
    }

    /// Install a reading; returns whether it differs from the previous one.
    pub fn install(&self, membership: PublicNameMembership) -> bool {
        let mut current = self.current.write().unwrap_or_else(PoisonError::into_inner);
        if **current == membership {
            return false;
        }
        *current = Arc::new(membership);
        true
    }
}

/// Parse one document. Anything that is not a membership document (the
/// household's `authority.json` declaration, a half-written file) is `None`.
pub fn parse_document(bytes: &[u8]) -> Option<PublicNameDocument> {
    serde_json::from_slice::<PublicNameDocument>(bytes).ok()
}

/// Read every `*.json` membership document in `dir`, in file-name order.
/// A missing or unreadable directory reads as no public names at all.
pub async fn read_membership_dir(dir: &Path) -> PublicNameMembership {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(error) => {
            debug!(dir = %dir.display(), %error, "public names: membership directory unreadable");
            return PublicNameMembership::default();
        }
    };
    let mut paths = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    paths.sort();
    let mut documents = Vec::new();
    for path in paths {
        if let Ok(bytes) = tokio::fs::read(&path).await {
            if let Some(doc) = parse_document(&bytes) {
                documents.push(doc);
            }
        }
    }
    PublicNameMembership::from_documents(documents)
}

/// Load `dir` once now, then keep re-reading it every [`MEMBERSHIP_REFRESH`].
pub async fn start_membership_refresh(registry: Arc<PublicNameRegistry>, dir: PathBuf) {
    let first = read_membership_dir(&dir).await;
    info!(
        dir = %dir.display(),
        names = first.documents.len(),
        "public names: membership directory configured — a public name this doorway does not stand for is refused"
    );
    registry.install(first);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(MEMBERSHIP_REFRESH);
        interval.tick().await;
        loop {
            interval.tick().await;
            let reading = read_membership_dir(&dir).await;
            if registry.install(reading) {
                debug!(dir = %dir.display(), "public names: membership changed");
            }
        }
    });
}

/// Warn when a membership directory is configured but this doorway declares
/// no public name it stands for. It then answers 421 for every name whose
/// document does not list it at this moment — including its OWN names while
/// the serving probe has it withdrawn. Right for a doorway that stands for
/// nothing (the household's gamma), wrong for a name's doorway, so a warning
/// and not a refusal.
pub fn warn_if_standing_for_nothing(public_names: &[String], doorway_id: &str) {
    if normalize_names(public_names).is_empty() {
        warn!(
            doorway_id,
            "public names: DOORWAY_MEMBERSHIP_DIR is set but DOORWAY_PUBLIC_NAMES is not — this \
             doorway stands for no public name, so whenever a membership document does not list \
             it (including while its own name's probe has it withdrawn) it answers 421 for that name"
        );
    }
}

/// Warn when a membership directory is configured but this doorway declares
/// no origin: it could then recognise itself only by an owner equal to its
/// doorway id, and would refuse every name whose owner slug differs.
pub fn warn_if_blind(origins: &[String], doorway_id: &str) {
    if origins.is_empty() {
        warn!(
            doorway_id,
            "public names: DOORWAY_MEMBERSHIP_DIR is set but DOORWAY_URL is not — this doorway \
             recognises its membership entries only by owner == doorway id"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(owner: &str, origin: &str) -> PublicNameMember {
        PublicNameMember {
            owner: owner.to_string(),
            origin: origin.to_string(),
        }
    }

    fn doc(name: &str, members: Vec<PublicNameMember>) -> PublicNameDocument {
        PublicNameDocument {
            name: name.to_string(),
            members,
        }
    }

    /// The household as `just mesh start` stages it: both names carry alpha
    /// and apex, in owner order; gamma stands for neither.
    fn household() -> PublicNameMembership {
        let both = || {
            vec![
                member("alpha", "http://localhost:8888"),
                member("apex", "http://localhost:8889"),
            ]
        };
        PublicNameMembership::from_documents(vec![
            doc("elohim.local", both()),
            doc("alpha.elohim.local", both()),
        ])
    }

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    const READ: RequestShape = RequestShape {
        relayable: true,
        refusable: true,
    };

    fn never_asked(_: &PublicNameMember) -> HeadAgreement {
        panic!("the head comparison is not consulted for this decision")
    }

    #[test]
    fn name_routing_a_host_that_is_no_public_name_is_not_this_rule() {
        let (o, c) = (
            strings(&["http://localhost:8889/"]),
            strings(&["elohim.local"]),
        );
        let me = SelfIdentity {
            doorway_id: "apex-elohim-host",
            origins: &o,
            candidate_names: &c,
        };
        for host in [
            None,
            Some("localhost:8889"),
            Some("nonsense.example"),
            Some(""),
        ] {
            assert_eq!(
                served_under_standing(host, &household(), &me, READ, never_asked),
                None
            );
        }
        let empty = PublicNameMembership::default();
        assert_eq!(
            served_under_standing(Some("elohim.local"), &empty, &me, READ, never_asked),
            None,
            "no membership configured: today's behaviour"
        );
    }

    #[test]
    fn name_routing_the_holder_serves_its_name() {
        let (o, c) = (strings(&["http://localhost:8888"]), strings(&[]));
        let me = SelfIdentity {
            doorway_id: "alpha-elohim-host",
            origins: &o,
            candidate_names: &c,
        };
        assert_eq!(
            served_under_standing(
                Some("alpha.elohim.local"),
                &household(),
                &me,
                READ,
                never_asked
            ),
            Some(NameDecision::Serve)
        );
    }

    #[test]
    fn name_routing_a_member_with_the_same_head_serves_under_its_own_contract() {
        let (o, c) = (strings(&["http://localhost:8889"]), strings(&[]));
        let me = SelfIdentity {
            doorway_id: "apex-elohim-host",
            origins: &o,
            candidate_names: &c,
        };
        let mut asked_about = None;
        let decision = served_under_standing(
            Some("ALPHA.elohim.local:8889"),
            &household(),
            &me,
            READ,
            |h| {
                asked_about = Some(h.owner.clone());
                HeadAgreement::Same
            },
        );
        assert_eq!(decision, Some(NameDecision::Serve));
        assert_eq!(
            asked_about.as_deref(),
            Some("alpha"),
            "compared against the holder"
        );
    }

    #[test]
    fn name_routing_a_member_behind_the_holder_relays_to_it() {
        let (o, c) = (strings(&["http://localhost:8889"]), strings(&[]));
        let me = SelfIdentity {
            doorway_id: "apex-elohim-host",
            origins: &o,
            candidate_names: &c,
        };
        assert_eq!(
            served_under_standing(Some("elohim.local"), &household(), &me, READ, |_| {
                HeadAgreement::Differs
            }),
            Some(NameDecision::Relay(member(
                "alpha",
                "http://localhost:8888"
            )))
        );
    }

    #[test]
    fn name_routing_a_member_request_that_cannot_relay_is_served_here() {
        let (o, c) = (strings(&["http://localhost:8889"]), strings(&[]));
        let me = SelfIdentity {
            doorway_id: "apex-elohim-host",
            origins: &o,
            candidate_names: &c,
        };
        let write = RequestShape {
            relayable: false,
            refusable: true,
        };
        assert_eq!(
            served_under_standing(Some("elohim.local"), &household(), &me, write, never_asked),
            Some(NameDecision::Serve)
        );
    }

    #[test]
    fn name_routing_head_agreement_needs_both_declared_heads_equal() {
        assert_eq!(
            head_agreement(Some("bafk-a"), Some("bafk-a")),
            HeadAgreement::Same
        );
        assert_eq!(
            head_agreement(Some("bafk-a"), Some("bafk-b")),
            HeadAgreement::Differs
        );
        assert_eq!(head_agreement(None, Some("bafk-a")), HeadAgreement::Differs);
        assert_eq!(head_agreement(Some("bafk-a"), None), HeadAgreement::Differs);
        assert_eq!(head_agreement(None, None), HeadAgreement::Differs);
    }

    #[test]
    fn name_routing_a_withdrawn_candidate_relays_to_the_holder() {
        let membership = PublicNameMembership::from_documents(vec![doc(
            "elohim.local",
            vec![member("apex", "http://localhost:8889")],
        )]);
        let (o, c) = (
            strings(&["http://localhost:8888"]),
            strings(&["Elohim.Local"]),
        );
        let c = normalize_names(&c);
        let me = SelfIdentity {
            doorway_id: "alpha-elohim-host",
            origins: &o,
            candidate_names: &c,
        };
        assert_eq!(
            served_under_standing(Some("elohim.local"), &membership, &me, READ, never_asked),
            Some(NameDecision::Relay(member("apex", "http://localhost:8889")))
        );
        let write = RequestShape {
            relayable: false,
            refusable: true,
        };
        assert_eq!(
            served_under_standing(Some("elohim.local"), &membership, &me, write, never_asked),
            Some(NameDecision::Serve),
            "a withdrawn candidate answers what it cannot relay"
        );
    }

    #[test]
    fn name_routing_a_withdrawn_candidate_with_no_holder_serves() {
        let membership =
            PublicNameMembership::from_documents(vec![doc("elohim.local", Vec::new())]);
        let (o, c) = (
            strings(&["http://localhost:8888"]),
            strings(&["elohim.local"]),
        );
        let me = SelfIdentity {
            doorway_id: "alpha-elohim-host",
            origins: &o,
            candidate_names: &c,
        };
        assert_eq!(
            served_under_standing(Some("elohim.local"), &membership, &me, READ, never_asked),
            Some(NameDecision::Serve)
        );
    }

    #[test]
    fn name_routing_an_empty_member_list_never_refuses() {
        let membership =
            PublicNameMembership::from_documents(vec![doc("elohim.local", Vec::new())]);
        let (o, c) = (strings(&["http://localhost:8890"]), strings(&[]));
        let me = SelfIdentity {
            doorway_id: "gamma-elohim-host",
            origins: &o,
            candidate_names: &c,
        };
        assert_eq!(
            served_under_standing(Some("elohim.local"), &membership, &me, READ, never_asked),
            Some(NameDecision::Serve)
        );
    }

    #[test]
    fn name_routing_a_non_candidate_refuses_and_names_the_members() {
        let (o, c) = (strings(&["http://localhost:8890"]), strings(&[]));
        let me = SelfIdentity {
            doorway_id: "gamma-elohim-host",
            origins: &o,
            candidate_names: &c,
        };
        match served_under_standing(Some("elohim.local"), &household(), &me, READ, never_asked) {
            Some(NameDecision::Refuse {
                name,
                member_of,
                members,
            }) => {
                assert_eq!(name, "elohim.local");
                assert!(
                    member_of.is_empty(),
                    "gamma is a member of no household name"
                );
                assert_eq!(
                    members,
                    vec![
                        member("alpha", "http://localhost:8888"),
                        member("apex", "http://localhost:8889"),
                    ]
                );
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn name_routing_a_non_candidate_still_answers_its_operational_paths() {
        let (o, c) = (strings(&["http://localhost:8890"]), strings(&[]));
        let me = SelfIdentity {
            doorway_id: "gamma-elohim-host",
            origins: &o,
            candidate_names: &c,
        };
        let health = RequestShape {
            relayable: false,
            refusable: false,
        };
        assert_eq!(
            served_under_standing(Some("elohim.local"), &household(), &me, health, never_asked),
            Some(NameDecision::Serve)
        );
    }

    #[test]
    fn name_routing_owner_matches_self_only_when_no_origin_is_declared() {
        let membership = PublicNameMembership::from_documents(vec![doc(
            "elohim.host",
            vec![member("apex-elohim-host", "https://203.0.113.9")],
        )]);
        let none: Vec<String> = Vec::new();
        let me = SelfIdentity {
            doorway_id: "apex-elohim-host",
            origins: &none,
            candidate_names: &none,
        };
        assert_eq!(
            served_under_standing(Some("elohim.host"), &membership, &me, READ, never_asked),
            Some(NameDecision::Serve)
        );
        let declared = strings(&["https://198.51.100.7"]);
        let elsewhere = SelfIdentity {
            doorway_id: "apex-elohim-host",
            origins: &declared,
            candidate_names: &none,
        };
        assert!(
            matches!(
                served_under_standing(
                    Some("elohim.host"),
                    &membership,
                    &elsewhere,
                    READ,
                    never_asked
                ),
                Some(NameDecision::Refuse { .. })
            ),
            "with an origin declared, an owner slug equal to the id is not this doorway"
        );
    }

    #[test]
    fn name_routing_served_by_names_the_doorway_whose_bytes_they_are() {
        assert_eq!(
            served_by_value(None, "apex-elohim-host"),
            "apex-elohim-host"
        );
        assert_eq!(
            served_by_value(Some("relay:alpha-elohim-host"), "apex-elohim-host"),
            "alpha-elohim-host"
        );
        assert_eq!(
            served_by_value(Some("relay:"), "apex-elohim-host"),
            "apex-elohim-host"
        );
        assert_eq!(
            served_by_value(Some("local"), "apex-elohim-host"),
            "apex-elohim-host"
        );
    }

    #[tokio::test]
    async fn name_routing_the_refusal_is_a_421_naming_members() {
        use http_body_util::BodyExt;
        let response = misdirected_response(
            "elohim.local",
            "gamma-elohim-host",
            &[],
            &[
                member("alpha", "http://localhost:8888"),
                member("apex", "http://localhost:8889"),
            ],
        );
        assert_eq!(response.status(), StatusCode::MISDIRECTED_REQUEST);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"], MISDIRECTED_ERROR);
        assert_eq!(body["name"], "elohim.local");
        assert_eq!(
            body["members"],
            serde_json::json!([
                {"owner": "alpha", "origin": "http://localhost:8888"},
                {"owner": "apex", "origin": "http://localhost:8889"}
            ])
        );
    }

    #[test]
    fn name_routing_only_membership_documents_parse() {
        let doc = br#"{"name":"elohim.local","members":[{"owner":"alpha","origin":"http://localhost:8888","updated_at":"x"}],"updated_at":"x"}"#;
        assert_eq!(parse_document(doc).map(|d| d.members.len()), Some(1));
        let authority =
            br#"{"kind":"relay-addr-beacon-file-sink","publicName":"elohim.local","lanes":[]}"#;
        assert_eq!(parse_document(authority), None);
        assert_eq!(parse_document(b"{\"name\":\"half"), None);
    }

    #[tokio::test]
    async fn name_routing_reads_the_membership_directory() {
        let dir = std::env::temp_dir().join(format!("doorway-public-names-{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(
            dir.join("elohim.local.json"),
            br#"{"name":"elohim.local","members":[{"owner":"apex","origin":"http://localhost:8889"}]}"#,
        )
        .await
        .unwrap();
        tokio::fs::write(dir.join("authority.json"), br#"{"kind":"x"}"#)
            .await
            .unwrap();
        tokio::fs::write(dir.join("elohim.local.json.lock"), b"")
            .await
            .unwrap();
        let membership = read_membership_dir(&dir).await;
        assert!(membership.document(Some("elohim.local")).is_some());
        assert_eq!(membership.documents.len(), 1);

        let registry = PublicNameRegistry::new();
        assert!(registry.snapshot().is_empty());
        assert!(registry.install(membership.clone()));
        assert!(
            !registry.install(membership),
            "an unchanged reading is not a change"
        );

        let missing = read_membership_dir(&dir.join("absent")).await;
        assert!(missing.is_empty());
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
