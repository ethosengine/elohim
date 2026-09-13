// elohim/elohim-views/src/projection.rs

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Wire view for a doorway projection commitment.
///
/// Represents a pillar EPR's projection at a specific doorway: the URL path it
/// serves from, the caching mode, gate hints for the access layer, and the
/// steward-direct endpoint fallback. Seeded via REA Commitment with
/// `action="project-epr"` on the elohim DNA.
///
/// Source of truth: Holochain DHT (`Commitment` entry type on elohim DNA
/// `content_store_integrity` zome, action discriminator `"project-epr"` — see
/// `REA_ACTIONS` in the integrity zome). The `rea_commitments` SQLite table is
/// a read-optimized projection. Category A — notarized commitment;
/// `dht_anchor_hash` links back to DHT.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprProjectionView {
    /// REA Commitment ID that backs this projection declaration.
    pub commitment_id: String,
    /// EPR atom CID (CIDv1 base32) that is being projected.
    pub epr_id: String,
    /// Doorway instance identifier (e.g. "doorway:alpha-elohim-host").
    pub doorway_id: String,
    /// URL path this projection is served from (e.g. "/lamad").
    pub url_path: String,
    /// The public names this contract answers for. EMPTY = ANY host, which is
    /// every contract before rung 4 and remains byte-for-byte today's routing.
    ///
    /// A LIST, matching Gateway API `hostnames`: ONE contract per (doorway,
    /// EPR) names many names — the field never multiplies rows. The router
    /// indexes each name as its own `RouteKey { host, path }`; a host-BOUND
    /// contract outranks an any-host one at equal path.
    ///
    /// `serde(default)`: a contract written before this field existed reads as
    /// any-host, so no existing contract becomes unroutable when the field
    /// arrives (C3 liveness), and nothing is defaulted into a NEW meaning
    /// (C10 contract-evolution honesty).
    #[serde(default)]
    pub hostnames: Vec<String>,
    /// WHICH TIER of the canonical-head election these hostnames serve.
    ///
    /// A label on the contract, never a stored version: nothing here pins a
    /// head, so promotion stays the collective's one notarized act
    /// (`declare_earned_canonical_head`) and no doorway gets a vote.
    ///
    /// `serde(default)` = [`Channel::Converged`] — today's behaviour for every
    /// contract that does not say otherwise.
    #[serde(default)]
    pub channel: Channel,
    /// How the doorway serves this projection — cached build vs steward-direct relay.
    pub mode: ProjectionMode,
    /// Reach class of the projected EPR atom.
    pub reach: String,
    /// Base href for the SPA or static asset tree (e.g. "/lamad/").
    pub base_href: String,
    /// Entry file relative to base_href (e.g. "index.html").
    pub entry_file: String,
    /// Serve `entry_file` for extension-less deep routes (SPA deep-link
    /// fallback). Defaults `true` for bundle EPRs. See spec §12.2 of
    /// `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md`
    /// — a sub-path is a ROUTE (fallback-eligible) iff its final segment
    /// contains no `.`; otherwise it is an ASSET and a miss stays an honest 404.
    #[serde(default = "crate::shared::default_true")]
    pub spa_fallback: bool,
    /// Additional URL paths that redirect to url_path (legacy aliases).
    pub redirects_from: Vec<String>,
    /// Route-level alias promises (spec §4 — the notarized bridge story).
    #[serde(default)]
    pub redirect_templates: Vec<RedirectTemplate>,
    /// GRANTED route claims (spec §3.2). None = no claims granted.
    #[serde(default)]
    pub route_claims: Option<RouteClaimGrant>,
    /// EPR CID of a preview / draft build, surfaced alongside the stable build.
    pub preview_epr_ref: Option<String>,
    /// Gate hints the access layer should surface when reach is restricted.
    pub gate_hints: Vec<GateHintRef>,
    /// True when this projection has no onward link — a terminal destination.
    pub dead_end: bool,
    /// Steward-direct endpoint, populated when mode is StewardDirect.
    pub steward_direct_endpoint: Option<StewardDirectEndpoint>,
    /// RFC3339 timestamp of when the projection was seeded into the doorway.
    pub seeded_at: String,
    /// PeerId of the steward node that seeded this projection.
    pub seeded_by: String,
}

/// WHICH TIER of an EPR's canonical-head election a hostname serves.
///
/// The two tiers are not invented here: `content_store` already notarizes
/// them as the `canonical-head:earned` and `canonical-head:staging` link tags
/// on a `Content` id's `canonical_head` anchor, arbitrated by the pure
/// `select_canonical_winner` / `select_staging_candidate` pair every peer runs
/// identically. This enum only names WHICH of them a contract's hostnames
/// answer from — it adds no head, no row and no election candidate.
///
/// [`Channel::Converged`] is the default everywhere absence is possible: an
/// older contract, an older peer, a wire payload written before the field
/// existed. Defaulting the other way would silently serve unreleased bytes.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum Channel {
    /// The EARNED winner of the election — what the collective has promoted.
    #[default]
    Converged,
    /// The STAGING declaration standing beneath the earned winner: the next
    /// version awaiting promotion.
    ///
    /// Where no staging declaration stands, this channel answers a NAMED
    /// ABSENCE. It must never fall through to the converged head — silently
    /// serving production bytes at a staging name is how a candidate channel
    /// stops meaning anything (C4 honest absence).
    Candidate,
}

/// How the doorway serves a projected EPR.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum ProjectionMode {
    /// Doorway holds a cached build of the EPR atom and serves it directly.
    Cached,
    /// Doorway relays requests to the steward node that holds the live build.
    StewardDirect,
}

/// A reference to a related EPR atom surfaced as a gate hint.
///
/// When the access layer needs to guide a person toward fulfilling a gate
/// condition, these refs let the UI link to the relevant EPR (e.g. a
/// membership prerequisite, a capability to earn, a payment to offer).
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GateHintRef {
    /// CIDv1 base32 of the EPR atom being referenced.
    pub epr_ref: String,
    /// Short human-readable label for the hint (optional — populated from
    /// locally held atom metadata, absent when not held).
    pub label: Option<String>,
    /// Semantic relation type that explains why this EPR is being surfaced.
    pub relation: GateHintRelation,
}

/// Semantic relation type for a gate hint reference.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum GateHintRelation {
    /// A person who can grant access to the gated resource.
    PersonWhoCanGrant,
    /// A membership the person must hold before access is granted.
    MembershipPrerequisite,
    /// Content the person needs to sync before proceeding.
    ContentToSync,
    /// A place the person should visit to unlock access.
    PlaceToVisit,
    /// A capability the person can earn to satisfy the gate.
    CapabilityToEarn,
    /// A payment the person can offer to satisfy the gate.
    PaymentToOffer,
    /// A witness who needs to be involved to satisfy the gate.
    WitnessToInvolve,
}

/// Steward-direct relay endpoint for StewardDirect projection mode.
///
/// Populated when mode == StewardDirect; identifies the steward node that
/// holds the live build and describes TLS and projection scope constraints.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardDirectEndpoint {
    /// libp2p or iroh PeerId of the steward node.
    pub peer_id: String,
    /// Alternative host name or IP when the peer cannot be reached via PeerId
    /// alone (e.g. on a private LAN).
    pub alt_host: Option<String>,
    /// TLS certificate Subject Alternative Name the doorway should validate
    /// when opening the relay connection.
    pub tls_cert_san: String,
    /// EPR CIDs that this endpoint accepts relay requests for.
    pub accepts_projection_for: Vec<String>,
}

/// A serializable route-claim template (spec §3, 2026-06-06 route-claims design).
/// `template` and `fragments` values use `{id}` / `{n}` placeholders, substituted
/// segment-safe (a placeholder binds exactly one path segment).
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RouteClaimTemplate {
    /// The EPR contentType this claim binds (e.g. "path").
    pub content_type: String,
    /// Mount-relative route template (e.g. "path/{id}").
    pub template: String,
    /// Fragment-type → deeper route template (e.g. step → "path/{id}/step/{n}").
    #[serde(default)]
    pub fragments: std::collections::BTreeMap<String, String>,
}

/// The GRANTED claims block on a project-epr commitment (spec §3.2): the
/// steward-authored operative routing law. `claims_manifest_cid` fingerprints
/// the bundle-manifest declaration acknowledged at grant time (claims-stale
/// drift detection, §3.4).
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RouteClaimGrant {
    pub schema_version: u32,
    pub claims_manifest_cid: Option<String>,
    pub claims: Vec<RouteClaimTemplate>,
}

/// A route-level alias promise on the commitment (spec §4): requests matching
/// `from` 302 to `to`. One hop; `to` must be a canonical address.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RedirectTemplate {
    pub from: String,
    pub to: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_view_serializes_to_camel_case() {
        let view = EprProjectionView {
            commitment_id: "abc".into(),
            epr_id: "lamad-spa".into(),
            doorway_id: "doorway:alpha-elohim-host".into(),
            url_path: "/lamad".into(),
            hostnames: vec![],
            channel: Channel::Converged,
            mode: ProjectionMode::Cached,
            reach: "commons".into(),
            base_href: "/lamad/".into(),
            entry_file: "index.html".into(),
            spa_fallback: true,
            redirects_from: vec![],
            redirect_templates: vec![],
            route_claims: None,
            preview_epr_ref: None,
            gate_hints: vec![],
            dead_end: false,
            steward_direct_endpoint: None,
            seeded_at: "2026-05-25T00:00:00Z".into(),
            seeded_by: "12D3Koo...".into(),
        };
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("\"commitmentId\":\"abc\""));
        assert!(json.contains("\"eprId\":\"lamad-spa\""));
        assert!(json.contains("\"urlPath\":\"/lamad\""));
        assert!(json.contains("\"mode\":\"cached\""));
        assert!(json.contains("\"baseHref\":\"/lamad/\""));
        assert!(json.contains("\"spaFallback\":true"));
        assert!(json.contains("\"hostnames\":[]"));
        assert!(json.contains("\"channel\":\"converged\""));
    }

    #[test]
    fn hostnames_and_channel_default_to_any_host_converged_when_absent() {
        // A contract written before rung 4 omits both keys. It must read as
        // "answers for any host, serves the earned winner" — today's exact
        // behaviour — rather than becoming unroutable (C3) or being defaulted
        // into serving a staging head (C4/C10).
        let json = r#"{
            "commitmentId": "abc",
            "eprId": "lamad-spa",
            "doorwayId": "doorway:alpha-elohim-host",
            "urlPath": "/lamad",
            "mode": "cached",
            "reach": "commons",
            "baseHref": "/lamad/",
            "entryFile": "index.html",
            "redirectsFrom": [],
            "previewEprRef": null,
            "gateHints": [],
            "deadEnd": false,
            "stewardDirectEndpoint": null,
            "seededAt": "2026-05-25T00:00:00Z",
            "seededBy": "12D3Koo..."
        }"#;
        let view: EprProjectionView = serde_json::from_str(json).unwrap();
        assert!(view.hostnames.is_empty(), "absent hostnames must mean ANY host");
        assert_eq!(view.channel, Channel::Converged, "absent channel must mean converged");
    }

    #[test]
    fn channel_serializes_to_camel_case_both_ways() {
        assert_eq!(serde_json::to_string(&Channel::Converged).unwrap(), "\"converged\"");
        assert_eq!(serde_json::to_string(&Channel::Candidate).unwrap(), "\"candidate\"");
        assert_eq!(
            serde_json::from_str::<Channel>("\"candidate\"").unwrap(),
            Channel::Candidate
        );
    }

    #[test]
    fn a_host_bound_candidate_contract_round_trips() {
        let json = r#"{
            "commitmentId": "abc",
            "eprId": "lamad-spa",
            "doorwayId": "doorway:alpha-elohim-host",
            "urlPath": "/lamad",
            "hostnames": ["alpha.elohim.local"],
            "channel": "candidate",
            "mode": "cached",
            "reach": "stewards-only",
            "baseHref": "/lamad/",
            "entryFile": "index.html",
            "redirectsFrom": [],
            "previewEprRef": null,
            "gateHints": [],
            "deadEnd": false,
            "stewardDirectEndpoint": null,
            "seededAt": "2026-05-25T00:00:00Z",
            "seededBy": "12D3Koo..."
        }"#;
        let view: EprProjectionView = serde_json::from_str(json).unwrap();
        assert_eq!(view.hostnames, vec!["alpha.elohim.local".to_string()]);
        assert_eq!(view.channel, Channel::Candidate);
    }

    #[test]
    fn projection_view_spa_fallback_defaults_true_when_absent() {
        // Wire payloads from pre-spaFallback seeds omit the field; serde default
        // must fill `true` so existing bundle projections stay SPA-eligible.
        let json = r#"{
            "commitmentId": "abc",
            "eprId": "lamad-spa",
            "doorwayId": "doorway:alpha-elohim-host",
            "urlPath": "/lamad",
            "mode": "cached",
            "reach": "commons",
            "baseHref": "/lamad/",
            "entryFile": "index.html",
            "redirectsFrom": [],
            "previewEprRef": null,
            "gateHints": [],
            "deadEnd": false,
            "stewardDirectEndpoint": null,
            "seededAt": "2026-05-25T00:00:00Z",
            "seededBy": "12D3Koo..."
        }"#;
        let view: EprProjectionView = serde_json::from_str(json).unwrap();
        assert!(view.spa_fallback, "spaFallback must default to true");
    }

    #[test]
    fn projection_mode_steward_direct_serializes_correctly() {
        let view = ProjectionMode::StewardDirect;
        let json = serde_json::to_string(&view).unwrap();
        assert_eq!(json, "\"stewardDirect\"");
    }

    #[test]
    fn gate_hint_relation_all_variants_serialize() {
        use GateHintRelation::*;
        for (variant, expected) in [
            (PersonWhoCanGrant, "\"personWhoCanGrant\""),
            (MembershipPrerequisite, "\"membershipPrerequisite\""),
            (ContentToSync, "\"contentToSync\""),
            (PlaceToVisit, "\"placeToVisit\""),
            (CapabilityToEarn, "\"capabilityToEarn\""),
            (PaymentToOffer, "\"paymentToOffer\""),
            (WitnessToInvolve, "\"witnessToInvolve\""),
        ] {
            assert_eq!(serde_json::to_string(&variant).unwrap(), expected);
        }
    }
}
