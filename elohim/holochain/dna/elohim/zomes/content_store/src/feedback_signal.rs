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
//!
//! The target index is SEPARABLE from the act: `create_feedback_signal` may
//! defer it (`defer_target_link`), and `publish_feedback_signal_target_link`
//! publishes it afterwards from the act's own `target_cid`. The act's identity
//! is `(origin DNA hash, action hash)`; the link is an unordered index over it,
//! restorable by any peer holding a verified reference (contract §3).

use content_store_integrity::{Content, EntryTypes, FeedbackSignal, LinkTypes, StringAnchor};
use hdk::prelude::*;
use holochain_serialized_bytes::prelude::SerializedBytes;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// Input for `create_feedback_signal`.
///
/// `signer_pubkey` is intentionally absent from this struct. The coordinator
/// derives it from `agent_info()?.agent_initial_pubkey.get_raw_39().to_vec()`
/// at call time, making it impossible for a caller to spoof another agent's
/// pubkey in the entry payload.
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

    /// Commit the act WITHOUT its `TargetToFeedbackSignal` index link, leaving
    /// the index to a later [`publish_feedback_signal_target_link`] call
    /// (contract §3).
    ///
    /// The act's identity is `(origin DNA hash, action hash)`; the link is an
    /// unordered INDEX over it, not part of the act. Separating the two is what
    /// makes "a late link is picked up on a later tick" an exercisable path
    /// rather than an assertion: an act may exist, be fetchable by reference,
    /// and become discoverable-by-enumeration afterwards.
    ///
    /// Additive and defaulted: an older caller that omits the field commits
    /// entry and both index links in one flush exactly as before.
    #[serde(default)]
    pub defer_target_link: bool,
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
/// - Derives `signer_pubkey` from `agent_info()?.agent_initial_pubkey` so the
///   entry payload cannot be spoofed by the caller.
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
        // ADMISSION (contract §8). Resolution alone is not enough: the evidence
        // must be a PUBLIC Correction EPR whose embedded, immutable request
        // matches this act's own fields. That binding is what makes a second
        // act filed under the same operation a GROUP MEMBER (§7) rather than a
        // second contribution — the substrate half of "one intended act
        // survives a lost response".
        let operation_id = admit_correction_evidence(
            evidence_hash,
            &input.target_action_hash,
            &input.signal_kind,
            &input.standing_impact,
        )?;
        debug!(
            "create_feedback_signal: correction admitted under operation {}",
            operation_id
        );
    }

    // ------------------------------------------------------------------
    // Build the integrity entry.
    // ------------------------------------------------------------------
    // Derive signer_pubkey from agent_info() — never accept it from the
    // caller, which would allow a malicious caller to spoof another agent's
    // pubkey in the entry payload while the action's author field remains
    // correct.
    let signer_pubkey = agent_info()?.agent_initial_pubkey.get_raw_39().to_vec();

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
        vouch_kind: None, // create_feedback_signal handles non-vouch kinds only; use create_vouch for vouches
        evidence_cid,
        standing_impact: input.standing_impact.clone(),
        signer_pubkey,
    };

    // create_entry validates via HDI before writing.
    let action_hash = create_entry(&EntryTypes::FeedbackSignal(fs))?;

    // ------------------------------------------------------------------
    // Index links.
    // ------------------------------------------------------------------

    // TargetToFeedbackSignal: base = target entry's action hash.
    // Deferred (§3): the act still commits and is still fetchable by reference;
    // only its index publication moves to publish_feedback_signal_target_link.
    if !input.defer_target_link {
        create_link(
            input.target_action_hash.clone(),
            action_hash.clone(),
            LinkTypes::TargetToFeedbackSignal,
            (),
        )?;
    }

    // SignerToFeedbackSignal: base = per-agent StringAnchor.
    // Always written: this is the SIGNER's own index over their own acts, and
    // §8's phase-2 recovery enumerates it. Deferring the target index never
    // costs an author the ability to find their own act again.
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

// ---------------------------------------------------------------------------
// create_vouch — T6 addition (Light Up the Graph sprint)
// ---------------------------------------------------------------------------

/// Input for `create_vouch`.
///
/// The vouching agent's pubkey is derived from `agent_info()` and cannot be
/// caller-supplied (prevents signer spoofing, same pattern as `create_feedback_signal`).
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct CreateVouchInput {
    /// ActionHash (encoded as ActionHash) of the FeedbackSignal being vouched on.
    /// Must resolve to an existing FeedbackSignal entry in the DHT.
    pub target_action_hash: ActionHash,

    /// "accept-correction" or "restitution". Validated against VOUCH_KINDS
    /// in the integrity layer; the coordinator also pre-checks for clarity.
    pub vouch_kind: String,

    /// "advisory", "debit-soft", or "debit-firm". Whitelisted by the integrity
    /// validator; passed through without coordinator-side override.
    pub standing_impact: String,
}

/// Create a vouch signal on an existing FeedbackSignal.
///
/// A vouch is a positive attestation that endorses the validity of another
/// agent's signal. The vouching agent MUST differ from the original signer
/// (no-self-vouch). This guard is enforced both here (returns a clean error
/// to the caller) and in the integrity validator (validator backstop).
///
/// On success:
/// - Derives `signer_pubkey` from `agent_info()?.agent_initial_pubkey`.
/// - Creates a FeedbackSignal entry with `signal_kind = "vouch"`.
/// - Creates a `TargetToFeedbackSignal` link from the target FeedbackSignal.
/// - Creates a `SignerToFeedbackSignal` link from the per-agent anchor.
/// - Returns the new `ActionHash`.
///
/// `post_commit` emits `ProjectionSignal::FeedbackSignalCommitted` automatically.
#[hdk_extern]
pub fn create_vouch(input: CreateVouchInput) -> ExternResult<ActionHash> {
    // ------------------------------------------------------------------
    // Resolve target — validates it is a real, valid DHT record.
    // ------------------------------------------------------------------
    let target_record = must_get_valid_record(input.target_action_hash.clone())?;
    let target_signal: FeedbackSignal = target_record
        .entry()
        .to_app_option()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "target entry decode failed: {}",
                e
            )))
        })?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "target_action_hash does not point to a FeedbackSignal".to_string()
            ))
        })?;

    // ------------------------------------------------------------------
    // Derive signer_pubkey from agent_info — caller cannot spoof.
    // ------------------------------------------------------------------
    let signer_pubkey = agent_info()?.agent_initial_pubkey.get_raw_39().to_vec();

    // ------------------------------------------------------------------
    // Coordinator-side no-self-vouch guard.
    // (Integrity validator backstops this; coordinator provides caller UX.)
    // ------------------------------------------------------------------
    if signer_pubkey == target_signal.signer_pubkey {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "self-vouch forbidden: vouch signer must differ from target signal signer".to_string()
        )));
    }

    // ------------------------------------------------------------------
    // Build the FeedbackSignal entry.
    // ------------------------------------------------------------------
    let target_cid = format!("{}", input.target_action_hash);

    let fs = FeedbackSignal {
        target_cid,
        signal_kind: "vouch".to_string(),
        vouch_kind: Some(input.vouch_kind),
        evidence_cid: None,
        standing_impact: input.standing_impact,
        signer_pubkey,
    };

    // create_entry validates through HDI before writing to source chain.
    let action_hash = create_entry(&EntryTypes::FeedbackSignal(fs))?;

    // ------------------------------------------------------------------
    // Index links.
    // ------------------------------------------------------------------

    // TargetToFeedbackSignal: base = target FeedbackSignal action hash.
    // Supports get_feedback_signals_for_target querying vouches on a signal.
    create_link(
        input.target_action_hash.clone(),
        action_hash.clone(),
        LinkTypes::TargetToFeedbackSignal,
        (),
    )?;

    // SignerToFeedbackSignal: base = per-agent StringAnchor.
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
    let query = LinkQuery::try_new(target_action_hash, LinkTypes::TargetToFeedbackSignal)?;
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

// ===========================================================================
// Accountable correction — slice 1 additions (contract §§1, 3, 8)
// ===========================================================================
//
// Three things land here, all coordinator-only (no DNA hash move):
//
//   1. `get_feedback_signal_record` — the signed action + entry for ONE act,
//      so a discovering peer can verify §1 (entry type, entry-hash binding,
//      entry bytes present) and recover the author from the SIGNED action
//      rather than trusting the coordinator-derived `signer_pubkey`.
//
//   2. `get_feedback_signal_refs_for_target` / `list_feedback_signal_refs_by_signer`
//      — REFERENCE-plus-OUTCOME queries. The existing `_for_target` /
//      `_by_signer` functions fetch every linked record before returning AND
//      silently drop fetch failures, so a peer pays for history it has already
//      applied and cannot tell "absent" from "unfetchable". These return one
//      row per DEDUPLICATED link with an explicit outcome, and only fetch when
//      the caller asks. The originals are untouched for compatibility.
//
//   3. Correction ADMISSION. Today the coordinator checks only that the
//      evidence action resolves. §8 binds the act to an immutable request: the
//      evidence must be a PUBLIC Correction EPR whose embedded request matches
//      the feedback's own fields, so a second act filed under the same
//      operation is a GROUP MEMBER rather than a second contribution.

/// The immutable request an operation pins, embedded in the Correction EPR's
/// `metadata_json` under the `correctionRequest` key (contract §8).
///
/// This is a JSON field on the EXISTING `Content` entry — deliberately not a new
/// entry type. The DNA's entry-type budget is the scarce resource; a new social
/// binding is carried in the body of a record that already exists.
///
/// Wire shape (camelCase, because it is authored by the storage outbox and read
/// by both the coordinator and the projector):
///
/// ```json
/// { "correctionRequest": {
///     "operationId": "1f0c…",
///     "targetActionHash": "uhCkk…",
///     "signalKind": "correction",
///     "standingImpact": "debit-soft" } }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionRequest {
    pub operation_id: String,
    pub target_action_hash: String,
    pub signal_kind: String,
    pub standing_impact: String,
}

/// The `metadata_json` key the request lives under.
pub const CORRECTION_REQUEST_KEY: &str = "correctionRequest";

/// Reach tiers a Correction EPR may carry. §8 requires the evidence to be a
/// PUBLIC Correction EPR: a correction whose evidence no peer may read cannot
/// be verified by the peers the correction is addressed to.
pub const PUBLIC_EVIDENCE_REACH: [&str; 2] = ["public", "commons"];

/// **Pure.** Extract the embedded request from a Content entry's
/// `metadata_json`. `Err` carries the refusal text the caller returns verbatim.
pub fn parse_correction_request(metadata_json: &str) -> Result<CorrectionRequest, String> {
    let root: serde_json::Value = serde_json::from_str(metadata_json)
        .map_err(|e| format!("evidence metadata_json is not valid JSON: {e}"))?;
    let node = root.get(CORRECTION_REQUEST_KEY).ok_or_else(|| {
        format!("evidence carries no '{CORRECTION_REQUEST_KEY}' object in metadata_json")
    })?;
    let req: CorrectionRequest = serde_json::from_value(node.clone())
        .map_err(|e| format!("evidence '{CORRECTION_REQUEST_KEY}' is malformed: {e}"))?;
    if req.operation_id.trim().is_empty() {
        return Err("evidence correctionRequest.operationId is empty".to_string());
    }
    Ok(req)
}

/// **Pure.** The request must match the act being filed, field for field.
///
/// A mismatch is a REJECTION, not a retryable condition: it means the act does
/// not belong to the operation its evidence names, and §7 rejects such an act as
/// a non-member of the group rather than collapsing it into one.
pub fn check_correction_request(
    req: &CorrectionRequest,
    target_action_hash: &str,
    signal_kind: &str,
    standing_impact: &str,
) -> Result<(), String> {
    if req.target_action_hash != target_action_hash {
        return Err(format!(
            "evidence correctionRequest.targetActionHash '{}' does not match the feedback target \
             '{}'",
            req.target_action_hash, target_action_hash
        ));
    }
    if req.signal_kind != signal_kind {
        return Err(format!(
            "evidence correctionRequest.signalKind '{}' does not match the feedback signal_kind \
             '{}'",
            req.signal_kind, signal_kind
        ));
    }
    if req.standing_impact != standing_impact {
        return Err(format!(
            "evidence correctionRequest.standingImpact '{}' does not match the feedback \
             standing_impact '{}'",
            req.standing_impact, standing_impact
        ));
    }
    Ok(())
}

/// Admission for a `correction` act (contract §8).
///
/// The evidence record must (a) resolve, (b) carry a `Content` entry, (c) be
/// public, and (d) embed a `correctionRequest` matching this act's own fields.
/// Returns the pinned `operation_id` so the caller can log the group key.
fn admit_correction_evidence(
    evidence_hash: &ActionHash,
    target_action_hash: &ActionHash,
    signal_kind: &str,
    standing_impact: &str,
) -> ExternResult<String> {
    let record = must_get_valid_record(evidence_hash.clone())?;
    let content: Content = record
        .entry()
        .to_app_option()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "correction evidence decode failed: {e}"
            )))
        })?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "correction evidence must be a Correction EPR (a Content record); the referenced \
                 action carries no Content entry"
                    .to_string()
            ))
        })?;

    if !PUBLIC_EVIDENCE_REACH.contains(&content.reach.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "correction evidence must be a PUBLIC Correction EPR (reach one of {:?}); this \
             evidence carries reach '{}' — a correction whose evidence its addressee cannot read \
             is not accountable",
            PUBLIC_EVIDENCE_REACH, content.reach
        ))));
    }

    let req = parse_correction_request(&content.metadata_json)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("correction evidence: {e}"))))?;
    check_correction_request(
        &req,
        &format!("{}", target_action_hash),
        signal_kind,
        standing_impact,
    )
    .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("correction evidence: {e}"))))?;

    Ok(req.operation_id)
}

// ---------------------------------------------------------------------------
// Reference + outcome discovery (contract §3)
// ---------------------------------------------------------------------------

/// One discovered act reference with an EXPLICIT outcome.
///
/// `fetch_outcome` is `"referenced"` when the caller did not ask for
/// resolution — the cheap path that lets a peer skip acts it has already
/// applied. With `resolve: true` it is one of `"fetched"`, `"not-found"`,
/// `"no-entry"` or `"wrong-type"`; an unfetchable act is REPORTED so the caller
/// can hold it pending instead of reading absence as "uncontested".
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct FeedbackSignalRef {
    pub action_hash: ActionHash,
    pub fetch_outcome: String,
    /// Present only when `resolve` was set AND the outcome is `"fetched"`.
    pub entry: Option<FeedbackSignal>,
}

/// The full answer of a reference query, including what it could NOT turn into
/// a reference. An empty `refs` at one tick means "not observed", never
/// "uncontested" (§3).
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct FeedbackSignalRefs {
    pub refs: Vec<FeedbackSignalRef>,
    pub link_count: u32,
    pub duplicate_links: u32,
    pub invalid_link_targets: u32,
}

/// Input for [`get_feedback_signal_refs_for_target`].
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct FeedbackSignalRefsForTargetInput {
    pub target_action_hash: ActionHash,
    /// Fetch each referenced record and report a real outcome. Default `false`
    /// — the point of this query is to NOT pay for records the caller has
    /// already applied.
    #[serde(default)]
    pub resolve: bool,
}

/// Input for [`list_feedback_signal_refs_by_signer`].
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct FeedbackSignalRefsBySignerInput {
    pub signer_pubkey: AgentPubKey,
    #[serde(default)]
    pub resolve: bool,
}

/// Shared body: enumerate links from `base`, deduplicate targets, and report an
/// explicit outcome per reference.
fn refs_from_links(base: AnyLinkableHash, resolve: bool) -> ExternResult<FeedbackSignalRefs> {
    let query = LinkQuery::try_new(base, LinkTypes::TargetToFeedbackSignal)?;
    let links = get_links(query, GetStrategy::default())?;
    refs_from_link_set(links, resolve)
}

fn refs_from_link_set(links: Vec<Link>, resolve: bool) -> ExternResult<FeedbackSignalRefs> {
    let link_count = links.len() as u32;
    let mut duplicate_links = 0u32;
    let mut invalid_link_targets = 0u32;
    let mut seen: Vec<ActionHash> = Vec::new();
    for link in &links {
        match ActionHash::try_from(link.target.clone()) {
            Ok(ah) => {
                if seen.contains(&ah) {
                    duplicate_links += 1;
                } else {
                    seen.push(ah);
                }
            }
            Err(_) => invalid_link_targets += 1,
        }
    }

    let mut refs = Vec::with_capacity(seen.len());
    for ah in seen {
        if !resolve {
            refs.push(FeedbackSignalRef {
                action_hash: ah,
                fetch_outcome: "referenced".to_string(),
                entry: None,
            });
            continue;
        }
        match get(ah.clone(), GetOptions::default())? {
            None => refs.push(FeedbackSignalRef {
                action_hash: ah,
                fetch_outcome: "not-found".to_string(),
                entry: None,
            }),
            Some(record) => {
                let decoded: Option<FeedbackSignal> =
                    record.entry().to_app_option().ok().flatten();
                match decoded {
                    Some(entry) => refs.push(FeedbackSignalRef {
                        action_hash: ah,
                        fetch_outcome: "fetched".to_string(),
                        entry: Some(entry),
                    }),
                    None => refs.push(FeedbackSignalRef {
                        action_hash: ah,
                        // A record with no readable app entry is either a
                        // Hidden/NotStored remote answer or a different entry
                        // type behind the link. Either way it is NOT evidence.
                        fetch_outcome: if record.entry().as_option().is_none() {
                            "no-entry".to_string()
                        } else {
                            "wrong-type".to_string()
                        },
                        entry: None,
                    }),
                }
            }
        }
    }

    Ok(FeedbackSignalRefs {
        refs,
        link_count,
        duplicate_links,
        invalid_link_targets,
    })
}

/// Reference + outcome variant of [`get_feedback_signals_for_target`] (§3).
#[hdk_extern]
pub fn get_feedback_signal_refs_for_target(
    input: FeedbackSignalRefsForTargetInput,
) -> ExternResult<FeedbackSignalRefs> {
    refs_from_links(input.target_action_hash.into(), input.resolve)
}

/// Reference + outcome variant of [`list_feedback_signals_by_signer`] (§3).
///
/// This is the query §8's phase-2 recovery reads: after an uncertain
/// `create_feedback_signal`, the submitting cell enumerates its OWN acts and
/// matches the operation tuple. Zero matches is `unresolved`, never a licence
/// to resubmit.
#[hdk_extern]
pub fn list_feedback_signal_refs_by_signer(
    input: FeedbackSignalRefsBySignerInput,
) -> ExternResult<FeedbackSignalRefs> {
    let signer_anchor_key = format!("feedback_signer/{}", input.signer_pubkey);
    let signer_anchor = StringAnchor::new("feedback_signer", &signer_anchor_key);
    let signer_anchor_hash = hash_entry(&EntryTypes::StringAnchor(signer_anchor))?;
    let query = LinkQuery::try_new(signer_anchor_hash, LinkTypes::SignerToFeedbackSignal)?;
    let links = get_links(query, GetStrategy::default())?;
    refs_from_link_set(links, input.resolve)
}

/// Return the SIGNED record for one FeedbackSignal action (§1, §3).
///
/// The caller verifies from this record, not from the entry payload:
///   - the returned action hash equals the requested one,
///   - the entry is the public `FeedbackSignal` variant,
///   - the entry hash matches the action's entry hash,
///   - entry BYTES are present (a remote fetch may answer `Hidden`/`NotStored`;
///     a signed header alone is never evidence),
///   - the author is `record.action().author()` — `signer_pubkey` in the entry
///     is coordinator-derived, not integrity-bound to the action author.
///
/// `Ok(None)` = not retrievable from THIS peer's DHT view right now. That is a
/// PENDING condition, never proof of absence.
#[hdk_extern]
pub fn get_feedback_signal_record(action_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(action_hash, GetOptions::default())
}

// ---------------------------------------------------------------------------
// Index publication — link-only, act-derived (contract §3)
// ---------------------------------------------------------------------------

/// Input for [`publish_feedback_signal_target_link`].
///
/// There is deliberately NO caller-supplied target. The index this extern
/// publishes is derived from the act's own `target_cid`, so the extern can
/// restore an index but can never forge one: no caller can point somebody
/// else's act at a target that act never named.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct PublishFeedbackSignalTargetLinkInput {
    pub feedback_action_hash: ActionHash,
}

/// Result of [`publish_feedback_signal_target_link`].
///
/// `already_published` is the idempotent answer, not a failure: the index is a
/// set, and a second publication of the same edge would only inflate the
/// `duplicate_links` counter that §3's honest reference query reports.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct PublishFeedbackSignalTargetLinkOutput {
    pub feedback_action_hash: ActionHash,
    pub target_action_hash: ActionHash,
    /// The `CreateLink` action, or `None` when an equivalent edge already stood.
    pub link_action_hash: Option<ActionHash>,
    pub already_published: bool,
}

/// Publish the `TargetToFeedbackSignal` index edge for an act that already
/// exists — link only, no entry (contract §3).
///
/// An act's identity is `(origin DNA hash, action hash)`; the target link is an
/// unordered INDEX that makes the act discoverable by enumeration from the
/// record it names. The two are separable, and the substrate has to be able to
/// say so out loud for three reasons:
///
///   1. **Index repair.** Delete-link validation is permissive
///      (`content_store_integrity/src/lib.rs`), so an index edge can be removed
///      by anyone. An act whose edge is gone is still valid, still fetchable by
///      reference, and invisible to every peer that does not already hold the
///      reference. Any peer holding a verified reference can restore the edge —
///      it is derived from the act, so restoring it is not a claim.
///   2. **Deferred publication.** `create_feedback_signal { defer_target_link }`
///      commits the act without its index; this extern completes it later. That
///      makes "a late link is picked up on a later tick" (§3) an exercisable
///      path, which is what station 1 of the accountable-correction feature
///      measures — arrival order must never become order of effect.
///   3. **Bounded cost.** Publishing an index is one `get_links` plus at most
///      one `create_link`; it never re-creates the act, never re-runs
///      admission, and never touches standing.
///
/// Refusals: the referenced action must resolve to a `FeedbackSignal` entry,
/// and that entry's `target_cid` must parse as an `ActionHash`. Neither is
/// retryable — both mean the caller named something that is not an act.
#[hdk_extern]
pub fn publish_feedback_signal_target_link(
    input: PublishFeedbackSignalTargetLinkInput,
) -> ExternResult<PublishFeedbackSignalTargetLinkOutput> {
    let record = must_get_valid_record(input.feedback_action_hash.clone())?;
    let signal: FeedbackSignal = record
        .entry()
        .to_app_option()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "feedback act decode failed: {e}"
            )))
        })?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "feedback_action_hash does not point to a FeedbackSignal".to_string()
            ))
        })?;

    // The target comes from the ACT, never from the caller.
    let target_action_hash = ActionHash::try_from(signal.target_cid.clone()).map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "feedback act target_cid '{}' does not parse as an ActionHash",
            signal.target_cid
        )))
    })?;

    // Idempotent: the index is a set.
    let query = LinkQuery::try_new(
        target_action_hash.clone(),
        LinkTypes::TargetToFeedbackSignal,
    )?;
    let already_published = get_links(query, GetStrategy::default())?
        .into_iter()
        .filter_map(|l| ActionHash::try_from(l.target.clone()).ok())
        .any(|ah| ah == input.feedback_action_hash);

    if already_published {
        return Ok(PublishFeedbackSignalTargetLinkOutput {
            feedback_action_hash: input.feedback_action_hash,
            target_action_hash,
            link_action_hash: None,
            already_published: true,
        });
    }

    let link_action_hash = create_link(
        target_action_hash.clone(),
        input.feedback_action_hash.clone(),
        LinkTypes::TargetToFeedbackSignal,
        (),
    )?;

    Ok(PublishFeedbackSignalTargetLinkOutput {
        feedback_action_hash: input.feedback_action_hash,
        target_action_hash,
        link_action_hash: Some(link_action_hash),
        already_published: false,
    })
}

// ---------------------------------------------------------------------------
// Tests — pure admission helpers (the externs need a conductor: sweettests)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod correction_admission_tests {
    use super::*;

    fn metadata(op: &str, target: &str, kind: &str, impact: &str) -> String {
        format!(
            r#"{{"correctionRequest":{{"operationId":"{op}","targetActionHash":"{target}",
               "signalKind":"{kind}","standingImpact":"{impact}"}}}}"#
        )
    }

    #[test]
    fn well_formed_request_parses_and_matches() {
        let md = metadata("op-1", "uhCkkTARGET", "correction", "debit-soft");
        let req = parse_correction_request(&md).expect("parses");
        assert_eq!(req.operation_id, "op-1");
        check_correction_request(&req, "uhCkkTARGET", "correction", "debit-soft").expect("matches");
    }

    #[test]
    fn missing_request_object_is_refused() {
        let err = parse_correction_request("{}").unwrap_err();
        assert!(err.contains("correctionRequest"), "got: {err}");
    }

    #[test]
    fn empty_operation_id_is_refused() {
        let md = metadata("", "uhCkkTARGET", "correction", "debit-soft");
        let err = parse_correction_request(&md).unwrap_err();
        assert!(err.contains("operationId"), "got: {err}");
    }

    #[test]
    fn non_json_metadata_is_refused() {
        let err = parse_correction_request("not json").unwrap_err();
        assert!(err.contains("valid JSON"), "got: {err}");
    }

    /// §7: an act whose fields do not match the immutable request is a
    /// NON-MEMBER, refused — not silently collapsed into the group.
    #[test]
    fn mismatched_target_kind_or_impact_is_refused() {
        let md = metadata("op-1", "uhCkkTARGET", "correction", "debit-soft");
        let req = parse_correction_request(&md).unwrap();
        assert!(
            check_correction_request(&req, "uhCkkOTHER", "correction", "debit-soft").is_err(),
            "target mismatch must be refused"
        );
        assert!(
            check_correction_request(&req, "uhCkkTARGET", "squelch", "debit-soft").is_err(),
            "signal_kind mismatch must be refused"
        );
        assert!(
            check_correction_request(&req, "uhCkkTARGET", "correction", "debit-firm").is_err(),
            "standing_impact mismatch must be refused"
        );
    }

    #[test]
    fn public_reach_whitelist_is_exactly_public_and_commons() {
        assert!(PUBLIC_EVIDENCE_REACH.contains(&"public"));
        assert!(PUBLIC_EVIDENCE_REACH.contains(&"commons"));
        assert!(!PUBLIC_EVIDENCE_REACH.contains(&"private"));
        assert!(!PUBLIC_EVIDENCE_REACH.contains(&"community"));
    }
}
