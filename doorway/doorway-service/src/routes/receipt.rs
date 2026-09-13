//! `GET /api/v1/receipt/{eprId}` — the fair-trade receipt for a serve.
//!
//! The chrome's answer to "what was traded for my visit?", read from REA
//! commitments that already stood before the request arrived. The shaping is
//! [`crate::services::serve_receipt`]; this module is the three thin reads that
//! turn a live request into that pure function's arguments, plus the ONE-HOP
//! merge that keeps a courier doorway from crediting itself for a holder's work.
//!
//! ## Why the receipt is doorway-operational, not a storage route
//!
//! The answer depends on WHICH doorway was asked and WHICH holder relayed the
//! bytes — the same class of per-edge, per-request fact
//! [`crate::routes::federation`] answers. Nothing here is stored, nothing is
//! notarized, and a restart costs exactly one re-read.
//!
//! ## The merge, and the line it draws
//!
//! When this doorway holds the contract, all three clauses are its own read.
//! When it does NOT — it relayed the bytes from a holder — then:
//!
//! - `exchanged` and the `held` credit come from **the holder's own receipt**,
//!   fetched over the same one-hop budget the byte relay uses. They are the
//!   holder's facts about the holder's work.
//! - the `projected` credit is **this doorway's own `operate-doorway` binding**,
//!   substituted for whatever the holder said about its own projection.
//!
//! So "a doorway that credits itself for a holder's work" is unreachable rather
//! than discouraged: the held credit is never something this doorway can author,
//! and the projected credit is never something it can take from the holder.
//!
//! ## Unauthenticated on purpose
//!
//! A receipt names who was CREDITED, never who was SERVED — there is no visitor
//! in it to protect, and a person who was just refused should still be able to
//! read what the place they were refused at trades in.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

use crate::server::AppState;
use crate::services::name_routing::FEDERATION_HOP_HEADER;
use crate::services::serve_receipt::{
    build_receipt, ExchangeClause, HeldClause, ProjectedClause, CLAUSE_EXCHANGED, CLAUSE_HELD,
    CLAUSE_PROJECTED, PROTOCOL_FORM_QUERY, RECEIPT_ROUTE_PREFIX,
};

/// How long any one ledger read may take. Short: this sits behind a chrome
/// affordance, and a read that does not answer becomes a NAMED absence rather
/// than a hung page.
const LEDGER_READ_TIMEOUT: Duration = Duration::from_secs(3);

/// How long the one-hop holder fetch may take. Longer than a local read (it is
/// a whole doorway round-trip) and still bounded.
const HOLDER_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Ceiling on the commitment list a scope search will walk. A receipt reads a
/// handful of rows; a doorway that walked an unbounded ledger to render a
/// chrome line would be one slow query from an unserveable page.
const SCOPE_SEARCH_LIMIT: usize = 200;

/// REA actions this route reads. Named here so the three clauses of a receipt
/// and the three actions they read are visibly the same list.
const HOSTING_AGREEMENT_ACTION: &str = "hosting-agreement";
const OPERATE_DOORWAY_ACTION: &str = "operate-doorway";

/// Commitment states that mean "this is no longer the operative term".
///
/// Deliberately a NOT-list rather than `state == "active"`: a commitment is
/// born `created` by REA convention and only graduates to `active` once a
/// lifecycle transition runs, so an `active`-only filter would report a
/// perfectly live, freshly-notarized agreement as `unrecorded` — a false
/// absence, which is the same lie as a false credit pointing the other way.
/// This mirrors storage's own definition of active for projection commitments
/// (`find_active_projections`).
const TERMINAL_STATES: &[&str] = &["cancelled", "terminated", "superseded"];

/// The protocol form: the underlying commitments, verbatim, exactly as the
/// ledger holds them. One request away from the sentence, never in front of it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProtocolForm {
    epr: String,
    /// The `ReaCommitmentView`s this receipt reads, untouched.
    commitments: Vec<serde_json::Value>,
    /// The clauses with no record to show, by name.
    unrecorded: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    served_by: Option<String>,
}

/// Route match: `/api/v1/receipt/{eprId}`. Returns the (percent-decoded) EPR id
/// when the path is one of ours.
#[must_use]
pub fn match_receipt_route(path: &str) -> Option<String> {
    let rest = path.strip_prefix(RECEIPT_ROUTE_PREFIX)?;
    // ONE segment. `/api/v1/receipt/a/b` is not a receipt for `a`.
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

/// Whether the query asked for the protocol form.
#[must_use]
pub fn wants_protocol_form(query: Option<&str>) -> bool {
    query.is_some_and(|q| q.split('&').any(|pair| pair == PROTOCOL_FORM_QUERY))
}

/// Handle `GET /api/v1/receipt/{eprId}`.
pub async fn handle_receipt_request(
    state: Arc<AppState>,
    epr_id: &str,
    query: Option<&str>,
    inbound_hop: bool,
) -> Response<Full<Bytes>> {
    let protocol = wants_protocol_form(query);

    // What this doorway can say about its OWN projection of this record. `None`
    // means it holds no contract for it — it is a courier, not the holder.
    let local = state.epr_router.projection_for_epr_id(epr_id);

    // This doorway's own projection credit, always its own read. Never taken
    // from a holder's answer, and never handed to one.
    let own_operator = read_own_operator_binding(&state).await;

    // The holder's half. Local when we hold the contract; one hop when we do
    // not — and NOTHING when the hop budget is already spent, because a courier
    // may not go looking for a second courier.
    let (held, exchanged, holder_commitments, served_by) = if let Some(projection) = local.as_ref()
    {
        let held = Some(HeldClause {
            commitment_id: projection.commitment_id.clone(),
            party: projection.seeded_by.clone(),
            party_label: None,
        });
        let exchanged = read_hosting_agreement(&state, projection, epr_id).await;
        let mut verbatim = Vec::new();
        if protocol {
            if let Some(v) = read_commitment(&state, &projection.commitment_id).await {
                verbatim.push(v);
            }
            if let Some(clause) = exchanged.as_ref() {
                if let Some(v) = read_commitment(&state, &clause.commitment_id).await {
                    verbatim.push(v);
                }
            }
        }
        (held, exchanged, verbatim, None)
    } else if inbound_hop {
        // The hop budget is already spent: we ARE the second doorway. A courier
        // that went looking for another courier is the loop the budget exists
        // to refuse, so the holder's half is honestly absent here.
        (None, None, Vec::new(), None)
    } else {
        relay_holder_half(&state, epr_id, protocol).await
    };

    if protocol {
        return protocol_response(
            epr_id,
            holder_commitments,
            own_operator.as_ref(),
            &state,
            served_by,
        )
        .await;
    }

    let receipt = build_receipt(
        epr_id,
        exchanged.as_ref(),
        held.as_ref(),
        own_operator.as_ref(),
        served_by.as_deref(),
    );
    json_response(StatusCode::OK, &receipt)
}

/// The protocol form: the holder's verbatim commitments plus THIS doorway's own
/// operator binding — never the holder's, which is a fact about the holder's
/// doorway and not about the one that answered.
async fn protocol_response(
    epr_id: &str,
    mut commitments: Vec<serde_json::Value>,
    own_operator: Option<&ProjectedClause>,
    state: &AppState,
    served_by: Option<String>,
) -> Response<Full<Bytes>> {
    commitments
        .retain(|c| c.get("action").and_then(|a| a.as_str()) != Some(OPERATE_DOORWAY_ACTION));

    let mut unrecorded: Vec<&'static str> = Vec::new();
    if !commitments
        .iter()
        .any(|c| c.get("action").and_then(|a| a.as_str()) == Some("project-epr"))
    {
        unrecorded.push(CLAUSE_HELD);
    }
    if !commitments
        .iter()
        .any(|c| c.get("action").and_then(|a| a.as_str()) == Some(HOSTING_AGREEMENT_ACTION))
    {
        unrecorded.push(CLAUSE_EXCHANGED);
    }

    match own_operator {
        Some(clause) => match read_commitment(state, &clause.commitment_id).await {
            Some(v) => commitments.push(v),
            None => unrecorded.push(CLAUSE_PROJECTED),
        },
        None => unrecorded.push(CLAUSE_PROJECTED),
    }

    json_response(
        StatusCode::OK,
        &ProtocolForm {
            epr: epr_id.to_string(),
            commitments,
            unrecorded,
            served_by,
        },
    )
}

/// Ask each candidate holder, in the fold's order, for its OWN receipt. One hop,
/// first answer wins.
///
/// Returns the holder's `held` + `exchanged` clauses, its verbatim commitments
/// when the protocol form was asked for, and the origin that answered. Nothing
/// about this doorway's own projection comes back through here.
async fn relay_holder_half(
    state: &AppState,
    epr_id: &str,
    protocol: bool,
) -> (
    Option<HeldClause>,
    Option<ExchangeClause>,
    Vec<serde_json::Value>,
    Option<String>,
) {
    let self_id = state.args.doorway_id.as_deref().unwrap_or("");
    let holders = state.name_routes.all_holders(self_id);
    for holder in holders {
        let mut url = format!(
            "{}{RECEIPT_ROUTE_PREFIX}{}",
            holder.origin.trim_end_matches('/'),
            urlencoding::encode(epr_id)
        );
        if protocol {
            url.push('?');
            url.push_str(PROTOCOL_FORM_QUERY);
        }
        let reply = state
            .storage_proxy_client
            .get(&url)
            .header(FEDERATION_HOP_HEADER, "1")
            .timeout(HOLDER_READ_TIMEOUT)
            .send()
            .await;
        let Ok(reply) = reply else { continue };
        if !reply.status().is_success() {
            continue;
        }
        let Ok(body) = reply.json::<serde_json::Value>().await else {
            continue;
        };
        if protocol {
            let commitments = body
                .get("commitments")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();
            if commitments.is_empty() {
                continue;
            }
            return (None, None, commitments, Some(holder.origin.clone()));
        }
        let held = held_from_holder_receipt(&body);
        let exchanged = exchange_from_holder_receipt(&body);
        if held.is_none() && exchanged.is_none() {
            // The holder had nothing to say either; keep asking, and if nobody
            // does, the receipt reports the absence by name.
            continue;
        }
        return (held, exchanged, Vec::new(), Some(holder.origin.clone()));
    }
    (None, None, Vec::new(), None)
}

/// The `held` credit out of a holder's own receipt. Read by ROLE, never by
/// position — a holder that reordered its credits must not shift which one this
/// doorway reads as the holder's.
fn held_from_holder_receipt(body: &serde_json::Value) -> Option<HeldClause> {
    let credit = body
        .get("credits")?
        .as_array()?
        .iter()
        .find(|c| c.get("role").and_then(|r| r.as_str()) == Some("held"))?;
    Some(HeldClause {
        commitment_id: credit.get("commitment")?.as_str()?.to_string(),
        party: credit.get("party")?.as_str()?.to_string(),
        party_label: credit
            .get("partyLabel")
            .and_then(|l| l.as_str())
            .map(str::to_string),
    })
}

/// The `exchanged` clause out of a holder's own receipt.
fn exchange_from_holder_receipt(body: &serde_json::Value) -> Option<ExchangeClause> {
    let exchanged = body.get("exchanged")?;
    if exchanged.is_null() {
        return None;
    }
    Some(ExchangeClause {
        commitment_id: exchanged.get("commitment")?.as_str()?.to_string(),
        // The holder already said WHO gives it, in its own sentence; re-derive
        // nothing. The party is carried so this doorway's sentence says the
        // same name the holder's record does.
        party: exchanged
            .get("party")
            .and_then(|p| p.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| {
                exchanged
                    .get("what")
                    .and_then(|w| w.as_str())
                    .unwrap_or("the steward")
                    .to_string()
            }),
        resource: exchanged
            .get("resource")
            .and_then(|r| r.as_str())
            .map(str::to_string),
    })
}

// ════════════════════════════════════════════════════════════════════════════
// The ledger reads
// ════════════════════════════════════════════════════════════════════════════

/// This doorway's own `operate-doorway` binding — who runs the doorway that
/// answered this request.
///
/// Scoped to `doorway:{own id}`: a doorway reads its OWN operator and nobody
/// else's, which is why a merge can never hand one doorway another's credit.
async fn read_own_operator_binding(state: &AppState) -> Option<ProjectedClause> {
    let doorway_id = state.args.doorway_id.as_deref()?.trim();
    if doorway_id.is_empty() {
        return None;
    }
    let needle = format!("doorway:{doorway_id}");
    let row = find_commitment_in_scope(state, OPERATE_DOORWAY_ACTION, &needle).await?;
    Some(ProjectedClause {
        commitment_id: row.get("id")?.as_str()?.to_string(),
        party: row.get("provider")?.as_str()?.to_string(),
        party_label: None,
        doorway_id: doorway_id.to_string(),
    })
}

/// The `hosting-agreement` that bounds this projection.
///
/// The contract's own `hostingAgreementId` is authoritative when the steward
/// declared one — a pointer the collective wrote beats a scope guess this
/// doorway made. Only when it declared none does this fall back to the
/// `epr_root:{id}` scope search, and a miss there is reported as the named
/// absence `unrecorded: ["exchanged"]` rather than as nothing at all.
async fn read_hosting_agreement(
    state: &AppState,
    projection: &elohim_views::projection::EprProjectionView,
    epr_id: &str,
) -> Option<ExchangeClause> {
    let row = match projection.hosting_agreement_id.as_deref().map(str::trim) {
        Some(declared) if !declared.is_empty() => read_commitment(state, declared).await?,
        _ => {
            let needle = format!("epr_root:{epr_id}");
            find_commitment_in_scope(state, HOSTING_AGREEMENT_ACTION, &needle).await?
        }
    };
    Some(ExchangeClause {
        commitment_id: row.get("id")?.as_str()?.to_string(),
        party: row.get("provider")?.as_str()?.to_string(),
        resource: row
            .get("resourceClassifiedAs")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

/// One commitment, by id, as the ledger holds it.
async fn read_commitment(state: &AppState, id: &str) -> Option<serde_json::Value> {
    let storage = state.args.storage_url.as_deref()?.trim_end_matches('/');
    let url = format!("{storage}/api/v1/commitments/{}", urlencoding::encode(id));
    let resp = state
        .storage_proxy_client
        .get(&url)
        .timeout(LEDGER_READ_TIMEOUT)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let row: serde_json::Value = resp.json().await.ok()?;
    is_operative(&row).then_some(row)
}

/// The first non-terminal commitment of `action` whose `inScopeOf` names
/// `needle`.
async fn find_commitment_in_scope(
    state: &AppState,
    action: &str,
    needle: &str,
) -> Option<serde_json::Value> {
    let storage = state.args.storage_url.as_deref()?.trim_end_matches('/');
    let url = format!(
        "{storage}/api/v1/commitments?action={}&limit={SCOPE_SEARCH_LIMIT}",
        urlencoding::encode(action)
    );
    let resp = state
        .storage_proxy_client
        .get(&url)
        .timeout(LEDGER_READ_TIMEOUT)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    let rows = body
        .get("items")
        .and_then(|i| i.as_array())
        .or_else(|| body.as_array())?;
    rows.iter()
        .find(|row| is_operative(row) && scope_names(row, needle))
        .cloned()
}

/// Whether a commitment row is still the operative term. See [`TERMINAL_STATES`].
pub(crate) fn is_operative(row: &serde_json::Value) -> bool {
    match row.get("state").and_then(|s| s.as_str()) {
        None => true,
        Some(state) => !TERMINAL_STATES.contains(&state),
    }
}

/// Whether a commitment's `inScopeOf` names `needle` exactly.
///
/// Exact element match, never a substring: `epr_root:garden` must not be
/// satisfied by `epr_root:garden-archive`, or a receipt would credit the wrong
/// agreement while looking entirely correct.
pub(crate) fn scope_names(row: &serde_json::Value, needle: &str) -> bool {
    match row.get("inScopeOf") {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str())
            .any(|s| s.trim() == needle),
        Some(serde_json::Value::String(s)) => {
            s.split('|').map(str::trim).any(|part| part == needle)
        }
        _ => false,
    }
}

fn json_response<T: Serialize>(status: StatusCode, body: &T) -> Response<Full<Bytes>> {
    let bytes = serde_json::to_vec(body).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        // A receipt is about ONE serve through ONE doorway at one moment; it is
        // never a shared-cacheable answer.
        .header("cache-control", "no-store")
        .body(Full::new(Bytes::from(bytes)))
        .expect("infallible receipt response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_route_matches_exactly_one_segment() {
        assert_eq!(
            match_receipt_route("/api/v1/receipt/community-garden-club").as_deref(),
            Some("community-garden-club")
        );
        assert!(match_receipt_route("/api/v1/receipt/").is_none());
        assert!(match_receipt_route("/api/v1/receipt").is_none());
        assert!(
            match_receipt_route("/api/v1/receipt/a/b").is_none(),
            "a receipt is for ONE record — a deeper path is not a receipt for its first segment"
        );
        assert!(match_receipt_route("/api/v1/commitments/x").is_none());
    }

    #[test]
    fn a_percent_encoded_id_is_decoded_once() {
        assert_eq!(
            match_receipt_route("/api/v1/receipt/garden%20club").as_deref(),
            Some("garden club")
        );
        // A path separator that was encoded stays one segment on the wire and
        // decodes to the id it was — it never becomes a second segment.
        assert_eq!(
            match_receipt_route("/api/v1/receipt/a%2Fb").as_deref(),
            Some("a/b")
        );
    }

    #[test]
    fn the_protocol_form_is_asked_for_explicitly_never_by_accident() {
        assert!(wants_protocol_form(Some("form=protocol")));
        assert!(wants_protocol_form(Some("x=1&form=protocol")));
        assert!(!wants_protocol_form(Some("form=protocolish")));
        assert!(!wants_protocol_form(Some("form=friendly")));
        assert!(!wants_protocol_form(None));
        assert!(!wants_protocol_form(Some("")));
    }

    #[test]
    fn a_terminal_commitment_is_not_an_operative_term() {
        assert!(is_operative(&json!({ "state": "created" })));
        assert!(is_operative(&json!({ "state": "proposed" })));
        assert!(
            is_operative(&json!({ "state": "active" })),
            "the graduated state must still read as operative"
        );
        assert!(
            is_operative(&json!({})),
            "a row that states no lifecycle is not a row that withdrew"
        );
        for dead in ["cancelled", "terminated", "superseded"] {
            assert!(
                !is_operative(&json!({ "state": dead })),
                "{dead} must not be read as an operative term"
            );
        }
    }

    /// A freshly-created agreement is live. Reading only `state == "active"`
    /// would report it as `unrecorded` — a false absence, which lies exactly as
    /// much as a false credit.
    #[test]
    fn a_freshly_created_agreement_is_not_reported_as_absent() {
        assert!(is_operative(
            &json!({ "id": "hosting-agreement-1", "state": "created" })
        ));
    }

    #[test]
    fn a_scope_is_matched_by_whole_element_never_by_prefix() {
        let row = json!({
            "inScopeOf": ["host:alpha.elohim.host", "epr_root:community-garden-club"]
        });
        assert!(scope_names(&row, "epr_root:community-garden-club"));
        assert!(scope_names(&row, "host:alpha.elohim.host"));
        assert!(
            !scope_names(&row, "epr_root:community-garden"),
            "a prefix must not match — crediting the wrong agreement looks entirely correct"
        );
        assert!(!scope_names(&row, "community-garden-club"));
    }

    #[test]
    fn a_pipe_joined_legacy_scope_is_matched_too() {
        let row = json!({ "inScopeOf": "doorway:alpha-elohim-host|epr:garden" });
        assert!(scope_names(&row, "doorway:alpha-elohim-host"));
        assert!(scope_names(&row, "epr:garden"));
        assert!(!scope_names(&row, "doorway:alpha"));
    }

    #[test]
    fn a_row_with_no_scope_names_nothing() {
        assert!(!scope_names(&json!({}), "doorway:x"));
        assert!(!scope_names(&json!({ "inScopeOf": null }), "doorway:x"));
    }

    // ── the merge ───────────────────────────────────────────────────────────

    fn holder_receipt() -> serde_json::Value {
        json!({
            "sentence": "Someone kept this ready…",
            "epr": "community-garden-club",
            "exchanged": {
                "what": "computing time and storage",
                "sentence": "matthew gives computing time and storage…",
                "commitment": "hosting-agreement-ef5a",
                "record": "/api/v1/commitments/hosting-agreement-ef5a",
                "party": "matthew",
                "resource": "compute"
            },
            "credits": [
                { "role": "projected", "party": "alpha-operator",
                  "commitment": "operate-doorway-alpha", "sentence": "…",
                  "record": "/api/v1/commitments/operate-doorway-alpha" },
                { "role": "held", "party": "12D3KooWAlphaHolder",
                  "commitment": "project-epr-garden", "sentence": "…",
                  "record": "/api/v1/commitments/project-epr-garden" }
            ],
            "unrecorded": []
        })
    }

    /// The whole point of the merge: a courier takes the holder's HELD credit
    /// and leaves the holder's PROJECTED credit behind.
    #[test]
    fn a_courier_takes_the_holders_held_credit_and_not_its_projection_credit() {
        let held = held_from_holder_receipt(&holder_receipt()).expect("the holder said who holds");
        assert_eq!(held.commitment_id, "project-epr-garden");
        assert_eq!(held.party, "12D3KooWAlphaHolder");
        assert_ne!(
            held.commitment_id, "operate-doorway-alpha",
            "the holder's own doorway binding must never arrive as the held credit"
        );
    }

    /// Read by ROLE, not by position — a holder that lists its credits in
    /// another order must not shift which one a courier reads.
    #[test]
    fn the_held_credit_is_read_by_role_not_by_position() {
        let mut body = holder_receipt();
        let credits = body["credits"].as_array().cloned().unwrap();
        body["credits"] = json!(credits.into_iter().rev().collect::<Vec<_>>());
        let held = held_from_holder_receipt(&body).expect("still found by role");
        assert_eq!(held.commitment_id, "project-epr-garden");
    }

    #[test]
    fn the_exchange_clause_carries_the_holders_own_party_and_resource() {
        let exchanged =
            exchange_from_holder_receipt(&holder_receipt()).expect("the holder named an exchange");
        assert_eq!(exchanged.commitment_id, "hosting-agreement-ef5a");
        assert_eq!(exchanged.party, "matthew");
        assert_eq!(exchanged.resource.as_deref(), Some("compute"));
    }

    #[test]
    fn a_holder_with_nothing_recorded_yields_nothing_rather_than_a_guess() {
        let empty = json!({ "epr": "x", "credits": [], "exchanged": null, "unrecorded": [] });
        assert!(held_from_holder_receipt(&empty).is_none());
        assert!(exchange_from_holder_receipt(&empty).is_none());
    }

    #[test]
    fn a_receipt_built_from_the_merged_halves_credits_two_different_parties() {
        let held = held_from_holder_receipt(&holder_receipt()).unwrap();
        let exchanged = exchange_from_holder_receipt(&holder_receipt()).unwrap();
        let ours = ProjectedClause {
            commitment_id: "operate-doorway-beta".into(),
            party: "human-matthew-manager".into(),
            party_label: None,
            doorway_id: "apex-elohim-host".into(),
        };
        let receipt = build_receipt(
            "community-garden-club",
            Some(&exchanged),
            Some(&held),
            Some(&ours),
            Some("http://localhost:8888"),
        );
        assert!(receipt.is_complete());
        assert_eq!(receipt.served_by.as_deref(), Some("http://localhost:8888"));
        let credited: Vec<&str> = receipt
            .credits
            .iter()
            .map(|c| c.commitment.as_str())
            .collect();
        assert_eq!(credited, vec!["project-epr-garden", "operate-doorway-beta"]);
        assert!(
            !credited.contains(&"operate-doorway-alpha"),
            "the holder's own doorway binding must not survive the merge"
        );
    }
}
