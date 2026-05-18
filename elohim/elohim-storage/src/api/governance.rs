//! Governance API controller
//!
//! Routes: `/api/v1/governance/{state|states|challenges|proposals|precedents|discussions}[/{id}]`
//!
//! Governance is per-entity, not app-scoped. Delegates directly to `db::governance`
//! CRUD functions.

use bytes::Bytes;
use diesel::prelude::*;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::diesel_schema::proposals;
use crate::db::governance;
use crate::db::models::{
    NewAppeal, NewChallenge, NewDiscussion, NewGovernanceSignal, NewProposal, NewProposalOption,
    NewRankedVote, NewVote,
};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::{disposition_service, response, sla_service};
use crate::tally;
use crate::views::{
    AppealView, CastRankedVoteInputView, CastVoteInputView, ChallengeView,
    CreateDiscussionInputView, CreateProposalInputView, CreateProposalOptionInputView,
    CreateStatementInputView, DiscussionView, FileAppealInputView, FileChallengeInputView,
    GovernanceDispositionView, GovernanceSignalView, GovernanceStateView, PostMessageInputView,
    PrecedentView, ProposalOptionView, ProposalView, RankedVoteView, RecordSignalInputView,
    RespondToChallengeInputView, StatementView, StatementVoteView, UpdateDispositionInputView,
    VoteOnStatementInputView, VoteView,
};

use super::get_conn;

// ---------------------------------------------------------------------------
// Query param types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GovernanceStateQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContentQuery {
    pub content_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ChallengeQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ProposalQuery {
    pub content_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SignalQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SensemakingQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Extract `{id}` from a resource path like `/{id}`
fn extract_id(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let id = trimmed.split('/').next()?;
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/governance*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let (path_only, _query) = resource_path.split_once('?').unwrap_or((resource_path, ""));
    let query_str = req.uri().query().unwrap_or("");

    match (&method, path_only) {
        // GET /api/v1/governance/state?entityType=X&entityId=Y
        (&Method::GET, "/state") => {
            let params: GovernanceStateQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let entity_type = params.entity_type.as_deref().unwrap_or("");
            let entity_id = params.entity_id.as_deref().unwrap_or("");
            if entity_type.is_empty() || entity_id.is_empty() {
                return Ok(response::bad_request(
                    "entityType and entityId query params are required",
                ));
            }
            let mut conn = get_conn(pool)?;
            let result = governance::get_governance_state(&mut conn, entity_type, entity_id)?;
            Ok(response::from_option(
                Ok(result.map(GovernanceStateView::from)),
                &format!(
                    "Governance state not found for {}:{}",
                    entity_type, entity_id
                ),
            ))
        }

        // GET /api/v1/governance/states?entityType=X
        (&Method::GET, "/states") => {
            let params: GovernanceStateQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let entity_type = params.entity_type.as_deref().unwrap_or("");
            if entity_type.is_empty() {
                return Ok(response::bad_request("entityType query param is required"));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::query_governance_states(&mut conn, entity_type)?;
            let views: Vec<GovernanceStateView> =
                results.into_iter().map(GovernanceStateView::from).collect();
            Ok(response::ok(&views))
        }

        // POST /api/v1/governance/challenges — File a new challenge
        (&Method::POST, "/challenges") => {
            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: FileChallengeInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let now = crate::db::models::current_timestamp();
            let challenge_id = format!("chal-{}", uuid::Uuid::new_v4());
            let response_deadline = sla_service::compute_response_deadline(&now);

            let new = NewChallenge {
                id: &challenge_id,
                entity_type: &input.entity_type,
                entity_id: &input.entity_id,
                challenger_id: &input.challenger_id,
                standing_basis: &input.standing_basis,
                grounds_primary: &input.grounds_primary,
                grounds_secondary: input.grounds_secondary.as_deref(),
                evidence: &input.evidence,
                requested_outcome: input.requested_outcome.as_deref(),
                state: "pending",
                filed_at: &now,
                response_deadline: &response_deadline,
                created_at: &now,
            };

            // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
            // Classification A (Notarized): challenges are public acts, must be witnessed by the community.
            // Currently null for direct storage writes. Backfill needed for pre-coherence data.
            let mut conn = get_conn(pool)?;
            let challenge = governance::create_challenge(&mut conn, &new)?;
            let sla = sla_service::compute_sla_status(&challenge);
            let mut view = ChallengeView::from(challenge);
            view.sla_status = sla.as_str().to_string();
            Ok(response::created(&view))
        }

        // POST /api/v1/governance/challenges/{id}/respond — Respond to a challenge
        (&Method::POST, p) if p.starts_with("/challenges/") && p.ends_with("/respond") => {
            let id = p
                .strip_prefix("/challenges/")
                .and_then(|s| s.strip_suffix("/respond"))
                .ok_or_else(|| StorageError::InvalidInput("Challenge ID required".to_string()))?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: RespondToChallengeInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let mut conn = get_conn(pool)?;
            let challenge = governance::respond_to_challenge(
                &mut conn,
                id,
                &input.outcome,
                &input.reasoning,
                input.actions.as_deref(),
                "system",
                input.sets_precedent,
            )?;
            let sla = sla_service::compute_sla_status(&challenge);
            let mut view = ChallengeView::from(challenge);
            view.sla_status = sla.as_str().to_string();
            Ok(response::ok(&view))
        }

        // POST /api/v1/governance/challenges/{id}/appeal — File an appeal against a challenge decision
        (&Method::POST, p) if p.starts_with("/challenges/") && p.ends_with("/appeal") => {
            let challenge_id = p
                .strip_prefix("/challenges/")
                .and_then(|s| s.strip_suffix("/appeal"))
                .ok_or_else(|| StorageError::InvalidInput("Challenge ID required".to_string()))?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: FileAppealInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let now = crate::db::models::current_timestamp();
            let appeal_id = format!("appl-{}", uuid::Uuid::new_v4());

            // Get the challenge to derive appellant_id (challenger is the default appellant)
            let mut conn = get_conn(pool)?;
            let challenge =
                governance::get_challenge(&mut conn, challenge_id)?.ok_or_else(|| {
                    StorageError::NotFound(format!("Challenge {} not found", challenge_id))
                })?;

            let new = NewAppeal {
                id: &appeal_id,
                challenge_id,
                appellant_id: &challenge.challenger_id,
                grounds: &input.grounds,
                additional_evidence: input.additional_evidence.as_deref(),
                state: "pending",
                filed_at: &now,
                created_at: &now,
            };

            // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
            // Classification A (Notarized): appeals are public acts, must be witnessed by the community.
            // Currently null for direct storage writes. Backfill needed for pre-coherence data.
            let appeal = governance::create_appeal(&mut conn, &new)?;
            Ok(response::created(&AppealView::from(appeal)))
        }

        // GET /api/v1/governance/challenges/{id}
        (&Method::GET, p) if p.starts_with("/challenges/") => {
            let sub = p.strip_prefix("/challenges").unwrap_or("");
            let id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Challenge ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = governance::get_challenge(&mut conn, id)?;
            match result {
                Some(challenge) => {
                    let sla = sla_service::compute_sla_status(&challenge);
                    let mut view = ChallengeView::from(challenge);
                    view.sla_status = sla.as_str().to_string();
                    Ok(response::ok(&view))
                }
                None => Ok(response::not_found(&format!("Challenge {} not found", id))),
            }
        }

        // GET /api/v1/governance/challenges?entityType=X&entityId=Y
        (&Method::GET, "/challenges") => {
            let params: ChallengeQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let entity_type = params.entity_type.as_deref().unwrap_or("");
            let entity_id = params.entity_id.as_deref().unwrap_or("");
            if entity_type.is_empty() || entity_id.is_empty() {
                return Ok(response::bad_request(
                    "entityType and entityId query params are required",
                ));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::query_challenges(&mut conn, entity_type, entity_id)?;
            let views: Vec<ChallengeView> = results
                .into_iter()
                .map(|c| {
                    let sla = sla_service::compute_sla_status(&c);
                    let mut view = ChallengeView::from(c);
                    view.sla_status = sla.as_str().to_string();
                    view
                })
                .collect();
            Ok(response::ok(&views))
        }

        // GET /api/v1/governance/appeals/{challengeId} — List appeals for a challenge
        (&Method::GET, p) if p.starts_with("/appeals/") => {
            let sub = p.strip_prefix("/appeals").unwrap_or("");
            let challenge_id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Challenge ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let results = governance::query_appeals_for_challenge(&mut conn, challenge_id)?;
            let views: Vec<AppealView> = results.into_iter().map(AppealView::from).collect();
            Ok(response::ok(&views))
        }

        // POST /api/v1/governance/proposals — Create a proposal
        (&Method::POST, "/proposals") => {
            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: CreateProposalInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let new = NewProposal {
                id: &input.id,
                content_id: &input.content_id,
                proposer_presence_id: &input.proposer_presence_id,
                proposal_type: &input.proposal_type,
                title: &input.title,
                body: &input.body,
                voting_mechanism: &input.voting_mechanism,
                score_min: input.score_min,
                score_max: input.score_max,
                dots_per_voter: input.dots_per_voter,
                quorum_percentage: input.quorum_percentage,
                passage_threshold: input.passage_threshold,
            };

            // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
            // Classification A (Notarized): proposals are public deliberative acts.
            // Currently null for direct storage writes. Backfill needed for pre-coherence data.
            let mut conn = get_conn(pool)?;
            let _result = governance::create_proposal(&mut conn, &new)?;

            // Set voting_anonymous after insert if requested
            if input.voting_anonymous {
                diesel::update(proposals::table.filter(proposals::id.eq(&input.id)))
                    .set(proposals::voting_anonymous.eq(1))
                    .execute(&mut conn)
                    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
            }

            let final_result = governance::get_proposal(&mut conn, &input.id)?
                .ok_or_else(|| StorageError::Internal("Created proposal not found".to_string()))?;
            Ok(response::created(&ProposalView::from(final_result)))
        }

        // POST /api/v1/governance/proposals/{id}/votes — Cast or update a vote
        (&Method::POST, p) if p.starts_with("/proposals/") && p.ends_with("/votes") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/votes"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: CastVoteInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let mut conn = get_conn(pool)?;

            let proposal = governance::get_proposal(&mut conn, id)?
                .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;

            let vote_id = format!("vote-{}-{}", id, input.human_id);
            let now = crate::db::models::current_timestamp();
            let new_vote = NewVote {
                id: &vote_id,
                proposal_id: id,
                human_id: &input.human_id,
                position: &input.position,
                reason: input.reason.as_deref(),
                anonymous: proposal.voting_anonymous,
                created_at: &now,
                updated_at: &now,
            };

            // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
            // Classification B2 (Agent-Scoped + Attestation): raw vote is private to the agent's source chain;
            // dht_anchor_hash set when the tally Attestation is issued by the counting zome.
            // Currently null for direct storage writes. Backfill needed for pre-coherence data.
            let vote = governance::cast_vote(&mut conn, &new_vote)?;
            let hide = proposal.voting_anonymous == 1;
            Ok(response::created(&crate::views::vote_view_from_vote(vote, hide)))
        }

        // POST /api/v1/governance/proposals/{id}/options — Create proposal options
        (&Method::POST, p) if p.starts_with("/proposals/") && p.ends_with("/options") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/options"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let inputs: Vec<CreateProposalOptionInputView> =
                serde_json::from_slice(&body.to_bytes())
                    .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let now = crate::db::models::current_timestamp();
            let new_options: Vec<NewProposalOption> = inputs
                .iter()
                .map(|input| NewProposalOption {
                    id: &input.id,
                    proposal_id: id,
                    label: &input.label,
                    description: &input.description,
                    position: input.position,
                    source: input.source.as_deref(),
                    source_justification: input.source_justification.as_deref(),
                    created_at: &now,
                })
                .collect();

            // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
            // Classification A2 (Derived): proposal_options are derived from the parent proposal via Link;
            // dht_anchor_hash links to parent Proposal ActionHash.
            // Currently null for direct storage writes. Backfill needed for pre-coherence data.
            let mut conn = get_conn(pool)?;
            let results = governance::create_proposal_options(&mut conn, &new_options)?;
            let views: Vec<ProposalOptionView> =
                results.into_iter().map(ProposalOptionView::from).collect();
            Ok(response::created(&views))
        }

        // GET /api/v1/governance/proposals/{id}/options — List proposal options
        (&Method::GET, p) if p.starts_with("/proposals/") && p.ends_with("/options") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/options"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let mut conn = get_conn(pool)?;
            let results = governance::query_proposal_options(&mut conn, id)?;
            let views: Vec<ProposalOptionView> =
                results.into_iter().map(ProposalOptionView::from).collect();
            Ok(response::ok(&views))
        }

        // POST /api/v1/governance/proposals/{id}/ranked-votes — Cast ranked/scored/dot/approval votes
        (&Method::POST, p) if p.starts_with("/proposals/") && p.ends_with("/ranked-votes") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/ranked-votes"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: CastRankedVoteInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let mut conn = get_conn(pool)?;

            // Fetch proposal for mechanism and config
            let proposal = governance::get_proposal(&mut conn, id)?
                .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;

            // Fetch options for validation
            let options = governance::query_proposal_options(&mut conn, id)?;

            // Build VotingConfig from proposal fields
            let config = tally::VotingConfig {
                score_min: proposal.score_min,
                score_max: proposal.score_max,
                dots_per_voter: proposal.dots_per_voter,
                quorum_percentage: proposal.quorum_percentage,
                passage_threshold: proposal.passage_threshold,
            };

            // Build temporary RankedVote structs for validation
            let now = crate::db::models::current_timestamp();
            let temp_votes: Vec<crate::db::models::RankedVote> = input
                .ballots
                .iter()
                .enumerate()
                .map(|(i, b)| crate::db::models::RankedVote {
                    id: format!("rv-{}-{}-{}", id, input.human_id, i),
                    proposal_id: id.to_string(),
                    human_id: input.human_id.clone(),
                    option_id: b.option_id.clone(),
                    rank: b.rank,
                    score: b.score,
                    dots: b.dots,
                    approved: b.approved.map(|a| if a { 1 } else { 0 }),
                    reasoning: input.reasoning.clone(),
                    proxy_elohim_id: input.proxy_elohim_id.clone(),
                    proxy_justification: input.proxy_justification.clone(),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    dht_anchor_hash: None,
                })
                .collect();

            // Validate ballot against mechanism
            let strategy = tally::get_strategy(&proposal.voting_mechanism).ok_or_else(|| {
                StorageError::InvalidInput(format!(
                    "Unknown voting mechanism: {}",
                    proposal.voting_mechanism
                ))
            })?;
            strategy
                .validate_ballot(&temp_votes, &options, &config)
                .map_err(|e| {
                    StorageError::InvalidInput(format!("Ballot validation failed: {}", e))
                })?;

            // Build NewRankedVote entries
            let vote_ids: Vec<String> = input
                .ballots
                .iter()
                .enumerate()
                .map(|(i, _)| format!("rv-{}-{}-{}", id, input.human_id, i))
                .collect();

            let new_votes: Vec<NewRankedVote> = input
                .ballots
                .iter()
                .enumerate()
                .map(|(i, b)| NewRankedVote {
                    id: &vote_ids[i],
                    proposal_id: id,
                    human_id: &input.human_id,
                    option_id: &b.option_id,
                    rank: b.rank,
                    score: b.score,
                    dots: b.dots,
                    approved: b.approved.map(|a| if a { 1 } else { 0 }),
                    reasoning: input.reasoning.as_deref(),
                    proxy_elohim_id: input.proxy_elohim_id.as_deref(),
                    proxy_justification: input.proxy_justification.as_deref(),
                    created_at: &now,
                    updated_at: &now,
                })
                .collect();

            // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
            // Classification B2 (Agent-Scoped + Attestation): raw ranked vote is private to the agent's source chain;
            // dht_anchor_hash set when the tally Attestation is issued by the counting zome.
            // Currently null for direct storage writes. Backfill needed for pre-coherence data.
            let results =
                governance::cast_ranked_votes(&mut conn, id, &input.human_id, &new_votes)?;
            let hide = proposal.voting_anonymous == 1;
            let views: Vec<RankedVoteView> = results
                .into_iter()
                .map(|v| crate::views::ranked_vote_view_from_ranked_vote(v, hide))
                .collect();
            Ok(response::created(&views))
        }

        // GET /api/v1/governance/proposals/{id}/ranked-votes — List ranked votes
        (&Method::GET, p) if p.starts_with("/proposals/") && p.ends_with("/ranked-votes") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/ranked-votes"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let mut conn = get_conn(pool)?;
            let proposal = governance::get_proposal(&mut conn, id)?
                .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;
            let hide = proposal.voting_anonymous == 1;
            let votes = governance::query_ranked_votes(&mut conn, id)?;
            let views: Vec<RankedVoteView> = votes
                .into_iter()
                .map(|v| crate::views::ranked_vote_view_from_ranked_vote(v, hide))
                .collect();
            Ok(response::ok(&views))
        }

        // GET /api/v1/governance/proposals/{id}/tally — Compute tally
        (&Method::GET, p) if p.starts_with("/proposals/") && p.ends_with("/tally") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/tally"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let mut conn = get_conn(pool)?;
            let proposal = governance::get_proposal(&mut conn, id)?
                .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;
            let options = governance::query_proposal_options(&mut conn, id)?;
            let votes = governance::query_ranked_votes(&mut conn, id)?;

            let config = tally::VotingConfig {
                score_min: proposal.score_min,
                score_max: proposal.score_max,
                dots_per_voter: proposal.dots_per_voter,
                quorum_percentage: proposal.quorum_percentage,
                passage_threshold: proposal.passage_threshold,
            };

            let strategy = tally::get_strategy(&proposal.voting_mechanism).ok_or_else(|| {
                StorageError::InvalidInput(format!(
                    "Unknown voting mechanism: {}",
                    proposal.voting_mechanism
                ))
            })?;
            let result = strategy.tally(&votes, &options, &config);
            Ok(response::ok(&result))
        }

        // GET /api/v1/governance/proposals/{id}/votes — List votes
        (&Method::GET, p) if p.starts_with("/proposals/") && p.ends_with("/votes") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/votes"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let mut conn = get_conn(pool)?;
            let proposal = governance::get_proposal(&mut conn, id)?
                .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;
            let hide = proposal.voting_anonymous == 1;
            let votes = governance::query_votes(&mut conn, id)?;
            let views: Vec<VoteView> = votes
                .into_iter()
                .map(|v| crate::views::vote_view_from_vote(v, hide))
                .collect();
            Ok(response::ok(&views))
        }

        // GET /api/v1/governance/proposals/{id}/proxy-votes — List proxy votes
        (&Method::GET, p) if p.starts_with("/proposals/") && p.ends_with("/proxy-votes") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/proxy-votes"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let mut conn = get_conn(pool)?;
            let proposal = governance::get_proposal(&mut conn, id)?
                .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;
            let hide = proposal.voting_anonymous == 1;

            let votes = governance::get_proxy_votes(&mut conn, id)?;
            let views: Vec<RankedVoteView> = votes
                .into_iter()
                .map(|v| crate::views::ranked_vote_view_from_ranked_vote(v, hide))
                .collect();
            Ok(response::ok(&views))
        }

        // GET /api/v1/governance/proposals/{id}
        (&Method::GET, p) if p.starts_with("/proposals/") => {
            let sub = p.strip_prefix("/proposals").unwrap_or("");
            let id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = governance::get_proposal(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(ProposalView::from)),
                &format!("Proposal {} not found", id),
            ))
        }

        // GET /api/v1/governance/proposals?contentId=X&status=Y
        (&Method::GET, "/proposals") => {
            let params: ProposalQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let content_id = params.content_id.as_deref().unwrap_or("");
            if content_id.is_empty() {
                return Ok(response::bad_request("contentId query param is required"));
            }
            let mut conn = get_conn(pool)?;
            let results =
                governance::query_proposals(&mut conn, content_id, params.status.as_deref())?;
            let views: Vec<ProposalView> = results.into_iter().map(ProposalView::from).collect();
            Ok(response::ok(&views))
        }

        // GET /api/v1/governance/precedents/{id}
        (&Method::GET, p) if p.starts_with("/precedents/") => {
            let sub = p.strip_prefix("/precedents").unwrap_or("");
            let id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Precedent ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = governance::get_precedent(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(PrecedentView::from)),
                &format!("Precedent {} not found", id),
            ))
        }

        // GET /api/v1/governance/precedents?contentId=X
        (&Method::GET, "/precedents") => {
            let params: ContentQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let content_id = params.content_id.as_deref().unwrap_or("");
            if content_id.is_empty() {
                return Ok(response::bad_request("contentId query param is required"));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::query_precedents(&mut conn, content_id)?;
            let views: Vec<PrecedentView> = results.into_iter().map(PrecedentView::from).collect();
            Ok(response::ok(&views))
        }

        // GET /api/v1/governance/discussions/{id}
        (&Method::GET, p) if p.starts_with("/discussions/") => {
            let sub = p.strip_prefix("/discussions").unwrap_or("");
            let id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Discussion ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = governance::get_discussion(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(DiscussionView::from)),
                &format!("Discussion {} not found", id),
            ))
        }

        // GET /api/v1/governance/discussions?contentId=X
        (&Method::GET, "/discussions") => {
            let params: ContentQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let content_id = params.content_id.as_deref().unwrap_or("");
            if content_id.is_empty() {
                return Ok(response::bad_request("contentId query param is required"));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::query_discussions(&mut conn, content_id)?;
            let views: Vec<DiscussionView> =
                results.into_iter().map(DiscussionView::from).collect();
            Ok(response::ok(&views))
        }

        // POST /api/v1/governance/discussions — Create a discussion
        (&Method::POST, "/discussions") => {
            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: CreateDiscussionInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let new = NewDiscussion {
                id: &input.id,
                content_id: &input.content_id,
                author_presence_id: &input.author_presence_id,
                body: &input.body,
                parent_id: input.parent_id.as_deref(),
            };

            let mut conn = get_conn(pool)?;
            let result = governance::create_discussion(&mut conn, &new)?;
            Ok(response::created(&DiscussionView::from(result)))
        }

        // POST /api/v1/governance/discussions/{id}/messages — Reply to discussion
        (&Method::POST, p) if p.starts_with("/discussions/") && p.ends_with("/messages") => {
            let discussion_id = p
                .strip_prefix("/discussions/")
                .and_then(|s| s.strip_suffix("/messages"))
                .ok_or_else(|| StorageError::InvalidInput("Discussion ID required".to_string()))?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: PostMessageInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let new = NewDiscussion {
                id: &input.id,
                content_id: discussion_id,
                author_presence_id: &input.author_presence_id,
                body: &input.body,
                parent_id: Some(discussion_id),
            };

            let mut conn = get_conn(pool)?;
            let result = governance::create_discussion(&mut conn, &new)?;
            Ok(response::created(&DiscussionView::from(result)))
        }

        // POST /api/v1/governance/signals — Record a governance signal
        (&Method::POST, "/signals") => {
            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: RecordSignalInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let now = crate::db::models::current_timestamp();
            let signal_id = format!("sig-{}-{}-{}", input.entity_type, input.entity_id, now);
            let new_signal = NewGovernanceSignal {
                id: &signal_id,
                entity_type: &input.entity_type,
                entity_id: &input.entity_id,
                human_id: &input.human_id,
                signal_type: &input.signal_type,
                signal_value: &input.signal_value,
                mechanism_level: input.mechanism_level,
                proxy_elohim_id: input.proxy_elohim_id.as_deref(),
                created_at: &now,
            };

            // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
            // Classification B2 (Agent-Scoped + Attestation): raw governance signal is private to the agent;
            // dht_anchor_hash set when the aggregate signal Attestation is notarized.
            // Currently null for direct storage writes. Backfill needed for pre-coherence data.
            let mut conn = get_conn(pool)?;
            let result = governance::record_signal(&mut conn, &new_signal)?;
            Ok(response::created(&GovernanceSignalView::from(result)))
        }

        // GET /api/v1/governance/signals/aggregate?entityType=X&entityId=Y — Aggregate signals
        (&Method::GET, "/signals/aggregate") => {
            let params: SignalQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let entity_type = params.entity_type.as_deref().unwrap_or("");
            let entity_id = params.entity_id.as_deref().unwrap_or("");
            if entity_type.is_empty() || entity_id.is_empty() {
                return Ok(response::bad_request(
                    "entityType and entityId query params are required",
                ));
            }
            let mut conn = get_conn(pool)?;
            let aggregate = governance::aggregate_signals(&mut conn, entity_type, entity_id)?;
            Ok(response::ok(&aggregate))
        }

        // GET /api/v1/governance/signals?entityType=X&entityId=Y — List governance signals
        (&Method::GET, "/signals") => {
            let params: SignalQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let entity_type = params.entity_type.as_deref().unwrap_or("");
            let entity_id = params.entity_id.as_deref().unwrap_or("");
            if entity_type.is_empty() || entity_id.is_empty() {
                return Ok(response::bad_request(
                    "entityType and entityId query params are required",
                ));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::query_signals(&mut conn, entity_type, entity_id)?;
            let views: Vec<GovernanceSignalView> = results
                .into_iter()
                .map(GovernanceSignalView::from)
                .collect();
            Ok(response::ok(&views))
        }

        // =================================================================
        // Sensemaking: Statements, Votes, Clustering
        // =================================================================

        // POST /api/v1/governance/sensemaking/statements — Create a statement
        (&Method::POST, "/sensemaking/statements") => {
            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: CreateStatementInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let now = crate::db::models::current_timestamp();
            let statement_id = format!("stmt-{}", uuid::Uuid::new_v4());

            let new = crate::db::models::NewStatement {
                id: &statement_id,
                entity_type: &input.entity_type,
                entity_id: &input.entity_id,
                human_id: &input.human_id,
                text: &input.text,
                created_at: &now,
            };

            // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
            // Classification A (Notarized): statements are community-visible Polis sensemaking entries.
            // Currently null for direct storage writes. Backfill needed for pre-coherence data.
            let mut conn = get_conn(pool)?;
            let statement = governance::create_statement(&mut conn, &new)?;
            Ok(response::created(&StatementView::from(statement)))
        }

        // GET /api/v1/governance/sensemaking/statements?entityType=X&entityId=Y
        (&Method::GET, "/sensemaking/statements") => {
            let params: SensemakingQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let entity_type = params.entity_type.as_deref().unwrap_or("");
            let entity_id = params.entity_id.as_deref().unwrap_or("");
            if entity_type.is_empty() || entity_id.is_empty() {
                return Ok(response::bad_request(
                    "entityType and entityId query params are required",
                ));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::query_statements(&mut conn, entity_type, entity_id)?;
            let views: Vec<StatementView> = results.into_iter().map(StatementView::from).collect();
            Ok(response::ok(&views))
        }

        // POST /api/v1/governance/sensemaking/statements/{id}/vote
        (&Method::POST, p) if p.starts_with("/sensemaking/statements/") && p.ends_with("/vote") => {
            let statement_id = p
                .strip_prefix("/sensemaking/statements/")
                .and_then(|s| s.strip_suffix("/vote"))
                .ok_or_else(|| StorageError::InvalidInput("Statement ID required".to_string()))?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: VoteOnStatementInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            // Validate vote value
            if !["agree", "disagree", "pass"].contains(&input.vote.as_str()) {
                return Ok(response::bad_request(
                    "vote must be one of: agree, disagree, pass",
                ));
            }

            let now = crate::db::models::current_timestamp();
            let vote_id = format!("sv-{}-{}", statement_id, input.human_id);

            let new_vote = crate::db::models::NewStatementVote {
                id: &vote_id,
                statement_id,
                human_id: &input.human_id,
                vote: &input.vote,
                created_at: &now,
            };

            // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
            // Classification B2 (Agent-Scoped + Attestation): statement vote is private to the agent's source chain;
            // dht_anchor_hash set when the clustered aggregate Attestation is notarized.
            // Currently null for direct storage writes. Backfill needed for pre-coherence data.
            let mut conn = get_conn(pool)?;
            let vote = governance::vote_on_statement(&mut conn, &new_vote)?;
            Ok(response::created(&StatementVoteView::from(vote)))
        }

        // GET /api/v1/governance/sensemaking/votes?entityType=X&entityId=Y
        (&Method::GET, "/sensemaking/votes") => {
            let params: SensemakingQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let entity_type = params.entity_type.as_deref().unwrap_or("");
            let entity_id = params.entity_id.as_deref().unwrap_or("");
            if entity_type.is_empty() || entity_id.is_empty() {
                return Ok(response::bad_request(
                    "entityType and entityId query params are required",
                ));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::get_all_votes_for_entity(&mut conn, entity_type, entity_id)?;
            let views: Vec<StatementVoteView> =
                results.into_iter().map(StatementVoteView::from).collect();
            Ok(response::ok(&views))
        }

        // GET /api/v1/governance/sensemaking/clusters?entityType=X&entityId=Y
        (&Method::GET, "/sensemaking/clusters") => {
            let params: SensemakingQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let entity_type = params.entity_type.as_deref().unwrap_or("");
            let entity_id = params.entity_id.as_deref().unwrap_or("");
            if entity_type.is_empty() || entity_id.is_empty() {
                return Ok(response::bad_request(
                    "entityType and entityId query params are required",
                ));
            }
            let mut conn = get_conn(pool)?;
            let stmts = governance::query_statements(&mut conn, entity_type, entity_id)?;
            let votes = governance::get_all_votes_for_entity(&mut conn, entity_type, entity_id)?;
            let result = crate::sensemaking::clustering::cluster_opinions(
                entity_type,
                entity_id,
                &stmts,
                &votes,
            );
            Ok(response::ok(&result))
        }

        // =================================================================
        // Dispositions (Sprint 8: governance profiles)
        // =================================================================

        // POST /api/v1/governance/dispositions/{humanId}/compute — Compute and return
        (&Method::POST, p) if p.starts_with("/dispositions/") && p.ends_with("/compute") => {
            let human_id = p
                .strip_prefix("/dispositions/")
                .and_then(|s| s.strip_suffix("/compute"))
                .ok_or_else(|| StorageError::InvalidInput("Human ID required".to_string()))?;

            let mut conn = get_conn(pool)?;
            let disposition = disposition_service::compute_disposition(&mut conn, human_id)?;
            Ok(response::ok(&GovernanceDispositionView::from(disposition)))
        }

        // GET /api/v1/governance/dispositions/{humanId} — Get current disposition
        (&Method::GET, p) if p.starts_with("/dispositions/") => {
            let sub = p.strip_prefix("/dispositions").unwrap_or("");
            let human_id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Human ID required".to_string()))?;

            let mut conn = get_conn(pool)?;
            let result = governance::get_disposition(&mut conn, human_id)?;
            Ok(response::from_option(
                Ok(result.map(GovernanceDispositionView::from)),
                &format!("Disposition not found for human {}", human_id),
            ))
        }

        // PUT /api/v1/governance/dispositions/{humanId} — Manual override
        (&Method::PUT, p) if p.starts_with("/dispositions/") => {
            let sub = p.strip_prefix("/dispositions").unwrap_or("");
            let human_id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Human ID required".to_string()))?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: UpdateDispositionInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let mut conn = get_conn(pool)?;

            // Ensure disposition exists — compute first if not
            let existing = governance::get_disposition(&mut conn, human_id)?;
            if existing.is_none() {
                disposition_service::compute_disposition(&mut conn, human_id)?;
            }

            let values_json = input
                .priority_values
                .map(|v| serde_json::to_string(&v.0).unwrap_or_default());

            let disposition = governance::update_disposition_overrides(
                &mut conn,
                human_id,
                input.risk_tolerance,
                input.change_openness,
                input.consensus_preference,
                values_json.as_deref(),
            )?;
            Ok(response::ok(&GovernanceDispositionView::from(disposition)))
        }

        // =================================================================
        // Proxy Voting (Sprint 8)
        // =================================================================

        // POST /api/v1/governance/proposals/{id}/proxy-votes — Cast proxy vote
        (&Method::POST, p) if p.starts_with("/proposals/") && p.ends_with("/proxy-votes") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/proxy-votes"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: CastRankedVoteInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            // Validate proxy_elohim_id is set
            if input.proxy_elohim_id.is_none() {
                return Ok(response::bad_request(
                    "proxy_elohim_id is required for proxy votes",
                ));
            }

            let mut conn = get_conn(pool)?;

            // Validate proposal exists
            let proposal = governance::get_proposal(&mut conn, id)?
                .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;

            // Validate human hasn't voted directly
            if governance::has_direct_vote(&mut conn, id, &input.human_id)? {
                return Ok(response::bad_request(
                    "Human has already voted directly — proxy vote not allowed",
                ));
            }

            // Fetch options for validation
            let options = governance::query_proposal_options(&mut conn, id)?;

            // Build VotingConfig from proposal fields
            let config = tally::VotingConfig {
                score_min: proposal.score_min,
                score_max: proposal.score_max,
                dots_per_voter: proposal.dots_per_voter,
                quorum_percentage: proposal.quorum_percentage,
                passage_threshold: proposal.passage_threshold,
            };

            // Build temporary votes for validation
            let now = crate::db::models::current_timestamp();
            let temp_votes: Vec<crate::db::models::RankedVote> = input
                .ballots
                .iter()
                .enumerate()
                .map(|(i, b)| crate::db::models::RankedVote {
                    id: format!("rv-proxy-{}-{}-{}", id, input.human_id, i),
                    proposal_id: id.to_string(),
                    human_id: input.human_id.clone(),
                    option_id: b.option_id.clone(),
                    rank: b.rank,
                    score: b.score,
                    dots: b.dots,
                    approved: b.approved.map(|a| if a { 1 } else { 0 }),
                    reasoning: input.reasoning.clone(),
                    proxy_elohim_id: input.proxy_elohim_id.clone(),
                    proxy_justification: input.proxy_justification.clone(),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    dht_anchor_hash: None,
                })
                .collect();

            // Validate ballot
            let strategy = tally::get_strategy(&proposal.voting_mechanism).ok_or_else(|| {
                StorageError::InvalidInput(format!(
                    "Unknown voting mechanism: {}",
                    proposal.voting_mechanism
                ))
            })?;
            strategy
                .validate_ballot(&temp_votes, &options, &config)
                .map_err(|e| {
                    StorageError::InvalidInput(format!("Ballot validation failed: {}", e))
                })?;

            // Build NewRankedVote entries
            let vote_ids: Vec<String> = input
                .ballots
                .iter()
                .enumerate()
                .map(|(i, _)| format!("rv-proxy-{}-{}-{}", id, input.human_id, i))
                .collect();

            let new_votes: Vec<NewRankedVote> = input
                .ballots
                .iter()
                .enumerate()
                .map(|(i, b)| NewRankedVote {
                    id: &vote_ids[i],
                    proposal_id: id,
                    human_id: &input.human_id,
                    option_id: &b.option_id,
                    rank: b.rank,
                    score: b.score,
                    dots: b.dots,
                    approved: b.approved.map(|a| if a { 1 } else { 0 }),
                    reasoning: input.reasoning.as_deref(),
                    proxy_elohim_id: input.proxy_elohim_id.as_deref(),
                    proxy_justification: input.proxy_justification.as_deref(),
                    created_at: &now,
                    updated_at: &now,
                })
                .collect();

            let results =
                governance::cast_ranked_votes(&mut conn, id, &input.human_id, &new_votes)?;
            let hide = proposal.voting_anonymous == 1;
            let views: Vec<RankedVoteView> = results
                .into_iter()
                .map(|v| crate::views::ranked_vote_view_from_ranked_vote(v, hide))
                .collect();
            Ok(response::created(&views))
        }

        // POST /api/v1/governance/proposals/{id}/override-proxy — Override proxy with direct vote
        (&Method::POST, p) if p.starts_with("/proposals/") && p.ends_with("/override-proxy") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/override-proxy"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
            let input: CastRankedVoteInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            // Override must NOT have proxy_elohim_id (it's a direct vote)
            if input.proxy_elohim_id.is_some() {
                return Ok(response::bad_request(
                    "proxy_elohim_id must not be set for direct override votes",
                ));
            }

            let mut conn = get_conn(pool)?;

            let proposal = governance::get_proposal(&mut conn, id)?
                .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;

            // Delete proxy votes for this human
            let deleted = governance::delete_proxy_votes_for_human(&mut conn, id, &input.human_id)?;

            // Fetch options for validation
            let options = governance::query_proposal_options(&mut conn, id)?;

            let config = tally::VotingConfig {
                score_min: proposal.score_min,
                score_max: proposal.score_max,
                dots_per_voter: proposal.dots_per_voter,
                quorum_percentage: proposal.quorum_percentage,
                passage_threshold: proposal.passage_threshold,
            };

            // Validate the direct ballot
            let now = crate::db::models::current_timestamp();
            let temp_votes: Vec<crate::db::models::RankedVote> = input
                .ballots
                .iter()
                .enumerate()
                .map(|(i, b)| crate::db::models::RankedVote {
                    id: format!("rv-{}-{}-{}", id, input.human_id, i),
                    proposal_id: id.to_string(),
                    human_id: input.human_id.clone(),
                    option_id: b.option_id.clone(),
                    rank: b.rank,
                    score: b.score,
                    dots: b.dots,
                    approved: b.approved.map(|a| if a { 1 } else { 0 }),
                    reasoning: input.reasoning.clone(),
                    proxy_elohim_id: None,
                    proxy_justification: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    dht_anchor_hash: None,
                })
                .collect();

            let strategy = tally::get_strategy(&proposal.voting_mechanism).ok_or_else(|| {
                StorageError::InvalidInput(format!(
                    "Unknown voting mechanism: {}",
                    proposal.voting_mechanism
                ))
            })?;
            strategy
                .validate_ballot(&temp_votes, &options, &config)
                .map_err(|e| {
                    StorageError::InvalidInput(format!("Ballot validation failed: {}", e))
                })?;

            // Cast the direct vote
            let vote_ids: Vec<String> = input
                .ballots
                .iter()
                .enumerate()
                .map(|(i, _)| format!("rv-{}-{}-{}", id, input.human_id, i))
                .collect();

            let new_votes: Vec<NewRankedVote> = input
                .ballots
                .iter()
                .enumerate()
                .map(|(i, b)| NewRankedVote {
                    id: &vote_ids[i],
                    proposal_id: id,
                    human_id: &input.human_id,
                    option_id: &b.option_id,
                    rank: b.rank,
                    score: b.score,
                    dots: b.dots,
                    approved: b.approved.map(|a| if a { 1 } else { 0 }),
                    reasoning: input.reasoning.as_deref(),
                    proxy_elohim_id: None,
                    proxy_justification: None,
                    created_at: &now,
                    updated_at: &now,
                })
                .collect();

            let results =
                governance::cast_ranked_votes(&mut conn, id, &input.human_id, &new_votes)?;

            // Record governance signal for the proxy override
            let signal_id = format!("sig-proxy-override-{}-{}-{}", id, input.human_id, now);
            let new_signal = crate::db::models::NewGovernanceSignal {
                id: &signal_id,
                entity_type: "proposal",
                entity_id: id,
                human_id: &input.human_id,
                signal_type: "proxy-override",
                signal_value: &format!("overrode {} proxy vote(s) with direct vote", deleted),
                mechanism_level: 3, // Level 3: formal voting
                proxy_elohim_id: None,
                created_at: &now,
            };
            governance::record_signal(&mut conn, &new_signal)?;

            let hide = proposal.voting_anonymous == 1;
            let views: Vec<RankedVoteView> = results
                .into_iter()
                .map(|v| crate::views::ranked_vote_view_from_ranked_vote(v, hide))
                .collect();
            Ok(response::created(&views))
        }

        _ => Ok(response::not_found(&format!(
            "Unknown governance route: {} {}",
            method, resource_path
        ))),
    }
}
