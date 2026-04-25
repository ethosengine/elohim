//! Portal-host discovery types for GET /auth/portal-host.
//!
//! The route handler lives in `crate::routes::auth_routes` (alongside all
//! other /auth/* handlers) so it can share private helpers (`get_jwt_validator`,
//! `json_response`, `BoxBody`, etc.).  This module exports the response type
//! so other modules (e.g. future Angular-facing services) can reference it
//! without importing from the routes layer.

use serde::Serialize;

/// Response for GET /auth/portal-host.
///
/// Returns the first reachable portal host registered for the authenticated
/// human.  When no hosts are registered or none responds within the probe
/// timeout, `reachable` is false and `host_url` is None.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalHostResponse {
    /// True when at least one host URL responded to a HEAD /healthz probe
    /// within the 1 s timeout.
    pub reachable: bool,
    /// The first reachable host URL, or None when all hosts are unreachable.
    pub host_url: Option<String>,
    /// All registered host URLs for this human (ordered most-recent first).
    pub all_hosts: Vec<String>,
}
