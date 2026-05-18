//! Lamad-domain Wire→View converters.
//!
//! Converts internal DB models (App, Content, Relationship, HumanRelationship,
//! ContentMastery, Comment) to View types defined in `elohim_views::lamad`.

use elohim_views::shared::parse_json_opt;
use elohim_views::{
    AppView, CommentView, ContentMasteryView, ContentView, ContentWithTagsView,
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
        Self {
            id: c.id,
            h_app_id: c.h_app_id,
            title: c.title,
            description: c.description,
            content_type: c.content_type,
            content_format: c.content_format,
            blob_hash: c.blob_hash,
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
        }
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
