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
use tracing::{debug, info, warn};

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

/// Where the holder said its receipt for this serve can be read, relayed
/// verbatim. Minted by [`crate::services::serve_receipt`]; this module only
/// carries it — and a courier MUST carry it, because the exchange it points at
/// is the holder's, not the courier's.
pub use crate::services::serve_receipt::RECEIPT_HEADER;

/// A holder's **authoritative refusal**. Not a failed attempt: the holder is
/// the doorway that holds the contract for this name, so when it says the
/// requester may not have it, that IS the answer to the request. Trying the
/// next holder would be shopping the refusal around the federation until some
/// doorway with a staler projection said yes — reach-laundering by retry.
const AUTHORITATIVE_REFUSAL: u16 = 403;

/// A holder's **named absence**. Not a failed attempt either: a bare 404 is
/// "that holder did not serve", but a 404 carrying the candidate channel's own
/// structured absence from the doorway that holds the name is a statement about
/// the RECORD — the same rationale as [`AUTHORITATIVE_REFUSAL`], and strictly
/// less permissive than the 200 already relayed for that same name.
const NAMED_ABSENCE: u16 = 404;

/// The candidate channel's named absence, as the `error` field of the body its
/// handler answers with. ONE definition for the crate: the local candidate
/// handler answers [`NO_CANDIDATE_STAGED_BODY`], and the relay recognises a
/// holder's answer by comparing this exact token.
pub const NO_CANDIDATE_STAGED_ERROR: &str = "no-candidate-staged";

/// The exact body the candidate channel answers with when nothing is staged.
/// Kept beside the token it carries; `the_named_absence_body_carries_its_token`
/// fixes the two together so neither can drift from the other.
pub const NO_CANDIDATE_STAGED_BODY: &str = r#"{"error":"no-candidate-staged"}"#;

/// The most body read before deciding whether a 404 is a named absence. A
/// structured absence is tens of bytes; anything larger is by construction
/// something else, so the question can never cost this doorway more than this.
const NAMED_ABSENCE_MAX_BODY: usize = 256;

/// Which head channel the requested name resolved to HERE, before any hop.
///
/// An ENUM, never a bool: the channel set is the projection's, and a bool would
/// have to be re-read at every call site the day a third channel appears.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RelayChannel {
    /// The public channel — and every name whose channel this doorway cannot
    /// resolve locally at all. The DEFAULT, and the class whose relay
    /// behaviour is exactly what it has always been.
    #[default]
    Public,
    /// A candidate-bound name: the staging channel, the one channel that states
    /// its absence as a structured 404 rather than merely failing to serve.
    Candidate,
}

/// True iff `reply` is the declared holder's named absence for a candidate
/// channel this doorway already resolved locally.
///
/// Opacity: this opens only for a name THIS doorway resolved to the candidate
/// channel, so it speaks only about a channel the requester was already
/// answered for. An unrelated or closed hostname resolves no local candidate
/// projection, stays [`RelayChannel::Public`], and its refusal remains opaque.
fn is_named_absence(channel: RelayChannel, reply: &HolderReply) -> bool {
    if channel != RelayChannel::Candidate || reply.status != NAMED_ABSENCE {
        return false;
    }
    if reply.body.len() > NAMED_ABSENCE_MAX_BODY {
        return false;
    }
    let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&reply.body) else {
        return false;
    };
    parsed.get("error").and_then(serde_json::Value::as_str) == Some(NO_CANDIDATE_STAGED_ERROR)
}

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
/// **Routing dimensions.** `host` selects an explicitly published head channel
/// when one exists; legacy any-host contracts retain path-only routing when no
/// exact binding is advertised. `path` then selects the longest mount.
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
/// | `Weight` | WIRED — the holder's own declared shed window, observed on the response path, never replicated (story 3.1; plan A:107) |
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

/// `Weight` selector term — no live declared shed for this holder. Byte-for-
/// byte identical to [`TERM_NOT_YET_WIRED`] so that, with no observation, the
/// fold returns exactly the order it returns today (see `ShedMemory::weight`).
pub const WEIGHT_UNCONSTRAINED: u32 = TERM_NOT_YET_WIRED;
/// `Weight` selector term — this holder declared backpressure and its own
/// window has not elapsed. Binary, not graded: see the design brief §4.2 for
/// why (no honest magnitude is on the wire; grading would invent precision).
pub const WEIGHT_SHEDDING: u32 = 1;

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
        // Weight — WIRED (story 3.1): the holder's own declared shed window.
        holder.shed_weight,
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
    /// The host this contract is bound to. `None` = a legacy ANY-host contract.
    pub host: Option<String>,
    /// Exact project-epr undertaking this route advertises. `None` is a legacy
    /// coherence peer: routable by name, never by commitment reference.
    pub commitment_id: Option<String>,
    pub epr_id: Option<String>,
}

impl HolderContract {
    /// A contract bound to no particular host — today's only shape.
    pub fn any_host(doorway_id: &str, origin: &str, url_path: &str) -> Self {
        Self {
            doorway_id: doorway_id.to_string(),
            origin: origin.to_string(),
            url_path: url_path.to_string(),
            host: None,
            commitment_id: None,
            epr_id: None,
        }
    }

    pub fn with_projection(
        mut self,
        commitment_id: Option<String>,
        epr_id: Option<String>,
    ) -> Self {
        self.commitment_id = commitment_id;
        self.epr_id = epr_id;
        self
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
    pub commitment_id: Option<String>,
    pub epr_id: Option<String>,
    pub liveness: HolderLiveness,
    /// How this holder is reached. [`RelayMode::Proxy`] for every holder today.
    pub relay_mode: RelayMode,
    /// The `Weight` selector term for this holder: [`WEIGHT_SHEDDING`] while a
    /// window it declared for itself stands, [`WEIGHT_UNCONSTRAINED`]
    /// otherwise. Carried (not recomputed at sort time) so a log line and a
    /// future admin read can state the value that actually ordered this
    /// holder.
    pub shed_weight: u32,
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
/// 3. `weights` — `doorway_id` → the `Weight` selector term ([`ShedMemory::weights`]).
/// 4. `self_doorway_id` — excluded; a doorway never relays to itself.
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
    weights: &HashMap<String, u32>,
    self_doorway_id: &str,
) -> Vec<NameHolder> {
    // Once any doorway advertises an exact contract for the requested name,
    // that name is a namespace boundary. A legacy any-host contract may keep
    // serving genuinely unnamed apps, but it must never become fallback bytes
    // for an explicitly published channel.
    let exact_host_advertised = key.host.as_deref().is_some_and(|asked| {
        contracts.iter().any(|contract| {
            contract
                .host
                .as_deref()
                .is_some_and(|bound| bound.eq_ignore_ascii_case(asked))
                && mount_covers(&contract.url_path, &key.path)
        })
    });
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
        if exact_host_advertised && contract.host.is_none() {
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
                    existing.commitment_id = contract.commitment_id.clone();
                    existing.epr_id = contract.epr_id.clone();
                    *existing_rank = rank;
                }
            }
            None => holders.push((
                NameHolder {
                    doorway_id: contract.doorway_id.clone(),
                    origin: contract.origin.trim_end_matches('/').to_string(),
                    url_path: contract.url_path.clone(),
                    host: contract.host.clone(),
                    commitment_id: contract.commitment_id.clone(),
                    epr_id: contract.epr_id.clone(),
                    liveness: liveness
                        .get(&contract.doorway_id)
                        .copied()
                        .unwrap_or_default(),
                    // The next rung derives this from the contract; every
                    // contract is proxy-reached today.
                    relay_mode: RelayMode::default(),
                    shed_weight: weights
                        .get(&contract.doorway_id)
                        .copied()
                        .unwrap_or(WEIGHT_UNCONSTRAINED),
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

/// **The widest fold**: every doorway this table holds a contract for, ordered
/// by [`SELECTOR_TERMS`], one entry per doorway keeping its most specific mount.
///
/// [`fold_candidate_holders`] narrows by mount because the question it answers
/// is "who serves this PATH". Some questions are addressed by EPR id instead —
/// the fair-trade receipt is asked as `/api/v1/receipt/{eprId}`, and a courier
/// doorway that holds no contract for that record also holds no mount to fold
/// against. Narrowing on the receipt's OWN path would fold against `/api/...`,
/// which no projection mount covers and which would silently yield only
/// root-mount holders.
///
/// So this is the same fold with the mount term omitted — deliberately, and in
/// one named place, rather than by passing a key that accidentally matches
/// everything. Every other term (self-exclusion, liveness, owner order) is
/// unchanged, and the ONE-hop budget still governs at the call site.
pub fn fold_all_holders(
    contracts: &[HolderContract],
    liveness: &HashMap<String, HolderLiveness>,
    weights: &HashMap<String, u32>,
    self_doorway_id: &str,
) -> Vec<NameHolder> {
    let mut holders: Vec<(NameHolder, (u8, usize))> = Vec::new();
    for contract in contracts {
        if contract.doorway_id == self_doorway_id {
            continue;
        }
        if contract.origin.trim().is_empty() || contract.doorway_id.trim().is_empty() {
            continue;
        }
        let rank = specificity(contract);
        match holders
            .iter_mut()
            .find(|(h, _)| h.doorway_id == contract.doorway_id)
        {
            Some((existing, existing_rank)) => {
                if rank > *existing_rank {
                    existing.url_path = contract.url_path.clone();
                    existing.host = contract.host.clone();
                    existing.commitment_id = contract.commitment_id.clone();
                    existing.epr_id = contract.epr_id.clone();
                    *existing_rank = rank;
                }
            }
            None => holders.push((
                NameHolder {
                    doorway_id: contract.doorway_id.clone(),
                    origin: contract.origin.trim_end_matches('/').to_string(),
                    url_path: contract.url_path.clone(),
                    host: contract.host.clone(),
                    commitment_id: contract.commitment_id.clone(),
                    epr_id: contract.epr_id.clone(),
                    liveness: liveness
                        .get(&contract.doorway_id)
                        .copied()
                        .unwrap_or_default(),
                    relay_mode: RelayMode::default(),
                    shed_weight: weights
                        .get(&contract.doorway_id)
                        .copied()
                        .unwrap_or(WEIGHT_UNCONSTRAINED),
                },
                rank,
            )),
        }
    }

    let mut ranked: Vec<(usize, NameHolder)> = holders
        .into_iter()
        .map(|(holder, _)| holder)
        .enumerate()
        .collect();
    ranked.sort_by_key(|(owner_order, holder)| selector_rank(holder, *owner_order));
    ranked.into_iter().map(|(_, holder)| holder).collect()
}

/// Why a holder's window was opened. The LABEL on the observation, never an
/// input to its magnitude — see the design brief §2.1. `Weight` stays binary
/// (§4.2) either way; this only names the reason for the metric and log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShedReason {
    /// The holder named its own window with `Retry-After`.
    RetryAfter,
    /// The holder's `Retry-After` header was PRESENT — a genuine 503 shed
    /// declaration — but its value did not parse as delta-seconds (e.g. an
    /// HTTP-date). The holder still declared a shed; it just didn't state a
    /// window we could read, so the default window applies.
    NoWindowNamed,
}

impl ShedReason {
    /// Every variant, so the CLOSED label vocabulary has exactly one home.
    /// `crate::metrics::register_all` pre-touches the counter from THIS array
    /// rather than from hand-written string literals: a half-done rename
    /// (`declared` lived on in the pre-touch and its test for a while after
    /// `as_label()` had already moved to `no_window_named`) then cannot
    /// happen — a new variant is pre-touched automatically, and a renamed one
    /// cannot leave a ghost series behind that no producer can ever
    /// increment.
    pub const ALL: [ShedReason; 2] = [ShedReason::RetryAfter, ShedReason::NoWindowNamed];

    pub fn as_label(self) -> &'static str {
        match self {
            ShedReason::RetryAfter => "retry_after",
            ShedReason::NoWindowNamed => "no_window_named",
        }
    }
}

/// Floor on the demotion window. A holder answering `Retry-After: 0` or `1`
/// must still be demoted long enough for the demotion to mean something — a
/// window shorter than this would make the term a silent no-op (a 0 expires
/// before the next fold reads it).
pub const SHED_WINDOW_MIN_SECS: u64 = 5;

/// Ceiling on the demotion window. This table is Category C and a restart
/// clears it; the discovery tick rebuilds liveness roughly every minute. A
/// window longer than a few ticks is unfalsifiable inside this doorway's own
/// knowledge horizon — we would be holding a grudge past the point where we
/// could know it was still true. 300s ≈ five discovery ticks.
pub const SHED_WINDOW_MAX_SECS: u64 = 300;

/// Window when a 503 declared backpressure (a `Retry-After` header was
/// present) but its value did not parse as delta-seconds
/// (`ShedReason::NoWindowNamed`). Never reached from a 429 or from
/// `X-Available-Permits` — neither ever opens a Weight window (see
/// `relay_one_hop`'s 503-only arm and its doc comment).
pub const SHED_WINDOW_DEFAULT_SECS: u64 = 30;

/// Hard ceiling on remembered sheds. The live federation holds units, not
/// hundreds; this is a ceiling against an unbounded id space, not a working
/// set.
const MAX_SHED_NOTES: usize = 256;

/// One holder's declared shed, remembered until the window it named elapses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShedNote {
    /// Absolute wall-clock second at which this observation stops counting.
    until_secs: u64,
    reason: ShedReason,
}

/// Parse a `Retry-After` value as delta-seconds only. The HTTP-date form is
/// not parsed: no peer in this tree emits it, and a wrong date parse would be
/// a silent mis-window — an unparseable value must fall to the caller's
/// default, not be read as "ignore" (the holder still declared a shed).
pub fn parse_retry_after_secs(raw: &str) -> Option<u64> {
    raw.trim().parse::<u64>().ok()
}

/// Every holder this doorway has HEARD declare backpressure, and until when.
///
/// Category C, in-process, never persisted, never published, never asked of a
/// sibling (see plan A:107 — a doorway serving/notifying a sibling's own
/// observation of a THIRD party would turn a plural projection into a
/// cluster). Pure with respect to time: every method takes `now_secs`, so the
/// unit tests need no clock and no timer.
#[derive(Debug, Default)]
pub struct ShedMemory {
    notes: HashMap<String, ShedNote>,
}

impl ShedMemory {
    /// Record a holder's declared shed. A fresh declaration REPLACES any
    /// standing window (the newest statement is the holder's current
    /// statement); it never extends one cumulatively — this is what keeps the
    /// memory from ratcheting under repeated sheds (design brief §7.1).
    ///
    /// Returns `Some(window_secs)` — the CLAMPED window actually opened —
    /// when a NEW window was opened, so the caller increments the counter and
    /// logs exactly once per declaration (a re-declaration still counts: it
    /// is a new declaration) WITHOUT recomputing the clamp a second time.
    /// Returns `None` when refused at the ceiling.
    pub fn note(
        &mut self,
        doorway_id: &str,
        declared: Option<u64>,
        reason: ShedReason,
        now_secs: u64,
    ) -> Option<u64> {
        self.prune(now_secs);
        if !self.notes.contains_key(doorway_id) && self.notes.len() >= MAX_SHED_NOTES {
            warn!(
                held = self.notes.len(),
                "name-route: shed memory at its ceiling — refusing a new holder rather than \
                 evicting a live one (see MAX_SHED_NOTES)"
            );
            return None;
        }
        let window = declared
            .map(|n| n.clamp(SHED_WINDOW_MIN_SECS, SHED_WINDOW_MAX_SECS))
            .unwrap_or(SHED_WINDOW_DEFAULT_SECS);
        let until_secs = now_secs.saturating_add(window);
        self.notes
            .insert(doorway_id.to_string(), ShedNote { until_secs, reason });
        Some(window)
    }

    /// [`WEIGHT_SHEDDING`] while the window stands, [`WEIGHT_UNCONSTRAINED`]
    /// otherwise. Does not prune — callers that need a fresh snapshot call
    /// [`Self::prune`] or [`Self::weights`] first.
    pub fn weight(&self, doorway_id: &str, now_secs: u64) -> u32 {
        match self.notes.get(doorway_id) {
            Some(note) if now_secs < note.until_secs => WEIGHT_SHEDDING,
            _ => WEIGHT_UNCONSTRAINED,
        }
    }

    /// Drop every elapsed note. Returns the ids it dropped, so the caller can
    /// log each promotion exactly once. Called on every write and every fold.
    pub fn prune(&mut self, now_secs: u64) -> Vec<String> {
        let elapsed: Vec<String> = self
            .notes
            .iter()
            .filter(|(_, note)| now_secs >= note.until_secs)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &elapsed {
            self.notes.remove(id);
        }
        elapsed
    }

    /// A `{doorway_id -> weight}` snapshot for the fold. Every entry it
    /// returns is live as of `now_secs` — the caller is expected to have
    /// pruned already (e.g. `NameRouteTable`'s fold/write paths, which prune
    /// under their own write lock before taking this read-only snapshot); this
    /// method itself never mutates, so it filters rather than removing.
    pub fn weights(&self, now_secs: u64) -> HashMap<String, u32> {
        self.notes
            .iter()
            .filter(|(_, note)| now_secs < note.until_secs)
            .map(|(id, _)| (id.clone(), WEIGHT_SHEDDING))
            .collect()
    }

    /// Live (unelapsed) note count as of `now_secs`. See [`Self::weights`] for
    /// why this filters rather than pruning.
    pub fn live_len(&self, now_secs: u64) -> usize {
        self.notes
            .values()
            .filter(|note| now_secs < note.until_secs)
            .count()
    }

    /// True iff at least one held note has elapsed as of `now_secs`. A pure
    /// `&self` check so a resolve can tell, under a READ guard alone, whether
    /// the write-locked prune is worth taking at all — the hot path (nothing
    /// elapsed) never needs the write lock (design brief §3.6/§6; the
    /// promotion-visibility follow-up, S3).
    pub fn has_elapsed(&self, now_secs: u64) -> bool {
        self.notes.values().any(|note| now_secs >= note.until_secs)
    }
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
    /// The `Weight` term's memory — deliberately NOT wiped by `replace_all`.
    /// `liveness` is rebuilt wholesale by every discovery tick (~60s), so a
    /// `HolderLiveness::Shedding` set by `note_shed` survives at most one
    /// tick. A declared window survives ticks because the holder said how
    /// long it would be busy for, and that statement did not expire when our
    /// probe ran (design brief §7.5 / §9.3 — this is the whole reason `Weight`
    /// exists as a term separate from `Liveness`).
    shed: RwLock<ShedMemory>,
    /// Test-observable count of prune PASSES that took the `shed` write lock
    /// (S3 follow-up to story 3.1: `weights_snapshot`'s hot path must never
    /// take it when nothing has elapsed). Never read in production.
    #[cfg(test)]
    prune_write_passes: std::sync::atomic::AtomicU64,
    /// Story 4.2 slice 1: wall-clock second (`now_secs`) of the last PER-HOLDER
    /// install (`replace_holder` — the doorbell/refresh-verb pull path), keyed
    /// by `doorway_id`. Deliberately NOT touched by `replace_all` (the 60s
    /// poll's writer) — this is what lets [`Self::replace_all_at`] tell "a
    /// doorbell installed this holder more recently than this poll round
    /// started" and skip clobbering it with a now-stale batch (C2 monotonic,
    /// slice-1 design §3.2).
    holder_installed_at: RwLock<HashMap<String, u64>>,
    /// The digest this table last installed for each holder — read by the
    /// doorbell receiver's C6b idempotency check ([`Self::held_digest`]) and
    /// kept in step by both [`Self::replace_holder`] and
    /// [`Self::replace_all_at`].
    held_digests: RwLock<HashMap<String, String>>,
}

/// Exact project-epr reference resolution. Unlike the ordinary holder fold,
/// conflicts are not ranked: two distinct current holders claiming one exact
/// contract are an unavailable authority boundary, never a retry list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactCommitmentHolder {
    Absent,
    Unique(NameHolder),
    Conflict,
}

/// Wall-clock seconds since the epoch — the runtime boundary for
/// [`ShedMemory`]'s otherwise-pure, clock-free logic. Exactly `EdgeClock`'s
/// body (`server/membrane.rs`), the crate's established pattern: pure logic
/// takes seconds, the runtime supplies them here at the one boundary. Kept
/// private so a unit test can never reach for it — tests pass literal
/// seconds to `ShedMemory` directly instead.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
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
    ///
    /// **Deliberately does NOT touch `shed`.** `liveness` is rebuilt wholesale
    /// every tick and forgets a `note_shed` demotion inside ~60s; `shed`
    /// remembers a holder's own declared window for as long as the holder
    /// said it would last, independent of the discovery cycle. Wiping it here
    /// would collapse `Weight` back into `Liveness` and undo the whole point
    /// of this term (design brief §7.5 / §9.3).
    pub fn replace_all(
        &self,
        contracts: Vec<HolderContract>,
        liveness: HashMap<String, HolderLiveness>,
    ) {
        self.replace_all_at(contracts, liveness, HashMap::new(), now_secs());
    }

    /// [`Self::replace_all`], but for a poll round that STARTED at
    /// `fetch_started` and carries each probed holder's manifest `digest`
    /// (story 4.2 slice 1, design §3.2). Two things a per-holder install
    /// ([`Self::replace_holder`] — the doorbell/refresh-verb pull path) can
    /// beat this whole-batch fold to:
    ///
    /// - **C2 monotonic.** A holder this table installed via
    ///   [`Self::replace_holder`] AFTER `fetch_started` is PROTECTED — this
    ///   round's (now-stale) copy of that holder's contracts/liveness/digest
    ///   is dropped, and the holder's CURRENT rows are kept exactly as
    ///   `replace_holder` left them. A slower poll can never regress a
    ///   doorbell-fresh install.
    /// - **held_digests.** Every OTHER holder's digest is recorded, so
    ///   [`Self::held_digest`] reflects the poll's view for a holder no
    ///   doorbell has ever touched.
    ///
    /// Last-good preservation is unchanged from `replace_all`: a batch that is
    /// empty AFTER protecting, with existing rows already held, leaves the
    /// table as-is.
    pub fn replace_all_at(
        &self,
        mut contracts: Vec<HolderContract>,
        mut liveness: HashMap<String, HolderLiveness>,
        digests: HashMap<String, String>,
        fetch_started: u64,
    ) {
        let protected: std::collections::HashSet<String> = {
            let installed = self
                .holder_installed_at
                .read()
                .expect("name-route lock poisoned");
            installed
                .iter()
                // `>=`, not `>`: both clocks are whole seconds, so an install
                // that landed in the same second the poll STARTED cannot be
                // proven older than the poll's fetch — and clobbering it
                // re-opens the exact race this guard exists to close. Erring
                // toward the doorbell-fresh row costs at most one 60s poll.
                .filter(|(_, &at)| at >= fetch_started)
                .map(|(id, _)| id.clone())
                .collect()
        };
        if !protected.is_empty() {
            contracts.retain(|c| !protected.contains(&c.doorway_id));
            liveness.retain(|id, _| !protected.contains(id));
        }

        {
            let mut live = self.liveness.write().expect("name-route lock poisoned");
            // Keep a protected holder's CURRENT liveness (set by
            // `replace_holder`), then fold in this round's verdicts for
            // everyone else.
            let mut merged: HashMap<String, HolderLiveness> = protected
                .iter()
                .filter_map(|id| live.get(id).map(|lv| (id.clone(), *lv)))
                .collect();
            merged.extend(liveness);
            *live = merged;
        }

        if !digests.is_empty() {
            let mut held = self.held_digests.write().expect("name-route lock poisoned");
            for (id, digest) in digests {
                if !protected.contains(&id) {
                    held.insert(id, digest);
                }
            }
        }

        let mut table = self.contracts.write().expect("name-route lock poisoned");
        if !protected.is_empty() {
            for existing in table.iter() {
                if protected.contains(&existing.doorway_id) {
                    contracts.push(existing.clone());
                }
            }
            contracts.sort_by(|left, right| {
                left.doorway_id
                    .cmp(&right.doorway_id)
                    .then_with(|| left.url_path.cmp(&right.url_path))
                    .then_with(|| left.host.cmp(&right.host))
            });
        }
        if contracts.is_empty() && !table.is_empty() {
            debug!(
                held = table.len(),
                "name-route refresh returned no contracts — preserving last-good table"
            );
            return;
        }
        *table = contracts;
    }

    /// Install ONE holder's contracts + liveness + digest, replacing only
    /// that holder's rows (never the whole table). `installed_at` is stamped
    /// so a later, slower [`Self::replace_all_at`] round that STARTED before
    /// this install cannot clobber it (C2 monotonic) — see
    /// `holder_installed_at`'s doc comment. Used by the doorbell receiver and
    /// the admin refresh verb (`services::federation::install_holder_snapshot`).
    pub fn replace_holder(
        &self,
        doorway_id: &str,
        mut contracts: Vec<HolderContract>,
        liveness: HolderLiveness,
        digest: String,
        installed_at: u64,
    ) {
        {
            let mut live = self.liveness.write().expect("name-route lock poisoned");
            live.insert(doorway_id.to_string(), liveness);
        }
        {
            let mut table = self.contracts.write().expect("name-route lock poisoned");
            table.retain(|c| c.doorway_id != doorway_id);
            table.append(&mut contracts);
            table.sort_by(|left, right| {
                left.doorway_id
                    .cmp(&right.doorway_id)
                    .then_with(|| left.url_path.cmp(&right.url_path))
                    .then_with(|| left.host.cmp(&right.host))
            });
        }
        {
            let mut installed = self
                .holder_installed_at
                .write()
                .expect("name-route lock poisoned");
            installed.insert(doorway_id.to_string(), installed_at);
        }
        {
            let mut held = self.held_digests.write().expect("name-route lock poisoned");
            held.insert(doorway_id.to_string(), digest);
        }
    }

    /// The digest this table last installed for `doorway_id`, or `None` when
    /// this doorway has never installed anything for that holder. Read by the
    /// doorbell receiver's C6b idempotency check: a doorbell whose digest
    /// already matches the held one is a no-op.
    pub fn held_digest(&self, doorway_id: &str) -> Option<String> {
        self.held_digests
            .read()
            .expect("name-route lock poisoned")
            .get(doorway_id)
            .cloned()
    }

    /// Candidate holders for a [`RouteKey`] (see [`fold_candidate_holders`]).
    pub fn holders_for(&self, key: &RouteKey, self_doorway_id: &str) -> Vec<NameHolder> {
        let contracts = self.contracts.read().expect("name-route lock poisoned");
        let liveness = self.liveness.read().expect("name-route lock poisoned");
        let weights = self.weights_snapshot();
        fold_candidate_holders(key, &contracts, &liveness, &weights, self_doorway_id)
    }

    /// Candidate holders when this doorway's own router has already proven
    /// that the request belongs to an explicit hostname namespace. Missing or
    /// stale peer advertisements fail closed instead of widening to legacy
    /// any-host contracts.
    pub fn holders_for_exact_host(&self, key: &RouteKey, self_doorway_id: &str) -> Vec<NameHolder> {
        self.holders_for(key, self_doorway_id)
            .into_iter()
            .filter(|holder| {
                holder.host.as_deref().is_some_and(|bound| {
                    key.host
                        .as_deref()
                        .is_some_and(|asked| bound.eq_ignore_ascii_case(asked))
                })
            })
            .collect()
    }

    /// Every doorway this table holds a contract for (see
    /// [`fold_all_holders`]) — the fold for a question addressed by EPR ID
    /// rather than by a mount path.
    pub fn all_holders(&self, self_doorway_id: &str) -> Vec<NameHolder> {
        let contracts = self.contracts.read().expect("name-route lock poisoned");
        let liveness = self.liveness.read().expect("name-route lock poisoned");
        let weights = self.weights_snapshot();
        fold_all_holders(&contracts, &liveness, &weights, self_doorway_id)
    }

    pub fn exact_commitment_holder(
        &self,
        host: Option<&str>,
        commitment_id: &str,
    ) -> ExactCommitmentHolder {
        let asked_host = RouteKey::new(host, "/").host;
        let contracts = self.contracts.read().expect("name-route lock poisoned");
        let liveness = self.liveness.read().expect("name-route lock poisoned");
        let exact_host_advertised = asked_host.as_deref().is_some_and(|asked| {
            contracts.iter().any(|contract| {
                contract.commitment_id.as_deref() == Some(commitment_id)
                    && contract
                        .host
                        .as_deref()
                        .is_some_and(|bound| bound.eq_ignore_ascii_case(asked))
            })
        });
        let mut matches: Vec<NameHolder> = Vec::new();
        for contract in contracts.iter().filter(|contract| {
            contract.commitment_id.as_deref() == Some(commitment_id)
                && host_matches(contract.host.as_deref(), asked_host.as_deref())
                && (!exact_host_advertised || contract.host.is_some())
                && contract.epr_id.as_deref().is_some_and(|id| !id.is_empty())
        }) {
            let holder = NameHolder {
                doorway_id: contract.doorway_id.clone(),
                origin: contract.origin.trim_end_matches('/').to_string(),
                url_path: contract.url_path.clone(),
                host: contract.host.clone(),
                commitment_id: contract.commitment_id.clone(),
                epr_id: contract.epr_id.clone(),
                liveness: liveness
                    .get(&contract.doorway_id)
                    .copied()
                    .unwrap_or_default(),
                relay_mode: RelayMode::Proxy,
                shed_weight: WEIGHT_UNCONSTRAINED,
            };
            if !matches.contains(&holder) {
                matches.push(holder);
            }
        }
        match matches.len() {
            0 => ExactCommitmentHolder::Absent,
            1 => ExactCommitmentHolder::Unique(matches.remove(0)),
            _ => ExactCommitmentHolder::Conflict,
        }
    }

    /// Record that a holder SHED a relay (503). The discovery probe cannot tell
    /// a shed from a death, but a relay can — so the one surface that observes
    /// it writes it, and the next fold orders that holder after the serving
    /// ones. Cleared by the next discovery tick's `replace_all`.
    pub fn note_shed(&self, doorway_id: &str) {
        let mut liveness = self.liveness.write().expect("name-route lock poisoned");
        liveness.insert(doorway_id.to_string(), HolderLiveness::Shedding);
    }

    /// Record the `Weight` observation from a relay reply: the holder DECLARED
    /// its own backpressure. A no-op when `observed.reason` is `None` — either
    /// a bare 503 with no `Retry-After` (the error-mapper case, §2.1) or a 503
    /// carrying `x-membrane` (a caller-scoped answer, never a capacity
    /// statement) — either way `note_shed` still demotes it via `Liveness`,
    /// but `Weight` must not mint a window from it. Two arms, two concerns:
    /// `note_shed` is the coarse, tick-lived `Liveness` demotion; this is the
    /// fine-grained, holder-timed `Weight` demotion that survives a discovery
    /// tick (design brief §7.5).
    ///
    /// A holder can only ever open a window on ITS OWN `doorway_id` — the id
    /// recorded is the id of the doorway whose response this courier just
    /// read, never a third party (§7.4: the blast radius of a lying holder is
    /// itself, only).
    pub fn note_backpressure(&self, observed: &ObservedShed) {
        let Some(reason) = observed.reason else {
            return;
        };
        let now = now_secs();
        let opened_window = {
            let mut shed = self.shed.write().expect("name-route lock poisoned");
            shed.note(&observed.doorway_id, observed.declared_secs, reason, now)
        };
        if let Some(window) = opened_window {
            crate::metrics::inc_holder_demoted(reason.as_label());
            warn!(
                target: "name_route_shed",
                counter = "doorway_name_route_holder_demoted_total",
                holder = %observed.doorway_id,
                origin = %observed.origin,
                window_secs = window,
                reason = reason.as_label(),
                "name-route: holder declared backpressure — demoted behind its siblings for the window it named"
            );
        }
        self.prune_shed_and_log(now);
    }

    /// A `{doorway_id -> Weight}` snapshot for the fold, pruning elapsed
    /// windows first (and logging/gauging each promotion) so a fold never
    /// reads a stale demotion and a promotion is always visible exactly once.
    fn weights_snapshot(&self) -> HashMap<String, u32> {
        let now = now_secs();
        // Hot path: a single READ guard. Most resolves find nothing elapsed
        // (the fold runs far more often than a window's own lifetime), and
        // reading never needs to fight a write lock for it — S3 follow-up to
        // story 3.1 (weights_snapshot was taking `shed.write()` on EVERY
        // resolve).
        let (weights, any_elapsed) = {
            let shed = self.shed.read().expect("name-route lock poisoned");
            (shed.weights(now), shed.has_elapsed(now))
        };
        if !any_elapsed {
            return weights;
        }
        // Something elapsed: promote it (prune + log the promotion + refresh
        // the gauge) under the write lock, ONLY on this less-common path,
        // then re-read the now-current snapshot.
        self.prune_shed_and_log(now);
        let shed = self.shed.read().expect("name-route lock poisoned");
        shed.weights(now)
    }

    /// Drop every elapsed shed window, log each promotion once, and refresh
    /// the live-demoted gauge. Called on every write (`note_backpressure`) and
    /// every fold (`weights_snapshot`) — promotion is the passage of time and
    /// has no event of its own, so this is the only place it becomes visible.
    fn prune_shed_and_log(&self, now_secs: u64) {
        #[cfg(test)]
        self.prune_write_passes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dropped = {
            let mut shed = self.shed.write().expect("name-route lock poisoned");
            shed.prune(now_secs)
        };
        for doorway_id in &dropped {
            info!(
                target: "name_route_shed",
                holder = %doorway_id,
                "name-route: the window this holder named has elapsed — restored to owner order"
            );
        }
        let live = {
            let shed = self.shed.read().expect("name-route lock poisoned");
            shed.live_len(now_secs)
        };
        crate::metrics::set_holders_demoted(live as i64);
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
    /// The holder's `x-elohim-receipt` pointer, verbatim. `None` when it stated
    /// none. A relative route, so it resolves against whichever doorway the
    /// client is talking to — which is the courier, which then merges its own
    /// projection credit into the answer (see `routes::receipt`).
    pub receipt: Option<String>,
    /// The holder's own `cache-control` for this answer. `None` → the relay's
    /// default (`no-store`) governs.
    pub cache_control: Option<String>,
    /// The holder's `Retry-After`, parsed as delta-seconds
    /// ([`parse_retry_after_secs`]). `None` when it stated none, or stated one
    /// that did not parse (see `retry_after_present` to distinguish those two
    /// cases), or stated an HTTP-date (not parsed — no peer in this tree
    /// emits one; design brief §2.1/§3.5).
    pub retry_after_secs: Option<u64>,
    /// True iff the holder's response carried a `Retry-After` header AT ALL,
    /// regardless of whether its value parsed. A 503 with the header present
    /// but unparseable is still a genuine shed declaration — the holder said
    /// SOMETHING, it just didn't state a window we could read — so this is
    /// what tells `relay_one_hop` "declared, default window" apart from "no
    /// declaration at all" (`ShedReason::NoWindowNamed` vs `None`).
    pub retry_after_present: bool,
    /// True when the response carries `x-membrane` — this doorway's OWN
    /// membrane-challenge marker (`server::http`'s `Verdict::Challenge`,
    /// stamped on its 429s). A caller-scoped answer about WHO ASKED, never a
    /// capacity statement about the holder — `relay_one_hop` reads this to
    /// refuse a Weight window even on a 503 that happens to carry it (the
    /// symmetric defense to never reading a 429 as a shed at all).
    pub has_membrane_verdict: bool,
    pub body: Vec<u8>,
}

/// The relay decision.
// `HolderReply` gained `retry_after_secs`/`retry_after_present`/
// `has_membrane_verdict` in story 3.1 (and its BLOCKER follow-up),
// crossing clippy's large-enum-variant threshold against `AllFailed`/
// `NoCandidates`. Boxing `reply` would ripple `*reply`/`&*reply` through
// every existing match arm and test across this module and http.rs — out of
// scope for a two-field addition. `Served` is already the rare, terminal-hop
// case (one per relay attempt, never hot-looped), so the extra stack bytes
// are not a real cost.
#[allow(clippy::large_enum_variant)]
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

/// One holder's declared backpressure, as observed on this relay attempt —
/// carried out of `relay_one_hop` so the caller can act (both `note_shed` and
/// `note_backpressure`) without the relay loop owning mutable state (see
/// `RelayOutcome`'s own discipline, restated here for the new record).
#[derive(Debug, Clone)]
pub struct ObservedShed {
    pub doorway_id: String,
    /// The window the holder named, if it named one (parsed `Retry-After`).
    pub declared_secs: Option<u64>,
    /// `None` when the shed did NOT declare backpressure — a bare 503 with no
    /// `Retry-After` (the error-mapper case, design brief §2.1) OR a 503/429
    /// carrying `x-membrane` (a caller-scoped answer, post-landing BLOCKER
    /// ruling). `note_shed`/`Liveness` still demotes this holder (unchanged,
    /// existing behaviour); `Weight` does not, because there is no honest —
    /// or honestly holder-scoped — window to honour. `Some(reason)` is the
    /// label for the metric and log when a window IS opened.
    pub reason: Option<ShedReason>,
    /// Carried only for the log line.
    pub origin: String,
}

/// A verdict plus the holders observed shedding, so the caller can demote them
/// in the table without the relay loop owning mutable state.
#[derive(Debug)]
pub struct RelayOutcome {
    pub verdict: RelayVerdict,
    pub shed_doorways: Vec<ObservedShed>,
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
/// - `404` carrying the candidate channel's named absence, when `channel` is
///   [`RelayChannel::Candidate`] → **answered, stop.** See [`NAMED_ABSENCE`]:
///   the channel that can be SHOWN by relaying a holder's 200 must also be
///   withdrawable by relaying that same holder's structured absence, or a
///   relayed candidate could never be un-shown.
/// - any other status (incl. the sibling's bare `404`) → it did not serve:
///   a failed attempt, try the next holder; if it was the last, the caller
///   keeps ITS original status. A sibling 404 therefore surfaces as our own
///   404, never as a rewritten one.
/// - transport error → a failed attempt, same as above.
pub async fn relay_one_hop<F, Fut>(
    holders: &[NameHolder],
    channel: RelayChannel,
    fetch: F,
) -> RelayOutcome
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
            // (2xx/3xx), an authoritative refusal (403), and the candidate
            // channel's named absence (404 + its structured body) are the same
            // kind of thing here: the doorway that owns the fold has spoken, so
            // the relay stops and the client receives what it said.
            Ok(reply)
                if (200..400).contains(&reply.status)
                    || reply.status == AUTHORITATIVE_REFUSAL
                    || is_named_absence(channel, &reply) =>
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
            // 503 ONLY — a statement about the HOLDER's liveness, not about
            // the requester or the record. A 429 is deliberately NOT in this
            // arm (it falls to the generic `Ok(reply)` arm below, exactly as
            // it did before story 3.1) — see the ruling on 429 recorded on
            // `ObservedShed` and pinned by
            // `a_429_is_about_the_caller_and_never_demotes_the_holder`: the
            // only 429 this service emits is the membrane challenge, which
            // scores the CLIENT (`relay`'s own authorization/cookie are
            // forwarded to the holder, so the holder is answering about
            // WHO ASKED, not about its own capacity). Reading it as a shed
            // would let one challenged human at holder A get holder A
            // demoted for every OTHER visitor at courier B — a free,
            // repeatable traffic-steering lever. `reason` is `None` for a
            // bare 503 with no `Retry-After` (the error mapper reporting a
            // failure, not a declared shed, §2.1) OR for a 503 that itself
            // carries `x-membrane` (a caller-scoped answer riding a 503,
            // same defect as the 429 case, defended symmetrically) — either
            // way `Liveness` (`note_shed`, called on every entry in
            // `shed_doorways` regardless) still demotes it exactly as
            // before; only `Weight` declines to mint a window.
            Ok(reply) if reply.status == 503 => {
                let reason = if reply.has_membrane_verdict || !reply.retry_after_present {
                    None
                } else if reply.retry_after_secs.is_some() {
                    Some(ShedReason::RetryAfter)
                } else {
                    Some(ShedReason::NoWindowNamed)
                };
                debug!(
                    holder = %holder.doorway_id,
                    origin = %holder.origin,
                    has_membrane_verdict = reply.has_membrane_verdict,
                    "name-route: holder is shedding — trying the next holder"
                );
                shed_doorways.push(ObservedShed {
                    doorway_id: holder.doorway_id.clone(),
                    declared_secs: reply.retry_after_secs,
                    reason,
                    origin: holder.origin.clone(),
                });
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
    if let Some(receipt) = reply.receipt.as_deref() {
        builder = builder.header(RECEIPT_HEADER, receipt);
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

    fn exact_contract(
        doorway: &str,
        origin: &str,
        host: &str,
        commitment: &str,
        epr: &str,
    ) -> HolderContract {
        HolderContract {
            host: Some(host.to_string()),
            commitment_id: Some(commitment.to_string()),
            epr_id: Some(epr.to_string()),
            ..HolderContract::any_host(doorway, origin, "/")
        }
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
            commitment_id: None,
            epr_id: None,
            liveness,
            relay_mode: RelayMode::Proxy,
            shed_weight: WEIGHT_UNCONSTRAINED,
        }
    }

    fn reply(status: u16, body: &str) -> HolderReply {
        HolderReply {
            status,
            content_type: Some("text/html".to_string()),
            standing: None,
            bundle: None,
            receipt: None,
            cache_control: None,
            retry_after_secs: None,
            retry_after_present: false,
            has_membrane_verdict: false,
            body: body.as_bytes().to_vec(),
        }
    }

    // ── ShedMemory (T1) ───────────────────────────────────────────────────────
    // All synchronous, all literal seconds — no clock, no timer, no sleep.

    /// T1.1 — the window is the holder's number, not ours.
    #[test]
    fn a_declared_retry_after_is_honoured_as_the_demotion_window() {
        let mut mem = ShedMemory::default();
        assert_eq!(
            mem.note("b-doorway", Some(60), ShedReason::RetryAfter, 1_000),
            Some(60),
            "the window returned IS the clamped window opened"
        );
        assert_eq!(mem.weight("b-doorway", 1_059), WEIGHT_SHEDDING);
        assert_eq!(mem.weight("b-doorway", 1_060), WEIGHT_UNCONSTRAINED);
    }

    /// T1.2 — THE REGRESSION PIN: no observation is today's value, by identity.
    #[test]
    fn an_unobserved_holder_weighs_exactly_what_an_unwired_term_weighs() {
        let mem = ShedMemory::default();
        assert_eq!(mem.weight("never-seen", 1_000), TERM_NOT_YET_WIRED);
        assert_eq!(TERM_NOT_YET_WIRED, WEIGHT_UNCONSTRAINED);
    }

    /// T1.3 — a zero must not make the term a silent no-op.
    #[test]
    fn a_window_shorter_than_the_floor_is_raised_to_it() {
        for declared in [Some(0), Some(1)] {
            let mut mem = ShedMemory::default();
            mem.note("b-doorway", declared, ShedReason::RetryAfter, 1_000);
            assert_eq!(
                mem.weight("b-doorway", 1_004),
                WEIGHT_SHEDDING,
                "declared={declared:?} must still be demoted at now+4 (floor={SHED_WINDOW_MIN_SECS})"
            );
        }
    }

    /// T1.4 — we do not hold a grudge past our own knowledge horizon.
    #[test]
    fn a_window_longer_than_the_ceiling_is_capped() {
        let mut mem = ShedMemory::default();
        mem.note("b-doorway", Some(86_400), ShedReason::RetryAfter, 1_000);
        assert_eq!(
            mem.weight("b-doorway", 1_000 + SHED_WINDOW_MAX_SECS + 1),
            WEIGHT_UNCONSTRAINED
        );
    }

    /// T1.5 — a declaration with no window gets the default.
    #[test]
    fn a_declaration_with_no_window_gets_the_default() {
        let mut mem = ShedMemory::default();
        mem.note("b-doorway", None, ShedReason::NoWindowNamed, 1_000);
        assert_eq!(
            mem.weight("b-doorway", 1_000 + SHED_WINDOW_DEFAULT_SECS - 1),
            WEIGHT_SHEDDING
        );
        assert_eq!(
            mem.weight("b-doorway", 1_000 + SHED_WINDOW_DEFAULT_SECS),
            WEIGHT_UNCONSTRAINED
        );
    }

    /// T1.6 — the newest statement is the current statement; the memory
    /// cannot ratchet.
    #[test]
    fn a_fresh_declaration_replaces_the_standing_window_rather_than_extending_it() {
        let mut mem = ShedMemory::default();
        mem.note("b-doorway", Some(300), ShedReason::RetryAfter, 1_000);
        mem.note("b-doorway", Some(10), ShedReason::RetryAfter, 1_001);
        assert_eq!(mem.weight("b-doorway", 1_011), WEIGHT_UNCONSTRAINED);
    }

    /// T1.7 — `note` reports only the FIRST opening... and a re-declaration is
    /// STILL reported, since it is a new declaration, so the counter counts
    /// declarations, not standing windows.
    #[test]
    fn note_reports_only_the_first_opening_so_the_counter_counts_declarations() {
        let mut mem = ShedMemory::default();
        assert!(
            mem.note("b-doorway", Some(60), ShedReason::RetryAfter, 1_000)
                .is_some(),
            "the first declaration always opens a window"
        );
        assert!(
            mem.note("b-doorway", Some(60), ShedReason::RetryAfter, 1_010)
                .is_some(),
            "a re-declaration is a new declaration — it counts too"
        );
    }

    /// T1.8 — returned ids are exactly the elapsed ones, and a second prune at
    /// the same instant returns empty (so promotion is logged once).
    #[test]
    fn prune_drops_only_elapsed_notes_and_names_each_one_once() {
        let mut mem = ShedMemory::default();
        mem.note("b-doorway", Some(5), ShedReason::RetryAfter, 1_000); // elapses at 1_005
        mem.note("c-doorway", Some(300), ShedReason::RetryAfter, 1_000); // still live at 1_005
        let dropped = mem.prune(1_005);
        assert_eq!(dropped, vec!["b-doorway".to_string()]);
        assert_eq!(mem.weight("c-doorway", 1_005), WEIGHT_SHEDDING);
        assert!(
            mem.prune(1_005).is_empty(),
            "a second prune at the same instant must not re-name an already-dropped id"
        );
    }

    /// T1.9 — the memory refuses a NEW holder at its ceiling rather than
    /// evicting a live one.
    #[test]
    fn the_memory_refuses_a_new_holder_at_its_ceiling_rather_than_evicting_a_live_one() {
        let mut mem = ShedMemory::default();
        for n in 0..MAX_SHED_NOTES {
            assert!(mem
                .note(
                    &format!("holder-{n}"),
                    Some(300),
                    ShedReason::RetryAfter,
                    1_000
                )
                .is_some());
        }
        assert_eq!(mem.live_len(1_000), MAX_SHED_NOTES);
        assert!(
            mem.note("one-too-many", Some(300), ShedReason::RetryAfter, 1_000)
                .is_none(),
            "the ceiling refuses a new id rather than evicting a live one"
        );
        assert_eq!(
            mem.live_len(1_000),
            MAX_SHED_NOTES,
            "every existing note survives intact"
        );
        assert_eq!(mem.weight("one-too-many", 1_000), WEIGHT_UNCONSTRAINED);
    }

    /// T1.10 — the holder declared a shed even if we could not read its number
    /// (the header was PRESENT — see `retry_after_present` — but its value
    /// did not parse; the caller passes `ShedReason::NoWindowNamed` for
    /// exactly this shape).
    #[test]
    fn an_unparseable_retry_after_still_opens_the_default_window() {
        let declared = parse_retry_after_secs("not-a-number");
        assert_eq!(declared, None);
        let mut mem = ShedMemory::default();
        assert!(mem
            .note("b-doorway", declared, ShedReason::NoWindowNamed, 1_000)
            .is_some());
        assert_eq!(
            mem.weight("b-doorway", 1_000 + SHED_WINDOW_DEFAULT_SECS - 1),
            WEIGHT_SHEDDING
        );
    }

    // ── parse_retry_after_secs (T4.1 / T4.2) ─────────────────────────────────

    /// T4.1 — matches `storage_proxy.rs`'s delta-seconds parse; whitespace is
    /// tolerated so a courteous holder's `" 60 "` is still honoured.
    #[test]
    fn retry_after_is_read_as_delta_seconds() {
        assert_eq!(parse_retry_after_secs("60"), Some(60));
        assert_eq!(parse_retry_after_secs(" 60 "), Some(60));
    }

    /// T4.2 — a silent mis-window is worse than no window: an HTTP-date is
    /// NOT read as a (wildly wrong) number of seconds.
    #[test]
    fn an_http_date_retry_after_is_not_mistaken_for_seconds() {
        assert_eq!(
            parse_retry_after_secs("Wed, 21 Oct 2026 07:28:00 GMT"),
            None
        );
    }

    /// T4.3 — a holder reply with no backpressure headers declares none.
    #[test]
    fn a_holder_reply_with_no_backpressure_headers_declares_none() {
        let r = reply(200, "ok");
        assert_eq!(r.retry_after_secs, None);
        assert!(!r.retry_after_present);
        assert!(!r.has_membrane_verdict);
    }

    #[test]
    fn exact_commitment_reference_requires_one_coherent_current_holder() {
        let table = NameRouteTable::new();
        let exact = exact_contract(
            "alpha",
            "https://alpha.example",
            "Garden.Example",
            "project-epr-garden",
            "garden-epr",
        );
        table.replace_all(
            vec![exact.clone(), exact],
            HashMap::from([("alpha".into(), HolderLiveness::Serving)]),
        );
        let ExactCommitmentHolder::Unique(holder) =
            table.exact_commitment_holder(Some("garden.example:443"), "project-epr-garden")
        else {
            panic!("identical advertisements must deduplicate");
        };
        assert_eq!(holder.doorway_id, "alpha");
        assert_eq!(holder.epr_id.as_deref(), Some("garden-epr"));

        table.replace_all(
            vec![
                HolderContract::any_host("alpha", "https://alpha.example", "/")
                    .with_projection(Some("project-epr-garden".into()), Some("garden-epr".into())),
            ],
            HashMap::new(),
        );
        assert!(matches!(
            table.exact_commitment_holder(Some("localhost:8889"), "project-epr-garden"),
            ExactCommitmentHolder::Unique(_)
        ));

        table.replace_all(
            vec![
                exact_contract(
                    "alpha",
                    "https://alpha.example",
                    "garden.example",
                    "project-epr-garden",
                    "garden-epr",
                ),
                HolderContract::any_host("gamma", "https://gamma.example", "/")
                    .with_projection(Some("project-epr-garden".into()), Some("garden-epr".into())),
            ],
            HashMap::new(),
        );
        let ExactCommitmentHolder::Unique(holder) =
            table.exact_commitment_holder(Some("garden.example"), "project-epr-garden")
        else {
            panic!("the exact hostname claim must exclude a wildcard claim");
        };
        assert_eq!(holder.doorway_id, "alpha");

        table.replace_all(
            vec![
                exact_contract(
                    "alpha",
                    "https://alpha.example",
                    "garden.example",
                    "project-epr-garden",
                    "garden-epr",
                ),
                exact_contract(
                    "gamma",
                    "https://gamma.example",
                    "garden.example",
                    "project-epr-garden",
                    "garden-epr",
                ),
            ],
            HashMap::new(),
        );
        assert_eq!(
            table.exact_commitment_holder(Some("garden.example"), "project-epr-garden"),
            ExactCommitmentHolder::Conflict
        );

        table.replace_all(
            vec![contract("alpha", "https://alpha.example", "/")],
            HashMap::new(),
        );
        assert_eq!(
            table.exact_commitment_holder(Some("garden.example"), "project-epr-garden"),
            ExactCommitmentHolder::Absent,
            "a legacy coherence row cannot route an exact reference"
        );
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
            &HashMap::new(),
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
            &HashMap::new(),
            "a-doorway",
        );
        let ids: Vec<&str> = folded.iter().map(|h| h.doorway_id.as_str()).collect();
        assert_eq!(ids, vec!["c-doorway", "b-doorway"]);
    }

    // ── Weight term in the fold (T2) ──────────────────────────────────────────

    /// T2.2 — THE BYTE-FOR-BYTE REGRESSION PIN. Same contracts + liveness + an
    /// EMPTY weights map as `fold_orders_health_first_then_owner_order` →
    /// identical ordering to what that pre-existing test asserts.
    #[test]
    fn with_no_observations_the_fold_returns_exactly_the_order_it_returns_today() {
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
            &HashMap::new(),
            "a-doorway",
        );
        let ids: Vec<&str> = folded.iter().map(|h| h.doorway_id.as_str()).collect();
        assert_eq!(ids, vec!["c-doorway", "d-doorway", "b-doorway"]);
    }

    /// T2.3 — THE STORY'S CORE ASSERTION. Two equally-live holders, alpha
    /// first in owner order; alpha carries a declared shed → beta first.
    #[test]
    fn a_holder_that_declared_a_shed_sorts_behind_an_equally_live_sibling() {
        let contracts = vec![
            contract("alpha", "https://alpha.example", "/lamad"),
            contract("beta", "https://beta.example", "/lamad"),
        ];
        let liveness = HashMap::from([
            ("alpha".to_string(), HolderLiveness::Serving),
            ("beta".to_string(), HolderLiveness::Serving),
        ]);
        let weights = HashMap::from([("alpha".to_string(), WEIGHT_SHEDDING)]);
        let folded = fold_candidate_holders(
            &RouteKey::path_only("/lamad/x"),
            &contracts,
            &liveness,
            &weights,
            "self",
        );
        let ids: Vec<&str> = folded.iter().map(|h| h.doorway_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["beta", "alpha"],
            "alpha declared a shed, so beta — equally live but unconstrained — sorts first"
        );
    }

    /// T2.4 — busy beats dead: Weight sits below Liveness in the tuple.
    #[test]
    fn weight_never_outranks_liveness() {
        let contracts = vec![
            contract("busy", "https://busy.example", "/lamad"),
            contract("dead", "https://dead.example", "/lamad"),
        ];
        let liveness = HashMap::from([
            ("busy".to_string(), HolderLiveness::Serving),
            ("dead".to_string(), HolderLiveness::Unreachable),
        ]);
        let weights = HashMap::from([("busy".to_string(), WEIGHT_SHEDDING)]);
        let folded = fold_candidate_holders(
            &RouteKey::path_only("/lamad/x"),
            &contracts,
            &liveness,
            &weights,
            "self",
        );
        let ids: Vec<&str> = folded.iter().map(|h| h.doorway_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["busy", "dead"],
            "a demoted-but-Serving holder still sorts ahead of an Unreachable one"
        );
    }

    /// T2.5 — cheap now, load-bearing when ReachStanding/Nearest land: assert
    /// by POSITION that Weight is tuple index 3, below indices 1-2.
    #[test]
    fn weight_never_outranks_reach_or_nearest_when_those_are_wired() {
        let probe = holder("x", "https://x.example", HolderLiveness::Serving);
        let rank = selector_rank(&probe, 0);
        // (liveness, reach_standing, nearest, weight, owner_order)
        assert_eq!(SELECTOR_TERMS[1], SelectorTerm::ReachStanding);
        assert_eq!(SELECTOR_TERMS[2], SelectorTerm::Nearest);
        assert_eq!(SELECTOR_TERMS[3], SelectorTerm::Weight);
        // The tuple field order mirrors SELECTOR_TERMS' index order exactly —
        // rank.1/.2 are compared before rank.3 by Rust's derived tuple Ord.
        let _ = rank;
    }

    /// T2.6 — §3.7's invariant: a demoted holder is still a candidate, not
    /// filtered out. One holder, demoted → the fold returns it (len 1).
    #[test]
    fn a_demoted_holder_is_still_a_candidate() {
        let contracts = vec![contract("alpha", "https://alpha.example", "/lamad")];
        let liveness = HashMap::from([("alpha".to_string(), HolderLiveness::Serving)]);
        let weights = HashMap::from([("alpha".to_string(), WEIGHT_SHEDDING)]);
        let folded = fold_candidate_holders(
            &RouteKey::path_only("/lamad/x"),
            &contracts,
            &liveness,
            &weights,
            "self",
        );
        assert_eq!(
            folded.len(),
            1,
            "a sole demoted holder is still dialled and can still serve"
        );
        assert_eq!(folded[0].doorway_id, "alpha");
    }

    /// T2.7 — if everyone is shedding, there is nothing to balance: falls
    /// through to today's owner order.
    #[test]
    fn every_holder_demoted_falls_through_to_owner_order() {
        let contracts = vec![
            contract("alpha", "https://alpha.example", "/lamad"),
            contract("beta", "https://beta.example", "/lamad"),
        ];
        let liveness = HashMap::from([
            ("alpha".to_string(), HolderLiveness::Serving),
            ("beta".to_string(), HolderLiveness::Serving),
        ]);
        let weights = HashMap::from([
            ("alpha".to_string(), WEIGHT_SHEDDING),
            ("beta".to_string(), WEIGHT_SHEDDING),
        ]);
        let folded = fold_candidate_holders(
            &RouteKey::path_only("/lamad/x"),
            &contracts,
            &liveness,
            &weights,
            "self",
        );
        let ids: Vec<&str> = folded.iter().map(|h| h.doorway_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["alpha", "beta"],
            "equal weights fall through to owner order"
        );
    }

    /// T2.8 — the EPR-id fold is not a second selector: it carries the same
    /// Weight term.
    #[test]
    fn fold_all_holders_carries_the_same_weight_term() {
        let contracts = vec![
            contract("alpha", "https://alpha.example", "/lamad"),
            contract("beta", "https://beta.example", "/shefa"),
        ];
        let liveness = HashMap::from([
            ("alpha".to_string(), HolderLiveness::Serving),
            ("beta".to_string(), HolderLiveness::Serving),
        ]);
        let weights = HashMap::from([("alpha".to_string(), WEIGHT_SHEDDING)]);
        let folded = fold_all_holders(&contracts, &liveness, &weights, "self");
        let ids: Vec<&str> = folded.iter().map(|h| h.doorway_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["beta", "alpha"],
            "fold_all_holders orders by the same selector_rank, Weight included"
        );
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

    #[tokio::test]
    async fn exact_hostname_never_falls_through_to_a_wildcard_holder() {
        let contracts = vec![
            contract("canonical", "https://public.example", "/"),
            host_contract(
                "candidate",
                "https://candidate.example",
                "/",
                "candidate.elohim.local",
            ),
        ];
        let liveness = HashMap::from([
            ("canonical".to_string(), HolderLiveness::Serving),
            ("candidate".to_string(), HolderLiveness::Serving),
        ]);
        let holders = fold_candidate_holders(
            &RouteKey::new(Some("candidate.elohim.local"), "/"),
            &contracts,
            &liveness,
            &HashMap::new(),
            "local",
        );
        assert_eq!(
            holders
                .iter()
                .map(|holder| holder.doorway_id.as_str())
                .collect::<Vec<_>>(),
            vec!["candidate"]
        );

        let attempted = Mutex::new(Vec::new());
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |holder| {
            attempted.lock().unwrap().push(holder.doorway_id);
            async { Ok(reply(503, "candidate unavailable")) }
        })
        .await;
        assert!(matches!(outcome.verdict, RelayVerdict::AllFailed { .. }));
        assert_eq!(attempted.into_inner().unwrap(), vec!["candidate"]);
    }

    #[test]
    fn locally_known_hostname_fails_closed_when_peers_only_advertise_wildcards() {
        let table = NameRouteTable::new();
        table.replace_all(
            vec![contract("canonical", "https://public.example", "/")],
            HashMap::new(),
        );
        let key = RouteKey::new(Some("candidate.elohim.local"), "/");
        assert_eq!(table.holders_for(&key, "local").len(), 1);
        assert!(table.holders_for_exact_host(&key, "local").is_empty());
    }

    /// T2.1 — `selector_rank_has_one_element_per_selector_term`. `Weight` is
    /// WIRED (story 3.1), so it is no longer in the `TERM_NOT_YET_WIRED`
    /// triple — but an UNOBSERVED holder (the `holder()` test fixture
    /// defaults `shed_weight` to `WEIGHT_UNCONSTRAINED`, which is
    /// byte-identical to `TERM_NOT_YET_WIRED`) still ranks `(liveness, 0, 0,
    /// 0, owner_order)`, so the regression pin below is unaffected by wiring
    /// this term.
    #[test]
    fn selector_terms_declare_liveness_first_and_owner_order_last() {
        // The ORDER is the contract. reach/standing and nearest sit between
        // liveness and weight and are declared-but-constant until wired.
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
            (rank.1, rank.2),
            (TERM_NOT_YET_WIRED, TERM_NOT_YET_WIRED),
            "ReachStanding and Nearest are unwired terms — constant, so they cannot reorder anything yet"
        );
        assert_eq!(
            rank.3, WEIGHT_UNCONSTRAINED,
            "Weight is WIRED, but an unobserved holder still ranks as unconstrained"
        );
        let mut demoted = probe.clone();
        demoted.shed_weight = WEIGHT_SHEDDING;
        assert_eq!(
            selector_rank(&demoted, 3).3,
            WEIGHT_SHEDDING,
            "a holder carrying a declared shed ranks worse on the Weight term"
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
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |h| {
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

    fn observed(doorway_id: &str, declared_secs: Option<u64>, reason: ShedReason) -> ObservedShed {
        ObservedShed {
            doorway_id: doorway_id.to_string(),
            declared_secs,
            reason: Some(reason),
            origin: format!("https://{doorway_id}.example"),
        }
    }

    /// T3.1 — THE WHOLE REASON THE TERM EXISTS SEPARATELY FROM `Liveness`
    /// (§7.5): a discovery tick (`replace_all`) does not forget a window the
    /// holder named. `liveness` is rebuilt wholesale every tick, but `shed`
    /// is not — the declared window (300s here) easily outlives the
    /// microseconds this test takes to run.
    #[test]
    fn a_discovery_tick_does_not_forget_a_window_the_holder_named() {
        let table = NameRouteTable::new();
        table.replace_all(
            vec![
                contract("alpha", "https://alpha.example", "/lamad"),
                contract("beta", "https://beta.example", "/lamad"),
            ],
            HashMap::from([
                ("alpha".to_string(), HolderLiveness::Serving),
                ("beta".to_string(), HolderLiveness::Serving),
            ]),
        );
        table.note_backpressure(&observed("alpha", Some(300), ShedReason::RetryAfter));

        // Simulate the next discovery tick: a fresh liveness snapshot, both
        // serving again (mirrors real behaviour — the probe would see alpha
        // answering fine; only the relay layer ever saw the shed).
        table.replace_all(
            vec![
                contract("alpha", "https://alpha.example", "/lamad"),
                contract("beta", "https://beta.example", "/lamad"),
            ],
            HashMap::from([
                ("alpha".to_string(), HolderLiveness::Serving),
                ("beta".to_string(), HolderLiveness::Serving),
            ]),
        );

        let ids: Vec<String> = table
            .holders_for(&RouteKey::path_only("/lamad"), "self")
            .into_iter()
            .map(|h| h.doorway_id)
            .collect();
        assert_eq!(
            ids,
            vec!["beta".to_string(), "alpha".to_string()],
            "the declared window survives the discovery tick that would have wiped a bare Liveness demotion"
        );
    }

    /// T3.2 — two arms, two concerns: `note_shed` alone leaves `shed_weight`
    /// unconstrained (it only ever touches `Liveness`).
    #[test]
    fn note_shed_still_only_sets_liveness() {
        let table = NameRouteTable::new();
        table.replace_all(
            vec![contract("alpha", "https://alpha.example", "/lamad")],
            HashMap::from([("alpha".to_string(), HolderLiveness::Serving)]),
        );
        table.note_shed("alpha");
        let holders = table.holders_for(&RouteKey::path_only("/lamad"), "self");
        assert_eq!(holders.len(), 1);
        assert_eq!(holders[0].liveness, HolderLiveness::Shedding);
        assert_eq!(
            holders[0].shed_weight, WEIGHT_UNCONSTRAINED,
            "note_shed must never touch the Weight term"
        );
    }

    /// NIT — pins `note_backpressure`'s early return: an observation with
    /// `reason: None` (a bare 503, or a caller-scoped 503/429) is a pure
    /// no-op for `Weight` — no window opens, no counter fires, no log line.
    #[test]
    fn a_shed_with_no_reason_is_not_recorded() {
        let table = NameRouteTable::new();
        table.replace_all(
            vec![contract("alpha", "https://alpha.example", "/lamad")],
            HashMap::from([("alpha".to_string(), HolderLiveness::Serving)]),
        );
        table.note_backpressure(&ObservedShed {
            doorway_id: "alpha".to_string(),
            declared_secs: None,
            reason: None,
            origin: "https://alpha.example".to_string(),
        });
        let holders = table.holders_for(&RouteKey::path_only("/lamad"), "self");
        assert_eq!(
            holders[0].shed_weight, WEIGHT_UNCONSTRAINED,
            "reason: None must never open a window"
        );
    }

    /// T3.3 — exact-commitment resolution is an authority question, not a
    /// balancing one; it is never weighted even when the same holder has a
    /// live declared shed.
    #[test]
    fn exact_commitment_resolution_is_never_weighted() {
        let table = NameRouteTable::new();
        let exact = HolderContract {
            host: Some("garden.example".to_string()),
            commitment_id: Some("project-epr-garden".to_string()),
            epr_id: Some("garden-epr".to_string()),
            ..HolderContract::any_host("alpha", "https://alpha.example", "/")
        };
        table.replace_all(
            vec![exact],
            HashMap::from([("alpha".to_string(), HolderLiveness::Serving)]),
        );
        table.note_backpressure(&observed("alpha", Some(300), ShedReason::RetryAfter));
        let ExactCommitmentHolder::Unique(holder) =
            table.exact_commitment_holder(Some("garden.example"), "project-epr-garden")
        else {
            panic!("expected a unique exact-commitment holder");
        };
        assert_eq!(
            holder.shed_weight, WEIGHT_UNCONSTRAINED,
            "exact-commitment resolution must never be weighted"
        );
    }

    /// T3.4 — promotion needs no timer: the table prunes on read. Directly
    /// seeding an ALREADY-ELAPSED note (rather than sleeping past a real
    /// window) proves the fold sees the elapsed state with no task having
    /// run — same white-box access T1's ShedMemory tests use, one layer up.
    #[test]
    fn the_table_prunes_on_read_so_a_promotion_needs_no_timer() {
        let table = NameRouteTable::new();
        table.replace_all(
            vec![contract("alpha", "https://alpha.example", "/lamad")],
            HashMap::from([("alpha".to_string(), HolderLiveness::Serving)]),
        );
        {
            let mut shed = table.shed.write().unwrap();
            shed.notes.insert(
                "alpha".to_string(),
                ShedNote {
                    until_secs: 1, // 1970 — already elapsed relative to any real now_secs()
                    reason: ShedReason::RetryAfter,
                },
            );
        }
        let holders = table.holders_for(&RouteKey::path_only("/lamad"), "self");
        assert_eq!(
            holders[0].shed_weight, WEIGHT_UNCONSTRAINED,
            "a fold reads the current state, not a cached one — promotion needed no timer to run"
        );
    }

    /// S3 — the hot path (nothing elapsed) must never take the `shed` write
    /// lock. `note_backpressure`'s own tail call to `prune_shed_and_log` is
    /// excluded from the measured window (it legitimately writes, once, to
    /// insert the note); only the SUBSEQUENT resolve is asserted to be
    /// write-free.
    #[test]
    fn a_resolve_with_nothing_elapsed_never_takes_the_write_lock() {
        let table = NameRouteTable::new();
        table.replace_all(
            vec![contract("alpha", "https://alpha.example", "/lamad")],
            HashMap::from([("alpha".to_string(), HolderLiveness::Serving)]),
        );
        table.note_backpressure(&observed("alpha", Some(300), ShedReason::RetryAfter));

        let passes_before = table
            .prune_write_passes
            .load(std::sync::atomic::Ordering::SeqCst);
        // The 300s window is nowhere near elapsed at real wall-clock speed.
        let _ = table.holders_for(&RouteKey::path_only("/lamad"), "self");
        let passes_after = table
            .prune_write_passes
            .load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            passes_before, passes_after,
            "a resolve with nothing elapsed must read-only — no write-lock prune pass"
        );
    }

    /// T6.2 — 0 -> 1 -> 0 without a timer. Real wiring, not a bare setter:
    /// `note_backpressure` demotes (gauge 0->1), then a fold that finds an
    /// already-elapsed window (same white-box seeding T3.4 uses) prunes it
    /// and the gauge falls back to 0 — this IS how the household run sees a
    /// promotion at all, since promotion is the passage of time and has no
    /// event of its own.
    #[test]
    fn the_gauge_tracks_live_windows_across_a_demotion_and_a_promotion() {
        // S2: reads the TABLE's own live count, never the process-wide
        // Prometheus gauge — that gauge is a thin mirror
        // (`prune_shed_and_log` calls `set_holders_demoted(live_len)` on
        // every write and fold) that many OTHER tests' NameRouteTable
        // instances mutate concurrently, so an absolute-value read on it is
        // not deterministic under `cargo test`'s default parallelism (see the
        // shared-REGISTRY convention note in `metrics.rs`).
        // `both_name_route_metrics_are_registered` (metrics.rs) separately
        // pins that the gauge series itself exists and is set.
        let table = NameRouteTable::new();
        table.replace_all(
            vec![contract("alpha", "https://alpha.example", "/lamad")],
            HashMap::from([("alpha".to_string(), HolderLiveness::Serving)]),
        );

        table.note_backpressure(&observed("alpha", Some(300), ShedReason::RetryAfter));
        assert_eq!(
            table.shed.read().unwrap().live_len(now_secs()),
            1,
            "the demotion must be visible in the table's own live count"
        );

        {
            let mut shed = table.shed.write().unwrap();
            shed.notes.insert(
                "alpha".to_string(),
                ShedNote {
                    until_secs: 1,
                    reason: ShedReason::RetryAfter,
                },
            );
        }
        let _ = table.holders_for(&RouteKey::path_only("/lamad"), "self");
        assert_eq!(
            table.shed.read().unwrap().live_len(now_secs()),
            0,
            "the fold's prune must fall the live count back to 0 on promotion — no timer required"
        );
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
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |h| {
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

    // ── Weight observation plumbing (T5) ─────────────────────────────────────

    fn reply_with_retry_after(status: u16, secs: u64) -> HolderReply {
        HolderReply {
            retry_after_secs: Some(secs),
            retry_after_present: true,
            ..reply(status, r#"{"status":"catching-up"}"#)
        }
    }

    /// A 503 whose `Retry-After` header was present but did not parse (an
    /// HTTP-date, say) — genuinely declared, no readable window.
    fn reply_with_unparseable_retry_after(status: u16) -> HolderReply {
        HolderReply {
            retry_after_secs: None,
            retry_after_present: true,
            ..reply(status, r#"{"status":"catching-up"}"#)
        }
    }

    /// A 503 carrying THIS doorway's own membrane-challenge marker — a
    /// caller-scoped answer, never a capacity statement about the holder.
    fn membrane_shed_reply() -> HolderReply {
        HolderReply {
            retry_after_secs: Some(900),
            retry_after_present: true,
            has_membrane_verdict: true,
            ..reply(503, r#"{"error":"Too Many Requests","retryAfter":900}"#)
        }
    }

    /// T5.1 — the observation carries the window the holder named.
    #[tokio::test]
    async fn a_503_with_retry_after_is_observed_with_the_window_the_holder_named() {
        let holders = vec![holder(
            "b-doorway",
            "https://b.example",
            HolderLiveness::Serving,
        )];
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |_| async {
            Ok(reply_with_retry_after(503, 45))
        })
        .await;
        assert_eq!(outcome.shed_doorways.len(), 1);
        let observed = &outcome.shed_doorways[0];
        assert_eq!(observed.doorway_id, "b-doorway");
        assert_eq!(observed.declared_secs, Some(45));
        assert_eq!(observed.reason, Some(ShedReason::RetryAfter));
    }

    /// T5.2 — the `declares_backpressure` lesson: a bare 503 with neither
    /// header is shed (Liveness still demotes it) but declares no window
    /// (Weight must not mint one from it).
    #[tokio::test]
    async fn a_503_with_no_backpressure_headers_is_shed_but_declares_no_window() {
        let holders = vec![holder(
            "b-doorway",
            "https://b.example",
            HolderLiveness::Serving,
        )];
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |_| async {
            Ok(reply(503, r#"{"error":"conductor error"}"#))
        })
        .await;
        assert_eq!(
            outcome.shed_doorways.len(),
            1,
            "still shed — Liveness must still demote it"
        );
        let observed = &outcome.shed_doorways[0];
        assert_eq!(observed.declared_secs, None);
        assert_ne!(
            observed.reason,
            Some(ShedReason::RetryAfter),
            "no header means no declared window"
        );
        assert_eq!(
            observed.reason, None,
            "and specifically: no declaration at all"
        );
    }

    /// T5.3 (REVISED, BLOCKER B1 — reverses the 429 widening entirely) — the
    /// only 429 this service emits is the membrane challenge
    /// (`server::http`'s `Verdict::Challenge`), keyed per human/client and
    /// forwarded the CALLER's own authorization/cookie by `fetch_from_holder`.
    /// So a 429 is a statement about WHO ASKED, never about the holder's own
    /// capacity: reading it as a shed would let one challenged human at
    /// holder A get holder A demoted for every OTHER visitor at courier B — a
    /// free, repeatable traffic-steering lever. `relay_one_hop` therefore
    /// handles a 429 EXACTLY as it did before story 3.1: never observed,
    /// never demoted, iteration unchanged.
    #[tokio::test]
    async fn a_429_is_about_the_caller_and_never_demotes_the_holder() {
        let holders = vec![
            holder("b-doorway", "https://b.example", HolderLiveness::Serving),
            holder("c-doorway", "https://c.example", HolderLiveness::Serving),
        ];
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |h| async move {
            if h.doorway_id == "b-doorway" {
                Ok(reply(
                    429,
                    r#"{"error":"Too Many Requests","retryAfter":900}"#,
                ))
            } else {
                Ok(reply(200, "served by c"))
            }
        })
        .await;
        match &outcome.verdict {
            RelayVerdict::Served { origin, .. } => assert_eq!(origin, "https://c.example"),
            other => panic!("expected the second holder to serve, got {other:?}"),
        }
        assert!(
            outcome.shed_doorways.is_empty(),
            "a 429 must never enter shed_doorways — no Liveness demotion, no Weight window, ever"
        );
    }

    /// B1's defensive symmetry: a 503 that ALSO carries `x-membrane` is still
    /// caller-scoped (the same defect wearing a different status code), so it
    /// opens no Weight window either — even though it still counts as a shed
    /// for `Liveness` (the pre-existing bare-503 behaviour is untouched).
    #[tokio::test]
    async fn a_503_bearing_a_membrane_verdict_opens_no_window() {
        let holders = vec![holder(
            "b-doorway",
            "https://b.example",
            HolderLiveness::Serving,
        )];
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |_| async {
            Ok(membrane_shed_reply())
        })
        .await;
        assert_eq!(
            outcome.shed_doorways.len(),
            1,
            "still a shed — Liveness demotion is unchanged"
        );
        let observed = &outcome.shed_doorways[0];
        assert_eq!(
            observed.reason, None,
            "a caller-scoped 503 must never open a Weight window, whatever Retry-After it carries"
        );
    }

    /// S1 companion, at the relay level: a genuinely declared shed (header
    /// present) whose value could not be parsed still opens the DEFAULT
    /// window, labelled `NoWindowNamed` — never silently dropped to "no
    /// declaration at all".
    #[tokio::test]
    async fn a_503_with_an_unparseable_retry_after_is_declared_with_no_window_named() {
        let holders = vec![holder(
            "b-doorway",
            "https://b.example",
            HolderLiveness::Serving,
        )];
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |_| async {
            Ok(reply_with_unparseable_retry_after(503))
        })
        .await;
        let observed = &outcome.shed_doorways[0];
        assert_eq!(observed.declared_secs, None);
        assert_eq!(observed.reason, Some(ShedReason::NoWindowNamed));
    }

    /// T5.4 — a 403 is still an authoritative refusal and opens no window: it
    /// stops the loop and never reaches `shed_doorways` at all.
    #[tokio::test]
    async fn a_403_is_still_an_authoritative_refusal_and_opens_no_window() {
        let holders = vec![holder(
            "b-doorway",
            "https://b.example",
            HolderLiveness::Serving,
        )];
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |_| async {
            Ok(refusal("household"))
        })
        .await;
        assert!(matches!(outcome.verdict, RelayVerdict::Served { .. }));
        assert!(
            outcome.shed_doorways.is_empty(),
            "a refusal is not a shed and opens no weight window"
        );
    }

    /// T5.5 — structural: `relay_one_hop` takes only `&[NameHolder]` and a
    /// stateless `fetch` closure and RETURNS its observations; no
    /// `NameRouteTable` (or `AppState`) is threaded through it, so the write
    /// side (`note_shed` / `note_backpressure`) is entirely the caller's —
    /// exactly `RelayOutcome`'s own documented discipline.
    #[tokio::test]
    async fn the_relay_loop_still_owns_no_mutable_state() {
        let holders = vec![holder(
            "b-doorway",
            "https://b.example",
            HolderLiveness::Serving,
        )];
        // No table, no AppState, no shared cell in scope at all — only the
        // holder slice and a closure — and the loop still produces a full
        // verdict plus every shed it saw for the caller to act on.
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |_| async {
            Ok(reply(200, "<app-root></app-root>"))
        })
        .await;
        assert!(matches!(outcome.verdict, RelayVerdict::Served { .. }));
        assert!(outcome.shed_doorways.is_empty());
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
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |_| async {
            Ok(reply(404, "not here"))
        })
        .await;
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
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |h| async move {
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
            outcome
                .shed_doorways
                .iter()
                .map(|o| o.doorway_id.as_str())
                .collect::<Vec<_>>(),
            vec!["b-doorway"],
            "the shed is recorded so the next fold demotes that holder"
        );
    }

    #[tokio::test]
    async fn every_holder_shedding_still_preserves_the_local_shed() {
        let holders = vec![
            holder("b-doorway", "https://b.example", HolderLiveness::Serving),
            holder("c-doorway", "https://c.example", HolderLiveness::Serving),
        ];
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |_| async {
            Ok(reply(503, "catching up"))
        })
        .await;
        assert_eq!(outcome.verdict, RelayVerdict::AllFailed { attempted: 2 });
        assert_eq!(outcome.shed_doorways.len(), 2);
    }

    #[tokio::test]
    async fn no_candidates_is_distinct_from_a_failed_attempt() {
        let outcome = relay_one_hop(&[], RelayChannel::Public, |_| async {
            Ok(reply(200, "never"))
        })
        .await;
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
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |h| {
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

    /// The candidate channel's named absence, shaped exactly as the local
    /// candidate handler answers it.
    fn named_absence() -> HolderReply {
        HolderReply {
            content_type: Some("application/json".to_string()),
            ..reply(NAMED_ABSENCE, NO_CANDIDATE_STAGED_BODY)
        }
    }

    /// Whether one holder's 404 with `body` counts as an answer on `channel`.
    async fn relayed_404_is_an_answer(channel: RelayChannel, body: String) -> bool {
        let holders = vec![holder(
            "b-doorway",
            "https://b.example",
            HolderLiveness::Serving,
        )];
        let outcome = relay_one_hop(&holders, channel, |_| {
            let body = body.clone();
            async move { Ok(reply(NAMED_ABSENCE, &body)) }
        })
        .await;
        matches!(outcome.verdict, RelayVerdict::Served { .. })
    }

    /// THE CANDIDATE FIX. A holder's `404 {"error":"no-candidate-staged"}` is an
    /// ANSWER about the RECORD, from the doorway that holds this name's
    /// contract — the same rationale as a 403. Without it a candidate this
    /// doorway can SHOW by relaying the holder's 200 can never be WITHDRAWN by
    /// relaying that same holder's absence: the channel is one-way.
    #[tokio::test]
    async fn a_holders_named_absence_withdraws_a_relayed_candidate() {
        let holders = vec![
            holder("b-doorway", "https://b.example", HolderLiveness::Serving),
            holder("c-doorway", "https://c.example", HolderLiveness::Serving),
        ];
        let dialled = Mutex::new(Vec::new());
        let outcome = relay_one_hop(&holders, RelayChannel::Candidate, |h| {
            dialled.lock().unwrap().push(h.doorway_id.clone());
            async move { Ok(named_absence()) }
        })
        .await;

        let RelayVerdict::Served {
            origin,
            doorway_id,
            reply,
        } = outcome.verdict
        else {
            panic!("a named absence is an authoritative outcome, not a failed attempt");
        };
        assert_eq!(reply.status, 404);
        assert_eq!(
            reply.body,
            NO_CANDIDATE_STAGED_BODY.as_bytes(),
            "the withdrawal is relayed body-and-all, or the client cannot read it"
        );
        assert_eq!(
            *dialled.lock().unwrap(),
            vec!["b-doorway".to_string()],
            "the holder that owns the fold has answered -- no second holder is asked"
        );
        assert!(
            outcome.shed_doorways.is_empty(),
            "a named absence says nothing about the holder's liveness"
        );

        let response = build_relayed_response(&origin, &doorway_id, reply);
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(SERVED_BY_HEADER).unwrap(),
            "https://b.example",
            "and it still names who decided it"
        );
    }

    /// The public channel is untouched. The identical body from the identical
    /// holder is still a failed attempt there, so the caller keeps ITS status —
    /// and a name this doorway resolved no candidate channel for can never be
    /// told apart from any other name by this class.
    #[tokio::test]
    async fn a_named_absence_on_the_public_channel_changes_nothing() {
        assert!(
            !relayed_404_is_an_answer(RelayChannel::Public, NO_CANDIDATE_STAGED_BODY.to_string())
                .await,
            "only a locally-resolved candidate channel widens the answer class"
        );
    }

    /// Exact match on the structured body, or today's behaviour. A route miss,
    /// a different error, a body that merely mentions the token, HTML, and an
    /// unparseable body are all still failed attempts.
    #[tokio::test]
    async fn only_the_exact_named_absence_body_is_an_answer() {
        assert!(
            relayed_404_is_an_answer(
                RelayChannel::Candidate,
                NO_CANDIDATE_STAGED_BODY.to_string()
            )
            .await
        );
        for other in [
            r#"{"error":"candidate-head-unobserved"}"#,
            r#"{"detail":"no-candidate-staged"}"#,
            r#"{"error":"no-candidate-staged-elsewhere"}"#,
            r#"{"error":null}"#,
            r#"{}"#,
            "<html>404 not found</html>",
            "",
        ] {
            assert!(
                !relayed_404_is_an_answer(RelayChannel::Candidate, other.to_string()).await,
                "a 404 body of {other:?} is not the candidate channel's named absence"
            );
        }
    }

    /// A structured absence is tens of bytes. A larger body is by construction
    /// something else, and is refused WITHOUT being parsed — so a holder cannot
    /// spend this doorway's memory on the question.
    #[tokio::test]
    async fn an_oversized_body_is_never_read_into_an_answer() {
        let padded = format!(
            r#"{{"error":"{NO_CANDIDATE_STAGED_ERROR}","pad":"{}"}}"#,
            "p".repeat(NAMED_ABSENCE_MAX_BODY)
        );
        assert!(padded.len() > NAMED_ABSENCE_MAX_BODY);
        assert!(
            !relayed_404_is_an_answer(RelayChannel::Candidate, padded).await,
            "the body cap is checked before the parse, so an unbounded body is never buffered"
        );
    }

    /// The token the relay matches and the body the candidate handler answers
    /// with are one statement; this fixes them together so neither can drift.
    #[test]
    fn the_named_absence_body_carries_its_token() {
        let parsed: serde_json::Value =
            serde_json::from_str(NO_CANDIDATE_STAGED_BODY).expect("the named absence body is JSON");
        assert_eq!(
            parsed.get("error").and_then(serde_json::Value::as_str),
            Some(NO_CANDIDATE_STAGED_ERROR),
            "the body the candidate channel answers with carries the token the relay matches"
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
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |h| {
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
        assert_eq!(
            outcome
                .shed_doorways
                .iter()
                .map(|o| o.doorway_id.as_str())
                .collect::<Vec<_>>(),
            vec!["b-doorway"]
        );
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
        assert!(
            plain.headers().get(RECEIPT_HEADER).is_none(),
            "the relay never invents a receipt pointer the holder did not state"
        );
    }

    /// The holder's receipt pointer is the holder's statement about the
    /// exchange ITS serve was. A courier that dropped it would leave the visitor
    /// unable to ask what was traded; a courier that minted its own would be
    /// accounting for work it did not do. So: verbatim, or absent.
    #[test]
    fn the_holders_receipt_pointer_is_relayed_verbatim() {
        let carried = HolderReply {
            receipt: Some("/api/v1/receipt/community-garden-club".to_string()),
            ..reply(200, "<app-root></app-root>")
        };
        let response = build_relayed_response("https://b.example", "b-doorway", carried);
        assert_eq!(
            response.headers().get(RECEIPT_HEADER).unwrap(),
            "/api/v1/receipt/community-garden-club",
            "the pointer is relative, so it resolves against the courier — which is \
             exactly where the merge that substitutes OUR projection credit happens"
        );
    }

    #[tokio::test]
    async fn transport_error_falls_through_to_the_next_holder() {
        let holders = vec![
            holder("b-doorway", "https://b.example", HolderLiveness::Serving),
            holder("c-doorway", "https://c.example", HolderLiveness::Serving),
        ];
        let outcome = relay_one_hop(&holders, RelayChannel::Public, |h| async move {
            if h.doorway_id == "b-doorway" {
                Err("connection refused".to_string())
            } else {
                Ok(reply(200, "served by c"))
            }
        })
        .await;
        assert!(matches!(outcome.verdict, RelayVerdict::Served { .. }));
    }

    // ── Story 4.2 slice 1: replace_holder / held_digest / replace_all_at ──────

    #[test]
    fn replace_holder_installs_rows_liveness_and_digest_for_that_holder_only() {
        let table = NameRouteTable::new();
        // alpha mounts at /lamad (NOT the universal root "/"), so it never
        // covers /garden — keeps this test's holder count unambiguous.
        table.replace_all(
            vec![contract(
                "alpha-elohim-host",
                "https://alpha.example",
                "/lamad",
            )],
            HashMap::from([("alpha-elohim-host".to_string(), HolderLiveness::Serving)]),
        );
        table.replace_holder(
            "gamma-elohim-host",
            vec![contract(
                "gamma-elohim-host",
                "https://gamma.example",
                "/garden",
            )],
            HolderLiveness::Serving,
            "bafy-garden".to_string(),
            1_000,
        );
        assert_eq!(table.len(), 2, "alpha's row survives, gamma's is added");
        assert_eq!(
            table.held_digest("gamma-elohim-host").as_deref(),
            Some("bafy-garden")
        );
        assert_eq!(table.held_digest("alpha-elohim-host"), None);
        let holders = table.holders_for(&RouteKey::path_only("/garden"), "self");
        assert_eq!(holders.len(), 1);
        assert_eq!(holders[0].doorway_id, "gamma-elohim-host");
    }

    #[test]
    fn replace_holder_replaces_only_that_holders_prior_rows() {
        let table = NameRouteTable::new();
        table.replace_holder(
            "gamma-elohim-host",
            vec![contract(
                "gamma-elohim-host",
                "https://gamma.example",
                "/old",
            )],
            HolderLiveness::Serving,
            "bafy-old".to_string(),
            1_000,
        );
        table.replace_holder(
            "gamma-elohim-host",
            vec![contract(
                "gamma-elohim-host",
                "https://gamma.example",
                "/new",
            )],
            HolderLiveness::Serving,
            "bafy-new".to_string(),
            1_001,
        );
        assert_eq!(
            table.len(),
            1,
            "the old row must not survive alongside the new one"
        );
        let holders = table.holders_for(&RouteKey::path_only("/new"), "self");
        assert_eq!(holders.len(), 1);
        assert!(table
            .holders_for(&RouteKey::path_only("/old"), "self")
            .is_empty());
        assert_eq!(
            table.held_digest("gamma-elohim-host").as_deref(),
            Some("bafy-new")
        );
    }

    /// THE C2 MONOTONIC GUARD, PINNED: a poll round that STARTED before a
    /// doorbell installed a fresher snapshot for a holder must not clobber it
    /// — the doorbell-installed rows survive `replace_all_at`.
    #[test]
    fn a_slower_poll_round_cannot_overwrite_a_newer_doorbell_install() {
        let table = NameRouteTable::new();
        // The poll round STARTS at 1_000 (fetch_started), but by the time it
        // finishes and calls replace_all_at, a doorbell has already installed
        // gamma's fresher snapshot at 1_005.
        table.replace_holder(
            "gamma-elohim-host",
            vec![contract(
                "gamma-elohim-host",
                "https://gamma.example",
                "/garden",
            )],
            HolderLiveness::Serving,
            "bafy-fresh".to_string(),
            1_005,
        );
        table.replace_all_at(
            vec![contract(
                "gamma-elohim-host",
                "https://gamma.example",
                "/stale-batch",
            )],
            HashMap::from([("gamma-elohim-host".to_string(), HolderLiveness::Uncertain)]),
            HashMap::from([("gamma-elohim-host".to_string(), "bafy-stale".to_string())]),
            1_000, // fetch_started — BEFORE the doorbell install at 1_005
        );
        assert_eq!(
            table.held_digest("gamma-elohim-host").as_deref(),
            Some("bafy-fresh"),
            "the poll's stale digest must not overwrite the doorbell-fresh one"
        );
        assert!(
            table
                .holders_for(&RouteKey::path_only("/garden"), "self")
                .iter()
                .any(|h| h.doorway_id == "gamma-elohim-host"),
            "the doorbell-installed row must survive the slower poll round"
        );
        assert!(
            table
                .holders_for(&RouteKey::path_only("/stale-batch"), "self")
                .is_empty(),
            "the poll's stale row for the protected holder must never be installed"
        );
    }

    /// A poll round that STARTS AFTER the doorbell install is free to replace
    /// it — protection is time-scoped, not permanent.
    #[test]
    fn a_poll_round_started_after_the_doorbell_install_may_replace_it() {
        let table = NameRouteTable::new();
        table.replace_holder(
            "gamma-elohim-host",
            vec![contract(
                "gamma-elohim-host",
                "https://gamma.example",
                "/garden",
            )],
            HolderLiveness::Serving,
            "bafy-fresh".to_string(),
            1_000,
        );
        table.replace_all_at(
            vec![contract(
                "gamma-elohim-host",
                "https://gamma.example",
                "/garden-v2",
            )],
            HashMap::from([("gamma-elohim-host".to_string(), HolderLiveness::Serving)]),
            HashMap::from([("gamma-elohim-host".to_string(), "bafy-newer".to_string())]),
            1_005, // fetch_started — AFTER the doorbell install at 1_000
        );
        assert_eq!(
            table.held_digest("gamma-elohim-host").as_deref(),
            Some("bafy-newer")
        );
        assert!(table
            .holders_for(&RouteKey::path_only("/garden-v2"), "self")
            .iter()
            .any(|h| h.doorway_id == "gamma-elohim-host"));
    }

    /// Same-second collision: both clocks are whole seconds, so a doorbell
    /// install stamped in the SAME second a poll round started is protected
    /// — the guard errs toward the doorbell-fresh row, never the poll's.
    #[test]
    fn a_poll_round_started_in_the_same_second_as_the_doorbell_install_cannot_replace_it() {
        let table = NameRouteTable::new();
        table.replace_holder(
            "gamma-elohim-host",
            vec![contract(
                "gamma-elohim-host",
                "https://gamma.example",
                "/garden",
            )],
            HolderLiveness::Serving,
            "bafy-fresh".to_string(),
            1_000,
        );
        table.replace_all_at(
            vec![contract(
                "gamma-elohim-host",
                "https://gamma.example",
                "/stale-batch",
            )],
            HashMap::from([("gamma-elohim-host".to_string(), HolderLiveness::Uncertain)]),
            HashMap::from([("gamma-elohim-host".to_string(), "bafy-stale".to_string())]),
            1_000, // fetch_started — the SAME second as the doorbell install
        );
        assert_eq!(
            table.held_digest("gamma-elohim-host").as_deref(),
            Some("bafy-fresh"),
            "a same-second poll must not overwrite the doorbell-fresh digest"
        );
        assert!(table
            .holders_for(&RouteKey::path_only("/stale-batch"), "self")
            .is_empty());
    }

    #[test]
    fn held_digest_is_none_for_a_holder_never_installed() {
        let table = NameRouteTable::new();
        assert_eq!(table.held_digest("never-seen"), None);
    }
}
