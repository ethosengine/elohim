//! App context for multi-tenant database operations
//!
//! All database operations are scoped by h_app_id to enable multiple apps
//! to store content in the same database without interference.

/// Canonical `h_app_id` scope for the `humans` projection.
///
/// Humans are the **imagodei** (identity) pillar's projection — production
/// writers (`api/identity.rs::register_human`, `services/genesis_self_heal.rs`)
/// write them under this scope, and the membership projection
/// (`reconcile/controller.rs`) UPDATEs `household_id` onto those rows in place.
///
/// A reader that joins/filters `humans` for IDENTITY or HOUSEHOLD data MUST scope
/// by this constant — NOT by the operating app scope (`"lamad"` for content
/// distribution). Filtering humans by `"lamad"` silently empties the join (the
/// 2026-06-19 dark-card probe artifact; see
/// `backlog/resilience-card-membership-humans-projection-gap-2026-06-19.md`).
/// This is the single source of truth so writers and readers cannot drift
/// ("decide ONE scope, flip both together").
pub const HUMANS_HAPP_ID: &str = "imagodei";

/// App context passed to all database operations for multi-tenant isolation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppContext {
    /// Holochain application identifier for scoping database operations
    pub h_app_id: String,
    /// Local libp2p PeerId (base58-encoded) for self-dedup in FederatedEprStore.
    /// None when no P2P swarm is configured (tests, Tauri-only builds).
    pub local_libp2p_peer_id: Option<String>,
}

impl AppContext {
    /// Create a new app context with the specified Holochain app ID
    pub fn new(h_app_id: impl Into<String>) -> Self {
        Self {
            h_app_id: h_app_id.into(),
            local_libp2p_peer_id: None,
        }
    }

    /// Default context for learning content (paths, concepts, quizzes)
    pub fn default_lamad() -> Self {
        Self::new("lamad")
    }

    /// Default context for shared infrastructure (resources, sensemaking)
    pub fn default_elohim() -> Self {
        Self::new("elohim")
    }

    /// Get the h_app_id as a string reference
    pub fn h_app_id(&self) -> &str {
        &self.h_app_id
    }
}

impl Default for AppContext {
    /// Defaults to lamad for backwards compatibility with existing content
    fn default() -> Self {
        Self::default_lamad()
    }
}

impl std::fmt::Display for AppContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppContext({})", self.h_app_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_contexts() {
        assert_eq!(AppContext::default_lamad().h_app_id, "lamad");
        assert_eq!(AppContext::default_elohim().h_app_id, "elohim");
        assert_eq!(AppContext::default().h_app_id, "lamad");
    }

    #[test]
    fn test_custom_context() {
        let ctx = AppContext::new("calendar");
        assert_eq!(ctx.h_app_id, "calendar");
    }

    #[test]
    fn humans_happ_id_is_imagodei() {
        // Humans are the imagodei pillar's projection. Production writers
        // (api/identity.rs register_human; services/genesis_self_heal.rs) write this
        // scope; every household-join reader MUST filter humans by it (not the
        // operating content scope "lamad"), or the join silently empties.
        assert_eq!(super::HUMANS_HAPP_ID, "imagodei");
    }
}
