//! Graduating a hosted account onto the human's own node key.
//!
//! `POST /auth/node-key/challenge` and `POST /auth/node-key/bind`. Spec:
//! `genesis/docs/superpowers/specs/2026-10-16-graduate-onto-own-node-key-design.md`.
//!
//! Two credentials from two parties move the account: the doorway session plus
//! the re-entered password open a challenge; the human's node signs it with the
//! key being claimed. The decision lives in pure functions
//! ([`build_bind_challenge`], [`parse_bind_challenge`], [`node_key_bind_verdict`])
//! so every named refusal is testable without Mongo, a JWT or a conductor.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bson::doc;
use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::Verifier;
use hyper::{Request, Response, StatusCode};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::auth::{extract_token_from_header, verify_password, Claims};
use crate::conductor::AgentProvisioner;
use crate::custodial_keys::CustodialKeyService;
use crate::db::schemas::{UserDoc, USER_COLLECTION};
use crate::routes::auth_routes::{
    account_stamp, active_user_filter, gateway_domain, generate_auth_response, get_auth_header,
    get_jwt_validator, json_response, parse_json_body, BoxBody, ErrorResponse,
};
use crate::routes::hosted_cell::rfc3339_utc_secs;
use crate::server::AppState;

/// First line of every bind challenge; the node re-checks it, so the route is
/// never a general signing oracle.
pub const BIND_PURPOSE: &str = "elohim:doorway-account-bind:v1";
/// Challenge lifetime.
pub const CHALLENGE_TTL_SECS: i64 = 300;
/// Largest challenge text either side accepts.
pub const MAX_CHALLENGE_BYTES: usize = 1024;

// =============================================================================
// Pure decision logic
// =============================================================================

/// The seven fields of a v1 challenge, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindChallenge {
    pub audience: String,
    pub identifier: String,
    pub human: String,
    pub from: String,
    pub nonce: String,
    pub expires: DateTime<Utc>,
}

/// Why a text is not a v1 bind challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseRefusal {
    TooLarge,
    CarriageReturn,
    WrongLineCount,
    WrongPurpose,
    WrongField(&'static str),
    BadNonce,
    BadExpires,
}

/// A challenge the doorway issued and still holds.
#[derive(Debug, Clone)]
pub struct StoredChallenge {
    pub identifier: String,
    pub challenge_text: String,
    pub from_key: String,
    pub expires: DateTime<Utc>,
}

/// Named refusals of the bind, one per code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindRefusal {
    /// 404: unknown, spent or expired.
    ChallengeUnknown,
    /// 403
    ChallengeNotYours,
    /// 409
    SameKey,
    /// 403
    SignatureInvalid,
}

impl BindRefusal {
    pub fn status(self) -> StatusCode {
        match self {
            Self::ChallengeUnknown => StatusCode::NOT_FOUND,
            Self::ChallengeNotYours | Self::SignatureInvalid => StatusCode::FORBIDDEN,
            Self::SameKey => StatusCode::CONFLICT,
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::ChallengeUnknown => "CHALLENGE_UNKNOWN",
            Self::ChallengeNotYours => "CHALLENGE_NOT_YOURS",
            Self::SameKey => "SAME_KEY",
            Self::SignatureInvalid => "SIGNATURE_INVALID",
        }
    }
}

/// A bind whose signature checked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedBind {
    pub new_key: String,
    pub from_key: String,
}

/// The exact challenge text: seven LF-separated lines, no trailing newline.
/// Callers must have checked [`line_safe`] on every value.
pub fn build_bind_challenge(
    audience: &str,
    identifier: &str,
    human: &str,
    from: &str,
    nonce_hex: &str,
    expires: DateTime<Utc>,
) -> String {
    format!(
        "{BIND_PURPOSE}\naudience={audience}\nidentifier={identifier}\nhuman={human}\nfrom={from}\nnonce={nonce_hex}\nexpires={}",
        rfc3339_utc_secs(expires)
    )
}

/// A field value that cannot break the line structure.
pub fn line_safe(value: &str) -> bool {
    !value.contains('\n') && !value.contains('\r')
}

/// Parse a v1 challenge: exactly seven lines, in order, purpose first, at most
/// 1 KiB, no CR.
pub fn parse_bind_challenge(text: &str) -> Result<BindChallenge, ParseRefusal> {
    if text.len() > MAX_CHALLENGE_BYTES {
        return Err(ParseRefusal::TooLarge);
    }
    if text.contains('\r') {
        return Err(ParseRefusal::CarriageReturn);
    }
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() != 7 {
        return Err(ParseRefusal::WrongLineCount);
    }
    if lines[0] != BIND_PURPOSE {
        return Err(ParseRefusal::WrongPurpose);
    }
    let field = |i: usize, key: &'static str| -> Result<String, ParseRefusal> {
        lines[i]
            .strip_prefix(key)
            .and_then(|r| r.strip_prefix('='))
            .map(str::to_string)
            .ok_or(ParseRefusal::WrongField(key))
    };
    let audience = field(1, "audience")?;
    let identifier = field(2, "identifier")?;
    let human = field(3, "human")?;
    let from = field(4, "from")?;
    let nonce = field(5, "nonce")?;
    let expires_raw = field(6, "expires")?;
    if nonce.len() != 64
        || !nonce
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(ParseRefusal::BadNonce);
    }
    let expires = DateTime::parse_from_rfc3339(&expires_raw)
        .map_err(|_| ParseRefusal::BadExpires)?
        .with_timezone(&Utc);
    if rfc3339_utc_secs(expires) != expires_raw {
        return Err(ParseRefusal::BadExpires);
    }
    Ok(BindChallenge {
        audience,
        identifier,
        human,
        from,
        nonce,
        expires,
    })
}

/// The bind's whole decision: is this signature, by this key, over this stored
/// challenge, from the account that was issued it, still good?
pub fn node_key_bind_verdict(
    stored: &StoredChallenge,
    caller_identifier: &str,
    now: DateTime<Utc>,
    agent_pub_key: &str,
    signature: &str,
) -> Result<VerifiedBind, BindRefusal> {
    if now >= stored.expires {
        return Err(BindRefusal::ChallengeUnknown);
    }
    if stored.identifier != caller_identifier {
        return Err(BindRefusal::ChallengeNotYours);
    }
    // The claimed key must be a VALID agent key: `AgentPubKey::try_from` checks
    // the uhCAk prefix and the 4-byte location checksum, which a bare
    // 39-byte length check does not. A key with altered prefix or location
    // bytes would otherwise keep the same signing bytes and be stored as an
    // unusable identity.
    let claimed_key = holo_hash::AgentPubKey::try_from(
        crate::conductor::agent_key::normalize_agent_key(agent_pub_key).as_str(),
    )
    .map_err(|_| BindRefusal::SignatureInvalid)?;
    let same = match holo_hash::AgentPubKey::try_from(
        crate::conductor::agent_key::normalize_agent_key(&stored.from_key).as_str(),
    ) {
        Ok(from) => from == claimed_key,
        Err(_) => agent_pub_key == stored.from_key,
    };
    if same {
        return Err(BindRefusal::SameKey);
    }
    let core: [u8; 32] = claimed_key
        .get_raw_32()
        .try_into()
        .map_err(|_| BindRefusal::SignatureInvalid)?;
    let verifying = ed25519_dalek::VerifyingKey::from_bytes(&core)
        .map_err(|_| BindRefusal::SignatureInvalid)?;
    let sig_bytes = BASE64
        .decode(signature)
        .map_err(|_| BindRefusal::SignatureInvalid)?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| BindRefusal::SignatureInvalid)?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);
    verifying
        .verify(stored.challenge_text.as_bytes(), &sig)
        .map_err(|_| BindRefusal::SignatureInvalid)?;
    Ok(VerifiedBind {
        new_key: claimed_key.to_string(),
        from_key: stored.from_key.clone(),
    })
}

// =============================================================================
// Challenge store (process-local, single-use, 5 minutes)
// =============================================================================

fn challenge_store() -> &'static Arc<RwLock<HashMap<String, StoredChallenge>>> {
    static STORE: OnceLock<Arc<RwLock<HashMap<String, StoredChallenge>>>> = OnceLock::new();
    STORE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

// =============================================================================
// HTTP
// =============================================================================

#[derive(Debug, Deserialize)]
struct ChallengeRequest {
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeResponse {
    challenge_id: String,
    challenge: String,
    expires_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BindRequest {
    challenge_id: String,
    agent_pub_key: String,
    signature: String,
}

fn refuse(status: StatusCode, code: &str, error: &str) -> Response<BoxBody> {
    json_response(
        status,
        &ErrorResponse {
            error: error.into(),
            code: Some(code.into()),
        },
    )
}

#[allow(clippy::result_large_err)]
fn verify_session(
    auth_header: Option<&str>,
    state: &AppState,
) -> Result<Claims, Response<BoxBody>> {
    let Some(token) = extract_token_from_header(auth_header) else {
        return Err(refuse(
            StatusCode::UNAUTHORIZED,
            "NO_TOKEN",
            "No token provided",
        ));
    };
    let jwt = get_jwt_validator(state)?;
    let result = jwt.verify_token(token);
    match (result.valid, result.claims) {
        (true, Some(c)) => Ok(c),
        _ => Err(refuse(
            StatusCode::UNAUTHORIZED,
            "INVALID_TOKEN",
            &result.error.unwrap_or_else(|| "Invalid token".into()),
        )),
    }
}

/// Whether the account is still in the hosted phase and has not graduated.
pub fn may_graduate(user: &UserDoc) -> bool {
    user.may_be_hosted() && !user.is_steward
}

/// POST /auth/node-key/challenge
pub async fn handle_challenge(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    let auth_header = get_auth_header(&req).map(str::to_string);
    let claims = match verify_session(auth_header.as_deref(), &state) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let body: ChallengeRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return refuse(
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                &format!("Invalid JSON body: {e}"),
            )
        }
    };
    let Some(mongo) = &state.mongo else {
        return refuse(
            StatusCode::SERVICE_UNAVAILABLE,
            "DB_UNAVAILABLE",
            "Database not available",
        );
    };
    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return refuse(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Database error: {e}"),
            )
        }
    };
    let user = match collection
        .find_one(active_user_filter(&claims.identifier))
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return refuse(StatusCode::NOT_FOUND, "USER_NOT_FOUND", "User not found");
        }
        Err(e) => {
            return refuse(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Database error: {e}"),
            )
        }
    };

    match verify_password(&body.password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => {
            warn!(identifier = %claims.identifier, "node-key challenge: wrong password");
            return refuse(StatusCode::FORBIDDEN, "WRONG_PASSWORD", "Wrong password");
        }
        Err(e) => {
            warn!("node-key challenge: password verification error: {}", e);
            return refuse(
                StatusCode::INTERNAL_SERVER_ERROR,
                "AUTH_ERROR",
                "Authentication error",
            );
        }
    }
    if !may_graduate(&user) {
        return refuse(
            StatusCode::CONFLICT,
            "NOT_HOSTED",
            "This account is not in the hosted phase (already graduated, or never hosted)",
        );
    }

    let audience = state
        .args
        .doorway_id
        .clone()
        .or_else(|| gateway_domain(state.args.doorway_url.as_deref()))
        .unwrap_or_else(|| "local".to_string());
    for v in [
        &audience,
        &user.identifier,
        &user.human_id,
        &user.agent_pub_key,
    ] {
        if !line_safe(v) {
            return refuse(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CHALLENGE_UNBUILDABLE",
                "Account field cannot be placed in a challenge",
            );
        }
    }

    let mut nonce_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = hex::encode(nonce_bytes);
    let expires = Utc::now() + Duration::seconds(CHALLENGE_TTL_SECS);
    let text = build_bind_challenge(
        &audience,
        &user.identifier,
        &user.human_id,
        &user.agent_pub_key,
        &nonce,
        expires,
    );
    {
        let mut store = challenge_store().write().await;
        let now = Utc::now();
        store.retain(|_, c| c.expires > now);
        store.insert(
            nonce.clone(),
            StoredChallenge {
                identifier: user.identifier.clone(),
                challenge_text: text.clone(),
                from_key: user.agent_pub_key.clone(),
                expires,
            },
        );
    }
    json_response(
        StatusCode::OK,
        &ChallengeResponse {
            challenge_id: nonce,
            challenge: text,
            expires_at: rfc3339_utc_secs(expires),
        },
    )
}

/// POST /auth/node-key/bind
pub async fn handle_bind(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<BoxBody> {
    let auth_header = get_auth_header(&req).map(str::to_string);
    let claims = match verify_session(auth_header.as_deref(), &state) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let body: BindRequest = match parse_json_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return refuse(
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                &format!("Invalid JSON body: {e}"),
            )
        }
    };

    // Single use: the challenge leaves the store before anything is judged.
    let stored = challenge_store().write().await.remove(&body.challenge_id);
    let Some(stored) = stored else {
        let r = BindRefusal::ChallengeUnknown;
        return refuse(r.status(), r.code(), "Challenge unknown, spent or expired");
    };
    let verified = match node_key_bind_verdict(
        &stored,
        &claims.identifier,
        Utc::now(),
        &body.agent_pub_key,
        &body.signature,
    ) {
        Ok(v) => v,
        Err(r) => {
            warn!(identifier = %claims.identifier, code = r.code(), "node-key bind refused");
            return refuse(r.status(), r.code(), r.code());
        }
    };

    let Some(mongo) = &state.mongo else {
        return refuse(
            StatusCode::SERVICE_UNAVAILABLE,
            "DB_UNAVAILABLE",
            "Database not available",
        );
    };
    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return refuse(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Database error: {e}"),
            )
        }
    };
    let user = match collection
        .find_one(doc! { "identifier": &claims.identifier })
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => return refuse(StatusCode::NOT_FOUND, "USER_NOT_FOUND", "User not found"),
        Err(e) => {
            return refuse(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Database error: {e}"),
            )
        }
    };

    let now = bson::DateTime::now();
    let update = doc! {
        "$set": {
            "agent_pub_key": &verified.new_key,
            "agency_phase": "node",
            "conductor_id": bson::Bson::Null,
            "node_key_bound_at": now,
            "previous_agent_pub_key": &verified.from_key,
            "is_steward": true,
            "stewardship_at": now,
        },
        "$inc": { "token_version": 1 },
    };
    match collection
        .update_one(
            doc! { "identifier": &claims.identifier, "agent_pub_key": &verified.from_key },
            update,
        )
        .await
    {
        Ok(r) if r.matched_count == 0 => {
            return refuse(
                StatusCode::CONFLICT,
                "KEY_MOVED",
                "The account's key changed while binding",
            )
        }
        Ok(_) => {}
        Err(e) => {
            return refuse(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to update user: {e}"),
            )
        }
    }

    // Best effort, after the swap: retire the hosted cell and the custodial keys.
    if let Some(ref registry) = state.conductor_registry {
        let provisioner = AgentProvisioner::new(Arc::clone(registry))
            .with_app_id(state.args.installed_app_id.clone())
            .with_bundle_path(state.args.happ_bundle_path.clone());
        match provisioner.deprovision_agent(&verified.from_key).await {
            Ok(()) => {
                info!(agent = %verified.from_key, "Hosted cell deprovisioned at node-key bind")
            }
            Err(e) => warn!(
                agent = %verified.from_key,
                error = %e,
                "Failed to deprovision hosted cell at node-key bind (cleaned up later)"
            ),
        }
    }
    CustodialKeyService::new().deactivate_all(&user.human_id);

    info!(
        identifier = %claims.identifier,
        bound_at = %account_stamp(now),
        "Account graduated onto its own node key"
    );

    let jwt = match get_jwt_validator(&state) {
        Ok(j) => j,
        Err(r) => return r,
    };
    generate_auth_response(
        &jwt,
        &state,
        &user.human_id,
        &verified.new_key,
        &user.identifier,
        None,
        StatusCode::OK,
        None,
        user.permission_level,
        None,
        None,
        true,
        false,
        None,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn key_for(seed: u8) -> (SigningKey, String) {
        let sk = SigningKey::from_bytes(&[seed; 32]);
        let raw = holo_hash::AgentPubKey::from_raw_32(sk.verifying_key().to_bytes().to_vec())
            .get_raw_39()
            .to_vec();
        (sk, crate::conductor::agent_key::canonical_agent_key(&raw))
    }

    fn expires_in(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(Utc::now().timestamp() + secs, 0).unwrap()
    }

    fn nonce() -> String {
        "ab".repeat(32)
    }

    fn challenge_for(from: &str, expires: DateTime<Utc>) -> String {
        build_bind_challenge(
            "alpha.elohim.host",
            "ann",
            "human-ann",
            from,
            &nonce(),
            expires,
        )
    }

    fn stored(from: &str, expires: DateTime<Utc>) -> StoredChallenge {
        StoredChallenge {
            identifier: "ann".into(),
            challenge_text: challenge_for(from, expires),
            from_key: from.into(),
            expires,
        }
    }

    fn sign(sk: &SigningKey, text: &str) -> String {
        BASE64.encode(sk.sign(text.as_bytes()).to_bytes())
    }

    #[test]
    fn challenge_round_trips_through_build_and_parse() {
        let (_, from) = key_for(1);
        let exp = expires_in(300);
        let parsed = parse_bind_challenge(&challenge_for(&from, exp)).expect("parses");
        assert_eq!(parsed.audience, "alpha.elohim.host");
        assert_eq!(parsed.identifier, "ann");
        assert_eq!(parsed.human, "human-ann");
        assert_eq!(parsed.from, from);
        assert_eq!(parsed.nonce, nonce());
        assert_eq!(parsed.expires, exp);
    }

    #[test]
    fn parse_refuses_wrong_order_extra_line_oversize_and_cr() {
        let (_, from) = key_for(1);
        let good = challenge_for(&from, expires_in(300));
        let lines: Vec<&str> = good.split('\n').collect();

        let mut swapped = lines.clone();
        swapped.swap(2, 3);
        assert_eq!(
            parse_bind_challenge(&swapped.join("\n")),
            Err(ParseRefusal::WrongField("identifier"))
        );

        assert_eq!(
            parse_bind_challenge(&format!("{good}\nextra=1")),
            Err(ParseRefusal::WrongLineCount)
        );
        assert_eq!(
            parse_bind_challenge(&format!("{good}\n")),
            Err(ParseRefusal::WrongLineCount)
        );

        let mut bad_purpose = lines.clone();
        bad_purpose[0] = "elohim:something-else:v1";
        assert_eq!(
            parse_bind_challenge(&bad_purpose.join("\n")),
            Err(ParseRefusal::WrongPurpose)
        );

        let big = good.replace("alpha.elohim.host", &"x".repeat(MAX_CHALLENGE_BYTES));
        assert_eq!(parse_bind_challenge(&big), Err(ParseRefusal::TooLarge));

        assert_eq!(
            parse_bind_challenge(&good.replace('\n', "\r\n")),
            Err(ParseRefusal::CarriageReturn)
        );

        assert_eq!(
            parse_bind_challenge(&good.replace(&nonce(), "AB")),
            Err(ParseRefusal::BadNonce)
        );
    }

    #[test]
    fn a_valid_signature_from_the_claimed_key_is_accepted() {
        let (_, from) = key_for(1);
        let (sk, new_key) = key_for(2);
        let st = stored(&from, expires_in(300));
        let sig = sign(&sk, &st.challenge_text);
        let v = node_key_bind_verdict(&st, "ann", Utc::now(), &new_key, &sig).expect("accepted");
        assert_eq!(v.new_key, new_key);
        assert_eq!(v.from_key, from);
    }

    #[test]
    fn a_signature_by_a_different_key_is_refused() {
        let (_, from) = key_for(1);
        let (_, claimed) = key_for(2);
        let (other, _) = key_for(3);
        let st = stored(&from, expires_in(300));
        let sig = sign(&other, &st.challenge_text);
        assert_eq!(
            node_key_bind_verdict(&st, "ann", Utc::now(), &claimed, &sig),
            Err(BindRefusal::SignatureInvalid)
        );
    }

    #[test]
    fn a_signature_over_tampered_text_is_refused() {
        let (_, from) = key_for(1);
        let (sk, new_key) = key_for(2);
        let st = stored(&from, expires_in(300));
        let sig = sign(&sk, &st.challenge_text.replace("ann", "bob"));
        assert_eq!(
            node_key_bind_verdict(&st, "ann", Utc::now(), &new_key, &sig),
            Err(BindRefusal::SignatureInvalid)
        );
    }

    #[test]
    fn malformed_key_or_signature_is_refused() {
        let (_, from) = key_for(1);
        let (sk, new_key) = key_for(2);
        let st = stored(&from, expires_in(300));
        let sig = sign(&sk, &st.challenge_text);
        assert_eq!(
            node_key_bind_verdict(&st, "ann", Utc::now(), "uhCAk-not-a-key", &sig),
            Err(BindRefusal::SignatureInvalid)
        );
        assert_eq!(
            node_key_bind_verdict(&st, "ann", Utc::now(), &new_key, "AAAA"),
            Err(BindRefusal::SignatureInvalid)
        );
    }

    /// The raw 39 bytes of a real key, for building deliberately broken ones.
    fn raw39(seed: u8) -> Vec<u8> {
        let (sk, _) = key_for(seed);
        holo_hash::AgentPubKey::from_raw_32(sk.verifying_key().to_bytes().to_vec())
            .get_raw_39()
            .to_vec()
    }

    fn refused_with_signature_by(
        sk: &SigningKey,
        claimed: &str,
    ) -> Result<VerifiedBind, BindRefusal> {
        let (_, from) = key_for(1);
        let st = stored(&from, expires_in(300));
        let sig = sign(sk, &st.challenge_text);
        node_key_bind_verdict(&st, "ann", Utc::now(), claimed, &sig)
    }

    #[test]
    fn a_key_with_the_wrong_prefix_bytes_is_refused() {
        let (sk, _) = key_for(2);
        let mut raw = raw39(2);
        raw[0] ^= 0xff;
        let bad = crate::conductor::agent_key::canonical_agent_key(&raw);
        assert_eq!(
            refused_with_signature_by(&sk, &bad),
            Err(BindRefusal::SignatureInvalid)
        );
    }

    #[test]
    fn a_key_with_altered_location_bytes_is_refused() {
        let (sk, _) = key_for(2);
        let mut raw = raw39(2);
        raw[38] ^= 0xff;
        let bad = crate::conductor::agent_key::canonical_agent_key(&raw);
        assert_eq!(
            refused_with_signature_by(&sk, &bad),
            Err(BindRefusal::SignatureInvalid)
        );
    }

    #[test]
    fn the_current_key_with_altered_location_is_refused_not_stored() {
        // Same signing core as `from`, different location: not SAME_KEY by
        // bytes, and not a valid key either, so it must never reach the row.
        let (sk, _) = key_for(1);
        let mut raw = raw39(1);
        raw[36] ^= 0xff;
        let bad = crate::conductor::agent_key::canonical_agent_key(&raw);
        assert_eq!(
            refused_with_signature_by(&sk, &bad),
            Err(BindRefusal::SignatureInvalid)
        );
    }

    #[test]
    fn an_expired_challenge_is_unknown() {
        let (_, from) = key_for(1);
        let (sk, new_key) = key_for(2);
        let st = stored(&from, expires_in(-1));
        let sig = sign(&sk, &st.challenge_text);
        assert_eq!(
            node_key_bind_verdict(&st, "ann", Utc::now(), &new_key, &sig),
            Err(BindRefusal::ChallengeUnknown)
        );
    }

    #[test]
    fn another_accounts_challenge_is_not_yours() {
        let (_, from) = key_for(1);
        let (sk, new_key) = key_for(2);
        let st = stored(&from, expires_in(300));
        let sig = sign(&sk, &st.challenge_text);
        assert_eq!(
            node_key_bind_verdict(&st, "mallory", Utc::now(), &new_key, &sig),
            Err(BindRefusal::ChallengeNotYours)
        );
    }

    #[test]
    fn binding_the_same_key_is_refused() {
        let (sk, from) = key_for(1);
        let st = stored(&from, expires_in(300));
        let sig = sign(&sk, &st.challenge_text);
        assert_eq!(
            node_key_bind_verdict(&st, "ann", Utc::now(), &from, &sig),
            Err(BindRefusal::SameKey)
        );
    }

    #[test]
    fn refusal_codes_and_statuses_are_the_named_ones() {
        assert_eq!(
            BindRefusal::ChallengeUnknown.status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(BindRefusal::ChallengeUnknown.code(), "CHALLENGE_UNKNOWN");
        assert_eq!(
            BindRefusal::ChallengeNotYours.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(BindRefusal::SameKey.status(), StatusCode::CONFLICT);
        assert_eq!(BindRefusal::SignatureInvalid.code(), "SIGNATURE_INVALID");
    }

    #[test]
    fn only_a_hosted_unstewarded_account_may_graduate() {
        let mk = |phase: Option<&str>, steward: bool| {
            let mut u = UserDoc::new(
                "ann".into(),
                "username".into(),
                "h".into(),
                "human-ann".into(),
                "uhCAkx".into(),
                None,
            );
            u.agency_phase = phase.map(str::to_string);
            u.is_steward = steward;
            u
        };
        assert!(may_graduate(&mk(Some("hosted"), false)));
        assert!(!may_graduate(&mk(Some("hosted"), true)));
        assert!(!may_graduate(&mk(Some("node"), false)));
        assert!(!may_graduate(&mk(None, false)));
    }
}
