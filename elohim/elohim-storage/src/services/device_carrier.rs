//! The first carrier: a device on the private network asks, an approving node lists
//! it, someone decides, and the code goes back the same way, with nobody
//! carrying a link.
//!
//! What carries it. The asking node offers its ask to every peer its local
//! discovery found on the private network, and to no one else: in libp2p and
//! dual modes that is libp2p mDNS (`DeliveryPeer::network == "lan"`), never
//! Kademlia, relay, gossip or a bootstrap peer. The messages ride the existing
//! `/elohim/epr/1.0.0` request/response protocol as one `DeviceCarry` variant
//! each way. In iroh-only mode the node has no local discovery (its only
//! discovery is pkarr, which is not local), so the carrier is absent there and
//! says so; the link-and-paste path always remains.
//!
//! What it is not:
//! - Not authority. An ask that arrives is evidence that a node asked. A
//!   decision is made through `consent_grant::decide`, and agreeing still runs
//!   the agree path on this node's own machine, under its loopback rule.
//! - Not stored. Pending asks, the device's announce and codes in transit are
//!   memory only; nothing is notarized, linked or written to a table. A node
//!   that restarts is asked again.
//! - Not reachable from the network beyond the one protocol message. The list,
//!   the decision and the device's announce are this-machine-only routes.
//!
//! Who lists. A node lists an ask only when it speaks for a person: its own
//! cell is a controller of an identity's authority. A joined device that is
//! not a controller, or a node with no identity, drops every ask with its
//! reason and answers the asking device nothing at all, so it never appears to
//! the asking terminal as a node that lists the ask. Whom the node speaks for is
//! read from its own cell into memory (`Carrier::speaks`) by one refresher,
//! every `SPEAKS_REFRESH_SECS` and at once when this node's identity may have
//! changed (`identity_changed`), never per incoming ask. A read older than
//! `SPEAKS_MAX_AGE_MICROS` counts as unknown, and unknown lists nothing.
//!
//! Identity namespaces stay apart: the carrier sees a transport id, kept only
//! to answer the peer that asked; the request names the device's agent key,
//! which is what a person is shown. The two are never compared.

use std::sync::{Arc, Mutex, OnceLock};

use consent_grant::{
    redeem, Ask, CarryRequest, CarryResponse, Dropped, GrantRequest, Listed, MemoryStore,
    NodeState, PendingAsks, Speaks, ASK_LIFE_MICROS,
};

/// How often the refresher reads whom this node speaks for.
pub const SPEAKS_REFRESH_SECS: u64 = 30;
/// How old that read may be before it counts as unknown.
pub const SPEAKS_MAX_AGE_MICROS: i64 = 120 * 1_000_000;
use tracing::{info, warn};

/// Everything the carrier holds, for this process.
pub struct Carrier {
    /// Asks waiting on this node as an approving node.
    pub pending: PendingAsks,
    /// Codes this node issued, shared with `/auth/consent/redeem`.
    pub deliveries: Arc<MemoryStore>,
    /// This node's controller key, once its cell is reachable.
    self_key: OnceLock<String>,
    /// This node's own announce, when it is the asking device.
    announce: Mutex<Option<Announce>>,
    /// Whom this node speaks for, as last read from its own cell, and when.
    speaks: Mutex<Option<(Speaks, i64)>>,
    /// Woken when this node's identity may have changed.
    speaks_changed: tokio::sync::Notify,
}

/// The device's side of one ask.
#[derive(Debug, Clone)]
pub struct Announce {
    pub request: GrantRequest,
    pub state: NodeState,
    pub for_approver: Option<String>,
    pub started_at_micros: i64,
    /// Approving nodes that listed it: (transport id, approver key, number there).
    pub listed_by: Vec<(String, Option<String>, u32)>,
    /// The code an approving node that listed it handed back, and from where.
    pub code: Option<(String, String)>,
}

impl Announce {
    pub fn ask(&self) -> Ask {
        Ask {
            request: self.request.clone(),
            state: self.state,
            for_approver: self.for_approver.clone(),
        }
    }

    pub fn expired(&self, now_micros: i64) -> bool {
        now_micros >= self.started_at_micros.saturating_add(ASK_LIFE_MICROS)
    }
}

pub fn carrier() -> &'static Carrier {
    static CARRIER: OnceLock<Carrier> = OnceLock::new();
    CARRIER.get_or_init(|| Carrier {
        pending: PendingAsks::new(),
        deliveries: Arc::new(MemoryStore::new()),
        self_key: OnceLock::new(),
        announce: Mutex::new(None),
        speaks: Mutex::new(None),
        speaks_changed: tokio::sync::Notify::new(),
    })
}

fn now_micros() -> i64 {
    chrono::Utc::now().timestamp_micros()
}

impl Carrier {
    pub fn set_self_key(&self, key: String) {
        let _ = self.self_key.set(key);
    }

    pub fn self_key(&self) -> Option<&str> {
        self.self_key.get().map(String::as_str)
    }

    /// Whom this node speaks for: the last read, or unknown when there is none
    /// or it is older than `SPEAKS_MAX_AGE_MICROS`.
    pub fn speaks(&self, now_micros: i64) -> Speaks {
        match &*self.speaks.lock().unwrap_or_else(|e| e.into_inner()) {
            Some((speaks, at)) if now_micros < at.saturating_add(SPEAKS_MAX_AGE_MICROS) => {
                speaks.clone()
            }
            _ => Speaks::Unknown,
        }
    }

    /// Record a fresh read of whom this node speaks for.
    pub fn set_speaks(&self, speaks: Speaks, now_micros: i64) {
        let mut slot = self.speaks.lock().unwrap_or_else(|e| e.into_inner());
        if slot.as_ref().map(|(s, _)| s) != Some(&speaks) {
            info!(speaks = %speaks.words(), "device carrier: whom this node speaks for");
        }
        *slot = Some((speaks, now_micros));
    }

    /// This node's identity may have changed (begun, bootstrapped, enrolled,
    /// declared): read whom it speaks for again now.
    pub fn identity_changed(&self) {
        self.speaks_changed.notify_one();
    }

    fn announce_lock(&self) -> std::sync::MutexGuard<'_, Option<Announce>> {
        self.announce.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Begin announcing `request`, replacing any earlier announce.
    pub fn start_announce(&self, announce: Announce) {
        info!(
            device = %consent_grant::hash_shape::fingerprint(&announce.request.device_key),
            state = ?announce.state,
            "device carrier: announcing an ask on the private network"
        );
        *self.announce_lock() = Some(announce);
    }

    pub fn stop_announce(&self) {
        *self.announce_lock() = None;
    }

    /// The current announce, dropped once its five minutes are up.
    pub fn announce(&self) -> Option<Announce> {
        let mut slot = self.announce_lock();
        if slot.as_ref().is_some_and(|a| a.expired(now_micros())) {
            info!("device carrier: the announce ended unanswered after five minutes");
            *slot = None;
        }
        slot.clone()
    }

    /// Record that the approving node at `source` listed this node's ask.
    pub fn note_listed(&self, source: &str, approver: Option<String>, number: u32) {
        if let Some(a) = self.announce_lock().as_mut() {
            a.listed_by.retain(|(s, _, _)| s != source);
            a.listed_by.push((source.to_string(), approver, number));
        }
    }

    /// Answer one carrier request from the peer `source`. `None` is silence:
    /// the asking device is told nothing.
    pub fn on_request(&self, source: &str, request: CarryRequest) -> Option<CarryResponse> {
        Some(match request {
            CarryRequest::Ask { ask } => return self.on_ask(source, ask),
            CarryRequest::Code { state, code, .. } => self.on_code(source, &state, code),
            CarryRequest::Redeem { redemption } => {
                // bounded-work: one sweep over deliveries that each live minutes.
                self.deliveries.sweep(now_micros());
                match redeem(&self.deliveries, &redemption, now_micros()) {
                    Ok(delivered) => {
                        info!(
                            source,
                            "device carrier: a code was redeemed over the carrier"
                        );
                        CarryResponse::Delivered {
                            delivered: Box::new(delivered),
                        }
                    }
                    Err(r) => {
                        warn!(
                            source,
                            code = r.code(),
                            "device carrier: redemption refused"
                        );
                        CarryResponse::Refused {
                            code: r.code().to_string(),
                        }
                    }
                }
            }
        })
    }

    fn on_ask(&self, source: &str, ask: Ask) -> Option<CarryResponse> {
        let device = consent_grant::hash_shape::fingerprint(&ask.request.device_key);
        let now = now_micros();
        match self.pending.admit(
            ask,
            source,
            self.self_key(),
            &self.speaks(now),
            &super::device_consent::policy(),
            now,
        ) {
            Ok(listed) => {
                let (number, how) = match listed {
                    Listed::New(n) => (n, "listed"),
                    Listed::Replaced(n) => (n, "replaced its earlier ask"),
                };
                info!(
                    source,
                    device, number, "device carrier: ask received and {how}"
                );
                Some(CarryResponse::Listed {
                    number,
                    approver: self.self_key().map(str::to_string),
                })
            }
            Err(dropped) => {
                let code = dropped.code();
                match dropped {
                    // Not an approving node for anyone: dropped, and the
                    // asking device is told nothing.
                    Dropped::SpeaksForNobody | Dropped::StandingUnknown => {
                        info!(
                            source,
                            device, code, "device carrier: ask received and dropped unanswered"
                        );
                        return None;
                    }
                    Dropped::Malformed(_)
                    | Dropped::NotForThisApprover
                    | Dropped::AlreadyDecided => {
                        info!(
                            source,
                            device, code, "device carrier: ask received and dropped"
                        )
                    }
                    Dropped::Full | Dropped::SourceOverLimit => {
                        warn!(
                            source,
                            device, code, "device carrier: ask dropped, list bounded"
                        )
                    }
                }
                Some(CarryResponse::Refused {
                    code: code.to_string(),
                })
            }
        }
    }

    /// A code for this node's own ask. Taken only from an approving node that listed
    /// it, so another peer cannot preempt the real code with a false one.
    fn on_code(&self, source: &str, state: &str, code: String) -> CarryResponse {
        let mut slot = self.announce_lock();
        let refused = |code: &str| CarryResponse::Refused { code: code.into() };
        let Some(a) = slot.as_mut() else {
            return refused("carry_no_announce");
        };
        if a.request.state != state {
            return refused("carry_not_this_ask");
        }
        if !a.listed_by.iter().any(|(s, _, _)| s == source) {
            warn!(
                source,
                "device carrier: a code came from a peer that never listed the ask"
            );
            return refused("carry_from_unlisting_peer");
        }
        info!(
            source,
            "device carrier: the approving node handed the code back"
        );
        a.code = Some((code, source.to_string()));
        CarryResponse::CodeTaken
    }
}

// =============================================================================
// The routes: this machine only
// =============================================================================

use async_trait::async_trait;
use consent_grant::{
    decide, ByAnswer, ByDeclaration, Decider, Decision, Declaration, NoElohimAttending,
    PendingView, Redemption, RequestedAct, ReturnPath, ReturnTargetView, WitnessBeat,
};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use serde::Deserialize;

use super::device_consent::{agree_request, refusal, ControllerCell, IdentifierSource, Refused};
use super::response;

/// How the carrier reaches peers. The libp2p handle in production; a fake in
/// tests.
#[async_trait]
pub trait CarrierLink: Send + Sync {
    /// The peers local discovery found on the private network.
    fn peers(&self) -> Vec<String>;
    async fn send(&self, peer: &str, request: CarryRequest) -> Result<CarryResponse, String>;
}

#[async_trait]
impl CarrierLink for crate::p2p::P2PHandle {
    fn peers(&self) -> Vec<String> {
        self.private_network_peers()
    }
    async fn send(&self, peer: &str, request: CarryRequest) -> Result<CarryResponse, String> {
        self.device_carry(peer, request).await
    }
}

fn carrier_absent() -> Response<Full<Bytes>> {
    refusal(
        StatusCode::CONFLICT,
        "this node has no local discovery in its transport mode, so nothing on the private \
         network can carry an ask; use the link instead",
        "carrier_absent",
    )
}

/// `GET /auth/consent/pending`: what is asking this node, and whose
/// identity an approval here would be for. A node that speaks for nobody
/// lists nothing and says so.
pub fn pending_list(link: Option<&dyn CarrierLink>, speaks: &Speaks) -> Response<Full<Bytes>> {
    let now = now_micros();
    let asks: Vec<PendingView> = match speaks {
        Speaks::Person(_) => carrier()
            .pending
            .list(now)
            .iter()
            .map(|p| PendingView::of(p, now, speaks))
            .collect(),
        Speaks::Nobody | Speaks::Unknown => Vec::new(),
    };
    response::ok(&serde_json::json!({
        "carrier": if link.is_some() { "private-network" } else { "absent" },
        "approver": carrier().self_key(),
        "speaksFor": speaks,
        "speaksForWords": speaks.words(),
        "asks": asks,
    }))
}

/// Read whom this node speaks for from its own cell into the carrier. A cell
/// that cannot answer leaves the last read to age into unknown.
pub async fn refresh_speaks(cell: &dyn ControllerCell, names: &dyn IdentifierSource) {
    if let Some(speaks) = read_speaks(cell, names).await {
        carrier().set_speaks(speaks, now_micros());
    }
}

/// Whom this node speaks for, from its own cell: one standing read, and one
/// read of its Human only when it is a controller, which `names` names by the
/// person's sign-in word (a claim, shown only). `None` when the cell cannot
/// answer.
pub async fn read_speaks(
    cell: &dyn ControllerCell,
    names: &dyn IdentifierSource,
) -> Option<Speaks> {
    use super::device_consent::CellStanding;
    let me = cell.agent();
    Some(match cell.standing().await {
        Ok(CellStanding::Ready(standing)) => {
            let person = if standing.controllers.contains(&me) {
                cell.my_human().await.ok().flatten()
            } else {
                None
            };
            let identifier = person
                .as_ref()
                .and_then(|p| super::device_consent::identifier_of(names, p, &me));
            Speaks::of(Some(&standing), &me, person.as_ref(), identifier)
        }
        Ok(CellStanding::NoPerson | CellStanding::Unbootstrapped { .. }) => Speaks::Nobody,
        Err(why) => {
            warn!(
                ?why,
                "device carrier: whom this node speaks for could not be read"
            );
            return None;
        }
    })
}

/// Wait until the next refresh is due: `SPEAKS_REFRESH_SECS`, or sooner when
/// this node's identity may have changed.
pub async fn until_speaks_refresh() {
    tokio::select! {
        _ = tokio::time::sleep(std::time::Duration::from_secs(SPEAKS_REFRESH_SECS)) => {}
        _ = carrier().speaks_changed.notified() => {}
    }
}

/// The body of `POST /auth/consent/pending/decide`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DecideInput {
    /// The ask's number, its device key, or that key's fingerprint.
    ask: String,
    /// An answer given on this machine, taken as the person's: the acts agreed,
    /// none meaning decline. Absent, the other deciders are asked.
    #[serde(default)]
    answer: Option<AnswerInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AnswerInput {
    agreed_acts: Vec<RequestedAct>,
}

/// `POST /auth/consent/pending/decide`: decide one waiting ask, and when it
/// is agreed, agree through the agree path and hand the code back over the
/// carrier to the peer that brought the ask.
#[allow(clippy::too_many_arguments)]
pub async fn decide_pending(
    store: &MemoryStore,
    cell: Option<&dyn ControllerCell>,
    beat: &dyn WitnessBeat,
    link: Option<&dyn CarrierLink>,
    declaration: &Declaration,
    speaks: &Speaks,
    signed_in: bool,
    body: &[u8],
) -> Response<Full<Bytes>> {
    match decide_inner(
        store,
        cell,
        beat,
        link,
        declaration,
        speaks,
        signed_in,
        body,
    )
    .await
    {
        Ok(answer) => answer,
        Err(refused) => *refused,
    }
}

#[allow(clippy::too_many_arguments)]
async fn decide_inner(
    store: &MemoryStore,
    cell: Option<&dyn ControllerCell>,
    beat: &dyn WitnessBeat,
    link: Option<&dyn CarrierLink>,
    declaration: &Declaration,
    speaks: &Speaks,
    signed_in: bool,
    body: &[u8],
) -> Result<Response<Full<Bytes>>, Refused> {
    if speaks.person().is_none() {
        return Err(Box::new(refusal(
            StatusCode::CONFLICT,
            &speaks.words(),
            "node_speaks_for_nobody",
        )));
    }
    if !signed_in {
        return Err(Box::new(refusal(
            StatusCode::UNAUTHORIZED,
            "sign in on this node before deciding an ask",
            "consent_not_signed_in",
        )));
    }
    let input: DecideInput = serde_json::from_slice(body)
        .map_err(|e| Box::new(response::bad_request(&format!("Invalid JSON: {e}"))))?;
    let now = now_micros();
    let pending = carrier().pending.pick(&input.ask, now).ok_or_else(|| {
        Box::new(refusal(
            StatusCode::NOT_FOUND,
            "nothing is asking under that number or key; it may have expired",
            "pending_unknown",
        ))
    })?;
    let by_answer = input.answer.map(|a| ByAnswer(a.agreed_acts));
    let by_declaration = ByDeclaration(declaration);
    let mut chain: Vec<&dyn Decider> = vec![&NoElohimAttending];
    match &by_answer {
        // Whoever answers here is present and answering; their answer stands.
        Some(answer) => chain.push(answer),
        None => chain.push(&by_declaration),
    }
    let decision = match decide(&chain, &pending) {
        Ok(decision) => decision,
        Err(reasons) => {
            info!(
                number = pending.number,
                ?reasons,
                "device carrier: ask left for an answer"
            );
            return Ok(response::json_response(
                StatusCode::CONFLICT,
                &serde_json::json!({
                    "error": reasons.last().cloned().unwrap_or_default(),
                    "code": "pending_needs_answer",
                    "reasons": reasons,
                    "ask": PendingView::of(&pending, now, speaks),
                }),
            ));
        }
    };
    let (acts, by) = match decision {
        Decision::Decline { by } => {
            carrier().pending.decided(pending.number, now);
            info!(number = pending.number, ?by, "device carrier: ask declined");
            return Ok(response::ok(&serde_json::json!({
                "number": pending.number, "decidedBy": by, "declined": true,
            })));
        }
        Decision::Agree { acts, by } => (acts, by),
        Decision::Defer { .. } => unreachable!("decide never returns a deferral"),
    };
    if pending.ask.request.return_path != ReturnPath::Paste {
        return Err(Box::new(refusal(
            StatusCode::BAD_REQUEST,
            "an ask carried here must take its code back the same way",
            "carry_needs_paste_return",
        )));
    }
    let agreed = agree_request(store, cell, beat, &pending.ask.request, acts, now).await?;
    carrier().pending.decided(pending.number, now);
    info!(
        number = pending.number,
        ?by,
        "device carrier: ask decided and agreed"
    );
    let ReturnTargetView::Display { value } = &agreed.return_target else {
        return Err(Box::new(response::internal_error(
            "an announced ask got no code",
        )));
    };
    let code = value.split('#').next().unwrap_or_default().to_string();
    let handed_back = match link {
        Some(link) => {
            link.send(
                &pending.source,
                CarryRequest::Code {
                    state: pending.ask.request.state.clone(),
                    code,
                    approver: carrier().self_key().map(str::to_string),
                },
            )
            .await
        }
        None => Err("this node has no carrier".into()),
    };
    let handed_back = match handed_back {
        Ok(CarryResponse::CodeTaken) => serde_json::json!({ "taken": true }),
        Ok(other) => serde_json::json!({ "taken": false, "answer": other }),
        Err(why) => serde_json::json!({ "taken": false, "error": why }),
    };
    info!(number = pending.number, handed_back = %handed_back, "device carrier: code handed back");
    Ok(response::ok(&serde_json::json!({
        "number": pending.number,
        "decidedBy": by,
        "agreed": agreed,
        "handedBack": handed_back,
    })))
}

/// The body of `POST /auth/device/announce`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AnnounceInput {
    request: GrantRequest,
    #[serde(default)]
    approver_key: Option<String>,
}

/// `POST /auth/device/announce`: begin offering this node's ask to the
/// approving nodes local discovery finds. `state` is what this node says about itself.
pub fn start_announce(
    link: Option<&dyn CarrierLink>,
    state: NodeState,
    body: &[u8],
) -> Result<(Response<Full<Bytes>>, bool), Refused> {
    let Some(link) = link else {
        return Err(Box::new(carrier_absent()));
    };
    let input: AnnounceInput = serde_json::from_slice(body)
        .map_err(|e| Box::new(response::bad_request(&format!("Invalid JSON: {e}"))))?;
    consent_grant::admit_request(&input.request, &super::device_consent::policy()).map_err(
        |r| {
            Box::new(refusal(
                StatusCode::BAD_REQUEST,
                "the request would be refused by any approving node",
                r.code(),
            ))
        },
    )?;
    if input.request.return_path != ReturnPath::Paste {
        return Err(Box::new(refusal(
            StatusCode::BAD_REQUEST,
            "an announced ask takes its code back over the carrier, so it asks for paste",
            "carry_needs_paste_return",
        )));
    }
    carrier().start_announce(Announce {
        request: input.request,
        state,
        for_approver: input.approver_key,
        started_at_micros: now_micros(),
        listed_by: Vec::new(),
        code: None,
    });
    Ok((
        response::json_response(
            StatusCode::ACCEPTED,
            &serde_json::json!({ "state": state, "peers": link.peers().len() }),
        ),
        true,
    ))
}

/// The most peers one announce round offers its ask to.
const MAX_PEERS_PER_ROUND: usize = 16;
/// How often an announce offers its ask again.
const ROUND_SECS: u64 = 5;

/// Offer the current announce to every private-network peer, every few
/// seconds, until a code arrives, the five minutes are up, or it is stopped.
pub async fn run_announce(link: std::sync::Arc<dyn CarrierLink>) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static RUNNING: AtomicBool = AtomicBool::new(false);
    // One loop serves whatever announce is current; a second start reuses it.
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    announce_rounds(link).await;
    RUNNING.store(false, Ordering::SeqCst);
}

async fn announce_rounds(link: std::sync::Arc<dyn CarrierLink>) {
    // bounded-work: at most MAX_PEERS_PER_ROUND messages per ROUND_SECS, for
    // at most ASK_LIFE_MICROS; each message waits at most DEVICE_CARRY_TIMEOUT.
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(ROUND_SECS));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        ticker.tick().await;
        let Some(announce) = carrier().announce() else {
            return;
        };
        if announce.code.is_some() {
            return;
        }
        for peer in link.peers().into_iter().take(MAX_PEERS_PER_ROUND) {
            match link
                .send(
                    &peer,
                    CarryRequest::Ask {
                        ask: announce.ask(),
                    },
                )
                .await
            {
                Ok(CarryResponse::Listed { number, approver }) => {
                    carrier().note_listed(&peer, approver, number)
                }
                Ok(CarryResponse::Refused { code }) => {
                    tracing::debug!(peer, code, "device carrier: a peer did not list the ask")
                }
                Ok(_) => {}
                Err(why) => tracing::debug!(peer, why, "device carrier: a peer did not answer"),
            }
        }
    }
}

/// `GET /auth/device/announce`: how this node's ask is doing.
pub fn announce_status() -> Response<Full<Bytes>> {
    let Some(a) = carrier().announce() else {
        return response::ok(&serde_json::json!({ "status": "none" }));
    };
    let left = ((a.started_at_micros + ASK_LIFE_MICROS - now_micros()) / 1_000_000).max(0);
    let listed_by: Vec<_> = a
        .listed_by
        .iter()
        .map(|(_, approver, number)| {
            serde_json::json!({
                "approver": approver,
                "approverFingerprint": approver.as_deref().map(consent_grant::hash_shape::fingerprint),
                "number": number,
            })
        })
        .collect();
    let status = match (&a.code, a.listed_by.is_empty()) {
        (Some(_), _) => "code",
        (None, false) => "listed",
        (None, true) => "announcing",
    };
    response::ok(&serde_json::json!({
        "status": status,
        "secondsLeft": left,
        "state": a.state,
        "listedBy": listed_by,
        "code": a.code.as_ref().map(|(c, _)| format!("{c}#{}", a.request.state)),
    }))
}

/// The body of `POST /auth/device/announce/redeem`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RedeemInput {
    redemption: Redemption,
}

/// `POST /auth/device/announce/redeem`: redeem the code at the approving node that
/// handed it back, over the carrier, with the terminal's verifier.
pub async fn announce_redeem(link: Option<&dyn CarrierLink>, body: &[u8]) -> Response<Full<Bytes>> {
    let Some(link) = link else {
        return carrier_absent();
    };
    let input: RedeemInput = match serde_json::from_slice(body) {
        Ok(i) => i,
        Err(e) => return response::bad_request(&format!("Invalid JSON: {e}")),
    };
    let Some((_, source)) = carrier().announce().and_then(|a| a.code) else {
        return refusal(
            StatusCode::CONFLICT,
            "no approving node has handed a code back yet",
            "carry_no_code",
        );
    };
    match link
        .send(
            &source,
            CarryRequest::Redeem {
                redemption: input.redemption,
            },
        )
        .await
    {
        Ok(CarryResponse::Delivered { delivered }) => {
            carrier().stop_announce();
            response::ok(&*delivered)
        }
        Ok(CarryResponse::Refused { code }) => refusal(
            StatusCode::BAD_REQUEST,
            "the approving node would not hand over the consent",
            &code,
        ),
        Ok(_) => refusal(
            StatusCode::BAD_GATEWAY,
            "the approving node answered something else",
            "carry_unexpected_answer",
        ),
        Err(why) => refusal(
            StatusCode::BAD_GATEWAY,
            &format!("the approving node could not be reached: {why}"),
            "carry_unreachable",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use consent_grant::{pkce, GRANT_DOMAIN};

    fn request() -> GrantRequest {
        GrantRequest {
            domain: GRANT_DOMAIN.into(),
            client_id: "epr-cli".into(),
            device_key: "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl".into(),
            device_root_key: None,
            label: "home".into(),
            network_dna: "uhC0kQwOEwmIBZhBT3I7vGZPz0kEL3_hqavyFW0upoO_hyEwuEglj".into(),
            content_dna: "uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt".into(),
            acts: vec![consent_grant::RequestedAct::EnrollDevice],
            code_challenge: pkce::challenge(&"v".repeat(43)),
            state: "s".repeat(32),
            return_path: ReturnPath::Paste,
        }
    }

    fn fresh() -> Carrier {
        Carrier {
            pending: PendingAsks::new(),
            deliveries: Arc::new(MemoryStore::new()),
            self_key: OnceLock::new(),
            announce: Mutex::new(None),
            speaks: Mutex::new(None),
            speaks_changed: tokio::sync::Notify::new(),
        }
    }

    /// A node that speaks for the fake cell's person.
    fn speaking() -> Speaks {
        let ConsentStanding::Ready(standing) = ready() else {
            unreachable!()
        };
        Speaks::of(
            Some(&standing),
            crate::services::device_consent::tests::CONTROLLER,
            None,
            None,
        )
    }

    #[test]
    fn an_approving_node_lists_an_ask_and_says_which_it_is() {
        let c = fresh();
        c.set_self_key("uhCAkApprover".into());
        c.set_speaks(speaking(), now_micros());
        let ask = Ask {
            request: request(),
            state: NodeState::Unassigned,
            for_approver: None,
        };
        assert_eq!(
            c.on_request("peer-a", CarryRequest::Ask { ask: ask.clone() }),
            Some(CarryResponse::Listed {
                number: 1,
                approver: Some("uhCAkApprover".into())
            })
        );
        let mut elsewhere = ask;
        elsewhere.for_approver = Some("uhCAkOther".into());
        elsewhere.request.device_key =
            "uhCAk0t54SuXFHcSgZ4Bx9gQeXPf7zlckCS0ol65f7cgFl3DucibY".into();
        assert_eq!(
            c.on_request("peer-a", CarryRequest::Ask { ask: elsewhere }),
            Some(CarryResponse::Refused {
                code: "ask_for_another_approver".into()
            })
        );
        assert_eq!(c.pending.len(), 1);
    }

    #[test]
    fn a_node_that_speaks_for_nobody_drops_asks_and_answers_nothing() {
        let c = fresh();
        c.set_self_key(key(60));
        let ask = || CarryRequest::Ask {
            ask: Ask {
                request: request(),
                state: NodeState::Unassigned,
                for_approver: None,
            },
        };
        // Never read: unknown, so nothing is listed and nothing is said.
        assert_eq!(c.speaks(now_micros()), Speaks::Unknown);
        assert_eq!(c.on_request("peer-a", ask()), None);
        // A joined device that is not a controller.
        c.set_speaks(Speaks::Nobody, now_micros());
        assert_eq!(c.on_request("peer-a", ask()), None);
        assert!(c.pending.is_empty());
        // A read too old counts as unknown.
        c.set_speaks(speaking(), now_micros() - SPEAKS_MAX_AGE_MICROS - 1);
        assert_eq!(c.speaks(now_micros()), Speaks::Unknown);
        assert_eq!(c.on_request("peer-a", ask()), None);
        // Fresh, and a controller: listed.
        c.set_speaks(speaking(), now_micros());
        assert!(matches!(
            c.on_request("peer-a", ask()),
            Some(CarryResponse::Listed { number: 1, .. })
        ));
    }

    /// What a node recorded when its identity was begun: `matthew`.
    struct Names;
    impl IdentifierSource for Names {
        fn recorded(&self, human_id: &str, _: &str) -> Option<String> {
            (human_id == "h-1").then(|| "matthew".to_string())
        }
        fn declared(&self) -> Option<consent_grant::DeclaredIdentity> {
            None
        }
    }

    #[tokio::test]
    async fn whom_a_node_speaks_for_is_read_from_its_own_cell() {
        use crate::services::device_consent::tests::{CONTROLLER, IDENTITY};
        let ready_cell = FakeCell::new(Ok(ready()));
        // No Human to name: nobody's word is read.
        let read = read_speaks(&ready_cell, &Names).await.unwrap();
        assert_eq!(read.person().unwrap().identity_root, IDENTITY);
        assert_eq!(read.person().unwrap().identifier, None);
        // With the node's own Human, the word it recorded names the person.
        ready_cell
            .humans
            .lock()
            .unwrap()
            .push(crate::services::device_consent::NewHuman {
                id: "h-1".into(),
                display_name: "Matthew".into(),
                profile_reach: "private".into(),
            });
        let named = read_speaks(&ready_cell, &Names).await.unwrap();
        let person = named.person().unwrap();
        assert_eq!(person.identifier.as_deref(), Some("matthew"));
        assert!(named
            .words()
            .starts_with("An approval here is for matthew (Matthew),"));
        // The cell's key resolves to an identity whose authority does not name it.
        let joined = FakeCell::new(Ok(ConsentStanding::Ready(
            consent_grant::ControllerStanding {
                identity_root: IDENTITY.into(),
                authority: "a".into(),
                network_dna: "n".into(),
                controllers: vec![key(61)],
                required: 1,
            },
        )));
        assert_ne!(key(61), CONTROLLER);
        assert_eq!(read_speaks(&joined, &Names).await, Some(Speaks::Nobody));
        let none = FakeCell::new(Ok(ConsentStanding::NoPerson));
        assert_eq!(read_speaks(&none, &Names).await, Some(Speaks::Nobody));
        let down = FakeCell::new(Err(
            crate::services::device_consent::CellFailure::Unavailable("down".into()),
        ));
        assert_eq!(read_speaks(&down, &Names).await, None);
    }

    #[tokio::test]
    async fn the_list_says_whom_the_node_speaks_for_and_a_node_for_nobody_lists_nothing() {
        let bytes = |r: Response<Full<Bytes>>| async move {
            let b = r.into_body().collect().await.unwrap().to_bytes();
            serde_json::from_slice::<serde_json::Value>(&b).unwrap()
        };
        listed(45, NodeState::Unassigned);
        let nobody = bytes(pending_list(None, &Speaks::Nobody)).await;
        assert_eq!(nobody["speaksFor"], serde_json::json!({"kind": "nobody"}));
        assert_eq!(nobody["asks"], serde_json::json!([]));
        assert!(nobody["speaksForWords"]
            .as_str()
            .unwrap()
            .contains("speaks for nobody"));
        let person = bytes(pending_list(None, &speaking())).await;
        assert_eq!(person["speaksFor"]["kind"], "person");
        let asks = person["asks"].as_array().unwrap();
        assert!(!asks.is_empty());
        for a in asks {
            assert_eq!(
                a["forIdentity"]["identityRoot"],
                crate::services::device_consent::tests::IDENTITY
            );
        }
    }

    #[tokio::test]
    async fn a_node_that_speaks_for_nobody_decides_nothing() {
        let device = listed(46, NodeState::Unassigned);
        let cell = FakeCell::new(Ok(ready()));
        let answer = decide_pending(
            &MemoryStore::new(),
            Some(&cell),
            &consent_grant::Unattended,
            None,
            &declaring(46),
            &Speaks::Nobody,
            true,
            &serde_json::to_vec(&serde_json::json!({ "ask": device })).unwrap(),
        )
        .await;
        assert_eq!(answer.status(), StatusCode::CONFLICT);
        let b = answer.into_body().collect().await.unwrap().to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&b).unwrap();
        assert_eq!(v["code"], "node_speaks_for_nobody");
    }

    #[test]
    fn a_device_takes_a_code_only_for_its_ask_from_a_node_that_listed_it() {
        let c = fresh();
        let code = |source: &str, state: &str| {
            c.on_request(
                source,
                CarryRequest::Code {
                    state: state.into(),
                    code: "c".repeat(32),
                    approver: None,
                },
            )
            .unwrap()
        };
        assert_eq!(
            code("peer-a", &"s".repeat(32)),
            CarryResponse::Refused {
                code: "carry_no_announce".into()
            }
        );
        c.start_announce(Announce {
            request: request(),
            state: NodeState::Unassigned,
            for_approver: None,
            started_at_micros: now_micros(),
            listed_by: vec![],
            code: None,
        });
        assert_eq!(
            code("peer-a", &"s".repeat(32)),
            CarryResponse::Refused {
                code: "carry_from_unlisting_peer".into()
            }
        );
        c.note_listed("peer-a", Some("uhCAkApprover".into()), 1);
        assert_eq!(
            code("peer-a", &"x".repeat(32)),
            CarryResponse::Refused {
                code: "carry_not_this_ask".into()
            }
        );
        assert_eq!(code("peer-a", &"s".repeat(32)), CarryResponse::CodeTaken);
        assert_eq!(
            c.announce().unwrap().code,
            Some(("c".repeat(32), "peer-a".into()))
        );
    }

    use crate::services::device_consent::tests::{ready, FakeCell};
    use crate::services::device_consent::CellStanding as ConsentStanding;
    use http_body_util::BodyExt;

    /// A carrier link that answers every message with `CodeTaken` and keeps
    /// what it was asked to carry.
    struct FakeLink(std::sync::Mutex<Vec<(String, CarryRequest)>>);

    #[async_trait]
    impl CarrierLink for FakeLink {
        fn peers(&self) -> Vec<String> {
            vec!["peer-approver".into()]
        }
        async fn send(&self, peer: &str, request: CarryRequest) -> Result<CarryResponse, String> {
            self.0.lock().unwrap().push((peer.to_string(), request));
            Ok(CarryResponse::CodeTaken)
        }
    }

    fn key(n: u8) -> String {
        holochain_types::prelude::AgentPubKey::from_raw_32(vec![n; 32]).to_string()
    }

    fn state_of(n: u8) -> String {
        format!("state{n:0>27}")
    }

    /// List an ask from a device with key `key(n)` on the global carrier.
    fn listed(n: u8, state: NodeState) -> String {
        let mut r = request();
        r.device_key = key(n);
        // Each ask its own state token: a decided one is remembered by it.
        r.state = state_of(n);
        carrier()
            .pending
            .admit(
                Ask {
                    request: r,
                    state,
                    for_approver: None,
                },
                &format!("peer-device-{n}"),
                None,
                &speaking(),
                &crate::services::device_consent::policy(),
                now_micros(),
            )
            .unwrap();
        key(n)
    }

    fn declaring(n: u8) -> Declaration {
        Declaration {
            identity: None,
            devices: vec![consent_grant::DeclaredDevice {
                label: "home".into(),
                device_key: key(n),
                acts: vec![RequestedAct::EnrollDevice],
            }],
            asking: None,
        }
    }

    async fn decided(
        declaration: &Declaration,
        body: serde_json::Value,
        link: &FakeLink,
    ) -> (StatusCode, serde_json::Value) {
        let cell = FakeCell::new(Ok(ready()));
        let store = MemoryStore::new();
        let answer = decide_pending(
            &store,
            Some(&cell),
            &consent_grant::Unattended,
            Some(link),
            declaration,
            &speaking(),
            true,
            &serde_json::to_vec(&body).unwrap(),
        )
        .await;
        let status = answer.status();
        let bytes = answer.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn a_declared_unassigned_device_is_agreed_by_declaration_and_its_code_goes_back() {
        let device = listed(41, NodeState::Unassigned);
        let link = FakeLink(Default::default());
        let (status, answer) =
            decided(&declaring(41), serde_json::json!({ "ask": device }), &link).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        assert_eq!(answer["decidedBy"], "declaration");
        assert_eq!(answer["handedBack"]["taken"], true);
        let sent = link.0.lock().unwrap().clone();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "peer-device-41");
        assert!(matches!(&sent[0].1, CarryRequest::Code { state, .. } if *state == state_of(41)));
        assert!(carrier().pending.pick(&device, now_micros()).is_none());
    }

    #[tokio::test]
    async fn an_undeclared_device_waits_for_an_answer_and_the_answer_decides() {
        let device = listed(42, NodeState::Unassigned);
        let link = FakeLink(Default::default());
        let (status, answer) = decided(
            &Declaration::default(),
            serde_json::json!({ "ask": device }),
            &link,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(answer["code"], "pending_needs_answer");
        assert_eq!(answer["ask"]["deviceKey"], device);
        assert!(link.0.lock().unwrap().is_empty());
        let (status, answer) = decided(
            &Declaration::default(),
            serde_json::json!({ "ask": device, "answer": { "agreedActs": ["device.enroll"] } }),
            &link,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        assert_eq!(answer["decidedBy"], "answer");
    }

    #[tokio::test]
    async fn a_declared_node_with_an_identity_of_its_own_still_goes_to_an_answer() {
        let device = listed(
            43,
            NodeState::OwnIdentity {
                made: consent_grant::Made::Unknown,
            },
        );
        let link = FakeLink(Default::default());
        let (status, answer) =
            decided(&declaring(43), serde_json::json!({ "ask": device }), &link).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(answer["error"]
            .as_str()
            .unwrap()
            .contains("already has an identity of its own"));
        assert!(link.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_empty_answer_declines_and_sends_nothing() {
        let device = listed(44, NodeState::Unassigned);
        let link = FakeLink(Default::default());
        let (status, answer) = decided(
            &declaring(44),
            serde_json::json!({ "ask": device, "answer": { "agreedActs": [] } }),
            &link,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(answer["declined"], true);
        assert!(link.0.lock().unwrap().is_empty());
        assert!(carrier().pending.pick(&device, now_micros()).is_none());
    }

    #[tokio::test]
    async fn nobody_signed_in_decides_nothing() {
        let cell = FakeCell::new(Ok(ready()));
        let answer = decide_pending(
            &MemoryStore::new(),
            Some(&cell),
            &consent_grant::Unattended,
            None,
            &Declaration::default(),
            &speaking(),
            false,
            b"{\"ask\":\"1\"}",
        )
        .await;
        assert_eq!(answer.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn with_no_local_discovery_the_carrier_is_absent_and_says_so() {
        let refused = start_announce(None, NodeState::Unassigned, b"{}").unwrap_err();
        assert_eq!(refused.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn an_unknown_code_redeems_nothing_over_the_carrier() {
        let c = fresh();
        let refused = c.on_request(
            "peer-a",
            CarryRequest::Redeem {
                redemption: consent_grant::Redemption {
                    code: "c".repeat(32),
                    code_verifier: "v".repeat(43),
                    client_id: "epr-cli".into(),
                    device_key: request().device_key,
                },
            },
        );
        let refused = refused.unwrap();
        assert_eq!(
            refused,
            CarryResponse::Refused {
                code: "redemption_code_unknown".into()
            }
        );
    }
}
