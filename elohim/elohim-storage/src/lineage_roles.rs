//! `LineageRoles` — per-role happ-app-id resolver (Holochain Evolution Epic
//! MVP, Task 6).
//!
//! During a mishpat-rung happ lineage upgrade, a role's zome calls need to
//! route to a candidate ("lineage") app id for AUTHORING while reads may
//! stay pinned to the base app id — a window the apply vehicle (Task 7)
//! opens and later reverts or sunsets. `HcClientRegistry::connect_role` /
//! `connect_role_forever` consult this resolver instead of a hard-coded
//! `app_id` string, so every `HcClient` this crate builds for a role is
//! routed through ONE place.
//!
//! With NO window ever opened, [`LineageRoles::app_id_for`] always resolves
//! to the base app id for every role — byte-for-byte identical to the
//! `inputs.app_id.clone()` this resolver replaces. Opening, reverting, and
//! sunsetting a window are each one flag flip on a `BTreeMap` entry; nothing
//! here talks to a conductor, a DHT, or governance — those live upstream in
//! Task 7's apply vehicle and the mishpat zome.

use std::collections::BTreeMap;
use std::sync::RwLock;

/// Per-role lineage state: which app id currently serves READS, which
/// serves AUTHORING (writes), and whether the window has been permanently
/// closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleLineage {
    pub reading_app_id: String,
    pub authoring_app_id: String,
    pub closed: bool,
}

impl RoleLineage {
    fn at_base(base_app_id: &str) -> Self {
        Self {
            reading_app_id: base_app_id.to_string(),
            authoring_app_id: base_app_id.to_string(),
            closed: false,
        }
    }
}

/// Role-keyed resolver of which happ app id a role's zome calls should
/// route to. Interior-mutable (`RwLock`, same pattern as
/// [`crate::hc_client_registry::HcClientRegistry`]'s slots) so the apply
/// vehicle can flip a role's routing without threading `&mut` through the
/// HTTP/bridge layers.
#[derive(Debug)]
pub struct LineageRoles {
    base_app_id: String,
    inner: RwLock<BTreeMap<String, RoleLineage>>,
}

impl LineageRoles {
    /// Every named role starts with `reading_app_id == authoring_app_id ==
    /// base_app_id`, `closed == false`. A role NOT named here still
    /// resolves correctly through `app_id_for` (falls back to
    /// `base_app_id`) — the map only needs an entry once a window is
    /// opened on that role.
    pub fn new(base_app_id: &str, roles: &[&str]) -> Self {
        let mut inner = BTreeMap::new();
        for role in roles {
            inner.insert((*role).to_string(), RoleLineage::at_base(base_app_id));
        }
        Self {
            base_app_id: base_app_id.to_string(),
            inner: RwLock::new(inner),
        }
    }

    /// The AUTHORING app id for `role` — what a `call_zome` for this role
    /// should route to. An unknown role (never registered at `new`, never
    /// opened) resolves to `base_app_id`; this never panics and never
    /// errors, matching the pre-Task-6 unconditional `app_id.clone()`.
    pub fn app_id_for(&self, role: &str) -> String {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .map(|lineage| lineage.authoring_app_id.clone())
            .unwrap_or_else(|| self.base_app_id.clone())
    }

    /// Open a lineage window on `role`: reads stay pinned to the base app
    /// id, authoring (writes) move to `lineage_app_id`. Un-closes the role
    /// if a prior `sunset` had closed it — opening a fresh window
    /// supersedes a closed one.
    pub fn open_window(&self, role: &str, lineage_app_id: &str) {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let entry = inner
            .entry(role.to_string())
            .or_insert_with(|| RoleLineage::at_base(&self.base_app_id));
        entry.reading_app_id = self.base_app_id.clone();
        entry.authoring_app_id = lineage_app_id.to_string();
        entry.closed = false;
    }

    /// Roll AUTHORING back to the base app id. The CURRENT authoring app id
    /// (the lineage app, while a window is open) moves into `reading_app_id`
    /// first — a disabled cell: a historical read pointer at the lineage
    /// app, kept for reference, no longer live for writes.
    pub fn revert(&self, role: &str) {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let entry = inner
            .entry(role.to_string())
            .or_insert_with(|| RoleLineage::at_base(&self.base_app_id));
        entry.reading_app_id = entry.authoring_app_id.clone();
        entry.authoring_app_id = self.base_app_id.clone();
    }

    /// Terminally close the lineage window on `role`. Unlike `revert`,
    /// AUTHORING is left exactly where it was (typically the lineage app
    /// id) — sunset marks the window closed without rolling authorship
    /// back to base.
    pub fn sunset(&self, role: &str) {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let entry = inner
            .entry(role.to_string())
            .or_insert_with(|| RoleLineage::at_base(&self.base_app_id));
        entry.closed = true;
    }

    /// Reset every currently-tracked role back to the base app id
    /// (`reading_app_id == authoring_app_id == base_app_id`,
    /// `closed == false`) in one call. Backs the operator-facing
    /// `/admin/lineage/reset` route (Task 10 part 2).
    pub fn reset_all(&self) {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        for lineage in inner.values_mut() {
            *lineage = RoleLineage::at_base(&self.base_app_id);
        }
    }

    /// A point-in-time copy of every tracked role's lineage state, for the
    /// passport (Task 8).
    pub fn snapshot(&self) -> BTreeMap<String, RoleLineage> {
        self.inner.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_base() {
        let l = LineageRoles::new("elohim", &["node_registry"]);
        assert_eq!(l.app_id_for("node_registry"), "elohim");
        assert_eq!(l.app_id_for("unknown"), "elohim");
    }

    #[test]
    fn window_then_revert() {
        let l = LineageRoles::new("elohim", &["node_registry"]);
        l.open_window("node_registry", "elohim@EKiIscIk5BDd");
        assert_eq!(l.app_id_for("node_registry"), "elohim@EKiIscIk5BDd");
        l.revert("node_registry");
        assert_eq!(l.app_id_for("node_registry"), "elohim");
        assert_eq!(
            l.snapshot()["node_registry"].reading_app_id,
            "elohim@EKiIscIk5BDd"
        );
    }

    #[test]
    fn sunset_closes_without_reverting_authoring() {
        let l = LineageRoles::new("elohim", &["node_registry"]);
        l.open_window("node_registry", "elohim@EKiIscIk5BDd");
        l.sunset("node_registry");
        let snap = l.snapshot();
        let role = &snap["node_registry"];
        assert!(role.closed);
        // Sunset does NOT revert authorship the way `revert` does — the
        // lineage app id keeps authoring even after the window is closed.
        assert_eq!(role.authoring_app_id, "elohim@EKiIscIk5BDd");
        // Reading stays exactly where `open_window` left it (pinned to
        // base) — sunset touches only `closed`, neither app id.
        assert_eq!(role.reading_app_id, "elohim");
        assert_eq!(l.app_id_for("node_registry"), "elohim@EKiIscIk5BDd");
    }

    #[test]
    fn reset_all_returns_every_tracked_role_to_base() {
        let l = LineageRoles::new("elohim", &["node_registry", "lamad"]);
        l.open_window("node_registry", "elohim@AAA");
        l.open_window("lamad", "elohim@BBB");
        l.sunset("lamad");
        l.reset_all();
        for role in ["node_registry", "lamad"] {
            assert_eq!(l.app_id_for(role), "elohim");
            let snap = l.snapshot();
            assert_eq!(snap[role].reading_app_id, "elohim");
            assert!(!snap[role].closed);
        }
    }

    #[test]
    fn a_role_never_named_at_new_still_opens_a_real_window() {
        // A role absent from the `roles` list passed to `new` (e.g. added
        // later) still gets a real entry the first time a window opens on
        // it, rather than silently no-op'ing.
        let l = LineageRoles::new("elohim", &[]);
        l.open_window("node_registry", "elohim@CCC");
        assert_eq!(l.app_id_for("node_registry"), "elohim@CCC");
        assert_eq!(l.snapshot().len(), 1);
    }
}
