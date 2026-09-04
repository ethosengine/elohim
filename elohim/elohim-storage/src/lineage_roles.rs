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

/// What OPENED a window — the three addresses the revert trigger (Task 13a)
/// has to re-read, recorded at open time because nothing else survives to
/// name them later.
///
/// # Why it lives on the window and not on the applied release
///
/// The applied-release row (`release_adoption::state::AppliedRelease`) is
/// keyed by CHANNEL; a window is keyed by ROLE. Going role → release →
/// commitment needs a link, and the honest place for it is the record the
/// crossing itself minted. The controller's revert sweep can then be
/// self-contained on [`LineageRoles`]: it reads the window, re-reads the
/// path that authorised it, and decides — with no channel-to-role table to
/// keep in step.
///
/// `Option` on the window, not on the fields: a window opened by a build
/// that predates this struct has no origin at all, and the sweep skips it
/// rather than guessing which commitment to re-read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowOrigin {
    /// The release channel whose head elected the crossing. The re-election
    /// arm of the revert trigger re-resolves THIS channel.
    pub channel_id: String,
    /// The release (the winning version's action hash) that opened the
    /// window. A head that has moved off this cid is the re-election shape.
    pub release_cid: String,
    /// The `migrates-lineage` commitment the release's
    /// `adoptionDiscipline.path` named — its ENTRY hash, as
    /// `PathEvidence::commitment_cid` renders it. Re-read every sweep while
    /// the window is open; `revoked` on it is the revert trigger.
    pub path_commitment_cid: String,
}

/// Per-role lineage state: which app id currently serves READS, which
/// serves AUTHORING (writes), and whether the window has been permanently
/// closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleLineage {
    pub reading_app_id: String,
    pub authoring_app_id: String,
    pub closed: bool,
    /// **Task 13a.** What opened this window, or `None` for a role that was
    /// never crossed (and for a window opened without an origin — see
    /// [`WindowOrigin`]). Carried THROUGH a revert and a sunset rather than
    /// cleared: the receipt an operator reads afterwards has to be able to
    /// name the path that was revoked.
    pub origin: Option<WindowOrigin>,
}

impl RoleLineage {
    fn at_base(base_app_id: &str) -> Self {
        Self {
            reading_app_id: base_app_id.to_string(),
            authoring_app_id: base_app_id.to_string(),
            closed: false,
            origin: None,
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
    ///
    /// `origin` records what authorised the crossing (Task 13a). It is
    /// `Option` rather than required because a fixture may open a window
    /// with no release behind it at all; a window with no origin is simply
    /// one the revert sweep cannot re-read, and it says so rather than
    /// guessing.
    pub fn open_window(&self, role: &str, lineage_app_id: &str, origin: Option<WindowOrigin>) {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let entry = inner
            .entry(role.to_string())
            .or_insert_with(|| RoleLineage::at_base(&self.base_app_id));
        entry.reading_app_id = self.base_app_id.clone();
        entry.authoring_app_id = lineage_app_id.to_string();
        entry.closed = false;
        entry.origin = origin;
    }

    /// Every role whose window is currently OPEN, paired with its state.
    ///
    /// **The ONE definition of "open".** The predicate is
    /// `!closed && reading == base && authoring != base` — exactly the shape
    /// [`Self::open_window`] builds, and all three clauses are load-bearing.
    /// The tempting shorthand `authoring != reading && !closed` is WRONG:
    /// [`Self::revert`] leaves the two ids still different but INVERTED (the
    /// side app moves into `reading_app_id` as a historical pointer while
    /// authoring returns to base), so a reverted role would match it. A
    /// SUNSET role is excluded by `!closed`.
    ///
    /// It lives here, on the resolver, because two callers now need the same
    /// answer — the trailing bridge sweep
    /// (`crate::services::lineage_bridge`) and the revert sweep
    /// (`crate::services::release_adoption::watch`) — and two copies of a
    /// three-clause predicate is exactly how the inverted-ids case gets
    /// re-introduced.
    pub fn open_windows(&self) -> Vec<(String, RoleLineage)> {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|(_, lineage)| {
                !lineage.closed
                    && lineage.reading_app_id == self.base_app_id
                    && lineage.authoring_app_id != self.base_app_id
            })
            .map(|(role, lineage)| (role.clone(), lineage.clone()))
            .collect()
    }

    /// Roll AUTHORING back to the base app id. The CURRENT authoring app id
    /// (the lineage app, while a window is open) moves into `reading_app_id`
    /// first — a disabled cell: a historical read pointer at the lineage
    /// app, kept for reference, no longer live for writes.
    ///
    /// # What `/version` shows afterwards, and why it is not "untouched"
    ///
    /// The reverted shape is `authoring == base`, `reading == <side app>`,
    /// `closed == false` — the two ids DIFFER, so the passport keeps
    /// rendering a lineage view for the role rather than hiding it. That is
    /// deliberate and it is what the story asks for: *"james and matthew mark
    /// v1 authoring and v2 reading, disable their v2 cells, and uninstall
    /// nothing"* (`genesis/a2o/features/delivery/happ-lineage-migration.feature`,
    /// Station 7). A revert that also reset `reading_app_id` to base would
    /// make the role look never-crossed, which is the one thing an operator
    /// reading a reverted peer must not be told. The passport's
    /// hide-an-equal-ids-window rule
    /// (`crate::runtime_passport::lineage_view_for`) is about the UNTOUCHED
    /// single-cell shape, not about this one.
    ///
    /// [`RoleLineage::origin`] is carried through untouched: the receipt an
    /// operator reads after a revert has to be able to name the path that was
    /// revoked.
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
        l.open_window("node_registry", "elohim@EKiIscIk5BDd", None);
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
        l.open_window("node_registry", "elohim@EKiIscIk5BDd", None);
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
        l.open_window("node_registry", "elohim@AAA", None);
        l.open_window("lamad", "elohim@BBB", None);
        l.sunset("lamad");
        l.reset_all();
        for role in ["node_registry", "lamad"] {
            assert_eq!(l.app_id_for(role), "elohim");
            let snap = l.snapshot();
            assert_eq!(snap[role].reading_app_id, "elohim");
            assert!(!snap[role].closed);
        }
    }

    fn origin() -> WindowOrigin {
        WindowOrigin {
            channel_id: "runtime:coordinators:elohim:commons".to_string(),
            release_cid: "uhCkkRELEASE".to_string(),
            path_commitment_cid: "uhCEkPATH".to_string(),
        }
    }

    /// **Task 13a, the ONE definition of open.** `revert` leaves the two app
    /// ids DIFFERENT but inverted, and the shorthand
    /// `authoring != reading && !closed` would read that as still-open. The
    /// predicate must exclude it, and must exclude a sunset role too.
    #[test]
    fn open_windows_excludes_reverted_and_sunset_roles() {
        let l = LineageRoles::new("elohim", &["node_registry", "lamad"]);
        assert!(l.open_windows().is_empty());

        l.open_window("node_registry", "elohim@AAA", Some(origin()));
        l.open_window("lamad", "elohim@BBB", Some(origin()));
        assert_eq!(
            l.open_windows()
                .into_iter()
                .map(|(role, _)| role)
                .collect::<Vec<_>>(),
            vec!["lamad".to_string(), "node_registry".to_string()]
        );

        // Reverted: authoring back at base, reading left on the side app —
        // still two DIFFERENT ids, and still not open.
        l.revert("node_registry");
        // Sunset: ids untouched, `closed` set — also not open.
        l.sunset("lamad");
        assert!(
            l.open_windows().is_empty(),
            "a reverted role and a sunset role are both closed windows"
        );
    }

    /// **Task 13a.** The reverted shape, asserted against the story's own
    /// words: *"james and matthew mark v1 authoring and v2 reading"*. Reading
    /// is NOT rolled back to base — the side app stays as a historical
    /// pointer, so the passport keeps rendering the role's lineage view
    /// rather than reporting it never-crossed.
    #[test]
    fn revert_marks_v1_authoring_and_v2_reading_and_keeps_the_origin() {
        let l = LineageRoles::new("elohim", &["node_registry"]);
        l.open_window("node_registry", "elohim@AAA", Some(origin()));
        l.revert("node_registry");

        let snap = l.snapshot();
        let role = &snap["node_registry"];
        assert_eq!(role.authoring_app_id, "elohim", "v1 authoring");
        assert_eq!(role.reading_app_id, "elohim@AAA", "v2 reading");
        assert!(!role.closed, "a revert is not a sunset");
        assert_eq!(
            role.origin.as_ref().map(|o| o.path_commitment_cid.as_str()),
            Some("uhCEkPATH"),
            "the receipt an operator reads has to be able to name the revoked path"
        );
        // The two ids differ, so `runtime_passport::lineage_view_for`'s
        // hide-an-equal-ids-window rule does NOT fire here — the role stays
        // visible as crossed-then-reverted.
        assert_ne!(role.authoring_app_id, role.reading_app_id);
    }

    #[test]
    fn a_role_never_named_at_new_still_opens_a_real_window() {
        // A role absent from the `roles` list passed to `new` (e.g. added
        // later) still gets a real entry the first time a window opens on
        // it, rather than silently no-op'ing.
        let l = LineageRoles::new("elohim", &[]);
        l.open_window("node_registry", "elohim@CCC", None);
        assert_eq!(l.app_id_for("node_registry"), "elohim@CCC");
        assert_eq!(l.snapshot().len(), 1);
    }
}
