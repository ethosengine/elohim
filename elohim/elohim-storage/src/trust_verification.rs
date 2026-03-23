//! Three-pillar trust verification via conductor.
//!
//! Calls verify_credentials() zome functions in imagodei and mishpat DNAs
//! to verify credential CIDs against the DHT. Storage doesn't care which
//! DNA answered — it gets a unified result across all three pillars.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::debug;

use crate::error::StorageError;
use crate::hc_client::HcClient;

// =============================================================================
// Domain Types
// =============================================================================

/// Credentials presented by a peer during trust negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustCredentials {
    pub agent_pubkey: String,
    /// Qahal: collective membership CIDs
    pub membership_cids: Vec<String>,
    /// Qahal: interpersonal relationship CIDs
    pub relationship_cids: Vec<String>,
    /// Any pillar: mastery, consent, payment attestation CIDs
    pub attestation_cids: Vec<String>,
    /// Shefa: stewardship commitment CIDs
    pub stewardship_cids: Vec<String>,
}

/// Verified trust context — the result of DHT verification.
#[derive(Debug, Clone)]
pub struct VerifiedTrustContext {
    pub agent_pubkey: String,
    pub agent_verified: bool,
    /// Highest reach tier this peer qualifies for (ambient ceiling)
    pub reach_ceiling: String,
    pub verified_memberships: Vec<VerifiedMembership>,
    pub verified_relationships: Vec<VerifiedRelationship>,
    pub verified_attestations: Vec<VerifiedAttestation>,
    pub verified_stewardship: Vec<VerifiedStewardship>,
    pub verified_at: Instant,
    pub ttl: Duration,
}

#[derive(Debug, Clone)]
pub struct VerifiedMembership {
    pub cid: String,
    pub collective_id: String,
    pub consent_state: String,
}

#[derive(Debug, Clone)]
pub struct VerifiedRelationship {
    pub cid: String,
    pub party_a_id: String,
    pub party_b_id: String,
    pub intimacy_level: String,
    pub consent_given_by_a: bool,
    pub consent_given_by_b: bool,
}

#[derive(Debug, Clone)]
pub struct VerifiedAttestation {
    pub cid: String,
    pub attestation_type: String,
    pub reference: Option<String>,
    pub is_revoked: bool,
}

#[derive(Debug, Clone)]
pub struct VerifiedStewardship {
    pub cid: String,
    pub content_id: String,
    pub allocation_ratio: f64,
}

/// Default TTL for verified trust context (1 hour)
const DEFAULT_TTL_SECS: u64 = 3600;

// =============================================================================
// Verification via Conductor
// =============================================================================

/// Verify a peer's trust credentials against the DHT via conductor zome calls.
///
/// Routes CIDs to the appropriate DNA (imagodei for relationships/attestations,
/// mishpat for collective memberships) and returns a unified context.
///
/// If the conductor is unavailable, returns Err — callers should fall back
/// to per-request SQLite checks.
pub async fn verify_trust_context(
    hc_client: &HcClient,
    credentials: &TrustCredentials,
) -> Result<VerifiedTrustContext, StorageError> {
    debug!(
        agent = %credentials.agent_pubkey,
        memberships = credentials.membership_cids.len(),
        relationships = credentials.relationship_cids.len(),
        attestations = credentials.attestation_cids.len(),
        stewardship = credentials.stewardship_cids.len(),
        "Verifying trust credentials via conductor"
    );

    // Verify memberships via mishpat DNA
    let verified_memberships =
        verify_membership_cids(hc_client, &credentials.membership_cids).await?;

    // Verify relationships + attestations via imagodei DNA
    let verified_relationships =
        verify_relationship_cids(hc_client, &credentials.relationship_cids).await?;
    let verified_attestations =
        verify_attestation_cids(hc_client, &credentials.attestation_cids).await?;

    // Verify stewardship via imagodei DNA (stewardship coordinator)
    let verified_stewardship =
        verify_stewardship_cids(hc_client, &credentials.stewardship_cids).await?;

    // Calculate ambient reach ceiling from verified credentials
    let reach_ceiling =
        calculate_reach_ceiling(&verified_memberships, &verified_relationships);

    Ok(VerifiedTrustContext {
        agent_pubkey: credentials.agent_pubkey.clone(),
        agent_verified: true,
        reach_ceiling,
        verified_memberships,
        verified_relationships,
        verified_attestations,
        verified_stewardship,
        verified_at: Instant::now(),
        ttl: Duration::from_secs(DEFAULT_TTL_SECS),
    })
}

/// Calculate the highest ambient reach ceiling from verified credentials.
///
/// Note: familiar/trusted/intimate are content-specific (depend on which
/// stewards are allocated). The ceiling here is the POTENTIAL maximum —
/// per-request checks still validate against specific content's stewards.
fn calculate_reach_ceiling(
    memberships: &[VerifiedMembership],
    relationships: &[VerifiedRelationship],
) -> String {
    // Check for intimate: mutual intimate relationship with both consents
    if relationships.iter().any(|r| {
        r.intimacy_level == "intimate" && r.consent_given_by_a && r.consent_given_by_b
    }) {
        return "intimate".to_string();
    }

    // Check for trusted: any relationship at intimacy >= trusted
    let trusted_idx =
        crate::db::models::intimacy_levels::index_of("trusted").unwrap_or(2);
    if relationships.iter().any(|r| {
        crate::db::models::intimacy_levels::index_of(&r.intimacy_level)
            .map(|idx| idx >= trusted_idx)
            .unwrap_or(false)
    }) {
        return "trusted".to_string();
    }

    // Check for community: any consented membership
    if memberships
        .iter()
        .any(|m| m.consent_state == "consented")
    {
        return "community".to_string();
    }

    // Default: public (anyone can see commons/public content)
    "public".to_string()
}

// =============================================================================
// Per-DNA Verification Helpers (stubs — pending conductor integration)
// =============================================================================

async fn verify_membership_cids(
    _hc_client: &HcClient,
    cids: &[String],
) -> Result<Vec<VerifiedMembership>, StorageError> {
    if cids.is_empty() {
        return Ok(Vec::new());
    }
    debug!(
        count = cids.len(),
        "Membership CID verification: stub (pending conductor integration)"
    );
    Ok(Vec::new())
}

async fn verify_relationship_cids(
    _hc_client: &HcClient,
    cids: &[String],
) -> Result<Vec<VerifiedRelationship>, StorageError> {
    if cids.is_empty() {
        return Ok(Vec::new());
    }
    debug!(
        count = cids.len(),
        "Relationship CID verification: stub (pending conductor integration)"
    );
    Ok(Vec::new())
}

async fn verify_attestation_cids(
    _hc_client: &HcClient,
    cids: &[String],
) -> Result<Vec<VerifiedAttestation>, StorageError> {
    if cids.is_empty() {
        return Ok(Vec::new());
    }
    debug!(
        count = cids.len(),
        "Attestation CID verification: stub (pending conductor integration)"
    );
    Ok(Vec::new())
}

async fn verify_stewardship_cids(
    _hc_client: &HcClient,
    cids: &[String],
) -> Result<Vec<VerifiedStewardship>, StorageError> {
    if cids.is_empty() {
        return Ok(Vec::new());
    }
    debug!(
        count = cids.len(),
        "Stewardship CID verification: stub (pending conductor integration)"
    );
    Ok(Vec::new())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reach_ceiling_intimate_with_mutual_consent() {
        let memberships = vec![];
        let relationships = vec![VerifiedRelationship {
            cid: "bafkrei-test".to_string(),
            party_a_id: "human-a".to_string(),
            party_b_id: "human-b".to_string(),
            intimacy_level: "intimate".to_string(),
            consent_given_by_a: true,
            consent_given_by_b: true,
        }];
        assert_eq!(
            calculate_reach_ceiling(&memberships, &relationships),
            "intimate"
        );
    }

    #[test]
    fn reach_ceiling_trusted_relationship() {
        let memberships = vec![];
        let relationships = vec![VerifiedRelationship {
            cid: "bafkrei-test".to_string(),
            party_a_id: "human-a".to_string(),
            party_b_id: "human-b".to_string(),
            intimacy_level: "trusted".to_string(),
            consent_given_by_a: true,
            consent_given_by_b: false,
        }];
        assert_eq!(
            calculate_reach_ceiling(&memberships, &relationships),
            "trusted"
        );
    }

    #[test]
    fn reach_ceiling_community_membership() {
        let memberships = vec![VerifiedMembership {
            cid: "bafkrei-test".to_string(),
            collective_id: "church-001".to_string(),
            consent_state: "consented".to_string(),
        }];
        let relationships = vec![];
        assert_eq!(
            calculate_reach_ceiling(&memberships, &relationships),
            "community"
        );
    }

    #[test]
    fn reach_ceiling_defaults_to_public() {
        assert_eq!(calculate_reach_ceiling(&[], &[]), "public");
    }

    #[test]
    fn reach_ceiling_intimate_without_consent_falls_to_trusted() {
        let relationships = vec![VerifiedRelationship {
            cid: "bafkrei-test".to_string(),
            party_a_id: "human-a".to_string(),
            party_b_id: "human-b".to_string(),
            intimacy_level: "intimate".to_string(),
            consent_given_by_a: true,
            consent_given_by_b: false, // no mutual consent
        }];
        // intimate without mutual consent -> still >= trusted index
        assert_eq!(calculate_reach_ceiling(&[], &relationships), "trusted");
    }
}
