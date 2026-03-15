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
use crate::db::models::{NewDiscussion, NewProposal, NewVote};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{
    CastVoteInputView, ChallengeView, CreateDiscussionInputView, CreateProposalInputView,
    DiscussionView, GovernanceStateView, PostMessageInputView, PrecedentView, ProposalView,
    VoteView,
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
struct ProposalQuery {
    pub content_id: Option<String>,
    pub status: Option<String>,
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

        // GET /api/v1/governance/challenges/{id}
        (&Method::GET, p) if p.starts_with("/challenges/") => {
            let sub = p.strip_prefix("/challenges").unwrap_or("");
            let id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Challenge ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = governance::get_challenge(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(ChallengeView::from)),
                &format!("Challenge {} not found", id),
            ))
        }

        // GET /api/v1/governance/challenges?contentId=X
        (&Method::GET, "/challenges") => {
            let params: ContentQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let content_id = params.content_id.as_deref().unwrap_or("");
            if content_id.is_empty() {
                return Ok(response::bad_request("contentId query param is required"));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::query_challenges(&mut conn, content_id)?;
            let views: Vec<ChallengeView> = results.into_iter().map(ChallengeView::from).collect();
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
            };

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

            let vote = governance::cast_vote(&mut conn, &new_vote)?;
            let hide = proposal.voting_anonymous == 1;
            Ok(response::created(&VoteView::from_vote(vote, hide)))
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
                .map(|v| VoteView::from_vote(v, hide))
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
                .ok_or_else(|| {
                    StorageError::InvalidInput("Discussion ID required".to_string())
                })?;

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

        _ => Ok(response::not_found(&format!(
            "Unknown governance route: {} {}",
            method, resource_path
        ))),
    }
}
