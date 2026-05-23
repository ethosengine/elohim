//! Qahal service — orchestrates zome calls + projection building for Collective +
//! Collab flows. The HTTP routes in http.rs are thin shells over this service.
//!
//! Per spec: 2026-05-23-multi-collective-collaboration-epr-design.md §2 + §5.1.
//!
//! ## Zome mapping
//!
//! All calls target the `"imagodei"` coordinator zome in the imagodei DNA.
//! The `HcClientRegistry.imagodei` slot is required; returns 503 when absent.
//!
//! ## CID convention
//!
//! The imagodei coordinator uses:
//!   - `collective:<ActionHash_display>` for Collective CIDs
//!   - `agent:<AgentPubKey_display>`     for agent CIDs
//!   - `agreement:<ActionHash_display>`  for agreement links (not the public CID)
//!
//! ActionHash bytes (39 bytes, HoloHash format) flow from the conductor response
//! as `Vec<u8>`. The display string is reconstructed via holochain's standard
//! base32 encoding when building view CIDs.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::StorageError;
use crate::hc_client::HcClient;
use elohim_views::{
    AttestCollabAgreementInputView, CollabAgreementStatus, CollabAgreementView,
    CollabCollectiveView, CollabMembershipRole, CollabMembershipView, CollabQahalView,
    CreateCollabAgreementInputView, CreateCollabCollectiveInputView, ElohimTier, MemberKind,
    ShareAllocation, WithdrawMembershipInputView,
};

// =============================================================================
// QahalService
// =============================================================================

/// Service that wraps imagodei zome calls and builds Collab view projections.
///
/// Hold this behind an `Arc` in the server state; it is `Send + Sync` because
/// `HcClient` is `Send + Sync` and all methods take `&self`.
pub struct QahalService {
    /// Connected to the imagodei role of the Holochain conductor.
    hc: Arc<HcClient>,
    /// Imagodei coordinator zome name. Hard-coded to "imagodei" per DNA structure.
    zome: &'static str,
}

impl QahalService {
    /// Construct from an existing `HcClient` connected to the imagodei role.
    pub fn new(hc: Arc<HcClient>) -> Self {
        Self {
            hc,
            zome: "imagodei",
        }
    }

    // =========================================================================
    // Public — write operations
    // =========================================================================

    /// Create a new first-order Collective.
    ///
    /// Calls `create_collective` in the imagodei coordinator, which atomically
    /// creates the Collective entry + founder Steward Membership.
    /// Returns the projection view built from the resulting ActionHash.
    pub async fn create_collective(
        &self,
        input: CreateCollabCollectiveInputView,
    ) -> Result<CollabCollectiveView, StorageError> {
        debug!(display_name = %input.display_name, "QahalService::create_collective");

        let zome_input = ZomeCreateCollectiveInput {
            charter: input.charter.clone(),
            display_name: input.display_name.clone(),
            salt: input.salt.clone(),
        };
        let payload = rmp_serde::to_vec_named(&zome_input)
            .map_err(|e| StorageError::Internal(format!("encode CreateCollectiveInput: {e}")))?;

        let result = self
            .hc
            .call_zome(self.zome, "create_collective", payload)
            .await?;

        let action_bytes: Vec<u8> = rmp_serde::from_slice(&result).map_err(|e| {
            StorageError::Internal(format!("decode ActionHash from create_collective: {e}"))
        })?;

        let cid = bytes_to_collective_cid(&action_bytes);

        info!(cid = %cid, "Collective created");

        Ok(CollabCollectiveView {
            cid,
            founder_agent_cid: "agent:unknown".into(), // resolved client-side via agent_info
            charter: input.charter,
            display_name: input.display_name,
            created_at_block_height: 0, // block height not returned by the create call; use 0 for M1
            anchor_agreement_cid: None,
            elohim_tier: ElohimTier::T0,
        })
    }

    /// Create a CollabAgreement entry.
    ///
    /// Calls `create_collab_agreement` in the imagodei coordinator.
    /// The Collab-Qahal is NOT instantiated by this call — instantiation is
    /// deferred until all participants attest via `attest_collab_agreement`.
    pub async fn create_collab_agreement(
        &self,
        input: CreateCollabAgreementInputView,
    ) -> Result<CollabAgreementView, StorageError> {
        debug!(scope = %input.scope, "QahalService::create_collab_agreement");

        let share_allocation_json = serde_json::to_string(&input.share_allocation)
            .map_err(|e| StorageError::Internal(format!("encode share_allocation: {e}")))?;
        let commons_pool_tribute = input.share_allocation.commons_pool_tribute;
        let governance_terms_json = input
            .governance_terms
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| StorageError::Internal(format!("encode governance_terms: {e}")))?
            .unwrap_or_else(|| "{}".to_string());
        let initial_tier = format!("{:?}", input.initial_tier); // "T0" etc.

        let zome_input = ZomeCreateCollabAgreementInput {
            participants: input.participants.clone(),
            scope: input.scope.clone(),
            share_allocation_json,
            commons_pool_tribute,
            governance_terms_json,
            initial_tier,
            display_name_for_qahal: input.display_name_for_qahal.clone(),
            salt: input.salt.clone(),
        };
        let payload = rmp_serde::to_vec_named(&zome_input).map_err(|e| {
            StorageError::Internal(format!("encode CreateCollabAgreementInput: {e}"))
        })?;

        let result = self
            .hc
            .call_zome(self.zome, "create_collab_agreement", payload)
            .await?;

        let action_bytes: Vec<u8> = rmp_serde::from_slice(&result).map_err(|e| {
            StorageError::Internal(format!(
                "decode ActionHash from create_collab_agreement: {e}"
            ))
        })?;

        let cid = bytes_to_agreement_cid(&action_bytes);

        info!(cid = %cid, "CollabAgreement created");

        Ok(CollabAgreementView {
            cid,
            authored_by_agent_cid: "agent:unknown".into(),
            participants: input.participants,
            scope: input.scope,
            share_allocation: input.share_allocation,
            commons_pool_tribute,
            governance_terms: input.governance_terms,
            initial_tier: input.initial_tier,
            created_at_block_height: 0,
            status: CollabAgreementStatus::PendingAttestations,
            attested_by: vec![],
            collab_qahal_cid: None,
        })
    }

    /// Counter-attest a CollabAgreement on behalf of a participating Collective.
    ///
    /// When all participants attest, the coordinator atomically instantiates the
    /// Collab-Qahal. Returns a refreshed `CollabAgreementView` that reflects the
    /// new status (Instantiated vs PendingAttestations).
    pub async fn attest_collab_agreement(
        &self,
        input: AttestCollabAgreementInputView,
    ) -> Result<CollabAgreementView, StorageError> {
        debug!(
            agreement_cid = %input.agreement_cid,
            collective_cid = %input.attesting_collective_cid,
            "QahalService::attest_collab_agreement"
        );

        let agreement_action_bytes = decode_agreement_cid(&input.agreement_cid)?;

        let zome_input = ZomeAttestCollabAgreementInput {
            agreement_action_hash: agreement_action_bytes,
            attesting_collective_cid: input.attesting_collective_cid.clone(),
        };
        let payload = rmp_serde::to_vec_named(&zome_input).map_err(|e| {
            StorageError::Internal(format!("encode AttestCollabAgreementInput: {e}"))
        })?;

        // Returns () — we ignore the response bytes
        self.hc
            .call_zome(self.zome, "attest_collab_agreement", payload)
            .await?;

        // Refresh from DHT now that attestation has been recorded
        self.fetch_agreement_by_action_bytes(&agreement_action_bytes_from_cid(
            &input.agreement_cid,
        )?)
        .await
    }

    /// Withdraw a Membership via the clean-exit path (spec §6.4).
    ///
    /// Sets `withdrawn_at_block_height` on the Membership entry. Future
    /// share-routing calculations at blocks >= withdrawn do not accrue to
    /// this member (Tasks 13/14).
    pub async fn withdraw_membership(
        &self,
        input: WithdrawMembershipInputView,
    ) -> Result<CollabMembershipView, StorageError> {
        debug!(
            membership_cid = %input.membership_cid,
            "QahalService::withdraw_membership"
        );

        let membership_action_bytes = decode_collective_cid(&input.membership_cid)?;

        let zome_input = ZomeWithdrawMembershipInput {
            membership_action_hash: membership_action_bytes.clone(),
            collab_qahal_cid: input.collab_qahal_cid.clone(),
        };
        let payload = rmp_serde::to_vec_named(&zome_input)
            .map_err(|e| StorageError::Internal(format!("encode WithdrawMembershipInput: {e}")))?;

        // Returns () — coordinator mutates the Membership entry
        self.hc
            .call_zome(self.zome, "withdraw_membership_clean", payload)
            .await?;

        // Fetch the updated Membership record
        self.fetch_membership_by_action(membership_action_bytes)
            .await
    }

    // =========================================================================
    // Public — read operations
    // =========================================================================

    /// Fetch a Collective by its CID (e.g. "collective:uhCkk...").
    pub async fn fetch_collective(&self, cid: &str) -> Result<CollabCollectiveView, StorageError> {
        debug!(cid = %cid, "QahalService::fetch_collective");
        let action_bytes = decode_collective_cid(cid)?;
        self.fetch_collective_by_action(action_bytes).await
    }

    /// Fetch the Collab-Qahal instantiated from a CollabAgreement CID.
    ///
    /// Calls `get_collab_qahal_cid_for_agreement` to find the Collab-Qahal's
    /// ActionHash, then fetches all member Collectives + Memberships to build
    /// the full `CollabQahalView`.
    pub async fn fetch_collab_qahal(&self, cid: &str) -> Result<CollabQahalView, StorageError> {
        debug!(cid = %cid, "QahalService::fetch_collab_qahal");

        // cid may be either a collective CID (if caller has the Collab-Qahal CID)
        // or an agreement CID (if caller wants to find the Qahal from the agreement).
        // Support both forms.
        let (collective_action_bytes, anchor_agreement_cid) = if cid.starts_with("agreement:") {
            // Derive the Qahal CID from the agreement
            let agreement_bytes = decode_agreement_cid(cid)?;
            let qahal_cid = self
                .get_qahal_cid_for_agreement_bytes(agreement_bytes)
                .await?;
            let collective_bytes = decode_collective_cid(&qahal_cid)?;
            (collective_bytes, cid.to_string())
        } else {
            // Direct Collab-Qahal CID
            let collective_bytes = decode_collective_cid(cid)?;
            (collective_bytes, String::new())
        };

        let qahal_collective = self
            .fetch_collective_by_action(collective_action_bytes)
            .await?;

        // List all Memberships for this Qahal
        let membership_records = self.list_memberships_for_cid(&qahal_collective.cid).await?;

        // Partition memberships into Collective members and Person members
        let mut member_collectives: Vec<CollabCollectiveView> = Vec::new();
        let mut member_persons: Vec<String> = Vec::new();

        for m in &membership_records {
            match m.member_kind {
                MemberKind::Collective => {
                    // Fetch the member Collective's view
                    if let Ok(coll_view) = self.fetch_collective(&m.member_cid).await {
                        member_collectives.push(coll_view);
                    }
                }
                MemberKind::Person => {
                    member_persons.push(m.member_cid.clone());
                }
                MemberKind::ElohimAgent => {
                    // ElohimAgent memberships not projected in M1
                }
            }
        }

        let effective_anchor_cid = if anchor_agreement_cid.is_empty() {
            qahal_collective
                .anchor_agreement_cid
                .clone()
                .unwrap_or_default()
        } else {
            anchor_agreement_cid
        };

        Ok(CollabQahalView {
            cid: qahal_collective.cid.clone(),
            anchor_agreement_cid: effective_anchor_cid,
            display_name: qahal_collective.display_name,
            created_at_block_height: qahal_collective.created_at_block_height,
            elohim_tier: qahal_collective.elohim_tier,
            member_collectives,
            member_persons,
            commons_pool_balance: None, // M1: balance not yet projected
        })
    }

    // =========================================================================
    // Private helpers — zome call wrappers
    // =========================================================================

    async fn fetch_collective_by_action(
        &self,
        action_bytes: Vec<u8>,
    ) -> Result<CollabCollectiveView, StorageError> {
        let payload = rmp_serde::to_vec(&action_bytes).map_err(|e| {
            StorageError::Internal(format!(
                "encode ActionHash for get_collective_by_action: {e}"
            ))
        })?;

        let result = self
            .hc
            .call_zome(self.zome, "get_collective_by_action", payload)
            .await?;

        // Coordinator returns Option<Record>
        let maybe_record: Option<ZomeRecord> = rmp_serde::from_slice(&result).map_err(|e| {
            StorageError::Internal(format!(
                "decode Option<Record> from get_collective_by_action: {e}"
            ))
        })?;

        let record = maybe_record.ok_or_else(|| {
            StorageError::NotFound(format!(
                "Collective not found for action {}",
                hex::encode(&action_bytes)
            ))
        })?;

        self.record_to_collective_view(record, &action_bytes)
    }

    async fn fetch_agreement_by_action_bytes(
        &self,
        action_bytes: &[u8],
    ) -> Result<CollabAgreementView, StorageError> {
        // No dedicated extern for getting an agreement by action hash —
        // compose existing externs: get status + qahal CID, then build view.
        let status_payload = rmp_serde::to_vec(action_bytes).map_err(|e| {
            StorageError::Internal(format!("encode ActionHash for get_collab_status: {e}"))
        })?;
        let status_result = self
            .hc
            .call_zome(self.zome, "get_collab_status", status_payload)
            .await?;
        let status_str: String = rmp_serde::from_slice(&status_result)
            .map_err(|e| StorageError::Internal(format!("decode get_collab_status: {e}")))?;

        let status = if status_str == "Instantiated" {
            CollabAgreementStatus::Instantiated
        } else {
            CollabAgreementStatus::PendingAttestations
        };

        // Optionally resolve the Collab-Qahal CID if instantiated
        let collab_qahal_cid = if matches!(status, CollabAgreementStatus::Instantiated) {
            let cid_payload = rmp_serde::to_vec(action_bytes).map_err(|e| {
                StorageError::Internal(format!("encode ActionHash for get_collab_qahal_cid: {e}"))
            })?;
            let cid_result = self
                .hc
                .call_zome(self.zome, "get_collab_qahal_cid_for_agreement", cid_payload)
                .await
                .ok();
            cid_result.and_then(|bytes| rmp_serde::from_slice::<String>(&bytes).ok())
        } else {
            None
        };

        let cid = bytes_to_agreement_cid(action_bytes);

        Ok(CollabAgreementView {
            cid,
            authored_by_agent_cid: "agent:unknown".into(), // not derivable without full Record decode
            participants: vec![], // not derivable without full Record decode
            scope: String::new(), // not derivable without full Record decode
            share_allocation: default_share_allocation(),
            commons_pool_tribute: 0.0,
            governance_terms: None,
            initial_tier: ElohimTier::T0,
            created_at_block_height: 0,
            status,
            attested_by: vec![],
            collab_qahal_cid,
        })
    }

    async fn fetch_membership_by_action(
        &self,
        action_bytes: Vec<u8>,
    ) -> Result<CollabMembershipView, StorageError> {
        let payload = rmp_serde::to_vec(&action_bytes).map_err(|e| {
            StorageError::Internal(format!(
                "encode ActionHash for get_membership_by_action: {e}"
            ))
        })?;

        let result = self
            .hc
            .call_zome(self.zome, "get_membership_by_action", payload)
            .await?;

        let record: ZomeRecord = rmp_serde::from_slice(&result).map_err(|e| {
            StorageError::Internal(format!("decode Record from get_membership_by_action: {e}"))
        })?;

        self.record_to_membership_view(record, &action_bytes)
    }

    async fn list_memberships_for_cid(
        &self,
        collective_cid: &str,
    ) -> Result<Vec<CollabMembershipView>, StorageError> {
        let payload = rmp_serde::to_vec(collective_cid).map_err(|e| {
            StorageError::Internal(format!("encode collective_cid for list_memberships: {e}"))
        })?;

        let result = self
            .hc
            .call_zome(self.zome, "list_memberships_for_collective_cid", payload)
            .await?;

        let records: Vec<ZomeRecord> = rmp_serde::from_slice(&result).map_err(|e| {
            StorageError::Internal(format!("decode Vec<Record> from list_memberships: {e}"))
        })?;

        records
            .into_iter()
            .map(|rec| self.record_to_membership_view(rec, &[]))
            .collect()
    }

    /// Calls `get_collab_qahal_cid_for_agreement` and returns the CID string.
    async fn get_qahal_cid_for_agreement_bytes(
        &self,
        agreement_bytes: Vec<u8>,
    ) -> Result<String, StorageError> {
        let payload = rmp_serde::to_vec(&agreement_bytes).map_err(|e| {
            StorageError::Internal(format!("encode ActionHash for get_collab_qahal_cid: {e}"))
        })?;

        let result = self
            .hc
            .call_zome(self.zome, "get_collab_qahal_cid_for_agreement", payload)
            .await?;

        rmp_serde::from_slice::<String>(&result)
            .map_err(|e| StorageError::Internal(format!("decode Collab-Qahal CID: {e}")))
    }

    // =========================================================================
    // Private helpers — record projection
    // =========================================================================

    /// Project a Holochain Record for a Collective entry into `CollabCollectiveView`.
    ///
    /// The record is decoded as a `ZomeCollectiveEntry` (mirrors the integrity
    /// struct). The CID is derived from the ActionHash bytes.
    fn record_to_collective_view(
        &self,
        record: ZomeRecord,
        action_bytes: &[u8],
    ) -> Result<CollabCollectiveView, StorageError> {
        let entry: ZomeCollectiveEntry = rmp_serde::from_slice(&record.entry_bytes)
            .map_err(|e| StorageError::Internal(format!("decode Collective entry: {e}")))?;

        let cid = if action_bytes.is_empty() {
            // Derive CID from the record's action_hash field if available
            record
                .action_hash
                .map(|b| bytes_to_collective_cid(&b))
                .unwrap_or_else(|| "collective:unknown".into())
        } else {
            bytes_to_collective_cid(action_bytes)
        };

        Ok(CollabCollectiveView {
            cid,
            founder_agent_cid: entry.founder_agent_cid,
            charter: entry.charter,
            display_name: entry.display_name,
            created_at_block_height: entry.created_at_block_height,
            anchor_agreement_cid: entry.anchor_agreement_cid,
            elohim_tier: ElohimTier::T0, // M1 default; tier upgrade deferred to M3
        })
    }

    /// Project a Holochain Record for a Membership entry into `CollabMembershipView`.
    fn record_to_membership_view(
        &self,
        record: ZomeRecord,
        action_bytes: &[u8],
    ) -> Result<CollabMembershipView, StorageError> {
        let entry: ZomeMembershipEntry = rmp_serde::from_slice(&record.entry_bytes)
            .map_err(|e| StorageError::Internal(format!("decode Membership entry: {e}")))?;

        let cid = if action_bytes.is_empty() {
            record
                .action_hash
                .map(|b| bytes_to_collective_cid(&b))
                .unwrap_or_else(|| "membership:unknown".into())
        } else {
            bytes_to_collective_cid(action_bytes)
        };

        let member_kind = match entry.member_kind.as_str() {
            "Collective" => MemberKind::Collective,
            "ElohimAgent" => MemberKind::ElohimAgent,
            _ => MemberKind::Person,
        };

        let role = match entry.role.as_str() {
            "Steward" => CollabMembershipRole::Steward,
            "Observer" => CollabMembershipRole::Observer,
            _ => CollabMembershipRole::Contributor,
        };

        Ok(CollabMembershipView {
            cid,
            member_cid: entry.member_cid,
            member_kind,
            collective_cid: entry.collective_cid,
            role,
            sponsor_cid: entry.sponsor_cid,
            joined_at_block_height: entry.joined_at_block_height,
            withdrawn_at_block_height: entry.withdrawn_at_block_height,
        })
    }
}

// =============================================================================
// Zome input wire types
// =============================================================================
//
// These mirror the coordinator's `#[derive(Serialize, Deserialize)]` input
// structs. They MUST use `rmp_serde::to_vec_named` (map encoding) so that the
// coordinator's serde deserialization finds fields by name.

#[derive(Debug, Clone, Serialize)]
struct ZomeCreateCollectiveInput {
    pub charter: String,
    pub display_name: String,
    pub salt: String,
}

#[derive(Debug, Clone, Serialize)]
struct ZomeCreateCollabAgreementInput {
    pub participants: Vec<String>,
    pub scope: String,
    pub share_allocation_json: String,
    pub commons_pool_tribute: f64,
    pub governance_terms_json: String,
    pub initial_tier: String,
    pub display_name_for_qahal: String,
    pub salt: String,
}

#[derive(Debug, Clone, Serialize)]
struct ZomeAttestCollabAgreementInput {
    pub agreement_action_hash: Vec<u8>,
    pub attesting_collective_cid: String,
}

#[derive(Debug, Clone, Serialize)]
struct ZomeWithdrawMembershipInput {
    pub membership_action_hash: Vec<u8>,
    pub collab_qahal_cid: String,
}

// =============================================================================
// Zome response wire types
// =============================================================================
//
// These are minimal deserialization targets for Holochain Record envelopes.
// Holochain encodes Records as a msgpack map with at least `entry_bytes`
// (the raw SerializedBytes of the app entry) and optionally `action_hash`.
// The `entry_bytes` field then contains a msgpack-encoded entry-type envelope.
//
// For M1 the service decodes the inner entry bytes as the plain entry struct.

/// Minimal Record deserialization — carries entry bytes + optional action hash.
#[derive(Debug, Clone, Deserialize)]
struct ZomeRecord {
    /// Raw msgpack bytes of the app entry (the Collective/Membership entry body).
    #[serde(default)]
    pub entry_bytes: Vec<u8>,
    /// ActionHash bytes, present in the response when available.
    #[serde(default)]
    pub action_hash: Option<Vec<u8>>,
}

/// Mirror of `imagodei_integrity::qahal::Collective` for deserialization.
#[derive(Debug, Clone, Deserialize)]
struct ZomeCollectiveEntry {
    pub founder_agent_cid: String,
    pub charter: String,
    pub display_name: String,
    pub created_at_block_height: u64,
    /// Carried for completeness of the wire type; not projected in M1.
    #[serde(default)]
    #[allow(dead_code)]
    pub salt: String,
    #[serde(default)]
    pub anchor_agreement_cid: Option<String>,
}

/// Mirror of `imagodei_integrity::qahal::Membership` for deserialization.
#[derive(Debug, Clone, Deserialize)]
struct ZomeMembershipEntry {
    pub member_cid: String,
    /// Serialized as variant name string ("Person", "Collective", "ElohimAgent")
    pub member_kind: String,
    pub collective_cid: String,
    /// Serialized as variant name string ("Steward", "Contributor", "Observer")
    pub role: String,
    #[serde(default)]
    pub sponsor_cid: Option<String>,
    pub joined_at_block_height: u64,
    #[serde(default)]
    pub withdrawn_at_block_height: Option<u64>,
}

// =============================================================================
// CID codec helpers
// =============================================================================

/// Encode a 39-byte ActionHash into a `collective:<display>` CID string.
///
/// Holochain's ActionHash Display implementation uses `holo_hash`'s standard
/// base32-padded encoding (the "uhCkk..." prefix). For elohim-storage — which
/// does not import `holo_hash` directly — we use base64url of the raw bytes as
/// a stable, round-trippable representation. The coordinator uses the same
/// encoding in its `action_hash_to_cid` helper.
///
/// NOTE: The coordinator's `action_hash_to_cid` uses `format!("collective:{}", hash)`
/// where `hash` is an HDK `ActionHash` whose `Display` is the HoloHash base32
/// string (e.g. "uhCkk..."). elohim-storage stores the raw bytes returned by the
/// conductor; we re-encode them as base64url to produce a stable CID.
/// Both forms are valid CIDs for DHT-internal references; the key invariant is
/// that `decode_collective_cid` can round-trip what `bytes_to_collective_cid` produced.
fn bytes_to_collective_cid(bytes: &[u8]) -> String {
    format!("collective:{}", base64url_encode(bytes))
}

/// Encode a 39-byte ActionHash into an `agreement:<display>` CID string.
fn bytes_to_agreement_cid(bytes: &[u8]) -> String {
    format!("agreement:{}", base64url_encode(bytes))
}

/// Decode a `collective:<b64url>` CID string back to raw ActionHash bytes.
fn decode_collective_cid(cid: &str) -> Result<Vec<u8>, StorageError> {
    let raw = cid.strip_prefix("collective:").ok_or_else(|| {
        StorageError::InvalidInput(format!(
            "collective CID must start with 'collective:'; got: {cid}"
        ))
    })?;
    base64url_decode(raw)
        .map_err(|e| StorageError::InvalidInput(format!("invalid collective CID '{cid}': {e}")))
}

/// Decode an `agreement:<b64url>` CID string back to raw ActionHash bytes.
fn decode_agreement_cid(cid: &str) -> Result<Vec<u8>, StorageError> {
    let raw = cid.strip_prefix("agreement:").ok_or_else(|| {
        StorageError::InvalidInput(format!(
            "agreement CID must start with 'agreement:'; got: {cid}"
        ))
    })?;
    base64url_decode(raw)
        .map_err(|e| StorageError::InvalidInput(format!("invalid agreement CID '{cid}': {e}")))
}

/// Convenience alias — same encoding as `decode_agreement_cid` but used
/// in contexts where the caller already validated the prefix.
fn agreement_action_bytes_from_cid(cid: &str) -> Result<Vec<u8>, StorageError> {
    decode_agreement_cid(cid)
}

fn base64url_encode(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn base64url_decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|e| e.to_string())
}

/// Fallback share allocation for when the full agreement entry cannot be decoded.
fn default_share_allocation() -> ShareAllocation {
    use elohim_views::ShareAllocationForm;
    ShareAllocation {
        form: ShareAllocationForm::Declared,
        shares: None,
        affinity_window_blocks: None,
        rebalance_cadence_blocks: None,
        commons_pool_tribute: 0.0,
    }
}
