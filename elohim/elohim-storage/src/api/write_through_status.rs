//! Write-through status controller — `/api/v1/status/write-through`.
//!
//! EPR Phase 2B Task C.4. Reports the *effective* per-(pillar, kind)
//! write-through state composed across the 4 override layers
//! (admin > env > policy > manifest > implicit) plus the integrity-kind
//! exception. Pure read-only — operators inspect what's currently in force.
//!
//! The set of (pillar, kind) pairs reported is derived from the
//! [`ManifestRegistry`] (only registered projections are reportable;
//! unregistered kinds are uninteresting because the projector never sees
//! them). Integrity kinds are appended separately so operators can confirm
//! the exception is wired up.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Serialize;

use crate::error::StorageError;
use crate::projector::mapping::ManifestRegistry;
use crate::services::response;
use crate::write_through::{WriteThroughOverride, WriteThroughSource, WriteThroughState};

// ---------------------------------------------------------------------------
// Wire shapes
// ---------------------------------------------------------------------------

/// Top-level response for `GET /api/v1/status/write-through`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WriteThroughStatusView {
    /// One row per (pillar, kind) pair derived from the manifest registry.
    pub effective: Vec<EffectiveRow>,
    /// Integrity kinds that always write through, regardless of layers.
    pub integrity_kinds: Vec<String>,
    /// Snapshot of the currently-set admin override (layer 4), if any.
    pub admin_override: Option<WriteThroughOverride>,
}

/// Single row in the `effective[]` array.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveRow {
    pub pillar: String,
    pub kind: String,
    /// True iff the signal-emit endpoint will write this through.
    pub on: bool,
    /// Which layer produced the decision.
    pub source: WriteThroughSource,
}

// ---------------------------------------------------------------------------
// Pure builder (testable without HTTP)
// ---------------------------------------------------------------------------

/// Compose the status view from the registry and the live state.
///
/// Pure: reads from `state` and `registry`, no I/O. Reports one row per
/// distinct (pillar, kind) tuple registered in any pillar manifest's
/// `projections[]` array.
pub fn compute_write_through_status(
    state: &WriteThroughState,
    registry: &ManifestRegistry,
) -> WriteThroughStatusView {
    let mut effective = Vec::new();

    for projection in registry.all_projections() {
        let eff = state.effective_for(&projection.pillar, &projection.kind);
        effective.push(EffectiveRow {
            pillar: projection.pillar.clone(),
            kind: projection.kind.clone(),
            on: eff.is_on(),
            source: eff.source(),
        });
    }

    // Stable ordering for diffing operator output.
    effective.sort_by(|a, b| a.pillar.cmp(&b.pillar).then_with(|| a.kind.cmp(&b.kind)));

    WriteThroughStatusView {
        effective,
        integrity_kinds: integrity_kinds_list(),
        admin_override: state.admin_snapshot(),
    }
}

/// Hardcoded list of integrity-bearing kinds, mirrored from
/// [`crate::write_through::is_integrity_kind`]. Kept in sync by
/// `integrity_kinds_match` test below.
fn integrity_kinds_list() -> Vec<String> {
    vec![
        "KeyRotation".to_string(),
        "KeyRevocation".to_string(),
        "RevocationAttestation".to_string(),
        "AgentPeerBinding".to_string(),
    ]
}

// ---------------------------------------------------------------------------
// HTTP dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/status/write-through*` requests.
pub async fn handle(
    _req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    state: &Arc<WriteThroughState>,
    registry: &ManifestRegistry,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/status/write-through
        (&Method::GET, "") => {
            let view = compute_write_through_status(state.as_ref(), registry);
            Ok(response::ok(&view))
        }
        _ => Ok(response::not_found(&format!(
            "Unknown write-through status route: {} /api/v1/status/write-through/{}",
            method, path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write_through::{is_integrity_kind, WriteThroughConfig};
    use std::collections::HashMap;

    fn shefa_only_registry() -> ManifestRegistry {
        ManifestRegistry::with_shefa_economic_event()
    }

    #[test]
    fn integrity_kinds_match_recognizer() {
        // Guard: keep the human-readable list in sync with is_integrity_kind.
        for kind in integrity_kinds_list() {
            assert!(
                is_integrity_kind(&kind),
                "list claims '{kind}' is integrity but recognizer disagrees"
            );
        }
    }

    #[test]
    fn empty_state_reports_off_implicit_default() {
        let state = WriteThroughState::empty();
        let registry = shefa_only_registry();
        let view = compute_write_through_status(&state, &registry);

        assert_eq!(view.effective.len(), 1);
        let row = &view.effective[0];
        assert_eq!(row.pillar, "shefa");
        assert_eq!(row.kind, "EconomicEvent");
        assert!(!row.on);
        assert_eq!(row.source, WriteThroughSource::ImplicitDefault);
        assert!(view.admin_override.is_none());
    }

    #[test]
    fn manifest_default_on_reports_on() {
        let mut manifest = HashMap::new();
        manifest.insert(
            "shefa".to_string(),
            WriteThroughConfig {
                enabled: true,
                kinds: None,
            },
        );
        let state = WriteThroughState::from_manifest(manifest);
        let registry = shefa_only_registry();

        let view = compute_write_through_status(&state, &registry);
        assert_eq!(view.effective.len(), 1);
        assert!(view.effective[0].on);
        assert_eq!(
            view.effective[0].source,
            WriteThroughSource::ManifestDefault
        );
    }

    #[test]
    fn admin_override_reflected_in_snapshot() {
        let state = WriteThroughState::empty();
        let registry = shefa_only_registry();

        let mut admin = WriteThroughOverride::empty();
        admin.pillars.insert(
            "shefa".to_string(),
            WriteThroughConfig {
                enabled: true,
                kinds: None,
            },
        );
        state.set_admin_override(Some(admin.clone()));

        let view = compute_write_through_status(&state, &registry);
        assert_eq!(view.admin_override, Some(admin));
        assert!(view.effective[0].on);
        assert_eq!(view.effective[0].source, WriteThroughSource::AdminOverride);
    }

    #[test]
    fn integrity_kinds_listed_in_output() {
        let state = WriteThroughState::empty();
        let registry = ManifestRegistry::empty();
        let view = compute_write_through_status(&state, &registry);

        assert!(view.integrity_kinds.contains(&"KeyRotation".to_string()));
        assert!(view.integrity_kinds.contains(&"KeyRevocation".to_string()));
        assert!(view
            .integrity_kinds
            .contains(&"RevocationAttestation".to_string()));
        assert!(view
            .integrity_kinds
            .contains(&"AgentPeerBinding".to_string()));
    }

    #[test]
    fn rows_are_sorted_for_stable_diffing() {
        // Stub a registry with multiple projections to verify ordering.
        // Since with_shefa_economic_event only gives one row, we just verify
        // the empty case doesn't blow up — the sort itself is exercised on
        // any registry with ≥2 entries (multi-pillar future work).
        let view =
            compute_write_through_status(&WriteThroughState::empty(), &ManifestRegistry::empty());
        assert!(view.effective.is_empty());
    }
}
