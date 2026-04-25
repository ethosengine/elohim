//! Write-through admin controller — `POST /admin/write-through`.
//!
//! EPR Phase 2B Task C.6 layer 4. Mutates the live admin override layer of
//! the composed `WriteThroughState`. Routed under `/admin/*` (mirroring the
//! existing extraction-cache admin endpoints) so doorway proxies do not
//! expose it to browser clients — admin endpoints are operator-local.
//!
//! Request body shape: `WriteThroughOverride`
//! ```json
//! {
//!   "pillars": {
//!     "shefa": { "enabled": true, "kinds": ["EconomicEvent"] },
//!     "lamad": { "enabled": false }
//!   }
//! }
//! ```
//!
//! POST replaces the entire admin override (idempotent, declarative). To
//! clear the override layer entirely, POST `{ "pillars": {} }`.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, header, Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::StorageError;
use crate::services::response;
use crate::write_through::{WriteThroughOverride, WriteThroughState};

// ---------------------------------------------------------------------------
// Wire shapes
// ---------------------------------------------------------------------------

/// Response from `POST /admin/write-through`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminApplyResponse {
    /// Number of pillar entries the override now contains (0 when cleared).
    pub pillar_count: usize,
    /// Echoed snapshot of what was just applied — useful for audit logging.
    pub applied: WriteThroughOverride,
}

#[derive(Debug, Deserialize)]
struct ApplyBody(WriteThroughOverride);

// ---------------------------------------------------------------------------
// Pure builder (testable without HTTP)
// ---------------------------------------------------------------------------

/// Apply an admin override to the state, returning a status response.
///
/// Pure modulo the state's interior `RwLock` mutation. Callers test this
/// with a dedicated `WriteThroughState` and assert the snapshot reflects
/// the apply.
pub fn apply_admin_override(
    state: &WriteThroughState,
    override_in: WriteThroughOverride,
) -> AdminApplyResponse {
    let pillar_count = override_in.pillars.len();
    let applied = override_in.clone();
    if pillar_count == 0 {
        // Zero-pillar request = clear the layer entirely.
        state.set_admin_override(None);
    } else {
        state.set_admin_override(Some(override_in));
    }
    AdminApplyResponse {
        pillar_count,
        applied,
    }
}

// ---------------------------------------------------------------------------
// HTTP dispatcher
// ---------------------------------------------------------------------------

/// Handle `/admin/write-through` requests.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    state: &Arc<WriteThroughState>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::POST {
        return Ok(response::method_not_allowed());
    }

    let body_bytes = match req.into_body().collect().await {
        Ok(b) => b.to_bytes(),
        Err(e) => return Ok(response::bad_request(&format!("body read failed: {e}"))),
    };

    let parsed: ApplyBody = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return Ok(response::bad_request(&format!(
                "invalid WriteThroughOverride JSON: {e}"
            )))
        }
    };

    let result = apply_admin_override(state.as_ref(), parsed.0);
    info!(
        pillar_count = result.pillar_count,
        "admin write-through override applied"
    );
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(
            serde_json::to_string(&result).unwrap_or_default(),
        )))
        .unwrap())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write_through::{WriteThroughConfig, WriteThroughSource};
    use std::collections::HashMap;

    fn make_override(pillar: &str, enabled: bool) -> WriteThroughOverride {
        let mut pillars = HashMap::new();
        pillars.insert(
            pillar.to_string(),
            WriteThroughConfig {
                enabled,
                kinds: None,
            },
        );
        WriteThroughOverride { pillars }
    }

    #[test]
    fn apply_admin_override_sets_layer_4() {
        let state = WriteThroughState::empty();
        let result = apply_admin_override(&state, make_override("shefa", true));
        assert_eq!(result.pillar_count, 1);
        // Effective state for shefa now resolves via admin.
        let eff = state.effective_for("shefa", "EconomicEvent");
        assert!(eff.is_on());
        assert_eq!(eff.source(), WriteThroughSource::AdminOverride);
    }

    #[test]
    fn apply_admin_override_replaces_previous() {
        let state = WriteThroughState::empty();
        let _ = apply_admin_override(&state, make_override("shefa", true));
        let _ = apply_admin_override(&state, make_override("lamad", false));
        // shefa is no longer in the admin override (lamad replaced the whole thing).
        let snap = state.admin_snapshot().expect("admin set");
        assert!(snap.pillars.contains_key("lamad"));
        assert!(!snap.pillars.contains_key("shefa"));
    }

    #[test]
    fn apply_admin_override_with_empty_pillars_clears_layer() {
        let state = WriteThroughState::empty();
        let _ = apply_admin_override(&state, make_override("shefa", true));
        assert!(state.admin_snapshot().is_some());
        let cleared = apply_admin_override(&state, WriteThroughOverride::empty());
        assert_eq!(cleared.pillar_count, 0);
        assert!(state.admin_snapshot().is_none());
    }
}
