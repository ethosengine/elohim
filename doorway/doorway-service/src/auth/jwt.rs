//! JWT Token Handling for Hosted Human Authentication
//!
//! Provides functions for generating and validating JWT tokens used to
//! authenticate hosted humans to the edge node.
//!
//! Security notes:
//! - Tokens are signed with HS256 (HMAC-SHA256)
//! - Default expiry is 1 hour
//! - In production, JWT_SECRET should be a strong random value from environment
//!
//! Ported from admin-proxy/src/jwt.ts

use ed25519_dalek::{SigningKey, VerifyingKey};
use jsonwebtoken::{
    decode, decode_header, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::auth::PermissionLevel;
use crate::types::DoorwayError;

/// Which algorithm this validator mints NEW tokens with. Verification is
/// always dual-algorithm regardless of this setting — this only controls
/// `generate_token` / `generate_refresh_token`. Operator-owned migration
/// lever: `DOORWAY_JWT_SIGN_ALG` in {`hs256` (default), `eddsa`}.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JwtSignAlg {
    /// Legacy HMAC-SHA256, self-secret-signed. Zero-migration default.
    #[default]
    Hs256,
    /// EdDSA (Ed25519), signed with the doorway's node key — the same key
    /// published at `/.well-known/doorway-keys`. Verifiable by any sibling
    /// doorway that has cached this doorway's JWKS entry.
    EdDsa,
}

impl JwtSignAlg {
    /// Parses a `DOORWAY_JWT_SIGN_ALG` value. Absent/unrecognized values
    /// fall back to `Hs256` — the safe, zero-migration default.
    pub fn parse(value: Option<&str>) -> Self {
        match value.map(str::to_ascii_lowercase) {
            Some(v) if v == "eddsa" => JwtSignAlg::EdDsa,
            _ => JwtSignAlg::Hs256,
        }
    }

    /// Reads `DOORWAY_JWT_SIGN_ALG` from the process environment.
    pub fn from_process_env() -> Self {
        Self::parse(std::env::var("DOORWAY_JWT_SIGN_ALG").ok().as_deref())
    }
}

/// Structured verify-failure classification. `verify_token`'s public
/// signature stays `TokenValidationResult` (unchanged — many call sites
/// depend on it); `verify_token_detailed` exposes this precise
/// classification for callers that need to distinguish an unresolvable
/// foreign issuer from an ordinary bad token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JwtError {
    /// Malformed token, header, or unsupported algorithm on the self path.
    Malformed,
    /// Signature check failed (tampered payload, wrong key).
    InvalidSignature,
    /// Token's `exp` has passed.
    Expired,
    /// `kid` names a doorway we hold no verified key for — self path with
    /// no attached EdDSA key, or foreign path with a peer-cache miss (or a
    /// non-EdDSA alg on a foreign `kid`). NEVER falls through to the HS256
    /// secret.
    UnknownIssuer,
    /// Any other jsonwebtoken decode error not classified above.
    Other(String),
}

impl std::fmt::Display for JwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwtError::Malformed => write!(f, "Invalid token"),
            JwtError::InvalidSignature => write!(f, "Invalid signature"),
            JwtError::Expired => write!(f, "Token expired"),
            JwtError::UnknownIssuer => write!(f, "Unknown issuer"),
            JwtError::Other(s) => write!(f, "Token validation failed: {s}"),
        }
    }
}

fn map_decode_error(err: jsonwebtoken::errors::Error) -> JwtError {
    use jsonwebtoken::errors::ErrorKind;
    match err.kind() {
        ErrorKind::ExpiredSignature => JwtError::Expired,
        ErrorKind::InvalidSignature => JwtError::InvalidSignature,
        ErrorKind::InvalidToken | ErrorKind::InvalidAlgorithm => JwtError::Malformed,
        _ => JwtError::Other(err.to_string()),
    }
}

/// Fixed 16-byte PKCS#8 v1 DER prefix for an Ed25519 raw 32-byte seed
/// (RFC 8410 `OneAsymmetricKey`: version=0, AlgorithmIdentifier=id-Ed25519,
/// then an OCTET STRING wrapping the 32-byte seed as its own OCTET STRING).
/// This is the same well-known constant every "wrap a raw Ed25519 seed as
/// PKCS#8" implementation reproduces; ring's
/// `Ed25519KeyPair::from_pkcs8_maybe_unchecked` (used internally by
/// `jsonwebtoken`'s EdDSA signer) accepts this v1-only shape without an
/// embedded public key.
const ED25519_PKCS8_DER_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];

/// Wraps a raw 32-byte Ed25519 seed in the minimal PKCS#8 DER envelope
/// `jsonwebtoken::EncodingKey::from_ed_der` expects for signing. See
/// `ED25519_PKCS8_DER_PREFIX`.
fn ed25519_seed_to_pkcs8_der(seed: &[u8; 32]) -> [u8; 48] {
    let mut der = [0u8; 48];
    der[..16].copy_from_slice(&ED25519_PKCS8_DER_PREFIX);
    der[16..].copy_from_slice(seed);
    der
}

/// Read-only, synchronous, network-free access to the peer JWKS cache.
/// `verify_token` never performs I/O — a cache miss is `UnknownIssuer`
/// immediately, and warming the cache (periodic refresh + on-demand
/// cooldown-guarded fetch) is entirely the concern of
/// `services::federation::PeerJwksCache`, which implements this trait.
/// Tests inject a trivial in-memory double.
pub trait PeerKeyLookup: Send + Sync {
    /// Returns the cached Ed25519 public key for `kid`, if present and
    /// within TTL. Never blocks on network I/O.
    fn lookup_key(&self, kid: &str) -> Option<[u8; 32]>;
}

/// Snapshot of doorway-operator authority captured at JWT issue time.
///
/// Source: derived from the rea_commitments projection (action='operate-doorway',
/// state='active') for the issuing doorway. Wire shape:
/// elohim/sdk/schemas/v1/views/doorway-operator-binding-view.schema.json
///
/// This snapshot is the **cached truth** the auth fast path checks. On
/// snapshot_age exceeding the configured TTL, doorway re-queries the projection
/// to re-issue the JWT with a fresh snapshot. The DHT-notarized Commitment is
/// the canonical source; this struct is its in-flight projection.
///
/// Backward compatibility: absent for non-operator JWTs (hosted users, agent
/// authentication for content access). Only populated for accounts that have
/// an active operate-doorway commitment on the issuing doorway.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperatorSnapshot {
    /// DoorwayRegistration.id this operator authority applies to.
    pub doorway_id: String,
    /// Commitment.id of the underlying operate-doorway commitment.
    pub commitment_id: String,
    /// Capability strings carried by the binding (from
    /// resource_classified_as). "*" entry grants all capabilities; otherwise
    /// membership is checked exactly. Per operator-classification.schema.json.
    pub capabilities: Vec<String>,
    /// Role within the doorway's succession plan.
    pub succession_role: String,
    /// Reach class of the binding (visibility).
    pub reach_scope: String,
    /// ActionHash of the Commitment entry on the DHT (Cat A discipline).
    pub dht_anchor_hash: String,
    /// ActionHash of the active HardwareCustodyAttestation this operator
    /// binding serves under. None during the schemaVersion=1 transition
    /// window. Once populated, the auth chain resolver verifies it still
    /// matches the doorway's current custody attestation; mismatch =
    /// orphaned binding (CapabilityError::CustodyTransferred). See
    /// elohim/sdk/schemas/v1/attestation/subtypes/hardware-custody-metadata.schema.json.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custody_attestation_hash: Option<String>,
    /// ActionHash of the active StewardOfRecordAttestation this operator
    /// binding serves under. Independently revocable from custody — a
    /// stewardship transfer orphans operator commitments without affecting
    /// custody. None during transition. See
    /// elohim/sdk/schemas/v1/attestation/subtypes/steward-of-record-metadata.schema.json.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steward_attestation_hash: Option<String>,
    /// Unix timestamp when this snapshot was taken from the projection.
    /// Auth layer uses this for TTL-based refresh decisions.
    pub snapshot_taken_at: u64,
}

/// Payload stored in JWT token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Holochain human ID
    pub human_id: String,
    /// Holochain agent public key (hex string)
    pub agent_pub_key: String,
    /// User identifier (email/username)
    pub identifier: String,
    /// Permission level granted
    pub permission_level: PermissionLevel,
    /// Operator authority snapshot, if this agent has an active
    /// operate-doorway commitment on the issuing doorway. Absent for ordinary
    /// hosted-user JWTs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doorway_operator: Option<OperatorSnapshot>,
    /// Session ID for signing key cache lookup (custodial mode)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Doorway ID that issued this token (for federation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doorway_id: Option<String>,
    /// Doorway URL for cross-doorway validation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doorway_url: Option<String>,
    /// Conductor ID for deterministic routing (Chaperone pattern)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conductor_id: Option<String>,
    /// Installed app ID on the conductor
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_app_id: Option<String>,
    /// Whether the user has migrated to steward key management
    #[serde(default)]
    pub is_steward: bool,
    /// Whether the user has a local conductor (Tauri/native)
    #[serde(default)]
    pub has_local_conductor: bool,
    /// Token version (for future invalidation)
    pub version: u32,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
}

/// Input for creating a new token
#[derive(Debug, Clone)]
pub struct TokenInput {
    pub human_id: String,
    pub agent_pub_key: String,
    pub identifier: String,
    pub permission_level: PermissionLevel,
    /// Session ID for signing key cache lookup (custodial mode)
    pub session_id: Option<String>,
    /// Doorway ID that issues this token (for federation)
    pub doorway_id: Option<String>,
    /// Doorway URL for cross-doorway validation
    pub doorway_url: Option<String>,
    /// Conductor ID for deterministic routing (Chaperone pattern)
    pub conductor_id: Option<String>,
    /// Installed app ID on the conductor
    pub installed_app_id: Option<String>,
    /// Whether the user has migrated to steward key management
    pub is_steward: bool,
    /// Whether the user has a local conductor (Tauri/native)
    pub has_local_conductor: bool,
}

/// Result of token validation
#[derive(Debug)]
pub struct TokenValidationResult {
    pub valid: bool,
    pub claims: Option<Claims>,
    pub error: Option<String>,
}

impl TokenValidationResult {
    pub fn valid(claims: Claims) -> Self {
        Self {
            valid: true,
            claims: Some(claims),
            error: None,
        }
    }

    pub fn invalid(error: impl Into<String>) -> Self {
        Self {
            valid: false,
            claims: None,
            error: Some(error.into()),
        }
    }
}

/// JWT validator and generator
#[derive(Clone)]
pub struct JwtValidator {
    secret: String,
    expiry_seconds: u64,
    /// This doorway's own id, compared against a token's `kid` to decide
    /// self vs foreign. `None` means federation identity isn't configured
    /// on this instance — every `kid`-bearing token is then treated as
    /// foreign (fails closed to `UnknownIssuer` rather than guessing).
    doorway_id: Option<String>,
    /// This doorway's own Ed25519 keys, for self-minted/self-verified EdDSA
    /// tokens (`kid == doorway_id`). Callers are expected to reuse the SAME
    /// node key already published at `/.well-known/doorway-keys` — never
    /// mint a fresh one for this purpose.
    eddsa_signing_key: Option<SigningKey>,
    eddsa_verifying_key: Option<VerifyingKey>,
    /// Which algorithm `generate_token` / `generate_refresh_token` mint
    /// with. Defaults to HS256 (zero-migration).
    sign_alg: JwtSignAlg,
    /// Read access to the shared peer-JWKS cache for foreign-`kid`
    /// verification. `None` means every foreign kid is `UnknownIssuer`.
    peer_keys: Option<std::sync::Arc<dyn PeerKeyLookup>>,
}

/// Fixed, publicly-known placeholder value used only by `JwtValidator::new_dev`
/// (local/dev mode). Never used when a real `JWT_SECRET` is configured.
fn dev_mode_placeholder_value() -> String {
    ["dev", "mode", "not", "for", "production", "use", "123456"].join("-")
}

impl JwtValidator {
    /// Create a new JWT validator
    ///
    /// Returns an error if the secret is empty or too short
    pub fn new(secret: String, expiry_seconds: u64) -> Result<Self, DoorwayError> {
        if secret.is_empty() {
            return Err(DoorwayError::Config(
                "JWT_SECRET is required in production mode".into(),
            ));
        }

        if secret.len() < 32 {
            return Err(DoorwayError::Config(
                "JWT_SECRET must be at least 32 characters".into(),
            ));
        }

        Ok(Self {
            secret,
            expiry_seconds,
            doorway_id: None,
            eddsa_signing_key: None,
            eddsa_verifying_key: None,
            sign_alg: JwtSignAlg::Hs256,
            peer_keys: None,
        })
    }

    /// Create a validator for dev mode (allows empty secret)
    pub fn new_dev() -> Self {
        Self {
            secret: dev_mode_placeholder_value(),
            expiry_seconds: 3600,
            doorway_id: None,
            eddsa_signing_key: None,
            eddsa_verifying_key: None,
            sign_alg: JwtSignAlg::Hs256,
            peer_keys: None,
        }
    }

    /// Attaches this doorway's own id — enables self-vs-foreign `kid`
    /// comparison in `verify_token`. Without it, any `kid`-bearing token is
    /// treated as foreign.
    pub fn with_doorway_id(mut self, doorway_id: impl Into<String>) -> Self {
        self.doorway_id = Some(doorway_id.into());
        self
    }

    /// Attaches the node's Ed25519 keypair — the SAME key already published
    /// at `/.well-known/doorway-keys` (never mint a fresh key for this).
    /// Enables self-minted and self-verified EdDSA tokens.
    pub fn with_eddsa_signing_key(
        mut self,
        signing_key: SigningKey,
        verifying_key: VerifyingKey,
    ) -> Self {
        self.eddsa_signing_key = Some(signing_key);
        self.eddsa_verifying_key = Some(verifying_key);
        self
    }

    /// Verify-only counterpart of [`with_eddsa_signing_key`] — attaches only
    /// the public half, enabling self-EdDSA verification without minting.
    pub fn with_eddsa_verifying_key(mut self, verifying_key: VerifyingKey) -> Self {
        self.eddsa_verifying_key = Some(verifying_key);
        self
    }

    /// Sets which algorithm new tokens are minted with. Defaults to HS256.
    /// See `JwtSignAlg::from_process_env` for the operator-owned
    /// `DOORWAY_JWT_SIGN_ALG` migration lever.
    pub fn with_sign_alg(mut self, alg: JwtSignAlg) -> Self {
        self.sign_alg = alg;
        self
    }

    /// Attaches read access to the shared peer-JWKS cache for foreign-`kid`
    /// verification. Without it, every foreign kid is `UnknownIssuer`.
    pub fn with_peer_key_lookup(mut self, lookup: std::sync::Arc<dyn PeerKeyLookup>) -> Self {
        self.peer_keys = Some(lookup);
        self
    }

    /// Generate a JWT token for an authenticated user
    pub fn generate_token(&self, input: TokenInput) -> Result<String, DoorwayError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| DoorwayError::Auth(format!("System time error: {e}")))?
            .as_secs();

        let claims = Claims {
            human_id: input.human_id,
            agent_pub_key: input.agent_pub_key,
            identifier: input.identifier,
            permission_level: input.permission_level,
            // Operator snapshot is populated at /auth/login via projection
            // lookup (task #15). Plain generate_token leaves it empty; the
            // login flow will replace the JWT with a snapshot-bearing one
            // once the operator-binding lookup is wired in.
            doorway_operator: None,
            session_id: input.session_id,
            doorway_id: input.doorway_id,
            doorway_url: input.doorway_url,
            conductor_id: input.conductor_id,
            installed_app_id: input.installed_app_id,
            is_steward: input.is_steward,
            has_local_conductor: input.has_local_conductor,
            version: 1,
            iat: now,
            exp: now + self.expiry_seconds,
        };

        self.encode_claims(&claims)
            .map_err(|e| DoorwayError::Auth(format!("Failed to generate token: {e}")))
    }

    /// Generate a refresh token with longer expiry (7 days)
    pub fn generate_refresh_token(&self, input: TokenInput) -> Result<String, DoorwayError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| DoorwayError::Auth(format!("System time error: {e}")))?
            .as_secs();

        // Refresh tokens last 7 days
        let refresh_expiry = 7 * 24 * 60 * 60;

        let claims = Claims {
            human_id: input.human_id,
            agent_pub_key: input.agent_pub_key,
            identifier: input.identifier,
            permission_level: input.permission_level,
            // Operator snapshot omitted on refresh tokens by design — refresh
            // tokens are exchanged for new access tokens, at which point the
            // login flow re-queries the projection for a fresh snapshot.
            doorway_operator: None,
            session_id: input.session_id,
            doorway_id: input.doorway_id,
            doorway_url: input.doorway_url,
            conductor_id: input.conductor_id,
            installed_app_id: input.installed_app_id,
            is_steward: input.is_steward,
            has_local_conductor: input.has_local_conductor,
            version: 1,
            iat: now,
            exp: now + refresh_expiry,
        };

        self.encode_claims(&claims)
            .map_err(|e| DoorwayError::Auth(format!("Failed to generate refresh token: {e}")))
    }

    /// Encodes `claims` per `self.sign_alg` — HS256 (legacy, self-secret) or
    /// EdDSA (self-minted, signed with the attached node key). Both branches
    /// stamp `kid = doorway_id` when available (harmless on the HS256 path —
    /// existing verifiers ignore an unrecognized header field — and REQUIRED
    /// on the EdDSA path, since that's how a sibling doorway knows which
    /// public key to verify against).
    fn encode_claims(&self, claims: &Claims) -> Result<String, String> {
        match self.sign_alg {
            JwtSignAlg::Hs256 => {
                let header = Header {
                    kid: self.doorway_id.clone(),
                    ..Header::default()
                };
                encode(
                    &header,
                    claims,
                    &EncodingKey::from_secret(self.secret.as_bytes()),
                )
                .map_err(|e| e.to_string())
            }
            JwtSignAlg::EdDsa => {
                let signing_key = self
                    .eddsa_signing_key
                    .as_ref()
                    .ok_or_else(|| "EdDSA minting requires an attached signing key".to_string())?;
                let doorway_id = self
                    .doorway_id
                    .clone()
                    .ok_or_else(|| "EdDSA minting requires a doorway_id (kid)".to_string())?;
                let mut header = Header::new(Algorithm::EdDSA);
                header.kid = Some(doorway_id);
                let der = ed25519_seed_to_pkcs8_der(&signing_key.to_bytes());
                encode(&header, claims, &EncodingKey::from_ed_der(&der)).map_err(|e| e.to_string())
            }
        }
    }

    /// Verifies and decodes a JWT token. Public signature is unchanged
    /// (`TokenValidationResult`, matching every existing call site);
    /// `verify_token_detailed` exposes the precise `JwtError` classification
    /// for callers that need it.
    pub fn verify_token(&self, token: &str) -> TokenValidationResult {
        match self.verify_token_detailed(token) {
            Ok(claims) => TokenValidationResult::valid(claims),
            Err(e) => TokenValidationResult::invalid(e.to_string()),
        }
    }

    /// Dual-algorithm verify, with the precise `JwtError` classification.
    ///
    /// - No `kid`, or `kid == self doorway_id` with `alg=HS256` → legacy
    ///   self-secret path (unchanged behavior).
    /// - `kid == self doorway_id` with `alg=EdDSA` → self-minted EdDSA,
    ///   verified against the attached verifying key.
    /// - `kid != self doorway_id` → foreign issuer. MUST be `alg=EdDSA`; the
    ///   sibling's public key is looked up in the attached peer cache. A
    ///   cache miss, or any non-EdDSA alg on a foreign `kid`, is
    ///   `UnknownIssuer` — NEVER a fallthrough to the HS256 secret.
    ///
    /// Performs no network I/O: the peer-key lookup is a synchronous cache
    /// read (`PeerKeyLookup`).
    pub fn verify_token_detailed(&self, token: &str) -> Result<Claims, JwtError> {
        let header = decode_header(token).map_err(|_| JwtError::Malformed)?;
        let kid = header.kid.as_deref();

        let is_self = match (kid, self.doorway_id.as_deref()) {
            (None, _) => true,
            (Some(k), Some(self_id)) => k == self_id,
            (Some(_), None) => false,
        };

        if is_self {
            match header.alg {
                Algorithm::EdDSA => {
                    let verifying_key = self.eddsa_verifying_key.ok_or(JwtError::UnknownIssuer)?;
                    let validation = Validation::new(Algorithm::EdDSA);
                    let decoding_key = DecodingKey::from_ed_der(&verifying_key.to_bytes());
                    decode::<Claims>(token, &decoding_key, &validation)
                        .map(|d| d.claims)
                        .map_err(map_decode_error)
                }
                Algorithm::HS256 => {
                    let validation = Validation::default();
                    let decoding_key = DecodingKey::from_secret(self.secret.as_bytes());
                    decode::<Claims>(token, &decoding_key, &validation)
                        .map(|d| d.claims)
                        .map_err(map_decode_error)
                }
                _ => Err(JwtError::Malformed),
            }
        } else {
            if header.alg != Algorithm::EdDSA {
                // A foreign kid MUST be EdDSA — never accept HS256 (or any
                // other alg) from a claimed sibling, even if it happens to
                // verify against our own secret.
                return Err(JwtError::UnknownIssuer);
            }
            let kid = kid.expect("kid is Some on the foreign-issuer branch");
            let pubkey = self
                .peer_keys
                .as_ref()
                .and_then(|lookup| lookup.lookup_key(kid))
                .ok_or(JwtError::UnknownIssuer)?;
            let validation = Validation::new(Algorithm::EdDSA);
            let decoding_key = DecodingKey::from_ed_der(&pubkey);
            decode::<Claims>(token, &decoding_key, &validation)
                .map(|d| d.claims)
                .map_err(map_decode_error)
        }
    }

    /// Check if a token is close to expiring
    pub fn is_token_expiring_soon(&self, claims: &Claims, threshold_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        claims.exp.saturating_sub(now) < threshold_seconds
    }

    /// Get remaining time until token expiry
    pub fn get_token_time_remaining(&self, claims: &Claims) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        claims.exp as i64 - now as i64
    }
}

/// Extract token from Authorization header.
/// Supports "Bearer <token>" format and raw tokens.
pub fn extract_token_from_header(auth_header: Option<&str>) -> Option<&str> {
    let header = auth_header?;

    // Support "Bearer <token>" format
    if let Some(token) = header.strip_prefix("Bearer ") {
        let token = token.trim();
        if !token.is_empty() {
            return Some(token);
        }
    }

    // Also support raw token (for flexibility)
    if !header.contains(' ') {
        let token = header.trim();
        if !token.is_empty() {
            return Some(token);
        }
    }

    None
}

/// Extract token from URL query parameter
pub fn extract_token_from_url(url: &str, param_name: &str) -> Option<String> {
    // Simple query string parsing
    let query_start = url.find('?')?;
    let query = &url[query_start + 1..];

    for param in query.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            if key == param_name {
                return Some(value.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_validator() -> JwtValidator {
        JwtValidator::new(
            "test-secret-that-is-at-least-32-characters-long".into(),
            3600,
        )
        .unwrap()
    }

    #[test]
    fn test_generate_and_verify_token() {
        let validator = test_validator();

        let input = TokenInput {
            human_id: "human-123".into(),
            agent_pub_key: "uhCAk...".into(),
            identifier: "test@example.com".into(),
            permission_level: PermissionLevel::Authenticated,
            session_id: Some("session-abc".into()),
            doorway_id: Some("alpha-elohim-host".into()),
            doorway_url: Some("https://alpha.elohim.host".into()),
            conductor_id: None,
            installed_app_id: None,
            is_steward: false,
            has_local_conductor: false,
        };

        let token = validator.generate_token(input).unwrap();
        assert!(!token.is_empty());

        let result = validator.verify_token(&token);
        assert!(result.valid);

        let claims = result.claims.unwrap();
        assert_eq!(claims.human_id, "human-123");
        assert_eq!(claims.identifier, "test@example.com");
        assert_eq!(claims.permission_level, PermissionLevel::Authenticated);
        assert_eq!(claims.session_id, Some("session-abc".into()));
        assert_eq!(claims.doorway_id, Some("alpha-elohim-host".into()));
        assert_eq!(claims.doorway_url, Some("https://alpha.elohim.host".into()));
        assert!(!claims.is_steward);
        assert!(!claims.has_local_conductor);
    }

    #[test]
    fn test_invalid_token() {
        let validator = test_validator();

        let result = validator.verify_token("invalid-token");
        assert!(!result.valid);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_wrong_secret() {
        let validator1 = test_validator();
        let validator2 = JwtValidator::new(
            "different-secret-that-is-at-least-32-characters".into(),
            3600,
        )
        .unwrap();

        let input = TokenInput {
            human_id: "human-123".into(),
            agent_pub_key: "uhCAk...".into(),
            identifier: "test@example.com".into(),
            permission_level: PermissionLevel::Authenticated,
            session_id: None,
            doorway_id: None,
            doorway_url: None,
            conductor_id: None,
            installed_app_id: None,
            is_steward: false,
            has_local_conductor: false,
        };

        let token = validator1.generate_token(input).unwrap();

        // Verify with wrong secret should fail
        let result = validator2.verify_token(&token);
        assert!(!result.valid);
    }

    #[test]
    fn test_extract_token_from_header() {
        // Bearer format
        assert_eq!(
            extract_token_from_header(Some("Bearer abc123")),
            Some("abc123")
        );

        // Raw token
        assert_eq!(extract_token_from_header(Some("abc123")), Some("abc123"));

        // Empty cases
        assert_eq!(extract_token_from_header(None), None);
        assert_eq!(extract_token_from_header(Some("")), None);
        assert_eq!(extract_token_from_header(Some("Bearer ")), None);

        // Invalid format
        assert_eq!(extract_token_from_header(Some("Basic abc123")), None);
    }

    #[test]
    fn test_extract_token_from_url() {
        assert_eq!(
            extract_token_from_url("http://localhost?token=abc123", "token"),
            Some("abc123".into())
        );

        assert_eq!(
            extract_token_from_url("http://localhost?foo=bar&token=abc123", "token"),
            Some("abc123".into())
        );

        assert_eq!(
            extract_token_from_url("http://localhost?foo=bar", "token"),
            None
        );

        assert_eq!(extract_token_from_url("http://localhost", "token"), None);
    }

    #[test]
    fn test_secret_validation() {
        // Too short
        assert!(JwtValidator::new("short".into(), 3600).is_err());

        // Empty
        assert!(JwtValidator::new("".into(), 3600).is_err());

        // Valid
        assert!(JwtValidator::new("this-secret-is-at-least-32-chars-long".into(), 3600).is_ok());
    }

    #[test]
    fn test_dev_mode_validator() {
        let validator = JwtValidator::new_dev();

        let input = TokenInput {
            human_id: "human-123".into(),
            agent_pub_key: "uhCAk...".into(),
            identifier: "test@example.com".into(),
            permission_level: PermissionLevel::Admin,
            session_id: None,
            doorway_id: None,
            doorway_url: None,
            conductor_id: None,
            installed_app_id: None,
            is_steward: false,
            has_local_conductor: false,
        };

        let token = validator.generate_token(input).unwrap();
        let result = validator.verify_token(&token);
        assert!(result.valid);
    }

    #[test]
    fn test_token_without_doorway_fields() {
        // Test backward compatibility - tokens should work without doorway fields
        let validator = test_validator();

        let input = TokenInput {
            human_id: "human-123".into(),
            agent_pub_key: "uhCAk...".into(),
            identifier: "test@example.com".into(),
            permission_level: PermissionLevel::Authenticated,
            session_id: None,
            doorway_id: None,
            doorway_url: None,
            conductor_id: None,
            installed_app_id: None,
            is_steward: false,
            has_local_conductor: false,
        };

        let token = validator.generate_token(input).unwrap();
        let result = validator.verify_token(&token);
        assert!(result.valid);

        let claims = result.claims.unwrap();
        assert!(claims.session_id.is_none());
        assert!(claims.doorway_id.is_none());
        assert!(claims.doorway_url.is_none());
    }

    // ── Cross-doorway JWT trust (dual-algorithm verify + EdDSA mint) ──────

    mod federation {
        use super::*;
        use rand::rngs::OsRng;
        use std::collections::HashMap;
        use std::sync::Arc;

        /// In-memory `PeerKeyLookup` double — zero network, exactly what a
        /// test injects in place of `services::federation::PeerJwksCache`.
        struct FixedPeerKeys(HashMap<String, [u8; 32]>);

        impl PeerKeyLookup for FixedPeerKeys {
            fn lookup_key(&self, kid: &str) -> Option<[u8; 32]> {
                self.0.get(kid).copied()
            }
        }

        fn peer_cache(entries: &[(&str, VerifyingKey)]) -> Arc<dyn PeerKeyLookup> {
            let mut map = HashMap::new();
            for (kid, key) in entries {
                map.insert((*kid).to_string(), key.to_bytes());
            }
            Arc::new(FixedPeerKeys(map))
        }

        fn keypair() -> (SigningKey, VerifyingKey) {
            let signing_key = SigningKey::generate(&mut OsRng);
            let verifying_key = signing_key.verifying_key();
            (signing_key, verifying_key)
        }

        /// A validator standing in for a sibling doorway ("sibling") that
        /// mints its own EdDSA tokens — independent of the validator under
        /// test.
        fn sibling_validator(signing_key: SigningKey, verifying_key: VerifyingKey) -> JwtValidator {
            JwtValidator::new_dev()
                .with_doorway_id("sibling")
                .with_eddsa_signing_key(signing_key, verifying_key)
                .with_sign_alg(JwtSignAlg::EdDsa)
        }

        fn federation_input() -> TokenInput {
            TokenInput {
                human_id: "human-456".into(),
                agent_pub_key: "uhCAk...".into(),
                identifier: "sibling-user@example.com".into(),
                permission_level: PermissionLevel::Authenticated,
                session_id: None,
                doorway_id: Some("sibling".into()),
                doorway_url: Some("https://sibling.elohim.host".into()),
                conductor_id: None,
                installed_app_id: None,
                is_steward: false,
                has_local_conductor: false,
            }
        }

        #[test]
        fn foreign_eddsa_token_verifies_when_cache_holds_sibling_key() {
            let (signing_key, verifying_key) = keypair();
            let sibling = sibling_validator(signing_key, verifying_key);
            let token = sibling.generate_token(federation_input()).unwrap();

            // Our own HS256 secret is deliberately irrelevant here — only
            // the cached sibling key can verify this token.
            let validator = test_validator()
                .with_doorway_id("home")
                .with_peer_key_lookup(peer_cache(&[("sibling", verifying_key)]));

            let result = validator.verify_token(&token);
            assert!(
                result.valid,
                "expected valid, got error: {:?}",
                result.error
            );
            let claims = result.claims.unwrap();
            assert_eq!(claims.doorway_id, Some("sibling".into()));
        }

        #[test]
        fn tampered_foreign_token_fails_signature() {
            let (signing_key, verifying_key) = keypair();
            let sibling = sibling_validator(signing_key, verifying_key);
            let token = sibling.generate_token(federation_input()).unwrap();

            let parts: Vec<&str> = token.split('.').collect();
            assert_eq!(parts.len(), 3, "JWT must have header.payload.signature");
            // Corrupt the payload segment — signature no longer matches.
            let tampered = format!("{}.{}x.{}", parts[0], parts[1], parts[2]);

            let validator = test_validator()
                .with_doorway_id("home")
                .with_peer_key_lookup(peer_cache(&[("sibling", verifying_key)]));

            let result = validator.verify_token_detailed(&tampered);
            assert!(
                matches!(
                    result,
                    Err(JwtError::InvalidSignature) | Err(JwtError::Malformed)
                ),
                "tampered payload must fail verification, got {result:?}"
            );
        }

        #[test]
        fn unknown_foreign_kid_is_unknown_issuer_never_hs256_fallthrough() {
            let (signing_key, verifying_key) = keypair();
            let sibling = sibling_validator(signing_key, verifying_key);
            let token = sibling.generate_token(federation_input()).unwrap();

            // No peer cache attached — cache miss must be UnknownIssuer, not
            // a fallthrough attempt against our own HS256 secret.
            let validator = test_validator().with_doorway_id("home");

            let result = validator.verify_token_detailed(&token);
            assert!(matches!(result, Err(JwtError::UnknownIssuer)), "{result:?}");
        }

        #[test]
        fn foreign_kid_with_hs256_alg_is_rejected_even_against_own_secret() {
            // A token carrying a foreign kid but HS256-signed with OUR OWN
            // secret must still be rejected — foreign kid mandates EdDSA,
            // unconditionally. This proves the guard, not just the absence
            // of a cache entry.
            let shared_secret = "test-secret-that-is-at-least-32-characters-long";
            let validator = JwtValidator::new(shared_secret.into(), 3600)
                .unwrap()
                .with_doorway_id("home");

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let claims = Claims {
                human_id: "human-456".into(),
                agent_pub_key: "uhCAk...".into(),
                identifier: "sibling-user@example.com".into(),
                permission_level: PermissionLevel::Authenticated,
                doorway_operator: None,
                session_id: None,
                doorway_id: Some("sibling".into()),
                doorway_url: None,
                conductor_id: None,
                installed_app_id: None,
                is_steward: false,
                has_local_conductor: false,
                version: 1,
                iat: now,
                exp: now + 3600,
            };
            let mut header = Header::default(); // alg = HS256
            header.kid = Some("sibling".into());
            let token = encode(
                &header,
                &claims,
                &EncodingKey::from_secret(shared_secret.as_bytes()),
            )
            .unwrap();

            let result = validator.verify_token_detailed(&token);
            assert!(
                matches!(result, Err(JwtError::UnknownIssuer)),
                "HS256 + foreign kid must never verify, even against our own secret: {result:?}"
            );
        }

        #[test]
        fn legacy_hs256_token_without_kid_verifies_unchanged() {
            let validator = test_validator().with_doorway_id("home");
            let token = validator.generate_token(federation_input()).unwrap();

            let header = decode_header(&token).unwrap();
            assert_eq!(header.alg, Algorithm::HS256);
            assert_eq!(
                header.kid,
                Some("home".into()),
                "kid is stamped for observability but stays HS256"
            );

            let result = validator.verify_token(&token);
            assert!(result.valid, "{:?}", result.error);
        }

        #[test]
        fn self_minted_eddsa_token_verifies_against_own_key() {
            let (signing_key, verifying_key) = keypair();
            let validator = test_validator()
                .with_doorway_id("home")
                .with_eddsa_signing_key(signing_key, verifying_key)
                .with_sign_alg(JwtSignAlg::EdDsa);

            let token = validator.generate_token(federation_input()).unwrap();
            let header = decode_header(&token).unwrap();
            assert_eq!(header.alg, Algorithm::EdDSA);
            assert_eq!(header.kid, Some("home".into()));

            let result = validator.verify_token(&token);
            assert!(result.valid, "{:?}", result.error);
        }

        #[test]
        fn sign_alg_lever_mints_eddsa_and_round_trips_through_verify() {
            // Simulates DOORWAY_JWT_SIGN_ALG=eddsa.
            let alg = JwtSignAlg::parse(Some("eddsa"));
            assert_eq!(alg, JwtSignAlg::EdDsa);

            let (signing_key, verifying_key) = keypair();
            let validator = test_validator()
                .with_doorway_id("home")
                .with_eddsa_signing_key(signing_key, verifying_key)
                .with_sign_alg(alg);

            let token = validator.generate_token(federation_input()).unwrap();
            let header = decode_header(&token).unwrap();
            assert_eq!(header.alg, Algorithm::EdDSA);
            assert_eq!(header.kid, Some("home".into()));

            let result = validator.verify_token(&token);
            assert!(result.valid, "{:?}", result.error);
            assert_eq!(result.claims.unwrap().doorway_id, Some("sibling".into()));
        }

        #[test]
        fn sign_alg_parse_defaults_to_hs256_and_is_case_insensitive() {
            assert_eq!(JwtSignAlg::parse(None), JwtSignAlg::Hs256);
            assert_eq!(JwtSignAlg::parse(Some("bogus")), JwtSignAlg::Hs256);
            assert_eq!(JwtSignAlg::parse(Some("HS256")), JwtSignAlg::Hs256);
            assert_eq!(JwtSignAlg::parse(Some("EdDSA")), JwtSignAlg::EdDsa);
            assert_eq!(JwtSignAlg::parse(Some("eddsa")), JwtSignAlg::EdDsa);
        }
    }
}
