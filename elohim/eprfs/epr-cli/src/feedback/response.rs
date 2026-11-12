//! Feedback proposes on the existing flow plane; execution admission stays with the launcher.
use super::{required, Result};
use cid::Cid;
use elohim_epr_rea::{
    feedback::{experiences, Feedback, Policy, Response},
    feedback_store::FeedbackStore,
    AgentRef, Commitment, CommitmentState, FlowRecord, FlowStore, Intent, ReaVerb, ResourceSpec,
    SidecarFlowStore,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

pub(super) fn check_commitment(root: &Path, cid: &Cid) -> Result<Commitment> {
    if !SidecarFlowStore::probe(root)? {
        return Err("waiting: no existing development commitments".into());
    }
    SidecarFlowStore::open(root)?
        .commitments()?
        .into_iter()
        .find(|(id, _)| id == cid)
        .map(|(_, c)| c)
        .ok_or_else(|| "waiting: commitment is not in the existing flow store".into())
}

pub(super) fn propose(
    root: &Path,
    _store: &FeedbackStore,
    policy: &Policy,
    options: &BTreeMap<String, String>,
) -> Result<Value> {
    if policy.response == Response::Report || !policy.deliberate_evidence {
        return Err("policy does not allow deliberate repair proposals".into());
    }
    let cid: Cid = required(options, "id")?.parse()?;
    let actor = required(options, "as")?;
    let objective = required(options, "objective")?;
    let flows = SidecarFlowStore::open(root)?;
    let mut tx = flows.transaction()?;
    let entries = tx.feedback_records(policy.clone())?;
    if !entries
        .iter()
        .any(|e| e.cid == cid && matches!(e.record, Feedback::Occurrence(_)))
    {
        return Err("occurrence not found".into());
    }
    let record = FlowRecord::Intent(Intent {
        action: ReaVerb::Produce,
        raised_by: AgentRef(actor.into()),
        in_scope_of: cid,
        resource_spec: ResourceSpec {
            classified_as: vec![
                "feedback:repair".into(),
                cid.to_string(),
                format!("objective:{objective}"),
            ],
            quantity: None,
        },
    });
    let intent = record.cid()?;
    if tx.records()?.iter().any(|(id,r)| matches!(r, FlowRecord::Intent(i) if i.in_scope_of == cid
        && i.resource_spec.classified_as.first().is_some_and(|tag| tag == "feedback:repair") && *id != intent)) {
        return Err("repair proposal already exists for this occurrence; inspect it before proposing again".into());
    }
    if !tx.records()?.iter().any(|(id, _)| *id == intent) {
        tx.append(record)?;
    }
    Ok(
        json!({"intent":intent.to_string(),"admitted":false,"next":"epr flow claim --on <intent> --as <actor>; execute only through existing bounded shift admission"}),
    )
}

pub(super) fn triage(
    root: &Path,
    store: &FeedbackStore,
    policy: &Policy,
    options: &BTreeMap<String, String>,
) -> Result<Value> {
    let requested: Response = serde_json::from_value(json!(options
        .get("response")
        .map(String::as_str)
        .unwrap_or("report")))?;
    let entries = store.read()?;
    let flows = if SidecarFlowStore::probe(root)? {
        SidecarFlowStore::open(root)?.records()?
    } else {
        vec![]
    };
    let mut rows = vec![];
    for experience in experiences(&entries) {
        let mut waiting = Vec::<String>::new();
        if requested != Response::Report && policy.response == Response::Report {
            waiting.push("policy permits reporting only".into());
        }
        if requested == Response::Repair {
            if policy.response != Response::Repair {
                waiting.push("policy does not delegate repair".into());
            }
            if experience.commitments.is_empty() {
                waiting.push(
                    "no admitted development commitment; propose then claim through epr flow"
                        .into(),
                );
            }
            for cid in &experience.commitments {
                let commitment = flows.iter().find_map(|(id, r)| match r {
                    FlowRecord::Commitment(c) if id == cid => Some(c),
                    _ => None,
                });
                if let Some(c) = commitment {
                    if c.state != CommitmentState::Active {
                        waiting.push("commitment is not active".into());
                    }
                    if options.get("as").map(String::as_str) != Some(c.provider.0.as_str()) {
                        waiting
                            .push("repair ownership is absent or belongs to another actor".into());
                    }
                    if c.bound.as_ref().is_some_and(|b| b.validate().is_err()) {
                        waiting.push("declared commitment bound is invalid".into());
                    }
                    if flows
                        .iter()
                        .any(|(_, r)| matches!(r,FlowRecord::Event(e) if e.fulfills.contains(cid)))
                    {
                        waiting.push(
                            "commitment already fulfilled; a new experience needs new admission"
                                .into(),
                        );
                    }
                    let now = chrono::Utc::now();
                    for (date, start) in [(&c.valid_from, true), (&c.valid_until, false)] {
                        if let Some(date) = date {
                            match chrono::DateTime::parse_from_rfc3339(date) {
                                Ok(t) if (start && t > now) || (!start && t < now) => {
                                    waiting.push("commitment outside validity window".into())
                                }
                                Err(_) => waiting.push("commitment validity unmeasured".into()),
                                _ => {}
                            }
                        }
                    }
                } else {
                    waiting.push("linked commitment unavailable".into());
                }
            }
            // Feedback never invents a second admission engine. The existing launcher resolves
            // actual session claims, authority, isolation and resource budget at execution time.
            waiting.push("execution requires existing shift admission (authority, session claim, isolation, budget and gates)".into());
        }
        let age = chrono::DateTime::parse_from_rfc3339(&experience.occurrence.observed_at)
            .ok()
            .map(|t| (chrono::Utc::now() - t.to_utc()).num_seconds().max(0));
        rows.push(json!({"experience":experience,"ageSeconds":age,"response":requested,"waiting":waiting,"admitted":false}));
    }
    Ok(
        json!({"policy":policy,"rows":rows,"algedonic":"only existing declared measures and commitment bounds may escalate; feedback alone does not"}),
    )
}
