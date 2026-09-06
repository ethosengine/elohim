//! Accountable correction — coordinator externs (slice 1).
//!
//! Implements the coordinator-only surface the accountable-correction contract
//! (`genesis/docs/superpowers/specs/2026-09-06-accountable-correction-contract.md`,
//! revision 3) requires. **Coordinator only**: nothing here touches an integrity
//! zome, so the DNA hash does not move and the change heals through the
//! `update_coordinators` hot-swap path.
//!
//! Three externs land here:
//!
//! - [`get_content_lineage`] (§6) — the verified EXACT root Create identity for a
//!   referenced action, every candidate `(action, predecessor, author, timestamp)`
//!   filtered to that exact root and Content id, repeated links deduplicated, and
//!   an explicit per-candidate fetch outcome. `gather_content_chain` (lib.rs) is
//!   private, id-scoped, silently skips unavailable records and derives the root
//!   author from `records[0]`, so it is NOT reused as-is.
//!
//! - [`amend_content`] (§5.3) — an author-gated Update written against an
//!   EXPLICIT predecessor. `update_content` has no author gate and reselects the
//!   latest ID link as its predecessor; a correction successor may do neither.
//!
//! - `get_feedback_signal_record` (§3, in `feedback_signal.rs`) — the single-record getter that lets a
//!   discovering peer fetch only the acts it has not applied, and verify §1
//!   (action hash, entry type, entry-hash binding, entry bytes present, author
//!   from the SIGNED action) from the record itself.
//!
//! The shared total order for "newest authored version wins" lives here too
//! ([`pick_authored_head`]): §6 requires the ordinary resolver's bare
//! `max_by_key(timestamp)` and the declaration path to share ONE tiebreak, so
//! equal timestamps stop resolving by link-enumeration order.

use content_store_integrity::{Content, EntryTypes, LinkTypes, StringAnchor};
use hdk::prelude::*;
use holochain_serialized_bytes::prelude::SerializedBytes;
use lamad_types::ContentOutput;
use seam_contracts::election::{select_arbitrated_winner, ArbitrationKey};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Bounds. Every walk this module performs is a SEQUENTIAL `get` per link, so
// each bound is a conductor-round-trip budget, not a style preference.
// ---------------------------------------------------------------------------

/// Maximum Update-chain depth walked when resolving an action to its root
/// Create. A chain deeper than this is refused rather than walked forever: a
/// cyclic or adversarial lineage must not turn one zome call into an unbounded
/// sequence of network `get`s.
pub const MAX_LINEAGE_DEPTH: usize = 64;

/// Maximum distinct `IdToContent` link targets classified by
/// [`get_content_lineage`] in one call. Exceeding it sets `truncated` on the
/// output — a partial answer that SAYS it is partial, never a silent one.
pub const MAX_LINEAGE_CANDIDATES: usize = 64;

// Per-candidate fetch outcomes. Strings, not an enum, so a storage consumer
// built against an older coordinator sees an unfamiliar outcome as data rather
// than a decode failure.
pub const OUTCOME_FETCHED: &str = "fetched";
pub const OUTCOME_NOT_FOUND: &str = "not-found";
pub const OUTCOME_NO_ENTRY: &str = "no-entry";
pub const OUTCOME_WRONG_CONTENT_ID: &str = "wrong-content-id";
pub const OUTCOME_OTHER_ROOT: &str = "other-root";
pub const OUTCOME_ROOT_UNRESOLVABLE: &str = "root-unresolvable";
pub const OUTCOME_REFERENCED: &str = "referenced";
pub const OUTCOME_WRONG_TYPE: &str = "wrong-type";

// ---------------------------------------------------------------------------
// Root-Create resolution
// ---------------------------------------------------------------------------

/// Walk an action's Update lineage down to its root Create and return that
/// root's RECORD (not just its author — the caller needs the root's identity to
/// separate two roots that share an id AND an author).
///
/// `Ok(None)` when a link in the chain is not retrievable from this peer's DHT
/// view; that is a PENDING condition for the caller, never a rejection.
pub(crate) fn resolve_root_create(
    start: ActionHash,
    strategy: GetStrategy,
) -> ExternResult<Option<Record>> {
    let mut action_hash = start;
    for _ in 0..MAX_LINEAGE_DEPTH {
        let record = match get(action_hash.clone(), GetOptions::from(strategy))? {
            Some(r) => r,
            None => return Ok(None),
        };
        match &record.action().data {
            ActionData::Update(update) => action_hash = update.original_action_address.clone(),
            _ => return Ok(Some(record)),
        }
    }
    Err(wasm_error!(WasmErrorInner::Guest(format!(
        "resolve_root_create: update lineage exceeded {MAX_LINEAGE_DEPTH} links — refusing an \
         unbounded walk"
    ))))
}

/// The predecessor an action names, when it is an Update.
/// `Update.original_action_address` IS the Supersedes edge.
fn predecessor_of(record: &Record) -> Option<ActionHash> {
    match &record.action().data {
        ActionData::Update(update) => Some(update.original_action_address.clone()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The shared total order (§6)
// ---------------------------------------------------------------------------

/// Pick the newest ROOT-AUTHORED record from a candidate set, on a TOTAL order:
/// action timestamp first, action hash as the tiebreak.
///
/// The tiebreak is the point. The ordinary resolver's bare
/// `max_by_key(|r| r.action().timestamp())` is enumeration-order dependent on
/// equal timestamps, so two peers holding the same two versions could serve
/// different heads. `select_arbitrated_winner` makes the tiebreak
/// non-optional by construction (there is no `ArbitrationKey` without one), and
/// this function is the ONE place the version-DAG order is expressed — the read
/// resolver, the declaration path and [`get_content_lineage`] all call it.
///
/// `tier` is `()`: a version-DAG pick has no authority tier (that is the
/// CROSS-ROOT canonical election's job, which keeps its own two-tier key).
pub(crate) fn pick_authored_head<'a>(
    records: &'a [Record],
    root_author: &AgentPubKey,
) -> Option<&'a Record> {
    let authored: Vec<&Record> = records
        .iter()
        .filter(|r| r.action().author() == root_author)
        .collect();
    select_arbitrated_winner(authored, |r| ArbitrationKey {
        tier: (),
        clock: r.action().timestamp(),
        tiebreak: r.action_hashed().hash.clone(),
    })
}

// ---------------------------------------------------------------------------
// get_content_lineage
// ---------------------------------------------------------------------------

/// Input for [`get_content_lineage`].
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct GetContentLineageInput {
    /// The REFERENCED action — typically a FeedbackSignal's target. The lineage
    /// is scoped to THIS action's exact root Create, never to "the id's chain".
    pub action_hash: ActionHash,
    /// Read with `GetStrategy::Local` instead of `Network`.
    ///
    /// The default is `Network`: this extern backs an AUTHOR GATE (acceptance
    /// verification, amendment admission), and answering a gate from a cold
    /// local view rejects legitimate authors. A caller that is polling on an
    /// interval and must not stall sets this true and treats an unfetchable
    /// dependency as PENDING (contract §6), which is exactly what the storage
    /// projector does.
    #[serde(default)]
    pub local: bool,
}

/// One version-DAG candidate with an explicit fetch outcome.
///
/// `author`/`timestamp`/`predecessor` are `None` when `fetch_outcome` is not
/// [`OUTCOME_FETCHED`] — an unfetchable candidate is REPORTED, never silently
/// skipped the way `gather_content_chain` skips it.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct LineageCandidate {
    pub action_hash: ActionHash,
    /// `Update.original_action_address`; `None` for the root Create.
    pub predecessor: Option<ActionHash>,
    pub author: Option<AgentPubKey>,
    pub timestamp: Option<Timestamp>,
    pub fetch_outcome: String,
    /// True only when this candidate resolves to the SAME exact root Create as
    /// the referenced action AND carries the same Content id.
    pub in_root: bool,
}

/// Verified lineage of ONE referenced action.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct ContentLineageOutput {
    pub referenced_action_hash: ActionHash,
    /// The EXACT root Create of the referenced action. Two roots that share a
    /// Content id — and even a root author — are separated by this hash.
    pub root_action_hash: ActionHash,
    /// Author of that root Create. The ONLY agent slice 1 admits as an
    /// acceptance signer or an amendment author.
    pub root_author: AgentPubKey,
    pub content_id: String,
    pub candidates: Vec<LineageCandidate>,
    /// Deterministic pick over in-root, root-authored candidates
    /// ([`pick_authored_head`]). Not a claim of global completeness.
    pub head_action_hash: Option<ActionHash>,
    /// Two in-root candidates name the SAME predecessor: an observed fork.
    /// The deterministic pick does not clear it.
    pub contested: bool,
    pub contested_predecessors: Vec<ActionHash>,
    pub link_count: u32,
    pub duplicate_links: u32,
    pub invalid_link_targets: u32,
    pub other_root_candidates: u32,
    pub unfetchable_candidates: u32,
    /// `MAX_LINEAGE_CANDIDATES` was reached: this answer is partial and says so.
    pub truncated: bool,
}

/// Verified exact-root lineage for a referenced action (contract §6).
///
/// Every consumer that confers acceptance or admits an amendment reads the root
/// author from HERE, never from `Content.author_id` (a string), the current
/// head's author, or `records[0]` of the ID-anchor enumeration.
#[hdk_extern]
pub fn get_content_lineage(input: GetContentLineageInput) -> ExternResult<ContentLineageOutput> {
    let strategy = if input.local {
        GetStrategy::Local
    } else {
        GetStrategy::Network
    };

    // 1. The referenced action itself must be retrievable AND carry a Content
    //    entry. A signed header alone is never enough (§1).
    let referenced = get(input.action_hash.clone(), GetOptions::from(strategy))?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "get_content_lineage: referenced action {:?} is not retrievable — PENDING, not a \
             rejection",
            input.action_hash
        )))
    })?;
    let referenced_content: Content = referenced
        .entry()
        .to_app_option()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "get_content_lineage: decode referenced entry: {e}"
            )))
        })?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "get_content_lineage: referenced action carries no Content entry".to_string()
            ))
        })?;
    let content_id = referenced_content.id.clone();

    // 2. Walk to the EXACT root Create.
    let root = resolve_root_create(input.action_hash.clone(), strategy)?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "get_content_lineage: root Create not retrievable — PENDING, not a rejection"
                .to_string()
        ))
    })?;
    let root_action_hash = root.action_hashed().hash.clone();
    let root_author = root.action().author().clone();
    let root_content: Content = root
        .entry()
        .to_app_option()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "get_content_lineage: decode root entry: {e}"
            )))
        })?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "get_content_lineage: root Create carries no Content entry".to_string()
            ))
        })?;
    if root_content.id != content_id {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "get_content_lineage: root Create names Content id '{}' but the referenced action \
             names '{}' — refusing a mislabeled lineage",
            root_content.id, content_id
        ))));
    }

    // 3. Enumerate the id anchor's links, DEDUPLICATED. Repeated links are
    //    normal (every update writes a fresh IdToContent link and old links
    //    stay), and counting one action twice would fabricate a fork.
    let anchor = StringAnchor::new("content_id", &content_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
    let links = get_links(query, strategy)?;
    let link_count = links.len() as u32;

    let mut seen: Vec<ActionHash> = Vec::new();
    let mut duplicate_links: u32 = 0;
    let mut invalid_link_targets: u32 = 0;
    let mut truncated = false;
    for link in &links {
        let ah = match ActionHash::try_from(link.target.clone()) {
            Ok(h) => h,
            Err(_) => {
                invalid_link_targets += 1;
                continue;
            }
        };
        if seen.contains(&ah) {
            duplicate_links += 1;
            continue;
        }
        if seen.len() >= MAX_LINEAGE_CANDIDATES {
            truncated = true;
            break;
        }
        seen.push(ah);
    }

    // 4. Classify each distinct candidate with an EXPLICIT outcome.
    let mut candidates: Vec<LineageCandidate> = Vec::with_capacity(seen.len());
    let mut other_root_candidates: u32 = 0;
    let mut unfetchable_candidates: u32 = 0;
    let mut in_root_records: Vec<Record> = Vec::new();
    for ah in seen {
        let record = match get(ah.clone(), GetOptions::from(strategy))? {
            Some(r) => r,
            None => {
                unfetchable_candidates += 1;
                candidates.push(LineageCandidate {
                    action_hash: ah,
                    predecessor: None,
                    author: None,
                    timestamp: None,
                    fetch_outcome: OUTCOME_NOT_FOUND.to_string(),
                    in_root: false,
                });
                continue;
            }
        };
        let decoded: Option<Content> = record.entry().to_app_option().ok().flatten();
        let Some(candidate_content) = decoded else {
            candidates.push(LineageCandidate {
                action_hash: ah,
                predecessor: predecessor_of(&record),
                author: Some(record.action().author().clone()),
                timestamp: Some(record.action().timestamp()),
                fetch_outcome: OUTCOME_NO_ENTRY.to_string(),
                in_root: false,
            });
            continue;
        };
        if candidate_content.id != content_id {
            candidates.push(LineageCandidate {
                action_hash: ah,
                predecessor: predecessor_of(&record),
                author: Some(record.action().author().clone()),
                timestamp: Some(record.action().timestamp()),
                fetch_outcome: OUTCOME_WRONG_CONTENT_ID.to_string(),
                in_root: false,
            });
            continue;
        }
        let candidate_root = resolve_root_create(ah.clone(), strategy)?;
        let outcome = match candidate_root {
            None => {
                unfetchable_candidates += 1;
                OUTCOME_ROOT_UNRESOLVABLE
            }
            Some(r) if r.action_hashed().hash != root_action_hash => {
                other_root_candidates += 1;
                OUTCOME_OTHER_ROOT
            }
            Some(_) => OUTCOME_FETCHED,
        };
        let in_root = outcome == OUTCOME_FETCHED;
        candidates.push(LineageCandidate {
            action_hash: ah,
            predecessor: predecessor_of(&record),
            author: Some(record.action().author().clone()),
            timestamp: Some(record.action().timestamp()),
            fetch_outcome: outcome.to_string(),
            in_root,
        });
        if in_root {
            in_root_records.push(record);
        }
    }

    // 5. Deterministic head pick + observed-fork marker.
    let head_action_hash =
        pick_authored_head(&in_root_records, &root_author).map(|r| r.action_hashed().hash.clone());
    let contested = contested_predecessors(&candidates, &root_author);

    Ok(ContentLineageOutput {
        referenced_action_hash: input.action_hash,
        root_action_hash,
        root_author,
        content_id,
        candidates,
        head_action_hash,
        contested: !contested.is_empty(),
        contested_predecessors: contested,
        link_count,
        duplicate_links,
        invalid_link_targets,
        other_root_candidates,
        unfetchable_candidates,
        truncated,
    })
}

/// **Pure.** Predecessors named by TWO OR MORE in-root, root-authored
/// candidates — an observed fork. Sequential amendments (each naming the prior
/// head) never collide here; two siblings on one predecessor always do.
fn contested_predecessors(
    candidates: &[LineageCandidate],
    root_author: &AgentPubKey,
) -> Vec<ActionHash> {
    let mut counted: Vec<(ActionHash, u32)> = Vec::new();
    for c in candidates {
        if !c.in_root {
            continue;
        }
        if c.author.as_ref() != Some(root_author) {
            continue;
        }
        let Some(pred) = c.predecessor.as_ref() else {
            continue;
        };
        match counted.iter_mut().find(|(p, _)| p == pred) {
            Some((_, n)) => *n += 1,
            None => counted.push((pred.clone(), 1)),
        }
    }
    counted
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(p, _)| p)
        .collect()
}

// ---------------------------------------------------------------------------
// amend_content
// ---------------------------------------------------------------------------

/// The patch half of [`AmendContentInput`]. Mirrors `UpdateContentInput`'s
/// field set MINUS `id` — an amendment names its predecessor by ACTION, and the
/// Content id comes from that predecessor, never from the caller.
#[derive(Debug, Clone, Default, Serialize, Deserialize, SerializedBytes)]
pub struct AmendContentPatch {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default)]
    pub blob_cid: Option<String>,
    #[serde(default)]
    pub content_size_bytes: Option<u64>,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub reach: Option<String>,
}

/// Input for [`amend_content`].
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct AmendContentInput {
    /// The EXPLICIT predecessor this amendment supersedes. Not "the latest ID
    /// link" — a correction successor must name what it corrects.
    pub predecessor_action_hash: ActionHash,
    pub content: AmendContentPatch,
}

/// Publish an amended version of a content record as an Update against an
/// EXPLICIT predecessor, gated on the caller being that predecessor's EXACT
/// root-Create author (contract §5.3).
///
/// Three differences from `update_content`, all load-bearing:
///
/// 1. **Author gate.** `update_content` has none: any agent may write an Update
///    on any content id. An amendment is a successor to a correction, so it is
///    admitted only from the root author.
/// 2. **Explicit predecessor.** `update_content` reselects the newest
///    `IdToContent` link, so a concurrent write silently changes what an
///    amendment supersedes. Here the caller names the action.
/// 3. **Served-head admission.** If a CROSS-ROOT canonical answer already
///    stands for this id, the ordinary resolver prefers it, so this Update could
///    never become the served head. That is refused up front rather than
///    silently accepted (contract §5.3).
///
/// Refusal text keeps the literal substring `"not the author"`, which the
/// storage facade already classifies to 403.
#[hdk_extern]
pub fn amend_content(input: AmendContentInput) -> ExternResult<ContentOutput> {
    // WRITE PATH → Network. This is an author gate; a cold local view would
    // reject a legitimate author.
    let strategy = GetStrategy::Network;

    let predecessor = get(input.predecessor_action_hash.clone(), GetOptions::from(strategy))?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "amend_content: predecessor {:?} is not retrievable",
                input.predecessor_action_hash
            )))
        })?;
    let mut content: Content = predecessor
        .entry()
        .to_app_option()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "amend_content: decode predecessor Content entry: {e}"
            )))
        })?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "amend_content: predecessor carries no Content entry".to_string()
            ))
        })?;

    // Root Create identity — the ONLY source of the amendment authority.
    let root = resolve_root_create(input.predecessor_action_hash.clone(), strategy)?.ok_or_else(
        || {
            wasm_error!(WasmErrorInner::Guest(
                "amend_content: root Create of the predecessor is not retrievable".to_string()
            ))
        },
    )?;
    let root_author = root.action().author().clone();
    let root_content: Content = root
        .entry()
        .to_app_option()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "amend_content: decode root Content entry: {e}"
            )))
        })?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "amend_content: root Create carries no Content entry".to_string()
            ))
        })?;
    // Predecessor's Content id verified against the root's (§5.3).
    if root_content.id != content.id {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "amend_content: predecessor names Content id '{}' but its root Create names '{}' — \
             refusing a cross-id amendment",
            content.id, root_content.id
        ))));
    }

    let me = agent_info()?.agent_initial_pubkey;
    if me != root_author {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "amend_content: agent {me:?} is not the author of content '{}' (root author \
             {root_author:?}); slice 1 admits the root author only — delegate-authored \
             successors are a named gap",
            content.id
        ))));
    }

    // Served-head admission precondition (§5.3).
    if let Some(canonical) = crate::gather_canonical_head_record(&content.id, strategy)? {
        let canonical_hash = canonical.record.action_hashed().hash.clone();
        let canonical_root = resolve_root_create(canonical_hash, strategy)?;
        let canonical_is_cross_root = match canonical_root {
            // Unresolvable canonical root: we cannot prove it is the same root,
            // so we refuse rather than write an amendment that may be unservable.
            None => true,
            Some(r) => r.action_hashed().hash != root.action_hashed().hash,
        };
        if canonical_is_cross_root {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "amend_content: a cross-root canonical head already stands for content '{}' — \
                 this amendment cannot become the served head, refusing rather than silently \
                 accepting it (contract §5.3)",
                content.id
            ))));
        }
    }

    // Apply the patch. `None` preserves the existing value, exactly as
    // `update_content` does.
    if let Some(v) = input.content.blob_cid {
        content.blob_cid = Some(v);
    }
    if let Some(v) = input.content.content_size_bytes {
        content.content_size_bytes = Some(v);
    }
    if let Some(v) = input.content.content_hash {
        content.content_hash = Some(v);
    }
    if let Some(v) = input.content.title {
        content.title = v;
    }
    if let Some(v) = input.content.description {
        content.description = v;
    }
    if let Some(v) = input.content.content {
        content.content = v;
    }
    if let Some(v) = input.content.metadata_json {
        content.metadata_json = v;
    }
    if let Some(v) = input.content.reach {
        content.reach = v;
    }
    content.updated_at = format!("{:?}", sys_time()?);

    let content = crate::healing_integration::prepare_content_for_storage(content)?;

    // Write against the NAMED predecessor.
    let new_action_hash = update_entry(
        input.predecessor_action_hash,
        &EntryTypes::Content(content.clone()),
    )?;
    let entry_hash = hash_entry(&EntryTypes::Content(content.clone()))?;
    crate::create_id_to_content_link(&content.id, &new_action_hash)?;

    Ok(ContentOutput {
        action_hash: new_action_hash,
        entry_hash,
        content: crate::content_to_wire(&content),
    })
}

// ---------------------------------------------------------------------------
// Tests (pure helpers only — the externs need a conductor, see the sweettests)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod correction_order_tests {
    use super::*;

    fn ah(seed: u8) -> ActionHash {
        ActionHash::from_raw_36(vec![seed; 36])
    }

    fn agent(seed: u8) -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![seed; 36])
    }

    fn candidate(
        action: u8,
        pred: Option<u8>,
        author: &AgentPubKey,
        in_root: bool,
    ) -> LineageCandidate {
        LineageCandidate {
            action_hash: ah(action),
            predecessor: pred.map(ah),
            author: Some(author.clone()),
            timestamp: Some(Timestamp::from_micros(0)),
            fetch_outcome: OUTCOME_FETCHED.to_string(),
            in_root,
        }
    }

    #[test]
    fn sequential_amendments_are_not_contested() {
        let a = agent(1);
        // root(1) <- v2(2) <- v3(3): each names the prior head.
        let cands = vec![
            candidate(1, None, &a, true),
            candidate(2, Some(1), &a, true),
            candidate(3, Some(2), &a, true),
        ];
        assert!(contested_predecessors(&cands, &a).is_empty());
    }

    #[test]
    fn two_updates_on_one_predecessor_are_contested() {
        let a = agent(1);
        let cands = vec![
            candidate(1, None, &a, true),
            candidate(2, Some(1), &a, true),
            candidate(3, Some(1), &a, true),
        ];
        let contested = contested_predecessors(&cands, &a);
        assert_eq!(contested, vec![ah(1)]);
    }

    #[test]
    fn out_of_root_and_foreign_author_candidates_never_contest() {
        let a = agent(1);
        let b = agent(2);
        let cands = vec![
            candidate(2, Some(1), &a, true),
            // same predecessor but a DIFFERENT root — excluded by `in_root`.
            candidate(3, Some(1), &a, false),
            // same predecessor, in-root, but not the root author.
            candidate(4, Some(1), &b, true),
        ];
        assert!(contested_predecessors(&cands, &a).is_empty());
    }

    /// The tiebreak is the whole point of `pick_authored_head`: two versions
    /// with the SAME timestamp must resolve identically on every peer,
    /// regardless of the order `get_links` handed them over.
    #[test]
    fn equal_timestamps_break_on_action_hash_not_enumeration_order() {
        // Pure re-expression of the order the helper delegates to, exercised
        // without a conductor (a `Record` cannot be built in a unit test).
        let key = |t: i64, h: ActionHash| ArbitrationKey {
            tier: (),
            clock: Timestamp::from_micros(t),
            tiebreak: h,
        };
        let forward = vec![key(10, ah(1)), key(10, ah(9))];
        let reverse = vec![key(10, ah(9)), key(10, ah(1))];
        let w1 = select_arbitrated_winner(forward, |k| ArbitrationKey {
            tier: k.tier,
            clock: k.clock,
            tiebreak: k.tiebreak.clone(),
        })
        .unwrap();
        let w2 = select_arbitrated_winner(reverse, |k| ArbitrationKey {
            tier: k.tier,
            clock: k.clock,
            tiebreak: k.tiebreak.clone(),
        })
        .unwrap();
        assert_eq!(w1.tiebreak, ah(9));
        assert_eq!(w2.tiebreak, ah(9));
    }

    #[test]
    fn newer_timestamp_beats_a_higher_action_hash() {
        let key = |t: i64, h: ActionHash| ArbitrationKey {
            tier: (),
            clock: Timestamp::from_micros(t),
            tiebreak: h,
        };
        let winner = select_arbitrated_winner(vec![key(10, ah(9)), key(20, ah(1))], |k| {
            ArbitrationKey {
                tier: k.tier,
                clock: k.clock,
                tiebreak: k.tiebreak.clone(),
            }
        })
        .unwrap();
        assert_eq!(winner.clock, Timestamp::from_micros(20));
    }
}
