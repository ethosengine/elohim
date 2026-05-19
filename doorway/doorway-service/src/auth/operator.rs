//! Operator-Authority Resolution
//!
//! Replaces the legacy `permission_level >= Admin` god-flag check (status.rs)
//! with a capability-scoped lookup against the doorway-operator authority
//! substrate landed in 2026-05-19-000000_doorway_operator_action_indexes:
//!
//!  - Truth lives in the Holochain DHT as an `operate-doorway` Commitment
//!    (Notarized, Category A — existing entry type, no new shape).
//!  - SQLite projection: `rea_commitments` filtered by action + scope + state
//!    (Operational, Category C — see
//!    `elohim/elohim-storage/src/db/rea_commitments.rs::find_active_operator_binding`).
//!  - Wire shape: `DoorwayOperatorBindingView`
//!    (`elohim/sdk/schemas/v1/views/doorway-operator-binding-view.schema.json`).
//!
//! ## Auth-resolution flow
//!
//! 1. Extract JWT from `Authorization: Bearer …` header or `doorway_token`
//!    cookie (same plumbing as the legacy [`crate::routes::status`] check).
//! 2. Validate signature + expiry via [`JwtValidator::verify_token`].
//! 3. Read [`OperatorSnapshot`] from the claims. The snapshot is populated at
//!    `/auth/login` time by querying the projection — see follow-up task #15
//!    for that wiring. JWTs minted before the snapshot field existed (legacy
//!    Admin-level tokens) deserialize with `doorway_operator = None`; this
//!    resolver returns [`CapabilityError::NotOperator`] for them. Routes that
//!    must continue accepting legacy Admin JWTs during the transition window
//!    can fall back to [`PermissionLevel`] explicitly.
//! 4. Match `required` against the snapshot's capability list. The
//!    [`WILDCARD_CAPABILITY`] token (`"*"`) matches any required capability —
//!    reserved for the primary operator binding seeded at doorway bootstrap.
//! 5. Emit a [`CapabilityResult`] carrying enough context for the request
//!    handler to log, audit, and re-check on TTL expiry.
//!
//! ## Non-goals (this module)
//!
//!  - Refresh on TTL expiry. The auth refresh path lives at the login flow;
//!    this resolver is a pure function over the incoming request + state.
//!  - Token issuance. See [`JwtValidator::generate_token`] and the upcoming
//!    `/auth/login` integration in task #15.
//!  - Per-route policy declaration. Routes name the capability they need
//!    inline at call time. A future refactor may extract policy into the
//!    manifest registry; for now the call site is the source of truth.

use hyper::body::Incoming;
use hyper::{header, Request};

use crate::auth::{
    extract_token_from_header, Claims, JwtValidator, OperatorSnapshot, PermissionLevel,
};
use crate::server::AppState;

/// Capability string that grants every operation a route may require.
///
/// Reserved for the primary operator binding emitted at doorway bootstrap
/// (seeded from `STEWARD_AGENT_KEY` per the bootstrap path documented in the
/// L3 substrate). Deputies and recovery-witnesses MUST enumerate their
/// capabilities explicitly — this string in a non-primary binding indicates
/// drift from the operator-classification schema.
pub const WILDCARD_CAPABILITY: &str = "*";

/// Outcome of a successful capability resolution.
///
/// Carries audit-relevant context. Handlers should log `commitment_id` and
/// `snapshot_age_secs` so revocation events have a forensic trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityResult {
    /// Agent public key from the JWT (operator's identity).
    pub agent_pub_key: String,
    /// Doorway scope this authority applies to (must match the issuing
    /// doorway when cross-doorway federation is in scope; the resolver does
    /// not enforce cross-doorway equality today).
    pub doorway_id: String,
    /// Underlying commitment in the projection.
    pub commitment_id: String,
    /// The required capability that was matched (may equal
    /// `WILDCARD_CAPABILITY` if the binding grants `"*"`).
    pub matched_capability: String,
    /// Age of the embedded snapshot at resolution time. Used by callers to
    /// decide whether to schedule a refresh.
    pub snapshot_age_secs: u64,
}

/// Failure modes for capability resolution.
///
/// All variants short-circuit the request with a non-2xx response; handlers
/// pick the HTTP status (401 for token issues, 403 for authorization issues).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    /// No `Authorization` header and no `doorway_token` cookie.
    NoToken,
    /// JWT signature invalid, expired, or malformed.
    InvalidToken(String),
    /// Token verified but carries no operator snapshot. Legacy Admin-level
    /// tokens land here; the snapshot is populated at login time by the
    /// projection lookup (task #16).
    NotOperator,
    /// Snapshot present but predates the freshness window. Caller should
    /// reissue via the refresh flow rather than authorizing the request.
    SnapshotExpired { age_secs: u64, max_age_secs: u64 },
    /// Snapshot present and fresh but the required capability is not in the
    /// binding's capability list.
    CapabilityMissing { required: String, held: Vec<String> },
    /// The hardware custody attestation the operator served under has been
    /// superseded — a CustodyTransferAttestation references it as predecessor.
    /// All commitments below the superseded attestation are orphaned; the
    /// caller MUST re-authenticate to obtain a fresh snapshot under the new
    /// custody chain. Carries the predecessor + current attestation hashes
    /// for forensic logging.
    CustodyTransferred {
        previous_hash: String,
        current_hash: String,
    },
    /// The steward-of-record attestation the operator served under has been
    /// superseded — independently of custody. Behaves like CustodyTransferred
    /// but at one tier down the chain.
    StewardTransferred {
        previous_hash: String,
        current_hash: String,
    },
}

/// Decide whether a capability list grants a required capability.
///
/// Wildcard semantics: a binding containing the literal `"*"` string grants
/// every capability. Anything else requires exact string match.
pub fn capability_satisfies(held: &[String], required: &str) -> bool {
    held.iter()
        .any(|c| c == WILDCARD_CAPABILITY || c == required)
}

/// Resolve operator authority for the current request against a required
/// capability.
///
/// Returns the matched snapshot on success. The `max_age_secs` parameter
/// gates how stale a snapshot may be — pass `u64::MAX` to disable the
/// freshness check (e.g., in tests).
pub fn resolve_operator_capability(
    req: &Request<Incoming>,
    state: &AppState,
    required: &str,
    max_age_secs: u64,
) -> Result<CapabilityResult, CapabilityError> {
    let token = extract_token(req).ok_or(CapabilityError::NoToken)?;
    let claims = verify_claims(&token, state).map_err(CapabilityError::InvalidToken)?;

    let snapshot = claims
        .doorway_operator
        .as_ref()
        .ok_or(CapabilityError::NotOperator)?;

    let age = snapshot_age(snapshot);
    if age > max_age_secs {
        return Err(CapabilityError::SnapshotExpired {
            age_secs: age,
            max_age_secs,
        });
    }

    if !capability_satisfies(&snapshot.capabilities, required) {
        return Err(CapabilityError::CapabilityMissing {
            required: required.to_string(),
            held: snapshot.capabilities.clone(),
        });
    }

    let matched = if snapshot.capabilities.iter().any(|c| c == required) {
        required.to_string()
    } else {
        WILDCARD_CAPABILITY.to_string()
    };

    Ok(CapabilityResult {
        agent_pub_key: claims.agent_pub_key.clone(),
        doorway_id: snapshot.doorway_id.clone(),
        commitment_id: snapshot.commitment_id.clone(),
        matched_capability: matched,
        snapshot_age_secs: age,
    })
}

fn extract_token(req: &Request<Incoming>) -> Option<String> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    if let Some(token) = extract_token_from_header(auth_header) {
        return Some(token.to_string());
    }

    req.headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix("doorway_token=").map(|t| t.to_string())
            })
        })
}

fn verify_claims(token: &str, state: &AppState) -> Result<Claims, String> {
    let jwt = if state.args.dev_mode {
        JwtValidator::new_dev()
    } else {
        let secret = state
            .args
            .jwt_secret
            .as_ref()
            .ok_or_else(|| "jwt_secret unset".to_string())?;
        JwtValidator::new(secret.clone(), state.args.jwt_expiry_seconds)
            .map_err(|e| format!("jwt validator init failed: {:?}", e))?
    };

    let result = jwt.verify_token(token);
    if !result.valid {
        return Err(result.error.unwrap_or_else(|| "invalid token".to_string()));
    }
    result.claims.ok_or_else(|| "missing claims".to_string())
}

fn snapshot_age(snapshot: &OperatorSnapshot) -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(snapshot.snapshot_taken_at);
    now.saturating_sub(snapshot.snapshot_taken_at)
}

/// Transitional helper for the auth migration window: returns true if a JWT
/// claim set carries either the new operator snapshot OR the legacy
/// Admin-level permission. Routes opting into the new model unconditionally
/// should NOT use this; it exists so a handful of /admin/* endpoints can
/// migrate incrementally without locking out tokens minted before task #15.
///
/// New code paths should call [`resolve_operator_capability`] directly. This
/// helper is expected to be removed once all operator JWTs carry snapshots.
pub fn legacy_admin_fallback(claims: &Claims) -> bool {
    claims.doorway_operator.is_some() || claims.permission_level >= PermissionLevel::Admin
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(capabilities: Vec<&str>, age_secs: u64) -> OperatorSnapshot {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        OperatorSnapshot {
            doorway_id: "alpha-elohim-host".to_string(),
            commitment_id: "cmt-test-1".to_string(),
            capabilities: capabilities.into_iter().map(str::to_string).collect(),
            succession_role: "deputy".to_string(),
            reach_scope: "operator-private".to_string(),
            dht_anchor_hash: "uhCkk-fake-hash".to_string(),
            // None during the schemaVersion=1 transition window. Once the
            // doorway has bootstrapped its custody + steward attestations,
            // tests should construct snapshots with concrete hashes here
            // and exercise the CustodyTransferred / StewardTransferred error
            // paths (added but not yet wired into resolve_operator_capability).
            custody_attestation_hash: None,
            steward_attestation_hash: None,
            snapshot_taken_at: now.saturating_sub(age_secs),
        }
    }

    #[test]
    fn wildcard_satisfies_any_capability() {
        let held = vec!["*".to_string()];
        assert!(capability_satisfies(&held, "federation.peers.mutate"));
        assert!(capability_satisfies(&held, "hosted_users.lifecycle"));
        // Even an unknown / future capability matches wildcard. By design —
        // the primary operator can do anything the schema admits.
        assert!(capability_satisfies(&held, "future.invented.capability"));
    }

    #[test]
    fn exact_match_required_when_no_wildcard() {
        let held = vec![
            "federation.peers.read".to_string(),
            "cache.read".to_string(),
        ];
        assert!(capability_satisfies(&held, "federation.peers.read"));
        assert!(!capability_satisfies(&held, "federation.peers.mutate"));
        assert!(!capability_satisfies(&held, "*"));
    }

    #[test]
    fn empty_held_satisfies_nothing() {
        assert!(!capability_satisfies(&[], "federation.peers.read"));
        // Even a wildcard request against empty held fails. Wildcard only
        // applies when the BINDING carries "*", not when the request asks
        // for "*".
        assert!(!capability_satisfies(&[], "*"));
    }

    #[test]
    fn snapshot_age_reports_seconds_since_taken_at() {
        let snap = snapshot(vec!["*"], 30);
        let age = snapshot_age(&snap);
        // Allow 5s slack for test scheduling.
        assert!(age >= 30 && age < 35, "expected ~30s, got {}", age);
    }

    #[test]
    fn legacy_fallback_recognises_admin_or_snapshot() {
        let mut claims = Claims {
            human_id: "h-1".to_string(),
            agent_pub_key: "agent-1".to_string(),
            identifier: "test@example.com".to_string(),
            permission_level: PermissionLevel::Public,
            doorway_operator: None,
            session_id: None,
            doorway_id: None,
            doorway_url: None,
            conductor_id: None,
            installed_app_id: None,
            is_steward: false,
            has_local_conductor: false,
            version: 1,
            iat: 0,
            exp: u64::MAX,
        };
        assert!(!legacy_admin_fallback(&claims));

        // Path A: legacy Admin permission_level.
        claims.permission_level = PermissionLevel::Admin;
        assert!(legacy_admin_fallback(&claims));

        // Path B: new operator snapshot, no Admin permission_level.
        claims.permission_level = PermissionLevel::Authenticated;
        claims.doorway_operator = Some(snapshot(vec!["federation.peers.read"], 0));
        assert!(legacy_admin_fallback(&claims));
    }

    #[test]
    fn capability_error_variants_carry_diagnostic_payload() {
        // Sanity that the error structures carry context for handler logging.
        let err = CapabilityError::CapabilityMissing {
            required: "graduation.force".to_string(),
            held: vec!["cache.read".to_string()],
        };
        match err {
            CapabilityError::CapabilityMissing { required, held } => {
                assert_eq!(required, "graduation.force");
                assert_eq!(held, vec!["cache.read".to_string()]);
            }
            _ => panic!("variant mismatch"),
        }
    }

    #[test]
    fn custody_and_steward_transferred_variants_carry_hash_pair() {
        // Forensic logging needs both the previous and current attestation
        // hashes so a handler can log the supersession event clearly.
        let err = CapabilityError::CustodyTransferred {
            previous_hash: "uhCkk-A1".to_string(),
            current_hash: "uhCkk-A2".to_string(),
        };
        match err {
            CapabilityError::CustodyTransferred {
                previous_hash,
                current_hash,
            } => {
                assert_eq!(previous_hash, "uhCkk-A1");
                assert_eq!(current_hash, "uhCkk-A2");
            }
            _ => panic!("variant mismatch"),
        }

        let err = CapabilityError::StewardTransferred {
            previous_hash: "uhCkk-S1".to_string(),
            current_hash: "uhCkk-S2".to_string(),
        };
        assert!(matches!(err, CapabilityError::StewardTransferred { .. }));
    }

    #[test]
    fn snapshot_chain_fields_optional_during_transition() {
        // schemaVersion=1 commitments produce snapshots with no chain
        // references. The auth resolver tolerates this — chain enforcement
        // is wired in the consumer-side landing (task #16) which queries the
        // attestation projection for current valid hashes.
        let snap = snapshot(vec!["*"], 10);
        assert_eq!(snap.custody_attestation_hash, None);
        assert_eq!(snap.steward_attestation_hash, None);
    }
}
