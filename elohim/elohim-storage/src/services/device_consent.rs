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
//! One controller's agreement is enough. An identity with other controllers
//! may have them affirm the device later; nothing here waits for them.

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use consent_grant::{
    admit_request, attend, issue, redeem, AdmittedRequest, AgreedView, ConsentRecord,
    ConsentSignature, ConsentView, ControllerProof, ControllerStanding, Enrollment,
    EnrollmentIntent, GrantPolicy, GrantRequest, MemoryStore, Redemption, RequestedAct,
    SignedConsent, SignerRole, WitnessBeat,
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
    let refuse = || {
        Some(refusal(
            StatusCode::FORBIDDEN,
            "agreeing must come from this node's own portal",
            "consent_origin_refused",
        ))
    };
    let text = |name: header::HeaderName| headers.get(name).and_then(|v| v.to_str().ok());
    let is_json = text(header::CONTENT_TYPE)
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"));
    if !is_json {
        return refuse();
    }
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
    /// There is a Human, but `bootstrap_device_identity` has not run for it.
    Unbootstrapped,
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
    let standing = match cell.standing().await {
        Ok(CellStanding::Ready(standing)) => standing,
        Ok(CellStanding::NoPerson) => {
            return refusal(
                StatusCode::CONFLICT,
                "this node has no person yet; register one here before approving a device",
                "consent_identity_unbootstrapped",
            )
        }
        Ok(CellStanding::Unbootstrapped) => {
            return refusal(
                StatusCode::CONFLICT,
                "your identity has no authority record yet; set up device identity on this \
                 node (bootstrap_device_identity) before approving a device",
                "consent_identity_unbootstrapped",
            )
        }
        Err(failure) => return cell_failure(failure),
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
        standing: Result<CellStanding, CellFailure>,
        signs: Result<(), CellFailure>,
        asked: Mutex<Vec<ApprovalRequest>>,
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
                standing,
                signs: Ok(()),
                asked: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ControllerCell for FakeCell {
        fn agent(&self) -> String {
            CONTROLLER.into()
        }
        async fn standing(&self) -> Result<CellStanding, CellFailure> {
            self.standing.clone()
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

        for standing in [CellStanding::Unbootstrapped, CellStanding::NoPerson] {
            let (status, refused) = agree_with(&store, &FakeCell::new(Ok(standing)), &body).await;
            assert_eq!(status, StatusCode::CONFLICT);
            assert_eq!(refused["code"], "consent_identity_unbootstrapped");
        }
        let (_, refused) = agree_with(
            &store,
            &FakeCell::new(Ok(CellStanding::Unbootstrapped)),
            &body,
        )
        .await;
        assert!(refused["error"]
            .as_str()
            .unwrap()
            .contains("bootstrap_device_identity"));

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
