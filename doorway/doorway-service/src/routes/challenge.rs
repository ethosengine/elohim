//! `POST /api/v1/challenge` and `GET /api/v1/challenge/{id}` — redress with
//! teeth.
//!
//! The first half of this story's refusal told the visitor where to be heard.
//! This is what happens when they take that offer: the challenge becomes a
//! witnessed REA `Commitment` with a named party who owes an answer and a due
//! date fixed by a term the collective declared before the visitor sent
//! anything. A challenge that lands in a queue nobody owes anything to is the
//! suggestion box this protocol replaces.
//!
//! ## The doorway carries; it does not answer
//!
//! - WHO owes is copied verbatim from the contract's own `responsiveReach`
//!   ([`crate::services::owed_response`]). The doorway never picks a party.
//! - WHICH contract was challenged is read from this doorway's own live router
//!   projection — never a client-supplied commitment id, or a visitor could
//!   witness a challenge against a record nobody projected to them.
//! - WHO is challenging comes from the SAME verifier the fold used to refuse
//!   them (`serve_eligibility::standing_from_request`), so the person who owes
//!   an answer owes it to a real, verified someone. An anonymous challenge is
//!   a named 401: a challenge needs somebody to answer TO.
//! - `carriedBy` on the record is this doorway. That it is a DIFFERENT party
//!   from `provider` is the record that the doorway did not answer for the
//!   collective.
//!
//! ## Ensure-not-create
//!
//! The commitment id is an agent-scoped composite over (challenger, contract,
//! challenge reference), so re-sending the same challenge mints nothing twice
//! and returns the record that already stands
//! ([`ChallengeOutcome::Replay`]). The same reference carrying DIFFERENT words
//! is a conflict, not an overwrite: two challenges may not share one record.
//!
//! ## What does NOT ride this commitment
//!
//! No refused path, no IP, no session id, no user-agent, no prior browsing. A
//! redress record is not a visitor log.
//!
//! ## Relay
//!
//! A courier doorway — one that relayed the bytes and therefore relayed the
//! holder's 403 — holds no contract for the record and so cannot resolve the
//! term. It forwards the challenge ONE hop to the holder, whose fold actually
//! refused, and carries the answer back. The visitor's `authorization` travels
//! with it, because the challenger must stay the verified person they are.

use bytes::Bytes;
use chrono::{DateTime, Utc};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::server::AppState;
use crate::services::name_routing::FEDERATION_HOP_HEADER;
use crate::services::owed_response::{
    challenge_commitment_id, challenge_reference_from_words, due_from_responsive_reach,
    owed_from_contract, OwedResponse, CHALLENGE_ROUTE, MAX_CHALLENGE_WORDS,
    RESPOND_TO_CHALLENGE_ACTION,
};
use crate::services::serve_receipt::ChallengeOutcome;

/// How long a storage read/write may take before the challenge is SHED rather
/// than hung. A visitor told "not now, nothing was recorded" can try again; a
/// visitor left waiting learns nothing.
const LEDGER_TIMEOUT: Duration = Duration::from_secs(8);

/// How long the one-hop relay to a holder may take.
const RELAY_TIMEOUT: Duration = Duration::from_secs(10);

/// Route prefix that reads a commitment back.
const COMMITMENT_RECORD_ROUTE: &str = "/api/v1/commitments/";

/// The fold term a reach refusal names, recorded on the challenge so a reader
/// knows WHICH of the four terms was being challenged.
const TERM_REACH: &str = "reach";

/// What a visitor sends.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeInput {
    /// The record they were refused. The CONTRACT is resolved from this
    /// doorway's own projection of it — never supplied by the client.
    epr: String,
    /// Their words, carried onto the record verbatim.
    words: String,
    /// An optional reference of their own, so a client can retry safely.
    /// Absent ⇒ derived from the words, which makes re-sending the same
    /// sentence a replay rather than a duplicate.
    #[serde(default)]
    challenge_id: Option<String>,
}

/// What the chrome reads back — the same shape for every outcome, so a client
/// never has to branch on a status code to find out what happened.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeAnswer {
    outcome: ChallengeOutcome,
    /// Why, in the words a friend would use.
    reason: String,
    /// The record this doorway was asked about.
    epr: String,
    /// The commitment that stands, when one does.
    #[serde(skip_serializing_if = "Option::is_none")]
    challenge: Option<String>,
    /// Where to read that commitment back.
    #[serde(skip_serializing_if = "Option::is_none")]
    record: Option<String>,
    /// WHO owes the answer, as the contract declares it. `null` when the
    /// collective declared nobody — never a party this doorway chose.
    owed: Option<OwedResponse>,
    /// When the answer comes due.
    #[serde(skip_serializing_if = "Option::is_none")]
    due: Option<String>,
    /// The contract that was challenged.
    #[serde(skip_serializing_if = "Option::is_none")]
    contract: Option<String>,
    /// The doorway that CARRIED this challenge. Different from the owed party
    /// by construction — the record that no doorway answered for a collective.
    #[serde(skip_serializing_if = "Option::is_none")]
    carried_by: Option<String>,
    /// The holder that answered, when this doorway relayed.
    #[serde(skip_serializing_if = "Option::is_none")]
    served_by: Option<String>,
}

/// What a standing challenge looks like when read back.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeStanding {
    challenge: String,
    record: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    contract: Option<String>,
    owed: Option<OwedResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    due: Option<String>,
    /// The lifecycle state the ledger holds.
    state: String,
    finished: bool,
    /// Whether the due date has passed with no answer recorded.
    ///
    /// DERIVED AT READ TIME on this doorway's clock, and honestly local: there
    /// is no witnessed overdue record yet, so this is a reading, not a claim
    /// anyone else has attested. `null` when no due date stands.
    #[serde(skip_serializing_if = "Option::is_none")]
    overdue: Option<bool>,
    /// The visitor's own words, as the record carries them.
    #[serde(skip_serializing_if = "Option::is_none")]
    words: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    carried_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    served_by: Option<String>,
}

/// Route match for `GET /api/v1/challenge/{id}`.
#[must_use]
pub fn match_challenge_read(path: &str) -> Option<String> {
    let rest = path.strip_prefix(CHALLENGE_ROUTE)?.strip_prefix('/')?;
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    let decoded = urlencoding::decode(rest).ok()?.into_owned();
    let trimmed = decoded.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

// ════════════════════════════════════════════════════════════════════════════
// POST — witness a challenge
// ════════════════════════════════════════════════════════════════════════════

/// Handle `POST /api/v1/challenge`.
pub async fn handle_challenge_post(
    state: Arc<AppState>,
    req: Request<Incoming>,
    inbound_hop: bool,
) -> Response<Full<Bytes>> {
    // WHO is challenging — from the same verifier the fold used to refuse them.
    // Resolved BEFORE the body is read: an anonymous challenge is refused for
    // what it lacks, not for what it says.
    let standing = crate::services::standing_from_request(&state, &req);
    let challenger = match (standing.authenticated, standing.human_id.clone()) {
        (true, Some(id)) if !id.trim().is_empty() => id,
        _ => {
            return answer(
                ChallengeOutcome::Anonymous,
                ChallengeAnswer {
                    outcome: ChallengeOutcome::Anonymous,
                    reason: ChallengeOutcome::Anonymous.reason().to_string(),
                    epr: String::new(),
                    challenge: None,
                    record: None,
                    owed: None,
                    due: None,
                    contract: None,
                    carried_by: state.args.doorway_id.clone(),
                    served_by: None,
                },
            );
        }
    };

    let authorization = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let Ok(collected) = req.collect().await else {
        return bad_request("This doorway could not read your challenge. Please send it again.");
    };
    let body_bytes = collected.to_bytes();
    let Ok(input) = serde_json::from_slice::<ChallengeInput>(&body_bytes) else {
        return bad_request(
            "A challenge needs to say which record it is about and what you want to say \
             about it.",
        );
    };
    let epr_id = input.epr.trim().to_string();
    let words = input.words.trim().to_string();
    if epr_id.is_empty() || words.is_empty() {
        return bad_request(
            "A challenge needs to say which record it is about and what you want to say \
             about it.",
        );
    }
    if words.chars().count() > MAX_CHALLENGE_WORDS {
        return bad_request(
            "That is longer than this doorway can put on the record. Please say the heart \
             of it in fewer words — the answer you are owed is to the point you are making.",
        );
    }

    // WHICH contract — this doorway's own live projection, never the client's word.
    let Some(projection) = state.epr_router.projection_for_epr_id(&epr_id) else {
        if inbound_hop {
            // We are already the second doorway. Answering "not found" is the
            // honest end of the budget; forwarding again is the loop the budget
            // exists to refuse.
            return not_found(&epr_id, state.args.doorway_id.as_deref());
        }
        return relay_challenge_post(&state, &epr_id, &body_bytes, authorization.as_deref()).await;
    };

    let carried_by = state.args.doorway_id.clone();
    let contract_id = projection.commitment_id.clone();

    // WHO owes — the contract's declaration, or nobody.
    let Some(owed) = owed_from_contract(&projection) else {
        return answer(
            ChallengeOutcome::NothingOwed,
            ChallengeAnswer {
                outcome: ChallengeOutcome::NothingOwed,
                reason: ChallengeOutcome::NothingOwed.reason().to_string(),
                epr: epr_id,
                challenge: None,
                record: None,
                owed: None,
                due: None,
                contract: Some(contract_id),
                carried_by,
                served_by: None,
            },
        );
    };

    let term = projection
        .responsive_reach
        .as_ref()
        .expect("owed_from_contract returned Some, so the term is present");
    let received_at: DateTime<Utc> = Utc::now();
    let due = due_from_responsive_reach(term, received_at).map(|d| d.to_rfc3339());

    let reference = match input.challenge_id.as_deref().map(str::trim) {
        Some(r) if !r.is_empty() => r.to_string(),
        _ => challenge_reference_from_words(&words),
    };
    let commitment_id = challenge_commitment_id(&challenger, &contract_id, &reference);

    // Ensure-not-create: a replay mints nothing twice.
    if let Some(existing) = read_commitment(&state, &commitment_id).await {
        let recorded = existing.get("note").and_then(|n| n.as_str()).unwrap_or("");
        let outcome = if recorded.trim() == words {
            ChallengeOutcome::Replay
        } else {
            ChallengeOutcome::Conflict
        };
        return answer(
            outcome,
            ChallengeAnswer {
                outcome,
                reason: outcome.reason().to_string(),
                epr: epr_id,
                challenge: Some(commitment_id.clone()),
                record: Some(record_route(&commitment_id)),
                due: existing
                    .get("due")
                    .and_then(|d| d.as_str())
                    .map(str::to_string)
                    .or(due),
                owed: Some(owed),
                contract: Some(contract_id),
                carried_by,
                served_by: None,
            },
        );
    }

    // The metadata a reader needs to check the window without trusting us: the
    // term challenged, the reach it was refused at, and the record + timestamp
    // that DECLARED the window — all of which predate this request.
    let metadata = serde_json::json!({
        "challengeId": reference,
        "term": TERM_REACH,
        "reach": projection.reach,
        "partyLabel": owed.party_label,
        "withinHours": owed.within_hours,
        "declaredBy": owed.declared_by,
        "declaredAt": owed.declared_at,
        // Not the provider, deliberately. A doorway that carried a challenge is
        // not a doorway that answered it.
        "carriedBy": carried_by,
    });

    let body = serde_json::json!({
        "id": commitment_id,
        "action": RESPOND_TO_CHALLENGE_ACTION,
        // VERBATIM from the contract. The doorway never sets who owes.
        "provider": owed.party,
        // The person the answer is owed TO.
        "receiver": challenger,
        "clauseOf": contract_id,
        "hasBeginning": received_at.to_rfc3339(),
        "due": due,
        "note": words,
        "inScopeOf": [format!("epr:{epr_id}"), format!("contract:{contract_id}")],
        "resourceClassifiedAs": ["stewardship"],
        "metadata": metadata,
    });

    match write_commitment(&state, &body, authorization.as_deref()).await {
        Some(status) if status == 201 || status == 200 => answer(
            ChallengeOutcome::Witnessed,
            ChallengeAnswer {
                outcome: ChallengeOutcome::Witnessed,
                reason: ChallengeOutcome::Witnessed.reason().to_string(),
                epr: epr_id,
                challenge: Some(commitment_id.clone()),
                record: Some(record_route(&commitment_id)),
                owed: Some(owed),
                due,
                contract: Some(contract_id),
                carried_by,
                served_by: None,
            },
        ),
        // A 409 here is the race the ensure-not-create check above just lost:
        // somebody witnessed the same challenge between the read and the write.
        // That is a replay, not a failure.
        Some(409) => answer(
            ChallengeOutcome::Replay,
            ChallengeAnswer {
                outcome: ChallengeOutcome::Replay,
                reason: ChallengeOutcome::Replay.reason().to_string(),
                epr: epr_id,
                challenge: Some(commitment_id.clone()),
                record: Some(record_route(&commitment_id)),
                owed: Some(owed),
                due,
                contract: Some(contract_id),
                carried_by,
                served_by: None,
            },
        ),
        _ => answer(
            ChallengeOutcome::Shed,
            ChallengeAnswer {
                outcome: ChallengeOutcome::Shed,
                reason: ChallengeOutcome::Shed.reason().to_string(),
                epr: epr_id,
                challenge: None,
                record: None,
                owed: Some(owed),
                due: None,
                contract: Some(contract_id),
                carried_by,
                served_by: None,
            },
        ),
    }
}

// ════════════════════════════════════════════════════════════════════════════
// GET — read the owed response and its due date back
// ════════════════════════════════════════════════════════════════════════════

/// Handle `GET /api/v1/challenge/{commitmentId}` — the standing record, read
/// LIVE so state, finished and overdue are current rather than whatever was
/// true when the challenge was sent.
pub async fn handle_challenge_read(
    state: Arc<AppState>,
    commitment_id: &str,
    inbound_hop: bool,
) -> Response<Full<Bytes>> {
    let (row, served_by) = match read_commitment(&state, commitment_id).await {
        Some(row) => (row, None),
        None if inbound_hop => return challenge_not_found(commitment_id),
        None => match relay_challenge_read(&state, commitment_id).await {
            Some((row, origin)) => (row, Some(origin)),
            None => return challenge_not_found(commitment_id),
        },
    };

    let metadata = row
        .get("metadata")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let due = row.get("due").and_then(|d| d.as_str()).map(str::to_string);
    let finished = row
        .get("finished")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let overdue = due.as_deref().and_then(|d| {
        DateTime::parse_from_rfc3339(d)
            .ok()
            .map(|deadline| !finished && Utc::now() > deadline.with_timezone(&Utc))
    });

    let owed = metadata
        .get("withinHours")
        .and_then(serde_json::Value::as_u64)
        .map(|hours| OwedResponse {
            party: row
                .get("provider")
                .and_then(|p| p.as_str())
                .unwrap_or_default()
                .to_string(),
            party_label: metadata
                .get("partyLabel")
                .and_then(|l| l.as_str())
                .map(str::to_string),
            within_hours: u32::try_from(hours).unwrap_or(u32::MAX),
            declared_by: metadata
                .get("declaredBy")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string(),
            declared_at: metadata
                .get("declaredAt")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string(),
        });

    json_response(
        StatusCode::OK,
        &ChallengeStanding {
            challenge: commitment_id.to_string(),
            record: record_route(commitment_id),
            contract: row
                .get("clauseOf")
                .and_then(|c| c.as_str())
                .map(str::to_string),
            owed,
            due,
            state: row
                .get("state")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string(),
            finished,
            overdue,
            words: row.get("note").and_then(|n| n.as_str()).map(str::to_string),
            carried_by: metadata
                .get("carriedBy")
                .and_then(|c| c.as_str())
                .map(str::to_string),
            served_by,
        },
    )
}

// ════════════════════════════════════════════════════════════════════════════
// The reads and the writes
// ════════════════════════════════════════════════════════════════════════════

async fn read_commitment(state: &AppState, id: &str) -> Option<serde_json::Value> {
    let storage = state.args.storage_url.as_deref()?.trim_end_matches('/');
    let url = format!(
        "{storage}{COMMITMENT_RECORD_ROUTE}{}",
        urlencoding::encode(id)
    );
    let resp = state
        .storage_proxy_client
        .get(&url)
        .timeout(LEDGER_TIMEOUT)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json().await.ok()
}

/// Returns the storage status code, or `None` when the write could not be made
/// at all. A `None` is a SHED, never a silent success.
async fn write_commitment(
    state: &AppState,
    body: &serde_json::Value,
    authorization: Option<&str>,
) -> Option<u16> {
    let storage = state.args.storage_url.as_deref()?.trim_end_matches('/');
    let url = format!("{storage}/api/v1/commitments");
    let mut request = state
        .storage_proxy_client
        .post(&url)
        .header("content-type", "application/json")
        .timeout(LEDGER_TIMEOUT)
        .json(body);
    if let Some(auth) = authorization {
        request = request.header("authorization", auth);
    }
    match request.send().await {
        Ok(resp) => Some(resp.status().as_u16()),
        Err(e) => {
            tracing::warn!(error = %e, "challenge: the ledger write could not be made — shedding");
            None
        }
    }
}

/// Forward one challenge to the holder whose fold actually refused. ONE hop.
async fn relay_challenge_post(
    state: &AppState,
    epr_id: &str,
    body: &[u8],
    authorization: Option<&str>,
) -> Response<Full<Bytes>> {
    let self_id = state.args.doorway_id.as_deref().unwrap_or("");
    for holder in state.name_routes.all_holders(self_id) {
        let url = format!("{}{CHALLENGE_ROUTE}", holder.origin.trim_end_matches('/'));
        let mut request = state
            .storage_proxy_client
            .post(&url)
            .header("content-type", "application/json")
            .header(FEDERATION_HOP_HEADER, "1")
            .timeout(RELAY_TIMEOUT)
            .body(body.to_vec());
        if let Some(auth) = authorization {
            request = request.header("authorization", auth);
        }
        let Ok(resp) = request.send().await else {
            continue;
        };
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            // That holder does not project it either — try the next.
            continue;
        }
        let Ok(bytes) = resp.bytes().await else {
            continue;
        };
        // The holder's answer, carried verbatim — including its outcome, its
        // owed party and its due date. A courier does not re-decide any of them.
        return Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .header("cache-control", "no-store")
            .header(
                crate::services::name_routing::SERVED_BY_HEADER,
                holder.origin.clone(),
            )
            .body(Full::new(Bytes::from(bytes.to_vec())))
            .unwrap_or_else(|_| not_found(epr_id, Some(self_id)));
    }
    not_found(epr_id, Some(self_id))
}

/// Read one challenge back from the holder that witnessed it. ONE hop.
async fn relay_challenge_read(
    state: &AppState,
    commitment_id: &str,
) -> Option<(serde_json::Value, String)> {
    let self_id = state.args.doorway_id.as_deref().unwrap_or("");
    for holder in state.name_routes.all_holders(self_id) {
        let url = format!(
            "{}{COMMITMENT_RECORD_ROUTE}{}",
            holder.origin.trim_end_matches('/'),
            urlencoding::encode(commitment_id)
        );
        let Ok(resp) = state
            .storage_proxy_client
            .get(&url)
            .header(FEDERATION_HOP_HEADER, "1")
            .timeout(RELAY_TIMEOUT)
            .send()
            .await
        else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        if let Ok(row) = resp.json::<serde_json::Value>().await {
            return Some((row, holder.origin.clone()));
        }
    }
    None
}

// ════════════════════════════════════════════════════════════════════════════
// Responses
// ════════════════════════════════════════════════════════════════════════════

fn record_route(commitment_id: &str) -> String {
    format!(
        "{COMMITMENT_RECORD_ROUTE}{}",
        urlencoding::encode(commitment_id.trim())
    )
}

fn answer(outcome: ChallengeOutcome, body: ChallengeAnswer) -> Response<Full<Bytes>> {
    json_response(
        StatusCode::from_u16(outcome.status()).unwrap_or(StatusCode::OK),
        &body,
    )
}

fn json_response<T: Serialize>(status: StatusCode, body: &T) -> Response<Full<Bytes>> {
    let bytes = serde_json::to_vec(body).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("cache-control", "no-store")
        .body(Full::new(Bytes::from(bytes)))
        .expect("infallible challenge response")
}

fn bad_request(reason: &str) -> Response<Full<Bytes>> {
    json_response(
        StatusCode::BAD_REQUEST,
        &serde_json::json!({ "reason": reason }),
    )
}

fn not_found(epr_id: &str, doorway_id: Option<&str>) -> Response<Full<Bytes>> {
    json_response(
        StatusCode::NOT_FOUND,
        &serde_json::json!({
            "reason": "This doorway holds no record of that, so it has nobody to carry a \
                       challenge to. Try the doorway that showed it to you.",
            "epr": epr_id,
            "carriedBy": doorway_id,
        }),
    )
}

fn challenge_not_found(commitment_id: &str) -> Response<Full<Bytes>> {
    json_response(
        StatusCode::NOT_FOUND,
        &serde_json::json!({
            "reason": "No challenge stands under that reference here.",
            "challenge": commitment_id,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_read_route_matches_exactly_one_segment() {
        assert_eq!(
            match_challenge_read("/api/v1/challenge/respond-to-challenge-abc123").as_deref(),
            Some("respond-to-challenge-abc123")
        );
        assert!(
            match_challenge_read("/api/v1/challenge").is_none(),
            "the bare route is the POST surface, never a read of a challenge called ''"
        );
        assert!(match_challenge_read("/api/v1/challenge/").is_none());
        assert!(match_challenge_read("/api/v1/challenge/a/b").is_none());
        assert!(match_challenge_read("/api/v1/challenges/x").is_none());
    }

    /// Every outcome answers with the status the closed vocabulary declares —
    /// the route never invents one beside the enum.
    #[test]
    fn each_outcome_answers_with_its_declared_status() {
        for (outcome, want) in [
            (ChallengeOutcome::Witnessed, 201),
            (ChallengeOutcome::NothingOwed, 200),
            (ChallengeOutcome::Replay, 200),
            (ChallengeOutcome::Anonymous, 401),
            (ChallengeOutcome::Conflict, 409),
            (ChallengeOutcome::Shed, 503),
        ] {
            let response = answer(
                outcome,
                ChallengeAnswer {
                    outcome,
                    reason: outcome.reason().to_string(),
                    epr: "community-garden-club".into(),
                    challenge: None,
                    record: None,
                    owed: None,
                    due: None,
                    contract: None,
                    carried_by: Some("apex-elohim-host".into()),
                    served_by: None,
                },
            );
            assert_eq!(response.status().as_u16(), want, "{outcome:?}");
            assert_eq!(
                response.headers().get("cache-control").unwrap(),
                "no-store",
                "a redress answer is this person's, at this moment — never shared-cacheable"
            );
        }
    }

    /// `owed` is present-or-null on the wire, NEVER omitted: a chrome that saw
    /// no `owed` key could not tell "nobody owes you an answer here" from "this
    /// doorway forgot to say".
    #[test]
    fn nothing_owed_says_owed_null_out_loud_rather_than_omitting_it() {
        let body = ChallengeAnswer {
            outcome: ChallengeOutcome::NothingOwed,
            reason: ChallengeOutcome::NothingOwed.reason().to_string(),
            epr: "community-garden-club".into(),
            challenge: None,
            record: None,
            owed: None,
            due: None,
            contract: Some("project-epr-garden".into()),
            carried_by: Some("apex-elohim-host".into()),
            served_by: None,
        };
        let json = serde_json::to_value(&body).unwrap();
        assert!(json.get("owed").is_some(), "the key must be present");
        assert!(json["owed"].is_null(), "and it must be null");
        assert!(
            json.get("challenge").is_none(),
            "no commitment stands, so none is named"
        );
        assert!(!json["reason"].as_str().unwrap().is_empty());
    }

    #[test]
    fn a_witnessed_answer_names_the_record_and_the_doorway_that_carried_it() {
        let owed = OwedResponse {
            party: "household-dowell".into(),
            party_label: Some("the Dowell household".into()),
            within_hours: 72,
            declared_by: "/api/v1/commitments/project-epr-garden".into(),
            declared_at: "2026-09-01T00:00:00Z".into(),
        };
        let body = ChallengeAnswer {
            outcome: ChallengeOutcome::Witnessed,
            reason: ChallengeOutcome::Witnessed.reason().to_string(),
            epr: "community-garden-club".into(),
            challenge: Some("respond-to-challenge-abc1234567890def".into()),
            record: Some(record_route("respond-to-challenge-abc1234567890def")),
            owed: Some(owed),
            due: Some("2026-09-16T10:00:00+00:00".into()),
            contract: Some("project-epr-garden".into()),
            carried_by: Some("apex-elohim-host".into()),
            served_by: None,
        };
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["owed"]["party"], "household-dowell");
        assert_eq!(
            json["record"],
            "/api/v1/commitments/respond-to-challenge-abc1234567890def"
        );
        assert_ne!(
            json["carriedBy"], json["owed"]["party"],
            "the doorway that carried a challenge is never the party that owes the answer"
        );
        assert_eq!(
            json["owed"]["declaredBy"], "/api/v1/commitments/project-epr-garden",
            "the window reads back to the record that declared it"
        );
    }

    #[test]
    fn the_record_route_is_percent_encoded() {
        let route = record_route("a b/../admin");
        assert!(route.starts_with("/api/v1/commitments/"));
        assert!(!route.contains(' '));
        assert!(!route.contains("/../"));
    }
}
