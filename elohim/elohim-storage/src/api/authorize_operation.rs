//! HTTP handler for `POST /api/v1/authorize-operation` (Che op-gate Slice 1).
//!
//! This is the storage-side endpoint that the doorway's op-gate calls (via
//! direct HTTP, not via the public doorway proxy) to ask: "is performer
//! authorized to perform capability on this node?"
//!
//! ## Security boundary
//!
//! - `performer` is read from the **request body** — the doorway sets it from
//!   its verified JWT before forwarding.  Never from an `X-Agent-Cid` header
//!   (which is trivially spoofable).
//! - The endpoint is deliberately **NOT** in `build_manifest()` — that function
//!   auto-promotes routes to the public doorway proxy.  Exposing this as a
//!   doorway-proxied route would create a verdict-oracle DoS vector (Review C6).
//! - 200 + `allowed:false` is a verdict, not an error; 503 is reserved for
//!   infra failures.  The doorway's gate enforcement layer interprets the verdict.
//!
//! ## Wire format
//!
//! Request body (JSON):
//! ```json
//! {
//!   "performer": "uhCAk-matthew",
//!   "capability": "orchestrate-node",
//!   "targetEprId": null,        // optional; null maps to "*" (any EPR)
//!   "reach": "commons"
//! }
//! ```
//!
//! Response (always 200 for a verdict; 503 only on infra failure):
//! ```json
//! { "allowed": true,  "commitmentCid": "commitment:…", "reason": "ok" }
//! { "allowed": false, "commitmentCid": null,            "reason": "no active …" }
//! ```

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Request, Response};
use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::operation_authorization::{authorize_operation, AuthorizeOperationRequest};
use crate::services::response;

use super::parse_body;

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

/// Deserialized request body for `POST /api/v1/authorize-operation`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizeOperationInput {
    /// Agent CID of the requester — set by the doorway from its verified JWT.
    performer: String,
    /// REA action / capability name (e.g. `"orchestrate-node"`).
    capability: String,
    /// Target EPR identifier (optional; absent/null → wildcard `"*"`).
    target_epr_id: Option<String>,
    /// Reach level claimed (must be ≤ `bounds.reach_ceiling`).
    reach: String,
}

/// Response body for `POST /api/v1/authorize-operation`.
///
/// Always 200 for a verdict (allowed:true or allowed:false).
/// camelCase per the Rust-to-TypeScript boundary convention.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizeOperationResponse {
    allowed: bool,
    commitment_cid: Option<String>,
    reason: String,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// `POST /api/v1/authorize-operation`
///
/// Delegates to [`authorize_operation`] with the wall-clock `now` as `signed_at`.
/// Always returns **200 + verdict** for both allowed and denied outcomes:
/// `authorize_operation` never returns `Err` — every infra failure (pool
/// unavailable, fetch breakdown) maps to `allowed:false` with a diagnostic
/// `reason`, so the gate is fail-closed and the handler still answers 200. The
/// only non-200 path is a malformed request body, surfaced by `parse_body`.
///
/// Deliberately **NOT** registered in `build_manifest()` (Review C6).
pub async fn handle(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: AuthorizeOperationInput = parse_body(req).await?;

    let signed_at = chrono::Utc::now().to_rfc3339();

    let result = authorize_operation(
        pool,
        AuthorizeOperationRequest {
            performer: input.performer,
            capability: input.capability,
            target_epr_id: input.target_epr_id,
            reach: input.reach,
        },
        signed_at,
    )
    .await;

    Ok(response::ok(&AuthorizeOperationResponse {
        allowed: result.allowed,
        commitment_cid: result.commitment_cid,
        reason: result.reason,
    }))
}
