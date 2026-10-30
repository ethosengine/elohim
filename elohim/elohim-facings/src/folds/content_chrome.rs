//! The content-chrome facing's fold — every reading the protocol chrome shows for
//! one EPR content record, composed from the folds that already own each reading.
//!
//! Pure (DB-free) by construction. The storage loader
//! (`elohim-storage::services::content_chrome_facing`) selects the record and the
//! projections it composes, maps each diesel row onto the DB-free rows below, and
//! calls [`fold_content_chrome`]. Nothing here re-derives a reading another fold
//! owns:
//!
//! - **holders** — [`resiliency::build_felt_status`] words the reading; its
//!   `headline` is carried verbatim, never re-phrased;
//! - **legs** — [`epr_content::fold_coupling_legs`] pivots the record's EPR
//!   couplings into knowledge · value · governance;
//! - **claims** — the record's attestations by subject (the same dataset the EPR
//!   page reads), never the couplings;
//! - **steward** — the record's active stewardship allocations;
//! - **witness** — [`witness_reading`]: green once a conductor other than the
//!   record's author holds it as valid ([`WitnessEvidence::Witnessed`], which
//!   the loader derives), and only for a record whose projection carries its
//!   DHT anchor (lamad-types [`witness_from_anchor`] stays the precondition);
//! - **app** — lamad-types' one lenient parser, [`Html5AppBody::from_json_str`]
//!   (an unknown capability is dropped, never granted).
//!
//! The not-selected contract: a lens whose rows the loader did not select
//! (`None` on [`ContentChromeRelation`]) is absent from the view.
//!
//! Source of truth: none of its own — Ephemeral (Category C), recomputed per
//! request from the Notarized record and the projections above.

use elohim_views::{
    ContentChromeAppLens, ContentChromeClaim, ContentChromeClaimsLens, ContentChromeFacingView,
    ContentChromeHoldersLens, ContentChromeSteward, ContentChromeStewardLens,
    ContentChromeThemeHint, PlacementGapView, StewardingCollectiveEntry,
};
use lamad_types::{witness_from_anchor, ColorScheme, Html5AppBody, Witness};

use crate::folds::epr_content::{self, CouplingRow};
use crate::folds::resiliency;

/// The trust mode of every record this facing reads: it has an Elohim Protocol
/// record, so it is gated content. The switch is data (the record exists), never
/// inferred from bytes.
pub const MODE_TRUSTFUL: &str = "trustful";

/// The content format of a framed interactive bundle.
pub const HTML5_APP_FORMAT: &str = "html5-app";

/// The app-bundle formats whose `contentBody` is the app-bundle body
/// (`html5-app-body.schema.json`: slug, entryPoint, fallbackUrl, capabilities,
/// sources, themeHint, presents). `spa-bundle` is a wrapper's format — lamad's
/// own record — and its body is the same shape, so one parser reads both and a
/// wrapper's `presents` reaches the chrome through this lens.
pub const APP_BUNDLE_FORMATS: [&str; 2] = [HTML5_APP_FORMAT, "spa-bundle"];

/// DB-free mirror of the `content` row the facing reads.
#[derive(Debug, Clone, Default)]
pub struct RecordRow {
    pub id: String,
    pub title: String,
    pub content_format: String,
    pub reach: String,
    /// The bundle's content address as the loader renders it (CIDv1), or `None`
    /// when the record names no bytes.
    pub content_address: Option<String>,
    /// The record's `contentBody` as stored (a JSON string, or `None`).
    pub content_body: Option<String>,
    /// The projection's DHT anchor — the action the witness is read for.
    /// `None` / blank ⇒ amber whatever the evidence says.
    pub dht_anchor_hash: Option<String>,
}

/// What the loader learned about whether the network has witnessed the
/// record's action: whether a conductor OTHER than the author holds the
/// record's op as valid. Derived on every read, never written — the loader
/// takes it from the conductor (and, on the authoring peer, one other peer);
/// nothing stores it as truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WitnessEvidence {
    /// Not read: no conductor, a timeout, or an error. Reads amber — an
    /// unanswered question is never a witness.
    #[default]
    Unread,
    /// Read, and no conductor other than the author is known to hold the record
    /// as valid yet. Reads amber; the loader asks again on the next read.
    Awaiting,
    /// A conductor other than the author holds the record as valid: a remote
    /// validator's receipt, this (non-authoring) peer's own validated copy, or
    /// another peer that answered it holds the record. Reads green.
    Witnessed,
}

/// The witness reading: green iff the record carries its DHT anchor AND the
/// loader found a non-author holder ([`WitnessEvidence::Witnessed`]); amber
/// otherwise. The anchor alone is not a witness — authoring stamps it the
/// moment the action is committed, before any validator has seen it.
pub fn witness_reading(dht_anchor_hash: Option<&str>, evidence: WitnessEvidence) -> Witness {
    match (witness_from_anchor(dht_anchor_hash), evidence) {
        (Witness::Green, WitnessEvidence::Witnessed) => Witness::Green,
        _ => Witness::Amber,
    }
}

/// The inputs `build_felt_status` takes, as the resilience snapshot carries them,
/// plus the time this peer took the reading.
#[derive(Debug, Clone)]
pub struct HoldersRow {
    /// `"measured"` | `"unmeasured"`.
    pub distribution_state: String,
    pub placement_gaps: Vec<PlacementGapView>,
    pub held_by: Vec<StewardingCollectiveEntry>,
    pub tier: String,
    pub tier_declared: bool,
    /// Distinct collectives holding it under an active provide commitment, as
    /// the resilience snapshot computes it.
    pub commitment_backed_collectives: i32,
    /// RFC 3339.
    pub observed_at: String,
}

/// DB-free mirror of one `attestations` row about the record.
#[derive(Debug, Clone)]
pub struct ClaimRow {
    pub id: String,
    pub attestation_kind: String,
    pub issuer_cid: String,
    pub proof_class: String,
    pub title: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

/// DB-free mirror of one active stewardship allocation (with the steward's
/// presence display name when the presence is held).
#[derive(Debug, Clone)]
pub struct StewardRow {
    pub presence_id: String,
    pub display_name: Option<String>,
    pub contribution_type: String,
    pub allocation_ratio: f32,
    pub governance_state: String,
}

/// Everything the loader selected for one record. `None` on a lens means the
/// loader did not select it (or could not), and the view omits that lens.
#[derive(Debug, Clone, Default)]
pub struct ContentChromeRelation {
    pub record: RecordRow,
    /// The witness evidence for the record's anchor; `Unread` (amber) unless the
    /// loader read it.
    pub witness: WitnessEvidence,
    pub holders: Option<HoldersRow>,
    /// The record's forward EPR couplings; `None` when no EPR atom names it.
    pub couplings: Option<Vec<CouplingRow>>,
    pub claims: Option<Vec<ClaimRow>>,
    pub stewards: Option<Vec<StewardRow>>,
}

/// Fold one record's relation into its content-chrome facing. `computed_at` is
/// stamped by the caller (the handler edge) so the fold stays deterministic.
pub fn fold_content_chrome(
    rel: &ContentChromeRelation,
    computed_at: &str,
) -> ContentChromeFacingView {
    let record = &rel.record;
    ContentChromeFacingView {
        record_id: record.id.clone(),
        title: record.title.clone(),
        format: record.content_format.clone(),
        mode: MODE_TRUSTFUL.to_string(),
        reach: record.reach.clone(),
        witness: witness_reading(record.dht_anchor_hash.as_deref(), rel.witness)
            .as_str()
            .to_string(),
        computed_at: computed_at.to_string(),
        content_address: record
            .content_address
            .as_deref()
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .map(str::to_string),
        app: app_lens(record),
        holders: rel.holders.as_ref().map(holders_lens),
        claims: rel.claims.as_deref().map(claims_lens),
        legs: rel
            .couplings
            .as_deref()
            .map(epr_content::fold_coupling_legs),
        steward: rel.stewards.as_deref().map(steward_lens),
    }
}

/// An app bundle's declarations (entry, capabilities, sources, theme, wrapper),
/// parsed by the SDK's one lenient parser. Absent for any format that is not an
/// app bundle ([`APP_BUNDLE_FORMATS`]).
pub fn app_lens(record: &RecordRow) -> Option<ContentChromeAppLens> {
    if !APP_BUNDLE_FORMATS.contains(&record.content_format.as_str()) {
        return None;
    }
    let body = record
        .content_body
        .as_deref()
        .map(Html5AppBody::from_json_str)
        .unwrap_or_default();
    Some(ContentChromeAppLens {
        entry: body.entry_file().to_string(),
        capabilities: body
            .capabilities
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
        presents: body.presents,
        sources: (!body.sources.is_empty()).then(|| body.sources.clone()),
        theme_hint: body.theme_hint.as_ref().map(|hint| ContentChromeThemeHint {
            color_scheme: hint.color_scheme.map(|s| {
                match s {
                    ColorScheme::Light => "light",
                    ColorScheme::Dark => "dark",
                }
                .to_string()
            }),
            theme_color: hint.theme_color.clone(),
        }),
        fallback_url: body.fallback_url.clone(),
    })
}

/// The holders reading: `build_felt_status` decides the wording; this lens only
/// carries it, with the counts as non-negative integers.
pub fn holders_lens(row: &HoldersRow) -> ContentChromeHoldersLens {
    let felt = resiliency::build_felt_status(
        &row.distribution_state,
        &row.placement_gaps,
        row.held_by.clone(),
        &row.tier,
        row.tier_declared,
    );
    ContentChromeHoldersLens {
        holders: u32::try_from(felt.floor.has_households).unwrap_or(0),
        target: u32::try_from(felt.floor.wants_households).unwrap_or(0),
        headline: felt.headline,
        reassurance: felt.reassurance,
        as_of: row.observed_at.clone(),
        held_by: felt.held_by,
        commitment_backed_collectives: u32::try_from(row.commitment_backed_collectives)
            .unwrap_or(0),
    }
}

/// The Claims reading, newest first (ties broken by id, so the wire order is
/// deterministic whatever order the loader read).
pub fn claims_lens(rows: &[ClaimRow]) -> ContentChromeClaimsLens {
    let mut attestations: Vec<ContentChromeClaim> = rows
        .iter()
        .map(|r| ContentChromeClaim {
            id: r.id.clone(),
            kind: r.attestation_kind.clone(),
            issuer_cid: r.issuer_cid.clone(),
            proof_class: r.proof_class.clone(),
            title: r.title.clone(),
            created_at: r.created_at.clone(),
            revoked: r
                .revoked_at
                .as_deref()
                .is_some_and(|at| !at.trim().is_empty()),
            expires_at: r.expires_at.clone().filter(|e| !e.trim().is_empty()),
        })
        .collect();
    attestations.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));
    attestations.dedup_by(|a, b| a.id == b.id);
    ContentChromeClaimsLens { attestations }
}

/// The steward reading: largest share first, ties by presence id.
pub fn steward_lens(rows: &[StewardRow]) -> ContentChromeStewardLens {
    let mut stewards: Vec<ContentChromeSteward> = rows
        .iter()
        .map(|r| ContentChromeSteward {
            presence_id: r.presence_id.clone(),
            contribution_type: r.contribution_type.clone(),
            allocation_ratio: r.allocation_ratio,
            governance_state: r.governance_state.clone(),
            display_name: r.display_name.clone().filter(|n| !n.trim().is_empty()),
        })
        .collect();
    stewards.sort_by(|a, b| {
        b.allocation_ratio
            .total_cmp(&a.allocation_ratio)
            .then(a.presence_id.cmp(&b.presence_id))
    });
    ContentChromeStewardLens { stewards }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> RecordRow {
        RecordRow {
            id: "evolution-of-trust".into(),
            title: "The Evolution of Trust".into(),
            content_format: "html5-app".into(),
            reach: "commons".into(),
            content_address: Some("bafkreiexample".into()),
            content_body: Some(
                r##"{"slug":"evolution-of-trust","entryPoint":"index.html","capabilities":["forms","teleport","fullscreen"],"sources":["https://Fonts.example.org","https://x.example/path"],"themeHint":{"colorScheme":"dark","themeColor":"#111"},"fallbackUrl":"https://ncase.me/trust/"}"##
                    .into(),
            ),
            dht_anchor_hash: Some("uhCkkanchor".into()),
        }
    }

    fn household(id: &str, label: Option<&str>) -> StewardingCollectiveEntry {
        StewardingCollectiveEntry {
            id: id.into(),
            kind: "household".into(),
            label: label.map(str::to_string),
            intra_hub_peers: Some(1),
        }
    }

    fn holders(held_by: Vec<StewardingCollectiveEntry>) -> HoldersRow {
        HoldersRow {
            distribution_state: "measured".into(),
            placement_gaps: vec![],
            held_by,
            tier: "standard".into(),
            tier_declared: false,
            commitment_backed_collectives: 2,
            observed_at: "2026-10-15T12:00:00Z".into(),
        }
    }

    #[test]
    fn record_readings_witness_mode_and_address() {
        let view = fold_content_chrome(
            &ContentChromeRelation {
                record: record(),
                witness: WitnessEvidence::Witnessed,
                ..Default::default()
            },
            "2026-10-15T12:00:01Z",
        );
        assert_eq!(view.record_id, "evolution-of-trust");
        assert_eq!(view.mode, "trustful");
        assert_eq!(view.witness, "green");
        assert_eq!(view.reach, "commons");
        assert_eq!(view.content_address.as_deref(), Some("bafkreiexample"));
        assert_eq!(view.computed_at, "2026-10-15T12:00:01Z");

        let mut amber = record();
        amber.dht_anchor_hash = Some("  ".into());
        let view = fold_content_chrome(
            &ContentChromeRelation {
                record: amber,
                witness: WitnessEvidence::Witnessed,
                ..Default::default()
            },
            "t",
        );
        assert_eq!(view.witness, "amber", "a blank anchor is not a witness");
    }

    #[test]
    fn an_anchor_alone_is_not_a_witness() {
        // Authoring stamps the anchor the moment the action commits; until a
        // non-author holds the record the chip stays amber.
        for evidence in [WitnessEvidence::Unread, WitnessEvidence::Awaiting] {
            let view = fold_content_chrome(
                &ContentChromeRelation {
                    record: record(),
                    witness: evidence,
                    ..Default::default()
                },
                "t",
            );
            assert_eq!(view.witness, "amber", "{evidence:?}");
        }
        assert_eq!(
            ContentChromeRelation::default().witness,
            WitnessEvidence::Unread,
            "an unread witness is the default, and it reads amber"
        );
    }

    #[test]
    fn witness_reading_needs_both_the_anchor_and_a_non_author_holder() {
        use WitnessEvidence::*;
        let cases = [
            (Some("uhCkkanchor"), Witnessed, Witness::Green),
            (Some("uhCkkanchor"), Awaiting, Witness::Amber),
            (Some("uhCkkanchor"), Unread, Witness::Amber),
            (Some("  "), Witnessed, Witness::Amber),
            (None, Witnessed, Witness::Amber),
            (None, Unread, Witness::Amber),
        ];
        for (anchor, evidence, want) in cases {
            assert_eq!(
                witness_reading(anchor, evidence),
                want,
                "{anchor:?} {evidence:?}"
            );
        }
    }

    #[test]
    fn app_lens_uses_the_lenient_parser_and_drops_unknown_capabilities() {
        let app = app_lens(&record()).expect("html5-app selects the app lens");
        assert_eq!(app.entry, "index.html");
        assert_eq!(app.capabilities, vec!["forms", "fullscreen"]);
        assert!(!app.presents);
        let hint = app.theme_hint.expect("declared theme hint");
        assert_eq!(hint.color_scheme.as_deref(), Some("dark"));
        assert_eq!(hint.theme_color.as_deref(), Some("#111"));
        assert_eq!(app.fallback_url.as_deref(), Some("https://ncase.me/trust/"));
        assert_eq!(
            app.sources,
            Some(vec!["https://fonts.example.org".to_string()]),
            "declared origins only, normalized; a path is no origin"
        );

        let mut garbled = record();
        garbled.content_body = Some("not json".into());
        let app = app_lens(&garbled).unwrap();
        assert_eq!(app.entry, "index.html");
        assert!(app.capabilities.is_empty());
        assert!(
            app.sources.is_none(),
            "no declared origin → the key is absent"
        );
        assert!(app.theme_hint.is_none());

        let mut no_body = record();
        no_body.content_body = None;
        assert_eq!(app_lens(&no_body).unwrap().entry, "index.html");

        let mut markdown = record();
        markdown.content_format = "markdown".into();
        assert!(
            app_lens(&markdown).is_none(),
            "only an app bundle declares framing"
        );

        // A wrapper (lamad's own record is a spa-bundle) declares `presents`
        // through the same body.
        let wrapper = RecordRow {
            id: "lamad-spa".into(),
            content_format: "spa-bundle".into(),
            content_body: Some(
                r#"{"slug":"lamad","entryPoint":"index.html","presents":true}"#.into(),
            ),
            ..record()
        };
        let app = app_lens(&wrapper).expect("spa-bundle selects the app lens");
        assert!(app.presents);
        assert_eq!(app.entry, "index.html");
    }

    #[test]
    fn holders_headline_is_the_resiliency_folds_verbatim() {
        let row = holders(vec![
            household("h1", Some("Dowell")),
            household("h2", Some("Rivera")),
            household("h3", Some("Okafor")),
        ]);
        let lens = holders_lens(&row);
        let felt =
            resiliency::build_felt_status("measured", &[], row.held_by.clone(), "standard", false);
        assert_eq!(lens.headline, felt.headline);
        assert_eq!(
            lens.headline,
            "Held by 3 households: Dowell, Rivera, Okafor"
        );
        assert_eq!(lens.reassurance, "protected");
        assert_eq!((lens.holders, lens.target), (3, 3));
        assert_eq!(lens.as_of, "2026-10-15T12:00:00Z");
        assert_eq!(lens.commitment_backed_collectives, 2);

        let mut unmeasured = holders(vec![]);
        unmeasured.distribution_state = "unmeasured".into();
        let lens = holders_lens(&unmeasured);
        assert_eq!(lens.reassurance, "not-yet-seen");
        assert_eq!(lens.holders, 0, "zero holders is 0, never absent");
    }

    #[test]
    fn claims_are_attestations_newest_first_with_revocation_visible() {
        let rows = vec![
            ClaimRow {
                id: "a1".into(),
                attestation_kind: "attestation:content-quality".into(),
                issuer_cid: "uhCAkreviewer".into(),
                proof_class: "witness".into(),
                title: "Reviewed".into(),
                created_at: "2026-10-01T00:00:00Z".into(),
                expires_at: None,
                revoked_at: None,
            },
            ClaimRow {
                id: "a2".into(),
                attestation_kind: "attestation:inspection".into(),
                issuer_cid: "uhCAkscout".into(),
                proof_class: "computational".into(),
                title: "Inspected".into(),
                created_at: "2026-10-10T00:00:00Z".into(),
                expires_at: Some("".into()),
                revoked_at: Some("2026-10-11T00:00:00Z".into()),
            },
        ];
        let lens = claims_lens(&rows);
        let ids: Vec<&str> = lens.attestations.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["a2", "a1"]);
        assert!(lens.attestations[0].revoked);
        assert!(
            lens.attestations[0].expires_at.is_none(),
            "blank expiry is absent"
        );
        assert!(!lens.attestations[1].revoked);
    }

    #[test]
    fn couplings_fold_into_legs_and_absence_is_not_selected() {
        let rel = ContentChromeRelation {
            record: record(),
            couplings: Some(vec![
                CouplingRow {
                    epr_cid: "atom".into(),
                    leg: "governance".into(),
                    target_cid: "g".into(),
                },
                CouplingRow {
                    epr_cid: "atom".into(),
                    leg: "knowledge".into(),
                    target_cid: "k".into(),
                },
            ]),
            ..Default::default()
        };
        let view = fold_content_chrome(&rel, "t");
        let legs = view.legs.expect("couplings selected → legs present");
        assert_eq!(legs.governance.as_deref(), Some("g"));
        assert_eq!(legs.knowledge.as_deref(), Some("k"));
        assert!(legs.value.is_none());

        let bare = fold_content_chrome(
            &ContentChromeRelation {
                record: record(),
                ..Default::default()
            },
            "t",
        );
        let wire = serde_json::to_value(&bare).unwrap();
        for lens in ["holders", "claims", "legs", "steward"] {
            assert!(
                wire.get(lens).is_none(),
                "an unselected lens is absent, never null: {lens}"
            );
        }
        assert!(wire.get("app").is_some());
    }

    #[test]
    fn stewards_sort_by_share_and_selected_empty_is_present() {
        let lens = steward_lens(&[
            StewardRow {
                presence_id: "p-b".into(),
                display_name: Some("".into()),
                contribution_type: "curator".into(),
                allocation_ratio: 0.25,
                governance_state: "active".into(),
            },
            StewardRow {
                presence_id: "p-a".into(),
                display_name: Some("Matthew".into()),
                contribution_type: "author".into(),
                allocation_ratio: 0.75,
                governance_state: "active".into(),
            },
        ]);
        assert_eq!(lens.stewards[0].presence_id, "p-a");
        assert_eq!(lens.stewards[0].display_name.as_deref(), Some("Matthew"));
        assert!(lens.stewards[1].display_name.is_none());

        let view = fold_content_chrome(
            &ContentChromeRelation {
                record: record(),
                claims: Some(vec![]),
                stewards: Some(vec![]),
                ..Default::default()
            },
            "t",
        );
        let wire = serde_json::to_value(&view).unwrap();
        assert_eq!(wire["claims"]["attestations"], serde_json::json!([]));
        assert_eq!(wire["steward"]["stewards"], serde_json::json!([]));
    }
}
