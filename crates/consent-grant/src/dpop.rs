//! Session keys: a sign-in session bound to a key the browser holds, proven
//! per request (RFC 9449, DPoP, adapted to a cookie-carried session).
//!
//! A session proven by sign-in may be bound to a public key at sign-in. Every
//! request that makes the node sign (and sign-out) must then carry a `DPoP`
//! header: a compact JWS made with that key, so a copied cookie is worth
//! nothing on those routes. The node checks "a public key and a signature
//! under a named algorithm", never one kind of key: each algorithm is one
//! [`AlgVerifier`], so a platform-held key under another algorithm is an
//! addition, not a change.
//!
//! The proof, exactly:
//! - three base64url segments without padding, `header.claims.signature`;
//! - the signed bytes are the ASCII of `header.claims` as sent;
//! - header `{"typ":"dpop+jwt","alg":"ES256"|"EdDSA","jwk":{…public…}}`; no
//!   `crit`, no other algorithm, never `none`, no private key members;
//! - claims `{jti, htm, htu, iat, bsh}`: `jti` a string of 1 to 128
//!   characters, single use; `htm` the request method, exactly; `htu` the
//!   request path; `iat` an integer number of seconds within
//!   [`IAT_WINDOW_SECS`] of the node's clock either way; `bsh` (not in the RFC)
//!   the base64url SHA-256 of the exact request body bytes, the empty body
//!   hashing as empty. Other claims are ignored.
//! - `htu` is compared on its path alone: scheme, host, query and fragment are
//!   ignored, because the node may sit behind a proxy and cannot know its
//!   public origin. `https://n.example/auth/consent/agree?x#y` and
//!   `/auth/consent/agree` both name `/auth/consent/agree`.
//! - ES256 signatures are the 64-byte `r || s` of JWS (what WebCrypto
//!   produces), not DER. EdDSA is Ed25519, verified strictly.
//! - A bound key is compared by its RFC 7638 thumbprint.

use std::collections::HashMap;
use std::sync::Mutex;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

/// How far a proof's `iat` may be from the node's clock, either way.
pub const IAT_WINDOW_SECS: i64 = 60;
/// The longest `jti` accepted.
pub const MAX_JTI_LEN: usize = 128;
/// The longest proof accepted, so parsing stays bounded.
pub const MAX_PROOF_LEN: usize = 4096;
/// How many proofs the replay set holds at once. Over it, a proof is refused:
/// the set never grows past this.
pub const MAX_SEEN: usize = 4096;

/// Why a proof was not accepted. Each answers 401 with plain words that ask
/// the person only to sign in again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofRefusal {
    /// The session is bound to a key and the request carried no proof.
    Missing,
    /// Bad signature, wrong key, wrong method or path, body mismatch, or a
    /// proof that is not one. `why` is for the log, never the person.
    Invalid { why: &'static str },
    /// `iat` is outside the window.
    Stale,
    /// This proof was used before, or the node cannot hold another right now.
    Replayed,
}

impl ProofRefusal {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Missing => "session_proof_missing",
            Self::Invalid { .. } => "session_proof_invalid",
            Self::Stale => "session_proof_stale",
            Self::Replayed => "session_proof_replayed",
        }
    }

    pub fn words(&self) -> &'static str {
        match self {
            Self::Missing => "this needs proof from the browser you signed in with; sign in again",
            Self::Invalid { .. } => {
                "this did not come with a valid proof from the browser you signed in with; sign in \
                 again"
            }
            Self::Stale => {
                "this came with an out-of-date proof; check this device's clock, or sign in again"
            }
            Self::Replayed => "this proof was already used; sign in again",
        }
    }
}

fn invalid(why: &'static str) -> ProofRefusal {
    ProofRefusal::Invalid { why }
}

/// One signature algorithm the node verifies.
pub trait AlgVerifier: Send + Sync {
    /// The JWS `alg` name.
    fn alg(&self) -> &'static str;
    /// The RFC 7638 thumbprint input (the required members, sorted, no
    /// whitespace) for a public JWK of this algorithm's key type, or why not.
    fn thumbprint_input(&self, jwk: &Map<String, Value>) -> Result<String, ProofRefusal>;
    /// Whether `signature` signs `message` under the public JWK.
    fn verify(
        &self,
        jwk: &Map<String, Value>,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), ProofRefusal>;
}

fn member<'a>(jwk: &'a Map<String, Value>, name: &str) -> Result<&'a str, ProofRefusal> {
    jwk.get(name)
        .and_then(Value::as_str)
        .ok_or(invalid("jwk member missing"))
}

fn b64(text: &str) -> Result<Vec<u8>, ProofRefusal> {
    URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| invalid("not base64url"))
}

/// ES256: ECDSA over P-256 with SHA-256.
pub struct Es256;

impl AlgVerifier for Es256 {
    fn alg(&self) -> &'static str {
        "ES256"
    }

    fn thumbprint_input(&self, jwk: &Map<String, Value>) -> Result<String, ProofRefusal> {
        if member(jwk, "kty")? != "EC" || member(jwk, "crv")? != "P-256" {
            return Err(invalid("ES256 needs an EC P-256 key"));
        }
        let (x, y) = (member(jwk, "x")?, member(jwk, "y")?);
        Ok(format!(
            r#"{{"crv":"P-256","kty":"EC","x":{},"y":{}}}"#,
            Value::from(x),
            Value::from(y)
        ))
    }

    fn verify(
        &self,
        jwk: &Map<String, Value>,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), ProofRefusal> {
        use p256::ecdsa::signature::Verifier;
        let (x, y) = (b64(member(jwk, "x")?)?, b64(member(jwk, "y")?)?);
        if x.len() != 32 || y.len() != 32 {
            return Err(invalid("P-256 coordinates are 32 bytes"));
        }
        let mut point = Vec::with_capacity(65);
        point.push(0x04);
        point.extend_from_slice(&x);
        point.extend_from_slice(&y);
        let key = p256::ecdsa::VerifyingKey::from_sec1_bytes(&point)
            .map_err(|_| invalid("not a P-256 point"))?;
        if signature.len() != 64 {
            return Err(invalid("ES256 signatures are 64 bytes"));
        }
        let signature = p256::ecdsa::Signature::from_slice(signature)
            .map_err(|_| invalid("not an ES256 signature"))?;
        key.verify(message, &signature)
            .map_err(|_| invalid("bad signature"))
    }
}

/// EdDSA: Ed25519.
pub struct EdDsa;

impl AlgVerifier for EdDsa {
    fn alg(&self) -> &'static str {
        "EdDSA"
    }

    fn thumbprint_input(&self, jwk: &Map<String, Value>) -> Result<String, ProofRefusal> {
        if member(jwk, "kty")? != "OKP" || member(jwk, "crv")? != "Ed25519" {
            return Err(invalid("EdDSA needs an OKP Ed25519 key"));
        }
        Ok(format!(
            r#"{{"crv":"Ed25519","kty":"OKP","x":{}}}"#,
            Value::from(member(jwk, "x")?)
        ))
    }

    fn verify(
        &self,
        jwk: &Map<String, Value>,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), ProofRefusal> {
        let x: [u8; 32] = b64(member(jwk, "x")?)?
            .try_into()
            .map_err(|_| invalid("Ed25519 keys are 32 bytes"))?;
        let key = ed25519_dalek::VerifyingKey::from_bytes(&x)
            .map_err(|_| invalid("not an Ed25519 key"))?;
        let signature: [u8; 64] = signature
            .try_into()
            .map_err(|_| invalid("Ed25519 signatures are 64 bytes"))?;
        key.verify_strict(message, &ed25519_dalek::Signature::from_bytes(&signature))
            .map_err(|_| invalid("bad signature"))
    }
}

/// The algorithms this node verifies, by name. An addition here is all a new
/// one needs.
pub fn verifier(alg: &str) -> Option<&'static dyn AlgVerifier> {
    static ES256: Es256 = Es256;
    static EDDSA: EdDsa = EdDsa;
    match alg {
        "ES256" => Some(&ES256),
        "EdDSA" => Some(&EDDSA),
        _ => None,
    }
}

/// A public key a session is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundKey {
    pub alg: String,
    /// The public JWK, as given.
    pub jwk: Map<String, Value>,
    /// Its RFC 7638 thumbprint, base64url SHA-256.
    pub thumbprint: String,
}

const PRIVATE_MEMBERS: [&str; 8] = ["d", "p", "q", "dp", "dq", "qi", "oth", "k"];

impl BoundKey {
    /// A key from `{alg, jwk}`: a known algorithm and a public key of its kind.
    pub fn new(alg: &str, jwk: &Value) -> Result<Self, ProofRefusal> {
        let verifier = verifier(alg).ok_or(invalid("algorithm not accepted"))?;
        let jwk = jwk.as_object().ok_or(invalid("jwk is not an object"))?;
        if PRIVATE_MEMBERS.iter().any(|m| jwk.contains_key(*m)) {
            return Err(invalid("jwk carries private key material"));
        }
        let input = verifier.thumbprint_input(jwk)?;
        Ok(Self {
            alg: verifier.alg().to_string(),
            jwk: jwk.clone(),
            thumbprint: URL_SAFE_NO_PAD.encode(Sha256::digest(input.as_bytes())),
        })
    }
}

/// The key a person's browser offers at sign-in: `{alg, jwk}` on the wire.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionKey {
    pub alg: String,
    pub jwk: Value,
}

/// What the host knows of the request a proof is for.
#[derive(Debug, Clone, Copy)]
pub struct ProofFacts<'a> {
    /// The HTTP method, as received (`POST`).
    pub method: &'a str,
    /// The request path, without query.
    pub path: &'a str,
    /// The exact body bytes.
    pub body: &'a [u8],
    pub now_secs: i64,
}

/// A proof, read and its signature checked against the key it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    pub key: BoundKey,
    pub jti: String,
    pub htm: String,
    pub htu: String,
    pub iat: i64,
    pub bsh: String,
}

/// The path an `htu` names: scheme, host, query and fragment dropped.
pub fn htu_path(htu: &str) -> &str {
    let rest = match htu.find("://") {
        Some(i) => {
            let after = &htu[i + 3..];
            match after.find('/') {
                Some(j) => &after[j..],
                None => "/",
            }
        }
        None => htu,
    };
    let end = rest.find(['?', '#']).unwrap_or(rest.len());
    &rest[..end]
}

/// The `bsh` of a body: base64url SHA-256 of its exact bytes.
pub fn body_hash(body: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(body))
}

/// Read a compact JWS proof strictly and check its signature against the key
/// in its own header. Nothing about the request is checked here.
pub fn read_proof(compact: &str) -> Result<Proof, ProofRefusal> {
    if compact.len() > MAX_PROOF_LEN {
        return Err(invalid("proof too long"));
    }
    let mut parts = compact.split('.');
    let (Some(h), Some(c), Some(s), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(invalid("not three segments"));
    };
    let object = |bytes: Vec<u8>| -> Result<Map<String, Value>, ProofRefusal> {
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(Value::Object(m)) => Ok(m),
            _ => Err(invalid("segment is not a JSON object")),
        }
    };
    let header = object(b64(h)?)?;
    let claims = object(b64(c)?)?;
    let signature = b64(s)?;
    if header.get("typ").and_then(Value::as_str) != Some("dpop+jwt") {
        return Err(invalid("typ is not dpop+jwt"));
    }
    if header.contains_key("crit") {
        return Err(invalid("critical headers are not understood"));
    }
    let alg = header
        .get("alg")
        .and_then(Value::as_str)
        .ok_or(invalid("no alg"))?;
    let key = BoundKey::new(alg, header.get("jwk").ok_or(invalid("no jwk"))?)?;
    let message = format!("{h}.{c}");
    verifier(alg)
        .ok_or(invalid("algorithm not accepted"))?
        .verify(&key.jwk, message.as_bytes(), &signature)?;
    let text = |name: &str| {
        claims
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or(invalid("claim missing"))
    };
    let jti = text("jti")?;
    if jti.is_empty() || jti.chars().count() > MAX_JTI_LEN {
        return Err(invalid("jti length"));
    }
    let iat = claims
        .get("iat")
        .and_then(Value::as_i64)
        .ok_or(invalid("iat is not an integer"))?;
    Ok(Proof {
        key,
        jti,
        htm: text("htm")?,
        htu: text("htu")?,
        iat,
        bsh: text("bsh")?,
    })
}

/// Proofs already used, by key thumbprint and `jti`, each kept for twice the
/// `iat` window: a proof outside the window is refused as stale anyway. In
/// memory; at [`MAX_SEEN`] a new proof is refused rather than the set grown.
#[derive(Debug, Default)]
pub struct ReplaySet {
    seen: Mutex<HashMap<(String, String), i64>>,
}

impl ReplaySet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `jti` for `thumbprint` at `now_secs`, or refuse it as seen.
    pub fn first_use(
        &self,
        thumbprint: &str,
        jti: &str,
        now_secs: i64,
    ) -> Result<(), ProofRefusal> {
        let mut seen = self.seen.lock().unwrap_or_else(|e| e.into_inner());
        let key = (thumbprint.to_string(), jti.to_string());
        if seen.contains_key(&key) {
            return Err(ProofRefusal::Replayed);
        }
        if seen.len() >= MAX_SEEN {
            // bounded-work: one sweep over at most MAX_SEEN entries.
            seen.retain(|_, at| now_secs - *at <= 2 * IAT_WINDOW_SECS);
            if seen.len() >= MAX_SEEN {
                return Err(ProofRefusal::Replayed);
            }
        }
        seen.insert(key, now_secs);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.seen.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Check a proof for one request: made by `expected` (when given, compared by
/// thumbprint), for this method, path and body, fresh, and never used before.
/// Returns the key that made it.
pub fn check_proof(
    proof: Option<&str>,
    expected: Option<&BoundKey>,
    facts: &ProofFacts<'_>,
    replay: &ReplaySet,
) -> Result<BoundKey, ProofRefusal> {
    let proof = read_proof(proof.ok_or(ProofRefusal::Missing)?.trim())?;
    if let Some(expected) = expected {
        if proof.key.thumbprint != expected.thumbprint || proof.key.alg != expected.alg {
            return Err(invalid("proof from another key"));
        }
    }
    if proof.htm != facts.method {
        return Err(invalid("wrong method"));
    }
    if htu_path(&proof.htu) != facts.path {
        return Err(invalid("wrong path"));
    }
    if proof.bsh != body_hash(facts.body) {
        return Err(invalid("body mismatch"));
    }
    if (proof.iat - facts.now_secs).abs() > IAT_WINDOW_SECS {
        return Err(ProofRefusal::Stale);
    }
    replay.first_use(&proof.key.thumbprint, &proof.jti, facts.now_secs)?;
    Ok(proof.key)
}

/// The rule for a request to a route that makes the node sign, or sign-out: a
/// session bound to a key must present a valid proof from that key; a session
/// with no bound key is accepted without one, as before binding existed.
pub fn session_proof_verdict(
    bound: Option<&BoundKey>,
    proof: Option<&str>,
    facts: &ProofFacts<'_>,
    replay: &ReplaySet,
) -> Result<(), ProofRefusal> {
    match bound {
        None => Ok(()),
        Some(key) => check_proof(proof, Some(key), facts, replay).map(|_| ()),
    }
}

/// Binding at sign-in: the offered key, proven by a proof it made for the
/// sign-in request itself.
pub fn bind_session_key(
    offered: &SessionKey,
    proof: Option<&str>,
    facts: &ProofFacts<'_>,
    replay: &ReplaySet,
) -> Result<BoundKey, ProofRefusal> {
    let key = BoundKey::new(&offered.alg, &offered.jwk)?;
    check_proof(proof, Some(&key), facts, replay)?;
    Ok(key)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The worked example of the report and the portal: Ed25519 with the seed
    /// 0x01 repeated (deterministic signatures), for `POST /auth/consent/agree`
    /// with body `{"x":1}` at iat 1759700000, jti "jti-1".
    const EXAMPLE_HEADER: &str = r#"{"typ":"dpop+jwt","alg":"EdDSA","jwk":{"kty":"OKP","crv":"Ed25519","x":"iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w"}}"#;

    /// The compact JWS those bytes sign to (Ed25519 is deterministic).
    const EXAMPLE_JWS: &str = "eyJ0eXAiOiJkcG9wK2p3dCIsImFsZyI6IkVkRFNBIiwiandrIjp7Imt0eSI6Ik9LUCIsImNydiI6IkVkMjU1MTkiLCJ4IjoiaW9qajNYUUo4Wlg5VXRzdFBMcGRjc3BuQ2I4ZGxCSWI4M1NJQWJRUGIxdyJ9fQ.eyJqdGkiOiJqdGktMSIsImh0bSI6IlBPU1QiLCJodHUiOiIvYXV0aC9jb25zZW50L2FncmVlIiwiaWF0IjoxNzU5NzAwMDAwLCJic2giOiJVRUdfSDNFOThnUjRRMVBvTDJwS1Uxa3h5MlR4OUxTbHJxXzh0eUNSaXlJIn0.ZDsvfVw2UGDpG9gCB3gncqBHr0uXGa_UXTMyCVBjKV9bsyvcXZtQUBg0hnepDc4zWB0h7q7u9SKgkJG7_YJrCg";

    fn ed_key(seed: u8) -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
    }

    fn ed_jwk(key: &ed25519_dalek::SigningKey) -> Value {
        serde_json::json!({
            "kty": "OKP", "crv": "Ed25519",
            "x": URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes()),
        })
    }

    fn p256_key(seed: u8) -> p256::ecdsa::SigningKey {
        p256::ecdsa::SigningKey::from_bytes(&[seed; 32].into()).unwrap()
    }

    fn p256_jwk(key: &p256::ecdsa::SigningKey) -> Value {
        let point = key.verifying_key().to_encoded_point(false);
        serde_json::json!({
            "kty": "EC", "crv": "P-256",
            "x": URL_SAFE_NO_PAD.encode(point.x().unwrap()),
            "y": URL_SAFE_NO_PAD.encode(point.y().unwrap()),
            "ext": true, "key_ops": ["verify"],
        })
    }

    pub(crate) enum Signer {
        Ed(ed25519_dalek::SigningKey),
        P256(p256::ecdsa::SigningKey),
    }

    impl Signer {
        pub(crate) fn ed(seed: u8) -> Self {
            Self::Ed(ed_key(seed))
        }
        pub(crate) fn p256(seed: u8) -> Self {
            Self::P256(p256_key(seed))
        }
        pub(crate) fn session_key(&self) -> SessionKey {
            match self {
                Self::Ed(k) => SessionKey {
                    alg: "EdDSA".into(),
                    jwk: ed_jwk(k),
                },
                Self::P256(k) => SessionKey {
                    alg: "ES256".into(),
                    jwk: p256_jwk(k),
                },
            }
        }
        pub(crate) fn proof_with(&self, header: Value, claims: Value) -> String {
            let h = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
            let c = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
            let input = format!("{h}.{c}");
            let sig = match self {
                Self::Ed(k) => {
                    use ed25519_dalek::Signer as _;
                    k.sign(input.as_bytes()).to_bytes().to_vec()
                }
                Self::P256(k) => {
                    use p256::ecdsa::signature::Signer as _;
                    let s: p256::ecdsa::Signature = k.sign(input.as_bytes());
                    s.to_bytes().to_vec()
                }
            };
            format!("{input}.{}", URL_SAFE_NO_PAD.encode(sig))
        }
        pub(crate) fn proof(
            &self,
            method: &str,
            htu: &str,
            body: &[u8],
            iat: i64,
            jti: &str,
        ) -> String {
            let key = self.session_key();
            self.proof_with(
                serde_json::json!({"typ": "dpop+jwt", "alg": key.alg, "jwk": key.jwk}),
                serde_json::json!({
                    "jti": jti, "htm": method, "htu": htu, "iat": iat, "bsh": body_hash(body),
                }),
            )
        }
    }

    const NOW: i64 = 1_759_700_000;

    fn facts<'a>(body: &'a [u8]) -> ProofFacts<'a> {
        ProofFacts {
            method: "POST",
            path: "/auth/consent/agree",
            body,
            now_secs: NOW,
        }
    }

    #[test]
    fn the_worked_example_verifies_byte_for_byte() {
        let signer = Signer::ed(1);
        // The header is any JSON; these are the bytes the portal example sends.
        let header: Value = serde_json::from_str(EXAMPLE_HEADER).unwrap();
        assert_eq!(header["jwk"], signer.session_key().jwk);
        let claims = r#"{"jti":"jti-1","htm":"POST","htu":"/auth/consent/agree","iat":1759700000,"bsh":"UEG_H3E98gR4Q1PoL2pKU1kxy2Tx9LSlrq_8tyCRiyI"}"#;
        assert_eq!(
            body_hash(br#"{"x":1}"#),
            "UEG_H3E98gR4Q1PoL2pKU1kxy2Tx9LSlrq_8tyCRiyI"
        );
        let h = URL_SAFE_NO_PAD.encode(EXAMPLE_HEADER);
        let c = URL_SAFE_NO_PAD.encode(claims);
        let sig = {
            use ed25519_dalek::Signer as _;
            ed_key(1).sign(format!("{h}.{c}").as_bytes()).to_bytes()
        };
        let jws = format!("{h}.{c}.{}", URL_SAFE_NO_PAD.encode(sig));
        assert_eq!(jws, EXAMPLE_JWS);
        let key = BoundKey::new("EdDSA", &signer.session_key().jwk).unwrap();
        assert_eq!(
            key.thumbprint,
            "UDDReOZl1ipXAfp9wYsm13sDBMK5og--QWdBjzuf6o4"
        );
        assert_eq!(
            check_proof(
                Some(&jws),
                Some(&key),
                &facts(br#"{"x":1}"#),
                &ReplaySet::new()
            ),
            Ok(key)
        );
    }

    /// The portal's cross-check: a proof its shipped module made (ES256,
    /// WebCrypto), accepted byte for byte with the clock at its `iat`.
    #[test]
    fn the_portals_proof_is_accepted_byte_for_byte() {
        const BODY: &[u8] =
            br#"{"request":{"clientId":"epr-cli","label":"workspace"},"agreedActs":["device.enroll"]}"#;
        const JWS: &str = "eyJ0eXAiOiJkcG9wK2p3dCIsImFsZyI6IkVTMjU2IiwiandrIjp7Imt0eSI6IkVDIiwiY3J2IjoiUC0yNTYiLCJ4IjoiUng2WUlQR2hYOHc4XzRwVl9UVzl0Q2pLN0M0czY3ZFpWUHlCLTdrWlUzTSIsInkiOiIxNElCVTJPaWhwNWRiaDhaM1BUTDBLUUFOb2N6R0VGcXd2NDFyeVpwWjJvIn19.eyJqdGkiOiJRbTl2ZEhOMGNtRndMWEJ5YjI5bUxURSIsImh0bSI6IlBPU1QiLCJodHUiOiJodHRwczovL3dvcmtzcGFjZS5sb2NhbDo4MDkwL2F1dGgvY29uc2VudC9hZ3JlZSIsImlhdCI6MTc5MTEwMDAwMCwiYnNoIjoiX0t5clc4YWpzVV82WE5zNURsZVVtVFYwdFRCWUtLb1lHSENkb2dnNUc0YyJ9.ONSIMro0LXlTLdrbXu_Gt1DBjAIJeRzVyg8HoFh-yIoUa2Rgk209hqplVYtmtmCwgYr4zrjyODBW8k4I4UNbIQ";
        assert_eq!(BODY.len(), 85);
        assert_eq!(
            body_hash(BODY),
            "_KyrW8ajsU_6XNs5DleUmTV0tTBYKKoYGHCdogg5G4c"
        );
        let proof = read_proof(JWS).expect("the portal's proof reads");
        assert_eq!(proof.key.alg, "ES256");
        assert_eq!(proof.jti, "Qm9vdHN0cmFwLXByb29mLTE");
        assert_eq!(htu_path(&proof.htu), "/auth/consent/agree");
        let facts = ProofFacts {
            method: "POST",
            path: "/auth/consent/agree",
            body: BODY,
            now_secs: 1_791_100_000,
        };
        let replay = ReplaySet::new();
        assert_eq!(
            session_proof_verdict(Some(&proof.key), Some(JWS), &facts, &replay),
            Ok(())
        );
        // Once only.
        assert_eq!(
            session_proof_verdict(Some(&proof.key), Some(JWS), &facts, &replay),
            Err(ProofRefusal::Replayed)
        );
    }

    #[test]
    fn both_algorithms_verify_and_are_compared_by_thumbprint() {
        for signer in [Signer::ed(2), Signer::p256(3)] {
            let offered = signer.session_key();
            let proof = signer.proof("POST", "/auth/login", b"{}", NOW, "a");
            let replay = ReplaySet::new();
            let login = ProofFacts {
                method: "POST",
                path: "/auth/login",
                body: b"{}",
                now_secs: NOW,
            };
            let bound = bind_session_key(&offered, Some(&proof), &login, &replay).unwrap();
            // Extra JWK members (WebCrypto's ext, key_ops) do not move the thumbprint.
            let mut bare = offered.jwk.clone();
            bare.as_object_mut().unwrap().remove("ext");
            bare.as_object_mut().unwrap().remove("key_ops");
            assert_eq!(
                BoundKey::new(&offered.alg, &bare).unwrap().thumbprint,
                bound.thumbprint
            );
            let next = signer.proof("POST", "/auth/consent/agree", b"{}", NOW, "b");
            assert_eq!(
                session_proof_verdict(Some(&bound), Some(&next), &facts(b"{}"), &replay),
                Ok(())
            );
        }
    }

    #[test]
    fn an_unbound_session_needs_no_proof_and_a_bound_one_does() {
        let replay = ReplaySet::new();
        assert_eq!(
            session_proof_verdict(None, None, &facts(b""), &replay),
            Ok(())
        );
        let key = BoundKey::new("EdDSA", &Signer::ed(4).session_key().jwk).unwrap();
        assert_eq!(
            session_proof_verdict(Some(&key), None, &facts(b""), &replay),
            Err(ProofRefusal::Missing)
        );
    }

    #[test]
    fn each_way_a_proof_fails_is_refused_by_name() {
        let signer = Signer::ed(5);
        let key = BoundKey::new("EdDSA", &signer.session_key().jwk).unwrap();
        let replay = ReplaySet::new();
        let body = br#"{"ask":"1"}"#;
        let check = |p: &str, b: &[u8]| check_proof(Some(p), Some(&key), &facts(b), &replay);
        let code = |r: Result<BoundKey, ProofRefusal>| r.unwrap_err().code();
        // Another key.
        let other = Signer::ed(6).proof("POST", "/auth/consent/agree", body, NOW, "o");
        assert_eq!(code(check(&other, body)), "session_proof_invalid");
        // Wrong method, wrong path, altered body.
        let get = signer.proof("GET", "/auth/consent/agree", body, NOW, "m");
        assert_eq!(code(check(&get, body)), "session_proof_invalid");
        let path = signer.proof("POST", "/auth/consent/pending/decide", body, NOW, "p");
        assert_eq!(code(check(&path, body)), "session_proof_invalid");
        let good = signer.proof("POST", "/auth/consent/agree", body, NOW, "g");
        assert_eq!(
            code(check(&good, br#"{"ask":"2"}"#)),
            "session_proof_invalid"
        );
        // Stale either way.
        let old = signer.proof("POST", "/auth/consent/agree", body, NOW - 61, "s1");
        assert_eq!(code(check(&old, body)), "session_proof_stale");
        let ahead = signer.proof("POST", "/auth/consent/agree", body, NOW + 61, "s2");
        assert_eq!(code(check(&ahead, body)), "session_proof_stale");
        // Used once, then replayed.
        assert!(check(&good, body).is_ok());
        assert_eq!(code(check(&good, body)), "session_proof_replayed");
        // A tampered signature.
        let mut bad = good.clone();
        bad.pop();
        bad.push(if good.ends_with('A') { 'B' } else { 'A' });
        assert_eq!(code(check(&bad, body)), "session_proof_invalid");
    }

    #[test]
    fn the_htu_names_a_path_and_nothing_else() {
        assert_eq!(
            htu_path("https://n.example/auth/consent/agree?x=1#y"),
            "/auth/consent/agree"
        );
        assert_eq!(htu_path("http://10.0.0.1:8191/auth/login"), "/auth/login");
        assert_eq!(htu_path("/auth/login?q"), "/auth/login");
        assert_eq!(htu_path("https://n.example"), "/");
        let signer = Signer::p256(7);
        let key = BoundKey::new("ES256", &signer.session_key().jwk).unwrap();
        let full = signer.proof(
            "POST",
            "https://node.example:443/auth/consent/agree?z",
            b"",
            NOW,
            "h",
        );
        assert!(check_proof(Some(&full), Some(&key), &facts(b""), &ReplaySet::new()).is_ok());
        assert_eq!(
            body_hash(b""),
            "47DEQpj8HBSa-_TImW-5JCeuQeRkm5NMpJWZG3hSuFU"
        );
    }

    #[test]
    fn only_two_algorithms_and_only_public_keys_are_accepted() {
        let signer = Signer::ed(8);
        let jwk = signer.session_key().jwk;
        let claims = serde_json::json!({"jti":"n","htm":"POST","htu":"/auth/consent/agree","iat":NOW,"bsh":body_hash(b"")});
        for header in [
            serde_json::json!({"typ":"dpop+jwt","alg":"none","jwk":jwk}),
            serde_json::json!({"typ":"dpop+jwt","alg":"HS256","jwk":jwk}),
            serde_json::json!({"typ":"dpop+jwt","alg":"ES256","jwk":jwk}),
            serde_json::json!({"typ":"jwt","alg":"EdDSA","jwk":jwk}),
            serde_json::json!({"typ":"dpop+jwt","alg":"EdDSA","jwk":jwk,"crit":["exp"]}),
            serde_json::json!({"typ":"dpop+jwt","alg":"EdDSA"}),
        ] {
            let proof = signer.proof_with(header.clone(), claims.clone());
            assert!(
                matches!(read_proof(&proof), Err(ProofRefusal::Invalid { .. })),
                "{header}"
            );
        }
        let mut private = jwk.clone();
        private["d"] = Value::from("AAAA");
        assert!(BoundKey::new("EdDSA", &private).is_err());
        for bad in [
            "",
            "a.b",
            "a.b.c.d",
            "a=.b.c",
            &"x".repeat(MAX_PROOF_LEN + 1),
        ] {
            assert!(read_proof(bad).is_err(), "{bad}");
        }
        assert!(verifier("RS256").is_none());
        // A float iat is not an integer.
        let floaty = signer.proof_with(
            serde_json::json!({"typ":"dpop+jwt","alg":"EdDSA","jwk":jwk}),
            serde_json::json!({"jti":"f","htm":"POST","htu":"/","iat":1.5,"bsh":""}),
        );
        assert!(read_proof(&floaty).is_err());
    }

    #[test]
    fn the_replay_set_is_bounded_and_forgets_by_age() {
        let replay = ReplaySet::new();
        for n in 0..MAX_SEEN {
            replay.first_use("t", &n.to_string(), 0).unwrap();
        }
        // Full and nothing old: refused, not grown.
        assert_eq!(replay.first_use("t", "new", 1), Err(ProofRefusal::Replayed));
        assert_eq!(replay.len(), MAX_SEEN);
        // Later, the old entries are swept and the proof is taken.
        assert_eq!(
            replay.first_use("t", "new", 2 * IAT_WINDOW_SECS + 1),
            Ok(())
        );
        assert_eq!(replay.len(), 1);
        // The same jti under another key is another proof.
        assert_eq!(
            replay.first_use("u", "new", 2 * IAT_WINDOW_SECS + 1),
            Ok(())
        );
    }
}
