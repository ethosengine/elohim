//! AttentionTending coordinator functions — Phase 3.5 T9.
//!
//! Implements the coordinator layer for AttentionTending entries:
//! create, refresh TTL, and source-chain-only list. These entries are
//! private (`Visibility::Private`) and never gossiped to the DHT.
//!
//! ## Why no links
//!
//! FeedbackSignal uses links for cross-agent indexing because it is a public
//! DHT entry. AttentionTending is private to the author's source chain. There
//! is nothing to index on the DHT — peers cannot see this entry, so no link
//! is valid or useful. `list_my_tending` queries the local source chain only.
//!
//! ## JSON pre-validation
//!
//! The integrity zome accepts any `String` for `filter_subject_json` and
//! `context_json` (deferred to T15). The coordinator gates JSON *syntax* with
//! `serde_json::from_str::<serde_json::Value>` before calling `create_entry`
//! so malformed data never reaches the source chain.
//!
//! ## signer_pubkey derivation
//!
//! `signer_pubkey` is derived from `agent_info()?.agent_initial_pubkey` at
//! call time — never accepted from the caller. This mirrors the T8 I1 fix on
//! `FeedbackSignal` and prevents caller-side spoofing of another agent's key
//! while the action's author field remains correct.

use content_store_integrity::{AttentionTending, EntryTypes, UnitEntryTypes};
use hdk::prelude::*;
use holochain_serialized_bytes::prelude::SerializedBytes;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// Input for `create_attention_tending`.
///
/// `signer_pubkey` is intentionally absent — the coordinator derives it from
/// `agent_info()?.agent_initial_pubkey.get_raw_39().to_vec()` so a caller
/// cannot spoof another agent's key in the entry payload.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct CreateAttentionTendingInput {
    /// JSON-encoded FilterSubject. Must be syntactically valid JSON; the
    /// coordinator will return a Guest error on parse failure.
    pub filter_subject_json: String,

    /// One of: values-forward / fatigue / scope-mismatch / safety.
    pub classification: String,

    /// Optional human-readable reason.
    pub reason: Option<String>,

    /// TTL in seconds; minimum 3600 (enforced by the integrity floor).
    pub ttl_seconds: u64,

    /// JSON-encoded ContextScope. Must be syntactically valid JSON.
    pub context_json: String,
}

/// Input for `refresh_tending_ttl`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct RefreshTendingTtlInput {
    /// ActionHash of the original AttentionTending entry.
    pub original_action_hash: ActionHash,

    /// New TTL in seconds; minimum 3600 (enforced by the integrity floor).
    pub new_ttl_seconds: u64,
}

/// One AttentionTending record returned from `list_my_tending`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct AttentionTendingRecord {
    pub action_hash: ActionHash,
    pub entry: AttentionTending,
}

// ---------------------------------------------------------------------------
// Coordinator functions
// ---------------------------------------------------------------------------

/// Create an AttentionTending entry on the caller's private source chain.
///
/// Pre-commit gates:
/// 1. `filter_subject_json` must be syntactically valid JSON.
/// 2. `context_json` must be syntactically valid JSON.
///
/// On success:
/// - Derives `signer_pubkey` from `agent_info()?.agent_initial_pubkey` so
///   the entry payload cannot be spoofed.
/// - Sets `tended_at` to a vec containing the creation timestamp.
/// - Creates the `AttentionTending` integrity entry (Visibility::Private).
/// - Returns the new `ActionHash`.
///
/// No links are written — private entries have no cross-agent DHT presence.
#[hdk_extern]
pub fn create_attention_tending(input: CreateAttentionTendingInput) -> ExternResult<ActionHash> {
    // ------------------------------------------------------------------
    // Gate 1: filter_subject_json must be valid JSON.
    // ------------------------------------------------------------------
    serde_json::from_str::<serde_json::Value>(&input.filter_subject_json).map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "filter_subject_json must be valid JSON: {e}"
        )))
    })?;

    // ------------------------------------------------------------------
    // Gate 2: context_json must be valid JSON.
    // ------------------------------------------------------------------
    serde_json::from_str::<serde_json::Value>(&input.context_json).map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "context_json must be valid JSON: {e}"
        )))
    })?;

    // ------------------------------------------------------------------
    // Build the integrity entry.
    // ------------------------------------------------------------------
    // Derive signer_pubkey from agent_info() — never accept from caller.
    let signer_pubkey = agent_info()?.agent_initial_pubkey.get_raw_39().to_vec();

    // Creation timestamp (seconds since epoch).
    let now_secs = sys_time()?.as_seconds_and_nanos().0 as u64;
    let tended_at = vec![now_secs];

    let at = AttentionTending {
        filter_subject_json: input.filter_subject_json,
        classification: input.classification,
        reason: input.reason,
        ttl_seconds: input.ttl_seconds,
        tended_at,
        context_json: input.context_json,
        signer_pubkey,
    };

    // create_entry validates via HDI before writing.
    let action_hash = create_entry(&EntryTypes::AttentionTending(at))?;

    // No links — private entry; cross-agent visibility is intentionally absent.
    Ok(action_hash)
}

/// Refresh the TTL of an existing AttentionTending by appending a new
/// re-tending timestamp and updating `ttl_seconds`.
///
/// Only the original author can refresh their own private entry. This is
/// structurally guaranteed by source-chain locality (private entries cannot
/// be updated by other agents), but the authorship check is also applied
/// defensively.
#[hdk_extern]
pub fn refresh_tending_ttl(input: RefreshTendingTtlInput) -> ExternResult<ActionHash> {
    // Fetch the original record — fails if not found.
    let record = must_get_valid_record(input.original_action_hash.clone())?;

    // Decode the prior entry.
    let prior: AttentionTending = record
        .entry()
        .to_app_option()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "failed to decode AttentionTending entry: {e}"
            )))
        })?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "record has no app entry (tombstoned or wrong type)".to_string()
            ))
        })?;

    // Defensive authorship check — only the original author may re-tend.
    let caller = agent_info()?.agent_initial_pubkey;
    let author = record.action().author().clone();
    if author != caller {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "only the original author may refresh an AttentionTending TTL".to_string()
        )));
    }

    // Append the current timestamp to tended_at.
    let now_secs = sys_time()?.as_seconds_and_nanos().0 as u64;
    let mut new_tended_at = prior.tended_at.clone();
    new_tended_at.push(now_secs);

    let updated = AttentionTending {
        filter_subject_json: prior.filter_subject_json,
        classification: prior.classification,
        reason: prior.reason,
        ttl_seconds: input.new_ttl_seconds,
        tended_at: new_tended_at,
        context_json: prior.context_json,
        signer_pubkey: prior.signer_pubkey,
    };

    let action_hash = update_entry(
        input.original_action_hash,
        &EntryTypes::AttentionTending(updated),
    )?;

    Ok(action_hash)
}

/// Return all AttentionTending entries on the caller's local source chain.
///
/// Queries the source chain only — no DHT access, no peer visibility.
/// Private entries authored by other agents are never returned here.
#[hdk_extern]
pub fn list_my_tending(_: ()) -> ExternResult<Vec<AttentionTendingRecord>> {
    let filter = ChainQueryFilter::new().entry_type(UnitEntryTypes::AttentionTending.try_into()?);
    let records = query(filter)?;

    let mut results = Vec::new();
    for record in records {
        let action_hash = record.action_address().clone();
        if let Ok(Some(entry)) = record.entry().to_app_option::<AttentionTending>() {
            results.push(AttentionTendingRecord { action_hash, entry });
        }
    }

    Ok(results)
}
