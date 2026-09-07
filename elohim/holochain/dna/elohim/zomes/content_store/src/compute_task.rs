//! Native delegated compute: immutable REA request and runner-authored events.
//! No new integrity types or content heads. Storage checks current execution
//! grants; these reads independently bind claims to signed action authors.
use crate::*;
use serde_json::{json, Value};

fn bad(message: &str) -> WasmError {
    wasm_error!(WasmErrorInner::Guest(message.to_owned()))
}
fn string<'a>(value: &'a Value, name: &str) -> ExternResult<&'a str> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= 512)
        .ok_or_else(|| bad("missing or oversized compute field"))
}
fn action(value: &str) -> ExternResult<ActionHash> {
    ActionHash::try_from(value.to_owned()).map_err(|_| bad("invalid action reference"))
}
fn anchor(namespace: &str, id: &str) -> ExternResult<EntryHash> {
    hash_entry(&EntryTypes::StringAnchor(StringAnchor::new(namespace, id)))
}
fn metadata(raw: &str) -> ExternResult<Value> {
    serde_json::from_str(raw).map_err(|_| bad("invalid compute metadata"))
}
fn record(reference: &str) -> ExternResult<Record> {
    get(action(reference)?, GetOptions::default())?.ok_or_else(|| bad("compute record unavailable"))
}

/// Exact action reference, never the first mutable ID-anchor target.
fn read_request(reference: &str) -> ExternResult<(Commitment, Value)> {
    let record = record(reference)?;
    if !matches!(record.action().data, ActionData::Create(_)) {
        return Err(bad("compute requests are immutable create actions"));
    }
    let entry: Commitment = record
        .entry()
        .to_app_option()
        .map_err(|_| bad("not a compute commitment"))?
        .ok_or_else(|| bad("missing request entry"))?;
    let meta = metadata(&entry.metadata_json)?;
    if meta["kind"] != "rakia-compute-request-v1"
        || entry.action != "work"
        || record.action().author().to_string() != entry.receiver
        || meta["envelope"]["requester"] != entry.receiver
        || meta["envelope"]["provider"] != entry.provider
    {
        return Err(bad("compute request author or identity mismatch"));
    }
    Ok((entry, meta))
}

fn status(reference: &str) -> ExternResult<Value> {
    let (request, meta) = read_request(reference)?;
    let links = get_links(
        LinkQuery::try_new(
            anchor("compute_task_events", reference)?,
            LinkTypes::IdToEvent,
        )?,
        GetStrategy::default(),
    )?;
    if links.len() > 16 {
        return Err(bad("compute event budget exceeded"));
    }
    let mut accepted: Option<Value> = None;
    let mut completed: Option<Value> = None;
    let mut refused: Option<Value> = None;
    // bounded-work: at most sixteen event records for one immutable request.
    let deadline = sys_time()?.as_micros().saturating_add(2_000_000);
    for link in links {
        if sys_time()?.as_micros() > deadline {
            return Err(bad("compute evidence deadline exceeded"));
        }
        let Some(hash) = link.target.into_action_hash() else {
            continue;
        };
        let Some(record) = get(hash.clone(), GetOptions::default())? else {
            continue;
        };
        if record.action().author().to_string() != request.provider
            || !matches!(record.action().data, ActionData::Create(_))
        {
            continue;
        }
        let Some(event): Option<EconomicEvent> = record
            .entry()
            .to_app_option()
            .map_err(|_| bad("invalid task event"))?
        else {
            continue;
        };
        let event_meta = metadata(&event.metadata_json)?;
        if event.provider != request.provider
            || event.receiver != request.receiver
            || event_meta["requestActionHash"] != reference
            || event_meta["taskCid"] != meta["taskCid"]
        {
            continue;
        }
        let fulfills: Vec<String> = serde_json::from_str(&event.fulfills_json)
            .map_err(|_| bad("invalid compute fulfillment"))?;
        if (matches!(event.action.as_str(), "accept" | "reject") && !fulfills.is_empty())
            || (event.action == "work" && fulfills != vec![request.id.clone()])
        {
            return Err(bad("compute event fulfillment does not match its phase"));
        }
        let mut evidence = event_meta.clone();
        evidence["actionHash"] = json!(hash.to_string());
        if event.action == "accept" && event_meta["kind"] == "rakia-compute-accept-v1" {
            if accepted.as_ref().is_some_and(|a| a != &evidence) {
                return Err(bad("conflicting compute acceptances"));
            }
            accepted = Some(evidence);
        } else if event.action == "reject" && event_meta["kind"] == "rakia-compute-decline-v1" {
            if refused.as_ref().is_some_and(|r| r != &evidence) {
                return Err(bad("conflicting compute refusals"));
            }
            refused = Some(evidence);
        } else if event.action == "work" && event_meta["kind"] == "rakia-compute-complete-v1" {
            if completed.as_ref().is_some_and(|c| c != &evidence) {
                return Err(bad("conflicting compute completions"));
            }
            completed = Some(evidence);
        }
    }
    if completed.is_some() && refused.is_some() {
        return Err(bad("conflicting terminal compute events"));
    }
    if let Some(completion) = &completed {
        let acceptance = accepted
            .as_ref()
            .ok_or_else(|| bad("completion without acceptance"))?;
        if completion["attemptId"] != acceptance["attemptId"]
            || completion["acceptanceActionHash"] != acceptance["actionHash"]
        {
            return Err(bad("completion references another attempt"));
        }
    }
    Ok(
        json!({"requestActionHash":reference,"taskCid":meta["taskCid"],"requester":request.receiver,
        "provider":request.provider,"grantActionHash":meta["grantActionHash"],"envelope":meta["envelope"],
        "state":if completed.is_some(){"completed"}else if refused.is_some(){"refused"}else if accepted.is_some(){"accepted"}else{"requested"},
        "acceptance":accepted,"completion":completed,"refusal":refused}),
    )
}

/// One bounded task operation. Coordinator-only; the HTTP adapter also enforces
/// local-cell authentication and grant bounds before acceptance/launch.
#[hdk_extern]
pub fn compute_task(input: serde_json::Value) -> ExternResult<serde_json::Value> {
    let operation = string(&input, "operation")?;
    match operation {
        "get" => status(string(&input, "requestActionHash")?),
        "list" => {
            let provider = string(&input, "provider")?;
            let offset = input["offset"].as_u64().unwrap_or(0) as usize;
            let limit = input["limit"].as_u64().unwrap_or(20).clamp(1, 50) as usize;
            let mut links = get_links(
                LinkQuery::try_new(
                    anchor("compute_task_provider", provider)?,
                    LinkTypes::CommitmentByProvider,
                )?,
                GetStrategy::default(),
            )?;
            if links.len() > 4096 {
                return Err(bad("compute provider discovery index exceeds v1 budget"));
            }
            links.sort_by(|a, b| {
                a.timestamp
                    .cmp(&b.timestamp)
                    .then(a.create_link_hash.cmp(&b.create_link_hash))
            });
            let mut end = offset.min(links.len());
            let deadline = sys_time()?.as_micros().saturating_add(2_000_000);
            let mut tasks = Vec::new();
            // bounded-work: at most fifty request reads; event reads bounded in status.
            for link in links.iter().skip(offset).take(limit) {
                if sys_time()?.as_micros() > deadline {
                    break;
                }
                end += 1;
                if let Some(hash) = link.target.clone().into_action_hash() {
                    let task = status(&hash.to_string())?;
                    if task["provider"] == provider {
                        tasks.push(task);
                    }
                }
            }
            Ok(json!({"tasks":tasks,"nextOffset":if end<links.len(){Some(end)}else{None}}))
        }
        "submit" => {
            let task_cid = string(&input, "taskCid")?;
            let provider = string(&input, "provider")?;
            let grant = string(&input, "grantActionHash")?;
            action(grant)?;
            let requester = agent_info()?.agent_initial_pubkey.to_string();
            let envelope = &input["envelope"];
            if envelope["schemaVersion"] != 1
                || envelope["taskKind"] != "feedback_signal"
                || envelope["requester"] != requester
                || envelope["provider"] != provider
                || envelope.to_string().len() > 65536
            {
                return Err(bad("invalid compute envelope"));
            }
            let id = format!("compute:{requester}:{task_cid}");
            let links = get_links(
                LinkQuery::try_new(anchor("commitment_id", &id)?, LinkTypes::IdToCommitment)?,
                GetStrategy::default(),
            )?;
            if links.len() > 8 {
                return Err(bad("compute submission budget exceeded"));
            }
            for link in links {
                if let Some(hash) = link.target.into_action_hash() {
                    let existing = status(&hash.to_string())?;
                    if existing["requester"] == requester {
                        if existing["envelope"] != *envelope || existing["grantActionHash"] != grant
                        {
                            return Err(bad("task CID already bound to different input"));
                        }
                        return Ok(existing);
                    }
                }
            }
            let metadata = json!({"kind":"rakia-compute-request-v1","taskCid":task_cid,"grantActionHash":grant,"envelope":envelope});
            let creation: CreateReaCommitmentInput = serde_json::from_value(json!({"id":id,"action":"work","provider":provider,"receiver":requester,"metadata_json":metadata.to_string()})).map_err(|_|bad("invalid request"))?;
            let out = create_rea_commitment(creation)?;
            create_link(
                anchor("compute_task_provider", provider)?,
                out.action_hash.clone(),
                LinkTypes::CommitmentByProvider,
                (),
            )?;
            status(&out.action_hash.to_string())
        }
        "accept" | "complete" | "decline" => {
            let reference = string(&input, "requestActionHash")?;
            let attempt = string(&input, "attemptId")?;
            let existing = status(reference)?;
            let runner = agent_info()?.agent_initial_pubkey.to_string();
            if existing["provider"] != runner {
                return Err(bad("only selected runner can act"));
            }
            let key = match operation {
                "accept" => "acceptance",
                "decline" => "refusal",
                _ => "completion",
            };
            if !existing[key].is_null() {
                if existing[key]["attemptId"] != attempt
                    || (operation == "complete"
                        && existing[key]["receiptCid"] != input["receiptCid"])
                    || (operation == "decline" && existing[key]["reason"] != input["reason"])
                {
                    return Err(bad("conflicting compute attempt"));
                }
                return Ok(existing);
            }
            if existing["state"] == "refused" || existing["state"] == "completed" {
                return Err(bad("compute task already terminal"));
            }
            let mut meta = json!({"kind":match operation {"accept"=>"rakia-compute-accept-v1","decline"=>"rakia-compute-decline-v1",_=>"rakia-compute-complete-v1"},
                "requestActionHash":reference,"taskCid":existing["taskCid"],"attemptId":attempt});
            if operation == "decline" {
                if !existing["acceptance"].is_null()
                    && existing["acceptance"]["attemptId"] != attempt
                {
                    return Err(bad("refusal references another accepted attempt"));
                }
                let reason = string(&input, "reason")?;
                if !matches!(
                    reason,
                    "compute-grant-refused"
                        | "runtime-incompatible"
                        | "capacity-unavailable"
                        | "invalid-artifact"
                        | "interrupted"
                ) {
                    return Err(bad("invalid refusal reason"));
                }
                meta["reason"] = json!(reason);
            }
            let (request, _) = read_request(reference)?;
            let fulfills = if operation == "complete" {
                if existing["acceptance"]["attemptId"] != attempt {
                    return Err(bad("acceptance required for attempt"));
                }
                let receipt_cid = string(&input, "receiptCid")?;
                if input["receipt"]["taskCid"] != existing["taskCid"]
                    || input["receipt"]["requestAction"] != reference
                    || input["receipt"]["grantAction"] != existing["grantActionHash"]
                    || input["receipt"]["provider"] != runner
                    || input["receipt"]["requester"] != existing["requester"]
                {
                    return Err(bad("receipt does not bind this signed task"));
                }
                if input["receipt"].to_string().len() > 65536 {
                    return Err(bad("receipt too large"));
                }
                meta["receiptCid"] = json!(receipt_cid);
                meta["receipt"] = input["receipt"].clone();
                meta["acceptanceActionHash"] = existing["acceptance"]["actionHash"].clone();
                vec![request.id]
            } else {
                Vec::new()
            };
            let creation: CreateReaEconomicEventInput = serde_json::from_value(json!({"id":format!("compute:{reference}:{operation}"),"action":match operation {"accept"=>"accept","decline"=>"reject",_=>"work"},"provider":runner,"receiver":request.receiver,"has_point_in_time":format!("{:?}",sys_time()?),"fulfills":fulfills,"metadata_json":meta.to_string()})).map_err(|_|bad("invalid event"))?;
            let out = create_rea_economic_event(creation)?;
            create_link(
                anchor("compute_task_events", reference)?,
                out.action_hash,
                LinkTypes::IdToEvent,
                (),
            )?;
            status(reference)
        }
        _ => Err(bad("unknown compute operation")),
    }
}
