//! Imagodei-domain Wire→View converters.
//!
//! Converts internal DB models for identity, sessions, device policy, and
//! recovery protocol domains to View types defined in `elohim_views::imagodei`.

use elohim_views::shared::parse_json;
use elohim_views::{
    DevicePolicyView, HumanView, KeyRevocationView, LocalSessionView, MonitoringRulesInput,
    RecoveryWitnessView, RevocationVoteView, UpsertPolicyInputView,
};

use crate::db::models::{DevicePolicy, Human, KeyRevocationRow, LocalSession, RevocationVoteRow};

// ============================================================================
// Device Policy Views (Stewardship v5)
// ============================================================================

impl From<DevicePolicy> for DevicePolicyView {
    fn from(p: DevicePolicy) -> Self {
        Self {
            id: p.id,
            subject_id: p.subject_id,
            device_id: p.device_id,
            author_id: p.author_id,
            author_tier: p.author_tier,
            inherits_from: p.inherits_from,
            blocked_categories: parse_json(&p.blocked_categories_json),
            blocked_hashes: parse_json(&p.blocked_hashes_json),
            age_rating_max: p.age_rating_max,
            reach_level_max: p.reach_level_max,
            session_max_minutes: p.session_max_minutes,
            daily_max_minutes: p.daily_max_minutes,
            time_windows: parse_json(&p.time_windows_json),
            cooldown_minutes: p.cooldown_minutes,
            disabled_features: parse_json(&p.disabled_features_json),
            disabled_routes: parse_json(&p.disabled_routes_json),
            require_approval: parse_json(&p.require_approval_json),
            log_sessions: p.log_sessions != 0,
            log_categories: p.log_categories != 0,
            log_policy_events: p.log_policy_events != 0,
            retention_days: p.retention_days,
            subject_can_view: p.subject_can_view != 0,
            effective_from: p.effective_from,
            effective_until: p.effective_until,
            version: p.version,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

// ============================================================================
// Local Session Views
// ============================================================================

impl From<LocalSession> for LocalSessionView {
    fn from(s: LocalSession) -> Self {
        Self {
            id: s.id,
            human_id: s.human_id,
            agent_pub_key: s.agent_pub_key,
            doorway_url: s.doorway_url,
            doorway_id: s.doorway_id,
            identifier: s.identifier,
            display_name: s.display_name,
            profile_image_hash: s.profile_image_hash,
            is_active: s.is_active == 1,
            created_at: s.created_at,
            updated_at: s.updated_at,
            last_synced_at: s.last_synced_at,
            bootstrap_url: s.bootstrap_url,
        }
    }
}

// ============================================================================
// Human Identity Views (imagodei pillar)
// ============================================================================

impl From<Human> for HumanView {
    fn from(h: Human) -> Self {
        let affinities: Vec<String> = serde_json::from_str(&h.affinities).unwrap_or_default();
        Self {
            id: h.id,
            agent_pub_key: h.agent_pub_key,
            display_name: h.display_name,
            bio: h.bio,
            affinities,
            profile_reach: h.profile_reach,
            location: h.location,
            profile_photo_url: h.profile_photo_url,
            h_app_id: h.h_app_id,
            created_at: h.created_at,
            updated_at: h.updated_at,
            dht_anchor_hash: h.dht_anchor_hash,
        }
    }
}

// ============================================================================
// Recovery Protocol Phase 2 Views
// Source of truth: DHT (imagodei RecoveryWitness / KeyRevocation / RevocationVote entries)
// ============================================================================

impl From<crate::db::models::RecoveryWitnessRow> for RecoveryWitnessView {
    fn from(r: crate::db::models::RecoveryWitnessRow) -> Self {
        Self {
            dht_anchor_hash: r.dht_anchor_hash,
            recovery_request_hash: r.recovery_request_hash,
            witness_agent_id: r.witness_agent_id,
            human_id: r.human_id,
            note: r.note,
            submitted_at: r.submitted_at,
        }
    }
}

impl From<KeyRevocationRow> for KeyRevocationView {
    fn from(r: KeyRevocationRow) -> Self {
        // dht_anchor_hash is BLOB (Vec<u8>) on the DB side; serialize to
        // hex for the wire so clients see a stable string identifier.
        Self {
            dht_anchor_hash: hex::encode(&r.dht_anchor_hash),
            id: r.id,
            subject_human_id: r.subject_human_id,
            revoked_key: r.revoked_key,
            reason: r.reason,
            trigger_type: r.trigger_type,
            initiated_by_cid: r.initiated_by_cid,
            required_votes: r.required_votes as u32,
            current_votes: r.current_votes as u32,
            threshold_reached: r.threshold_reached == 1,
            effective_at: r.effective_at,
            derived_compromise_at: r.derived_compromise_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

impl From<RevocationVoteRow> for RevocationVoteView {
    fn from(r: RevocationVoteRow) -> Self {
        Self {
            dht_anchor_hash: r.dht_anchor_hash,
            id: r.id,
            revocation_dht_anchor_hash: r.revocation_dht_anchor_hash,
            revocation_id: r.revocation_id,
            steward_id: r.steward_id,
            approved: r.approved == 1,
            attestation: r.attestation,
            voted_at: r.voted_at,
        }
    }
}

// ============================================================================
// Policy helper (free function — convert UpsertPolicyInputView to DB input)
// ============================================================================

/// Convert UpsertPolicyInputView to DB input with author context.
pub fn upsert_policy_to_db_input(
    input_view: UpsertPolicyInputView,
    author_id: &str,
    author_tier: &str,
) -> crate::db::device_policies::CreateDevicePolicyInput {
    let this = input_view;
    let monitoring = this.monitoring_rules.unwrap_or(MonitoringRulesInput {
        log_sessions: false,
        log_categories: false,
        log_policy_events: true,
        retention_days: 30,
        subject_can_view: true,
    });
    crate::db::device_policies::CreateDevicePolicyInput {
        subject_id: this.subject_id.unwrap_or_default(),
        device_id: this.device_id,
        author_id: author_id.to_string(),
        author_tier: author_tier.to_string(),
        inherits_from: None,
        blocked_categories_json: serde_json::to_string(&this.content_rules.blocked_categories)
            .unwrap_or_else(|_| "[]".into()),
        blocked_hashes_json: serde_json::to_string(&this.content_rules.blocked_hashes)
            .unwrap_or_else(|_| "[]".into()),
        age_rating_max: this.content_rules.age_rating_max,
        reach_level_max: this.content_rules.reach_level_max,
        session_max_minutes: this.time_rules.session_max_minutes,
        daily_max_minutes: this.time_rules.daily_max_minutes,
        time_windows_json: serde_json::to_string(
            &this
                .time_rules
                .time_windows
                .iter()
                .map(|v| &v.0)
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".into()),
        cooldown_minutes: this.time_rules.cooldown_minutes,
        disabled_features_json: serde_json::to_string(&this.feature_rules.disabled_features)
            .unwrap_or_else(|_| "[]".into()),
        disabled_routes_json: serde_json::to_string(&this.feature_rules.disabled_routes)
            .unwrap_or_else(|_| "[]".into()),
        require_approval_json: serde_json::to_string(&this.feature_rules.require_approval)
            .unwrap_or_else(|_| "[]".into()),
        log_sessions: monitoring.log_sessions,
        log_categories: monitoring.log_categories,
        log_policy_events: monitoring.log_policy_events,
        retention_days: monitoring.retention_days,
        subject_can_view: monitoring.subject_can_view,
    }
}
