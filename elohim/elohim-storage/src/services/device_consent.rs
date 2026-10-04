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

use consent_grant::{
    admit_request, redeem, ConsentView, GrantPolicy, GrantRequest, MemoryStore, Redemption,
};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};

use super::response;

/// The relying parties this node will show a consent screen for. The `epr`
/// CLI is the terminal a device asks from.
const KNOWN_CLIENTS: [&str; 1] = ["epr-cli"];

fn policy() -> GrantPolicy {
    GrantPolicy::for_clients(KNOWN_CLIENTS)
}

fn refusal(status: StatusCode, error: &str, code: &str) -> Response<Full<Bytes>> {
    response::json_response(status, &serde_json::json!({ "error": error, "code": code }))
}

/// What the consent screen shows for a terminal's request, or why the request
/// is not fit to show a person. Reads nothing and writes nothing.
pub fn consent_view(body: &[u8]) -> Response<Full<Bytes>> {
    let request: GrantRequest = match serde_json::from_slice(body) {
        Ok(r) => r,
        // An act this node does not recognise fails here, by name, before any
        // consent screen exists for it.
        Err(e) if e.to_string().contains("act_unknown") => {
            return refusal(
                StatusCode::BAD_REQUEST,
                "the request asks for something this node does not recognise",
                "act_unknown",
            )
        }
        Err(e) => return response::bad_request(&format!("Invalid JSON: {e}")),
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
    // bounded-work: one pass over the deliveries this node holds, each of
    // which lives minutes and was created by an authenticated agreement.
    store.sweep(now_micros);
    match redeem(store, &redemption, now_micros) {
        Ok(consent) => response::ok(&consent),
        Err(r) => refusal(
            StatusCode::BAD_REQUEST,
            "the code cannot be redeemed",
            r.code(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use consent_grant::{
        issue, pkce, Agreement, ConsentRecord, ConsentSignature, RequestedAct, ReturnPath,
        SignedConsent, SignerRole, GRANT_DOMAIN,
    };
    use http_body_util::BodyExt;

    const AGENT: &str = "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl";
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
        let signed = SignedConsent::new(record)
            .unwrap()
            .with_signature(ConsentSignature {
                role: SignerRole::Controller,
                signer: CONTROLLER.into(),
                signature: "sig".into(),
            });
        let (held, _) = issue(&admitted, signed, CODE, NOW, TTL).unwrap();
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
        // Swept before it is looked up, so it reads as unknown.
        assert_eq!(body["code"], "redemption_code_unknown");
        assert!(store.is_empty());
    }
}
