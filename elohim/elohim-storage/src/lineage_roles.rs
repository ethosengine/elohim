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
    /// **Task 33 — the V1 BINDING.** Which installed app holds the
    /// PREDECESSOR cell this role's crossing reads from, carries out of, and
    /// ultimately SEALS. `None` means the base app, which is the only shape
    /// a household peer ever has.
    ///
    /// # Why this is a declaration and not a derivation
    ///
    /// Before this field the vehicle resolved v1 STRUCTURALLY —
    /// `HappLineageVehicle::seal_close` took `role_cell(&base_app_info, role)`
    /// with `base_app_id` fixed at boot — so a sunset could only ever close
    /// the household's OWN node-registry chain. A close is irreversible and a
    /// post-close write earns a permanent block from every neighbour, so the
    /// a2o rehearsal of the sunset spent the household's real chain every time
    /// it ran (Task 30's measurement: matthew seq 1052, jessica seq 732, both
    /// warranted "No more actions are allowed after a chain close").
    ///
    /// Binding v1 makes the crossing DISPOSABLE: a fixture installs a
    /// run-scoped predecessor (a per-run network seed on the same DNA,
    /// installed beside the base app under the same key), binds it here, and
    /// the whole ceremony — install, carry, window, seal — happens on a chain
    /// nobody needs afterwards. The household's base cell is only ever READ.
    ///
    /// `None` rather than a copy of `base_app_id`: the default has to be
    /// indistinguishable from the pre-Task-33 state on every surface that
    /// projects this struct, and an `Option` that is absent cannot drift from
    /// a base app id that moves.
    pub v1_app_id: Option<String>,
    /// **Task 13a.** What opened this window, or `None` for a role that was
    /// never crossed (and for a window opened without an origin — see
    /// [`WindowOrigin`]). Carried THROUGH a revert and a sunset rather than
    /// cleared: the receipt an operator reads afterwards has to be able to
    /// name the path that was revoked.
    pub origin: Option<WindowOrigin>,
}

/// What one [`LineageRoles::reset_all`] call actually did — the two lists
/// `/admin/lineage/reset` renders back to its caller, so a reset that left a
/// sunset role alone says so instead of reporting a silent success.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResetReport {
    /// Roles rolled back to the base app id by this call.
    pub reset: Vec<String>,
    /// Roles left EXACTLY as they were because their lineage window is
    /// closed (sunset). Empty whenever `force_closed` was passed.
    pub skipped_closed: Vec<String>,
    /// **Task 33.** Roles whose V1 BINDING this call dropped, paired with the
    /// app id they were bound to. Reported rather than silent because a
    /// binding is a run-scoped fixture declaration: a baseline reset that
    /// quietly returned a role to the household's own chain — the chain the
    /// binding exists to keep out of the ceremony — would be the one silent
    /// step back toward spending it.
    pub cleared_v1_bindings: Vec<(String, String)>,
}

impl RoleLineage {
    /// This role's PREDECESSOR app id: the binding when one is declared, the
    /// base app otherwise. The one place the default is applied, so no caller
    /// can spell it differently.
    pub fn v1_app_id_or<'a>(&'a self, base_app_id: &'a str) -> &'a str {
        self.v1_app_id.as_deref().unwrap_or(base_app_id)
    }

    fn at_base(base_app_id: &str) -> Self {
        Self {
            reading_app_id: base_app_id.to_string(),
            authoring_app_id: base_app_id.to_string(),
            closed: false,
            origin: None,
            v1_app_id: None,
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

    /// The app id this resolver falls back to for every role — the boot-time
    /// `HOLOCHAIN_APP_ID`.
    pub fn base_app_id(&self) -> &str {
        &self.base_app_id
    }

    /// **Task 33.** The PREDECESSOR app id for `role` — the app whose
    /// provisioned cell for this role is the v1 of a crossing: what the carry
    /// reads FROM, what the window's `reading_app_id` points at, and what a
    /// sunset SEALS.
    ///
    /// An unbound role (every role on every household peer) answers the base
    /// app id, so every caller that used `base_app_id` directly before this
    /// existed is byte-for-byte unchanged.
    pub fn v1_app_id_for(&self, role: &str) -> String {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .and_then(|lineage| lineage.v1_app_id.clone())
            .unwrap_or_else(|| self.base_app_id.clone())
    }

    /// **Task 33.** Declare (or clear, with `None`) the v1 binding for `role`.
    /// Returns the binding that was in force before this call.
    ///
    /// # Why a bound role must be at BASE
    ///
    /// The binding decides which chain the ceremony reads, carries and
    /// finally CLOSES. Re-aiming it while a window is open would leave the
    /// carry having read one chain and the seal closing another — and the
    /// seal is the irreversible act. So this refuses BY NAME unless the role
    /// is in the untouched state (`authoring == reading == v1`, not closed),
    /// which is exactly the state `reset_all` converges to and the state a
    /// fixture is in when it stages a run-scoped predecessor.
    ///
    /// Refusing is a `String` and not a panic or a silent no-op because the
    /// caller is an HTTP route whose whole job is to say why.
    pub fn bind_v1(&self, role: &str, v1_app_id: Option<&str>) -> Result<Option<String>, String> {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let entry = inner
            .entry(role.to_string())
            .or_insert_with(|| RoleLineage::at_base(&self.base_app_id));
        let current_v1 = entry.v1_app_id_or(&self.base_app_id).to_string();
        if entry.closed {
            return Err(format!(
                "role '{role}' has a CLOSED lineage window (v1 '{current_v1}' is sealed) — the \
                 predecessor a sunset already closed cannot be re-aimed, and a closed chain is \
                 never crossed a second time"
            ));
        }
        if entry.authoring_app_id != current_v1 || entry.reading_app_id != current_v1 {
            return Err(format!(
                "role '{role}' has an OPEN lineage window (reading '{}', authoring '{}') — the \
                 v1 binding decides which chain the carry reads and the sunset SEALS, so \
                 re-aiming it mid-crossing would close a chain nothing was carried out of. \
                 Reset the role to base first.",
                entry.reading_app_id, entry.authoring_app_id
            ));
        }
        let previous = entry.v1_app_id.clone();
        entry.v1_app_id = v1_app_id.map(str::to_string);
        // The untouched state is defined against the PREDECESSOR, so moving
        // the binding moves both ids with it. Without this a bound role would
        // read as an open window (`reading != authoring` is not the test, but
        // `open_windows`' `reading == v1` is) the moment it was bound.
        let bound = entry.v1_app_id_or(&self.base_app_id).to_string();
        entry.reading_app_id = bound.clone();
        entry.authoring_app_id = bound;
        Ok(previous)
    }

    /// Open a lineage window on `role`: reads stay pinned to the role's V1
    /// app id (its binding, or the base app), authoring (writes) move to
    /// `lineage_app_id`. Un-closes the role if a prior `sunset` had closed
    /// it — opening a fresh window supersedes a closed one.
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
        entry.reading_app_id = entry.v1_app_id_or(&self.base_app_id).to_string();
        entry.authoring_app_id = lineage_app_id.to_string();
        entry.closed = false;
        entry.origin = origin;
    }

    /// Every role whose window is currently OPEN, paired with its state.
    ///
    /// **The ONE definition of "open".** The predicate is
    /// `!closed && reading == v1 && authoring != v1`, where `v1` is the role's
    /// binding or the base app (Task 33) — exactly the shape
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
                let v1 = lineage.v1_app_id_or(&self.base_app_id);
                !lineage.closed && lineage.reading_app_id == v1 && lineage.authoring_app_id != v1
            })
            .map(|(role, lineage)| (role.clone(), lineage.clone()))
            .collect()
    }

    /// Roll AUTHORING back to the role's V1 app id — its binding, or the base
    /// app (Task 33; unbound roles are byte-for-byte unchanged). The CURRENT authoring app id
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
        entry.authoring_app_id = entry.v1_app_id_or(&self.base_app_id).to_string();
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

    /// Reset currently-tracked roles back to the base app id
    /// (`reading_app_id == authoring_app_id == base_app_id`,
    /// `closed == false`) in one call. Backs the operator-facing
    /// `/admin/lineage/reset` route (Task 10 part 2).
    ///
    /// # A CLOSED role is skipped, and that is the point
    ///
    /// `RoleLineage::at_base` clears `closed` AND the v1 binding (Task 33 —
    /// a run-scoped predecessor belongs to the run that staged it, and a
    /// baseline is the household's own chain). Applied blindly, this call
    /// would therefore route a SUNSET role back to authoring on a chain that
    /// is permanently sealed — writes a remote authority refuses and
    /// warrants (Probe B2) — which is exactly what spec §4 step 5's *"at no
    /// point after"* forbids. So `force_closed == false` (the default, and
    /// the only shape an operator should reach for) leaves every role with
    /// `closed == true` untouched and NAMES it in
    /// [`ResetReport::skipped_closed`]; only the roles that were still open
    /// come back to base.
    ///
    /// `force_closed == true` is the fixture/operator seat: the a2o story's
    /// `Before`/`AfterAll` legitimately has to converge a rehearsal mesh to
    /// the v1 baseline after Station 8 has sealed a window, and that is a
    /// deliberate act on a rehearsal peer, never a reset an operator reaches
    /// for by accident. Note what it does NOT license: §7 C14's *"kept,
    /// never deleted"* is about the witnessed residual — the carried
    /// notarizations on the side app's chain. Re-opening a role's ROUTING is
    /// reversible; deleting that chain is not, which is why the uninstall
    /// arm of `/admin/lineage/reset` reads the same flag before it touches a
    /// closed role's side app.
    pub fn reset_all(&self, force_closed: bool) -> ResetReport {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let mut report = ResetReport::default();
        for (role, lineage) in inner.iter_mut() {
            if lineage.closed && !force_closed {
                report.skipped_closed.push(role.clone());
                continue;
            }
            if let Some(bound) = lineage.v1_app_id.clone() {
                report.cleared_v1_bindings.push((role.clone(), bound));
            }
            *lineage = RoleLineage::at_base(&self.base_app_id);
            report.reset.push(role.clone());
        }
        report
    }

    /// A point-in-time copy of every tracked role's lineage state, for the
    /// passport (Task 8).
    pub fn snapshot(&self) -> BTreeMap<String, RoleLineage> {
        self.inner.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// **Task 33 — the pure half of `POST /admin/lineage/v1-binding`.**
///
/// Decide which app id a requested v1 binding resolves to, given what this
/// conductor actually has installed. Pure, so all three answers the route can
/// give are testable with no conductor: the default, the named side app, and
/// the refusal.
///
/// - `None` (or the base app id itself) → `Ok(None)`: the DEFAULT. Naming the
///   base explicitly stores `None` rather than a copy, so the resolver's
///   fallback stays the single definition of "the household's own chain" and
///   `/version` cannot start reporting a binding where there is none.
/// - a named app this conductor has installed → `Ok(Some(id))`.
/// - a named app it does NOT have → `Err`, naming the app AND listing what is
///   installed. A binding aimed at an app that is not there would resolve at
///   apply time into "base app has no cell for this role" — a message about
///   the wrong app, arriving at the wrong moment, after the ceremony has
///   started. Refusing here, by name, is the difference between a typo and a
///   crossing that reads one chain and seals another.
pub fn resolve_v1_binding(
    base_app_id: &str,
    role: &str,
    requested: Option<&str>,
    installed_app_ids: &[String],
) -> Result<Option<String>, String> {
    let Some(requested) = requested.map(str::trim).filter(|r| !r.is_empty()) else {
        return Ok(None);
    };
    if requested == base_app_id {
        return Ok(None);
    }
    if !installed_app_ids.iter().any(|id| id == requested) {
        return Err(format!(
            "role '{role}': app '{requested}' is not installed on this conductor, so it holds \
             no v1 cell to cross from, carry out of or seal. Installed: {installed_app_ids:?}"
        ));
    }
    Ok(Some(requested.to_string()))
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
    fn reset_all_returns_every_open_role_to_base() {
        let l = LineageRoles::new("elohim", &["node_registry", "lamad"]);
        l.open_window("node_registry", "elohim@AAA", None);
        l.open_window("lamad", "elohim@BBB", None);
        let report = l.reset_all(false);
        assert_eq!(
            report.reset,
            vec!["lamad".to_string(), "node_registry".to_string()]
        );
        assert!(report.skipped_closed.is_empty());
        for role in ["node_registry", "lamad"] {
            assert_eq!(l.app_id_for(role), "elohim");
            let snap = l.snapshot();
            assert_eq!(snap[role].reading_app_id, "elohim");
            assert!(!snap[role].closed);
        }
    }

    /// **Whole-branch I1 / spec §4 step 5.** A SUNSET role is not something a
    /// baseline reset may quietly re-open: routing it back to base would put
    /// authorship on a chain that is permanently sealed. The still-open role
    /// beside it converges exactly as before, so one closed role never blocks
    /// the rest of the reset.
    #[test]
    fn reset_all_skips_a_closed_role_and_names_it() {
        let l = LineageRoles::new("elohim", &["node_registry", "lamad"]);
        l.open_window("node_registry", "elohim@AAA", None);
        l.open_window("lamad", "elohim@BBB", None);
        l.sunset("lamad");

        let report = l.reset_all(false);

        assert_eq!(report.reset, vec!["node_registry".to_string()]);
        assert_eq!(report.skipped_closed, vec!["lamad".to_string()]);

        let snap = l.snapshot();
        // The open role converged.
        assert_eq!(l.app_id_for("node_registry"), "elohim");
        assert!(!snap["node_registry"].closed);
        // The sunset role is byte-for-byte where the sunset left it: still
        // closed, still authoring on the lineage app.
        assert!(snap["lamad"].closed);
        assert_eq!(snap["lamad"].authoring_app_id, "elohim@BBB");
        assert_eq!(l.app_id_for("lamad"), "elohim@BBB");
    }

    /// The fixture/operator seat: `force_closed` DOES reset a closed role,
    /// because a rehearsal mesh has to converge to the v1 baseline after
    /// Station 8 has sealed a window. Nothing is skipped, so the report's
    /// skip list is empty.
    #[test]
    fn reset_all_with_force_closed_resets_a_closed_role() {
        let l = LineageRoles::new("elohim", &["node_registry", "lamad"]);
        l.open_window("lamad", "elohim@BBB", None);
        l.sunset("lamad");

        let report = l.reset_all(true);

        assert!(report.skipped_closed.is_empty());
        assert!(report.reset.contains(&"lamad".to_string()));
        let snap = l.snapshot();
        assert!(!snap["lamad"].closed);
        assert_eq!(l.app_id_for("lamad"), "elohim");
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
    // -----------------------------------------------------------------
    // Task 33 — the v1 binding (the disposable crossing)
    // -----------------------------------------------------------------

    fn installed() -> Vec<String> {
        vec![
            "elohim".to_string(),
            "elohim@a2o-v1-20260905".to_string(),
            "elohim@EKiIscIk5BDd".to_string(),
        ]
    }

    /// **The DEFAULT is the base app**, and it is the same answer whether the
    /// caller says nothing or says "elohim" out loud — both store `None`, so
    /// nothing on the wire can tell a defaulted role from a never-bound one.
    #[test]
    fn an_unbound_role_resolves_v1_to_the_base_app() {
        let l = LineageRoles::new("elohim", &["node_registry"]);
        assert_eq!(l.v1_app_id_for("node_registry"), "elohim");
        assert_eq!(l.v1_app_id_for("never-registered"), "elohim");

        assert_eq!(
            resolve_v1_binding("elohim", "node_registry", None, &installed()),
            Ok(None)
        );
        assert_eq!(
            resolve_v1_binding("elohim", "node_registry", Some("elohim"), &installed()),
            Ok(None),
            "naming the base app IS the default, stored as None"
        );
        assert_eq!(
            resolve_v1_binding("elohim", "node_registry", Some("   "), &installed()),
            Ok(None)
        );
    }

    /// **An EXPLICIT binding resolves to the named side app** and, once set,
    /// is what `v1_app_id_for` answers.
    #[test]
    fn an_explicit_binding_names_the_side_app() {
        assert_eq!(
            resolve_v1_binding(
                "elohim",
                "node_registry",
                Some("elohim@a2o-v1-20260905"),
                &installed()
            ),
            Ok(Some("elohim@a2o-v1-20260905".to_string()))
        );

        let l = LineageRoles::new("elohim", &["node_registry"]);
        assert_eq!(
            l.bind_v1("node_registry", Some("elohim@a2o-v1-20260905")),
            Ok(None),
            "the previous binding was the default"
        );
        assert_eq!(l.v1_app_id_for("node_registry"), "elohim@a2o-v1-20260905");
        // The untouched state moves WITH the binding: both ids are the
        // predecessor, so the role still reads as never-crossed.
        let snap = l.snapshot();
        assert_eq!(
            snap["node_registry"].reading_app_id,
            "elohim@a2o-v1-20260905"
        );
        assert_eq!(
            snap["node_registry"].authoring_app_id,
            "elohim@a2o-v1-20260905"
        );
        assert!(
            l.open_windows().is_empty(),
            "binding v1 is not opening a window"
        );
        // `app_id_for` (authoring) follows, so a fixture seeding v1 records
        // through the resolver lands them on the disposable chain.
        assert_eq!(l.app_id_for("node_registry"), "elohim@a2o-v1-20260905");
    }

    /// **An UNKNOWN app is refused BY NAME**, and the refusal says what is
    /// installed — the whole point of resolving the binding against the
    /// conductor rather than trusting the body.
    #[test]
    fn an_uninstalled_v1_app_is_refused_by_name() {
        let err = resolve_v1_binding("elohim", "node_registry", Some("elohim@typo"), &installed())
            .expect_err("an app this conductor does not have is not a v1");
        assert!(
            err.contains("elohim@typo"),
            "the refusal names the app: {err}"
        );
        assert!(
            err.contains("node_registry"),
            "the refusal names the role: {err}"
        );
        assert!(
            err.contains("elohim@a2o-v1-20260905"),
            "the refusal lists what IS installed: {err}"
        );
    }

    /// The whole crossing rides the binding: the window's READING side is the
    /// run-scoped predecessor (never the household's own app), the window is
    /// still recognised as open, and a revert returns authoring to the
    /// PREDECESSOR rather than to the base app.
    #[test]
    fn a_bound_crossing_reads_reverts_and_sunsets_on_the_run_scoped_v1() {
        let l = LineageRoles::new("elohim", &["node_registry"]);
        l.bind_v1("node_registry", Some("elohim@a2o-v1-20260905"))
            .expect("binding an installed app is allowed at base");

        l.open_window("node_registry", "elohim@EKiIscIk5BDd", None);
        let snap = l.snapshot();
        assert_eq!(
            snap["node_registry"].reading_app_id, "elohim@a2o-v1-20260905",
            "the PREDECESSOR is the run-scoped app, never the household's own"
        );
        assert_eq!(
            l.open_windows()
                .into_iter()
                .map(|(role, _)| role)
                .collect::<Vec<_>>(),
            vec!["node_registry".to_string()],
            "a bound window is still an OPEN window"
        );

        l.revert("node_registry");
        assert_eq!(
            l.app_id_for("node_registry"),
            "elohim@a2o-v1-20260905",
            "a revert returns authoring to the PREDECESSOR, not to the household's base app"
        );
        assert!(l.open_windows().is_empty(), "a reverted window is not open");
    }

    /// Re-aiming a binding mid-crossing is refused by name — the carry read
    /// one chain and the seal would close another, and the seal is the
    /// irreversible act.
    #[test]
    fn a_binding_cannot_be_re_aimed_under_an_open_or_closed_window() {
        let l = LineageRoles::new("elohim", &["node_registry"]);
        l.bind_v1("node_registry", Some("elohim@a2o-v1-20260905"))
            .expect("at base");
        l.open_window("node_registry", "elohim@EKiIscIk5BDd", None);

        let err = l
            .bind_v1("node_registry", Some("elohim"))
            .expect_err("an open window may not be re-aimed");
        assert!(err.contains("OPEN lineage window"), "{err}");
        assert_eq!(
            l.v1_app_id_for("node_registry"),
            "elohim@a2o-v1-20260905",
            "a refused re-aim changes nothing"
        );

        l.sunset("node_registry");
        let err = l
            .bind_v1("node_registry", Some("elohim"))
            .expect_err("a sealed predecessor may not be re-aimed");
        assert!(err.contains("CLOSED lineage window"), "{err}");
        assert!(
            err.contains("elohim@a2o-v1-20260905"),
            "names the sealed v1: {err}"
        );
    }

    /// A baseline reset drops the binding and SAYS SO — the run-scoped
    /// predecessor belongs to the run that staged it, and a silent return to
    /// the household's own chain is the one step back toward spending it.
    #[test]
    fn reset_all_clears_a_v1_binding_and_names_it() {
        let l = LineageRoles::new("elohim", &["node_registry", "lamad"]);
        l.bind_v1("node_registry", Some("elohim@a2o-v1-20260905"))
            .expect("at base");

        let report = l.reset_all(false);

        assert_eq!(
            report.cleared_v1_bindings,
            vec![(
                "node_registry".to_string(),
                "elohim@a2o-v1-20260905".to_string()
            )]
        );
        assert_eq!(l.v1_app_id_for("node_registry"), "elohim");
        assert_eq!(l.v1_app_id_for("lamad"), "elohim");
    }
}
