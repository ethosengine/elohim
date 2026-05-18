//! Shefa-domain Wire→View converters.
//!
//! Converts internal DB models for the economic/stewardship domain to View
//! types defined in `elohim_views::shefa`.

use elohim_views::shared::{parse_json, parse_json_opt};
use elohim_views::{
    AccessGrantView, AgreementView, ContributorDashboardView, ContributorImpactView,
    ContributorPresenceView, ContentStewardshipView, EconomicEventView, MeasureView,
    NodeStewardshipView, PremiumGateView, ReaCommitmentView, ResponsibilityDemandConfigView,
    StewardAffinityView, StewardCredentialView, StewardRevenueSummaryView, StewardedNodeView,
    StewardshipAllocationView, StewardshipAllocationWithPresenceView, TokenBalanceView,
    TokenDecayEventView, TokenMintEventView, TokenTransferView,
};

use crate::db::contributors::ImpactSummary;
use crate::db::models::{
    AccessGrant, AgreementRow, ContributorDashboard, ContributorPresence, ContentStewardship,
    EconomicEvent, NodeStewardship, PremiumGate, ReaCommitment, ResponsibilityDemandConfig,
    StewardAffinity, StewardCredential, StewardedNode, StewardshipAllocation,
    StewardshipAllocationWithPresence, TokenBalance, TokenDecayEvent, TokenMintEvent, TokenTransfer,
};
use crate::db::steward_operations::RevenueSummary;

// ============================================================================
// Contributor Presence Views
// ============================================================================

impl From<ContributorPresence> for ContributorPresenceView {
    fn from(c: ContributorPresence) -> Self {
        Self {
            id: c.id,
            h_app_id: c.h_app_id,
            display_name: c.display_name,
            presence_state: c.presence_state,
            external_identifiers: parse_json_opt(&c.external_identifiers_json),
            establishing_content_ids: parse_json(&c.establishing_content_ids_json),
            affinity_total: c.affinity_total,
            unique_engagers: c.unique_engagers,
            citation_count: c.citation_count,
            recognition_score: c.recognition_score,
            recognition_by_content: parse_json_opt(&c.recognition_by_content_json),
            last_recognition_at: c.last_recognition_at,
            steward_id: c.steward_id,
            stewardship_started_at: c.stewardship_started_at,
            stewardship_commitment_id: c.stewardship_commitment_id,
            stewardship_quality_score: c.stewardship_quality_score,
            claim_initiated_at: c.claim_initiated_at,
            claim_verified_at: c.claim_verified_at,
            claim_verification_method: c.claim_verification_method,
            claim_evidence: parse_json_opt(&c.claim_evidence_json),
            claimed_agent_id: c.claimed_agent_id,
            claim_recognition_transferred_value: c.claim_recognition_transferred_value,
            claim_facilitated_by: c.claim_facilitated_by,
            image: c.image,
            note: c.note,
            metadata: parse_json_opt(&c.metadata_json),
            created_at: c.created_at,
            updated_at: c.updated_at,
            dht_anchor_hash: c.dht_anchor_hash,
        }
    }
}

// ============================================================================
// Economic Event Views
// ============================================================================

impl From<EconomicEvent> for EconomicEventView {
    fn from(e: EconomicEvent) -> Self {
        Self {
            id: e.id,
            h_app_id: e.h_app_id,
            action: e.action,
            provider: e.provider,
            receiver: e.receiver,
            resource_conforms_to: e.resource_conforms_to,
            resource_inventoried_as: e.resource_inventoried_as,
            resource_classified_as: parse_json_opt(&e.resource_classified_as_json),
            resource_quantity_value: e.resource_quantity_value,
            resource_quantity_unit: e.resource_quantity_unit,
            effort_quantity_value: e.effort_quantity_value,
            effort_quantity_unit: e.effort_quantity_unit,
            has_point_in_time: e.has_point_in_time,
            has_duration: e.has_duration,
            input_of: e.input_of,
            output_of: e.output_of,
            lamad_event_type: e.lamad_event_type,
            content_id: e.content_id,
            contributor_presence_id: e.contributor_presence_id,
            path_id: e.path_id,
            triggered_by: e.triggered_by,
            state: e.state,
            note: e.note,
            metadata: parse_json_opt(&e.metadata_json),
            dht_anchor_hash: e.dht_anchor_hash,
            created_at: e.created_at,
            at_location: e.at_location,
        }
    }
}

// ============================================================================
// REA Commitment Views
// ============================================================================

impl From<ReaCommitment> for ReaCommitmentView {
    fn from(c: ReaCommitment) -> Self {
        Self {
            id: c.id,
            action: c.action,
            provider: c.provider,
            receiver: c.receiver,
            resource_conforms_to: c.resource_conforms_to,
            resource_classified_as: c
                .resource_classified_as
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            resource_quantity: match (c.resource_quantity_value, &c.resource_quantity_unit) {
                (Some(v), Some(u)) => Some(MeasureView {
                    has_numerical_value: v,
                    has_unit: u.clone(),
                }),
                _ => None,
            },
            effort_quantity: match (c.effort_quantity_value, &c.effort_quantity_unit) {
                (Some(v), Some(u)) => Some(MeasureView {
                    has_numerical_value: v,
                    has_unit: u.clone(),
                }),
                _ => None,
            },
            has_beginning: c.has_beginning,
            has_end: c.has_end,
            due: c.due,
            clause_of: c.clause_of,
            in_scope_of: c
                .in_scope_of
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            medium_of_exchange_id: c.medium_of_exchange_id,
            state: c.state,
            finished: c.finished != 0,
            note: c.note,
            metadata: parse_json_opt(&c.metadata_json),
            dht_anchor_hash: c.dht_anchor_hash,
            created_at: c.created_at,
        }
    }
}

// ============================================================================
// Stewardship Allocation Views
// ============================================================================

impl From<StewardshipAllocation> for StewardshipAllocationView {
    fn from(a: StewardshipAllocation) -> Self {
        Self {
            id: a.id,
            h_app_id: a.h_app_id,
            content_id: a.content_id,
            steward_presence_id: a.steward_presence_id,
            allocation_ratio: a.allocation_ratio,
            allocation_method: a.allocation_method,
            contribution_type: a.contribution_type,
            contribution_evidence: parse_json_opt(&a.contribution_evidence_json),
            governance_state: a.governance_state,
            dispute_id: a.dispute_id,
            dispute_reason: a.dispute_reason,
            disputed_at: a.disputed_at,
            disputed_by: a.disputed_by,
            negotiation_session_id: a.negotiation_session_id,
            elohim_ratified_at: a.elohim_ratified_at,
            elohim_ratifier_id: a.elohim_ratifier_id,
            effective_from: a.effective_from,
            effective_until: a.effective_until,
            superseded_by: a.superseded_by,
            recognition_accumulated: a.recognition_accumulated,
            last_recognition_at: a.last_recognition_at,
            note: a.note,
            metadata: parse_json_opt(&a.metadata_json),
            created_at: a.created_at,
            updated_at: a.updated_at,
            dht_anchor_hash: a.dht_anchor_hash,
        }
    }
}

impl From<StewardshipAllocationWithPresence> for StewardshipAllocationWithPresenceView {
    fn from(a: StewardshipAllocationWithPresence) -> Self {
        Self {
            allocation: a.allocation.into(),
            steward: a.steward.map(|s| s.into()),
        }
    }
}

impl From<ContentStewardship> for ContentStewardshipView {
    fn from(s: ContentStewardship) -> Self {
        Self {
            content_id: s.content_id,
            allocations: s.allocations.into_iter().map(|a| a.into()).collect(),
            total_allocation: s.total_allocation,
            has_disputes: s.has_disputes,
            primary_steward: s.primary_steward.map(|a| a.into()),
        }
    }
}

// ============================================================================
// Token Views (Shefa economy — elohim-token sprint 1)
// ============================================================================

impl From<TokenMintEvent> for TokenMintEventView {
    fn from(m: TokenMintEvent) -> Self {
        Self {
            id: m.id,
            h_app_id: m.h_app_id,
            amount: m.amount,
            provenance_event_id: m.provenance_event_id,
            mint_tier: m.mint_tier,
            source_epr_id: m.source_epr_id,
            agent_id: m.agent_id,
            constitutional_context: m.constitutional_context,
            elohim_attestation: m.elohim_attestation,
            reasoning_trace: m.reasoning_trace,
            dht_anchor_hash: m.dht_anchor_hash,
            created_at: m.created_at,
        }
    }
}

impl From<TokenBalance> for TokenBalanceView {
    fn from(b: TokenBalance) -> Self {
        Self {
            agent_id: b.agent_id,
            h_app_id: b.h_app_id,
            governance_layer: b.governance_layer,
            balance: b.balance,
            total_minted: b.total_minted,
            total_transferred_in: b.total_transferred_in,
            total_transferred_out: b.total_transferred_out,
            last_activity_at: b.last_activity_at,
            created_at: b.created_at,
        }
    }
}

impl From<TokenTransfer> for TokenTransferView {
    fn from(t: TokenTransfer) -> Self {
        Self {
            id: t.id,
            h_app_id: t.h_app_id,
            from_agent: t.from_agent,
            to_agent: t.to_agent,
            amount: t.amount,
            governance_layer: t.governance_layer,
            note: t.note,
            dht_anchor_hash: t.dht_anchor_hash,
            created_at: t.created_at,
        }
    }
}

// ============================================================================
// Responsibility Demand Config Views (Shefa — elohim-token sprint 2)
// ============================================================================

impl From<ResponsibilityDemandConfig> for ResponsibilityDemandConfigView {
    fn from(c: ResponsibilityDemandConfig) -> Self {
        Self {
            id: c.id,
            governance_layer: c.governance_layer,
            dignity_floor: c.dignity_floor,
            median_estimate: c.median_estimate,
            soft_ceiling_multiplier: c.soft_ceiling_multiplier,
            hard_ceiling_multiplier: c.hard_ceiling_multiplier,
            social_contract_health: c.social_contract_health,
            enforcement_active: c.enforcement_active != 0,
            ratified_by: c.ratified_by,
            ratified_at: c.ratified_at,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

// ============================================================================
// Token Decay Event Views (Shefa — elohim-token sprint 3)
// ============================================================================

impl From<TokenDecayEvent> for TokenDecayEventView {
    fn from(d: TokenDecayEvent) -> Self {
        Self {
            id: d.id,
            agent_id: d.agent_id,
            governance_layer: d.governance_layer,
            balance_before: d.balance_before,
            balance_after: d.balance_after,
            decay_amount: d.decay_amount,
            obligation_level: d.obligation_level,
            dignity_floor: d.dignity_floor,
            created_at: d.created_at,
        }
    }
}

// ============================================================================
// Steward Views
// ============================================================================

impl From<StewardCredential> for StewardCredentialView {
    fn from(s: StewardCredential) -> Self {
        Self {
            id: s.id,
            presence_id: s.presence_id,
            content_id: s.content_id,
            affinity_coefficient: s.affinity_coefficient,
            credential_type: s.credential_type,
            status: s.status,
            dht_anchor_hash: s.dht_anchor_hash,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

impl From<PremiumGate> for PremiumGateView {
    fn from(g: PremiumGate) -> Self {
        Self {
            id: g.id,
            steward_credential_id: g.steward_credential_id,
            steward_presence_id: g.steward_presence_id,
            gated_resource_type: g.gated_resource_type,
            gated_resource_ids: parse_json(&g.gated_resource_ids),
            gate_title: g.gate_title,
            gate_description: g.gate_description,
            dht_anchor_hash: g.dht_anchor_hash,
            created_at: g.created_at,
        }
    }
}

impl From<AccessGrant> for AccessGrantView {
    fn from(a: AccessGrant) -> Self {
        Self {
            id: a.id,
            gate_id: a.gate_id,
            grantee_presence_id: a.grantee_presence_id,
            contributor_presence_id: a.contributor_presence_id,
            granted_at: a.granted_at,
            expires_at: a.expires_at,
            status: a.status,
            dht_anchor_hash: a.dht_anchor_hash,
        }
    }
}

impl From<RevenueSummary> for StewardRevenueSummaryView {
    fn from(r: RevenueSummary) -> Self {
        Self {
            total_credentials: r.total_credentials,
            total_gates: r.total_gates,
            total_grants: r.total_grants,
        }
    }
}

// ============================================================================
// Contributor Views
// ============================================================================

impl From<ContributorDashboard> for ContributorDashboardView {
    fn from(d: ContributorDashboard) -> Self {
        Self {
            presence_id: d.presence_id,
            total_contributions: d.total_contributions,
            total_recognitions: d.total_recognitions,
            impact_score: d.impact_score,
            last_contribution_at: d.last_contribution_at,
            updated_at: d.updated_at,
        }
    }
}

impl From<ImpactSummary> for ContributorImpactView {
    fn from(i: ImpactSummary) -> Self {
        Self {
            presence_id: i.presence_id,
            total_events: i.total_events,
            unique_content_ids: i.unique_content_ids,
        }
    }
}

// ============================================================================
// Agreement Views
// ============================================================================

impl From<AgreementRow> for AgreementView {
    fn from(a: AgreementRow) -> Self {
        Self {
            id: a.id,
            name: a.name,
            note: a.note,
            dht_anchor_hash: a.dht_anchor_hash,
            metadata: parse_json_opt(&a.metadata_json),
            created_at: a.created_at,
        }
    }
}

// ============================================================================
// Stewarded Node Views
// ============================================================================

impl From<StewardedNode> for StewardedNodeView {
    fn from(n: StewardedNode) -> Self {
        Self {
            id: n.id,
            display_name: n.display_name,
            claim_status: n.claim_status,
            cpu_cores: n.cpu_cores,
            memory_gb: n.memory_gb,
            storage_tb: n.storage_tb,
            bandwidth_mbps: n.bandwidth_mbps,
            steward_tier: n.steward_tier,
            custodian_opt_in: n.custodian_opt_in == 1,
            region: n.region,
            context_epr_id: n.context_epr_id,
            dht_anchor_hash: n.dht_anchor_hash,
            created_at: n.created_at,
            updated_at: n.updated_at,
            stewards: vec![],
        }
    }
}

// ============================================================================
// Steward Affinity Views
// ============================================================================

impl From<StewardAffinity> for StewardAffinityView {
    fn from(a: StewardAffinity) -> Self {
        Self {
            id: a.id,
            steward_id: a.steward_id,
            content_id: a.content_id,
            affinity_score: a.affinity_score,
            source: a.source,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

// ============================================================================
// Free functions
// ============================================================================

/// Build a NodeStewardshipView from DB model + joined display name.
pub fn node_stewardship_view_from_with_name(
    s: NodeStewardship,
    display_name: String,
) -> NodeStewardshipView {
    NodeStewardshipView {
        human_id: s.human_id,
        display_name,
        affinity_score: s.affinity_score,
        relationship: s.relationship,
        context_epr_id: s.context_epr_id,
        granted_at: s.granted_at,
    }
}
