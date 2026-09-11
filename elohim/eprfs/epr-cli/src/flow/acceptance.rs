//! Appointed experiential acceptance: additive notes, with shared admission/read validation.
//! These local claims establish accountability, not cryptographic peer qualification.
use std::path::Path;

use cid::Cid;
use elohim_epr_rea::{ActorStore, Commitment, FlowEvent, FlowRecord, ReaVerb, SidecarActorStore};
use serde::Deserialize;

use super::{body_cid_of_file, confine_under, rel_to_root, FlowError, FlowResult};

/// Optional slots on existing notes. Empty options preserve the historical encoding.
#[derive(Debug, Default, Clone)]
pub struct AcceptanceOptions {
    pub appoint: Option<String>,
    pub purpose: Option<String>,
    pub appointment: Option<String>,
    pub fulfillment: Option<String>,
    pub review: Option<String>,
    pub report: Option<String>,
    pub supersedes: Option<String>,
}

fn refused(message: impl Into<String>) -> FlowError {
    FlowError::InvalidArguments(format!("acceptance: {}", message.into()))
}

/// Read an additive metadata slot. Validation rejects duplicates before trusting it.
pub fn slot<'a>(event: &'a FlowEvent, prefix: &str) -> Option<&'a str> {
    event
        .classified_as
        .iter()
        .find_map(|s| s.strip_prefix(prefix))
}

fn cid(raw: &str) -> FlowResult<Cid> {
    raw.parse()
        .map_err(|_| refused(format!("invalid CID `{raw}`")))
}

fn required<'a>(event: &'a FlowEvent, prefix: &str) -> FlowResult<&'a str> {
    slot(event, prefix)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| refused(format!("missing {prefix}")))
}

fn commitment<'a>(records: &'a [(Cid, FlowRecord)], id: &Cid) -> FlowResult<&'a Commitment> {
    records
        .iter()
        .find_map(|(c, r)| match r {
            FlowRecord::Commitment(v) if c == id => Some(v),
            _ => None,
        })
        .ok_or_else(|| refused("target must be one exact recorded commitment"))
}

fn event<'a>(records: &'a [(Cid, FlowRecord)], id: &Cid) -> FlowResult<&'a FlowEvent> {
    records
        .iter()
        .find_map(|(c, r)| match r {
            FlowRecord::Event(v) if c == id => Some(v),
            _ => None,
        })
        .ok_or_else(|| refused(format!("event reference not found: {id}")))
}

fn actor_identity(root: &Path, id: &Cid) -> FlowResult<String> {
    if !root.join(".eprfs/status/actors.jsonl").is_file() {
        return Err(refused("registered actor claim required"));
    }
    SidecarActorStore::open(root)?
        .claims()?
        .into_iter()
        .find(|(c, _)| c == id)
        .map(|(_, claim)| claim.claimed.0)
        .ok_or_else(|| refused("actor claim is not registered"))
}

fn independent_actor(
    root: &Path,
    records: &[(Cid, FlowRecord)],
    target: &Cid,
    actor: &Cid,
) -> FlowResult<String> {
    let promise = commitment(records, target)?;
    let implementer = promise
        .resource_spec
        .classified_as
        .iter()
        .find_map(|s| s.strip_prefix("actor-claim:"))
        .ok_or_else(|| refused("implementer has no exact registered actor claim"))?;
    let implementer = cid(implementer)?;
    let identity = actor_identity(root, actor)?;
    let implementer_identity = actor_identity(root, &implementer)?;
    if implementer_identity != promise.provider.0
        || implementer == *actor
        || identity == promise.provider.0
    {
        return Err(refused(
            "acceptor must be independent of the attributed implementer",
        ));
    }
    Ok(identity)
}

#[derive(Debug, Deserialize)]
struct Report {
    intent: String,
    revision: String,
    environment: String,
    actions: Vec<String>,
    observations: Vec<String>,
    evidence: Vec<Evidence>,
    limitations: Vec<String>,
}
#[derive(Debug, Deserialize)]
struct Evidence {
    path: String,
    cid: String,
}

fn pinned_file(root: &Path, path: &str, pin: Option<&str>) -> FlowResult<(String, Cid)> {
    let root = std::fs::canonicalize(root)?;
    let path = confine_under(&root, &root.join(path))?;
    if !path.is_file() {
        return Err(refused("evidence must resolve to a file"));
    }
    let actual = body_cid_of_file(&path)
        .ok_or_else(|| refused("evidence must be readable UTF-8 body content"))?;
    if let Some(pin) = pin {
        if cid(pin)? != actual {
            return Err(refused(format!("evidence changed: {}", path.display())));
        }
    }
    Ok((rel_to_root(&root, &path), actual))
}

fn validate_report(
    root: &Path,
    path: &str,
    pin: &str,
    fulfillment_resource: &Cid,
) -> FlowResult<()> {
    let (path, _) = pinned_file(root, path, Some(pin))?;
    let report: Report = serde_json::from_str(&std::fs::read_to_string(root.join(path))?)?;
    if [&report.intent, &report.revision, &report.environment]
        .iter()
        .any(|s| s.trim().is_empty())
        || report.actions.is_empty()
        || report.observations.is_empty()
        || report.evidence.is_empty()
        || report
            .actions
            .iter()
            .chain(&report.observations)
            .chain(&report.limitations)
            .any(|s| s.trim().is_empty())
    {
        return Err(refused("report needs intent, revision, environment, actions, observations, evidence and explicit limitations array"));
    }
    if !report
        .evidence
        .iter()
        .any(|e| e.cid == fulfillment_resource.to_string())
    {
        return Err(refused("report evidence must pin the fulfillment resource"));
    }
    for evidence in report.evidence {
        pinned_file(root, &evidence.path, Some(&evidence.cid))?;
    }
    Ok(())
}

impl AcceptanceOptions {
    /// Build metadata before actor/steward trailers; validation runs on the completed event.
    pub(crate) fn slots(
        &self,
        root: &Path,
        records: &[(Cid, FlowRecord)],
        target: &Cid,
        kind: &str,
    ) -> FlowResult<Vec<String>> {
        let has_acceptance = self.purpose.is_some()
            || self.appointment.is_some()
            || self.fulfillment.is_some()
            || self.review.is_some()
            || self.report.is_some()
            || self.supersedes.is_some();
        if let Some(actor) = &self.appoint {
            if kind != "run:ruling" || has_acceptance {
                return Err(refused("--appoint belongs only to a ruling"));
            }
            independent_actor(root, records, target, &cid(actor)?)?;
            return Ok(vec![format!("appoint:{actor}")]);
        }
        if !has_acceptance {
            return Ok(vec![]);
        }
        if kind != "run:verdict" || self.purpose.as_deref() != Some("acceptance") {
            return Err(refused(
                "acceptance metadata needs --kind verdict --purpose acceptance",
            ));
        }
        let mut slots = vec!["purpose:acceptance".into()];
        for (name, value) in [
            ("appointment", &self.appointment),
            ("fulfillment", &self.fulfillment),
            ("review", &self.review),
        ] {
            let value = value
                .as_deref()
                .ok_or_else(|| refused(format!("missing --{name}")))?;
            cid(value)?;
            slots.push(format!("{name}:{value}"));
        }
        let report = self
            .report
            .as_deref()
            .ok_or_else(|| refused("missing --report"))?;
        let (path, report_cid) = pinned_file(root, report, None)?;
        slots.push(format!("report-path:{path}"));
        slots.push(format!("report-cid:{report_cid}"));
        slots.push(format!(
            "scope-cid:{}",
            commitment(records, target)?.in_scope_of
        ));
        if let Some(previous) = &self.supersedes {
            cid(previous)?;
            slots.push(format!("supersedes:{previous}"));
        }
        Ok(slots)
    }
}

fn validate_note_shape(current: &FlowEvent) -> FlowResult<()> {
    for prefix in [
        "appoint:",
        "purpose:",
        "appointment:",
        "fulfillment:",
        "review:",
        "report-path:",
        "report-cid:",
        "scope-cid:",
        "supersedes:",
        "actor-claim:",
        "verdict:",
    ] {
        if current
            .classified_as
            .iter()
            .filter(|s| s.starts_with(prefix))
            .count()
            > 1
        {
            return Err(refused(format!("duplicate {prefix}")));
        }
    }
    if !matches!(&current.quantity, elohim_epr_rea::Magnitude::Count { unit, .. } if unit == "run-note")
        || current.action != ReaVerb::Cite
        || !current.fulfills.is_empty()
        || !current.satisfies.is_empty()
    {
        return Err(refused("acceptance must be a non-discharging Cite note"));
    }
    Ok(())
}

/// Immutable authority/link checks, separate from artifact availability. A lost
/// report invalidates current acceptance, not its explicit supersession history.
pub fn validate_structure(
    root: &Path,
    records: &[(Cid, FlowRecord)],
    current: &FlowEvent,
) -> FlowResult<()> {
    validate_note_shape(current)?;
    if let Some(actor) = slot(current, "appoint:") {
        if current.classified_as.first().map(String::as_str) != Some("run:ruling") {
            return Err(refused("appointment is not a ruling"));
        }
        let identity = independent_actor(root, records, &current.resource, &cid(actor)?)?;
        if identity == current.provider.0 || slot(current, "actor-claim:") == Some(actor) {
            return Err(refused("acceptor cannot appoint itself"));
        }
        return Ok(());
    }
    if required(current, "purpose:")? != "acceptance"
        || current.classified_as.first().map(String::as_str) != Some("run:verdict")
        || !matches!(
            required(current, "verdict:")?,
            "approved" | "changes-requested"
        )
    {
        return Err(refused("not an acceptance verdict"));
    }
    let actor = cid(required(current, "actor-claim:")?)?;
    if independent_actor(root, records, &current.resource, &actor)? != current.provider.0 {
        return Err(refused("actor claim differs from provider"));
    }
    let appointment = event(records, &cid(required(current, "appointment:")?)?)?;
    if appointment.resource != current.resource
        || required(appointment, "appoint:")? != actor.to_string()
    {
        return Err(refused(
            "appointment does not appoint this actor to this commitment",
        ));
    }
    validate_structure(root, records, appointment)?;
    let fulfillment = event(records, &cid(required(current, "fulfillment:")?)?)?;
    if fulfillment.action != ReaVerb::Produce || !fulfillment.fulfills.contains(&current.resource) {
        return Err(refused("fulfillment does not produce this commitment"));
    }
    let review = event(records, &cid(required(current, "review:")?)?)?;
    validate_note_shape(review)?;
    if review.resource != current.resource
        || review.action != ReaVerb::Cite
        || review.classified_as.first().map(String::as_str) != Some("run:verdict")
        || slot(review, "verdict:") != Some("approved")
        || slot(review, "purpose:").is_some()
    {
        return Err(refused(
            "review must be an approved technical verdict on the commitment",
        ));
    }
    if cid(required(current, "scope-cid:")?)? != commitment(records, &current.resource)?.in_scope_of
    {
        return Err(refused("scope differs from commitment"));
    }
    if let Some(previous) = slot(current, "supersedes:") {
        let previous = event(records, &cid(previous)?)?;
        if previous.resource != current.resource
            || slot(previous, "purpose:") != Some("acceptance")
            || previous.classified_as.first().map(String::as_str) != Some("run:verdict")
        {
            return Err(refused(
                "supersedes must name acceptance on the same commitment",
            ));
        }
    }
    required(current, "report-path:")?;
    cid(required(current, "report-cid:")?)?;
    Ok(())
}

/// Recheck the same evidence on admission and reconciliation. Never infer fitness from fields.
pub fn validate_record(
    root: &Path,
    records: &[(Cid, FlowRecord)],
    current: &FlowEvent,
) -> FlowResult<()> {
    validate_structure(root, records, current)?;
    if slot(current, "appoint:").is_some() {
        return Ok(());
    }
    let fulfillment = event(records, &cid(required(current, "fulfillment:")?)?)?;
    validate_report(
        root,
        required(current, "report-path:")?,
        required(current, "report-cid:")?,
        &fulfillment.resource,
    )
}
