//! `epr flow enrich` — append one local attestation about re-executing an existing gate verdict.
//!
//! The enriched verdict remains immutable. This writer names its exact event CID, copies the
//! project, closure and commit from that event, and appends a separate `Cite`. In shadow the new
//! measure is read by nothing and is absent from the peer crossing allow-list.
//!
//! What the record is not. It is unsigned and stays on this device, so it carries no authority: a
//! reader may count it, never obey it. Its outcome and method are the caller's assertion; this verb
//! checks only that the verdict it names exists in the fold and has the gate-verdict shape, not that
//! the caller re-executed anything. It lives in the flow sidecar (`.eprfs/status/flows.jsonl`), and
//! its only bound there is content deduplication: the same (verdict, kind, outcome, method, run)
//! from this device is written once.

use std::collections::BTreeMap;
use std::path::Path;

use cid::Cid;
use elohim_epr_rea::{
    AgentRef, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb, SidecarFlowStore,
};
use serde::Serialize;

use super::measures::{self, MeasureRef, Registry};
use super::{head_commit_provenance, repository_agent, repository_scope, FlowError, FlowResult};

const ENRICHMENT_MEASURE: &str = "attestation-enrichment@1";
const GATE_VERDICT_MEASURE: &str = "gate-verdict@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnrichmentKind {
    ReExecuted,
}

impl EnrichmentKind {
    fn parse(raw: &str) -> FlowResult<Self> {
        match raw.trim() {
            "re-executed" => Ok(Self::ReExecuted),
            "household-witnessed" | "accepted" => Err(FlowError::InvalidArguments(format!(
                "kind {} is reserved, not implemented",
                raw.trim()
            ))),
            other => Err(FlowError::InvalidArguments(format!(
                "unknown kind `{other}`"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::ReExecuted => "re-executed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnrichmentOutcome {
    Green,
    Red,
    NotComparable,
}

impl EnrichmentOutcome {
    fn parse(raw: &str) -> FlowResult<Self> {
        match raw.trim() {
            "green" => Ok(Self::Green),
            "red" => Ok(Self::Red),
            "not-comparable" => Ok(Self::NotComparable),
            other => Err(FlowError::InvalidArguments(format!(
                "unknown outcome `{other}`; expected green|red|not-comparable"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Green => "green",
            Self::Red => "red",
            Self::NotComparable => "not-comparable",
        }
    }
}

pub struct EnrichmentTargetRequest<'a> {
    pub verdict: Option<&'a str>,
    pub project: Option<&'a str>,
    pub closure: Option<&'a str>,
    pub from_device: Option<&'a str>,
}

pub struct EnrichmentRequest<'a> {
    pub verdict: Option<&'a str>,
    pub project: Option<&'a str>,
    pub closure: Option<&'a str>,
    pub from_device: Option<&'a str>,
    pub kind: &'a str,
    pub outcome: &'a str,
    pub method: &'a str,
    pub reason: Option<&'a str>,
    pub run: Option<&'a str>,
    pub seconds: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentReceipt {
    pub appended: bool,
    pub cid: String,
    pub verdict: String,
    pub kind: String,
    pub outcome: String,
    pub project: String,
    pub closure: String,
    pub device: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentTargetReceipt {
    pub verdict: String,
    pub project: String,
    pub closure: String,
    pub commit: String,
    pub device: String,
}

impl EnrichmentReceipt {
    pub fn render(&self) {
        println!(
            "enrich   {} {} of verdict {} ({} {})",
            self.kind,
            self.outcome,
            short(&self.verdict),
            self.project,
            short(&self.closure)
        );
    }
}

impl EnrichmentTargetReceipt {
    pub fn render(&self) {
        println!(
            "enrich target {} ({} {})",
            short(&self.verdict),
            self.project,
            short(&self.closure)
        );
    }
}

#[derive(Debug, Clone)]
struct VerdictFields {
    project: String,
    closure: String,
    commit: String,
}

/// Append one enrichment, or return the existing record for the same idempotency tuple.
pub fn enrich(root: &Path, request: &EnrichmentRequest<'_>) -> FlowResult<EnrichmentReceipt> {
    let kind = EnrichmentKind::parse(request.kind)?;
    let outcome = EnrichmentOutcome::parse(request.outcome)?;
    let method = one_line(request.method, "--method")?;
    let reason = request
        .reason
        .map(|value| one_line(value, "--reason"))
        .transpose()?;
    match (outcome, reason) {
        (EnrichmentOutcome::NotComparable, None) => {
            return Err(FlowError::InvalidArguments(
                "--outcome not-comparable needs --reason <one-line reason>".into(),
            ));
        }
        (EnrichmentOutcome::Green | EnrichmentOutcome::Red, Some(_)) => {
            return Err(FlowError::InvalidArguments(
                "--reason is only valid with --outcome not-comparable".into(),
            ));
        }
        _ => {}
    }
    let run = request
        .run
        .map(|raw| {
            raw.parse::<Cid>().map(|cid| cid.to_string()).map_err(|_| {
                FlowError::InvalidArguments(format!("--run `{raw}` is not a content identifier"))
            })
        })
        .transpose()?;
    let seconds = request
        .seconds
        .map(|value| {
            if value.is_finite() && value >= 0.0 {
                Ok(canonical_number(value))
            } else {
                Err(FlowError::InvalidArguments(
                    "--seconds must be a finite nonnegative number".into(),
                ))
            }
        })
        .transpose()?;

    let measures_path = measures::default_measures(root);
    let measure = MeasureRef::parse(ENRICHMENT_MEASURE, &measures_path)?;
    let registry = Registry::open(&measures_path)?;
    if !registry.declares(&measure) {
        return Err(FlowError::InvalidArguments(format!(
            "unknown measure `{ENRICHMENT_MEASURE}` — declare it in {} before writing an enrichment",
            measures_path.display()
        )));
    }
    let unit = registry.declared_unit(&measure).ok_or_else(|| {
        FlowError::InvalidArguments(format!(
            "measure `{ENRICHMENT_MEASURE}` has no declared unit in {}",
            measures_path.display()
        ))
    })?;

    let target = EnrichmentTargetRequest {
        verdict: request.verdict,
        project: request.project,
        closure: request.closure,
        from_device: request.from_device,
    };
    let (records, resolved) = resolve_target_with_records(root, &target)?;
    let verdict: Cid = resolved.verdict.parse().map_err(|_| {
        FlowError::InvalidArguments("resolved gate verdict is not a content identifier".into())
    })?;
    let fields = VerdictFields {
        project: resolved.project,
        closure: resolved.closure,
        commit: resolved.commit,
    };
    let device = resolved.device;

    if let Some((cid, _)) = records.iter().find(|(_, record)| {
        same_enrichment(
            record,
            &verdict,
            kind,
            outcome,
            method,
            run.as_deref(),
            &device,
        )
    }) {
        return Ok(receipt(
            false, *cid, verdict, kind, outcome, &fields, device,
        ));
    }

    let (_, occurred_at) = head_commit_provenance(root).ok_or_else(|| {
        FlowError::InvalidArguments(format!(
            "cannot date an enrichment in `{}`: git has no HEAD commit",
            root.display()
        ))
    })?;
    let mut env = BTreeMap::from([
        ("closure", fields.closure.clone()),
        ("commit", fields.commit.clone()),
        ("device", device.clone()),
        ("kind", kind.as_str().to_string()),
        ("method", method.to_string()),
        ("outcome", outcome.as_str().to_string()),
        ("project", fields.project.clone()),
        ("verdict", verdict.to_string()),
    ]);
    if let Some(run) = run {
        env.insert("run", run);
    }
    if let Some(seconds) = seconds {
        env.insert("seconds", seconds);
    }
    if let Some(reason) = reason {
        env.insert("reason", reason.to_string());
    }
    let mut classified_as = vec![
        "run:observation".to_string(),
        ".".to_string(),
        format!("reason:{}", kind.as_str()),
        format!("measure:{ENRICHMENT_MEASURE}"),
        "value:1".to_string(),
        format!("unit:{unit}"),
    ];
    classified_as.extend(
        env.into_iter()
            .map(|(key, value)| format!("env:{key}={value}")),
    );
    let record = FlowRecord::Event(FlowEvent {
        action: ReaVerb::Cite,
        provider: AgentRef(device.clone()),
        receiver: repository_agent(root)?,
        resource: verdict,
        quantity: Magnitude::Count {
            value: 1.0,
            unit: super::note::NOTE_UNIT.to_string(),
        },
        process: None,
        in_scope_of: repository_scope(root)?,
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at,
        classified_as,
    });
    let cid = record.cid()?;
    // Target selection and tuple lookup use the store's shared read. The exclusive transaction is
    // acquired only for the append, so a large fold never blocks unrelated readers while it is
    // being scanned.
    let mut store = SidecarFlowStore::open(root)?.transaction()?;
    store.append(record)?;
    Ok(receipt(true, cid, verdict, kind, outcome, &fields, device))
}

/// Resolve the immutable gate verdict a sampler will re-execute without appending anything.
pub fn resolve_target(
    root: &Path,
    request: &EnrichmentTargetRequest<'_>,
) -> FlowResult<EnrichmentTargetReceipt> {
    let (_, receipt) = resolve_target_with_records(root, request)?;
    Ok(receipt)
}

fn resolve_target_with_records(
    root: &Path,
    request: &EnrichmentTargetRequest<'_>,
) -> FlowResult<(Vec<(Cid, FlowRecord)>, EnrichmentTargetReceipt)> {
    let selector = enrichment_target(request)?;
    let key_path = crate::device_key::resolve_path().map_err(|error| {
        FlowError::InvalidArguments(format!(
            "flow enrich cannot resolve this device's key: {error}"
        ))
    })?;
    let device_key = crate::device_key::DeviceKey::load(&key_path).map_err(|error| {
        FlowError::InvalidArguments(format!(
            "flow enrich cannot read this device's key: {error}"
        ))
    })?;
    let device = device_key.did_key();
    let store = SidecarFlowStore::open(root)?;
    let records = store.records()?;
    let admits = |writer: &str| {
        crate::actor::standing_here(root).is_some_and(|standing| {
            standing.device == device
                && crate::actor::device_enrolled(root, &standing.handle, writer)
        })
    };
    let (verdict, fields) = resolve_verdict(&records, selector, &device, admits)?;
    let receipt = EnrichmentTargetReceipt {
        verdict: verdict.to_string(),
        project: fields.project,
        closure: fields.closure,
        commit: fields.commit,
        device,
    };
    Ok((records, receipt))
}

enum EnrichmentTarget<'a> {
    Verdict(Cid),
    Selector {
        project: &'a str,
        closure: &'a str,
        from_device: Option<&'a str>,
    },
}

fn enrichment_target<'a>(
    request: &'a EnrichmentTargetRequest<'_>,
) -> FlowResult<EnrichmentTarget<'a>> {
    match (
        request.verdict,
        request.project,
        request.closure,
        request.from_device,
    ) {
        (Some(verdict), None, None, None) => {
            verdict.parse().map(EnrichmentTarget::Verdict).map_err(|_| {
                FlowError::InvalidArguments(format!(
                    "--verdict `{verdict}` is not a content identifier"
                ))
            })
        }
        (Some(_), _, _, _) => Err(FlowError::InvalidArguments(
            "--verdict cannot be combined with --project, --closure, or --from-device".into(),
        )),
        (None, Some(project), Some(closure), from_device) => Ok(EnrichmentTarget::Selector {
            project: one_line(project, "--project")?,
            closure: one_line(closure, "--closure")?,
            from_device: from_device
                .map(|value| one_line(value, "--from-device"))
                .transpose()?,
        }),
        (None, _, _, _) => Err(FlowError::InvalidArguments(
            "enrich needs either --verdict <cid> or --project <p> --closure <key>".into(),
        )),
    }
}

/// Resolve a selector from the latest matching green in store order.
///
/// `FlowStore::records` preserves fold order. Walking it backwards deliberately defines newest as
/// latest in that order, not by clocks written on different devices.
fn resolve_verdict(
    records: &[(Cid, FlowRecord)],
    target: EnrichmentTarget<'_>,
    device: &str,
    admits: impl Fn(&str) -> bool,
) -> FlowResult<(Cid, VerdictFields)> {
    match target {
        EnrichmentTarget::Verdict(verdict) => records
            .iter()
            .find_map(|(cid, record)| {
                (cid == &verdict)
                    .then(|| gate_verdict_fields(record, device, &admits))
                    .flatten()
            })
            .map(|fields| (verdict, fields))
            .ok_or_else(|| {
                FlowError::InvalidArguments(format!(
                    "no gate-verdict@1 with CID {verdict} is in this device's fold"
                ))
            }),
        EnrichmentTarget::Selector {
            project,
            closure,
            from_device,
        } => records
            .iter()
            .rev()
            .find_map(|(cid, record)| {
                let fields = gate_verdict_fields(record, device, &admits)?;
                let FlowRecord::Event(event) = record else {
                    return None;
                };
                let source_matches = match (from_device, slots(event, "via:").as_slice()) {
                    (None, []) => true,
                    (Some(expected), [writer]) => expected == *writer,
                    _ => false,
                };
                (fields.project == project
                    && fields.closure == closure
                    && one_slot(event, "env:success=") == Some("true")
                    && source_matches)
                    .then_some((*cid, fields))
            })
            .ok_or_else(|| {
                let source = from_device.unwrap_or(device);
                FlowError::InvalidArguments(format!(
                    "no green gate-verdict@1 for {project} {closure} from {source} in this device's fold"
                ))
            }),
    }
}

fn gate_verdict_fields(
    record: &FlowRecord,
    me: &str,
    admits: impl Fn(&str) -> bool,
) -> Option<VerdictFields> {
    let FlowRecord::Event(event) = record else {
        return None;
    };
    if event.action != ReaVerb::Cite
        || one_slot(event, "measure:")? != GATE_VERDICT_MEASURE
        || one_slot(event, "reason:")? != "verdict"
    {
        return None;
    }
    match slots(event, "via:").as_slice() {
        [] => {}
        [writer] if *writer != me && admits(writer) => {}
        _ => return None,
    }
    let value = |name: &str| one_slot(event, &format!("env:{name}="));
    let project = nonempty(value("project")?)?;
    let closure = nonempty(value("closure")?)?;
    let commit = nonempty(value("commit")?)?;
    match value("success")? {
        "true" | "false" => {}
        _ => return None,
    }
    let seconds = value("seconds")?.parse::<f64>().ok()?;
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    Some(VerdictFields {
        project: project.to_string(),
        closure: closure.to_string(),
        commit: commit.to_string(),
    })
}

fn same_enrichment(
    record: &FlowRecord,
    verdict: &Cid,
    kind: EnrichmentKind,
    outcome: EnrichmentOutcome,
    method: &str,
    run: Option<&str>,
    device: &str,
) -> bool {
    let FlowRecord::Event(event) = record else {
        return false;
    };
    let verdict_text = verdict.to_string();
    event.action == ReaVerb::Cite
        && event.resource == *verdict
        && slots(event, "via:").is_empty()
        && one_slot(event, "measure:") == Some(ENRICHMENT_MEASURE)
        && one_slot(event, "env:kind=") == Some(kind.as_str())
        && one_slot(event, "env:outcome=") == Some(outcome.as_str())
        && one_slot(event, "env:method=") == Some(method)
        && one_slot(event, "env:verdict=") == Some(verdict_text.as_str())
        && one_slot(event, "env:device=") == Some(device)
        && optional_slot(event, "env:run=") == Some(run)
}

fn receipt(
    appended: bool,
    cid: Cid,
    verdict: Cid,
    kind: EnrichmentKind,
    outcome: EnrichmentOutcome,
    fields: &VerdictFields,
    device: String,
) -> EnrichmentReceipt {
    EnrichmentReceipt {
        appended,
        cid: cid.to_string(),
        verdict: verdict.to_string(),
        kind: kind.as_str().to_string(),
        outcome: outcome.as_str().to_string(),
        project: fields.project.clone(),
        closure: fields.closure.clone(),
        device,
    }
}

fn one_slot<'a>(event: &'a FlowEvent, prefix: &str) -> Option<&'a str> {
    let mut values = event
        .classified_as
        .iter()
        .filter_map(|slot| slot.strip_prefix(prefix));
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

fn optional_slot<'a>(event: &'a FlowEvent, prefix: &str) -> Option<Option<&'a str>> {
    let values = slots(event, prefix);
    match values.as_slice() {
        [] => Some(None),
        [value] => Some(Some(*value)),
        _ => None,
    }
}

fn slots<'a>(event: &'a FlowEvent, prefix: &str) -> Vec<&'a str> {
    event
        .classified_as
        .iter()
        .filter_map(|slot| slot.strip_prefix(prefix))
        .collect()
}

fn one_line<'a>(raw: &'a str, flag: &str) -> FlowResult<&'a str> {
    let value = raw.trim();
    if value.is_empty() || value.contains(['\n', '\r']) {
        Err(FlowError::InvalidArguments(format!(
            "{flag} needs nonempty one-line text"
        )))
    } else {
        Ok(value)
    }
}

fn nonempty(raw: &str) -> Option<&str> {
    (!raw.is_empty()).then_some(raw)
}

fn canonical_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn short(raw: &str) -> String {
    raw.parse::<Cid>().map_or_else(
        |_| raw.chars().take(12).collect(),
        |cid| super::short_cid(&cid),
    )
}
