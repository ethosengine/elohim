//! Viewer-lens facing — the impure shell over the pure
//! [`elohim_facings::viewer_lens::ViewerLens`] fold.
//!
//! A thin adapter, exactly like `contributor_reflexive_facing`: it does the
//! three reads, maps the diesel rows onto the DB-free fold inputs, then calls
//! the pure fold and assembles the wire view. NEVER folds in here.
//!
//! The three reads (spec §3 "where the computation lives", §4(c)):
//!   1. `get_consented_relationship_between(viewer, subject)` — the C1
//!      consent-filtered read (NEVER the raw consent-blind `get_relationship_between`).
//!   2. `Standing::evaluate(viewer, subject)` — called, but treated as
//!      **`Unknown` → no demotion (0)** in this slice (the standing-demotion
//!      substrate is a later slice; spec §3 H2 fix is honestly cold-start-inert).
//!   3. `load_subject_facets` — the labeled facets over the EXISTING profile
//!      fields (the minimal slice of gap-item #6).
//!
//! Identity-namespace discipline (the silent-collapse trap): the consent read
//! filters on human **`id`** (e.g. `human-matthew`), so viewer + subject are
//! resolved to their `Human` rows and joined on `id`. The agent pubkey is
//! decoded ONLY to feed `Standing::evaluate` (which keys on pubkey bytes). We
//! never feed an agent pubkey into the id-keyed consent read — that would empty
//! the join and silently collapse every viewer to the commons floor.
//!
//! Design: `genesis/docs/superpowers/specs/2026-06-22-imagodei-profile-page-viewer-lens-design.md`
//! §3 + §5 (this fold = Category C, ephemeral, recomputed-never-stored).

use diesel::SqliteConnection;
use elohim_facings::viewer_lens::{
    ConsentedEdge, Corridor, FacetClass, LabeledFacet, ResolvedLens, StrangerKind, ViewerLens,
};
use serde::Serialize;
use ts_rs::TS;

use crate::db::models::Human;
use crate::db::{human_relationships, humans, AppContext};
use crate::error::StorageError;

// ===========================================================================
// Wire view — defined HERE (in-bounds) with a ts-rs export, NOT in elohim-views
// (which is the shared, codegen-racing crate the facings CLAUDE.md says to
// serialize against the concurrent session). Generated TS lands in
// sdk/storage-client-ts/src/generated/ as acceptable codegen collateral.
// ===========================================================================

/// One disclosed facet of a subject's profile, as seen through the viewer's lens.
/// Carries the facet's reach-vocabulary sensitivity label so the client can
/// render the band it sits in. Recomputed-never-stored (Category C).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProfileFacetView {
    /// Stable facet key (`displayName`, `bio`, `affinities`, …).
    pub key: String,
    /// The facet's rendered value (JSON-stringified for richer fields).
    pub value: Option<String>,
    /// The reach-vocabulary sensitivity label of this facet.
    pub sensitivity_label: String,
}

/// The viewer-relative profile lens — which facets THIS viewer may see of the
/// subject. Computed per request; no persistence, no per-viewer permission row
/// (that would be the O(viewers²) ACL anti-pattern the design-gate forbids).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProfileLensView {
    /// The subject's stable human id.
    pub subject_id: String,
    /// The effective disclosure tier (0=commons … 5=self) after demotion.
    pub effective_tier: u8,
    /// The corridor the viewer was routed into (`commons`/`work`/`family`/
    /// `community`/`self`).
    pub corridor: String,
    /// Whether the viewer is the subject (self-view).
    pub is_self: bool,
    /// The facets the viewer may see, in stable input order.
    pub visible_facets: Vec<ProfileFacetView>,
}

// ===========================================================================
// The impure adapter
// ===========================================================================

/// Build the viewer-lens profile for `subject_id` as seen by `viewer_id`.
///
/// `viewer_id` is the authenticated session identity (the human `id` resolved
/// from `X-Agent-Id`), or `None` for an anonymous caller (→ commons floor).
/// Returns `None` when the subject has no `Human` row (the route surfaces 404).
pub fn build_profile_lens_view(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    subject_id: &str,
    viewer_id: Option<&str>,
) -> Result<Option<ProfileLensView>, StorageError> {
    // Resolve the subject's Human row (the facet source). No row → 404.
    let subject = match humans::get_human_by_id(conn, subject_id)? {
        Some(h) => h,
        None => return Ok(None),
    };

    let is_self = viewer_id == Some(subject.id.as_str());

    // Read 1 — the consent-filtered edge (C1). Resolve only when there is a
    // distinct authenticated viewer; a self-view or anon caller skips it.
    let edge: Option<ConsentedEdge> = match viewer_id {
        Some(vid) if !is_self => {
            // Direction-insensitive, both-consents-required. Take the deepest
            // consent-complete edge if multiple exist (most-permissive-edge wins
            // on intimacy, while per-facet most-restrictive-wins still gates output).
            human_relationships::get_consented_relationship_between(
                conn,
                ctx,
                vid,
                &subject.id,
                None,
            )?
            .into_iter()
            .max_by_key(|r| elohim_facings::viewer_lens::intimacy_to_tier(&r.intimacy_level))
            .map(|r| ConsentedEdge {
                relationship_type: r.relationship_type,
                intimacy_level: r.intimacy_level,
            })
        }
        _ => None,
    };

    // The stranger floor split: a viewer with a session but no consented edge
    // is `Recognized` (→T1); no session is `Anonymous` (→T0).
    let stranger = if viewer_id.is_some() {
        StrangerKind::Recognized
    } else {
        StrangerKind::Anonymous
    };

    // Read 2 — standing. Called for shape, but STUBBED to no-demotion this
    // slice: `Standing::evaluate` returns `Unknown` at cold-start, and the
    // demotion helper is a later slice. `Unknown → 0` (no demotion).
    let standing_demotion: u8 = standing_demotion_stub(conn, viewer_id, &subject);

    // Read 3 — the labeled facets over the subject's existing profile fields.
    let labeled = load_subject_facets(&subject);

    // Pure fold.
    let resolved: ResolvedLens = ViewerLens::resolve(
        is_self,
        edge.as_ref(),
        stranger,
        standing_demotion,
        &labeled,
    );

    // Assemble the wire view — re-join the resolved labels to their values.
    let visible_facets = resolved
        .visible_facets
        .iter()
        .map(|f| ProfileFacetView {
            key: f.key.clone(),
            value: facet_value(&subject, &f.key),
            sensitivity_label: f.sensitivity_label.clone(),
        })
        .collect();

    Ok(Some(ProfileLensView {
        subject_id: subject.id.clone(),
        effective_tier: resolved.effective_tier,
        corridor: corridor_label(resolved.corridor).to_string(),
        is_self,
        visible_facets,
    }))
}

/// Standing demotion — STUBBED to 0 (no demotion) this slice. We still call the
/// evaluator for shape so the wiring is honest: it returns `Unknown` at
/// cold-start (no `standing_view` projection), which maps to no demotion. When
/// the standing-demotion substrate lands (later slice), this becomes
/// `Standing::evaluate(viewer_pk, subject_pk, conn).disclosure_demotion()`.
fn standing_demotion_stub(
    conn: &mut SqliteConnection,
    viewer_id: Option<&str>,
    subject: &Human,
) -> u8 {
    // Resolve agent pubkeys ONLY for the standing read (pubkey-keyed namespace).
    // Decoding is best-effort: a missing pubkey just means Unknown → 0.
    let viewer_pk = viewer_id.and_then(|vid| {
        humans::get_human_by_id(conn, vid)
            .ok()
            .flatten()
            .and_then(|h| h.agent_pub_key)
    });
    let subject_pk = subject.agent_pub_key.clone();

    if let (Some(v), Some(s)) = (viewer_pk, subject_pk) {
        // Call evaluate for shape; Unknown (cold-start) → no demotion.
        let standing =
            crate::services::standing::Standing::evaluate(v.as_bytes(), s.as_bytes(), conn);
        // Demote-only, inert on Unknown. No `disclosure_demotion` helper yet
        // (later slice) — Unknown and every Computed score map to 0 here so the
        // slice is honestly cold-start-inert and never widens.
        match standing {
            crate::services::standing::Standing::Unknown => 0,
            crate::services::standing::Standing::Computed { .. } => 0,
        }
    } else {
        0
    }
}

/// Map the subject's EXISTING profile fields to labeled facets — the minimal
/// slice of gap-item #6. A simple typed mapping (NOT the manifest-composite /
/// domain-facet system, which is a later slice). Reuses the reach vocabulary as
/// the label (no new enum). Stable order so the visible-facet output is stable.
///
/// - `displayName` → `commons` / Universal (the public face; always visible).
/// - `affinities`  → `public`  / Universal (public affinities, T1).
/// - `location`    → `familiar`/ Universal (known-circle, T2).
/// - `bio`         → `trusted` / Universal (fuller self-description, T3).
///
/// All Universal in this slice (the imagodei *core* profile is one face; the
/// corridor-gated Work/Family/Community facets arrive with domain composites in
/// a later slice). The labels span T0..T3 so the tier gradient is observable.
fn load_subject_facets(subject: &Human) -> Vec<LabeledFacet> {
    let mut facets = vec![LabeledFacet {
        key: "displayName".to_string(),
        sensitivity_label: "commons".to_string(),
        class: FacetClass::Universal,
    }];
    // Only emit value-bearing optional facets so the lens never advertises a
    // field the subject left empty.
    facets.push(LabeledFacet {
        key: "affinities".to_string(),
        sensitivity_label: "public".to_string(),
        class: FacetClass::Universal,
    });
    if subject.location.is_some() {
        facets.push(LabeledFacet {
            key: "location".to_string(),
            sensitivity_label: "familiar".to_string(),
            class: FacetClass::Universal,
        });
    }
    if subject.bio.is_some() {
        facets.push(LabeledFacet {
            key: "bio".to_string(),
            sensitivity_label: "trusted".to_string(),
            class: FacetClass::Universal,
        });
    }
    facets
}

/// Resolve a facet key to its rendered value off the subject row.
fn facet_value(subject: &Human, key: &str) -> Option<String> {
    match key {
        "displayName" => Some(subject.display_name.clone()),
        // `affinities` is stored as a JSON array string; pass it through as-is.
        "affinities" => Some(subject.affinities.clone()),
        "location" => subject.location.clone(),
        "bio" => subject.bio.clone(),
        _ => None,
    }
}

/// Stable lowercase label for a corridor (the wire shape).
fn corridor_label(c: Corridor) -> &'static str {
    match c {
        Corridor::Commons => "commons",
        Corridor::Work => "work",
        Corridor::Family => "family",
        Corridor::Community => "community",
        Corridor::Selfsame => "self",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;
    use crate::db::DbPool;
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:viewer_lens_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn insert_human(conn: &mut SqliteConnection, id: &str, name: &str, bio: Option<&str>) {
        humans::create_human(
            conn,
            humans::CreateHumanInput {
                id: id.to_string(),
                agent_pub_key: Some(format!("uhCAk-{id}")),
                display_name: name.to_string(),
                bio: bio.map(str::to_string),
                affinities: "[\"music\"]".to_string(),
                profile_reach: "self".to_string(),
                location: Some("Toronto".to_string()),
                profile_photo_url: None,
                h_app_id: "imagodei".to_string(),
                household_id: None,
            },
        )
        .expect("insert human");
    }

    fn insert_edge(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        a: &str,
        b: &str,
        rel_type: &str,
        intimacy: &str,
        consent: (bool, bool),
    ) {
        let (consent_a, consent_b) = consent;
        human_relationships::create_human_relationship(
            conn,
            ctx,
            human_relationships::CreateHumanRelationshipInput {
                id: None,
                party_a_id: a.to_string(),
                party_b_id: b.to_string(),
                relationship_type: rel_type.to_string(),
                intimacy_level: intimacy.to_string(),
                is_bidirectional: true,
                consent_given_by_a: consent_a,
                consent_given_by_b: consent_b,
                initiated_by: a.to_string(),
                governance_layer: None,
                reach: "private".to_string(),
                context_json: None,
                expires_at: None,
            },
        )
        .expect("insert edge");
    }

    fn ctx() -> AppContext {
        AppContext::new("imagodei")
    }

    /// End-to-end: a both-consented colleague edge yields the WORK corridor —
    /// the differentiation the pure test alone cannot prove (it would silently
    /// die if the adapter joined on the wrong identity namespace).
    #[test]
    fn both_consented_colleague_edge_yields_work_corridor_end_to_end() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = ctx();
        insert_human(&mut conn, "subject", "Subject", Some("a bio"));
        insert_human(&mut conn, "viewer", "Viewer", None);
        insert_edge(
            &mut conn,
            &ctx,
            "viewer",
            "subject",
            "colleague",
            "trusted",
            (true, true),
        );

        let lens = build_profile_lens_view(&mut conn, &ctx, "subject", Some("viewer"))
            .expect("query")
            .expect("subject exists");
        assert_eq!(lens.corridor, "work");
        assert_eq!(lens.effective_tier, 3); // trusted = T3
        assert!(!lens.is_self);
    }

    /// End-to-end: a both-consented SIBLING edge yields the FAMILY corridor —
    /// the routing counterpart to the colleague test, proving both corridors are
    /// wired through the real identity-resolved adapter.
    ///
    /// HONESTY NOTE (this slice): because every imagodei *core* facet in
    /// `load_subject_facets` is `FacetClass::Universal`, the colleague (work) and
    /// sibling (family) lenses at the same tier expose the IDENTICAL facet SET —
    /// only the `corridor` label differs. Corridor-disjoint facet differentiation
    /// becomes observable on the real surface only when corridor-gated domain
    /// facets land (a later slice; the pure fold already proves the gating with
    /// synthetic Work/Family facets). This is the precise scope boundary of the
    /// Slice-1 a2o ("a sibling and a colleague see different facets").
    #[test]
    fn both_consented_sibling_edge_yields_family_corridor_end_to_end() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = ctx();
        insert_human(&mut conn, "subject", "Subject", Some("a bio"));
        insert_human(&mut conn, "kin", "Kin", None);
        insert_edge(
            &mut conn,
            &ctx,
            "kin",
            "subject",
            "sibling",
            "trusted",
            (true, true),
        );

        let lens = build_profile_lens_view(&mut conn, &ctx, "subject", Some("kin"))
            .expect("query")
            .expect("subject exists");
        assert_eq!(lens.corridor, "family");
        assert_eq!(lens.effective_tier, 3); // trusted = T3
    }

    /// A half-consented edge must NOT route the viewer into any corridor — the
    /// C1 fix proven end-to-end through the adapter.
    #[test]
    fn half_consented_edge_collapses_viewer_to_commons_end_to_end() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = ctx();
        insert_human(&mut conn, "subject", "Subject", Some("a bio"));
        insert_human(&mut conn, "mallory", "Mallory", None);
        // Mallory unilaterally inserts a half-consented intimate family edge.
        insert_edge(
            &mut conn,
            &ctx,
            "mallory",
            "subject",
            "sibling",
            "intimate",
            (true, false),
        );

        let lens = build_profile_lens_view(&mut conn, &ctx, "subject", Some("mallory"))
            .expect("query")
            .expect("subject exists");
        // Recognized stranger floor — NOT the family corridor.
        assert_eq!(lens.corridor, "commons");
        assert_eq!(lens.effective_tier, 1); // recognized → T1 public
                                            // Only the universal commons + public facets; no bio (trusted=T3).
        let keys: Vec<&str> = lens.visible_facets.iter().map(|f| f.key.as_str()).collect();
        assert!(keys.contains(&"displayName"));
        assert!(
            !keys.contains(&"bio"),
            "bio (trusted) must not leak to a stranger"
        );
    }

    /// Anonymous caller → commons floor, commons facet only.
    #[test]
    fn anonymous_viewer_sees_only_commons() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = ctx();
        insert_human(&mut conn, "subject", "Subject", Some("a bio"));

        let lens = build_profile_lens_view(&mut conn, &ctx, "subject", None)
            .expect("query")
            .expect("subject exists");
        assert_eq!(lens.corridor, "commons");
        assert_eq!(lens.effective_tier, 0); // anon → T0 commons
        let keys: Vec<&str> = lens.visible_facets.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["displayName"]);
    }

    /// Self-view sees every facet (T5).
    #[test]
    fn self_view_sees_all_facets() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = ctx();
        insert_human(&mut conn, "subject", "Subject", Some("a bio"));

        let lens = build_profile_lens_view(&mut conn, &ctx, "subject", Some("subject"))
            .expect("query")
            .expect("subject exists");
        assert_eq!(lens.corridor, "self");
        assert_eq!(lens.effective_tier, 5);
        assert!(lens.is_self);
        // displayName, affinities, location, bio — all four.
        assert_eq!(lens.visible_facets.len(), 4);
    }

    /// Missing subject → None (404 at the route).
    #[test]
    fn missing_subject_returns_none() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = ctx();
        let lens = build_profile_lens_view(&mut conn, &ctx, "ghost", None).expect("query");
        assert!(lens.is_none());
    }
}
