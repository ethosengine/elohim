//! The device consent ceremony, mounted in the peer runtime.
//!
//! A terminal on a device asks to be recognized as one of the person's
//! devices; the person agrees on their own node; a single-use code returns to
//! the terminal, which redeems it. The rules live in `consent_grant`; this
//! module is the node-local mount, and holds no rule of its own.
//!
//! One runtime runs the whole ceremony alone. Nothing here needs a doorway, a
//! database or another peer: the delivery is held in memory for a few minutes
//! and is recovered by asking again if the node restarts.
//!
//! This is recognition: whose device this is. It confers no authority over any
//! content.
//!
//! Three routes:
//!
//! - `view`: what the consent screen shows. Reads nothing.
//! - `agree`: the person, signed in on this node, agrees. This node's own cell
//!   signs as the controller ([`ControllerCell`]), the witness beat runs, and a
//!   one-time code is held for the terminal.
//! - `redeem`: the terminal collects the signed consent and the signed
//!   enrollment it completes on its own cell.
//!
//! Two more for the person's identity on this node:
//!
//! - `identity standing`: what the signed-in person's identity rests on.
//! - `identity bootstrap`: begin the identity here, with this node as its
//!   first steward. Asking again when it exists writes nothing.
//!
//! And two for the asking device's own node, answered only to a terminal on
//! the same machine: what this node is (`device self`), and enrolling it with
//! what the terminal collected (`device enroll`).
//!
//! One controller's agreement is enough. An identity with other controllers
//! may have them affirm the device later; nothing here waits for them.

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use consent_grant::{
    admit_request, attend, check_delivered, issue, redeem, AdmittedRequest, AgreedView,
    ConsentRecord, ConsentSignature, ConsentView, ControllerProof, ControllerStanding, Delivered,
    Enrollment, EnrollmentIntent, GrantPolicy, GrantRequest, MemoryStore, Redemption, RequestedAct,
    SignedConsent, SignerRole, StandingView, WitnessBeat,
};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{header, HeaderMap, Response, StatusCode};
use serde::Deserialize;
use tracing::warn;

use super::response;

/// The relying parties this node will show a consent screen for. The `epr`
/// CLI is the terminal a device asks from.
const KNOWN_CLIENTS: [&str; 1] = ["epr-cli"];

fn policy() -> GrantPolicy {
    GrantPolicy::for_clients(KNOWN_CLIENTS)
}

/// How long a code waits for the terminal: five minutes.
const CODE_TTL_MICROS: i64 = 5 * 60 * 1_000_000;

/// A refusal on its way out, boxed because a response is large to carry in an
/// error.
type Refused = Box<Response<Full<Bytes>>>;

fn refusal(status: StatusCode, error: &str, code: &str) -> Response<Full<Bytes>> {
    response::json_response(status, &serde_json::json!({ "error": error, "code": code }))
}

/// Parse a request body, naming an act this node does not recognise so it
/// fails by name before any consent screen exists for it.
fn parse<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, Refused> {
    serde_json::from_slice(body).map_err(|e| {
        Box::new(if e.to_string().contains("act_unknown") {
            refusal(
                StatusCode::BAD_REQUEST,
                "the request asks for something this node does not recognise",
                "act_unknown",
            )
        } else {
            response::bad_request(&format!("Invalid JSON: {e}"))
        })
    })
}

/// What the consent screen shows for a terminal's request, or why the request
/// is not fit to show a person. Reads nothing and writes nothing.
pub fn consent_view(body: &[u8]) -> Response<Full<Bytes>> {
    let request: GrantRequest = match parse(body) {
        Ok(r) => r,
        Err(refused) => return *refused,
    };
    match admit_request(&request, &policy()) {
        Ok(admitted) => response::ok(&ConsentView::of(&admitted)),
        Err(r) => refusal(
            StatusCode::BAD_REQUEST,
            "the request cannot be shown for consent",
            r.code(),
        ),
    }
}

/// Hand a signed consent to the terminal that asked for it, once.
pub fn redeem_code(store: &MemoryStore, body: &[u8], now_micros: i64) -> Response<Full<Bytes>> {
    let redemption: Redemption = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(e) => return response::bad_request(&format!("Invalid JSON: {e}")),
    };
    // Redeem before sweeping, so a code presented after its window is refused
    // as expired rather than as unknown.
    let outcome = redeem(store, &redemption, now_micros);
    // bounded-work: one pass over the deliveries this node holds, each of
    // which lives minutes and was created by an authenticated agreement.
    store.sweep(now_micros);
    match outcome {
        Ok(delivered) => response::ok(&delivered),
        Err(r) => refusal(
            StatusCode::BAD_REQUEST,
            "the code cannot be redeemed",
            r.code(),
        ),
    }
}

// =============================================================================
// Agree
// =============================================================================

/// Refuse an agreement that a page on another site could have sent.
///
/// Agreeing makes this node's key sign, and this server answers every origin
/// with `Access-Control-Allow-Origin: *` and authenticates the single active
/// session even without a cookie. So, on top of the session, the request must
/// be JSON (a plain cross-site form cannot send that) and, when a browser names
/// an origin, the origin must be this server's own or a loopback one: the
/// node's portal is served by this node or from the same machine. A caller
/// that sends no `Origin` is not a browser page and is let through.
pub fn cross_site_refusal(headers: &HeaderMap) -> Option<Response<Full<Bytes>>> {
    let is_json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"));
    if !is_json {
        return Some(origin_refused());
    }
    foreign_origin_refusal(headers)
}

fn origin_refused() -> Response<Full<Bytes>> {
    refusal(
        StatusCode::FORBIDDEN,
        "this must come from this node's own portal",
        "consent_origin_refused",
    )
}

/// The origin half of [`cross_site_refusal`], for reads: a page on another
/// site may not read what a person's identity rests on either.
pub fn foreign_origin_refusal(headers: &HeaderMap) -> Option<Response<Full<Bytes>>> {
    let refuse = || Some(origin_refused());
    let text = |name: header::HeaderName| headers.get(name).and_then(|v| v.to_str().ok());
    if text(header::HeaderName::from_static("sec-fetch-site")) == Some("cross-site") {
        return refuse();
    }
    // No origin: not a browser page.
    let origin = text(header::ORIGIN)?;
    let Some(authority) = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
    else {
        return refuse();
    };
    let same_origin = text(header::HOST).is_some_and(|host| host.eq_ignore_ascii_case(authority));
    if same_origin || is_loopback(authority) {
        None
    } else {
        refuse()
    }
}

/// Whether `authority` (`host[:port]`) names this machine.
fn is_loopback(authority: &str) -> bool {
    let host = match authority.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => authority.split(':').next().unwrap_or(""),
    };
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

/// What the person sends when they agree: the terminal's request exactly as it
/// came, and the acts they agree to.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgreeInput {
    request: GrantRequest,
    agreed_acts: Vec<RequestedAct>,
}

/// What the controller's cell reports about the person this node speaks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellStanding {
    /// The cell's agent has no Human record.
    NoPerson,
    /// There is a Human (`identity_root`), but `bootstrap_device_identity` has
    /// not run for it.
    Unbootstrapped {
        identity_root: String,
    },
    Ready(ControllerStanding),
}

/// Why the cell could not answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellFailure {
    /// The conductor or the cell could not be reached.
    Unavailable(String),
    /// The cell answered and refused.
    Refused(String),
}

/// What the controller is asked to sign for one approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequest {
    pub consent_cid: String,
    pub authority: String,
    pub identity_root: String,
    /// Present exactly when the person agreed to enroll the device.
    pub enrollment: Option<EnrollmentIntent>,
}

/// The controller's signatures on one approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalProofs {
    /// Over the consent message of the record's address.
    pub consent: ControllerProof,
    /// Over the enrollment intent, when one was asked for.
    pub enrollment: Option<ControllerProof>,
}

/// The cell that signs as the person's controller. On a person's own node it is
/// the node's own cell; a host that keeps cells for others passes the one it
/// keeps for this person.
#[async_trait]
pub trait ControllerCell: Send + Sync {
    /// The cell's agent key: the controller key that signs.
    fn agent(&self) -> String;
    async fn standing(&self) -> Result<CellStanding, CellFailure>;
    async fn sign(&self, approval: &ApprovalRequest) -> Result<ApprovalProofs, CellFailure>;
    /// Record the identity authority for the Human `identity_root`, with this
    /// cell as its first and only controller.
    async fn bootstrap(&self, identity_root: &str) -> Result<(), CellFailure>;
    /// Create this cell's agent's Human record. The extern returns the existing
    /// one, writing nothing, when the agent already has one.
    async fn create_human(&self, human: &NewHuman) -> Result<(), CellFailure>;
}

/// What a person supplies to begin their identity: the minimum the imagodei
/// `create_human` extern requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewHuman {
    pub id: String,
    pub display_name: String,
    /// `public`, `community` or `private`.
    pub profile_reach: String,
}

/// Who may make this node's key sign as its person.
///
/// The person's own node signs for a caller on its own machine. Nothing else
/// a caller can bring today proves it is the person: `POST /session` takes
/// whoever asks, `GET /session` hands any caller the active session's id, and
/// with no cookie the single active session stands in for everyone. So a
/// caller from any other address is refused by name, whatever session it
/// presents. Reads are not affected.
pub fn signing_caller_refusal(caller_is_local: bool) -> Option<Response<Full<Bytes>>> {
    (!caller_is_local).then(|| {
        refusal(
            StatusCode::FORBIDDEN,
            "this signs with this node's key as you, so it is done on this node's own machine",
            "consent_caller_not_local",
        )
    })
}

fn unavailable(detail: &str) -> Response<Full<Bytes>> {
    warn!(detail, "device consent: controller cell unavailable");
    refusal(
        StatusCode::SERVICE_UNAVAILABLE,
        "this node cannot reach the key that would sign; try again shortly",
        "consent_signing_unavailable",
    )
}

fn cell_failure(failure: CellFailure) -> Response<Full<Bytes>> {
    match failure {
        CellFailure::Unavailable(detail) => unavailable(&detail),
        CellFailure::Refused(detail) => {
            warn!(detail, "device consent: controller cell refused to sign");
            refusal(
                StatusCode::CONFLICT,
                "this node's key refused to sign this agreement",
                "consent_signing_refused",
            )
        }
    }
}

/// A one-time code: 256 bits from the operating system's generator.
fn draw_code() -> Result<String, getrandom::Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// The person agrees to a terminal's request on this node.
///
/// `signed_in` is whether this node has an active local session for the
/// person. `cell` is the controller cell, `None` when no conductor is reached.
/// Nothing is written anywhere but the controller's capability grant for the
/// one signing call and the delivery held in `store`.
pub async fn agree(
    store: &MemoryStore,
    cell: Option<&dyn ControllerCell>,
    beat: &dyn WitnessBeat,
    signed_in: bool,
    body: &[u8],
    now_micros: i64,
) -> Response<Full<Bytes>> {
    if !signed_in {
        return refusal(
            StatusCode::UNAUTHORIZED,
            "sign in on this node before approving a device",
            "consent_not_signed_in",
        );
    }
    let input: AgreeInput = match parse(body) {
        Ok(input) => input,
        Err(refused) => return *refused,
    };
    let admitted = match admit_request(&input.request, &policy()) {
        Ok(admitted) => admitted,
        Err(r) => {
            return refusal(
                StatusCode::BAD_REQUEST,
                "the request cannot be agreed to",
                r.code(),
            )
        }
    };
    let Some(cell) = cell else {
        return unavailable("no conductor client");
    };
    let standing = match ready_standing(cell).await {
        Ok(standing) => standing,
        Err(refused) => return *refused,
    };
    let signer = cell.agent();
    if let Err(r) = standing.admits(&admitted, &signer) {
        let (status, error) = match r {
            consent_grant::StandingRefusal::NetworkForeign => (
                StatusCode::CONFLICT,
                "the device asks for a network your identity is not kept on",
            ),
            consent_grant::StandingRefusal::NotAController => (
                StatusCode::FORBIDDEN,
                "this node's key is not one of your identity's controllers",
            ),
        };
        return refusal(status, error, r.code());
    }
    let record =
        match ConsentRecord::agree(&admitted, standing.agreement(input.agreed_acts, now_micros)) {
            Ok(record) => record,
            Err(r) => {
                return refusal(
                    StatusCode::BAD_REQUEST,
                    "what was agreed does not fit the request",
                    r.code(),
                )
            }
        };
    let consent = match SignedConsent::new(record) {
        Ok(consent) => consent,
        Err(r) => return response::internal_error(r.code()),
    };
    let held =
        match sign_and_issue(store, cell, beat, &admitted, consent, &signer, now_micros).await {
            Ok(held) => held,
            Err(refused) => return *refused,
        };
    response::ok(&AgreedView::of(&held.0, held.1, standing.required, &signer))
}

fn no_person() -> Response<Full<Bytes>> {
    refusal(
        StatusCode::CONFLICT,
        "this node has no person yet; register one here before your identity can begin",
        "consent_identity_unbootstrapped",
    )
}

fn unbootstrapped() -> Response<Full<Bytes>> {
    refusal(
        StatusCode::CONFLICT,
        "your identity has no authority record yet; create your identity on this node \
         (POST /auth/identity/bootstrap) before approving a device",
        "consent_identity_unbootstrapped",
    )
}

/// The standing of an identity that already has its authority, or the refusal
/// that says what it is missing.
async fn ready_standing(cell: &dyn ControllerCell) -> Result<ControllerStanding, Refused> {
    match cell.standing().await {
        Ok(CellStanding::Ready(standing)) => Ok(standing),
        Ok(CellStanding::NoPerson) => Err(Box::new(no_person())),
        Ok(CellStanding::Unbootstrapped { .. }) => Err(Box::new(unbootstrapped())),
        Err(failure) => Err(Box::new(cell_failure(failure))),
    }
}

fn not_signed_in() -> Response<Full<Bytes>> {
    refusal(
        StatusCode::UNAUTHORIZED,
        "sign in on this node first",
        "consent_not_signed_in",
    )
}

/// What the signed-in person's identity rests on. Reads only.
pub async fn identity_standing(
    cell: Option<&dyn ControllerCell>,
    signed_in: bool,
) -> Response<Full<Bytes>> {
    if !signed_in {
        return not_signed_in();
    }
    let Some(cell) = cell else {
        return unavailable("no conductor client");
    };
    match ready_standing(cell).await {
        Ok(standing) => response::ok(&StandingView::of(&standing, &cell.agent())),
        Err(refused) => *refused,
    }
}

/// Begin the signed-in person's identity on this node: record its authority,
/// with this node as its first steward.
///
/// From the person's side this can be asked any number of times. When the
/// authority already exists it is returned and nothing is written; otherwise
/// the cell records it (one capability grant and one authority record on the
/// cell's chain) and the new standing is returned.
pub async fn bootstrap_identity(
    cell: Option<&dyn ControllerCell>,
    signed_in: bool,
) -> Response<Full<Bytes>> {
    if !signed_in {
        return not_signed_in();
    }
    let Some(cell) = cell else {
        return unavailable("no conductor client");
    };
    let identity_root = match cell.standing().await {
        Ok(CellStanding::Ready(standing)) => {
            return response::ok(&StandingView::of(&standing, &cell.agent()))
        }
        Ok(CellStanding::NoPerson) => return no_person(),
        Ok(CellStanding::Unbootstrapped { identity_root }) => identity_root,
        Err(failure) => return cell_failure(failure),
    };
    if let Err(failure) = cell.bootstrap(&identity_root).await {
        return cell_failure(failure);
    }
    match ready_standing(cell).await {
        Ok(standing) => response::json_response(
            StatusCode::CREATED,
            &StandingView::of(&standing, &cell.agent()),
        ),
        Err(refused) => *refused,
    }
}

/// What a person on this node's machine sends to begin their identity here.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BeginInput {
    /// What the person is called. Required: the Human record cannot be empty.
    pub display_name: String,
    /// The Human's id. A random one is drawn when absent, as a doorway does.
    #[serde(default)]
    pub human_id: Option<String>,
    /// The word the person signs in with. Defaults to the Human's id.
    #[serde(default)]
    pub identifier: Option<String>,
    /// Defaults to `private`: a profile begun on one's own node is shown to
    /// no one until the person says otherwise.
    #[serde(default)]
    pub profile_reach: Option<String>,
}

/// What beginning an identity did, for the host to finish with a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Begun {
    pub standing: StandingView,
    pub human_id: String,
    pub identifier: String,
    pub display_name: String,
    pub human_created: bool,
    pub authority_created: bool,
}

const MAX_NAME_LEN: usize = 128;

/// Begin a person's identity on this node, with no doorway: create their
/// Human if the node has none, record the identity authority if there is
/// none, and say what was done. Every step is skipped when already done, so
/// asking again writes nothing. The host then opens the person's session.
pub async fn begin_identity(
    cell: Option<&dyn ControllerCell>,
    body: &[u8],
) -> Result<Begun, Refused> {
    let input: BeginInput = parse(body)?;
    let name = input.display_name.trim().to_string();
    let reach = input.profile_reach.unwrap_or_else(|| "private".into());
    let id = input
        .human_id
        .map(|h| h.trim().to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let identifier = input
        .identifier
        .map(|i| i.trim().to_string())
        .unwrap_or_else(|| id.clone());
    let fits = |t: &str| {
        !t.is_empty() && t.chars().count() <= MAX_NAME_LEN && !t.chars().any(char::is_control)
    };
    if !fits(&name) || !fits(&id) || !fits(&identifier) {
        return Err(Box::new(refusal(
            StatusCode::BAD_REQUEST,
            "a name, id and sign-in word are each plain text of at most 128 characters",
            "identity_name_malformed",
        )));
    }
    if !matches!(reach.as_str(), "public" | "community" | "private") {
        return Err(Box::new(refusal(
            StatusCode::BAD_REQUEST,
            "a profile is public, community or private",
            "identity_reach_unknown",
        )));
    }
    let Some(cell) = cell else {
        return Err(Box::new(unavailable("no conductor client")));
    };
    let mut human_created = false;
    let mut authority_created = false;
    let mut standing = cell
        .standing()
        .await
        .map_err(|f| Box::new(cell_failure(f)))?;
    if standing == CellStanding::NoPerson {
        let human = NewHuman {
            id: id.clone(),
            display_name: name.clone(),
            profile_reach: reach,
        };
        cell.create_human(&human)
            .await
            .map_err(|f| Box::new(cell_failure(f)))?;
        human_created = true;
        standing = cell
            .standing()
            .await
            .map_err(|f| Box::new(cell_failure(f)))?;
    }
    if let CellStanding::Unbootstrapped { identity_root } = &standing {
        cell.bootstrap(identity_root)
            .await
            .map_err(|f| Box::new(cell_failure(f)))?;
        authority_created = true;
    }
    let ready = ready_standing(cell).await?;
    Ok(Begun {
        standing: StandingView::of(&ready, &cell.agent()),
        human_id: id,
        identifier,
        display_name: name,
        human_created,
        authority_created,
    })
}

// =============================================================================
// The asking device's own node
// =============================================================================

/// What this node is, for a terminal on it building its request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSelf {
    /// The agent key the node would be enrolled under.
    pub device_key: String,
    /// The network the node's identity cell is on.
    pub network_dna: String,
    /// The network the node's content cell is on.
    pub content_dna: String,
}

/// Where the node notarized its joining record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingReceipt {
    pub binding_action: String,
    pub binding_entry: String,
}

/// The asking device's own cell: what it is, and enrolling it.
#[async_trait]
pub trait DeviceCell: Send + Sync {
    async fn whoami(&self) -> Result<DeviceSelf, CellFailure>;
    /// Sign possession of the enrollment's intent and notarize the binding on
    /// this cell's chain.
    async fn enroll(&self, enrollment: &Enrollment) -> Result<BindingReceipt, CellFailure>;
}

/// What the terminal hands its own node to enroll: the request it made and what
/// it collected for it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnrollInput {
    request: GrantRequest,
    delivered: Delivered,
}

/// Refuse a device step from anyone not on this machine. These steps make the
/// node's own key sign and have no person signed in to vouch for the caller, so
/// only a terminal on the node's own machine may ask.
pub fn remote_caller_refusal(caller_is_local: bool) -> Option<Response<Full<Bytes>>> {
    (!caller_is_local).then(|| {
        refusal(
            StatusCode::FORBIDDEN,
            "only a terminal on this node's own machine may ask this",
            "device_caller_not_local",
        )
    })
}

/// This node's device key and networks, for the terminal building its request.
pub async fn device_self(cell: Option<&dyn DeviceCell>) -> Response<Full<Bytes>> {
    let Some(cell) = cell else {
        return unavailable("no conductor client");
    };
    match cell.whoami().await {
        Ok(me) => response::ok(&me),
        Err(failure) => cell_failure(failure),
    }
}

/// Enroll this node with what its terminal collected.
///
/// The node checks again what the terminal checked (the consent is this
/// request's and every signature verifies), and that the request is for this
/// node's own key and network, before its key signs anything.
pub async fn device_enroll(cell: Option<&dyn DeviceCell>, body: &[u8]) -> Response<Full<Bytes>> {
    let input: EnrollInput = match parse(body) {
        Ok(input) => input,
        Err(refused) => return *refused,
    };
    if let Err(r) = check_delivered(&input.delivered, &input.request) {
        return refusal(
            StatusCode::BAD_REQUEST,
            "what was collected is not a valid consent for this request",
            r.code(),
        );
    }
    let Some(enrollment) = &input.delivered.enrollment else {
        return refusal(
            StatusCode::BAD_REQUEST,
            "the consent does not agree to enroll this device",
            "device_enrollment_not_agreed",
        );
    };
    let Some(cell) = cell else {
        return unavailable("no conductor client");
    };
    let me = match cell.whoami().await {
        Ok(me) => me,
        Err(failure) => return cell_failure(failure),
    };
    if input.request.device_key != me.device_key || input.request.network_dna != me.network_dna {
        return refusal(
            StatusCode::CONFLICT,
            "the consent is for another node's key or network",
            "device_not_this_node",
        );
    }
    match cell.enroll(enrollment).await {
        Ok(receipt) => response::json_response(StatusCode::CREATED, &receipt),
        Err(failure) => cell_failure(failure),
    }
}

/// Have the controller sign, let the witness beat attend, and hold the result.
async fn sign_and_issue(
    store: &MemoryStore,
    cell: &dyn ControllerCell,
    beat: &dyn WitnessBeat,
    admitted: &AdmittedRequest,
    consent: SignedConsent,
    signer: &str,
    now_micros: i64,
) -> Result<(consent_grant::Held, consent_grant::ReturnTarget), Refused> {
    let intent = EnrollmentIntent::agreed_in(&consent.record);
    let approval = ApprovalRequest {
        consent_cid: consent.cid.clone(),
        authority: consent.record.authority.clone(),
        identity_root: consent.record.identity_root.clone(),
        enrollment: intent.clone(),
    };
    let proofs = cell
        .sign(&approval)
        .await
        .map_err(|f| Box::new(cell_failure(f)))?;
    if proofs.consent.agent != signer {
        return Err(Box::new(cell_failure(CellFailure::Refused(
            "a key other than the cell's own signed".into(),
        ))));
    }
    let consent = consent.with_signature(ConsentSignature {
        role: SignerRole::Controller,
        signer: proofs.consent.agent,
        signature: proofs.consent.signature,
    });
    let enrollment = intent.map(|intent| Enrollment {
        intent,
        controllers: proofs.enrollment.into_iter().collect(),
    });
    let consent = attend(beat, admitted, consent);
    let code = draw_code().map_err(|e| Box::new(response::internal_error(&e.to_string())))?;
    // bounded-work: see `redeem_code`.
    store.sweep(now_micros);
    let (held, target) = issue(
        admitted,
        consent,
        enrollment,
        &code,
        now_micros,
        CODE_TTL_MICROS,
    )
    .map_err(|r| Box::new(response::internal_error(r.code())))?;
    store.insert(held.clone());
    Ok((held, target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use consent_grant::{pkce, Agreement, ReturnPath, Unattended, GRANT_DOMAIN};
    use http_body_util::BodyExt;
    use std::sync::Mutex;

    const AGENT: &str = "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl";
    /// Another of the person's nodes, a second controller.
    const AGENT_OTHER_NODE: &str = "uhCAkKmJ2dFuy9oYHzjQIBrBKkDrcm8ZgOP14Ph6OjbF2lb6HDQzS";
    const CONTROLLER: &str = "uhCAk0t54SuXFHcSgZ4Bx9gQeXPf7zlckCS0ol65f7cgFl3DucibY";
    const NETWORK: &str = "uhC0kQwOEwmIBZhBT3I7vGZPz0kEL3_hqavyFW0upoO_hyEwuEglj";
    const CONTENT: &str = "uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt";
    const IDENTITY: &str = "uhCkkRrENFlI2RlXCelrj6C6ttNq8qTI_wSh0fFmsrSvBgdkkM39j";
    const AUTHORITY: &str = "uhCkkN_k9u6glm7eRydJ_WWbyUSSWbBYMHd_6aWO4ag-PrcWHA5_-";
    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const CODE: &str = "c0dec0dec0dec0dec0dec0dec0dec0de";
    const NOW: i64 = 1_791_000_000_000_000;
    const TTL: i64 = 300_000_000;

    fn request() -> GrantRequest {
        GrantRequest {
            domain: GRANT_DOMAIN.into(),
            client_id: "epr-cli".into(),
            device_key: AGENT.into(),
            device_root_key: None,
            label: "workspace".into(),
            network_dna: NETWORK.into(),
            content_dna: CONTENT.into(),
            acts: vec![RequestedAct::EnrollDevice],
            code_challenge: pkce::challenge(VERIFIER),
            state: "s".repeat(32),
            return_path: ReturnPath::Paste,
        }
    }

    /// A store holding one agreed, signed consent, as the node would after the
    /// person agreed.
    fn store_after_agreement() -> MemoryStore {
        let admitted = admit_request(&request(), &policy()).unwrap();
        let record = ConsentRecord::agree(
            &admitted,
            Agreement {
                identity_root: IDENTITY.into(),
                authority: AUTHORITY.into(),
                agreed_acts: vec![RequestedAct::EnrollDevice],
                agreed_at_micros: NOW,
            },
        )
        .unwrap();
        let enrollment = EnrollmentIntent::agreed_in(&record).map(|intent| Enrollment {
            intent,
            controllers: vec![ControllerProof {
                agent: CONTROLLER.into(),
                signature: "enrollment-sig".into(),
            }],
        });
        let signed = SignedConsent::new(record)
            .unwrap()
            .with_signature(ConsentSignature {
                role: SignerRole::Controller,
                signer: CONTROLLER.into(),
                signature: "sig".into(),
            });
        let (held, _) = issue(&admitted, signed, enrollment, CODE, NOW, TTL).unwrap();
        let store = MemoryStore::new();
        store.insert(held);
        store
    }

    fn redemption() -> Redemption {
        Redemption {
            code: CODE.into(),
            code_verifier: VERIFIER.into(),
            client_id: "epr-cli".into(),
            device_key: AGENT.into(),
        }
    }

    async fn json(resp: Response<Full<Bytes>>) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn the_view_names_the_device_and_what_it_asks() {
        let body = serde_json::to_vec(&request()).unwrap();
        let (status, view) = json(consent_view(&body)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(view["label"], "workspace");
        assert_eq!(view["askedActs"][0], "device.enroll");
        assert_eq!(view["deviceFingerprint"], "uhCAkiqcz…TGMzzl");
        assert!(view.get("deviceRootFingerprint").is_none());
    }

    #[tokio::test]
    async fn an_unfit_request_is_refused_by_name() {
        let mut bad = request();
        bad.client_id = "stranger".into();
        let (status, body) = json(consent_view(&serde_json::to_vec(&bad).unwrap())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "request_client_unknown");

        let mut unknown = serde_json::to_value(request()).unwrap();
        unknown["acts"] = serde_json::json!(["content.publish"]);
        let (status, body) = json(consent_view(&serde_json::to_vec(&unknown).unwrap())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "act_unknown");

        let (status, _) = json(consent_view(b"not json")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn the_asking_terminal_collects_the_signed_consent_once() {
        let store = store_after_agreement();
        let body = serde_json::to_vec(&redemption()).unwrap();

        let (status, consent) = json(redeem_code(&store, &body, NOW + 1)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(consent["cid"].as_str().unwrap().starts_with("bafyrei"));
        assert_eq!(consent["record"]["agreedActs"][0], "device.enroll");
        assert_eq!(consent["signatures"][0]["signer"], CONTROLLER);
        assert_eq!(consent["enrollment"]["intent"]["deviceKey"], AGENT);
        assert_eq!(consent["enrollment"]["controllers"][0]["agent"], CONTROLLER);

        let (status, again) = json(redeem_code(&store, &body, NOW + 2)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(again["code"], "redemption_already_redeemed");
    }

    #[tokio::test]
    async fn a_stranger_with_the_code_is_refused_and_the_code_is_spent() {
        let store = store_after_agreement();
        let mut stranger = redemption();
        stranger.code_verifier = "x".repeat(43);
        let (status, body) = json(redeem_code(
            &store,
            &serde_json::to_vec(&stranger).unwrap(),
            NOW + 1,
        ))
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "redemption_verifier_mismatch");

        let (_, body) = json(redeem_code(
            &store,
            &serde_json::to_vec(&redemption()).unwrap(),
            NOW + 2,
        ))
        .await;
        assert_eq!(body["code"], "redemption_code_unknown");
    }

    #[tokio::test]
    async fn an_expired_code_is_refused() {
        let store = store_after_agreement();
        let body = serde_json::to_vec(&redemption()).unwrap();
        let (status, body) = json(redeem_code(&store, &body, NOW + TTL)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "redemption_expired");
        assert!(store.is_empty());
    }

    // --- agree ------------------------------------------------------------

    /// A controller cell that answers as configured and remembers what it was
    /// asked to sign.
    struct FakeCell {
        standing: Mutex<Result<CellStanding, CellFailure>>,
        signs: Result<(), CellFailure>,
        asked: Mutex<Vec<ApprovalRequest>>,
        bootstraps: Mutex<Vec<String>>,
        humans: Mutex<Vec<NewHuman>>,
    }

    fn ready() -> CellStanding {
        CellStanding::Ready(ControllerStanding {
            identity_root: IDENTITY.into(),
            authority: AUTHORITY.into(),
            network_dna: NETWORK.into(),
            controllers: vec![CONTROLLER.into()],
            required: 1,
        })
    }

    impl FakeCell {
        fn new(standing: Result<CellStanding, CellFailure>) -> Self {
            Self {
                standing: Mutex::new(standing),
                signs: Ok(()),
                asked: Mutex::new(Vec::new()),
                bootstraps: Mutex::new(Vec::new()),
                humans: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ControllerCell for FakeCell {
        fn agent(&self) -> String {
            CONTROLLER.into()
        }
        async fn standing(&self) -> Result<CellStanding, CellFailure> {
            self.standing.lock().unwrap().clone()
        }
        async fn bootstrap(&self, identity_root: &str) -> Result<(), CellFailure> {
            self.bootstraps.lock().unwrap().push(identity_root.into());
            *self.standing.lock().unwrap() = Ok(ready());
            Ok(())
        }
        async fn create_human(&self, human: &NewHuman) -> Result<(), CellFailure> {
            self.humans.lock().unwrap().push(human.clone());
            *self.standing.lock().unwrap() = Ok(CellStanding::Unbootstrapped {
                identity_root: IDENTITY.into(),
            });
            Ok(())
        }
        async fn sign(&self, approval: &ApprovalRequest) -> Result<ApprovalProofs, CellFailure> {
            self.asked.lock().unwrap().push(approval.clone());
            self.signs.clone()?;
            Ok(ApprovalProofs {
                consent: ControllerProof {
                    agent: CONTROLLER.into(),
                    signature: "consent-sig".into(),
                },
                enrollment: approval.enrollment.as_ref().map(|_| ControllerProof {
                    agent: CONTROLLER.into(),
                    signature: "enrollment-sig".into(),
                }),
            })
        }
    }

    // --- identity -----------------------------------------------------------

    #[tokio::test]
    async fn an_identity_begins_here_once_and_asking_again_writes_nothing() {
        let cell = FakeCell::new(Ok(CellStanding::Unbootstrapped {
            identity_root: IDENTITY.into(),
        }));
        let (status, refused) = json(identity_standing(Some(&cell), true).await).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(refused["code"], "consent_identity_unbootstrapped");

        let (status, view) = json(bootstrap_identity(Some(&cell), true).await).await;
        assert_eq!(status, StatusCode::CREATED, "{view}");
        assert_eq!(view["identityRoot"], IDENTITY);
        assert_eq!(view["authority"], AUTHORITY);
        assert_eq!(view["controllerCount"], 1);
        assert_eq!(view["required"], 1);
        assert_eq!(view["restsOnThisNodeAlone"], true);
        assert_eq!(cell.bootstraps.lock().unwrap().as_slice(), [IDENTITY]);

        let (status, again) = json(bootstrap_identity(Some(&cell), true).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(again, view);
        assert_eq!(cell.bootstraps.lock().unwrap().len(), 1);

        let (status, read) = json(identity_standing(Some(&cell), true).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(read, view);
    }

    #[tokio::test]
    async fn a_person_begins_on_their_own_node_with_no_doorway() {
        let cell = FakeCell::new(Ok(CellStanding::NoPerson));
        let begun = begin_identity(Some(&cell), br#"{"displayName":"Matthew"}"#)
            .await
            .unwrap();
        assert!(begun.human_created && begun.authority_created);
        assert!(begun.standing.rests_on_this_node_alone);
        assert_eq!(begun.identifier, begun.human_id);
        let humans = cell.humans.lock().unwrap().clone();
        assert_eq!(humans.len(), 1);
        assert_eq!(humans[0].display_name, "Matthew");
        assert_eq!(humans[0].profile_reach, "private");
        assert_eq!(humans[0].id, begun.human_id);

        // Again: nothing more is created.
        let again = begin_identity(Some(&cell), br#"{"displayName":"Matthew"}"#)
            .await
            .unwrap();
        assert!(!again.human_created && !again.authority_created);
        assert_eq!(again.standing, begun.standing);
        assert_eq!(cell.humans.lock().unwrap().len(), 1);
        assert_eq!(cell.bootstraps.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_begin_names_what_is_wrong_with_it() {
        let cell = FakeCell::new(Ok(CellStanding::NoPerson));
        for (body, code) in [
            (r#"{"displayName":"   "}"#, "identity_name_malformed"),
            (r#"{"displayName":"a\nb"}"#, "identity_name_malformed"),
            (
                r#"{"displayName":"M","profileReach":"commons"}"#,
                "identity_reach_unknown",
            ),
        ] {
            let refused = begin_identity(Some(&cell), body.as_bytes())
                .await
                .unwrap_err();
            let (status, json_body) = json(*refused).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
            assert_eq!(json_body["code"], code, "{body}");
        }
        let refused = begin_identity(Some(&cell), br#"{"displayName":"M","doorwayUrl":"x"}"#)
            .await
            .unwrap_err();
        assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
        assert!(cell.humans.lock().unwrap().is_empty());
    }

    #[test]
    fn only_this_machine_may_make_the_node_sign_as_its_person() {
        assert!(signing_caller_refusal(true).is_none());
        let refused = signing_caller_refusal(false).unwrap();
        assert_eq!(refused.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn an_identity_cannot_begin_for_nobody_or_for_a_stranger() {
        let cell = FakeCell::new(Ok(CellStanding::NoPerson));
        let (status, refused) = json(bootstrap_identity(Some(&cell), true).await).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(refused["code"], "consent_identity_unbootstrapped");
        assert!(cell.bootstraps.lock().unwrap().is_empty());

        for response in [
            bootstrap_identity(Some(&cell), false).await,
            identity_standing(Some(&cell), false).await,
        ] {
            let (status, refused) = json(response).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(refused["code"], "consent_not_signed_in");
        }
        let (status, refused) = json(identity_standing(None, true).await).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(refused["code"], "consent_signing_unavailable");
    }

    #[test]
    fn a_page_on_another_site_cannot_read_the_standing_either() {
        let host = ("host", "127.0.0.1:8090");
        assert!(foreign_origin_refusal(&headers(&[host])).is_none());
        assert!(
            foreign_origin_refusal(&headers(&[host, ("origin", "http://localhost:4200")]))
                .is_none()
        );
        assert!(
            foreign_origin_refusal(&headers(&[host, ("origin", "https://evil.example")])).is_some()
        );
    }

    // --- the asking device ---------------------------------------------------

    use elohim_epr::proof::{sign, AgentKeypair};

    fn agent_of(key: &AgentKeypair) -> String {
        holochain_types::prelude::AgentPubKey::from_raw_32(key.public_key_bytes().to_vec())
            .to_string()
    }

    /// What an honest steward delivers for `request()`, signed by a real key.
    fn honestly_delivered() -> Delivered {
        let key = AgentKeypair::from_secret(&[9; 32]).unwrap();
        let steward = agent_of(&key);
        let admitted = admit_request(&request(), &policy()).unwrap();
        let record = ConsentRecord::agree(
            &admitted,
            Agreement {
                identity_root: IDENTITY.into(),
                authority: AUTHORITY.into(),
                agreed_acts: vec![RequestedAct::EnrollDevice],
                agreed_at_micros: NOW,
            },
        )
        .unwrap();
        let consent = SignedConsent::new(record.clone()).unwrap();
        let signature = URL_SAFE_NO_PAD.encode(sign(&key, &consent.message()));
        let intent = EnrollmentIntent::agreed_in(&record).unwrap();
        let proof = URL_SAFE_NO_PAD.encode(sign(&key, &intent.signed_bytes().unwrap()));
        Delivered {
            consent: consent.with_signature(ConsentSignature {
                role: SignerRole::Controller,
                signer: steward.clone(),
                signature,
            }),
            enrollment: Some(Enrollment {
                intent,
                controllers: vec![ControllerProof {
                    agent: steward,
                    signature: proof,
                }],
            }),
        }
    }

    struct FakeDevice {
        enrolled: Mutex<Vec<Enrollment>>,
    }

    #[async_trait]
    impl DeviceCell for FakeDevice {
        async fn whoami(&self) -> Result<DeviceSelf, CellFailure> {
            Ok(DeviceSelf {
                device_key: AGENT.into(),
                network_dna: NETWORK.into(),
                content_dna: CONTENT.into(),
            })
        }
        async fn enroll(&self, enrollment: &Enrollment) -> Result<BindingReceipt, CellFailure> {
            self.enrolled.lock().unwrap().push(enrollment.clone());
            Ok(BindingReceipt {
                binding_action: AUTHORITY.into(),
                binding_entry: "uhCEk".into(),
            })
        }
    }

    fn enroll_body(request: &GrantRequest, delivered: &Delivered) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({ "request": request, "delivered": delivered }))
            .unwrap()
    }

    #[tokio::test]
    async fn the_device_enrolls_only_with_a_consent_that_checks() {
        let device = FakeDevice {
            enrolled: Mutex::new(Vec::new()),
        };
        let (status, me) = json(device_self(Some(&device)).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(me["deviceKey"], AGENT);
        assert_eq!(me["contentDna"], CONTENT);

        let (status, receipt) = json(
            device_enroll(
                Some(&device),
                &enroll_body(&request(), &honestly_delivered()),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{receipt}");
        assert_eq!(receipt["bindingAction"], AUTHORITY);
        assert_eq!(device.enrolled.lock().unwrap().len(), 1);

        let mut forged = honestly_delivered();
        forged.consent.signatures[0].signature = URL_SAFE_NO_PAD.encode([0u8; 64]);
        let (status, refused) =
            json(device_enroll(Some(&device), &enroll_body(&request(), &forged)).await).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(refused["code"], "delivered_signature_invalid");

        // A consent made for some other node's key never reaches this one's.
        let mut other = request();
        other.device_key = AGENT_OTHER_NODE.into();
        let (_, refused) =
            json(device_enroll(Some(&device), &enroll_body(&other, &honestly_delivered())).await)
                .await;
        assert_eq!(refused["code"], "delivered_not_this_request");
        assert_eq!(device.enrolled.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_device_step_answers_only_its_own_machine() {
        assert!(remote_caller_refusal(true).is_none());
        let refused = remote_caller_refusal(false).unwrap();
        assert_eq!(refused.status(), StatusCode::FORBIDDEN);
    }

    fn agree_body(request: &GrantRequest, acts: &[&str]) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({ "request": request, "agreedActs": acts })).unwrap()
    }

    async fn agree_with(
        store: &MemoryStore,
        cell: &FakeCell,
        body: &[u8],
    ) -> (StatusCode, serde_json::Value) {
        json(agree(store, Some(cell), &Unattended, true, body, NOW).await).await
    }

    /// The code the agreement returned, for a paste return path.
    fn pasted_code(answer: &serde_json::Value) -> String {
        let shown = answer["returnTarget"]["value"].as_str().unwrap();
        consent_grant::parse_pasted(shown).unwrap().0.to_string()
    }

    #[tokio::test]
    async fn one_node_runs_the_whole_ceremony_alone() {
        let store = MemoryStore::new();
        let cell = FakeCell::new(Ok(ready()));
        let (status, agreed) =
            agree_with(&store, &cell, &agree_body(&request(), &["device.enroll"])).await;
        assert_eq!(status, StatusCode::OK, "{agreed}");
        assert_eq!(agreed["returnTarget"]["kind"], "display");
        assert_eq!(agreed["expiresAt"], (NOW + CODE_TTL_MICROS) / 1000);
        assert!(agreed["consentCid"]
            .as_str()
            .unwrap()
            .starts_with("bafyrei"));
        assert_eq!(
            agreed["controllers"],
            serde_json::json!({"required": 1, "signed": 1})
        );
        assert_eq!(
            agreed["witnesses"],
            serde_json::json!([{
                "id": CONTROLLER, "act": "signed", "relation": "this-device", "state": "done"
            }])
        );

        // The controller was asked to sign the consent and the enrollment once.
        let asked = cell.asked.lock().unwrap().clone();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].consent_cid, agreed["consentCid"]);
        assert_eq!(asked[0].enrollment.as_ref().unwrap().device_key, AGENT);

        let mut collect = redemption();
        collect.code = pasted_code(&agreed);
        let (status, delivered) = json(redeem_code(
            &store,
            &serde_json::to_vec(&collect).unwrap(),
            NOW + 1,
        ))
        .await;
        assert_eq!(status, StatusCode::OK, "{delivered}");
        assert_eq!(delivered["cid"], agreed["consentCid"]);
        assert_eq!(delivered["signatures"][0]["signature"], "consent-sig");
        assert_eq!(
            delivered["enrollment"]["controllers"][0]["signature"],
            "enrollment-sig"
        );
        assert_eq!(delivered["enrollment"]["intent"]["identityRoot"], IDENTITY);
    }

    #[tokio::test]
    async fn a_loopback_terminal_is_handed_the_code_by_redirect() {
        let store = MemoryStore::new();
        let mut local = request();
        local.return_path = ReturnPath::Loopback { port: 49152 };
        let (status, agreed) = agree_with(
            &store,
            &FakeCell::new(Ok(ready())),
            &agree_body(&local, &["device.enroll"]),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(agreed["returnTarget"]["kind"], "redirect");
        assert!(agreed["returnTarget"]["url"]
            .as_str()
            .unwrap()
            .starts_with("http://127.0.0.1:49152/callback?code="));
    }

    #[tokio::test]
    async fn agreeing_again_leaves_only_the_newest_code_working() {
        let store = MemoryStore::new();
        let cell = FakeCell::new(Ok(ready()));
        let body = agree_body(&request(), &["device.enroll"]);
        let (_, first) = agree_with(&store, &cell, &body).await;
        let (_, second) = agree_with(&store, &cell, &body).await;
        assert_eq!(store.len(), 1);

        let mut old = redemption();
        old.code = pasted_code(&first);
        let (_, refused) = json(redeem_code(
            &store,
            &serde_json::to_vec(&old).unwrap(),
            NOW + 1,
        ))
        .await;
        assert_eq!(refused["code"], "redemption_code_unknown");
        let mut new = redemption();
        new.code = pasted_code(&second);
        let (status, _) = json(redeem_code(
            &store,
            &serde_json::to_vec(&new).unwrap(),
            NOW + 1,
        ))
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_policy_asking_for_more_controllers_does_not_hold_the_approval() {
        let store = MemoryStore::new();
        let CellStanding::Ready(mut standing) = ready() else {
            unreachable!()
        };
        standing.controllers.push(AGENT_OTHER_NODE.into());
        standing.required = 2;
        let cell = FakeCell::new(Ok(CellStanding::Ready(standing)));
        let (status, agreed) =
            agree_with(&store, &cell, &agree_body(&request(), &["device.enroll"])).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            agreed["controllers"],
            serde_json::json!({"required": 2, "signed": 1})
        );
        assert_eq!(agreed["witnesses"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn each_reason_not_to_agree_is_refused_by_name() {
        let body = agree_body(&request(), &["device.enroll"]);
        let store = MemoryStore::new();
        let cell = FakeCell::new(Ok(ready()));

        let (status, refused) =
            json(agree(&store, Some(&cell), &Unattended, false, &body, NOW).await).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(refused["code"], "consent_not_signed_in");

        let (status, refused) =
            json(agree(&store, None, &Unattended, true, &body, NOW).await).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(refused["code"], "consent_signing_unavailable");

        for standing in [
            CellStanding::Unbootstrapped {
                identity_root: IDENTITY.into(),
            },
            CellStanding::NoPerson,
        ] {
            let (status, refused) = agree_with(&store, &FakeCell::new(Ok(standing)), &body).await;
            assert_eq!(status, StatusCode::CONFLICT);
            assert_eq!(refused["code"], "consent_identity_unbootstrapped");
        }
        let (_, refused) = agree_with(
            &store,
            &FakeCell::new(Ok(CellStanding::Unbootstrapped {
                identity_root: IDENTITY.into(),
            })),
            &body,
        )
        .await;
        assert!(refused["error"]
            .as_str()
            .unwrap()
            .contains("/auth/identity/bootstrap"));

        let (status, refused) = agree_with(
            &store,
            &FakeCell::new(Err(CellFailure::Unavailable("socket closed".into()))),
            &body,
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(refused["code"], "consent_signing_unavailable");

        let mut refuses = FakeCell::new(Ok(ready()));
        refuses.signs = Err(CellFailure::Refused("Guest(no)".into()));
        let (status, refused) = agree_with(&store, &refuses, &body).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(refused["code"], "consent_signing_refused");

        let CellStanding::Ready(mut not_mine) = ready() else {
            unreachable!()
        };
        not_mine.controllers = vec![AGENT_OTHER_NODE.into()];
        let (status, refused) = agree_with(
            &store,
            &FakeCell::new(Ok(CellStanding::Ready(not_mine))),
            &body,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(refused["code"], "consent_not_a_controller");

        let mut elsewhere = request();
        elsewhere.network_dna = CONTENT.into();
        let (status, refused) = agree_with(
            &store,
            &FakeCell::new(Ok(ready())),
            &agree_body(&elsewhere, &["device.enroll"]),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(refused["code"], "consent_network_foreign");

        let mut bad = request();
        bad.client_id = "stranger".into();
        let (status, refused) =
            agree_with(&store, &cell, &agree_body(&bad, &["device.enroll"])).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(refused["code"], "request_client_unknown");

        let (status, refused) = agree_with(
            &store,
            &cell,
            &agree_body(&request(), &["device.bind-root"]),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(refused["code"], "consent_agreed_beyond_request");

        let (_, refused) =
            agree_with(&store, &cell, &agree_body(&request(), &["content.publish"])).await;
        assert_eq!(refused["code"], "act_unknown");

        // Nothing was held for any refusal, and nothing signed before standing held.
        assert!(store.is_empty());
        assert_eq!(cell.asked.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn binding_the_root_is_recorded_in_the_consent() {
        let mut both = request();
        both.acts.push(RequestedAct::BindDeviceRoot);
        both.device_root_key = Some(format!("did:key:z6Mk{}", "h".repeat(44)));
        let store = MemoryStore::new();
        let cell = FakeCell::new(Ok(ready()));
        let (status, agreed) = agree_with(
            &store,
            &cell,
            &agree_body(&both, &["device.enroll", "device.bind-root"]),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{agreed}");
        let mut collect = redemption();
        collect.code = pasted_code(&agreed);
        let (_, delivered) = json(redeem_code(
            &store,
            &serde_json::to_vec(&collect).unwrap(),
            NOW + 1,
        ))
        .await;
        assert_eq!(
            delivered["record"]["agreedActs"],
            serde_json::json!(["device.enroll", "device.bind-root"])
        );
        assert!(delivered["record"]["deviceRootKey"].is_string());
    }

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(*name, value.parse().unwrap());
        }
        map
    }

    #[test]
    fn only_this_nodes_own_portal_may_ask_it_to_sign() {
        let json_type = ("content-type", "application/json");
        let host = ("host", "127.0.0.1:8090");
        for allowed in [
            headers(&[json_type, host]),
            headers(&[("content-type", "application/json; charset=utf-8"), host]),
            headers(&[
                json_type,
                ("host", "node.example:8090"),
                ("origin", "https://node.example:8090"),
            ]),
            headers(&[json_type, host, ("origin", "http://localhost:8081")]),
            headers(&[json_type, host, ("origin", "http://[::1]:4200")]),
        ] {
            assert!(cross_site_refusal(&allowed).is_none(), "{allowed:?}");
        }
        for refused in [
            headers(&[host]),
            headers(&[("content-type", "text/plain"), host]),
            headers(&[("content-type", "application/x-www-form-urlencoded"), host]),
            headers(&[json_type, host, ("origin", "https://evil.example")]),
            headers(&[json_type, host, ("origin", "null")]),
            headers(&[json_type, host, ("origin", "http://localhost.evil.example")]),
            headers(&[json_type, host, ("sec-fetch-site", "cross-site")]),
        ] {
            let response = cross_site_refusal(&refused).expect("refused");
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{refused:?}");
        }
    }
}
