//! Account-bind signing — the node's half of "graduating onto your own node's
//! key" (spec: genesis/docs/superpowers/specs/2026-10-16-graduate-onto-own-node-key-design.md).
//!
//! `POST /api/v1/identity/account-bind/sign {challenge}` makes this node's agent
//! key sign ONE purpose-bound text a doorway issued. This module owns the pure
//! decisions (what text is a bind challenge, who may ask) and the zome call;
//! the HTTP handler in `http.rs` only reads the request and renders the answer.
//!
//! The route is never a general signing oracle: the text must parse as the seven
//! lines of the v1 wire contract and be unexpired, and the zome re-checks the
//! purpose line itself (`imagodei::sign_account_bind_challenge`).

use base64::Engine;
use chrono::{DateTime, NaiveDateTime, Utc};
use holochain_types::prelude::{AgentPubKey, ExternIO};
use hyper::StatusCode;

use crate::hc_client::HcClient;
use crate::services::node_account::BoundVerdict;

/// First line of every challenge.
pub const PURPOSE_LINE: &str = "elohim:doorway-account-bind:v1";
/// The challenge is at most this many bytes.
pub const MAX_CHALLENGE_BYTES: usize = 1024;
/// The longest a challenge may live ahead of now (the doorway issues 5 minutes).
pub const MAX_LIFETIME_SECS: i64 = 600;

/// A named refusal: HTTP status plus a stable code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub status: StatusCode,
    pub code: &'static str,
    pub words: String,
}

impl Refusal {
    fn new(status: StatusCode, code: &'static str, words: impl Into<String>) -> Self {
        Self {
            status,
            code,
            words: words.into(),
        }
    }
}

/// A parsed bind challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountBindChallenge {
    pub audience: String,
    pub identifier: String,
    pub human: String,
    pub from: String,
    pub nonce: String,
    pub expires: DateTime<Utc>,
}

fn not_a_challenge(why: &str) -> Refusal {
    Refusal::new(
        StatusCode::BAD_REQUEST,
        "NOT_A_BIND_CHALLENGE",
        format!("this is not an account-bind challenge: {why}"),
    )
}

/// RFC3339 UTC with a `Z` and seconds precision, e.g. `2026-10-16T20:15:00Z`.
fn parse_expires(v: &str) -> Option<DateTime<Utc>> {
    if v.len() != 20 || !v.ends_with('Z') {
        return None;
    }
    NaiveDateTime::parse_from_str(&v[..19], "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|n| n.and_utc())
}

/// Parse the challenge text: exactly the seven v1 lines, in order, at most 1 KiB,
/// no CR; refuse an expired one. Pure.
pub fn account_bind_challenge_parse(
    text: &str,
    now: DateTime<Utc>,
) -> Result<AccountBindChallenge, Refusal> {
    if text.len() > MAX_CHALLENGE_BYTES {
        return Err(not_a_challenge("over 1024 bytes"));
    }
    if text.contains('\r') {
        return Err(not_a_challenge("contains a carriage return"));
    }
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() != 7 {
        return Err(not_a_challenge("expected exactly 7 lines"));
    }
    if lines[0] != PURPOSE_LINE {
        return Err(not_a_challenge("first line is not the purpose line"));
    }
    let field = |i: usize, key: &str| -> Result<String, Refusal> {
        lines[i]
            .strip_prefix(key)
            .and_then(|r| r.strip_prefix('='))
            .map(str::to_string)
            .ok_or_else(|| not_a_challenge(&format!("line {} is not `{key}=…`", i + 1)))
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
        return Err(not_a_challenge("nonce is not 64 lowercase hex characters"));
    }
    let expires = parse_expires(&expires_raw)
        .ok_or_else(|| not_a_challenge("expires is not RFC3339 UTC seconds (…Z)"))?;
    if expires <= now {
        return Err(Refusal::new(
            StatusCode::BAD_REQUEST,
            "CHALLENGE_EXPIRED",
            "this challenge has expired; ask the doorway for a new one",
        ));
    }
    if expires > now + chrono::Duration::seconds(MAX_LIFETIME_SECS) {
        return Err(not_a_challenge("expires too far ahead"));
    }
    Ok(AccountBindChallenge {
        audience,
        identifier,
        human,
        from,
        nonce,
        expires,
    })
}

/// Who may ask the node to sign: only a proven, key-bound session with a valid
/// proof. `None` = go on. Pure. No loopback bypass exists on this route.
pub fn bound_verdict_refusal(verdict: &BoundVerdict) -> Option<Refusal> {
    match verdict {
        BoundVerdict::ProvenBound => None,
        BoundVerdict::NoSession => Some(Refusal::new(
            StatusCode::UNAUTHORIZED,
            "NO_SESSION",
            "sign in to this node first",
        )),
        BoundVerdict::NotBound => Some(Refusal::new(
            StatusCode::FORBIDDEN,
            "SESSION_NOT_BOUND",
            "this sign-in is not bound to a key of your browser; sign in again from the node's own portal",
        )),
        BoundVerdict::ProofRefused(r) => Some(Refusal::new(
            StatusCode::FORBIDDEN,
            "PROOF_REFUSED",
            r.words(),
        )),
    }
}

/// The zome's answer, as the cell encodes it.
#[derive(Debug, serde::Deserialize)]
struct SignedAccountBindWire {
    signature: Vec<u8>,
    signer: AgentPubKey,
}

/// A signature the node made, ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed {
    pub agent_pub_key: String,
    pub signature_b64: String,
}

/// Check, pure: the signature is 64 bytes, verifies over `challenge` under the
/// signer's key, and the signer is this node's own agent key.
pub fn check_signed(
    challenge: &str,
    signature: &[u8],
    signer: &AgentPubKey,
    own_key_uhcak: &str,
) -> Result<Signed, Refusal> {
    use ed25519_dalek::{Signature, VerifyingKey};
    let signer_str = signer.to_string();
    if signer_str != own_key_uhcak {
        return Err(Refusal::new(
            StatusCode::BAD_GATEWAY,
            "SIGNER_MISMATCH",
            "the cell signed with a key other than this node's own",
        ));
    }
    let sig: [u8; 64] = signature.try_into().map_err(|_| {
        Refusal::new(
            StatusCode::BAD_GATEWAY,
            "SIGN_FAILED",
            "this node could not sign the challenge",
        )
    })?;
    let key: [u8; 32] = signer.get_raw_32().try_into().map_err(|_| {
        Refusal::new(
            StatusCode::BAD_GATEWAY,
            "SIGN_FAILED",
            "this node could not sign the challenge",
        )
    })?;
    let ok = VerifyingKey::from_bytes(&key)
        .ok()
        .and_then(|k| {
            k.verify_strict(challenge.as_bytes(), &Signature::from_bytes(&sig))
                .ok()
        })
        .is_some();
    if !ok {
        return Err(Refusal::new(
            StatusCode::BAD_GATEWAY,
            "SIGN_FAILED",
            "this node could not sign the challenge",
        ));
    }
    Ok(Signed {
        agent_pub_key: signer_str,
        signature_b64: base64::engine::general_purpose::STANDARD.encode(sig),
    })
}

/// Ask the imagodei cell to sign `challenge`. The caller has already parsed it
/// (the work is bounded before this uncancellable conductor call is made).
/// Conductor and decode detail is logged here and never reaches the client.
pub async fn sign_with_cell(hc: &HcClient, challenge: &str) -> Result<Signed, Refusal> {
    let fail = |detail: String| {
        tracing::warn!(detail, "account-bind sign: cell call failed");
        Refusal::new(
            StatusCode::BAD_GATEWAY,
            "SIGN_FAILED",
            "this node could not sign the challenge",
        )
    };
    let payload = ExternIO::encode(challenge.to_string())
        .map_err(|e| fail(format!("challenge encoding: {e}")))?
        .into_vec();
    let answer = hc
        .call_zome_imagodei("imagodei", "sign_account_bind_challenge", payload)
        .await
        .map_err(|e| fail(format!("sign_account_bind_challenge: {e}")))?;
    let wire: SignedAccountBindWire = ExternIO::from(answer)
        .decode()
        .map_err(|e| fail(format!("answer decode: {e}")))?;
    check_signed(
        challenge,
        &wire.signature,
        &wire.signer,
        &hc.agent_key_uhcak(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use consent_grant::ProofRefusal;

    const NONCE: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn now() -> DateTime<Utc> {
        "2026-10-16T20:00:00Z".parse().unwrap()
    }
    fn good() -> String {
        format!(
            "{PURPOSE_LINE}\naudience=doorway.example\nidentifier=alice\nhuman=h1\nfrom=uhCAkabc\nnonce={NONCE}\nexpires=2026-10-16T20:05:00Z"
        )
    }
    fn code(t: &str) -> &'static str {
        account_bind_challenge_parse(t, now()).unwrap_err().code
    }

    #[test]
    fn a_well_formed_unexpired_challenge_parses() {
        let c = account_bind_challenge_parse(&good(), now()).unwrap();
        assert_eq!(c.identifier, "alice");
        assert_eq!(c.nonce, NONCE);
    }
    #[test]
    fn wrong_purpose_line_is_not_a_bind_challenge() {
        assert_eq!(code(&good().replace("v1", "v2")), "NOT_A_BIND_CHALLENGE");
    }
    #[test]
    fn wrong_line_count_is_not_a_bind_challenge() {
        assert_eq!(
            code(&format!("{}\nextra=1", good())),
            "NOT_A_BIND_CHALLENGE"
        );
        assert_eq!(code(&format!("{}\n", good())), "NOT_A_BIND_CHALLENGE");
        assert_eq!(code(PURPOSE_LINE), "NOT_A_BIND_CHALLENGE");
    }
    #[test]
    fn out_of_order_keys_are_not_a_bind_challenge() {
        let t = good().replace("identifier=alice\nhuman=h1", "human=h1\nidentifier=alice");
        assert_eq!(code(&t), "NOT_A_BIND_CHALLENGE");
    }
    #[test]
    fn carriage_return_is_not_a_bind_challenge() {
        assert_eq!(code(&good().replace('\n', "\r\n")), "NOT_A_BIND_CHALLENGE");
    }
    #[test]
    fn oversize_is_not_a_bind_challenge() {
        let t = good().replace("alice", &"a".repeat(1100));
        assert_eq!(code(&t), "NOT_A_BIND_CHALLENGE");
    }
    #[test]
    fn a_challenge_more_than_ten_minutes_ahead_is_refused() {
        let at = |e: &str| good().replace("2026-10-16T20:05:00Z", e);
        assert!(account_bind_challenge_parse(&at("2026-10-16T20:10:00Z"), now()).is_ok());
        assert_eq!(code(&at("2026-10-16T20:10:01Z")), "NOT_A_BIND_CHALLENGE");
    }
    #[test]
    fn a_nonce_must_be_64_lowercase_hex() {
        assert_eq!(code(&good().replace(NONCE, "ABCD")), "NOT_A_BIND_CHALLENGE");
        assert_eq!(
            code(&good().replace(NONCE, &NONCE.to_uppercase())),
            "NOT_A_BIND_CHALLENGE"
        );
    }
    #[test]
    fn expires_must_be_utc_seconds_with_z() {
        assert_eq!(
            code(&good().replace("2026-10-16T20:05:00Z", "2026-10-16T20:05:00+00:00")),
            "NOT_A_BIND_CHALLENGE"
        );
        assert_eq!(
            code(&good().replace("2026-10-16T20:05:00Z", "2026-10-16T20:05:00.5Z")),
            "NOT_A_BIND_CHALLENGE"
        );
    }
    #[test]
    fn an_expired_challenge_is_refused_as_expired() {
        let r = account_bind_challenge_parse(&good(), "2026-10-16T20:05:00Z".parse().unwrap())
            .unwrap_err();
        assert_eq!(
            (r.status, r.code),
            (StatusCode::BAD_REQUEST, "CHALLENGE_EXPIRED")
        );
    }
    #[test]
    fn only_a_proven_bound_session_goes_on() {
        assert!(bound_verdict_refusal(&BoundVerdict::ProvenBound).is_none());
        let no = bound_verdict_refusal(&BoundVerdict::NoSession).unwrap();
        assert_eq!(
            (no.status, no.code),
            (StatusCode::UNAUTHORIZED, "NO_SESSION")
        );
        let nb = bound_verdict_refusal(&BoundVerdict::NotBound).unwrap();
        assert_eq!(
            (nb.status, nb.code),
            (StatusCode::FORBIDDEN, "SESSION_NOT_BOUND")
        );
        let pr = bound_verdict_refusal(&BoundVerdict::ProofRefused(ProofRefusal::Missing)).unwrap();
        assert_eq!(
            (pr.status, pr.code),
            (StatusCode::FORBIDDEN, "PROOF_REFUSED")
        );
    }

    fn keypair() -> (ed25519_dalek::SigningKey, AgentPubKey) {
        let sk = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let pk = AgentPubKey::from_raw_32(sk.verifying_key().to_bytes().to_vec());
        (sk, pk)
    }
    #[test]
    fn a_good_signature_by_the_nodes_own_key_is_rendered() {
        use ed25519_dalek::Signer;
        let (sk, pk) = keypair();
        let text = good();
        let sig = sk.sign(text.as_bytes()).to_bytes();
        let s = check_signed(&text, &sig, &pk, &pk.to_string()).unwrap();
        assert_eq!(s.agent_pub_key, pk.to_string());
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(s.signature_b64)
                .unwrap(),
            sig.to_vec()
        );
    }
    #[test]
    fn a_foreign_signer_or_bad_signature_is_a_502() {
        use ed25519_dalek::Signer;
        let (sk, pk) = keypair();
        let text = good();
        let sig = sk.sign(text.as_bytes()).to_bytes();
        assert_eq!(
            check_signed(&text, &sig, &pk, "uhCAkother")
                .unwrap_err()
                .code,
            "SIGNER_MISMATCH"
        );
        assert_eq!(
            check_signed("tampered", &sig, &pk, &pk.to_string())
                .unwrap_err()
                .code,
            "SIGN_FAILED"
        );
        assert_eq!(
            check_signed(&text, &sig[..10], &pk, &pk.to_string())
                .unwrap_err()
                .code,
            "SIGN_FAILED"
        );
    }
}
