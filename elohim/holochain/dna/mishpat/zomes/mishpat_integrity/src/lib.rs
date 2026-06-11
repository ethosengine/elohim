use hdi::prelude::*;

// ============================================================
// ENTRY TYPES — Formal Governance (Mishpat DNA)
// Separated from Lamad DNA to give governance its own
// validation rules and free Lamad entry type capacity.
// ============================================================

// =============================================================================
// Governance Entry Types
// =============================================================================

/// Precedent - A binding decision that guides future decisions.
///
/// Precedents form the case law of the governance system.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Precedent {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub full_reasoning: String,
    pub binding: String, // constitutional, binding-network, binding-local, persuasive
    pub scope_json: String, // { entityTypes, categories, roles } as JSON
    pub citations: u32,  // How often this precedent is cited
    pub status: String,  // active, superseded, under-review
    pub established_by: String, // Proposal ID or governance body
    pub established_at: String,
    pub superseded_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Precedent binding levels
pub const PRECEDENT_BINDING: [&str; 4] = [
    "constitutional",
    "binding-network",
    "binding-local",
    "persuasive",
];

/// Discussion - A threaded discussion on an entity.
///
/// Enables structured deliberation on content, proposals, or challenges.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Discussion {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub category: String, // general, proposal, challenge, feedback
    pub title: String,
    pub messages_json: String, // DiscussionMessage[] as JSON
    pub status: String,        // open, closed, archived
    pub message_count: u32,
    pub last_activity_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Discussion categories
pub const DISCUSSION_CATEGORIES: [&str; 4] = ["general", "proposal", "challenge", "feedback"];

/// GovernanceState - Current governance status of an entity.
///
/// Tracks the governance posture of content, paths, etc.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GovernanceState {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub status: String,                 // approved, pending, challenged, suspended
    pub status_basis_json: String,      // StatusBasis as JSON
    pub labels_json: String,            // Label[] as JSON
    pub active_challenges_json: String, // String[] as JSON
    pub active_proposals_json: String,  // String[] as JSON
    pub precedent_ids_json: String,     // String[] as JSON
    pub last_updated: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Governance status states
pub const GOVERNANCE_STATUS: [&str; 4] = ["approved", "pending", "challenged", "suspended"];

// =============================================================================
// Qahal: Governance Signals - Contextual Feedback & Consensus
// =============================================================================
//
// Governance signals enable constitutional feedback mechanisms:
// - Low friction: Emotional reactions (moved, grateful, challenged, concerned)
// - Medium friction: Graduated feedback (Loomio-style scales)
// - High friction: Formal proposals (binding decisions)
//
// Inspired by:
// - Loomio: 4-position voting (Agree/Abstain/Disagree/Block)
// - Forby: ARCH intensity-based voting
// - Polis: 2D opinion clustering and consensus discovery

/// GraduatedFeedback - Medium friction scaled feedback (Loomio/Forby style).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GraduatedFeedback {
    pub id: String,
    pub content_id: String,
    pub content_type: String,
    pub responder_id: String,
    pub feedback_context: String,
    pub position: i8,
    pub intensity: u8,
    pub reasoning: Option<String>,
    pub updated_count: u32,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

pub const FEEDBACK_CONTEXTS: [&str; 5] =
    ["accuracy", "usefulness", "proposal", "clarity", "relevance"];

/// OpinionStatement - Polis-style statement for clustering.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct OpinionStatement {
    pub id: String,
    pub context_id: String,
    pub author_id: String,
    pub text: String,
    pub status: String,
    pub vote_count: u32,
    pub agree_count: u32,
    pub disagree_count: u32,
    pub pass_count: u32,
    pub consensus_score: i32,
    pub cluster_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

// =============================================================================
// Spatial Governance — Place (governed spatial entity)
// =============================================================================
//
// "This is our watershed." Communities witness and govern spatial boundaries.
// If centralized, someone becomes the boundary authority — the land registrar.
// Place is notarized because boundary-drawing IS governance.

/// Place - A named, governed spatial entity.
///
/// Cities, watersheds, land parcels, solar farms, gathering spaces.
/// Notarized on Mishpat DNA because the community must witness
/// "these are our boundaries" — if centralized, someone draws
/// the lines and everyone else lives inside them.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Place {
    pub id: String,
    pub name: String,
    pub place_type: String,           // PlaceType enum value
    pub constitutional_layer: String, // ConstitutionalLayer this place maps to
    /// Primary H3 cell at canonical resolution for this place
    pub h3_index: String,
    pub h3_resolution: u8,
    /// GeoJSON geometry (boundary polygon, point, or multipolygon)
    pub geometry_json: String,
    /// Centroid for quick spatial lookups
    pub centroid_lat: f64,
    pub centroid_lng: f64,
    /// Parent place ID (nesting: parcel → community → bioregion → global)
    pub parent_place_id: Option<String>,
    /// OpenStreetMap reference (OsmReference as JSON)
    pub osm_reference_json: Option<String>,
    /// Carrying capacity constraints (CarryingCapacity[] as JSON)
    pub carrying_capacity_json: String,
    /// Governance collective with authority over this place
    pub governing_collective_id: Option<String>,
    pub status: String, // active, proposed, disputed, dissolved
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

pub const PLACE_TYPES: [&str; 8] = [
    "administrative",
    "bioregional",
    "parcel",
    "infrastructure",
    "gathering",
    "watershed",
    "agricultural",
    "custom",
];

pub const PLACE_STATUS: [&str; 4] = ["active", "proposed", "disputed", "dissolved"];

// =============================================================================
// Challenge Outcome — Verdict + indemnification after review
// =============================================================================
//
// Notarized on Mishpat DHT because the outcome closes the accountability loop:
// the community's verdict must be as public and immutable as the original decision.
// An upheld challenge that only exists in a private database is theater, not
// accountability.
//
// The indemnification actions are co-committed with the verdict — there is no
// window where verdict exists but action is "pending".

/// ChallengeOutcome — Verdict and indemnification action after challenge review.
///
/// Closes a GateDecisionChallenge by recording the reviewer consensus verdict.
/// When verdict is "upheld", indemnification_actions_json MUST be a non-empty
/// JSON array — accountability without consequence is theater.
///
/// Ref: gate-challenge-and-indemnification-design.md §3.2, §4.3
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ChallengeOutcome {
    /// CID of this outcome (self-addressing; content-derived from fields).
    pub outcome_id: String,
    /// CID of the GateDecisionChallenge this outcome closes.
    pub challenge_cid: String,
    /// Verdict: upheld | dismissed | superseded
    pub verdict: String,
    /// AgentPubKeys of the reviewers who reached consensus (comma-separated base64 keys).
    pub reviewer_consensus: String,
    /// Full ConstitutionalReasoning as JSON (same structure as GateDecisionAttestation.reasoning_json).
    pub reasoning_json: String,
    /// ISO-8601 timestamp when the verdict was decided.
    pub decided_at: String,
    /// Indemnification actions as JSON array (empty array "[]" if no action required;
    /// MUST be non-empty when verdict = "upheld").
    /// Stored as JSON for flexibility — Phase 11+ may harden to typed enum.
    /// Shape: [{ "type": "ReputationDegrade", ... }, { "type": "ReparationAttestation", ... }]
    pub indemnification_actions_json: String,
}

/// Valid verdicts for ChallengeOutcome.
pub const CHALLENGE_VERDICTS: [&str; 3] = ["upheld", "dismissed", "superseded"];

// =============================================================================
// Commitment — REA compute delegation primitive (Z.D substrate-correct deploy)
// =============================================================================
//
// A bounded delegation of compute or action authority notarized on the Mishpat
// DHT. The `action` field is the discriminator (e.g., "delegates-compute",
// "acknowledges-reach-change"). `payload_json` carries the action-specific
// schema content validated by the coordinator; integrity performs defense-in-
// depth structural checks only.
//
// Source of truth: this DHT entry type. Operational projection lives in
// elohim-storage's `rea_commitments` SQLite table (post-commit signal).
//
// Ref: genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md §1
//      gospel-tier memory: project_rea_compute_commitment_primitive.md

/// Commitment — REA commitment notarizing a bounded delegation of compute or
/// action authority. Used by the Z.D substrate-correct deploy flow per
/// genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md §1.
///
/// The `action` field is the discriminator (e.g., "delegates-compute",
/// "acknowledges-reach-change"). `payload_json` carries the action-specific
/// schema content (validated by the coordinator before `create_entry`; the
/// integrity layer does defense-in-depth parse checks).
///
/// Source of truth: this DHT entry type. Operational projection lives in
/// elohim-storage's `rea_commitments` SQLite table (post-commit signal).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Commitment {
    pub action: String,
    pub payload_json: String,
    pub signed_at: String,
}

// =============================================================================
// Anchor Entry (for link indexing)
// =============================================================================

/// Generic string anchor for creating deterministic link bases.
///
/// Used by coordinator zome functions to index entries by ID, status, type,
/// entity, etc. without requiring a separate anchor DNA.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StringAnchor {
    pub anchor_type: String,
    pub anchor_value: String,
}

impl StringAnchor {
    pub fn new(anchor_type: &str, anchor_value: &str) -> Self {
        Self {
            anchor_type: anchor_type.to_string(),
            anchor_value: anchor_value.to_string(),
        }
    }
}

// ============================================================
// ENTRY TYPES ENUM
// ============================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Precedent(Precedent),
    Discussion(Discussion),
    GovernanceState(GovernanceState),
    GraduatedFeedback(GraduatedFeedback),
    OpinionStatement(OpinionStatement),
    Place(Place),
    StringAnchor(StringAnchor),
    ChallengeOutcome(ChallengeOutcome), // Stage C: Challenge/Proposal/GovernanceReaction/ProposalVote/StatementVote/GateDecisionAttestation/GateDecisionChallenge moved to elohim DNA
    Commitment(Commitment),             // Z.D substrate-correct deploy: REA compute delegation primitive
}

// ============================================================
// LINK TYPES
// ============================================================

#[hdk_link_types]
pub enum LinkTypes {
    // =========================================================================
    // Qahal: Governance Signal links (Loomio/Forby/Polis patterns)
    // =========================================================================
    // GovernanceReaction links removed (Stage C) — reactions live on elohim DNA
    ContentToFeedback,   // Content -> GraduatedFeedback
    AgentToFeedback,     // Anchor(agent_id) -> GraduatedFeedback
    FeedbackByContext,   // Anchor(feedback_context) -> GraduatedFeedback
    // ProposalVote links removed (Stage C) — votes live on elohim DNA
    ContextToStatements, // Anchor(context_id) -> OpinionStatement
    AgentToStatements,   // Anchor(agent_id) -> OpinionStatement
    // StatementVote links removed (Stage C) — votes live on elohim DNA

    // =========================================================================
    // Qahal: Formal Governance links
    // =========================================================================
    // Challenge links removed (Stage C) — challenges live on elohim DNA
    // Proposal links removed (Stage C) — proposals live on elohim DNA

    // Precedent
    IdToPrecedent,     // Anchor(precedent_id) -> Precedent
    PrecedentByScope,  // Anchor(scope) -> Precedent
    PrecedentByStatus, // Anchor(status) -> Precedent

    // Discussion
    IdToDiscussion,       // Anchor(discussion_id) -> Discussion
    EntityToDiscussion,   // Anchor(entity_type:entity_id) -> Discussion
    DiscussionByCategory, // Anchor(category) -> Discussion
    DiscussionByStatus,   // Anchor(status) -> Discussion

    // GovernanceState
    IdToGovernanceState,     // Anchor(entity_type:entity_id) -> GovernanceState
    GovernanceStateByStatus, // Anchor(status) -> GovernanceState

    // =========================================================================
    // Place — Governed Spatial Entity
    // =========================================================================
    IdToPlace,          // Anchor(place_id) -> Place
    H3CellToPlace,      // Anchor(h3_index) -> Place (THE key spatial query link)
    PlaceByType,        // Anchor(place_type) -> Place
    PlaceByLayer,       // Anchor(constitutional_layer) -> Place
    ParentToChildPlace, // Place -> Place (containment hierarchy)
    PlaceToCollective,  // Place -> Anchor(collective_id)

    // =========================================================================
    // Gate Decision Attestation / Challenge links removed (Stage C)
    // — attestations and challenges live on elohim DNA
    // =========================================================================

    // =========================================================================
    // Challenge Outcome — Verdicts closing GateDecisionChallenges
    // =========================================================================
    IdToOutcome,        // Anchor(outcome_id) -> ChallengeOutcome
    ChallengeToOutcome, // Anchor(challenge_cid) -> ChallengeOutcome
    VerdictToOutcomes,  // Anchor(verdict) -> ChallengeOutcome

    // =========================================================================
    // Commitment lifecycle — CommitmentByState (Slice-2b T11)
    // =========================================================================
    // Records a Commitment's lifecycle transition (proposed → active → …) as an
    // immutable DHT link so peers can verify lifecycle WITHOUT replaying every
    // EconomicEvent. The base is the Commitment's EntryHash (the live anchor);
    // the target is the ActionHash of the event that justifies the transition;
    // the LinkTag carries `state|signed_at` (the new state + the deterministic,
    // caller-supplied signing time). The SQL `state` column becomes a write-
    // through cache: `graduate_to_active` writes the cache, this link is truth.
    CommitmentByState, // EntryHash(Commitment) -> ActionHash(event); tag = state|signed_at
}

// =============================================================================
// Validation
// =============================================================================

#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

/// Validate DHT operations for all entry types
///
/// This validation callback runs on both:
/// - Author's node when creating entries (blocks invalid entries from source chain)
/// - All peers when gossiping entries (blocks invalid entries from DHT)
///
/// Validation must be deterministic - identical outcomes regardless of validator or timing.
/// Reference: https://developer.holochain.org/build/validation/
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_create_entry(&app_entry),
            OpEntry::UpdateEntry { app_entry, .. } => validate_update_entry(&app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::StoreRecord(store_record) => match store_record {
            OpRecord::CreateEntry { app_entry, .. } => validate_create_entry(&app_entry),
            OpRecord::UpdateEntry { app_entry, .. } => validate_update_entry(&app_entry),
            OpRecord::CreateLink { link_type, tag, .. } => validate_create_link(&link_type, &tag),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink { link_type, tag, .. } => validate_create_link(&link_type, &tag),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

/// Validate link creation. Defense-in-depth, deterministic, and HDI-safe
/// (no `get_links` — integrity validators may only call `must_get_*`).
///
/// `CommitmentByState` (Slice-2b T11): the LinkTag carries `state|signed_at`.
/// The DHT records the lifecycle transition; the coordinator already validated
/// the base/target hashes resolve, so integrity confirms only that the tag is
/// well-formed (`<state>|<signed_at>`, both non-empty) — a direct-source-chain
/// bypass that authored an empty/malformed tag is rejected here.
fn validate_create_link(
    link_type: &LinkTypes,
    tag: &LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::CommitmentByState => {
            let raw = String::from_utf8(tag.0.clone()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest(
                    "CommitmentByState tag must be UTF-8".to_string()
                ))
            })?;
            match validate_commitment_by_state_tag(&raw) {
                Ok(()) => Ok(ValidateCallbackResult::Valid),
                Err(msg) => Ok(ValidateCallbackResult::Invalid(msg)),
            }
        }
        // All other link types pass structural validation (coordinator-gated).
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

/// Pure, deterministic check for a `CommitmentByState` LinkTag string.
/// The tag wire shape is `<state>|<signed_at>` — both segments non-empty.
/// Extracted as a free function so it is unit-testable natively (no WASM).
fn validate_commitment_by_state_tag(raw: &str) -> Result<(), String> {
    let mut parts = raw.splitn(2, '|');
    let state = parts.next().unwrap_or("");
    let signed_at = parts.next().unwrap_or("");
    if state.is_empty() {
        return Err("CommitmentByState tag state segment must be non-empty".into());
    }
    if signed_at.is_empty() {
        return Err(
            "CommitmentByState tag must carry '<state>|<signed_at>' with a non-empty signed_at"
                .into(),
        );
    }
    Ok(())
}

fn validate_create_entry(app_entry: &EntryTypes) -> ExternResult<ValidateCallbackResult> {
    match app_entry {
        EntryTypes::Precedent(precedent) => validate_precedent(precedent),
        EntryTypes::Discussion(discussion) => validate_discussion(discussion),
        EntryTypes::GovernanceState(state) => validate_governance_state(state),
        EntryTypes::GraduatedFeedback(feedback) => validate_graduated_feedback(feedback),
        EntryTypes::OpinionStatement(statement) => validate_opinion_statement(statement),
        EntryTypes::Place(place) => validate_place(place),
        EntryTypes::StringAnchor(_) => Ok(ValidateCallbackResult::Valid),
        EntryTypes::ChallengeOutcome(outcome) => validate_challenge_outcome(outcome),
        EntryTypes::Commitment(commitment) => validate_commitment_entry(commitment),
    }
}

fn validate_update_entry(app_entry: &EntryTypes) -> ExternResult<ValidateCallbackResult> {
    match app_entry {
        // Commitments are immutable — they are REA notarizations of a point-in-time
        // delegation. An update would silently alter the notarized record without
        // revoking the original, which breaks the audit trail.
        EntryTypes::Commitment(_) => Ok(ValidateCallbackResult::Invalid(
            "Commitment entries are immutable; create a new Commitment to supersede".into(),
        )),
        // All other types delegate to create-time validation.
        _ => validate_create_entry(app_entry),
    }
}

fn validate_precedent(precedent: &Precedent) -> ExternResult<ValidateCallbackResult> {
    if precedent.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Precedent id cannot be empty".into(),
        ));
    }
    if !PRECEDENT_BINDING.contains(&precedent.binding.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid precedent binding level: {}",
            precedent.binding
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_discussion(discussion: &Discussion) -> ExternResult<ValidateCallbackResult> {
    if discussion.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Discussion id cannot be empty".into(),
        ));
    }
    if !DISCUSSION_CATEGORIES.contains(&discussion.category.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid discussion category: {}",
            discussion.category
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_governance_state(state: &GovernanceState) -> ExternResult<ValidateCallbackResult> {
    if state.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "GovernanceState id cannot be empty".into(),
        ));
    }
    if !GOVERNANCE_STATUS.contains(&state.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid governance status: {}",
            state.status
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_graduated_feedback(
    feedback: &GraduatedFeedback,
) -> ExternResult<ValidateCallbackResult> {
    if feedback.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "GraduatedFeedback id cannot be empty".into(),
        ));
    }
    if !FEEDBACK_CONTEXTS.contains(&feedback.feedback_context.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid feedback context: {}",
            feedback.feedback_context
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_opinion_statement(
    statement: &OpinionStatement,
) -> ExternResult<ValidateCallbackResult> {
    if statement.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "OpinionStatement id cannot be empty".into(),
        ));
    }
    if statement.text.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "OpinionStatement text cannot be empty".into(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_place(place: &Place) -> ExternResult<ValidateCallbackResult> {
    if place.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Place id cannot be empty".into(),
        ));
    }
    if place.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Place name cannot be empty".into(),
        ));
    }
    if !PLACE_TYPES.contains(&place.place_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid place type: {}",
            place.place_type
        )));
    }
    if !PLACE_STATUS.contains(&place.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid place status: {}",
            place.status
        )));
    }
    if place.h3_index.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Place h3_index cannot be empty".into(),
        ));
    }
    if place.h3_resolution > 15 {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid H3 resolution: {} (must be 0-15)",
            place.h3_resolution
        )));
    }
    // Validate latitude range
    if place.centroid_lat < -90.0 || place.centroid_lat > 90.0 {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid centroid latitude: {} (must be -90 to 90)",
            place.centroid_lat
        )));
    }
    // Validate longitude range
    if place.centroid_lng < -180.0 || place.centroid_lng > 180.0 {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid centroid longitude: {} (must be -180 to 180)",
            place.centroid_lng
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_challenge_outcome(outcome: &ChallengeOutcome) -> ExternResult<ValidateCallbackResult> {
    if outcome.outcome_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ChallengeOutcome outcome_id cannot be empty".into(),
        ));
    }
    if outcome.challenge_cid.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ChallengeOutcome challenge_cid cannot be empty".into(),
        ));
    }
    if !CHALLENGE_VERDICTS.contains(&outcome.verdict.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid ChallengeOutcome verdict: {} (expected one of {:?})",
            outcome.verdict, CHALLENGE_VERDICTS
        )));
    }
    // Validate that reasoning_json is at least parseable as JSON.
    // We do a minimal structural check — the full ConstitutionalReasoning shape
    // is validated by the coordinator at construction time.
    if outcome.reasoning_json.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ChallengeOutcome reasoning_json cannot be empty".into(),
        ));
    }
    // Use a simple check: must start with '{' for a JSON object.
    // Full serde_json::from_str is not available in HDI (no_std WASM env).
    let trimmed = outcome.reasoning_json.trim();
    if !trimmed.starts_with('{') {
        return Ok(ValidateCallbackResult::Invalid(
            "ChallengeOutcome reasoning_json must be a JSON object".into(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

/// Locate `"<field>"` used as a JSON *key* (quoted name, optional ASCII
/// whitespace, then `:`) and, if its value is a quoted string, return the raw
/// value slice (between the opening quote and the first *unescaped* closing
/// quote).
///
/// This is a structural scanner, not a parser — serde_json is dev-only here
/// (WASM size budget; see `validate_commitment_entry` preamble). It is far
/// stronger than a bare `meta.contains("field")`: the literal must appear as a
/// quoted key with a `:` after it, so smuggling the bare word into a comment or
/// free-text value no longer matches, and `"<field>_obligations"`-style sibling
/// keys (the schema's reciprocity block) are not confused for `<field>`.
///
/// Residual approximation (documented honestly): the scanner does not track
/// JSON string-context, so a key-shaped substring sitting *inside another
/// string value* — e.g. the value `"\"provider\":x"` — would be matched as if
/// it were a real key. Producing such a payload requires deliberately encoding
/// an escaped quoted-key inside a value; it cannot occur from ordinary
/// well-formed data, and the coordinator's full serde validation rejects it on
/// the create path. The integrity arm is defense-in-depth against a direct
/// source-chain write, and this residual is strictly narrower than the previous
/// `contains` behaviour.
fn json_string_field<'a>(meta: &'a str, field: &str) -> Option<&'a str> {
    let bytes = meta.as_bytes();
    let mut search_from = 0usize;
    while let Some(key_value_start) = find_json_key(meta, field, search_from) {
        // After the key + ':' we expect optional whitespace then a quoted string.
        let mut i = key_value_start;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'"' {
            let value_start = i + 1;
            let mut j = value_start;
            while j < bytes.len() {
                match bytes[j] {
                    b'\\' => j += 2, // skip escaped char (conservative)
                    b'"' => return Some(&meta[value_start..j]),
                    _ => j += 1,
                }
            }
            return None; // unterminated string value
        }
        // Key present but value is not a string (number/array/object/etc.).
        // Continue searching in case the same field name appears again.
        search_from = key_value_start;
    }
    None
}

/// Does `meta` contain `"<field>"` used as a JSON *key* (quoted name, optional
/// ASCII whitespace, then `:`)? Use this for non-string values (objects,
/// numbers, arrays). Same structural guarantees and same documented residual
/// as [`json_string_field`].
fn has_json_key(meta: &str, field: &str) -> bool {
    find_json_key(meta, field, 0).is_some()
}

/// Shared scan: return the byte index *just past the `:`* of the first
/// occurrence of `"<field>"` followed by optional ASCII whitespace and a `:`,
/// at or after `from`. Returns None if no such key occurrence exists.
fn find_json_key(meta: &str, field: &str, from: usize) -> Option<usize> {
    let bytes = meta.as_bytes();
    // The quoted key token, e.g. `"provider"`.
    let mut quoted = String::with_capacity(field.len() + 2);
    quoted.push('"');
    quoted.push_str(field);
    quoted.push('"');
    let needle = quoted.as_bytes();
    if from >= meta.len() {
        return None;
    }
    let mut start = from;
    while let Some(rel) = meta[start..].find(&quoted) {
        let key_end = start + rel + needle.len(); // index just past the closing quote
        let mut i = key_end;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b':' {
            return Some(i + 1);
        }
        // Not a key (no `:` after the quoted token). Advance past this match.
        start = start + rel + 1;
    }
    None
}

/// A JSON string value counts as "present and meaningful" only when it is
/// non-empty after trimming ASCII whitespace. Rejects `""`, `"   "`, etc.
fn json_string_field_nonblank(meta: &str, field: &str) -> bool {
    json_string_field(meta, field).is_some_and(|v| !v.trim().is_empty())
}

fn validate_commitment_entry(commitment: &Commitment) -> ExternResult<ValidateCallbackResult> {
    if commitment.action.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Commitment.action must be non-empty".into(),
        ));
    }
    if commitment.payload_json.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Commitment.payload_json must be non-empty".into(),
        ));
    }
    // Defense-in-depth: payload_json must look like a JSON object.
    // Full serde_json::from_str is not available at runtime in the integrity zome
    // (serde_json is dev-only here — WASM size budget). The coordinator performs
    // the full schema-specific validation; integrity only confirms the bytes are
    // at minimum a JSON object (same pattern as validate_challenge_outcome).
    let trimmed = commitment.payload_json.trim();
    if !trimmed.starts_with('{') {
        return Ok(ValidateCallbackResult::Invalid(
            "Commitment.payload_json must be a JSON object".into(),
        ));
    }
    if commitment.signed_at.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Commitment.signed_at must be non-empty".into(),
        ));
    }
    // Per-action defense-in-depth. The coordinator does full serde_json schema
    // validation on the create path; these integrity checks catch a direct
    // source-chain write that bypasses the coordinator. They run on the trimmed
    // payload (already confirmed to start with '{' by the preamble above) using
    // the structural `json_string_field` / `has_json_key` scanners — NOT bare
    // `contains`, which matched smuggled free-text and `*_obligations` siblings.
    let meta = commitment.payload_json.trim();
    if let Some(reason) = commitment_action_requirements(&commitment.action, meta) {
        return Ok(ValidateCallbackResult::Invalid(reason));
    }
    Ok(ValidateCallbackResult::Valid)
}

/// Per-action requirements table for [`validate_commitment_entry`]. Returns
/// `Some(reason)` if `meta` (a trimmed JSON object) fails the action's
/// defense-in-depth checks, `None` if it passes (or the action carries no
/// integrity-level requirements). All field lookups are structural (key-shaped
/// presence and string-value reads), so a bare-word smuggle or a sibling
/// `<field>_obligations` key cannot satisfy a requirement.
fn commitment_action_requirements(action: &str, meta: &str) -> Option<String> {
    match action {
        // Sprint 3: dwelling-hub mutual replication. Recipient is required and
        // must name a hub (no anonymous replication); provider_role is a
        // closed enum of two values (value match via the string-value read).
        "replicates-dwelling" => {
            if !has_json_key(meta, "recipient_dwelling_hub_id") {
                return Some("replicates-dwelling requires recipient_dwelling_hub_id field".into());
            }
            if !json_string_field_nonblank(meta, "recipient_dwelling_hub_id") {
                return Some(
                    "replicates-dwelling recipient_dwelling_hub_id must be non-empty".into(),
                );
            }
            match json_string_field(meta, "provider_role") {
                Some("steward_mutual") | Some("collective_steward") => None,
                _ => Some(
                    "replicates-dwelling provider_role must be steward_mutual or collective_steward"
                        .into(),
                ),
            }
        }
        // Slice-2b: commons replication. `variant` is a content|capacity enum;
        // `reach_ceiling` must equal "commons" (commons provide loop only).
        "replicates-commons" => {
            match json_string_field(meta, "variant") {
                Some("content") | Some("capacity") => {}
                _ => return Some("replicates-commons variant must be content or capacity".into()),
            }
            if json_string_field(meta, "reach_ceiling") != Some("commons") {
                return Some("replicates-commons reach_ceiling must be commons".into());
            }
            None
        }
        // N5: the REA compute-commitment primitive's first concrete instance
        // (Z.D deploy authority). provider/recipient are required non-empty
        // strings (no anonymous grant of authority); the bounds object must
        // carry the four fields the substrate walks on every bounded_by
        // EconomicEvent. The bound fields are non-string (arrays/numbers) and
        // live inside the nested `bounds` object, so we assert key presence with
        // `has_json_key`; the coordinator enforces types/minimums.
        // Ref: spec §1; schema v1/commitments/delegates-compute.schema.json.
        "delegates-compute" => {
            if !has_json_key(meta, "provider") {
                return Some("delegates-compute requires provider field".into());
            }
            if !has_json_key(meta, "recipient") {
                return Some("delegates-compute requires recipient field".into());
            }
            if !has_json_key(meta, "bounds") {
                return Some("delegates-compute requires bounds field".into());
            }
            if !json_string_field_nonblank(meta, "provider") {
                return Some("delegates-compute provider must be non-empty".into());
            }
            if !json_string_field_nonblank(meta, "recipient") {
                return Some("delegates-compute recipient must be non-empty".into());
            }
            for bound in [
                "epr_scope",
                "reach_ceiling",
                "rate_per_hour",
                "rotation_ttl_days",
            ] {
                if !has_json_key(meta, bound) {
                    return Some(format!("delegates-compute bounds must carry {bound}"));
                }
            }
            None
        }
        // Slice-2b: revocation. target_cid is required and non-empty.
        "revokes-commitment" => {
            if !has_json_key(meta, "target_cid") {
                return Some("revokes-commitment requires target_cid field".into());
            }
            if !json_string_field_nonblank(meta, "target_cid") {
                return Some("revokes-commitment target_cid must be non-empty".into());
            }
            None
        }
        _ => None,
    }
}

// =============================================================================
// Tests (native-compilable — no HDK WASM calls, pure logic)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ChallengeOutcome tests
    // =========================================================================

    fn make_valid_outcome() -> ChallengeOutcome {
        ChallengeOutcome {
            outcome_id: "bafybeioutcome1".into(),
            challenge_cid: "bafybeichallenge1".into(),
            verdict: "upheld".into(),
            reviewer_consensus: "uhCAkreviewerA,uhCAkreviewerB".into(),
            reasoning_json: r#"{"summary":"Challenge was valid","rules":[]}"#.into(),
            decided_at: "2026-04-20T09:00:00Z".into(),
            indemnification_actions_json: r#"[{"type":"ReputationDegrade","elohim_id":"uhCAkelohim1","dimensions":["factual-accuracy"],"magnitude":0.3}]"#.into(),
        }
    }

    #[test]
    fn valid_outcome_passes_validation() {
        let o = make_valid_outcome();
        let result = validate_challenge_outcome(&o).unwrap();
        assert_eq!(result, ValidateCallbackResult::Valid);
    }

    #[test]
    fn all_valid_verdicts_pass() {
        for verdict in &CHALLENGE_VERDICTS {
            let mut o = make_valid_outcome();
            o.verdict = (*verdict).into();
            let result = validate_challenge_outcome(&o).unwrap();
            assert_eq!(
                result,
                ValidateCallbackResult::Valid,
                "verdict {} should pass",
                verdict
            );
        }
    }

    #[test]
    fn invalid_verdict_fails() {
        let mut o = make_valid_outcome();
        o.verdict = "maybe".into();
        let result = validate_challenge_outcome(&o).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn empty_outcome_id_fails() {
        let mut o = make_valid_outcome();
        o.outcome_id = "".into();
        let result = validate_challenge_outcome(&o).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn empty_challenge_cid_fails() {
        let mut o = make_valid_outcome();
        o.challenge_cid = "".into();
        let result = validate_challenge_outcome(&o).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn empty_reasoning_json_fails() {
        let mut o = make_valid_outcome();
        o.reasoning_json = "".into();
        let result = validate_challenge_outcome(&o).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn non_object_reasoning_json_fails() {
        let mut o = make_valid_outcome();
        o.reasoning_json = r#"["array", "not", "object"]"#.into();
        let result = validate_challenge_outcome(&o).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn outcome_serde_roundtrip() {
        let o = make_valid_outcome();
        let json = serde_json::to_string(&o).unwrap();
        let decoded: ChallengeOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.outcome_id, o.outcome_id);
        assert_eq!(decoded.challenge_cid, o.challenge_cid);
        assert_eq!(decoded.verdict, o.verdict);
        assert_eq!(decoded.reasoning_json, o.reasoning_json);
        assert_eq!(
            decoded.indemnification_actions_json,
            o.indemnification_actions_json
        );
    }

    // =========================================================================
    // replicates-dwelling Commitment tests (Sprint 3)
    // =========================================================================

    #[test]
    fn replicates_dwelling_well_formed_accepted() {
        let event = Commitment {
            action: "replicates-dwelling".into(),
            payload_json: r#"{"action":"replicates-dwelling","provider_dwelling_hub_id":"hub:A","recipient_dwelling_hub_id":"hub:B","provider_role":"steward_mutual","capacity_bytes":1}"#.into(),
            signed_at: "2026-05-28T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&event).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Valid));
    }

    #[test]
    fn replicates_dwelling_empty_recipient_rejected() {
        let event = Commitment {
            action: "replicates-dwelling".into(),
            payload_json: r#"{"action":"replicates-dwelling","provider_dwelling_hub_id":"hub:A","recipient_dwelling_hub_id":"","provider_role":"steward_mutual"}"#.into(),
            signed_at: "2026-05-28T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&event).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn replicates_dwelling_missing_recipient_rejected() {
        let event = Commitment {
            action: "replicates-dwelling".into(),
            payload_json: r#"{"action":"replicates-dwelling","provider_dwelling_hub_id":"hub:A","provider_role":"steward_mutual"}"#.into(),
            signed_at: "2026-05-28T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&event).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn replicates_dwelling_unknown_role_rejected() {
        let event = Commitment {
            action: "replicates-dwelling".into(),
            payload_json: r#"{"action":"replicates-dwelling","provider_dwelling_hub_id":"hub:A","recipient_dwelling_hub_id":"hub:B","provider_role":"totally-bogus"}"#.into(),
            signed_at: "2026-05-28T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&event).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    // =========================================================================
    // delegates-compute Commitment integrity tests (N5)
    //
    // Defense-in-depth: the coordinator does full serde_json validation; these
    // assert the integrity arm catches a direct-source-chain bypass. The payloads
    // are built with serde_json (a dev-dependency) and stringified — the runtime
    // validator itself sees only the string and uses substring checks.
    // =========================================================================

    fn delegates_compute_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "delegates-compute",
            "scope": "republish-epr",
            "provider": "agent:matthew-steward",
            "recipient": "agent:deploy-svc-matthew",
            "bounds": {
                "epr_scope": ["epr:lamad-spa"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            },
            "valid_from": "2026-05-25T00:00:00Z",
            "valid_until": "2026-08-23T00:00:00Z"
        })
    }

    fn delegates_compute_commitment(payload: &serde_json::Value) -> Commitment {
        Commitment {
            action: "delegates-compute".into(),
            payload_json: payload.to_string(),
            signed_at: "2026-05-25T00:00:00Z".into(),
        }
    }

    #[test]
    fn delegates_compute_well_formed_accepted() {
        let c = delegates_compute_commitment(&delegates_compute_payload());
        let result = validate_commitment_entry(&c).unwrap();
        assert!(
            matches!(result, ValidateCallbackResult::Valid),
            "well-formed delegates-compute bounds must pass integrity: {result:?}"
        );
    }

    #[test]
    fn delegates_compute_missing_recipient_rejected() {
        let mut payload = delegates_compute_payload();
        payload.as_object_mut().unwrap().remove("recipient");
        let c = delegates_compute_commitment(&payload);
        let result = validate_commitment_entry(&c).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn delegates_compute_empty_provider_rejected() {
        let mut payload = delegates_compute_payload();
        payload["provider"] = serde_json::json!("");
        let c = delegates_compute_commitment(&payload);
        let result = validate_commitment_entry(&c).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn delegates_compute_missing_bounds_rejected() {
        let mut payload = delegates_compute_payload();
        payload.as_object_mut().unwrap().remove("bounds");
        let c = delegates_compute_commitment(&payload);
        let result = validate_commitment_entry(&c).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn delegates_compute_bounds_missing_rate_per_hour_rejected() {
        let mut payload = delegates_compute_payload();
        payload["bounds"]
            .as_object_mut()
            .unwrap()
            .remove("rate_per_hour");
        let c = delegates_compute_commitment(&payload);
        let result = validate_commitment_entry(&c).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn delegates_compute_bounds_missing_rotation_ttl_days_rejected() {
        let mut payload = delegates_compute_payload();
        payload["bounds"]
            .as_object_mut()
            .unwrap()
            .remove("rotation_ttl_days");
        let c = delegates_compute_commitment(&payload);
        let result = validate_commitment_entry(&c).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn delegates_compute_non_object_payload_rejected() {
        // Direct-source-chain bypass authoring a non-object payload.
        let c = Commitment {
            action: "delegates-compute".into(),
            payload_json: "\"not-an-object\"".into(),
            signed_at: "2026-05-25T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&c).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    // =========================================================================
    // Adversary tests — confirm the structural scanner closes the four
    // `contains`-bypass classes the previous arms were vulnerable to. These are
    // hand-authored direct-source-chain payloads (the attacker controls the raw
    // bytes), NOT serde_json-built fixtures.
    // =========================================================================

    /// Bypass (1): the required field names smuggled as bare words inside an
    /// unrelated string value. The old `meta.contains("provider")` matched this;
    /// the structural scanner requires `"provider"` as a quoted key + `:`.
    #[test]
    fn delegates_compute_note_smuggling_rejected() {
        let c = Commitment {
            action: "delegates-compute".into(),
            payload_json: r#"{"note":"provider recipient bounds epr_scope reach_ceiling rate_per_hour rotation_ttl_days"}"#
                .into(),
            signed_at: "2026-05-25T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&c).unwrap();
        assert!(
            matches!(result, ValidateCallbackResult::Invalid(_)),
            "bare-word smuggling in a note value must not satisfy required fields"
        );
    }

    /// Bypass (2): the schema's optional `reciprocity` block carries
    /// `provider_obligations` / `recipient_obligations` keys, whose names contain
    /// the substrings `provider` / `recipient`. A payload with NO top-level
    /// provider/recipient but a reciprocity block passed the old `contains`
    /// check; the structural scanner matches `"provider"` exactly, not the
    /// `*_obligations` sibling.
    #[test]
    fn delegates_compute_reciprocity_block_only_rejected() {
        let c = Commitment {
            action: "delegates-compute".into(),
            payload_json: r#"{"reciprocity":{"provider_obligations":["x"],"recipient_obligations":["y"]},"bounds":{"epr_scope":[],"reach_ceiling":"commons","rate_per_hour":1,"rotation_ttl_days":1}}"#
                .into(),
            signed_at: "2026-05-25T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&c).unwrap();
        assert!(
            matches!(result, ValidateCallbackResult::Invalid(_)),
            "a reciprocity block must not satisfy the top-level provider/recipient requirement"
        );
    }

    /// Bypass (3a): `"provider" : ""` — space-colon-space then empty value. The
    /// old arm only literal-matched `":""` and `": ""`; the structural scanner
    /// tolerates whitespace around the colon and still sees the empty value.
    #[test]
    fn delegates_compute_space_colon_space_empty_provider_rejected() {
        let c = Commitment {
            action: "delegates-compute".into(),
            payload_json: r#"{"provider" : "","recipient":"agent:r","bounds":{"epr_scope":[],"reach_ceiling":"commons","rate_per_hour":1,"rotation_ttl_days":1}}"#
                .into(),
            signed_at: "2026-05-25T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&c).unwrap();
        assert!(
            matches!(result, ValidateCallbackResult::Invalid(_)),
            "whitespace around the colon must not let an empty provider through"
        );
    }

    /// Bypass (3b): whitespace-only value `"   "`. The old empty-check only
    /// caught the exactly-empty string; `json_string_field_nonblank` trims.
    #[test]
    fn delegates_compute_whitespace_only_provider_rejected() {
        let c = Commitment {
            action: "delegates-compute".into(),
            payload_json: r#"{"provider":"   ","recipient":"agent:r","bounds":{"epr_scope":[],"reach_ceiling":"commons","rate_per_hour":1,"rotation_ttl_days":1}}"#
                .into(),
            signed_at: "2026-05-25T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&c).unwrap();
        assert!(
            matches!(result, ValidateCallbackResult::Invalid(_)),
            "a whitespace-only provider must be rejected"
        );
    }

    /// Same whitespace-only rejection on the replicates-dwelling recipient and
    /// the revokes-commitment target_cid — the shared `nonblank` helper covers
    /// every arm, not just delegates-compute.
    #[test]
    fn replicates_dwelling_whitespace_only_recipient_rejected() {
        let c = Commitment {
            action: "replicates-dwelling".into(),
            payload_json: r#"{"recipient_dwelling_hub_id":"  ","provider_role":"steward_mutual"}"#
                .into(),
            signed_at: "2026-05-28T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&c).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    #[test]
    fn revokes_commitment_whitespace_only_target_rejected() {
        let c = Commitment {
            action: "revokes-commitment".into(),
            payload_json: r#"{"target_cid":"   "}"#.into(),
            signed_at: "2026-05-28T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&c).unwrap();
        assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
    }

    /// The scanner must not treat a key-shaped substring sitting inside the
    /// VALUE of another field as a real key. Here `target_cid` appears only
    /// inside the value of `decoy` (as escaped text), never as a real key, so
    /// the revokes-commitment arm must reject for a missing target_cid.
    ///
    /// NOTE (honest residual): the scanner is string-context-unaware. It
    /// rejects THIS payload because the embedded `target_cid` is preceded by an
    /// escaped quote (`\"`), so `find_json_key` searching for the *unescaped*
    /// token `"target_cid"` does not match the escaped form. A payload that
    /// embedded the *unescaped* sequence `"target_cid":"x"` inside another
    /// string value (only constructable as `"\"target_cid\":\"x\""` at the JSON
    /// level, i.e. with escapes) is the documented residual: the scanner would
    /// see the inner token as a key. Producing it requires deliberately
    /// encoding an escaped key inside a value; the coordinator's full serde
    /// validation rejects it on the create path.
    #[test]
    fn revokes_commitment_key_inside_value_not_a_key() {
        let c = Commitment {
            action: "revokes-commitment".into(),
            // {"decoy":"\"target_cid\":\"x\""} — target_cid only as escaped text.
            payload_json: r#"{"decoy":"\"target_cid\":\"x\""}"#.into(),
            signed_at: "2026-05-28T00:00:00Z".into(),
        };
        let result = validate_commitment_entry(&c).unwrap();
        assert!(
            matches!(result, ValidateCallbackResult::Invalid(_)),
            "an escaped target_cid token inside a value must not satisfy the key requirement"
        );
    }

    /// Direct unit coverage of the shared matchers, independent of any arm.
    #[test]
    fn json_string_field_matchers_behave() {
        // Plain key/value.
        assert_eq!(
            json_string_field(r#"{"a":"b"}"#, "a"),
            Some("b"),
            "basic key/value read"
        );
        // Whitespace around the colon.
        assert_eq!(
            json_string_field(r#"{"a" : "b"}"#, "a"),
            Some("b"),
            "tolerates ws around colon"
        );
        // Sibling key with the target as a prefix must NOT match.
        assert_eq!(
            json_string_field(r#"{"a_obligations":"b"}"#, "a"),
            None,
            "prefix-sibling key is not the field"
        );
        // Bare word in a value is not a key.
        assert_eq!(
            json_string_field(r#"{"note":"a b c"}"#, "a"),
            None,
            "bare word in value is not a key"
        );
        // Non-string value: not returned by json_string_field, but seen by has_json_key.
        assert_eq!(json_string_field(r#"{"a":[1,2]}"#, "a"), None);
        assert!(has_json_key(r#"{"a":[1,2]}"#, "a"));
        assert!(has_json_key(r#"{"a" : 3}"#, "a"));
        assert!(!has_json_key(r#"{"a_obligations":3}"#, "a"));
        // Escaped quote inside value terminates conservatively at the escape's
        // following content — the value read stops at the first UNescaped quote.
        assert_eq!(
            json_string_field(r#"{"a":"x\"y"}"#, "a"),
            Some(r#"x\"y"#),
            "escaped quote does not terminate the value early"
        );
        // Blank-detection helper.
        assert!(json_string_field_nonblank(r#"{"a":"b"}"#, "a"));
        assert!(!json_string_field_nonblank(r#"{"a":""}"#, "a"));
        assert!(!json_string_field_nonblank(r#"{"a":"   "}"#, "a"));
    }

    // =========================================================================
    // CommitmentByState link tag tests (Slice-2b T11)
    // =========================================================================

    #[test]
    fn commitment_by_state_well_formed_tag_passes() {
        assert!(validate_commitment_by_state_tag("active|2026-06-11T10:00:00Z").is_ok());
    }

    #[test]
    fn commitment_by_state_empty_state_rejected() {
        // Leading '|' → empty state segment.
        assert!(validate_commitment_by_state_tag("|2026-06-11T10:00:00Z").is_err());
    }

    #[test]
    fn commitment_by_state_missing_signed_at_rejected() {
        // No delimiter → no signed_at segment.
        assert!(validate_commitment_by_state_tag("active").is_err());
    }

    #[test]
    fn commitment_by_state_empty_signed_at_rejected() {
        // Trailing '|' → empty signed_at segment.
        assert!(validate_commitment_by_state_tag("active|").is_err());
    }

    #[test]
    fn commitment_by_state_signed_at_with_delimiter_preserved() {
        // splitn(2) keeps any later '|' inside the signed_at segment — still valid.
        assert!(validate_commitment_by_state_tag("active|2026-06-11T10:00:00Z|extra").is_ok());
    }
}
