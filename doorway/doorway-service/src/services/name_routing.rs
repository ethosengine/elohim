//! Name routing — the federated one-hop relay for a name this doorway does not serve.
//!
//! # Doctrine (2026-09-12 operator ruling, WS3 of the doorway-federation sprint)
//!
//! A public name is a **DHT fact, not a DNS fact**. Internet DNS only has to
//! name doorways that can ROUTE; which doorway *serves* a given root is the set
//! of doorways holding a live projection contract for it, with liveness folded
//! on top. A doorway that does not host a requested name reads that registry,
//! picks the nearest live holder (health first, owner order until an RTT /
//! capacity advertisement lands), forwards ONE hop with the loop-prevention
//! header, and hands the client the holder's origin so the shipped sticky
//! client goes direct next time.
//!
//! This module is the **registry fold + the relay decision**. It owns no
//! endpoint truth and no name truth: every contract it folds is a projection of
//! a sibling's own `project-epr` commitments (read over the federation
//! coherence surface, which is that sibling's live `EprRouter` head set), and
//! every origin is a peer URL discovered through doorway registration /
//! federation peer discovery. Category C, Operational — the whole table is
//! reconstructable from the next discovery tick, so nothing here is persisted
//! and nothing here is notarized.
//!
//! # What this is NOT
//!
//! It is not blob fan-out (`doorway/CLAUDE.md` §"No Blob Fan-Out"). It never
//! iterates *storage peers* asking who holds bytes; it forwards ONE request to
//! ONE sibling *doorway* that holds a contract for the requested name, and that
//! sibling answers from its own single storage target under its own contract.
//! The predecessor of this module — `federation::fetch_from_remote_doorway` —
//! did iterate publishers for a blob hash, was never called by anything, and
//! was deleted when this landed.
//!
//! # Budget
//!
//! Exactly ONE hop. A request that arrives carrying [`FEDERATION_HOP_HEADER`]
//! is answered with the local verdict and never forwarded again, so a cycle of
//! doorways cannot amplify one client request into a storm.

use std::collections::HashMap;
use std::sync::RwLock;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use tracing::{debug, warn};

/// Loop-prevention header carried on every outbound relay. Inherited verbatim
/// from the deleted `fetch_from_remote_doorway` so any peer already honouring
/// it keeps working. Presence on an INBOUND request means "you are already the
/// second doorway" — answer locally, never forward.
pub const FEDERATION_HOP_HEADER: &str = "x-federation-hop";

/// Response header handing the client the origin that actually served the
/// bytes, so the shipped sticky client (`doorwayFallbacks` + the
/// `apiBaseUrlInterceptor`) can go direct on the next request instead of
/// paying the relay hop forever.
pub const SERVED_BY_HEADER: &str = "x-elohim-served-by";

/// Marks a response as relayed rather than locally served — the honest
/// provenance marker a cache or an operator can read.
pub const NAME_ROUTE_HEADER: &str = "x-elohim-name-route";

/// The relay budget. One. See the module doc.
pub const MAX_FEDERATION_HOPS: u8 = 1;

/// The standing the serving doorway resolved, relayed verbatim. One name for
/// this header across the crate — it is minted by
/// [`crate::services::serve_eligibility`] and this module only carries it.
pub use crate::services::serve_eligibility::STANDING_HEADER;

/// The bundle-freshness marker a holder stated, relayed verbatim.
pub const BUNDLE_HEADER: &str = "x-elohim-bundle";

/// A holder's **authoritative refusal**. Not a failed attempt: the holder is
/// the doorway that holds the contract for this name, so when it says the
/// requester may not have it, that IS the answer to the request. Trying the
/// next holder would be shopping the refusal around the federation until some
/// doorway with a staler projection said yes — reach-laundering by retry.
const AUTHORITATIVE_REFUSAL: u16 = 403;

/// How live a candidate holder looked at the last discovery tick.
///
/// Declaration order IS the preference order (`derive(Ord)`): a serving holder
/// is tried before an uncertain one, which is tried before one we have seen
/// shed, which is tried before one we could not reach at all. An unreachable
/// holder is kept in the fold rather than dropped — this classification is a
/// ≤60s-stale probe cache, and dropping on a stale verdict would make a name
/// unservable for a doorway that is actually up.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HolderLiveness {
    /// The peer answered its coherence manifest — it is serving.
    Serving,
    /// The peer answered 200 with a body we could not read (mixed version), or
    /// we have no verdict for it yet. The DEFAULT: an unclassified holder is
    /// neither trusted ahead of a known-serving one nor written off.
    #[default]
    Uncertain,
    /// The peer answered a relay with 503 (catching-up / admission shed). Only
    /// a relay attempt can observe this; the discovery probe cannot tell a shed
    /// from a death.
    Shedding,
    /// The peer did not answer.
    Unreachable,
}

/// The key the fold matches contracts against.
///
/// **Routing dimensions.** Today only `path` discriminates: every contract is
/// an any-host contract, so `host` is carried and matched but never narrows.
/// It exists NOW so the next rung — hostnames as head channels (`elohim.host`
/// = converged head at commons reach, `alpha.elohim.host` = candidate head at
/// stewards reach, both served by every doorway) — adds host WITHOUT re-keying
/// the fold, its table, or its callers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RouteKey {
    /// The host the client asked for, lowercased and port-stripped. `None`
    /// when the request named none.
    pub host: Option<String>,
    /// The request path.
    pub path: String,
}

impl RouteKey {
    /// Build a key, normalising the host (lowercase, port stripped, empty →
    /// `None`) so host comparison is never case- or port-sensitive.
    pub fn new(host: Option<&str>, path: &str) -> Self {
        Self {
            host: host.and_then(normalize_host),
            path: path.to_string(),
        }
    }

    /// A key that names no host — today's every request, and the shape the
    /// path-only tests use.
    pub fn path_only(path: &str) -> Self {
        Self::new(None, path)
    }
}

fn normalize_host(raw: &str) -> Option<String> {
    let host = raw.trim().split(':').next()?.trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_ascii_lowercase())
    }
}

/// How a holder is reached once selected.
///
/// An ENUM, never a bool: the next rung adds a redirect mode beside the proxy
/// one, and a bool would have to be re-read at every call site the day a third
/// mode appears.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RelayMode {
    /// WIRED. This doorway fetches from the holder and returns the bytes, so
    /// the client's connection, session and origin are untouched.
    #[default]
    Proxy,
    /// DECLARED, NOT IMPLEMENTED (next rung). Answer a redirect to the
    /// holder's origin and let the client go direct — cheaper for this doorway,
    /// but it moves the origin the client talks to, which is a session and
    /// CORS decision, not a routing one. Until it is implemented, a holder
    /// carrying this mode is SKIPPED with a warning, never silently proxied:
    /// a declared mode that quietly does something else is worse than no mode.
    Redirect,
}

/// The selector terms, in precedence order. **Index IS precedence**, and
/// `selector_rank` returns one tuple element per term in exactly this order.
///
/// Adding a term is: add the variant here, add its element to `selector_rank`
/// at the matching position, and fill it in. No call site moves, no re-keying,
/// and the existing terms keep their relative precedence by construction.
///
/// | Term | Today |
/// |------|-------|
/// | `Liveness` | WIRED — last observed [`HolderLiveness`] |
/// | `ReachStanding` | declared, constant — requester standing × the EPR's declared reach |
/// | `Nearest` | declared, constant — attested RTT, with region as a selector BESIDE it (never a separate authority) |
/// | `Weight` | declared, constant — holder-advertised capacity |
/// | `OwnerOrder` | WIRED — registry order, the stable final tiebreak |
pub const SELECTOR_TERMS: &[SelectorTerm] = &[
    SelectorTerm::Liveness,
    SelectorTerm::ReachStanding,
    SelectorTerm::Nearest,
    SelectorTerm::Weight,
    SelectorTerm::OwnerOrder,
];

/// One term of the holder selector. See [`SELECTOR_TERMS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectorTerm {
    Liveness,
    ReachStanding,
    Nearest,
    Weight,
    OwnerOrder,
}

/// The value an unwired selector term contributes: the same for every holder,
/// so the term is a no-op until it is filled in. Named rather than a bare `0`
/// so a reader can tell "not yet a term" from "ranked best".
const TERM_NOT_YET_WIRED: u32 = 0;

/// The comparable rank of one holder under [`SELECTOR_TERMS`]. One tuple
/// element per term, in term order.
fn selector_rank(holder: &NameHolder, owner_order: usize) -> (u8, u32, u32, u32, usize) {
    (
        // Liveness — WIRED.
        holder.liveness as u8,
        // ReachStanding — declared, not yet a term.
        TERM_NOT_YET_WIRED,
        // Nearest (attested RTT; region beside it) — declared, not yet a term.
        TERM_NOT_YET_WIRED,
        // Weight — declared, not yet a term.
        TERM_NOT_YET_WIRED,
        // OwnerOrder — WIRED, the stable final tiebreak.
        owner_order,
    )
}

/// One sibling doorway's live projection contract for one mounted root.
///
/// `url_path` is the sibling's mount (`EprProjectionView.url_path`), `origin`
/// is the sibling's gateway URL, `doorway_id` is its self-reported identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolderContract {
    pub doorway_id: String,
    pub origin: String,
    pub url_path: String,
    /// The host this contract is bound to. `None` = ANY host — which is every
    /// contract today, because a coherence head set carries no host. The next
    /// rung populates it (hostnames as head channels) and the fold already
    /// matches on it.
    pub host: Option<String>,
}

impl HolderContract {
    /// A contract bound to no particular host — today's only shape.
    pub fn any_host(doorway_id: &str, origin: &str, url_path: &str) -> Self {
        Self {
            doorway_id: doorway_id.to_string(),
            origin: origin.to_string(),
            url_path: url_path.to_string(),
            host: None,
        }
    }
}

/// A folded candidate: one doorway that holds a contract covering the requested
/// path, carrying the mount that matched and the liveness that ordered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameHolder {
    pub doorway_id: String,
    pub origin: String,
    pub url_path: String,
    /// The host the matched contract was bound to, `None` for an any-host
    /// contract. Carried so a relay can be reasoned about per channel once
    /// hosts become head channels.
    pub host: Option<String>,
    pub liveness: HolderLiveness,
    /// How this holder is reached. [`RelayMode::Proxy`] for every holder today.
    pub relay_mode: RelayMode,
}

/// True iff `mount` covers `request_path` on a segment boundary — `/lamad`
/// covers `/lamad`, `/lamad/`, `/lamad/x` but NOT `/lamadx`; `/` covers
/// everything (the universal root).
///
/// Deliberately identical to `EprRouter::path_matches_prefix`: the fold must
/// agree with local dispatch about what "serving this name" means, or a doorway
/// could relay a path it would have served itself.
pub fn mount_covers(mount: &str, request_path: &str) -> bool {
    if mount == "/" {
        return true;
    }
    request_path == mount || request_path.starts_with(&format!("{mount}/"))
}

/// True iff a contract bound to `contract_host` answers for `requested_host`.
///
/// An any-host contract (`None`) answers for every host — today's every
/// contract, so this term currently never narrows anything. A host-BOUND
/// contract answers only for that exact host (case-insensitive; the key is
/// already port-stripped).
pub fn host_matches(contract_host: Option<&str>, requested_host: Option<&str>) -> bool {
    match contract_host {
        None => true,
        Some(bound) => requested_host.is_some_and(|asked| asked.eq_ignore_ascii_case(bound)),
    }
}

/// How specific a mount is: host-bound beats any-host, then the longer path
/// wins. ONE definition, used both to pick a doorway's best contract and to
/// compare the federation's best against this doorway's own.
pub fn mount_specificity(host: Option<&str>, url_path: &str) -> (u8, usize) {
    (u8::from(host.is_some()), url_path.len())
}

fn specificity(contract: &HolderContract) -> (u8, usize) {
    mount_specificity(contract.host.as_deref(), &contract.url_path)
}

/// The mount THIS doorway would serve a request from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMount {
    pub host: Option<String>,
    pub url_path: String,
}

impl LocalMount {
    /// Today's shape: a local projection carries no host.
    pub fn path_only(url_path: &str) -> Self {
        Self {
            host: None,
            url_path: url_path.to_string(),
        }
    }

    fn specificity(&self) -> (u8, usize) {
        mount_specificity(self.host.as_deref(), &self.url_path)
    }
}

/// Keep only holders whose mount is **strictly more specific** than this
/// doorway's own best match.
///
/// The catch-all problem, measured on the household mesh (run
/// `20260912T211541Z`): doorway B holds the landing app at mount `/`, so
/// `GET /nrt-garden/` answered 200 with the landing shell and the relay tier —
/// which only fired on a local 404/shed — was never consulted. B's name-route
/// table DID hold A's `/nrt-garden` contract at the time: B's own cross-edge
/// divergence WARN at 21:16:34Z names `divergent_paths: ["/nrt-garden"]`, i.e.
/// A's head set carried it and B's did not. Serving our own catch-all for a
/// name a sibling holds the specific contract for is answering for somebody
/// else's name.
///
/// `local = None` means this doorway has no mount for the request, or cannot
/// serve from the one it has (a 404 or a shed) — every holder qualifies, which
/// is exactly today's behaviour.
///
/// **Ties go to LOCAL.** Equal specificity means we hold the same contract the
/// sibling does; serving it ourselves is cheaper, keeps the client's origin,
/// and cannot loop.
pub fn holders_more_specific_than(
    local: Option<&LocalMount>,
    holders: Vec<NameHolder>,
) -> Vec<NameHolder> {
    let Some(local) = local else {
        return holders;
    };
    let local_rank = local.specificity();
    holders
        .into_iter()
        .filter(|holder| mount_specificity(holder.host.as_deref(), &holder.url_path) > local_rank)
        .collect()
}

/// **The registry fold.** Candidate holders for a [`RouteKey`], matched
/// host-first then path, ordered by [`SELECTOR_TERMS`].
///
/// Inputs (all Category-C projections of substrate facts, never doorway-local
/// policy):
/// 1. `contracts` — live `project-epr` contracts held by OTHER doorways, in the
///    order the registry handed them (that order IS "owner order").
/// 2. `liveness` — `doorway_id` → last observed [`HolderLiveness`].
/// 3. `self_doorway_id` — excluded; a doorway never relays to itself.
///
/// Rules:
/// - a contract qualifies iff its mount [`mount_covers`] the request path;
/// - one entry per doorway, keeping its MOST SPECIFIC (longest) covering mount;
/// - order = liveness rank first, then first-appearance (owner) order — the
///   sort is stable, so equal-liveness holders keep registry order.
///
/// TODO(nearest): "nearest" is currently health-then-owner-order. The real term
/// is attested RTT — the peer-health probe already records
/// `RecordHealthAttestationInput.response_time_ms` into the infrastructure DNA,
/// and the ruling names region (the snapshot's local/regional/global) as a
/// selector term BESIDE RTT, never a separate authority. When that
/// advertisement lands, it sorts here, between liveness and owner order.
pub fn fold_candidate_holders(
    key: &RouteKey,
    contracts: &[HolderContract],
    liveness: &HashMap<String, HolderLiveness>,
    self_doorway_id: &str,
) -> Vec<NameHolder> {
    // `(holder, most-specific matching contract)`, in first-appearance (owner)
    // order — the index into this vec IS the OwnerOrder selector term.
    let mut holders: Vec<(NameHolder, (u8, usize))> = Vec::new();

    for contract in contracts {
        if contract.doorway_id == self_doorway_id {
            continue;
        }
        if contract.origin.trim().is_empty() || contract.doorway_id.trim().is_empty() {
            continue;
        }
        // Host first, then path — the match order the next rung needs.
        if !host_matches(contract.host.as_deref(), key.host.as_deref()) {
            continue;
        }
        if !mount_covers(&contract.url_path, &key.path) {
            continue;
        }
        let rank = specificity(contract);
        match holders
            .iter_mut()
            .find(|(h, _)| h.doorway_id == contract.doorway_id)
        {
            // Same doorway seen again: keep its most specific matching contract
            // (host-bound over any-host, then the longer mount).
            Some((existing, existing_rank)) => {
                if rank > *existing_rank {
                    existing.url_path = contract.url_path.clone();
                    existing.host = contract.host.clone();
                    *existing_rank = rank;
                }
            }
            None => holders.push((
                NameHolder {
                    doorway_id: contract.doorway_id.clone(),
                    origin: contract.origin.trim_end_matches('/').to_string(),
                    url_path: contract.url_path.clone(),
                    host: contract.host.clone(),
                    liveness: liveness
                        .get(&contract.doorway_id)
                        .copied()
                        .unwrap_or_default(),
                    // The next rung derives this from the contract; every
                    // contract is proxy-reached today.
                    relay_mode: RelayMode::default(),
                },
                rank,
            )),
        }
    }

    // THE SELECTOR — one place, one ordered term list (`SELECTOR_TERMS`).
    let mut ranked: Vec<(usize, NameHolder)> = holders
        .into_iter()
        .map(|(holder, _)| holder)
        .enumerate()
        .collect();
    ranked.sort_by_key(|(owner_order, holder)| selector_rank(holder, *owner_order));
    ranked.into_iter().map(|(_, holder)| holder).collect()
}

/// The doorway-local, in-memory name-route registry.
///
/// Category C (Operational): no table, no DHT entry, no persistence. Every row
/// is rebuilt from the next federation discovery tick, so a restart costs one
/// discovery interval and nothing else.
#[derive(Debug, Default)]
pub struct NameRouteTable {
    contracts: RwLock<Vec<HolderContract>>,
    liveness: RwLock<HashMap<String, HolderLiveness>>,
}

impl NameRouteTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Install a freshly-probed snapshot.
    ///
    /// Last-good preservation (mirrors `FallbackOutcome::AllUnreachable` in the
    /// EPR router): if the new contract set is EMPTY but we already hold rows,
    /// keep the rows and only update liveness. A tick where every sibling is
    /// briefly unreachable must not erase our knowledge of who holds which
    /// name — it should only make those holders sort last.
    pub fn replace_all(
        &self,
        contracts: Vec<HolderContract>,
        liveness: HashMap<String, HolderLiveness>,
    ) {
        {
            let mut live = self.liveness.write().expect("name-route lock poisoned");
            *live = liveness;
        }
        let mut table = self.contracts.write().expect("name-route lock poisoned");
        if contracts.is_empty() && !table.is_empty() {
            debug!(
                held = table.len(),
                "name-route refresh returned no contracts — preserving last-good table"
            );
            return;
        }
        *table = contracts;
    }

    /// Candidate holders for a [`RouteKey`] (see [`fold_candidate_holders`]).
    pub fn holders_for(&self, key: &RouteKey, self_doorway_id: &str) -> Vec<NameHolder> {
        let contracts = self.contracts.read().expect("name-route lock poisoned");
        let liveness = self.liveness.read().expect("name-route lock poisoned");
        fold_candidate_holders(key, &contracts, &liveness, self_doorway_id)
    }

    /// Record that a holder SHED a relay (503). The discovery probe cannot tell
    /// a shed from a death, but a relay can — so the one surface that observes
    /// it writes it, and the next fold orders that holder after the serving
    /// ones. Cleared by the next discovery tick's `replace_all`.
    pub fn note_shed(&self, doorway_id: &str) {
        let mut liveness = self.liveness.write().expect("name-route lock poisoned");
        liveness.insert(doorway_id.to_string(), HolderLiveness::Shedding);
    }

    pub fn len(&self) -> usize {
        self.contracts
            .read()
            .expect("name-route lock poisoned")
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// What a holder answered. Transport-agnostic so the relay loop is testable
/// without a network.
///
/// # Why the standing/bundle/cache terms are carried
///
/// A relay is a *courier*, not a re-decider. The holder is the doorway that
/// actually resolved the fold for this name, so the terms it stated on its own
/// face are the terms the client must receive: the standing it resolved
/// ([`STANDING_HEADER`]), the bundle-freshness it admitted
/// ([`BUNDLE_HEADER`]), and how it says its own answer may be cached. Dropping
/// them re-writes a refusal into an unexplained one and a "behind" bundle into
/// a silently-fresh one — the two honesty markers this protocol spends the most
/// care minting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolderReply {
    pub status: u16,
    pub content_type: Option<String>,
    /// The holder's `x-elohim-standing` — the standing IT resolved, verbatim.
    /// `None` when it stated none.
    pub standing: Option<String>,
    /// The holder's `x-elohim-bundle` staleness marker, verbatim. `None` when
    /// it stated none (i.e. it confirmed the head current and deliverable).
    pub bundle: Option<String>,
    /// The holder's own `cache-control` for this answer. `None` → the relay's
    /// default (`no-store`) governs.
    pub cache_control: Option<String>,
    pub body: Vec<u8>,
}

/// The relay decision.
#[derive(Debug, PartialEq, Eq)]
pub enum RelayVerdict {
    /// A holder answered. Caller serves these bytes and stamps
    /// [`SERVED_BY_HEADER`] with `origin`.
    Served {
        doorway_id: String,
        origin: String,
        reply: HolderReply,
    },
    /// No doorway holds a contract covering this name — the local verdict
    /// (404) is the whole truth.
    NoCandidates,
    /// Every holder was tried and none served. **The caller preserves its
    /// original 404/503 verbatim** — a relay that fails must never mask the
    /// local answer with an invented error.
    AllFailed { attempted: usize },
}

/// A verdict plus the holders observed shedding, so the caller can demote them
/// in the table without the relay loop owning mutable state.
#[derive(Debug)]
pub struct RelayOutcome {
    pub verdict: RelayVerdict,
    pub shed_doorways: Vec<String>,
}

/// Try each candidate holder in fold order until one serves. ONE hop total —
/// `fetch` is responsible for stamping [`FEDERATION_HOP_HEADER`] on the
/// outbound request, and the caller is responsible for never calling this at
/// all when the INBOUND request already carried it (see
/// [`relay_precondition`]).
///
/// Outcome per holder — the axis is **did this holder ANSWER**, not "did it
/// give us bytes we like":
/// - `2xx`/`3xx` → served, stop;
/// - `403` → **answered, stop.** The holder resolved the fold and refused this
///   requester. That is an authoritative outcome about the requester's
///   standing, so it is relayed as-is and NO further holder is dialled. Asking
///   the next one would be shopping a refusal around the federation until a
///   doorway with a staler projection admitted it — see
///   [`AUTHORITATIVE_REFUSAL`].
/// - `503` → that holder is shedding: a statement about the HOLDER's liveness,
///   not about the requester or the record. Record it and **try the next
///   holder before answering the shed**;
/// - any other status (incl. the sibling's honest `404`) → it did not serve:
///   a failed attempt, try the next holder; if it was the last, the caller
///   keeps ITS original status. A sibling 404 therefore surfaces as our own
///   404, never as a rewritten one.
/// - transport error → a failed attempt, same as above.
pub async fn relay_one_hop<F, Fut>(holders: &[NameHolder], fetch: F) -> RelayOutcome
where
    F: Fn(NameHolder) -> Fut,
    Fut: std::future::Future<Output = Result<HolderReply, String>>,
{
    if holders.is_empty() {
        return RelayOutcome {
            verdict: RelayVerdict::NoCandidates,
            shed_doorways: Vec::new(),
        };
    }

    let mut shed_doorways = Vec::new();
    let mut attempted = 0usize;

    for holder in holders {
        // The relay mode is an ENUM and this is its one match. A mode that is
        // declared but not implemented SKIPS the holder loudly — it is never
        // silently proxied, because "we said redirect and quietly proxied" is
        // the drift this arm exists to make impossible.
        match holder.relay_mode {
            RelayMode::Proxy => {}
            RelayMode::Redirect => {
                warn!(
                    holder = %holder.doorway_id,
                    origin = %holder.origin,
                    "name-route: RelayMode::Redirect is declared but not implemented — skipping this holder"
                );
                continue;
            }
        }
        attempted += 1;
        match fetch(holder.clone()).await {
            // An ANSWER from the holder that holds this name's contract. Bytes
            // (2xx/3xx) or an authoritative refusal (403) are the same kind of
            // thing here: the doorway that owns the fold has spoken, so the
            // relay stops and the client receives what it said.
            Ok(reply)
                if (200..400).contains(&reply.status) || reply.status == AUTHORITATIVE_REFUSAL =>
            {
                return RelayOutcome {
                    verdict: RelayVerdict::Served {
                        doorway_id: holder.doorway_id.clone(),
                        origin: holder.origin.clone(),
                        reply,
                    },
                    shed_doorways,
                };
            }
            Ok(reply) if reply.status == 503 => {
                debug!(
                    holder = %holder.doorway_id,
                    origin = %holder.origin,
                    "name-route: holder is shedding — trying the next holder"
                );
                shed_doorways.push(holder.doorway_id.clone());
            }
            Ok(reply) => {
                debug!(
                    holder = %holder.doorway_id,
                    origin = %holder.origin,
                    status = reply.status,
                    "name-route: holder did not serve"
                );
            }
            Err(e) => {
                debug!(
                    holder = %holder.doorway_id,
                    origin = %holder.origin,
                    error = %e,
                    "name-route: holder unreachable"
                );
            }
        }
    }

    RelayOutcome {
        verdict: RelayVerdict::AllFailed { attempted },
        shed_doorways,
    }
}

/// Why a relay is being considered. The two triggers answer different
/// questions: one is "we cannot serve this", the other is "we can, but
/// somebody else holds the more specific contract".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayTrigger {
    /// This doorway's own answer. Only a 404 (no contract for this root here)
    /// or a 503 (its own primary + pool are shedding) is relay-eligible.
    LocalVerdict(StatusCode),
    /// This doorway CAN answer, but only from a LESS SPECIFIC mount than a
    /// sibling's — typically its `/` catch-all against a sibling's real
    /// contract for the requested root. Eligible whatever the local status
    /// would have been: the local answer is not broken, it is simply not ours
    /// to give.
    LessSpecificThanHolder,
}

/// Whether a request is eligible for a name-routed relay at all. PURE, so the
/// gate is one testable predicate instead of a condition smeared across the
/// dispatch sites.
///
/// All must hold:
/// 1. `is_get` — only reads relay; a write is never replayed at a sibling.
/// 2. `!is_service_path` — a name is a projected root, not `/db/*`, `/admin/*`
///    or any other doorway/storage service surface.
/// 3. `!hop_seen` — the ONE-hop budget (see [`MAX_FEDERATION_HOPS`]).
/// 4. the trigger admits it (see [`RelayTrigger`]).
pub fn relay_precondition(
    is_get: bool,
    is_service_path: bool,
    hop_seen: bool,
    trigger: RelayTrigger,
) -> bool {
    if !is_get || is_service_path || hop_seen {
        return false;
    }
    match trigger {
        RelayTrigger::LocalVerdict(status) => {
            status == StatusCode::NOT_FOUND || status == StatusCode::SERVICE_UNAVAILABLE
        }
        RelayTrigger::LessSpecificThanHolder => true,
    }
}

/// True iff an inbound request already crossed a doorway. Any non-empty value
/// counts — we never trust a peer's hop COUNT, only the fact of the hop, so a
/// forged large count cannot buy extra hops and a forged small one cannot
/// either.
pub fn inbound_hop_seen(raw_header: Option<&str>) -> bool {
    raw_header.map(|v| !v.trim().is_empty()).unwrap_or(false)
}

/// Build the client-facing response for a served relay: the holder's status
/// and bytes, its content type, the terms it stated on its own face
/// ([`STANDING_HEADER`], [`BUNDLE_HEADER`], its `cache-control`),
/// [`SERVED_BY_HEADER`] naming the origin so the sticky client can go direct
/// next time, and [`NAME_ROUTE_HEADER`] marking the response as relayed.
///
/// # The relayed terms are the holder's, not ours
///
/// A relayed `403` that arrives stripped of its `x-elohim-standing` is a
/// refusal with no face: the chrome cannot say WHICH term refused, and the
/// visitor cannot be told what standing they would need. Same for a `behind`
/// bundle marker — dropping it presents a stale serve as a fresh one. So both
/// are carried verbatim when the holder stated them, and simply absent when it
/// did not; the relay never invents either.
///
/// `cache-control` likewise defers to the holder — its answer, its caching
/// terms. Absent one, the relay's own default governs (`no-store`: the relay
/// verdict is a liveness-dependent routing decision, not content truth, and
/// must not outlive the liveness that produced it).
pub fn build_relayed_response(
    origin: &str,
    doorway_id: &str,
    reply: HolderReply,
) -> Response<Full<Bytes>> {
    let status = StatusCode::from_u16(reply.status).unwrap_or(StatusCode::OK);
    let content_type = reply
        .content_type
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let mut builder = Response::builder()
        .status(status)
        .header("Content-Type", content_type)
        .header(SERVED_BY_HEADER, origin)
        .header(NAME_ROUTE_HEADER, format!("relay:{doorway_id}"))
        .header(
            "cache-control",
            reply.cache_control.as_deref().unwrap_or("no-store"),
        );
    if let Some(standing) = reply.standing.as_deref() {
        builder = builder.header(STANDING_HEADER, standing);
    }
    if let Some(bundle) = reply.bundle.as_deref() {
        builder = builder.header(BUNDLE_HEADER, bundle);
    }
    builder
        .body(Full::new(Bytes::from(reply.body)))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::new()))
                .expect("static bad-gateway response is infallible")
        })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn contract(id: &str, origin: &str, mount: &str) -> HolderContract {
        HolderContract::any_host(id, origin, mount)
    }

    fn host_contract(id: &str, origin: &str, mount: &str, host: &str) -> HolderContract {
        HolderContract {
            host: Some(host.to_string()),
            ..HolderContract::any_host(id, origin, mount)
        }
    }

    fn holder(id: &str, origin: &str, liveness: HolderLiveness) -> NameHolder {
        NameHolder {
            doorway_id: id.to_string(),
            origin: origin.to_string(),
            url_path: "/lamad".to_string(),
            host: None,
            liveness,
            relay_mode: RelayMode::Proxy,
        }
    }

    fn reply(status: u16, body: &str) -> HolderReply {
        HolderReply {
            status,
            content_type: Some("text/html".to_string()),
            standing: None,
            bundle: None,
            cache_control: None,
            body: body.as_bytes().to_vec(),
        }
    }

    /// A holder's authoritative refusal, shaped exactly as
    /// `serve_eligibility::Refusal::standing_header_value` mints it.
    fn refusal(reach: &str) -> HolderReply {
        HolderReply {
            standing: Some(format!("refused;reach={reach}")),
            ..reply(403, r#"{"term":"reach","reason":"this is held closer"}"#)
        }
    }

    // ── the fold ────────────────────────────────────────────────────────────

    #[test]
    fn mount_covers_on_segment_boundaries_only() {
        assert!(mount_covers("/lamad", "/lamad"));
        assert!(mount_covers("/lamad", "/lamad/"));
        assert!(mount_covers("/lamad", "/lamad/path/1"));
        assert!(!mount_covers("/lamad", "/lamadx"));
        assert!(mount_covers("/", "/anything/at/all"));
    }

    #[test]
    fn fold_orders_health_first_then_owner_order() {
        let contracts = vec![
            contract("b-doorway", "https://b.example", "/lamad"),
            contract("c-doorway", "https://c.example", "/lamad"),
            contract("d-doorway", "https://d.example", "/lamad"),
        ];
        let liveness = HashMap::from([
            ("b-doorway".to_string(), HolderLiveness::Unreachable),
            ("c-doorway".to_string(), HolderLiveness::Serving),
            ("d-doorway".to_string(), HolderLiveness::Serving),
        ]);

        let folded = fold_candidate_holders(
            &RouteKey::path_only("/lamad/x"),
            &contracts,
            &liveness,
            "a-doorway",
        );
        let ids: Vec<&str> = folded.iter().map(|h| h.doorway_id.as_str()).collect();
        // health first (c,d serving before b unreachable); within equal health,
        // registry (owner) order is preserved — c before d, not sorted by id.
        assert_eq!(ids, vec!["c-doorway", "d-doorway", "b-doorway"]);
    }

    #[test]
    fn fold_excludes_self_and_non_covering_mounts() {
        let contracts = vec![
            contract("a-doorway", "https://a.example", "/lamad"),
            contract("b-doorway", "https://b.example", "/shefa"),
            contract("c-doorway", "https://c.example", "/lamad"),
        ];
        let folded = fold_candidate_holders(
            &RouteKey::path_only("/lamad"),
            &contracts,
            &HashMap::new(),
            "a-doorway",
        );
        let ids: Vec<&str> = folded.iter().map(|h| h.doorway_id.as_str()).collect();
        assert_eq!(ids, vec!["c-doorway"], "self and /shefa must not qualify");
    }

    #[test]
    fn fold_keeps_the_most_specific_mount_per_doorway() {
        let contracts = vec![
            contract("b-doorway", "https://b.example/", "/"),
            contract("b-doorway", "https://b.example/", "/lamad"),
        ];
        let folded = fold_candidate_holders(
            &RouteKey::path_only("/lamad/deep"),
            &contracts,
            &HashMap::new(),
            "a-doorway",
        );
        assert_eq!(folded.len(), 1);
        assert_eq!(folded[0].url_path, "/lamad");
        assert_eq!(
            folded[0].origin, "https://b.example",
            "origin is normalised"
        );
    }

    #[test]
    fn fold_unknown_liveness_defaults_to_uncertain_and_sorts_after_serving() {
        let contracts = vec![
            contract("b-doorway", "https://b.example", "/"),
            contract("c-doorway", "https://c.example", "/"),
        ];
        let liveness = HashMap::from([("c-doorway".to_string(), HolderLiveness::Serving)]);
        let folded = fold_candidate_holders(
            &RouteKey::path_only("/x"),
            &contracts,
            &liveness,
            "a-doorway",
        );
        let ids: Vec<&str> = folded.iter().map(|h| h.doorway_id.as_str()).collect();
        assert_eq!(ids, vec!["c-doorway", "b-doorway"]);
    }

    // ── routing dimensions: host, and the selector's shape ──────────────────

    #[test]
    fn any_host_contract_answers_for_every_host_todays_shape() {
        // Every contract is any-host today, so naming a host must not narrow.
        let contracts = vec![contract("b-doorway", "https://b.example", "/lamad")];
        for host in [None, Some("elohim.host"), Some("alpha.elohim.host")] {
            let folded = fold_candidate_holders(
                &RouteKey::new(host, "/lamad"),
                &contracts,
                &HashMap::new(),
                "a-doorway",
            );
            assert_eq!(folded.len(), 1, "any-host contract answers for {host:?}");
            assert_eq!(folded[0].host, None);
        }
    }

    #[test]
    fn host_bound_contract_answers_only_for_its_host() {
        let contracts = vec![host_contract(
            "b-doorway",
            "https://b.example",
            "/lamad",
            "alpha.elohim.host",
        )];
        let matched = fold_candidate_holders(
            // Case and port must not matter.
            &RouteKey::new(Some("ALPHA.elohim.host:443"), "/lamad"),
            &contracts,
            &HashMap::new(),
            "a-doorway",
        );
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].host.as_deref(), Some("alpha.elohim.host"));

        for other in [None, Some("elohim.host")] {
            assert!(
                fold_candidate_holders(
                    &RouteKey::new(other, "/lamad"),
                    &contracts,
                    &HashMap::new(),
                    "a-doorway",
                )
                .is_empty(),
                "a host-bound contract must not answer for {other:?}"
            );
        }
    }

    #[test]
    fn host_bound_contract_beats_any_host_for_the_same_doorway() {
        let contracts = vec![
            contract("b-doorway", "https://b.example", "/lamad"),
            host_contract("b-doorway", "https://b.example", "/lamad", "elohim.host"),
        ];
        let folded = fold_candidate_holders(
            &RouteKey::new(Some("elohim.host"), "/lamad"),
            &contracts,
            &HashMap::new(),
            "a-doorway",
        );
        assert_eq!(folded.len(), 1, "one entry per doorway");
        assert_eq!(
            folded[0].host.as_deref(),
            Some("elohim.host"),
            "the more specific (host-bound) contract wins"
        );
    }

    #[test]
    fn selector_terms_declare_liveness_first_and_owner_order_last() {
        // The ORDER is the contract. reach/standing, nearest and weight sit
        // between them and are declared-but-constant until wired.
        assert_eq!(
            SELECTOR_TERMS,
            &[
                SelectorTerm::Liveness,
                SelectorTerm::ReachStanding,
                SelectorTerm::Nearest,
                SelectorTerm::Weight,
                SelectorTerm::OwnerOrder,
            ]
        );
        // One rank element per declared term.
        let probe = holder("x", "https://x.example", HolderLiveness::Serving);
        let rank = selector_rank(&probe, 3);
        assert_eq!(
            SELECTOR_TERMS.len(),
            5,
            "selector_rank returns one element per term — keep them in step"
        );
        assert_eq!(rank.0, HolderLiveness::Serving as u8);
        assert_eq!(
            (rank.1, rank.2, rank.3),
            (TERM_NOT_YET_WIRED, TERM_NOT_YET_WIRED, TERM_NOT_YET_WIRED),
            "unwired terms are constant, so they cannot reorder anything yet"
        );
        assert_eq!(rank.4, 3, "owner order is the final tiebreak");
    }

    #[tokio::test]
    async fn an_unimplemented_relay_mode_skips_the_holder_never_proxies_it() {
        // RelayMode::Redirect is declared, not implemented. It must not be
        // silently proxied; the next holder serves.
        let mut redirect_holder = holder("b-doorway", "https://b.example", HolderLiveness::Serving);
        redirect_holder.relay_mode = RelayMode::Redirect;
        let holders = vec![
            redirect_holder,
            holder("c-doorway", "https://c.example", HolderLiveness::Serving),
        ];
        let dialled = Mutex::new(Vec::new());
        let outcome = relay_one_hop(&holders, |h| {
            dialled.lock().unwrap().push(h.doorway_id.clone());
            async move { Ok(reply(200, "served by c")) }
        })
        .await;

        match &outcome.verdict {
            RelayVerdict::Served { origin, .. } => assert_eq!(origin, "https://c.example"),
            other => panic!("expected the proxy-mode holder to serve, got {other:?}"),
        }
        assert_eq!(
            *dialled.lock().unwrap(),
            vec!["c-doorway".to_string()],
            "the redirect-mode holder was never dialled"
        );
    }

    // ── the table ───────────────────────────────────────────────────────────

    #[test]
    fn table_preserves_last_good_on_an_empty_refresh() {
        let table = NameRouteTable::new();
        table.replace_all(
            vec![contract("b-doorway", "https://b.example", "/lamad")],
            HashMap::from([("b-doorway".to_string(), HolderLiveness::Serving)]),
        );
        table.replace_all(
            Vec::new(),
            HashMap::from([("b-doorway".to_string(), HolderLiveness::Unreachable)]),
        );
        let holders = table.holders_for(&RouteKey::path_only("/lamad"), "a-doorway");
        assert_eq!(
            holders.len(),
            1,
            "contracts survive an all-unreachable tick"
        );
        assert_eq!(holders[0].liveness, HolderLiveness::Unreachable);
    }

    #[test]
    fn table_note_shed_demotes_a_holder_for_the_next_fold() {
        let table = NameRouteTable::new();
        table.replace_all(
            vec![
                contract("b-doorway", "https://b.example", "/lamad"),
                contract("c-doorway", "https://c.example", "/lamad"),
            ],
            HashMap::from([
                ("b-doorway".to_string(), HolderLiveness::Serving),
                ("c-doorway".to_string(), HolderLiveness::Serving),
            ]),
        );
        table.note_shed("b-doorway");
        let ids: Vec<String> = table
            .holders_for(&RouteKey::path_only("/lamad"), "a-doorway")
            .into_iter()
            .map(|h| h.doorway_id)
            .collect();
        assert_eq!(ids, vec!["c-doorway".to_string(), "b-doorway".to_string()]);
    }

    // ── the one-hop budget ──────────────────────────────────────────────────

    const NOT_FOUND: RelayTrigger = RelayTrigger::LocalVerdict(StatusCode::NOT_FOUND);

    #[test]
    fn inbound_hop_header_refuses_the_forward() {
        // (c) inbound loop header → no forward, whatever the trigger is.
        assert!(!relay_precondition(
            true,
            false,
            inbound_hop_seen(Some("1")),
            NOT_FOUND
        ));
        assert!(!relay_precondition(
            true,
            false,
            inbound_hop_seen(Some("anything")),
            RelayTrigger::LocalVerdict(StatusCode::SERVICE_UNAVAILABLE)
        ));
        // The one-hop budget binds the specificity trigger exactly as hard.
        assert!(!relay_precondition(
            true,
            false,
            inbound_hop_seen(Some("1")),
            RelayTrigger::LessSpecificThanHolder
        ));
        // No header (or a blank one) → eligible.
        assert!(relay_precondition(
            true,
            false,
            inbound_hop_seen(None),
            NOT_FOUND
        ));
        assert!(relay_precondition(
            true,
            false,
            inbound_hop_seen(Some("  ")),
            NOT_FOUND
        ));
    }

    #[test]
    fn relay_precondition_gates_method_surface_and_status() {
        assert!(!relay_precondition(false, false, false, NOT_FOUND));
        assert!(!relay_precondition(true, true, false, NOT_FOUND));
        assert!(!relay_precondition(
            true,
            false,
            false,
            RelayTrigger::LocalVerdict(StatusCode::OK)
        ));
        assert!(!relay_precondition(
            true,
            false,
            false,
            RelayTrigger::LocalVerdict(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(relay_precondition(
            true,
            false,
            false,
            RelayTrigger::LocalVerdict(StatusCode::SERVICE_UNAVAILABLE)
        ));
        // The specificity trigger is status-independent: a local 200 from a
        // catch-all is exactly the case it exists for.
        assert!(relay_precondition(
            true,
            false,
            false,
            RelayTrigger::LessSpecificThanHolder
        ));
        assert!(!relay_precondition(
            false,
            false,
            false,
            RelayTrigger::LessSpecificThanHolder
        ));
        assert!(!relay_precondition(
            true,
            true,
            false,
            RelayTrigger::LessSpecificThanHolder
        ));
    }

    // ── specificity: a catch-all must not answer for somebody else's name ────

    fn holder_at(id: &str, origin: &str, mount: &str) -> NameHolder {
        NameHolder {
            url_path: mount.to_string(),
            ..holder(id, origin, HolderLiveness::Serving)
        }
    }

    #[test]
    fn sibling_specific_mount_beats_a_local_catch_all() {
        // THE measured case: B holds "/" (landing SPA), A holds "/nrt-garden".
        let holders = vec![holder_at("alpha", "https://a.example", "/nrt-garden")];
        let kept = holders_more_specific_than(Some(&LocalMount::path_only("/")), holders);
        assert_eq!(kept.len(), 1, "the sibling's specific mount wins over '/'");
        assert_eq!(kept[0].url_path, "/nrt-garden");
    }

    #[test]
    fn an_equal_mount_ties_and_local_wins() {
        let holders = vec![holder_at("alpha", "https://a.example", "/nrt-garden")];
        let kept = holders_more_specific_than(Some(&LocalMount::path_only("/nrt-garden")), holders);
        assert!(
            kept.is_empty(),
            "we hold the same contract — serve it ourselves, never relay"
        );
    }

    #[test]
    fn a_more_specific_local_mount_wins() {
        let holders = vec![holder_at("alpha", "https://a.example", "/nrt-garden")];
        let kept =
            holders_more_specific_than(Some(&LocalMount::path_only("/nrt-garden/deep")), holders);
        assert!(kept.is_empty(), "local is the more specific holder");
    }

    #[test]
    fn no_local_mount_keeps_every_holder_todays_404_path() {
        // `None` = we hold no mount (404) or cannot serve the one we hold
        // (shed). Behaviour must be exactly as before the specificity term.
        let holders = vec![
            holder_at("alpha", "https://a.example", "/nrt-garden"),
            holder_at("gamma", "https://g.example", "/"),
        ];
        let kept = holders_more_specific_than(None, holders.clone());
        assert_eq!(kept, holders, "unchanged when there is no local match");
    }

    #[test]
    fn an_empty_table_relays_nothing_and_local_serves() {
        // No name-route table (fresh boot, or a doorway with no siblings):
        // nothing qualifies, so dispatch is byte-for-byte today's.
        let kept = holders_more_specific_than(Some(&LocalMount::path_only("/")), Vec::new());
        assert!(kept.is_empty());
    }

    #[test]
    fn a_host_bound_sibling_mount_outranks_an_equal_length_local_any_host_mount() {
        // Forward-looking: when hosts become head channels, host-bound is more
        // specific than any-host at equal path length.
        let mut bound = holder_at("alpha", "https://a.example", "/nrt-garden");
        bound.host = Some("elohim.host".to_string());
        let kept =
            holders_more_specific_than(Some(&LocalMount::path_only("/nrt-garden")), vec![bound]);
        assert_eq!(kept.len(), 1, "host-bound beats any-host at equal length");
    }

    // ── the relay ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn local_404_plus_healthy_sibling_serves_bytes_and_names_the_origin() {
        // (a) local 404 + healthy sibling → bytes served + served-by header.
        let holders = vec![holder(
            "b-doorway",
            "https://b.example",
            HolderLiveness::Serving,
        )];
        let calls = Mutex::new(Vec::new());
        let outcome = relay_one_hop(&holders, |h| {
            calls.lock().unwrap().push(h.origin.clone());
            async move { Ok(reply(200, "<app-root></app-root>")) }
        })
        .await;

        let RelayVerdict::Served {
            origin,
            doorway_id,
            reply,
        } = outcome.verdict
        else {
            panic!("expected Served");
        };
        assert_eq!(origin, "https://b.example");
        assert_eq!(reply.body, b"<app-root></app-root>");
        assert_eq!(calls.lock().unwrap().len(), 1, "exactly one hop");

        let response = build_relayed_response(&origin, &doorway_id, reply);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(SERVED_BY_HEADER).unwrap(),
            "https://b.example"
        );
        assert_eq!(
            response.headers().get(NAME_ROUTE_HEADER).unwrap(),
            "relay:b-doorway"
        );
    }

    #[tokio::test]
    async fn sibling_404_preserves_the_original_404() {
        // (b) sibling 404 → original 404; the relay reports AllFailed and the
        // caller keeps its own response. No error masking.
        let holders = vec![holder(
            "b-doorway",
            "https://b.example",
            HolderLiveness::Serving,
        )];
        let outcome = relay_one_hop(&holders, |_| async { Ok(reply(404, "not here")) }).await;
        assert_eq!(outcome.verdict, RelayVerdict::AllFailed { attempted: 1 });
        assert!(outcome.shed_doorways.is_empty());
    }

    #[tokio::test]
    async fn first_holder_shedding_second_holder_serves() {
        // (d) a holder's 503 is not the answer — the next holder is tried first.
        let holders = vec![
            holder("b-doorway", "https://b.example", HolderLiveness::Serving),
            holder("c-doorway", "https://c.example", HolderLiveness::Serving),
        ];
        let outcome = relay_one_hop(&holders, |h| async move {
            if h.doorway_id == "b-doorway" {
                Ok(reply(503, r#"{"status":"catching-up"}"#))
            } else {
                Ok(reply(200, "served by c"))
            }
        })
        .await;

        match &outcome.verdict {
            RelayVerdict::Served { origin, .. } => assert_eq!(origin, "https://c.example"),
            other => panic!("expected the second holder to serve, got {other:?}"),
        }
        assert_eq!(
            outcome.shed_doorways,
            vec!["b-doorway".to_string()],
            "the shed is recorded so the next fold demotes that holder"
        );
    }

    #[tokio::test]
    async fn every_holder_shedding_still_preserves_the_local_shed() {
        let holders = vec![
            holder("b-doorway", "https://b.example", HolderLiveness::Serving),
            holder("c-doorway", "https://c.example", HolderLiveness::Serving),
        ];
        let outcome = relay_one_hop(&holders, |_| async { Ok(reply(503, "catching up")) }).await;
        assert_eq!(outcome.verdict, RelayVerdict::AllFailed { attempted: 2 });
        assert_eq!(outcome.shed_doorways.len(), 2);
    }

    #[tokio::test]
    async fn no_candidates_is_distinct_from_a_failed_attempt() {
        let outcome = relay_one_hop(&[], |_| async { Ok(reply(200, "never")) }).await;
        assert_eq!(outcome.verdict, RelayVerdict::NoCandidates);
    }

    /// THE FIX. A holder's 403 is an ANSWER about this requester's standing,
    /// and the holder is the doorway that holds the contract for this name. So
    /// it is relayed as-is, with the standing it named, and NO second holder is
    /// dialled: a refusal must not be shopped around the federation until a
    /// doorway with a staler projection says yes.
    #[tokio::test]
    async fn a_holders_refusal_is_relayed_with_its_standing_and_stops_the_loop() {
        let holders = vec![
            holder("b-doorway", "https://b.example", HolderLiveness::Serving),
            holder("c-doorway", "https://c.example", HolderLiveness::Serving),
        ];
        let dialled = Mutex::new(Vec::new());
        let outcome = relay_one_hop(&holders, |h| {
            dialled.lock().unwrap().push(h.doorway_id.clone());
            async move { Ok(refusal("household")) }
        })
        .await;

        let RelayVerdict::Served {
            origin,
            doorway_id,
            reply,
        } = outcome.verdict
        else {
            panic!("a 403 is an authoritative outcome, not a failed attempt");
        };
        assert_eq!(origin, "https://b.example");
        assert_eq!(reply.status, 403);
        assert_eq!(
            *dialled.lock().unwrap(),
            vec!["b-doorway".to_string()],
            "the refusal ends the loop -- the second holder must never be asked \
             to overturn it"
        );
        assert!(
            outcome.shed_doorways.is_empty(),
            "a refusal says nothing about the holder's liveness"
        );

        // ...and it reaches the client with its face intact.
        let response = build_relayed_response(&origin, &doorway_id, reply);
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            response.headers().get(STANDING_HEADER).unwrap(),
            "refused;reach=household",
            "a refusal stripped of its standing cannot tell the visitor WHICH \
             term refused them"
        );
        assert_eq!(
            response.headers().get(SERVED_BY_HEADER).unwrap(),
            "https://b.example",
            "and it still names who decided it"
        );
        assert_eq!(
            response.headers().get(NAME_ROUTE_HEADER).unwrap(),
            "relay:b-doorway"
        );
    }

    /// The contrast that keeps the two statuses from collapsing into one rule:
    /// a 503 is about the HOLDER (liveness), so the next holder is tried; a 403
    /// is about the REQUESTER, so it is not. Same loop, opposite handling.
    #[tokio::test]
    async fn a_holders_shed_still_tries_the_next_holder() {
        let holders = vec![
            holder("b-doorway", "https://b.example", HolderLiveness::Serving),
            holder("c-doorway", "https://c.example", HolderLiveness::Serving),
        ];
        let dialled = Mutex::new(Vec::new());
        let outcome = relay_one_hop(&holders, |h| {
            dialled.lock().unwrap().push(h.doorway_id.clone());
            async move {
                if h.doorway_id == "b-doorway" {
                    Ok(reply(503, r#"{"status":"catching-up"}"#))
                } else {
                    Ok(reply(200, "served by c"))
                }
            }
        })
        .await;

        match &outcome.verdict {
            RelayVerdict::Served { origin, .. } => assert_eq!(origin, "https://c.example"),
            other => panic!("expected the second holder to serve, got {other:?}"),
        }
        assert_eq!(
            *dialled.lock().unwrap(),
            vec!["b-doorway".to_string(), "c-doorway".to_string()],
            "a shed is a liveness statement -- unlike a refusal, it MUST fall \
             through to the next holder"
        );
        assert_eq!(outcome.shed_doorways, vec!["b-doorway".to_string()]);
    }

    /// The holder's own bundle marker and caching terms ride along too, and an
    /// absent `cache-control` leaves the relay's `no-store` default governing.
    #[test]
    fn the_holders_bundle_and_cache_terms_are_restamped_verbatim() {
        let carried = HolderReply {
            bundle: Some("behind;head-unconfirmed".to_string()),
            cache_control: Some("private, max-age=30".to_string()),
            ..reply(200, "<app-root></app-root>")
        };
        let response = build_relayed_response("https://b.example", "b-doorway", carried);
        assert_eq!(
            response.headers().get(BUNDLE_HEADER).unwrap(),
            "behind;head-unconfirmed",
            "dropping the staleness marker presents a stale serve as a fresh one"
        );
        assert_eq!(
            response.headers().get("cache-control").unwrap(),
            "private, max-age=30",
            "the holder's answer carries the holder's caching terms"
        );

        let plain = build_relayed_response(
            "https://b.example",
            "b-doorway",
            reply(200, "<app-root></app-root>"),
        );
        assert_eq!(
            plain.headers().get("cache-control").unwrap(),
            "no-store",
            "with no holder term, the relay's own default still governs"
        );
        assert!(plain.headers().get(BUNDLE_HEADER).is_none());
        assert!(
            plain.headers().get(STANDING_HEADER).is_none(),
            "the relay never invents a standing the holder did not state"
        );
    }

    #[tokio::test]
    async fn transport_error_falls_through_to_the_next_holder() {
        let holders = vec![
            holder("b-doorway", "https://b.example", HolderLiveness::Serving),
            holder("c-doorway", "https://c.example", HolderLiveness::Serving),
        ];
        let outcome = relay_one_hop(&holders, |h| async move {
            if h.doorway_id == "b-doorway" {
                Err("connection refused".to_string())
            } else {
                Ok(reply(200, "served by c"))
            }
        })
        .await;
        assert!(matches!(outcome.verdict, RelayVerdict::Served { .. }));
    }
}
