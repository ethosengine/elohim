//! FeedbackSignal coordinator functions — Phase 3.5 P3.5.1 (T8).
//!
//! Implements the coordinator layer for FeedbackSignal entries:
//! create, query by target, and query by signer. The cross-entity
//! gates that the HDI integrity layer cannot enforce (retraction
//! authorship, correction evidence resolution) are applied here as
//! pre-commit checks before `create_entry` is called.
//!
//! ## Cross-entity gates
//!
//! **Retraction authorship**: A retraction is a claim that the *original author*
//! is retracting their own content. The gate fetches the target entry's action
//! header with `must_get_valid_record` and compares `action.author()` against
//! `agent_info()?.agent_initial_pubkey`. If they differ the function returns an
//! error without writing anything to the source chain.
//!
//! **Correction evidence**: A correction must cite an existing Correction EPR.
//! The gate calls `must_get_valid_record` on the evidence action hash; if the
//! record cannot be resolved the function returns an error.
//!
//! ## Link topology
//!
//! Two link types are written on commit (both registered in T8):
//!
//! - `LinkTypes::TargetToFeedbackSignal`: base = target action hash,
//!   target = new FeedbackSignal action hash. Supports
//!   `get_feedback_signals_for_target`.
//!
//! - `LinkTypes::SignerToFeedbackSignal`: base = per-agent `StringAnchor`
//!   keyed on `"feedback_signer/<agent_pubkey_b64>"`, target = new
//!   FeedbackSignal action hash. Supports `list_feedback_signals_by_signer`.

use content_store_integrity::{EntryTypes, FeedbackSignal, LinkTypes, StringAnchor};
use hdk::prelude::*;
use holochain_serialized_bytes::prelude::SerializedBytes;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// Input for `create_feedback_signal`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct CreateFeedbackSignalInput {
    /// ActionHash of the target Content (or other EPR) that this signal acts on.
    /// Used as the base of the `TargetToFeedbackSignal` link and stored (as a
    /// base64 string) in `FeedbackSignal.target_cid`.
    pub target_action_hash: ActionHash,

    /// One of: squelch / correction / retraction / quarantine.
    pub signal_kind: String,

    /// ActionHash of a Correction EPR.  Required when `signal_kind == "correction"`.
    pub evidence_action_hash: Option<ActionHash>,

    /// One of: advisory / debit-soft / debit-firm.
    pub standing_impact: String,

    /// Raw ed25519 public key bytes of the issuing agent.
    /// Must match `agent_info()?.agent_initial_pubkey` bytes — validated on-chain
    /// so the integrity entry is auditable without re-deriving from the action header.
    pub signer_pubkey: Vec<u8>,
}

/// One FeedbackSignal record returned from query functions.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct FeedbackSignalRecord {
    pub action_hash: ActionHash,
    pub entry: FeedbackSignal,
}

// ---------------------------------------------------------------------------
// Coordinator functions
// ---------------------------------------------------------------------------

/// Create a FeedbackSignal with cross-entity pre-commit gates.
///
/// Pre-commit gates:
/// 1. If `signal_kind == "retraction"`: caller must be the original author of
///    `target_action_hash`. Checked via `must_get_valid_record`.
/// 2. If `signal_kind == "correction"`: `evidence_action_hash` must resolve to
///    an existing valid record. Checked via `must_get_valid_record`.
///
/// On success:
/// - Creates the `FeedbackSignal` integrity entry.
/// - Creates a `TargetToFeedbackSignal` link from the target action.
/// - Creates a `SignerToFeedbackSignal` link from a per-agent anchor.
/// - Returns the new `ActionHash`.
///
/// The `ProjectionSignal::FeedbackSignalCommitted` signal is emitted by
/// `post_commit` (wired in lib.rs) after the source chain flush, following
/// Phase 3's manifest signal pattern.
#[hdk_extern]
pub fn create_feedback_signal(input: CreateFeedbackSignalInput) -> ExternResult<ActionHash> {
    // ------------------------------------------------------------------
    // Gate 1: retraction signer must equal the original author.
    // ------------------------------------------------------------------
    if input.signal_kind == "retraction" {
        let target_record = must_get_valid_record(input.target_action_hash.clone())?;
        let target_author = target_record.action().author().clone();
        let caller = agent_info()?.agent_initial_pubkey;
        if target_author != caller {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "retraction signer must be the original author".to_string()
            )));
        }
    }

    // ------------------------------------------------------------------
    // Gate 2: correction evidence_action_hash must resolve.
    // ------------------------------------------------------------------
    if input.signal_kind == "correction" {
        let evidence_hash = input.evidence_action_hash.as_ref().ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "correction requires evidence_action_hash".to_string()
            ))
        })?;
        // Attempt resolution — returns error if not found.
        must_get_valid_record(evidence_hash.clone())?;
    }

    // ------------------------------------------------------------------
    // Build the integrity entry.
    // ------------------------------------------------------------------
    // Store the target action hash as a base64 string in target_cid so
    // the DHT entry is self-describing without requiring a separate lookup.
    let target_cid = format!("{}", input.target_action_hash);

    // Convert evidence action hash to an evidence CID string (same encoding).
    let evidence_cid = input
        .evidence_action_hash
        .as_ref()
        .map(|h| format!("{}", h));

    let fs = FeedbackSignal {
        target_cid,
        signal_kind: input.signal_kind.clone(),
        evidence_cid,
        standing_impact: input.standing_impact.clone(),
        signer_pubkey: input.signer_pubkey.clone(),
    };

    // create_entry validates via HDI before writing.
    let action_hash = create_entry(&EntryTypes::FeedbackSignal(fs))?;

    // ------------------------------------------------------------------
    // Index links.
    // ------------------------------------------------------------------

    // TargetToFeedbackSignal: base = target entry's action hash.
    create_link(
        input.target_action_hash.clone(),
        action_hash.clone(),
        LinkTypes::TargetToFeedbackSignal,
        (),
    )?;

    // SignerToFeedbackSignal: base = per-agent StringAnchor.
    // Anchor key: "feedback_signer/<pubkey_base64>"
    let signer_anchor_key = format!("feedback_signer/{}", agent_info()?.agent_initial_pubkey);
    let signer_anchor = StringAnchor::new("feedback_signer", &signer_anchor_key);
    let signer_anchor_hash = hash_entry(&EntryTypes::StringAnchor(signer_anchor))?;
    create_link(
        signer_anchor_hash,
        action_hash.clone(),
        LinkTypes::SignerToFeedbackSignal,
        (),
    )?;

    Ok(action_hash)
}

/// Return all FeedbackSignals that target the given action hash.
///
/// Walks `LinkTypes::TargetToFeedbackSignal` links from `target_action_hash`.
#[hdk_extern]
pub fn get_feedback_signals_for_target(
    target_action_hash: ActionHash,
) -> ExternResult<Vec<FeedbackSignalRecord>> {
    let query =
        LinkQuery::try_new(target_action_hash, LinkTypes::TargetToFeedbackSignal)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Ok(ah) = ActionHash::try_from(link.target.clone()) {
            if let Ok(Some(record)) = get(ah.clone(), GetOptions::default()) {
                if let Ok(Some(entry)) = record.entry().to_app_option::<FeedbackSignal>() {
                    results.push(FeedbackSignalRecord {
                        action_hash: ah,
                        entry,
                    });
                }
            }
        }
    }

    Ok(results)
}

/// Return all FeedbackSignals authored by `signer_pubkey`.
///
/// Walks `LinkTypes::SignerToFeedbackSignal` links from the per-agent anchor.
#[hdk_extern]
pub fn list_feedback_signals_by_signer(
    signer_pubkey: AgentPubKey,
) -> ExternResult<Vec<FeedbackSignalRecord>> {
    let signer_anchor_key = format!("feedback_signer/{}", signer_pubkey);
    let signer_anchor = StringAnchor::new("feedback_signer", &signer_anchor_key);
    let signer_anchor_hash = hash_entry(&EntryTypes::StringAnchor(signer_anchor))?;

    let query = LinkQuery::try_new(signer_anchor_hash, LinkTypes::SignerToFeedbackSignal)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Ok(ah) = ActionHash::try_from(link.target.clone()) {
            if let Ok(Some(record)) = get(ah.clone(), GetOptions::default()) {
                if let Ok(Some(entry)) = record.entry().to_app_option::<FeedbackSignal>() {
                    results.push(FeedbackSignalRecord {
                        action_hash: ah,
                        entry,
                    });
                }
            }
        }
    }

    Ok(results)
}
