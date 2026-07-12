//! Lamad-domain Wire→View converters.
//!
//! Converts internal DB models (App, Content, Relationship, HumanRelationship,
//! ContentMastery, Comment) to View types defined in `elohim_views::lamad`.

use elohim_views::shared::parse_json_opt;
use elohim_views::{
    AppView, CommentView, ContentHeadView, ContentMasteryView, ContentView, ContentWithTagsView,
    HumanRelationshipView, RelationshipView, RelationshipWithContentView,
};

use crate::db::models::{
    App, Content, ContentMastery, ContentWithTags, HumanRelationship, Relationship,
    RelationshipWithContent,
};

// ============================================================================
// App View
// ============================================================================

impl From<App> for AppView {
    fn from(a: App) -> Self {
        Self {
            id: a.id,
            name: a.name,
            description: a.description,
            created_at: a.created_at,
            enabled: a.enabled == 1,
        }
    }
}

// ============================================================================
// Content Views
// ============================================================================

impl From<Content> for ContentView {
    fn from(c: Content) -> Self {
        // REQ-F10 trust label. Priority: notarized (dht_anchor_hash) beats
        // published (p2p_published_at) beats unconfirmed. The "unconfirmed" arm
        // covers BOTH the crdt_converged_at-only (amber) row and the all-null
        // row — a converged-only row is functional but must never read as
        // notarized. Computed by reference before the fields are moved.
        let trust = trust_label(c.dht_anchor_hash.is_some(), c.p2p_published_at.is_some());
        Self {
            id: c.id,
            h_app_id: c.h_app_id,
            title: c.title,
            description: c.description,
            content_type: c.content_type,
            content_format: c.content_format,
            blob_hash: c.blob_hash,
            server_blob_hash: c.server_blob_hash,
            blob_cid: c.blob_cid,
            content_size_bytes: c.content_size_bytes,
            metadata: parse_json_opt(&c.metadata_json),
            reach: c.reach,
            validation_status: c.validation_status,
            created_by: c.created_by,
            created_at: c.created_at,
            updated_at: c.updated_at,
            content_body: c.content_body,
            dht_anchor_hash: c.dht_anchor_hash,
            trust,
        }
    }
}

/// Compute the REQ-F10 `trust` legibility label from a row's provenance markers.
/// `notarized` (green) > `published` (blue/peer-attested) > `unconfirmed`
/// (amber — CRDT-converged-only OR all-null). Single source so both
/// `From<Content>` and `content_view_from_epr_head` agree.
fn trust_label(has_dht_anchor: bool, has_p2p_published: bool) -> String {
    if has_dht_anchor {
        "notarized".to_string()
    } else if has_p2p_published {
        "published".to_string()
    } else {
        "unconfirmed".to_string()
    }
}

/// Convert ContentWithTags → ContentView (strips tags, for backward compat)
impl From<ContentWithTags> for ContentView {
    fn from(c: ContentWithTags) -> Self {
        c.content.into()
    }
}

/// Construct a minimal ContentView from an EPR Head resolved via P2P.
/// Provides enough data for the frontend to render content metadata
/// while the full content body is fetched asynchronously.
pub fn content_view_from_epr_head(head: &crate::epr_codec::EprHead) -> ContentView {
    ContentView {
        id: head.id.clone(),
        h_app_id: "lamad".to_string(),
        title: head.lamad.title.clone(),
        description: head.lamad.description.clone(),
        content_type: head.lamad.content_type.clone(),
        content_format: head
            .lamad
            .content_format
            .clone()
            .unwrap_or_else(|| "markdown".to_string()),
        blob_hash: None,
        // EPR-head projection carries no SSR server bundle; CSR fallback applies.
        server_blob_hash: None,
        blob_cid: if head.content.is_empty() {
            None
        } else {
            Some(head.content.clone())
        },
        content_size_bytes: None,
        metadata: None,
        reach: head
            .qahal
            .reach
            .clone()
            .unwrap_or_else(|| "commons".to_string()),
        validation_status: "valid".to_string(),
        created_by: head.author.clone(),
        created_at: head
            .updated
            .clone()
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string()),
        updated_at: head
            .updated
            .clone()
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string()),
        content_body: None,
        dht_anchor_hash: None,
        // EPR-head projection carries no dht_anchor_hash and no
        // p2p_published_at column — it is a minimal placeholder while the body
        // fetches, so it labels honestly as unconfirmed (never over-claim a
        // notarization the projection cannot evidence).
        trust: trust_label(false, false),
    }
}

impl From<ContentWithTags> for ContentWithTagsView {
    fn from(c: ContentWithTags) -> Self {
        Self {
            content: c.content.into(),
            tags: c.tags,
        }
    }
}

/// Project a content row's notary markers into a [`ContentHeadView`] — the
/// notary-authority HEAD answer (HEAD-election, Plan C3 / Leg 3 HTTP surface).
///
/// Returns `None` when BOTH `declared_head_action_hash` and `dht_anchor_hash`
/// are absent: the notary has no HEAD for this id, and the caller renders that
/// as an honest 404 rather than fabricating an answer. When present,
/// `head_action_hash` prefers the explicitly-declared HEAD and falls back to
/// the DHT anchor; `declared` records which of the two supplied it. `trust`
/// reuses the shared `trust_label` so this surface agrees with `ContentView`.
pub fn content_head_view_from_content(c: &Content) -> Option<ContentHeadView> {
    // `?` yields None exactly when neither a declared head nor a DHT anchor
    // exists — the "no notary answer" case the caller 404s.
    let head_action_hash = c
        .declared_head_action_hash
        .clone()
        .or_else(|| c.dht_anchor_hash.clone())?;
    Some(ContentHeadView {
        content_id: c.id.clone(),
        head_action_hash,
        declared: c.declared_head_action_hash.is_some(),
        dht_anchor_hash: c.dht_anchor_hash.clone(),
        trust: trust_label(c.dht_anchor_hash.is_some(), c.p2p_published_at.is_some()),
        blob_hash: c.blob_hash.clone(),
        updated_at: Some(c.updated_at.clone()),
    })
}

// ============================================================================
// Relationship Views
// ============================================================================

impl From<Relationship> for RelationshipView {
    fn from(r: Relationship) -> Self {
        Self {
            id: r.id,
            h_app_id: r.h_app_id,
            source_id: r.source_id,
            target_id: r.target_id,
            relationship_type: r.relationship_type,
            confidence: r.confidence,
            inference_source: r.inference_source,
            is_bidirectional: r.is_bidirectional == 1,
            inverse_relationship_id: r.inverse_relationship_id,
            provenance_chain: parse_json_opt(&r.provenance_chain_json),
            governance_layer: r.governance_layer,
            reach: r.reach,
            metadata: parse_json_opt(&r.metadata_json),
            created_at: r.created_at,
            updated_at: r.updated_at,
            dht_anchor_hash: r.dht_anchor_hash,
        }
    }
}

impl From<RelationshipWithContent> for RelationshipWithContentView {
    fn from(r: RelationshipWithContent) -> Self {
        Self {
            relationship: r.relationship.into(),
            source: r.source.map(|c| c.into()),
            target: r.target.map(|c| c.into()),
        }
    }
}

// ============================================================================
// Human Relationship Views
// ============================================================================

impl From<HumanRelationship> for HumanRelationshipView {
    fn from(h: HumanRelationship) -> Self {
        Self {
            id: h.id,
            h_app_id: h.h_app_id,
            party_a_id: h.party_a_id,
            party_b_id: h.party_b_id,
            relationship_type: h.relationship_type,
            intimacy_level: h.intimacy_level,
            is_bidirectional: h.is_bidirectional == 1,
            consent_given_by_a: h.consent_given_by_a == 1,
            consent_given_by_b: h.consent_given_by_b == 1,
            custody_enabled_by_a: h.custody_enabled_by_a == 1,
            custody_enabled_by_b: h.custody_enabled_by_b == 1,
            auto_custody_enabled: h.auto_custody_enabled == 1,
            emergency_access_enabled: h.emergency_access_enabled == 1,
            initiated_by: h.initiated_by,
            verified_at: h.verified_at,
            governance_layer: h.governance_layer,
            reach: h.reach,
            context: parse_json_opt(&h.context_json),
            created_at: h.created_at,
            updated_at: h.updated_at,
            expires_at: h.expires_at,
            dht_anchor_hash: h.dht_anchor_hash,
        }
    }
}

// ============================================================================
// Content Mastery Views
// ============================================================================

impl From<ContentMastery> for ContentMasteryView {
    fn from(m: ContentMastery) -> Self {
        Self {
            id: m.id,
            h_app_id: m.h_app_id,
            human_id: m.human_id,
            content_id: m.content_id,
            mastery_level: m.mastery_level,
            mastery_level_index: m.mastery_level_index,
            freshness_score: m.freshness_score,
            needs_refresh: m.needs_refresh == 1,
            engagement_count: m.engagement_count,
            last_engagement_type: m.last_engagement_type,
            last_engagement_at: m.last_engagement_at,
            level_achieved_at: m.level_achieved_at,
            content_version_at_mastery: m.content_version_at_mastery,
            assessment_evidence: parse_json_opt(&m.assessment_evidence_json),
            privileges: parse_json_opt(&m.privileges_json),
            created_at: m.created_at,
            updated_at: m.updated_at,
            dht_anchor_hash: m.dht_anchor_hash,
        }
    }
}

// ============================================================================
// Comment Views
// ============================================================================

impl From<crate::db::models::Comment> for CommentView {
    fn from(c: crate::db::models::Comment) -> Self {
        Self {
            id: c.id,
            content_id: c.content_id,
            human_id: c.human_id,
            body: c.body,
            reach: c.reach,
            governance_state: c.governance_state,
            created_at: c.created_at,
        }
    }
}

#[cfg(test)]
mod trust_label_tests {
    use super::*;

    /// Build a bare Content row with all provenance markers NULL; callers set
    /// only the marker under test.
    fn bare_content() -> Content {
        Content {
            id: "epr:trust-fixture".to_string(),
            h_app_id: "lamad".to_string(),
            title: "Trust fixture".to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "public".to_string(),
            validation_status: "valid".to_string(),
            created_by: None,
            created_at: "2026-07-01T00:00:00Z".to_string(),
            updated_at: "2026-07-01T00:00:00Z".to_string(),
            content_body: None,
            dht_anchor_hash: None,
            p2p_published_at: None,
            server_blob_hash: None,
            crdt_converged_at: None,
            declared_head_action_hash: None,
            declared_head_at: None,
        }
    }

    #[test]
    fn trust_notarized_for_dht_anchor_row() {
        let c = Content {
            dht_anchor_hash: Some("uhCkk_anchor".to_string()),
            ..bare_content()
        };
        assert_eq!(ContentView::from(c).trust, "notarized");
    }

    #[test]
    fn trust_notarized_wins_over_published_and_converged() {
        // Priority guard: dht_anchor beats p2p_published beats crdt_converged.
        let c = Content {
            dht_anchor_hash: Some("uhCkk_anchor".to_string()),
            p2p_published_at: Some("2026-07-01T00:00:00Z".to_string()),
            crdt_converged_at: Some("2026-07-01T00:00:00Z".to_string()),
            ..bare_content()
        };
        assert_eq!(ContentView::from(c).trust, "notarized");
    }

    #[test]
    fn trust_published_for_p2p_published_row() {
        let c = Content {
            p2p_published_at: Some("2026-07-01T00:00:00Z".to_string()),
            ..bare_content()
        };
        assert_eq!(ContentView::from(c).trust, "published");
    }

    #[test]
    fn trust_unconfirmed_for_crdt_converged_only_row() {
        // The amber tier: converged-but-not-notarized must read as unconfirmed,
        // never notarized — the whole point of REQ-F10 legibility.
        let c = Content {
            crdt_converged_at: Some("2026-07-01T00:00:00Z".to_string()),
            ..bare_content()
        };
        assert_eq!(ContentView::from(c).trust, "unconfirmed");
    }

    #[test]
    fn trust_unconfirmed_for_all_null_row() {
        assert_eq!(ContentView::from(bare_content()).trust, "unconfirmed");
    }

    // ── content_head_view_from_content: the three notary-answer states ──

    #[test]
    fn head_view_none_when_no_notary_answer() {
        // Neither a declared head nor a DHT anchor → no HEAD → None (→ 404).
        assert!(content_head_view_from_content(&bare_content()).is_none());
    }

    #[test]
    fn head_view_anchor_only_falls_back_and_is_not_declared() {
        // A DHT-anchored row with no explicit declaration: the anchor IS the head,
        // declared=false, and trust reads notarized.
        let c = Content {
            dht_anchor_hash: Some("uhCkk_anchor".to_string()),
            ..bare_content()
        };
        let v = content_head_view_from_content(&c).expect("anchor row has a head");
        assert_eq!(v.head_action_hash, "uhCkk_anchor");
        assert!(!v.declared);
        assert_eq!(v.dht_anchor_hash.as_deref(), Some("uhCkk_anchor"));
        assert_eq!(v.trust, "notarized");
        assert_eq!(v.content_id, "epr:trust-fixture");
    }

    #[test]
    fn head_view_declared_head_wins_over_anchor() {
        // An explicitly-declared head takes precedence over the anchor for
        // head_action_hash, and declared=true.
        let c = Content {
            declared_head_action_hash: Some("uhCkk_declared".to_string()),
            dht_anchor_hash: Some("uhCkk_anchor".to_string()),
            ..bare_content()
        };
        let v = content_head_view_from_content(&c).expect("declared row has a head");
        assert_eq!(v.head_action_hash, "uhCkk_declared");
        assert!(v.declared);
        // The anchor is still surfaced for provenance even when the declared head leads.
        assert_eq!(v.dht_anchor_hash.as_deref(), Some("uhCkk_anchor"));
        assert_eq!(v.trust, "notarized");
    }
}
