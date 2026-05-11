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
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
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
    }
}

fn validate_update_entry(app_entry: &EntryTypes) -> ExternResult<ValidateCallbackResult> {
    validate_create_entry(app_entry)
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
}
