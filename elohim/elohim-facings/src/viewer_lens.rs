//! The viewer-relative disclosure lens — the pure, DB-free fold that decides
//! *which facets of a subject a given viewer may see*.
//!
//! Pure (DB-free) by construction, exactly like `folds/contributor_reflexive.rs`.
//! The impure shell — `elohim-storage::services::viewer_lens_facing` — resolves
//! the consent-filtered edge, the (stubbed) standing demotion, and the subject's
//! labeled facets, maps them to the DB-free inputs below, then calls
//! [`ViewerLens::resolve`]. NEVER touch a connection here.
//!
//! Design: `genesis/docs/superpowers/specs/2026-06-22-imagodei-profile-page-viewer-lens-design.md`
//! §3 (the primitive, the local 6-rung ladder, the intimacy→tier and
//! reach-label→tier maps, the cut-line function) and §5 (the P2P design-gate row:
//! this fold is **Category C, ephemeral, recomputed-never-stored** — there is no
//! persistence and no write method here, by design).
//!
//! ## The primitive is `relationship_class × intimacy`, NOT reach
//!
//! Reach is content-scoped (it has no `(viewer, subject)` term — see spec §3) and
//! is demoted to a per-facet *sensitivity label* only. The lens is keyed on the
//! qualitative viewer↔subject **relationship class** (which facet *corridor*) and
//! the ordinal **intimacy** (how deep within it). Standing is an advisory,
//! demote-only, cold-start-inert narrowing — passed in as a delta, never widening.

use std::collections::BTreeSet;

// ===========================================================================
// The local 6-rung tier ladder (spec §3, M3 fix)
// ===========================================================================
//
// `{T0 commons, T1 public, T2 familiar, T3 trusted, T4 intimate, T5 self}`.
// This is DISTINCT from the 8-rung reach enum — using reach as the tier axis is
// the drift trap the spec rejects. Tiers are plain `u8` 0..=5 so the cut-line is
// arithmetic: `base.saturating_sub(demotion)` and `label <= effective_tier`.

/// T0 — commons. The anon / no-edge floor; world-readable public face.
pub const T0_COMMONS: u8 = 0;
/// T1 — public. The recognized-stranger floor (a viewer the subject's graph
/// recognizes but has no consented edge with).
pub const T1_PUBLIC: u8 = 1;
/// T2 — familiar. Known, not close.
pub const T2_FAMILIAR: u8 = 2;
/// T3 — trusted. Close.
pub const T3_TRUSTED: u8 = 3;
/// T4 — intimate. Inner; corridor-GATED (work-T4 ≠ family-T4 — same depth,
/// disjoint corridors).
pub const T4_INTIMATE: u8 = 4;
/// T5 — self. The subject viewing themselves; full self-knowledge surface.
pub const T5_SELF: u8 = 5;

/// The 4-rung `intimacy_level` → corridor depth tier (spec §3, M3 fix):
/// `recognition→T1`, `connection→T2`, `trusted→T3`, `intimate→T4`.
/// An unrecognized level maps to the conservative `T1_PUBLIC` floor (never
/// silently grants a deeper tier on a vocabulary surprise).
pub fn intimacy_to_tier(intimacy_level: &str) -> u8 {
    match intimacy_level {
        "recognition" => T1_PUBLIC,
        "connection" => T2_FAMILIAR,
        "trusted" => T3_TRUSTED,
        "intimate" => T4_INTIMATE,
        _ => T1_PUBLIC,
    }
}

/// The reach-vocabulary → sensitivity-label tier table (spec §3, M3 fix).
/// Reuses the reach vocabulary as the per-facet label (NO new enum); maps it
/// onto the SAME 6-rung ladder. The two reach rungs the first draft "silently
/// dropped" — `community` and `private` — are present here.
///
/// `commons→T0`, `public→T1`, `familiar→T2`, `community→T2`, `trusted→T3`,
/// `private→T4`, `intimate→T4`, `self→T5`. An unrecognized label maps to the
/// most-restrictive `T5_SELF` (most-restrictive-wins: an unknown label is never
/// disclosed below self).
pub fn reach_label_to_tier(reach_label: &str) -> u8 {
    match reach_label {
        "commons" => T0_COMMONS,
        "public" => T1_PUBLIC,
        "familiar" => T2_FAMILIAR,
        "community" => T2_FAMILIAR,
        "trusted" => T3_TRUSTED,
        "private" => T4_INTIMATE,
        "intimate" => T4_INTIMATE,
        "self" => T5_SELF,
        _ => T5_SELF,
    }
}

// ===========================================================================
// Facet corridors (the relationship-class router)
// ===========================================================================
//
// `relationship_class` routes which facet FAMILY a viewer may see. The spec's
// examples: colleague→work, sibling→family. Crucially (advisor coupling note):
// commons/public facets live in a UNIVERSAL class present in EVERY corridor,
// including the anon/stranger one — that is what makes "anon sees only the
// commons face" hold while work-T4 and family-T4 stay disjoint.

/// The corridor a viewer is routed into, derived from `relationship_class`.
/// A facet is visible only if its `class` is in the active corridor AND its
/// sensitivity label is at or below the effective tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corridor {
    /// No consented edge (anon or stranger). Only the universal/commons class.
    Commons,
    /// Work / professional family (colleague, coworker, mentor, business…).
    Work,
    /// Family / kin (sibling, parent, spouse, child, grandparent…).
    Family,
    /// Community / social (neighbor, congregation, learning-partner…).
    Community,
    /// The subject viewing themselves — every corridor.
    Selfsame,
}

/// The facet class a sensitivity-labeled facet belongs to. `Universal` is the
/// commons/public face present in every corridor; the others are corridor-gated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FacetClass {
    /// Present in every corridor (name, headline, public attestations…).
    Universal,
    Work,
    Family,
    Community,
}

/// Route a (freeform) `relationship_type` string to a [`Corridor`]. The
/// `human_relationships.relationship_type` vocabulary is open (coworker, sibling,
/// spouse, neighbor, mentor, …), so this buckets by qualitative family. An
/// unrecognized type routes to [`Corridor::Community`] — the least-intimate
/// non-stranger corridor — never to Work or Family (which would mis-grant a
/// corridor-gated facet on a vocabulary surprise).
pub fn class_to_corridor(relationship_type: &str) -> Corridor {
    let t = relationship_type.to_ascii_lowercase().replace('_', "-");
    match t.as_str() {
        // Work / professional
        "colleague" | "coworker" | "co-worker" | "mentor" | "mentor-of" | "mentee"
        | "mentee-of" | "business-partner" | "business" | "employer" | "employee"
        | "collaborator" | "network-connection" => Corridor::Work,
        // Family / kin
        "sibling" | "spouse" | "parent" | "parent-of" | "child" | "child-of" | "kin"
        | "grandparent" | "grandparent-of" | "grandchild" | "family" | "partner" | "cousin"
        | "aunt" | "uncle" | "niece" | "nephew" => Corridor::Family,
        // Community / social (explicit + the conservative default)
        _ => Corridor::Community,
    }
}

/// The set of facet classes visible within a corridor. `Universal` is ALWAYS
/// present (the coupling that preserves the commons face for everyone); the
/// corridor-specific class is added for non-stranger corridors. `Selfsame` sees
/// every class.
pub fn corridor_classes(corridor: Corridor) -> BTreeSet<FacetClass> {
    let mut set = BTreeSet::new();
    set.insert(FacetClass::Universal);
    match corridor {
        Corridor::Commons => {}
        Corridor::Work => {
            set.insert(FacetClass::Work);
        }
        Corridor::Family => {
            set.insert(FacetClass::Family);
        }
        Corridor::Community => {
            set.insert(FacetClass::Community);
        }
        Corridor::Selfsame => {
            set.insert(FacetClass::Work);
            set.insert(FacetClass::Family);
            set.insert(FacetClass::Community);
        }
    }
    set
}

// `Ord`/`PartialOrd` for `FacetClass` so it can live in a `BTreeSet`
// (deterministic iteration → no hash-order leak onto the wire).
impl PartialOrd for FacetClass {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FacetClass {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank().cmp(&other.rank())
    }
}
impl FacetClass {
    fn rank(self) -> u8 {
        match self {
            FacetClass::Universal => 0,
            FacetClass::Work => 1,
            FacetClass::Family => 2,
            FacetClass::Community => 3,
        }
    }
}

// ===========================================================================
// The fold inputs (DB-free mirrors)
// ===========================================================================

/// The consent-complete viewer↔subject edge, mapped from the storage row by the
/// adapter AFTER the consent + bidirectional filter (`get_consented_relationship_between`).
/// `None` at the call site means "no consent-complete edge" → the viewer is a
/// stranger (commons / recognized floor). A half-consented row is NEVER mapped
/// here — that is the C1 fix, enforced in the read, asserted in the adapter test.
#[derive(Debug, Clone)]
pub struct ConsentedEdge {
    /// `human_relationships.relationship_type` — routes the corridor.
    pub relationship_type: String,
    /// `human_relationships.intimacy_level` (4-rung) — routes the depth.
    pub intimacy_level: String,
}

/// A single labeled subject facet the fold filters. The `(sensitivity_label,
/// class)` pair is the minimal slice of gap-item #6: a typed label over the
/// EXISTING profile fields/sections, NOT the manifest-composite/domain-facet
/// system (a later slice). The adapter loads the facet *values* impurely; the
/// label/class taxonomy lives in the pure crate so it is testable.
#[derive(Debug, Clone)]
pub struct LabeledFacet {
    /// Stable facet key (e.g. `"displayName"`, `"bio"`, `"affinities"`).
    pub key: String,
    /// The reach-vocabulary sensitivity label (`commons`/`public`/…/`self`).
    pub sensitivity_label: String,
    /// The corridor class this facet belongs to.
    pub class: FacetClass,
}

/// Whether the subject recognizes the viewer at all (the stranger-floor split).
/// `recognized → T1 public`, `anon → T0 commons` (spec §3 `stranger_floor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrangerKind {
    /// Authenticated session, but no consented edge with the subject.
    Recognized,
    /// No session at all.
    Anonymous,
}

/// The resolved lens — the effective tier, the active corridor, and the facets
/// the viewer may see. Recomputed-never-stored.
#[derive(Debug, Clone)]
pub struct ResolvedLens {
    /// The tier after the (advisory, demote-only) standing demotion.
    pub effective_tier: u8,
    /// The corridor the viewer was routed into.
    pub corridor: Corridor,
    /// The facets passing both the corridor and the tier cut, in the order the
    /// caller supplied them (the adapter assembles a stable input order).
    pub visible_facets: Vec<LabeledFacet>,
}

/// The pure viewer-lens fold.
pub struct ViewerLens;

impl ViewerLens {
    /// Compute the disclosure lens for one (viewer, subject) request.
    ///
    /// `is_self` — the viewer IS the subject (T5, every corridor).
    /// `edge` — the consent-complete edge, or `None` for a stranger.
    /// `stranger` — when `edge` is `None`, whether the viewer is recognized
    ///   (→T1) or anonymous (→T0).
    /// `standing_demotion` — the advisory, demote-only delta (spec §3 H2 fix).
    ///   **Stubbed to 0 in this slice** (`Unknown` → no demotion); the adapter
    ///   passes 0 until the standing substrate lands. Never widens.
    /// `facets` — the subject's labeled facets to filter.
    ///
    /// Cut-line (spec §3):
    /// ```text
    /// base_tier      = self ? T5 : edge ? corridor_depth(class, intimacy) : stranger_floor
    /// effective_tier = base_tier.saturating_sub(standing_demotion)
    /// visible        = { f : reach_label(f) <= effective_tier AND f.class ∈ corridor_classes(corridor) }
    /// ```
    pub fn resolve(
        is_self: bool,
        edge: Option<&ConsentedEdge>,
        stranger: StrangerKind,
        standing_demotion: u8,
        facets: &[LabeledFacet],
    ) -> ResolvedLens {
        let (base_tier, corridor) = if is_self {
            (T5_SELF, Corridor::Selfsame)
        } else if let Some(e) = edge {
            (
                intimacy_to_tier(&e.intimacy_level),
                class_to_corridor(&e.relationship_type),
            )
        } else {
            // stranger_floor: recognized → T1 public; anon → T0 commons.
            let floor = match stranger {
                StrangerKind::Recognized => T1_PUBLIC,
                StrangerKind::Anonymous => T0_COMMONS,
            };
            (floor, Corridor::Commons)
        };

        // Standing is advisory + demote-only (never widens). 0 this slice.
        let effective_tier = base_tier.saturating_sub(standing_demotion);

        let allowed_classes = corridor_classes(corridor);

        let visible_facets: Vec<LabeledFacet> = facets
            .iter()
            .filter(|f| {
                allowed_classes.contains(&f.class)
                    && reach_label_to_tier(&f.sensitivity_label) <= effective_tier
            })
            .cloned()
            .collect();

        ResolvedLens {
            effective_tier,
            corridor,
            visible_facets,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facet(key: &str, label: &str, class: FacetClass) -> LabeledFacet {
        LabeledFacet {
            key: key.to_string(),
            sensitivity_label: label.to_string(),
            class,
        }
    }

    /// A representative facet set spanning corridors and tiers: a universal
    /// commons face, a universal public facet, a work-trusted facet, and a
    /// family-intimate facet.
    fn sample_facets() -> Vec<LabeledFacet> {
        vec![
            facet("displayName", "commons", FacetClass::Universal),
            facet("headline", "public", FacetClass::Universal),
            facet("workActivity", "trusted", FacetClass::Work),
            facet("familyNotes", "intimate", FacetClass::Family),
        ]
    }

    fn keys(lens: &ResolvedLens) -> Vec<&str> {
        lens.visible_facets.iter().map(|f| f.key.as_str()).collect()
    }

    #[test]
    fn anon_sees_only_the_commons_face() {
        let lens = ViewerLens::resolve(false, None, StrangerKind::Anonymous, 0, &sample_facets());
        assert_eq!(lens.effective_tier, T0_COMMONS);
        assert_eq!(lens.corridor, Corridor::Commons);
        // Only the commons-labeled universal facet — public is T1 > T0.
        assert_eq!(keys(&lens), vec!["displayName"]);
    }

    #[test]
    fn recognized_stranger_sees_commons_and_public_but_no_corridor_facets() {
        let lens = ViewerLens::resolve(false, None, StrangerKind::Recognized, 0, &sample_facets());
        assert_eq!(lens.effective_tier, T1_PUBLIC);
        assert_eq!(lens.corridor, Corridor::Commons);
        // commons (T0) + public (T1) universal facets; work/family corridors absent.
        assert_eq!(keys(&lens), vec!["displayName", "headline"]);
    }

    #[test]
    fn colleague_edge_and_sibling_edge_yield_different_corridors() {
        let colleague = ConsentedEdge {
            relationship_type: "colleague".to_string(),
            intimacy_level: "trusted".to_string(),
        };
        let sibling = ConsentedEdge {
            relationship_type: "sibling".to_string(),
            intimacy_level: "trusted".to_string(),
        };

        let work = ViewerLens::resolve(
            false,
            Some(&colleague),
            StrangerKind::Recognized,
            0,
            &sample_facets(),
        );
        let family = ViewerLens::resolve(
            false,
            Some(&sibling),
            StrangerKind::Recognized,
            0,
            &sample_facets(),
        );

        assert_eq!(work.corridor, Corridor::Work);
        assert_eq!(family.corridor, Corridor::Family);
        // Both reach T3 (trusted). Work sees the work-trusted facet; family does NOT.
        assert_eq!(work.effective_tier, T3_TRUSTED);
        assert!(keys(&work).contains(&"workActivity"));
        assert!(!keys(&work).contains(&"familyNotes"));
        // Family corridor sees neither workActivity (wrong corridor) nor
        // familyNotes (intimate=T4 > effective T3).
        assert!(!keys(&family).contains(&"workActivity"));
        assert!(!keys(&family).contains(&"familyNotes"));
        // Both still see the universal commons + public face.
        assert!(keys(&work).contains(&"displayName"));
        assert!(keys(&family).contains(&"headline"));
    }

    #[test]
    fn intimate_family_edge_unlocks_the_family_intimate_facet() {
        let sibling = ConsentedEdge {
            relationship_type: "sibling".to_string(),
            intimacy_level: "intimate".to_string(),
        };
        let lens = ViewerLens::resolve(
            false,
            Some(&sibling),
            StrangerKind::Recognized,
            0,
            &sample_facets(),
        );
        assert_eq!(lens.corridor, Corridor::Family);
        assert_eq!(lens.effective_tier, T4_INTIMATE);
        assert!(keys(&lens).contains(&"familyNotes"));
        // Family corridor never sees the work facet, even at the same depth.
        assert!(!keys(&lens).contains(&"workActivity"));
    }

    #[test]
    fn self_view_sees_every_corridor_and_tier() {
        let lens = ViewerLens::resolve(true, None, StrangerKind::Anonymous, 0, &sample_facets());
        assert_eq!(lens.effective_tier, T5_SELF);
        assert_eq!(lens.corridor, Corridor::Selfsame);
        assert_eq!(keys(&lens).len(), 4, "self sees all facets");
    }

    #[test]
    fn standing_demotion_narrows_an_already_consented_edge_demote_only() {
        let colleague = ConsentedEdge {
            relationship_type: "colleague".to_string(),
            intimacy_level: "trusted".to_string(),
        };
        // demote 1: T3 → T2; the work-trusted facet (T3) drops out.
        let lens = ViewerLens::resolve(
            false,
            Some(&colleague),
            StrangerKind::Recognized,
            1,
            &sample_facets(),
        );
        assert_eq!(lens.effective_tier, T2_FAMILIAR);
        assert!(!keys(&lens).contains(&"workActivity"));
        // Universal commons/public face survives the demotion.
        assert!(keys(&lens).contains(&"displayName"));
        assert!(keys(&lens).contains(&"headline"));
    }

    #[test]
    fn standing_demotion_saturates_at_commons_never_underflows() {
        // demote far past T0 — saturating_sub floors at 0, no panic/underflow.
        let lens = ViewerLens::resolve(false, None, StrangerKind::Recognized, 99, &sample_facets());
        assert_eq!(lens.effective_tier, T0_COMMONS);
        assert_eq!(keys(&lens), vec!["displayName"]);
    }

    #[test]
    fn intimacy_and_reach_maps_cover_the_full_local_ladder() {
        // intimacy 4-rung → tier
        assert_eq!(intimacy_to_tier("recognition"), T1_PUBLIC);
        assert_eq!(intimacy_to_tier("connection"), T2_FAMILIAR);
        assert_eq!(intimacy_to_tier("trusted"), T3_TRUSTED);
        assert_eq!(intimacy_to_tier("intimate"), T4_INTIMATE);
        assert_eq!(
            intimacy_to_tier("garbage"),
            T1_PUBLIC,
            "unknown → conservative floor"
        );

        // reach label → tier (incl. the two the first draft dropped)
        assert_eq!(reach_label_to_tier("commons"), T0_COMMONS);
        assert_eq!(reach_label_to_tier("public"), T1_PUBLIC);
        assert_eq!(reach_label_to_tier("familiar"), T2_FAMILIAR);
        assert_eq!(reach_label_to_tier("community"), T2_FAMILIAR);
        assert_eq!(reach_label_to_tier("trusted"), T3_TRUSTED);
        assert_eq!(reach_label_to_tier("private"), T4_INTIMATE);
        assert_eq!(reach_label_to_tier("intimate"), T4_INTIMATE);
        assert_eq!(reach_label_to_tier("self"), T5_SELF);
        assert_eq!(
            reach_label_to_tier("garbage"),
            T5_SELF,
            "unknown → most restrictive"
        );
    }

    #[test]
    fn unrecognized_relationship_type_routes_to_community_not_work_or_family() {
        assert_eq!(class_to_corridor("neighbor"), Corridor::Community);
        assert_eq!(
            class_to_corridor("congregation-member"),
            Corridor::Community
        );
        assert_eq!(
            class_to_corridor("totally-unknown-type"),
            Corridor::Community
        );
        // never silently grants the gated corridors
        assert_ne!(class_to_corridor("totally-unknown-type"), Corridor::Work);
        assert_ne!(class_to_corridor("totally-unknown-type"), Corridor::Family);
    }

    #[test]
    fn corridor_classes_always_include_universal() {
        for c in [
            Corridor::Commons,
            Corridor::Work,
            Corridor::Family,
            Corridor::Community,
            Corridor::Selfsame,
        ] {
            assert!(
                corridor_classes(c).contains(&FacetClass::Universal),
                "every corridor must carry the universal commons face: {c:?}"
            );
        }
        // Work corridor does not leak the family class and vice versa.
        assert!(!corridor_classes(Corridor::Work).contains(&FacetClass::Family));
        assert!(!corridor_classes(Corridor::Family).contains(&FacetClass::Work));
    }
}
