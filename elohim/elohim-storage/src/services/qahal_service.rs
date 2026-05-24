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

use holochain_types::prelude::ActionHash;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::StorageError;
use crate::hc_client::HcClient;
use elohim_views::{
    AttestCollabAgreementInputView, CollabAgreementStatus, CollabAgreementView,
    CollabCollectiveView, CollabMembershipRole, CollabMembershipView, CollabQahalView,
    CreateCollabAgreementInputView, CreateCollabCollectiveInputView, ElohimTier, GovernanceTerms,
    MemberKind, ShareAllocation, WithdrawMembershipInputView,
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

    /// Fetch a CollabAgreement by its CID (e.g. "agreement:uhCkk...").
    ///
    /// Fetches the full Agreement Record from the conductor via
    /// `get_collective_by_action` (generic `get()` in the coordinator) and
    /// decodes the entry body as `ZomeCollabAgreementEntry`. Composes
    /// `get_collab_status` + `get_collab_qahal_cid_for_agreement` for the
    /// status and Qahal-CID fields.
    ///
    /// Returns a `CollabAgreementView` with fully-decoded `share_allocation`,
    /// `participants`, `scope`, and `governance_terms` fields.
    pub async fn fetch_collab_agreement(
        &self,
        cid: &str,
    ) -> Result<CollabAgreementView, StorageError> {
        debug!(cid = %cid, "QahalService::fetch_collab_agreement");
        let action_bytes = decode_agreement_cid(cid)?;
        self.fetch_agreement_by_action_bytes(&action_bytes).await
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
        // --- Step 1: Fetch the full Agreement Record ---
        //
        // The coordinator's `get_collective_by_action` extern calls
        // `get(action_hash, GetOptions::default())` — it is a generic HDK
        // `get()` and returns whatever Record lives at the ActionHash.  We
        // call it here with an agreement ActionHash; the returned entry_bytes
        // contain a CollabAgreement (not a Collective), which we decode below
        // as `ZomeCollabAgreementEntry`.
        let record_payload = rmp_serde::to_vec(action_bytes).map_err(|e| {
            StorageError::Internal(format!(
                "encode ActionHash for get_collective_by_action (agreement): {e}"
            ))
        })?;
        let record_result = self
            .hc
            .call_zome(self.zome, "get_collective_by_action", record_payload)
            .await?;

        let maybe_record: Option<ZomeRecord> =
            rmp_serde::from_slice(&record_result).map_err(|e| {
                StorageError::Internal(format!(
                    "decode Option<Record> from get_collective_by_action (agreement): {e}"
                ))
            })?;

        let record = maybe_record.ok_or_else(|| {
            StorageError::NotFound(format!(
                "CollabAgreement not found for action {}",
                hex::encode(action_bytes)
            ))
        })?;

        // --- Step 2: Decode entry bytes as ZomeCollabAgreementEntry ---
        let entry: ZomeCollabAgreementEntry =
            rmp_serde::from_slice(&record.entry_bytes).map_err(|e| {
                StorageError::Internal(format!("decode CollabAgreement entry: {e}"))
            })?;

        // --- Step 3: Parse share_allocation_json → ShareAllocation ---
        let share_allocation: ShareAllocation =
            serde_json::from_str(&entry.share_allocation_json).map_err(|e| {
                StorageError::Internal(format!(
                    "parse share_allocation_json for agreement {}: {e}",
                    bytes_to_agreement_cid(action_bytes)
                ))
            })?;

        // --- Step 4: Parse governance_terms_json → GovernanceTerms ---
        let governance_terms: Option<GovernanceTerms> =
            if entry.governance_terms_json.trim().is_empty()
                || entry.governance_terms_json.trim() == "{}"
            {
                None
            } else {
                serde_json::from_str(&entry.governance_terms_json)
                    .map(Some)
                    .map_err(|e| {
                        StorageError::Internal(format!(
                            "parse governance_terms_json for agreement {}: {e}",
                            bytes_to_agreement_cid(action_bytes)
                        ))
                    })?
            };

        // --- Step 5: Fetch status (get_collab_status extern) ---
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

        // --- Step 6: Optionally resolve the Collab-Qahal CID if instantiated ---
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

        // --- Step 7: Parse initial_tier ---
        let initial_tier = match entry.initial_tier.as_str() {
            "T0" => ElohimTier::T0,
            other => {
                return Err(StorageError::Internal(format!(
                    "unrecognised initial_tier '{other}' in CollabAgreement entry — M1 only supports T0"
                )))
            }
        };

        Ok(CollabAgreementView {
            cid,
            authored_by_agent_cid: entry.authored_by_agent_cid,
            participants: entry.participants,
            scope: entry.scope,
            share_allocation,
            commons_pool_tribute: entry.commons_pool_tribute,
            governance_terms,
            initial_tier,
            created_at_block_height: entry.created_at_block_height,
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

/// Mirror of `imagodei_integrity::qahal::CollabAgreement` for deserialization.
///
/// Field order and names must match the integrity entry struct exactly, since
/// rmp_serde uses the named-map encoding and matches fields by key.  The HDK
/// `#[hdk_entry_helper]` macro delegates to serde with default field names.
///
/// `Serialize` is derived to allow round-trip tests via `rmp_serde::to_vec_named`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ZomeCollabAgreementEntry {
    pub authored_by_agent_cid: String,
    pub participants: Vec<String>,
    pub scope: String,
    /// JSON-stringified `ShareAllocation` (camelCase keys).
    pub share_allocation_json: String,
    pub commons_pool_tribute: f64,
    /// JSON-stringified `GovernanceTerms` (camelCase keys). May be `"{}"` when absent.
    pub governance_terms_json: String,
    /// Populated once the Collab-Qahal is instantiated; `None` before full attestation.
    #[serde(default)]
    pub anchor_collective_cid: Option<String>,
    /// "T0" only in M1.
    pub initial_tier: String,
    pub created_at_block_height: u64,
    /// Carried for completeness of the wire type; not projected.
    #[serde(default)]
    #[allow(dead_code)]
    pub salt: String,
}

// =============================================================================
// CID codec helpers
// =============================================================================

/// Encode a 39-byte ActionHash into a `collective:<display>` CID string.
///
/// Uses `ActionHash`'s `Display` implementation (HoloHash standard base32-padded
/// encoding, "uhCkk..." prefix) — identical to the coordinator's
/// `format!("collective:{}", hash)`. Both sides of the storage/zome boundary
/// now produce byte-equal CIDs for the same DHT entry.
fn bytes_to_collective_cid(bytes: &[u8]) -> String {
    format!("collective:{}", action_hash_from_bytes(bytes))
}

/// Encode a 39-byte ActionHash into an `agreement:<display>` CID string.
fn bytes_to_agreement_cid(bytes: &[u8]) -> String {
    format!("agreement:{}", action_hash_from_bytes(bytes))
}

/// Decode a `collective:<HoloHash-display>` CID string back to raw ActionHash bytes.
fn decode_collective_cid(cid: &str) -> Result<Vec<u8>, StorageError> {
    let raw = cid.strip_prefix("collective:").ok_or_else(|| {
        StorageError::InvalidInput(format!(
            "collective CID must start with 'collective:'; got: {cid}"
        ))
    })?;
    ActionHash::try_from(raw)
        .map(|h| h.get_raw_39().to_vec())
        .map_err(|e| StorageError::InvalidInput(format!("invalid collective CID '{cid}': {e}")))
}

/// Decode an `agreement:<HoloHash-display>` CID string back to raw ActionHash bytes.
fn decode_agreement_cid(cid: &str) -> Result<Vec<u8>, StorageError> {
    let raw = cid.strip_prefix("agreement:").ok_or_else(|| {
        StorageError::InvalidInput(format!(
            "agreement CID must start with 'agreement:'; got: {cid}"
        ))
    })?;
    ActionHash::try_from(raw)
        .map(|h| h.get_raw_39().to_vec())
        .map_err(|e| StorageError::InvalidInput(format!("invalid agreement CID '{cid}': {e}")))
}

/// Convenience alias — same decoding as `decode_agreement_cid` but used
/// in contexts where the caller already validated the prefix.
fn agreement_action_bytes_from_cid(cid: &str) -> Result<Vec<u8>, StorageError> {
    decode_agreement_cid(cid)
}

/// Construct an `ActionHash` from raw conductor bytes (39-byte HoloHash format).
///
/// Panics on malformed bytes — callers must only pass bytes originating from the
/// Holochain conductor, which always emits valid 39-byte ActionHash values.
fn action_hash_from_bytes(bytes: &[u8]) -> ActionHash {
    ActionHash::from_raw_39(bytes.to_vec())
}

// =============================================================================
// Unit tests — decode path only (no conductor required)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that a `ZomeCollabAgreementEntry` round-trips through msgpack
    /// serialization (the encoding the conductor uses) and that the JSON sub-fields
    /// (`share_allocation_json`, `governance_terms_json`) parse into the correct
    /// view-layer types — exercising the full decode path in
    /// `fetch_agreement_by_action_bytes` without a live Holochain conductor.
    #[test]
    fn zome_collab_agreement_entry_round_trips_through_decode() {
        // Build a realistic entry matching what the conductor would emit.
        let share_allocation_json = serde_json::json!({
            "form": "declared",
            "shares": [
                { "collectiveCid": "collective:coll-a", "share": 0.475 },
                { "collectiveCid": "collective:coll-b", "share": 0.475 }
            ],
            "commonsPoolTribute": 0.05
        })
        .to_string();

        let governance_terms_json = serde_json::json!({
            "exitTerms": "clean"
        })
        .to_string();

        let entry = ZomeCollabAgreementEntry {
            authored_by_agent_cid: "agent:test-author".into(),
            participants: vec![
                "collective:coll-a".into(),
                "collective:coll-b".into(),
            ],
            scope: "test-project".into(),
            share_allocation_json: share_allocation_json.clone(),
            commons_pool_tribute: 0.05,
            governance_terms_json: governance_terms_json.clone(),
            anchor_collective_cid: None,
            initial_tier: "T0".into(),
            created_at_block_height: 42,
            salt: "0000000000000000000000000000000a".into(),
        };

        // --- Serialize as the conductor would (named msgpack) ---
        let bytes = rmp_serde::to_vec_named(&entry)
            .expect("rmp_serde::to_vec_named should not fail for ZomeCollabAgreementEntry");

        // --- Deserialize back, mirroring fetch_agreement_by_action_bytes step 2 ---
        let decoded: ZomeCollabAgreementEntry = rmp_serde::from_slice(&bytes)
            .expect("rmp_serde::from_slice should decode ZomeCollabAgreementEntry");

        assert_eq!(decoded.authored_by_agent_cid, "agent:test-author");
        assert_eq!(decoded.participants.len(), 2);
        assert_eq!(decoded.scope, "test-project");
        assert_eq!(decoded.initial_tier, "T0");
        assert_eq!(decoded.created_at_block_height, 42);
        assert!((decoded.commons_pool_tribute - 0.05).abs() < 1e-9);

        // --- Parse share_allocation_json, mirroring step 3 ---
        let share_allocation: ShareAllocation =
            serde_json::from_str(&decoded.share_allocation_json)
                .expect("share_allocation_json should parse into ShareAllocation");

        assert!(
            share_allocation.shares.is_some(),
            "ShareAllocation.shares must be Some(_) after full decode"
        );
        let shares = share_allocation.shares.as_ref().unwrap();
        assert_eq!(shares.len(), 2, "expected 2 declared shares");

        let coll_a = shares
            .iter()
            .find(|s| s.collective_cid == "collective:coll-a")
            .expect("collective:coll-a share entry must be present");
        assert!((coll_a.share - 0.475).abs() < 1e-9, "coll-a share ~ 0.475");

        let coll_b = shares
            .iter()
            .find(|s| s.collective_cid == "collective:coll-b")
            .expect("collective:coll-b share entry must be present");
        assert!((coll_b.share - 0.475).abs() < 1e-9, "coll-b share ~ 0.475");

        assert!(
            (share_allocation.commons_pool_tribute - 0.05).abs() < 1e-9,
            "commons_pool_tribute ~ 0.05"
        );

        // --- Parse governance_terms_json, mirroring step 4 ---
        let governance_terms: Option<GovernanceTerms> =
            if decoded.governance_terms_json.trim().is_empty()
                || decoded.governance_terms_json.trim() == "{}"
            {
                None
            } else {
                serde_json::from_str(&decoded.governance_terms_json)
                    .map(Some)
                    .expect("governance_terms_json should parse into GovernanceTerms")
            };

        assert!(governance_terms.is_some(), "governance_terms must be Some");
        assert_eq!(
            governance_terms.unwrap().exit_terms,
            "clean",
            "exit_terms must be 'clean'"
        );

        // --- Parse initial_tier, mirroring step 7 ---
        assert_eq!(decoded.initial_tier, "T0");
    }

    /// Verify that an empty / `"{}"` governance_terms_json maps to None,
    /// consistent with the defensive check in fetch_agreement_by_action_bytes.
    #[test]
    fn empty_governance_terms_json_decodes_to_none() {
        for empty_val in &["{}", "  {}  ", ""] {
            let governance_terms: Option<GovernanceTerms> =
                if empty_val.trim().is_empty() || empty_val.trim() == "{}" {
                    None
                } else {
                    serde_json::from_str::<GovernanceTerms>(empty_val)
                        .map(Some)
                        .unwrap_or(None)
                };
            assert!(
                governance_terms.is_none(),
                "governance_terms for '{}' should be None",
                empty_val
            );
        }
    }
}

